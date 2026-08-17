# DISP_22: Remove Hardcoded Model String from auth_detect.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-22`](../ISSUE-TRACKER.md#disp-22)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.22
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`auth_detect.rs` at line 42 hardcodes `"claude-sonnet-4-6"`:
```rust
if let Ok(key) = std::env::var("ZAI_API_KEY") { ... }
if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") { ... }
if let Ok(key) = std::env::var("OPENAI_API_KEY") { ... }
```

This module detects available backends by probing env vars and CLI availability. It has a `Command::new("claude")` at line 102 for version detection.

## Exact Changes

1. Replace hardcoded model string with `config.agent.default_model` where config is available
2. If config is not available (auth_detect runs before config is loaded), use a constant from `model_selection.rs` rather than an inline string
3. The `Command::new("claude")` for version detection is acceptable (it's checking if the CLI is installed, not dispatching a model call)
4. Consolidate env var probing: instead of checking 3 env vars independently, check `config.effective_providers()` for providers with configured credentials

## Design Guidance

Auth detection is a bootstrap step -- it runs before the full provider system is initialized. Some hardcoding is acceptable here, but it should use shared constants, not inline strings. The env var checks are also acceptable as a pre-flight, but they should not determine model selection (that's `model_selection.rs`'s job).

## Write Scope

- `crates/roko-cli/src/auth_detect.rs`

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

- [ ] `grep -n '"claude-sonnet-4-6"' crates/roko-cli/src/auth_detect.rs` returns zero results
- [ ] Auth detection still correctly identifies available backends

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n '"claude-sonnet-4-6"' crates/roko-cli/src/auth_detect.rs` returns zero results
- Auth detection still correctly identifies available backends
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
