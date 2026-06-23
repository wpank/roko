# INNO_06: Define ModelContextProfile and calibration data

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-06`](../ISSUE-TRACKER.md#inno-06)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.6
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Models have wildly different effective context windows. A prompt that works well
with Claude's 200K window may be catastrophically pruned for a Cerebras 8B
model with an 8K effective window. The compose crate needs per-model calibration
data.

## Exact Changes

1. Define `ModelContextProfile` struct: `model_slug: String`,
   `context_window: usize`, `sweet_spot_range: (usize, usize)`,
   `degradation_threshold: usize`, `calibrated: bool`.
2. Implement `ModelContextProfile::default_for(slug: &str)` with known values:
   - `claude-opus-4-*`: context_window = 200_000, sweet_spot = (4000, 40_000)
   - `claude-sonnet-4-*`: context_window = 200_000, sweet_spot = (3000, 30_000)
   - `claude-haiku-4-*`: context_window = 200_000, sweet_spot = (2000, 20_000)
   - `gpt-4o*`: context_window = 128_000, sweet_spot = (3000, 30_000)
   - `gemini-*`: context_window = 1_000_000, sweet_spot = (5000, 50_000)
   - `cerebras-*`: context_window = 8_192, sweet_spot = (1000, 5_000)
   - Default fallback: context_window = 128_000, sweet_spot = (2000, 20_000)
3. Implement serde for persistence to `.roko/learn/model-profiles/{slug}.json`.
4. Implement `optimal_size(&self, content_tokens: usize) -> usize` that returns
   the ideal context size: `min(content_tokens, sweet_spot.1)`, clamped to
   `[sweet_spot.0, degradation_threshold]`.
5. Add `pub mod context_profile;` to `crates/roko-compose/src/lib.rs`.

## Write Scope

- `crates/roko-compose/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `ModelContextProfile::default_for("claude-sonnet-4-6")` returns a profile with context_window = 200_000
- [ ] Profile serializes to and deserializes from JSON
- [ ] Unit test: `optimal_size` returns a value within the sweet spot range for moderate content sizes

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `ModelContextProfile::default_for("claude-sonnet-4-6")` returns a profile with context_window = 200_000
- Profile serializes to and deserializes from JSON
- Unit test: `optimal_size` returns a value within the sweet spot range for moderate content sizes
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
