# PROM_23: Wire Per-Model Curves into dynamic_placement()

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-23`](../ISSUE-TRACKER.md#prom-23)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.23
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When `dynamic_placement()` is called (line 98), look up the
model-specific curve instead of always using the default.

## Exact Changes

1. Add `model_slug: Option<&str>` parameter to `dynamic_placement()` signature
2. Load `ModelAttentionCurves` from `.roko/learn/attention-curves.json` (or use `default_model_curves()`)
3. Use `curves.for_model(slug)` to get the appropriate curve
4. Apply the model-specific curve when computing the information density threshold for placement decisions
5. Update callers in `role_prompts.rs` to pass the model slug

## Write Scope

- `crates/roko-compose/src/attention.rs`

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

- [ ] Dispatching to Haiku uses the Haiku curve (stronger primacy -> critical sections at start)
- [ ] Dispatching to Opus uses the Opus curve (less aggressive placement optimization)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Dispatching to Haiku uses the Haiku curve (stronger primacy -> critical sections at start)
- Dispatching to Opus uses the Opus curve (less aggressive placement optimization)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
