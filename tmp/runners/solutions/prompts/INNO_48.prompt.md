# INNO_48: Implement tool-output sanitization

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-48`](../ISSUE-TRACKER.md#inno-48)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.48
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_48 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Tool outputs at `crates/roko-agent/src/tool_loop/result_msg.rs` are included
in agent context without truncation or filtering. `result_msg.rs` has
`initial_messages()`, `initial_messages_with_few_shot()`, and `append_results()`.

Research: Augment Code SWE-bench analysis -- 30-40% of wasted tokens come from
verbose tool outputs. MCPTox: tool-poisoning 84.2% with auto-approve.

## Exact Changes

1. Define `ToolOutputSanitizer` with configurable max output size
   (default: 4096 tokens).
2. In `append_results()`, before appending tool output:
   - Truncate to max size with "... (truncated, {N} tokens omitted)" suffix.
   - Strip ANSI escape codes.
   - Filter known injection patterns (tool calls embedded in output).
   - Validate UTF-8 encoding.
3. Log sanitization events when content is modified.
4. Make max size configurable per tool (some tools like `read_file` need
   larger output than `bash`).

## Write Scope

- `crates/roko-agent/src/tool_loop/result_msg.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A `bash` tool output of 10K tokens is truncated to 4K with a truncation notice
- [ ] ANSI codes are stripped from all tool outputs
- [ ] A tool output containing a fake tool call has it sanitized
- [ ] Sanitization is visible in verbose logging

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_48 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A `bash` tool output of 10K tokens is truncated to 4K with a truncation notice
- ANSI codes are stripped from all tool outputs
- A tool output containing a fake tool call has it sanitized
- Sanitization is visible in verbose logging
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_48 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
