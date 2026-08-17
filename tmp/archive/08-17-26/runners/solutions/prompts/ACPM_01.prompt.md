# ACPM_01: Consolidate estimate_tokens into roko-core

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-01`](../ISSUE-TRACKER.md#acpm-01)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.1
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`estimate_tokens` is reimplemented 6 times across the workspace (AP-DUPETOKEN). The canonical version is `Budget::estimate_tokens(bytes: usize) -> usize` at `crates/roko-core/src/query.rs:167` which divides by 4 and rounds up. Other versions do `text.len() / 4`, `text.chars().count().div_ceil(4)`, or `(text.len() as u64 / 4).max(1)`. These are all trying to do the same thing with minor variations.

## Exact Changes

1. In `crates/roko-core/src/query.rs`, add a standalone public function alongside the existing `Budget::estimate_tokens`:
   ```rust
   /// Estimate token count from text using 4-chars-per-token heuristic.
   pub const fn estimate_tokens_for_text(text: &str) -> usize {
       text.len().div_ceil(4)
   }
   ```
2. Re-export from `crates/roko-core/src/lib.rs`: `pub use query::estimate_tokens_for_text;`
3. Replace each of the 5 other `estimate_tokens` implementations to call `roko_core::estimate_tokens_for_text()`, adjusting signatures where needed (e.g., `roko-cli/src/bench.rs` returns `u64` -- cast the result).
4. For `roko-compose/src/compaction.rs:239` which operates on `&[ChatMessage]`, keep the wrapper that serializes and then calls the core function.

## Design Guidance

Use `text.len()` (bytes) not `text.chars().count()` since the 4-chars heuristic is calibrated on byte length. This matches the canonical `Budget::estimate_tokens` which takes `bytes: usize`. The function must be `const fn` for compile-time contexts.

## Write Scope

- `crates/roko-core/src/query.rs`
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/compaction.rs`
- `crates/roko-index/src/workspace.rs`
- `crates/roko-cli/src/bench.rs`
- `crates/roko-cli/src/dispatch/prompt_builder.rs`

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

- [ ] `grep -rn 'fn estimate_tokens' crates/ --include='*.rs' | grep -v target/ | grep -v test` shows at most 2 definitions (the core one and the ChatMessage wrapper)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'fn estimate_tokens' crates/ --include='*.rs' | grep -v target/ | grep -v test` shows at most 2 definitions (the core one and the ChatMessage wrapper)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
