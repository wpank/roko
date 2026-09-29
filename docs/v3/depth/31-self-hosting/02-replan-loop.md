# 31-02 -- Gate-Failure Replan Controller

> **Parent:** [31-SELF-HOSTING.md](../../31-SELF-HOSTING.md) section 3
> **Source:** `crates/roko-execution/src/replan_controller.rs`
> **Cross-references:** [07-GATES](../../07-GATES.md) (gate pipeline, failure
> classification), [04-EXECUTION](../../04-EXECUTION.md) (plan mutation,
> checkpoint extension)

---

## 1. Purpose

When a task exhausts its retry budget and still fails the gate pipeline, the
replan controller applies deterministic structural mutations to the plan
itself. This is qualitatively different from retry (which re-attempts the same
task with different provider output) -- replanning changes what the task *is*.

The controller is the "Failure -> Replanning" cybernetic feedback loop
(08-LEARNING, Loop 4). It converts gate failure evidence into plan mutations
that address the structural cause of failure.

---

## 2. When Replanning Fires

Replanning is triggered by the Graph engine when all of the following are true:

1. A task has failed its gate pipeline (`GateCell` emits `NodeStatus::Failed`)
2. The task has exhausted its configured retry count
3. Auto-replan is enabled (`learning_config.replan_on_gate_failure = true`)
4. The replan cap for this run has not been reached

The Graph engine constructs a `ReplanRequest` containing the failed task ID,
the gate failure classification, the current plan fingerprint, completed task
IDs, and any prior replan attempts from checkpoint extensions.

---

## 3. Five Strategies in Fixed Order

The controller evaluates strategies in a deterministic fixed order:

### 3.1 ChangeApproach

Replace the failed task's metadata and prompt approach. Dependencies and task
ID remain unchanged. This is the lightest mutation -- it changes *how* the
task is attempted without changing the plan structure.

**When it helps:** The original prompt led the agent down a wrong algorithm or
approach. Changing the approach metadata triggers a different prompt assembly
path.

### 3.2 SplitTask

Replace the failed task with two ordered child tasks: `<id>-part-1` and
`<id>-part-2`. Incoming dependencies target part-1, part-2 depends on part-1,
outgoing dependencies move to part-2.

**When it helps:** The task is too large or complex for a single agent turn.
Splitting distributes the work across two focused tasks.

### 3.3 AddPrerequisite

Insert `<id>-prerequisite` containing the missing-context/dependency evidence
and make the failed task depend on it.

**When it helps:** The task failed because it lacked necessary context or a
dependency that was not declared in the original plan. The prerequisite task
gathers the missing information.

### 3.4 MergeSiblingTasks

Merge the failed task with the next lexicographically sorted pending sibling
that has identical incoming dependencies. Completed and running siblings are
ineligible.

**When it helps:** Two tasks overlap and the agent is confused by the boundary
between them. Merging into a single task eliminates the ambiguity.

### 3.5 RemoveInvalidDependency

Remove a dependency named by structured gate evidence as missing or invalid.
Absent explicit evidence produces a typed rejection -- the controller will not
guess which dependency is wrong.

**When it helps:** A declared dependency no longer exists (crate renamed,
function deleted) and the dependency graph is preventing the task from
receiving correct inputs.

---

## 4. Strategy Selection Algorithm

```rust
pub fn decide(request: &ReplanRequest) -> ReplanDecision {
    // 1. Check cap
    let effective_cap = request.max_replans.min(ABSOLUTE_MAX_REPLANS);
    if request.prior_attempts.len() as u32 >= effective_cap {
        return ReplanDecision::CapReached;
    }

    // 2. Check failure class eligibility
    if !is_replannable(&request.gate_classification) {
        return ReplanDecision::Reject { reason: "..." };
    }

    // 3. Compute evidence fingerprint
    let evidence_fp = canonical_fingerprint(&request.gate_classification);

    // 4. Try strategies in fixed order, skipping already-tried pairs
    for strategy in ReplanStrategy::ORDERED {
        if request.prior_attempts.contains(&(strategy.clone(), evidence_fp.clone())) {
            continue;
        }

        // 5. Construct and validate the mutation
        match build_mutation(&strategy, &request) {
            Ok(mutation) => {
                // 6. Apply atomically
                return ReplanDecision::Apply { strategy, mutation };
            }
            Err(_) => continue,  // strategy inapplicable, try next
        }
    }

    ReplanDecision::Reject { reason: "all strategies exhausted" }
}
```

### 4.1 Deduplication

Each `(strategy, evidence_fingerprint)` pair is tried at most once. This
prevents infinite loops where the same strategy is applied to the same failure
repeatedly. The evidence fingerprint is computed from the gate failure
classification using BLAKE3 via `canonical_fingerprint()`.

### 4.2 Cap

The absolute cap is `min(request.max_replans, 5)`. This is independent of the
task retry count -- a task may have 3 retries and then up to 5 structural
replans, for a total of 8 attempts at the task.

### 4.3 Failure Class Eligibility

Not all failure classes are eligible for replanning. Catastrophic failures
(data corruption, auth revoked) and resource failures (disk full, OOM) are
rejected -- structural plan changes cannot fix infrastructure problems.

---

## 5. Plan Mutation Contract

Every replan mutation goes through the `roko_core::plan_mutation` contract:

```rust
pub struct PlanMutationV1 {
    pub mutation_id: String,        // UUID
    pub plan_id: String,
    pub operations: Vec<PlanMutationOpV1>,
    pub author: MutationAuthorV1,
    pub evidence: Vec<MutationEvidenceV1>,
    pub timestamp: DateTime<Utc>,
}

pub enum PlanMutationOpV1 {
    AddTask { task: MutableTaskV1 },
    RemoveTask { task_id: String },
    UpdateTask { task_id: String, updates: TaskUpdatesV1 },
    AddDependency { from: String, to: String },
    RemoveDependency { from: String, to: String },
}
```

Mutations are applied atomically via `apply_mutation()`. The plan state before
and after mutation is fingerprinted. The mutation is rejected if:

- The plan exceeds `MAX_PLAN_TASKS` (200) after mutation
- A cycle would be introduced in the dependency graph
- A referenced task ID does not exist (for removal/update operations)

---

## 6. Durable Receipts

Every applied replan produces a `ReplanReceiptV1` stored as a checkpoint
extension under the key `roko.replan@1`:

```rust
pub struct ReplanReceiptV1 {
    pub strategy: ReplanStrategy,
    pub evidence_fingerprint: String,
    pub before_fingerprint: String,
    pub after_fingerprint: String,
    pub ordinal: u32,           // 0-indexed within this run
    pub mutation_id: String,
}
```

On resume, the checkpoint extension is loaded and prior attempts are
reconstructed from receipts, ensuring deduplication across crashes.

---

## 7. Integration with Graph Engine

The replan controller operates outside the DAG -- it is invoked by the
controller layer (not by a Cell within the Graph). The flow is:

```
GateCell emits Failed
    |
    v
Controller detects exhausted retries + auto-replan enabled
    |
    v
ReplanController::decide(request)
    |
    v
If Apply: apply_mutation() -> write receipt -> resume graph
If Reject/CapReached: propagate failure to plan status
```

The Graph engine re-computes the topological order after mutation. New tasks
(from SplitTask or AddPrerequisite) enter the execution queue at the
appropriate wave position.

---

## 8. Configuration

```toml
[learning]
replan_on_gate_failure = true   # Enable/disable auto-replan
max_replans = 3                 # Per-run cap (clamped to absolute max of 5)
```

---

## References

- Source: `crates/roko-execution/src/replan_controller.rs`
- Plan mutation contract: `crates/roko-core/src/plan_mutation.rs`
- Gate failure classification: `crates/roko-gate/src/compile_errors.rs`
