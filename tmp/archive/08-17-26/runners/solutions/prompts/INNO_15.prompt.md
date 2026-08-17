# INNO_15: Implement DiffAnalyzer for rung relevance

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-15`](../ISSUE-TRACKER.md#inno-15)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.15
- Priority: **P1**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Gate pipeline runs the same 7 rungs regardless of what changed. A diff touching
only documentation files should skip compile, clippy, and test gates.

## Exact Changes

1. Create `crates/roko-gate/src/diff_analyzer.rs`.
2. Define `DiffAnalysis` struct: `files_changed: Vec<PathBuf>`,
   `categories: HashSet<FileCategory>`, `estimated_complexity: Complexity`.
3. `FileCategory` enum: `Source`, `Test`, `Documentation`, `Config`, `Build`.
4. `Complexity` enum: `Trivial`, `Moderate`, `Complex`.
5. Implement `analyze_diff(diff: &str) -> DiffAnalysis` using file extension
   and path heuristics:
   - `.rs` in `src/` -> Source
   - `.rs` in `tests/` or `*_test.rs` -> Test
   - `.md`, `.txt`, `.adoc` -> Documentation
   - `.toml`, `.yaml`, `.json` in root or config dirs -> Config
   - `Cargo.toml`, `build.rs` -> Build
6. Implement `relevant_rungs(analysis: &DiffAnalysis) -> Vec<RungId>`:
   - Documentation-only: format + diff only
   - Config-only: format + diff + validate
   - Test-only: format + test + diff
   - Source: all rungs
7. Wire into `GateService::run_gates()`: skip irrelevant rungs.

## Write Scope

- `crates/roko-gate/src/lib.rs`
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

- [ ] A diff touching only `.md` files skips compile, clippy, and test gates
- [ ] A diff touching only test files skips clippy but runs test and format
- [ ] Gate report shows which rungs were skipped with reason "irrelevant to diff"

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A diff touching only `.md` files skips compile, clippy, and test gates
- A diff touching only test files skips clippy but runs test and format
- Gate report shows which rungs were skipped with reason "irrelevant to diff"
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
