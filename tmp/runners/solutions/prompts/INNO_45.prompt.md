# INNO_45: Wire dream consolidation cron trigger

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-45`](../ISSUE-TRACKER.md#inno-45)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.45
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_45 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

DreamRunner at `crates/roko-dreams/src/runner.rs` is built (line 721:
`pub struct DreamRunner`) with `DreamRuntimeControls` but has no runtime
trigger (CLAUDE.md item 14: "Cold substrate archival -- built but not
instantiated at runtime (no cron/trigger)").

Research: AXIOM (arxiv 2505.24784) -- BMR with 7.6x sample efficiency.

## Exact Changes

1. Add `dream.schedule` to roko.toml config:
   `schedule = "after_10_runs"` or `"daily"` or `"manual"`.
2. In the daemon or post-run hook, check if dream cycle should trigger:
   count completed runs since last dream cycle.
3. If count >= threshold, spawn dream cycle as a background task.
4. Dream cycle performs: load episodes, run hypnagogia, run imagination,
   run consolidation, persist distilled knowledge.
5. Log dream cycle outcomes.

## Write Scope

- `crates/roko-dreams/src/runner.rs`
- `crates/roko-cli/src/commands/`

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

- [ ] After 10 completed runs, the dream cycle triggers automatically
- [ ] Dream cycle output appears in `.roko/neuro/knowledge.jsonl`
- [ ] Dream cycle does not block the next run (runs in background)
- [ ] `roko knowledge dream run` still works for manual triggering

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_45 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 10 completed runs, the dream cycle triggers automatically
- Dream cycle output appears in `.roko/neuro/knowledge.jsonl`
- Dream cycle does not block the next run (runs in background)
- `roko knowledge dream run` still works for manual triggering
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_45 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
