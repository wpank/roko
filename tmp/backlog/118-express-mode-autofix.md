# 118 — Express Mode Auto-Fix (`cargo fix` + Retry Loop)

**Priority**: P2 — Backlog #05 covers the strategist bypass but not the `cargo fix --allow-dirty` auto-fixer agent dispatch and the up-to-3 retry loop that makes express mode useful for routine compile fixes.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/do_cmd.rs`
**Depends on**: Backlog #05 (Express Mode — strategist bypass predicate must exist first)
**Sources**: `tmp/backlog/_checklist-gaps.md` §1.3, `tmp/backlog/_mori-old-gaps.md` MO-10

---

## Background

Backlog #05 (Express Mode) specifies how roko skips strategist and review phases when `--express` is set. It explicitly marks `cargo fix --allow-dirty` and the auto-fixer retry loop as out of scope. This item covers that complement.

In practice, the most time-consuming part of a gate failure cycle is the round-trip: gate fails → agent re-implements the change → gate runs again. For routine compile errors (unused imports, type mismatches, missing derives), `cargo fix --allow-dirty` resolves 60-80% of failures without any LLM invocation. The auto-fix sequence only escalates to an LLM when `cargo fix` doesn't resolve the error.

Mori's express mode sequence for compile failures was:
1. Run `cargo fix --allow-dirty`
2. If gate passes: continue (no LLM call consumed).
3. If gate still fails: dispatch a lightweight auto-fixer agent with the error digest and a constrained prompt ("fix only the compile errors listed").
4. Retry gate. If it passes: continue.
5. If still failing after 3 rounds: fail-forward (mark the task failed, move on).

## Current State

- Backlog #05 tracks the strategist bypass predicate and phase transition logic.
- `--express` CLI flag: may or may not exist (backlog #05 is the tracker); this item assumes it exists or adds it.
- No `cargo fix --allow-dirty` invocation exists anywhere in the gate/retry path.
- No auto-fixer agent dispatch is wired in the retry loop.
- `runner/event_loop.rs` — has a `RetryAction` enum with `Retry` and `MarkFatal`; no `AutoFix` variant.

## Implementation Plan

1. **Add `RetryAction::AutoFix` variant**: Extend the `RetryAction` enum in `event_loop.rs` (or `types.rs`) with:
   ```rust
   AutoFix {
       attempt: u32,
       max_attempts: u32,
   }
   ```

2. **Auto-fix decision logic**: After a compile gate failure in express mode, choose `RetryAction::AutoFix` on attempts 1-3. After 3 `AutoFix` attempts, fall through to `MarkFatal` (fail-forward).

3. **`cargo fix --allow-dirty` step**: Before dispatching the auto-fixer agent, run `cargo fix --allow-dirty` in the worktree. This is a `tokio::process::Command` call, not an LLM dispatch. If it exits 0 and the gate subsequently passes, log "auto-fixed without LLM" in the efficiency event.

4. **Auto-fixer agent dispatch** (only when `cargo fix` alone fails):
   - Role: `AutoFixer` (lightweight, no context budget beyond the error digest)
   - Prompt: "Fix only the compile errors listed below. Do not change any other code."
   - Error digest: the last 50 lines of gate stderr, filtered to error-level messages only.
   - Model: cheapest available (fast tier).

5. **`--express` flag on `plan run`**: Verify the flag is present (backlog #05 may have added it); if not, add `express: bool` to the run config struct.

6. **Retry counter persistence**: Track per-task auto-fix attempts in the runner state snapshot so that on resume, attempts are not reset.

7. **Efficiency logging**: In express mode auto-fix, emit a special `AgentEfficiencyEvent` with `role: "auto-fixer"`, `model: "cargo-fix"` (non-LLM), and `tokens_used: 0` when `cargo fix` resolves the issue without LLM.

## Acceptance Criteria

1. In `--express` mode, a compile gate failure triggers `cargo fix --allow-dirty` before dispatching an LLM.
2. If `cargo fix` resolves the error, the gate passes and no LLM is invoked for that retry.
3. If `cargo fix` fails, an auto-fixer agent is dispatched with a constrained prompt.
4. After 3 failed auto-fix attempts, the task is marked failed and execution continues (fail-forward).
5. The auto-fix attempt count is included in the efficiency event.
6. Non-express mode is unaffected.

## Verification Checklist

- [ ] Create a task with a deliberate unused-import compile error; run in `--express` mode; verify `cargo fix` resolves it without LLM invocation.
- [ ] Create a task with an error `cargo fix` cannot resolve; verify auto-fixer agent is dispatched.
- [ ] After 3 auto-fix attempts, verify the task is marked failed and the runner continues with the next task.
- [ ] Verify efficiency event includes `auto_fix_attempts: 2` or similar field.
- [ ] Run without `--express` and verify no auto-fix behaviour occurs.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Add `RetryAction::AutoFix`; wire `cargo fix` and auto-fixer agent dispatch |
| `crates/roko-cli/src/runner/types.rs` | Add `AutoFix` variant to `RetryAction` |
| `crates/roko-cli/src/commands/do_cmd.rs` | Verify or add `--express` flag |
| `crates/roko-cli/src/runner/mod.rs` | Export auto-fix utilities if extracted |
