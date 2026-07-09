# XCUT_04: Standardize Error Logging with Span Context

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-04`](../ISSUE-TRACKER.md#xcut-04)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.4
- Priority: **P1**
- Effort: 6 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Errors are logged inconsistently across `roko-cli/src/`: 179 `eprintln!` calls across 22 files. The heaviest files: `commands/util.rs` (18), `commands/prd.rs` (18), `main.rs` (17), `chat.rs` (16), `commands/plan.rs` (15), `orchestrate.rs` (15), `prd.rs` (13), `auth_detect.rs` (9). Meanwhile, structured `tracing::*` calls exist in only 10 files (356 total) with no consistent span hierarchy.

Error events lack run ID, task ID, and agent ID needed to correlate failures in multi-agent runs. The workflow engine (`crates/roko-runtime/src/workflow_engine.rs`) creates `RuntimeEventEnvelope` with `run_id` but does not create tracing spans.

## Exact Changes

1. Define a standard span hierarchy documented in `roko-runtime`:
   ```
   roko.run[run_id] -> roko.task[task_id] -> roko.agent[agent_id] -> roko.gate[gate_name]
   ```
2. In `workflow_engine.rs`, wrap the main run loop in `tracing::info_span!("roko.run", run_id = %run_id)`.
3. In `event_loop.rs`, wrap each task dispatch in `tracing::info_span!("roko.task", task_id = %task_id)`.
4. In ACP `runner.rs`, add `roko.acp.session[session_id]` span.
5. Replace `eprintln!` with `tracing::error!` / `tracing::warn!` in the target files. Preserve TUI raw terminal output (those `eprintln!` calls are intentional for direct terminal rendering -- skip `chat_inline.rs` line 1 and similar TUI paths).
6. In serve routes middleware, ensure all error responses include `request_id`.

## Design Guidance

Not all 179 `eprintln!` calls should be converted. Some are intentional user-facing CLI output (e.g., `eprintln!("Error: {e}")` in command handlers that format errors for the terminal). Convert error/warning paths; leave user-facing output that is part of CLI UX. Use `tracing::error!` for errors that should be machine-parseable, `eprintln!` only for formatted terminal output that bypasses the log layer.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-acp/src/runner.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/auth_detect.rs`
- `crates/roko-cli/src/chat.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/prd.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `RUST_LOG=roko=debug roko plan run` produces structured tracing output with nested spans
- [ ] Every error event in the log has `run_id` and `task_id` fields when applicable
- [ ] `eprintln!` count in `roko-cli/src/` reduced by at least 50% (from 179 to <90)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `RUST_LOG=roko=debug roko plan run` produces structured tracing output with nested spans
- Every error event in the log has `run_id` and `task_id` fields when applicable
- `eprintln!` count in `roko-cli/src/` reduced by at least 50% (from 179 to <90)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
