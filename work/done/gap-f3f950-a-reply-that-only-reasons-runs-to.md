+++
id = "gap-f3f950"
kind = "gap"
title = "A reply that only reasons runs to max_output before roko throws it away and asks again: stop it at 3/8 of max_output with no text and no tool call (glm-4.7, phases 0-3: 7 of 335 calls, 38% of the model time; the re-ask acted in 23-97 s each time)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "6d15615f7"
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

[closed]
at = 2026-10-21
at_ts = "2026-10-21T07:56:18Z"
commit = "6d15615f7"
by = "roko-speed"
executor = "unknown"
forced = false
evidence = "6d15615f7. roko-agent tool_loop: guard_reasoning wraps each streamed turn (send_turn_streaming) and ends the stream once it has streamed reasoning_limit(max_tokens) = 3/8 of the turn's max_tokens in reasoning tokens (the larger of deltas and chars/4) with no non-blank text and no tool call, emitting Done with finish reason reasoning_limit (streaming::REASONING_LIMIT_FINISH, which normalize_finish_reason reads as Length). bug-872885's length-cut branch then asks again with length_cut_note(tokens, stopped) (\"...reasoned for about N tokens without a tool call or any text, so it was stopped...\"), and a second cut in a row ends the run as before; bug-cbbaa8's hand-off continue skips a cut as it skips a length cut, and a cut with no tool work fails as \"model hit output token limit (finish_reason=reasoning_limit)\". Not guarded: a backend whose LlmBackend::reasoning_budget() is Some (the Anthropic API tool loop's thinking budget) and a turn whose max_tokens is 0; OpenAI's hidden reasoning never streams. ToolLoopTurnTrace.duration_ms, in the agent.trace JSON. roko-cli: GraphTaskDispatcher::publish_reasoning_cuts, once a try's dispatch returns, publishes DashboardEvent::ReasoningCut {plan_id, task_id, attempt, turn, model, reasoning_tokens, limit_tokens, secs, saved_secs (the rest of max_output at the reply's pace), reask (acted, answered, cut_again, ended), detail} per stopped turn; red_flags' output_limit_note names a stopped closing reply; serve logs reasoning_cut, health lists it and GET /api/runs/{id}/summary lists reasoning_cuts; docs/v3/26-HTTP-API.md. A call the watchdog, the conductor or a stop cancels keeps no turn traces, so its cuts are not published. Tests: a_reply_that_only_reasons_is_stopped_at_its_reasoning_limit (scripted SSE, the 16,384-token default: a reply of 7,000 one-token deltas is stopped at 6,144 with finish reasoning_limit, its tool call never runs, the re-ask's note says it was stopped and the try goes on to pass; a reply that writes text first, or a backend with its own reasoning budget, is not stopped), a_reply_stopped_twice_for_its_reasoning_ends_the_try_and_says_so (roko-cli end to end over the OpenAI-compatible SSE mock with a 1,024-token model: two replies of about 500 reasoning tokens are each stopped at 384, the try is rejected as no_changes saying finish_reason=reasoning_limit, and the hub gets reasoning_cut events cut_again then ended), a_reasoning_cut_reaches_the_hub_with_what_the_reask_did (12,288 tokens in 330 s of a 32,768 limit: saved 550 s, acted; a provider's own length cut is not one), a_runs_summary_lists_its_reasoning_cuts; bug-872885's length-cut tests and bug-cbbaa8's hand-off tests beside them pass. a_call_cut_off_mid_stream_records_what_it_streamed now gives its model a 32,768-token output limit: its 8,000-delta reasoning stream was stopped at the 16,384 default's 6,144 before its network cut. On main 7f20b9578's tree: fmt and clippy -D warnings clean; lib suites roko-agent 2396, roko-cli 3983, roko-core 2047, roko-learn 1393, roko-runtime 213, roko-serve 1081, all passing."
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
