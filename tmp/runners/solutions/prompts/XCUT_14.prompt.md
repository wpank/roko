# XCUT_14: Propagate CancelToken Through All Dispatch Paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-14`](../ISSUE-TRACKER.md#xcut-14)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.14
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CancelToken` from `crates/roko-runtime/src/cancel.rs` supports hierarchical cancellation (parent cancels children). It is used in 20 files. However, key dispatch paths lack it:

- `crates/roko-cli/src/chat_inline.rs` -- no CancelToken; Ctrl-C during chat leaves orphan agent processes.
- `crates/roko-cli/src/run.rs` -- creates its own ad-hoc cancellation via `tokio::signal::ctrl_c()`.
- `crates/roko-cli/src/run_inline.rs` -- similar ad-hoc cancellation.

The WorkflowEngine (`crates/roko-runtime/src/workflow_engine.rs`) does accept a `CancelToken` in its run method. The ACP server (`crates/roko-acp/src/session.rs`) creates per-session `CancelToken`s. The plan runner (`crates/roko-cli/src/runner/event_loop.rs`) uses `CancelToken`. The gap is the CLI entry points that should create a root token.

## Exact Changes

1. In `main.rs`, create a root `CancelToken` and wire SIGINT/SIGTERM to call `root.cancel()`.
2. Pass `root.child()` to every dispatch entry point: `run()`, `chat_inline()`.
3. In each dispatch function, check `cancel.is_cancelled()` before each major phase (agent spawn, gate run, merge).
4. Replace any `tokio::signal::ctrl_c().await` with `cancel.cancelled().await` in select loops.
5. Ensure the ACP server's per-session `CancelToken` is a child of the root token.

## Design Guidance

The root `CancelToken` should be created once in `main()` and passed as an argument to command dispatch functions. Do not store it in a static or global; the hierarchical parent-child relationship is the correct pattern. When a child is cancelled, it does not cancel the parent, but when the root is cancelled, all children are cancelled.

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/chat_inline.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/run_inline.rs`

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

- [ ] Ctrl-C in `roko run "hello"` cancels within 2 seconds (no orphan agent processes)
- [ ] Ctrl-C in `roko chat` cancels the current agent call and returns to the prompt
- [ ] Ctrl-C in `roko plan run` cancels the current task, persists state, and exits
- [ ] `cancel.is_cancelled()` is checked in all dispatch paths

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Ctrl-C in `roko run "hello"` cancels within 2 seconds (no orphan agent processes)
- Ctrl-C in `roko chat` cancels the current agent call and returns to the prompt
- Ctrl-C in `roko plan run` cancels the current task, persists state, and exits
- `cancel.is_cancelled()` is checked in all dispatch paths
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
