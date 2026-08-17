# SAFE_14: Scrub Secrets on CLI `--share` (Gist) Path

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-14`](../ISSUE-TRACKER.md#safe-14)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.14
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The HTTP share path (`shared_runs.rs`) applies `scrub_run_transcript()`
before persisting. The CLI `--share` path creates a GitHub Gist with the raw
agent transcript without scrubbing. API keys, tokens, and secrets in agent
output are uploaded to GitHub as-is.

## Exact Changes

1. Find the CLI `--share` gist creation code
2. Import `LogScrubber` or `scrub_secrets()` from the safety module
3. Apply scrubbing to the transcript text before uploading
4. Apply the same `scrub_long_secret_like_strings()` regex patterns used in
   `shared_runs.rs` (hex strings > 32 chars, base64 strings > 32 chars)
5. Log a count of redacted secrets at `tracing::info!` level

## Write Scope

- `crates/roko-cli/src/`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko run --share` with `ANTHROPIC_API_KEY=sk-ant-...` in output produces
- [ ] Long hex and base64 strings are scrubbed
- [ ] The scrubbing matches the HTTP path's behavior

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --share` with `ANTHROPIC_API_KEY=sk-ant-...` in output produces
- Long hex and base64 strings are scrubbed
- The scrubbing matches the HTTP path's behavior
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
