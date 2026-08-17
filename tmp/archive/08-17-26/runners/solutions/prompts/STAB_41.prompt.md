# STAB_41: Add retry logic for transient provider failures

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-41`](../ISSUE-TRACKER.md#stab-41)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.41
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_41 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ModelCallService` has `fallback_models` for model-level failover but no retry logic for
transient errors (500, timeout, rate limit with retry-after).

## Exact Changes

1. Add configurable retry policy.
2. Retry on rate limit (honor `retry_after_ms`).
3. Retry on server error (exponential backoff: 1s, 2s, 4s).
4. Retry on timeout (once with 1.5x timeout).
5. Never retry on auth failure, model not found, context overflow.
6. Max retries: configurable, default 2.
7. After retries exhausted, fall through to fallback models.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`

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

- [ ] Transient 500 followed by 200 succeeds without model switch
- [ ] Auth failure does not retry

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_41 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Transient 500 followed by 200 succeeds without model switch
- Auth failure does not retry
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_41 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
