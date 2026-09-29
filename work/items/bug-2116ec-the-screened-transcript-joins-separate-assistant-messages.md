+++
id = "bug-2116ec"
kind = "bug"
title = "The screened transcript joins separate assistant messages with no separator"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-agent/claude-cli", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "85ba4cb4d"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-agent/src/provider/claude_cli/stream.rs:164", "crates/roko-cli/src/graph_task_dispatch.rs:2842"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo build -p roko-cli && bash -c 'source plans/portal-programme/_harness/lib.sh && require_binary && make_workspace && touch \"$WS/.roko/fake-claude-live\" && start_server && start_capture && [ \"$(api POST /api/plans/live-b/execute)\" = 202 ] && wait_idle live-b 120 && sleep 3 && stop_capture && sse has agent_output plan_id=live-b \"content~Wrote the requested artifacts\" \"content!~.Wrote the requested\"'"

[closed]
at = 2026-09-29
commit = "85ba4cb4d"
evidence = "85ba4cb4d: BackendResponse::extract_text (the Claude CLI screened body) starts a new paragraph for text from a different assistant message; blocks of one message still join as-is. [[verify]] passes (fake-agent live run: screened text 'Working on out/live-b-t01.txt.\\n\\nWrote the requested artifacts.'); tests output_text_separates_assistant_messages, stream_json_extract_text_separates_assistant_messages, stream_json_extract_text_joins_blocks_of_one_message pass; live-output-check.sh 15/15 PASS"
+++

## Problem

The real-model task's screened transcript is a single `text` record: "I'll start by reading the
relevant files to understand the current state of the repository.No `Cargo.toml` exists yet. I'll
create both files now.Both files are created. Here's a summary…". That is three assistant messages,
separated by tool calls in the session, joined with nothing between them. The portal shows it that
way (`tmp/portal-audit/evidence/hello-world-real-run1/3-done.png`). The 11:40 re-run shows the same: "…the workspace
state.No `Cargo.toml` exists yet. I'll create both files now.Both files are created…".

The fake agent reproduces it: its two assistant messages arrive as "Working on
out/live-b-t01.txt.Wrote the requested artifacts.", while the CLI's own `result` field holds only the
second. So the join happens in roko.

## Why it matters

Goal `visibility`. The screened transcript is the one text record that reaches the portal (the
immune boundary releases only the final body), and running sentences together makes it harder to
read.

## Where

The Claude CLI stream parser turns each assistant text block into an
`AgentRuntimeEvent::MessageDelta` (`provider/claude_cli/stream.rs:164-165`), so whole messages travel
as token deltas. Somewhere between dispatch and the screened `agent_output`, consecutive deltas are
concatenated; `graph_task_dispatch.rs::forward_dispatch_events_to_tui` (:2842) forwards the result.
The exact join point was not pinned down.

## Plan

Carry a message boundary for whole-message blocks (a separator, or a distinct event), and join
messages with a newline when the screened body is built.

## Done when

The `[[verify]]` (fake-agent harness, live mode) finds the screened text with a separator before
"Wrote the requested artifacts".

## Notes

Found by plan 09 T04 (see VERDICT).
