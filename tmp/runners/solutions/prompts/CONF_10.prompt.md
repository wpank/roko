# CONF_10: Route All CLI Entry Points Through `ServiceFactory::build()`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-10`](../ISSUE-TRACKER.md#conf-10)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.10
- Priority: **P1**
- Effort: Large
- Depends on: `CONF_01` (source 16.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Nine dispatch paths exist with inconsistent model selection. Only paths through
`ServiceFactory` get full feedback recording, cost tracking, cascade routing, and
knowledge injection. `ServiceFactory` is used in `run.rs` but callers in
`chat_session.rs`, `chat_inline.rs`, `commands/prd.rs`, and `dispatch_v2.rs` may
construct agents directly.

Hardcoded model strings like `"claude-sonnet-4-6"` exist in production code paths
(not just tests): `plan_generate.rs:131`, `explain.rs:407` (uses `"gpt-4o"`).

## Exact Changes

1. Audit each entry point for direct `create_agent_for_model()` calls that bypass
   `ServiceFactory::build()`. Replace with ServiceFactory.
2. Remove hardcoded model strings in non-test code. Replace with
   `config.agent.default_model` resolution or tier-based lookup.
3. Ensure `FeedbackService` is constructed for every path (fixes the "chat records
   zero learning signals" issue).
4. Verify `dispatch_direct.rs` is unreachable from default builds (it is behind
   `legacy-orchestrate` which is currently default -- so it IS reachable).

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/chat_inline.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/plan_generate.rs`
- `crates/roko-cli/src/explain.rs`

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

- [ ] `grep -rn '"claude-sonnet-4-6"' crates/roko-cli/src/ --include='*.rs' | grep -v test | grep -v doc | grep -v comment`
- [ ] All dispatch paths log the model source via `EffectiveModelSelection`.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn '"claude-sonnet-4-6"' crates/roko-cli/src/ --include='*.rs' | grep -v test | grep -v doc | grep -v comment`
- All dispatch paths log the model source via `EffectiveModelSelection`.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
