# ACPM_21: Add Federation to roko-mcp-github

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-21`](../ISSUE-TRACKER.md#acpm-21)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.21
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ACPM_19` (source 9.19)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-mcp-github` at `crates/roko-mcp-github/src/main.rs` (~800 LOC) exposes GitHub API tools. Adding a `pr_impact_analysis` tool that queries code intelligence enables change impact analysis.

## Exact Changes

1. Add `roko-mcp-stdio` dependency.
2. Add optional `FederatedClient` to the GitHub server's state.
3. Add `pr_impact_analysis` tool that:
   - Takes `{ pr_number }` args
   - Calls `github_get_pr` locally for the PR diff
   - Extracts changed function/struct names from the diff (simple regex on `fn `, `struct `, `impl `)
   - If `FederatedClient` available, calls `call_graph` via federation for each changed function (max 10)
   - Returns: affected functions, call chains, test coverage gaps
4. Register the GitHub server with the MCP registry on startup.
5. When code MCP is not running, return diff-only analysis.

## Write Scope

- `crates/roko-mcp-github/src/main.rs`
- `crates/roko-mcp-github/Cargo.toml`

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

- [ ] `pr_impact_analysis` returns call graph data for changed functions
- [ ] When code MCP is not running, returns diff-only analysis
- [ ] Tool handles large PRs (>50 files) by capping analysis to 10 functions

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `pr_impact_analysis` returns call graph data for changed functions
- When code MCP is not running, returns diff-only analysis
- Tool handles large PRs (>50 files) by capping analysis to 10 functions
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
