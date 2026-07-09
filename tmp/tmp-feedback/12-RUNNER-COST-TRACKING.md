# Runner Cost Tracking: Full System Design

## 1. Current State of Cost Tracking

### 1.1 Data Captured Today

Cost data flows through four parallel subsystems, each capturing overlapping but
incomplete slices of the same underlying token/cost information:

#### A. `RunState` (per-task, in-memory)

**File**: `crates/roko-cli/src/runner/state.rs`

The runner's mutable state accumulates token counts per task and per run:

```rust
// Per-task (reset between tasks)
pub tokens_in: u64,
pub tokens_out: u64,
pub cache_read_tokens: u64,
pub cache_write_tokens: u64,
pub cost_usd: f64,
pub task_agent_calls: u32,

// Per-run (cumulative)
pub total_tokens_in: u64,
pub total_tokens_out: u64,
pub total_cost_usd: f64,
pub total_agent_calls: usize,
pub plan_costs: HashMap<String, f64>,
```

These fields are populated by `handle_agent_event()` in
`crates/roko-cli/src/runner/agent_events.rs`:
- `AgentEvent::TokenUsage` increments `tokens_in`, `tokens_out`,
  `cache_read_tokens`, `cache_write_tokens`
- `AgentEvent::TurnCompleted` overwrites `cost_usd` with the authoritative
  provider-reported cost

At task boundaries, per-task fields are folded into `total_*` fields and reset.

#### B. `AgentEfficiencyEvent` (per-turn, JSONL)

**File**: `crates/roko-learn/src/efficiency.rs`
**Storage**: `.roko/learn/efficiency.jsonl`

The richest per-turn snapshot. 30+ fields including:
- Identity: `agent_id`, `role`, `backend`, `model`, `plan_id`, `task_id`, `attempt_id`
- Token accounting: `input_tokens`, `output_tokens`, `reasoning_tokens`,
  `cache_read_tokens`, `cache_write_tokens`
- Cost: `cost_usd`, `cost_usd_without_cache`
- Prompt composition: per-section `PromptSectionMeta` with token attribution
- Tool utilization: per-call `ToolCallMeta` with duration, result tokens, success
- Timing: `wall_time_ms`, `time_to_first_token_ms`, `was_warm_start`
- Outcome: `gate_passed`, `gate_errors`, `model_used`, `frequency`

Emitted by orchestrate.rs after each agent turn completes and gate runs.
Subject to JSONL rotation at 10 MiB (`crates/roko-learn/src/jsonl_rotation.rs`).

#### C. `Episode` (per-task, JSONL)

**File**: `crates/roko-cli/src/runtime_feedback/episodes.rs`
**Storage**: `.roko/episodes.jsonl`

The `EpisodeSink` converts `FeedbackEvent::TaskCompleted` into an `Episode`:
```rust
episode.usage = Usage {
    input_tokens: outcome.tokens_in,
    output_tokens: outcome.tokens_out,
    cost_usd: outcome.cost_usd,
    wall_ms: outcome.duration_ms,
};
episode.backend = outcome.provider.clone();
episode.model = outcome.model.clone();
```

Episodes carry HDC fingerprints and feed into the cascade router and knowledge store.

#### D. `run-ledger.jsonl` (per-run, JSONL)

**File**: `crates/roko-cli/src/runner/event_loop.rs` (append helpers at line 4474)
**Storage**: `.roko/state/run-ledger.jsonl`

A best-effort audit trail recording lifecycle events:
- `task_started`: `{plan_id, task_id, timestamp_ms}`
- `task_completed`: `{plan_id, task_id, passed, duration_ms, timestamp_ms}`
- `task_failed`: `{plan_id, task_id, reason, timestamp_ms}`
- `gate_outcome`: `{plan_id, task_id, rung, passed, verdicts_count, duration_ms}`
- `run_summary`: `{run_id, started_at_ms, phase_transitions, agent_outcomes, gate_runs}`

The ledger does NOT include token counts, costs, or model identity per task.

#### E. `RunLedger` (in-memory, typed)

**File**: `crates/roko-runtime/src/run_ledger.rs`

A typed in-memory counterpart to the JSONL ledger. Its `AgentOutcome::Completed`
variant carries `TokenUsage { input_tokens, output_tokens, total_tokens, cost_usd }`,
but this is only used by the `WorkflowEngine` path (`roko run`), NOT by the plan
runner (`roko plan run`). The plan runner creates a `RunLedger` instance but only
calls `record_phase_transition` and `record_gate_run` -- never `record_agent_completed`.

#### F. `RunReport` (per-run, in-memory)

**File**: `crates/roko-cli/src/runner/event_loop.rs` (line 72)

The final report returned to the CLI:
```rust
pub struct RunReport {
    pub plans: Vec<PlanReport>,
    pub total_tasks: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_cost_usd: f64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_agent_calls: usize,
    pub duration: Duration,
    pub failure_reasons: HashMap<String, String>,
}
```

Contains run-level totals but NO per-task cost breakdown. No `task_costs` field.

### 1.2 Provider-Level Token Extraction

Each provider extracts tokens differently:

**Claude CLI** (`crates/roko-agent/src/provider/claude_cli/stream.rs`):
- `ClaudeUsage { input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens }`
- Emitted as `AgentRuntimeEvent::TokenUsage` from both `assistant` messages and `result` events
- `total_cost_usd` comes from the `result` event's top-level field

**Anthropic API** (`crates/roko-agent/src/provider/anthropic_api.rs`):
- Extracts `usage.input_tokens`, `usage.output_tokens`,
  `usage.cache_read_input_tokens`, `usage.cache_creation_input_tokens` from response JSON
- Maps to `TokenUsage { input_tokens, output_tokens, total_tokens, cost_usd }`

**OpenAI-compat** (`crates/roko-agent/src/provider/openai_compat.rs`):
- Extracts from `usage.prompt_tokens` and `usage.completion_tokens`
- Cost is estimated from token counts (not provider-reported)

### 1.3 Budget Enforcement Today

Two budget checks exist in the event loop:

1. **Per-turn budget** (line 627): `config.max_turn_usd` -- kills the agent mid-run
   if a single turn exceeds the limit
2. **Per-plan budget** (line 2940): `config.max_plan_usd` -- checked before spawning
   a new agent; if exceeded, the plan is aborted

Both use `RunState.cost_usd` and `RunState.plan_costs` respectively. There is no
per-run budget, per-session budget, or budget warning threshold.

---

## 2. Gaps Analysis

### 2.1 Token Counts Available But Not Captured

| Where | What's Available | What's Missing |
|-------|-----------------|----------------|
| `run-ledger.jsonl` task_completed entries | Duration, pass/fail | Token counts, cost, model, provider |
| `run-ledger.jsonl` run_summary | Agent outcome count | Total cost, total tokens |
| `RunReport` | Run-level totals | Per-task breakdown (`task_costs: Vec<TaskCostReport>`) |
| `PlanReport` | Task counts, gate results | Per-task cost, model, tokens |
| `RunLedger.record_agent_completed()` | Method exists with full signature | Never called from plan runner event loop |
| `RunState` at task completion | All fields populated | Not harvested into a per-task cost record |
| `AgentOutcome` (dispatch) | `tokens_in`, `tokens_out`, `cost_usd` | Not connected to run-ledger JSONL entries |

### 2.2 Costs: Estimated vs Actual

| Source | Type | Notes |
|--------|------|-------|
| Claude CLI `total_cost_usd` | **Actual** | Provider-reported, authoritative |
| Anthropic API response | **Actual** | Computed from provider pricing |
| OpenAI-compat | **Estimated** | Token count * hardcoded rate |
| Ollama/local models | **Zero** | No cost data emitted |
| Gate execution | **Not tracked** | Gate commands (cargo test, clippy) have wall-clock time but no cost |
| Tool execution | **Not tracked** | Tool calls have `duration_ms` in `ToolCallMeta` but no cost attribution |
| Enrichment phase | **Estimated** | `EnrichmentPhaseSummary.estimated_cost_usd` -- pre-execution estimate only |
| Prompt assembly | **Not tracked** | System prompt builder token counts are computed but not recorded as cost |

### 2.3 Missing Aggregation and Reporting

1. **No per-task cost in CLI output**: `roko plan run` prints a summary with total
   cost but no task-by-task breakdown
2. **No `--json` cost array**: The `RunReport` has no `task_costs` field for machine
   consumption
3. **No cost history**: Past runs are not queryable -- the run-ledger JSONL has no
   cost data and efficiency events require post-processing
4. **No cost comparison**: No way to compare cost between runs, models, or strategies
5. **No budget warnings**: Budget enforcement is binary (kill vs proceed) with no
   warning at 80% or 90% thresholds
6. **Duplicate `run_summary`**: `persist_run_ledger()` is called both inside the
   `all_plans_terminal` check (line 1536) AND after the event loop exits (line 1548),
   producing two summary entries per successful run
7. **Unbounded `run-ledger.jsonl`**: Not subject to JSONL rotation; grows without limit
8. **No gate cost attribution**: Gates run cargo commands that consume CPU/time but
   this is not attributed to the task's total cost

---

## 3. Complete Cost Tracking Design

### 3.1 Data Model

#### CostEvent -- atomic cost observation

Every cost-producing action emits one `CostEvent`. This is the write-once unit of
cost accounting.

```rust
// crates/roko-learn/src/cost.rs (new file)

use serde::{Deserialize, Serialize};

/// Atomic cost observation emitted at a single point in the execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEvent {
    /// Unique event id (UUID v7 for temporal ordering).
    pub event_id: String,
    /// ISO-8601 UTC timestamp.
    pub timestamp: String,
    /// Run id this event belongs to.
    pub run_id: String,
    /// Plan id.
    pub plan_id: String,
    /// Task id within the plan.
    pub task_id: String,
    /// Dispatch attempt number (1-based).
    pub attempt: u32,
    /// What produced this cost.
    pub source: CostSource,
    /// Model slug (empty for gate/tool costs).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Provider label (empty for gate/tool costs).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provider: String,
    /// Input tokens consumed.
    pub input_tokens: u64,
    /// Output tokens produced.
    pub output_tokens: u64,
    /// Cache read tokens (subset of input).
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Cache write tokens.
    #[serde(default)]
    pub cache_write_tokens: u64,
    /// Reasoning/thinking tokens (subset of output, for models that report it).
    #[serde(default)]
    pub reasoning_tokens: u64,
    /// Cost in USD. Zero for local models or gate/tool costs.
    pub cost_usd: f64,
    /// Whether this cost is provider-reported (actual) or estimated.
    pub cost_kind: CostKind,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// What produced the cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSource {
    /// LLM agent dispatch (the main cost driver).
    AgentDispatch,
    /// Gate execution (cargo test, clippy, compile).
    Gate { gate_name: String },
    /// Tool call within an agent turn (Read, Write, Bash, etc.).
    ToolCall { tool_name: String },
    /// Enrichment LLM call (context gathering before dispatch).
    Enrichment { step: String },
    /// Prompt assembly token counting (no monetary cost, but token budget).
    PromptAssembly,
}

/// Whether cost is actual (provider-reported) or estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostKind {
    /// Provider reported this cost authoritatively.
    Actual,
    /// Cost estimated from token counts and rate tables.
    Estimated,
    /// No cost data available (local model, gate, tool).
    Unknown,
}
```

#### TaskCostReport -- per-task cost summary

Aggregated from `CostEvent`s at task completion. This is the missing type
identified in the original gap analysis.

```rust
/// Per-task cost summary, computed when a task reaches terminal state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCostReport {
    /// Plan id.
    pub plan_id: String,
    /// Task id.
    pub task_id: String,
    /// Number of dispatch attempts for this task.
    pub attempts: u32,
    /// Model used for the final (or only) attempt.
    pub model: String,
    /// Provider used for the final attempt.
    pub provider: String,
    /// Total input tokens across all attempts.
    pub tokens_in: u64,
    /// Total output tokens across all attempts.
    pub tokens_out: u64,
    /// Total cache read tokens.
    pub cache_read_tokens: u64,
    /// Total cache write tokens.
    pub cache_write_tokens: u64,
    /// Total cost in USD across all attempts (agent + enrichment).
    pub cost_usd: f64,
    /// Cost of gate execution for this task (wall-time based, not monetary).
    pub gate_duration_ms: u64,
    /// Number of agent calls (dispatch attempts).
    pub agent_calls: u32,
    /// Wall-clock duration from task start to terminal state.
    pub duration_ms: u64,
    /// Task outcome.
    pub outcome: TaskCostOutcome,
    /// Cost breakdown by source.
    pub breakdown: TaskCostBreakdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCostOutcome {
    Passed,
    Failed,
    Skipped,
    BudgetExceeded,
}

/// Per-source cost breakdown within a task.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskCostBreakdown {
    /// Cost from LLM agent dispatches.
    pub agent_cost_usd: f64,
    /// Cost from enrichment LLM calls.
    pub enrichment_cost_usd: f64,
    /// Wall-time cost of gate runs in milliseconds.
    pub gate_duration_ms: u64,
    /// Number of gate runs.
    pub gate_runs: u32,
}
```

#### RunCostReport -- per-run cost summary

```rust
/// Per-run cost summary, computed when all plans are terminal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunCostReport {
    /// Run id.
    pub run_id: String,
    /// ISO-8601 start time.
    pub started_at: String,
    /// Wall-clock duration.
    pub duration_ms: u64,
    /// Per-task cost reports, ordered by completion time.
    pub task_costs: Vec<TaskCostReport>,
    /// Per-plan cost subtotals.
    pub plan_costs: Vec<PlanCostSummary>,
    /// Run-level totals.
    pub totals: RunCostTotals,
    /// Budget status at run completion.
    pub budget: BudgetStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanCostSummary {
    pub plan_id: String,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub cost_usd: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunCostTotals {
    pub cost_usd: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub agent_calls: usize,
    pub gate_runs: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    /// Cost per successful task (total_cost / tasks_completed).
    pub cost_per_pass: f64,
    /// Model breakdown: model slug -> (cost, tokens, calls).
    pub by_model: Vec<ModelCostEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCostEntry {
    pub model: String,
    pub provider: String,
    pub cost_usd: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub calls: usize,
    pub pass_rate: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// Configured per-plan budget (0 = unlimited).
    pub per_plan_limit_usd: f64,
    /// Configured per-turn budget (0 = unlimited).
    pub per_turn_limit_usd: f64,
    /// Whether any plan was aborted due to budget.
    pub any_plan_budget_exceeded: bool,
    /// Whether any turn was killed due to budget.
    pub any_turn_budget_exceeded: bool,
    /// Per-plan spend vs limit.
    pub plan_spend: Vec<PlanBudgetEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanBudgetEntry {
    pub plan_id: String,
    pub spent_usd: f64,
    pub limit_usd: f64,
    pub pct_used: f64,
}
```

#### SessionCostReport -- cross-run aggregate

```rust
/// Cross-run cost summary for a time window or session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCostReport {
    /// Time window start (ISO-8601).
    pub from: String,
    /// Time window end (ISO-8601).
    pub to: String,
    /// Number of runs in the window.
    pub run_count: usize,
    /// Total cost.
    pub total_cost_usd: f64,
    /// Average cost per run.
    pub avg_cost_per_run: f64,
    /// Average cost per successful task.
    pub avg_cost_per_pass: f64,
    /// Model breakdown across all runs.
    pub by_model: Vec<ModelCostEntry>,
    /// Cost trend: per-day totals.
    pub daily_totals: Vec<DailyCostEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyCostEntry {
    pub date: String,      // YYYY-MM-DD
    pub cost_usd: f64,
    pub runs: usize,
    pub tasks: usize,
}
```

### 3.2 Capture Points

Every place in the code that should emit cost data, mapped to the specific file
and function where the capture should be added:

#### Point 1: Provider Response Parsing (tokens in/out per call)

**Already captured.** Each provider emits `AgentRuntimeEvent::TokenUsage` and
`AgentRuntimeEvent::TurnCompleted` which flow through `handle_agent_event()` into
`RunState`.

No changes needed here -- this is the data source for all downstream aggregation.

#### Point 2: Agent Turn Completion -> CostEvent

**File**: `crates/roko-cli/src/runner/agent_events.rs`, inside the
`AgentEvent::TurnCompleted` arm (line 122)

After updating `RunState`, emit a `CostEvent`:
```rust
AgentEvent::TurnCompleted { .. } => {
    // ... existing state updates ...

    // NEW: emit CostEvent
    if let Some(cost_sink) = cost_sink {
        cost_sink.record(CostEvent {
            event_id: uuid::Uuid::now_v7().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            run_id: state.run_id().to_string(),
            plan_id: state.plan_id.clone(),
            task_id: state.current_task.clone(),
            attempt: state.task_agent_calls,
            source: CostSource::AgentDispatch,
            model: state.agent_model.clone(),
            provider: state.agent_provider.clone(),
            input_tokens: state.tokens_in,
            output_tokens: state.tokens_out,
            cache_read_tokens: state.cache_read_tokens,
            cache_write_tokens: state.cache_write_tokens,
            reasoning_tokens: 0, // TODO: extract from provider
            cost_usd: state.cost_usd,
            cost_kind: if state.cost_usd > 0.0 { CostKind::Actual } else { CostKind::Unknown },
            duration_ms: state.task_started_at.elapsed().as_millis() as u64,
        });
    }
}
```

#### Point 3: Gate Completion -> CostEvent

**File**: `crates/roko-cli/src/runner/event_loop.rs`, inside the gate completion
handler (line ~859)

After recording gate outcome in run ledger, emit gate cost:
```rust
// After existing: ledger.record_gate_run(...)
if let Some(ref cost_sink) = cost_sink {
    cost_sink.record(CostEvent {
        source: CostSource::Gate { gate_name: completion.verdicts.first()
            .map(|v| v.gate_name.clone()).unwrap_or_default() },
        cost_usd: 0.0,
        cost_kind: CostKind::Unknown,
        duration_ms: completion.duration_ms,
        // ... other fields from context ...
    });
}
```

#### Point 4: Task Completion -> TaskCostReport Harvest

**File**: `crates/roko-cli/src/runner/event_loop.rs`, at task completion (line ~1027)
and task failure (line ~1271)

This is the critical missing step. When a task reaches terminal state (pass or fail),
harvest `RunState` fields into a `TaskCostReport`:

```rust
// At gate pass (line ~1027):
state.task_completed();
let task_cost = TaskCostReport {
    plan_id: completion.plan_id.clone(),
    task_id: completion.task_id.clone(),
    attempts: state.task_agent_calls,
    model: state.agent_model.clone(),
    provider: state.agent_provider.clone(),
    tokens_in: state.tokens_in,
    tokens_out: state.tokens_out,
    cache_read_tokens: state.cache_read_tokens,
    cache_write_tokens: state.cache_write_tokens,
    cost_usd: state.cost_usd,
    gate_duration_ms: completion.duration_ms,
    agent_calls: state.task_agent_calls,
    duration_ms: state.task_started_at.elapsed().as_millis() as u64,
    outcome: TaskCostOutcome::Passed,
    breakdown: TaskCostBreakdown {
        agent_cost_usd: state.cost_usd,
        enrichment_cost_usd: 0.0, // TODO: track separately
        gate_duration_ms: completion.duration_ms,
        gate_runs: 1,
    },
};
task_costs.push(task_cost);

// Also record in the run ledger JSONL with cost data:
append_ledger_entry(&paths.run_ledger_jsonl, "task_completed", &serde_json::json!({
    "plan_id": completion.plan_id,
    "task_id": completion.task_id,
    "passed": true,
    "duration_ms": completion.duration_ms,
    "model": state.agent_model,
    "provider": state.agent_provider,
    "tokens_in": state.tokens_in,
    "tokens_out": state.tokens_out,
    "cache_read_tokens": state.cache_read_tokens,
    "cost_usd": state.cost_usd,
    "agent_calls": state.task_agent_calls,
    "timestamp_ms": now_ms,
}));
```

The same pattern applies at the task failure path (line ~1271).

#### Point 5: Run Completion -> RunCostReport

**File**: `crates/roko-cli/src/runner/event_loop.rs`, in `build_report()` (line 4739)

Extend `RunReport` to include per-task costs:

```rust
pub struct RunReport {
    // ... existing fields ...
    pub task_costs: Vec<TaskCostReport>,  // NEW
    pub cost_report: Option<RunCostReport>,  // NEW: full cost report
}
```

#### Point 6: Enrichment Phase Cost

**File**: `crates/roko-cli/src/orchestrate.rs`, in `EnrichmentRuntimeClient::record_usage()`
(line ~1612)

The enrichment client already accumulates `cost_usd` in `EnrichmentRunStats`. This
should emit a `CostEvent` with `source: CostSource::Enrichment`.

#### Point 7: RunLedger.record_agent_completed() -- Wire the Existing Method

**File**: `crates/roko-cli/src/runner/event_loop.rs`, at task completion

The typed `RunLedger` has `record_agent_completed()` with a full `TokenUsage`
parameter. It is never called from the plan runner. Add the call alongside the
JSONL append:

```rust
if let Some(ref mut ledger) = run_ledger {
    ledger.record_agent_completed(
        "implementer",           // role
        &state.agent_output,     // output (truncated)
        output_files.len() as u32,
        "",                      // requested model
        &state.agent_model,      // final model
        Some(state.agent_provider.clone()),
        roko_core::foundation::TokenUsage {
            input_tokens: state.tokens_in,
            output_tokens: state.tokens_out,
            total_tokens: state.tokens_in + state.tokens_out,
            cost_usd: state.cost_usd,
        },
    );
}
```

### 3.3 Storage

#### Cost Event Log

**Path**: `.roko/learn/cost-events.jsonl`

One line per `CostEvent`, append-only. Subject to the same JSONL rotation as
efficiency events (10 MiB threshold, 5 rotated files).

Format:
```json
{"event_id":"...","timestamp":"2026-05-05T12:00:00Z","run_id":"run-abc","plan_id":"plan-1","task_id":"task-001","attempt":1,"source":"agent_dispatch","model":"claude-sonnet-4-6","provider":"claude_cli","input_tokens":12500,"output_tokens":3200,"cache_read_tokens":8000,"cache_write_tokens":0,"reasoning_tokens":0,"cost_usd":0.0847,"cost_kind":"actual","duration_ms":45230}
```

#### Run Cost Report

**Path**: `.roko/state/run-costs/{run_id}.json`

One JSON file per completed run. Written atomically at run completion alongside
the existing `executor.json` snapshot. Contains the full `RunCostReport`.

#### Task Cost Ledger (within run-ledger.jsonl)

The existing `run-ledger.jsonl` entries for `task_completed` and `task_failed` should
be extended with cost fields (model, provider, tokens, cost_usd) as shown in
Point 4 above. This enriches the audit trail without adding a new file.

#### Rotation Policy

| File | Rotation | Retention |
|------|----------|-----------|
| `cost-events.jsonl` | 10 MiB, 5 rotated files | ~50 MiB total |
| `run-costs/{run_id}.json` | Per-file (one per run) | Last 100 runs |
| `run-ledger.jsonl` | 10 MiB, 5 rotated files (NEW) | ~50 MiB total |

`run-ledger.jsonl` must be added to JSONL rotation. Currently it uses raw
`OpenOptions::append()` with no size check.

### 3.4 Reporting

#### CLI Output: `roko plan run`

After run completion, print a per-task cost table:

```
Task costs:
  plan-1/task-001 (claude-sonnet-4-6)  12,500 in / 3,200 out  $0.0847  45.2s  PASS
  plan-1/task-002 (claude-sonnet-4-6)   8,300 in / 1,800 out  $0.0523  32.1s  PASS
  plan-1/task-003 (claude-sonnet-4-6)  15,100 in / 4,500 out  $0.1204  58.7s  FAIL
  ─────────────────────────────────────────────────────────────────────────
  Total: 3 tasks, 2 passed, $0.2574, 35,900 in / 9,500 out, 2m 16s
```

#### CLI Output: `--json` mode

Add `task_costs` array to the JSON output:
```json
{
  "task_costs": [
    {
      "plan_id": "plan-1",
      "task_id": "task-001",
      "model": "claude-sonnet-4-6",
      "provider": "claude_cli",
      "tokens_in": 12500,
      "tokens_out": 3200,
      "cost_usd": 0.0847,
      "duration_ms": 45230,
      "outcome": "passed",
      "attempts": 1
    }
  ]
}
```

#### CLI Command: `roko learn costs`

**File**: `crates/roko-cli/src/commands/learn.rs`

Read `cost-events.jsonl` and aggregate:
```
Cost summary (last 7 days):
  Runs: 12
  Total: $4.82
  By model:
    claude-sonnet-4-6    $3.21  (66.6%)  142 calls  89% pass
    claude-haiku-3-5     $0.87  (18.0%)   89 calls  72% pass
    gpt-4o               $0.74  (15.4%)   34 calls  91% pass
  By role:
    Implementer          $3.89  (80.7%)  avg $0.12/task
    Reviewer             $0.93  (19.3%)  avg $0.03/task
  Cost/pass: $0.19
  Cache savings: $1.47 (23.3%)
```

#### API Endpoints

**File**: `crates/roko-serve/src/routes/learning/mod.rs`

Existing routes to extend:
- `GET /api/learn/costs` -- already exists, should return `RunCostReport` for the
  latest run plus `SessionCostReport` for the configurable time window
- `GET /api/learn/cost-tiers` -- already exists, should include per-model cost breakdown

New routes:
- `GET /api/learn/costs/history?days=7` -- daily cost trend
- `GET /api/learn/costs/run/{run_id}` -- specific run's `RunCostReport`
- `GET /api/learn/costs/task/{plan_id}/{task_id}` -- specific task's cost events

#### Dashboard Widgets

**File**: `crates/roko-cli/src/tui/views/`

TUI tab additions (F5 or dedicated cost tab):
- Cost sparkline: last N runs' total cost
- Model cost pie chart: cost breakdown by model
- Budget gauge: per-plan spend vs limit
- Cost-per-pass trend: rolling average

### 3.5 Budget Enforcement

#### Current Enforcement

Two enforcement points exist:

1. `max_turn_usd` in `RunConfig` -- checked per turn in the event loop (line 627)
2. `max_plan_usd` in `RunConfig` -- checked before agent spawn (line 2940)

#### Proposed Additions

**Configuration** (`roko.toml`):
```toml
[budget]
# Existing
max_turn_usd = 1.0
max_plan_usd = 10.0
# New
max_run_usd = 50.0           # Total across all plans in one `plan run`
max_session_usd = 200.0      # Rolling 24h window
warn_pct = 0.8               # Emit warning at 80% of any limit
hard_stop_pct = 1.0           # Kill at 100% (default behavior)
```

**Enforcement points**:

| Check | Where | Action |
|-------|-------|--------|
| Per-turn | `event_loop.rs` line 627 | Kill agent, emit `BudgetExceeded` |
| Per-plan | `event_loop.rs` line 2940 | Skip plan, mark failed |
| Per-run (NEW) | `event_loop.rs` after `state.task_completed()` | Break event loop |
| Warning (NEW) | Same locations | Log warning, TUI alert, continue |

**Implementation in event loop**:
```rust
// After state.accrue_task_cost() in the turn completion handler:
let run_limit = config.max_run_usd;
if run_limit > 0.0 {
    let run_spent = state.total_cost_usd;
    let pct = run_spent / run_limit;
    if pct >= config.hard_stop_pct.unwrap_or(1.0) {
        warn!(spent = run_spent, limit = run_limit, "run budget exceeded");
        // Terminate all remaining plans
        break;
    } else if pct >= config.warn_pct.unwrap_or(0.8) && !state.budget_warning_emitted {
        warn!(spent = run_spent, limit = run_limit, pct = pct, "approaching run budget");
        tui.warning(&format!("budget: ${run_spent:.2} / ${run_limit:.2} ({:.0}%)", pct * 100.0));
        state.budget_warning_emitted = true;
    }
}
```

---

## 4. Integration with Existing Systems

### 4.1 RunLedger <-> Efficiency Events

**Current**: These are independent. The run-ledger JSONL records lifecycle events
(task started/completed/failed) without cost data. Efficiency events record cost
data without lifecycle context.

**Design**: The `CostEvent` bridges them. At task completion:
1. `TaskCostReport` is constructed from `RunState` (harvest step)
2. A `CostEvent` is appended to `cost-events.jsonl`
3. The existing `AgentEfficiencyEvent` continues to be emitted (it has the
   prompt/tool detail that `CostEvent` omits)
4. The `run-ledger.jsonl` entry gains cost fields

The join key is `(run_id, plan_id, task_id, attempt)` -- present in all three.

### 4.2 TaskCostReport <-> CascadeRouter

**Current**: The cascade router receives feedback via `FeedbackEvent::TaskCompleted`
which carries an `AgentOutcome` with `tokens_in`, `tokens_out`, `cost_usd`. The
router's LinUCB bandit uses cost as one signal for model selection.

**Design**: `TaskCostReport` provides richer cost data that can improve routing:

```rust
// In crates/roko-cli/src/runtime_feedback/routing.rs
// When recording a routing observation, include cost-per-pass data:
fn build_routing_observation(
    task_cost: &TaskCostReport,
    routing_context: &RoutingContext,
) -> RoutingObservation {
    RoutingObservation {
        model: task_cost.model.clone(),
        context: routing_context.clone(),
        reward: compute_cost_adjusted_reward(task_cost),
        cost_usd: task_cost.cost_usd,
        tokens: task_cost.tokens_in + task_cost.tokens_out,
    }
}

fn compute_cost_adjusted_reward(tc: &TaskCostReport) -> f64 {
    let base = if tc.outcome == TaskCostOutcome::Passed { 1.0 } else { 0.0 };
    // Penalize expensive successes, reward cheap successes
    let cost_penalty = (tc.cost_usd * 10.0).min(0.5); // cap penalty at 0.5
    (base - cost_penalty).max(0.0)
}
```

### 4.3 Cost Data <-> Model Selection

**Current**: `CascadeRouter` picks models based on historical pass rate per
`RoutingContext`. Cost is not a routing signal.

**Design**: Add cost awareness to model selection:

1. **RoleCostProfile** (already exists in `efficiency.rs`) should be loaded at
   dispatch time and used to estimate task cost before selecting a model
2. **CascadeRouter** should incorporate `cost_per_pass` into the bandit reward:
   a cheap model that passes is better than an expensive model that passes
3. **FrequencyCostProfile** should gate model selection: a `Gamma` frequency
   (low priority) task should prefer cheaper models

This connects through the existing `RoutingContext` mechanism -- no new
plumbing needed, just enriching the reward function.

---

## 5. Specific Code Changes

### 5.1 New Module: `crates/roko-learn/src/cost.rs`

```rust
// Contains: CostEvent, CostSource, CostKind, TaskCostReport,
// TaskCostOutcome, TaskCostBreakdown, RunCostReport, PlanCostSummary,
// RunCostTotals, ModelCostEntry, BudgetStatus, PlanBudgetEntry,
// SessionCostReport, DailyCostEntry
//
// Plus: CostEventWriter (JSONL appender with rotation)
// Plus: aggregate_task_cost(events: &[CostEvent]) -> TaskCostReport
// Plus: aggregate_run_cost(task_costs: &[TaskCostReport]) -> RunCostReport
// Plus: aggregate_session_cost(run_costs: &[RunCostReport]) -> SessionCostReport
```

Add to `crates/roko-learn/src/lib.rs`:
```rust
pub mod cost;
```

### 5.2 Extend RunState

**File**: `crates/roko-cli/src/runner/state.rs`

```rust
impl RunState {
    // Add field:
    pub task_costs: Vec<TaskCostReport>,
    pub budget_warning_emitted: bool,

    // Add method:
    pub fn harvest_task_cost(
        &mut self,
        plan_id: &str,
        task_id: &str,
        outcome: TaskCostOutcome,
        gate_duration_ms: u64,
    ) -> TaskCostReport {
        let report = TaskCostReport {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempts: self.task_agent_calls,
            model: self.agent_model.clone(),
            provider: self.agent_provider.clone(),
            tokens_in: self.tokens_in,
            tokens_out: self.tokens_out,
            cache_read_tokens: self.cache_read_tokens,
            cache_write_tokens: self.cache_write_tokens,
            cost_usd: self.cost_usd,
            gate_duration_ms,
            agent_calls: self.task_agent_calls,
            duration_ms: self.task_started_at.elapsed().as_millis() as u64,
            outcome,
            breakdown: TaskCostBreakdown {
                agent_cost_usd: self.cost_usd,
                enrichment_cost_usd: 0.0,
                gate_duration_ms,
                gate_runs: 1,
            },
        };
        self.task_costs.push(report.clone());
        report
    }
}
```

### 5.3 Extend RunReport

**File**: `crates/roko-cli/src/runner/event_loop.rs`

```rust
pub struct RunReport {
    // ... existing fields ...
    pub task_costs: Vec<TaskCostReport>,  // NEW
}

fn build_report(executor: &ParallelExecutor, plans: &[Plan], state: &RunState) -> RunReport {
    // ... existing logic ...
    RunReport {
        // ... existing fields ...
        task_costs: state.task_costs.clone(),  // NEW
    }
}
```

### 5.4 Wire TaskCostReport at Task Completion

**File**: `crates/roko-cli/src/runner/event_loop.rs`

At the gate-passed path (line ~1027):
```rust
state.task_completed();
let task_cost = state.harvest_task_cost(
    &completion.plan_id,
    &completion.task_id,
    TaskCostOutcome::Passed,
    completion.duration_ms,
);
// Include cost data in the ledger entry
append_ledger_entry(&paths.run_ledger_jsonl, "task_completed", &serde_json::json!({
    "plan_id": completion.plan_id,
    "task_id": completion.task_id,
    "passed": true,
    "duration_ms": completion.duration_ms,
    "model": task_cost.model,
    "provider": task_cost.provider,
    "tokens_in": task_cost.tokens_in,
    "tokens_out": task_cost.tokens_out,
    "cost_usd": task_cost.cost_usd,
    "agent_calls": task_cost.agent_calls,
    "timestamp_ms": now_ms,
}));
```

At the gate-failed terminal path (line ~1271):
```rust
let task_cost = state.harvest_task_cost(
    &completion.plan_id,
    &completion.task_id,
    TaskCostOutcome::Failed,
    completion.duration_ms,
);
```

### 5.5 Print Task Costs in CLI Output

**File**: `crates/roko-cli/src/commands/plan.rs` (where `plan run` prints results)

```rust
fn print_task_costs(task_costs: &[TaskCostReport]) {
    if task_costs.is_empty() {
        return;
    }
    println!("\nTask costs:");
    for tc in task_costs {
        let outcome_label = match tc.outcome {
            TaskCostOutcome::Passed => "PASS",
            TaskCostOutcome::Failed => "FAIL",
            TaskCostOutcome::Skipped => "SKIP",
            TaskCostOutcome::BudgetExceeded => "BUDGET",
        };
        println!(
            "  {}/{} ({})  {} in / {} out  ${:.4}  {:.1}s  {}",
            tc.plan_id, tc.task_id, tc.model,
            fmt_tokens(tc.tokens_in), fmt_tokens(tc.tokens_out),
            tc.cost_usd,
            tc.duration_ms as f64 / 1000.0,
            outcome_label,
        );
    }
    let total_cost: f64 = task_costs.iter().map(|tc| tc.cost_usd).sum();
    let total_in: u64 = task_costs.iter().map(|tc| tc.tokens_in).sum();
    let total_out: u64 = task_costs.iter().map(|tc| tc.tokens_out).sum();
    let passed = task_costs.iter().filter(|tc| tc.outcome == TaskCostOutcome::Passed).count();
    let total_duration_ms: u64 = task_costs.iter().map(|tc| tc.duration_ms).sum();
    println!(
        "  {}\n  Total: {} tasks, {} passed, ${:.4}, {} in / {} out, {}",
        "-".repeat(72),
        task_costs.len(), passed, total_cost,
        fmt_tokens(total_in), fmt_tokens(total_out),
        format_duration_ms(total_duration_ms),
    );
}
```

### 5.6 Fix Duplicate run_summary

**File**: `crates/roko-cli/src/runner/event_loop.rs`

The `persist_run_ledger()` call inside the `all_plans_terminal` check (line 1536) and
the one after the event loop (line 1548) both write `run_summary`. Fix by adding a
guard flag:

```rust
// Before the event loop:
let mut run_summary_written = false;

// Inside all_plans_terminal (line 1536):
if !run_summary_written {
    persist_run_ledger(&run_ledger, &paths.run_ledger_jsonl);
    run_summary_written = true;
}
break;

// After the event loop (line 1548):
if !run_summary_written {
    persist_run_ledger(&run_ledger, &paths.run_ledger_jsonl);
}
```

### 5.7 Add run-ledger.jsonl to JSONL Rotation

**File**: `crates/roko-cli/src/runner/event_loop.rs`

Before the event loop, register the run ledger path for rotation:
```rust
// Near line 459, after creating run_ledger:
if let Err(e) = roko_learn::jsonl_rotation::rotate_if_needed(
    &paths.run_ledger_jsonl,
    roko_learn::jsonl_rotation::DEFAULT_ROTATION_THRESHOLD_BYTES,
).await {
    warn!(error = %e, "run ledger rotation failed");
}
```

### 5.8 Wire RunLedger.record_agent_completed

**File**: `crates/roko-cli/src/runner/event_loop.rs`, at gate-pass (line ~1029)

```rust
if let Some(ref mut ledger) = run_ledger {
    ledger.record_agent_completed(
        "implementer",
        state.agent_output.chars().take(500).collect::<String>(),
        output_files.len() as u32,
        "",
        &state.agent_model,
        Some(state.agent_provider.clone()),
        roko_core::foundation::TokenUsage {
            input_tokens: state.tokens_in,
            output_tokens: state.tokens_out,
            total_tokens: state.tokens_in + state.tokens_out,
            cost_usd: state.cost_usd,
        },
    );
    // ... existing record_phase_transition + append_ledger_entry ...
}
```

---

## 6. Test Strategy

### 6.1 Unit Tests

**File**: `crates/roko-learn/src/cost.rs`

| Test | What it verifies |
|------|-----------------|
| `cost_event_serialization_roundtrip` | CostEvent -> JSON -> CostEvent preserves all fields |
| `task_cost_report_serialization_roundtrip` | TaskCostReport round-trips through serde |
| `aggregate_task_cost_sums_attempts` | Multiple CostEvents for the same task produce correct totals |
| `aggregate_run_cost_groups_by_plan` | RunCostReport correctly groups per-plan subtotals |
| `aggregate_run_cost_model_breakdown` | ModelCostEntry computed correctly from heterogeneous events |
| `session_cost_daily_grouping` | DailyCostEntry groups events by calendar day |
| `budget_status_computes_pct` | PlanBudgetEntry.pct_used is correct |

**File**: `crates/roko-cli/src/runner/state.rs`

| Test | What it verifies |
|------|-----------------|
| `harvest_task_cost_captures_all_fields` | All RunState cost fields flow into TaskCostReport |
| `harvest_task_cost_accumulates_per_run` | Multiple calls build up task_costs vec |
| `task_cost_reset_between_tasks` | Per-task fields reset, totals accumulate |

### 6.2 Integration Tests

**File**: `crates/roko-cli/tests/runner_cost_tracking.rs`

| Test | What it verifies |
|------|-----------------|
| `run_report_includes_task_costs` | After a mock plan run, RunReport.task_costs is populated |
| `run_ledger_entries_have_cost_fields` | run-ledger.jsonl task_completed entries contain model/tokens/cost |
| `no_duplicate_run_summary` | Only one run_summary entry in run-ledger.jsonl per run |
| `budget_warning_emitted_at_threshold` | Warning logged when spend reaches 80% of limit |
| `budget_exceeded_terminates_run` | Run aborts when spend reaches 100% of limit |
| `json_output_includes_task_costs` | `--json` output contains task_costs array |
| `cost_events_written_to_jsonl` | cost-events.jsonl contains one entry per agent turn |
| `run_cost_report_written_at_completion` | run-costs/{run_id}.json exists after run |

### 6.3 Property Tests

| Test | What it verifies |
|------|-----------------|
| `task_cost_totals_equal_run_totals` | Sum of task_costs.cost_usd == RunReport.total_cost_usd |
| `cost_events_sum_to_task_cost` | Sum of CostEvents for a task == TaskCostReport.cost_usd |
| `run_ledger_cost_matches_report` | run-ledger.jsonl cost fields match RunReport |

### 6.4 End-to-End Smoke Test

```bash
# Run a small plan and verify cost output appears:
cargo run -p roko-cli -- plan run test-plans/ 2>&1 | grep "Task costs:"

# Verify JSON output:
cargo run -p roko-cli -- plan run test-plans/ --json | jq '.task_costs'

# Verify cost-events.jsonl was written:
wc -l .roko/learn/cost-events.jsonl

# Verify run cost report:
ls .roko/state/run-costs/

# Verify no duplicate run_summary:
grep -c '"kind":"run_summary"' .roko/state/run-ledger.jsonl
# Should output: 1
```

---

## 7. Priority and Sequencing

| Step | What | Effort | Impact |
|------|------|--------|--------|
| 1 | Add `TaskCostReport` type to roko-learn | S | Foundation |
| 2 | Add `harvest_task_cost()` to RunState | S | Foundation |
| 3 | Wire harvest at gate-pass and gate-fail paths | M | Per-task cost capture |
| 4 | Extend RunReport + build_report | S | Machine-readable output |
| 5 | Print "Task costs:" summary in CLI | S | Human-readable output |
| 6 | Fix duplicate run_summary bug | S | Correctness |
| 7 | Add run-ledger JSONL rotation | S | Prevents unbounded growth |
| 8 | Enrich run-ledger entries with cost fields | S | Audit trail |
| 9 | Wire RunLedger.record_agent_completed | S | Typed ledger completeness |
| 10 | Add CostEvent type + JSONL writer | M | Granular cost log |
| 11 | Add `--json` task_costs output | S | Tooling integration |
| 12 | Budget warning thresholds | M | Proactive cost control |
| 13 | Per-run budget limit | M | Hard cost control |
| 14 | RunCostReport JSON persistence | M | Cross-run comparison |
| 15 | `roko learn costs` CLI command | M | Cost visibility |
| 16 | API endpoint enrichment | M | Dashboard/external integration |
| 17 | Cost-adjusted cascade router reward | L | Intelligent model selection |
| 18 | SessionCostReport + daily trends | L | Long-term cost management |
| 19 | TUI cost widgets | L | Real-time cost visibility |

Steps 1-9 close the original gap (no per-task cost visibility). Steps 10-19 build
the full cost management system.
