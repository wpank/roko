# INNO_18: Implement debugger hypothesis generation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-18`](../ISSUE-TRACKER.md#inno-18)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.18
- Priority: **P2**
- Effort: 8 hours
- Depends on: `INNO_17` (source 11.17)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Once failures are classified, the system should generate ranked hypotheses
about root cause and propose interventions.

## Exact Changes

1. Create `crates/roko-learn/src/debug_engine.rs`.
2. Define `Hypothesis` struct: `cause: String`, `confidence: f64`,
   `intervention: Intervention`, `evidence: Vec<String>`.
3. Define `Intervention` enum: `RouteToModel(String)`, `AddContext(String)`,
   `FixPermissions(String)`, `AdjustPrompt(String)`, `TuneGate(String)`.
4. Implement `generate_hypotheses(failure: &FailureKind, context: &TaskContext,
   history: &[Episode]) -> Vec<Hypothesis>`:
   - ConvergenceFailure -> ["missing context", "wrong model", "prompt interference"]
   - ResourceFailure -> ["context too large", "model too expensive"]
   - ToolFailure -> ["permission mismatch", "tool not available"]
   - QualityFailure -> ["wrong model tier", "missing relevant code"]
5. Rank hypotheses by: recurrence in history, similarity to past interventions
   that worked (from PlaybookStore).
6. Add `pub mod debug_engine;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`

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

- [ ] A ConvergenceFailure produces at least 3 ranked hypotheses
- [ ] Hypotheses include actionable interventions, not just descriptions
- [ ] A previously successful intervention ranks higher than novel ones

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A ConvergenceFailure produces at least 3 ranked hypotheses
- Hypotheses include actionable interventions, not just descriptions
- A previously successful intervention ranks higher than novel ones
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
