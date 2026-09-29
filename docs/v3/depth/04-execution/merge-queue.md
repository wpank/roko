# Merge Queue

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 11.
> Preserves and updates content from v1 `01-orchestration/08-merge-queue.md`.

---

## Overview

The merge queue serializes plan merges to prevent file conflicts. When
multiple plans complete simultaneously, they cannot all merge at once --
if Plan A and Plan B both modified the same file, merging both
simultaneously would create a conflict.

The merge queue solves this by tracking which files each plan modified,
detecting file overlaps between pending merges, allowing non-conflicting
merges to proceed in parallel, and serializing conflicting merges with
retry logic.

**Source:** `crates/roko-cli/src/runner/merge_queue.rs`

---

## Architecture

```rust
pub struct MergeQueue {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    pending:      Vec<MergeRequest>,              // ordered by priority
    merging:      HashMap<String, MergeRequest>,  // currently merging
    locked_files: HashSet<String>,                // files reserved by merges
    completed:    Vec<MergeResult>,
}
```

The queue uses `parking_lot::Mutex` for thread-safe access without
poisoning. The `Arc` wrapper allows cloning the queue handle across
async tasks. A panic in one task does not permanently lock the queue.

---

## MergeRequest

```rust
pub struct MergeRequest {
    pub plan_id: String,
    pub branch_name: String,
    pub files_changed: Vec<String>,
    pub priority: u32,
    pub retry_count: u32,
}
```

The `files_changed` list is populated from the plan's accumulated file
modifications -- as agents complete tasks and modify files, those file
paths are collected. This list is the key input for conflict detection.

---

## Operations

### enqueue()

Adds a merge request to the pending queue. The queue maintains priority
ordering: higher-priority requests are processed first. Equal-priority
requests use FIFO ordering.

### next_mergeable()

Returns the highest-priority pending request that does not conflict with
any currently merging request:

```rust
fn is_conflicting(request: &MergeRequest, locked: &HashSet<String>) -> bool {
    request.files_changed.iter().any(|f| locked.contains(f))
}
```

If all pending requests conflict with in-progress merges, returns `None`.
The caller waits for current merges to complete before retrying.

This algorithm is the critical safety mechanism. It guarantees that no two
concurrent merges touch the same files, preventing git merge conflicts at
the filesystem level.

### mark_merging()

Moves a request from `pending` to `merging` and adds its files to
`locked_files`. This reserves the files for the duration of the merge.

### mark_complete()

Removes a request from `merging`, releases its files from `locked_files`,
and records the result. Releasing files may unblock other pending requests.

### mark_failed()

Handles merge failure with retry logic:

1. Increment `retry_count`.
2. If `retry_count < MAX_RETRIES` (5), re-enqueue with reduced priority
   (positioned after other requests at the same priority level).
3. If at max retries, move to completed with failure status.

---

## Conflict Detection

Conflicts are tracked at individual file granularity, not plan or crate
level. This maximizes parallelism:

```
Plan A modifies: crates/roko-core/src/lib.rs, crates/roko-core/src/config.rs
Plan B modifies: crates/roko-core/src/types.rs, crates/roko-agent/src/pool.rs
```

These two plans do NOT conflict despite touching the same crate. They can
merge in parallel. Only plans modifying the exact same files are
serialized.

### Algorithm complexity

The conflict check is O(P * F) where P is the number of pending requests
and F is the average number of files per request. In practice, P is small
(< 10 concurrent plans) and F is manageable (< 100 files per plan), so
performance is not a concern.

### Why file-level, not crate-level

Crate-level granularity would be overly conservative. Two plans modifying
different files in `crates/roko-core/` can merge independently -- they
only conflict if they touch the exact same file. File-level granularity
maximizes parallelism while maintaining correctness.

---

## Priority Ordering

Merges are processed in priority order:

1. Higher `priority` value processes first.
2. Equal priority uses FIFO ordering (first-enqueued first).

Priority comes from the plan's frontmatter `priority` field and can be
dynamically adjusted by the conductor or operator. Higher-priority plans
merge first, reducing the likelihood that their work is blocked by
lower-priority merges.

---

## Retry with Backoff

When a merge fails, the request is re-enqueued with the same priority but
positioned after other requests at the same priority level. This
implements positional backoff -- the failed merge waits for other merges
to complete, which may resolve the conflict.

### Why retry helps

Consider this scenario:

1. Plan A merges first, modifying `Cargo.lock`.
2. Plan B tries to merge -- its `Cargo.lock` conflicts with Plan A.
3. Plan B is re-enqueued.
4. The batch branch is updated with Plan A's changes.
5. Plan B rebases onto the updated branch, resolving the `Cargo.lock`
   conflict.
6. Plan B's retry succeeds.

This pattern is common with auto-generated files like `Cargo.lock`,
`Cargo.toml`, and aggregate exports. The retry mechanism handles these
transient conflicts automatically.

### Maximum retries

`MAX_RETRIES = 5` prevents infinite retry loops. After 5 failures, the
plan transitions to `PlanPhase::Failed { reason: Deadlock }`.

---

## Post-Merge Regression Detection

After a successful merge, the system runs regression detection:

1. Compile the merged result in the target branch.
2. Run tests on the merged result.
3. If regressions detected, flag for follow-up.

Even though individual plans passed their gates in isolation (in their
worktrees), the combination may fail. For example, Plan A adds a function
that Plan B's tests depend on -- but Plan B was tested without Plan A's
function. Post-merge regression detection catches these cross-plan
integration failures.

---

## CI Ordering Integration

The merge queue integrates with GitHub CI through the `roko github status`
command and E46 workflow integration:

- Local merges are ordered before CI merge gates.
- Regression detection results feed into CI status checks.
- Draft plan PRs carry the merge queue's conflict analysis.
- The `roko github status` command shows pending/merged/failed status.

---

## Relationship to the DAG

The merge queue complements the `CrossPlanDag`:

- The **DAG** prevents conflicting plans from executing simultaneously
  (via crate-overlap warnings at scheduling time).
- The **merge queue** prevents conflicting plans from merging
  simultaneously (via file-level lock tracking at integration time).

Both use file/crate overlap as the conflict signal, but at different
stages of the pipeline. The DAG provides coarse-grained advisory warnings;
the merge queue provides fine-grained enforced serialization.

---

## Thread Safety

The merge queue is designed for concurrent access:

- `Arc<Mutex<Inner>>` allows multiple async tasks to enqueue, query, and
  complete merges simultaneously.
- `parking_lot::Mutex` is non-poisoning -- a panic in one task does not
  permanently lock the queue.
- File locks are tracked in a `HashSet<String>` for O(1) conflict checks.

---

## Error Types

Merge failures produce `MergeResult` entries with:

- `plan_id`: which plan failed
- `success`: false
- `error`: the git merge error message
- `retry_count`: how many retries were attempted
- `files_conflicting`: which files had conflicts

---

## Verification Commands

```bash
# Execute plans (merge queue runs automatically)
cargo run -p roko-cli -- plan run plans/<dir>

# Check GitHub merge status
cargo run -p roko-cli -- github status
```
