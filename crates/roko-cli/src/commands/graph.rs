//! `roko graph` subcommand handlers: run, validate, and show graph definitions.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use clap::Subcommand;

use roko_core::trigger::{TriggerEvent, TriggerSource};
use roko_core::{Body, Kind, Provenance, Signal, TelemetryEventSink};
use roko_graph::{CellContext, GraphEngine, GraphOutput, default_registry, loader};
use roko_runtime::{LensExecutor, LensQueueConfig, QueuedLensExecutor, SharedStateHub};

/// Exit code for success.
const EXIT_SUCCESS: i32 = 0;
/// Exit code for failure.
const EXIT_FAILURE: i32 = 1;

/// Subcommands for `roko graph`.
#[derive(Debug, Subcommand)]
pub enum GraphCmd {
    /// Execute a graph definition file.
    #[command(after_help = "\
Examples:
  roko graph run examples/graphs/linear-gates.toml
  roko graph run my-pipeline.toml")]
    Run {
        /// Path to a graph TOML file.
        path: PathBuf,
    },
    /// Validate a graph definition (check for cycles, unknown cell types, unresolved refs).
    #[command(after_help = "\
Examples:
  roko graph validate examples/graphs/linear-gates.toml
  roko graph validate my-pipeline.toml")]
    Validate {
        /// Path to a graph TOML file.
        path: PathBuf,
    },
    /// Show a summary of nodes and edges in a graph definition.
    #[command(after_help = "\
Examples:
  roko graph show examples/graphs/linear-gates.toml")]
    Show {
        /// Path to a graph TOML file.
        path: PathBuf,
    },
}

/// Dispatch a `roko graph` subcommand.
pub async fn cmd_graph(cmd: GraphCmd) -> Result<i32> {
    match cmd {
        GraphCmd::Run { path } => cmd_graph_run(&path).await,
        GraphCmd::Validate { path } => cmd_graph_validate(&path),
        GraphCmd::Show { path } => cmd_graph_show(&path),
    }
}

/// Execute a graph: load the TOML, build the engine with the default registry,
/// run all nodes sequentially, and print the results.
async fn cmd_graph_run(path: &Path) -> Result<i32> {
    let telemetry_hub = SharedStateHub::new_in_process();
    let output = execute_graph(path, &telemetry_hub, None, None).await?;

    // Print the summary
    println!("{}", output.summary());
    let projections = telemetry_hub.projections();
    if !projections.is_empty() {
        println!("Telemetry projections:");
        for (id, projection) in projections {
            println!("  {id} v{}: {}", projection.version, projection.data);
        }
    }

    if output.success {
        Ok(EXIT_SUCCESS)
    } else {
        Ok(EXIT_FAILURE)
    }
}

/// Execute one graph without printing, using the caller's live state hub for
/// Lens projection and operator control.
pub(crate) async fn execute_graph(
    path: &Path,
    telemetry_hub: &SharedStateHub,
    trigger_event: Option<&TriggerEvent>,
    capabilities: Option<&roko_core::CapabilitySet>,
) -> Result<GraphOutput> {
    let graph = loader::load_from_file(path)
        .map_err(|e| anyhow!("failed to load graph '{}': {}", path.display(), e))?;
    let telemetry_runtime_id = trigger_event.map_or_else(
        || format!("graph:{}", graph.metadata.name),
        |event| format!("graph:{}:{}", graph.metadata.name, event.trace_id),
    );

    let mut queued_telemetry: Option<QueuedLensExecutor> = None;
    let telemetry = if graph.lenses.registrations().is_empty() {
        None
    } else {
        let executor = LensExecutor::from_registry(&graph.lenses, telemetry_hub.sender())
            .map_err(|error| anyhow!("failed to initialize graph telemetry: {error}"))?;
        let queued = executor
            .into_queued(telemetry_runtime_id, LensQueueConfig::default())
            .map_err(|error| anyhow!("failed to initialize graph telemetry queue: {error}"))?;
        queued_telemetry = Some(queued.clone());
        Some(Arc::new(queued) as Arc<dyn TelemetryEventSink>)
    };

    let registry = default_registry();
    let mut engine = GraphEngine::new(graph, registry);
    if let Some(event) = trigger_event {
        engine = engine.with_root_inputs(vec![trigger_input_signal(event)]);
    }
    if let Some(telemetry) = telemetry {
        engine = engine.with_telemetry(telemetry);
    }
    let mut ctx = trigger_event.map_or_else(CellContext::new, |event| {
        CellContext::new()
            .with_trace_id(event.trace_id.clone())
            .with_run_id(event.trace_id.clone())
    });
    if let Some(capabilities) = capabilities {
        ctx = ctx.with_capabilities(capabilities.clone());
    }

    // Validate before running
    let issues = engine.validate();
    if !issues.is_empty() {
        return Err(anyhow!(
            "graph validation failed: {}",
            issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    let output = engine.execute(&ctx).await;
    if let Some(queue) = &queued_telemetry
        && !queue.wait_idle(Duration::from_secs(30)).await
    {
        return Err(anyhow!(
            "timed out waiting for queued graph telemetry to drain"
        ));
    }
    output.map_err(|e| anyhow!("graph execution error: {e}"))
}

/// Convert a trigger firing into the universal Signal accepted by Graph roots.
///
/// The coordinator has already applied the binding's input mapping to
/// `event.payload`; retaining the event metadata in tags preserves correlation
/// without forcing every Cell to understand the Trigger protocol type.
fn trigger_input_signal(event: &TriggerEvent) -> Signal {
    let provenance = match &event.source {
        TriggerSource::Manual { user } => Provenance::user(user.clone()),
        _ => Provenance::external(format!("trigger:{}", event.trigger_id)),
    };
    let mut signal = Signal::builder(Kind::Custom("trigger.input".to_string()))
        .body(Body::Json(event.payload.clone()))
        .provenance(provenance)
        .tag("trigger_id", event.trigger_id.clone())
        .tag("trace_id", event.trace_id.clone());
    if let Some(space_id) = &event.space_id {
        signal = signal.tag("space_id", space_id.clone());
    }
    signal.build()
}

/// Validate a graph definition without executing it.
fn cmd_graph_validate(path: &Path) -> Result<i32> {
    let graph = loader::load_from_file(path)
        .map_err(|e| anyhow!("failed to load graph '{}': {}", path.display(), e))?;

    let registry = default_registry();
    let engine = GraphEngine::new(graph, registry);
    let issues = engine.validate();

    if issues.is_empty() {
        println!("graph '{}' is valid", path.display());
        Ok(EXIT_SUCCESS)
    } else {
        println!("graph '{}' has {} issue(s):", path.display(), issues.len());
        for issue in &issues {
            println!("  - {issue}");
        }
        Ok(EXIT_FAILURE)
    }
}

/// Show a summary of nodes and edges in a graph.
fn cmd_graph_show(path: &Path) -> Result<i32> {
    let graph = loader::load_from_file(path)
        .map_err(|e| anyhow!("failed to load graph '{}': {}", path.display(), e))?;
    let registry = default_registry();

    println!("Graph: {}", graph.metadata.name);
    if let Some(desc) = &graph.metadata.description {
        println!("Description: {desc}");
    }
    if let Some(ver) = &graph.metadata.version {
        println!("Version: {ver}");
    }
    println!();

    // Print nodes, annotating with resolved cell status from the registry.
    let mut stub_count: usize = 0;
    println!("Nodes ({}):", graph.node_count());
    for (node_id, idx) in &graph.node_map {
        let node = &graph.inner[*idx];
        let resolution = match registry.create(
            &node.cell_type,
            node.config.clone(),
        ) {
            Ok(cell) if cell.is_stub() => {
                stub_count += 1;
                " (STUB)"
            }
            Ok(_) => "",
            Err(_) => " (UNRESOLVED)",
        };
        println!("  {node_id}  [cell_type: {}]{resolution}", node.cell_type);
        if !node.inputs.is_empty() {
            println!("    inputs: {}", node.inputs.join(", "));
        }
        if !node.outputs.is_empty() {
            println!("    outputs: {}", node.outputs.join(", "));
        }
    }
    if stub_count > 0 {
        println!(
            "\n  WARNING: {stub_count} node(s) resolve to stub cells. \
             These need real implementations before production use."
        );
    }
    println!();

    // Print edges using the raw edge indices from petgraph
    println!("Edges ({}):", graph.edge_count());
    for edge_idx in graph.inner.edge_indices() {
        let edge = &graph.inner[edge_idx];
        let cond = match &edge.condition {
            Some(c) => format!(" [condition: {c:?}]"),
            None => String::new(),
        };
        println!("  {} -> {}{}", edge.from, edge.to, cond);
    }

    // Print topological order if DAG
    if let Ok(order) = roko_graph::topo::topological_order(&graph) {
        println!();
        println!("Execution order: {}", order.join(" -> "));
    }

    Ok(EXIT_SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::trigger::TriggerSource;
    use serde_json::json;

    #[tokio::test]
    async fn trigger_payload_is_injected_into_real_graph_root() {
        let directory = tempfile::tempdir().expect("tempdir");
        let graph_path = directory.path().join("trigger-input.toml");
        std::fs::write(
            &graph_path,
            r#"
[graph]
name = "trigger-input"

[[nodes]]
id = "root"
cell_type = "noop"
"#,
        )
        .expect("write graph");
        let event = TriggerEvent::new(
            "deploy".to_string(),
            json!({"event": {"ref": "refs/heads/main"}, "inputs": {"branch": "main"}}),
            TriggerSource::Manual {
                user: "operator".to_string(),
            },
            "trace-root-input".to_string(),
        )
        .with_space("alpha".to_string());

        let signal = trigger_input_signal(&event);
        assert_eq!(signal.body, Body::Json(event.payload.clone()));
        assert_eq!(signal.tag("space_id"), Some("alpha"));

        let output = execute_graph(
            &graph_path,
            &SharedStateHub::new_in_process(),
            Some(&event),
            None,
        )
        .await
        .expect("execute graph");
        assert!(output.success);
        assert_eq!(output.node_results[0].output_count, 1);
    }
}
