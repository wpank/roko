# OBS__08: Emit `RouterDecision` events from CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#obs--08`](../ISSUE-TRACKER.md#obs--08)
- Source: `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` — Task 18.8
- Priority: **??**
- Effort: ?
- Depends on: `OBS__07` (source 18.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: OBS__08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/orchestrate.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `rg 'RouterDecision\|router_decision' crates/roko-learn/src/cascade_router.rs` matches >= 1.
- [ ] `rg 'EscalationEvent' crates/roko-cli/src/orchestrate.rs` matches >= 1.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: OBS__08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `rg 'RouterDecision\|router_decision' crates/roko-learn/src/cascade_router.rs` matches >= 1.
- `rg 'EscalationEvent' crates/roko-cli/src/orchestrate.rs` matches >= 1.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: OBS__08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
