# DISP_36: Auto-Populate CostTable from OpenRouter Metadata

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-36`](../ISSUE-TRACKER.md#disp-36)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.36
- Priority: **P3**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CostTable` in `task_runner.rs` stores per-model pricing for cost calculation. It is currently populated manually. The `OpenRouter` metadata helper at `provider/openrouter_meta.rs` can fetch live pricing via the OpenRouter catalog API but is not wired to auto-populate the cost table.

## Exact Changes

1. Add `pub async fn fetch_pricing(models: &[String]) -> HashMap<String, ModelPricing>` to `openrouter_meta.rs`
2. Cache fetched pricing to `.roko/learn/model-pricing.json` with a 24-hour TTL
3. On `CostTable` construction, if a cached pricing file exists and is fresh, load it
4. If stale or missing and an OpenRouter API key is configured, fetch and cache
5. Merge fetched pricing with any hardcoded defaults (hardcoded takes precedence for known models)

## Design Guidance

Pricing fetch should be opportunistic, not blocking. If the fetch fails (no API key, network error), use hardcoded defaults silently. The 24-hour TTL prevents excessive API calls while keeping prices reasonably current. Log when pricing is refreshed.

## Write Scope

- `crates/roko-agent/src/task_runner.rs`
- `crates/roko-agent/src/provider/openrouter_meta.rs`

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

- [ ] With OpenRouter API key set, pricing is fetched and cached
- [ ] Without API key, hardcoded defaults are used
- [ ] Cached pricing is used on subsequent startups within 24 hours

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With OpenRouter API key set, pricing is fetched and cached
- Without API key, hardcoded defaults are used
- Cached pricing is used on subsequent startups within 24 hours
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
