# PROM_22: Populate Initial Attention Curves for Major Models

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-22`](../ISSUE-TRACKER.md#prom-22)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.22
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add hardcoded initial curves for major model families. The
`ModelAttentionCurves` struct is at line 58; `PositionAttentionModel`
default is at line 28.

## Exact Changes

1. Add `pub fn default_model_curves() -> ModelAttentionCurves`
2. Populate curves for:
   - `claude-opus-4` / `claude-3-opus`: `primacy=0.30, recency=0.35, baseline=0.35` (less middle degradation)
   - `claude-sonnet-4` / `claude-3.5-sonnet`: default curve (0.35, 0.30, 0.35)
   - `claude-haiku` / `claude-3-haiku`: `primacy=0.40, recency=0.25, baseline=0.35` (stronger primacy bias)
   - `gpt-4` / `gpt-4o`: `primacy=0.35, recency=0.30, baseline=0.35`
   - `gpt-4o-mini`: `primacy=0.38, recency=0.27, baseline=0.35`
3. Wire `default_model_curves()` as initialization path when no persisted curves exist

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

- [ ] `ModelAttentionCurves::default_model_curves().for_model("claude-opus-4")` returns Opus curve (not default)
- [ ] `ModelAttentionCurves::default_model_curves().for_model("unknown-model")` returns default curve

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `ModelAttentionCurves::default_model_curves().for_model("claude-opus-4")` returns Opus curve (not default)
- `ModelAttentionCurves::default_model_curves().for_model("unknown-model")` returns default curve
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
