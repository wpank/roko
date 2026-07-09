# PERF_14: Wire Express Gate Mode Into Gate Service

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-14`](../ISSUE-TRACKER.md#perf-14)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.14
- Priority: **??**
- Effort: ?
- Depends on: `PERF_13` (source 10.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Implement gate filtering based on `GateMode`. Express mode skips
compile, clippy, and test. Auto mode detects from changed file types.

## Exact Changes

1. Add `fn filter_gates_for_mode(gates: &[GateEntry], mode: GateMode, workdir: &Path) -> Vec<GateEntry>` or equivalent filtering in `run_gates()`
2. For `Express`: retain only rungs 3 (diff) and 4 (fmt) per the existing
   `rung_for_name()` mapping at line 392
3. For `None`: return empty -- all gates skipped
4. For `Auto`:
   - Run `git diff --stat HEAD`
   - If any `.rs`/`.ts`/`.py` files modified -> Full
   - If only `.toml`/`.json`/`.yaml` -> Express
   - If only `.md`/`.txt` -> None
5. In `run_gates()` (line 235), apply filter before iterating over gates
6. Log which gates are skipped and the resolved mode

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `GateMode::Express` skips compile/clippy/test (verify via trace log)
- [ ] `GateMode::Auto` correctly classifies code vs config vs docs changes
- [ ] `GateMode::Full` unchanged from current behavior

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateMode::Express` skips compile/clippy/test (verify via trace log)
- `GateMode::Auto` correctly classifies code vs config vs docs changes
- `GateMode::Full` unchanged from current behavior
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
