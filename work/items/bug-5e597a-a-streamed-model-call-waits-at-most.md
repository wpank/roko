+++
id = "bug-5e597a"
kind = "bug"
title = "A streamed model call waits at most 120 s for its response whatever [providers.*] timeout_ms says: ToolLoop::turn_config hard-codes DEFAULT_REQUEST_TIMEOUT_MS, and timeout_ms reaches only the non-streamed post (phase 3 run bdc4461e: two glm-4.7 tries, ~51 min, lost to 'no response within 120000ms'; a $0 stand-in with timeout_ms = 300000 fails the same at 120 s)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#M-p3-run-bdc4461e"
discovered_from = "agent:hermes-pilot (round v7 phase 3 run bdc4461e; $0 proof in tmp/hermes-roko-v7/pilot/timeout-e2e)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::turn_config", "crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_providers_timeout_ms_bounds_a_streamed_calls_wait_for_its_response' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_providers_timeout_ms_bounds_a_streamed_calls_wait_for_its_response"
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
