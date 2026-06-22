# ACPM_20: Add Federation to roko-mcp-code

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-20`](../ISSUE-TRACKER.md#acpm-20)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.20
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ACPM_19` (source 9.19)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-mcp-code` at `crates/roko-mcp-code/src/lib.rs` (~1,500 LOC) exposes 12 code intelligence tools. It has no awareness of other MCP servers. Adding a federated `github_enriched_context` tool enables compound queries.

## Exact Changes

1. Add `roko-mcp-stdio` dependency to `roko-mcp-code/Cargo.toml`.
2. Add optional `FederatedClient` to the code server's state (constructed from registry path env var).
3. Add `github_enriched_context` tool that:
   - Takes `{ symbol_name, pr_number }` args
   - Calls `symbol_lookup` locally for code context
   - If `FederatedClient` is available, calls `github_get_pr` via federation for PR diff
   - Merges results: "Symbol X was modified in PR #N, here is the change"
4. Register the code server with the MCP registry on startup.
5. If federation client is not available, the tool returns code-only results with a warning.
6. Add the new tool to the `tools/list` response.

## Write Scope

- `crates/roko-mcp-code/src/lib.rs`
- `crates/roko-mcp-code/Cargo.toml`

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

- [ ] When GitHub MCP is running: `github_enriched_context` returns merged result
- [ ] When GitHub MCP is not running: returns code-only result with warning
- [ ] New tool appears in `tools/list` response

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- When GitHub MCP is running: `github_enriched_context` returns merged result
- When GitHub MCP is not running: returns code-only result with warning
- New tool appears in `tools/list` response
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
