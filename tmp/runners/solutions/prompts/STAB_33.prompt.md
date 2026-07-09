# STAB_33: Remove direct env var reads for API keys

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-33`](../ISSUE-TRACKER.md#stab-33)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.33
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Two live code paths read API keys directly from environment variables (`ANTHROPIC_API_KEY`,
`PERPLEXITY_API_KEY`) instead of going through provider configuration. Keys are not tracked
in cost accounting.

## Exact Changes

1. `episode_completion.rs`: accept a configured `Agent` or `ModelCallService` through
   dependency injection instead of constructing its own HTTP client with env var key.
2. `web_search.rs`: accept a provider config or API key through the tool's configuration
   rather than reading env vars directly.
3. Remove the `std::env::var("ANTHROPIC_API_KEY")` and `std::env::var("PERPLEXITY_API_KEY")` calls.
4. Fall back to the provider config's `api_key_env` pattern for resolution.

## Design Guidance

API key resolution should always go through the provider configuration system. If a
provider is configured with `api_key_env = "ANTHROPIC_API_KEY"`, the provider system reads
that env var -- individual subsystems should not.

## Write Scope

- `crates/roko-neuro/src/episode_completion.rs`
- `crates/roko-std/src/tool/builtin/web_search.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -rn 'env::var.*API_KEY' crates/roko-neuro/ crates/roko-std/` returns zero matches
- [ ] Both subsystems function when key is configured in `roko.toml` providers

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'env::var.*API_KEY' crates/roko-neuro/ crates/roko-std/` returns zero matches
- Both subsystems function when key is configured in `roko.toml` providers
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
