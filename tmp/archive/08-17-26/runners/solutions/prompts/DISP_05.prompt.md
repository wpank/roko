# DISP_05: Wire CascadeRouter into `roko chat` / chat_inline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-05`](../ISSUE-TRACKER.md#disp-05)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.5
- Priority: **P0**
- Effort: 4 hours
- Depends on: `DISP_01` (source 3.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The interactive chat REPL (`roko chat`, `roko <prompt>`) at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` calls `resolve_effective_model` at line 1466 with no cascade router. The `dispatch_via_model_call_service` call at line 1780 creates a `ModelCallService` without a router.

The chat session can last many turns. The router should be loaded at session start, consulted each turn, and persisted on session exit.

## Exact Changes

1. Load the CascadeRouter at chat session initialization (near the top of the main chat loop)
2. Pass the router reference to `resolve_effective_model()` at line 1466
3. For the `dispatch_via_model_call_service()` path at line 1780, the `ModelCallService` should receive the router via `with_cascade_router()`. Modify `dispatch_via_model_call_service()` in `dispatch_v2.rs` to accept an optional `Arc<CascadeRouter>` parameter.
4. Persist the router on session exit (both clean exit via `/exit` and Ctrl-C handler)
5. Use a `Drop` guard or explicit save in the session shutdown path

## Design Guidance

Chat sessions are long-lived. The router learns within a session and carries those observations to the next session. Since `dispatch_via_model_call_service` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs:53` constructs a fresh `ModelCallService` per call, the router should be passed via `with_cascade_router(Arc::clone(&router))` each time rather than stored in the service.

## Write Scope

- `crates/roko-cli/src/chat_inline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Interactive `roko chat` session uses CascadeRouter
- [ ] After several turns and exit, `.roko/learn/cascade-router.json` is updated
- [ ] `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/chat_inline.rs` shows both

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Interactive `roko chat` session uses CascadeRouter
- After several turns and exit, `.roko/learn/cascade-router.json` is updated
- `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/chat_inline.rs` shows both
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
