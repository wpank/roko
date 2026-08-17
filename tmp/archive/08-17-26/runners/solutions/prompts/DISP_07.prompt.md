# DISP_07: Extract Shared Truncation Utility

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-07`](../ISSUE-TRACKER.md#disp-07)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.7
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The 4096-byte truncation logic is inlined in 4 places:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs:125` (constant `TOOL_OUTPUT_TRUNCATE_AT`)
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs:186-191` (inline 4096)
3. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs:238-243` (inline 4096)
4. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs:660-665` (inline 4096)

All four implement the same pattern: check `len() > 4096`, walk backward to find `char_boundary`, append `"...[truncated]"`.

## Exact Changes

1. In `stream.rs`, make the existing `TOOL_OUTPUT_TRUNCATE_AT` constant `pub`
2. Add a `pub fn truncate_tool_output(content: &str, max_bytes: usize) -> String`:
   ```rust
   pub fn truncate_tool_output(content: &str, max_bytes: usize) -> String {
       if content.len() <= max_bytes {
           return content.to_string();
       }
       let mut end = max_bytes;
       while !content.is_char_boundary(end) && end > 0 {
           end -= 1;
       }
       format!("{}...[truncated]", &content[..end])
   }
   ```
3. Re-export from `crate::provider::claude_cli::stream` and from `crate::provider::claude_cli` module
4. Add tests: empty string, ASCII within limit, ASCII over limit, multi-byte UTF-8 boundary, exact boundary

## Design Guidance

The function should be a pure utility with no side effects. `max_bytes` parameter allows callers to use different limits if needed (though 4096 is the standard). Return `String` always -- the caller can decide to borrow if needed later.

## Write Scope

- `crates/roko-agent/src/provider/claude_cli/stream.rs`

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

- [ ] `truncate_tool_output` is publicly accessible from `roko_agent::provider::claude_cli::stream`
- [ ] Function handles multi-byte UTF-8 correctly (test with emoji or CJK characters)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `truncate_tool_output` is publicly accessible from `roko_agent::provider::claude_cli::stream`
- Function handles multi-byte UTF-8 correctly (test with emoji or CJK characters)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
