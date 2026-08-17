# EVAL_40: Runtime event bus extensions

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-40`](../ISSUE-TRACKER.md#eval-40)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.40
- Priority: **P1**
- Effort: 4 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_40 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The existing `RuntimeEvent` enum at `crates/roko-core/src/runtime_event.rs:56-128` has gate events (`GateStarted`, `GatePassed`, `GateFailed`). Add parallel eval events.

## Exact Changes

1. Add variants to `RuntimeEvent`:
   ```rust
   EvalStarted { run_id: String, profile_id: String, task_id: Option<String> },
   EvalCriterionCompleted { run_id: String, criterion_name: String, passed: bool, score: f64, duration_ms: u64 },
   EvalCompleted { run_id: String, verdict_passed: bool, score: f64, criteria_passed: usize, criteria_total: usize, duration_ms: u64, cost_usd: f64 },
   ```
2. Update the `run_id()` method to handle new variants.
3. Update the `Display` impl if one exists.

## Design Guidance

Do NOT remove existing `GateStarted/GatePassed/GateFailed` variants -- they must remain for backward compatibility. The new `Eval*` variants coexist. The SSE adapter and TUI will consume both.

## Write Scope

- `crates/roko-core/src/runtime_event.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Event serialization round-trip test for new variants

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_40 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Event serialization round-trip test for new variants
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_40 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
