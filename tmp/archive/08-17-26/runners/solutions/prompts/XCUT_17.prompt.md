# XCUT_17: Add Shutdown Hooks for ACP Server

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-17`](../ISSUE-TRACKER.md#xcut-17)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.17
- Priority: **P8**
- Effort: 3 hours
- Depends on: `XCUT_14` (source 19.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The ACP server runs as a subprocess of editors (Zed, JetBrains). When the editor sends EOF on stdin, the ACP server should drain active sessions, flush episodes, and persist cascade router state. Currently it exits immediately on EOF. `crates/roko-acp/src/session.rs` already creates per-session `CancelToken`s but there is no coordinated shutdown sequence.

## Exact Changes

1. In `run_acp_server()` handler, when the transport returns `None` (EOF), enter shutdown.
2. Create `GracefulShutdown::with_deadline(Duration::from_secs(3))`.
3. Register hooks:
   - `"active-sessions"`: cancel all active session CancelTokens, wait for prompts to complete.
   - `"episode-flush"`: flush any buffered episode data to `.roko/episodes.jsonl`.
   - `"router-save"`: persist cascade router to `.roko/learn/cascade-router.json`.
   - `"session-persist"`: save all session state to disk for later `session/load`.
4. After drain, exit cleanly with code 0.

## Write Scope

- `crates/roko-acp/src/handler.rs`
- `crates/roko-acp/src/session.rs`
- `crates/roko-acp/src/bridge_events.rs`

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

- [ ] ACP server flushes episodes on editor close
- [ ] Active prompts are cancelled (not left running as orphan processes)
- [ ] Session state is persisted for later resume
- [ ] Shutdown completes within 3 seconds

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP server flushes episodes on editor close
- Active prompts are cancelled (not left running as orphan processes)
- Session state is persisted for later resume
- Shutdown completes within 3 seconds
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
