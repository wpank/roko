# DISP_17: Add thinking_tokens to UsageObservation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-17`](../ISSUE-TRACKER.md#disp-17)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.17
- Priority: **P2**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`UsageObservation` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/usage.rs:17` tracks `input_tokens`, `output_tokens`, `cache_creation_tokens`, `cache_read_tokens`, `cost_usd` but not thinking/reasoning tokens.

The CascadeRouter already tracks thinking tokens in its Gemini observations (`cascade/types.rs:437`). The Gemini native backend extracts them (`gemini/native.rs:333`). But the canonical `UsageObservation` type -- which is used by `ModelCallService`, episode logging, and all feedback paths -- does not carry this data.

## Exact Changes

1. Add `pub thinking_tokens: Option<u64>` field to `UsageObservation` (after `cache_read_tokens`)
2. Update the `From<Usage> for UsageObservation` implementation to set `thinking_tokens: None` (legacy `Usage` struct doesn't have it)
3. Update the `From<UsageObservation> for Usage` implementation to ignore `thinking_tokens` (legacy struct can't represent it)
4. Ensure `#[serde(default)]` on the new field for backward-compatible deserialization
5. Update the Gemini adapter to populate `thinking_tokens` from `GeminiMetadata.thinking_tokens`
6. Update the Anthropic API adapter to populate from response when available (Claude extended thinking returns reasoning token counts)
7. Update `CostTable` pricing to account for thinking tokens (often priced differently than output tokens)

## Design Guidance

`Option<u64>` matches the existing pattern for all fields. `None` means "this provider/model doesn't report thinking tokens" -- distinct from `Some(0)` meaning "thinking was enabled but produced zero tokens." This distinction matters for cost accounting: a model that doesn't support thinking should not have zero thinking cost attributed.

## Write Scope

- `crates/roko-agent/src/usage.rs`
- `crates/roko-core/src/chat_types.rs`

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

- [ ] Serialization/deserialization roundtrip preserves `thinking_tokens`
- [ ] Existing usage data without `thinking_tokens` deserializes correctly (defaults to `None`)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Serialization/deserialization roundtrip preserves `thinking_tokens`
- Existing usage data without `thinking_tokens` deserializes correctly (defaults to `None`)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
