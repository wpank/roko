# SAFE_20: MCP Server Version Pinning

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-20`](../ISSUE-TRACKER.md#safe-20)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.20
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Require explicit version pinning for MCP servers when
`safety.mcp.strict = true`. Reject `@latest` or unversioned packages.

## Exact Changes

1. Extend MCP config schema to include version and trust_level fields
2. When `safety.mcp.strict = true`, reject configs where version is
   `latest`, missing, or a range
3. Emit a warning for `unknown` trust_level servers
4. Block `unknown` servers when `safety.mcp.block_unknown = true`
5. Add validation to `roko config mcp list` output

## Write Scope

- `crates/roko-core/src/config/`
- `crates/roko-agent/src/mcp.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `@modelcontextprotocol/server-filesystem@latest` is rejected in strict mode
- [ ] `@modelcontextprotocol/server-filesystem@1.2.3` is accepted
- [ ] `roko config mcp list` shows version and trust level per server

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `@modelcontextprotocol/server-filesystem@latest` is rejected in strict mode
- `@modelcontextprotocol/server-filesystem@1.2.3` is accepted
- `roko config mcp list` shows version and trust level per server
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
