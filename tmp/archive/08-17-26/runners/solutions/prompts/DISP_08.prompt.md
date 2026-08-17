# DISP_08: Replace Inline Truncation in translate/mod.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-08`](../ISSUE-TRACKER.md#disp-08)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.8
- Priority: **P1**
- Effort: 2 hours
- Depends on: `DISP_07` (source 3.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BackendResponse::extract_text()` at line 186-191 and `BackendResponse::extract_tool_outputs()` at line 238-243 both inline the truncation logic for tool output content. Both operate on `serde_json::Value` event streams from `StreamJson` variant.

The `extract_text()` method formats tool output as `"\n[{tool_name}]\n{content}\n"` with truncation. The `extract_tool_outputs()` method returns `Vec<(Option<String>, String)>` with truncation.

## Exact Changes

1. Add import: `use crate::provider::claude_cli::stream::{truncate_tool_output, TOOL_OUTPUT_TRUNCATE_AT};`
2. In `extract_text()` (line 186-192), replace the inline block:
   ```rust
   // Before:
   if content.len() > 4096 {
       let mut end = 4096;
       while !content.is_char_boundary(end) { end -= 1; }
       buf.push_str(&content[..end]);
       buf.push_str("...[truncated]\n");
   } else {
       buf.push_str(content);
       buf.push('\n');
   }
   // After:
   let display = truncate_tool_output(content, TOOL_OUTPUT_TRUNCATE_AT);
   buf.push_str(&display);
   buf.push('\n');
   ```
3. In `extract_tool_outputs()` (line 238-246), replace the inline block:
   ```rust
   // Before:
   let truncated = if content.len() > 4096 { ... } else { content.to_string() };
   // After:
   let truncated = truncate_tool_output(content, TOOL_OUTPUT_TRUNCATE_AT);
   ```
4. Run existing tests to verify no behavior change

## Design Guidance

This is a mechanical replacement. The output format should be identical -- verify by comparing test assertions before and after. The `truncate_tool_output` function appends `"...[truncated]"` which matches the existing inline format.

## Write Scope

- `crates/roko-agent/src/translate/mod.rs`

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

- [ ] `grep -n '4096' crates/roko-agent/src/translate/mod.rs` returns zero results
- [ ] `grep -n 'truncate_tool_output' crates/roko-agent/src/translate/mod.rs` shows 2 call sites

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n '4096' crates/roko-agent/src/translate/mod.rs` returns zero results
- `grep -n 'truncate_tool_output' crates/roko-agent/src/translate/mod.rs` shows 2 call sites
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
