# CONF_07: Replace Direct `ANTHROPIC_API_KEY` Read in `episode_completion.rs`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-07`](../ISSUE-TRACKER.md#conf-07)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.7
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`crates/roko-neuro/src/episode_completion.rs:46` reads `ANTHROPIC_API_KEY` directly
from the environment to construct its own HTTP client for neuro distillation calls.
This bypasses the provider system, credential rotation, and cost tracking.

## Exact Changes

1. Add a `model_caller: Option<Arc<dyn ModelCaller>>` parameter to the distillation
   entry point (or accept a configured `ModelCallService`).
2. Remove the direct `std::env::var("ANTHROPIC_API_KEY")` read.
3. The caller passes the `ModelCallService` it already has access to.
4. If no model caller is available, skip distillation with a logged warning.

## Write Scope

- `crates/roko-neuro/src/episode_completion.rs`

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

- [ ] `episode_completion.rs` has zero `std::env::var` calls.
- [ ] Distillation works when configured through provider config.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `episode_completion.rs` has zero `std::env::var` calls.
- Distillation works when configured through provider config.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
