# INNO_24: Add TUI steering panel (F8)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-24`](../ISSUE-TRACKER.md#inno-24)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.24
- Priority: **P3**
- Effort: 12 hours
- Depends on: `INNO_21` (source 11.21), `INNO_22` (source 11.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

TUI at `crates/roko-cli/src/tui/` has F1-F7 tabs and a `modals/` directory.
F8 is the natural binding for the steering panel.

## Exact Changes

1. Bind F8 to open the steering panel.
2. Panel shows: current task, confidence score, agent state.
3. Key bindings within panel:
   - `s`: redirect (text input for guidance)
   - `k`: skip current task
   - `b`: adjust budget (numeric input)
   - `c`: inject context (text input)
   - `Esc`: close panel
4. On action, send via `SteeringSender` to the execution loop.
5. Show confirmation: "Steering action applied: Redirect sent to task-07".

## Write Scope

- `crates/roko-cli/src/tui/input.rs`
- `crates/roko-cli/src/tui/mod.rs`

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

- [ ] F8 opens the steering panel during a running plan
- [ ] Pressing `s` and typing guidance redirects the running agent
- [ ] Panel shows current task confidence score
- [ ] Esc closes without action

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- F8 opens the steering panel during a running plan
- Pressing `s` and typing guidance redirects the running agent
- Panel shows current task confidence score
- Esc closes without action
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
