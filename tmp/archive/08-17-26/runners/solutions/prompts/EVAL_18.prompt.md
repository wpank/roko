# EVAL_18: Scaffold `roko-eval-judge` crate

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-18`](../ISSUE-TRACKER.md#eval-18)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.18
- Priority: **P1**
- Effort: 2 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for `JudgeOracle` trait at `crates/roko-gate/src/llm_judge_gate.rs:53-56`), `async-trait`, `serde`, `serde_json`, `rand`.

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

- `Cargo.toml`

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

- [ ] The change matches the Implementation Steps above.
- [ ] No files outside Write Scope were touched.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
