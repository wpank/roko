# DISP_30: Implement Speculative Decoding Pattern

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-30`](../ISSUE-TRACKER.md#disp-30)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.30
- Priority: **P3**
- Effort: 6 hours
- Depends on: `DISP_06` (source 3.6), `DISP_14` (source 3.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

For interactive paths, dispatching to a fast model (Haiku/Flash) while simultaneously starting a slower premium model can reduce perceived latency. If the fast model's output passes a quality check, cancel the slow model and return immediately.

The `ModelCallService` already has `fallback_models` and cost prediction. It needs a new `call_speculative()` method that races a fast model against a premium model.

## Exact Changes

1. Add `pub async fn call_speculative(&self, req: ModelCallRequest) -> Result<ModelCallResponse>`:
   ```rust
   let fast_model = self.fastest_viable_model(&req);
   let premium_model = self.resolve_model(&req);

   if fast_model == premium_model {
       return self.call(req).await;  // No speculative benefit
   }

   let (fast_result, premium_handle) = tokio::join!(
       self.call_model(&req, &fast_model),
       tokio::spawn(self.call_model(&req, &premium_model)),
   );
   ```
2. Add quality check: if fast result is successful and the response length > threshold (suggesting a complete answer), use it and cancel premium
3. If fast result fails or is too short, await premium
4. Track both calls in cost accounting (fast model cost is wasted if premium is used)
5. Add `with_speculative_threshold(min_quality_score: f64)` builder

## Design Guidance

Speculative decoding is a latency optimization. It increases cost (~10-20% more from abandoned fast calls) but reduces p50 latency significantly for simple queries. Only enable for interactive paths (`roko chat`, `roko <prompt>`), not for plan execution where latency is less important.

The quality check should be simple: response length > 100 chars AND no error indicators. A more sophisticated check (confidence score, semantic similarity) can be added later.

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

- [ ] A test demonstrates speculative decoding: fast model returns quickly, premium is cancelled
- [ ] Cost accounting tracks both the fast model call and any premium model call
- [ ] `call_speculative` falls back to standard `call` when only one model is available

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A test demonstrates speculative decoding: fast model returns quickly, premium is cancelled
- Cost accounting tracks both the fast model call and any premium model call
- `call_speculative` falls back to standard `call` when only one model is available
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
