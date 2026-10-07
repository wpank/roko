+++
id = "bug-6ae24e"
kind = "bug"
title = "The inline chat joins text from separate assistant messages with no separator"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/chat"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "review of bug-2116ec's fix (85ba4cb4d), which covered only the screened transcript"
anchors = ["crates/roko-cli/src/chat_session.rs::send_turn_streaming_with_program", "crates/roko-cli/src/chat_session.rs::render_stream_event", "crates/roko-cli/src/chat_inline/event_loop.rs::run_main_loop", "crates/roko-agent/src/provider/claude_cli/stream.rs::parse_stream_line"]
links = { depends_on = [], blocks = [], related = ["bug-2116ec"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_turn_separates_assistant_messages' crates/roko-cli/src/ && cargo test -p roko-cli --lib streaming_turn_separates_assistant_messages"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:17Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:29Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The inline chat (`roko` with no subcommand on a terminal; `run_unified_inline`, `DispatchMode::Session`) runs
`claude --print --output-format stream-json` for each turn. When the agent writes text, calls a tool, then writes
more text, the two assistant messages run together: "…the repository.No `Cargo.toml` exists yet…". This is the
symptom bug-2116ec fixed for the screened plan-run transcript, on a path that fix did not touch.

## Why it matters

Goal `tooling`: the chat is roko's interactive front door, and its replies are hard to read whenever the agent uses
tools, which is most turns.

## Where

- `crates/roko-agent/src/provider/claude_cli/stream.rs::parse_stream_line` (:157-160): each assistant text block
  becomes one `AgentRuntimeEvent::MessageDelta`, so whole messages arrive as deltas. `MessageDelta` carries no
  message id.
- `crates/roko-cli/src/chat_session.rs::send_turn_streaming_with_program` (:1259):
  `accumulated_text.push_str(text)` (:1392) builds `TurnResult.text` (:1491). That becomes the chat's final reply
  (`chat_inline/session.rs:287`, `DispatchResult.text`).
- `crates/roko-cli/src/chat_session.rs::render_stream_event` (:289-292): the plain terminal path writes each delta
  raw to stdout.
- `crates/roko-cli/src/chat_inline/event_loop.rs::run_main_loop`: the live TUI stream appends each delta with
  `session.streaming.append(text)` (:368-372).

## Current state

Checked at `d5c1dc6be` by reading the code. bug-2116ec's fix (`85ba4cb4d`) changed only
`BackendResponse::extract_text` in `roko-agent/src/translate/mod.rs` and `claude_cli_agent.rs`. That code uses the
message id from the raw stream, which the chat's `MessageDelta` path does not have.

## Plan

1. In the chat, start a new paragraph (`"\n\n"`) before a text delta that follows a tool event (`ToolCall` or
   `ToolOutput`) seen since the previous text delta. In practice, tool calls are what separate assistant messages.
   Apply this in all three consumers above, or once in `send_turn_streaming_with_program` before the events are
   forwarded, so that the final reply, the terminal and the live view agree.
2. Alternative: carry a message boundary in the event (a message id on `MessageDelta`, or a boundary event from
   `parse_stream_line`). That is more exact, but it changes `AgentRuntimeEvent`, which many providers and consumers
   share.
3. Add `streaming_turn_separates_assistant_messages`: run `send_turn_streaming_with_program` against a fake program
   that prints two assistant text messages with a tool call between them, and assert that `TurnResult.text` has a
   separator between them.

## Done when

- A chat turn with a tool call between two messages shows them as separate paragraphs, in the final reply and while
  streaming.
- The `[[verify]]` command passes.

## Notes

The existing mock-stream tests in `chat_session.rs` (around :2866-3031) show how to feed `parse_stream_line` output.
- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise still true at BASE `ebdc0f5d5`. Plan step 1, applied once: `send_turn_streaming_with_program` passes each
  parsed event through the new `separate_assistant_messages` before it accumulates or forwards it, so a text delta
  after a `ToolCall`/`ToolOutput` starts a new paragraph (two newlines, fewer when the text so far already ends in
  one). The reply, `render_stream_event` and the inline TUI's `streaming.append` all read the rewritten events, so
  `render_stream_event` and `run_main_loop` needed no change. Test `streaming_turn_separates_assistant_messages`.
