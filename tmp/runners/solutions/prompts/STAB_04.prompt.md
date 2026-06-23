# STAB_04: Add secret scrubbing to CLI Gist share path

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-04`](../ISSUE-TRACKER.md#stab-04)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.04
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`share.rs` already has `scrub_share_text()` (line 49) and `scrub_long_secret_like_strings()`
(line 61). It already applies scrubbing to the prompt and output text before creating the
share artifact (lines 93-103). The function `scrub_share_text` uses `LogScrubber` and
secondary regex patterns for long hex/base64 strings.

**Status re-assessment**: The CLI share path ALREADY scrubs secrets. Tests at lines 302-328
verify this behavior (`scrub_share_text_redacts_api_key_in_prompt`, etc.).

The audit finding may be stale or the scrubbing may have been added after the audit.
Verify that the scrubbing covers all paths (Gist upload, local file write, stdout output).

## Exact Changes

1. Verify that `share_run()` (or equivalent) calls `scrub_share_text()` on ALL text content
   before writing to disk or uploading to Gist.
2. Verify Gist upload path (if separate from local file write) also scrubs.
3. Add a test: transcript containing `ANTHROPIC_API_KEY=sk-ant-test123` produces Gist
   content with `[REDACTED]`.
4. If any path is found unscrubbed, add `scrub_share_text()` call.

## Design Guidance

The scrubbing is already well-implemented. The key improvement is ensuring coverage of all
output paths, not adding new scrubbing logic. Consider also scrubbing the `tool_calls` field
in the report if tools received secrets as arguments.

## Write Scope

- `crates/roko-cli/src/share.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All share output paths (local file, Gist upload) apply `scrub_share_text()`
- [ ] Tests verify redaction of API keys, long hex strings, long base64 strings

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All share output paths (local file, Gist upload) apply `scrub_share_text()`
- Tests verify redaction of API keys, long hex strings, long base64 strings
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
