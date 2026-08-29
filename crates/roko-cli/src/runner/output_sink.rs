//! Output sink abstraction for plan runner progress reporting.
//!
//! The [`RunOutputSink`] trait provides a uniform interface for receiving
//! structured progress events from the plan runner. Implementations:
//!
//! - [`StderrSink`] — writes rich inline progress to stderr (delegates to
//!   `RunnerInlineTerminal` internally, owns `AgentStreamBuffer` state).
//! - [`FormattedStderrSink`] — writes structured `[plan/task]` prefixed
//!   output to stderr with color support, agent output truncation, and
//!   progress indicators. This is the default for `roko do` / `roko plan run`.
//! - [`FanOutSink`] — forwards every event to multiple sinks in registration
//!   order, allowing machine-readable and human-readable output to coexist.
//! - [`AcpProgressSink`] — writes machine-readable progress records to stdout.
//! - [`NoopSink`] — discards all events (for testing / embedded use).
//!
//! # Design decision (Task 006)
//!
//! `AgentStreamBuffer` is moved into `StderrSink` as internal `Mutex<String>`
//! state. The trait uses `&self` throughout so it can be wrapped in
//! `Arc<dyn RunOutputSink + Send + Sync>`. Interior mutability is handled
//! via `std::sync::Mutex` inside `StderrSink`.

use std::fmt;
use std::sync::{Arc, Mutex};

use super::inline_output::RunnerInlineTerminal;
use super::types::StderrSeverity;
use crate::inline::DiffEntry;

/// Coarse gate verdict passed to the output sink.
#[derive(Debug, Clone)]
pub struct GateResultSummary {
    pub rung: u32,
    pub passed: bool,
    pub gate_name: String,
    pub summary: String,
    pub duration_ms: u64,
}

/// Token usage reported by agent runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

/// Per-plan entry within [`RunCompleteSummary`].
#[derive(Debug, Clone)]
pub struct PlanCompleteSummary {
    pub plan_id: String,
    pub completed: bool,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub tasks_failed: usize,
}

/// Per-task cost entry within [`RunCompleteSummary`].
#[derive(Debug, Clone)]
pub struct TaskCostSummary {
    pub task_id: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub agent_calls: u32,
    pub outcome: String,
}

/// Aggregate summary of a completed run, emitted before post-plan cleanup.
///
/// This is a lightweight projection of `RunReport` that avoids the output
/// sink depending on event-loop types.
#[derive(Debug, Clone)]
pub struct RunCompleteSummary {
    pub succeeded: bool,
    pub total_tasks: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_cost_usd: f64,
    pub duration_secs: u64,
    pub plans: Vec<PlanCompleteSummary>,
    pub task_costs: Vec<TaskCostSummary>,
    /// Per-task failure reasons keyed by "plan_id:task_id".
    pub failure_reasons: Vec<(String, String)>,
}

/// Structured progress events emitted by the plan runner.
///
/// Implementors receive callbacks as the runner progresses through tasks.
/// All methods have default no-op implementations so consumers can opt in
/// to only the events they care about.
pub trait RunOutputSink: Send + Sync + fmt::Debug {
    // ─── Task lifecycle ─────────────────────────────────────────────────

    /// A task is about to be dispatched to an agent.
    fn task_started(
        &self,
        _plan_id: &str,
        _task_id: &str,
        _role: &str,
        _title: &str,
        _attempt: u32,
    ) {
    }

    /// A task completed successfully (with progress counts).
    fn task_completed(
        &self,
        _plan_id: &str,
        _task_id: &str,
        _completed: usize,
        _total: usize,
        _duration_ms: u64,
    ) {
    }

    /// A task failed permanently (retries exhausted or non-retryable).
    fn task_failed(&self, _plan_id: &str, _task_id: &str, _error: &str) {}

    // ─── Agent events ───────────────────────────────────────────────────

    /// Agent process started.
    fn agent_started(
        &self,
        _plan_id: &str,
        _task_id: &str,
        _provider: &str,
        _model: &str,
        _pid: Option<u32>,
    ) {
    }

    /// A text delta was received from the agent (buffered internally).
    fn agent_text_delta(&self, _plan_id: &str, _task_id: &str, _text: &str) {}

    /// Flush buffered agent text (called before structural events).
    fn flush_agent_text(&self, _plan_id: &str, _task_id: &str) {}

    /// The agent invoked a tool.
    fn tool_call(&self, _plan_id: &str, _task_id: &str, _tool_id: &str, _tool_name: &str) {}

    /// A tool produced output.
    fn tool_output(&self, _plan_id: &str, _task_id: &str, _tool_id: &str, _output: &str) {}

    /// Token usage update from agent runtime.
    fn token_usage(&self, _plan_id: &str, _task_id: &str, _usage: TokenUsage) {}

    /// Agent turn completed.
    fn agent_turn_completed(
        &self,
        _plan_id: &str,
        _task_id: &str,
        _total_cost_usd: Option<f64>,
        _is_error: bool,
        _model: &str,
        _total_input_tokens: u64,
        _total_output_tokens: u64,
    ) {
    }

    /// Agent reported an error.
    fn agent_error(
        &self,
        _plan_id: &str,
        _task_id: &str,
        _message: &str,
        _severity: StderrSeverity,
    ) {
    }

    // ─── Gate events ────────────────────────────────────────────────────

    /// A gate rung completed with a verdict.
    fn gate_result(&self, _plan_id: &str, _task_id: &str, _result: &GateResultSummary) {}

    /// Gate failed, retrying after backoff.
    fn gate_retry(&self, _plan_id: &str, _task_id: &str, _next_attempt: u32, _cooldown_ms: u64) {}

    // ─── Warm cache ─────────────────────────────────────────────────────

    /// Cargo cache warm started.
    fn warm_cache_started(&self) {}

    /// Cargo cache warm completed.
    fn warm_cache_completed(&self, _warm_ms: u64) {}

    // ─── Diff ───────────────────────────────────────────────────────────

    /// Show a diff block for task output.
    fn diff_block(&self, _plan_id: &str, _task_id: &str, _entries: &[DiffEntry]) {}

    // ─── Plan summary ───────────────────────────────────────────────────

    /// The entire plan run finished — summary statistics.
    fn plan_summary(
        &self,
        _plan_id: &str,
        _tasks_passed: usize,
        _tasks_failed: usize,
        _total_duration_ms: u64,
    ) {
    }

    // ─── Run complete ────────────────────────────────────────────────────

    /// The entire run finished — overall summary before cleanup begins.
    ///
    /// Called immediately after the main event loop exits but *before*
    /// post-plan cleanup (dream consolidation, learning, episode compaction,
    /// filesystem GC, worktree cleanup). This gives the user immediate
    /// feedback on the run outcome while background maintenance proceeds.
    fn run_complete(&self, _summary: &RunCompleteSummary) {}

    /// A line of output was received from the agent process.
    /// Legacy compatibility method — prefer `agent_text_delta`.
    fn agent_line(&self, _plan_id: &str, _task_id: &str, _line: &str) {}
}

// ─── StderrSink ─────────────────────────────────────────────────────────────

/// Writes rich inline progress to stderr, delegating to `RunnerInlineTerminal`.
///
/// Owns an internal `AgentStreamBuffer` (as `Mutex<String>`) so that text
/// deltas can be accumulated and flushed at structural boundaries (tool calls,
/// turn completions) without exposing mutable state to callers.
pub struct StderrSink {
    inner: Mutex<RunnerInlineTerminal>,
    text_buf: Mutex<String>,
}

impl StderrSink {
    /// Create a new `StderrSink`. The internal `RunnerInlineTerminal` is
    /// always enabled.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RunnerInlineTerminal::new(true)),
            text_buf: Mutex::new(String::new()),
        }
    }

    /// Drain the last N non-empty lines from the text buffer, truncating
    /// each to `max_chars`. Returns collected lines.
    fn drain_lines(&self, max_lines: usize, max_chars: usize) -> Vec<String> {
        let mut buf = self.text_buf.lock().unwrap_or_else(|e| e.into_inner());
        if buf.trim().is_empty() {
            buf.clear();
            return Vec::new();
        }

        let lines: Vec<&str> = buf.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(max_lines);
        let drained = lines[start..]
            .iter()
            .map(|line| truncate_chars(line.trim(), max_chars))
            .filter(|line| !line.is_empty())
            .collect();
        buf.clear();
        drained
    }
}

impl Default for StderrSink {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for StderrSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StderrSink")
    }
}

impl RunOutputSink for StderrSink {
    fn task_started(&self, _plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.task_started(task_id, role, title, attempt);
    }

    fn task_completed(
        &self,
        _plan_id: &str,
        _task_id: &str,
        completed: usize,
        total: usize,
        duration_ms: u64,
    ) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.task_done(completed, total, duration_ms);
    }

    fn task_failed(&self, _plan_id: &str, _task_id: &str, error: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.task_failed(error);
    }

    fn agent_started(
        &self,
        _plan_id: &str,
        _task_id: &str,
        provider: &str,
        model: &str,
        pid: Option<u32>,
    ) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.agent_started(provider, model, pid);
    }

    fn agent_text_delta(&self, _plan_id: &str, _task_id: &str, text: &str) {
        let mut buf = self.text_buf.lock().unwrap_or_else(|e| e.into_inner());
        buf.push_str(text);
    }

    fn flush_agent_text(&self, _plan_id: &str, _task_id: &str) {
        let lines = self.drain_lines(3, 120);
        if !lines.is_empty() {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.agent_text(lines);
        }
    }

    fn tool_call(&self, _plan_id: &str, _task_id: &str, tool_id: &str, tool_name: &str) {
        // Flush buffered text before showing tool call.
        let lines = self.drain_lines(3, 120);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.agent_text(lines);
        inner.tool_call_started(tool_id, tool_name);
    }

    fn tool_output(&self, _plan_id: &str, _task_id: &str, tool_id: &str, output: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.tool_output(tool_id, output);
    }

    fn token_usage(&self, _plan_id: &str, _task_id: &str, usage: TokenUsage) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // The model field is not available here; use empty string — the
        // inline terminal accumulates per-model stats via the full
        // `agent_turn_completed` path.
        inner.token_usage(
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_tokens,
            usage.cache_write_tokens,
            "",
        );
    }

    fn agent_turn_completed(
        &self,
        _plan_id: &str,
        _task_id: &str,
        total_cost_usd: Option<f64>,
        is_error: bool,
        model: &str,
        total_input_tokens: u64,
        total_output_tokens: u64,
    ) {
        // Flush any remaining buffered text.
        let lines = self.drain_lines(3, 120);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.agent_text(lines);
        inner.agent_turn_completed(
            total_cost_usd,
            is_error,
            model,
            total_input_tokens,
            total_output_tokens,
        );
    }

    fn agent_error(&self, _plan_id: &str, _task_id: &str, message: &str, severity: StderrSeverity) {
        // Skip infra-level noise (INFO banners, empty lines) entirely.
        if severity == StderrSeverity::Infra {
            return;
        }
        // Flush any remaining buffered text.
        let lines = self.drain_lines(3, 120);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.agent_text(lines);
        inner.agent_error(message);
    }

    fn gate_result(&self, _plan_id: &str, _task_id: &str, result: &GateResultSummary) {
        use super::types::{GateCompletion, GateCompletionKind, GateVerdictSummary};
        let completion = GateCompletion {
            effect: None,
            kind: GateCompletionKind::Gate,
            attempt: None,
            plan_id: String::new(),
            task_id: String::new(),
            rung: result.rung,
            passed: result.passed,
            failure_kind: None,
            verdicts: vec![GateVerdictSummary {
                gate_name: result.gate_name.clone(),
                passed: result.passed,
                skipped: false,
                summary: result.summary.clone(),
                error_digest: None,
                failure_kind: None,
                rung_index: None,
            }],
            output: String::new(),
            duration_ms: result.duration_ms,
            selected_rungs: Vec::new(),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.gate_completed(&completion);
    }

    fn gate_retry(&self, _plan_id: &str, _task_id: &str, next_attempt: u32, cooldown_ms: u64) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.gate_retry(next_attempt, cooldown_ms);
    }

    fn warm_cache_started(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.warm_cache_started();
    }

    fn warm_cache_completed(&self, warm_ms: u64) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.warm_cache_completed(warm_ms);
    }

    fn diff_block(&self, _plan_id: &str, _task_id: &str, entries: &[DiffEntry]) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.diff_block(entries);
    }

    fn plan_summary(
        &self,
        plan_id: &str,
        tasks_passed: usize,
        tasks_failed: usize,
        total_duration_ms: u64,
    ) {
        eprintln!(
            "[{plan_id}] summary: {tasks_passed} passed, {tasks_failed} failed, \
             total {total_duration_ms}ms"
        );
    }

    fn run_complete(&self, summary: &RunCompleteSummary) {
        print_run_complete_summary(summary);
    }

    fn agent_line(&self, plan_id: &str, task_id: &str, line: &str) {
        eprintln!("[{plan_id}/{task_id}]   {line}");
    }
}

// ─── NoopSink ───────────────────────────────────────────────────────────────

/// Discards all output events. Useful for testing or embedded/library usage
/// where no user-facing output is desired.
pub struct NoopSink;

impl Default for NoopSink {
    fn default() -> Self {
        Self
    }
}

impl fmt::Debug for NoopSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NoopSink")
    }
}

impl RunOutputSink for NoopSink {}

// ─── FanOutSink ──────────────────────────────────────────────────────────────

/// Forwards each runner event to multiple output sinks in registration order.
///
/// Fan-out is synchronous because [`RunOutputSink`] callbacks are synchronous.
/// The wrapper owns no mutable event state, so composing sinks does not change
/// the buffering or flushing semantics of any child sink.
pub struct FanOutSink {
    sinks: Vec<Arc<dyn RunOutputSink>>,
}

impl FanOutSink {
    /// Compose the provided sinks. An empty collection behaves like [`NoopSink`].
    pub fn new(sinks: Vec<Arc<dyn RunOutputSink>>) -> Self {
        Self { sinks }
    }

    /// Compose two sinks without requiring call sites to construct a vector.
    pub fn pair(first: Arc<dyn RunOutputSink>, second: Arc<dyn RunOutputSink>) -> Self {
        Self::new(vec![first, second])
    }

    /// Number of child sinks receiving every callback.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    /// Whether this fan-out has no child sinks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

impl fmt::Debug for FanOutSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FanOutSink")
            .field("sink_count", &self.sinks.len())
            .finish()
    }
}

impl RunOutputSink for FanOutSink {
    fn task_started(&self, plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
        for sink in &self.sinks {
            sink.task_started(plan_id, task_id, role, title, attempt);
        }
    }

    fn task_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        completed: usize,
        total: usize,
        duration_ms: u64,
    ) {
        for sink in &self.sinks {
            sink.task_completed(plan_id, task_id, completed, total, duration_ms);
        }
    }

    fn task_failed(&self, plan_id: &str, task_id: &str, error: &str) {
        for sink in &self.sinks {
            sink.task_failed(plan_id, task_id, error);
        }
    }

    fn agent_started(
        &self,
        plan_id: &str,
        task_id: &str,
        provider: &str,
        model: &str,
        pid: Option<u32>,
    ) {
        for sink in &self.sinks {
            sink.agent_started(plan_id, task_id, provider, model, pid);
        }
    }

    fn agent_text_delta(&self, plan_id: &str, task_id: &str, text: &str) {
        for sink in &self.sinks {
            sink.agent_text_delta(plan_id, task_id, text);
        }
    }

    fn flush_agent_text(&self, plan_id: &str, task_id: &str) {
        for sink in &self.sinks {
            sink.flush_agent_text(plan_id, task_id);
        }
    }

    fn tool_call(&self, plan_id: &str, task_id: &str, tool_id: &str, tool_name: &str) {
        for sink in &self.sinks {
            sink.tool_call(plan_id, task_id, tool_id, tool_name);
        }
    }

    fn tool_output(&self, plan_id: &str, task_id: &str, tool_id: &str, output: &str) {
        for sink in &self.sinks {
            sink.tool_output(plan_id, task_id, tool_id, output);
        }
    }

    fn token_usage(&self, plan_id: &str, task_id: &str, usage: TokenUsage) {
        for sink in &self.sinks {
            sink.token_usage(plan_id, task_id, usage);
        }
    }

    fn agent_turn_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        total_cost_usd: Option<f64>,
        is_error: bool,
        model: &str,
        total_input_tokens: u64,
        total_output_tokens: u64,
    ) {
        for sink in &self.sinks {
            sink.agent_turn_completed(
                plan_id,
                task_id,
                total_cost_usd,
                is_error,
                model,
                total_input_tokens,
                total_output_tokens,
            );
        }
    }

    fn agent_error(&self, plan_id: &str, task_id: &str, message: &str, severity: StderrSeverity) {
        for sink in &self.sinks {
            sink.agent_error(plan_id, task_id, message, severity);
        }
    }

    fn gate_result(&self, plan_id: &str, task_id: &str, result: &GateResultSummary) {
        for sink in &self.sinks {
            sink.gate_result(plan_id, task_id, result);
        }
    }

    fn gate_retry(&self, plan_id: &str, task_id: &str, next_attempt: u32, cooldown_ms: u64) {
        for sink in &self.sinks {
            sink.gate_retry(plan_id, task_id, next_attempt, cooldown_ms);
        }
    }

    fn warm_cache_started(&self) {
        for sink in &self.sinks {
            sink.warm_cache_started();
        }
    }

    fn warm_cache_completed(&self, warm_ms: u64) {
        for sink in &self.sinks {
            sink.warm_cache_completed(warm_ms);
        }
    }

    fn diff_block(&self, plan_id: &str, task_id: &str, entries: &[DiffEntry]) {
        for sink in &self.sinks {
            sink.diff_block(plan_id, task_id, entries);
        }
    }

    fn plan_summary(
        &self,
        plan_id: &str,
        tasks_passed: usize,
        tasks_failed: usize,
        total_duration_ms: u64,
    ) {
        for sink in &self.sinks {
            sink.plan_summary(plan_id, tasks_passed, tasks_failed, total_duration_ms);
        }
    }

    fn run_complete(&self, summary: &RunCompleteSummary) {
        for sink in &self.sinks {
            sink.run_complete(summary);
        }
    }

    fn agent_line(&self, plan_id: &str, task_id: &str, line: &str) {
        for sink in &self.sinks {
            sink.agent_line(plan_id, task_id, line);
        }
    }
}

// ─── FormattedStderrSink ────────────────────────────────────────────────────

/// Maximum number of agent output lines before truncation kicks in.
const AGENT_OUTPUT_TRUNCATE_THRESHOLD: usize = 50;
/// Number of head lines to show when truncating.
const AGENT_OUTPUT_HEAD_LINES: usize = 10;
/// Number of tail lines to show when truncating.
const AGENT_OUTPUT_TAIL_LINES: usize = 10;

/// Writes structured `[plan/task]` prefixed progress to stderr with
/// optional ANSI color, agent output truncation, and progress indicators.
///
/// This is the default output sink for `roko do` and `roko plan run`.
/// It formats output as:
/// ```text
/// [plan-id/task-id] > Agent starting: "Add rate limiting middleware"
/// [plan-id/task-id] | Writing crates/roko-serve/src/middleware/rate_limit.rs
/// [plan-id/task-id] > Running gate: compile
/// [plan-id/task-id] + Gate passed: compile (2.3s)
/// [plan-id/task-id] x Gate failed: test -- 2 test failures
/// ```
///
/// Agent code output longer than 50 lines is truncated: first 10, last 10,
/// with an omission notice in between.
pub struct FormattedStderrSink {
    color: bool,
    text_buf: Mutex<String>,
}

impl FormattedStderrSink {
    /// Create a new sink. When `color` is true, ANSI escape codes are emitted.
    ///
    /// Callers should resolve color from CLI flags + `NO_COLOR` + `CLICOLOR`
    /// before constructing the sink.
    pub fn new(color: bool) -> Self {
        Self {
            color,
            text_buf: Mutex::new(String::new()),
        }
    }

    /// Write a formatted line to stderr.
    fn emit(&self, line: &str) {
        let mut stderr = std::io::stderr().lock();
        use std::io::Write;
        let _ = writeln!(stderr, "{line}");
    }

    /// Format a `[plan/task]` prefix.
    fn prefix(plan_id: &str, task_id: &str) -> String {
        if plan_id.is_empty() && task_id.is_empty() {
            String::new()
        } else if plan_id.is_empty() {
            format!("[{task_id}]")
        } else if task_id.is_empty() {
            format!("[{plan_id}]")
        } else {
            format!("[{plan_id}/{task_id}]")
        }
    }

    /// Drain and format accumulated agent text with truncation.
    fn drain_text(&self, plan_id: &str, task_id: &str) {
        let text = {
            let mut buf = self.text_buf.lock().unwrap_or_else(|e| e.into_inner());
            let t = buf.clone();
            buf.clear();
            t
        };

        if text.trim().is_empty() {
            return;
        }

        let pfx = Self::prefix(plan_id, task_id);
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let formatted = format_truncated_lines(&lines);

        for line in &formatted {
            let trimmed = truncate_chars(line.trim(), 120);
            if self.color {
                self.emit(&format!("{pfx} \x1b[2m| {trimmed}\x1b[0m"));
            } else {
                self.emit(&format!("{pfx} | {trimmed}"));
            }
        }
    }

    /// Emit a structural event line with an icon.
    fn emit_event(&self, plan_id: &str, task_id: &str, icon: &str, msg: &str) {
        let pfx = Self::prefix(plan_id, task_id);
        self.emit(&format!("{pfx} {icon} {msg}"));
    }

    /// Emit a success line (green when color is on).
    fn emit_pass(&self, plan_id: &str, task_id: &str, msg: &str) {
        let pfx = Self::prefix(plan_id, task_id);
        if self.color {
            self.emit(&format!("{pfx} \x1b[32m+ {msg}\x1b[0m"));
        } else {
            self.emit(&format!("{pfx} + {msg}"));
        }
    }

    /// Emit a failure line (red when color is on).
    fn emit_fail(&self, plan_id: &str, task_id: &str, msg: &str) {
        let pfx = Self::prefix(plan_id, task_id);
        if self.color {
            self.emit(&format!("{pfx} \x1b[31mx {msg}\x1b[0m"));
        } else {
            self.emit(&format!("{pfx} x {msg}"));
        }
    }

    /// Emit an in-progress line (yellow when color is on).
    fn emit_progress(&self, plan_id: &str, task_id: &str, msg: &str) {
        let pfx = Self::prefix(plan_id, task_id);
        if self.color {
            self.emit(&format!("{pfx} \x1b[33m> {msg}\x1b[0m"));
        } else {
            self.emit(&format!("{pfx} > {msg}"));
        }
    }
}

impl Default for FormattedStderrSink {
    fn default() -> Self {
        Self::new(false)
    }
}

impl fmt::Debug for FormattedStderrSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FormattedStderrSink")
    }
}

impl RunOutputSink for FormattedStderrSink {
    fn task_started(&self, plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
        let attempt_str = if attempt > 1 {
            format!(" (attempt {attempt})")
        } else {
            String::new()
        };
        let title_trunc = truncate_chars(title, 100);
        self.emit_progress(
            plan_id,
            task_id,
            &format!("Agent starting{attempt_str}: \"{title_trunc}\" [{role}]"),
        );
    }

    fn task_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        completed: usize,
        total: usize,
        duration_ms: u64,
    ) {
        self.drain_text(plan_id, task_id);
        let secs = duration_ms as f64 / 1000.0;
        self.emit_pass(
            plan_id,
            task_id,
            &format!("Task completed ({completed}/{total}) in {secs:.1}s"),
        );
    }

    fn task_failed(&self, plan_id: &str, task_id: &str, error: &str) {
        self.drain_text(plan_id, task_id);
        let first_line = error.lines().next().unwrap_or("unknown error");
        let err_trunc = truncate_chars(first_line, 120);
        self.emit_fail(plan_id, task_id, &format!("Task failed: {err_trunc}"));
    }

    fn agent_started(
        &self,
        plan_id: &str,
        task_id: &str,
        provider: &str,
        model: &str,
        pid: Option<u32>,
    ) {
        let pid_str = pid.map(|p| format!(" pid {p}")).unwrap_or_default();
        self.emit_event(
            plan_id,
            task_id,
            ">",
            &format!("Agent: {model} ({provider}{pid_str})"),
        );
    }

    fn agent_text_delta(&self, _plan_id: &str, _task_id: &str, text: &str) {
        let mut buf = self.text_buf.lock().unwrap_or_else(|e| e.into_inner());
        buf.push_str(text);
    }

    fn flush_agent_text(&self, plan_id: &str, task_id: &str) {
        self.drain_text(plan_id, task_id);
    }

    fn tool_call(&self, plan_id: &str, task_id: &str, _tool_id: &str, tool_name: &str) {
        self.drain_text(plan_id, task_id);
        self.emit_event(plan_id, task_id, ">", &format!("Tool: {tool_name}"));
    }

    fn tool_output(&self, plan_id: &str, task_id: &str, _tool_id: &str, output: &str) {
        let first = output.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            let pfx = Self::prefix(plan_id, task_id);
            let preview = truncate_chars(first, 80);
            if self.color {
                self.emit(&format!("{pfx} \x1b[2m    {preview}\x1b[0m"));
            } else {
                self.emit(&format!("{pfx}     {preview}"));
            }
        }
    }

    fn token_usage(&self, plan_id: &str, task_id: &str, usage: TokenUsage) {
        let total = usage.input_tokens + usage.output_tokens;
        if total > 0 {
            self.emit_event(
                plan_id,
                task_id,
                " ",
                &format!(
                    "Tokens: {} in / {} out (cache: {} read, {} write)",
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.cache_read_tokens,
                    usage.cache_write_tokens
                ),
            );
        }
    }

    fn agent_turn_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        total_cost_usd: Option<f64>,
        is_error: bool,
        model: &str,
        total_input_tokens: u64,
        total_output_tokens: u64,
    ) {
        self.drain_text(plan_id, task_id);
        let cost = total_cost_usd
            .map(|c| format!("${c:.4}"))
            .unwrap_or_else(|| String::from("$?.????"));
        let summary = format!(
            "Agent turn {}: {cost} | {} in / {} out | {model}",
            if is_error { "failed" } else { "complete" },
            total_input_tokens,
            total_output_tokens,
        );
        if is_error {
            self.emit_fail(plan_id, task_id, &summary);
        } else {
            self.emit_pass(plan_id, task_id, &summary);
        }
    }

    fn agent_error(&self, plan_id: &str, task_id: &str, message: &str, severity: StderrSeverity) {
        // Skip infra-level noise entirely.
        if severity == StderrSeverity::Infra {
            return;
        }
        self.drain_text(plan_id, task_id);
        let msg = truncate_chars(message, 120);
        match severity {
            StderrSeverity::Warning => {
                self.emit_progress(plan_id, task_id, &format!("Agent warning: {msg}"));
            }
            StderrSeverity::Error => {
                self.emit_fail(plan_id, task_id, &format!("Agent error: {msg}"));
            }
            StderrSeverity::Infra => unreachable!(),
        }
    }

    fn gate_result(&self, plan_id: &str, task_id: &str, result: &GateResultSummary) {
        let secs = result.duration_ms as f64 / 1000.0;
        if result.passed {
            self.emit_pass(
                plan_id,
                task_id,
                &format!("Gate passed: {} ({secs:.1}s)", result.gate_name),
            );
        } else {
            let summary_trunc = truncate_chars(&result.summary, 80);
            self.emit_fail(
                plan_id,
                task_id,
                &format!(
                    "Gate failed: {} ({secs:.1}s) -- {summary_trunc}",
                    result.gate_name
                ),
            );
        }
    }

    fn gate_retry(&self, plan_id: &str, task_id: &str, next_attempt: u32, cooldown_ms: u64) {
        let secs = cooldown_ms as f64 / 1000.0;
        self.emit_progress(
            plan_id,
            task_id,
            &format!("Gate retry: attempt {next_attempt}, backoff {secs:.1}s"),
        );
    }

    fn warm_cache_started(&self) {
        self.emit_progress("", "", "Warming cargo cache...");
    }

    fn warm_cache_completed(&self, warm_ms: u64) {
        let secs = warm_ms as f64 / 1000.0;
        self.emit_pass("", "", &format!("Cargo cache warm ({secs:.1}s)"));
    }

    fn diff_block(&self, plan_id: &str, task_id: &str, entries: &[DiffEntry]) {
        if entries.is_empty() {
            return;
        }
        let pfx = Self::prefix(plan_id, task_id);
        for entry in entries {
            let path = &entry.path;
            let adds = entry.additions;
            let dels = entry.deletions;
            if self.color {
                self.emit(&format!(
                    "{pfx} \x1b[2m  {path} \x1b[32m+{adds}\x1b[0m\x1b[2m/\x1b[31m-{dels}\x1b[0m"
                ));
            } else {
                self.emit(&format!("{pfx}   {path} +{adds}/-{dels}"));
            }
        }
    }

    fn plan_summary(
        &self,
        plan_id: &str,
        tasks_passed: usize,
        tasks_failed: usize,
        total_duration_ms: u64,
    ) {
        let secs = total_duration_ms as f64 / 1000.0;
        let total = tasks_passed + tasks_failed;
        if tasks_failed == 0 {
            self.emit_pass(
                plan_id,
                "",
                &format!("Plan complete: {tasks_passed}/{total} passed in {secs:.1}s"),
            );
        } else {
            self.emit_fail(
                plan_id,
                "",
                &format!(
                    "Plan finished: {tasks_passed} passed, {tasks_failed} failed ({secs:.1}s)"
                ),
            );
        }
    }

    fn run_complete(&self, summary: &RunCompleteSummary) {
        print_run_complete_summary(summary);
    }

    fn agent_line(&self, plan_id: &str, task_id: &str, line: &str) {
        let pfx = Self::prefix(plan_id, task_id);
        if self.color {
            self.emit(&format!("{pfx} \x1b[2m| {line}\x1b[0m"));
        } else {
            self.emit(&format!("{pfx} | {line}"));
        }
    }
}

// ─── AcpProgressSink ──────────────────────────────────────────────────────────

/// Emits `ROKO_PROGRESS: <json>` lines to stdout for structured progress parsing.
///
/// Designed to run alongside `FormattedStderrSink`. The ACP layer reads stdout
/// of CLI subprocess output and parses these prefixed JSON lines to track
/// task/agent progress without scraping human-readable stderr.
pub struct AcpProgressSink;

impl AcpProgressSink {
    pub fn new() -> Self {
        Self
    }

    /// Write a JSON progress line to stdout.
    fn emit(&self, value: &serde_json::Value) {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, "ROKO_PROGRESS: ");
        let _ = serde_json::to_writer(&mut stdout, value);
        let _ = writeln!(stdout);
    }
}

impl Default for AcpProgressSink {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AcpProgressSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AcpProgressSink")
    }
}

impl RunOutputSink for AcpProgressSink {
    fn task_started(&self, plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
        self.emit(&serde_json::json!({
            "type": "task_started",
            "plan_id": plan_id,
            "task_id": task_id,
            "role": role,
            "title": title,
            "attempt": attempt,
        }));
    }

    fn task_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        completed: usize,
        total: usize,
        duration_ms: u64,
    ) {
        self.emit(&serde_json::json!({
            "type": "task_completed",
            "plan_id": plan_id,
            "task_id": task_id,
            "completed": completed,
            "total": total,
            "duration_ms": duration_ms,
        }));
    }

    fn task_failed(&self, plan_id: &str, task_id: &str, error: &str) {
        self.emit(&serde_json::json!({
            "type": "task_failed",
            "plan_id": plan_id,
            "task_id": task_id,
            "error": error,
        }));
    }

    fn agent_started(
        &self,
        plan_id: &str,
        task_id: &str,
        provider: &str,
        model: &str,
        pid: Option<u32>,
    ) {
        self.emit(&serde_json::json!({
            "type": "agent_started",
            "plan_id": plan_id,
            "task_id": task_id,
            "provider": provider,
            "model": model,
            "pid": pid,
        }));
    }

    fn tool_call(&self, plan_id: &str, task_id: &str, tool_id: &str, tool_name: &str) {
        self.emit(&serde_json::json!({
            "type": "tool_call",
            "plan_id": plan_id,
            "task_id": task_id,
            "tool_id": tool_id,
            "tool": tool_name,
        }));
    }

    fn agent_turn_completed(
        &self,
        plan_id: &str,
        task_id: &str,
        total_cost_usd: Option<f64>,
        is_error: bool,
        model: &str,
        total_input_tokens: u64,
        total_output_tokens: u64,
    ) {
        self.emit(&serde_json::json!({
            "type": "agent_turn_completed",
            "plan_id": plan_id,
            "task_id": task_id,
            "total_cost_usd": total_cost_usd,
            "is_error": is_error,
            "model": model,
            "total_input_tokens": total_input_tokens,
            "total_output_tokens": total_output_tokens,
        }));
    }

    fn gate_result(&self, plan_id: &str, task_id: &str, result: &GateResultSummary) {
        self.emit(&serde_json::json!({
            "type": "gate_result",
            "plan_id": plan_id,
            "task_id": task_id,
            "rung": result.rung,
            "passed": result.passed,
            "gate_name": result.gate_name,
            "summary": result.summary,
            "duration_ms": result.duration_ms,
        }));
    }

    fn plan_summary(
        &self,
        plan_id: &str,
        tasks_passed: usize,
        tasks_failed: usize,
        total_duration_ms: u64,
    ) {
        self.emit(&serde_json::json!({
            "type": "plan_summary",
            "plan_id": plan_id,
            "tasks_passed": tasks_passed,
            "tasks_failed": tasks_failed,
            "total_duration_ms": total_duration_ms,
        }));
    }

    fn run_complete(&self, summary: &RunCompleteSummary) {
        self.emit(&serde_json::json!({
            "type": "run_complete",
            "succeeded": summary.succeeded,
            "total_tasks": summary.total_tasks,
            "tasks_completed": summary.tasks_completed,
            "tasks_failed": summary.tasks_failed,
            "total_cost_usd": summary.total_cost_usd,
            "duration_secs": summary.duration_secs,
            "plans": summary.plans.iter().map(|p| serde_json::json!({
                "plan_id": p.plan_id,
                "completed": p.completed,
                "tasks_completed": p.tasks_completed,
                "tasks_total": p.tasks_total,
                "tasks_failed": p.tasks_failed,
            })).collect::<Vec<_>>(),
        }));
    }
}

/// Add machine-readable ACP progress output while preserving the selected
/// human-facing sink. When `enabled` is false, returns `human_sink` unchanged.
pub fn with_acp_progress_sink(
    human_sink: Arc<dyn RunOutputSink>,
    enabled: bool,
) -> Arc<dyn RunOutputSink> {
    if enabled {
        Arc::new(FanOutSink::pair(
            human_sink,
            Arc::new(AcpProgressSink::new()),
        ))
    } else {
        human_sink
    }
}

/// Interpret the optional `ROKO_ACP_PROGRESS` value using the producer
/// protocol's exact opt-in contract. Only the literal value `1` enables it.
#[must_use]
pub fn is_acp_progress_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

// ─── Shared formatting ─────────────────────────────────────────────────────

/// Format a `DashboardEvent` (from SSE or local state hub) into a single
/// human-readable line suitable for terminal output.
///
/// Returns `None` for events that are not interesting for CLI streaming
/// (e.g. bulk data refreshes like `MarketplaceJobsUpdated`).
///
/// This function is used by both `FormattedStderrSink` (for local events
/// mapped to DashboardEvent) and the SSE client (for remote events).
pub fn format_dashboard_event(
    event: &roko_core::dashboard_snapshot::DashboardEvent,
    color: bool,
) -> Option<String> {
    use roko_core::dashboard_snapshot::DashboardEvent;

    let (pfx, icon, msg) = match event {
        DashboardEvent::PlanStarted { plan_id } => {
            (format!("[{plan_id}]"), ">", format!("Plan started"))
        }
        DashboardEvent::PlanCompleted { plan_id, success } => {
            let outcome = if *success { "completed" } else { "failed" };
            let icon = if *success { "+" } else { "x" };
            (format!("[{plan_id}]"), icon, format!("Plan {outcome}"))
        }
        DashboardEvent::TaskStarted {
            plan_id,
            task_id,
            title,
            phase,
        } => {
            let t = if title.is_empty() {
                String::new()
            } else {
                format!(": \"{title}\"")
            };
            (
                format!("[{plan_id}/{task_id}]"),
                ">",
                format!("Task started{t} [{phase}]"),
            )
        }
        DashboardEvent::TaskCompleted {
            plan_id,
            task_id,
            outcome,
        } => {
            let icon = if outcome == "pass" || outcome == "success" {
                "+"
            } else {
                "x"
            };
            (
                format!("[{plan_id}/{task_id}]"),
                icon,
                format!("Task {outcome}"),
            )
        }
        DashboardEvent::TaskPhaseChanged {
            plan_id,
            task_id,
            old_phase,
            new_phase,
        } => (
            format!("[{plan_id}/{task_id}]"),
            ">",
            format!("Phase: {old_phase} -> {new_phase}"),
        ),
        DashboardEvent::AgentSpawned {
            agent_id,
            role,
            model,
            ..
        } => (
            format!("[{agent_id}]"),
            ">",
            format!("Agent spawned: {role} ({model})"),
        ),
        DashboardEvent::AgentOutput {
            agent_id, content, ..
        } => {
            let preview = content.lines().next().unwrap_or("").trim();
            if preview.is_empty() {
                return None;
            }
            let trunc = truncate_chars(preview, 100);
            (format!("[{agent_id}]"), "|", trunc)
        }
        DashboardEvent::AgentCompleted { agent_id, .. } => {
            (format!("[{agent_id}]"), "+", format!("Agent completed"))
        }
        DashboardEvent::GateResult {
            plan_id,
            task_id,
            gate,
            passed,
        } => {
            let (icon, word) = if *passed {
                ("+", "passed")
            } else {
                ("x", "failed")
            };
            (
                format!("[{plan_id}/{task_id}]"),
                icon,
                format!("Gate {word}: {gate}"),
            )
        }
        DashboardEvent::PhaseTransition { plan_id, from, to } => (
            format!("[{plan_id}]"),
            ">",
            format!("Phase: {from} -> {to}"),
        ),
        DashboardEvent::Error { message } => {
            let trunc = truncate_chars(message, 120);
            (String::new(), "x", format!("Error: {trunc}"))
        }
        DashboardEvent::EventLogEntry {
            event_type,
            plan_id,
            task_id,
            message,
            ..
        } => {
            let scope = if plan_id.is_empty() && task_id.is_empty() {
                String::new()
            } else if task_id.is_empty() {
                format!("[{plan_id}]")
            } else {
                format!("[{plan_id}/{task_id}]")
            };
            (scope, ">", format!("{event_type}: {message}"))
        }
        DashboardEvent::JobExecutionStarted {
            job_id,
            job_type,
            agent_id,
        } => (
            format!("[{job_id}]"),
            ">",
            format!("Job started: {job_type} (agent {agent_id})"),
        ),
        DashboardEvent::JobProgress {
            job_id,
            percent,
            message,
        } => (
            format!("[{job_id}]"),
            ">",
            format!("Job {percent}%: {message}"),
        ),
        DashboardEvent::PaymentReceived {
            feed_id,
            protocol,
            amount_korai,
            payer,
            payee,
        } => (
            format!("[feed:{feed_id}]"),
            "+",
            format!("Payment {amount_korai:.4} KORAI via {protocol}: {payer} -> {payee}"),
        ),
        DashboardEvent::SettlementCompleted {
            protocol,
            batch_size,
            total_korai,
        } => (
            format!("[settlement:{protocol}]"),
            "+",
            format!("Settled {batch_size} payment(s), {total_korai:.4} KORAI total"),
        ),
        DashboardEvent::ProjectionUpdated {
            projection_id,
            version,
            source_lens,
        } => (
            format!("[projection:{projection_id}]"),
            ">",
            format!("Updated to v{version} by {source_lens}"),
        ),
        // Bulk data refresh events are not useful for CLI streaming.
        DashboardEvent::EfficiencyEvent { .. }
        | DashboardEvent::Diagnosis { .. }
        | DashboardEvent::ExperimentWinnersUpdated { .. }
        | DashboardEvent::CFactorTrendUpdated { .. }
        | DashboardEvent::EpisodeRecorded { .. }
        | DashboardEvent::TaskOutputAppended { .. }
        | DashboardEvent::CascadeRouterUpdated { .. }
        | DashboardEvent::GateThresholdsUpdated { .. }
        | DashboardEvent::MarketplaceJobsUpdated { .. }
        | DashboardEvent::AtelierPrdsUpdated { .. }
        | DashboardEvent::KnowledgeEntriesUpdated { .. }
        | DashboardEvent::EfficiencyTrendUpdated { .. }
        | DashboardEvent::FeedTick { .. }
        | DashboardEvent::FeedAgentOnline { .. }
        | DashboardEvent::FeedAgentOffline { .. }
        | DashboardEvent::InboxItemReceived { .. }
        | DashboardEvent::InboxApprove { .. }
        | DashboardEvent::InboxReject { .. }
        | DashboardEvent::InboxDefer { .. }
        | DashboardEvent::InboxDismiss { .. }
        | DashboardEvent::ChainBlock { .. }
        | DashboardEvent::ChainTx { .. }
        | DashboardEvent::ChainContractEvent { .. }
        | DashboardEvent::AgentHeartbeat { .. }
        | DashboardEvent::GateRungStarted { .. }
        | DashboardEvent::AffectUpdated { .. }
        | DashboardEvent::AgentTopologyUpdated { .. } => return None,
    };

    let line = if pfx.is_empty() {
        format!("{icon} {msg}")
    } else {
        format!("{pfx} {icon} {msg}")
    };

    if color {
        // Colorize by icon type.
        let colored = match icon {
            "+" => format!("\x1b[32m{line}\x1b[0m"),
            "x" => format!("\x1b[31m{line}\x1b[0m"),
            ">" => format!("\x1b[33m{line}\x1b[0m"),
            "|" => format!("\x1b[2m{line}\x1b[0m"),
            _ => line,
        };
        Some(colored)
    } else {
        Some(line)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Apply the 50-line truncation rule: if lines exceed the threshold,
/// show the first `AGENT_OUTPUT_HEAD_LINES`, an omission notice, and
/// the last `AGENT_OUTPUT_TAIL_LINES`.
pub(crate) fn format_truncated_lines(lines: &[&str]) -> Vec<String> {
    if lines.len() <= AGENT_OUTPUT_TRUNCATE_THRESHOLD {
        return lines.iter().map(|s| (*s).to_string()).collect();
    }

    let omitted = lines.len() - AGENT_OUTPUT_HEAD_LINES - AGENT_OUTPUT_TAIL_LINES;
    let mut result: Vec<String> = lines[..AGENT_OUTPUT_HEAD_LINES]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    result.push(format!("... ({omitted} lines omitted)"));
    result.extend(
        lines[lines.len() - AGENT_OUTPUT_TAIL_LINES..]
            .iter()
            .map(|s| (*s).to_string()),
    );
    result
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

// ─── Shared summary printer ──────────────────────────────────────────────────

/// Print a human-readable run-complete summary to stderr.
///
/// Used by both `StderrSink` and `FormattedStderrSink` so the format is
/// consistent regardless of which sink is active. Matches the output
/// previously produced by the `plan.rs` and `do_cmd.rs` CLI callers.
fn print_run_complete_summary(summary: &RunCompleteSummary) {
    eprintln!(
        "\n\u{25b8} Plan complete: {}/{} tasks, ${:.2}, {}s",
        summary.tasks_completed, summary.total_tasks, summary.total_cost_usd, summary.duration_secs,
    );
    for p in &summary.plans {
        let status = if p.completed { "\u{2713}" } else { "\u{2717}" };
        eprintln!(
            "  {status} {} \u{2014} {}/{} tasks",
            p.plan_id, p.tasks_completed, p.tasks_total,
        );
    }
    // Per-task cost breakdown.
    if !summary.task_costs.is_empty() {
        eprintln!("\n  Task costs:");
        eprintln!(
            "  {:.<24} {:>8} {:>8} {:>9} {:>6} {:>6}",
            "task", "tok_in", "tok_out", "cost", "calls", "result"
        );
        for tc in &summary.task_costs {
            // Flag zero-token failed tasks as orphaned so the user knows
            // dispatch never occurred (e.g. worktree or provider error).
            if tc.tokens_in == 0
                && tc.tokens_out == 0
                && tc.agent_calls == 0
                && (tc.outcome == "orphaned" || tc.outcome == "failed" || tc.outcome == "error")
            {
                eprintln!(
                    "  {:.<24} {:>8} {:>8} {:>9} {:>6} FAILED (orphaned \u{2014} no dispatch occurred)",
                    tc.task_id, "-", "-", "-", "-",
                );
            } else {
                eprintln!(
                    "  {:.<24} {:>8} {:>8} ${:>7.4} {:>6} {:>6}",
                    tc.task_id,
                    tc.tokens_in,
                    tc.tokens_out,
                    tc.cost_usd,
                    tc.agent_calls,
                    tc.outcome,
                );
            }
        }
    }
    // Failure details.
    if !summary.failure_reasons.is_empty() {
        eprintln!("\nFailure details:");
        for (key, reason) in &summary.failure_reasons {
            if reason.contains('\n') {
                eprintln!("  \u{2717} {key}:");
                for line in reason.lines() {
                    eprintln!("    {line}");
                }
            } else {
                eprintln!("  \u{2717} {key}: {reason}");
            }
        }
        eprintln!("\nhint: check .roko/roko.log for full failure output");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct RecordingSink {
        name: &'static str,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingSink {
        fn record(&self, event: &str) {
            self.events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(format!("{}:{event}", self.name));
        }
    }

    impl RunOutputSink for RecordingSink {
        fn task_started(
            &self,
            _plan_id: &str,
            _task_id: &str,
            _role: &str,
            _title: &str,
            _attempt: u32,
        ) {
            self.record("task_started");
        }

        fn agent_text_delta(&self, _plan_id: &str, _task_id: &str, _text: &str) {
            self.record("agent_text_delta");
        }

        fn gate_result(&self, _plan_id: &str, _task_id: &str, _result: &GateResultSummary) {
            self.record("gate_result");
        }

        fn plan_summary(
            &self,
            _plan_id: &str,
            _tasks_passed: usize,
            _tasks_failed: usize,
            _total_duration_ms: u64,
        ) {
            self.record("plan_summary");
        }
    }

    #[test]
    fn fan_out_sink_forwards_callbacks_in_registration_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let first: Arc<dyn RunOutputSink> = Arc::new(RecordingSink {
            name: "first",
            events: Arc::clone(&events),
        });
        let second: Arc<dyn RunOutputSink> = Arc::new(RecordingSink {
            name: "second",
            events: Arc::clone(&events),
        });
        let sink = FanOutSink::pair(first, second);
        let gate = GateResultSummary {
            rung: 1,
            passed: true,
            gate_name: "test".to_string(),
            summary: "ok".to_string(),
            duration_ms: 5,
        };

        sink.task_started("p", "t", "implementer", "title", 1);
        sink.agent_text_delta("p", "t", "chunk");
        sink.gate_result("p", "t", &gate);
        sink.plan_summary("p", 1, 0, 10);

        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());
        assert_eq!(
            *events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            vec![
                "first:task_started",
                "second:task_started",
                "first:agent_text_delta",
                "second:agent_text_delta",
                "first:gate_result",
                "second:gate_result",
                "first:plan_summary",
                "second:plan_summary",
            ]
        );
    }

    #[test]
    fn empty_fan_out_sink_is_a_noop() {
        let sink = FanOutSink::new(Vec::new());
        sink.task_failed("p", "t", "error");
        assert!(sink.is_empty());
        assert_eq!(format!("{sink:?}"), "FanOutSink { sink_count: 0 }");
    }

    #[test]
    fn acp_progress_composition_preserves_or_wraps_human_sink() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let disabled = with_acp_progress_sink(
            Arc::new(RecordingSink {
                name: "human",
                events: Arc::clone(&events),
            }),
            false,
        );
        disabled.task_started("p", "t", "role", "title", 1);
        assert!(!format!("{disabled:?}").contains("FanOutSink"));

        let enabled = with_acp_progress_sink(
            Arc::new(RecordingSink {
                name: "human",
                events,
            }),
            true,
        );
        assert_eq!(format!("{enabled:?}"), "FanOutSink { sink_count: 2 }");
    }

    #[test]
    fn acp_progress_requires_exact_opt_in_value() {
        assert!(is_acp_progress_enabled(Some("1")));
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some(" 1"),
            Some("1 "),
        ] {
            assert!(!is_acp_progress_enabled(value));
        }
    }

    #[test]
    fn noop_sink_in_arc_does_not_panic() {
        let sink: Arc<dyn RunOutputSink> = Arc::new(NoopSink);
        sink.task_started("plan-1", "task-1", "implementer", "Build feature X", 1);
        sink.agent_text_delta("plan-1", "task-1", "hello ");
        sink.agent_text_delta("plan-1", "task-1", "world\n");
        sink.flush_agent_text("plan-1", "task-1");
        sink.tool_call("plan-1", "task-1", "tc-1", "Read");
        sink.tool_output("plan-1", "task-1", "tc-1", "file contents...");
        sink.token_usage(
            "plan-1",
            "task-1",
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 10,
                cache_write_tokens: 5,
            },
        );
        sink.agent_turn_completed(
            "plan-1",
            "task-1",
            Some(0.01),
            false,
            "claude-sonnet-4-6",
            100,
            50,
        );
        sink.gate_result(
            "plan-1",
            "task-1",
            &GateResultSummary {
                rung: 0,
                passed: true,
                gate_name: "compile".to_string(),
                summary: "ok".to_string(),
                duration_ms: 1200,
            },
        );
        sink.gate_retry("plan-1", "task-1", 2, 5000);
        sink.warm_cache_started();
        sink.warm_cache_completed(1500);
        sink.diff_block("plan-1", "task-1", &[]);
        sink.task_completed("plan-1", "task-1", 3, 10, 5000);
        sink.task_failed("plan-1", "task-1", "compile error");
        sink.plan_summary("plan-1", 9, 1, 60000);
    }

    #[test]
    fn stderr_sink_can_be_constructed() {
        let sink = StderrSink::new();
        // Just verify it doesn't panic on construction.
        assert_eq!(format!("{sink:?}"), "StderrSink");
    }

    // ─── FormattedStderrSink tests ───────────────────────────────────

    #[test]
    fn formatted_sink_in_arc_does_not_panic() {
        let sink: Arc<dyn RunOutputSink> = Arc::new(FormattedStderrSink::new(false));
        sink.task_started("plan-1", "task-1", "implementer", "Build feature X", 1);
        sink.agent_started(
            "plan-1",
            "task-1",
            "claude",
            "claude-sonnet-4-6",
            Some(1234),
        );
        sink.agent_text_delta("plan-1", "task-1", "hello ");
        sink.agent_text_delta("plan-1", "task-1", "world\n");
        sink.flush_agent_text("plan-1", "task-1");
        sink.tool_call("plan-1", "task-1", "tc-1", "Read");
        sink.tool_output("plan-1", "task-1", "tc-1", "file contents...");
        sink.token_usage(
            "plan-1",
            "task-1",
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 10,
                cache_write_tokens: 5,
            },
        );
        sink.agent_turn_completed("plan-1", "task-1", Some(0.01), false, "sonnet", 100, 50);
        sink.agent_error("plan-1", "task-1", "test error", StderrSeverity::Error);
        sink.gate_result(
            "plan-1",
            "task-1",
            &GateResultSummary {
                rung: 0,
                passed: true,
                gate_name: "compile".to_string(),
                summary: "ok".to_string(),
                duration_ms: 1200,
            },
        );
        sink.gate_result(
            "plan-1",
            "task-1",
            &GateResultSummary {
                rung: 1,
                passed: false,
                gate_name: "test".to_string(),
                summary: "2 test failures".to_string(),
                duration_ms: 3400,
            },
        );
        sink.gate_retry("plan-1", "task-1", 2, 5000);
        sink.warm_cache_started();
        sink.warm_cache_completed(1500);
        sink.diff_block("plan-1", "task-1", &[]);
        sink.task_completed("plan-1", "task-1", 3, 10, 5000);
        sink.task_failed("plan-1", "task-1", "compile error");
        sink.plan_summary("plan-1", 9, 1, 60000);
    }

    #[test]
    fn formatted_sink_debug_name() {
        let sink = FormattedStderrSink::new(false);
        assert_eq!(format!("{sink:?}"), "FormattedStderrSink");
    }

    #[test]
    fn formatted_sink_prefix_formats() {
        assert_eq!(FormattedStderrSink::prefix("p", "t"), "[p/t]");
        assert_eq!(FormattedStderrSink::prefix("p", ""), "[p]");
        assert_eq!(FormattedStderrSink::prefix("", "t"), "[t]");
        assert_eq!(FormattedStderrSink::prefix("", ""), "");
    }

    // ─── Truncation tests ────────────────────────────────────────────

    #[test]
    fn truncation_short_input_unchanged() {
        let lines: Vec<&str> = (0..10)
            .map(|i| match i {
                0 => "line-0",
                1 => "line-1",
                2 => "line-2",
                3 => "line-3",
                4 => "line-4",
                5 => "line-5",
                6 => "line-6",
                7 => "line-7",
                8 => "line-8",
                _ => "line-9",
            })
            .collect();
        let result = format_truncated_lines(&lines);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0], "line-0");
        assert_eq!(result[9], "line-9");
    }

    #[test]
    fn truncation_at_threshold_unchanged() {
        let strs: Vec<String> = (0..50).map(|i| format!("line-{i}")).collect();
        let lines: Vec<&str> = strs.iter().map(|s| s.as_str()).collect();
        let result = format_truncated_lines(&lines);
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn truncation_over_threshold_truncates() {
        let strs: Vec<String> = (0..60).map(|i| format!("line-{i}")).collect();
        let lines: Vec<&str> = strs.iter().map(|s| s.as_str()).collect();
        let result = format_truncated_lines(&lines);
        // 10 head + 1 omission + 10 tail = 21
        assert_eq!(result.len(), 21);
        assert_eq!(result[0], "line-0");
        assert_eq!(result[9], "line-9");
        assert!(result[10].contains("40 lines omitted"));
        assert_eq!(result[11], "line-50");
        assert_eq!(result[20], "line-59");
    }

    // ─── format_dashboard_event tests ────────────────────────────────

    #[test]
    fn format_event_plan_started() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::PlanStarted {
            plan_id: "my-plan".to_string(),
        };
        let line = format_dashboard_event(&event, false).unwrap();
        assert!(line.contains("[my-plan]"));
        assert!(line.contains("Plan started"));
    }

    #[test]
    fn format_event_gate_result_pass() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::GateResult {
            plan_id: "p".to_string(),
            task_id: "t".to_string(),
            gate: "compile".to_string(),
            passed: true,
        };
        let line = format_dashboard_event(&event, false).unwrap();
        assert!(line.contains("[p/t]"));
        assert!(line.contains("Gate passed: compile"));
    }

    #[test]
    fn format_event_gate_result_fail() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::GateResult {
            plan_id: "p".to_string(),
            task_id: "t".to_string(),
            gate: "test".to_string(),
            passed: false,
        };
        let line = format_dashboard_event(&event, false).unwrap();
        assert!(line.contains("Gate failed: test"));
    }

    #[test]
    fn format_event_color_has_ansi() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::PlanCompleted {
            plan_id: "p".to_string(),
            success: true,
        };
        let colored = format_dashboard_event(&event, true).unwrap();
        assert!(colored.contains("\x1b[32m"), "expected green ANSI");
        let plain = format_dashboard_event(&event, false).unwrap();
        assert!(!plain.contains("\x1b["), "expected no ANSI escapes");
    }

    #[test]
    fn format_event_skips_bulk_events() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::CascadeRouterUpdated {
            snapshot_json: "{}".to_string(),
        };
        assert!(format_dashboard_event(&event, false).is_none());
    }

    #[test]
    fn format_event_error() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::Error {
            message: "something broke".to_string(),
        };
        let line = format_dashboard_event(&event, false).unwrap();
        assert!(line.contains("Error: something broke"));
    }

    #[test]
    fn format_event_projection_updated() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let event = DashboardEvent::ProjectionUpdated {
            projection_id: "cost_meter".to_string(),
            version: 3,
            source_lens: "cost-monitor".to_string(),
        };
        let line = format_dashboard_event(&event, false).unwrap();
        assert!(line.contains("[projection:cost_meter]"));
        assert!(line.contains("Updated to v3 by cost-monitor"));
    }

    #[test]
    fn format_event_payment_and_settlement_are_visible() {
        use roko_core::dashboard_snapshot::DashboardEvent;
        let payment = DashboardEvent::PaymentReceived {
            feed_id: "prices".into(),
            protocol: "x402".into(),
            amount_korai: 1.25,
            payer: "agent-a".into(),
            payee: "agent-b".into(),
        };
        let payment_line = format_dashboard_event(&payment, false).unwrap();
        assert!(payment_line.contains("[feed:prices]"));
        assert!(payment_line.contains("1.2500 KORAI"));

        let settlement = DashboardEvent::SettlementCompleted {
            protocol: "mpp".into(),
            batch_size: 3,
            total_korai: 4.5,
        };
        let settlement_line = format_dashboard_event(&settlement, false).unwrap();
        assert!(settlement_line.contains("[settlement:mpp]"));
        assert!(settlement_line.contains("3 payment(s)"));
    }
}
