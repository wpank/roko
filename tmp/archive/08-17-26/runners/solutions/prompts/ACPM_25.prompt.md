# ACPM_25: Wire Learned Strategy into roko-mcp-code

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-25`](../ISSUE-TRACKER.md#acpm-25)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.25
- Priority: **P2**
- Effort: 3 hours
- Depends on: `ACPM_24` (source 9.24)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-mcp-code`'s `search_code` tool accepts a `strategy` parameter with values `keyword`, `structural`, `hdc`, `embedding`, `hybrid`. When the strategy is omitted or `"auto"`, it should consult the `ToolEffectivenessBandit`.

## Exact Changes

1. Add `roko-learn` dependency to `roko-mcp-code/Cargo.toml`.
2. Load `ToolEffectivenessBandit` from `.roko/learn/tool-effectiveness.json` at server startup (pass workdir via env var `ROKO_WORKDIR`).
3. When `search_code` is called with `strategy: "auto"` or strategy omitted, call `bandit.recommend_strategy("search_code")`.
4. Use the recommended strategy for the search.
5. Fall back to "hybrid" when bandit has fewer than 10 total observations.
6. Explicit strategy parameter overrides bandit recommendation.

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

- [ ] First 10 calls use "hybrid" (cold start default)
- [ ] After training data accumulates, "auto" selects learned-best strategy
- [ ] Explicit strategy parameter ("keyword") overrides bandit recommendation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- First 10 calls use "hybrid" (cold start default)
- After training data accumulates, "auto" selects learned-best strategy
- Explicit strategy parameter ("keyword") overrides bandit recommendation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
