# Orchestration, Gate Pipeline, and Runtime Engine Audit

> Source-grounded analysis of the three execution engines, gate infrastructure,
> learning feedback loops, and prompt assembly system. All file paths, line numbers,
> and struct names are verified against the current codebase on branch `wp-arch2`.

---

## 1. The Three Execution Engines

Roko has three distinct execution engines that coexist. This is the central
structural problem. Each engine was built for a different purpose, wired into
different entry points, and carries different feature sets.

### 1.1 orchestrate.rs (Legacy PlanRunner) -- 22,522 lines

**File**: `crates/roko-cli/src/orchestrate.rs`
**Entry point**: `PlanRunner::run_task_plans()`
**Status**: Fully featured but deprecated. Not the default path.

The PlanRunner is a monolith. It wraps `ParallelExecutor` (a pure state machine
from `roko-orchestrator`) and performs all I/O: agent spawning, gate execution,
git operations, learning persistence. The struct has 80+ fields covering every
subsystem in the codebase.

**Core fields (verified from source)**:

| Category | Fields |
|---|---|
| Execution | `executor: ParallelExecutor`, `event_log: EventLog`, `workdir`, `config` |
| Worktree | `WorktreeManager`, `PostMergeRunner` |
| Learning | `LearningRuntime`, `PlaybookStore`, `SkillLibrary`, `DaimonState`, `KnowledgeStore`, `FeedbackService` |
| Gates | `AdaptiveThresholds`, `GateArtifactStore`, `GateRatchet`, `VerdictPublisher` |
| Routing | `LatencyRegistry`, `RouterCalibration`, `ProfileBandit`, `CrateFamiliarityTracker` |
| Anomaly | `Conductor`, `HealthMonitor`, `StuckDetector`, `AnomalyDetector` |
| Budget | `plan_costs: HashMap`, `task_costs: HashMap` |
| Context | `ContextAttributionTracker`, `code_index_cache` |
| Process | `ProcessSupervisor`, `CancelToken`, `MetricRegistry`, `CustodyLogger` |
| MCP | `mcp_server_names`, `mcp_state`, `tool_registry` |
| Events | `LearningEventBus`, `RuntimeEventBus` |

**Execution flow**:
1. `run_task_plans()` -- signal handling (SIGINT/SIGTERM), `tokio::select!` races plan completion vs shutdown
2. `dispatch_action()` -- routes `ExecutorAction` variants to phase handlers
3. `dispatch_agent_with()` (line ~14477) -- the core dispatch pipeline with 12 steps
4. `run_gate_pipeline()` -- runs the 7-rung gate pipeline with adaptive thresholds

**What it has that others lack**:
- CascadeRouter observations persisted to `cascade-router.json`
- AdaptiveThresholds persisted to `gate-thresholds.json`
- Replan-on-gate-failure via `build_gate_failure_plan_revision()`
- Episode logging to `episodes.jsonl`
- Section effectiveness tracking
- C-factor computation
- Playbook extraction from successful tool-call sequences
- Knowledge store queries injected into system prompts
- HDC fingerprint per episode
- Crate familiarity tracking
- Daimon affect modulation per dispatch
- Full enrichment pipeline (code context, prior outputs, research, pheromones)

**Why it is deprecated**: 22K lines in a single file. Every dogfood fix touched it.
Memory leaks from unbounded vectors. Batch-only output parsing (waits for agent exit,
reads all output at once). No streaming to TUI.

### 1.2 Runner v2 -- 15 files, ~2,181 lines

**Directory**: `crates/roko-cli/src/runner/`
**Entry point**: `runner::run(plans, &config, &state_hub, cancel)`
**Status**: Default for `plan run`. Streaming architecture, cleaner code. Missing learning features.

Runner v2 was built to fix orchestrate.rs's fundamental problems:

| File | Lines | Purpose |
|---|---|---|
| `mod.rs` | 38 | Module organization, re-exports |
| `event_loop.rs` | ~400 | Core `tokio::select!` loop driving executor |
| `agent_stream.rs` | ~200 | Line-by-line JSON stream parsing from agent |
| `agent_events.rs` | ~150 | Agent completion/failure event handling |
| `gate_dispatch.rs` | ~120 | Gate execution dispatch |
| `merge.rs` | ~180 | Worktree merge operations |
| `persist.rs` | ~200 | State serialization to disk |
| `resume.rs` | ~150 | Resume from prior snapshot with fingerprint validation |
| `state.rs` | ~130 | RunState tracking per-task attempts |
| `task_dag.rs` | ~100 | Task dependency graph construction |
| `plan_loader.rs` | ~120 | Plan/tasks.toml loading |
| `tui_bridge.rs` | ~100 | StateHub integration for TUI updates |
| `types.rs` | ~200 | Shared type definitions |
| `projection.rs` | ~80 | Run summary projection |
| `extension_loader.rs` | ~50 | Extension chain loading |

**Architecture**: Event-driven loop with `RunContext` struct (11 fields, not 80+).
Agent output parsed line-by-line via `--output-format stream-json`. State flushed
to disk after every task completion. Process groups ensure clean agent teardown.

**Key imports in event_loop.rs (verified)**:
```rust
use roko_learn::contextual_bandit::{ActionSafetyBounds, BanditContextFeatures, BanditDecisionKind};
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::{Episode, GateVerdict, Usage};
use roko_learn::model_router::RoutingContext;
use roko_learn::runtime_feedback::{CompletedRunInput, LearningPaths, LearningRuntime, ...};
```

This shows runner v2 *imports* learning types but the wiring is partial. The
`LearningRuntime` is constructed but not all feedback paths are connected.

**What it has that orchestrate.rs lacks**:
- Streaming agent output (line-by-line parsing)
- Per-task state flushing (crash-safe)
- Fingerprint-based resume validation
- JSONL truncation recovery on corrupt files
- Process group management
- Clean StateHub integration for TUI

**What it is missing (verified gaps)**:
- Does NOT persist CascadeRouter observations
- Does NOT persist AdaptiveThreshold updates
- Does NOT fire replan-on-gate-failure
- Does NOT compute section effectiveness
- Does NOT query playbooks for prompt injection
- Does NOT compute C-factor metrics
- Model name shows "-" in TUI (empty string passed)
- No knowledge store queries during dispatch
- No crate familiarity tracking
- No Daimon affect modulation

### 1.3 WorkflowEngine (roko-runtime) -- The Unified Path

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**Entry point**: `WorkflowEngine::run(config).await`
**Status**: Used by `roko run`, `roko chat`, HTTP API. Single-task workflows only.

The WorkflowEngine is the cleanest architecture. It separates concerns into three
layers with zero coupling between them:

1. **PipelineStateV2** (`pipeline_state.rs`) -- Pure state machine. No I/O.
   Takes `PipelineInput`, returns `PipelineOutput`. Three templates:
   - Express: implement -> gate -> commit
   - Standard: implement -> gate -> review -> commit
   - Full: strategy -> implement -> gate -> review -> commit

2. **EffectDriver** (`effect_driver.rs`) -- Executes pipeline actions via
   foundation-style services. Calls `ModelCaller`, `GateRunner`, `PromptAssembler`,
   `FeedbackSink`, `AffectPolicy`.

3. **ServiceFactory** (`roko-orchestrator/src/service_factory.rs`) -- Constructs
   all services through a canonical path. Resolves models, wires CascadeRouter,
   KnowledgeStore, PlaybookStore, SectionEffectiveness, FeedbackService.

**ServiceBundle fields (verified from service_factory.rs line 66)**:
```rust
pub struct ServiceBundle {
    pub model: String,
    pub model_call_service: Arc<ModelCallService>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
}
```

**What it has**:
- Clean trait-based dispatch (ModelCaller, GateRunner, PromptAssembler)
- Cooperative cancellation via CancelToken
- RuntimeEvent emission to event bus
- Feedback recording via FeedbackSink (backed by FeedbackService with CascadeRouter)
- Affect modulation via DaimonPolicy
- Knowledge store wired into prompt assembly
- Playbook store wired into prompt assembly
- Section effectiveness wired into prompt assembly
- WorkflowRunReport with gate outcomes, events, token/cost tracking

**What it cannot do**:
- Multi-task plans (it runs a single prompt through the pipeline)
- DAG execution (no task dependencies)
- Worktree isolation (works in the current directory)
- Resume from prior state (no snapshot persistence)
- Plan-level concepts (no PlanRunner, no ParallelExecutor)

---

## 2. Gate Pipeline (`roko-gate`)

### 2.1 GateService -- The Canonical Gate Runner

**File**: `crates/roko-gate/src/gate_service.rs` (680 lines)
**Entry point**: `GateService::run_gates(config).await`
**Implements**: `GateRunner` trait from `roko-core::foundation`

GateService maps gate names to concrete implementations and runs them in rung order.
It is the only gate execution path used by the WorkflowEngine.

**Rung mapping (verified from `rung_for_name()` at line 51)**:

| Rung | Index | Gate Names | Implementation |
|---|---|---|---|
| Compile | 0 | `compile`, `compile:cargo` | `CompileGate` |
| Lint | 1 | `clippy`, `clippy:cargo` | `ClippyGate` |
| Test | 2 | `test`, `test:cargo` | `TestGate` |
| Diff | 3 | `diff`, `diff:git` | `ShellGate("git", ["diff", "--stat"])` |
| Format | 4 | `fmt`, `fmt:cargo`, `format` | `FormatCheckGate` (wraps `cargo fmt --check`) |
| Custom | 5 | `custom`, `shell`, `custom:shell` | `ShellGate` from `ShellGateCommand` config |
| Judge | 6 | `judge`, `llm-judge` | `StubJudgeGate` (always skipped, not implemented) |

**Key behaviors**:
- Gates are sorted by rung index before execution (`ordered_gate_names()`)
- `max_rung` config filters out higher rungs (e.g., `max_rung: 3` runs only compile, clippy, test, diff)
- Sequential short-circuit: stops at first non-skipped failure
- Shell/custom gates require explicit `ShellGateCommand` from config -- no stub returned
- Judge gate is always skipped with "not implemented" message

**Adaptive threshold integration (verified from line 120-136)**:
- `should_skip_rung_adaptively()` checks if a rung has a long consecutive-pass streak
- Rung 0 (compile) is NEVER skipped regardless of thresholds
- After each gate execution, `thresholds.observe(rung, passed)` is called
- Thresholds are wrapped in `Arc<Mutex<AdaptiveThresholds>>`

### 2.2 Adaptive Thresholds

**File**: `crates/roko-gate/src/adaptive_threshold.rs`

Per-rung statistics tracked:

| Metric | Description | Initial Value |
|---|---|---|
| `ema_pass_rate` | Exponential moving average of pass rate | 0.5 (neutral) |
| `total_observations` | Cumulative observation count | 0 |
| `consecutive_passes` | Reset on any failure | 0 |
| `cusum_high` / `cusum_low` | CUSUM change-point accumulators | 0.0 |
| `cusum_shift_detected` | Whether CUSUM detected a shift | false |

**Constants (verified)**:
- EMA alpha: 0.1
- Min retries: 1, Max retries: 5
- Skip streak threshold: 20 consecutive passes
- CUSUM sensitivity (k): 0.25
- CUSUM decision threshold: 4.0

**SPC detectors**: CUSUM + EWMA Control Chart + BOCPD per rung
**Hotelling's T-squared**: Joint multi-gate anomaly detection
**Domain profiles**: coding, research, security -- each with rung-specific priors

### 2.3 The 7-Rung Pipeline (orchestrate.rs)

The full 7-rung pipeline exists only in orchestrate.rs via `run_rung()` dispatch.
This is richer than GateService:

| Rung | What orchestrate.rs adds |
|---|---|
| 0 (Compile) | Same as GateService |
| 1 (Lint) | Same as GateService |
| 2 (Test) | Same as GateService |
| 3 (Symbol) | `SymbolGate` -- checks symbol manifest expectations |
| 4 (GeneratedTest) | `GeneratedTestGate` + `VerifyChainGate` |
| 5 (PropertyTest) | `PropertyTestGate` + `FactCheckGate` |
| 6 (Integration) | `LlmJudgeGate` (real LLM call, not stub) + `IntegrationGate` |

Rungs 3-6 have oracle support (`enrich_rung_config()`) that queries LLMs for
deeper verification. These oracles are unique to orchestrate.rs.

### 2.4 Additional Gate Infrastructure

| Component | File | Status |
|---|---|---|
| `GateRatchet` | `roko-gate/src/ratchet.rs` | Per-plan rung watermark. Wired in orchestrate.rs |
| `ArtifactStore` | `roko-gate/src/artifact_store.rs` | Content-addressed storage. Wired in orchestrate.rs |
| `VerdictPublisher` | `roko-gate/src/verdict_publisher.rs` | Broadcasts verdicts as signals. Wired in orchestrate.rs |
| `Forensic` | `roko-gate/src/forensic.rs` | Causal chain reconstruction. Built, partially wired |
| `ProcessReward` | `roko-gate/src/process_reward.rs` | Step-level reward model. Built, not wired at runtime |
| `GatePipeline` | `roko-gate/src/gate_pipeline.rs` | Composition modes (sequential, parallel, voting, fallback) |

---

## 3. DAG Execution (`roko-orchestrator`)

### 3.1 UnifiedTaskDag

**File**: `crates/roko-orchestrator/src/dag.rs`

The DAG supports four edge types:
1. **Intra-plan `depends_on`** -- task A -> task B within one plan
2. **Cross-plan `depends_on`** -- `"09-foo:t3"` in plan `"10-bar"` creates an edge
3. **Plan-level `depends_on`** -- plan B depends on plan A: all A tasks -> all B tasks
4. **File-overlap inference** (opt-in via `DagConfig::infer_file_overlap`) -- two tasks
   touching the same file get serialized; lexicographically earlier `GlobalTaskId` first

**Key types**:
- `ExecutionWave` -- wave ordinal + task list (sorted by GlobalTaskId) + estimated_minutes
- `DagConfig` -- `infer_file_overlap: bool`, `max_wave_width: usize` (0 = unbounded)
- `DagStats` -- nodes, edges, waves, critical_path_minutes
- `CpmAnalysis` -- Critical Path Method for scheduling optimization

**DAG mutations**: Runtime modifications to the DAG for replanning and task injection.

### 3.2 ParallelExecutor (Pure State Machine)

**File**: `crates/roko-orchestrator/src/executor.rs`

Default config:
- max 4 concurrent plans
- 8 concurrent tasks (runner v2 overrides to 1)
- 5 auto-fix iterations
- 3 merge attempts
- 600s task timeout

**14-Phase Task Lifecycle**:

```
Queued -> Enriching -> Implementing -> Gating -> Verifying -> Reviewing -> DocRevision -> Merging -> Complete
                                        |                                      |
                                        v                                      v
                                   AutoFixing -> Gating                   Implementing (rework)
                                        |
                                        v (iterations >= 5)
                                      Failed
```

Key transitions:
- `GateFailed` at Gating: iteration < 5 -> AutoFixing; else Failed
- `MergeFailed` at Merging: merge_attempts < 3 -> retry; else Failed
- `ReviewRejected`: loops back to Implementing

### 3.3 State Snapshots and Resume

**File**: `crates/roko-orchestrator/src/runtime_snapshot.rs`

`ExecutorSnapshot` includes:
- Schema version
- All plan states and queue order
- Circuit breaker state
- Speculative executions
- Timestamp

Persisted atomically (tmp + rename) at `.roko/state/executor.json`.
Auto-save every 5 dispatched actions. `RecoveryEngine` reconciles snapshots
with event logs for crash recovery.

Runner v2 adds **fingerprint validation**: on resume, every task's content hash
must match the prior run's recorded fingerprint. Drift is a hard error.

---

## 4. Learning System (`roko-learn`)

### 4.1 Episode Logger

Append-only JSONL at `.roko/episodes.jsonl`. Each episode records:

| Field | Type | Description |
|---|---|---|
| agent_id | String | Which agent ran |
| task_id | String | Which task was attempted |
| kind | String | Episode type (implement, autofix, review, etc.) |
| model | String | Model slug used |
| backend | String | Provider backend |
| input_tokens | u64 | Input token count |
| output_tokens | u64 | Output token count |
| cache_read | u64 | Cache read tokens |
| cost_usd | f64 | Cost in USD |
| wall_ms | u64 | Wall-clock time |
| gate_verdicts | Vec<GateVerdict> | Gate results for this attempt |
| hdc_fingerprint | Option<Vec<f32>> | HDC vector for similarity matching |
| metadata | Map | Extra metadata (capped 16KB) |

### 4.2 CascadeRouter (3-Stage Model Selection)

Static (< 50 obs) -> Confidence (50-200) -> UCB1 (> 200 observations):

The router uses a 17-dimensional context vector including:
- task_category, complexity_band, iteration_count
- role, crate_familiarity, prior_failure_count
- domain, estimated_minutes, file_count
- prior_model_success_rate, time_of_day, etc.

Features: Pareto frontier for cost-quality trade-offs, shadow evaluation via Gemini,
hysteresis, cache affinity, temperament exploration, override learning (UX34).

**Wired in ServiceFactory** (`service_factory.rs` line 123): CascadeRouter is loaded
from `.roko/learn/cascade-router.json` and passed into both `FeedbackService` and
`ModelCallService`. The ServiceFactory also collects all configured model slugs
(default, fallback, tier, configured) to seed the router's candidate set.

### 4.3 Playbook Store

Named action sequences proven to achieve goals. Each playbook has:
- `PlaybookStep` entries with index, description, action_kind, expected_signals
- Success/failure counters
- Merge threshold (0.80 similarity)
- Auto-extraction from successful tool call sequences

**Wired in ServiceFactory** (`service_factory.rs` line 172): PlaybookStore is constructed
from `.roko/learn/playbooks/` and passed into PromptAssemblyService. Playbooks are
injected as Layer 6 (Techniques) in the 9-layer system prompt.

### 4.4 Efficiency Monitoring

30+ fields per agent turn. Key fields:
- Token accounting (input, output, reasoning, cache read/write)
- Cost (actual, without cache)
- Prompt composition metadata (per-section attribution)
- Tool utilization (available vs used)
- Timing (wall, TTFT, warm/cold)
- Letter grades (A-D)

### 4.5 LearningRuntime (runner v2)

Runner v2 constructs a `LearningRuntime` from `LearningPaths`:
```rust
LearningPaths {
    episodes_jsonl,
    efficiency_jsonl,
    cascade_router_json,
    gate_thresholds_json,
    section_effects_json,
    ...
}
```

The `CompletedRunInput` struct aggregates end-of-run learning data. The
`LearningRuntime` has methods for recording episodes, efficiency events,
gate outcomes, and routing observations. The gap is that not all of these
recording paths are called from the event loop.

---

## 5. Five Feedback Loops

### Loop 1: Gate Failure -> Autofix -> Retry

```
Gate -> failure -> AutoFixing phase -> fix agent receives error context ->
Gating phase again -> max 5 iterations -> Failed
```

The autofix agent receives the gate's `error_digest` and `detail` fields as context.
The pipeline state machine tracks `iteration` count and caps at `max_auto_fix_iterations`.

### Loop 2: Gate Failure -> Replan (orchestrate.rs only)

```
gate_failure_count per plan ->
maybe_emit_gate_failure_plan_revision() ->
GateFailureAction::NeedsReplan ->
build_gate_failure_plan_revision() ->
ReplanLedger deduplication ->
PlanRevision event ->
replan_plan() with architectural model
```

**NOT WIRED in runner v2 or WorkflowEngine.** This is the most significant missing
feedback loop. Without it, repeated gate failures just exhaust the autofix budget
and fail, rather than stepping back and reconsidering the approach.

### Loop 3: Success -> Learning

`record_task_success()` in orchestrate.rs:
- Multi-objective observation to CascadeRouter (quality, cost, duration)
- Router calibration update
- Playbook recorded/updated
- Knowledge store success entry
- Crate familiarity updated
- Section effectiveness updated
- Efficiency event emitted

### Loop 4: Failure -> Anti-Knowledge

`record_task_failure()` in orchestrate.rs:
- Anti-knowledge entry in neuro store
- Error pattern stored
- Conductor bandit updated
- Adaptive thresholds adjusted
- Anomaly detector fed

### Loop 5: Cross-Task Context Propagation

- Prior task outputs loaded and injected as context sections
- Pheromone field carries gate verdicts between tasks
- Knowledge store queried per-task for relevant prior learnings
- Playbook store consulted for matching action sequences
- Section effectiveness adjusts prompt section priorities

---

## 6. Prompt Assembly (`roko-compose`)

### 6.1 SystemPromptBuilder (9 Layers)

**File**: `crates/roko-compose/src/system_prompt_builder.rs`

| Layer | Content | Cache Tier |
|---|---|---|
| 1 | Role identity | System (stable) |
| 2 | Conventions | System (semi-stable) |
| 3 | Domain context | Session (semi-stable) |
| 3c | Active pheromone signals | Session |
| 4 | Task context | Task (volatile) |
| 4b | Gate feedback | Dynamic |
| 5 | Tool instructions | System (stable) |
| 6 | Techniques (playbooks, skills) | Task (volatile) |
| 7 | Anti-patterns | Task (volatile) |
| 8 | Affect guidance | Dynamic |

Features: cache-aligned normalization, canonical tool ordering, token budget
enforcement, per-layer section caps, section effectiveness integration,
temperament dial.

### 6.2 PromptAssemblyService

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**Implements**: `PromptAssembler` trait from `roko-core::foundation`

This is the trait-based version wired through ServiceFactory. It wraps the
SystemPromptBuilder with concrete stores:
- KnowledgeStore for Layer 3 domain context
- PlaybookStore for Layer 6 techniques
- Episodes for anti-pattern extraction (Layer 7)
- SectionEffectiveness for priority weighting
- Tool instructions for Layer 5

### 6.3 VCG Auction

**File**: `crates/roko-compose/src/auction.rs`

Context allocation mechanism design:
- `vcg_allocate()` for section allocation under budget
- Bidders: Learning, Affect, per-subsystem (Neuro, Task, Research)
- Fairness config, Pareto optimality detection, cost attribution

**Status**: `vcg_allocate()` built and exported but greedy path dominates at runtime.
The auction is never called from any live dispatch path.

### 6.4 Enrichment Pipeline

Multi-step pre-dispatch enrichment:
- Steps: code context, prior outputs, knowledge query, research, pheromone signals
- Budget-aware step selection
- Quality validation
- Token estimation

**Used in orchestrate.rs only.** Runner v2 has `skip_enrichment` per-task but
does not call the enrichment pipeline.

---

## 7. Per-Agent Sidecar (`roko-agent-server`)

### Routes (verified)

| Module | Routes | Auth |
|---|---|---|
| health | `GET /health`, `GET /capabilities` | Public |
| stats | `GET /stats` | Protected |
| logs | `GET /logs` | Protected |
| messaging | `POST /message`, `GET /stream` (WebSocket) | Protected |
| predictions | `POST /predictions`, `GET /predictions` | Protected |
| research | Research endpoints | Protected |
| tasks | Task queue | Protected |

### Messaging Architecture

- `POST /message`: Synchronous dispatch -> JSON response (content, reasoning, usage,
  session state, finish reason)
- `GET /stream`: WebSocket upgrade -> StreamChunk variants (reasoning deltas,
  content deltas, tool call deltas, usage, errors)
- Heartbeat loop: POSTs to roko-serve at 30s intervals
- Agent registration with capability cards

---

## 8. Runner Lessons Applied to Orchestration

The parallel agent runner (`tmp/solutions/runner/LESSONS.md`) validates the orchestration
loop at scale (195 batches, 177K LOC). Key architectural insights that directly apply:

### 8.1 Isolation Is Non-Negotiable

The runner uses per-batch git worktrees. Each batch gets a full checkout forked from
the runner's main branch. Roko's `WorktreeManager` does the same for plan tasks.
The runner proves this works at scale and identifies the key failure modes:
worktree ghosts, branch name collisions, and stale refs.

### 8.2 Context Handoff Is the Hard Problem

The runner's "cumulative section" -- telling each batch what prior batches changed --
is more important than the batch prompt itself. This maps to roko's Layer 3 domain
context and pheromone propagation. The runner found that without cumulative context,
30% of batches produce merge conflicts.

### 8.3 Gates Should Be Batched, Not Per-Agent

Per-agent compilation takes 15-40 minutes. Wave gates (compile after a batch of changes)
take 3-8 minutes. The runner proves that deferred gating is 10-100x faster with
acceptable error rates (10-30 compile errors after 195 batches with no-build mode).

This suggests roko should support an "express gate" mode where compile/lint are
deferred until wave boundaries, with only anti-pattern checks per-task.

### 8.4 The --continue Pattern Is Essential

The runner's `--continue` flag resumes from disk state, not memory state. Any process
that runs for hours will crash. Roko's executor snapshot + runner v2's fingerprint
validation implement this pattern. The runner validates it works at scale.

### 8.5 Result Files as Coordination Mechanism

The runner uses simple files on disk (`BATCH.result` containing "success" or "failed")
as the primary coordination mechanism. This enables manual intervention at any point.
Roko's `.roko/state/executor.json` serves a similar purpose but is more complex
(full JSON snapshot vs. single-word status files).

### 8.6 Anti-Pattern Checks as Fast Gates

The runner's AP checks (grep-based, millisecond latency) catch LLM code-gen mistakes
without compilation. These are structurally equivalent to roko's Symbol gate (rung 3)
but faster and more targeted. AP checks catch:
- Stub gates returning pass (AP-1)
- `block_on` in async code (AP-2)
- Duplicate trait definitions (AP-3)
- Raw `Command::new("claude")` shell-outs (AP-5)
- Inline prompt strings (AP-6)
- `std::sync::Mutex` held across `.await` (AP-7)
- Empty function bodies (AP-8)
- `unimplemented!`/`unreachable!` left behind (AP-9)

---

## 9. Summary Assessment

### What Works Well
- The pure state machine pattern (PipelineStateV2, ParallelExecutor) is sound
- ServiceFactory provides a clean canonical construction path
- Adaptive thresholds with SPC detectors are sophisticated and correct
- The 9-layer prompt builder is well-structured
- RuntimeEvent emission provides observability
- Runner v2's streaming + fingerprint resume is production-quality

### What Does Not Work
- Three execution engines with inconsistent features
- Learning persistence only in the deprecated path (orchestrate.rs)
- VCG auction built but never called
- Judge gate is a stub in GateService
- Enrichment pipeline only in orchestrate.rs
- Replan-on-gate-failure only in orchestrate.rs
- Two gate config formats (`[[gate]]` vs `[gates]`)
- Two model selection paths that can disagree
- 80+ field PlanRunner struct is unmaintainable

### The Path Forward
Port orchestrate.rs's learning features into runner v2 and WorkflowEngine.
Do NOT port the code -- port the *wiring patterns*. The ServiceFactory +
FeedbackService + CascadeRouter architecture already supports what's needed.
The work is calling the right methods at the right points in the event loop.
