# CONF_22: Consolidate Hardcoded Max-Token Values

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-22`](../ISSUE-TRACKER.md#conf-22)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.22
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Max tokens for the same model vary by entry point:
- `dispatch_direct.rs:196,280` -> 8192
- `chat_session.rs:555` -> 4096
- `batch_client.rs:293` -> 4096
- `lifecycle.rs:329` -> 4096
- `gateway.rs:1027` -> 1024
- `demo scenarios` -> 512

## Exact Changes

1. Add `max_output_tokens: Option<u32>` to `ModelProfile` in the config schema.
2. `resolve_model()` returns `max_output_tokens` as part of the resolved model info.
3. All dispatch paths use `model_profile.max_output_tokens.unwrap_or(4096)` instead
   of hardcoded values.
4. Demo/gateway can override to lower values but must do so explicitly via config.

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] Setting `max_output_tokens = 16384` on a model profile produces longer responses.
- [ ] No hardcoded max_tokens constants remain outside test code and demos.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Setting `max_output_tokens = 16384` on a model profile produces longer responses.
- No hardcoded max_tokens constants remain outside test code and demos.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
