# INNO_19: Wire debugging into gate failure handler

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-19`](../ISSUE-TRACKER.md#inno-19)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.19
- Priority: **P2**
- Effort: 12 hours
- Depends on: `INNO_17` (source 11.17), `INNO_18` (source 11.18), `INNO_03` (source 11.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

When gate failures exhaust the autofix budget, execution currently halts. The
debug engine should classify the failure, generate hypotheses, apply the top
intervention, and retry.

`build_gate_failure_plan_revision` exists at `crates/roko-cli/src/orchestrate.rs`
but is in the legacy monolith. The WorkflowEngine at
`crates/roko-runtime/src/workflow_engine.rs` is the active runtime.

## Exact Changes

1. After autofix budget is exhausted, call `classify()` on the failure.
2. Call `generate_hypotheses()` with the classified failure.
3. Apply the top hypothesis's intervention:
   - `RouteToModel` -> override model for retry
   - `AddContext` -> inject additional context into prompt
   - `FixPermissions` -> adjust role for retry
   - `AdjustPrompt` -> modify section weights
4. Retry the task with the intervention applied.
5. If retry succeeds: record the intervention as a playbook entry.
6. If retry fails: try next hypothesis (up to 3 attempts).
7. If all hypotheses fail: generate a debug report and write to
   `.roko/debug/{task_id}.md`.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-learn/src/debug_engine.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A task that fails 3 times with "missing module" -> debug engine adds repo tree context -> retry succeeds
- [ ] Successful intervention is saved as a playbook
- [ ] Debug report is written for failures that exhaust all hypotheses

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A task that fails 3 times with "missing module" -> debug engine adds repo tree context -> retry succeeds
- Successful intervention is saved as a playbook
- Debug report is written for failures that exhaust all hypotheses
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
