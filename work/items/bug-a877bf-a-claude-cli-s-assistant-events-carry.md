+++
id = "bug-a877bf"
kind = "bug"
title = "A Claude CLI's assistant events carry each message's output count from its start, so the budget's streamed estimate misses nearly all output and thinking (probe: 6 of 186 tokens)"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-agent/claude-cli"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16, bug-acfced probe)"
discovered_from = "bug-acfced: probe of Claude Code 2.1.282's stream-json in print mode"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::StreamedMessages", "crates/roko-agent/src/claude_cli_agent.rs::stream_usage"]
links = { depends_on = [], blocks = [], related = ["bug-acfced", "gap-ca1c92", "bug-aa2044"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_streamed_claude_cli_estimate_counts_a_message_s_whole_output' crates/roko-agent/src && cargo test -p roko-agent --lib a_streamed_claude_cli_estimate_counts_a_message_s_whole_output"
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
