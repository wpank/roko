# GATE_06: Migrate `roko run` gate dispatch to GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-06`](../ISSUE-TRACKER.md#gate-06)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.6
- Priority: **P0**
- Effort: 3 hours
- Depends on: `GATE_03` (source 4.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run_gate()` at `crates/roko-cli/src/run.rs:2942` (behind `#[cfg(feature = "legacy-orchestrate")]`) matches on a `GateConfig` enum with 4 variants: Shell, Compile, Clippy, Test. This is the simplest dispatch path with no adaptive thresholds, no feedback, no rung selection. Replacing it with GateService adds all those features for free.

Note this function is behind a feature flag `legacy-orchestrate`. The primary `roko run` path now goes through WorkflowEngine's EffectDriver, which already uses `GateRunner` trait. Verify which code path is active before modifying.

## Exact Changes

1. Search for the active gate dispatch path in `roko run` (not behind feature flags):
   - The WorkflowEngine path (`crates/roko-runtime/src/effect_driver.rs:280` `run_gates()`) already constructs `GateConfig` and calls `self.services.gate_runner.run_gates(config)`. This path already uses GateService when ServiceFactory wires it.
2. For the legacy path behind `#[cfg(feature = "legacy-orchestrate")]`:
   - Replace the match-based dispatch with a GateService call similar to Task 4.5.
   - Or mark the legacy path as deprecated and schedule removal.
3. Verify ServiceFactory at `crates/roko-orchestrator/src/service_factory.rs` constructs `GateService` as the `gate_runner`. If it does, `roko run` already uses GateService transitively. Document this finding.
4. If `roko run` still has a non-legacy gate path outside the WorkflowEngine, migrate it.

## Design Guidance

The WorkflowEngine + EffectDriver + ServiceFactory path is the canonical one for `roko run`. If ServiceFactory already wires GateService, this task is primarily verification + cleanup of the legacy path.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] `cargo run -p roko-cli -- run "add a comment"` uses GateService for gates
- [ ] No inline gate construction outside GateService in the active `roko run` code path
- [ ] Legacy `run_gate()` behind feature flag is either migrated or marked deprecated

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No inline gate construction outside GateService in the active `roko run` code path
- Legacy `run_gate()` behind feature flag is either migrated or marked deprecated
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
