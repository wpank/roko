# Orchestration: Patterns and Lessons

Distilled patterns from the mega-parity runner (195 batches, 6 hours, 177K LOC),
the three existing runtimes, and analysis of the orchestrator source code.
These patterns should inform all future orchestration work.

---

## 1. The Worktree Isolation Model

### 1.1 Why Worktrees, Not Branches

Branches alone do not give a working tree. Worktrees give each agent a full
checkout where it can read, write, and optionally compile independently.
The alternative -- agents sharing a single working directory -- causes:

- File corruption when two agents write the same file simultaneously
- Stale reads when agent B reads a file agent A is mid-write
- Git index conflicts when concurrent `git add` operations interleave
- Build cache invalidation when agent A's changes break agent B's incremental
  build

The mega-parity runner used one worktree per batch:
- Each worktree is ~500MB for a 177K LOC repo (source only, no target dir)
- With PARALLEL=15, that is ~7.5GB of working trees
- Worktrees are never deleted automatically -- they are the undo mechanism

### 1.2 The Three-Tier Branch Model

```
source branch (wp-arch2)              <-- developer's working branch
  |-- integration branch              <-- all merges accumulate here
       |-- task-001 branch            <-- one per agent
       |-- task-002 branch
       |-- task-003-backup-20260429   <-- backup on retry
```

**Source branch**: Where development happens. Runner optionally merges back.
**Integration branch**: Where all task merges accumulate. One per run.
**Task branches**: One per task execution, forked from integration branch.

This model is implemented in `WorktreeManager` (`roko-orchestrator/src/worktree.rs`)
with `format_branch_name()` for naming and `create_for_plan()` for allocation.

### 1.3 Serialized Merges

Merges to the integration branch must be serialized. The mega-parity runner
uses a mkdir-based lock with stale PID detection. Roko uses `MergeQueue`
(`roko-orchestrator/src/merge_queue.rs`) with file-overlap-aware serialization.

Why serialization matters:
- Concurrent `git merge` operations corrupt the Git index
- File-overlap detection prevents logical conflicts (two agents modifying the
  same function)
- Post-merge regression gates catch cross-task integration errors

### 1.4 Numbers to Remember

| Metric | Value | Source |
|---|---|---|
| Worktree size (177K LOC) | ~500 MB | Mega-parity runner |
| 15 concurrent worktrees | ~7.5 GB | Mega-parity runner |
| Worktree creation time | ~2 seconds | Git clone + checkout |
| Merge time (serial) | ~5 seconds | Git merge + conflict check |
| Merge conflict rate | ~30% of cherry-picks | Large runs with shared files |

---

## 2. Wave Gating: The Build Deferral Pattern

### 2.1 The Core Insight

> "Compiling after every agent turn is too expensive. Compile after a batch
> of changes accumulates. The trade-off is delayed error detection, but the
> time savings are 10-100x."

Three levels of build deferral:

| Level | What | Speed | Safety |
|---|---|---|---|
| Per-task | Each agent runs `cargo check -p <crate>` | Slowest | Safest |
| Wave gates | `cargo check --workspace` after each wave | Medium | Good |
| Deferred | Only compile at the end | Fastest | Riskiest |

### 2.2 Measured Performance

From the mega-parity runner (195 batches):

| Gate Strategy | Per-Task Time | Total Time | Error Count |
|---|---|---|---|
| Per-task verify | 15-40 min | 50+ hours | 0 (caught immediately) |
| Wave gates only | 3-8 min/wave | ~5 hours | 5-10 (caught at wave boundary) |
| No gates (deferred) | 0 min | ~3 hours | 10-30 (caught at end) |

The wave-gate strategy is the sweet spot: 10x faster than per-task verification
with only 5-10 additional errors to fix at wave boundaries.

### 2.3 How to Identify the Offending Task

When a wave gate fails, determine which task caused the regression:
1. Git log the merge commits in the wave
2. Binary search (git bisect) across the merge commits
3. Revert the offending merge, retry the task with failure context

The DAG infrastructure supports this: `DagExecutionSnapshot` tracks which tasks
are in each wave, and `MergeQueue` records merge order.

### 2.4 The No-Build Context Pattern

The mega-parity runner used context files to prevent agents from building:

```markdown
# Build Policy
**Do NOT run any compilation or test commands.** This includes:
- `cargo check`, `cargo clippy`, `cargo build`, `cargo test`
Focus exclusively on writing correct code.
```

This reduced batch times from 15-40 minutes to 1-5 minutes. Agent compliance
was ~95% -- 5% of agents ignored the instruction and ran builds anyway.

For Roko: implement this as a prompt section injected by EffectDriver when
wave gating is active. Consider also providing a fake cargo binary or
restricted PATH as a fallback for non-compliant agents.

---

## 3. Context Handoff Patterns

### 3.1 The Cumulative Section

The most impactful context pattern from the mega-parity runner: a section
showing what files changed in prior tasks.

Structure:
```markdown
## What Changed Before You

Files modified by prior tasks in this plan:

- `src/gate/compile.rs` (+45 -12): Added run_compile_gate, modified gate_pipeline
- `src/lib.rs` (+3 -0): Added `pub mod gate;`
- `tests/gate_test.rs` (+80 -0): New test file
```

Token budget: ~4000 tokens maximum. For large files, use signature-only views
(function name + parameter types, no body). Truncate oldest task summaries when
exceeding budget.

### 3.2 Progressive Refinement

Multi-pass strategy where each pass builds on the prior:

**Pass 1 (Fast model, mechanical)**: Write initial implementation (80% correct)
- Model: gpt-5.4-mini or similar fast model
- Context: task description + cumulative section
- Expected: compiles, basic logic correct, edge cases missed

**Pass 2 (Strong model, review)**: Review and fix critical issues
- Model: claude-opus-4-6 or similar strong model
- Context: pass 1 output + diff + gate results
- Expected: edge cases handled, error handling added

**Pass 3 (Targeted fix)**: Fix specific gate failures
- Model: same as pass 2
- Context: pass 2 output + specific gate error output
- Expected: all gates pass

PipelineStateV2 supports this natively via its iteration mechanism, but only
drives it via gate failures (reactive), not proactively (multi-pass by design).

### 3.3 Failure Context Accumulation

Each failed attempt should accumulate context for the next attempt:

```rust
pub struct FailureContext {
    pub attempt: u32,
    pub gate_name: String,
    pub gate_output: String,          // raw gate output
    pub diff_from_prior: String,      // what the agent changed
    pub error_pattern: Option<String>, // matched pattern from ErrorPatternStore
    pub suggested_fix: Option<String>, // from error pattern or LLM judge
}
```

PipelineStateV2 already accumulates `review_findings` across iterations. Extend
this pattern to gate failures with structured failure records instead of raw
strings.

### 3.4 Agent-to-Agent Messaging

For multi-role workflows (strategist -> implementer -> reviewer), structured
handoff documents:

**Strategist -> Implementer**:
```markdown
## Strategy Brief
- Approach: Refactor existing `gate_pipeline` to accept configurable gate list
- Key constraint: Must not break existing callers (backward compatible)
- Files to modify: src/gate/mod.rs, src/gate/pipeline.rs
- Files NOT to modify: tests/ (separate task)
- Estimated complexity: focused (2-3 files, clear API boundary)
```

**Reviewer -> Implementer (on revision)**:
```markdown
## Review Findings (must-fix)
1. `gate_pipeline()` at line 47: missing error propagation (? operator)
2. New `GateConfig` struct should derive `Clone` for test compatibility
3. `run_gate()` timeout hardcoded to 60s -- should read from config

## Review Findings (nit)
1. Variable `g` on line 23 should be descriptive (`gate_name`)
2. Missing doc comment on public `GateConfig` struct
```

PipelineStateV2 carries `strategist_brief` and `review_findings` but these
are unstructured strings. Structured handoff documents would improve agent
comprehension.

---

## 4. Failure Recovery Patterns

### 4.1 Gate-Specific Recovery

Not all gate failures are equal. Recovery should be gate-specific:

| Gate | Typical Failure | Best Recovery |
|---|---|---|
| compile (rung 0) | Type mismatch, missing import | Autofix with error output |
| clippy (rung 1) | Unused variable, redundant clone | Autofix (trivial) |
| test (rung 2) | Assertion failure, panic | Reimplement with test output |
| diff (rung 3) | No changes made | Reimplement with stronger prompt |
| fmt (rung 4) | Formatting violation | Autofix (run formatter) |
| custom/shell (rung 5) | Script-dependent | Depends on script |
| judge (rung 6) | LLM review rejection | Revise with findings |

PipelineStateV2 currently treats all gate failures identically (autofix ->
reimplement -> halt). Gate-specific recovery would route compile failures to
a quick autofix agent while routing test failures to a full reimplementation.

### 4.2 Model Escalation

When a cheap model fails repeatedly, escalate to a stronger model:

```
Attempt 1: gpt-5.4-mini (fast, cheap)
  -> compile failure
Attempt 2: gpt-5.4-mini + autofix context
  -> compile failure (same error)
Attempt 3: claude-opus-4-6 (strong, expensive)
  -> success
```

The CascadeRouter already supports this for initial model selection. Failure
escalation would trigger mid-task model switches based on retry count.

### 4.3 The --continue Pattern

> "Any long-running process will crash. The ability to resume from disk state
> (not memory state) is what makes the system reliable."

Requirements for robust resume:
1. **Atomic checkpoints**: write state to tmp file, rename atomically
   (EffectDriver::save_checkpoint already does this)
2. **Fingerprint validation**: hash task definitions to detect mid-run edits
   (Runner v2's TaskDefFingerprint)
3. **JSONL recovery**: reconcile partial writes via line-by-line parsing
   (Runner v2's prepare_resume)
4. **Manual override**: allow humans to mark tasks as completed/skipped via
   file edits (mega-parity runner's .result files)

### 4.4 The Replan Pattern

When a task fails in a way that suggests the plan is wrong (not just the
implementation), trigger replanning:

```rust
pub enum ReplanTrigger {
    /// Same gate fails 3+ times despite autofix
    GateExhausted { gate: String, attempts: u32 },
    /// Task discovers prerequisite work is needed
    MissingDependency { required: String },
    /// Task scope is too large for a single agent
    ScopeTooLarge { estimated_files: usize },
    /// External change invalidates the plan
    ExternalChange { changed_files: Vec<String> },
}
```

The orchestrator has `ReplanStrategy` and `PlanRevisionRequest` in
`roko-orchestrator/src/replan.rs` and `build_gate_failure_plan_revision` in
orchestrate.rs. These need to be wired into WorkflowEngine.

---

## 5. Scheduling Patterns

### 5.1 Critical Path Optimization

The DAG's CPM analysis identifies zero-slack tasks that determine total
execution time. These tasks should receive priority in every decision:

- **Dispatch first**: When multiple tasks are ready, dispatch critical path
  tasks before non-critical ones
- **Speculate**: Pre-dispatch critical path tasks when dependencies are 80%+
  complete
- **Better models**: Route critical path tasks to stronger models (they cannot
  afford to fail and retry)
- **No defer**: Never defer gating for critical path tasks (catch errors early)

### 5.2 Dependency Fan-Out Optimization

Tasks with many downstream dependents should run first because they unblock
the most work:

```
T1 (fans out to T3, T4, T5, T6)  -> priority = 4
T2 (fans out to T7)              -> priority = 1
```

Running T1 before T2 unblocks 4 tasks instead of 1, even if T2 is cheaper.

### 5.3 Wave Width Tuning

`DagConfig::max_wave_width` limits how many tasks can be in a single wave.
Tuning this affects the trade-off between parallelism and merge complexity:

| Width | Effect |
|---|---|
| 0 (unbounded) | Maximum parallelism, most merge conflicts |
| N = max_parallel | Natural bound, no overflow |
| 1 | Fully serial, no conflicts |

The mega-parity runner used PARALLEL=15, which was the sweet spot for 20
API workers. Setting max_wave_width = max_parallel prevents over-scheduling.

### 5.4 Chain Fusion

`UnifiedTaskDag::fuse_linear_chains()` collapses linear sequences:
```
A -> B -> C   becomes   ABC (fused)
```

This reduces wave count and overhead, but fused tasks lose the ability to
checkpoint between subtasks. The FusionConfig controls:
- `max_chain_length`: maximum tasks to fuse (default: reasonable bound)
- `same_tier_only`: only fuse tasks with matching complexity bands
- `ave_width`: minimum average parallelism to maintain after fusion

Fusion is most valuable for mechanical tasks (tier 0) that are naturally
serial but independently small.

---

## 6. Cost Management Patterns

### 6.1 Token Budget Allocation

Given a total token budget for a plan, allocate per-task based on tier:

| Tier | Token Multiplier | Model | Rationale |
|---|---|---|---|
| Mechanical | 1.0x | Fast (gpt-5.4-mini) | Simple, well-defined tasks |
| Focused | 1.5x | Standard (claude-sonnet) | Multi-file, clear boundaries |
| Integrative | 2.5x | Strong (claude-opus-4-6) | Cross-module, complex interactions |
| Architectural | 4.0x | Strongest | System-wide changes, design decisions |

Reserve 20% of budget for retries and autofix. Track actual vs estimated cost
per tier to refine multipliers over time.

### 6.2 The Audit Phase Trade-off

The mega-parity runner's two-pass model:
- **Implementation pass** (fast model): Write code, no compilation
- **Audit pass** (strong model): Verify and fix

With audit enabled, each task takes 2x time but catches more issues. For
large batch runs, disable audit and do a manual audit pass on the merged
result.

Cost comparison for 195 batches:
- With audit: ~$40 total, ~8 hours
- Without audit: ~$20 total, ~4 hours, 30 min fix-up at end

The break-even point depends on the fix-up cost. For mechanical tasks, audit
is rarely worth the 2x overhead. For integrative/architectural tasks, audit
catches errors that are expensive to fix later.

### 6.3 Speculative Execution Cost

Speculative execution wastes tokens when dependencies fail. The expected
waste is:

```
expected_waste = speculation_cost * (1 - dependency_completion_probability)
```

At 80% completion probability: waste is 20% of speculation cost.
At 95% completion probability: waste is 5%.

Only speculate when:
- The task is on the critical path (time savings are proportional to plan
  duration)
- The wasted cost is below `speculative_threshold_multiplier * remaining_budget`
- The dependency completion probability exceeds the threshold

---

## 7. Monitoring and Intervention Patterns

### 7.1 The Dashboard Model

Real-time monitoring requires these data streams:

| Stream | Update Frequency | What |
|---|---|---|
| Task status | On each transition | Per-task phase, progress, duration |
| Wave progress | On task completion | Tasks completed/total in current wave |
| Gate results | On each gate run | Pass/fail per gate, duration |
| Merge status | On each merge | Success/conflict, affected files |
| Cost accumulation | On each model call | Tokens, cost_usd, by model |
| Error rate | Rolling window | failures / (failures + successes) |
| Disk usage | Every 60 seconds | Available space, worktree count |

The StateHub pattern (push-based via watch::Sender) already exists in
Runner v2. WorkflowEngine should emit RuntimeEvent envelopes that the
dashboard can consume.

### 7.2 Manual Intervention Points

> "Manual intervention is a feature, not a bug."

The orchestrator should expose these intervention points:

| Action | Mechanism | Effect |
|---|---|---|
| Pause execution | Cancel token | Stop dispatching new tasks |
| Resume execution | Reset cancel token | Resume from checkpoint |
| Skip a task | TaskScheduler::mark_completed(task_id) | Skip and unblock dependents |
| Retry a task | Reset task status to Ready | Re-dispatch with fresh context |
| Override model | Per-task config override | Use different model for next attempt |
| Inject context | Write to context buffer | Add information for next agent |
| Force merge | Bypass merge queue | Merge worktree directly |
| Kill agent | ProcessSupervisor::kill(pid) | Terminate stuck agent |

The mega-parity runner's `.result` file pattern (write "success" to override
status) is crude but effective. WorkflowEngine should provide a similar
file-based or HTTP-based intervention mechanism.

### 7.3 Health Monitoring

From the mega-parity runner's operational experience:

- **Batch duration monitoring**: Agents that take >3x expected time are likely
  non-compliant (running builds when told not to). Auto-kill after timeout.
- **Merge conflict trending**: If conflict rate exceeds 30%, reduce parallelism
  and tighten file-overlap serialization.
- **API rate limit detection**: Monitor 429 responses and back off proactively.
- **Disk space trending**: Monitor available space every 60 seconds. Pause
  dispatch when below 5GB.

orchestrate.rs has `HeartbeatClock` and `HeartbeatSnapshot` for health
monitoring, but these are not ported to WorkflowEngine.

---

## 8. The Conveyor Belt Pattern

> "Having a separate process that watches for completed work and integrates
> it into your branch means you can keep working while agents generate code."

Three processes running concurrently:

1. **Generator** (WorkflowEngine): dispatches agents, collects results
2. **Integrator** (MergeQueue / auto-pick): merges completed work into
   the integration branch
3. **Reviewer** (human or audit agent): reviews merged work, provides
   feedback

The generator produces work at agent speed. The integrator merges at git
speed. The reviewer reviews at human speed. Each process runs independently
and communicates via the file system (worktrees, branches, result files).

This is the "assembly line" model for AI-assisted development:
- Agents write code in parallel
- A background process integrates completed work
- A human reviews the accumulated result
- Feedback flows back into the next generation of agents

---

## 9. Key Numbers

From operational experience (mega-parity runner + roko self-hosting):

| Metric | Value | Context |
|---|---|---|
| Agent write speed (no build) | 1-5 min/task | gpt-5.4-mini at xhigh reasoning |
| Agent write speed (with build) | 15-40 min/task | Includes cargo check + clippy |
| Wave gate (cargo check) | 3-8 min | Full workspace, 18 crates |
| Wave gate (cargo clippy) | 2-5 min | Additional to check |
| Wave gate (cargo test) | 5-15 min | Full test suite |
| Merge time (serial) | ~5 sec | Git merge per task |
| Cherry-pick time | ~30 sec each | With auto-resolve |
| Worktree creation | ~2 sec | Git worktree add |
| Worktree size | ~500 MB | Source only, no target |
| Target dir size | 3-33 GB | Grows with incremental builds |
| Merge conflict rate | ~30% | Large runs, shared files |
| AP false positive rate | ~2-3% | Mostly AP-10 (localhost) |
| Agent compliance rate | ~95% | 5% ignore no-build instructions |
| Post-run fix-up errors | 10-30 | After 195 batches with no-build |
| Post-run fix-up time | ~30 min | Compile errors + clippy + tests |
| Optimal parallelism | 15 | 20 API workers, MacBook Pro |
| API cost (no audit) | ~$0.10/batch | gpt-5.4-mini input + output |
| Total run cost (195 batches) | ~$20 | Without audit pass |

---

## 10. Decision Matrix: When to Use Each Pattern

| Situation | Pattern | Why |
|---|---|---|
| Small plan (1-5 tasks) | Serial + per-task gates | Low overhead, immediate feedback |
| Medium plan (5-20 tasks) | Parallel + wave gates | Good balance of speed and safety |
| Large plan (20+ tasks) | Parallel + deferred gates | Maximum speed, fix-up at end |
| Mechanical tasks | Fast model, no audit | Cheap, high compliance |
| Architectural tasks | Strong model, with audit | Expensive but worth the accuracy |
| High merge conflict rate | Tighter file-overlap serialization | Prevent wasted work |
| Budget-constrained | Cost-aware scheduling | Cheap tasks first |
| Time-constrained | Critical path optimization | Unblock dependencies first |
| Unreliable agents | Adaptive parallelism | Reduce concurrency on errors |
| Unknown task complexity | Progressive refinement | Start cheap, escalate on failure |

---

## Sources

- `crates/roko-orchestrator/src/dag.rs` -- UnifiedTaskDag, waves, CPM, fusion, mutations
- `crates/roko-orchestrator/src/worktree.rs` -- WorktreeManager, branch naming
- `crates/roko-orchestrator/src/merge_queue.rs` -- MergeQueue, file-overlap serialization
- `crates/roko-orchestrator/src/replan.rs` -- ReplanStrategy, PlanRevisionRequest
- `crates/roko-runtime/src/pipeline_state.rs` -- PipelineStateV2 iteration and recovery
- `crates/roko-runtime/src/effect_driver.rs` -- EffectDriver affect modulation
- `crates/roko-runtime/src/task_scheduler.rs` -- TaskScheduler file exclusion, next_batch
- `crates/roko-runtime/src/process.rs` -- ProcessSupervisor, supervision strategies
- `crates/roko-cli/src/orchestrate.rs` -- Legacy features (heartbeat, knowledge routing)
- `tmp/solutions/runner/LESSONS.md` -- Mega-parity runner operational lessons
