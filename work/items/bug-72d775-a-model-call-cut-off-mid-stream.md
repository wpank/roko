+++
id = "bug-72d775"
kind = "bug"
title = "A model call cut off mid-stream records no usage: the provider bills what it streamed while roko records $0, so caps and the spend counter undercount"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "tmp/hermes-roko-v6/findings/dogfood-will.md#R3-3 (dogfood-will, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: dogfood-will Run 3, pocket-synth through Hermes"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming", "crates/roko-agent/src/tool_loop/mod.rs::run_inner"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_call_cut_off_mid_stream_records_what_it_streamed' crates/roko-agent/ && cargo test -p roko-agent a_call_cut_off_mid_stream_records_what_it_streamed"
+++

## Problem

A model call cut off mid-stream records no usage at all, although the provider bills what it streamed. In
dogfood-will's Run 3 (`$V6/findings/dogfood-will.md` R3-3), glm-4.7 attempts SVF:5 and SVF:6 each streamed about
4,000 reasoning events before roko's 120 s request timeout cut them ("read chunk failed"). Their verdicts record 0
tokens, `cost.source: unknown` and no `billed_usd`, and z.ai billed roughly $0.03–0.08 for the run's glm-4.7 calls
that roko recorded at $0.

The usage is lost in the tool loop:
- `send_turn_streaming` → `collect_stream_to_response` returns at the first stream error (`event?`), and drops what
  the call streamed;
- `run_inner`'s error arm returns `total_usage` from the completed turns only.

The immune boundary's `denied_result` keeps the usage it is given, so a denial whose stream had broken records $0 too.

## Why it matters

The run's spend, the page's counter, the plan and daily caps, and the cost-per-verified-task numbers all undercount
real provider spend (goal `core`; caps fail open). The counter sat at $0.18 through 6.5 minutes of glm-4.7 streaming.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming`: tees the stream, so it can count what streamed.
- `crates/roko-agent/src/tool_loop/mod.rs::run_inner`: the error arms of a turn.

## Plan

1. While a turn streams, count what it delivered (text, reasoning, tool-call arguments) and the usage the provider
   last reported for it.
2. When the turn fails before its usage arrives, or its stream ends without reporting usage, estimate it:
   - output from what streamed: about 4 bytes a token, and at least one token a delta (a reasoning model streams
     about one token a delta); a reported output stands when it covers what streamed;
   - input as reported (the Anthropic API reports it when a message starts), else from the request, as uncached
     input.
3. Price the estimate with the model's rates, add it to the loop's usage and to the turn budget, and record the turn
   with source `estimated`. The attempt's cost then reads `estimated` with a `billed_usd`, and the plan ledger and
   caps settle it. A call that broke before it streamed anything stays unrecorded, as before.

## Done when

`[[verify]]` passes: a call that streams reasoning and then breaks before its usage arrives returns a failed result
whose usage is estimated, priced and above zero, and the budget tracker saw it; so does one whose stream closes
without usage.
