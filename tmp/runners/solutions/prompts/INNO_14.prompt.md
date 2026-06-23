# INNO_14: Wire generated gates into GateService runtime

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-14`](../ISSUE-TRACKER.md#inno-14)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.14
- Priority: **P1**
- Effort: 4 hours
- Depends on: `INNO_13` (source 11.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

GateService at `crates/roko-gate/src/gate_service.rs` runs the 7-rung pipeline.
Generated gates need to be inserted as pre-flight checks before the standard
rung pipeline (rung 0).

## Exact Changes

1. In `GateService::run_gates()`, load generated gates from
   `.roko/learn/gate-evolution.json`.
2. Filter to non-retired gates whose target pattern matches the current diff.
3. Run matching generated gates as rung 0 (before compile).
4. If a generated gate catches the issue, skip the expensive standard rung
   that would have caught it (e.g., skip clippy if a grep-based gate already
   found unused imports).
5. Record generated gate outcomes for effectiveness tracking.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] A generated "unused import" gate fires before clippy and catches the issue
- [ ] Gate report shows the generated gate ran at rung 0
- [ ] If the generated gate passes but clippy later catches the same issue, the generated gate's effectiveness score decreases

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A generated "unused import" gate fires before clippy and catches the issue
- Gate report shows the generated gate ran at rung 0
- If the generated gate passes but clippy later catches the same issue, the generated gate's effectiveness score decreases
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
