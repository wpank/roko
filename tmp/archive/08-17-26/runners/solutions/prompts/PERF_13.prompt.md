# PERF_13: Define GateMode Enum

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-13`](../ISSUE-TRACKER.md#perf-13)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.13
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add a `GateMode` enum to `WorkflowConfig` for gate-level mode
selection (Full/Express/None/Auto).

## Exact Changes

1. Define:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
   pub enum GateMode {
       #[default]
       Full,     // all configured gates
       Express,  // lightweight only (diff, fmt)
       None,     // skip all gates
       Auto,     // detect from changed file types
   }
   ```
2. Add `pub gate_mode: GateMode` to `WorkflowConfig` with `Default::default()`
   (Full -- backwards compatible)
3. Add `pub fn with_gate_mode(mut self, mode: GateMode) -> Self`
4. Derive `clap::ValueEnum` so it can be used as CLI argument
5. Implement `Display` for log output

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`

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

- [ ] `GateMode::default()` returns `Full`
- [ ] Serializes/deserializes in JSON and TOML correctly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateMode::default()` returns `Full`
- Serializes/deserializes in JSON and TOML correctly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
