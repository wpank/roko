# SAFE_09: Wire `ScrubPolicy` Into Prompt Assembly and Episode Logging

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-09`](../ISSUE-TRACKER.md#safe-09)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.9
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `ScrubPolicy` has 10+ secret patterns (Anthropic keys, OpenAI keys,
AWS keys, GitHub tokens, JWTs, private keys, .env values). It is used in the
HTTP share path (`shared_runs.rs` calls `LogScrubber`) but NOT in:
- Prompt assembly (secrets in repo files could leak into prompts)
- Episode logging (agent output containing secrets persists to `episodes.jsonl`)
- CLI share/gist path

## Exact Changes

1. Import `scrub_secrets()` from `roko_agent::safety::scrub`
2. In prompt assembly, scan the assembled system prompt for secret patterns
   before sending to the LLM. If found, redact and log a warning
3. In episode recording, scan episode content before writing to
   `.roko/episodes.jsonl`. Redact any detected secrets
4. In the CLI `--share` gist path, apply `scrub_secrets()` before upload
   (this fixes the LOW security finding from the audit)
5. Add custom scrub patterns via `[safety.scrub.patterns]` in roko.toml

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] A prompt containing `ANTHROPIC_API_KEY=sk-ant-...` has the key redacted
- [ ] Episode logs in `.roko/episodes.jsonl` never contain raw API keys
- [ ] `roko run --share` with secrets in output produces a Gist with `[REDACTED]`
- [ ] Custom patterns from config are applied alongside defaults

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A prompt containing `ANTHROPIC_API_KEY=sk-ant-...` has the key redacted
- Episode logs in `.roko/episodes.jsonl` never contain raw API keys
- `roko run --share` with secrets in output produces a Gist with `[REDACTED]`
- Custom patterns from config are applied alongside defaults
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
