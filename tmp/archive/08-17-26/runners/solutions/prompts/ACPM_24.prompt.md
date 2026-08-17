# ACPM_24: Record MCP Tool Calls in Episode Log

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-24`](../ISSUE-TRACKER.md#acpm-24)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.24
- Priority: **P1**
- Effort: 4 hours
- Depends on: `ACPM_23` (source 9.23)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

During `stream_events_to_editor()` in `bridge_events.rs`, `CognitiveEvent::ToolCallStart` and `CognitiveEvent::ToolCallComplete` events stream through. MCP tool calls are a subset of these (identified by tool name prefix or a known tool name list). The `Episode` struct in `roko-learn/src/episode_logger.rs` needs a field for tool call records.

## Exact Changes

1. Add `tool_calls: Vec<ToolCallRecord>` field to `Episode` in `crates/roko-learn/src/episode_logger.rs` (with `#[serde(default)]` for backward compat).
2. In `stream_events_to_editor()`, track MCP tool calls: when `ToolCallStart` fires, record `{ tool, start_time }`; when `ToolCallComplete` fires for that tool, compute latency and record result quality (heuristic: non-empty result content = potentially useful).
3. Use a `HashMap<String, Instant>` keyed by tool_call_id to track in-flight calls.
4. After dispatch completes, append tool records to the episode before persisting.
5. Feed each record to `ToolEffectivenessBandit.observe()`.

## Write Scope

- `crates/roko-acp/src/bridge_events.rs`
- `crates/roko-learn/src/episode_logger.rs`

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

- [ ] Episode entries in `.roko/episodes.jsonl` include `tool_calls` array when MCP tools were used
- [ ] Each tool call record has non-zero `latency_ms`
- [ ] Bandit file is updated after each dispatch with tool call observations

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Episode entries in `.roko/episodes.jsonl` include `tool_calls` array when MCP tools were used
- Each tool call record has non-zero `latency_ms`
- Bandit file is updated after each dispatch with tool call observations
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
