+++
id = "bug-a877bf"
kind = "bug"
title = "A Claude CLI's assistant events carry each message's output count from its start, so the budget's streamed estimate misses nearly all output and thinking (probe: 6 of 186 tokens)"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-agent/claude-cli"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "78f2db374"
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16, bug-acfced probe)"
discovered_from = "bug-acfced: probe of Claude Code 2.1.282's stream-json in print mode"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::StreamedMessages", "crates/roko-agent/src/claude_cli_agent.rs::stream_usage"]
links = { depends_on = [], blocks = [], related = ["bug-acfced", "gap-ca1c92", "bug-aa2044"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_streamed_claude_cli_estimate_counts_a_message_s_whole_output' crates/roko-agent/src && cargo test -p roko-agent --lib a_streamed_claude_cli_estimate_counts_a_message_s_whole_output"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:32:23Z"
commit = "78f2db374"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-16T13:24:16Z"
forced = false
evidence = "78f2db374: roko passes --include-partial-messages; each message's message_delta (its final usage) settles its output exactly, and until then its text and tool-input deltas (at 4 characters a token) and its thinking (that, or the CLI's system/thinking_tokens estimate, whichever is larger) estimate it; a message cut short keeps that estimate; usage updates go out every 32 estimated tokens. Live text is unchanged: whole blocks from assistant events, the deltas reaching the live stream only as usage updates, so the conductor's ghost-turn watcher does not count fragments. A real Claude Code 2.1.282 run on Haiku 4.5 (two messages, a Read call), captured with roko's flags, is the fixture crates/roko-agent/tests/fixtures/claude-cli/partial_messages_haiku.jsonl: replayed as the live stream sees it, the estimate before its result event is the CLI's own $0.009188 (input 5,778, output 682), where the assistant events alone counted 7 output tokens ($0.0058). a_streamed_claude_cli_estimate_counts_a_message_s_whole_output, a_message_cut_short_counts_the_output_it_streamed, partial_messages_send_their_text_live_once and a_run_asks_for_partial_messages pass. At 4f647ebd5 (with gap-130413): fmt --check, clippy -D warnings (roko-agent, roko-cli), roko-agent lib tests 2363 passed, roko-cli lib tests 3725 passed, and the [[verify]] command passes. Live spend $0.0114 (two Haiku probes)."
+++

## Problem

While a Claude CLI attempt runs, roko estimates its spend from the usage in each stream-json `assistant` event
(`StreamedMessages`, bug-aa2044). The plan budget's watch counts that estimate, and so does the settlement of an
attempt that is stopped or cancelled (gap-ca1c92). An `assistant` event carries its message's usage as it stood when
the message started: its output count is the first few tokens. The message's real output, thinking included, arrives
only in the `result` event at the end of the run. So the estimate misses nearly all output.

Probe (roko-caps, 2026-10-16, Claude Code 2.1.282, `claude-haiku-4-5`, print mode with roko's flags,
`tmp/hermes-roko-v5/roko-caps/ttl-probe/probe1.jsonl`): both `assistant` events of the one message reported
`output_tokens: 6`. The `result` reported 186, and the CLI's own cost ($0.022718) includes all 186. Thinking shows
up in the stream only as `system`/`thinking_tokens` events with a running `estimated_tokens` (50, 150, 243).

## Why it matters

At Sonnet's $15 per million output tokens, a message that writes 1,000 tokens of code or thinking adds $0.015 that
the cap does not see while the attempt runs. A CLI attempt can therefore pass the run's cap by more than one
message's worth before the watch stops it, and a stopped attempt settles below what it cost.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::StreamedMessages`: keeps each message's usage from its events.
- `crates/roko-agent/src/claude_cli_agent.rs::stream_usage`: prices it.

## Plan

1. Count each message's output as the larger of its reported count and an estimate from its content blocks (text,
   thinking and tool input: characters / 4). Add the run's latest `thinking_tokens` estimate when the thinking
   blocks carry no text.
2. Or pass `--include-partial-messages` and read each message's final usage from its `message_delta` stream event,
   if that is cheaper to parse than the content.
3. Test with a scripted stream shaped like the probe: the estimate comes within a stated margin of the result's
   count.

## Done when

`a_streamed_claude_cli_estimate_counts_a_message_s_whole_output` passes (the `[[verify]]` command).
