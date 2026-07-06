# Roko Orchestration: Parallel Execution & The Unified DAG

> **Audience**: Systems architects, orchestration engineers, distributed systems devs
> **Scope**: Extended target-state specification mapping Roko's dynamic file-locked Task DAG, Wave pipelining, and Merge Checkpoint integrity scaling.

---

In classical LLM execution suites, operations are sequential. Agent 1 builds Plan 1. The framework halts, merges, and proceeds to Agent 2 to build Plan 2. Consequently, if Plan 1 has five files and Plan 2 relies on an entirely discrete API namespace, vast processing time is lost waiting for synchronous execution to conclude.

Roko abandons plan-level orchestration and transitions entirely to **Task-Level Graph Serialization**.

## 1. The Unified File-Conflict DAG

Through Roko's deterministic enrichment pipeline, every Plan is atomized into Tasks mapped to exact implementation requirements (`tasks.toml`).

These globally identifiable tasks (`GlobalTaskId: 03:T1`) are evaluated by the continuous topological scheduler via four checks:
1. Is the task already completed?
2. Is the task already in flight?
3. Are there unmet precursor plan dependencies restricting this wave?
4. **Do any files tracked inside this specific Task intersect with the union-set of all files currently checked out by in-flight Agents?**

When two disjoint tasks request simultaneous processing on non-overlapping source matrices, Roko spawns their Agents natively in parallel. Across large migrations, a structural wave that historically demanded 8 hours of serialized processing collapses to under 45 minutes as 12 models concurrently manipulate mutually-exclusive zones.

## 2. Robust File Isolation: Git Worktrees

The DAG prevents logical target collisions, but native repositories still require hard partition spaces. Roko utilizes **Git Worktrees** instead of primitive branches or sprawling mult-clone clones.
A worktree shares the centralized hidden `.git` structure, meaning 20 separate 50-MB operations only consume the local tracking overlay footprint, not a 1+ GB memory surge.

### The SCCache Bottleneck Loop
Because every isolated git worktree produces a localized `target/` artifact directory for Rust evaluation, sequential parallel execution forces catastrophic 30+ second dependency recompilations on parallel gates. 

Roko circumvents this mathematically:
* Every spawned execution sets `CARGO_INCREMENTAL=0` and directs `SCCACHE_BASEDIRS` dynamically globally over the root instance. 
* Rather than the compiler flagging the modified workspace boundary as uncacheable dynamically, SCCache correctly fingerprints identical crate layers across parallel branches. The `reqwest` or `golem-core` models compile instantly via <100ms artifact pulls instead of full compilation passes.

## 3. Cybernetic Warm Pools

Cold-spawning an LLM process requires initializing connections, loading context boundaries, parsing parameters, and spinning the system context (~5-15 seconds). 
During Phase execution, waiting out these dead intervals compounds dangerously across thousands of loops.

While an Implementer is fighting a compile cycle, Roko pre-emptively caches **Warm Agent Subnets**. Reviewer branches invoke `MultiAgentPool::pre_spawn_warm()`, preloading models globally on idle. The moment the Compile Gate drops success, Roko switches from the cold loop straight to native execution evaluation, totally erasing the 10-15s dead-time payload transfer parameter. If the gate fails, the buffer memory drops gracefully with 0 token waste.

## 4. Checkpoints & Merge Resolution

Merging independent models backwards onto the Integration Batch requires rigid dependency lockstepping. Roko ensures safety using **Merge Checkpoints** stored persistently on JSON logic.

If an orchestration fault forces an application crash during a parallel Worktree sequence, rebooting reads `task-state.json`. Finding a broken `MERGE_HEAD` state immediately triggers `git merge --abort`, gracefully restoring the topological map without corrupting the batch layer branch.

When a collision on merge happens deterministically, `MergeResolver` Agents are tasked structurally directly inside the faulting local worktree environment parsing diff paths without touching the primary orchestrator pipeline to minimize branch contamination.

---

## 5. Detailed Mechanisms

### 5.1 File-Conflict Detection Algorithm

The `MergeQueue` maintains `locked_files: BTreeMap<String, String>` mapping each file path to the owning `plan_id`. When a new merge candidate arrives, the algorithm checks for set intersection:

```rust
fn next_mergeable(&self) -> Option<MergeRequest> {
    for plan_id in &self.order {
        let entry = &self.entries[plan_id];
        if entry.status != MergeStatus::Queued { continue; }
        let files: HashSet<&str> = entry.request.files_changed.iter().map(String::as_str).collect();
        let conflicts = self.locked_files.keys().any(|f| files.contains(f.as_str()));
        if !conflicts { return Some(entry.request.clone()); }
    }
    None
}
```

Complexity: O(F_candidate × F_locked) per candidate. With typical plan file sets of 5-30 files and ≤4 concurrent merges, this is effectively constant time. `BTreeMap` provides deterministic iteration order for logging.

### 5.2 Graham's Bound for DAG Scheduling

The topological scheduler partitions tasks into waves (levels where all predecessors are complete). Graham's bound (1966) gives the theoretical makespan guarantee:

```
makespan ≤ W/P + D
```

Where W = total work (sum of task durations), P = parallel agents, D = critical path (longest dependency chain).

**Concrete example**: W = 96 hours total work, P = 12 agents, D = 3 hours critical path:
```
makespan ≤ 96/12 + 3 = 11 hours
```

With file conflicts adding serialization time C: `makespan ≤ W/P + D + C`. With well-structured plans (tasks scoped to disjoint crate boundaries), C is typically negligible.

The cited "8 hours sequential → 45 minutes" scenario: W = 8 hours, D ~ 15 min, P = 12:
```
makespan ≤ 480/12 + 15 = 55 minutes (45 min observed — shorter tasks free agents early)
```

### 5.3 Warm Pool State Machine (Detailed)

The `MultiAgentPool` implements a three-state lifecycle:

```
           pre_spawn_warm()
                |
                v
          +-----------+       evict_warm()
          | WarmIdle  |-------------------------> [Dropped]
          +-----------+
                |
     promote_warm() / promote_warm_if_capacity()
                |
                v
          +-----------+       run_task() → Done/Failed
          |  Active   |-------------------------> [Running → Terminal]
          +-----------+
                ^                                      |
                |    recycle_terminal_to_warm()         |
                +<-------------------------------------+
```

**Concurrency limit**: Default 4 per role, configurable via `set_concurrency_limit(role, limit)`. `at_capacity(role)` checks before promotion.

**Kill operations**: Immediate and synchronous. `kill_all(deadline)` sweeps active first, then warm. `KillReport` tracks `killed_active + killed_warm + aborted`.

### 5.4 sccache Cross-Worktree Performance

Each worktree produces its own `target/` directory. Without shared caching, 12 worktrees recompile the full dependency tree independently (~30-90s penalty per gate).

Configuration:
```bash
CARGO_INCREMENTAL=0          # Defeats sccache fingerprinting
SCCACHE_BASEDIRS=/repo/root  # Normalize paths across worktrees
```

Because sccache uses content-addressed hashing (preprocessed source + compiler flags), identical dependency crates compile to the same hash regardless of which worktree triggered the build.

- **Cache hit rate**: 98%+ (only 1-3 modified crates miss per agent)
- **Per-gate latency**: <100ms artifact pulls vs 30+ second full compilations
- **Disk savings**: 20 worktrees share one cache layer instead of duplicating 1GB+

### 5.5 MergeCheckpoint JSON Schema

```json
{
  "plan_states": {
    "46-reputation-engine": {
      "phase": "executing",
      "current_task_idx": 3,
      "completed_tasks": ["T1", "T2", "T3"],
      "failed_tasks": [],
      "retry_counts": { "T3": 1 },
      "worktree_path": "/repo/.claude/worktrees/plan-46",
      "branch_name": "roko/plan-46-reputation-engine",
      "files_changed": ["crates/roko-core/src/reputation.rs"],
      "merge_status": null
    }
  },
  "queue_order": ["46-reputation-engine", "47-cascade-router"],
  "timestamp_ms": 1712764800000
}
```

Recovery invariants:
- If `merge_status` is `"merging"` with `MERGE_HEAD` present → `git merge --abort` before restart
- Completed tasks never re-executed
- `queue_order` preserves priority across restarts

### 5.6 The MergeResolver Agent

When `git merge` produces conflicts, a specialized `MergeResolver` agent is spawned in the faulting worktree (not the primary orchestrator's directory) with constrained context:

1. `git diff --diff-filter=U` (only conflicted files)
2. Plan's task description (intent)
3. Base branch version of conflicted files
4. Feature branch version

The MergeResolver handles **semantic conflicts** (branch A renamed a function, branch B added a caller of the old name) — not just textual overlap. It operates exclusively inside the faulting worktree to prevent batch branch contamination.

After `MAX_RETRIES = 5` consecutive failures, the plan is moved to `failed_plans` and file locks are released, unblocking serialized plans behind it.

### 5.7 Priority Demotion on Retry

Effective priority incorporates retry history: `effective_priority = priority.saturating_sub(retry_count)`. A plan with base priority 12 that has retried 3 times drops to effective 9, letting fresher plans merge first. The queue is rebuilt on every mutation via `rebuild_order()`.

---

## 6. The Executor State Machine (14-Phase)

Each plan progresses through a state machine with 14 possible phases:

```rust
pub enum PlanPhase {
    Implementing,    // Agent writing code
    Gating,          // Running compile/test/lint gates
    Verifying,       // Running symbol/generated/property gates
    Reviewing,       // Architect + Auditor + Scribe examining code
    Done,            // All gates passed, ready for merge
    Merging,         // In merge queue, being applied to batch branch
    Complete,        // Successfully merged
    AutoFixing,      // AutoFixer addressing syntactic gate failures
    Failed(FailureKind), // Terminal failure state
}
```

**Transitions**:
```
Implementing → Gating (agent completes turn)
Gating → Verifying (compile + test pass)
Gating → AutoFixing (syntactic failure detected)
AutoFixing → Gating (fix applied, re-run gates)
Verifying → Reviewing (all verification rungs pass)
Reviewing → Done (reviewers approve)
Done → Merging (plan reaches head of merge queue)
Merging → Complete (merge successful)

Any phase → Failed (retries exhausted, budget exceeded, deadlock, etc.)
```

### The ExecutorAction Enum (17 Actions)

The parallel executor processes a queue of actions:

```rust
pub enum ExecutorAction {
    CreatePipeline,           // Set up new plan for execution
    EnsureWorktree,          // Create or verify git worktree
    SpawnTaskAgent,          // Start single agent for task
    SpawnTaskAgentBatch,     // Start multiple agents for independent tasks
    SpawnImplementer,        // Start implementer with full context
    RunPlanGates,            // Execute gate pipeline
    PreSpawnWarmReviewer,    // Pre-spawn reviewer while implementer works
    RunPlanReviews,          // Start review agents
    CancelActiveReviewer,   // Kill reviewer if gates fail
    MergePlanToBatch,        // Merge plan branch into batch branch
    RebasePlanBranch,        // Rebase plan branch onto updated batch
    SpawnRefactorer,         // Start refactoring agent
    SpawnDocVerifier,        // Verify documentation
    RunIntegrationTests,     // Run end-to-end tests
    RunPostMergeRegression,  // Verify nothing broke after merge
    AutoFixErrors,           // Route syntactic errors to AutoFixer
    CleanRetryPlan,          // Reset plan state for retry
    ReGatePlan,              // Re-run gates after fix
    PlanTimeout,             // Handle plan that exceeded time budget
    ForceAdvancePlan,        // Operator override to skip phase
}
```

### The Ephemeral Branch Model (Recommended)

Rather than maintaining long-lived plan branches that diverge from main:

```rust
fn plan_lifecycle(plan: &Plan) -> Result<()> {
    let branch = create_branch_from_head(&plan.id)?;
    let result = run_agent_on_branch(&branch, &plan)?;
    match result {
        GatePassed => merge_to_main(&branch)?,
        GateFailed => discard_branch(&branch)?,
    }
    // Branch never lives long enough to diverge
    Ok(())
}
```

**Benefits**: No rebase failures, no stale branches, no divergence headaches. Each plan starts from HEAD and either merges immediately or is discarded.

---

## 7. Discovered Patterns (Stigmergic Coordination)

Agents share knowledge via a file, not direct communication:

```json
// .mori/runs/discovered-patterns.json (max 20 entries, FIFO)
[
  { "plan": "46", "error_signature": "E0433: unresolved module", "discovered_at": "2026-04-09T14:30:00Z" },
  { "plan": "47", "error_signature": "lifetime mismatch in generic impl", "discovered_at": "2026-04-09T14:35:00Z" }
]
```

**Mechanism**: Agent B reads this before starting its task. If the same error signature appears in its output, it can preemptively apply the fix that Agent A discovered. No direct communication needed.

**Limits**: Max 20 entries prevents unbounded growth. FIFO eviction ensures freshness. Limited to error patterns today; could expand to successful strategies.

**Research**: Grassé (1959) — stigmergy. Ants coordinate through pheromone deposits. `discovered-patterns.json` is a digital pheromone trail.

---

## 8. The Batch Task Spawning Optimization

When a plan has N small tasks in independent file-conflict groups, spawning them one-at-a-time wastes cold-start time:

```
Naive: Spawn Agent 1 (5s) → Run (30s) → Kill → Spawn Agent 2 (5s) → Run (30s) → Kill → ...
Total: N × (5s + 30s) = N × 35s

Batch: Spawn Agents 1-4 simultaneously (5s shared) → Run all (30s parallel) → Kill all
Total: 5s + 30s = 35s (regardless of N, up to concurrency limit)
```

For a 20-task wave with 4 concurrent agents: **175s** (batch) vs **700s** (sequential) = **4× speedup**.

The `SpawnTaskAgentBatch` executor action handles this:
1. Group tasks by file-conflict partition (union-find)
2. Select up to `max_agents` tasks from independent groups
3. Spawn all simultaneously (shared cold-start overhead)
4. Each agent works on its assigned task in its own worktree
5. As agents complete, promote next tasks from queue

---

## 9. Ephemeral vs Long-Lived Branches

### The Divergence Problem

Long-lived plan branches accumulate drift from main:
- Other plans merge to main → plan branch is now behind
- Rebase required → can fail if changes overlap
- Failed rebase → manual intervention → blocked pipeline

### The Ephemeral Branch Solution

```
For each task:
  1. Create branch from current HEAD
  2. Run agent (modify files)
  3. Run gates (verify changes)
  4. If pass: fast-forward merge to main
  5. If fail: discard branch entirely

Branch lifetime: 5-60 minutes (never diverges)
```

**Benefits**:
- No rebase ever needed (branch starts from HEAD)
- No stale branches (discarded on failure)
- No merge conflicts from drift (always current)
- Merge queue simplified (fast-forward only)

**Tradeoff**: Each task starts from a fresh base, so it can't see uncommitted work from earlier tasks in the same wave. This is handled by dependency ordering — tasks that need prior work's output wait for the merge.

---

## 10. The .mori/runs/ Directory (Per-Run State)

Each orchestration run produces state in `.mori/runs/{run_id}/`:

```
.mori/runs/20260409-143000/
├── task-state.json              # Executor checkpoint (resumable)
├── metrics.jsonl                # Per-task metrics (append-only)
├── discovered-patterns.json     # Shared error patterns (max 20)
├── iteration-memory/
│   ├── plan-46-memory.json     # Iteration feedback for plan 46
│   └── plan-47-memory.json     # Iteration feedback for plan 47
├── worktrees/
│   ├── plan-46/                # Physical git worktree
│   └── plan-47/
└── agent-pids.json              # PID registry for cleanup
```

**Lifecycle**: Run state is kept until explicitly cleaned (`roko clean runs --older-than 7d`). Worktrees are never automatically deleted — the operator may need them for inspection or debugging.

---

## 11. The Batch Task Spawning Optimization (Detailed)

### Why One-at-a-Time Is Wasteful

The naive executor treats each task as a fully independent lifecycle: spawn a fresh agent process, wait for it to initialize, run the task, tear it down, then repeat. Every agent incurs a cold-start penalty (process launch, context loading, system prompt parsing, provider handshake). When tasks are small, the overhead dominates:

```
Naive sequential (20 tasks, 1 agent):
  Spawn Agent 1 (5s cold start) → Run (30s) → Kill
  Spawn Agent 2 (5s cold start) → Run (30s) → Kill
  ...
  Spawn Agent 20 (5s cold start) → Run (30s) → Kill
  Total: 20 × 35s = 700s
```

Even with concurrency, if agents are spawned one-at-a-time as predecessors complete, the cold-start tax is paid sequentially:

```
Sequential spawn with 4 concurrent slots:
  Wave 1: Spawn A1 (5s) → Spawn A2 (5s) → Spawn A3 (5s) → Spawn A4 (5s) → Run all (30s) → Kill all
  Wave 2: Spawn A5 (5s) → Spawn A6 (5s) → ...
  Total: 5 waves × (20s spawn + 30s run) = 250s
```

### The Batch Solution

`SpawnTaskAgentBatch` eliminates staggered cold starts by spawning all agents in a wave simultaneously. The operating system parallelizes process creation, and provider connections are established concurrently:

```
Batch spawn with 4 concurrent slots:
  Wave 1: Spawn A1-A4 simultaneously (5s shared) → Run all (30s) → Kill all
  Wave 2: Spawn A5-A8 simultaneously (5s shared) → Run all (30s) → Kill all
  ...
  Total: 5 waves × 35s = 175s
```

For a 20-task wave with 4 concurrent agents: **175s (batch) vs 700s (sequential) = 4x speedup**. The savings scale with task count because the cold-start cost is amortized across the entire batch rather than paid per-agent.

### The File-Conflict Partition Algorithm

Not all tasks can run simultaneously. Two tasks touching the same file would produce conflicting writes. The batch spawner partitions tasks into conflict-free groups using union-find:

```
Algorithm: SpawnTaskAgentBatch
  1. Build file-conflict graph:
     - Nodes = tasks
     - Edge between T_i and T_j if files(T_i) ∩ files(T_j) ≠ ∅
  2. Compute connected components via union-find
     - Each component = a set of mutually conflicting tasks
     - Tasks in DIFFERENT components can run simultaneously
  3. Select up to max_agents tasks from independent components:
     - One task per component (no two selected tasks share files)
     - Priority: smallest component first (reduce serialization bottleneck)
  4. Spawn all selected tasks simultaneously
  5. As agents complete:
     - Remove completed task from its component
     - If component has remaining tasks, promote the next one
     - If new components become single-task, they can join the next batch
```

The union-find structure is O(n * alpha(n)) for n tasks, effectively linear. Rebuilding it per wave is negligible compared to the 5-30s agent runtimes.

### Interaction with Warm Pools

Batch spawning and warm pools are complementary. The warm pool pre-spawns agents before the batch is ready; `SpawnTaskAgentBatch` then promotes warm agents to active in bulk rather than one-at-a-time. The combined effect: zero cold-start overhead when warm pool capacity matches the batch size.

---

## 12. Ephemeral vs Long-Lived Branches (Extended Analysis)

### The Divergence Problem in Detail

Long-lived branches are the default in most git workflows, but they create compounding problems in automated pipelines:

1. **Drift accumulation**: While Agent A works on branch-plan-46, plans 40-45 merge to main. Branch-plan-46 is now 6 merges behind. Every merged plan potentially touches files that plan-46 also modifies.

2. **Rebase cascade**: To merge plan-46, it must first rebase onto current main. If plan-43 renamed a function that plan-46 calls, the rebase produces conflicts. The automated MergeResolver agent must be invoked. If that fails, the entire plan blocks.

3. **Conflict probability scales quadratically**: With N concurrent long-lived branches, each touching F files on average, the probability of at least one conflict grows as O(N^2 * F^2 / total_files^2). For 10 branches touching 5 files each in a 500-file codebase: ~40% chance of at least one conflict per merge cycle.

4. **Stale branch accumulation**: Failed plans leave orphaned branches. Over 100+ plans, the repository accumulates dozens of stale branches that confuse `git branch` listings and waste worktree disk space.

### The Ephemeral Branch Solution (Detailed)

```
For each task:
  1. git checkout -b roko/ephemeral/{task_id} HEAD    # Branch from current HEAD
  2. Run agent in worktree (modify files)               # 30s-5min
  3. Run gate pipeline (compile, test, lint, etc.)       # 15s-5min
  4. If all gates pass:
       git merge --ff-only roko/ephemeral/{task_id}     # Fast-forward merge
       git branch -d roko/ephemeral/{task_id}           # Delete immediately
  5. If any gate fails:
       git branch -D roko/ephemeral/{task_id}           # Discard entirely
       # Retry creates a NEW branch from NEW HEAD

Branch lifetime: 5-60 minutes. Maximum divergence: 0 commits behind HEAD at creation time.
```

### Why Fast-Forward Only Matters

A fast-forward merge means the branch tip is a direct descendant of the merge target. No merge commit is created, no three-way merge is needed, and the operation cannot produce conflicts. This is only possible because the branch was created from HEAD moments ago and no other merges happened in the interval.

If another task merges between branch creation and merge attempt, the fast-forward fails. The solution is simple: discard the branch, create a new one from the updated HEAD, and re-run. Because task runtimes are short (30s-5min), the probability of collision is low, and the retry cost is bounded.

### Benefits Over Long-Lived Branches

| Property | Long-Lived | Ephemeral |
|---|---|---|
| Rebase needed | Always (accumulates drift) | Never (created from HEAD) |
| Merge conflicts from drift | Common (quadratic scaling) | Impossible (fast-forward only) |
| Stale branch cleanup | Manual or cron job | Automatic (deleted on completion or failure) |
| Branch lifetime | Hours to days | Minutes |
| MergeResolver invocations | Frequent | Zero (no three-way merges) |
| Retry cost | High (must re-resolve conflicts) | Low (just re-create from HEAD) |

### Tradeoff: Wave Visibility

The one limitation: tasks in the same wave cannot see each other's uncommitted work. If Task B depends on a type that Task A introduces, Task B must wait for Task A to merge before it can branch. This is handled by the dependency DAG — the topological sort naturally serializes dependent tasks while parallelizing independent ones.

---

## 13. The .mori/runs/ Directory: Per-Run State (Extended)

### What Each Orchestration Run Produces

Every invocation of `roko plan run` creates a run directory at `.mori/runs/{run_id}/` containing the complete state needed to understand, resume, and debug the execution:

```
.mori/runs/20260409-143000/
├── task-state.json              # Executor checkpoint (resumable)
├── metrics.jsonl                # Per-task timing, cost, pass/fail (append-only)
├── discovered-patterns.json     # Shared error patterns (max 20, FIFO)
├── iteration-memory/
│   ├── plan-46-memory.json     # What plan 46 learned across retries
│   └── plan-47-memory.json     # What plan 47 learned across retries
├── worktrees/
│   ├── plan-46/                # Physical git worktree
│   └── plan-47/                # Physical git worktree
└── agent-pids.json              # PID registry for cleanup on crash
```

### File-by-File Breakdown

**task-state.json**: The executor checkpoint. Contains: plan phases (which phase each plan is in), completed task sets, retry counts, worktree paths, branch names, merge status. On crash, `--resume` reads this file and resumes from the last consistent state. Written atomically (temp file + rename) after every phase transition.

**metrics.jsonl**: One JSON object per task completion, appended as tasks finish. Fields: `task_id`, `plan_id`, `start_time`, `end_time`, `duration_ms`, `model_used`, `tokens_in`, `tokens_out`, `cost_usd`, `gate_results` (per-rung pass/fail), `retry_count`, `final_status`. Used by the learning system to train the CascadeRouter and update adaptive thresholds.

**discovered-patterns.json**: The stigmergic coordination file (see section 14). Max 20 entries. FIFO eviction when full. Each entry: `plan_id`, `task_id`, `error_signature`, `fix_applied`, `discovered_at`. Agents read this before starting work to preemptively avoid known failure modes.

**iteration-memory/**: Per-plan feedback from failed attempts. When a plan fails a gate and retries, the failure context (error digest, reflection, reviewer feedback) is persisted here. On the next attempt, this context is injected into the agent's prompt so it does not repeat the same mistake. Structure: `{ "attempt": N, "error_digest": "...", "reflection": "...", "reviewer_feedback": [...] }`.

**worktrees/**: Physical git worktrees, one per active plan. These are full working copies sharing the repository's `.git` directory. Disk-efficient (only tracking overhead, not full clone). Each worktree has its own `target/` directory for compilation, linked to the shared sccache.

**agent-pids.json**: Maps `plan_id` to `[pid1, pid2, ...]` for all agent processes spawned during the run. On crash recovery, the supervisor reads this file, sends SIGTERM to all listed PIDs (handles the case where orphaned agents are still running), waits 5s, then SIGKILL. On clean shutdown, PIDs are removed as agents terminate.

### Lifecycle and Cleanup

Run state is kept indefinitely by default. The operator controls cleanup:

```bash
# List all runs with sizes
roko clean runs --list

# Remove runs older than 7 days (keeps worktrees)
roko clean runs --older-than 7d

# Remove runs older than 7 days INCLUDING worktrees
roko clean runs --older-than 7d --include-worktrees

# Never auto-deleted: the operator may need worktrees for debugging,
# metrics for analysis, or iteration-memory for understanding failures
```

Worktrees are never automatically deleted because they may contain uncommitted diagnostic changes, partial implementations useful for debugging, or represent branch state the operator wants to inspect manually.

---

## 14. Stigmergic Coordination via Discovered Patterns (Extended)

### Digital Pheromones

In biological systems, stigmergy is coordination through environmental modification rather than direct communication. Ants deposit pheromones on trails; other ants follow the gradient. No ant needs to know any other ant's identity, location, or intent. The environment mediates all coordination.

Roko's `discovered-patterns.json` implements the same principle for coding agents. Agents never communicate directly. Instead, they read from and write to a shared file that accumulates knowledge about the current build's error landscape.

### The Mechanism in Detail

```
Agent A encounters error E0433 (unresolved import):
  1. Agent A resolves the error (adds missing `use` statement)
  2. After gate pass, Agent A's error signature is appended to discovered-patterns.json:
     {
       "plan_id": "46",
       "task_id": "T3",
       "error_signature": "E0433: unresolved module `tokio::sync`",
       "fix_applied": "Added `use tokio::sync::Mutex` to imports",
       "discovered_at": "2026-04-09T14:30:00Z"
     }

Agent B starts its task 10 minutes later:
  1. Before generating code, Agent B's prompt includes discovered patterns
  2. Agent B's task touches the same crate
  3. Agent B preemptively adds the `tokio::sync` import
  4. Agent B passes compile gate on first attempt (no wasted retry)
```

### Why Max 20 Entries, FIFO Eviction

The cap prevents unbounded growth and ensures freshness:

- **Unbounded growth**: Over a 200-task build, hundreds of patterns could accumulate. Most become irrelevant as the codebase evolves. Feeding stale patterns to agents wastes tokens and can cause confusion (applying fixes for errors that no longer exist).

- **FIFO eviction**: The oldest pattern is the least likely to be relevant. Errors discovered in early tasks are typically resolved by the time later tasks run. Newer patterns reflect the current state of the build.

- **Token budget**: 20 entries at ~100 tokens each = ~2,000 tokens. This fits comfortably in the "learned context" section of the prompt without displacing higher-priority context (plan, brief, code).

### Expanding Beyond Error Patterns

The current implementation is limited to error patterns, but the stigmergic mechanism generalizes to:

- **Successful strategies**: "Using `Arc<Mutex<T>>` instead of `Rc<RefCell<T>>` for this crate's async context" — positive pheromones, not just negative ones
- **File ownership hints**: "Plan 46 is actively modifying `crates/roko-core/src/types.rs` — avoid concurrent edits" — territorial pheromones
- **Performance observations**: "Compile time for `roko-agent` increased to 45s after adding `reqwest` — consider feature-gating" — environmental signals

### Research Context

Grassé (1959) introduced stigmergy in the study of termite nest construction. Termites do not follow a central blueprint; they respond to the current state of the structure, adding material where it is needed. The result is a complex, adaptive structure built without central coordination.

Heylighen (2016) generalized stigmergy as a universal coordination mechanism applicable to any multi-agent system. Key insight: stigmergy scales better than direct communication because coordination cost is O(1) per agent (read the environment) rather than O(N) (communicate with every other agent).

Bonabeau, Dorigo, & Theraulaz (1999) formalized the computational properties of stigmergic systems in the context of ant colony optimization. Their work showed that stigmergic coordination produces near-optimal solutions for combinatorial problems (TSP, graph coloring) without any agent having global knowledge.

In the Roko context, `discovered-patterns.json` is a digital pheromone trail. Each agent deposits knowledge; subsequent agents follow the gradient toward working code. The system as a whole converges on correct implementations faster than any individual agent could alone.
