//! Agent-work probes of the model ladder's rungs (decision 1119, 3-A;
//! backlog 1121).
//!
//! A rung whose model cannot do agent work, such as GLM-4.7 answering every
//! real attempt with a blank in R3, costs each task two attempts before the
//! ladder climbs. At plan start [`probe_ladder`] gives each rung model that
//! roko's tool loop drives one task: call the `echo` tool once. The call goes
//! through the normal provider factory, so the real response parser and the
//! immune boundary judge it. A model that makes the call passes; a blank
//! answer, an error, a text answer, or no answer within [`PROBE_TIMEOUT_MS`]
//! fails. Verdicts are kept in `.roko/learn/rung-probes.json` and reused for
//! [`PROBE_TTL_MS`], so a day of runs costs one probe per rung. A CLI or ACP
//! harness brings its own tools and is not probed. The plan runner then skips
//! the rungs whose fresh probe failed ([`RoutingLadder::without_models`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use roko_agent::dispatcher::HandlerResolver;
use roko_agent::provider::{AgentOptions, LocalToolRuntime};
use roko_agent::safety::contract::AgentContract;
use roko_agent::{Agent, AgentResult, create_agent_for_model};
use roko_core::config::schema::RokoConfig;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolContext, ToolDef, ToolHandler, ToolPermission, ToolResult,
    ToolSchema,
};
use roko_core::{Body, Context, Kind, Signal};
use roko_learn::provider_failover::provider_brings_own_tools;
use serde::{Deserialize, Serialize};

use super::model_routing::RoutingLadder;

/// How long a probe verdict stands before its rung is probed again.
pub const PROBE_TTL_MS: i64 = 24 * 60 * 60 * 1_000;

/// How long one probe may take.
pub const PROBE_TIMEOUT_MS: u64 = 20_000;

/// The probe cache, under `.roko/learn`.
const PROBES_FILE: &str = "rung-probes.json";

/// The one tool a probe offers, and asks the model to call.
const PROBE_TOOL: &str = "echo";

const PROBE_SYSTEM_PROMPT: &str =
    "You are checking that tool calls work. Use the tool you are given when asked.";

const PROBE_PROMPT: &str = "Call the `echo` tool exactly once, with the arguments \
     {\"text\": \"roko rung probe\"}. Do not answer in text before you call it.";

/// One rung model's probe verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RungProbe {
    /// Whether the model made a well-formed tool call.
    pub passed: bool,
    /// Why the probe failed; empty when it passed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// When the probe ran (unix ms).
    pub probed_at_ms: i64,
}

/// `.roko/learn/rung-probes.json`: the latest probe verdict of each rung
/// model, by the model name dispatch runs for its rung.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RungProbes {
    /// Verdicts, by model.
    #[serde(default)]
    pub models: BTreeMap<String, RungProbe>,
}

impl RungProbes {
    /// Where the workspace at `workdir` keeps its probe verdicts.
    #[must_use]
    pub fn path(workdir: &Path) -> PathBuf {
        roko_fs::RokoLayout::for_project(workdir)
            .learn_dir()
            .join(PROBES_FILE)
    }

    /// The verdicts saved at `path`; none when it is missing or unreadable.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    /// `model`'s verdict, when it is younger than [`PROBE_TTL_MS`] at
    /// `now_ms`.
    #[must_use]
    pub fn fresh(&self, model: &str, now_ms: i64) -> Option<&RungProbe> {
        self.models
            .get(model)
            .filter(|probe| now_ms.saturating_sub(probe.probed_at_ms) < PROBE_TTL_MS)
    }
}

/// Probe each runnable rung model of `ladder` that roko's tool loop drives
/// and that has no fresh verdict, save the verdicts, and return the models
/// whose fresh probe failed, with why.
pub async fn probe_ladder(
    config: &RokoConfig,
    ladder: &RoutingLadder,
    workdir: &Path,
) -> BTreeMap<String, String> {
    let path = RungProbes::path(workdir);
    let mut probes = RungProbes::load(&path);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let models: Vec<String> = ladder
        .rung_models()
        .into_iter()
        .filter(|model| tool_loop_drives(config, model))
        .collect();
    let stale: Vec<&String> = models
        .iter()
        .filter(|model| probes.fresh(model, now_ms).is_none())
        .collect();
    if !stale.is_empty() {
        let verdicts =
            futures::future::join_all(stale.iter().map(|model| probe_rung(config, model, workdir)))
                .await;
        for (model, probe) in stale.into_iter().zip(verdicts) {
            tracing::info!(
                model = %model,
                passed = probe.passed,
                reason = %probe.reason,
                "routing ladder: probed a rung model with one tool-use call"
            );
            probes.models.insert(model.clone(), probe);
        }
        if let Err(error) = roko_fs::atomic_write_json(&path, &probes) {
            tracing::warn!(
                path = %path.display(),
                %error,
                "routing ladder: the rung probe verdicts could not be saved"
            );
        }
    }
    models
        .into_iter()
        .filter_map(|model| {
            let probe = probes.fresh(&model, now_ms)?;
            (!probe.passed).then(|| (model, probe.reason.clone()))
        })
        .collect()
}

/// Whether roko's tool loop drives `model`'s calls. A CLI or ACP harness
/// brings its own tools and is not probed.
fn tool_loop_drives(config: &RokoConfig, model: &str) -> bool {
    roko_core::agent::resolve_model(config, model)
        .provider_config
        .is_some_and(|provider| !provider_brings_own_tools(&provider))
}

/// Give `model` one task, to call the `echo` tool once, through the normal
/// provider factory and immune boundary, and judge what it did.
async fn probe_rung(config: &RokoConfig, model: &str, workdir: &Path) -> RungProbe {
    let probed_at_ms = chrono::Utc::now().timestamp_millis();
    let echo = Arc::new(EchoTool::default());
    let handler = Arc::clone(&echo);
    let resolver: Arc<dyn HandlerResolver> = Arc::new(move |name: &str| {
        (name == PROBE_TOOL).then(|| Arc::clone(&handler) as Arc<dyn ToolHandler>)
    });
    let tools = LocalToolRuntime::new(vec![echo_tool()], resolver);
    let options = AgentOptions {
        // A probe runs under an id of its own, so an isolation the boundary
        // records for it never denies a later probe or a task.
        name: format!("rung-probe-{probed_at_ms}"),
        working_dir: Some(workdir.to_path_buf()),
        immune_root: Some(workdir.to_path_buf()),
        timeout_ms: Some(PROBE_TIMEOUT_MS),
        system_prompt: Some(PROBE_SYSTEM_PROMPT.to_string()),
        pre_discovered_local_tools: Some(Arc::new(tools)),
        agent_contract: Some(AgentContract {
            allowed_tools: Some(vec![PROBE_TOOL.to_string()]),
            ..AgentContract::default()
        }),
        // No safety_layer: the factory builds the production one from these
        // options, whose contract allows only the probe's echo tool.
        ..AgentOptions::default()
    };
    let verdict = match create_agent_for_model(config, model, options) {
        Err(error) => Err(format!("the agent could not be created: {error}")),
        Ok(agent) => run_probe(agent.as_ref(), &echo).await,
    };
    RungProbe {
        passed: verdict.is_ok(),
        reason: verdict.err().unwrap_or_default(),
        probed_at_ms,
    }
}

/// Run the probe task on `agent`: `Ok` once the model called `echo`, else
/// why not.
async fn run_probe(agent: &dyn Agent, echo: &EchoTool) -> Result<(), String> {
    let input = Signal::builder(Kind::Prompt)
        .body(Body::text(PROBE_PROMPT))
        .build();
    let ctx = Context::now();
    let timeout = Duration::from_millis(PROBE_TIMEOUT_MS);
    let Ok(result) = tokio::time::timeout(timeout, agent.run(&input, &ctx)).await else {
        return Err(format!("no answer within {PROBE_TIMEOUT_MS} ms"));
    };
    if echo.called.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(failure_reason(&result))
    }
}

/// Why a probe in which the model made no tool call failed.
fn failure_reason(result: &AgentResult) -> String {
    let text = result.output.body.as_text().unwrap_or_default().trim();
    let class = crate::dispatch_v2::classify_provider_error(&text.to_ascii_lowercase());
    let said: String = text.chars().take(160).collect();
    if text.is_empty() || class == "empty_response" {
        "empty_response: a blank answer with no tool call".to_string()
    } else if result.success {
        format!("no tool call; the model answered: {said}")
    } else {
        format!("{class}: {said}")
    }
}

/// The probe's one tool: `echo`, taking a `text` string.
fn echo_tool() -> ToolDef {
    ToolDef::new(
        PROBE_TOOL,
        "Echo the text back.",
        ToolCategory::Meta,
        ToolPermission::read_only(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "description": "The text to echo." }
        },
        "required": ["text"]
    })))
}

/// The handler of the probe's `echo` tool, which notes that the model called
/// it.
#[derive(Default)]
struct EchoTool {
    called: AtomicBool,
}

#[async_trait::async_trait]
impl ToolHandler for EchoTool {
    fn name(&self) -> &str {
        PROBE_TOOL
    }

    async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
        self.called.store(true, Ordering::SeqCst);
        ToolResult::text(call.arguments.to_string())
    }
}
