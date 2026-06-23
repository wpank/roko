# PERF_01: Add Tracing Spans to Hot-Path Functions

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-01`](../ISSUE-TRACKER.md#perf-01)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.1
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add `#[tracing::instrument(skip_all, fields(phase = "..."))]` to seven
hot-path functions so every optimization can be measured before and after.

## Exact Changes

1. `run.rs`: Instrument `resolve_workflow_model_selection()` with
   `fields(phase = "config_load")`
2. `runtime_feedback.rs`: Instrument `LearningRuntime::open_under()` with
   `fields(phase = "learning_open")`
3. `effect_driver.rs`: Instrument `EffectDriver::spawn_agent()` with
   `fields(phase = "agent_construct")`
4. `prompt_assembly_service.rs`: Instrument the `assemble()` impl on
   `PromptAssemblyService` with `fields(phase = "prompt_assemble")`
5. `file_substrate.rs`: Instrument `FileSubstrate::put()` (the `Store` impl at
   line 271) with `fields(phase = "substrate_write")`
6. `gate_service.rs`: Instrument `GateService::run_gates()` (the `GateRunner`
   impl at line 235) with `fields(phase = "gate_run")`
7. `jsonl_logger.rs`: Instrument `JsonlLogger::write_event()` (line 62) with
   `fields(phase = "feedback_flush")`

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-fs/src/file_substrate.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-runtime/src/jsonl_logger.rs`

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

- [ ] `RUST_LOG=roko=trace cargo run --release -p roko-cli -- config show` produces

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
