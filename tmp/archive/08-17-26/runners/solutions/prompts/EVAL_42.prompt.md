# EVAL_42: `roko eval` serve routes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-42`](../ISSUE-TRACKER.md#eval-42)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.42
- Priority: **P2**
- Effort: 6 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_42 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. `GET /api/eval/traces` -- list recent, supports limit/offset/filter.
2. `GET /api/eval/traces/{id}` -- full trace with evidence.
3. `GET /api/eval/summary` -- aggregate stats.
4. `GET /api/eval/criteria` -- list registered criteria.
5. `GET /api/eval/profiles` -- list profiles.
6. `POST /api/eval/run` -- trigger ad-hoc evaluation.
7. Register routes in `crates/roko-serve/src/routes/mod.rs`.

## Write Scope

- `crates/roko-serve/src/routes/mod.rs`
- `crates/roko-serve/Cargo.toml`

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

- [ ] API integration test for trace listing

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_42 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- API integration test for trace listing
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_42 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
