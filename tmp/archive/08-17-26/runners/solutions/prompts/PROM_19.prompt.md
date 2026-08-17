# PROM_19: Wire VCG Allocation as Actual Allocator

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-19`](../ISSUE-TRACKER.md#prom-19)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.19
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When strategy resolves to `Vcg`, use VCG welfare-maximizing
allocation to determine section inclusion, not just post-hoc diagnostics.

## Exact Changes

1. In `PromptComposer::compose()` (or equivalent), when resolved strategy is `Vcg`:
   - Call `vcg_allocate()` with current sections, budget, and bidder values
   - Use VCG allocation result to determine which sections are included
   - Store payments in `CompositionManifest` for observability
2. When strategy is `DensityGreedy` (cold start): keep existing greedy behavior
3. Add config guard: `composition.vcg_enabled = true` (default true) to allow disabling

## Write Scope

- `crates/roko-compose/src/prompt.rs`

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

- [ ] With 5+ warm bidders and `vcg_enabled = true`, VCG determines section inclusion
- [ ] VCG allocation respects the tier token budget as a hard ceiling
- [ ] Payments are recorded in `CompositionManifest`
- [ ] DensityGreedy still works when VCG is disabled or bidders are cold

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With 5+ warm bidders and `vcg_enabled = true`, VCG determines section inclusion
- VCG allocation respects the tier token budget as a hard ceiling
- Payments are recorded in `CompositionManifest`
- DensityGreedy still works when VCG is disabled or bidders are cold
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
