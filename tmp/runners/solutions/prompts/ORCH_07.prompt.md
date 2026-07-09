# ORCH_07: Gate Failure Context for Retries

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-07`](../ISSUE-TRACKER.md#orch-07)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.7
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ORCH_06` (source 2.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PipelineStateV2` carries `last_gate_failure: Option<String>` (line 546) which is a raw string. When a gate fails and the agent retries, the retry context is unstructured:
```rust
context: Some(format!("Previous attempt failed gate '{gate}'. Error:\n{output}"))
```
(pipeline_state.rs lines 676-679)

A structured failure context with per-gate breakdowns, attempt count, and error pattern matching would improve retry success rates.

## Exact Changes

1. Add a `FailureRecord` struct to `pipeline_state.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FailureRecord {
       pub attempt: u32,
       pub gate_name: String,
       pub gate_output: String,
       pub diff_summary: Option<String>,
   }
   ```
2. Replace `last_gate_failure: Option<String>` with `failure_history: Vec<FailureRecord>` in `PipelineStateV2`.
3. Update `step()` GateFailed handlers to push to `failure_history` instead of overwriting `last_gate_failure`.
4. Render the failure history as structured context when spawning retry agents.
5. Maintain backward compat: `checkpoint()` / `from_checkpoint()` must handle both the old `last_gate_failure` field and the new `failure_history` field (use `#[serde(default)]`).

## Design Guidance

Keep the failure history bounded (last 5 failures max) to prevent unbounded growth. The structured format enables ErrorPatternStore matching in a later phase.

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Gate failures append to `failure_history` instead of overwriting
- [ ] Retry agent receives structured context with all prior failures
- [ ] Checkpoint round-trip preserves failure history
- [ ] Backward compat: old checkpoints without `failure_history` deserialize correctly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate failures append to `failure_history` instead of overwriting
- Retry agent receives structured context with all prior failures
- Checkpoint round-trip preserves failure history
- Backward compat: old checkpoints without `failure_history` deserialize correctly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
