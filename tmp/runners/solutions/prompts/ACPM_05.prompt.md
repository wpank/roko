# ACPM_05: Add Context Budget Session Config Option

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-05`](../ISSUE-TRACKER.md#acpm-05)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.5
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ACPM_04` (source 9.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AcpSession` in `crates/roko-acp/src/session.rs` currently exposes 9 config options built by `build_config_options()`. The session stores config values but has no `context_budget` field.

## Exact Changes

1. Add `pub context_budget: Option<usize>` field to `AcpSession` (default `None` = auto).
2. Add config option #10 to `build_config_options()`:
   - `id: "context_budget"`, `name: "Context Budget"`, `option_type: Select`
   - `options: ["auto", "small (32k)", "medium (64k)", "large (128k)", "max"]`
   - `default: "auto"`
3. In `update_config()`, map the string value to a token count:
   - `"auto"` -> `None` (let ContextManager use model's max_tokens / 2)
   - `"small"` -> `Some(32_000)`
   - `"medium"` -> `Some(64_000)`
   - `"large"` -> `Some(128_000)`
   - `"max"` -> `None` with a flag to use model's full max_tokens
4. Pass to `ContextManager` construction in bridge_events.

## Write Scope

- `crates/roko-acp/src/session.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP `session/new` response includes 10 config options (was 9)
- [ ] Setting context budget to "small" reduces knowledge items in prompt
- [ ] Setting to "max" includes all available context
- [ ] Existing session tests pass

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP `session/new` response includes 10 config options (was 9)
- Setting context budget to "small" reduces knowledge items in prompt
- Setting to "max" includes all available context
- Existing session tests pass
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
