# PROM_21: Persist and Learn Foraging Profile Parameters

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-21`](../ISSUE-TRACKER.md#prom-21)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.21
- Priority: **??**
- Effort: 2-3 days | **Impact**: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `ModelAttentionCurves` supports per-model parameters but only
the default curve is populated. Hardcoded knowledge thresholds and episode
counts ignore tier.

## Exact Changes

1. Add `pub fn record_outcome(&mut self, source: &ContextSource, iterations: usize, items_found: usize, relevance_sum: f64)` to `MultiPatchForager`
2. Update `g_max` and `lambda` via EMA from observed data
3. Add `pub fn save(&self, path: &Path) -> std::io::Result<()>` and `pub fn load(path: &Path) -> std::io::Result<Option<Self>>` persistence methods
4. Persist profiles to `.roko/learn/foraging-profiles.json`
5. Load profiles at startup, falling back to hardcoded defaults

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 10+ runs, foraging profiles in `.roko/learn/foraging-profiles.json` reflect actual retrieval patterns
- [ ] Profile values drift toward observed data (not stuck at initial defaults)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 10+ runs, foraging profiles in `.roko/learn/foraging-profiles.json` reflect actual retrieval patterns
- Profile values drift toward observed data (not stuck at initial defaults)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
