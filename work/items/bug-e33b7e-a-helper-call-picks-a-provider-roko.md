+++
id = "bug-e33b7e"
kind = "bug"
title = "A helper call picks a provider roko last saw out of balance: helper_healthy checks only an open circuit and this run's failed probes, so once z.ai's 30-minute exhaustion cooldown had lapsed, phase 4.2's retry-diagnosis helper (11:14:05Z) went to glm-4.7 on z.ai, out of balance since 10-21, and failed in 0.6 s"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 8, run 1447385e), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::helper_healthy", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::select_cheap_model_key", "crates/roko-learn/src/provider_failover.rs::record_exhaustion"]
links = { depends_on = [], blocks = [], related = ["bug-3e3d08", "gap-20301d", "bug-5b04d3", "gap-f1dd0c", "bug-f9ca2f", "bug-390e4b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_helper_skips_a_provider_last_seen_exhausted' crates/roko-cli/src && cargo test -p roko-cli --lib a_helper_skips_a_provider_last_seen_exhausted"
+++

## Problem

Helper calls (the retry diagnosis, gate reflections, the judge) take the cheapest helper model that `helper_healthy`
accepts (bug-f9ca2f): its provider's circuit is closed in the run's health registry, and it didn't fail a probe in
this run. On 2026-10-26 neither check knew that z.ai was out of balance.

- **Provider health had recorded it.** z.ai was marked `Exhausted` twice on 2026-10-21 at 09:08Z (code 1113,
  insufficient balance). `record_exhaustion` quarantines a provider until its reported reset, else for
  `routing.exhaustion_cooldown_secs`. Undertow sets that to 1800, and a balance error reports no reset, so the
  circuit closed 30 minutes later. The balance was never restored.
- **No probe in this run covered it.** glm-4.7 isn't on phase 4.2's ladder (OpenAI only), so the run never probed it.
  Its last probe in `learn/rung-probes.json` passed, on 2026-10-20.

So at 11:14:05Z, run 1447385e's retry-diagnosis helper for T1 went to glm-4.7 on z.ai.
- z.ai answered "exhausted" in 0.6 s. That was z.ai's fourth failure in a row, so its circuit opened again, cooling
  down until 11:44Z.
- The retry got roko's fallback diagnosis, which bug-3e3d08 shows comes out garbled for doctest output.
- It cost $0, but the retry lost the helper's diagnosis.

Expected:
- A helper never calls a provider roko last saw exhausted until something shows it can pay again.
- In a run whose ladder is restricted, helpers follow the run's ladder. A chat host's run and an OpenAI-only config
  are both restricted.

## Why it matters

Golden path. The retry diagnosis is what the next try reads first.
- In round v7 the cheapest configured models were the ones out of balance: z.ai since 10-21 06:13Z, and Cerebras
  (402) all round.
- So helpers keep reaching for exactly the providers that can't answer.
- Each such call is fast and free, but the diagnosis is lost. bug-f9ca2f saw the same shape with Cerebras in run
  82b7e069.

## Where

- `roko-cli/src/graph_task_dispatch.rs::helper_healthy`: circuit closed, and no failed probe in this run.
- `graph_task_dispatch/routing_context.rs::select_cheap_model_key`: ranks the helper candidates by price.
- `roko-learn/src/provider_failover.rs::record_exhaustion`: the quarantine, until the provider's reset or for
  `routing.exhaustion_cooldown_secs` (default 600).
- What roko knew: Undertow's `.roko/learn/provider-health.json` (z.ai's failure window) and `rung-probes.json`.

## Current state

- bug-f9ca2f (done) added the circuit and probe checks, after Cerebras's 402s in run 82b7e069.
- bug-390e4b (done) made 1113 and 402 answers count as exhaustion: no retry, and the provider is skipped for the
  cooldown.
- gap-20301d (roko-ladder, in progress) reports a run's skipped rungs. Helpers aren't rungs.

## Plan

There are three options:
1. **Make exhaustion last for helpers.** A provider last seen exhausted stays skipped until one of its calls succeeds
   or a probe passes. Billing doesn't recover on a timer.
2. **Probe a helper model before its first use in a run**, as rungs are probed (cached 24 h, one tiny call).
3. **In a run whose ladder is restricted, take helpers from its own rungs** whose probes passed. That covers a chat
   host's run and a tool-loop-only config.

Recommended: 1 and 3. Option 2 alone would still call z.ai once a day.

## Done when

- A run whose cheapest helper candidate's provider was last seen exhausted, with its cooldown lapsed and no call
  since, picks another model. Test: `a_helper_skips_a_provider_last_seen_exhausted`.
- The `[[verify]]` passes.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 8.
- Undertow's records:
  - `learn/costs.jsonl`: role `helper`, glm-4.7 on zai, 11:14:06Z, success false, 599 ms;
  - `roko.log.2026-10-26`, 11:14:05–06Z: "creating agent … glm-4.7 … api.z.ai", then "provider circuit opened …
    error_class=Exhausted consecutive_failures=4".
- The diagnosis text itself is bug-3e3d08. The helper's whole-catalog prompt is bug-5b04d3. Choosing the
  distiller's model is gap-f1dd0c.
- **A second occurrence: 2026-10-26, phase 4.3 (run 3752b8ff).** T2#1's retry-diagnosis helper went to glm-4.7 on
  z.ai again.
  - `roko.log.2026-10-26`, 14:37:02.108Z: "creating agent … glm-4.7 … api.z.ai".
  - 14:37:02.717Z: "provider circuit opened … provider=zai error_class=Exhausted consecutive_failures=5",
    `cooldown_until_ms` 15:07:02Z.
  - `learn/costs.jsonl`: role `helper`, glm-4.7 on zai, 14:37:02.726Z, 615 ms, $0, 0 tokens.
  - `learn/provider-health.json` puts z.ai's last success at 2026-10-21T03:58Z.
- **So T2#2 got roko's fallback diagnosis**: "The primary error is [doctest] test cases: 4 | 2 passed | 2 failed |
  0 skipped. The retry should focus on the failing code path for …".
  - Its prompt log: `.roko/prompt-logs/p4-3-the-editor-s-finish-T2-1793025422990.txt`.
  - The feedback log line still says `has_enriched_diagnosis=true`.
- **T2#3 got a specific one.** With z.ai's circuit open, T2#2's helper call (14:40:28Z) went to gpt-5.4-mini ($0.0027,
  1.0 s), and its diagnosis named `head_test.cpp` lines 73, 77 and 87. T2#3 passed.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 9.
