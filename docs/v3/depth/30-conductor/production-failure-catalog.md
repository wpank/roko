# Production Failure Catalog -- All 21 Failures

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 15.
> Source: production batch runs, March-April 2026

---

## 1. Summary

21 production failures across 6 categories. Each failure maps to one or more
Conductor mechanisms (watchers, circuit breaker, diagnosis engine, anomaly
detector, or process supervisor) that detects or prevents recurrence.

### Category Overview

| Category | Issues | Conductor Coverage |
|----------|--------|-------------------|
| State Corruption | #1-4 | Event-sourced state, circuit breaker |
| Data Pipeline | #5, #13-15 | Diagnosis engine, typed pipelines |
| Process Management | #6-9 | ProcessSupervisor, ghost-turn watcher |
| Resource Management | #10-12 | Cost watcher, context pressure watcher, anomaly detector |
| Merge and Coordination | #16-18 | Review loop watcher, spec drift watcher |
| Observability | #19-21 | Efficiency events, structured signals |

---

## 2. State Corruption (Issues #1-4)

These failures share a pattern: mutable state updated non-atomically by multiple
callers, producing states that violate invariants.

### Issue #1: in_flight/completed Overlap

**Symptom**: Tasks appeared in both `in_flight` and `completed_tasks` sets
simultaneously. Downstream logic that assumed mutual exclusivity would double-count
work or skip gating.

**Root Cause**: `snapshot()` returned state with a task in both sets. The transition
from in_flight to completed was not atomic -- the task was added to completed before
being removed from in_flight, and a concurrent snapshot captured the intermediate
state.

**Conductor Response**: Circuit breaker detects repeated failures from corrupted
state. Structural prevention: event-sourced state where `TaskCompleted` is a single
atomic fact.

---

### Issue #2: Orphaned Plans

**Symptom**: Tasks existed in task state but had no corresponding `plan_phase`
entry. The orchestrator tried to advance these tasks but had no phase context,
causing panics or silent drops.

**Root Cause**: Plan creation involved multiple writes -- inserting tasks, then
inserting the plan_phase. A crash between writes left tasks belonging to no plan.

**Conductor Response**: Iteration loop watcher detects repeated failures from
orphaned tasks. Structural prevention: event-sourced state where `PlanCreated`
contains both plan metadata and initial task set.

---

### Issue #3: Branch Divergence

**Symptom**: Worktree branches diverged from the batch branch. Merging back produced
conflicts or quarantine loops.

**Root Cause**: Plan branches were long-lived. The batch branch advanced as other
plans merged. The longer a plan ran, the further its branch diverged.

**Conductor Response**: Review loop watcher detects repeated merge-conflict
rejections. Circuit breaker opens at `MAX_PLAN_FAILURES=2`. Structural prevention:
ephemeral branch model where branches are born from current HEAD and merged or
discarded immediately.

---

### Issue #4: CONTEXT.md Concurrent Appends

**Symptom**: Multiple agents writing to shared CONTEXT.md simultaneously. Content
was interleaved, truncated, or lost.

**Root Cause**: CONTEXT.md was a plain file in the shared worktree. No locking, no
coordination.

**Conductor Response**: Spec drift watcher detects quality degradation from
corrupted context. Anomaly detector catches quality score drops. Structural
prevention: event-sourced context via `context/in/` (read-only) and `context/out/`.

---

## 3. Data Pipeline (Issues #5, #13-15)

These failures share a root cause: LLM-generated artifacts consumed without
validation.

### Issue #5: Counter Bug (TOML Fences)

**Symptom**: `task_weighted_progress` reported ~2.5% when actual completion was
much higher. ETA stuck at 8 hours. 388 of 544 task files affected.

**Root Cause**: Enrichment LLM (Haiku) wrapped TOML in markdown code fences. TOML
parser returned `Err`, which was silently swallowed. Empty checklists produced wrong
progress fractions.

**Conductor Response**: Diagnosis engine includes `TomlParsing` as an error
category. Structural prevention: schema validation at generation time -- parse
generated TOML immediately, reject/retry on failure.

---

### Issue #13: Enrichment TOML Fences

**Symptom**: Identical to Issue #5 from the pipeline perspective. 388/544
enrichment-generated TOML files wrapped in markdown fences.

**Root Cause**: LLMs trained on chat data wrap structured output in code fences even
when told not to.

**Conductor Response**: Same as Issue #5 -- diagnosis engine detects the pattern,
schema validation prevents recurrence.

---

### Issue #14: Verify Script Stale References

**Symptom**: Verify scripts referenced packages, modules, functions that did not
exist. Scripts failed with "not found" during gate phase, failing correct
implementations.

**Root Cause**: Enrichment LLM hallucinated plausible package names and function
signatures. Scripts were not validated against the codebase at generation time.

**Conductor Response**: Compile-fail-repeat watcher fires at `MAX_COMPILE_FAILS=3`.
Diagnosis engine matches `E0432` (unresolved import) and `E0433` (unresolved path).
Structural prevention: dry-run validation at enrichment time.

---

### Issue #15: Review Verdict Parsing

**Symptom**: Review verdicts parsed incorrectly. Plans that should have passed were
failed, and vice versa.

**Root Cause**: Reviewers output TOML in markdown. The regex fallback parser was
fragile and confused by similar patterns in commentary.

**Conductor Response**: Review loop watcher fires at `MAX_REVIEW_CYCLES=3`.
Structural prevention: typed review pipeline with `ReviewReport` struct and
schema-validated JSON.

---

## 4. Process Management (Issues #6-9)

These failures stem from treating agent processes as fire-and-forget.

### Issue #6: Spawn Races

**Symptom**: Agents exited with near-zero output. Retry fired instantly. Exit event
from attempt N confused with attempt N+1 -- killing the new attempt or
double-counting the failure.

**Root Cause**: No attempt tracking. Exit events did not identify which spawn
attempt they belonged to. Retries fired without backoff.

**Conductor Response**: Ghost-turn watcher detects near-zero output agents at
`MAX_GHOST_TURNS=3`. ProcessSupervisor provides monotonically increasing attempt IDs
so stale events are ignored structurally.

---

### Issue #7: Orphaned Cargo Processes

**Symptom**: Timeout killed the shell script but not the cargo process tree.
Orphaned cargo processes accumulated, starving CPU and memory.

**Root Cause**: `kill(pid)` does not kill descendants unless they are in the same
process group.

**Conductor Response**: Cost overrun watcher fires as orphaned processes indirectly
increase turn costs. ProcessSupervisor provides `kill_all_descendants(pid)` with
bottom-up kill ordering and periodic orphan reaper sweep.

---

### Issue #8: Claude CLI Cold Start

**Symptom**: Every agent turn took 2-5s startup overhead. Over hundreds of turns,
10-30 minutes of pure waste.

**Root Cause**: CLI spawns a new subprocess per turn. No persistent connection or
subprocess reuse.

**Conductor Response**: Time overrun watcher detects cumulative cold start overhead.
Efficiency events capture per-turn `time_to_first_token_ms` and `wall_time_ms`.
Structural prevention: agent connection pooling.

---

### Issue #9: Agent Ghost Turns

**Symptom**: Agent appeared active but produced no useful output -- repeating
itself, asking clarifying questions to nobody, or describing what it would do
without doing it. Burned significant token budget.

**Root Cause**: LLM agents enter degenerate loops when context is confusing,
instructions ambiguous, or errors unhandled.

**Conductor Response**: Ghost-turn watcher (primary) fires at `MAX_GHOST_TURNS=3`.
Stuck-pattern watcher detects repetitive output at `MAX_STUCK_PATTERNS=4`. Anomaly
detector catches prompt loops (5 identical hashes in 20-prompt window) and cost
spikes (z-score > 3.0).

---

## 5. Resource Management (Issues #10-12)

These failures arise from treating resources as unlimited.

### Issue #10: Disk Pressure

**Symptom**: Build failures with cryptic errors. Only 7.3 GB free on a 1.8 TB
drive. Cargo target directories and worktree copies had accumulated.

**Root Cause**: No proactive disk monitoring. Multiple worktrees each with their
own target directory. GC only ran when explicitly triggered.

**Conductor Response**: Health monitor extended with disk pressure checks. Anomaly
detector catches budget exhaustion. Structural prevention: DiskBudget estimates disk
footprint before starting a plan and refuses if budget exceeds available space.
`roko doctor disk` now provides proactive diagnostics.

---

### Issue #11: Gate Serialization Bottleneck

**Symptom**: Plans completed implementation quickly but waited in queue for gate
verification. Serialized gate processing, one at a time.

**Root Cause**: Double semaphore (`cargo_gate` + `verify_chain`, both with 1 permit)
serialized all compilation. After worktree isolation, separate target directories
made serialization unnecessary.

**Conductor Response**: Time overrun watcher makes the bottleneck visible when gate
queue waiting pushes phase time past 80%. Efficiency events reveal queue wait time
through `wall_time_ms` vs `duration_ms` gap.

---

### Issue #12: Memory Pressure from Large Prompts

**Symptom**: Agent output quality degraded as prompt size increased. Agents ignored
relevant context buried in large prompts or fixated on irrelevant context.

**Root Cause**: "Include everything" prompt strategy. Prompts exceeding 100K tokens.
LLM attention is not uniform -- middle content gets less attention.

**Conductor Response**: Context window pressure watcher fires at 80% of model's
context window (primary defense). Spec drift watcher catches quality degradation
from oversized prompts. Structural prevention: adaptive context dropping via
SystemPromptBuilder signal_ratio scoring.

---

## 6. Merge and Coordination (Issues #16-18)

These failures arise from concurrent plans interacting through shared branches
and files.

### Issue #16: Rebase Failures

**Symptom**: "batch rebase failed" permanently killed plans. Work was lost with no
recovery.

**Root Cause**: Long-lived branches needed rebasing onto advancing batch branch.
Rebase failure was treated as permanent rather than recoverable.

**Conductor Response**: Iteration loop watcher fires at `MAX_ITERATIONS=3` with
Critical severity. Circuit breaker opens at `MAX_PLAN_FAILURES=2`. Structural
prevention: ephemeral branches that are never rebased.

---

### Issue #17: Merge Conflicts at Gate

**Symptom**: Two plans that both passed gates individually would conflict when
merged. Second plan fails, retries, fails again -- loop.

**Root Cause**: Plans scheduled without considering file overlap. Both succeed in
isolation but conflict when combined.

**Conductor Response**: Compile-fail-repeat watcher fires when merge conflicts
produce compile errors. Circuit breaker catches the retry-conflict-retry loop.
Structural prevention: dependency graph with pre-merge conflict detection.

---

### Issue #18: Worktree Symlinks to Shared State

**Symptom**: Race conditions when multiple agents accessed shared state through
symlinks. Changes by one agent affected another's view.

**Root Cause**: Worktrees had symlinks to shared mutable files. Writes from any
worktree mutated the same file.

**Conductor Response**: Spec drift watcher detects output drift from corrupted
shared state. Stuck-pattern watcher catches repetitive confused output. Structural
prevention: full worktree isolation with no shared mutable state.

---

## 7. Observability (Issues #19-21)

These failures share a theme: insufficient information to diagnose problems quickly.

### Issue #19: Buried Failures in Logs

**Symptom**: Critical errors hidden in 50,000-line log files. Required manual grep
to find failures.

**Root Cause**: Unstructured logging. All events to same stream with no severity
routing or queryability.

**Conductor Response**: Structured `AgentEfficiencyEvent` with 20+ fields replaces
unstructured logging. Every conductor intervention produces a typed `Signal` with
severity, watcher name, plan ID, and count -- queryable, not buried.

---

### Issue #20: No Signal on WHY Plans Fail

**Symptom**: TUI showed "Failed" with no root cause. Operator had to dig through
logs, worktree state, and git history.

**Root Cause**: Failure path recorded status change but not reason. Error messages
logged but not attached to plan state.

**Conductor Response**: Diagnosis engine classifies errors into 20 categories with
suggested interventions, providing the "why" that was missing. Conductor
intervention signals include watcher name, severity, and descriptive message.

---

### Issue #21: ETA Completely Wrong

**Symptom**: ETA showed 8+ hours when actual remaining was ~2 hours. Progress bar at
~2.5% when actual completion was ~40%.

**Root Cause**: Directly caused by Issue #5. Weighted progress depended on checklist
counts from broken TOML files.

**Conductor Response**: Anomaly detector detects internal inconsistency (40% gates
passed but 2.5% progress shown). Efficiency events provide `gate_passed` as a
reliable progress signal. Structural prevention: progress derived from gate outcomes,
not checklist parsing.

---

## 8. Cross-Reference: Issue to Conductor Mechanism

| # | Issue | Primary Mechanism | Secondary Mechanism |
|---|-------|------------------|-------------------|
| 1 | in_flight/completed overlap | Circuit breaker | Event-sourced state |
| 2 | Orphaned plans | Iteration loop watcher | Event-sourced state |
| 3 | Branch divergence | Review loop watcher | Circuit breaker |
| 4 | CONTEXT.md concurrent appends | Spec drift watcher | Quality anomaly detector |
| 5 | Counter bug (TOML fences) | Diagnosis engine | Schema validation |
| 6 | Spawn races | Ghost-turn watcher | ProcessSupervisor |
| 7 | Orphaned cargo processes | ProcessSupervisor | Cost watcher |
| 8 | Claude CLI cold start | Time overrun watcher | Efficiency events |
| 9 | Agent ghost turns | Ghost-turn watcher | Prompt loop detector |
| 10 | Disk pressure | Health monitor | Budget anomaly |
| 11 | Gate serialization | Time overrun watcher | Efficiency events |
| 12 | Large prompt pressure | Context pressure watcher | Spec drift watcher |
| 13 | Enrichment TOML fences | Diagnosis engine | Schema validation |
| 14 | Verify script stale refs | Compile-fail-repeat watcher | Diagnosis engine |
| 15 | Review verdict parsing | Review loop watcher | Typed review pipeline |
| 16 | Rebase failures | Iteration loop watcher | Circuit breaker |
| 17 | Merge conflicts at gate | Compile-fail-repeat watcher | Circuit breaker |
| 18 | Worktree symlinks | Spec drift watcher | Stuck-pattern watcher |
| 19 | Buried failures | Efficiency events | Conductor signals |
| 20 | No failure signal | Diagnosis engine | Conductor signals |
| 21 | ETA wrong | Anomaly detector | Efficiency events |

---

## 9. Cross-Reference: Issue to Design Principle

| Principle | Prevents Issues |
|-----------|----------------|
| #1 Single source of truth | #1, #2, #4, #18 |
| #2 Event-sourced state | #1, #2, #4 |
| #3 Ephemeral everything | #3, #16 |
| #4 Typed pipelines | #5, #13, #14, #15 |
| #5 Fail loud, recover fast | #1, #2, #5, #6, #19, #20, #21 |
| #6 Resource budgets | #10, #11, #17 |
| #7 Process isolation | #4, #6, #7, #9, #18 |
| #8 Measure everything | #8, #9, #11, #12, #19, #20, #21 |
| #9 Immutable artifacts | #4, #15 |
| #10 Monotonic progress | #3, #16 |
| #11 Anticipate, don't react | #9, #10, #12, #14, #17, #21 |

---

## 10. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/watchers/` | Watcher mechanisms referenced throughout |
| `crates/roko-conductor/src/circuit_breaker.rs` | Circuit breaker responses |
| `crates/roko-conductor/src/diagnosis.rs` | Error classification for data pipeline failures |
| `crates/roko-conductor/src/health.rs` | System health checks for resource failures |
| `crates/roko-learn/src/anomaly.rs` | Anomaly detection for quality and cost failures |
| `crates/roko-runtime/` | Process supervision for process management failures |
