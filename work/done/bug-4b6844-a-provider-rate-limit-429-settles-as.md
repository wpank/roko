+++
id = "bug-4b6844"
kind = "bug"
title = "A provider rate limit (429) settles as a provider_error attempt and counts against the task: on mid, z.ai rate limits used up T1's last two attempts with no real try, and the task ended"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-18
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "ce3200bcc"
source = "round-v6-dogfood-desktop#L4-g1-ratelimit"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L4, 2026-10-18)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs::FailoverChain", "crates/roko-agent/src/tool_loop/mod.rs::ToolLoop::send_turn_streaming"]
links = { depends_on = [], blocks = [], related = ["bug-df9aff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_429_from_the_rungs_provider_does_not_spend_a_try' crates/ && cargo test -p roko-cli -- a_429_from_the_rungs_provider_does_not_spend_a_try a_rung_whose_provider_keeps_answering_429_fails_over_within_the_try a_429_with_nowhere_to_fail_over_waits_inside_the_try"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T16:12:37Z"
commit = "ce3200bcc"
size = "M"
claimed_at = "2026-10-19T14:46:47Z"
forced = false
evidence = "ce3200bcc: (1) Tool loop (roko-agent tool_loop/mod.rs, ToolLoop::open_stream): a plan attempt's calls stream, and that path never retried; a streamed call the provider refused with a 429 before it streamed anything is now sent again after the Retry-After it named, else the retry policy's backoff, up to max_attempts (3) calls, as the answered path did; a Retry-After over 60 s is not waited there. Test a_rate_limited_streamed_call_is_retried_within_the_run (paused time): 429 twice with Retry-After 1.5 s then served = 3 calls and at least 3 s waited; a lasting 429 = 3 calls then the rate-limit error; a 10-minute Retry-After = 1 call, no wait. It failed before the fix: the run ended at the first 429. (2) Dispatch (roko-cli failover.rs, run_bridge_with_failover): a call still rate-limited fails over within the attempt (refusal class rate_limited, not definitive); with nowhere to go, a pinned model too, the attempt waits (Retry-After, else 5 s doubling to 60 s), marked Queued for the stall watchdog and announced as a dashboard diagnosis, then calls the same model again, until it is served, fails otherwise, fails over, or the run stops, so a 429 never settles a try. Refused calls keep their cost and efficiency rows. Tests against a stand-in OpenAI-compatible server answering HTTP 429 with z.ai's error body and a Retry-After: a_429_from_the_rungs_provider_does_not_spend_a_try (429 twice then served; max_retries 0 and the task passes in its one try), a_rung_whose_provider_keeps_answering_429_fails_over_within_the_try (mid always 429; one try, run by strong, failover_chain [mid-1], ladder reason failover), a_429_with_nowhere_to_fail_over_waits_inside_the_try (3 x 429 then served, no fallback, max_retries 0; passes). All three failed before the fix with Agent { provider error: rate limited; retry after 1000 ms }. Checks at ce3200bcc: roko-agent lib 2374 passed; roko-cli lib 3865 passed, 2 failed: the known load flake plan_check_publishes_start_heartbeat_and_finish (passes alone) and my ladder test's wrong outcome assertion (an unchecked task settles unverified; fixed, failover:: 20 of 20 pass); clippy -D warnings (roko-agent, roko-cli) clean; nightly rustfmt clean."
+++

## Problem

A provider's rate limit used up a task's attempts with no real try. In round v6's showcase take L4 (2026-10-18, roko
68528f4b2), plan run `83c4222a-3523-4547-b587-1a37408b486f` on pocket-synth-prd had four tasks fail twice on cheap at
once and climb to mid (glm-4.7 via z.ai). z.ai answered "rate limited". Each such call settled as a whole attempt:

| Task | Attempt | Rung | Model / provider | Verdict | Detail | When (UTC) |
|---|---|---|---|---|---|---|
| T1 | 3 | mid | glm-4.7 / zai | `provider_error` (reason `climb`) | `provider: provider error: rate limited` | 23:20:01.808 → 23:20:02.326 |
| T2 | 3 | mid | glm-4.7 / zai | `provider_error` (reason `climb`) | `provider: provider error: rate limited` | 23:20:03.539 → 23:20:03.976 |
| T1 | 4 | mid | glm-4.7 / zai | `provider_error` (reason `retry`) | `provider: provider error: rate limited` | 23:20:05.233 → 23:20:05.631 |
| T1 | — | — | — | `task_completed`, with no pass | | right after |
| T2 | 4 | strong | gpt-5.4-mini / openai | `gate_failed` (reason `failover`) | | 23:20:06.360 → 23:20:15.858 |

T1 never had a real try on mid: two half-second 429s took its last two attempts, and the task ended. T2's second
rate limit failed over to strong, which is the behaviour every rate-limited call should get, inside the try.

## Why it matters

A rate limit is the provider's state, not the model's failure or the task's. When it spends an attempt, a task can
end without the ladder ever trying it on the rung it climbed to. This happens most when several tasks climb at once,
which is the moment the ladder matters. The run's records then read as failures on mid that never happened.

## Where

- `crates/roko-cli/src/graph_task_dispatch/failover.rs` (`FailoverChain`): the same model on another provider, then
  the rungs above. It isn't reached for T1's 429s.
- The attempt settlement that counts a `provider_error` as one of the task's attempts.

## Current state

A 429 from the rung's only provider settles the attempt as `provider_error`, and it counts. T1's ended its task.

## Plan

Inside the try, back off and retry the call while its rate limit lasts (honouring `Retry-After`), then fail over as
`FailoverChain` does. Settle the try as `provider_error` only when the chain is exhausted, and don't count it against
the task's attempts or the rung's climbs. The try's wall time stays bounded (see bug-f0007b).

## Done when

A task whose rung's provider answers 429 still gets its tries on that rung (or a failover's), and a 429 never ends a
task by itself. A test drives a stand-in provider that answers 429 twice, then succeeds.

## Notes

- Evidence:
  - `/Users/Shared/pocket-synth-prd/.roko/runs/83c4222a…/attempts.jsonl`, and `events-by-run/` (the
    `attempt_settled` and `task_completed` events above);
  - the take `tmp/hermes-roko-v6/media/pipeline/work/takes/l4-g1-prd-0117`;
  - `tmp/hermes-roko-v6/findings/dogfood-desktop.md`.
- Related:
  - bug-df9aff: the same run, whose tasks could not create their files;
  - bug-f0007b: no time bound on a try.

## Root cause (2026-10-19, roko-ladder, round v7)

- A plan attempt's calls stream (`ToolLoop::run_streaming` → `send_turn_streaming`), and that path never retried:
  the first 429 (`LlmError::Provider(RateLimit)`, returned before any byte streams) ended the run as
  `BackendError("provider error: rate limited")`. Only the non-streaming `send_turn_with_retry` retried rate
  limits, and its test used `run`.
- `run_bridge_with_failover` failed over after a call only on usage exhaustion. A plain rate limit returned the
  failed dispatch, which settled `provider_error` and spent one of `TaskExecutorCell`'s retries. Failover reached a
  rate-limited rung only once its circuit was open (three straight failures), before the call: that is how v6's T2
  reached strong on its second 429 while T1's last two tries were spent.
- Rate limits already never climb a task (`ladder.rs::ACCOUNT_FAILURES`).
