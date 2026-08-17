# EVAL_17: `RuntimeTraceCollector` and `CoverageCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-17`](../ISSUE-TRACKER.md#eval-17)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.17
- Priority: **P2**
- Effort: 5 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runs tests with coverage instrumentation (cargo-tarpaulin for Rust). Parses coverage output.

## Exact Changes

1. `RuntimeTraceCollector`: runs `cargo tarpaulin --out Json`, parses JSON output into `CoverageData { line_coverage: f64, branch_coverage: f64, files: Vec<FileCoverage> }`. Produces `EvidenceKind::RuntimeTrace`.
2. `CoverageCriterion`: checks minimum line coverage threshold (default 0.7). Optional `diff_only` mode: only consider coverage for changed files. Reports files with lowest coverage as Findings. Soft severity.

## Write Scope

- `crates/roko-eval-metrics/src/lib.rs`

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

- [ ] Test with mock coverage JSON output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test with mock coverage JSON output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
