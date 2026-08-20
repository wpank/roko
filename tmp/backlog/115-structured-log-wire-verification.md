# 115 — Structured Log Wire Verification (`--log-file` on `plan run`)

**Priority**: P2 — The `StructuredLogger` exists and is conceptually wired, but the `--log-file` CLI flag and the hook call sites in the event loop have not been verified end-to-end, meaning the feature may silently be a no-op.
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli/src/runner/structured_log.rs`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/do_cmd.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §0.6

---

## Background

`crates/roko-cli/src/runner/structured_log.rs` was introduced as an untracked new file (visible in `git status` as of 2026-08-19). It implements `StructuredLogger` with `open()`, `noop()`, and `log(event: &RunnerEvent)` methods, intended to write per-event JSONL to a caller-specified file for real-time tailing.

The implementation checklist requires `roko plan run --log-file /tmp/roko.jsonl` to work. Three things need verification: (1) is `--log-file` actually present in the `plan run` CLI arg spec, (2) is `StructuredLogger.log()` called at the right hook points in `event_loop.rs`, and (3) do wave transition events (which depend on the wave system from #118) need a stub or placeholder.

This is a verification-and-wire task, not a build task. If the wiring is correct, closing this item requires only a test confirming the file is written.

## Current State

- `crates/roko-cli/src/runner/structured_log.rs` — `StructuredLogger` struct exists with `open()`, `noop()`, `log()`.
- `crates/roko-cli/src/commands/do_cmd.rs` — unknown whether `--log-file` is present; needs inspection.
- `crates/roko-cli/src/runner/event_loop.rs` — unknown whether `StructuredLogger.log()` is called; needs inspection.
- Wave transition events: the wave system is not built yet (#118); `RunnerEvent` may not have a wave transition variant. A stub event or skip is acceptable.

## Implementation Plan

1. **Verify `--log-file` flag in CLI**: Read `crates/roko-cli/src/commands/do_cmd.rs` and confirm the flag exists on the `plan run` subcommand. If missing, add `log_file: Option<PathBuf>` to the struct and wire it through to the runner config.

2. **Verify event loop hook points**: Read `crates/roko-cli/src/runner/event_loop.rs` and locate every `RunnerEvent` emission site. Confirm that `structured_logger.log(&event)` is called before or after each emission. If call sites are missing, add them.

3. **Key events that must be logged**:
   - `TaskStarted`, `TaskCompleted`, `TaskFailed`
   - `AgentSpawned`, `AgentExited`
   - `GateStarted`, `GateCompleted`, `GateFailed`
   - `RunStarted`, `RunCompleted`, `RunFailed`

4. **Wave transition stub**: If `RunnerEvent` lacks a wave variant, add a `PhantomData`-style placeholder comment noting the future variant. Do not block this item on the wave system.

5. **Line-flush guarantee**: Verify that `StructuredLogger` calls `flush()` after each write. If using `BufWriter`, call `flush()` explicitly so that consumers tailing the file see events in real time.

6. **Integration test**: Add a short integration test that runs a stub task, specifies `--log-file /tmp/test-roko.jsonl`, and asserts the file contains at least one JSON line with `event_type: "TaskCompleted"`.

## Acceptance Criteria

1. `roko plan run plans/ --log-file /tmp/roko.jsonl` creates the file and writes at least one JSON line per task event.
2. Each JSON line is a valid JSON object with `event_type` and `timestamp` fields.
3. Lines are flushed immediately so that `tail -f /tmp/roko.jsonl` shows events in real time.
4. The command works when `--log-file` is omitted (uses the noop logger).
5. `cargo test -p roko-cli structured_log` passes.

## Verification Checklist

- [ ] Inspect `do_cmd.rs` for `--log-file` flag; add if missing.
- [ ] Inspect `event_loop.rs` for `StructuredLogger.log()` call sites; add for each key event type.
- [ ] Run `roko plan run <small-plan> --log-file /tmp/test.jsonl` and verify the file exists with content.
- [ ] `cat /tmp/test.jsonl | jq .event_type` lists at least `TaskCompleted` or `TaskFailed`.
- [ ] Unit test: `StructuredLogger::open` + `log` + assert file content.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/structured_log.rs` | Verify flush behaviour; add line-flush if missing |
| `crates/roko-cli/src/runner/event_loop.rs` | Add `structured_logger.log()` call sites for all key events |
| `crates/roko-cli/src/commands/do_cmd.rs` | Verify or add `--log-file` CLI flag |
| `crates/roko-cli/tests/` | Add integration test for `--log-file` |
