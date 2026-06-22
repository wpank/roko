# DISP_04: Wire CascadeRouter into `roko plan run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-04`](../ISSUE-TRACKER.md#disp-04)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.4
- Priority: **P0**
- Effort: 3 hours
- Depends on: `DISP_01` (source 3.1), `DISP_02` (source 3.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko plan run` already loads a `CascadeRouter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs:322-323`:
```rust
let cascade_router = std::sync::Arc::new(
    roko_learn::cascade_router::CascadeRouter::load_or_new(&router_path, model_slugs),
);
```

But this router is only passed to the Runner v2 infrastructure (runner/types.rs). The model selection calls at lines 559 and 608 use `resolve_effective_model_key()` which hardcodes `None`. After Task 3.2, those calls accept a router parameter.

The runner v2 at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs:1306` loads its own router separately from the one in `plan.rs`. These should be unified.

## Exact Changes

1. Replace the inline `CascadeRouter::load_or_new` at plan.rs:322 with `load_cascade_router(&wd, &roko_config)` (from Task 3.1)
2. Pass `Some(&cascade_router)` to the `resolve_effective_model_key()` calls at lines 559 and 608
3. After plan execution completes, call `save_cascade_router(&wd, &cascade_router)`
4. Ensure runner/types.rs receives the same router instance (via `Arc`) rather than loading its own copy. The runner constructor at types.rs:1306 should accept an `Arc<CascadeRouter>` parameter.

## Design Guidance

`roko plan run` executes many tasks. Each task's model selection should consult the router, and each task's result should feed back as an observation. The router must be `Arc<CascadeRouter>` since the runner holds it across async task boundaries. Save after all tasks complete, not after each task (the router's internal Mutex handles concurrent updates).

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`

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

- [ ] `roko plan run plans/` uses CascadeRouter for model selection
- [ ] After plan execution, `.roko/learn/cascade-router.json` is updated with new observations
- [ ] The router loaded in plan.rs is the same instance used by runner/types.rs (no double-load)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run plans/` uses CascadeRouter for model selection
- After plan execution, `.roko/learn/cascade-router.json` is updated with new observations
- The router loaded in plan.rs is the same instance used by runner/types.rs (no double-load)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
