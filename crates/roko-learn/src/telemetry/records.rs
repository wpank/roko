//! Canonical telemetry records for S01 Phase 0: the attempt key, the
//! attempt-open line, the settled verdict, the run manifest, the decision
//! and exposure rows, and the line envelope every run file shares.
//!
//! Records are append-only facts. Unknown values are `null`, never `0`, and
//! fields added after a schema's first version carry `#[serde(default)]` so
//! older rows keep parsing. Route decisions reuse
//! [`RoutingDecisionLog`](crate::routing_log::RoutingDecisionLog); content
//! decisions (knowledge, playbooks, sections, error patterns) are
//! [`ContentDecisionRecord`]s in the same file.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use roko_core::usage::UsageSource;
use serde::{Deserialize, Serialize};

use super::assign::{Arm, Assignment};
use crate::prompt_experiment::PromptAttemptKey;
use crate::routing_log::{DecisionState, RoutingDecisionLog};

// ── Schema names and run files ────────────────────────────────────────

/// `schema_version` of the attempt-open line (S01 §5.2).
pub const ATTEMPT_OPEN_SCHEMA: &str = "roko.attempt_open/1";
/// `schema_version` of the settled verdict (S01 §5.5).
pub const VERDICT_SCHEMA: &str = "roko.verdict/1";
/// `schema_version` of a decision row (S01 §5.3).
pub const DECISION_SCHEMA: &str = "roko.decision/1";
/// `schema_version` of an exposure row (S01 §5.4).
pub const EXPOSURE_SCHEMA: &str = "roko.exposure/1";
/// `schema_version` of `manifest.json` (S01 §5.1).
pub const RUN_MANIFEST_SCHEMA: &str = "roko.run_manifest/1";

/// `.roko/runs/<run_id>/manifest.json`, rewritten atomically at open, resume
/// and close.
pub const MANIFEST_FILE: &str = "manifest.json";
/// `.roko/runs/<run_id>/attempts.jsonl`: attempt-open lines and verdicts.
pub const ATTEMPTS_FILE: &str = "attempts.jsonl";
/// `.roko/runs/<run_id>/decisions.jsonl`: decision rows.
pub const DECISIONS_FILE: &str = "decisions.jsonl";
/// `.roko/runs/<run_id>/exposures.jsonl`: exposure rows.
pub const EXPOSURES_FILE: &str = "exposures.jsonl";

/// An append-only telemetry file inside a run directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunFile {
    /// [`ATTEMPTS_FILE`].
    Attempts,
    /// [`DECISIONS_FILE`].
    Decisions,
    /// [`EXPOSURES_FILE`].
    Exposures,
}

impl RunFile {
    /// Every append-only run file.
    pub const ALL: [Self; 3] = [Self::Attempts, Self::Decisions, Self::Exposures];

    /// File name inside the run directory.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Attempts => ATTEMPTS_FILE,
            Self::Decisions => DECISIONS_FILE,
            Self::Exposures => EXPOSURES_FILE,
        }
    }

    /// Path of this file inside `run_dir` (`RokoLayout::run_dir(run_id)`).
    #[must_use]
    pub fn path_in(self, run_dir: &Path) -> PathBuf {
        run_dir.join(self.file_name())
    }
}

// ── Digests ───────────────────────────────────────────────────────────

/// `"b3:"` plus the full hex BLAKE3 digest of `bytes`: the form of this
/// module's digests and ids (`task_spec_hash`, `record_id`). S05's audit
/// ledger (`roko.audit/1`) is the exception: its digests are `sha256:`.
#[must_use]
pub fn b3_digest(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

/// The id readers dedupe on (S01 §4.7):
/// `blake3(schema | attempt_key | decision_point | item | seq_scope)`.
/// `roko.audit/1` rows compute the same id with SHA-256; none are written
/// here.
#[must_use]
pub fn record_id(
    schema: &str,
    attempt_key: &str,
    decision_point: &str,
    item: &str,
    seq_scope: &str,
) -> String {
    let parts = [schema, attempt_key, decision_point, item, seq_scope];
    b3_digest(parts.join("|").as_bytes())
}

// ── Attempt identity ──────────────────────────────────────────────────

/// `"{run_id}:{plan_id}:{task_id}"`: the chain every attempt of one task in
/// one run shares, and the default randomization unit (S01 §4.2).
#[must_use]
pub fn chain_key(run_id: &str, plan_id: &str, task_id: &str) -> String {
    format!("{run_id}:{plan_id}:{task_id}")
}

/// Identity of one dispatch attempt (S01 §4.2).
///
/// `attempt` is 1-based and durable: it continues from the highest ordinal
/// already recorded for the chain in the run's `attempts.jsonl`
/// ([`AttemptOrdinals`](super::writer::AttemptOrdinals)), so a resumed run
/// never reuses a key. It renders (`Display`) as the attempt key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AttemptKey {
    /// Durable run id: the Graph checkpoint's run id, stable across resume.
    pub run_id: String,
    /// Plan containing the task; `"-"` for dispatch outside a plan.
    pub plan_id: String,
    /// Task within the plan; the request id for dispatch outside a plan.
    pub task_id: String,
    /// 1-based attempt ordinal within the chain.
    pub attempt: u32,
}

impl AttemptKey {
    /// Construct an attempt identity.
    pub fn new(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        attempt: u32,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            attempt,
        }
    }

    /// The chain this attempt belongs to (see [`chain_key`]).
    #[must_use]
    pub fn chain_key(&self) -> String {
        chain_key(&self.run_id, &self.plan_id, &self.task_id)
    }

    /// `"{chain_key}:{attempt}"`. This is the layout of roko-execution's
    /// `TaskAttemptReceiptV1.idempotency_key`, so the receipt settler and
    /// these records name an attempt the same way.
    #[must_use]
    pub fn attempt_key(&self) -> String {
        self.to_string()
    }

    /// The receipt settler's idempotency key for this attempt. roko-learn
    /// does not depend on roko-execution, so the conversion goes through the
    /// shared string layout.
    #[must_use]
    pub fn receipt_idempotency_key(&self) -> String {
        self.attempt_key()
    }

    /// `"{attempt_key}#settle"`: the one settlement of this attempt.
    #[must_use]
    pub fn settlement_id(&self) -> String {
        format!("{}#settle", self.attempt_key())
    }

    /// Parse an attempt key or a receipt idempotency key.
    ///
    /// Returns `None` unless the key has exactly four `:`-separated parts,
    /// the three ids are non-empty, and the ordinal is a positive integer
    /// (ordinals are 1-based).
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split(':').collect();
        let [run_id, plan_id, task_id, attempt] = parts.as_slice() else {
            return None;
        };
        if run_id.is_empty() || plan_id.is_empty() || task_id.is_empty() {
            return None;
        }
        let attempt: u32 = attempt.parse().ok()?;
        (attempt > 0).then(|| Self::new(*run_id, *plan_id, *task_id, attempt))
    }

    /// This key for the prompt-experiment store. The ordinal is carried as
    /// is, so it is 1-based there too.
    #[must_use]
    pub fn to_prompt_attempt_key(&self) -> PromptAttemptKey {
        PromptAttemptKey::new(
            self.run_id.clone(),
            self.plan_id.clone(),
            self.task_id.clone(),
            self.attempt,
        )
    }
}

/// Carries the ordinal unchanged. Graph dispatch mints prompt keys from the
/// attempt key; prompt keys minted before that counted from 0, so they do
/// not meet the 1-based rule.
impl From<PromptAttemptKey> for AttemptKey {
    fn from(key: PromptAttemptKey) -> Self {
        Self {
            run_id: key.run_id,
            plan_id: key.plan_id,
            task_id: key.task_id,
            attempt: key.attempt,
        }
    }
}

impl fmt::Display for AttemptKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.run_id, self.plan_id, self.task_id, self.attempt
        )
    }
}

/// The attempt identity every per-attempt record flattens (S01 §5), so
/// `jq '.attempt_key'` works on every run file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptIdentity {
    /// See [`AttemptKey::run_id`].
    pub run_id: String,
    /// See [`AttemptKey::plan_id`].
    pub plan_id: String,
    /// See [`AttemptKey::task_id`].
    pub task_id: String,
    /// Graph node id (`CellContext.cell_id`); `null` when unknown.
    #[serde(default)]
    pub node_id: Option<String>,
    /// See [`AttemptKey::attempt`].
    pub attempt: u32,
    /// Invocation ordinal from `manifest.invocations`. It is carried, but it
    /// is not part of the key.
    #[serde(default)]
    pub inv: Option<u32>,
    /// [`AttemptKey::attempt_key`].
    pub attempt_key: String,
    /// [`AttemptKey::chain_key`].
    pub chain_key: String,
}

impl AttemptIdentity {
    /// Identity of `key`, with no node id or invocation yet.
    #[must_use]
    pub fn new(key: &AttemptKey) -> Self {
        Self {
            run_id: key.run_id.clone(),
            plan_id: key.plan_id.clone(),
            task_id: key.task_id.clone(),
            node_id: None,
            attempt: key.attempt,
            inv: None,
            attempt_key: key.attempt_key(),
            chain_key: key.chain_key(),
        }
    }

    /// Set the Graph node id.
    #[must_use]
    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Set the invocation ordinal.
    #[must_use]
    pub fn with_inv(mut self, inv: u32) -> Self {
        self.inv = Some(inv);
        self
    }

    /// The key these fields were built from.
    #[must_use]
    pub fn key(&self) -> AttemptKey {
        AttemptKey::new(
            self.run_id.clone(),
            self.plan_id.clone(),
            self.task_id.clone(),
            self.attempt,
        )
    }
}

// ── Settlement vocabulary ─────────────────────────────────────────────

/// The Graph task gate tag (`TaskGateVerdict` in roko-graph's
/// `cells::task_executor`), mirrored by its wire values because roko-learn
/// does not depend on roko-graph. roko-core's `GateVerdict` is one gate's
/// full result, a different thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdictTag {
    /// Every authored verify step passed.
    Passed,
    /// Every authored verify step passed, or failed only on tests that also
    /// failed on the plan run's start commit (gap-161be1).
    PassedWithPreexistingFailures,
    /// Every authored verify step passed on a tree the attempt left
    /// unchanged: the task's work was already there.
    AlreadySatisfied,
    /// The task declares no verify steps.
    Unverified,
    /// A judge accepted a failed verification. Only legacy checkpoints
    /// carry it.
    ForcedAccept,
}

impl GateVerdictTag {
    /// Tag value, identical to `TaskGateVerdict::as_str`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::PassedWithPreexistingFailures => "passed_with_preexisting_failures",
            Self::AlreadySatisfied => "already_satisfied",
            Self::Unverified => "unverified",
            Self::ForcedAccept => "forced_accept",
        }
    }

    /// Parse a tag value; unknown values return `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "passed" => Some(Self::Passed),
            "passed_with_preexisting_failures" => Some(Self::PassedWithPreexistingFailures),
            "already_satisfied" => Some(Self::AlreadySatisfied),
            "unverified" => Some(Self::Unverified),
            "forced_accept" => Some(Self::ForcedAccept),
            _ => None,
        }
    }
}

/// How an attempt ended (S01 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    /// Every verify step passed.
    Passed,
    /// A verify rung failed.
    GateFailed,
    /// The attempt changed nothing, and every verify step passed on the tree
    /// it left: the task's work was done before it ran (gap-9eb1e1). Neither
    /// a learning success nor a failure.
    AlreadySatisfied,
    /// The task has no verify steps, so nothing checked the result.
    Unverified,
    /// A judge accepted a failed verification (legacy checkpoints only).
    ForcedAccept,
    /// The agent hit its turn cap.
    TurnCap,
    /// The task timed out.
    Timeout,
    /// The provider returned an error.
    ProviderError,
    /// Every provider in the failover chain failed.
    ProviderExhausted,
    /// The run cancelled the attempt.
    Cancelled,
    /// The plan budget ran out.
    BudgetExhausted,
    /// The worktree or its lease failed.
    WorkspaceError,
    /// The engine terminated the attempt's promise.
    PromiseTerminated,
    /// The harness failed the attempt outside its provider call and verify
    /// steps: prompt assembly, or recording its spend in the plan's cost
    /// ledger.
    HarnessError,
    /// An attempt-open line with no verdict. It is derived offline; dispatch
    /// never writes it.
    Abandoned,
}

impl AttemptOutcome {
    /// Who the outcome is charged to (S01 §4.3). `first_token_seen` only
    /// matters for timeouts: after the first token the agent was working, so
    /// the timeout is the agent's; before it, the provider never answered.
    #[must_use]
    pub const fn blame(self, first_token_seen: bool) -> Blame {
        match self {
            Self::Passed | Self::AlreadySatisfied | Self::Unverified | Self::ForcedAccept => {
                Blame::None
            }
            Self::GateFailed | Self::TurnCap => Blame::Agent,
            Self::Timeout if first_token_seen => Blame::Agent,
            Self::Timeout | Self::ProviderError | Self::ProviderExhausted => Blame::Infra,
            Self::Cancelled
            | Self::BudgetExhausted
            | Self::WorkspaceError
            | Self::PromiseTerminated
            | Self::HarnessError
            | Self::Abandoned => Blame::Harness,
        }
    }
}

impl From<GateVerdictTag> for AttemptOutcome {
    fn from(verdict: GateVerdictTag) -> Self {
        match verdict {
            // The agent's work passed: what failed, failed before it ran too.
            // The record's `gate_verdict` keeps the difference.
            GateVerdictTag::Passed | GateVerdictTag::PassedWithPreexistingFailures => Self::Passed,
            GateVerdictTag::AlreadySatisfied => Self::AlreadySatisfied,
            GateVerdictTag::Unverified => Self::Unverified,
            GateVerdictTag::ForcedAccept => Self::ForcedAccept,
        }
    }
}

/// Who an outcome is charged to (S01 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Blame {
    /// Nobody: the attempt passed, or nothing judged it.
    None,
    /// The agent's work failed.
    Agent,
    /// The provider or the network failed. It only affects provider health.
    Infra,
    /// The harness stopped the attempt (cancel, budget, workspace, or its
    /// own error).
    Harness,
}

/// The one field learners read (S01 §4.1): `1` for a pass, `0` when the
/// agent's work failed, and `None` when the attempt carries no learning
/// signal (unverified, forced accept, infra and harness outcomes).
#[must_use]
pub const fn learning_label_for(outcome: AttemptOutcome, blame: Blame) -> Option<u8> {
    match (outcome, blame) {
        (AttemptOutcome::Passed, _) => Some(1),
        (_, Blame::Agent) => Some(0),
        _ => None,
    }
}

/// Typed detail of an attempt that did not pass (`failure_class`, S01 §5.5).
/// roko-gate's `FailureClass` classifies compiler and test output instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptFailureClass {
    /// The outcome being classified.
    pub kind: AttemptOutcome,
    /// Failing verify rung, e.g. `verify:0/test`.
    #[serde(default)]
    pub rung: Option<String>,
    /// Provider error class from `error_classify`.
    #[serde(default)]
    pub provider_class: Option<String>,
    /// What a timeout measured.
    #[serde(default)]
    pub basis: Option<String>,
    /// `sha256` of the failure text. The text itself is never stored.
    #[serde(default)]
    pub detail_sha256: Option<String>,
}

impl AttemptFailureClass {
    /// A class of `kind` with no detail yet.
    #[must_use]
    pub const fn new(kind: AttemptOutcome) -> Self {
        Self {
            kind,
            rung: None,
            provider_class: None,
            basis: None,
            detail_sha256: None,
        }
    }
}

/// One verify step of a settled attempt (`steps[]`, S01 §5.5).
/// roko-gate's `StepVerdict` scores reasoning steps instead.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerifyStepVerdict {
    /// Rung name, e.g. `verify:0/test`.
    pub rung: String,
    /// `sha256` of the command. The command text is never stored.
    pub command_sha256: Option<String>,
    /// `None` when the step was skipped.
    pub passed: Option<bool>,
    /// Process exit code, when the step ran to exit.
    pub exit_code: Option<i32>,
    /// Wall time in milliseconds, when measured.
    pub duration_ms: Option<u64>,
    /// Whether the step hit its timeout.
    pub timed_out: bool,
    /// Whether the step was skipped.
    pub skipped: bool,
    /// Why the step was skipped, e.g. `fail_fast`.
    pub skip_reason: Option<String>,
    /// Whether the person the work is for confirmed the outcome: a passed
    /// `confirm` rung (9137). It is a person's judgement, kept apart from
    /// the machine checks, which routing and audits read as such.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub confirmed_by_user: bool,
}

/// Most scope findings a verdict lists; it counts the rest
/// ([`AttemptVerdictRecord::scope_findings_omitted`]).
pub const SCOPE_FINDINGS_LISTED: usize = 50;

/// A path a settled attempt changed outside its task's `files`, as the
/// pre-verify screen found it (`scope_findings[]`, backlog 1125). Under
/// `[gates] diff_scope = "record"` it is recorded and does not fail the
/// attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeFinding {
    /// The path, relative to the attempt's working tree.
    pub path: String,
    /// The finding's kind, e.g. `outside_scope`.
    pub kind: String,
}

/// Unix-millisecond timestamps of one attempt (S01 §4.4); `None` when
/// unknown, never `0`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptTiming {
    /// The attempt-open moment.
    pub attempt_started_at: Option<i64>,
    /// Prompt assembly finished.
    pub prompt_assembled_at: Option<i64>,
    /// The provider call started.
    pub dispatch_started_at: Option<i64>,
    /// The first streamed token arrived.
    pub first_token_at: Option<i64>,
    /// Where `first_token_at` came from: `stream`, or `unavailable` on the
    /// batch path.
    pub ttft_source: Option<String>,
    /// The provider call ended.
    pub dispatch_ended_at: Option<i64>,
    /// The first verify step started.
    pub verify_started_at: Option<i64>,
    /// The last verify step ended.
    pub verify_ended_at: Option<i64>,
    /// The verdict was settled.
    pub settled_at: Option<i64>,
}

/// The model that actually ran (`executed`, S01 §5.5). After failover it
/// differs from the routed one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutedModel {
    /// Provider that served the final turn.
    pub provider: Option<String>,
    /// Model the dispatcher asked for: routing's plan, before any failover.
    pub model_requested: Option<String>,
    /// Model the provider bridge launched: the requested one, or the
    /// failover candidate that replaced it. Legacy rows name it `model`.
    pub model_dispatched: Option<String>,
    /// Model the provider reported serving; `None` when its responses named
    /// none, never the configured slug.
    pub model_reported: Option<String>,
    /// Every model the provider named, in order, when its responses
    /// disagreed; `model_reported` is the last of them.
    pub models_reported: Vec<String>,
    /// The provider reported serving another model than the one launched
    /// (a dated snapshot of it, such as `gpt-4o-2024-08-06`, is the same).
    pub model_mismatch: bool,
    /// Models tried before the one that ran, in order.
    pub failover_chain: Vec<String>,
    /// Why the first model of `failover_chain`, the planned one, did not run.
    pub failover_reason: Option<String>,
    /// Each model of `failover_chain`, with why it was refused and whether
    /// a call reached its provider first (bug-220385).
    pub failover_refusals: Vec<FailoverRefusal>,
    /// Agent turns taken: the Claude CLI's `num_turns`, or the model calls
    /// of roko's tool loop. `None` when the agent did not report a count.
    pub turns: Option<u32>,
    /// The sampling parameters roko sent with each request, by their
    /// request-body names (`temperature`, `top_p`, `seed`). Empty, and left
    /// out of the record, when the provider's defaults applied (gap-13bbbd).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub sampling: BTreeMap<String, serde_json::Value>,
    /// The tool policy the attempt's contract asked for and what its provider
    /// enforced, for a provider that runs its own tools; left out otherwise
    /// (gap-baab0a).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_policy: Option<ToolPolicyRecord>,
}

/// The tool policy an attempt's contract asked for and what its provider
/// enforced (`executed.tool_policy`, gap-baab0a).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolPolicyRecord {
    /// Requested: the contract's allowlist; `None` when it has none.
    pub allowed_tools: Option<Vec<String>>,
    /// Requested: the contract's forbidden tools.
    pub forbidden_tools: Vec<String>,
    /// Effective: how the provider enforced the contract. `broker`: roko
    /// stops the provider at the first operation the contract denies.
    pub enforcement: String,
    /// Effective: the provider's operations the broker denies, by the names
    /// the provider reports them under.
    pub denied_operations: Vec<String>,
    /// Effective: the provider's web search and sandbox network were
    /// switched off because the contract keeps the role off the network.
    pub network_off: bool,
    /// The operation the broker stopped the attempt at, if it did.
    pub denial: Option<String>,
}

/// One model provider failover passed over before the one that ran
/// (`executed.failover_refusals`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FailoverRefusal {
    /// The refused `[models.*]` key.
    pub model: String,
    /// The refused model's provider.
    pub provider: String,
    /// Why, as a class: `provider_exhausted` (out of usage), `billing`,
    /// `circuit_open`, `disabled`, `no_credentials`, `not_configured`,
    /// `not_dispatchable`, `contract_unsupported` (the provider cannot
    /// enforce the task's agent contract) or `unguarded_in_checkout` (a
    /// Codex, Cursor or Gemini CLI agent, which roko cannot guard, for an
    /// attempt in the operator's shared checkout).
    pub class: String,
    /// The provider's own words, or why it could not be called.
    pub reason: String,
    /// Whether a call reached the provider before it refused; that call's
    /// cost and efficiency rows carry the role `failover_refused`.
    pub called: bool,
    /// Unix ms of the refusal.
    pub at: Option<i64>,
    /// When the provider is expected to take work again (unix ms).
    pub until: Option<i64>,
}

/// Helper model calls one attempt made outside its agent run: after a failed
/// gate, a quality judgement, an error diagnosis and a gate reflection on the
/// cheap helper model (`helpers`, bug-62e3f4). [`AttemptUsage`] and
/// [`AttemptCost`] cover the agent run alone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HelperCallsUsage {
    /// Completed helper calls.
    pub calls: u32,
    /// Input tokens, as the provider reported them.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Input read from the prompt cache.
    pub tokens_cache_read: u64,
    /// Priced cost of the calls, in USD.
    pub cost_usd: f64,
    /// Calls that used tokens but have no price, so `cost_usd` leaves them
    /// out.
    pub unpriced_calls: u32,
}

/// Token usage of one attempt in five disjoint classes (S01 §4.4): no token
/// is in two of them. A class is `None` when the backend did not report it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptUsage {
    /// Uncached input only. OpenAI-style counts include cached tokens, so the
    /// usage parser subtracts them first.
    pub tokens_in: Option<u64>,
    /// Output, reasoning included.
    pub tokens_out: Option<u64>,
    /// Input read from the prompt cache.
    pub tokens_cache_read: Option<u64>,
    /// Input written to the prompt cache with a 5-minute TTL.
    pub tokens_cache_write_5m: Option<u64>,
    /// Input written to the prompt cache with a 1-hour TTL.
    pub tokens_cache_write_1h: Option<u64>,
    /// Reasoning tokens. Not a sixth class: they are already inside
    /// `tokens_out` and are never priced again.
    pub tokens_reasoning: Option<u64>,
}

/// Where a backend's prompt-cache writes fall among the token classes of
/// [`AttemptUsage`] (S01 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheWriteClass {
    /// `tokens_cache_write_5m`: the Anthropic API's default TTL.
    FiveMinutes,
    /// `tokens_cache_write_1h`: the TTL of the Claude CLI's sessions.
    OneHour,
    /// Inside `tokens_in`, so both write classes are 0: OpenAI-style usage
    /// bills cache writes as input.
    InInput,
    /// Not known for the backend; both write classes stay `None`.
    Unknown,
}

impl AttemptUsage {
    /// The classes of a backend's usage `observation`, whose input count
    /// already leaves out cached tokens (bug-b72a37); its cache writes go
    /// where `writes` says. A class the backend did not report stays `None`.
    #[must_use]
    pub fn from_observation(
        observation: &roko_core::usage::UsageObservation,
        writes: CacheWriteClass,
    ) -> Self {
        let written = observation.cache_creation_tokens;
        let (tokens_cache_write_5m, tokens_cache_write_1h) = match writes {
            CacheWriteClass::FiveMinutes => (written, None),
            CacheWriteClass::OneHour => (None, written),
            CacheWriteClass::InInput => (Some(0), Some(0)),
            CacheWriteClass::Unknown => (None, None),
        };
        Self {
            tokens_in: observation.input_tokens,
            tokens_out: observation.output_tokens,
            tokens_cache_read: observation.cache_read_tokens,
            tokens_cache_write_5m,
            tokens_cache_write_1h,
            tokens_reasoning: observation.reasoning_tokens,
        }
    }
}

/// Where an attempt's priced token usage came from (`cost.source`,
/// S01 §4.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSource {
    /// Usage reported in an API provider's response.
    ProviderUsage,
    /// Usage reported by a CLI agent (Claude CLI `modelUsage`, Codex
    /// `turn.completed`).
    CliUsage,
    /// Tokens estimated locally.
    Estimated,
    /// A test provider.
    Mock,
    /// Unknown provenance.
    #[default]
    Unknown,
}

impl CostSource {
    /// Map a backend's [`UsageSource`]. Provider-reported usage is
    /// `cli_usage` for a CLI agent backend and `provider_usage` otherwise.
    #[must_use]
    pub const fn from_usage_source(source: &UsageSource, cli_backend: bool) -> Self {
        match source {
            UsageSource::ProviderReported if cli_backend => Self::CliUsage,
            UsageSource::ProviderReported => Self::ProviderUsage,
            UsageSource::Estimated => Self::Estimated,
            UsageSource::Unknown => Self::Unknown,
        }
    }

    /// Whether cost-per-verified-success queries (P1, M1) may use the row:
    /// only reported usage is priced.
    #[must_use]
    pub const fn is_reported_usage(self) -> bool {
        matches!(self, Self::ProviderUsage | Self::CliUsage)
    }
}

/// The single cost-field set (S01 §4.4). P1 and M1 read only
/// `api_equiv_usd`, from rows whose `source` is reported usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptCost {
    /// What the account pays: 0 on a subscription CLI, the provider's charge
    /// when the response reports one, otherwise `api_equiv_usd`.
    pub billed_usd: Option<f64>,
    /// Tokens times the rates of `price_snapshot_id`; `None` when usage is
    /// unknown.
    pub api_equiv_usd: Option<f64>,
    /// The same tokens with cache reads priced as input.
    pub without_cache_usd: Option<f64>,
    /// The vendor's own figure: a provider charge or the CLI's reported cost.
    pub vendor_usd: Option<f64>,
    /// Where the priced usage came from.
    pub source: CostSource,
    /// Price snapshot behind the API-equivalent figures, e.g.
    /// `prices-2026-09-28`.
    pub price_snapshot_id: Option<String>,
}

/// How many content items were retrieved and included for the attempt
/// (S01 P0-9): the knowledge entries, cited episodes, playbooks and error
/// patterns of its exposure rows, those past the per-attempt row cap too.
/// The prompt's own sections have exposure rows but are not counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExposureCounts {
    /// Items retrieved for the prompt.
    pub retrieved: u32,
    /// Items that survived into the rendered prompt.
    pub included: u32,
}

// ── Attempt records ───────────────────────────────────────────────────

/// `roko.attempt_open/1` (S01 §5.2), written before prompt assembly. An
/// open line with no verdict is an abandoned attempt: it is counted, and its
/// ordinal is never reused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptOpenRecord {
    /// The attempt this line opens.
    #[serde(flatten)]
    pub identity: AttemptIdentity,
    /// `b3:` digest of the task definition JSON.
    #[serde(default)]
    pub task_spec_hash: Option<String>,
    /// Agent role dispatched.
    #[serde(default)]
    pub role: Option<String>,
    /// Retry budget of the chain.
    #[serde(default)]
    pub max_retries: Option<u32>,
    /// Unix ms when the attempt started.
    #[serde(default)]
    pub attempt_started_at: Option<i64>,
    /// The task's tier label (`roko_core::task::TaskTier`), which
    /// [`crate::tier_limits`] groups attempts by. Older lines lack it.
    #[serde(default)]
    pub tier: Option<String>,
}

impl AttemptOpenRecord {
    /// An open line for `identity`, started at `attempt_started_at` (unix ms).
    #[must_use]
    pub fn new(identity: AttemptIdentity, attempt_started_at: i64) -> Self {
        Self {
            identity,
            task_spec_hash: None,
            role: None,
            max_retries: None,
            attempt_started_at: Some(attempt_started_at),
            tier: None,
        }
    }
}

/// Why an attempt ran on its model, with respect to the model ladder
/// (`[routing.ladder]`, gap-460230).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LadderReason {
    /// The start rung of the task's role and tier.
    Start,
    /// The rung the task's `rung` hint names.
    Hint,
    /// A rung above the start, after agent-blamed failures.
    Escalated,
    /// `--model` or a `model_hint` pinned the model; the ladder never moves
    /// it.
    Pinned,
    /// Failover ran the attempt on this rung's model, because the routed
    /// rung's provider could not take it: the same model elsewhere, or a rung
    /// above (backlog 1120).
    Failover,
}

/// Where an attempt stood on the model ladder (gap-460230).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptLadder {
    /// Name of the rung that supplied the model; `None` when pinned.
    #[serde(default)]
    pub rung: Option<String>,
    /// Index of that rung among the task's rungs, cheapest first.
    #[serde(default)]
    pub index: Option<u32>,
    /// Rungs the task had climbed above its start rung.
    pub step: u32,
    /// Why the attempt ran on its model.
    pub reason: LadderReason,
    /// The agent's work failed the task's last attempt on its top rung: the
    /// ladder is exhausted, and the task needs a split or a replan.
    #[serde(default)]
    pub exhausted: bool,
    /// The cascade router's own pick beside the rung, which did not run
    /// (its shadow pick, G56); `None` without a cascade router, and for a
    /// pinned model. The attempt's route row has it as `proposals.learned`;
    /// this copy keeps the verdict readable alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router_pick: Option<String>,
}

/// `roko.verdict/1` (S01 §5.5): the one settled record per attempt. Not
/// [`crate::verdict_scorer::VerdictRecord`], which is one gate's pass or fail
/// held in memory for routing penalties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptVerdictRecord {
    /// The attempt this verdict settles.
    #[serde(flatten)]
    pub identity: AttemptIdentity,
    /// `"{attempt_key}#settle"`; a second settlement is a counted no-op.
    pub settlement_id: String,
    /// `b3:` digest of the task definition JSON.
    #[serde(default)]
    pub task_spec_hash: Option<String>,
    /// The Graph gate tag on the output, when one was stamped.
    #[serde(default)]
    pub gate_verdict: Option<GateVerdictTag>,
    /// How the attempt ended.
    pub outcome: AttemptOutcome,
    /// Detail for outcomes other than `passed` and `unverified`.
    #[serde(default)]
    pub failure_class: Option<AttemptFailureClass>,
    /// Who the outcome is charged to.
    pub blame: Blame,
    /// `1`, `0` or `null`: the only field learners read (S01 §4.1).
    #[serde(default)]
    pub learning_label: Option<u8>,
    /// Per-rung verify results.
    #[serde(default)]
    pub steps: Vec<VerifyStepVerdict>,
    /// Paths the attempt changed outside its task's `files`, at most
    /// [`SCOPE_FINDINGS_LISTED`]; empty when it changed none or the
    /// pre-verify screen did not diff it (backlog 1125).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope_findings: Vec<ScopeFinding>,
    /// How many more scope findings there were than the verdict lists;
    /// `None` when it lists them all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_findings_omitted: Option<u32>,
    /// Timestamps.
    #[serde(default)]
    pub timing: AttemptTiming,
    /// The model that actually ran.
    #[serde(default)]
    pub executed: ExecutedModel,
    /// Token usage.
    #[serde(default)]
    pub usage: AttemptUsage,
    /// Cost, with its source.
    #[serde(default)]
    pub cost: AttemptCost,
    /// Helper model calls the attempt made outside its agent run; `None`
    /// when it made none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helpers: Option<HelperCallsUsage>,
    /// Where the attempt stood on the model ladder; `None` when the ladder
    /// is off or did not route the attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ladder: Option<AttemptLadder>,
    /// How the agent run was kept apart from the invoking user's own
    /// configuration, as the agent reported it: a Claude CLI run's
    /// isolation tags (`setting_sources`, `mcp_servers`, `auto_memory`,
    /// `config_dir`, `shell_snapshot`). Empty for an agent that reports
    /// none (gap-751ac9).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub isolation: BTreeMap<String, String>,
    /// `sha256` of the provider request.
    #[serde(default)]
    pub request_sha256: Option<String>,
    /// `sha256` of the agent output.
    #[serde(default)]
    pub output_sha256: Option<String>,
    /// `sha256` of the diff the attempt produced.
    #[serde(default)]
    pub diff_sha256: Option<String>,
    /// Content exposure counts.
    #[serde(default)]
    pub exposures: Option<ExposureCounts>,
}

impl AttemptVerdictRecord {
    /// Settle `identity` with `outcome`. Blame and the learning label follow
    /// the S01 §4.3 table; `first_token_seen` only matters for timeouts.
    #[must_use]
    pub fn settle(
        identity: AttemptIdentity,
        outcome: AttemptOutcome,
        first_token_seen: bool,
    ) -> Self {
        let settlement_id = format!("{}#settle", identity.attempt_key);
        let blame = outcome.blame(first_token_seen);
        Self {
            identity,
            settlement_id,
            task_spec_hash: None,
            gate_verdict: None,
            outcome,
            failure_class: None,
            blame,
            learning_label: learning_label_for(outcome, blame),
            steps: Vec::new(),
            scope_findings: Vec::new(),
            scope_findings_omitted: None,
            timing: AttemptTiming::default(),
            executed: ExecutedModel::default(),
            usage: AttemptUsage::default(),
            cost: AttemptCost::default(),
            helpers: None,
            ladder: None,
            isolation: BTreeMap::new(),
            request_sha256: None,
            output_sha256: None,
            diff_sha256: None,
            exposures: None,
        }
    }

    /// Settle from the Graph gate tag of a completed attempt.
    #[must_use]
    pub fn from_gate_verdict(identity: AttemptIdentity, verdict: GateVerdictTag) -> Self {
        let mut record = Self::settle(identity, verdict.into(), false);
        record.gate_verdict = Some(verdict);
        record
    }

    /// List `findings` as the attempt's scope findings: the first
    /// [`SCOPE_FINDINGS_LISTED`] of them, and a count of the rest.
    pub fn set_scope_findings(&mut self, mut findings: Vec<ScopeFinding>) {
        let omitted = findings.len().saturating_sub(SCOPE_FINDINGS_LISTED);
        findings.truncate(SCOPE_FINDINGS_LISTED);
        self.scope_findings = findings;
        self.scope_findings_omitted = u32::try_from(omitted).ok().filter(|&count| count > 0);
    }

    /// The learning label as learners apply it (S01 §4.1): `Some(true)` for
    /// a pass, `Some(false)` when the agent's work failed, and `None` when
    /// the attempt teaches nothing, so no learner updates.
    #[must_use]
    pub const fn learning_success(&self) -> Option<bool> {
        match self.learning_label {
            Some(1) => Some(true),
            Some(0) => Some(false),
            _ => None,
        }
    }
}

// ── Decisions ─────────────────────────────────────────────────────────

/// Who produced a decision's chosen value (`source`, S01 §5.3). A guard that
/// rewrites the router's pick is `fallback`, not `router`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    /// The learned router's own pick.
    Router,
    /// A task-level model hint.
    TaskHint,
    /// An operator or config override.
    Override,
    /// The task's `[routing.ladder]` rung. The router's own pick, if any, is
    /// then a shadow proposal (`proposals.learned`).
    Ladder,
    /// A guard replaced the router's pick (unconfigured model, disabled
    /// provider, no tool support).
    Fallback,
    /// The configured default, with no router involved.
    Default,
    /// An exploration draw (S02).
    Explore,
    /// The controller's choice (S06).
    Control,
    /// The self-model's choice (S04).
    SelfModel,
}

/// A content decision point (S01 §4.5): where a prompt chooses which
/// retrieved items it includes. Route decisions are `route`
/// ([`ROUTE_DECISION_POINT`](crate::routing_log::ROUTE_DECISION_POINT)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentDecisionPoint {
    /// Durable knowledge entries, and the prior episodes a prompt cites.
    Knowledge,
    /// Learned playbooks.
    Playbooks,
    /// The prompt's own sections.
    Sections,
    /// Known error patterns.
    ErrorPatterns,
    /// Post-gate reflections.
    Reflections,
    /// Dream routing advice.
    DreamAdvice,
}

impl ContentDecisionPoint {
    /// Wire value, e.g. `error_patterns`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Knowledge => "knowledge",
            Self::Playbooks => "playbooks",
            Self::Sections => "sections",
            Self::ErrorPatterns => "error_patterns",
            Self::Reflections => "reflections",
            Self::DreamAdvice => "dream_advice",
        }
    }
}

/// What a retrieved item is (`item_kind`, S01 §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposureItemKind {
    /// A durable knowledge entry.
    Knowledge,
    /// A prior episode the prompt cites, apart from knowledge
    /// (`PromptDiagnostics::episode_ids`).
    Episode,
    /// A learned playbook.
    Playbook,
    /// A prompt section the composer could include.
    Section,
    /// A known error pattern.
    ErrorPattern,
    /// A post-gate reflection.
    Reflection,
    /// Dream routing advice.
    DreamAdvice,
}

impl ExposureItemKind {
    /// Wire value, e.g. `error_pattern`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Knowledge => "knowledge",
            Self::Episode => "episode",
            Self::Playbook => "playbook",
            Self::Section => "section",
            Self::ErrorPattern => "error_pattern",
            Self::Reflection => "reflection",
            Self::DreamAdvice => "dream_advice",
        }
    }

    /// The decision point that chooses items of this kind. Knowledge and the
    /// episodes a prompt cites are one decision: both feed its knowledge.
    #[must_use]
    pub const fn decision_point(self) -> ContentDecisionPoint {
        match self {
            Self::Knowledge | Self::Episode => ContentDecisionPoint::Knowledge,
            Self::Playbook => ContentDecisionPoint::Playbooks,
            Self::Section => ContentDecisionPoint::Sections,
            Self::ErrorPattern => ContentDecisionPoint::ErrorPatterns,
            Self::Reflection => ContentDecisionPoint::Reflections,
            Self::DreamAdvice => ContentDecisionPoint::DreamAdvice,
        }
    }
}

/// Why a retrieved item did not reach the prompt (`excluded_reason`,
/// S01 §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExcludedReason {
    /// Its section was dropped, or cut short, to fit the token budget.
    TokenBudget,
    /// The attempt's arm withholds it (S02, S03).
    WithheldArm,
    /// A bandit left it out.
    BanditExcluded,
    /// A screen removed it.
    Screened,
    /// The role's prompt has no place for it.
    RoleFilter,
}

/// One candidate of a content decision (`candidates[]`, S01 §5.3): a
/// retrieved item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentCandidate {
    /// The item's id.
    pub id: String,
    /// 1-based position in the retrieval's ranking, when it ranks.
    #[serde(default)]
    pub rank: Option<u32>,
    /// The retrieval's score, when it scores.
    #[serde(default)]
    pub score: Option<f64>,
    /// Whether the policy could include it.
    #[serde(default = "eligible_by_default")]
    pub eligible: bool,
    /// Probability the logging policy included it.
    #[serde(default)]
    pub p: Option<f64>,
}

fn eligible_by_default() -> bool {
    true
}

/// `decision_point` of the placebo's decision rows (S03 §4.3).
pub const PLACEBO_DECISION_POINT: &str = "placebo";
/// The placebo loop's registry id.
pub const PLACEBO_LOOP_ID: &str = "L-placebo";
/// What both arms of the placebo propose: the same thing, nothing.
pub const PLACEBO_PROPOSAL: &str = "no_op";

/// What the placebo's two arms propose: the same, by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceboProposals {
    /// The learned arm's proposal.
    pub learned: String,
    /// The default arm's proposal.
    pub default: String,
}

/// `roko.decision/1` at the placebo decision point (S03 §4.3, S02 L12): the
/// L-placebo loop's arm for the attempt's chain. Its two arms are identical
/// and cost nothing, so its true effect is 0, and S03 calibrates its false
/// transitions on it. Nothing reads the arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceboDecisionRecord {
    /// The attempt the decision belongs to.
    #[serde(flatten)]
    pub identity: AttemptIdentity,
    /// Always [`PLACEBO_DECISION_POINT`].
    pub decision_point: String,
    /// Always [`PLACEBO_LOOP_ID`].
    pub loop_id: String,
    /// The chain's assignment on the placebo layer.
    pub assignment: Assignment,
    /// What each arm proposes.
    pub proposals: PlaceboProposals,
    /// The realised arm's proposal.
    pub chosen: String,
    /// The realised arm's propensity.
    pub chosen_propensity: f64,
}

impl PlaceboDecisionRecord {
    /// The placebo decision of the attempt `identity`, from its chain's
    /// `assignment` on the placebo layer.
    #[must_use]
    pub fn new(identity: AttemptIdentity, assignment: Assignment) -> Self {
        Self {
            identity,
            decision_point: PLACEBO_DECISION_POINT.to_string(),
            loop_id: PLACEBO_LOOP_ID.to_string(),
            chosen_propensity: assignment.propensity,
            assignment,
            proposals: PlaceboProposals {
                learned: PLACEBO_PROPOSAL.to_string(),
                default: PLACEBO_PROPOSAL.to_string(),
            },
            chosen: PLACEBO_PROPOSAL.to_string(),
        }
    }
}

/// `roko.decision/1` at a content decision point (S01 §4.5, §5.3): the items
/// an attempt's prompt retrieved there are the candidates, and the set it
/// included is the choice. It shares `decisions.jsonl` with the route rows,
/// which have no content `decision_point`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentDecisionRecord {
    /// The attempt that made the decision.
    #[serde(flatten)]
    pub identity: AttemptIdentity,
    /// Where the prompt chose.
    pub decision_point: ContentDecisionPoint,
    /// The ranking that produced the candidates, e.g. `keyword_overlap_top3`.
    pub policy: String,
    /// Every retrieved item, in rank order.
    pub candidates: Vec<ContentCandidate>,
    /// Ids of the candidates the prompt included: a set decision's `chosen`
    /// is an array.
    pub chosen: Vec<String>,
    /// Probability the logging policy gave `chosen`: 1 for a fixed ranking.
    #[serde(default)]
    pub chosen_propensity: Option<f64>,
    /// Who produced `chosen`.
    #[serde(default)]
    pub source: Option<DecisionSource>,
    /// The learned state the candidates came from (S01 P0-10), in the form
    /// route rows use; `None` when the decision point reads none.
    #[serde(default)]
    pub state: Option<DecisionState>,
    /// `b3:` digest of the gate thresholds in force
    /// (`learn/gate-thresholds.json`); `None` when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thresholds_digest: Option<String>,
    /// The arms of the attempt's chain (S02.P1-14), which every decision row
    /// of the attempt carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm_set: Option<crate::loop_audit::arm_set::ArmSet>,
}

/// `b3(attempt_key|item_kind|item_id)`: one item's exposure in one attempt
/// (`exposure_id`, S01 §5.4).
#[must_use]
pub fn exposure_id(attempt_key: &str, item_kind: ExposureItemKind, item_id: &str) -> String {
    let parts = [attempt_key, item_kind.as_str(), item_id];
    b3_digest(parts.join("|").as_bytes())
}

/// `roko.exposure/1` (S01 §5.4): one item an attempt's prompt retrieved,
/// and whether it reached the prompt. It holds digests and counts, never
/// the item's text (S01 §4.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExposureRecord {
    /// The attempt whose prompt retrieved the item.
    #[serde(flatten)]
    pub identity: AttemptIdentity,
    /// [`exposure_id`] of the item in this attempt.
    pub exposure_id: String,
    /// The decision point that chose whether to include the item.
    pub decision_point: ContentDecisionPoint,
    /// The loop that reads the item's outcome (S03), e.g. `L-know`.
    #[serde(default)]
    pub loop_id: Option<String>,
    /// What the item is.
    pub item_kind: ExposureItemKind,
    /// The item's id in its store.
    pub item_id: String,
    /// The item's version, e.g. `kn-2f81@v7`.
    #[serde(default)]
    pub item_version: Option<String>,
    /// How the item came to exist, e.g. `distilled`.
    #[serde(default)]
    pub origin: Option<String>,
    /// The retrieval path that found it, e.g. `neuro_lexical_cache`.
    #[serde(default)]
    pub source: Option<String>,
    /// 1-based position in the retrieval's ranking, when it ranks.
    #[serde(default)]
    pub rank: Option<u32>,
    /// The retrieval's score, when it scores.
    #[serde(default)]
    pub score: Option<f64>,
    /// The prompt's sources retrieved the item. Always true today.
    pub retrieved: bool,
    /// Its section reached the prompt, and so did its rendered text.
    pub included: bool,
    /// Why it did not, when it did not.
    #[serde(default)]
    pub excluded_reason: Option<ExcludedReason>,
    /// The prompt section that carries the item, e.g. `domain_context`.
    #[serde(default)]
    pub section_id: Option<String>,
    /// Estimated tokens of the item's rendered text.
    #[serde(default)]
    pub tokens: Option<u32>,
    /// `sha256` of the item's rendered text.
    #[serde(default)]
    pub rendered_sha256: Option<String>,
    /// The arm the attempt's assignment put this decision in (S02, S03).
    #[serde(default)]
    pub arm: Option<Arm>,
}

impl ExposureRecord {
    /// An item the prompt of `identity`'s attempt retrieved, not included
    /// until the caller says so, chosen at its kind's decision point.
    #[must_use]
    pub fn new(
        identity: AttemptIdentity,
        item_kind: ExposureItemKind,
        item_id: impl Into<String>,
    ) -> Self {
        let item_id = item_id.into();
        Self {
            exposure_id: exposure_id(&identity.attempt_key, item_kind, &item_id),
            identity,
            decision_point: item_kind.decision_point(),
            loop_id: None,
            item_kind,
            item_id,
            item_version: None,
            origin: None,
            source: None,
            rank: None,
            score: None,
            retrieved: true,
            included: false,
            excluded_reason: None,
            section_id: None,
            tokens: None,
            rendered_sha256: None,
            arm: None,
        }
    }
}

// ── Run manifest ──────────────────────────────────────────────────────

/// `roko.run_manifest/1` (S01 §5.1): what produced a run's records. It
/// lives in [`MANIFEST_FILE`] and is rewritten atomically at open, resume
/// and close. It is not roko-runtime's `RunManifest`, the run registry's
/// lifecycle record in `.roko/state/runs/<plan_id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunProvenanceManifest {
    /// Always [`RUN_MANIFEST_SCHEMA`].
    pub schema_version: String,
    /// Durable run id.
    pub run_id: String,
    /// Run kind, e.g. `plan_run`.
    pub kind: String,
    /// Plans the run executes.
    #[serde(default)]
    pub plan_ids: Vec<String>,
    /// One entry per process that worked on the run; a resume appends one.
    #[serde(default)]
    pub invocations: Vec<RunInvocation>,
    /// The harness build of the run's first invocation. Each invocation
    /// records its own ([`RunInvocation::harness`]).
    #[serde(default)]
    pub harness: HarnessProvenance,
    /// The configuration fingerprint of the run's first invocation. Each
    /// invocation records its own ([`RunInvocation::config`]).
    #[serde(default)]
    pub config: ConfigHashProvenance,
    /// Whether a later invocation ran under another harness build or
    /// config than the first, so the run's records come from more than one.
    #[serde(default)]
    pub mixed_provenance: bool,
    /// The price snapshot.
    #[serde(default)]
    pub prices: PriceProvenance,
    /// Experiment assignment inputs.
    #[serde(default)]
    pub experiment: ExperimentProvenance,
    /// The workspace the run started from.
    #[serde(default)]
    pub workspace: WorkspaceProvenance,
    /// Raises of a plan's budget ceiling during the run, oldest first
    /// (`roko plan budget raise`, backlog 2118).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub budget_raises: Vec<BudgetRaise>,
    /// Where the run's request came from, when a server started it: `http`,
    /// or `mcp:<client>` for a chat host (backlog 9116).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Set when the run closes.
    #[serde(default)]
    pub closed: Option<RunClosed>,
}

impl RunProvenanceManifest {
    /// An open manifest for `run_id`, with no invocations yet.
    pub fn new(run_id: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            schema_version: RUN_MANIFEST_SCHEMA.to_string(),
            run_id: run_id.into(),
            kind: kind.into(),
            plan_ids: Vec::new(),
            invocations: Vec::new(),
            harness: HarnessProvenance::default(),
            config: ConfigHashProvenance::default(),
            mixed_provenance: false,
            prices: PriceProvenance::default(),
            experiment: ExperimentProvenance::default(),
            workspace: WorkspaceProvenance::default(),
            budget_raises: Vec::new(),
            origin: None,
            closed: None,
        }
    }

    /// The 1-based ordinal of the next invocation.
    #[must_use]
    pub fn next_inv(&self) -> u32 {
        let last = self.invocations.iter().map(|i| i.inv).max();
        last.unwrap_or(0).saturating_add(1)
    }
}

/// One process that worked on a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunInvocation {
    /// 1-based invocation ordinal.
    pub inv: u32,
    /// ISO-8601 UTC start time.
    pub started_at: String,
    /// Whether this invocation resumed the run.
    pub resumed: bool,
    /// Process id.
    pub pid: u32,
    /// Host name, or `local`.
    pub host: String,
    /// `sha256` of the command-line arguments.
    pub args_sha256: Option<String>,
    /// The harness build this invocation ran; `None` in manifests written
    /// before invocations recorded their own.
    pub harness: Option<HarnessProvenance>,
    /// The configuration fingerprint this invocation ran with; `None` in
    /// manifests written before invocations recorded their own.
    pub config: Option<ConfigHashProvenance>,
}

/// The harness build behind a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HarnessProvenance {
    /// Git commit of the harness build.
    pub sha: String,
    /// Whether the working tree was dirty.
    pub dirty: bool,
    /// `b3:` digest of the uncommitted diff, when dirty.
    pub dirty_digest: Option<String>,
    /// rustc version.
    pub rustc: Option<String>,
    /// Build profile, e.g. `debug`.
    pub profile: Option<String>,
}

/// The configuration fingerprint a run used. roko-core's
/// `ConfigProvenance` traces where one config value came from instead.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigHashProvenance {
    /// `b3:` digest of the canonical, secret-redacted config (S01 §4.7).
    pub hash: String,
    /// `b3:` digest of the harness parameters in force.
    pub params_digest: Option<String>,
    /// How many secret values the fingerprint redacted.
    pub redacted_keys: u32,
}

/// The price snapshot behind a run's API-equivalent costs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PriceProvenance {
    /// Snapshot id, e.g. `prices-2026-09-28`.
    pub snapshot_id: Option<String>,
}

/// Experiment inputs of a run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentProvenance {
    /// Experiment id, when the run belongs to one.
    pub experiment_id: Option<String>,
    /// Condition of the experiment the run executes.
    pub condition_id: Option<String>,
    /// `run_seed` of the assignment function.
    pub seed: Option<u64>,
    /// Loops switched off for the run.
    pub ablation_flags: Vec<String>,
    /// Layer name to `salt_id` (`"{layer}@{epoch}"`).
    pub salt_ids: BTreeMap<String, String>,
    /// Disturbance injected for controller experiments (S06), if any.
    pub disturbance_spec: Option<serde_json::Value>,
}

/// The workspace a run started from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceProvenance {
    /// Commit the run's worktrees branch from.
    pub base_commit: Option<String>,
}

/// A raise of a plan's budget ceiling while its run ran (backlog 2118).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetRaise {
    /// ISO-8601 UTC time the run applied it.
    pub at: String,
    /// The plan whose ceiling was raised.
    pub plan_id: String,
    /// The ceiling before, in USD.
    pub from_usd: f64,
    /// The ceiling from then on, in USD.
    pub to_usd: f64,
    /// What the plan had spent when it was raised, in USD.
    pub spent_usd: f64,
    /// Who raised it: the control surface the raise came through.
    pub by: String,
}

/// Terminal counts of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunClosed {
    /// ISO-8601 UTC end time.
    pub ended_at: String,
    /// Terminal plan status, e.g. `succeeded`.
    pub status: String,
    /// Attempt-open lines written.
    pub attempts_opened: u64,
    /// Verdicts written.
    pub attempts_settled: u64,
    /// Attempts opened but never settled.
    pub abandoned: u64,
    /// Records the writer dropped because its channel was full.
    pub telemetry_dropped: u64,
    /// Chains that hit the runaway guard.
    pub runaway_guard_trips: u64,
}

// ── Line envelope ─────────────────────────────────────────────────────

/// One line of a run file: the writer's envelope, with the record's own
/// fields flattened beside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stamped<T> {
    /// `roko.<name>/<major>`. Rows without it predate S01, and estimators
    /// skip them.
    pub schema_version: String,
    /// The id readers dedupe on ([`record_id`]).
    pub record_id: String,
    /// Per-run sequence number. It only grows, and ordering checks use it
    /// rather than clocks.
    pub seq: u64,
    /// ISO-8601 UTC time the writer appended the line.
    pub ts: String,
    /// The record.
    #[serde(flatten)]
    pub record: T,
}

/// A pre-S01 learning-log row stamped with its attempt's key (S01 §5).
///
/// The legacy logs (`learn/efficiency.jsonl`, `learn/costs.jsonl`) gain
/// `attempt_key` this way. The row's own fields sit beside the key, so a
/// reader that parses the row type alone still reads the line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptKeyed<T> {
    /// [`AttemptKey::attempt_key`] of the attempt the row belongs to.
    pub attempt_key: String,
    /// The row.
    #[serde(flatten)]
    pub row: T,
}

/// A record type the writer appends to a run file.
pub trait TelemetryRecord: Serialize {
    /// `schema_version` of every line of this type.
    const SCHEMA: &'static str;
    /// The run file lines of this type go to.
    const FILE: RunFile;

    /// The id readers dedupe on. The writer skips a record whose id it has
    /// already written.
    fn record_id(&self) -> String;
}

impl TelemetryRecord for AttemptOpenRecord {
    const SCHEMA: &'static str = ATTEMPT_OPEN_SCHEMA;
    const FILE: RunFile = RunFile::Attempts;

    fn record_id(&self) -> String {
        record_id(Self::SCHEMA, &self.identity.attempt_key, "", "", "")
    }
}

impl TelemetryRecord for AttemptVerdictRecord {
    const SCHEMA: &'static str = VERDICT_SCHEMA;
    const FILE: RunFile = RunFile::Attempts;

    fn record_id(&self) -> String {
        record_id(Self::SCHEMA, &self.identity.attempt_key, "", "", "")
    }
}

/// Route decisions reuse [`RoutingDecisionLog`] (S01 §4.5; W3a rule 1). A
/// row without an attempt key is identified by its trace id and timestamp,
/// so such rows are never merged by accident.
impl TelemetryRecord for RoutingDecisionLog {
    const SCHEMA: &'static str = DECISION_SCHEMA;
    const FILE: RunFile = RunFile::Decisions;

    fn record_id(&self) -> String {
        match &self.attempt_key {
            Some(attempt_key) => record_id(Self::SCHEMA, attempt_key, "route", "", ""),
            None => record_id(Self::SCHEMA, &self.trace_id, "route", "", &self.timestamp),
        }
    }
}

/// One content decision per decision point of an attempt.
impl TelemetryRecord for ContentDecisionRecord {
    const SCHEMA: &'static str = DECISION_SCHEMA;
    const FILE: RunFile = RunFile::Decisions;

    fn record_id(&self) -> String {
        let key = &self.identity.attempt_key;
        record_id(Self::SCHEMA, key, self.decision_point.as_str(), "", "")
    }
}

/// One placebo decision per attempt.
impl TelemetryRecord for PlaceboDecisionRecord {
    const SCHEMA: &'static str = DECISION_SCHEMA;
    const FILE: RunFile = RunFile::Decisions;

    fn record_id(&self) -> String {
        let key = &self.identity.attempt_key;
        record_id(Self::SCHEMA, key, PLACEBO_DECISION_POINT, "", "")
    }
}

/// The item is its kind and id, so a knowledge entry and an episode that
/// share an id stay two exposures.
impl TelemetryRecord for ExposureRecord {
    const SCHEMA: &'static str = EXPOSURE_SCHEMA;
    const FILE: RunFile = RunFile::Exposures;

    fn record_id(&self) -> String {
        let key = &self.identity.attempt_key;
        let item = format!("{}:{}", self.item_kind.as_str(), self.item_id);
        record_id(Self::SCHEMA, key, self.decision_point.as_str(), &item, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str = "gr-7f3c2a91";
    const PLAN: &str = "loop-census";

    /// A route decision row as the cascade router wrote it before S01.
    const PRE_S01_DECISION: &str = r#"{
        "timestamp": "2026-04-12T08:30:00Z",
        "trace_id": "trace-123",
        "task_id": "task-2m13",
        "requested_model": "kimi-k2.5",
        "role": "implementer",
        "task_complexity": "architectural",
        "task_category": "implementation",
        "selected_provider": "zai",
        "selected_model": "glm-5.1",
        "routing_stage": "ucb",
        "routing_reason": "highest_ucb_score",
        "candidates": [
            {"model": "glm-5.1", "provider": "zai", "score": 0.91, "disqualified": null}
        ],
        "outcome_success": null,
        "outcome_cost_usd": null,
        "outcome_latency_ms": null
    }"#;

    /// The S01 §5.1 example, with its elisions filled in.
    const MANIFEST_EXAMPLE: &str = r#"{
        "schema_version": "roko.run_manifest/1",
        "run_id": "gr-7f3c2a91",
        "kind": "plan_run",
        "plan_ids": ["loop-census"],
        "invocations": [{
            "inv": 1, "started_at": "2026-10-02T14:03:11.402Z", "resumed": false,
            "pid": 41822, "host": "local", "args_sha256": "9f2c"
        }],
        "harness": {
            "sha": "725f21e05", "dirty": true, "dirty_digest": "b3:9ac1",
            "rustc": "1.96.1", "profile": "debug"
        },
        "config": {"hash": "b3:5d0e", "params_digest": "b3:11aa", "redacted_keys": 7},
        "prices": {"snapshot_id": "prices-2026-09-28"},
        "experiment": {
            "experiment_id": null, "condition_id": null, "seed": 1234, "ablation_flags": [],
            "salt_ids": {"global": "global@2026-10-02", "route": "route@2026-10-02"},
            "disturbance_spec": null
        },
        "workspace": {"base_commit": "725f21e05"},
        "closed": {
            "ended_at": "2026-10-02T14:09:40.118Z", "status": "succeeded",
            "attempts_opened": 4, "attempts_settled": 4, "abandoned": 0,
            "telemetry_dropped": 0, "runaway_guard_trips": 0
        }
    }"#;

    /// The S01 §5.4 example, with its elisions filled in.
    const EXPOSURE_EXAMPLE: &str = r#"{
        "schema_version": "roko.exposure/1", "record_id": "b3:5e1f", "exposure_id": "b3:c4d0",
        "seq": 21, "ts": "2026-10-02T14:03:11.530Z",
        "run_id": "gr-7f3c2a91", "plan_id": "loop-census", "task_id": "T2", "node_id": "task:T2",
        "attempt": 2, "inv": 1, "attempt_key": "gr-7f3c2a91:loop-census:T2:2",
        "chain_key": "gr-7f3c2a91:loop-census:T2",
        "decision_point": "knowledge", "loop_id": "L-know", "item_kind": "knowledge",
        "item_id": "kn-2f81", "item_version": "kn-2f81@v7", "origin": "distilled",
        "source": "neuro_lexical_cache", "rank": 1, "score": 0.62, "retrieved": true,
        "included": true, "excluded_reason": null, "section_id": "task_context.domain_notes",
        "tokens": 143, "rendered_sha256": "9f2c", "arm": "learned"
    }"#;

    fn identity(task: &str, attempt: u32) -> AttemptIdentity {
        AttemptIdentity::new(&AttemptKey::new(RUN, PLAN, task, attempt))
    }

    #[test]
    fn attempt_key_matches_receipt_idempotency_layout() {
        // The literal of roko-execution's `idempotency_key_format` test.
        let key = AttemptKey::new("run-1", "plan-a", "task-b", 2);
        assert_eq!(key.receipt_idempotency_key(), "run-1:plan-a:task-b:2");
        assert_eq!(key.attempt_key(), key.receipt_idempotency_key());
        assert_eq!(key.chain_key(), "run-1:plan-a:task-b");
        assert_eq!(key.settlement_id(), "run-1:plan-a:task-b:2#settle");
        assert_eq!(AttemptKey::parse("run-1:plan-a:task-b:2"), Some(key));
        for malformed in [
            "run-1:plan-a:task-b",
            "run-1:plan-a:task-b:0",
            "run-1:plan-a:task-b:x",
            "run-1::task-b:2",
            "a:b:c:d:1",
        ] {
            assert_eq!(AttemptKey::parse(malformed), None, "{malformed}");
        }
    }

    #[test]
    fn attempt_key_converts_to_and_from_prompt_attempt_key() {
        let key = AttemptKey::new(RUN, PLAN, "T2", 2);
        let prompt_key = key.to_prompt_attempt_key();
        assert_eq!(prompt_key, PromptAttemptKey::new(RUN, PLAN, "T2", 2));
        assert_eq!(AttemptKey::from(prompt_key), key);
        let json = serde_json::to_string(&key).expect("serialize key");
        let back: AttemptKey = serde_json::from_str(&json).expect("parse key");
        assert_eq!(back, key);

        let identity = AttemptIdentity::new(&key)
            .with_node_id("task:T2")
            .with_inv(1);
        assert_eq!(identity.key(), key);
        assert_eq!(identity.chain_key, "gr-7f3c2a91:loop-census:T2");
        assert_eq!(identity.node_id.as_deref(), Some("task:T2"));
    }

    #[test]
    fn attempt_open_record_round_trips_with_flat_key_fields() {
        let mut record = AttemptOpenRecord::new(identity("T2", 2).with_inv(1), 1_759_413_791_402);
        record.task_spec_hash = Some(b3_digest(b"{\"id\":\"T2\"}"));
        record.role = Some("implementer".to_string());
        record.max_retries = Some(2);

        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["attempt_key"], "gr-7f3c2a91:loop-census:T2:2");
        assert_eq!(json["chain_key"], "gr-7f3c2a91:loop-census:T2");
        assert_eq!(json["attempt"], 2);
        assert_eq!(json["inv"], 1);
        assert_eq!(json["node_id"], serde_json::Value::Null);
        let back: AttemptOpenRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, record);
    }

    #[test]
    fn verdict_record_round_trips_through_json() {
        let outcome = AttemptOutcome::GateFailed;
        let mut record = AttemptVerdictRecord::settle(identity("T2", 2), outcome, true);
        record.failure_class = Some(AttemptFailureClass {
            rung: Some("verify:0/test".to_string()),
            ..AttemptFailureClass::new(outcome)
        });
        record.steps = vec![VerifyStepVerdict {
            rung: "verify:0/test".to_string(),
            passed: Some(false),
            exit_code: Some(1),
            duration_ms: Some(812),
            ..VerifyStepVerdict::default()
        }];
        record.timing.first_token_at = Some(1_759_413_792_310);
        record.timing.ttft_source = Some("stream".to_string());
        record.executed.model_reported = Some("gpt-oss-120b".to_string());
        record.usage.tokens_in = Some(38_211);
        record.usage.tokens_cache_write_5m = Some(1_024);
        record.cost = AttemptCost {
            billed_usd: Some(0.0118),
            api_equiv_usd: Some(0.0118),
            source: CostSource::ProviderUsage,
            price_snapshot_id: Some("prices-2026-09-28".to_string()),
            ..AttemptCost::default()
        };
        record.exposures = Some(ExposureCounts {
            retrieved: 7,
            included: 5,
        });

        let json = serde_json::to_value(&record).expect("serialize");
        let settlement_id = "gr-7f3c2a91:loop-census:T2:2#settle";
        assert_eq!(json["settlement_id"], settlement_id);
        assert_eq!(json["outcome"], "gate_failed");
        assert_eq!(json["blame"], "agent");
        assert_eq!(json["learning_label"], 0);
        assert_eq!(json["failure_class"]["kind"], "gate_failed");
        assert_eq!(json["cost"]["source"], "provider_usage");
        assert_eq!(json["usage"]["tokens_cache_write_5m"], 1_024);
        assert!(json["usage"]["tokens_cache_write_1h"].is_null());
        assert_eq!(json["cost"]["vendor_usd"], serde_json::Value::Null);
        let back: AttemptVerdictRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, record);
    }

    /// backlog 1125: a verdict lists at most `SCOPE_FINDINGS_LISTED` scope
    /// findings and counts the rest; a verdict with none writes neither
    /// field, and a row from before the fields still parses.
    #[test]
    fn scope_findings_are_capped_and_old_rows_still_parse() {
        let mut record =
            AttemptVerdictRecord::settle(identity("T3", 1), AttemptOutcome::Passed, true);
        let findings = (0..SCOPE_FINDINGS_LISTED + 3)
            .map(|n| ScopeFinding {
                path: format!("src/f{n}.rs"),
                kind: "outside_scope".to_string(),
            })
            .collect();
        record.set_scope_findings(findings);
        assert_eq!(record.scope_findings.len(), SCOPE_FINDINGS_LISTED);
        assert_eq!(record.scope_findings_omitted, Some(3));
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["scope_findings"][0]["path"], "src/f0.rs");
        assert_eq!(json["scope_findings"][0]["kind"], "outside_scope");
        assert_eq!(json["scope_findings_omitted"], 3);
        let back: AttemptVerdictRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, record);

        let mut json = serde_json::to_value(&record).expect("serialize");
        let row = json.as_object_mut().expect("a JSON object");
        row.remove("scope_findings");
        row.remove("scope_findings_omitted");
        let old: AttemptVerdictRecord = serde_json::from_value(json).expect("an old row parses");
        assert!(old.scope_findings.is_empty());
        assert_eq!(old.scope_findings_omitted, None);

        record.set_scope_findings(Vec::new());
        let json = serde_json::to_value(&record).expect("serialize");
        assert!(json.get("scope_findings").is_none(), "{json}");
        assert!(json.get("scope_findings_omitted").is_none(), "{json}");
    }

    #[test]
    fn verdict_labels_follow_the_settlement_table() {
        use AttemptOutcome as O;
        let cases = [
            (O::Passed, false, Blame::None, Some(1)),
            (O::GateFailed, false, Blame::Agent, Some(0)),
            (O::AlreadySatisfied, false, Blame::None, None),
            (O::Unverified, false, Blame::None, None),
            (O::ForcedAccept, false, Blame::None, None),
            (O::TurnCap, true, Blame::Agent, Some(0)),
            (O::Timeout, true, Blame::Agent, Some(0)),
            (O::Timeout, false, Blame::Infra, None),
            (O::ProviderError, true, Blame::Infra, None),
            (O::ProviderExhausted, false, Blame::Infra, None),
            (O::Cancelled, true, Blame::Harness, None),
            (O::BudgetExhausted, true, Blame::Harness, None),
            (O::WorkspaceError, false, Blame::Harness, None),
            (O::PromiseTerminated, false, Blame::Harness, None),
            (O::HarnessError, true, Blame::Harness, None),
            (O::Abandoned, false, Blame::Harness, None),
        ];
        for (outcome, first_token_seen, blame, label) in cases {
            let id = identity("T1", 1);
            let record = AttemptVerdictRecord::settle(id, outcome, first_token_seen);
            let got = (record.blame, record.learning_label);
            assert_eq!(got, (blame, label), "{outcome:?}");
            let learned = label.map(|label| label == 1);
            assert_eq!(record.learning_success(), learned, "{outcome:?}");
        }

        let verdict = GateVerdictTag::Unverified;
        let unverified = AttemptVerdictRecord::from_gate_verdict(identity("T3", 1), verdict);
        assert_eq!(unverified.outcome, AttemptOutcome::Unverified);
        assert_eq!(unverified.gate_verdict, Some(verdict));
        assert_eq!(unverified.learning_label, None);
    }

    #[test]
    fn gate_verdict_wire_values_match_the_graph_tag() {
        for verdict in [
            GateVerdictTag::Passed,
            GateVerdictTag::PassedWithPreexistingFailures,
            GateVerdictTag::AlreadySatisfied,
            GateVerdictTag::Unverified,
            GateVerdictTag::ForcedAccept,
        ] {
            assert_eq!(GateVerdictTag::parse(verdict.as_str()), Some(verdict));
            let json = serde_json::to_value(verdict).expect("serialize");
            assert_eq!(json, verdict.as_str());
        }
        assert_eq!(GateVerdictTag::parse("failed"), None);
    }

    #[test]
    fn cost_source_maps_usage_source_by_backend_kind() {
        use CostSource as C;
        use UsageSource as U;
        let cases = [
            (U::ProviderReported, false, C::ProviderUsage),
            (U::ProviderReported, true, C::CliUsage),
            (U::Estimated, true, C::Estimated),
            (U::Unknown, false, C::Unknown),
        ];
        for (usage, cli_backend, expected) in cases {
            let source = CostSource::from_usage_source(&usage, cli_backend);
            assert_eq!(source, expected, "{usage:?}");
        }
        assert!(CostSource::CliUsage.is_reported_usage());
        assert!(!CostSource::Mock.is_reported_usage());
        assert_eq!(AttemptCost::default().source, CostSource::Unknown);
    }

    #[test]
    fn run_manifest_round_trips_and_reads_the_spec_example() {
        let manifest: RunProvenanceManifest =
            serde_json::from_str(MANIFEST_EXAMPLE).expect("example");
        assert_eq!(manifest.schema_version, RUN_MANIFEST_SCHEMA);
        assert_eq!(manifest.next_inv(), 2);
        assert_eq!(manifest.experiment.seed, Some(1234));
        assert_eq!(manifest.config.redacted_keys, 7);
        let closed = manifest.closed.as_ref().expect("closed section");
        assert_eq!(closed.attempts_opened, 4);

        let json = serde_json::to_string(&manifest).expect("serialize");
        let back: RunProvenanceManifest = serde_json::from_str(&json).expect("reparse");
        assert_eq!(back, manifest);

        let fresh = RunProvenanceManifest::new("gr-1", "plan_run");
        assert_eq!(fresh.schema_version, RUN_MANIFEST_SCHEMA);
        assert_eq!(fresh.next_inv(), 1);
        assert!(fresh.closed.is_none());
    }

    #[test]
    fn decision_rows_extend_routing_decision_log_and_old_rows_still_parse() {
        let old: RoutingDecisionLog = serde_json::from_str(PRE_S01_DECISION).expect("old row");
        assert_eq!(old.selected_model, "glm-5.1");
        assert_eq!(old.attempt_key, None);
        assert_eq!(old.source, None);
        assert_eq!(old.default_model, None);
        assert_eq!(old.propensity, None);

        let key = AttemptKey::new(RUN, PLAN, "T4", 1);
        let mut row = old.clone();
        row.attempt_key = Some(key.attempt_key());
        row.source = Some(DecisionSource::Fallback);
        row.default_model = Some("gpt-oss-120b".to_string());
        row.propensity = Some(1.0);
        let json = serde_json::to_value(&row).expect("serialize");
        assert_eq!(json["source"], "fallback");
        assert_eq!(json["attempt_key"], "gr-7f3c2a91:loop-census:T4:1");
        let back: RoutingDecisionLog = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, row);

        // Keyed rows are identified by attempt; unkeyed ones by trace and time.
        let expected = record_id(DECISION_SCHEMA, &key.attempt_key(), "route", "", "");
        assert_eq!(row.record_id(), expected);
        assert_ne!(old.record_id(), expected);
    }

    #[test]
    fn stamped_lines_flatten_the_envelope_beside_the_record() {
        let passed = AttemptOutcome::Passed;
        let record = AttemptVerdictRecord::settle(identity("T2", 2), passed, true);
        let line = Stamped {
            schema_version: VERDICT_SCHEMA.to_string(),
            record_id: record.record_id(),
            seq: 40,
            ts: "2026-10-02T14:03:21.950Z".to_string(),
            record,
        };
        let json = serde_json::to_value(&line).expect("serialize");
        assert_eq!(json["schema_version"], VERDICT_SCHEMA);
        assert_eq!(json["seq"], 40);
        assert_eq!(json["attempt_key"], "gr-7f3c2a91:loop-census:T2:2");
        assert_eq!(json["learning_label"], 1);
        let back: Stamped<AttemptVerdictRecord> =
            serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, line);
    }

    #[test]
    fn keyed_legacy_rows_still_parse_as_the_row_alone() {
        let cost = crate::costs_db::CostRecord {
            timestamp: "2026-10-02T14:03:21.950Z".to_string(),
            model: "gpt-oss-120b".to_string(),
            provider: "cerebras".to_string(),
            role: "implementer".to_string(),
            plan_id: PLAN.to_string(),
            task_id: "T2".to_string(),
            complexity_band: "focused".to_string(),
            input_tokens: 38_211,
            output_tokens: 2_904,
            cached_tokens: 0,
            cost_usd: 0.0156,
            duration_ms: 9_461,
            success: false,
            session_id: String::new(),
            cost_source: CostSource::CliUsage,
            priced: None,
        };
        let keyed = AttemptKeyed {
            attempt_key: AttemptKey::new(RUN, PLAN, "T2", 2).attempt_key(),
            row: cost.clone(),
        };
        let json = serde_json::to_value(&keyed).expect("serialize");
        assert_eq!(json["attempt_key"], "gr-7f3c2a91:loop-census:T2:2");
        assert_eq!(json["model"], "gpt-oss-120b");
        let row: crate::costs_db::CostRecord =
            serde_json::from_value(json.clone()).expect("parse the row alone");
        assert_eq!(row, cost);
        let back: AttemptKeyed<crate::costs_db::CostRecord> =
            serde_json::from_value(json).expect("parse the keyed row");
        assert_eq!(back, keyed);
    }

    #[test]
    fn record_ids_are_b3_digests_of_the_joined_parts() {
        assert_eq!(
            b3_digest(b""),
            "b3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        let passed = AttemptOutcome::Passed;
        let verdict = AttemptVerdictRecord::settle(identity("T2", 2), passed, false);
        // Pinned with the reference `blake3` Python package.
        assert_eq!(
            verdict.record_id(),
            "b3:f12170e5078eb3151c345c73238671ac15e730beef9711ae0889fb8a4d7bc01c"
        );
        let open = AttemptOpenRecord::new(identity("T2", 2), 0);
        assert_ne!(open.record_id(), verdict.record_id());
        assert_eq!(RunFile::Attempts.file_name(), ATTEMPTS_FILE);
    }

    #[test]
    fn exposure_record_round_trips_s01_example() {
        let line: Stamped<ExposureRecord> =
            serde_json::from_str(EXPOSURE_EXAMPLE).expect("example");
        assert_eq!(line.schema_version, EXPOSURE_SCHEMA);
        assert_eq!(line.seq, 21);
        let record = &line.record;
        assert_eq!(record.identity.key(), AttemptKey::new(RUN, PLAN, "T2", 2));
        assert_eq!(record.decision_point, ContentDecisionPoint::Knowledge);
        assert_eq!(record.item_kind, ExposureItemKind::Knowledge);
        assert_eq!(record.item_version.as_deref(), Some("kn-2f81@v7"));
        assert_eq!((record.rank, record.tokens), (Some(1), Some(143)));
        assert_eq!(record.score, Some(0.62));
        assert!(record.retrieved && record.included);
        assert_eq!(record.excluded_reason, None);
        assert_eq!(record.arm, Some(Arm::Learned));

        let json = serde_json::to_string(&line).expect("serialize");
        let back: Stamped<ExposureRecord> = serde_json::from_str(&json).expect("reparse");
        assert_eq!(back, line);
        assert_eq!(RunFile::Exposures.file_name(), EXPOSURES_FILE);
        assert!(RunFile::ALL.contains(&RunFile::Exposures));

        // A cited episode is a knowledge decision, and a dropped item names
        // why. Its ids follow S01 §4.7 and §5.4.
        let mut episode =
            ExposureRecord::new(identity("T2", 2), ExposureItemKind::Episode, "kn-2f81");
        episode.excluded_reason = Some(ExcludedReason::TokenBudget);
        let json = serde_json::to_value(&episode).expect("serialize");
        assert_eq!(json["decision_point"], "knowledge");
        assert_eq!(json["item_kind"], "episode");
        assert_eq!(json["included"], false);
        assert_eq!(json["excluded_reason"], "token_budget");
        assert_eq!(json["attempt_key"], "gr-7f3c2a91:loop-census:T2:2");
        let expected = b3_digest(b"gr-7f3c2a91:loop-census:T2:2|episode|kn-2f81");
        assert_eq!(episode.exposure_id, expected);
        let back: ExposureRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, episode);

        // A knowledge entry with the episode's id is another exposure.
        let entry = ExposureRecord::new(identity("T2", 2), ExposureItemKind::Knowledge, "kn-2f81");
        assert_ne!(entry.exposure_id, episode.exposure_id);
        assert_ne!(entry.record_id(), episode.record_id());
        for kind in [
            ExposureItemKind::Playbook,
            ExposureItemKind::Section,
            ExposureItemKind::ErrorPattern,
        ] {
            let json = serde_json::to_value(kind.decision_point()).expect("serialize");
            assert_eq!(json, kind.decision_point().as_str());
        }
    }

    #[test]
    fn content_decision_rows_choose_a_set_and_carry_the_route_state_slot() {
        let state = DecisionState {
            read: true,
            version: "kn:n=2".to_string(),
            digest: b3_digest(b"knowledge"),
            age_s: None,
            n_obs: 2,
        };
        let candidate = |id: &str, rank: u32, p: f64| ContentCandidate {
            id: id.to_string(),
            rank: Some(rank),
            score: Some(1.0),
            eligible: true,
            p: Some(p),
        };
        let record = ContentDecisionRecord {
            identity: identity("T2", 2),
            decision_point: ContentDecisionPoint::Knowledge,
            policy: "keyword_overlap_top3".to_string(),
            candidates: vec![candidate("kn-1", 1, 1.0), candidate("kn-2", 2, 0.0)],
            chosen: vec!["kn-1".to_string()],
            chosen_propensity: Some(1.0),
            source: Some(DecisionSource::Default),
            state: Some(state),
            thresholds_digest: None,
            arm_set: None,
        };
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["decision_point"], "knowledge");
        assert_eq!(json["chosen"], serde_json::json!(["kn-1"]));
        assert_eq!(json["source"], "default");
        assert_eq!(json["state"]["version"], "kn:n=2");
        assert!(json.get("thresholds_digest").is_none());
        let back: ContentDecisionRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, record);

        // One row per decision point of an attempt; route rows stay apart.
        let key = AttemptKey::new(RUN, PLAN, "T2", 2).attempt_key();
        let expected = record_id(DECISION_SCHEMA, &key, "knowledge", "", "");
        assert_eq!(record.record_id(), expected);
        assert_ne!(
            record.record_id(),
            record_id(DECISION_SCHEMA, &key, "route", "", "")
        );
    }
}
