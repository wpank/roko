# Orchestration & Plan Execution Audit

3 runtimes, 2 state machines, 1 dead monolith — the plan execution subsystem has everything built but spread across incompatible implementations.

## The Problem

Roko has three functionally-overlapping runtimes with significant code duplication. The newest (ACP) is cleanest but simplest. The active one (Runner v2) is mid-featured. The dead one (orchestrate.rs, 21,577 LOC) has the most sophisticated features that were never ported. Features silently deactivate when switching runtimes.

---

## 1. The Three Runtimes

| Runtime | Files | LOC | Status | Architecture |
|---|---|---|---|---|
| ACP pipeline | `roko-acp/src/{pipeline,runner,workflow}.rs` | ~1,200 | Active (Zed) | Pure state machine + effect driver |
| Runner v2 | `roko-cli/src/runner/` (15 files) | ~3,000+ | Active (CLI) | Event-driven tokio `select!` executor |
| Orchestrate.rs | `roko-cli/src/orchestrate.rs` | 21,577 | Legacy | Batch monolith |

### ACP Pipeline (Lightweight, Newest)

**State machine:** `PipelinePhase` — 10 states:
```
Pending → Strategizing → Implementing → Gating → AutoFixing → Reviewing → Committing → Complete/Halted/Cancelled
```

**Actions emitted:** `SpawnStrategist`, `SpawnImplementer`, `SpawnAutoFixer`, `RunGates`, `SpawnReviewer`, `Commit`, `Done`, `Halt`

**Workflow templates:**
- Express: implement → gate → commit
- Standard: + review
- Full: + strategist

**Strengths:**
- Pure `step(event) → action` — side-effect free state machine
- Auto-selects template based on prompt complexity
- Iteration loop on gate/review failures with backoff
- Multi-role review for "thorough" mode

**Weaknesses:**
- Serial agents only (one role per phase)
- No resume across sessions (crash = loss)
- No DAG execution, no parallel tasks

### Runner v2 (Streaming, Active)

**State machine:** `PlanPhase` — 14 states:
```
Queued → Enriching → Implementing → Gating → Verifying → Reviewing → DocRevision → AutoFixing → RegeneratingVerify → Merging → Complete/Failed/Skipped
```

**Key features:**
- Line-by-line streaming output parsing (stream-json)
- Task DAG with dependency-driven wave scheduling
- Real-time TUI updates via StateHub
- Per-plan merge queue with conflict detection
- Strict resume via fingerprint validation
- Speculative execution for slow tasks

**Concurrency settings:**
```rust
max_concurrent_plans: 4    // default
max_concurrent_tasks: 1    // bottleneck — serial by task
```

### Orchestrate.rs (Batch, Legacy — 21,577 LOC)

**Same PlanPhase** as Runner v2, but implements 11,000+ lines of features never ported:
- SystemPromptBuilder 9-layer enrichment (30+ steps)
- DaimonState affect engine integration
- Dream consolidation (hypnagogia loops)
- Knowledge routing (neuro store queries)
- VCG auction composition
- C-factor computation
- Anophily detection + remediation
- Custody audit chain
- Skill extraction + SkillLibrary
- Full learning feedback loop

---

## 2. State Machine Comparison

### PlanPhase (roko-core, used by Runner v2 + orchestrate.rs)

14 states with typed failure kinds:
```
Queued | Enriching | Implementing | Gating | Verifying | Reviewing
| DocRevision | AutoFixing | RegeneratingVerify | Merging
| Complete | Done | Failed { reason: FailureKind } | Skipped
```

**FailureKind enum:** AutoFixExhausted, AllTasksFailed, TaskRetriesExhausted, SetupFailed, MaxIterations, SpawnFailures, Deadlock, WorktreeMissing, VacuousImplementation, VerifyScriptBroken, Other(String)

**Transition enforcement:** Valid transitions defined in `phase.rs:238-258`. Terminal states (Complete, Failed, Skipped) have no successors. Gating → AutoFixing loops until max iterations.

### PipelinePhase (roko-acp)

10 states — simpler subset, **not compatible** with PlanPhase:
- ACP `Strategizing` has no PlanPhase equivalent
- ACP lacks `Enriching`, `Verifying`, `DocRevision`, `RegeneratingVerify`, `Merging`
- ACP `Committing` maps to PlanPhase `Merging` (loose equivalence)

**Problem:** Two state machines for the same concept. Converging them requires mapping ACP phases into PlanPhase or vice versa.

---

## 3. Plan Format (tasks.toml)

```toml
[meta]
plan = "plan-id"
iteration = 1
total = N
done = M
status = "implementing"
max_parallel = 1

[[task]]
id = "T1"
title = "Implement feature X"
description = "..."
role = "implementer"
tier = "mechanical|focused|integrative|architectural"
model_hint = "claude-opus-4-6"
status = "pending|active|done|blocked"
files = ["src/main.rs"]
depends_on = ["T0"]
depends_on_plan = ["00-foundation"]
allowed_tools = ["bash", "edit"]
denied_tools = ["rm", "git"]
timeout_secs = 600
max_retries = 3
acceptance = ["All tests pass"]

[task.verify]
[task.verify.compile]
required = true
rung = 0

[task.verify.test]
gate = "test"
required = true
rung = 2
```

**Parsing:** `task_parser.rs` → `TasksFile` → `Vec<TaskDef>`. Validates cycles via `detect_cycle_nodes()`. Defaults: status=pending, tier=mechanical, timeout=600, max_retries=2.

**ACP doesn't use tasks.toml** — it works with single prompts, not plan files.

---

## 4. Agent Dispatch (3 Implementations)

| Runtime | Dispatch Path | LOC | Features |
|---|---|---|---|
| ACP | `runner.rs` → `run_claude_cli()` | ~120 | Fast, minimal, no safety layer |
| Runner v2 | `dispatch_v2.rs` → full routing | ~1,000 | Multi-provider, budget, tool translation |
| Orchestrate.rs | `dispatch_agent*()` | ~1,500 | Safety, MCP, warm reuse, custody, enrichment |

**What's in orchestrate.rs dispatch but not Runner v2:**
- SafetyLayer with provenance tracking
- Role system with 8 roles (Implementer, Reviewer, Strategist, etc.)
- MultiAgentPool for warm agent reuse
- Custody audit chain
- Gemini cache client
- Perplexity search integration

**Anti-Pattern #7:** Three dispatch implementations with different error handling, timeout logic, and token counting. Bug fixes only land in one.

---

## 5. Persistence & Resume

### ExecutorSnapshot (.roko/state/executor.json)

```rust
ExecutorSnapshot {
    schema_version: u32,          // v1
    plan_states: HashMap<PlanState>,
    queue_order: Vec<plan_id>,
    conductor_circuit_breaker: Option<...>,
    speculative_executions: HashMap<...>,
    timestamp_ms: u64,
}
```

### Resume by Runtime

| Runtime | Resume Support | Mechanism | Strictness |
|---|---|---|---|
| ACP | None | Crash = loss | N/A |
| Runner v2 | Full | Fingerprint validation + JSONL recovery | Strict — hash-validates every task definition |
| Orchestrate.rs | Full | ExecutorSnapshot load | Legacy — allows task edits between runs |

**Runner v2 resume flow:**
1. Load `.roko/state/run-state.json` fingerprints
2. Hash-validate every task definition against prior run
3. Hard error on mismatch (prevents mid-run edits)
4. Recover JSONL files from partial corruption
5. Reconcile snapshot + JSONL state via `prepare_resume()`

---

## 6. Concurrency & DAG Execution

### ExecutionWave (roko-orchestrator)

```rust
ExecutionWave {
    index: usize,              // 0 = first wave
    tasks: Vec<GlobalTaskId>,  // can run in parallel
    estimated_minutes: u32,
}
```

**DAG construction:** BFS layer-by-layer. Wave 0 has no deps, wave 1 depends only on wave 0, etc.

**File overlap inference:** Optional — tasks touching same file serialize (default: true).

**Speculative execution:** If task exceeds expected duration, spawn backup agent. Two branches race; slower gets cancelled.

**Bottleneck:** `max_concurrent_tasks: 1` means despite DAG and waves, only one task runs at a time by default.

---

## 7. Merge & Worktree Orchestration

### Worktree Manager

Per-plan isolation:
- Each plan gets its own git worktree (isolated branch)
- `WorktreeManager::create(plan_id)` → branch from batch branch
- Idle TTL cleanup (30 min default)
- Health checks verify branch exists and is reachable

### Merge Queue

**Status:** Fully built but never called in event loop. All merges done inline via `PostMergeRunner`.

### Post-Merge Follow-Up

- Validates merge didn't break tests
- Runs gates again on batch branch post-merge
- Can trigger replan if regression detected

---

## 8. Features Only in Dead Code

| Feature | Orchestrate.rs Location | Runner v2 | ACP |
|---|---|---|---|
| Dream consolidation | Line 7589+ | Missing | Missing |
| Daimon affect engine | Line 266+ | Missing | Missing |
| Knowledge routing | `build_knowledge_routing_advice()` | Missing | Missing |
| VCG auction | `vcg_allocate()` | Missing | Missing |
| Custody audit chain | `CustodyLogger` | Missing | Missing |
| Skill extraction | `SkillLibrary::extract()` | Missing | Missing |
| Anophily remediation | `pre_agent_remediation_log_path()` | Missing | Missing |
| C-factor computation | `CFactorSummary` | Missing | Missing |
| 30+ enrichment steps | `estimate_enrichment` | Partial | None |
| Merge queue deadlock detection | Built | Never called | N/A |

**Impact:** These features silently deactivated when switching to Runner v2. 11,000+ lines of logic not yet ported.

---

## 9. Anti-Patterns

| Anti-Pattern | Where |
|---|---|
| **#10 God file** | orchestrate.rs is 21,577 lines — implements features that belong in 6+ crates |
| **#7 Copy between runtimes** | 3 dispatch implementations, 3 gate dispatch paths, 2 state machines |
| **#3 Build another runtime** | Each runtime reimplements plan execution instead of sharing a core |
| **#4 Features in wrong layer** | Enrichment, custody, skill extraction all inline in orchestrate.rs |
| State machine mismatch | PipelinePhase (10 states) vs PlanPhase (14 states) — not interoperable |
| Bottleneck default | `max_concurrent_tasks: 1` despite full DAG infrastructure |

---

## 10. Entry Point Summary

| Command | Primary File | State Machine | Concurrency | Resume |
|---|---|---|---|---|
| `roko run <prompt>` | run.rs | PlanPhase | Serial | None |
| `roko plan run <dir>` v2 | runner/event_loop.rs | PlanPhase + Executor | DAG waves | Strict fingerprints |
| `roko plan run <dir>` legacy | orchestrate.rs | PlanPhase + Executor | Wave iteration | Legacy |
| Zed `/workflow` | acp/runner.rs | PipelinePhase | Serial agents | None |

---

## 11. What WorkflowEngine Should Do

The UNIFIED-IMPLEMENTATION-PLAN calls for a single `WorkflowEngine` (Phase 0.4) that:

1. Uses ACP's pure state machine pattern (`step(event) → action`)
2. Supports Runner v2's DAG execution and wave scheduling
3. Handles all plan phases (PlanPhase enum, 14 states)
4. Dispatches via ModelCallService (unified)
5. Records feedback via FeedbackService
6. Assembles prompts via PromptAssemblyService
7. Runs gates via unified GateService
8. Supports resume with fingerprint validation

Every entry point — `roko run`, `roko plan run`, ACP — goes through this engine.

---

## 12. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-cli/src/orchestrate.rs` | 21,577 | Dead monolith |
| `roko-cli/src/runner/event_loop.rs` | ~3,035 | Active — Runner v2 |
| `roko-cli/src/runner/mod.rs` | ~500 | Active |
| `roko-cli/src/run.rs` | ~500 | Active — oneshot |
| `roko-acp/src/pipeline.rs` | ~400 | Active — ACP state machine |
| `roko-acp/src/runner.rs` | ~969 | Active — ACP executor |
| `roko-acp/src/workflow.rs` | ~200 | Active — workflow templates |
| `roko-orchestrator/src/dag.rs` | ~400 | Active — DAG + wave scheduler |
| `roko-orchestrator/src/worktree.rs` | ~300 | Built — worktree isolation |
| `roko-orchestrator/src/merge_queue.rs` | ~200 | Built — never called |
| `roko-cli/src/task_parser.rs` | ~400 | Active — TOML parsing |
| `roko-core/src/phase.rs` | ~300 | Core — phase enum + transitions |
