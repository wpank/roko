# RNNR_09: Add gate mode and build policy CLI flags to `roko plan run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-09`](../ISSUE-TRACKER.md#rnnr-09)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.9
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_06` (source 14.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Expose wave gate mode as CLI flags on `roko plan run`, matching the
mega-parity runner's operational controls.

## Exact Changes

1. Add to `PlanCmd::Run` in `main.rs`:
   ```rust
   #[arg(long, value_enum, default_value = "per-task")]
   gate_mode: GateMode,  // per-task, per-wave, deferred
   #[arg(long)]
   no_gate: bool,        // shorthand for --gate-mode deferred
   #[arg(long)]
   no_build: bool,       // force BuildPolicy::Prohibited regardless of gate mode
   ```
2. Map CLI flags to `RunConfig` fields (add `gate_mode` and `build_policy` to `RunConfig`)
3. `--no-gate` is equivalent to `--gate-mode deferred`
4. Add `[execution]` section to `roko.toml` schema for persistent defaults:
   ```toml
   [execution]
   gate_mode = "per-task"
   ```
5. CLI flags override TOML config

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/commands/plan.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan run --gate-mode per-wave` uses wave-level gating
- [ ] `roko plan run --no-gate` defers all gating to end of run
- [ ] `roko plan run` without flags uses per-task gating (backward compatible)
- [ ] `roko plan run --no-build` injects build prohibition regardless of gate mode

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run --gate-mode per-wave` uses wave-level gating
- `roko plan run --no-gate` defers all gating to end of run
- `roko plan run` without flags uses per-task gating (backward compatible)
- `roko plan run --no-build` injects build prohibition regardless of gate mode
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
