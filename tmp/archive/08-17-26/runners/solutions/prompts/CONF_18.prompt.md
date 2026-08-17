# CONF_18: Normalize Model Aliases at Load Time

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-18`](../ISSUE-TRACKER.md#conf-18)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.18
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Duplicate model entries exist: `glm-5-1` on provider "zai" vs `glm51` on provider
"zhipu" both resolve to `glm-5.1`. No `normalize_model` function exists in
`roko-core/src/config/` (confirmed via grep). `CascadeRouter` treats them as separate
models, fragmenting observations.

## Exact Changes

1. Add `normalize_model_slug(slug: &str) -> String` that canonicalizes known aliases:
   `glm-5-1` / `glm51` / `glm-5.1` -> `glm-5.1`,
   `claude-sonnet-4-6` / `claude-sonnet-4-6-20250514` -> canonical form.
2. Call in `resolve_model()` before returning.
3. Call in `CascadeRouter` before recording observations or selecting models.

## Write Scope

- `crates/roko-core/src/agent.rs`
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-learn/src/cascade_router.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Running tasks with both `glm-5-1` and `glm51` produces observations against a

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Running tasks with both `glm-5-1` and `glm51` produces observations against a
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
