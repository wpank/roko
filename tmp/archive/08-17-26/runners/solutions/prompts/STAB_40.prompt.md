# STAB_40: Fix singleton rate limiter across providers

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-40`](../ISSUE-TRACKER.md#stab-40)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.40
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_40 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`shared_rate_limiter()` at line 31 uses `OnceLock` to create a single global
`ProviderRateLimiter` with 60 RPM default. All `OpenAiCompatLlmBackend` instances share it.
A provider with 1000 RPM is throttled to 60 RPM.

## Exact Changes

1. Move rate limiter configuration to `ProviderConfig` with `rate_limit_rpm`.
2. Create per-provider rate limiter instances keyed by provider name.
3. `with_rate_limiter()` should be auto-wired from config.
4. Default to 60 RPM only when no config specified.

## Write Scope

- `crates/roko-agent/src/openai_compat_backend.rs`
- `crates/roko-agent/src/rate_limit.rs`

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

- [ ] Two providers with different RPM limits respect their individual limits
- [ ] No global singleton rate limiter

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_40 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Two providers with different RPM limits respect their individual limits
- No global singleton rate limiter
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_40 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
