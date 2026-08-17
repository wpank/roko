# XCUT_05: Wire RPC Error Codes Across All JSON-RPC Surfaces

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-05`](../ISSUE-TRACKER.md#xcut-05)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.5
- Priority: **P7**
- Effort: 3 hours
- Depends on: `XCUT_03` (source 19.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`crates/roko-core/src/error/rpc.rs` defines `RpcError` with standard JSON-RPC codes (PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND, INVALID_PARAMS, INTERNAL_ERROR) and custom Roko codes (AGENT_FAILURE=-32000, GATE_FAILURE=-32001, TIMEOUT=-32002, BUDGET_EXCEEDED=-32003). But 10 files use `RpcError` with varying completeness, and ACP defines its own inline error codes (`SESSION_BUSY` as -32001 which collides with GATE_FAILURE).

## Exact Changes

1. Add ACP-specific error codes to `rpc.rs`: `SESSION_BUSY=-32010`, `SESSION_NOT_FOUND=-32011`, `PROMPT_TOO_LONG=-32012` (shifted to avoid collision with existing -32001 GATE_FAILURE).
2. Add MCP-specific error codes: `TOOL_NOT_FOUND=-32020`, `TOOL_TIMEOUT=-32021`, `SCRIPT_FAILED=-32022`.
3. In ACP `types.rs` and `transport.rs`, replace inline `serde_json::json!({"code": ..., ...})` with `RpcError::new(SESSION_BUSY, ...)`.
4. In each MCP crate's `main.rs` / `lib.rs`, replace inline error construction with `RpcError` conversions.
5. Document all codes in `rpc.rs` module docs.

## Write Scope

- `crates/roko-core/src/error/rpc.rs`
- `crates/roko-acp/src/types.rs`
- `crates/roko-acp/src/transport.rs`
- `crates/roko-mcp-stdio/src/lib.rs`
- `crates/roko-mcp-code/src/lib.rs`
- `crates/roko-mcp-github/src/main.rs`
- `crates/roko-mcp-slack/src/main.rs`
- `crates/roko-mcp-scripts/src/main.rs`

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

- [ ] All JSON-RPC error responses across ACP and MCP go through `RpcError`
- [ ] No inline error code constants remain in ACP/MCP crates
- [ ] Error code documentation is complete in `rpc.rs`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All JSON-RPC error responses across ACP and MCP go through `RpcError`
- No inline error code constants remain in ACP/MCP crates
- Error code documentation is complete in `rpc.rs`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
