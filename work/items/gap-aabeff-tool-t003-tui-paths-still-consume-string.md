+++
id = "gap-aabeff"
kind = "gap"
title = "TUI paths still consume string/tail projections that lose semantics (TUI adoption unproven)"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-14
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/tui/app/channels.rs:663", "crates/roko-cli/src/tui/state/signals.rs::push_agent_output_record", "crates/roko-cli/src/tui/views/agents_view.rs::render_output_body", "crates/roko-cli/src/tui/views/dashboard_view.rs::render_output_panel", "crates/roko-cli/src/tui/widgets/stream_output.rs::parse_stream_line", "crates/roko-cli/src/transcript/convert.rs::blocks_from_records", "crates/roko-core/src/transcript_store.rs::TranscriptStore", "crates/roko-cli/tests/tui_terminal_size.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'blocks_from_records' crates/roko-cli/src/tui && ! grep -q '\\[tool_call\\] {text}' crates/roko-cli/src/tui/app/channels.rs && grep -rqw 'fn tui_agent_output_renders_transcript_blocks_at_three_sizes' crates/roko-cli/ && cargo test -p roko-cli tui_agent_output_renders_transcript_blocks_at_three_sizes"
+++

## Problem

The dashboard (`roko dashboard`, and the TUI that `roko plan run --tui` opens) renders agent output from flat
strings, not from the shared transcript model. Tool calls, results, reasoning and text arrive as typed stream
chunks, get flattened into strings such as `"[tool_call] {json}"` or `"\x1eroko.stream.v1 {...}"`, are kept as a
200-entry string tail, and are re-parsed at render time. Start/result pairing, statuses and call IDs survive only
as far as the re-parser can recover them.

The shared renderer built for this (tool-audit findings T002/T003/T018: `transcript::TranscriptBlock` and
`blocks_from_records` in `roko-cli`, the bounded `TranscriptStore` in `roko-core`) is not used by any TUI code.
The audit's release gate for this work is also still open: TUI snapshots with agent output at 80x24, 120x40 and
200x60 (including resize and follow-tail) were never produced, and live-vs-replay equivalence is unproven.

Expected: Dashboard and Agents output panes render from one typed transcript model (records -> blocks -> lines),
with no string round-trip, and tests prove the rendering at the three sizes.

## Why it matters

- Goal `visibility` (live visibility in serve, dashboard and portal): the TUI is the operator's live view of a plan
  run; lossy strings show orphaned tool starts, mis-paired results and raw JSON.
- Three parallel output models now exist (see Current state); each new feature has to be built three times.
- Related: `gap-836ae9` (live vs replayed tool streams lack proven store/order parity, audit T007) owns the
  live-vs-replay proof; `gap-633184` (control events vs text under backpressure, T014) owns drop behaviour on
  bounded paths.

## Where

- `crates/roko-cli/src/tui/app/channels.rs` (~line 640-690): stream ingestion. For `StreamChunk::ToolCall` it calls
  `push_agent_output_record(.., OutputRecordKind::ToolCall, String::new(), tool_id, tool_name)` and also
  `push_agent_chunk(.., format!("[tool_call] {text}"))`; same double write for reasoning, usage and errors.
- `crates/roko-cli/src/tui/state/signals.rs`: `push_agent_chunk` (string ring, `MAX_AGENT_STREAM_CHUNKS`) and
  `push_agent_output_record` (typed history, #367).
- `crates/roko-cli/src/tui/state/mod.rs`: `OutputRecordKind`, `AgentOutputRecord` (TUI-local typed model),
  `AGENT_OUTPUT_PAGE_SIZE`.
- `crates/roko-cli/src/tui/views/agents_view.rs::render_output_body` (~line 840-870): renders from `agent_output_history` via
  `stream_output::render_output_records_styled` when present, else falls back to string lines
  (`render_output_lines_styled` or `render_agent_output_lines`).
- `crates/roko-cli/src/tui/views/dashboard_view.rs::render_output_panel` (~line 935-950): renders only from collected string lines via
  `render_output_lines_styled`.
- `crates/roko-cli/src/tui/widgets/stream_output.rs`: `parse_stream_line` (re-parses `\x1eroko.stream.v1 ` JSON
  strings), `render_output_lines_styled`, `render_output_records_styled`.
- `crates/roko-cli/src/transcript/` (`block.rs`, `convert.rs::blocks_from_records`, `projection.rs`, `fold.rs`,
  `tests.rs`): the shared block model and converter; `pub mod transcript` in `lib.rs`, used by nothing else.
- `crates/roko-core/src/transcript_store.rs::TranscriptStore` and `crates/roko-core/src/tool/transcript.rs`
  (`TranscriptRecord`): bounded store; only `roko-fs/src/classified_writer.rs` and tests use it. No runtime code
  constructs a `TranscriptRecord`.
- `crates/roko-agent/src/live_output.rs`: `LiveAgentEvent` (typed live events, added in `c41e78c7a`).
- `crates/roko-cli/tests/tui_terminal_size.rs`: renders a default `App` at the three sizes and checks overflow and
  header only; no agent output content, no resize or follow-tail.

## Current state

- `c41e78c7a` (2026-09-29) made live tool steps render as single lines from typed `roko.stream.v1` records (tests
  `renders_live_tool_steps` in `widgets/stream_output.rs`, `settle_screened_transcript_keeps_tool_steps` in
  `tui/state/tests.rs`). The records still travel as strings and are re-parsed.
- The Agents view partly uses typed records (`AgentOutputRecord`, #367); the Dashboard view does not.
- `output_lines` / `last_output_line` string tails appear about 90 times under `crates/roko-cli/src/tui/`
  (heaviest: `widgets/stream_output.rs`, `state/snapshot.rs`, `views/agents_view.rs`).
- No TUI file references `TranscriptStore`, `TranscriptBlock` or `blocks_from_records`.
- The audit register (tool audit 2026-09-21) lists T003 as "On main" only because `TranscriptStore` exists; its
  packet F ("shared virtualized transcript in Dashboard and Agents") was never adopted by the TUI.

## Plan

Design choice — which model the TUI standardises on:

- Option A (recommended): adopt the shared model. Convert at ingestion: each typed chunk
  (`StreamChunk::*`, `DashboardEvent::AgentOutput`, `roko.stream.v1` record) becomes a
  `roko_core::TranscriptRecord` appended to a per-agent `TranscriptStore`. Panes render
  `blocks_from_records(store page)` through one block renderer. Retire `AgentOutputRecord` and the string ring for
  agent output. One model shared with inline chat and replay; larger change.
- Option B: declare the TUI-local `AgentOutputRecord` canonical, move the Dashboard view onto it, and delete the
  unused `transcript/` module. Smaller, but leaves the TUI with its own model and defeats T018 (one renderer).

Steps (Option A):

1. Add an adapter `fn stream_chunk_to_transcript_record(..)` (in `tui/app/channels.rs` or a new
   `tui/state/transcript.rs`) mapping text, reasoning, tool start/result (with call ID), usage and error.
2. Give `TuiState` a per-agent `TranscriptStore` (bounded; capacity replaces `MAX_AGENT_STREAM_CHUNKS`) and write
   to it from every ingestion path; stop pushing `"[tool_call] {json}"` / `"[usage] ..."` strings.
3. Add `stream_output::render_transcript_blocks(&[TranscriptBlock], theme, &RenderOptions)` that keeps today's
   visual semantics (tool start/result glyphs, folding, search highlight). Reuse `transcript::projection` for fold
   state.
4. Switch `agents_view.rs` and `dashboard_view.rs` to the block renderer; keep `parse_stream_line` only for
   legacy log files read from disk, if any remain.
5. Update `state/snapshot.rs` and other `output_lines`/`last_output_line` users to derive summaries from blocks.
6. Tests: extend `tests/tui_terminal_size.rs` (or add a unit test) with an `App` holding a transcript that has
   paired and orphaned tool calls, rendered at 80x24, 120x40, 200x60, after a resize and with follow-tail on;
   assert no overflow and that each tool call appears once with its status.

## Done when

- `grep -rn 'blocks_from_records' crates/roko-cli/src/tui` shows the Dashboard and Agents panes rendering from it.
- `channels.rs` no longer pushes `"[tool_call] {json}"` strings.
- A test renders agent output (with tool calls) at the three sizes, after resize and in follow-tail, and passes.
- Verify (proposed; the current `[[verify]]` passes on any mention of `TranscriptBlock` under `tui/`, even a
  comment):

  ```
  grep -rq 'blocks_from_records' crates/roko-cli/src/tui && ! grep -q '\[tool_call\] {text}' crates/roko-cli/src/tui/app/channels.rs && grep -rqw 'fn tui_agent_output_renders_transcript_blocks_at_three_sizes' crates/roko-cli/ && cargo test -p roko-cli tui_agent_output_renders_transcript_blocks_at_three_sizes
  ```

## Notes

- Keep the secret-scrubbing behaviour of `live_output.rs` (tool steps expose only identity and target); the
  adapter must not reintroduce raw tool arguments into the pane.
- The live-vs-replay equivalence proof belongs to `gap-836ae9`; do not duplicate it here, but design the store so
  that test can feed the same records to both paths.
- Large, UI-wide change: conflicts with any concurrent work in `tui/views/agents_view.rs`,
  `tui/views/dashboard_view.rs`, `tui/widgets/stream_output.rs` or `tui/state/`. Not safe to run in parallel with
  other TUI items.
- Portal/serve use their own feeds (`LiveAgentEvent` over SSE/WS); this item is TUI only.

## Original notes

Register marks On main via bounded TranscriptStore in roko-core, but Packet F release gate is open: TUI terminal-size snapshots (80x24/120x40/200x60) never generated and live-vs-replay equivalence unverified. Related T002/T018 (start/result pairing, shared TranscriptBlock renderer).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check whether TUI Dashboard/Agents output panes render from TranscriptStore/TranscriptBlock (blocks_from_records) rather than output_lines/last_output_line string tails or '[tool_call] ...' strings.

Verified 2026-09-28: No file under crates/roko-cli/src/tui references TranscriptStore, TranscriptBlock or blocks_from_records, while output_lines/last_output_line string tails appear 91 times under tui/.

Re-verified 2026-09-29: still open. Since the last check the TUI renders live tool steps as single lines from typed roko.stream.v1 records (c41e78c7a; tests renders_live_tool_steps in widgets/stream_output.rs and settle_screened_transcript_keeps_tool_steps in tui/state/tests.rs). Those records are still carried as output_lines strings and re-parsed. The shared TranscriptBlock renderer (transcript/convert.rs::blocks_from_records) is still unused by the TUI, and tui/app/channels.rs:663 still pushes '[tool_call] {json}' chunks. Live-vs-replay equivalence is still unverified. tests/tui_terminal_size.rs checks only overflow of a default App at 80x24/120x40/200x60.
