+++
id = "gap-f3f950"
kind = "gap"
title = "A reply that only reasons runs to max_output before roko throws it away and asks again: stop it at 3/8 of max_output with no text and no tool call (glm-4.7, phases 0-3: 7 of 335 calls, 38% of the model time; the re-ask acted in 23-97 s each time)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7 roko-speed glm-4.7 speed study (findings/roko-speed.md), the lead's decision"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming", "crates/roko-agent/src/tool_loop/mod.rs::length_cut_note", "crates/roko-agent/src/translate/mod.rs::normalize_finish_reason"]
links = { depends_on = [], blocks = [], related = ["bug-872885"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_reply_that_only_reasons_is_stopped_at_its_reasoning_limit' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_reply_that_only_reasons_is_stopped_at_its_reasoning_limit"

[[verify]]
command = "grep -rqw 'fn a_reasoning_cut_reaches_the_hub_with_what_the_reask_did' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_reasoning_cut_reaches_the_hub_with_what_the_reask_did"

[[verify]]
command = "grep -rqw 'fn a_reply_stopped_twice_for_its_reasoning_ends_the_try_and_says_so' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_reply_stopped_twice_for_its_reasoning_ends_the_try_and_says_so"
+++

## Problem

A reasoning model can stream a whole reply of reasoning with no text and no tool call. roko throws that reply away
and asks again (bug-872885's re-ask, "make your next tool call now"); the re-ask never sees the cut reasoning. Until
the reply reaches max_output, roko only waits.

Round v7, Undertow phases 0–3 ($0, from `.roko/events-by-run`, `tmp/hermes-roko-v7/roko-speed/glm/analyze_calls.py`):
glm-4.7 on z.ai made 335 calls in 18 tries, 279 model-minutes, 85% of their output reasoning, at 33–40 tok/s. Seven
calls (2%) reasoned to max_output (32,768 tokens) with no text and no tool call, 14–16 min each: 107 of the 279
minutes (38%). In all five cases with a next call, the re-ask acted within 23–97 s and 700–3,500 tokens (it wrote
sequencer.h, rumble.h, output_stage.h or edited output_stage.cpp); c91b63ea T2 and T3 then passed. Phase 3's T1#1
(7b8cb121) spent 32.2 of its 49.4 model-minutes in two such replies before its 2700 s bound.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming`: consumes a turn's stream to its end.
- The same file's length-cut branch (`LENGTH_CUT_RETRIES`, `length_cut_note`), which asks again after a reply that
  reached the output limit with no text and no tool call.

## Plan

- Stop a streamed call once it has streamed R reasoning tokens with no text and no tool call, R = 3/8 of the turn's
  max_output (12,288 for glm-4.7's 32,768), with no new key. Its finish reason is `reasoning_limit`, which reads as a
  length cut, so bug-872885's re-ask follows (with a note that says roko stopped it), and a second cut in a row ends
  the try, as now. A provider that bounds reasoning itself (the Anthropic API's thinking budget) is not guarded;
  OpenAI's hidden reasoning never streams, so it is never cut.
- Turn traces carry each call's `duration_ms`. After a try's dispatch, roko publishes one `reasoning_cut` event per
  stopped reply: the tokens it reasoned, how long it ran, the seconds the stop saved by estimate, and what the re-ask
  did (`acted`, `answered`, `cut_again`, `ended`). The run summary lists them.

Simulated on the same 335 calls (a re-ask that replaces an action costs 60 s):

| R | Calls stopped | Of them, later acted | Saves |
|---|---|---|---|
| 8,192 | 16 | 9 | 102 of 279 model-min (37%) |
| **12,288 (chosen)** | **12** | **5** | **78 of 279 model-min (28%)** |
| 16,384 | 10 | 3 | 60 of 279 model-min (21%) |

The five calls a 12,288 guard would have stopped that later acted, all with small actions:

| Try and call | Output tokens | Reasoning | Ran | Then |
|---|---|---|---|---|
| 20c780ae T1#1 call 17 | 13,979 | 13,559 | 360 s | edit_file output_stage.h |
| 20c780ae T2#4 call 6 | 20,447 | 19,194 | 536 s | multi_edit filter.cpp |
| 7b8cb121 T1#2 call 10 | 26,095 | 26,054 | 756 s | glob tests/spec/**/spec_engine.h |
| 7b8cb121 T2#1 call 6 | 13,657 | 13,641 | 411 s | read_file voice.cpp |
| c91b63ea T3#1 call 8 | 24,774 | 24,621 | 641 s | read_file params.cpp |

T1#1 of 7b8cb121 alone: 1,198 of its 2,985 s (40%) at 12,288.

## Done when

A streamed reply that reasons R tokens with no text and no tool call is stopped there and asked again, a reply that
acts before R is not, a provider with its own reasoning budget is not guarded, and each stop is a `reasoning_cut`
event. Checked by the tests in `[[verify]]`.
