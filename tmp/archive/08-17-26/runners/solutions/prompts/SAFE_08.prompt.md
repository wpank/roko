# SAFE_08: Wire `RateLimiter` Into Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-08`](../ISSUE-TRACKER.md#safe-08)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.8
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `RateLimiter` has a complete sliding-window implementation with
`check_and_record()` method. It is never instantiated at runtime. Wire it into
the dispatch path to prevent runaway agents.

## Exact Changes

1. Instantiate `RateLimiter::new(RateLimitPolicy { max_calls_per_window: 120, window_duration: 60s })`
   in the runner event loop initialization
2. Before each agent dispatch, call `rate_limiter.check_and_record(&key)`
   with `key = RateLimitKey { role, tool: "agent_call" }`
3. If rate limited, delay (do not drop) the dispatch and log a warning
4. Make limits configurable via `[safety.rate_limits]` in roko.toml:
   ```toml
   [safety.rate_limits]
   per_tool = 120        # max calls per tool per minute
   per_role = 60         # max agent calls per role per minute
   global = 300          # max total agent calls per minute
   ```
5. Rate limit violations should appear in efficiency events

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/dispatch_v2.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] An agent making > 60 calls per minute is throttled (delayed, not terminated)
- [ ] Rate limit config is loaded from roko.toml
- [ ] `roko learn efficiency` shows rate-limited events
- [ ] Default limits are reasonable for normal execution (no false positives)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An agent making > 60 calls per minute is throttled (delayed, not terminated)
- Rate limit config is loaded from roko.toml
- `roko learn efficiency` shows rate-limited events
- Default limits are reasonable for normal execution (no false positives)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
