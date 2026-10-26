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
//! [`PROBE_TTL_MS`], so a day of runs costs one probe per rung. A failure on
//! the provider's billing or usage window ([`billing_refusal`]) stands only
//! within its run: a recharge fixes it at once, so the next run or resume
//! probes it again (bug-374384). A CLI or ACP harness brings its own tools and
//! is not probed. The plan runner then skips the rungs whose standing probe
//! failed ([`RoutingLadder::without_models`]).

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

    /// `model`'s verdict, when it still stands at `now_ms` in the run that
    /// started at `run_started_ms`: it is [fresh](Self::fresh), and a failure
    /// on its provider's billing or usage window was probed in this run
    /// (bug-374384).
    #[must_use]
    pub fn standing(&self, model: &str, now_ms: i64, run_started_ms: i64) -> Option<&RungProbe> {
        self.fresh(model, now_ms)
            .filter(|probe| !probe.billing() || probe.probed_at_ms >= run_started_ms)
    }
}

impl RungProbe {
    /// Whether the probe failed on its provider's billing or usage window
    /// ([`billing_refusal`]).
    #[must_use]
    pub fn billing(&self) -> bool {
        !self.passed && billing_refusal(&self.reason)
    }
}

/// Whether a probe failed on its provider's billing or usage window: an HTTP
/// 402 or another billing refusal (`insufficient_credits`, z.ai's 1113, an
/// insufficient balance) or an exhausted plan (`provider_exhausted`). A
/// recharge or the window's reset fixes it at once, so its verdict stands only
/// within the run that probed it (gap-c96097, bug-374384).
#[must_use]
pub fn billing_refusal(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.starts_with("insufficient_credits")
        || reason.starts_with("provider_exhausted")
        || roko_agent::provider::error_classify::is_billing_message(&reason)
}

/// Probe each runnable rung model of `ladder` that roko's tool loop drives
/// and whose verdict no longer stands in the run that started at
/// `run_started_ms` ([`RungProbes::standing`]), save the verdicts, and return
/// the models whose standing probe failed, with why. So a billing failure
/// from an earlier run is probed again once a run, and a rung whose provider
/// was recharged runs (bug-374384).
pub async fn probe_ladder(
    config: &RokoConfig,
    ladder: &RoutingLadder,
    workdir: &Path,
    run_started_ms: i64,
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
        .filter(|model| probes.standing(model, now_ms, run_started_ms).is_none())
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
            let probe = probes.standing(&model, now_ms, run_started_ms)?;
            (!probe.passed).then(|| (model, probe.reason.clone()))
        })
        .collect()
}

/// The `models` whose probe passed in the run that started at
/// `run_started_ms`, by the verdicts saved at `workdir`: each shows its
/// provider can pay, as a recharged provider's probe does (bug-e33b7e).
#[must_use]
pub fn passed_in_run(models: &[String], workdir: &Path, run_started_ms: i64) -> Vec<String> {
    let probes = RungProbes::load(&RungProbes::path(workdir));
    models
        .iter()
        .filter(|model| {
            probes
                .models
                .get(model.as_str())
                .is_some_and(|probe| probe.passed && probe.probed_at_ms >= run_started_ms)
        })
        .cloned()
        .collect()
}

/// A rung the run skips because its model's fresh probe failed: what its
/// `rung_skipped` event and the run's readout say (gap-20301d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkippedRung {
    /// The rung's name.
    pub rung: String,
    /// The model dispatch runs for it.
    pub model: String,
    /// The model's provider.
    pub provider: String,
    /// Why its probe failed, in the probe's own words.
    pub reason: String,
    /// When its verdict stops standing and the rung is probed again (unix
    /// ms); `None` for a billing failure, which the next run probes again,
    /// and when the probe cache does not say when it was probed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_ms: Option<i64>,
    /// The probe failed on the provider's billing or usage window
    /// ([`billing_refusal`]): the next run probes the rung again (bug-374384).
    pub billing: bool,
}

impl SkippedRung {
    /// The rung in words, for a refusal that lists the skipped rungs:
    /// `cheap (gpt-oss-120b on cerebras): <reason>`.
    #[must_use]
    pub fn named(&self) -> String {
        format!(
            "{} ({} on {}): {}",
            self.rung, self.model, self.provider, self.reason
        )
    }

    /// The rung's line in a run's readout, beside the rungs that ran: "cheap
    /// (gpt-oss-120b on cerebras): skipped until 20:33, its tool-use probe
    /// failed: …; to probe it again now, …".
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{} ({} on {}): skipped{}, its tool-use probe failed: {}; {}",
            self.rung,
            self.model,
            self.provider,
            self.until(),
            self.reason,
            self.way_out()
        )
    }

    /// The finding in plain words: "rung cheap (gpt-oss-120b on cerebras) is
    /// skipped until 20:33: its tool-use probe failed: …; a task that would
    /// start on it starts on the next rung that runs; to probe it again now,
    /// …".
    #[must_use]
    pub fn detail(&self) -> String {
        let until = self.until();
        format!(
            "rung {} ({} on {}) is skipped{until}: its tool-use probe failed: {}; a task that \
             would start on it starts on the next rung that runs; {}",
            self.rung,
            self.model,
            self.provider,
            self.reason,
            self.way_out()
        )
    }

    /// ` until 20:33`; ` for this run` for a billing failure, which the next
    /// run probes again; nothing when the probe's time is unknown.
    fn until(&self) -> String {
        if self.billing {
            return " for this run".to_string();
        }
        self.until_ms
            .map(|until_ms| format!(" until {}", local_until(until_ms)))
            .unwrap_or_default()
    }

    /// How to have the rung probed again before its verdict lapses.
    fn way_out(&self) -> String {
        if self.billing {
            "the next run probes it again, so recharge the account and run again".to_string()
        } else {
            format!(
                "to probe it again now, remove `{}` from .roko/learn/{PROBES_FILE}",
                self.model
            )
        }
    }

    /// This rung's `rung_skipped` event for the run's plan `plan_id`.
    #[must_use]
    pub fn event(&self, plan_id: &str) -> roko_core::DashboardEvent {
        roko_core::DashboardEvent::RungSkipped {
            plan_id: plan_id.to_string(),
            rung: self.rung.clone(),
            model: self.model.clone(),
            provider: self.provider.clone(),
            reason: self.reason.clone(),
            until_ms: self.until_ms,
            billing: self.billing,
            detail: self.detail(),
        }
    }
}

/// The rungs of `ladder` that the failed probes `failed` skip, by model with
/// why, as [`probe_ladder`] returns them: one per rung whose model failed,
/// with the model's provider and until when the verdict saved at `workdir`
/// stands, which a billing failure does only for this run.
#[must_use]
pub fn skipped_rungs(
    config: &Arc<RokoConfig>,
    ladder: &RoutingLadder,
    failed: &BTreeMap<String, String>,
    workdir: &Path,
) -> Vec<SkippedRung> {
    if failed.is_empty() {
        return Vec::new();
    }
    let probes = RungProbes::load(&RungProbes::path(workdir));
    let resolver = crate::dispatch_v2::ProviderDispatchResolver::new(Arc::clone(config));
    let mut skipped = Vec::new();
    for (model, reason) in failed {
        let provider = resolver.resolve(model).provider_id;
        let billing = billing_refusal(reason);
        let until_ms = probes
            .models
            .get(model)
            .filter(|_| !billing)
            .map(|probe| probe.probed_at_ms.saturating_add(PROBE_TTL_MS));
        for rung in ladder.rungs_running(model) {
            skipped.push(SkippedRung {
                rung,
                model: model.clone(),
                provider: provider.clone(),
                reason: reason.clone(),
                until_ms,
                billing,
            });
        }
    }
    skipped
}

/// `until_ms` as local time: `20:33` today, else with its date.
fn local_until(until_ms: i64) -> String {
    use chrono::TimeZone;

    let Some(at) = chrono::Local.timestamp_millis_opt(until_ms).single() else {
        return until_ms.to_string();
    };
    if at.date_naive() == chrono::Local::now().date_naive() {
        at.format("%H:%M").to_string()
    } else {
        at.format("%Y-%m-%d %H:%M").to_string()
    }
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

/// Why a probe in which the model made no tool call failed. A request the
/// provider refused keeps the refusal in the provider's own words
/// (bug-35b190).
fn failure_reason(result: &AgentResult) -> String {
    let text = result.output.body.as_text().unwrap_or_default().trim();
    let class = crate::dispatch_v2::classify_provider_error(&text.to_ascii_lowercase());
    let said: String = text.chars().take(160).collect();
    if text.is_empty() || class == "empty_response" {
        "empty_response: a blank answer with no tool call".to_string()
    } else if result.success {
        format!("no tool call; the model answered: {said}")
    } else if let Some(refusal) = refusal_words(text) {
        format!("{class}: {refusal}")
    } else {
        format!("{class}: {said}")
    }
}

/// The refusal a provider's HTTP 4xx became in `text`, from the provider's
/// name on: `openai refused the request (HTTP 400): …`.
fn refusal_words(text: &str) -> Option<&str> {
    let at = text.find(roko_agent::provider::error_classify::REQUEST_REFUSED_MARKER)?;
    let start = text[..at]
        .trim_end()
        .rfind(' ')
        .map_or(0, |space| space + 1);
    Some(text[start..].trim())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// gap-20301d: a skipped rung's readout line and event detail say which
    /// rung, on which model and provider, until when, why, and how to probe
    /// it again now (bug-374384): a billing failure stands for this run only.
    #[test]
    fn a_skipped_rung_says_why_and_until_when() {
        let until_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
        let until = local_until(until_ms);
        let refused = SkippedRung {
            rung: "mid".to_string(),
            model: "gpt-5.4-mini".to_string(),
            provider: "openai".to_string(),
            reason: "request_refused: openai refused the request (HTTP 400)".to_string(),
            until_ms: Some(until_ms),
            billing: false,
        };
        assert_eq!(
            refused.line(),
            format!(
                "mid (gpt-5.4-mini on openai): skipped until {until}, its tool-use probe failed: \
                 request_refused: openai refused the request (HTTP 400); to probe it again now, \
                 remove `gpt-5.4-mini` from .roko/learn/rung-probes.json"
            )
        );
        assert_eq!(
            refused.detail(),
            format!(
                "rung mid (gpt-5.4-mini on openai) is skipped until {until}: its tool-use probe \
                 failed: request_refused: openai refused the request (HTTP 400); a task that \
                 would start on it starts on the next rung that runs; to probe it again now, \
                 remove `gpt-5.4-mini` from .roko/learn/rung-probes.json"
            )
        );
        let billed = SkippedRung {
            rung: "cheap".to_string(),
            model: "gpt-oss-120b".to_string(),
            provider: "cerebras".to_string(),
            reason: "insufficient_credits: cerebras refused the request (HTTP 402)".to_string(),
            until_ms: None,
            billing: true,
        };
        assert_eq!(
            billed.line(),
            "cheap (gpt-oss-120b on cerebras): skipped for this run, its tool-use probe failed: \
             insufficient_credits: cerebras refused the request (HTTP 402); the next run probes \
             it again, so recharge the account and run again"
        );
        let roko_core::DashboardEvent::RungSkipped {
            plan_id,
            until_ms,
            billing,
            detail,
            ..
        } = billed.event("p1")
        else {
            panic!("a rung_skipped event");
        };
        assert_eq!(
            (plan_id.as_str(), until_ms, billing, detail),
            ("p1", None, true, billed.detail())
        );
    }

    /// bug-e33b7e: the models whose probe passed in this run are named, as a
    /// recharged provider's is; an earlier run's pass, a failure and a model
    /// never probed are not.
    #[test]
    fn the_probes_passed_in_this_run_are_named() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = RungProbes::path(temp.path());
        std::fs::create_dir_all(path.parent().expect("the learn directory")).expect("mkdir");
        let probe = |passed: bool, probed_at_ms: i64| RungProbe {
            passed,
            reason: if passed {
                String::new()
            } else {
                "insufficient_credits: http 402".to_string()
            },
            probed_at_ms,
        };
        let probes = RungProbes {
            models: [
                ("again", probe(true, 200)),
                ("earlier", probe(true, 100)),
                ("refused", probe(false, 200)),
            ]
            .into_iter()
            .map(|(model, probe)| (model.to_string(), probe))
            .collect(),
        };
        std::fs::write(&path, serde_json::to_vec(&probes).expect("json")).expect("probes");
        let models = ["again", "earlier", "refused", "unprobed"].map(String::from);
        assert_eq!(passed_in_run(&models, temp.path(), 150), ["again"]);
    }

    /// bug-374384: a failure on the provider's billing or usage window stands
    /// only within the run that probed it; any other failure, and a pass,
    /// for a day.
    #[test]
    fn a_billing_failure_stands_only_in_its_run() {
        let probe = |passed: bool, reason: &str, probed_at_ms: i64| RungProbe {
            passed,
            reason: reason.to_string(),
            probed_at_ms,
        };
        let probes = RungProbes {
            models: [
                (
                    "billed-before",
                    probe(false, "unknown: z.ai: insufficient balance (1113)", 100),
                ),
                (
                    "billed-in-run",
                    probe(false, "insufficient_credits: http 402", 200),
                ),
                (
                    "blank-before",
                    probe(false, "empty_response: reasoning only", 100),
                ),
                ("passed-before", probe(true, "", 100)),
            ]
            .into_iter()
            .map(|(model, probe)| (model.to_string(), probe))
            .collect(),
        };
        let standing = |model: &str| probes.standing(model, 300, 150).is_some();
        assert!(!standing("billed-before"), "probed again in this run");
        assert!(standing("billed-in-run"));
        assert!(standing("blank-before"));
        assert!(standing("passed-before"));
    }
}
