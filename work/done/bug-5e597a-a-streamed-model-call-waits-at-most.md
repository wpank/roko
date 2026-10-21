+++
id = "bug-5e597a"
kind = "bug"
title = "A streamed model call waits at most 120 s for its response whatever [providers.*] timeout_ms says: ToolLoop::turn_config hard-codes DEFAULT_REQUEST_TIMEOUT_MS, and timeout_ms reaches only the non-streamed post (phase 3 run bdc4461e: two glm-4.7 tries, ~51 min, lost to 'no response within 120000ms'; a $0 stand-in with timeout_ms = 300000 fails the same at 120 s)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "c08d870bf"
source = "round-v7-hermes-pilot#M-p3-run-bdc4461e"
discovered_from = "agent:hermes-pilot (round v7 phase 3 run bdc4461e; $0 proof in tmp/hermes-roko-v7/pilot/timeout-e2e)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::turn_config", "crates/roko-agent/src/provider/openai_compat.rs::create_agent", "crates/roko-agent/src/provider/cerebras.rs::create_agent", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::try_time_stop"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_providers_timeout_ms_bounds_a_streamed_calls_wait_for_its_response' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_providers_timeout_ms_bounds_a_streamed_calls_wait_for_its_response"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T02:52:46Z"
commit = "c08d870bf"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-21T01:45:01Z"
forced = false
evidence = "c08d870bf. ToolLoop::with_request_timeout_ms, read by turn_config (120 s when unset), is set by the tool-loop adapters (openai_compat, cerebras, anthropic_api, both gemini ones; Perplexity's builder has no provider config in scope) from the provider's own [providers.*] timeout_ms, not the attempt's AgentOptions::timeout_ms; it bounds a streamed call's wait for its response and each silence in its stream. Past the response wait: the TTFT (first chunk after the headers) keeps its own ttft_timeout_ms; the stall watchdog cancels a call of roko's own loop only after the larger of task_stall_secs and that timeout_ms; at a try's time bound the call in flight gets the larger of TRY_TIME_GRACE (120 s) and that timeout_ms. No longer default for reasoning models: supports_thinking marks models that stream their thinking, and holding the headers is the provider's behaviour, whose timeout_ms Undertow sets to 300000. The literal verify passes: a_providers_timeout_ms_bounds_a_streamed_calls_wait_for_its_response (adapter level: a server holding its headers 1.5 s against timeout_ms 400 fails 'no response within 400ms' though the attempt's timeout is 60 s, and answers with timeout_ms 5000); also a_tool_loop_waits_its_providers_timeout_ms_for_a_response and a_call_may_wait_its_providers_timeout_before_the_watchdog_stops_it (600 s timeout: silent 301 s warns, 599 s healthy, 600 s stalled; the try grace runs to 300 s past the bound). $0 proof (the pilot's harness, copied to tmp/hermes-roko-v7/roko-speed/timeout-e2e, the stand-in holding its headers 150 s): b-300000 exits 0 after 160 s (the call 152.2 s, T1 passed, hello.txt delivered on roko/plan/long-think); a-default exits 1 after 127 s with 'no response within 120000ms'. Lib suites: roko-agent 2389, roko-core 2046, roko-learn 1391, roko-serve 1077, roko-cli 3961 of 3962 (the failure is bug-3203c4's known lock race at load, a_cancel_stops_the_post_merge_regression_check); fmt and clippy clean. Post-merge (25946bd80): bug-3203c4's a_cancel_stops_the_post_merge_regression_check passed alone 3 of 3 (0.48-0.52 s) on work/gap-1430ea, which is 25946bd80 plus the side-call attempt field and touches no delivery code, and that branch's full roko-cli suite passed 3962 of 3962."
+++

## Problem

A streamed model call waits at most 120 s for its response, and no setting raises that. `[providers.<p>] timeout_ms`
reaches only the non-streamed path. So a reasoning model that holds its first response longer than 120 s is cut, and
the try is lost as an infra failure.

**Round v7, Undertow phase 3, run `bdc4461e`** (roko e4643dafd, plan `p3-undertow-hypnotic-sequencer`, T1 the
270-line sequencer, routed to mid = glm-4.7 on Z.ai):

| Try | Ran | Agent | Turns | Ended | Cost |
|---|---|---|---|---|---|
| T1#1 | mid, glm-4.7 | 2034 s | 10 | `provider_error` (infra): "agent error (zai): timeout: no response within 120000ms" | $0.1999 |
| T1#2 | mid, glm-4.7 | 1060 s | 9 | the same; roko climbed the task ("failed the same infra way again", bug-9b65dc) | $0.1128 |

- glm-4.7 averaged 203 s per model call (the run's `rung_slow` event).
- Both tries ended on a call whose response headers didn't come within 120 s. That's not a stalled stream and not a 429.
- About 51 minutes and $0.31 of glm-4.7 work were thrown away as infra. T1 then failed twice on strong, and the run
  failed.

**$0 proof, the same stand-in both times (`tmp/hermes-roko-v7/pilot/timeout-e2e/`):**
- `fake_reasoner.py` in a `late` mode holds its response headers for 150 s, then streams a short answer.
- `run.sh` makes one `roko plan run` of a one-task plan, zero cost, scratch HOME, egress blocked.

| Run | `[providers.fake-zai]` | Result |
|---|---|---|
| a-default | no `timeout_ms` | exit 1 after 121 s: "agent error (fake-zai): timeout: no response within 120000ms" |
| b-300000 | `timeout_ms = 300000` | exit 1 after 122 s: the same "no response within 120000ms" |

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::ToolLoop::turn_config` (~line 1140): builds `TurnConfig` with
  `request_timeout: Duration::from_millis(DEFAULT_REQUEST_TIMEOUT_MS)`, a constant. The model profile feeds only
  `max_tokens`.
- `crates/roko-agent/src/openai_compat_backend.rs` (~line 747): a streamed call's
  `wait = config.request_timeout`, and on expiry `crate::streaming::response_timeout(wait)`, "no response within
  …ms".
- The same file (~line 694): the non-streamed `post_json` uses the backend's own `self.timeout_ms`.
- `crates/roko-agent/src/tool_loop/backends/mod.rs` (~line 61):
  `.with_timeout_ms(provider.timeout_ms.unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS))` is where the provider's
  `timeout_ms` lands. `crates/roko-agent/src/provider/openai_compat.rs` (~lines 547, 567) passes the effective
  timeout there.
- `crates/roko-core/src/defaults.rs:12`: `DEFAULT_REQUEST_TIMEOUT_MS = 120_000`.

## Plan

- The streamed path's wait for its response, and between chunks, comes from the provider's `timeout_ms` when it is
  set, as the non-streamed path's does. `turn_config` reads it from the backend or the provider.
- Consider a longer default for reasoning models: a model-profile `request_timeout_ms`, or a per-provider default for
  providers known to hold responses (Z.ai's glm-4.7).
- Undertow can then set `[providers.zai] timeout_ms = 300000`.

## Done when

With `[providers.<p>] timeout_ms = 300000`, a streamed call whose response comes after 150 s succeeds, and without it
the 120 s default still applies. Checked by the test in `[[verify]]`.
