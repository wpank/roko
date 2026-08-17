# PROM_20: Wire MultiPatchForager into Context Retrieval

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-20`](../ISSUE-TRACKER.md#prom-20)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.20
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Replace direct knowledge/playbook/anti-pattern queries with
forager-driven retrieval that optimizes visitation order and stopping.

## Exact Changes

1. Import `roko_compose::foraging::{MultiPatchForager, SourceForagingProfile}`
2. Build `SourceForagingProfile` entries for each context source:
   - Knowledge store: `g_max=0.8, lambda=0.3, travel_cost=0.05`
   - Playbook store: `g_max=0.6, lambda=0.5, travel_cost=0.03`
   - Code index: `g_max=0.7, lambda=0.4, travel_cost=0.1`
   - Episode history: `g_max=0.4, lambda=0.6, travel_cost=0.02`
3. Call `forager.optimal_order()` to determine which sources to visit first
4. For each source, call `forager.optimal_iterations()` for iteration count
5. After each batch, check `should_stop_searching()` with `estimate_context_sufficiency()`
6. Stop early when sufficiency >= 0.85 or MVT ratio drops below threshold

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

- [ ] Context retrieval visits sources in priority order (not unconditionally)
- [ ] Retrieval stops early when sufficient context is gathered
- [ ] Simple tasks (Surgical tier) do fewer retrievals than complex tasks (Full tier)
- [ ] Log output shows foraging decisions: "visited knowledge_store (3 iterations), stopped: sufficiency=0.87"

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Context retrieval visits sources in priority order (not unconditionally)
- Retrieval stops early when sufficient context is gathered
- Simple tasks (Surgical tier) do fewer retrievals than complex tasks (Full tier)
- Log output shows foraging decisions: "visited knowledge_store (3 iterations), stopped: sufficiency=0.87"
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
