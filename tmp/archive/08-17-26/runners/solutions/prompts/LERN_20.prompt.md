# LERN_20: Wire Error Pattern Store Integration

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-20`](../ISSUE-TRACKER.md#lern-20)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.20
- Priority: **P2**
- Effort: 4 hours
- Depends on: `LERN_11` (source 7.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ErrorPatternStore` (at `error_pattern_store.rs:229`) tracks gate failure patterns with `observe_gate_failure()` (line 299), `top_patterns()` (line 359), `format_for_prompt()` (line 433), `bounded_summary()` (line 376). It persists via `save()` / `load()`.

`GateFailureObservation` (at line 62) takes `error_digest`, `gate_name`, `model`, `role`, `task_category`, `cost_usd`.

The store exists but is not loaded or written to from any live path.

## Exact Changes

1. Load `ErrorPatternStore::load(&roko_dir.join("learn/error-patterns.json"))` at run initialization.
2. On gate failure, call `store.observe_gate_failure(GateFailureObservation::new(...))` with the gate error output.
3. Before dispatch, call `store.bounded_summary(model, role, category, limit)` to get relevant patterns.
4. If patterns exist, inject them into the prompt as a hint section (e.g., "Known failure patterns for this type of task: ...").
5. Feed high-frequency patterns into the conductor's `error_pattern` context feature (from Task 7.11).
6. Save the store after each observation.
7. Add GC: `store.gc(max_age: Duration::from_secs(30 * 86400), max_patterns: 500)` periodically.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/error_pattern_store.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Run tasks that consistently fail with the same error, verify pattern store accumulates counts
- [ ] `error-patterns.json` shows stored patterns with frequency counts
- [ ] High-frequency patterns appear in `roko learn all` output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run tasks that consistently fail with the same error, verify pattern store accumulates counts
- `error-patterns.json` shows stored patterns with frequency counts
- High-frequency patterns appear in `roko learn all` output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
