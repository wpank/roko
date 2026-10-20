+++
id = "bug-4b6844"
kind = "bug"
title = "A provider rate limit (429) settles as a provider_error attempt and counts against the task: on mid, z.ai rate limits used up T1's last two attempts with no real try, and the task ended"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-18
updated = 2026-10-18
source = "round-v6-dogfood-desktop#L4-g1-ratelimit"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L4, 2026-10-18)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs::FailoverChain"]
links = { depends_on = [], blocks = [], related = ["bug-df9aff"], supersedes = [], duplicate_of = "" }
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
