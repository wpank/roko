# DISP_03: Wire CascadeRouter into `roko run` Entry Point

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-03`](../ISSUE-TRACKER.md#disp-03)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.3
- Priority: **P0**
- Effort: 4 hours
- Depends on: `DISP_01` (source 3.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko run "<prompt>"` is the primary CLI dispatch entry point. The model selection call at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs:452` currently passes `None` for cascade_router:
```rust
let selection = resolve_effective_model(cli_model_override, None, role, None, &model_config)
```

The `ServiceFactory::build()` call at line 466 feeds into `WorkflowEngine` but never loads a CascadeRouter. The dead `orchestrate.rs` was the only path that used the router with the workflow engine.

## Exact Changes

1. In the function that builds the model selection (around line 430-456), add:
   ```rust
   let cascade_router = load_cascade_router(&workdir, &model_config);
   ```
2. Change line 452 to pass `Some(&cascade_router)`:
   ```rust
   let selection = resolve_effective_model(cli_model_override, None, role, Some(&cascade_router), &model_config)
   ```
3. After the workflow engine completes (both success and error paths), persist the router:
   ```rust
   if let Err(e) = save_cascade_router(&workdir, &cascade_router) {
       tracing::warn!(error = %e, "failed to persist cascade router");
   }
   ```
4. Ensure the save happens even on early returns. Add save calls in all exit paths, or use a helper struct with `Drop` that calls save.
5. The `ServiceFactory` in `crates/roko-orchestrator/src/service_factory.rs:123` already loads a router. If the run.rs path builds its own `ServiceFactory`, verify it passes the same router to avoid double-loading.

## Design Guidance

Load once, pass by reference everywhere, save once at exit. The router should be `Arc<CascadeRouter>` if it needs to be shared across async tasks; otherwise a plain reference suffices for single-dispatch `roko run`. Match the pattern at `commands/plan.rs:322-323` which already loads the router.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] `cargo run -p roko-cli -- run "echo hello"` completes successfully
- [ ] After running, `.roko/learn/cascade-router.json` exists and has been updated (check `mtime`)
- [ ] `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/run.rs` shows both calls
- [ ] Model selection log line shows correct `source` field (may be `CascadeRouter` or fall through to `ProjectDefault`)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After running, `.roko/learn/cascade-router.json` exists and has been updated (check `mtime`)
- `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/run.rs` shows both calls
- Model selection log line shows correct `source` field (may be `CascadeRouter` or fall through to `ProjectDefault`)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
