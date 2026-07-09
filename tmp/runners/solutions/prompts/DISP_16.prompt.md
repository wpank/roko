# DISP_16: Add Retry Logic to ModelCallService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-16`](../ISSUE-TRACKER.md#disp-16)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.16
- Priority: **P2**
- Effort: 4 hours
- Depends on: `DISP_14` (source 3.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RetryPolicy` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/retry.rs:45` already exists with full-jitter exponential backoff, retryable error classification, and `retry_after_ms` support. It is used by `ToolLoop` at `tool_loop/mod.rs:195` for multi-turn retries.

`ModelCallService::call()` does not retry on transient failures. When the agent returns an error, it immediately fails or falls back to a different model. For transient errors (rate limits, 500s, timeouts), retrying the same model with backoff is more efficient than failover.

## Exact Changes

1. Add `retry_policy: RetryPolicy` field to `ModelCallService` with `RetryPolicy::default()` (3 attempts, 1s base, 60s max)
2. Add `with_retry_policy(policy: RetryPolicy)` builder
3. In `call()`, wrap the dispatch in a retry loop:
   ```rust
   for attempt in 0..self.retry_policy.max_attempts {
       match self.dispatch_once(&model, &request).await {
           Ok(response) => return Ok(response),
           Err(e) if self.retry_policy.should_retry_mcs(&e, attempt) => {
               let delay = self.retry_policy.delay_for_attempt(attempt);
               tokio::time::sleep(Duration::from_millis(delay)).await;
               continue;
           }
           Err(e) => return Err(e),
       }
   }
   ```
4. Add `should_retry_mcs()` that classifies `RokoError` into retryable/non-retryable (similar to `should_retry()` for `ProviderError`)
5. Integrate with health tracker: record failure on final retry exhaustion, not on each attempt

## Design Guidance

Retry before failover. Only fail over to a different model after all retries are exhausted. This prevents unnecessary model switches on transient network issues. The `retry_after_ms` from rate-limit headers should be honored (use `delay_with_retry_after`).

Rate limit retries should respect the provider's `retry_after_ms`. Server errors get exponential backoff. Timeouts get one retry with the same timeout. Auth failures, content policy, context overflow, and model-not-found are never retried.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`

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

- [ ] Unit test: transient error followed by success completes without failover
- [ ] Unit test: 3 consecutive failures triggers final error (not infinite loop)
- [ ] Unit test: rate limit with `retry_after_ms=2000` waits approximately 2 seconds

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: transient error followed by success completes without failover
- Unit test: 3 consecutive failures triggers final error (not infinite loop)
- Unit test: rate limit with `retry_after_ms=2000` waits approximately 2 seconds
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
