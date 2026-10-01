//! Full TUI state container matching Mori's `RunState`.
//!
//! `TuiState` holds every piece of mutable state the interactive dashboard
//! needs: navigation, scroll positions, modal visibility, agent/plan data,
//! cost tracking, git state, and more.
//!
//! The module is split into focused sub-modules:
//!
//! - [`snapshot`] — Dashboard snapshot application (`update_from_snapshot`,
//!   `update_from_dashboard_snapshot`)
//! - [`learning`] — Learning file sync, efficiency rates, provider status
//! - [`signals`] — Unified log cache, signal/episode/log loading
//! - [`scroll`] — Scroll-position management and log-level filtering
//! - [`safety`] — Safety incident records
//! - [`tests`] — All test cases

// Sub-modules
mod learning;
mod scroll;
mod signals;
pub(crate) mod snapshot;

mod safety;
pub use safety::SafetyIncident;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use roko_core::OperatingFrequency;

use super::atmosphere::Atmosphere;
use super::dashboard::{
    AgentSummary, AlertSummary, CascadeRouterState, DashboardData, EfficiencySummary,
    ExperimentSummary, GateResultSummary, GateResultsPageData, KnowledgeBrowseEntry,
    PlanExecutionSnapshot, PlaybookSummary, SignalSummary, TaskSummary,
};
use super::input::{ConfirmAction, FocusZone, InputMode, LogFilterLevel};
use super::modals::ModalState;
use super::segment::CachedRender;
use super::tabs::Tab;
use super::widgets::stream_output::display_text;
use crate::config::Config;
use crate::plan::PlanSummary;

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Pending command approval from an agent.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    /// Agent that requested approval.
    pub agent_id: String,
    /// Human-readable description of what the agent wants to do.
    pub description: String,
    /// The raw command or tool call.
    pub command: String,
    /// Optional graph execution run ID (P1-40: SurfaceEvent command path).
    pub run_id: Option<String>,
    /// Optional approval identifier (P1-40: SurfaceEvent command path).
    pub approval_id: Option<String>,
}

/// Health classification for a configured LLM provider.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProviderHealth {
    /// Provider is responsive and within latency/error budgets.
    #[default]
    Healthy,
    /// Provider is responding but with elevated latency or error rate.
    Degraded,
    /// Provider is unresponsive or all recent requests failed.
    Failed,
    /// Provider is being throttled or has billing issues.
    Billing,
}

/// Credit/billing classification for a provider.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CreditStatus {
    /// Credits/quota are available.
    #[default]
    Available,
    /// Credits are running low.
    Low,
    /// Credits are exhausted.
    Empty,
    /// Credit status cannot be determined.
    Unknown,
    /// CLI-based provider with no billing concept.
    CliProvider,
}

/// Snapshot of a single LLM provider's operational status for the Providers tab.
#[derive(Clone, Debug)]
pub struct ProviderStatus {
    /// Display name (e.g. "anthropic-primary").
    pub name: String,
    /// Provider kind slug (e.g. "anthropic_api", "openai_compat", "claude_cli").
    pub kind: String,
    /// Current health classification.
    pub health: ProviderHealth,
    /// Current credit/billing status.
    pub credit_status: CreditStatus,
    /// Cumulative cost in USD across all requests.
    pub total_cost_usd: f64,
    /// Total number of API requests made.
    pub total_requests: u64,
    /// Total number of failed requests.
    pub total_failures: u64,
    /// Success rate (0.0 - 1.0).
    pub success_rate: f64,
    /// Average response latency in milliseconds.
    pub avg_latency_ms: u64,
    /// Models currently routed through this provider.
    pub active_models: Vec<String>,
    /// Rolling cost samples for sparkline rendering (last 60).
    pub cost_history: Vec<f64>,
    /// Rolling latency samples for sparkline rendering (last 60).
    pub latency_history: Vec<u64>,
    /// Most recent error message, if any.
    pub last_error: Option<String>,
    /// Circuit breaker state label ("closed", "open", "half_open").
    pub circuit_state: String,
    /// Seconds remaining in cooldown period, if any.
    pub cooldown_remaining_secs: Option<u64>,
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: String::new(),
            health: ProviderHealth::default(),
            credit_status: CreditStatus::default(),
            total_cost_usd: 0.0,
            total_requests: 0,
            total_failures: 0,
            success_rate: 1.0,
            avg_latency_ms: 0,
            active_models: Vec::new(),
            cost_history: Vec::new(),
            latency_history: Vec::new(),
            last_error: None,
            circuit_state: String::from("closed"),
            cooldown_remaining_secs: None,
        }
    }
}

/// Cached MCP configuration used by the dashboard's MCP panel.
///
/// Keeping this in `TuiState` ensures rendering remains a pure in-memory
/// operation; filesystem parsing happens on the normal refresh cadence.
#[derive(Debug, Clone, Default)]
pub struct McpConfigView {
    pub configured_path: Option<PathBuf>,
    pub resolved_path: Option<PathBuf>,
    pub config: Option<roko_agent::mcp::McpConfig>,
    pub error: Option<String>,
}

/// Canonical status for an agent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
    #[default]
    Idle,
    Done,
    Failed,
}

impl AgentStatus {
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

impl From<&str> for AgentStatus {
    fn from(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "running" | "active" | "executing" => Self::Active,
            "done" | "completed" | "passed" => Self::Done,
            "failed" | "error" => Self::Failed,
            _ => Self::Idle,
        }
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Canonical status for a task.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TaskStatus {
    #[default]
    Pending,
    Active,
    Done,
    Failed,
    /// Completed although its verification failed (a forced accept). Its own
    /// state: neither passed nor failed.
    AcceptedWithFailures,
    /// Its work was already there: the attempt changed nothing, and its
    /// verify steps passed on the tree as it was. Verified, but not a pass.
    AlreadySatisfied,
    /// Completed, but no verify step judged it. Its own state: not passed.
    Unverified,
    /// Never ran: a task it depends on failed, or its condition was not met.
    Skipped,
    Blocked,
}

impl TaskStatus {
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Whether the task finished without failing: passed, already satisfied,
    /// accepted with failures, unverified or skipped (plan progress counts
    /// them all, as the dashboard snapshot does). Only [`Self::Done`] is a
    /// pass.
    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(
            self,
            Self::Done
                | Self::AlreadySatisfied
                | Self::AcceptedWithFailures
                | Self::Unverified
                | Self::Skipped
        )
    }

    #[must_use]
    pub const fn is_accepted_with_failures(self) -> bool {
        matches!(self, Self::AcceptedWithFailures)
    }

    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::AcceptedWithFailures => "accepted with failures",
            Self::AlreadySatisfied => "already satisfied",
            Self::Unverified => "unverified",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }
}

impl From<&str> for TaskStatus {
    fn from(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "done" | "completed" | "complete" | "passed" => Self::Done,
            roko_core::dashboard_snapshot::TASK_OUTCOME_ALREADY_SATISFIED | "already satisfied" => {
                Self::AlreadySatisfied
            }
            roko_core::dashboard_snapshot::TASK_OUTCOME_UNVERIFIED => Self::Unverified,
            "skipped" | "condition-skipped" => Self::Skipped,
            "running"
            | "active"
            | "executing"
            | "in_progress"
            | "implementing"
            | "gating"
            | "verifying"
            | "reviewing"
            | "review"
            | "doc revision"
            | "doc-revision"
            | "doc_revision"
            | "auto fixing"
            | "auto-fixing"
            | "auto_fixing"
            | "regenerating verify"
            | "regenerating-verify"
            | "regenerating_verify"
            | "preflight"
            | "strategist"
            | "implementer"
            | "compile-gate"
            | "compile_gate"
            | "test-gate"
            | "test_gate"
            | "critic-review"
            | "critic_review"
            | "verdict"
            | "committing"
            | "merge"
            | "merging"
            | "commit" => Self::Active,
            "failed" | "error" | "gate_rejected" | "gate-rejected" => Self::Failed,
            roko_core::dashboard_snapshot::TASK_OUTCOME_ACCEPTED_WITH_FAILURES
            | "accepted-with-failures"
            | "accepted with failures" => Self::AcceptedWithFailures,
            "blocked" => Self::Blocked,
            _ => Self::Pending,
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Canonical phase state for a plan or pipeline phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlanPhase {
    #[default]
    Pending,
    Active,
    Done,
    Failed,
}

impl PlanPhase {
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

impl From<&str> for PlanPhase {
    fn from(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "done" | "completed" | "complete" | "passed" | "skipped" => Self::Done,
            "failed" | "error" => Self::Failed,
            "pending" | "queued" | "" => Self::Pending,
            "running" | "active" | "executing" | "preflight" | "strategist" | "implementer"
            | "compile-gate" | "compile_gate" | "test-gate" | "test_gate" | "reviewing"
            | "critic-review" | "critic_review" | "verdict" | "committing" | "implementing"
            | "gating" | "verifying" | "review" | "merge" | "merging" | "commit" => Self::Active,
            _ => Self::Pending,
        }
    }
}

impl fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Fetch status for the agent-topology panel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AgentTopologyStatus {
    /// No fetch has been attempted yet in this session.
    #[default]
    Idle,
    /// A one-shot fetch is in flight.
    Loading,
    /// Topology data is available for rendering.
    Ready,
    /// The connected `roko serve` does not expose the endpoint.
    Unavailable,
    /// The fetch failed for some other reason.
    Error(String),
}

/// Agent row for the Vec-based agent roster used by widgets.
///
/// Widgets index into `TuiState::agents` by position, and read fields
/// like `.active`, `.role`, `.model`, `.current_plan`, `.current_task`,
/// `.context_limit`, `.output_lines`, `.last_output_line` etc.
#[derive(Debug, Clone, Default)]
pub struct AgentRow {
    /// Agent identifier.
    pub id: String,
    /// Whether the agent is currently active / running.
    pub active: bool,
    /// Canonical agent status.
    pub status: AgentStatus,
    /// Role label (e.g. "implementer", "strategist", "auditor").
    pub role: String,
    /// Model slug (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Cumulative input tokens.
    pub input_tokens: u64,
    /// Cumulative output tokens.
    pub output_tokens: u64,
    /// Context window limit in tokens.
    pub context_limit: u64,
    /// Plan this agent is working on.
    pub current_plan: String,
    /// Task this agent is working on.
    pub current_task: String,
    /// Current dispatch attempt number (1-based).
    pub attempt: u32,
    /// Timestamp (epoch ms) when agent was spawned.
    pub spawned_at_ms: u64,
    /// Timestamp (epoch ms) of the last event received from this agent.
    pub last_event_at_ms: u64,
    /// Accumulated output lines for the output pane.
    pub output_lines: Vec<String>,
    /// Last line of agent output (for the output pane).
    pub last_output_line: String,
}

const MAX_AGENT_STREAM_CHUNKS: usize = 200;
const MAX_AGENT_OUTPUT_LINES: usize = 50;

/// Maximum structured output records retained in memory per agent (#367).
pub const MAX_AGENT_OUTPUT_RECORDS: usize = 2_000;

/// Page size when loading older records from canonical history (#367).
pub const AGENT_OUTPUT_PAGE_SIZE: usize = 500;

// ---------------------------------------------------------------------------
// Structured agent output records (#367)
// ---------------------------------------------------------------------------

/// Kind tag for a structured agent output record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputRecordKind {
    /// Free-form text from the agent.
    Text,
    /// Internal reasoning / chain-of-thought.
    Reasoning,
    /// Tool invocation start.
    ToolCall,
    /// Result returned by a tool.
    ToolResult,
    /// An error message.
    Error,
    /// System-generated message (status updates, turn boundaries).
    System,
}

impl OutputRecordKind {
    /// Label string for display.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Reasoning => "reasoning",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::Error => "error",
            Self::System => "system",
        }
    }
}

/// One structured output record from an agent (#367).
///
/// Contains sequence, timestamp, role, kind, text payload, and optional
/// tool identification. The `redacted` flag indicates whether the canonical
/// redactor has already processed the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentOutputRecord {
    /// Monotonic sequence number (unique within an agent's history).
    pub seq: u64,
    /// Timestamp in epoch milliseconds.
    pub timestamp_ms: u64,
    /// Role that produced this record (e.g. "assistant", "tool", "system").
    pub role: String,
    /// Semantic kind of the record.
    pub kind: OutputRecordKind,
    /// The text payload (may be redacted).
    pub text: String,
    /// Whether the payload has been processed by the canonical redactor.
    pub redacted: bool,
    /// Optional tool identifier (for tool_call and tool_result kinds).
    pub tool_id: Option<String>,
    /// Optional tool name (for tool_call kind).
    pub tool_name: Option<String>,
}

/// Per-agent bounded output history with pagination support (#367).
///
/// Keeps the newest `MAX_AGENT_OUTPUT_RECORDS` records in memory per agent.
/// Provides `before()` for pagination and `search()` for regex matching.
/// Older records can be loaded from canonical runtime event history by the
/// caller; this type only manages the in-memory window.
#[derive(Debug, Clone, Default)]
pub struct AgentOutputHistory {
    /// Per-agent record deques keyed by agent identifier.
    records: HashMap<String, VecDeque<AgentOutputRecord>>,
    /// Oldest sequence number per agent (for pagination tracking).
    oldest_seq: HashMap<String, u64>,
    /// Next sequence number per agent (monotonic counter).
    next_seq: HashMap<String, u64>,
    /// Total records evicted across all agents.
    pub evicted: u64,
    /// Per-agent set of sequence numbers for live-unscreened non-tool records.
    ///
    /// These records are pending replacement by the settled screened
    /// transcript. They are removed (while preserving tool steps) when
    /// [`settle_screened_transcript`] is called.
    live_unscreened_seqs: HashMap<String, HashSet<u64>>,
    /// Per agent whose output arrives only through task-output rings: the
    /// ring last taken in, and the sequence number that followed it.
    ring_tails: HashMap<String, (Vec<String>, u64)>,
}

impl AgentOutputHistory {
    /// Push a new record for the given agent, assigning a sequence number.
    ///
    /// If the deque exceeds `MAX_AGENT_OUTPUT_RECORDS`, the oldest record(s)
    /// are evicted and `oldest_seq` is updated.
    ///
    /// # Tool-pair coherence (P1-TUI-G2)
    ///
    /// When a `ToolCall` record is evicted, its paired `ToolResult` (the next
    /// record in the deque that shares the same `tool_id`) is also evicted so
    /// the display never shows an orphaned result with no matching call.
    /// Similarly, if a `ToolResult` is evicted and its paired `ToolCall` (the
    /// immediately preceding record with the same `tool_id`) has already been
    /// evicted, the result is evicted cleanly. If the caller pushes a
    /// `ToolResult` whose matching `ToolCall` is still in the deque, the
    /// natural ordering ensures the call appears first; no special handling is
    /// needed for that direction.
    pub fn push(&mut self, agent_id: &str, mut record: AgentOutputRecord) {
        let seq = self.next_seq.entry(agent_id.to_string()).or_insert(1);
        record.seq = *seq;
        *seq += 1;

        let deque = self
            .records
            .entry(agent_id.to_string())
            .or_insert_with(VecDeque::new);

        deque.push_back(record);

        while deque.len() > MAX_AGENT_OUTPUT_RECORDS {
            let Some(evicted_record) = deque.pop_front() else {
                break;
            };
            let new_oldest = evicted_record.seq + 1;
            self.oldest_seq.insert(agent_id.to_string(), new_oldest);
            self.evicted += 1;
            // Only text and reasoning records are tracked as unscreened, and
            // they are never evicted as part of a tool pair below.
            if let Some(unscreened) = self.live_unscreened_seqs.get_mut(agent_id) {
                unscreened.remove(&evicted_record.seq);
            }

            // If we just evicted a ToolCall, also evict any immediately
            // following ToolResult with the same tool_id to keep pairs intact.
            if evicted_record.kind == OutputRecordKind::ToolCall {
                if let Some(evicted_id) = evicted_record.tool_id.as_deref() {
                    if deque.front().is_some_and(|r| {
                        r.kind == OutputRecordKind::ToolResult
                            && r.tool_id.as_deref() == Some(evicted_id)
                    }) {
                        if let Some(paired_result) = deque.pop_front() {
                            self.oldest_seq
                                .insert(agent_id.to_string(), paired_result.seq + 1);
                            self.evicted += 1;
                        }
                    }
                }
            }

            // If we just evicted a ToolResult whose paired ToolCall is already
            // gone (seq < oldest), nothing more to do. If the ToolCall is
            // still in the deque as the new front, it is now orphaned — evict
            // it too so the viewer never sees a ToolCall without its result.
            if evicted_record.kind == OutputRecordKind::ToolResult {
                if let Some(evicted_id) = evicted_record.tool_id.as_deref() {
                    if deque.front().is_some_and(|r| {
                        r.kind == OutputRecordKind::ToolCall
                            && r.tool_id.as_deref() == Some(evicted_id)
                    }) {
                        // This means a ToolCall follows its result — abnormal
                        // ordering; evict the orphaned call.
                        if let Some(orphaned_call) = deque.pop_front() {
                            self.oldest_seq
                                .insert(agent_id.to_string(), orphaned_call.seq + 1);
                            self.evicted += 1;
                        }
                    }
                }
            }
        }
    }

    /// Push a record for the given agent, deduplicating by sequence number.
    ///
    /// If a record with the same `seq` already exists, the push is skipped.
    /// This handles the live+settled duplicate scenario.
    pub fn push_dedup(&mut self, agent_id: &str, record: AgentOutputRecord) {
        let deque = self
            .records
            .entry(agent_id.to_string())
            .or_insert_with(VecDeque::new);

        // Check if this seq already exists (dedup live+settled copies).
        if deque.iter().any(|r| r.seq == record.seq) {
            return;
        }
        self.push(agent_id, record);
    }

    /// Return records for the agent, optionally before a given sequence.
    ///
    /// Returns up to `limit` records with sequence numbers strictly less
    /// than `before_seq`. If `before_seq` is `None`, returns the newest
    /// `limit` records (tail).
    #[must_use]
    pub fn before(
        &self,
        agent_id: &str,
        before_seq: Option<u64>,
        limit: usize,
    ) -> Vec<&AgentOutputRecord> {
        let deque = match self.records.get(agent_id) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let filtered: Vec<&AgentOutputRecord> = match before_seq {
            Some(seq) => deque.iter().filter(|r| r.seq < seq).collect(),
            None => deque.iter().collect(),
        };

        let start = filtered.len().saturating_sub(limit);
        filtered[start..].to_vec()
    }

    /// Search records for the given agent matching a regex pattern.
    ///
    /// Returns matching records (up to `limit`) with sequence numbers
    /// strictly less than `before_seq` (or all if `None`). Searches
    /// the `text`, `tool_name`, and `role` fields.
    #[must_use]
    pub fn search(
        &self,
        agent_id: &str,
        pattern: &regex::Regex,
        before_seq: Option<u64>,
        limit: usize,
    ) -> Vec<&AgentOutputRecord> {
        let deque = match self.records.get(agent_id) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let filtered: Vec<&AgentOutputRecord> = deque
            .iter()
            .filter(|r| {
                if let Some(seq) = before_seq {
                    if r.seq >= seq {
                        return false;
                    }
                }
                pattern.is_match(&display_text(&r.text))
                    || r.tool_name.as_deref().is_some_and(|n| pattern.is_match(n))
                    || pattern.is_match(&r.role)
            })
            .collect();

        let start = filtered.len().saturating_sub(limit);
        filtered[start..].to_vec()
    }

    /// Return all records for the given agent (in order).
    #[must_use]
    pub fn records_for(&self, agent_id: &str) -> &VecDeque<AgentOutputRecord> {
        static EMPTY: std::sync::LazyLock<VecDeque<AgentOutputRecord>> =
            std::sync::LazyLock::new(VecDeque::new);
        self.records.get(agent_id).unwrap_or(&EMPTY)
    }

    /// Total number of records currently held for the given agent.
    #[must_use]
    pub fn len(&self, agent_id: &str) -> usize {
        self.records.get(agent_id).map_or(0, VecDeque::len)
    }

    /// The oldest sequence number still in memory for the given agent.
    #[must_use]
    pub fn oldest_sequence(&self, agent_id: &str) -> u64 {
        self.oldest_seq.get(agent_id).copied().unwrap_or(1)
    }

    /// The next sequence number that will be assigned for the given agent.
    #[must_use]
    pub fn next_sequence(&self, agent_id: &str) -> u64 {
        self.next_seq.get(agent_id).copied().unwrap_or(1)
    }

    /// Clear all records for the given agent.
    pub fn clear_agent(&mut self, agent_id: &str) {
        self.records.remove(agent_id);
        self.oldest_seq.remove(agent_id);
        self.next_seq.remove(agent_id);
        self.ring_tails.remove(agent_id);
    }

    /// Take in the lines a task-output ring adds for an agent whose output
    /// arrives only through such rings (#367).
    ///
    /// A ring slides: each one holds the task's latest lines, so the new
    /// lines are those after its overlap with the ring taken in last. An
    /// agent whose history holds records from another path (`AgentOutput`
    /// events) is left to it, because the ring repeats what it delivers.
    pub fn ingest_ring(&mut self, agent_id: &str, ring: &[String], role: &str) {
        let next = self.next_sequence(agent_id);
        let new_from = match self.ring_tails.get(agent_id) {
            Some((tail, tail_next)) if *tail_next == next => ring_overlap(tail, ring),
            // Another path pushed records since the last ring: it owns them.
            Some(_) => {
                self.ring_tails.remove(agent_id);
                return;
            }
            None if self.len(agent_id) > 0 => return,
            None => 0,
        };
        self.ingest_lines(agent_id, &ring[new_from..], role);
        let next = self.next_sequence(agent_id);
        self.ring_tails
            .insert(agent_id.to_string(), (ring.to_vec(), next));
    }

    /// Convert raw output lines into records and populate the history for
    /// an agent. Used to backfill from legacy `AgentRow::output_lines` or
    /// `task_output_tails` during snapshot ingestion.
    ///
    /// Lines parsed as live-unscreened non-tool records are tracked in
    /// `live_unscreened_seqs` so they can be dropped when the screened
    /// transcript arrives via [`settle_screened_transcript`].
    pub fn ingest_lines(&mut self, agent_id: &str, lines: &[String], role: &str) {
        for line in lines {
            self.ingest_line(agent_id, line, role);
        }
    }

    /// Convert one raw output line into a record for an agent, as
    /// [`Self::ingest_lines`] does. A stream record keeps its encoded line as
    /// the record's text, which the renderer and search decode, so a line
    /// published live (`TuiState::ingest_agent_output`) and the same line
    /// backfilled from a snapshot give the same record.
    pub fn ingest_line(&mut self, agent_id: &str, line: &str, role: &str) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let (kind, tool_id, tool_name, is_live_unscreened) = classify_output_line(line);
        // Assign the sequence number before push so we can track it.
        let seq = *self.next_seq.entry(agent_id.to_string()).or_insert(1);
        self.push(
            agent_id,
            AgentOutputRecord {
                seq: 0, // overwritten by push()
                timestamp_ms: now_ms,
                role: role.to_string(),
                kind,
                text: line.to_string(),
                redacted: false,
                tool_id,
                tool_name,
            },
        );
        if is_live_unscreened {
            self.live_unscreened_seqs
                .entry(agent_id.to_string())
                .or_insert_with(HashSet::new)
                .insert(seq);
        }
    }

    /// Replace live-unscreened non-tool records for `agent_id` with the
    /// settled screened transcript.
    ///
    /// Called when the first non-`live` record arrives for an agent (or when
    /// `agent_completed` is signalled). Drops all previously tracked
    /// unscreened records from the deque while preserving every tool step.
    /// The new `settled_lines` are then ingested as normal screened records.
    pub fn settle_screened_transcript(
        &mut self,
        agent_id: &str,
        settled_lines: &[String],
        role: &str,
    ) {
        // Remove the unscreened non-tool records.
        if let Some(unscreened) = self.live_unscreened_seqs.remove(agent_id) {
            if let Some(deque) = self.records.get_mut(agent_id) {
                deque.retain(|r| !unscreened.contains(&r.seq));
            }
        }

        // Ingest the settled, screened lines.
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        for line in settled_lines {
            let (kind, tool_id, tool_name, _) = classify_output_line(line);
            self.push(
                agent_id,
                AgentOutputRecord {
                    seq: 0,
                    timestamp_ms: now_ms,
                    role: role.to_string(),
                    kind,
                    text: line.clone(),
                    redacted: false,
                    tool_id,
                    tool_name,
                },
            );
        }
    }
}

/// How many leading lines of `ring` repeat the end of `previous`: the longest
/// such overlap, so a ring that slid by `n` lines leaves its last `n` new.
fn ring_overlap(previous: &[String], ring: &[String]) -> usize {
    (0..=previous.len().min(ring.len()))
        .rev()
        .find(|&overlap| previous[previous.len() - overlap..] == ring[..overlap])
        .unwrap_or(0)
}

/// Classify a raw output line into an `OutputRecordKind` with optional
/// tool metadata, based on the `roko.stream.v1` protocol or text heuristics.
///
/// Returns `(kind, tool_id, tool_name, is_live_unscreened)`.  The fourth
/// element is `true` only when the record is a live-preview, non-tool record
/// that has not yet been validated by the safety screener (`screened: false`).
/// Callers use this to track which records should be replaced when the
/// settled, screened transcript arrives.
fn classify_output_line(line: &str) -> (OutputRecordKind, Option<String>, Option<String>, bool) {
    use super::widgets::stream_output::{StreamRecord, parse_stream_line};

    match parse_stream_line(line) {
        StreamRecord::Text { live, screened, .. } => {
            let unscreened = live && !screened;
            (OutputRecordKind::Text, None, None, unscreened)
        }
        StreamRecord::Reasoning { live, screened, .. } => {
            let unscreened = live && !screened;
            (OutputRecordKind::Reasoning, None, None, unscreened)
        }
        StreamRecord::ToolStart {
            tool_id, tool_name, ..
        } => {
            // Tool steps are never treated as unscreened for drop purposes —
            // they are kept even when the screened transcript replaces text.
            (
                OutputRecordKind::ToolCall,
                Some(tool_id),
                Some(tool_name),
                false,
            )
        }
        StreamRecord::ToolResult { tool_id, .. } => {
            (OutputRecordKind::ToolResult, Some(tool_id), None, false)
        }
        StreamRecord::Plain { ref content } => {
            // Legacy heuristic classification for untyped records.
            let trimmed = content.trim();
            if trimmed.starts_with("ERROR")
                || trimmed.starts_with("error")
                || trimmed.contains("FAILED")
            {
                (OutputRecordKind::Error, None, None, false)
            } else if trimmed.starts_with("────") || trimmed.is_empty() {
                (OutputRecordKind::System, None, None, false)
            } else {
                (OutputRecordKind::Text, None, None, false)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Bounded history limits (#366 — TUI data pipeline optimization)
// ---------------------------------------------------------------------------

/// Maximum entries in the unified log cache.
pub const MAX_UNIFIED_LOG: usize = 5_000;
/// Maximum diagnosis entries retained in TuiState.
pub const MAX_DIAGNOSES: usize = 200;
/// Maximum episode entries retained in TuiState.
pub const MAX_EPISODES: usize = 1_000;
/// Maximum error entries retained in TuiState.
pub const MAX_ERRORS: usize = 500;
/// Maximum token history samples per agent.
pub const MAX_TOKEN_SAMPLES: usize = 600;
/// Maximum gate output lines retained.
pub const MAX_GATE_LINES: usize = 2_000;
/// Maximum CPU/memory history samples for sparklines.
pub const MAX_METRIC_HISTORY: usize = 60;
/// Maximum notification history entries.
pub const MAX_NOTIFICATION_HISTORY: usize = 200;

/// Tracks evictions from bounded collections for observability.
#[derive(Debug, Clone, Default)]
pub struct EvictionCounters {
    /// Total entries evicted from the unified log.
    pub unified_log: u64,
    /// Total entries evicted from the diagnoses ring.
    pub diagnoses: u64,
    /// Total entries evicted from the episodes cache.
    pub episodes: u64,
    /// Total entries evicted from gate output lines.
    pub gate_output: u64,
    /// Total entries evicted from token history samples.
    pub token_history: u64,
    /// Total entries evicted from notification history.
    pub notifications: u64,
}

// ---------------------------------------------------------------------------
// Revision-tracked collection for cache invalidation (#366)
// ---------------------------------------------------------------------------

/// A monotonic revision counter for cache invalidation.
///
/// Each data source bumps its revision when it changes, and downstream
/// caches (like the unified log) compare their last-seen revision to
/// decide whether to rebuild.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Revision(u64);

impl Revision {
    /// Create a new revision at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Bump the revision counter. Returns the new value.
    pub fn bump(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    /// Current revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

pub(crate) fn bounded_output_lines(lines: &VecDeque<String>) -> Vec<String> {
    let first = lines.len().saturating_sub(MAX_AGENT_OUTPUT_LINES);
    lines.iter().skip(first).cloned().collect()
}

/// Live websocket-backed tail for one agent.
#[derive(Debug, Clone, Default)]
pub struct AgentStream {
    /// Recent streamed chunks for the Agents tab detail panel.
    pub chunks: VecDeque<String>,
    /// Whether the backing websocket is currently connected.
    pub connected: bool,
    /// Whether the latest observed stream has completed.
    pub completed: bool,
    /// When the most recent chunk arrived.
    pub last_chunk_at: Option<Instant>,
}

/// Per-agent routing and context metrics shown in the TUI.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RouteMetrics {
    /// Routed or observed model slug (for example `claude-sonnet-4-5`).
    pub model: String,
    /// Compact routing tier label such as `fast`, `balanced`, or `deep`.
    pub tier: String,
    /// Total tokens consumed for the latest observed turn.
    pub context_used: u64,
    /// Model context window in tokens.
    pub context_limit: u64,
    /// Prompt-focus score in the range `0.0..=1.0`.
    pub focus_score: f64,
}

/// Live system metrics for a supervised agent process.
#[derive(Debug, Clone, Default)]
pub struct ProcessMetrics {
    /// OS process identifier.
    pub pid: u32,
    /// Human-readable process role or label.
    pub role: String,
    /// Current CPU usage percentage.
    pub cpu_pct: f32,
    /// Resident memory in bytes.
    pub mem_bytes: u64,
    /// Compact state label such as `running`, `sleeping`, or `stopped`.
    pub state: String,
    /// Process uptime in seconds.
    pub uptime_secs: f64,
    /// Rolling CPU samples used for inline sparklines.
    pub cpu_history: VecDeque<f32>,
    /// Rolling memory samples used for inline sparklines.
    pub mem_history: VecDeque<u64>,
}

/// Resolve a model slug to its known context window in tokens.
#[must_use]
pub fn model_context_limit(model: &str) -> u64 {
    let model = model.trim().to_ascii_lowercase();
    if model.is_empty() {
        return 200_000;
    }

    if model.contains("gemini") && model.contains("pro") {
        1_000_000
    } else if model.contains("gpt-5") || model.contains("gpt-4") {
        128_000
    } else if model.contains("claude") {
        200_000
    } else {
        200_000
    }
}

#[must_use]
#[allow(dead_code)] // used by route_metrics_from_event; wired to TUI display in P2-TUI-7
fn route_tier_label_for_frequency(frequency: OperatingFrequency) -> &'static str {
    match frequency {
        OperatingFrequency::Gamma => "fast",
        OperatingFrequency::Theta => "balanced",
        OperatingFrequency::Delta => "deep",
    }
}

#[must_use]
#[allow(dead_code)] // used by route_metrics_from_event and fallback_route_metrics_for_agent; P2-TUI-7
fn route_tier_label_for_model(model: &str) -> &'static str {
    let lower = model.trim().to_ascii_lowercase();
    if lower.is_empty() {
        "balanced"
    } else if lower.contains("haiku")
        || lower.contains("flash-lite")
        || lower.contains("flash lite")
        || lower.contains("mini")
        || lower.contains("nano")
    {
        "fast"
    } else if lower.contains("opus")
        || lower.contains("pro-preview")
        || lower.contains("pro preview")
        || lower.contains("o1")
        || lower.contains("o3")
        || lower.contains("r1")
    {
        "deep"
    } else {
        "balanced"
    }
}

use crate::tui::display_utils::event_model_slug;

#[must_use]
#[allow(dead_code)] // focus scoring for route metrics; wired in P2-TUI-7
fn prompt_focus_score(event: &roko_learn::efficiency::AgentEfficiencyEvent) -> f64 {
    if event.prompt_sections.is_empty() {
        return if event.total_prompt_tokens > 0 {
            1.0
        } else {
            0.0
        };
    }

    let mut max_weighted = 0.0;
    let mut retained_weighted = 0.0;
    for section in &event.prompt_sections {
        let priority_weight = 1.0 / (1.0 + f64::from(section.priority));
        let weighted_tokens = section.tokens as f64 * priority_weight;
        max_weighted += weighted_tokens;

        let retention = if section.was_dropped {
            0.0
        } else if section.was_truncated {
            0.5
        } else {
            1.0
        };
        retained_weighted += weighted_tokens * retention;
    }

    if max_weighted > 0.0 {
        (retained_weighted / max_weighted).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[must_use]
#[allow(dead_code)] // route confidence scoring; wired in P2-TUI-7
fn route_focus_score(
    event: &roko_learn::efficiency::AgentEfficiencyEvent,
    data: &DashboardData,
    model: &str,
) -> f64 {
    if let Some(stats) = data.cascade_router.confidence_stats.get(model) {
        if stats.trials > 0 {
            return (stats.successes as f64 / stats.trials as f64).clamp(0.0, 1.0);
        }
    }
    prompt_focus_score(event)
}

#[must_use]
#[allow(dead_code)] // route metrics builder; wired in P2-TUI-7
fn route_metrics_from_event(
    event: &roko_learn::efficiency::AgentEfficiencyEvent,
    data: &DashboardData,
) -> RouteMetrics {
    let model = event_model_slug(event);
    let context_limit = model_context_limit(&model);
    let focus_score = route_focus_score(event, data, &model);
    RouteMetrics {
        tier: if model.is_empty() {
            route_tier_label_for_frequency(event.frequency).to_string()
        } else {
            route_tier_label_for_model(&model).to_string()
        },
        model,
        context_used: event.total_tokens(),
        context_limit,
        focus_score,
    }
}

#[must_use]
#[allow(dead_code)] // fallback route metrics; wired in P2-TUI-7
fn fallback_route_metrics_for_agent(agent: &AgentRow) -> RouteMetrics {
    let context_limit = agent
        .context_limit
        .max(model_context_limit(agent.model.as_str()));
    RouteMetrics {
        model: agent.model.clone(),
        tier: route_tier_label_for_model(&agent.model).to_string(),
        context_used: agent.input_tokens.saturating_add(agent.output_tokens),
        context_limit,
        focus_score: 0.0,
    }
}

/// A plan entry in the plan list.
///
/// Extended with fields required by the plan_tree, header_bar, status_bar,
/// and wave_progress widgets.
#[derive(Debug, Clone, Default)]
pub struct PlanEntry {
    pub id: String,
    pub name: String,
    pub status: PlanPhase,
    /// Whether the plan is currently executing.
    pub active: bool,
    /// Current phase label (e.g. "implementing", "done", "failed").
    pub phase: String,
    /// Total task count.
    pub tasks_total: usize,
    /// Completed task count.
    pub tasks_done: usize,
    /// Failed task count.
    pub tasks_failed: usize,
    /// Elapsed wall-clock seconds.
    pub elapsed_secs: f64,
    /// Wave index this plan belongs to, if any.
    pub wave: Option<usize>,
    /// Whether the plan tree node is expanded.
    pub expanded: bool,
    /// Nested task entries (for plan detail view).
    pub tasks: Vec<TaskEntry>,

    // -- git/change context (P5.3 + P5.4) --
    /// Branch name associated with this plan execution.
    pub branch: Option<String>,
    /// Worktree path used by this plan execution.
    pub worktree_path: Option<String>,
    /// Last commit hash produced by this plan execution.
    pub last_commit: Option<String>,
    /// Number of files modified by this plan execution.
    pub files_modified: Option<usize>,
    /// Total lines inserted by this plan execution.
    pub insertions: Option<usize>,
    /// Total lines deleted by this plan execution.
    pub deletions: Option<usize>,

    // -- per-plan elapsed timer (P5.5) --
    /// When this plan started executing (for live elapsed display).
    pub started_at: Option<Instant>,
}

impl PlanEntry {
    /// Tasks accepted although their verification failed. They are part of
    /// `tasks_done`, so a plan's passed count is `tasks_done` minus this.
    #[must_use]
    pub fn tasks_accepted_with_failures(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.status.is_accepted_with_failures())
            .count()
    }
}

/// A task within a plan entry.
#[derive(Debug, Clone, Default)]
pub struct TaskEntry {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub agent_id: Option<String>,
    /// Task IDs this task depends on (carried from TaskDef).
    pub depends_on: Vec<String>,
    /// Free-form acceptance criteria text (joined from TaskDef).
    pub acceptance_text: Option<String>,
    /// First verify command, if any (from TaskDef).
    pub verify_command: Option<String>,
    /// ISO timestamp when task started (from runtime metadata).
    pub started_at: Option<String>,
    /// Files this task will create or modify (from tasks.toml).
    pub files: Vec<String>,
}

/// Cost/budget projection for one plan in the F2 view.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlanBudgetSummary {
    pub spent_usd: f64,
    pub budget_usd: f64,
    pub projected_remaining_usd: f64,
    pub projected_total_usd: f64,
}

/// Git branch tree node.
#[derive(Debug, Clone, Default)]
pub struct GitBranchNode {
    /// Branch name.
    pub name: String,
    /// Whether this branch is currently checked out.
    pub is_current: bool,
    /// Upstream tracking branch, if configured.
    pub tracking: Option<String>,
    /// Number of commits ahead of the upstream branch.
    pub ahead: usize,
    /// Number of commits behind the upstream branch.
    pub behind: usize,
    /// Display indentation depth derived from the branch path.
    pub depth: u16,
    /// Nested child branches when rendered hierarchically.
    pub children: Vec<GitBranchNode>,
}

/// Git commit graph entry.
#[derive(Debug, Clone, Default)]
pub struct GitCommitEntry {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp_ms: i64,
    pub branch: Option<String>,
}

/// Severity level for a unified TUI log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryLevel {
    /// Debug-level event.
    Debug,
    /// Informational event.
    Info,
    /// Warning event.
    Warn,
    /// Error event.
    Error,
}

impl LogEntryLevel {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Debug => "DBG",
            Self::Info => "INF",
            Self::Warn => "WRN",
            Self::Error => "ERR",
        }
    }

    #[must_use]
    pub const fn filter_level(self) -> LogFilterLevel {
        match self {
            Self::Info => LogFilterLevel::Info,
            Self::Warn => LogFilterLevel::Warn,
            Self::Error => LogFilterLevel::Error,
            Self::Debug => LogFilterLevel::Debug,
        }
    }
}

/// A parsed, display-ready log row for the Logs tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Human-readable timestamp for the rendered row.
    pub timestamp: String,
    /// Severity level used for filtering and styling.
    pub level: LogEntryLevel,
    /// Compact source label such as `signal:gate`.
    pub source: String,
    /// Main message body shown in the Logs tab.
    pub message: String,
}

impl LogEntry {
    /// Construct a display-ready log entry.
    #[must_use]
    pub fn new(timestamp: String, level: LogEntryLevel, source: String, message: String) -> Self {
        Self {
            timestamp,
            level,
            source,
            message,
        }
    }
}

// ---------------------------------------------------------------------------
// Log search / filter state (#217)
// ---------------------------------------------------------------------------

/// Search mode within a log panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Matching lines are highlighted but all lines remain visible.
    #[default]
    Highlight,
    /// Only matching lines are shown.
    Filter,
}

/// State for the log search/filter feature (#217).
#[derive(Debug, Clone, Default)]
pub struct LogSearchState {
    /// Whether search mode is active (input bar visible).
    pub active: bool,
    /// Current search pattern text.
    pub pattern: String,
    /// Compiled regex from the pattern (None if pattern is empty or invalid).
    pub compiled: Option<regex::Regex>,
    /// Whether the pattern failed to compile.
    pub pattern_error: bool,
    /// Current search/filter display mode.
    pub mode: SearchMode,
    /// Total matches found in the log buffer.
    pub match_count: usize,
    /// Index of the currently highlighted match (0-based).
    pub current_match: usize,
    /// Line indices that match the current pattern (for navigation).
    pub match_indices: Vec<usize>,
}

impl LogSearchState {
    /// Update the compiled regex from the current pattern.
    pub fn recompile(&mut self) {
        if self.pattern.is_empty() {
            self.compiled = None;
            self.pattern_error = false;
            self.match_count = 0;
            self.current_match = 0;
            self.match_indices.clear();
        } else {
            match regex::RegexBuilder::new(&self.pattern)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => {
                    self.compiled = Some(re);
                    self.pattern_error = false;
                }
                Err(_) => {
                    self.compiled = None;
                    self.pattern_error = true;
                    self.match_count = 0;
                    self.current_match = 0;
                    self.match_indices.clear();
                }
            }
        }
    }

    /// Update match indices based on the given log lines.
    pub fn update_matches(&mut self, lines: &[LogEntry]) {
        self.match_indices.clear();
        if let Some(ref re) = self.compiled {
            for (i, entry) in lines.iter().enumerate() {
                if re.is_match(&entry.message) || re.is_match(&entry.source) {
                    self.match_indices.push(i);
                }
            }
        }
        self.match_count = self.match_indices.len();
        if self.current_match >= self.match_count && self.match_count > 0 {
            self.current_match = self.match_count - 1;
        }
    }

    /// Navigate to the next match, wrapping around.
    pub fn next_match(&mut self) {
        if self.match_count > 0 {
            self.current_match = (self.current_match + 1) % self.match_count;
        }
    }

    /// Navigate to the previous match, wrapping around.
    pub fn prev_match(&mut self) {
        if self.match_count > 0 {
            self.current_match = if self.current_match == 0 {
                self.match_count - 1
            } else {
                self.current_match - 1
            };
        }
    }

    /// Clear all search state.
    pub fn clear(&mut self) {
        self.active = false;
        self.pattern.clear();
        self.compiled = None;
        self.pattern_error = false;
        self.mode = SearchMode::Highlight;
        self.match_count = 0;
        self.current_match = 0;
        self.match_indices.clear();
    }
}

// ---------------------------------------------------------------------------
// Agent output search state (#367)
// ---------------------------------------------------------------------------

/// Search state for the F3:Agents output panel (#367).
///
/// Similar to `LogSearchState` but scoped to the selected agent's output
/// records. Activated by `/` on the Agents tab, navigated with `n`/`N`.
#[derive(Debug, Clone, Default)]
pub struct AgentOutputSearchState {
    /// Whether search mode is active (input bar visible).
    pub active: bool,
    /// Current search pattern text.
    pub pattern: String,
    /// Compiled regex from the pattern (None if pattern is empty or invalid).
    pub compiled: Option<regex::Regex>,
    /// Whether the pattern failed to compile.
    pub pattern_error: bool,
    /// Sequence numbers of matching records.
    pub match_seqs: Vec<u64>,
    /// Total matches found.
    pub match_count: usize,
    /// Index of the currently highlighted match (0-based).
    pub current_match: usize,
}

impl AgentOutputSearchState {
    /// Recompile the regex from the current pattern.
    pub fn recompile(&mut self) {
        if self.pattern.is_empty() {
            self.compiled = None;
            self.pattern_error = false;
            self.match_count = 0;
            self.current_match = 0;
            self.match_seqs.clear();
        } else {
            match regex::RegexBuilder::new(&self.pattern)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => {
                    self.compiled = Some(re);
                    self.pattern_error = false;
                }
                Err(_) => {
                    self.compiled = None;
                    self.pattern_error = true;
                    self.match_count = 0;
                    self.current_match = 0;
                    self.match_seqs.clear();
                }
            }
        }
    }

    /// Update match sequences from the given agent output history.
    pub fn update_matches(&mut self, history: &AgentOutputHistory, agent_id: &str) {
        self.match_seqs.clear();
        if let Some(ref re) = self.compiled {
            for record in history.records_for(agent_id) {
                if re.is_match(&display_text(&record.text))
                    || record.tool_name.as_deref().is_some_and(|n| re.is_match(n))
                    || re.is_match(&record.role)
                {
                    self.match_seqs.push(record.seq);
                }
            }
        }
        self.match_count = self.match_seqs.len();
        if self.current_match >= self.match_count && self.match_count > 0 {
            self.current_match = self.match_count - 1;
        }
    }

    /// Navigate to the next match, wrapping around.
    pub fn next_match(&mut self) {
        if self.match_count > 0 {
            self.current_match = (self.current_match + 1) % self.match_count;
        }
    }

    /// Navigate to the previous match, wrapping around.
    pub fn prev_match(&mut self) {
        if self.match_count > 0 {
            self.current_match = if self.current_match == 0 {
                self.match_count - 1
            } else {
                self.current_match - 1
            };
        }
    }

    /// Clear all search state.
    pub fn clear(&mut self) {
        self.active = false;
        self.pattern.clear();
        self.compiled = None;
        self.pattern_error = false;
        self.match_count = 0;
        self.current_match = 0;
        self.match_seqs.clear();
    }

    /// The sequence number of the currently highlighted match, if any.
    #[must_use]
    pub fn current_match_seq(&self) -> Option<u64> {
        self.match_seqs.get(self.current_match).copied()
    }
}

// ---------------------------------------------------------------------------
// Plan tree filter state (#219)
// ---------------------------------------------------------------------------

/// Filter state for the F2:Plans tab tree (#219).
#[derive(Debug, Clone, Default)]
pub struct PlanTreeFilter {
    /// Whether filter input mode is active.
    pub active: bool,
    /// Current filter text input.
    pub pattern: String,
    /// Pre-parsed status filter prefix, if present.
    pub status_filter: Option<PlanPhase>,
    /// Substring to match after stripping status prefix.
    pub text_filter: String,
}

impl PlanTreeFilter {
    /// Update parsed fields from the current pattern text.
    pub fn reparse(&mut self) {
        let lower = self.pattern.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("status:failed") {
            self.status_filter = Some(PlanPhase::Failed);
            self.text_filter = rest.trim().to_string();
        } else if let Some(rest) = lower.strip_prefix("status:active") {
            self.status_filter = Some(PlanPhase::Active);
            self.text_filter = rest.trim().to_string();
        } else if let Some(rest) = lower.strip_prefix("status:done") {
            self.status_filter = Some(PlanPhase::Done);
            self.text_filter = rest.trim().to_string();
        } else if let Some(rest) = lower.strip_prefix("status:pending") {
            self.status_filter = Some(PlanPhase::Pending);
            self.text_filter = rest.trim().to_string();
        } else {
            self.status_filter = None;
            self.text_filter = lower;
        }
    }

    /// Whether a plan entry matches the current filter.
    pub fn matches_plan(&self, entry: &PlanEntry) -> bool {
        if self.pattern.is_empty() {
            return true;
        }
        // Status filter
        if let Some(ref status) = self.status_filter {
            if entry.status != *status {
                return false;
            }
        }
        // Text filter (case-insensitive substring match)
        if !self.text_filter.is_empty() {
            let id_lower = entry.id.to_ascii_lowercase();
            let name_lower = entry.name.to_ascii_lowercase();
            if !id_lower.contains(&self.text_filter) && !name_lower.contains(&self.text_filter) {
                return false;
            }
        }
        true
    }

    /// Whether a plan matches because it or any of its tasks match.
    pub fn matches_plan_or_tasks(&self, entry: &PlanEntry) -> bool {
        if self.matches_plan(entry) {
            return true;
        }
        // Check if any child task matches (maintain tree structure)
        if !self.text_filter.is_empty() {
            for task in &entry.tasks {
                let task_lower = task.name.to_ascii_lowercase();
                let id_lower = task.id.to_ascii_lowercase();
                if task_lower.contains(&self.text_filter) || id_lower.contains(&self.text_filter) {
                    return true;
                }
            }
        }
        false
    }

    /// Clear filter state.
    pub fn clear(&mut self) {
        self.active = false;
        self.pattern.clear();
        self.status_filter = None;
        self.text_filter.clear();
    }
}

// ---------------------------------------------------------------------------
// Phase pipeline types (for phase_compact widget)
// ---------------------------------------------------------------------------

/// A single step in the phase pipeline.
#[derive(Debug, Clone, Default)]
pub struct PhaseStep {
    /// Phase name (e.g. "preflight", "implementer", "compile-gate").
    pub name: String,
    /// Current status of this phase.
    pub status: PlanPhase,
    /// Elapsed seconds in this phase.
    pub elapsed_secs: f64,
    /// Completion percentage (0.0 .. 100.0).
    pub pct: f64,
}

/// Status of a phase pipeline step.
pub type PhaseStatus = PlanPhase;

// ---------------------------------------------------------------------------
// Execution waves (for plan_tree, wave_progress, header_bar widgets)
// ---------------------------------------------------------------------------

/// An execution wave grouping plans for parallel execution.
#[derive(Debug, Clone, Default)]
pub struct Wave {
    /// Zero-based wave index.
    pub index: usize,
    /// Plan IDs in this wave.
    pub plans: Vec<String>,
    /// Number of completed plans in this wave.
    pub done: usize,
    /// Total plans in this wave.
    pub total: usize,
    /// Whether the wave tree node is expanded.
    pub expanded: bool,
    /// Wave indices that must complete before this wave can start.
    pub blocked_by_waves: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Task checklist (for task_progress widget)
// ---------------------------------------------------------------------------

/// Status of a task row in the checklist widget.
pub type TaskRowStatus = TaskStatus;

/// A row in the task checklist widget.
#[derive(Debug, Clone, Default)]
pub struct TaskRow {
    /// Task identifier.
    pub id: String,
    /// Human-readable task title.
    pub title: String,
    /// Task status.
    pub status: TaskStatus,
    /// Elapsed seconds for this task.
    pub elapsed_secs: f64,
    /// Task IDs this task depends on (from tasks.toml).
    pub depends_on: Vec<String>,
    /// Free-form acceptance criteria text (joined from tasks.toml).
    pub acceptance_text: Option<String>,
    /// First verify command, if any (from tasks.toml).
    pub verify_command: Option<String>,
    /// Files this task will create or modify (from tasks.toml).
    pub files: Vec<String>,
}

// ---------------------------------------------------------------------------
// System metrics (for sys_metrics, header_bar widgets)
// ---------------------------------------------------------------------------

/// A single gate result for the command_output widget.
#[derive(Debug, Clone, Default)]
pub struct GateResultEntry {
    /// Verify name (e.g. "compile", "clippy", "test").
    pub gate: String,
    /// Plan ID this gate ran against.
    pub plan_id: String,
    /// Task the gate ran for; empty when unknown.
    pub task_id: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Verify output text (stdout + stderr).
    pub output: String,
}

impl From<&GateResultSummary> for GateResultEntry {
    fn from(value: &GateResultSummary) -> Self {
        Self {
            gate: value.gate_name.clone(),
            plan_id: value.plan_id.clone(),
            task_id: String::new(),
            passed: value.passed,
            output: value.summary.clone(),
        }
    }
}

/// System resource metrics snapshot.
#[derive(Debug, Clone, Default)]
pub struct SysMetrics {
    /// CPU usage percentage (0.0 .. 100.0).
    pub cpu_pct: f32,
    /// Recent CPU usage history for sparkline.
    pub cpu_history: VecDeque<f32>,
    /// Memory currently used in bytes.
    pub mem_used_bytes: u64,
    /// Total system memory in bytes.
    pub mem_total_bytes: u64,
    /// Recent memory usage history (fractional, 0.0..1.0) for sparkline.
    pub mem_history: VecDeque<f32>,
    /// Network download bytes/sec (computed rate).
    pub net_down_bytes_sec: u64,
    /// Network upload bytes/sec (computed rate).
    pub net_up_bytes_sec: u64,
    /// Disk read bytes/sec (computed rate).
    pub disk_read_bytes_sec: u64,
    /// Disk write bytes/sec (computed rate).
    pub disk_write_bytes_sec: u64,
    /// Disk free space in bytes.
    pub disk_free_bytes: u64,
    /// Total disk space in bytes.
    pub disk_total_bytes: u64,
}

#[derive(Debug, Clone)]
struct SmoothedValue {
    current: f64,
    alpha: f64,
}

impl SmoothedValue {
    fn new(alpha: f64) -> Self {
        Self {
            current: 0.0,
            alpha,
        }
    }

    fn update(&mut self, sample: f64) -> f64 {
        self.current = self.alpha * sample + (1.0 - self.alpha) * self.current;
        self.current
    }
}

// ---------------------------------------------------------------------------
// TuiState
// ---------------------------------------------------------------------------

/// Which field has focus in the job creation form.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JobFormField {
    #[default]
    Title,
    Type,
    Priority,
    Description,
}

/// Result of a TUI-initiated command (e.g. job creation, PRD publish).
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded.
    pub ok: bool,
    /// Short label describing the command (e.g. "create-job").
    pub label: String,
    /// Human-readable result message.
    pub message: String,
}

/// Sort mode for the cost-by-model table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CostSortMode {
    /// Sort alphabetically by model name (default BTreeMap order).
    #[default]
    Name,
    /// Sort descending by total cost.
    Cost,
    /// Sort descending by task count.
    Tasks,
    /// Sort descending by pass rate.
    PassRate,
}

impl CostSortMode {
    /// Cycle to the next sort mode.
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Cost,
            Self::Cost => Self::Tasks,
            Self::Tasks => Self::PassRate,
            Self::PassRate => Self::Name,
        }
    }

    /// Short label for the header indicator.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Cost => "cost",
            Self::Tasks => "tasks",
            Self::PassRate => "pass%",
        }
    }
}

/// Complete TUI state, matching Mori's `RunState` field set.
#[derive(Debug, Clone)]
pub struct TuiState {
    // -- core orchestrator state --
    /// Serialized orchestrator state label (e.g. "running", "paused").
    pub orchestrator_state: String,
    /// Plan entries with nested tasks.
    pub plans: Vec<PlanEntry>,
    /// Index of the currently selected plan in the plan list.
    pub current_plan_idx: usize,
    /// Current iteration number.
    pub current_iteration: usize,
    /// Current phase label.
    pub current_phase: String,

    // -- phase pipeline --
    /// Ordered phase steps for the phase_compact widget.
    pub phase_pipeline: Vec<PhaseStep>,

    // -- execution waves --
    /// Execution waves grouping plans for parallel execution.
    pub execution_waves: Vec<Wave>,

    // -- critical path ETA --
    /// Remaining ETA minutes computed from the critical path of the task DAG.
    /// Updated when a critical-path task completes. `None` when not yet computed.
    pub critical_path_eta_minutes: Option<f64>,
    /// Computed C-factor value for status bar display.
    pub c_factor: Option<f64>,

    // -- task checklist --
    /// Task rows for the task_progress widget.
    pub current_task_checklist: Vec<TaskRow>,
    /// Smoothed task progress (0.0..1.0) for animated progress bars.
    /// Updated each frame via EMA to avoid jarring jumps.
    pub task_progress_smooth: super::smoothing::SmoothedValue,

    // -- gate results --
    /// Verify pipeline results for the command_output widget.
    pub gate_results: Vec<GateResultEntry>,
    /// Recent conductor diagnoses from the live dashboard snapshot.
    pub diagnoses: Vec<roko_core::dashboard_snapshot::DiagnosisSummary>,
    /// Concluded prompt experiment winners for the Learning tab.
    pub experiment_winners: Vec<roko_core::ExperimentWinnerSummary>,
    /// Rolling per-gate pass/fail trends from the live verdict reader.
    pub gate_trends: HashMap<String, roko_core::TrendBuckets>,
    /// Recent failing verdicts surfaced beside the trend grid.
    pub gate_recent_failures: Vec<roko_core::FailureEntry>,
    /// Latest gate output retained per task and verify step by the live
    /// snapshot: a leading `$ command` line (when published) plus the output
    /// tail.
    pub task_gate_outputs: Vec<roko_core::dashboard_snapshot::TaskGateOutput>,
    /// Plan set of each plan id seen in the live snapshot, from disk
    /// discovery (`None` for top-level or undiscovered plans). Read through
    /// [`TuiState::plan_group`], which prefers the announced plan set's group.
    pub plan_groups: HashMap<String, Option<String>>,
    /// Plans announced by the live snapshot's plan set, in execution order,
    /// with each plan's group, wave, prerequisites and conflicts. Empty when
    /// no plan set was announced.
    pub plan_set: Vec<roko_core::dashboard_snapshot::PlanSetEntry>,

    // -- gate output --
    /// Streaming gate output lines from rung executions (bounded).
    pub gate_output_lines: VecDeque<String>,
    /// Currently running gate rung: (rung_name, started_at). `None` when idle.
    pub current_gate_rung: Option<(String, Instant)>,
    /// Gate output scroll offset. 0 = auto-tail (follow latest output).
    pub gate_output_scroll: usize,

    // -- affect state --
    /// Latest Daimon affect state from the runner.
    pub affect: Option<roko_core::AffectSnapshot>,

    // -- agents (Vec-based roster for widgets) --
    /// Ordered agent roster, read by the Agents, Dashboard and Atelier views
    /// and the header_bar, status_bar, cost_by_model and token_sparkline widgets.
    pub agents: Vec<AgentRow>,
    /// Latest fetched agent-topology payload.
    pub agent_topology: roko_core::AgentTopology,
    /// Fetch status for the agent-topology panel.
    pub agent_topology_status: AgentTopologyStatus,
    /// Per-agent route and context metrics keyed by agent identifier.
    pub route_metrics: HashMap<String, RouteMetrics>,
    /// Cached styled agent output keyed by agent identifier.
    pub agent_output_cache: RefCell<HashMap<String, CachedRender>>,
    /// Live websocket tails keyed by agent identifier.
    pub agent_streams: HashMap<String, AgentStream>,
    /// Structured per-agent output history with pagination (#367).
    pub agent_output_history: AgentOutputHistory,
    /// Agent output search state for F3:Agents tab (#367).
    pub agent_output_search: AgentOutputSearchState,
    /// Tool IDs whose results are unfolded in the stream output widget.
    pub agent_output_unfolded: HashSet<String>,
    // -- navigation --
    /// Active top-level tab.
    pub active_tab: Tab,
    /// Selected plan index (legacy, may differ from current_plan_idx during browsing).
    pub selected_plan_idx: usize,
    /// Cached full name of the selected plan for status bar display.
    pub selected_plan_name: Option<String>,
    /// Selected agent index in the agent roster.
    pub selected_agent: usize,
    /// Selected agent sub-tab index.
    pub selected_agent_tab: usize,
    /// Selected Dashboard detail panel index.
    pub dashboard_sub_tab: usize,
    /// Selected Git tab sub-view index.
    pub git_sub_tab: usize,
    /// Selected Logs tab sub-view index.
    pub logs_sub_tab: usize,
    /// Selected Learning tab sub-view index.
    pub learning_sub_tab: usize,
    /// Selected Config tab sub-view index.
    pub config_sub_tab: usize,
    /// Selected Inspect tab sub-view index.
    pub inspect_sub_tab: usize,
    /// Selected Marketplace tab sub-view index.
    pub marketplace_sub_tab: usize,
    /// Selected Atelier tab sub-view index.
    pub atelier_sub_tab: usize,
    /// Which panel has keyboard focus.
    pub focus: FocusZone,

    // -- animation --
    /// Atmosphere animation state for breathing/heartbeat/spinners.
    pub atmosphere: Atmosphere,

    // -- input --
    /// Current input mode (normal, inject, filter, confirm).
    pub input_mode: InputMode,
    /// Text buffer for inject mode.
    pub message_input: String,
    /// Text buffer for filter mode.
    pub filter_text: String,
    /// Whether filter is actively applied.
    pub filter_active: bool,
    /// Filter alias (mirrors filter_text for widget compatibility).
    pub filter: String,

    // -- wave tree collapse state --
    /// Set of wave indices that the user has manually collapsed in the F2
    /// plan tree. Expanded is the default state; toggling adds/removes from
    /// this set.
    pub collapsed_waves: HashSet<usize>,

    // -- scroll positions --
    /// Agent output scroll. `None` means auto-tail (follow latest output).
    pub agent_scroll: Option<usize>,
    /// Diff panel scroll offset.
    pub diff_scroll: usize,
    /// Procs sub-tab scroll offset (independent from diff_scroll).
    pub procs_scroll: usize,
    /// Per-tab detail-pane scroll offsets (independent from diff_scroll).
    pub git_detail_scroll: usize,
    /// Log detail pane scroll offset.
    pub log_detail_scroll: usize,
    /// Config values pane scroll offset.
    pub config_values_scroll: usize,
    /// Inspect detail pane scroll offset.
    pub inspect_detail_scroll: usize,
    /// Marketplace detail pane scroll offset.
    pub marketplace_detail_scroll: usize,
    /// Atelier detail pane scroll offset.
    pub atelier_detail_scroll: usize,
    /// Learning detail pane scroll offset.
    pub learning_detail_scroll: usize,
    /// Task list scroll offset.
    pub task_scroll: usize,
    /// Command output panel scroll offset.
    pub command_output_scroll: usize,
    /// Plan detail overlay scroll offset.
    pub plan_detail_scroll: usize,
    /// Help modal scroll offset.
    pub help_scroll: usize,
    /// Plan list scroll offset (for long plan lists).
    pub plan_scroll_offset: usize,
    /// Log viewer scroll offset.
    pub log_scroll: usize,
    /// Whether the agent-topology overlay is visible.
    pub agent_topology_visible: bool,
    /// Agent-topology overlay scroll offset.
    pub agent_topology_scroll_offset: usize,
    /// Whether the log viewer is following the tail.
    pub log_auto_tail: bool,
    /// Active log levels shown in the Logs tab.
    pub log_filter_levels: HashSet<LogFilterLevel>,
    /// Log grouping mode (chronological, by-plan, by-task).
    pub log_grouping: super::views::logs_view::LogGrouping,
    /// Index of the currently expanded log entry (Enter key toggles).
    pub log_expanded_idx: Option<usize>,
    /// Set of collapsed group names in grouped log views.
    pub log_collapsed_groups: std::collections::HashSet<String>,

    // -- approval / confirm --
    /// Pending agent command approval, if any.
    pub pending_approval: Option<PendingApproval>,
    /// Pending confirmation dialog action, if any.
    pub pending_confirm: Option<ConfirmAction>,
    /// Active modal overlay, if any.
    pub active_modal: Option<ModalState>,

    // -- git --
    /// Current git branch name.
    pub git_branch: String,
    /// Short commit hash for the status bar.
    pub git_commit_short: String,
    /// Human-readable commit age for the status bar (e.g. "2m ago").
    pub git_age: String,
    /// Git branch tree for the Git tab.
    pub git_branch_tree: Vec<GitBranchNode>,
    /// Git commit graph entries.
    pub git_commit_graph: Vec<GitCommitEntry>,
    /// Git worktree list entries.
    pub git_worktree_list: Vec<String>,
    /// Cursor position in the git branch tree.
    pub git_branch_cursor: usize,
    /// Cached git summary lines for the dashboard sub-tab (populated by background thread).
    pub git_summary_lines: Vec<String>,
    /// Cached full git view data for F4 Git tab (populated by background thread).
    pub(crate) git_view_data: Option<super::views::git_view::GitViewData>,

    // -- plan detail --
    /// Active sub-tab in the plan detail overlay.
    pub plan_detail_tab: usize,

    // -- pipeline --
    /// Whether pipeline execution is currently paused.
    pub is_paused: bool,

    // -- cost / tokens --
    /// Sort mode for the cost-by-model table widget.
    pub cost_sort_mode: CostSortMode,
    /// Cost per plan (plan_id -> USD).
    pub cost_per_plan: HashMap<String, f64>,
    /// Cost per task (task_id -> USD).
    pub cost_per_task: HashMap<String, f64>,
    /// Effective per-plan ceiling; zero means unlimited.
    pub max_plan_budget_usd: f64,
    /// Effective task ceilings keyed by `"{plan_id}:{task_id}"`.
    pub task_budget_usd: HashMap<String, f64>,
    /// Cumulative input tokens across all agents.
    pub cumulative_input_tokens: u64,
    /// Cumulative output tokens across all agents.
    pub cumulative_output_tokens: u64,
    /// Total token count (input + output) for header_bar / token_sparkline.
    pub token_total: u64,
    /// Rolling per-agent cumulative token totals, bounded to recent samples.
    pub token_history: HashMap<String, VecDeque<u64>>,
    /// Current token burn rate (tokens per minute) for token_sparkline.
    pub token_rate: f64,
    /// Current cost burn rate (USD per minute).
    pub cost_rate: f64,
    /// Cumulative cost in USD for header_bar display.
    pub cost_dollars: f64,
    /// Live token burn rate (tokens per minute) for status bar display.
    pub token_rate_per_min: f64,
    /// Projected total cost based on current burn rate. `None` when not yet computed.
    pub projected_cost: Option<f64>,
    /// Per-process metrics for the dashboard Procs sub-tab.
    pub process_metrics: Vec<ProcessMetrics>,

    // -- system metrics --
    /// System resource metrics snapshot.
    pub sys: SysMetrics,

    // -- timing --
    /// When the current run started, for elapsed time calculation.
    pub run_started: Option<Instant>,
    /// Immutable elapsed time published by a terminal runner snapshot.
    pub run_duration_secs: Option<f64>,
    /// A connected runner announced a plan set that has not finished yet.
    /// Stays `true` in the gaps between plans, when no plan is active.
    pub plan_set_running: bool,

    // -- wave navigation --
    /// Selected wave index for wave prev/next navigation.
    pub selected_wave_idx: usize,

    // -- config editor --
    /// Cursor index into the flat config item list.
    pub config_cursor: usize,
    /// Unsaved edits: config key -> new value string.
    pub config_pending: HashMap<String, String>,
    /// Config panel scroll offset.
    pub config_scroll_offset: usize,
    /// Whether text-input mode is active for a config field.
    pub config_editing: bool,
    /// Text input buffer for the field being edited.
    pub config_edit_buffer: String,
    /// Which config key is currently being text-edited.
    pub config_edit_key: Option<String>,

    // -- agent pane --
    /// Active agent pane display group (cycles through available groups).
    pub agent_pane_group: usize,

    // -- push-path state (from DashboardSnapshot) --
    /// Orchestrator event log entries.
    pub event_log: Vec<roko_core::DashboardEventLogEntry>,
    /// Cascade router state as opaque JSON.
    pub cascade_router_json: String,
    /// Adaptive gate thresholds as opaque JSON.
    pub gate_thresholds_json: String,

    // -- view data (migrated from DashboardData) --
    /// Workspace root path for config file loading.
    pub workdir: PathBuf,
    /// Efficiency summary stats.
    pub efficiency_summary: EfficiencySummary,
    /// Raw efficiency events for per-agent metrics and aggregation.
    pub efficiency_events: Vec<roko_learn::efficiency::AgentEfficiencyEvent>,
    /// Efficiency trend buckets for charts.
    pub efficiency_trend: Vec<roko_learn::aggregate::EfficiencyBucket>,
    /// C-factor trend buckets for charts.
    pub cfactor_trend_buckets: Vec<roko_learn::aggregate::CFactorBucket>,
    /// Cascade router state for model routing display.
    pub cascade_router: CascadeRouterState,
    /// Recent signals for the logs tab.
    pub recent_signals: Vec<SignalSummary>,
    /// Current plan execution snapshot for the plan detail view.
    pub current_plan_execution: Option<PlanExecutionSnapshot>,
    /// Conductor alerts for the inspect tab.
    pub conductor_alerts: Vec<AlertSummary>,
    /// C-factor snapshot for the inspect tab.
    pub cfactor: Option<roko_learn::cfactor::CFactor>,
    /// Verify results page data (gate_rows, failure_rows, threshold_rows).
    pub gate_results_page: GateResultsPageData,
    /// Experiment summaries for the config tab.
    pub experiments: Vec<ExperimentSummary>,
    /// Playbook summaries for the F10 Learning tab (P2-05).
    pub playbook_summaries: Vec<PlaybookSummary>,
    /// Incremental tailer over `.roko/learn/efficiency.jsonl`, used in
    /// connected mode where the core snapshot cannot carry per-event
    /// learning payloads (they are `roko-learn` types).
    connected_efficiency_tailer:
        super::jsonl_tailer::IncrementalTailer<roko_learn::efficiency::AgentEfficiencyEvent>,
    /// Last observed efficiency log size; change detector for the tailer.
    connected_efficiency_len: u64,
    /// Last observed experiments store stamp `(len, mtime_ms)`.
    connected_experiments_stamp: (u64, i64),
    /// Per-task output tails.
    pub task_output_tails: HashMap<String, Vec<String>>,
    /// Git diff content.
    pub git_diff: String,
    /// Plan summaries (legacy format from DashboardData).
    pub plan_summaries: Vec<PlanSummary>,
    /// Agent summaries (legacy format from DashboardData).
    pub agent_summaries: Vec<AgentSummary>,
    /// Active task summaries (legacy format from DashboardData).
    pub active_task_summaries: Vec<TaskSummary>,
    /// Verify result summaries (legacy format from DashboardData).
    pub gate_result_summaries: Vec<GateResultSummary>,
    /// Cached episodes for the logs tab.
    pub episodes_cache: Vec<roko_learn::episode_logger::Episode>,
    /// Cached unified log for the logs tab.
    pub cached_unified_log: Vec<LogEntry>,

    // -- log search (#217) --
    /// Log search/filter state for regex highlighting and filtering.
    pub log_search: LogSearchState,
    /// Last yanked (copied) log entry text, set by `y` key on Logs tab.
    pub yanked_text: Option<String>,

    // -- plan tree filter (#219) --
    /// Plan tree filter state for F2:Plans tab.
    pub plan_tree_filter: PlanTreeFilter,

    // -- network stats (header bar) --
    /// Number of agents currently online/discovered.
    pub agents_online: usize,
    /// Recent gate pass rate displayed in the network-status header.
    pub gate_pass_rate: Option<f64>,
    /// Number of active MCP connections.
    pub mcp_connection_count: usize,
    /// Current TUI frames-per-second.
    pub tui_fps: f32,
    /// Whether the warning bar has been dismissed by the user (`n` key).
    pub warnings_dismissed: bool,
    /// Per-warning dismiss set -- individual warning keys dismissed by the user.
    /// New warnings that don't match a dismissed key will still appear.
    pub dismissed_warning_keys: HashSet<String>,

    // -- notification history --
    /// Retained history of expired/dismissed notifications (bounded to 200).
    pub notification_history: VecDeque<super::modals::NotificationRecord>,
    /// Count of entries evicted from history due to the 200-entry cap.
    pub notification_evicted_count: usize,
    /// Monotonic counter for assigning notification IDs.
    pub notification_next_id: u64,

    // -- knowledge browse --
    /// Knowledge entries for the Inspect tab's KnowledgeBrowse sub-view.
    pub knowledge_entries: Vec<KnowledgeBrowseEntry>,

    // -- marketplace / atelier --
    /// Jobs loaded from .roko/jobs/ for the Marketplace tab.
    pub marketplace_jobs: Vec<roko_core::MarketplaceJob>,
    /// Selected job index in the Marketplace tab.
    pub marketplace_selected_job: usize,
    /// PRD summaries for the Atelier tab.
    pub atelier_prds: Vec<roko_core::PrdSummary>,
    /// Selected PRD index in the Atelier tab.
    pub atelier_selected_prd: usize,
    /// Per-slug task lists for the Atelier tab.
    pub atelier_tasks_by_slug: HashMap<String, Vec<roko_core::job::TaskSummary>>,
    /// Whether the job creation form is in editing mode.
    pub job_form_editing: bool,
    /// Job form: title field.
    pub job_form_title: String,
    /// Job form: type field.
    pub job_form_type: String,
    /// Job form: priority field.
    pub job_form_priority: String,
    /// Job form: description field.
    pub job_form_description: String,
    /// Job form: currently focused field.
    pub job_form_focus: JobFormField,
    /// Whether the job assign inline prompt is active.
    pub job_assign_editing: bool,
    /// Text buffer for the assign-agent prompt.
    pub job_assign_buffer: String,
    /// Per-job progress entries (job_id → progress).
    pub job_progress: HashMap<String, roko_core::JobProgressEntry>,
    /// Results of TUI-initiated commands (e.g. job creation feedback).
    pub command_results: Vec<CommandResult>,

    // -- config items cache (P3.2) --
    /// Cached flat config item list to avoid re-parsing `roko.toml` per frame.
    pub config_items_cache: Vec<super::config_meta::ConfigItem>,
    /// When the config items cache was last rebuilt.
    pub config_items_refreshed_at: Option<Instant>,
    /// Snapshot of `config_pending.len()` when cache was built, used to detect edits.
    config_items_pending_len: usize,

    // -- inspect data cache (P3.3) --
    /// Cached data for the F7:Inspect three-panel layout, refreshed on a 5-second cadence.
    pub inspect_data: InspectData,
    /// Timestamp of the last inspect data refresh.
    pub inspect_last_refresh: Option<Instant>,

    // -- dream view cache (RC-4) --
    /// Cached journal/archive data for the F7:Inspect/Dreams sub-view.
    pub dream_view_cache: DreamViewCache,

    // -- knowledge health cache (RC-4) --
    /// Cached knowledge aggregate stats for the F7:Inspect/Knowledge Health sub-view.
    pub knowledge_health_cache: KnowledgeHealthCache,

    // -- runtime status cache (RC-4) --
    /// Cached relay+lens status for the F6 Config runtime sections.
    pub runtime_status_cache: RuntimeStatusCache,

    // -- MCP config cache (P3.1) --
    /// Cached MCP configuration for the Dashboard MCP panel.
    pub mcp_config_view: McpConfigView,
    /// Timestamp of the last MCP configuration refresh.
    pub mcp_config_refreshed_at: Option<Instant>,

    // -- conductor panel cache --
    /// Cached conductor snapshot for the F1:Dashboard Conductor sub-tab.
    pub conductor_snapshot: super::widgets::conductor_panel::ConductorSnapshot,
    /// Timestamp of the last conductor snapshot refresh.
    pub conductor_snapshot_refreshed_at: Option<Instant>,

    cpu_pct_smoothed: SmoothedValue,
    token_rate_smoothed: SmoothedValue,
    cost_rate_smoothed: SmoothedValue,
    last_rate_sample_at: Option<Instant>,
    last_token_total_sample: u64,
    last_cost_dollars_sample: f64,

    // -- revision tracking (#366) --
    /// Revision counter for the signals collection.
    pub(crate) rev_signals: Revision,
    /// Revision counter for the episodes collection.
    pub(crate) rev_episodes: Revision,
    /// Revision counter for the efficiency events collection.
    pub(crate) rev_efficiency: Revision,
    /// Revision counter for the gate results page (failure_rows).
    pub(crate) rev_gate_results: Revision,
    /// Revision counter for the event log.
    pub(crate) rev_event_log: Revision,
    /// Combined revision snapshot used to decide if the unified log cache
    /// needs rebuilding.
    unified_log_input_rev: u64,

    // -- eviction counters (#366) --
    /// Tracks evictions from bounded history collections.
    pub eviction_counters: EvictionCounters,

    // -- inbox --
    /// Unresolved human-attention inbox items, sorted by received_at_ms ascending.
    pub inbox_items: Vec<roko_core::dashboard_snapshot::InboxItemState>,
    /// Scroll offset for the Inbox sub-tab on the F1 Dashboard.
    pub inbox_scroll: usize,

    // -- safety incidents (P2-06) --
    /// Safety incidents loaded from `.roko/immune/` or extracted from log entries.
    pub safety_incidents: Vec<SafetyIncident>,

    // -- providers (F11) --
    /// Provider status snapshots for the F11 Providers NERV tab.
    pub provider_statuses: Vec<ProviderStatus>,
    /// Selected Providers tab sub-view index.
    pub providers_sub_tab: usize,
    /// Providers detail pane scroll offset.
    pub providers_detail_scroll: usize,
    /// Selected provider index in the provider list.
    pub providers_selected: usize,
}

impl Default for TuiState {
    fn default() -> Self {
        const METRIC_EMA_ALPHA: f64 = 0.25;

        Self {
            orchestrator_state: String::from("idle"),
            plans: Vec::new(),
            current_plan_idx: 0,
            current_iteration: 0,
            current_phase: String::new(),

            phase_pipeline: Vec::new(),
            execution_waves: Vec::new(),
            collapsed_waves: HashSet::new(),
            critical_path_eta_minutes: None,
            c_factor: None,
            current_task_checklist: Vec::new(),
            task_progress_smooth: super::smoothing::SmoothedValue::new(0.15),
            gate_results: Vec::new(),
            diagnoses: Vec::new(),
            experiment_winners: Vec::new(),
            gate_trends: HashMap::new(),
            gate_recent_failures: Vec::new(),
            task_gate_outputs: Vec::new(),
            plan_groups: HashMap::new(),
            plan_set: Vec::new(),

            gate_output_lines: VecDeque::new(),
            current_gate_rung: None,
            gate_output_scroll: 0,

            affect: None,

            agents: Vec::new(),
            agent_topology: roko_core::AgentTopology::default(),
            agent_topology_status: AgentTopologyStatus::Idle,
            route_metrics: HashMap::new(),
            agent_output_cache: RefCell::new(HashMap::new()),
            agent_streams: HashMap::new(),
            agent_output_history: AgentOutputHistory::default(),
            agent_output_search: AgentOutputSearchState::default(),
            agent_output_unfolded: HashSet::new(),
            active_tab: Tab::default(),
            selected_plan_idx: 0,
            selected_plan_name: None,
            selected_agent: 0,
            selected_agent_tab: 0,
            dashboard_sub_tab: 0,
            git_sub_tab: 0,
            logs_sub_tab: 0,
            learning_sub_tab: 0,
            config_sub_tab: 0,
            inspect_sub_tab: 0,
            marketplace_sub_tab: 0,
            atelier_sub_tab: 0,
            focus: FocusZone::default(),

            atmosphere: Atmosphere::default(),

            input_mode: InputMode::default(),
            message_input: String::new(),
            filter_text: String::new(),
            filter_active: false,
            filter: String::new(),

            agent_scroll: None,
            diff_scroll: 0,
            procs_scroll: 0,
            git_detail_scroll: 0,
            log_detail_scroll: 0,
            config_values_scroll: 0,
            inspect_detail_scroll: 0,
            marketplace_detail_scroll: 0,
            atelier_detail_scroll: 0,
            learning_detail_scroll: 0,
            task_scroll: 0,
            command_output_scroll: 0,
            plan_detail_scroll: 0,
            help_scroll: 0,
            plan_scroll_offset: 0,
            log_scroll: 0,
            agent_topology_visible: false,
            agent_topology_scroll_offset: 0,
            log_auto_tail: true,
            log_filter_levels: LogFilterLevel::all().into_iter().collect(),
            log_grouping: super::views::logs_view::LogGrouping::default(),
            log_expanded_idx: None,
            log_collapsed_groups: std::collections::HashSet::new(),

            pending_approval: None,
            pending_confirm: None,
            active_modal: None,

            git_branch: String::new(),
            git_commit_short: String::new(),
            git_age: String::new(),
            git_branch_tree: Vec::new(),
            git_commit_graph: Vec::new(),
            git_worktree_list: Vec::new(),
            git_branch_cursor: 0,
            git_summary_lines: Vec::new(),
            git_view_data: None,

            plan_detail_tab: 0,

            is_paused: false,

            cost_sort_mode: CostSortMode::default(),
            cost_per_plan: HashMap::new(),
            cost_per_task: HashMap::new(),
            max_plan_budget_usd: 0.0,
            task_budget_usd: HashMap::new(),
            cumulative_input_tokens: 0,
            cumulative_output_tokens: 0,
            token_total: 0,
            token_history: HashMap::new(),
            token_rate: 0.0,
            cost_rate: 0.0,
            cost_dollars: 0.0,
            token_rate_per_min: 0.0,
            projected_cost: None,
            process_metrics: Vec::new(),

            sys: SysMetrics::default(),

            run_started: None,
            run_duration_secs: None,
            plan_set_running: false,

            selected_wave_idx: 0,

            config_cursor: 0,
            config_scroll_offset: 0,
            config_pending: HashMap::new(),
            config_editing: false,
            config_edit_buffer: String::new(),
            config_edit_key: None,

            config_items_cache: Vec::new(),
            config_items_refreshed_at: None,
            config_items_pending_len: 0,

            inspect_data: InspectData::default(),
            inspect_last_refresh: None,

            dream_view_cache: DreamViewCache::default(),
            knowledge_health_cache: KnowledgeHealthCache::default(),
            runtime_status_cache: RuntimeStatusCache::default(),

            mcp_config_view: McpConfigView::default(),
            mcp_config_refreshed_at: None,

            conductor_snapshot: Default::default(),
            conductor_snapshot_refreshed_at: None,

            agent_pane_group: 0,

            event_log: Vec::new(),
            cascade_router_json: String::new(),
            gate_thresholds_json: String::new(),

            workdir: PathBuf::new(),
            efficiency_summary: EfficiencySummary::default(),
            efficiency_events: Vec::new(),
            efficiency_trend: Vec::new(),
            cfactor_trend_buckets: Vec::new(),
            cascade_router: CascadeRouterState::default(),
            recent_signals: Vec::new(),
            current_plan_execution: None,
            conductor_alerts: Vec::new(),
            cfactor: None,
            gate_results_page: GateResultsPageData::default(),
            experiments: Vec::new(),
            playbook_summaries: Vec::new(),
            connected_efficiency_tailer: super::jsonl_tailer::IncrementalTailer::default(),
            connected_efficiency_len: 0,
            connected_experiments_stamp: (0, 0),
            task_output_tails: HashMap::new(),
            git_diff: String::new(),
            plan_summaries: Vec::new(),
            agent_summaries: Vec::new(),
            active_task_summaries: Vec::new(),
            gate_result_summaries: Vec::new(),
            episodes_cache: Vec::new(),
            cached_unified_log: Vec::new(),

            log_search: LogSearchState::default(),
            yanked_text: None,
            plan_tree_filter: PlanTreeFilter::default(),

            agents_online: 0,
            gate_pass_rate: None,
            mcp_connection_count: 0,
            tui_fps: 0.0,
            warnings_dismissed: false,
            dismissed_warning_keys: HashSet::new(),

            notification_history: VecDeque::new(),
            notification_evicted_count: 0,
            notification_next_id: 0,

            knowledge_entries: Vec::new(),

            marketplace_jobs: Vec::new(),
            marketplace_selected_job: 0,
            atelier_prds: Vec::new(),
            atelier_selected_prd: 0,
            atelier_tasks_by_slug: HashMap::new(),
            job_form_editing: false,
            job_form_title: String::new(),
            job_form_type: String::new(),
            job_form_priority: String::new(),
            job_form_description: String::new(),
            job_form_focus: JobFormField::default(),
            job_assign_editing: false,
            job_assign_buffer: String::new(),
            job_progress: HashMap::new(),
            command_results: Vec::new(),

            cpu_pct_smoothed: SmoothedValue::new(METRIC_EMA_ALPHA),
            token_rate_smoothed: SmoothedValue::new(METRIC_EMA_ALPHA),
            cost_rate_smoothed: SmoothedValue::new(METRIC_EMA_ALPHA),
            last_rate_sample_at: None,
            last_token_total_sample: 0,
            last_cost_dollars_sample: 0.0,

            rev_signals: Revision::new(),
            rev_episodes: Revision::new(),
            rev_efficiency: Revision::new(),
            rev_gate_results: Revision::new(),
            rev_event_log: Revision::new(),
            unified_log_input_rev: 0,

            eviction_counters: EvictionCounters::default(),

            inbox_items: Vec::new(),
            inbox_scroll: 0,
            safety_incidents: Vec::new(),

            provider_statuses: Vec::new(),
            providers_sub_tab: 0,
            providers_detail_scroll: 0,
            providers_selected: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical phase names for the phase pipeline
// ---------------------------------------------------------------------------

/// The canonical phase names used by the orchestrator pipeline.
const CANONICAL_PHASES: &[&str] = &[
    "preflight",
    "strategist",
    "implementer",
    "compile-gate",
    "test-gate",
    "reviewing",
    "critic-review",
    "verdict",
    "committing",
];

pub(crate) fn is_online_agent_status(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done" | "completed" | "failed" | "cancelled"
    )
}

pub(crate) fn gate_pass_rate(gates: &[GateResultSummary]) -> Option<f64> {
    if gates.is_empty() {
        return None;
    }
    let passed = gates.iter().filter(|gate| gate.passed).count();
    Some(passed as f64 / gates.len() as f64)
}

pub(crate) fn snapshot_gate_pass_rate(
    gates: &[roko_core::dashboard_snapshot::GateVerdictView],
) -> Option<f64> {
    if gates.is_empty() {
        return None;
    }
    let passed = gates.iter().filter(|gate| gate.passed).count();
    Some(passed as f64 / gates.len() as f64)
}

pub(crate) fn count_online_from_files(root: &Path) -> usize {
    let jobs_dir = root.join(".roko").join("jobs");
    let Ok(entries) = std::fs::read_dir(jobs_dir) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| {
            let content = std::fs::read_to_string(entry.path()).ok()?;
            let value: serde_json::Value = serde_json::from_str(&content).ok()?;
            let status = value
                .get("status")
                .or_else(|| value.get("state"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            is_online_agent_status(status).then_some(())
        })
        .count()
}

/// Derive a minimal agent topology from the snapshot's agent map (item 41).
///
/// Each agent becomes one node.  Agents sharing a `current_plan` get a
/// bidirectional edge between them so the topology overlay shows which
/// agents are collaborating on the same plan.
pub(crate) fn derive_topology_from_agents(
    agents: &HashMap<String, roko_core::dashboard_snapshot::AgentState>,
) -> roko_core::AgentTopology {
    use roko_core::dashboard_snapshot::{AgentTopologyEdge, AgentTopologyNode};
    let nodes: Vec<AgentTopologyNode> = agents
        .values()
        .map(|a| AgentTopologyNode {
            id: a.agent_id.clone(),
            address: String::new(),
            insights_posted: 0,
            confirmations_given: 0,
            challenges_given: 0,
            total_weight: 0.0,
        })
        .collect();

    // Group agents by plan and create edges between co-plan agents.
    let mut plan_groups: HashMap<String, Vec<String>> = HashMap::new();
    for a in agents.values() {
        if !a.current_plan.is_empty() {
            plan_groups
                .entry(a.current_plan.clone())
                .or_default()
                .push(a.agent_id.clone());
        }
    }
    let mut edges = Vec::new();
    for group in plan_groups.values() {
        for (i, from) in group.iter().enumerate() {
            for to in &group[i + 1..] {
                edges.push(AgentTopologyEdge {
                    from: from.clone(),
                    to: to.clone(),
                    weight: 1,
                    edge_type: "co-plan".into(),
                });
            }
        }
    }

    roko_core::AgentTopology {
        nodes,
        edges,
        timestamp: chrono::Utc::now().timestamp() as u64,
    }
}

// ---------------------------------------------------------------------------
// TuiModel — unified data model (item 121)
// ---------------------------------------------------------------------------

/// Unified TUI data model that replaces the dual `DashboardData` + `TuiState`
/// pipeline.  Phase A: introduced alongside existing types so new tabs can
/// adopt `TuiModel` while existing tabs continue to read `TuiState` fields
/// directly.
///
/// The migration path is:
///   Phase A  — `TuiModel` wraps `TuiState` and adds unified fields.
///   Phase B  — each tab switches from `TuiState` to `TuiModel`.
///   Phase C  — `TuiState` merges into `TuiModel` and the bridge is deleted.
#[derive(Debug)]
pub struct TuiModel {
    /// Inner TuiState — kept during migration so existing tabs work unchanged.
    pub state: TuiState,
}

impl Default for TuiModel {
    fn default() -> Self {
        Self {
            state: TuiState::default(),
        }
    }
}

impl TuiModel {
    /// Create a new default model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a live `DashboardSnapshot` to the unified model.
    pub fn apply_snapshot(&mut self, snap: &roko_core::DashboardSnapshot) {
        self.state.update_from_dashboard_snapshot(snap);
    }

    /// Apply a pull-mode `DashboardData` to the unified model.
    pub fn apply_dashboard_data(&mut self, data: &DashboardData) {
        self.state.update_from_snapshot(data);
    }

    /// Whether the inspect data cache should be refreshed (5-second cadence).
    #[must_use]
    pub fn inspect_needs_refresh(&self) -> bool {
        self.state.inspect_needs_refresh()
    }

    /// Refresh the inspect data from disk files.
    pub fn refresh_inspect_data(&mut self, workdir: &Path) {
        let prev = std::mem::replace(&mut self.state.workdir, workdir.to_path_buf());
        self.state.refresh_inspect_data();
        self.state.workdir = prev;
    }
}

/// Cached data for the F7:Inspect three-panel layout (item 127).
#[derive(Debug, Clone, Default)]
pub struct InspectData {
    /// Column 1: MCP runtime status.
    pub mcp: McpRuntimeData,
    /// Column 2: Learning state metrics.
    pub learning: LearningData,
    /// Column 3: Prompt stats aggregates.
    pub prompt_stats: PromptStatsData,
}

// ---------------------------------------------------------------------------
// DreamViewCache — avoids reading journal.jsonl and archive.jsonl every frame
// ---------------------------------------------------------------------------

/// A parsed journal entry for the Dreams sub-tab display.
#[derive(Debug, Clone, Default)]
pub struct DreamJournalEntry {
    pub cycle_id: String,
    pub phase: String,
    pub summary: String,
    /// Raw line kept as fallback for entries that don't parse as JSON.
    pub raw: String,
}

/// A parsed archive entry for the Dreams sub-tab display.
#[derive(Debug, Clone, Default)]
pub struct DreamArchiveEntry {
    pub kind: String,
    pub quality_score: f64,
    pub summary: String,
    pub raw: String,
}

/// Cached content for the F7:Inspect / Dreams sub-view (sub_tab 7).
///
/// Populated by [`TuiState::refresh_dream_cache`] on the same 5-second
/// cadence as [`TuiState::refresh_inspect_data`]. The render function reads
/// these fields instead of calling `std::fs::read_to_string` directly.
#[derive(Debug, Clone, Default)]
pub struct DreamViewCache {
    /// Total number of lines in `journal.jsonl`.
    pub journal_entry_count: usize,
    /// Most recent 5 journal entries (newest first).
    pub journal_recent: Vec<DreamJournalEntry>,
    /// Total number of lines in `archive.jsonl`.
    pub archive_entry_count: usize,
    /// Most recent 5 archive entries (newest first).
    pub archive_recent: Vec<DreamArchiveEntry>,
    /// Display path for the dreams directory.
    pub dream_dir_display: String,
}

// ---------------------------------------------------------------------------
// KnowledgeHealthCache — avoids reading knowledge.jsonl every frame
// ---------------------------------------------------------------------------

/// Cached aggregate statistics for the F7:Inspect / Knowledge Health sub-view
/// (sub_tab 8).
///
/// Populated by [`TuiState::refresh_knowledge_health_cache`] on the same
/// 5-second cadence as [`TuiState::refresh_inspect_data`].
#[derive(Debug, Clone, Default)]
pub struct KnowledgeHealthCache {
    pub total: u64,
    pub transient: u64,
    pub working: u64,
    pub consolidated: u64,
    pub persistent: u64,
    pub anti_knowledge: u64,
    pub frozen: u64,
    pub avg_balance: f64,
    pub calibrated: u64,
}

// ---------------------------------------------------------------------------
// RuntimeStatusCache — avoids reading relay/lens status files every frame
// ---------------------------------------------------------------------------

/// Relay status parsed from `.roko/relay/status.json`.
#[derive(Debug, Clone, Default)]
pub struct RelayStatusCache {
    pub present: bool,
    pub connected: bool,
    pub cursor: u64,
    pub reconnect_count: u64,
}

/// Lens status parsed from `.roko/telemetry/lens-status.json` and `read_dir`.
#[derive(Debug, Clone, Default)]
pub struct LensStatusCache {
    pub lens_count: usize,
    pub lens_names: Vec<String>,
}

/// Cached status for runtime sections appended to the F6 Config view.
///
/// Populated by [`TuiState::refresh_runtime_status_cache`] on the same
/// 5-second cadence as config-items refresh. Avoids reading
/// `relay/status.json`, `telemetry/lens-status.json`, and `read_dir` on
/// every render frame.
#[derive(Debug, Clone, Default)]
pub struct RuntimeStatusCache {
    pub relay: RelayStatusCache,
    pub lens: LensStatusCache,
}

/// MCP runtime status for the F7 inspect panel.
#[derive(Debug, Clone, Default)]
pub struct McpRuntimeData {
    /// Path to the active MCP config file.
    pub config_path: String,
    /// Whether the config file exists on disk.
    pub config_exists: bool,
    /// Total registered tool count.
    pub tool_count: usize,
    /// Connected MCP server names.
    pub servers: Vec<String>,
    /// AST index: number of files indexed.
    pub index_file_count: usize,
    /// AST index: number of symbols indexed.
    pub index_symbol_count: usize,
}

/// Learning state metrics for the F7 inspect panel.
#[derive(Debug, Clone, Default)]
pub struct LearningData {
    /// Total episode count.
    pub episode_count: usize,
    /// Number of passing episodes.
    pub episodes_passed: usize,
    /// Number of failing episodes.
    pub episodes_failed: usize,
    /// Number of learned playbook rules.
    pub playbook_rule_count: usize,
    /// Routing coverage: fraction of models with non-default weights.
    pub routing_coverage_pct: f64,
    /// Gate threshold values per rung name.
    pub gate_thresholds: Vec<(String, f64)>,
}

/// Prompt statistics aggregates for the F7 inspect panel.
#[derive(Debug, Clone, Default)]
pub struct PromptStatsData {
    /// Average prompt tokens per role.
    pub tokens_per_role: Vec<(String, u64)>,
    /// Average context utilization per role (0.0 - 1.0).
    pub context_utilization: Vec<(String, f64)>,
    /// Top sections by token cost (section name, avg tokens).
    pub top_sections_by_cost: Vec<(String, u64)>,
}

impl InspectData {
    /// Load inspect data from on-disk files and current TuiState.
    pub fn load_from_workdir(workdir: &Path, tui_state: &TuiState) -> Self {
        let roko_dir = workdir.join(".roko");
        let learn_dir = roko_dir.join("learn");

        // Column 1: MCP runtime
        let mcp_config_path = workdir.join("roko.toml");
        let mcp = McpRuntimeData {
            config_path: mcp_config_path.display().to_string(),
            config_exists: mcp_config_path.exists(),
            tool_count: load_mcp_tool_count(&roko_dir),
            servers: load_mcp_servers(&roko_dir),
            index_file_count: load_index_stat(&roko_dir, "file_count"),
            index_symbol_count: load_index_stat(&roko_dir, "symbol_count"),
        };

        // Column 2: Learning state
        let (ep_total, ep_passed, ep_failed) = count_episodes(&tui_state.episodes_cache);
        let playbook_rule_count = load_playbook_rule_count(&learn_dir);
        let routing_coverage_pct = compute_routing_coverage(&tui_state.cascade_router);
        let gate_thresholds = load_gate_thresholds_summary(&learn_dir);
        let learning = LearningData {
            episode_count: ep_total,
            episodes_passed: ep_passed,
            episodes_failed: ep_failed,
            playbook_rule_count,
            routing_coverage_pct,
            gate_thresholds,
        };

        // Column 3: Prompt stats
        let (tokens_per_role, context_utilization) =
            aggregate_prompt_stats(&tui_state.efficiency_events);
        let prompt_stats = PromptStatsData {
            tokens_per_role,
            context_utilization,
            top_sections_by_cost: Vec::new(), // requires section effectiveness data
        };

        Self {
            mcp,
            learning,
            prompt_stats,
        }
    }
}

// -- InspectData helpers --

fn load_mcp_tool_count(roko_dir: &Path) -> usize {
    let stats_path = roko_dir.join("state").join("mcp-stats.json");
    std::fs::read_to_string(stats_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("tool_count")?.as_u64())
        .unwrap_or(0) as usize
}

fn load_mcp_servers(roko_dir: &Path) -> Vec<String> {
    let stats_path = roko_dir.join("state").join("mcp-stats.json");
    std::fs::read_to_string(stats_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("servers")?
                .as_array()?
                .iter()
                .map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn load_index_stat(roko_dir: &Path, key: &str) -> usize {
    let stats_path = roko_dir.join("index").join("stats.json");
    std::fs::read_to_string(stats_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(key)?.as_u64())
        .unwrap_or(0) as usize
}

fn count_episodes(episodes: &[roko_learn::episode_logger::Episode]) -> (usize, usize, usize) {
    let total = episodes.len();
    let passed = episodes.iter().filter(|e| e.success).count();
    let failed = total - passed;
    (total, passed, failed)
}

fn load_playbook_rule_count(learn_dir: &Path) -> usize {
    let path = learn_dir.join("playbook.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            // Try both array and object-with-rules-key formats.
            if let Some(arr) = v.as_array() {
                return Some(arr.len());
            }
            v.get("rules")?.as_array().map(|a| a.len())
        })
        .unwrap_or(0)
}

/// Load playbook summaries from `.roko/learn/playbooks/` (P2-05).
fn load_playbook_summaries(learn_dir: &Path) -> Vec<PlaybookSummary> {
    let playbooks_dir = learn_dir.join("playbooks");
    let entries = match std::fs::read_dir(&playbooks_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut summaries = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(pb) = serde_json::from_str::<serde_json::Value>(&contents) else {
            continue;
        };
        let id = pb
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = pb
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let goal = pb
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let step_count = pb
            .get("steps")
            .and_then(|v| v.as_array())
            .map_or(0, |a| a.len());
        let success_count = pb
            .get("success_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let failure_count = pb
            .get("failure_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = success_count + failure_count;
        let success_rate_pct = if total > 0 {
            Some(success_count as f64 / total as f64 * 100.0)
        } else {
            None
        };
        summaries.push(PlaybookSummary {
            id,
            name,
            goal,
            step_count,
            success_count,
            failure_count,
            success_rate_pct,
        });
    }
    // Sort by success_count descending.
    summaries.sort_by(|a, b| b.success_count.cmp(&a.success_count));
    summaries
}

fn compute_routing_coverage(router: &CascadeRouterState) -> f64 {
    if router.model_slugs.is_empty() {
        return 0.0;
    }
    let with_data = router
        .confidence_stats
        .values()
        .filter(|s| s.trials > 0)
        .count();
    with_data as f64 / router.model_slugs.len() as f64 * 100.0
}

fn load_gate_thresholds_summary(learn_dir: &Path) -> Vec<(String, f64)> {
    let path = learn_dir.join("gate-thresholds.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            let obj = v.as_object()?;
            Some(
                obj.iter()
                    .filter_map(|(k, val)| val.as_f64().map(|f| (k.clone(), f)))
                    .collect(),
            )
        })
        .unwrap_or_default()
}

fn aggregate_prompt_stats(
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> (Vec<(String, u64)>, Vec<(String, f64)>) {
    use std::collections::BTreeMap;
    let mut role_tokens: BTreeMap<String, (u64, u64)> = BTreeMap::new(); // (total, count)
    for ev in events {
        let entry = role_tokens.entry(ev.role.clone()).or_default();
        entry.0 += ev.input_tokens + ev.output_tokens;
        entry.1 += 1;
    }
    let tokens_per_role: Vec<(String, u64)> = role_tokens
        .iter()
        .map(|(role, (total, count))| {
            let avg = if *count > 0 { total / count } else { 0 };
            (role.clone(), avg)
        })
        .collect();
    // Context utilization: assume 200K context window for now.
    const DEFAULT_CONTEXT_WINDOW: u64 = 200_000;
    let utilization: Vec<(String, f64)> = tokens_per_role
        .iter()
        .map(|(role, avg)| (role.clone(), *avg as f64 / DEFAULT_CONTEXT_WINDOW as f64))
        .collect();
    (tokens_per_role, utilization)
}

impl TuiState {
    /// Create a new default state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a `TuiState` from a `DashboardData` snapshot.
    ///
    /// This is the primary constructor used by widget tests and the TUI app
    /// to bootstrap a full state from the snapshot data model.
    #[must_use]
    pub fn from_dashboard_data(data: &DashboardData) -> Self {
        let mut state = Self::default();
        state.update_from_snapshot(data);
        state
    }

    /// The plan set (directory under `plans/`) containing `plan_id`: the
    /// group its announced plan-set entry names, else the group disk
    /// discovery gave it. `None` for a top-level plan.
    #[must_use]
    pub fn plan_group(&self, plan_id: &str) -> Option<&str> {
        self.plan_set
            .iter()
            .find(|entry| entry.plan_id == plan_id)
            .and_then(|entry| entry.group.as_deref())
            .or_else(|| self.plan_groups.get(plan_id).and_then(Option::as_deref))
            .or_else(|| {
                self.plan_summaries
                    .iter()
                    .find(|summary| summary.id == plan_id)
                    .and_then(|summary| summary.group.as_deref())
            })
    }

    /// Why an announced plan has not started, as the plan-set scheduler
    /// holds it back: prerequisites that have not succeeded yet, else a plan
    /// it conflicts with that is running, or that comes earlier in the set
    /// and has not started. `None` for a plan that started, finished, or is
    /// waiting only for a free slot.
    #[must_use]
    pub fn plan_wait_reason(&self, plan_id: &str) -> Option<String> {
        let phase = |id: &str| match self.plans.iter().find(|plan| plan.id == id) {
            Some(plan) if plan.active => PlanPhase::Active,
            Some(plan) => plan.status,
            None => PlanPhase::Pending,
        };
        if phase(plan_id) != PlanPhase::Pending {
            return None;
        }
        let position = self
            .plan_set
            .iter()
            .position(|entry| entry.plan_id == plan_id)?;
        let entry = &self.plan_set[position];
        let unfinished: Vec<&str> = entry
            .depends_on
            .iter()
            .map(String::as_str)
            .filter(|prerequisite| !phase(prerequisite).is_done())
            .collect();
        if !unfinished.is_empty() {
            return Some(format!("waiting on {}", unfinished.join(", ")));
        }
        self.plan_set
            .iter()
            .enumerate()
            .find(|(index, other)| {
                entry.conflicts_with.contains(&other.plan_id)
                    && match phase(&other.plan_id) {
                        PlanPhase::Active => true,
                        PlanPhase::Pending => *index < position,
                        PlanPhase::Done | PlanPhase::Failed => false,
                    }
            })
            .map(|(_, other)| format!("conflicts with {}", other.plan_id))
    }

    // -- config items cache (P3.2) ------------------------------------------

    /// How often to re-parse `roko.toml` for the config view.
    const CONFIG_CACHE_TTL: Duration = Duration::from_secs(5);

    /// Return the cached config item list, rebuilding only when stale (>5 s),
    /// when `config_pending` has changed size, or on first access.
    pub fn config_items(&mut self) -> &[super::config_meta::ConfigItem] {
        let stale = self
            .config_items_refreshed_at
            .map_or(true, |t| t.elapsed() >= Self::CONFIG_CACHE_TTL);
        let pending_changed = self.config_pending.len() != self.config_items_pending_len;

        if stale || pending_changed {
            self.rebuild_config_items_cache();
        }
        &self.config_items_cache
    }

    /// Whether the config items cache should be refreshed (5-second cadence).
    #[must_use]
    pub fn config_needs_refresh(&self) -> bool {
        self.config_items_refreshed_at
            .map_or(true, |t| t.elapsed() >= Self::CONFIG_CACHE_TTL)
            || self.config_pending.len() != self.config_items_pending_len
    }

    /// Force-rebuild the config items cache (call after save or explicit invalidation).
    pub fn invalidate_config_cache(&mut self) {
        self.rebuild_config_items_cache();
    }

    fn rebuild_config_items_cache(&mut self) {
        self.config_items_cache =
            super::config_meta::build_flat_items(&self.workdir, &self.config_pending);
        // RC-4: Also refresh the runtime status cache (relay/lens) so the
        // config view render function reads cached data instead of hitting disk.
        self.refresh_runtime_status_cache();
        self.config_items_refreshed_at = Some(Instant::now());
        self.config_items_pending_len = self.config_pending.len();
    }

    // -- inspect data cache (P3.3) -------------------------------------------

    /// Whether the inspect data cache should be refreshed (5-second cadence).
    #[must_use]
    pub fn inspect_needs_refresh(&self) -> bool {
        const INSPECT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
        self.inspect_last_refresh
            .map_or(true, |t| t.elapsed() >= INSPECT_REFRESH_INTERVAL)
    }

    /// Refresh the inspect data cache from disk files and current state.
    pub fn refresh_inspect_data(&mut self) {
        self.inspect_data = InspectData::load_from_workdir(&self.workdir, self);
        // P2-05: Also refresh playbook summaries on the same cadence.
        let learn_dir = self.workdir.join(".roko").join("learn");
        self.playbook_summaries = load_playbook_summaries(&learn_dir);
        // RC-4: Refresh caches that avoid per-frame disk reads in context/config views.
        self.refresh_dream_cache();
        self.refresh_knowledge_health_cache();
        self.inspect_last_refresh = Some(Instant::now());
    }

    /// Refresh the Dream sub-view cache from `.roko/dreams/journal.jsonl` and
    /// `.roko/dreams/archive.jsonl`.
    ///
    /// Called from `refresh_inspect_data` (5-second cadence) so the render
    /// function reads pre-loaded data instead of hitting disk every frame.
    pub fn refresh_dream_cache(&mut self) {
        let dream_dir = self.workdir.join(".roko").join("dreams");
        let journal_path = dream_dir.join("journal.jsonl");
        let archive_path = dream_dir.join("archive.jsonl");

        let journal_text = std::fs::read_to_string(&journal_path).unwrap_or_default();
        let journal_lines: Vec<&str> = journal_text.lines().collect();
        let journal_entry_count = journal_lines.len();
        let journal_recent: Vec<DreamJournalEntry> = journal_lines
            .iter()
            .rev()
            .take(5)
            .map(|raw| {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
                    DreamJournalEntry {
                        cycle_id: val
                            .get("cycle_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        phase: val
                            .get("phase")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        summary: val
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        raw: String::new(),
                    }
                } else {
                    DreamJournalEntry {
                        raw: (*raw).to_string(),
                        ..Default::default()
                    }
                }
            })
            .collect();

        let archive_text = std::fs::read_to_string(&archive_path).unwrap_or_default();
        let archive_lines: Vec<&str> = archive_text.lines().collect();
        let archive_entry_count = archive_lines.len();
        let archive_recent: Vec<DreamArchiveEntry> = archive_lines
            .iter()
            .rev()
            .take(5)
            .map(|raw| {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
                    DreamArchiveEntry {
                        kind: val
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        quality_score: val
                            .get("quality_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        summary: val
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        raw: String::new(),
                    }
                } else {
                    DreamArchiveEntry {
                        raw: (*raw).to_string(),
                        ..Default::default()
                    }
                }
            })
            .collect();

        self.dream_view_cache = DreamViewCache {
            journal_entry_count,
            journal_recent,
            archive_entry_count,
            archive_recent,
            dream_dir_display: dream_dir.display().to_string(),
        };
    }

    /// Refresh the Knowledge Health cache from `.roko/knowledge.jsonl`.
    ///
    /// Called from `refresh_inspect_data` (5-second cadence).
    pub fn refresh_knowledge_health_cache(&mut self) {
        let knowledge_path = self.workdir.join(".roko").join("knowledge.jsonl");
        let text = std::fs::read_to_string(&knowledge_path).unwrap_or_default();

        let mut transient = 0_u64;
        let mut working = 0_u64;
        let mut consolidated = 0_u64;
        let mut persistent = 0_u64;
        let mut anti_knowledge = 0_u64;
        let mut frozen = 0_u64;
        let mut total_balance = 0.0_f64;
        let mut balance_count = 0_u64;
        let mut calibrated = 0_u64;
        let mut total = 0_u64;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(entry) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                continue;
            };
            total += 1;
            let tier = entry
                .get("tier")
                .and_then(|v| v.as_str())
                .unwrap_or("transient");
            match tier {
                "transient" => transient += 1,
                "working" => working += 1,
                "consolidated" => consolidated += 1,
                "persistent" => persistent += 1,
                _ => transient += 1,
            }
            if entry
                .get("anti_knowledge")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                anti_knowledge += 1;
            }
            if entry
                .get("frozen")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                frozen += 1;
            }
            if let Some(balance) = entry.get("balance").and_then(|v| v.as_f64()) {
                total_balance += balance;
                balance_count += 1;
            }
            if entry
                .get("heuristic_calibration")
                .and_then(|v| v.as_f64())
                .is_some()
            {
                calibrated += 1;
            }
        }

        let avg_balance = if balance_count > 0 {
            total_balance / balance_count as f64
        } else {
            0.0
        };

        self.knowledge_health_cache = KnowledgeHealthCache {
            total,
            transient,
            working,
            consolidated,
            persistent,
            anti_knowledge,
            frozen,
            avg_balance,
            calibrated,
        };
    }

    /// Refresh the runtime status cache for the F6 Config view's runtime sections.
    ///
    /// Reads `relay/status.json`, `telemetry/lens-status.json`, and does a
    /// `read_dir` on the telemetry dir. Called on the same 5-second cadence as
    /// config-items refresh so the render function avoids per-frame disk I/O.
    pub fn refresh_runtime_status_cache(&mut self) {
        // Relay status
        let relay_path = self.workdir.join(".roko").join("relay").join("status.json");
        let relay = if let Ok(text) = std::fs::read_to_string(&relay_path) {
            if let Ok(status) = serde_json::from_str::<serde_json::Value>(&text) {
                RelayStatusCache {
                    present: true,
                    connected: status
                        .get("connected")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    cursor: status.get("cursor").and_then(|v| v.as_u64()).unwrap_or(0),
                    reconnect_count: status
                        .get("reconnect_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                }
            } else {
                RelayStatusCache {
                    present: true,
                    ..Default::default()
                }
            }
        } else {
            RelayStatusCache::default()
        };

        // Lens status
        let telemetry_dir = self.workdir.join(".roko").join("telemetry");
        let lens_count = if telemetry_dir.exists() {
            std::fs::read_dir(&telemetry_dir)
                .map(|entries| entries.flatten().count())
                .unwrap_or(0)
        } else {
            0
        };
        let lens_status_path = telemetry_dir.join("lens-status.json");
        let lens_names: Vec<String> = std::fs::read_to_string(&lens_status_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|val| val.get("lenses").cloned())
            .and_then(|lenses| serde_json::from_value::<Vec<serde_json::Value>>(lenses).ok())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        let lens = LensStatusCache {
            lens_count,
            lens_names,
        };

        self.runtime_status_cache = RuntimeStatusCache { relay, lens };
    }

    /// Whether the cached MCP configuration should be refreshed.
    ///
    /// MCP config changes very rarely (only when roko.toml is edited), so a
    /// 30-second refresh cadence avoids unnecessary disk I/O while still
    /// picking up changes within a reasonable window.
    #[must_use]
    pub fn mcp_config_needs_refresh(&self) -> bool {
        const MCP_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
        self.mcp_config_refreshed_at
            .is_none_or(|refreshed| refreshed.elapsed() >= MCP_REFRESH_INTERVAL)
    }

    /// Refresh the dashboard MCP configuration cache from disk.
    pub fn refresh_mcp_config_view(&mut self) {
        let config_path = self.workdir.join("roko.toml");
        self.mcp_config_view = match Config::from_file(&config_path) {
            Ok(config) => {
                let Some(configured_path) = config.agent.mcp_config else {
                    self.mcp_config_view = McpConfigView::default();
                    self.mcp_config_refreshed_at = Some(Instant::now());
                    return;
                };
                let resolved_path = if configured_path.is_absolute() {
                    configured_path.clone()
                } else {
                    self.workdir.join(&configured_path)
                };
                if !resolved_path.is_file() {
                    McpConfigView {
                        configured_path: Some(configured_path),
                        resolved_path: Some(resolved_path),
                        ..McpConfigView::default()
                    }
                } else {
                    match roko_agent::mcp::McpConfig::load(&resolved_path) {
                        Ok(config) => McpConfigView {
                            configured_path: Some(configured_path),
                            resolved_path: Some(resolved_path),
                            config: Some(config),
                            error: None,
                        },
                        Err(error) => McpConfigView {
                            configured_path: Some(configured_path),
                            resolved_path: Some(resolved_path),
                            config: None,
                            error: Some(error.to_string()),
                        },
                    }
                }
            }
            Err(error) => McpConfigView {
                error: Some(format!("failed to load roko.toml: {error}")),
                ..McpConfigView::default()
            },
        };
        self.mcp_config_refreshed_at = Some(Instant::now());
    }

    /// Refresh the conductor snapshot from current alerts, diagnoses, and config.
    ///
    /// Called on a cadence (e.g. every 5 seconds) to avoid re-parsing config
    /// every frame. The snapshot captures watcher health status, recent
    /// interventions, circuit breaker state, and config thresholds.
    pub fn refresh_conductor_snapshot(&mut self) {
        let config_path = self.workdir.join("roko.toml");
        let conductor_config = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|text| toml::from_str::<roko_core::config::schema::RokoConfig>(&text).ok())
            .map(|c| c.conductor)
            .unwrap_or_default();
        self.conductor_snapshot = super::widgets::conductor_panel::build_conductor_snapshot(
            &self.conductor_alerts,
            &self.diagnoses,
            &conductor_config,
        );
        self.conductor_snapshot_refreshed_at = Some(Instant::now());
    }

    /// Whether the conductor snapshot is stale and needs a refresh.
    ///
    /// Conductor config is read from roko.toml which changes rarely; a
    /// 30-second cadence avoids per-frame disk I/O while keeping the panel
    /// reasonably up to date.
    #[must_use]
    pub fn conductor_snapshot_needs_refresh(&self) -> bool {
        self.conductor_snapshot_refreshed_at
            .map_or(true, |t| t.elapsed() > Duration::from_secs(30))
    }

    // -- aggregate queries (used by header_bar, status_bar, etc.) -----------

    /// Return (done, total) task counts summed across all plans.
    #[must_use]
    pub fn task_counts(&self) -> (usize, usize) {
        let total: usize = self.plans.iter().map(|p| p.tasks_total).sum();
        let done: usize = self.plans.iter().map(|p| p.tasks_done).sum();
        (done, total)
    }

    /// Elapsed seconds since `run_started`, or 0.0 if not set.
    #[must_use]
    pub fn elapsed_secs(&self) -> f64 {
        self.run_duration_secs.unwrap_or_else(|| {
            self.run_started
                .map(|s| s.elapsed().as_secs_f64())
                .unwrap_or(0.0)
        })
    }

    /// Update `elapsed_secs` on all active plans from their `started_at`
    /// `Instant`.
    ///
    /// Called from the animation tick so that live plan timers advance every
    /// frame without waiting for the next `DashboardSnapshot` push from the
    /// StateHub.  This eliminates the one-snapshot-period latency that caused
    /// the plan timing display to freeze during active graph-engine runs.
    pub fn tick_elapsed(&mut self) {
        for plan in self.plans.iter_mut().filter(|p| p.active) {
            if let Some(started) = plan.started_at {
                plan.elapsed_secs = started.elapsed().as_secs_f64();
            }
        }
    }

    /// Return the selected plan's spend, ceiling, and simple historical projection.
    #[must_use]
    pub fn plan_budget_summary(&self, plan: &PlanEntry) -> PlanBudgetSummary {
        let spent_usd = self.cost_per_plan.get(&plan.id).copied().unwrap_or(0.0);
        let prefix = format!("{}:", plan.id);
        let observed = self
            .cost_per_task
            .iter()
            .filter(|(key, cost)| key.starts_with(&prefix) && **cost > 0.0)
            .count();
        let remaining = plan.tasks_total.saturating_sub(observed);
        let average = if observed == 0 {
            0.0
        } else {
            spent_usd / observed as f64
        };
        let projected_remaining_usd = average * remaining as f64;
        PlanBudgetSummary {
            spent_usd,
            budget_usd: self.max_plan_budget_usd,
            projected_remaining_usd,
            projected_total_usd: spent_usd + projected_remaining_usd,
        }
    }

    /// Effective ceiling for one task; zero means unlimited.
    #[must_use]
    pub fn task_budget(&self, plan_id: &str, task_id: &str) -> f64 {
        self.task_budget_usd
            .get(&format!("{plan_id}:{task_id}"))
            .copied()
            .unwrap_or(0.0)
    }

    /// Aggregate configured ceiling represented by the header's total spend.
    #[must_use]
    pub fn aggregate_plan_budget(&self) -> f64 {
        if self.max_plan_budget_usd <= 0.0 {
            0.0
        } else {
            self.max_plan_budget_usd * self.plans.len().max(1) as f64
        }
    }

    /// Number of execution waves.
    #[must_use]
    pub fn wave_count(&self) -> usize {
        self.execution_waves.len()
    }

    /// Index of the currently selected wave.
    #[must_use]
    pub fn current_wave(&self) -> usize {
        self.selected_wave_idx
    }

    /// Count of agents with status "active" or "running".
    #[must_use]
    pub fn active_agent_count(&self) -> usize {
        self.agents.iter().filter(|a| a.active).count()
    }

    /// Collect active warnings for the persistent warning bar.
    ///
    /// Returns an empty vec when the user has dismissed all warnings (`n` key).
    /// Individual warnings dismissed via `dismissed_warning_keys` are filtered
    /// out while new warnings continue to appear.
    #[must_use]
    pub fn active_warnings(&self) -> Vec<String> {
        if self.warnings_dismissed {
            return Vec::new();
        }
        let mut candidates: Vec<(String, String)> = Vec::new(); // (key, message)

        // Disk low: warn when less than 1 GiB free and we have data.
        const LOW_DISK_THRESHOLD: u64 = 1 << 30; // 1 GiB
        if self.sys.disk_free_bytes > 0 && self.sys.disk_free_bytes < LOW_DISK_THRESHOLD {
            let free = crate::tui::widgets::header_bar::fmt_bytes_short(self.sys.disk_free_bytes);
            candidates.push(("dsk_low".into(), format!("DSK LOW: {free} free")));
        }
        // Provider unhealthy: any agent marked as failed.
        let unhealthy_count = self
            .agents
            .iter()
            .filter(|a| matches!(a.status, AgentStatus::Failed))
            .count();
        if unhealthy_count > 0 {
            candidates.push((
                "provider_unhealthy".into(),
                format!("{unhealthy_count} provider(s) unhealthy"),
            ));
        }
        // Stale snapshot: if no run is active but there are plans with active flag.
        if self.run_started.is_none() && self.plans.iter().any(|p| p.active) {
            candidates.push((
                "stale_snapshot".into(),
                "Stale snapshot: plans active but no run".to_string(),
            ));
        }
        // Budget approaching: warn when > 80% consumed.
        let budget = self.aggregate_plan_budget();
        if budget > 0.0 && self.cost_dollars / budget > 0.8 {
            let pct = (self.cost_dollars / budget * 100.0) as u32;
            candidates.push((
                "budget_high".into(),
                format!("BUDGET: ${:.2}/${budget:.2} ({pct}%)", self.cost_dollars),
            ));
        }

        candidates
            .into_iter()
            .filter(|(key, _)| !self.dismissed_warning_keys.contains(key))
            .map(|(_, msg)| msg)
            .collect()
    }

    /// Return the warning keys for currently active warnings.
    #[must_use]
    pub fn active_warning_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        const LOW_DISK_THRESHOLD: u64 = 1 << 30;
        if self.sys.disk_free_bytes > 0 && self.sys.disk_free_bytes < LOW_DISK_THRESHOLD {
            keys.push("dsk_low".into());
        }
        if self
            .agents
            .iter()
            .any(|a| matches!(a.status, AgentStatus::Failed))
        {
            keys.push("provider_unhealthy".into());
        }
        if self.run_started.is_none() && self.plans.iter().any(|p| p.active) {
            keys.push("stale_snapshot".into());
        }
        let budget = self.aggregate_plan_budget();
        if budget > 0.0 && self.cost_dollars / budget > 0.8 {
            keys.push("budget_high".into());
        }
        keys
    }

    /// Badge count for a tab. Returns `0` when the tab has nothing noteworthy,
    /// or a positive count representing the number of items the operator should
    /// be aware of on that tab.
    #[must_use]
    pub fn tab_badge(&self, tab: Tab) -> usize {
        match tab {
            // F3 Agents: number of currently running agents.
            Tab::Agents => self.active_agent_count(),
            // F2 Plans: number of failed tasks across all plans.
            Tab::Plans => self.plans.iter().map(|p| p.tasks_failed).sum(),
            // F4 Git: pending approvals (approval modal count).
            Tab::Git => usize::from(self.pending_approval.is_some()),
            // F5 Logs: recent gate failures.
            Tab::Logs => self.gate_results.iter().filter(|g| !g.passed).count(),
            // F10 Learning: number of concluded experiment winners.
            Tab::Learning => self.experiment_winners.len(),
            // F11 Providers: number of unhealthy providers.
            Tab::Providers => self
                .provider_statuses
                .iter()
                .filter(|p| !matches!(p.health, ProviderHealth::Healthy))
                .count(),
            _ => 0,
        }
    }

    /// Dynamic tab label with count badge appended when the badge is non-zero.
    ///
    /// Returns `"Plans (3)"` style strings for use in the breadcrumb bar and
    /// the active-tab indicator in the status bar. Returns the plain label
    /// (e.g. `"Dashboard"`) for tabs with no notable count.
    #[must_use]
    pub fn tab_label_with_badge(&self, tab: Tab) -> String {
        let badge = self.tab_badge(tab);
        if badge > 0 {
            format!("{} ({badge})", tab.label())
        } else {
            tab.label().to_string()
        }
    }

    /// Read the active sub-view index for a tab.
    #[must_use]
    pub fn sub_tab_for(&self, tab: Tab) -> usize {
        match tab {
            Tab::Dashboard => self.dashboard_sub_tab,
            Tab::Plans => self.plan_detail_tab,
            Tab::Agents => self.selected_agent_tab,
            Tab::Git => self.git_sub_tab,
            Tab::Logs => self.logs_sub_tab,
            Tab::Config => self.config_sub_tab,
            Tab::Inspect => self.inspect_sub_tab,
            Tab::Marketplace => self.marketplace_sub_tab,
            Tab::Atelier => self.atelier_sub_tab,
            Tab::Learning => self.learning_sub_tab,
            Tab::Providers => self.providers_sub_tab,
        }
    }

    /// Store the active sub-view index for a tab.
    pub fn set_sub_tab_for(&mut self, tab: Tab, idx: usize) {
        match tab {
            Tab::Dashboard => self.dashboard_sub_tab = idx,
            Tab::Plans => self.plan_detail_tab = idx,
            Tab::Agents => self.selected_agent_tab = idx,
            Tab::Git => self.git_sub_tab = idx,
            Tab::Logs => self.logs_sub_tab = idx,
            Tab::Config => self.config_sub_tab = idx,
            Tab::Inspect => self.inspect_sub_tab = idx,
            Tab::Marketplace => self.marketplace_sub_tab = idx,
            Tab::Atelier => self.atelier_sub_tab = idx,
            Tab::Learning => self.learning_sub_tab = idx,
            Tab::Providers => self.providers_sub_tab = idx,
        }
    }

    /// Return the current filter text.
    #[must_use]
    pub fn filter_ref(&self) -> &str {
        &self.filter
    }

    /// Return the display label for the active text input mode.
    #[must_use]
    pub const fn input_mode_label(&self) -> &'static str {
        match self.input_mode {
            InputMode::Normal => "",
            InputMode::Inject => "INJECT",
            InputMode::Filter => "FILTER",
            InputMode::LogSearch => "SEARCH",
            InputMode::AgentOutputSearch => "SEARCH",
            InputMode::PlanFilter => "PLAN FILTER",
            InputMode::Confirm | InputMode::ConfigEdit => "",
        }
    }

    pub fn update_cpu_pct(&mut self, sample: f32) -> f32 {
        let smoothed = self.cpu_pct_smoothed.update(sample as f64) as f32;
        self.sys.cpu_pct = smoothed;
        smoothed
    }
}
