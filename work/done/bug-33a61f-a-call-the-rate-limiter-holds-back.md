+++
id = "bug-33a61f"
kind = "bug"
title = "A call the rate limiter holds back mid-run ends the agent's run after the tool loop's three quick retries, so the try restarts the agent from its prompt and loses its conversation (phase 2.2 run 20c780ae: T2 lost 2 min of glm-4.7 work to a hold; phase 4 runs three editor tasks on a 2-slot glm-4.7)"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "41cfa0ef1"
source = "round-v7-roko-ladder#bug-62ae1e-followup"
discovered_from = "agent:roko-ladder (bug-62ae1e review of run 20c780ae; the lead asked for it before phase 4, 2026-10-21)"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
links = { depends_on = [], blocks = [], related = ["bug-62ae1e", "bug-4b6844"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_call_held_mid_run_waits_and_continues_its_conversation' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_call_held_mid_run_waits_and_continues_its_conversation"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T04:08:55Z"
commit = "41cfa0ef1"
via = "manual"
size = "M"
forced = false
evidence = "41cfa0ef1: the OpenAI-compatible backend's admit no longer fails a held call at once: it waits for its slot, polling the shared limiter every 250 ms for up to HELD_CALL_WAIT (5 min; the learned limit lapses 60 s after the provider's last refusal anyway), then sends the call in the same run and turn, so the agent's conversation goes on; past the wait it fails as a rate limit, without calling, as before. The wait is said as LiveAgentEvent::Queued on the agent's live output, which the OpenAI-compatible and Cerebras adapters now hand the backend (create_observed_openai_compat_backend), on every look, since the immune boundary's buffer can deliver the run's own last tool result after the first one; so the attempt's watchdog leaves the wait out of a stall's silence and of the try's time bound. It marks the call itself, not the provider (the lead agreed): a per-provider own_calls would hide this attempt's own silent call. rate_limit_endpoint now keys Perplexity and Cerebras providers with no base_url by their own default URLs. Tests: a_call_held_mid_run_waits_and_continues_its_conversation, a $0 stand-in that teaches a one-slot limit while the agent's first call is in flight, so its second is held 8 s; red before the fix: the second request was a fresh conversation (system + user only, the run restarted); green: 3 requests in one run, the second carries the first call's tool result, under a 5 s mid try bound, so the queued wait is not counted; parallel_calls_share_a_providers_rate_limit (a held call waits and goes out once the first call ends, Queued true then false on its live output; a call held past a 300 ms wait fails as rate-limited without calling); rate_limit_endpoint_is_the_chat_endpoint_its_backend_calls; bug-62ae1e's four tests and three_tasks_at_once still pass. Checks: roko-agent lib 2382 passed; roko-cli lib 3956 passed, 0 failed (share test skipped); clippy -D warnings (roko-agent, roko-cli) clean; nightly rustfmt clean."
+++

## Problem

A call the run's rate limiter holds back mid-run ends the agent's run. The limiter holds a call while the
provider is at the calls at once its rate limits showed it takes (bug-4b6844). The hold surfaces as a rate
limit, and the tool loop sends it again only three times, about 3 s in all (`ToolLoop::open_stream`, the
default retry policy). A slot rarely frees that fast: on glm-4.7 a call takes 10 s to 2 min. So the run
fails with the rate limit.

Since bug-62ae1e (873137110) the try then waits for a slot and calls the same model again. But that call
starts the agent over from its prompt. Its edits stay on disk; its conversation, and the reasoning in it, is
lost.

Evidence:
- Round v7, phase 2.2, run `20c780ae`: T2's first try ran 2 min on glm-4.7. Then one of the 16 holds ended
  its run, and failover handed the try to gpt-5.4-mini.
- Phase 4 runs seven editor tasks, three in parallel on glm-4.7, which Z.ai takes two at a time. A
  restart on every hold would throw away the long reasoning that makes glm-4.7 worth waiting for (the
  lead, 2026-10-21).

## Where

- `crates/roko-agent/src/openai_compat_backend.rs`: `admit` fails a held call at once as
  `ProviderError::RateLimit { retry_after_ms: None }`. Both `send_turn` and `stream_turn` call it.
- `crates/roko-agent/src/tool_loop/mod.rs::open_stream`: at most `RetryPolicy::max_attempts` (3) calls,
  with a 1 s then 2 s backoff.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs`: the stall watchdog counts silence, except while
  the call is marked queued (`LiveAgentEvent::Queued`). Only the Claude CLI marks its permit wait so.

## Plan

1. **A held call waits in place.** `admit` waits for its slot, polling the limiter, instead of failing at
   once. The call then goes out within the same run and turn, and the conversation goes on.
   - Bound: 5 minutes per call (`HELD_CALL_WAIT`). The learned limit lapses 60 s after the provider's
     last refusal, so a hold rarely lasts that long. Past it, the call fails as rate-limited, as today.
2. **The wait is marked queued.** The backend gets the agent's live output (`AgentOptions.live_output`,
   passed by the OpenAI-compatible and Cerebras adapters). It sends `Queued { waiting: true }` while the
   call is held and `Queued { waiting: false }` once admitted. So neither the stall watchdog nor the try's
   time bound counts the wait, just as with the Claude CLI's permit wait.
   - This marks the attempt's own call, not the provider. A provider full of the run's calls
     (`own_calls`) also counts this attempt's call while it is in flight, and must not hide a call that has
     really gone silent.
3. **The failover-level wait stays as the backstop** (bug-62ae1e), for a run that fails on real 429s while
   the run's own calls fill the provider.

## Done when

A call held mid-run continues the same conversation once its slot frees: its next request carries the
tool result of the call before it. The stall watchdog, set to cancel after 2 s of silence, takes the wait
for a queue. Checked with a $0 stand-in by the test in `[[verify]]`.
