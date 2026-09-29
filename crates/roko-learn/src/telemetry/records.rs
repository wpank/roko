//! Canonical telemetry records for S01 Phase 0: the attempt key, the
//! attempt-open line, the settled verdict, the run manifest and the line
//! envelope every run file shares.
//!
//! Records are append-only facts. Unknown values are `null`, never `0`, and
//! fields added after a schema's first version carry `#[serde(default)]` so
//! older rows keep parsing. Route decisions reuse
//! [`RoutingDecisionLog`](crate::routing_log::RoutingDecisionLog) rather than
//! a third decision-record type.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use roko_core::usage::UsageSource;
use serde::{Deserialize, Serialize};

use crate::prompt_experiment::PromptAttemptKey;
use crate::routing_log::RoutingDecisionLog;

// ── Schema names and run files ────────────────────────────────────────

/// `schema_version` of the attempt-open line (S01 §5.2).
pub const ATTEMPT_OPEN_SCHEMA: &str = "roko.attempt_open/1";
/// `schema_version` of the settled verdict (S01 §5.5).
pub const VERDICT_SCHEMA: &str = "roko.verdict/1";
/// `schema_version` of a decision row (S01 §5.3).
pub const DECISION_SCHEMA: &str = "roko.decision/1";
/// `schema_version` of `manifest.json` (S01 §5.1).
pub const RUN_MANIFEST_SCHEMA: &str = "roko.run_manifest/1";

/// `.roko/runs/<run_id>/manifest.json`, rewritten atomically at open, resume
/// and close.
pub const MANIFEST_FILE: &str = "manifest.json";
/// `.roko/runs/<run_id>/attempts.jsonl`: attempt-open lines and verdicts.
pub const ATTEMPTS_FILE: &str = "attempts.jsonl";
/// `.roko/runs/<run_id>/decisions.jsonl`: decision rows.
pub const DECISIONS_FILE: &str = "decisions.jsonl";

/// An append-only telemetry file inside a run directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunFile {
    /// [`ATTEMPTS_FILE`].
    Attempts,
    /// [`DECISIONS_FILE`].
    Decisions,
}

impl RunFile {
    /// Every append-only run file.
    pub const ALL: [Self; 2] = [Self::Attempts, Self::Decisions];

    /// File name inside the run directory.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Attempts => ATTEMPTS_FILE,
            Self::Decisions => DECISIONS_FILE,
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

/// Carries the ordinal unchanged. Prompt keys minted before S01 count from
/// 0, so they do not meet the 1-based rule.
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
            Self::Unverified => "unverified",
            Self::ForcedAccept => "forced_accept",
        }
    }

    /// Parse a tag value; unknown values return `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "passed" => Some(Self::Passed),
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
            Self::Passed | Self::Unverified | Self::ForcedAccept => Blame::None,
            Self::GateFailed | Self::TurnCap => Blame::Agent,
            Self::Timeout if first_token_seen => Blame::Agent,
            Self::Timeout | Self::ProviderError | Self::ProviderExhausted => Blame::Infra,
            Self::Cancelled
            | Self::BudgetExhausted
            | Self::WorkspaceError
            | Self::PromiseTerminated
            | Self::Abandoned => Blame::Harness,
        }
    }
}

impl From<GateVerdictTag> for AttemptOutcome {
    fn from(verdict: GateVerdictTag) -> Self {
        match verdict {
            GateVerdictTag::Passed => Self::Passed,
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
    /// The harness stopped the attempt (cancel, budget, workspace).
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
    /// Model the dispatcher asked for.
    pub model_requested: Option<String>,
    /// Model the provider reported running.
    pub model_reported: Option<String>,
    /// Models tried before the one that ran, in order.
    pub failover_chain: Vec<String>,
    /// Agent turns taken.
    pub turns: Option<u32>,
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

/// How many content items were retrieved and included for the attempt.
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
        }
    }
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
            timing: AttemptTiming::default(),
            executed: ExecutedModel::default(),
            usage: AttemptUsage::default(),
            cost: AttemptCost::default(),
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
    /// The harness build.
    #[serde(default)]
    pub harness: HarnessProvenance,
    /// The configuration fingerprint.
    #[serde(default)]
    pub config: ConfigHashProvenance,
    /// The price snapshot.
    #[serde(default)]
    pub prices: PriceProvenance,
    /// Experiment assignment inputs.
    #[serde(default)]
    pub experiment: ExperimentProvenance,
    /// The workspace the run started from.
    #[serde(default)]
    pub workspace: WorkspaceProvenance,
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
            prices: PriceProvenance::default(),
            experiment: ExperimentProvenance::default(),
            workspace: WorkspaceProvenance::default(),
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

    #[test]
    fn verdict_labels_follow_the_settlement_table() {
        use AttemptOutcome as O;
        let cases = [
            (O::Passed, false, Blame::None, Some(1)),
            (O::GateFailed, false, Blame::Agent, Some(0)),
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
            (O::Abandoned, false, Blame::Harness, None),
        ];
        for (outcome, first_token_seen, blame, label) in cases {
            let id = identity("T1", 1);
            let record = AttemptVerdictRecord::settle(id, outcome, first_token_seen);
            let got = (record.blame, record.learning_label);
            assert_eq!(got, (blame, label), "{outcome:?}");
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
}
