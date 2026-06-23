# STAB_35: Unify model selection paths (auth_detect vs ServiceFactory)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-35`](../ISSUE-TRACKER.md#stab-35)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.35
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

9+ dispatch paths have inconsistent model selection. `auth_detect.rs` scans env vars in
fixed priority, ignoring config. `ServiceFactory::resolve_model()` resolves correctly.
Setting `default_model = "glm51"` in roko.toml has no effect via `roko run`.

## Exact Changes

1. Make all entry points use `ServiceFactory::build()` (or its resolve_model function).
2. Demote `auth_detect.rs` to credential discovery only (not model selection).
3. Model resolution priority: CLI override > task config > role config > roko.toml
   `default_model` > env var heuristic.
4. Remove model-selection logic from `auth_detect.rs`.
5. Test all 9 entry points with `default_model` set.

## Design Guidance

Create a single `resolve_model_for_dispatch(overrides, config) -> ModelSelection` function
that all entry points call. Keep env var scanning as the lowest-priority fallback.

## Write Scope

- `crates/roko-cli/src/auth_detect.rs`
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/model_selection.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Set `default_model = "cerebras-70b"` in roko.toml
- [ ] `roko run`, `roko plan run`, `roko chat` all use Cerebras
- [ ] CLI `--model` override still works

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Set `default_model = "cerebras-70b"` in roko.toml
- `roko run`, `roko plan run`, `roko chat` all use Cerebras
- CLI `--model` override still works
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
