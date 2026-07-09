# EVAL_08: `CompileCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-08`](../ISSUE-TRACKER.md#eval-08)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.8
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Migrates `CompileGate` from `crates/roko-gate/src/compile.rs`. Consumes `EvidenceKind::ProcessOutput` + `EvidenceKind::ProcessStatus` from `ProcessCollector::for_compile()`. Reuses existing parse functions: `roko_gate::parse_cargo_json` and `roko_gate::parse_plain_stderr` at `crates/roko-gate/src/compile_errors.rs` for structured error extraction.

The existing `CompileGate` at `crates/roko-gate/src/compile.rs` spawns `cargo check --message-format=json`, parses the output, and produces a `Verdict`. The new `CompileCriterion` does the same evaluation logic but reads from the `EvidenceBag` instead of spawning the process itself.

## Exact Changes

1. Implement `CompileCriterion` implementing `Criterion`:
   - `name()` = "compile"
   - `criterion_kind()` = `CriterionKind::Deterministic`
   - `is_hard()` = `true`
   - `required_evidence()` = `[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]`
   - `default_threshold()` = `1.0` (binary: must compile)
   - `evaluate()`: extract stdout from ProcessOutput evidence, parse with `parse_cargo_json()` or `parse_plain_stderr()`, compute binary score (0.0 or 1.0), emit up to N `Finding` items with source_location, rule_id, fix_hint from `CompileError` structs.
2. Add configurable `max_error_findings: usize` (default 20) to limit Finding count.

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

- [ ] Test that findings include source_location with file/line

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test that findings include source_location with file/line
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
