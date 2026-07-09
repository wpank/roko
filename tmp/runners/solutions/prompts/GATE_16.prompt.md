# GATE_16: Track failure patterns in learning system

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-16`](../ISSUE-TRACKER.md#gate-16)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.16
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_15` (source 4.15)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`error_patterns.rs` at `crates/roko-gate/src/error_patterns.rs` extracts `ErrorPatternRecord` from gate classifications. `records_from_classification()` at line 64 converts a `GateFailureClassification` to pattern records with keys like `"E0425::src/main.rs"`. These records enable detection of recurring failure modes.

Runner v2 constructs a `LearningRuntime` but does not record error patterns from gate failures.

## Exact Changes

1. After each gate failure in the event loop, extract error patterns:
   ```rust
   use roko_gate::error_patterns::records_from_classification;

   if let Some(ref classification) = completion.failure_classification {
       let patterns = records_from_classification(classification);
       for pattern in patterns {
           learning_runtime.record_error_pattern(pattern);
       }
   }
   ```
2. If `LearningRuntime` does not have `record_error_pattern()`, add it -- or append directly to the error patterns file.
3. At plan completion, query top-K error patterns and log them for diagnostic purposes.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Error patterns are recorded after gate failures
- [ ] Patterns persist across runs (written to disk)
- [ ] Duplicate patterns increment count rather than creating new entries

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Error patterns are recorded after gate failures
- Patterns persist across runs (written to disk)
- Duplicate patterns increment count rather than creating new entries
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
