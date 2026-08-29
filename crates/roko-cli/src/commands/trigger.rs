//! `roko trigger` -- manage trigger bindings.
//!
//! Trigger bindings live as TOML files in `.roko/triggers/`. Each binding
//! connects an event source (cron, webhook, signal, etc.) to a graph that
//! is executed when the trigger fires.

use anyhow::{Context as _, Result};
use clap::Subcommand;
use roko_core::trigger::{
    TriggerBinding, TriggerEvent, TriggerEventKind, TriggerKind, TriggerSource,
    load_trigger_history, valid_trigger_name,
};
use roko_fs::RokoLayout;

use crate::*;

/// Trigger management subcommands.
#[derive(Debug, Subcommand)]
pub enum TriggerCmd {
    /// List all trigger bindings from `.roko/triggers/`.
    List {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show details of a named trigger binding.
    Show {
        /// Trigger binding name (file stem in `.roko/triggers/`).
        name: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Create a new trigger binding.
    Create {
        /// Trigger binding name (used as file stem).
        name: String,
        /// Trigger kind: webhook, cron, signal, manual.
        #[arg(long)]
        kind: String,
        /// Graph reference to fire when the trigger activates.
        #[arg(long)]
        graph: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Manually fire a trigger, publishing a TriggerEvent.
    Fire {
        /// Trigger binding name to fire.
        name: String,
        /// JSON payload to include in the trigger event.
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show durable firing history with correlated Flow run references.
    History {
        /// Trigger binding name.
        name: String,
        /// Maximum number of recent firings to return.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_trigger(cli: &Cli, cmd: TriggerCmd) -> Result<i32> {
    match cmd {
        TriggerCmd::List { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_list(cli, &wd)
        }
        TriggerCmd::Show { name, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_show(cli, &wd, &name)
        }
        TriggerCmd::Create {
            name,
            kind,
            graph,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_create(cli, &wd, &name, &kind, &graph)
        }
        TriggerCmd::Fire {
            name,
            payload,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_fire(cli, &wd, &name, &payload).await
        }
        TriggerCmd::History {
            name,
            limit,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_history(cli, &wd, &name, limit)
        }
    }
}

/// Directory where trigger bindings are stored.
fn triggers_dir(workdir: &Path) -> PathBuf {
    RokoLayout::for_project(workdir).triggers_dir()
}

/// Load a single trigger binding from its TOML file.
fn load_binding(path: &Path) -> Result<TriggerBinding> {
    TriggerBinding::load_from_file(path)
        .with_context(|| format!("load trigger binding {}", path.display()))
}

/// Load all trigger bindings from the triggers directory.
fn load_all_bindings(workdir: &Path) -> Result<Vec<TriggerBinding>> {
    TriggerBinding::load_all(&triggers_dir(workdir)).context("load trigger bindings")
}

fn cmd_list(cli: &Cli, workdir: &Path) -> Result<i32> {
    let bindings = load_all_bindings(workdir)?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&bindings)?);
        return Ok(EXIT_SUCCESS);
    }

    if bindings.is_empty() {
        println!("No trigger bindings found.");
        println!("Create one with: roko trigger create <name> --kind <kind> --graph <graph>");
        return Ok(EXIT_SUCCESS);
    }

    println!(
        "{:<20} {:<12} {:<30} {}",
        "NAME", "KIND", "GRAPH", "ENABLED"
    );
    println!("{}", "-".repeat(70));

    for b in &bindings {
        let kind_label = trigger_kind_label(&b.kind);
        println!(
            "{:<20} {:<12} {:<30} {}",
            b.name,
            kind_label,
            b.graph,
            if b.enabled { "yes" } else { "no" }
        );
    }
    println!("\n{} trigger(s)", bindings.len());

    Ok(EXIT_SUCCESS)
}

fn cmd_show(cli: &Cli, workdir: &Path, name: &str) -> Result<i32> {
    anyhow::ensure!(valid_trigger_name(name), "invalid trigger name '{name}'");
    let path = triggers_dir(workdir).join(format!("{name}.toml"));
    if !path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' not found")})
            );
        } else {
            eprintln!("trigger '{name}' not found");
            eprintln!("  -> Run `roko trigger list` to see available triggers.");
        }
        return Ok(EXIT_FAILURE);
    }

    let binding = load_binding(&path)?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&binding)?);
        return Ok(EXIT_SUCCESS);
    }

    println!("Trigger: {}", binding.name);
    println!("  kind:        {}", trigger_kind_label(&binding.kind));
    println!("  graph:       {}", binding.graph);
    println!("  enabled:     {}", binding.enabled);
    println!("  concurrency: {}", concurrency_label(&binding.concurrency));

    match &binding.kind {
        TriggerKind::Cron(c) => {
            println!("  expression:  {}", c.expression);
            if let Some(tz) = &c.timezone {
                println!("  timezone:    {tz}");
            }
        }
        TriggerKind::Webhook(w) => {
            println!("  path:        {}", w.path);
            if let Some(method) = &w.method {
                println!("  method:      {method}");
            }
        }
        TriggerKind::FileWatch(fw) => {
            println!("  watch_path:  {}", fw.path.display());
            let events: Vec<_> = fw.events.iter().map(|e| format!("{e:?}")).collect();
            println!("  events:      {}", events.join(", "));
            if let Some(g) = &fw.glob {
                println!("  glob:        {g}");
            }
        }
        TriggerKind::Bus(b) => {
            println!("  topic:       {}", b.topic);
        }
        TriggerKind::ChainEvent(ce) => {
            println!("  chain_id:    {}", ce.chain_id);
            println!("  contract:    {}", ce.contract);
            println!("  event_sig:   {}", ce.event_signature);
        }
        TriggerKind::Manual => {
            println!("  (manually fired)");
        }
        TriggerKind::SignalPattern(sp) => {
            println!("  description: {}", sp.description);
            println!("  required:    {}", sp.required_kinds.join(", "));
            println!("  window_secs: {}", sp.window_secs);
        }
    }

    if let Some(space) = &binding.space {
        println!("  space:       {space}");
    }
    if binding.filter.is_some() {
        println!("  filter:      (configured)");
    }
    if binding.auth.is_some() {
        println!("  auth:        (configured)");
    }

    Ok(EXIT_SUCCESS)
}

fn cmd_create(cli: &Cli, workdir: &Path, name: &str, kind: &str, graph: &str) -> Result<i32> {
    let trigger_kind = parse_trigger_kind(kind)?;
    let binding = TriggerBinding::new(name, trigger_kind, graph);
    binding.validate().context("validate trigger binding")?;

    let dir = triggers_dir(workdir);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create triggers directory {}", dir.display()))?;

    let path = dir.join(format!("{name}.toml"));
    if path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' already exists")})
            );
        } else {
            eprintln!("trigger '{name}' already exists at {}", path.display());
        }
        return Ok(EXIT_FAILURE);
    }

    binding
        .save_to_file(&path)
        .with_context(|| format!("write trigger binding to {}", path.display()))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&binding)?);
    } else {
        println!("Created trigger '{name}' at {}", path.display());
        println!("  kind:  {kind}");
        println!("  graph: {graph}");
    }

    Ok(EXIT_SUCCESS)
}

async fn cmd_fire(cli: &Cli, workdir: &Path, name: &str, payload_str: &str) -> Result<i32> {
    anyhow::ensure!(valid_trigger_name(name), "invalid trigger name '{name}'");
    let path = triggers_dir(workdir).join(format!("{name}.toml"));
    if !path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' not found")})
            );
        } else {
            eprintln!("trigger '{name}' not found");
            eprintln!("  -> Run `roko trigger list` to see available triggers.");
        }
        return Ok(EXIT_FAILURE);
    }

    let binding = load_binding(&path)?;
    if !binding.enabled {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' is disabled")})
            );
        } else {
            eprintln!("trigger '{name}' is disabled");
            eprintln!("  -> Run `roko trigger enable {name}` to enable it first.");
        }
        return Ok(EXIT_FAILURE);
    }

    let payload: serde_json::Value =
        serde_json::from_str(payload_str).context("parse --payload as JSON")?;

    let trace_id = format!("manual-{}", uuid::Uuid::new_v4());
    let mut event = TriggerEvent::new(
        name.to_string(),
        payload,
        TriggerSource::Manual {
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
        },
        trace_id.clone(),
    );
    if let Some(space_id) = &binding.space {
        event = event.with_space(space_id.clone());
    }

    // Persist into the durable inbox. A running server claims these files and
    // records the resulting lifecycle/event evidence separately.
    let events_dir = triggers_dir(workdir).join("inbox");
    std::fs::create_dir_all(&events_dir)
        .with_context(|| format!("create trigger inbox directory {}", events_dir.display()))?;

    let event_path = events_dir.join(format!("{name}-{trace_id}.json"));
    let temporary_path = event_path.with_extension("json.tmp");
    let event_json = serde_json::to_string_pretty(&event).context("serialize trigger event")?;
    std::fs::write(&temporary_path, &event_json)
        .with_context(|| format!("write trigger event to {}", temporary_path.display()))?;
    std::fs::rename(&temporary_path, &event_path)
        .with_context(|| format!("commit trigger event to {}", event_path.display()))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&event)?);
    } else {
        println!("Fired trigger '{name}'");
        println!("  trace_id: {trace_id}");
        println!("  graph:    {}", binding.graph);
        println!("  event:    {}", event_path.display());
    }

    Ok(EXIT_SUCCESS)
}

fn cmd_history(cli: &Cli, workdir: &Path, name: &str, limit: usize) -> Result<i32> {
    anyhow::ensure!(valid_trigger_name(name), "invalid trigger name '{name}'");
    anyhow::ensure!(
        (1..=1_000).contains(&limit),
        "history limit must be between 1 and 1000"
    );
    let binding_path = triggers_dir(workdir).join(format!("{name}.toml"));
    if !binding_path.is_file() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' not found")})
            );
        } else {
            eprintln!("trigger '{name}' not found");
            eprintln!("  -> Run `roko trigger list` to see available triggers.");
        }
        return Ok(EXIT_FAILURE);
    }

    let history = load_trigger_history(&triggers_dir(workdir), name, limit)
        .with_context(|| format!("read durable history for trigger '{name}'"))?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&history)?);
        return Ok(EXIT_SUCCESS);
    }

    if history.records.is_empty() {
        println!("No durable firings found for trigger '{name}'.");
        return Ok(EXIT_SUCCESS);
    }

    println!("History for trigger '{name}' ({} total)", history.total);
    println!(
        "{:<14} {:<38} {:<13} {:<38} {}",
        "FIRED_AT_MS", "TRACE_ID", "STATUS", "RUN_ID", "SOURCE"
    );
    for record in history.records {
        let completed = record
            .lifecycle
            .iter()
            .rev()
            .find(|event| event.kind == TriggerEventKind::FlowCompleted);
        let started = record
            .lifecycle
            .iter()
            .find(|event| event.kind == TriggerEventKind::FlowStarted);
        let status = completed.map_or("running", |event| {
            if event
                .detail
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                "completed"
            } else if event
                .detail
                .get("cancelled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                "cancelled"
            } else {
                "failed"
            }
        });
        let run_id = completed
            .or(started)
            .and_then(|event| event.detail.get("run_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-");
        println!(
            "{:<14} {:<38} {:<13} {:<38} {}",
            record.event.fired_at_ms,
            record.event.trace_id,
            status,
            run_id,
            trigger_source_label(&record.event.source),
        );
    }
    Ok(EXIT_SUCCESS)
}

/// Parse a user-supplied kind string into a `TriggerKind`.
///
/// For kinds that require additional configuration (cron expression, webhook
/// path, etc.), we create a minimal placeholder that the user can edit in the
/// generated TOML file.
fn parse_trigger_kind(kind: &str) -> Result<TriggerKind> {
    match kind.to_lowercase().as_str() {
        "webhook" => Ok(TriggerKind::Webhook(roko_core::trigger::WebhookTrigger {
            method: Some("POST".to_string()),
            path: "/hook/placeholder".to_string(),
        })),
        "cron" => Ok(TriggerKind::Cron(roko_core::trigger::CronTrigger {
            expression: "0 * * * *".to_string(),
            timezone: None,
        })),
        "signal" => Ok(TriggerKind::SignalPattern(
            roko_core::trigger::SignalPatternTrigger {
                description: "Signal pattern trigger".to_string(),
                required_kinds: vec![],
                window_secs: 60,
            },
        )),
        "manual" => Ok(TriggerKind::Manual),
        "bus" => Ok(TriggerKind::Bus(roko_core::trigger::BusTrigger {
            topic: "*".to_string(),
        })),
        "filewatch" | "file_watch" | "file-watch" => Ok(TriggerKind::FileWatch(
            roko_core::trigger::FileWatchTrigger {
                path: PathBuf::from("."),
                events: vec![roko_core::trigger::FileWatchEvent::Any],
                glob: None,
            },
        )),
        "chain" | "chain_event" | "chain-event" => Ok(TriggerKind::ChainEvent(
            roko_core::trigger::ChainEventTrigger {
                chain_id: 1,
                contract: "0x0000000000000000000000000000000000000000".to_string(),
                event_signature: "Transfer(address,address,uint256)".to_string(),
                abi: None,
                finality: roko_core::trigger::FinalityRequirement::default(),
            },
        )),
        other => anyhow::bail!(
            "unknown trigger kind '{other}'; valid kinds: webhook, cron, signal, manual, bus, filewatch, chain"
        ),
    }
}

/// Human-readable label for a `TriggerKind`.
fn trigger_kind_label(kind: &TriggerKind) -> &'static str {
    match kind {
        TriggerKind::Cron(_) => "cron",
        TriggerKind::Webhook(_) => "webhook",
        TriggerKind::FileWatch(_) => "filewatch",
        TriggerKind::Bus(_) => "bus",
        TriggerKind::ChainEvent(_) => "chain",
        TriggerKind::Manual => "manual",
        TriggerKind::SignalPattern(_) => "signal",
    }
}

fn trigger_source_label(source: &TriggerSource) -> &'static str {
    match source {
        TriggerSource::Cron { .. } => "cron",
        TriggerSource::Webhook { .. } => "webhook",
        TriggerSource::FileWatch { .. } => "filewatch",
        TriggerSource::Bus { .. } => "bus",
        TriggerSource::ChainEvent { .. } => "chain",
        TriggerSource::Manual { .. } => "manual",
        TriggerSource::SignalPattern { .. } => "signal",
    }
}

/// Human-readable label for a `ConcurrencyPolicy`.
fn concurrency_label(policy: &roko_core::trigger::ConcurrencyPolicy) -> String {
    match policy {
        roko_core::trigger::ConcurrencyPolicy::Queue { max_depth } => match max_depth {
            Some(d) => format!("queue (max {d})"),
            None => "queue (unbounded)".to_string(),
        },
        roko_core::trigger::ConcurrencyPolicy::Skip => "skip".to_string(),
        roko_core::trigger::ConcurrencyPolicy::CancelRunning => "cancel_running".to_string(),
        roko_core::trigger::ConcurrencyPolicy::Parallel { max_concurrent } => {
            match max_concurrent {
                Some(c) => format!("parallel (max {c})"),
                None => "parallel (unbounded)".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trigger_kind_webhook() {
        let kind = parse_trigger_kind("webhook").unwrap();
        assert!(matches!(kind, TriggerKind::Webhook(_)));
    }

    #[test]
    fn parse_trigger_kind_cron() {
        let kind = parse_trigger_kind("cron").unwrap();
        assert!(matches!(kind, TriggerKind::Cron(_)));
    }

    #[test]
    fn parse_trigger_kind_signal() {
        let kind = parse_trigger_kind("signal").unwrap();
        assert!(matches!(kind, TriggerKind::SignalPattern(_)));
    }

    #[test]
    fn parse_trigger_kind_manual() {
        let kind = parse_trigger_kind("manual").unwrap();
        assert!(matches!(kind, TriggerKind::Manual));
    }

    #[test]
    fn parse_trigger_kind_bus() {
        let kind = parse_trigger_kind("bus").unwrap();
        assert!(matches!(kind, TriggerKind::Bus(_)));
    }

    #[test]
    fn parse_trigger_kind_filewatch_variants() {
        for variant in &["filewatch", "file_watch", "file-watch"] {
            let kind = parse_trigger_kind(variant).unwrap();
            assert!(
                matches!(kind, TriggerKind::FileWatch(_)),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn parse_trigger_kind_chain_variants() {
        for variant in &["chain", "chain_event", "chain-event"] {
            let kind = parse_trigger_kind(variant).unwrap();
            assert!(
                matches!(kind, TriggerKind::ChainEvent(_)),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn parse_trigger_kind_unknown_fails() {
        assert!(parse_trigger_kind("unknown").is_err());
    }

    #[test]
    fn parse_trigger_kind_case_insensitive() {
        assert!(parse_trigger_kind("Webhook").is_ok());
        assert!(parse_trigger_kind("CRON").is_ok());
        assert!(parse_trigger_kind("Manual").is_ok());
    }

    #[test]
    fn trigger_kind_labels() {
        assert_eq!(trigger_kind_label(&TriggerKind::Manual), "manual");
        assert_eq!(
            trigger_kind_label(&TriggerKind::Cron(roko_core::trigger::CronTrigger {
                expression: "* * * * *".into(),
                timezone: None,
            })),
            "cron"
        );
    }

    #[test]
    fn concurrency_labels() {
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Skip),
            "skip"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Queue { max_depth: Some(5) }),
            "queue (max 5)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Queue { max_depth: None }),
            "queue (unbounded)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::CancelRunning),
            "cancel_running"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Parallel {
                max_concurrent: Some(3)
            }),
            "parallel (max 3)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Parallel {
                max_concurrent: None
            }),
            "parallel (unbounded)"
        );
    }

    #[test]
    fn load_binding_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let binding = TriggerBinding::new("test-trigger", TriggerKind::Manual, "plans/test.toml");
        let path = tmp.path().join("test-trigger.toml");
        binding.save_to_file(&path).unwrap();

        let loaded = load_binding(&path).unwrap();
        assert_eq!(loaded.name, "test-trigger");
        assert_eq!(loaded.graph, "plans/test.toml");
        assert!(loaded.enabled);
    }

    #[test]
    fn load_all_bindings_from_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bindings = load_all_bindings(tmp.path()).unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn load_all_bindings_skips_non_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let triggers = tmp.path().join(".roko").join("triggers");
        std::fs::create_dir_all(&triggers).unwrap();

        // Write a canonical TOML trigger file.
        let binding = TriggerBinding::new("good", TriggerKind::Manual, "plans/g.toml");
        binding.save_to_file(&triggers.join("good.toml")).unwrap();

        // Write a non-TOML file that should be skipped.
        std::fs::write(triggers.join("readme.txt"), "not a trigger").unwrap();

        let loaded = load_all_bindings(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "good");
    }
}
