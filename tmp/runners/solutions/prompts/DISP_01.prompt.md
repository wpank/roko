# DISP_01: Add load/save Helpers for CascadeRouter in model_selection.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-01`](../ISSUE-TRACKER.md#disp-01)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.1
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CascadeRouter` is a 3-stage LinUCB contextual bandit at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`. It has `load_or_new(path, model_slugs)` (line 1596) and `save(path)` (line 1578) methods. The serve runtime loads it at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs:522`. The runner v2 loads it at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs:1306`. Both use inline `load_or_new` calls with ad-hoc model slug extraction.

`model_selection.rs` already imports `CascadeRouter` at line 7 (`use roko_learn::cascade_router::CascadeRouter;`) and accepts `Option<&CascadeRouter>` in `resolve_effective_model` (line 144). But there is no centralized load/save utility, so every call site reimplements slug extraction + path construction.

`RokoConfig::effective_models()` returns the merged model map. The convention is `.roko/learn/cascade-router.json` for the persistence path.

## Exact Changes

1. Add `pub fn load_cascade_router(workdir: &Path, config: &RokoConfig) -> CascadeRouter`:
   - Build path as `workdir.join(".roko/learn/cascade-router.json")`
   - Extract model slugs from `config.effective_models().keys()`, sort and dedup
   - If slugs are empty and `config.agent.default_model` is non-empty, push it as the sole slug
   - Call `CascadeRouter::load_or_new(&path, model_slugs)`
   - This matches the pattern at `crates/roko-cli/src/commands/plan.rs:311-323`
2. Add `pub fn save_cascade_router(workdir: &Path, router: &CascadeRouter) -> std::io::Result<()>`:
   - Build path as `workdir.join(".roko/learn/cascade-router.json")`
   - Ensure parent directory exists via `std::fs::create_dir_all`
   - Call `router.save(&path)`
3. Add a `#[cfg(test)]` roundtrip test: create a temp dir, load (creates fresh), save, load again, verify model slugs are preserved

## Design Guidance

Use the same path convention as `serve_runtime.rs:522` and `commands/plan.rs:311`. Do not introduce a new path constant -- inline the join. The function should be infallible (returns a fresh router if the file is missing or corrupt), matching `load_or_new` semantics.

## Write Scope

- `crates/roko-cli/src/model_selection.rs`

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

- [ ] `grep -n 'load_cascade_router' crates/roko-cli/src/model_selection.rs` shows the new function
- [ ] `grep -n 'save_cascade_router' crates/roko-cli/src/model_selection.rs` shows the companion

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'load_cascade_router' crates/roko-cli/src/model_selection.rs` shows the new function
- `grep -n 'save_cascade_router' crates/roko-cli/src/model_selection.rs` shows the companion
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
