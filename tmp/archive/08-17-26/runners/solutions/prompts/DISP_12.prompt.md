# DISP_12: Migrate episode_completion.rs to Injected ModelCaller

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-12`](../ISSUE-TRACKER.md#disp-12)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.12
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The episode completion distillation at `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs` (re-exported from `lib.rs`) has a fallback path at line 46 that reads `ANTHROPIC_API_KEY` directly:
```rust
let Some(api_key) = std::env::var("ANTHROPIC_API_KEY")
    .ok()
    .map(|key| key.trim().to_owned())
    .filter(|key| !key.is_empty())
```

The function `distill_episode()` already accepts `model_caller: Option<Arc<dyn ModelCaller>>` and uses it when available (line 40-44). The env var fallback is only for when no `ModelCaller` is provided.

## Exact Changes

1. In `spawn_episode_distillation()`, ensure all callers provide a `model_caller`. Search for `spawn_episode_distillation` in the codebase to find all call sites.
2. If all callers can provide a `ModelCaller`, remove the `Option` wrapper -- make it `Arc<dyn ModelCaller>` (required). If some callers cannot, keep `Option` but emit a `tracing::warn!` and return early when `None`, rather than falling back to raw env var.
3. Remove the `std::env::var("ANTHROPIC_API_KEY")` fallback path
4. Remove the `Distiller::with_claude(api_key)` call that builds its own HTTP client
5. The `GATEWAY_DISTILLATION_MODEL` constant ("claude-haiku-3-5") should be configurable via config, not hardcoded

## Design Guidance

The `ModelCaller` trait (`roko_core::foundation::ModelCaller`) is the correct abstraction for all LLM calls. The raw API key path was a bootstrap hack. All callers should construct a `ModelCaller` via `ModelCallService` which handles credential resolution through the provider system.

## Write Scope

- `crates/roko-neuro/src/episode_completion.rs`

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

- [ ] `grep -n 'std::env::var.*ANTHROPIC_API_KEY' crates/roko-neuro/src/` returns zero results
- [ ] Distillation still works when a `ModelCaller` is provided

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'std::env::var.*ANTHROPIC_API_KEY' crates/roko-neuro/src/` returns zero results
- Distillation still works when a `ModelCaller` is provided
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
