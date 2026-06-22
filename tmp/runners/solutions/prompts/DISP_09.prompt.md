# DISP_09: Replace extract_clean_text with Typed Parsing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-09`](../ISSUE-TRACKER.md#disp-09)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.9
- Priority: **P1**
- Effort: 6 hours
- Depends on: `DISP_07` (source 3.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`extract_clean_text()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs:570-697` is a 127-line format guesser handling 10 different response shapes. It is called from:
- `chat.rs:206` -- chat REPL display
- `dispatch_direct.rs:170` -- legacy dispatch (feature-gated behind `legacy-orchestrate`)
- `agent_serve.rs:514` -- sidecar agent serving
- `chat_inline.rs:4268` -- inline chat

The canonical parser `parse_stream_line()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs:134` returns typed `AgentRuntimeEvent` variants. It is already used by:
- `chat_session.rs:1171` -- chat session
- `runner/agent_stream.rs:120` -- runner

## Exact Changes

1. For callers that receive Claude CLI stream-json output (JSONL lines), replace `extract_clean_text` with `parse_stream_line`:
   ```rust
   use roko_agent::provider::claude_cli::stream::parse_stream_line;
   let events: Vec<AgentRuntimeEvent> = raw.lines()
       .flat_map(|line| parse_stream_line(line))
       .collect();
   // Extract text from events
   let text = events.iter().filter_map(|e| match e {
       AgentRuntimeEvent::Text(t) => Some(t.as_str()),
       _ => None,
   }).collect::<Vec<_>>().join("");
   ```
2. For callers that receive single JSON objects (sidecar, API responses), keep a thin wrapper that checks for `result` or `content` fields -- but extract it from `extract_clean_text` into a smaller `extract_json_text(value: &serde_json::Value) -> Option<String>` function
3. Deprecate `extract_clean_text` with `#[deprecated]` annotation pointing to the typed alternatives
4. Update call sites one by one:
   - `chat.rs:206` -- use `parse_stream_line` for JSONL, `extract_json_text` for single objects
   - `agent_serve.rs:514` -- use `extract_json_text` (sidecar returns single JSON)
   - `chat_inline.rs:4268` -- use `parse_stream_line`
5. Keep existing tests passing by adding equivalent tests for the new functions

## Design Guidance

Do NOT delete `extract_clean_text` in this task -- deprecate it. The `dispatch_direct.rs` caller is behind a feature gate and will be removed when the legacy flag is dropped. The goal is to stop adding new callers and migrate existing ones. The replacement should use typed `AgentRuntimeEvent` variants, not raw JSON guessing.

## Write Scope

- `crates/roko-cli/src/chat.rs`
- `crates/roko-cli/src/dispatch_direct.rs`
- `crates/roko-cli/src/agent_serve.rs`
- `crates/roko-cli/src/chat_inline.rs`

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

- [ ] `extract_clean_text` has `#[deprecated]` annotation
- [ ] `grep -n 'extract_clean_text' crates/roko-cli/src/ -r | grep -v test | grep -v deprecated` shows only the deprecated definition and `dispatch_direct.rs` (feature-gated)
- [ ] All existing `extract_clean_text` tests have equivalents for the new functions

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `extract_clean_text` has `#[deprecated]` annotation
- `grep -n 'extract_clean_text' crates/roko-cli/src/ -r | grep -v test | grep -v deprecated` shows only the deprecated definition and `dispatch_direct.rs` (feature-gated)
- All existing `extract_clean_text` tests have equivalents for the new functions
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
