# CONF_03: Thread `workflow.template` Config Into Runner V2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-03`](../ISSUE-TRACKER.md#conf-03)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.3
- Priority: **P2**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`[workflow].template` (express/standard/full) is read by `WorkflowEngine` but runner v2
(`roko plan run`) uses its own hardcoded config, ignoring the template entirely.
`[workflow].max_iterations` is also ignored by runner v2 (which has its own `max_retries`
field at `runner/types.rs:1385`).

No `workflow.template` or `workflow_template` references exist in `crates/roko-cli/src/runner/`.

## Exact Changes

1. In `RunnerConfig::from_roko_config()`, read `workflow.template` and
   `workflow.max_iterations` from `RokoConfig`.
2. Map template to concrete settings: express (1 retry), standard (3 retries),
   full (5 retries + review gate).
3. Use `workflow.max_iterations` as the cap on `max_retries`, falling back to the
   template's default if not set.
4. Remove the hardcoded `max_retries` default in favor of the config-derived value.

## Write Scope

- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] Setting `workflow.template = "express"` in roko.toml reduces plan runner retries to 1.
- [ ] Setting `workflow.max_iterations = 5` overrides the template default.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Setting `workflow.template = "express"` in roko.toml reduces plan runner retries to 1.
- Setting `workflow.max_iterations = 5` overrides the template default.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
