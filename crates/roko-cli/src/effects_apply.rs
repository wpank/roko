//! Staged outbound effects, decided (9132): `roko effects` and serve's
//! effect routes list the tool calls a run held for approval
//! ([`EffectHold`], 9131), and approve or reject one.
//!
//! An approval replays the held call exactly once, on the authority of the
//! person's one-shot approval, through an [`EffectApplier`]: in production a
//! fresh [`ToolDispatcher`] over the workspace's MCP servers (`.mcp.json`).
//! Before the call an `applying` marker is written beside the hold, so a
//! crash between the marker and the result leaves the effect `ambiguous`:
//! the next approval records that and never runs the call again. When the
//! pack of the run's task has `receipt` rungs (9119), each runs after a call
//! that succeeded, with the effect's JSON in the file [`EFFECT_FILE_ENV`]
//! names, to check that the target system shows the effect.
//!
//! Every decision appends an [`EffectRecord`] to `.roko/state/effects.jsonl`,
//! and the hold is removed either way. The run has usually ended by then, so
//! the call's result goes to the record, not back to the agent. Two
//! approvals of one effect at the same moment are not told apart from a
//! crash: serve decides one effect at a time.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
use roko_agent::safety::authz::AuthorizationEvidence;
use roko_agent::safety::effects::EffectHold;
use roko_agent::{SafetyLayer, ToolPermissionPolicy};
use roko_core::config::schema::{GateRungConfig, RokoConfig, RungKind};
use roko_core::tool::{
    CorrelationEnvelope, NeverCancel, NoopAuditSink, NoopMetricsSink, NoopTraceSink, ToolCall,
    ToolContext, ToolError, ToolHandler, ToolRegistry, ToolResult, VecToolRegistry,
};
use roko_fs::RokoLayout;
use serde::{Deserialize, Serialize};

use crate::task_parser::TaskDef;

/// The variable that names the file holding an applied effect's JSON for
/// its receipt rungs: `{"effect": <hold>, "result": <text>}`.
pub const EFFECT_FILE_ENV: &str = "ROKO_EFFECT_FILE";

/// The most of a call's result or a receipt's output a record keeps, in
/// bytes, from the end.
const MAX_RECORDED_BYTES: usize = 4 * 1024;

/// How a decided effect ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOutcome {
    /// The call ran and succeeded.
    Applied,
    /// The call ran and failed.
    Failed,
    /// An earlier approval started the call and recorded no result: it may
    /// or may not have run, so it is not run again.
    Ambiguous,
    /// A person rejected the effect: the call never ran.
    Rejected,
}

impl EffectOutcome {
    /// The outcome's name: `applied`, `failed`, `ambiguous` or `rejected`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Failed => "failed",
            Self::Ambiguous => "ambiguous",
            Self::Rejected => "rejected",
        }
    }
}

/// A receipt rung's verdict on an applied effect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptVerdict {
    /// The rung's name.
    pub rung: String,
    /// Whether the target shows the effect: the command exited 0.
    pub passed: bool,
    /// The tail of the command's output, secrets scrubbed.
    pub output: String,
}

/// One decision on a staged effect: a line of `.roko/state/effects.jsonl`.
/// It names the call but never holds its arguments, which may carry
/// secrets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectRecord {
    /// The effect's id.
    pub effect_id: String,
    /// The run whose agent proposed the call.
    pub run_id: String,
    /// The plan of the task whose agent proposed the call.
    pub plan_id: String,
    /// The task whose agent proposed the call.
    pub task_id: String,
    /// The tool.
    pub tool: String,
    /// The MCP server of an MCP tool.
    pub server: Option<String>,
    /// How the effect ended.
    pub outcome: EffectOutcome,
    /// Who decided.
    pub decided_by: String,
    /// The decider's note.
    pub note: Option<String>,
    /// The authority an approved call ran on: the person's one-shot
    /// approval.
    pub authorization: Option<AuthorizationEvidence>,
    /// The tail of the call's result, or its error, secrets scrubbed.
    pub result: Option<String>,
    /// What the receipt rungs found after the call.
    pub receipts: Vec<ReceiptVerdict>,
    /// When the decision was made, in RFC 3339.
    pub decided_at: String,
}

/// A person's decision on a staged effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectDecision {
    /// Approve (apply) the effect, or reject it.
    pub approve: bool,
    /// Why, in the decider's words.
    pub note: Option<String>,
    /// Who decides.
    pub decided_by: String,
}

/// Why a decision could not be made.
#[derive(Debug, thiserror::Error)]
pub enum DecideError {
    /// No staged effect has the id.
    #[error("no staged effect {0}")]
    NotFound(String),
    /// The effect was decided already.
    #[error("effect {0} was decided already: {1}")]
    AlreadyDecided(String, &'static str),
    /// The hold or the record could not be read or written.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// What replays an approved effect's call.
#[async_trait::async_trait]
pub trait EffectApplier: Send + Sync {
    /// Run `hold`'s call once and return its result.
    async fn apply(&self, hold: &EffectHold) -> ToolResult;
}

/// `.roko/state/effects.jsonl` of the workspace at `workdir`.
#[must_use]
pub fn effect_records_path(workdir: &Path) -> PathBuf {
    RokoLayout::for_project(workdir)
        .state_dir()
        .join("effects.jsonl")
}

/// The effects waiting for a decision in the workspace at `workdir`, oldest
/// first, with their files. A hold that cannot be read is skipped.
#[must_use]
pub fn list_holds(workdir: &Path) -> Vec<(PathBuf, EffectHold)> {
    let root = RokoLayout::for_project(workdir).effect_holds_dir();
    let runs = std::fs::read_dir(root).into_iter().flatten().flatten();
    let mut holds: Vec<(PathBuf, EffectHold)> = runs
        .filter_map(|run| std::fs::read_dir(run.path()).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter_map(|path| {
            let hold = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
            Some((path, hold))
        })
        .collect();
    holds.sort_by(|(_, a), (_, b)| a.proposed_at.cmp(&b.proposed_at));
    holds
}

/// The decisions recorded in the workspace at `workdir`, oldest first. A
/// line that cannot be read is skipped.
#[must_use]
pub fn read_records(workdir: &Path) -> Vec<EffectRecord> {
    std::fs::read_to_string(effect_records_path(workdir))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The staged effects of the workspace at `workdir`, as `roko effects list
/// --json` and `GET /api/effects` show them (9133): those that wait, oldest
/// first and without their arguments, and the decisions, newest first; only
/// `run_id`'s when it is set.
#[must_use]
pub fn effects_report(workdir: &Path, run_id: Option<&str>) -> serde_json::Value {
    let of_run = |run: &str| run_id.is_none_or(|wanted| wanted == run);
    let waiting: Vec<serde_json::Value> = list_holds(workdir)
        .into_iter()
        .filter(|(_, hold)| of_run(&hold.run_id))
        .map(|(_, hold)| {
            serde_json::json!({
                "effect_id": hold.effect_id,
                "run_id": hold.run_id,
                "plan_id": hold.plan_id,
                "task_id": hold.task_id,
                "attempt": hold.attempt,
                "tool": hold.tool,
                "server": hold.server,
                "proposed_at": hold.proposed_at,
            })
        })
        .collect();
    let decided: Vec<EffectRecord> = read_records(workdir)
        .into_iter()
        .rev()
        .filter(|record| of_run(&record.run_id))
        .collect();
    serde_json::json!({ "waiting": waiting, "decided": decided })
}

/// Decide the staged effect `effect_id` in the workspace at `workdir`, as
/// `decision` says ([module docs](self)): an approval applies it once
/// through `applier` and runs the receipt rungs of its task's pack; a
/// rejection only records the decision. Either way the hold is removed and
/// the decision recorded, and the record is returned.
///
/// # Errors
///
/// [`DecideError::AlreadyDecided`] when the effect has a record already,
/// [`DecideError::NotFound`] when it has neither a record nor a hold, and
/// [`DecideError::Io`] when its files cannot be written.
pub async fn decide_effect(
    workdir: &Path,
    config: &RokoConfig,
    effect_id: &str,
    decision: EffectDecision,
    applier: &dyn EffectApplier,
) -> Result<EffectRecord, DecideError> {
    let decided = read_records(workdir)
        .into_iter()
        .find(|record| record.effect_id == effect_id);
    if let Some(record) = decided {
        let outcome = record.outcome.label();
        return Err(DecideError::AlreadyDecided(effect_id.to_string(), outcome));
    }
    let Some((path, hold)) = list_holds(workdir)
        .into_iter()
        .find(|(_, hold)| hold.effect_id == effect_id)
    else {
        return Err(DecideError::NotFound(effect_id.to_string()));
    };
    let mut record = EffectRecord {
        effect_id: hold.effect_id.clone(),
        run_id: hold.run_id.clone(),
        plan_id: hold.plan_id.clone(),
        task_id: hold.task_id.clone(),
        tool: hold.tool.clone(),
        server: hold.server.clone(),
        outcome: EffectOutcome::Rejected,
        decided_by: decision.decided_by.clone(),
        note: decision.note.clone(),
        authorization: None,
        result: None,
        receipts: Vec::new(),
        decided_at: chrono::Utc::now().to_rfc3339(),
    };
    let marker = path.with_extension("applying");
    if decision.approve && !claim(&marker)? {
        record.outcome = EffectOutcome::Ambiguous;
        record.result = Some(
            "an earlier approval started the call and recorded no result; it is not run again"
                .to_string(),
        );
    } else if decision.approve {
        let approval = format!("effect {effect_id} approved by {}", decision.decided_by);
        record.authorization = Some(AuthorizationEvidence::one_shot_approval(approval));
        let result = applier.apply(&hold).await;
        let text = match &result {
            ToolResult::Err(error) => error.to_string(),
            ok => ok.text_content(),
        };
        if result.is_ok() {
            record.outcome = EffectOutcome::Applied;
            record.receipts = check_receipts(workdir, config, &path, &hold, &text).await;
        } else {
            record.outcome = EffectOutcome::Failed;
        }
        record.result = Some(recorded_tail(&text));
    }
    append_record(workdir, &record)?;
    for file in [&path, &marker] {
        if let Err(error) = std::fs::remove_file(file)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(file = %file.display(), %error, "a decided effect's file stays");
        }
    }
    Ok(record)
}

/// Claim an effect for applying by creating `marker`: `false` when it exists
/// already, because an earlier approval claimed the effect.
fn claim(marker: &Path) -> std::io::Result<bool> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    match options.open(marker) {
        Ok(mut file) => {
            writeln!(file, "{}", chrono::Utc::now().to_rfc3339())?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

/// Append `record` to the workspace's `.roko/state/effects.jsonl`.
fn append_record(workdir: &Path, record: &EffectRecord) -> std::io::Result<()> {
    let path = effect_records_path(workdir);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let line = serde_json::to_string(record)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

/// `text` with secrets scrubbed, cut to its last [`MAX_RECORDED_BYTES`].
fn recorded_tail(text: &str) -> String {
    let scrubbed = roko_core::obs::scrub::scrub_secrets(text);
    let start = scrubbed.ceil_char_boundary(scrubbed.len().saturating_sub(MAX_RECORDED_BYTES));
    scrubbed[start..].to_string()
}

/// Run the receipt rungs that check `hold`'s effect, whose call gave
/// `result`: the `receipt` rungs of the pack its task follows. The effect's
/// JSON goes in a file beside the hold at `hold_path`, which the rungs read
/// through [`EFFECT_FILE_ENV`] and which is removed afterwards.
async fn check_receipts(
    workdir: &Path,
    config: &RokoConfig,
    hold_path: &Path,
    hold: &EffectHold,
    result: &str,
) -> Vec<ReceiptVerdict> {
    let rungs = receipt_rungs(workdir, config, hold);
    if rungs.is_empty() {
        return Vec::new();
    }
    let effect_file = hold_path.with_extension("receipt.json");
    let effect = serde_json::json!({ "effect": hold, "result": result });
    if let Err(error) = write_private(&effect_file, &effect) {
        let output = format!("the effect's file could not be written: {error}");
        let failed = |rung: &GateRungConfig| ReceiptVerdict {
            rung: rung.name.clone(),
            passed: false,
            output: output.clone(),
        };
        return rungs.iter().map(failed).collect();
    }
    let mut verdicts = Vec::new();
    for rung in &rungs {
        let passthrough = &config.gates.env_passthrough;
        verdicts.push(run_receipt(rung, workdir, &effect_file, passthrough).await);
    }
    let _ = std::fs::remove_file(&effect_file);
    verdicts
}

/// Write `value` as JSON to `path`, readable by its owner alone.
fn write_private(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)?
        .write_all(&serde_json::to_vec_pretty(value)?)
}

/// The `receipt` rungs that check `hold`'s effect: those of the pack the
/// work domain of its task follows, as a plan task's verify steps take it
/// (9121, 9125), that have a command.
fn receipt_rungs(workdir: &Path, config: &RokoConfig, hold: &EffectHold) -> Vec<GateRungConfig> {
    let default_domain = config.project.default_domain.as_ref();
    let domain = task_of(workdir, hold)
        .and_then(|task| task.effective_domain(default_domain))
        .or_else(|| default_domain.cloned());
    let named_pack = domain
        .as_ref()
        .and_then(|domain| config.domain_profile(domain)?.pack)
        .and_then(|name| config.gates.packs.get(&name));
    let rungs = named_pack.map_or_else(
        || config.gates.pack_for(domain.as_ref()),
        |pack| pack.rungs.as_slice(),
    );
    rungs
        .iter()
        .filter(|rung| rung.kind == RungKind::Receipt && !rung.command.trim().is_empty())
        .cloned()
        .collect()
}

/// The task whose agent proposed `hold`'s call: in the plan of its run's
/// directory, which `roko run` writes, or in the plans root.
fn task_of(workdir: &Path, hold: &EffectHold) -> Option<TaskDef> {
    let mut roots = vec![crate::plan::plans_dir(workdir)];
    if !hold.run_id.trim().is_empty() {
        roots.insert(0, RokoLayout::for_project(workdir).run_dir(&hold.run_id));
    }
    roots
        .iter()
        .filter_map(|root| crate::runner::plan_loader::load_plans(root).ok())
        .flatten()
        .find(|plan| plan.id == hold.plan_id)?
        .tasks
        .tasks
        .into_iter()
        .find(|task| task.id == hold.task_id)
}

/// Run receipt rung `rung` by `sh -c` in `workdir`, with `effect_file`'s path
/// in [`EFFECT_FILE_ENV`] and the gate environment (`passthrough` beyond its
/// allow-list).
async fn run_receipt(
    rung: &GateRungConfig,
    workdir: &Path,
    effect_file: &Path,
    passthrough: &[String],
) -> ReceiptVerdict {
    let mut command = std::process::Command::new("sh");
    command
        .arg("-c")
        .arg(&rung.command)
        .current_dir(workdir)
        .env(EFFECT_FILE_ENV, effect_file);
    roko_core::child_env::apply_gate_env(
        &mut command,
        roko_core::child_env::process_env(),
        passthrough,
    );
    let mut command = tokio::process::Command::from(command);
    command.stdin(Stdio::null()).kill_on_drop(true);
    let (passed, output) = match tokio::time::timeout(rung.timeout(), command.output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            (output.status.success(), format!("{stdout}{stderr}"))
        }
        Ok(Err(error)) => (false, format!("cannot start `sh`: {error}")),
        Err(_) => (false, format!("ran past its {} s limit", rung.timeout_secs)),
    };
    ReceiptVerdict {
        rung: rung.name.clone(),
        passed,
        output: recorded_tail(&output),
    }
}

/// The [`EffectApplier`] of `roko effects` and serve: a fresh
/// [`ToolDispatcher`] over the MCP servers of the workspace's `.mcp.json`,
/// which grants the held call's tool and nothing else.
pub struct McpEffectApplier {
    workdir: PathBuf,
    config: RokoConfig,
}

impl McpEffectApplier {
    /// The applier for the workspace at `workdir`, under `config`.
    #[must_use]
    pub fn new(workdir: PathBuf, config: RokoConfig) -> Self {
        Self { workdir, config }
    }

    /// Replay `hold`'s call, or say why it cannot be replayed.
    async fn replay(&self, hold: &EffectHold) -> Result<ToolResult, String> {
        let (_, mcp) = roko_agent::mcp::workspace_mcp_config(&self.workdir)
            .ok_or("the workspace has no .mcp.json to replay the call through")?
            .map_err(|error| format!("the workspace's .mcp.json cannot be read: {error}"))?;
        let runtime = roko_agent::mcp::discover_mcp_runtime(&mcp)
            .await
            .map_err(|error| format!("the workspace's MCP servers did not start: {error}"))?;
        let tools = runtime.tools().to_vec();
        let def = tools
            .iter()
            .find(|tool| tool.name == hold.tool)
            .cloned()
            .ok_or_else(|| format!("no MCP server of the workspace offers `{}`", hold.tool))?;
        let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(tools));
        let none: Arc<dyn HandlerResolver> = Arc::new(|_: &str| None::<Arc<dyn ToolHandler>>);
        let safety = SafetyLayer::from_config(&self.config).with_tool_permission_policy(
            ToolPermissionPolicy::AllowExplicit,
            vec![hold.tool.clone()],
        );
        let dispatcher = ToolDispatcher::new(registry, runtime.resolver(none)).with_safety(safety);
        let ctx = ToolContext::new(
            &self.workdir,
            Duration::from_millis(def.timeout_ms),
            def.permission,
            Arc::new(NoopAuditSink),
            Arc::new(NoopTraceSink),
            Arc::new(NoopMetricsSink),
            Arc::new(NeverCancel),
        )
        .with_correlation(CorrelationEnvelope {
            run_id: hold.run_id.clone(),
            task_id: hold.task_id.clone(),
            ..CorrelationEnvelope::empty()
        });
        let call = ToolCall::new(
            hold.effect_id.clone(),
            hold.tool.clone(),
            hold.arguments.clone(),
        );
        Ok(dispatcher.dispatch(call, &ctx).await)
    }
}

#[async_trait::async_trait]
impl EffectApplier for McpEffectApplier {
    async fn apply(&self, hold: &EffectHold) -> ToolResult {
        self.replay(hold)
            .await
            .unwrap_or_else(|reason| ToolResult::err(ToolError::Other(reason)))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use roko_core::tool::{ToolCategory, ToolDef, ToolPermission, ToolSource};

    use super::*;

    /// An applier that counts its calls and answers `result`.
    struct CountingApplier {
        calls: AtomicUsize,
        result: &'static str,
    }

    #[async_trait::async_trait]
    impl EffectApplier for CountingApplier {
        async fn apply(&self, _hold: &EffectHold) -> ToolResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ToolResult::text(self.result)
        }
    }

    /// Stage a call of `mail.send` in run `run-1`'s task `T1`, as the
    /// dispatcher does under a `stage` policy, and return its id.
    fn stage(workdir: &Path) -> String {
        let mut send = ToolDef::new(
            "mail.send",
            "Send a message",
            ToolCategory::Mcp,
            ToolPermission::writes(),
        );
        send.source = ToolSource::Mcp {
            server: "mail".to_string(),
        };
        let call = ToolCall::new(
            "c-send",
            "mail.send",
            serde_json::json!({ "to": "ops@example.com" }),
        );
        let ctx = ToolContext::testing(workdir).with_correlation(CorrelationEnvelope {
            run_id: "run-1".to_string(),
            task_id: "T1".to_string(),
            attempt_id: "run-1:run-1:T1:1".to_string(),
            ..CorrelationEnvelope::empty()
        });
        let hold = EffectHold::propose(&send, &call, &ctx);
        hold.write(workdir).expect("the hold");
        hold.effect_id
    }

    fn decision(approve: bool) -> EffectDecision {
        EffectDecision {
            approve,
            note: None,
            decided_by: "operator".to_string(),
        }
    }

    /// 9132: approving a staged effect twice applies it once; a receipt rung
    /// of the task's pack checks it, and a failing receipt is recorded as
    /// such; rejecting an effect never applies it; every decision is a
    /// record and removes the hold.
    #[tokio::test]
    async fn approved_effect_is_applied_once_and_receipt_checked() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path();
        // `roko run`'s one-task plan for the run: an ops task, whose pack
        // has a receipt rung that looks for the delivery in the effect.
        let run_dir = RokoLayout::for_project(workdir).run_dir("run-1");
        std::fs::create_dir_all(&run_dir).expect("run directory");
        std::fs::write(
            run_dir.join("tasks.toml"),
            "[meta]\nplan = \"run-1\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Tell ops\"\n\
             role = \"researcher\"\ndomain = \"ops\"\n",
        )
        .expect("the run's plan");
        let config: RokoConfig = toml::from_str(
            r#"
[[gates.packs.ops.rungs]]
name = "delivered"
kind = "receipt"
command = 'grep -q delivered "$ROKO_EFFECT_FILE"'
"#,
        )
        .expect("config");

        let delivered = CountingApplier {
            calls: AtomicUsize::new(0),
            result: "delivered as msg-1",
        };
        let first = stage(workdir);
        let record = decide_effect(workdir, &config, &first, decision(true), &delivered)
            .await
            .expect("the approval");
        assert_eq!(record.outcome, EffectOutcome::Applied);
        assert_eq!(record.receipts.len(), 1, "{record:?}");
        assert!(record.receipts[0].passed, "{record:?}");
        assert!(record.authorization.is_some());
        let again = decide_effect(workdir, &config, &first, decision(true), &delivered).await;
        assert!(
            matches!(again, Err(DecideError::AlreadyDecided(_, "applied"))),
            "{again:?}"
        );
        assert_eq!(delivered.calls.load(Ordering::SeqCst), 1);

        let queued = CountingApplier {
            calls: AtomicUsize::new(0),
            result: "queued",
        };
        let second = stage(workdir);
        let record = decide_effect(workdir, &config, &second, decision(true), &queued)
            .await
            .expect("the approval");
        assert_eq!(record.outcome, EffectOutcome::Applied);
        assert!(!record.receipts[0].passed, "{record:?}");

        let third = stage(workdir);
        let record = decide_effect(workdir, &config, &third, decision(false), &queued)
            .await
            .expect("the rejection");
        assert_eq!(record.outcome, EffectOutcome::Rejected);
        assert_eq!(
            queued.calls.load(Ordering::SeqCst),
            1,
            "a rejection applied"
        );

        assert!(list_holds(workdir).is_empty(), "a decided hold stays");
        let outcomes: Vec<EffectOutcome> = read_records(workdir)
            .iter()
            .map(|record| record.outcome)
            .collect();
        assert_eq!(
            outcomes,
            [
                EffectOutcome::Applied,
                EffectOutcome::Applied,
                EffectOutcome::Rejected
            ]
        );
        let missing = decide_effect(workdir, &config, "effect-none", decision(true), &queued).await;
        assert!(
            matches!(missing, Err(DecideError::NotFound(_))),
            "{missing:?}"
        );
    }
}
