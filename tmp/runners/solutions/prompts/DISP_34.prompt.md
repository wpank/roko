# DISP_34: Route dispatch_direct.rs Through ModelCallService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-34`](../ISSUE-TRACKER.md#disp-34)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.34
- Priority: **P2**
- Effort: 4 hours
- Depends on: `DISP_10` (source 3.10), `DISP_14` (source 3.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`dispatch_direct.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs` is already gated behind `#[cfg(feature = "legacy-orchestrate")]`. It has three dispatch functions:
- `dispatch_claude_cli()` -- bare Claude CLI subprocess
- `dispatch_anthropic_api()` -- direct HTTP to Anthropic
- `dispatch_openai_compat()` -- direct HTTP to OpenAI

All three bypass the provider system, feedback, health tracking, and budget enforcement.

## Exact Changes

1. Check if the `legacy-orchestrate` feature is enabled in any production build configuration
2. If not enabled in production, this task is deprioritized -- the gated code is already dead
3. If enabled, replace each function's body with a delegation to `ModelCallService`:
   ```rust
   pub async fn dispatch_claude_cli(prompt: &str, config: &RokoConfig) -> Result<DispatchResult> {
       dispatch_via_model_call_service(prompt).await
   }
   ```
4. Remove the manual `Command::new("claude")` and `reqwest::Client` usage
5. Remove `extract_clean_text` import (uses `chat::extract_clean_text`)

## Design Guidance

If the legacy feature gate is not enabled anywhere, leave this as-is -- it will be cleaned up when the feature gate is removed entirely. Do not spend effort on dead code migration unless the feature is actively used.

## Write Scope

- `crates/roko-cli/src/dispatch_direct.rs`

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

- [ ] If feature is enabled, all dispatch goes through `ModelCallService`
- [ ] If feature is disabled, no behavior change

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- If feature is enabled, all dispatch goes through `ModelCallService`
- If feature is disabled, no behavior change
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
