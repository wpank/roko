# ACPM_29: Wire ACP Permission Bridge

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-29`](../ISSUE-TRACKER.md#acpm-29)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.29
- Priority: **P2**
- Effort: 5 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ACP supports `session/request_permission` (agent -> editor) for user approval of destructive actions. The `StdioTransport` at `crates/roko-acp/src/transport.rs` supports `send_request()` with pending response tracking. The capability is declared but underutilized.

## Exact Changes

1. In the runner, when a destructive action is detected (e.g., file deletion, dangerous command), construct a permission request:
   ```json
   {
     "method": "session/request_permission",
     "params": {
       "title": "Delete file",
       "description": "The agent wants to delete src/old_module.rs",
       "permissions": [{ "name": "file_delete", "description": "Delete src/old_module.rs", "destructive": true }]
     }
   }
   ```
2. Send via `transport.send_request()` and await the editor's response.
3. Parse the response: `approved: true/false`.
4. If approved, proceed with the action.
5. If denied, feed `AgentFailed { error: "Permission denied by user for: {action}" }` to the pipeline.
6. Non-destructive actions skip the permission check.
7. Add a configurable timeout (default 60s) for the permission prompt -- if no response, deny.

## Write Scope

- `crates/roko-acp/src/handler.rs`
- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Agent requesting file deletion triggers permission prompt
- [ ] User approval allows the action to proceed
- [ ] User denial feeds back as agent failure with clear reason
- [ ] Timeout results in denial

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agent requesting file deletion triggers permission prompt
- User approval allows the action to proceed
- User denial feeds back as agent failure with clear reason
- Timeout results in denial
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
