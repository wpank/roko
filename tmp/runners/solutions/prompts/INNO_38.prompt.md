# INNO_38: Implement A2A JSON-RPC endpoint (server)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-38`](../ISSUE-TRACKER.md#inno-38)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.38
- Priority: **P3**
- Effort: 12 hours
- Depends on: `INNO_36` (source 11.36), `INNO_37` (source 11.37)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_38 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Implement `POST /a2a` route accepting JSON-RPC 2.0 requests.
2. Dispatch based on method: `tasks/send`, `tasks/get`, `tasks/cancel`, `tasks/sendSubscribe`.
3. Map A2A skills to internal AgentRole dispatch.
4. Stream progress updates via SSE for `sendSubscribe`.
5. Return A2A-compliant task completion with artifacts.

## Write Scope

- `crates/roko-serve/src/routes/a2a.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] External A2A client sends `tasks/send` with code-implementation skill, roko executes and returns result
- [ ] `tasks/cancel` correctly cancels a running agent
- [ ] SSE stream shows progress updates for subscribed tasks
- [ ] Invalid JSON-RPC returns proper error response

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_38 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- External A2A client sends `tasks/send` with code-implementation skill, roko executes and returns result
- `tasks/cancel` correctly cancels a running agent
- SSE stream shows progress updates for subscribed tasks
- Invalid JSON-RPC returns proper error response
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_38 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
