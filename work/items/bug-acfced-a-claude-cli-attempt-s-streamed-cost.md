+++
id = "bug-acfced"
kind = "bug"
title = "A Claude CLI attempt's streamed cost estimate prices its cache writes at the 5-minute rate, so the plan budget counts a Claude Code session's 1-hour cache writes about 37% low"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/claude-cli"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16, gap-ca1c92 live check)"
discovered_from = "gap-ca1c92: live check on claude-haiku-4-5 under a $0.04 cap"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::stream_usage", "crates/roko-agent/src/claude_cli_agent.rs::StreamedMessages"]
links = { depends_on = ["gap-a960a1"], blocks = [], related = ["gap-ca1c92", "bug-aa2044", "gap-a960a1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_streamed_claude_cli_estimate_prices_cache_writes_at_the_session_ttl' crates/roko-agent/src && cargo test -p roko-agent --lib a_streamed_claude_cli_estimate_prices_cache_writes_at_the_session_ttl"
+++

## Problem

While a Claude CLI attempt runs, roko estimates what it spent from the usage the CLI streams
(`StreamedMessages::stream_usage`). The plan budget's watch counts that estimate (gap-ca1c92), and so does the
settlement of a stalled, stopped or budget-stopped attempt (bug-aa2044). The estimate prices every cache write at
the model table's `cache_write_per_m`, the 5-minute rate (1.25 times input). roko's own records treat a Claude Code
session's cache writes as 1-hour writes (2 times input): `CacheWriteClass::OneHour` for the verdict's usage, and
`model_tokens` for the snapshot comparison (backlog 6105). So the plan budget counts a Claude CLI attempt's cache
writes about 37% low, and the verdict prices the same attempt higher.

Seen live (roko-caps, 2026-10-16, gap-ca1c92's check). A Claude CLI attempt on claude-haiku-4-5 under a $0.04 cap
was stopped by the plan budget at $0.0373 counted (`vendor_usd`). Its verdict prices the same usage, including
23,478 cache-write tokens in the 1-hour class, at `api_equiv_usd` $0.0549: 37% past the cap.

## Why it matters

A run's cap should hold in the dollars roko reports for the run. For a Claude CLI rung (the top rung of Will's
ladder), the cap now holds only in the lower estimate.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::stream_usage`: prices each streamed message.
- `crates/roko-agent/src/claude_cli_agent.rs::StreamedMessages`: keeps each message's usage. It does not keep the
  message's TTL split (`usage.cache_creation.ephemeral_5m_input_tokens` / `ephemeral_1h_input_tokens`) when the
  event carries one.

## Plan

1. Keep each streamed message's cache-write TTL split when its usage has one. Price its 1-hour writes at twice the
   input rate (Anthropic's 1-hour cache write price) and its 5-minute writes at `cache_write_per_m`. With no split,
   price all of them as 1-hour writes, as `model_tokens` does.
2. Update the tests that pin the 5-minute rate for streamed Claude CLI estimates:
   `parse_stream_usage_estimates_streamed_usage_without_result_event` and
   `a_timed_out_attempt_reports_the_usage_it_streamed` (roko-agent), `a_stalled_attempt_records_the_usage_it_streamed`
   (roko-cli watchdog), and the streamed-usage test in `graph_execution/plan_runner.rs` (around its "$3.75 cache
   write" comment). That file is held by gap-a960a1 this round, so this item waits for it.

## Done when

`a_streamed_claude_cli_estimate_prices_cache_writes_at_the_session_ttl` passes (the `[[verify]]` command).

## Notes

Found by roko-caps while checking gap-ca1c92 live. Logs: `tmp/hermes-roko-v5/roko-caps/logs/cli-run.json` and the
project `tmp/hermes-roko-v5/roko-caps/proj-cli/.roko/`.
