+++
id = "gap-74893d"
kind = "gap"
title = "The self-model's cascade start rule (cascade::start_rung) weighs cost only, though every forecast carries latency (Undertow phase 1: strong lat_q50 40-67 s, mid 418-695 s; measured mid tries of 7.5-46 min against a 44 s strong pass); design first, owner roko-speed"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "ad16cc844"
source = "round-v7-hermes-pilot#H-self-model"
discovered_from = "agent:hermes-pilot (round v7, self-model shadow review of Undertow phases 0-1)"
anchors = ["crates/roko-learn/src/self_model/cascade.rs::start_rung", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::decide"]
links = { depends_on = [], blocks = [], related = ["bug-a35286", "find-816b4b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_start_rule_weighs_time_on_phase_one_forecasts' crates/roko-learn/src && cargo test -p roko-learn --lib the_start_rule_weighs_time_on_phase_one_forecasts"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T18:21:34Z"
commit = "ad16cc844"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-20T17:30:46Z"
forced = false
evidence = "ad16cc844 (plus 67d5c926c, the fit's count). [self_model] start_rule = \"expected_total\", the new default, starts on the rung with the lowest expected total, cost plus λ·time, to a verified pass: cascade::lowest_total_start over total_to_vs, G_j = g_j + (1 − p_j)·G_{j+1} over the rungs K_max climbs reach, the highest retried until it passes (g/p), g = expected_total = E[c] + λ·E[t] from the forecast's log-normal means (policy::expected_latency), p the forecast P(VS); λ = time_value_usd_per_hour, default 3. start_rule = \"break_even\" restores S04 §4.4's rule exactly (the same start_rung(forecasts, BREAK_EVEN_MARGIN) call, raised to the 6101 B1 floor). Each chain's first roko.prediction/1 row logs both picks with their numbers, whichever rule routes (decision.start_rules: break_even {pick, bars}, expected_total {pick, usd_per_hour, totals_usd}; the lead's addition). The literal verify passes on the branch: the_start_rule_weighs_time_on_phase_one_forecasts (Undertow 0bed5c58 T3's logged forecasts: break_even and λ = 0 start on cheap, $3/h on strong at $0.99 against cheap's $1.08 and mid's $1.12). Also new and passing: start_rules_record_both_picks_and_their_numbers, a_starts_total_counts_the_rungs_its_climbs_reach, the_start_rule_setting_picks_the_start_and_both_picks_are_logged (CLI decide: expected_total starts strong, break_even cheap, both picks logged), the shadow test's rows carry start_rules, self_model_config_defaults_shadow (defaults, parse, validation). Full lib suites: roko-cli 3930 (7 ignored, the share test skipped), roko-core 2044, roko-learn 1389, roko-serve 1073; learn_self_model bin tests 5; nightly fmt and clippy -D warnings clean. Start-rung demo (rung_rerun.sh): before on a1d12fc1c all six pass conditions pass (-61% over tasks 6-20); after on ad16cc844 five pass and the cost drop fails (-22%). Not the rule: on all 50 first tries of the after run both rules picked the same rung except task 1 (no data, gate shut). The after run drew an exploration start on strong at task 8, which dropped the gate for tasks 9-12, and an audit at task 9 whose hidden suite failed (VS 0 at weight 20), which saturated the false-green head once warm (p_fg 1.0 for every arm from task 13, filed as bug-7683aa), so both rules fell back to strong. Design note: tmp/hermes-roko-v7/findings/roko-speed.md, Design note C; follow-up gap-98913f (after_failure)."
+++

## Problem

The cascade start rule, `cascade::start_rung`, picks the first rung j whose P(VS) LCB exceeds
`expected_cost(j) / expected_cost(j+1) + BREAK_EVEN_MARGIN` (0.05), else the last rung. It weighs cost only. Each
candidate's forecast carries latency (`lat_q50_s`, `lat_q90_s`), but the start rule never reads it.

Undertow's numbers, from round v7:
- In phase 1 the self-model forecast strong (gpt-5.4-mini) at lat_q50 40–67 s and mid (glm-4.7) at 418–695 s.
- Measured: mid tries of 7.5–46 min (T3 744 s and 461 s, T4 1,347 s, T2 2,781 s). They were 96–99% model time, with
  build and check at most 28 s of each, against a 44 s, $0.02 strong pass (T1).

Since bug-f0007b (7268ba404), a mid try stops at 900 s. Even so, the start rule cannot trade one minute on strong for
15–46 minutes on mid.

## Why it matters

The pitch is that cheap, quick models do as much as possible. A start rule that can't see time starts a slow model
whenever it is cheaper per success, however long it takes, and the person waits.

## Where

- `crates/roko-learn/src/self_model/cascade.rs`: `start_rung`, `cost_to_vs` and `BREAK_EVEN_MARGIN`.
- `CandidateForecast`: the latency quantiles.
- The active start path, `crates/roko-cli/src/graph_task_dispatch/self_model.rs`: `decide` and `active_start`.

## Plan

Owner: roko-speed (round v7). The design note is in `tmp/hermes-roko-v7/findings/roko-speed.md`, "Design note C"
(2026-10-20). The lead approved the objective: the exact cascade cost, with a price on time.

The rule it changes is S04 §4.4's start rule, as decision 6101 (B) adopted it: the first rung j with
p_vs_lcb(j) > c_j/c_{j+1} + m_b, else the last rung.

**The objective:** start on the rung with the lowest expected total, cost plus λ·time, to a verified pass.
- One try on rung k costs g_k = E[c_k] + λ·E[t_k], from the log-normal means of the forecast's q50 and q90.
- A task started on rung j climbs one rung per failure, at most K_max = 2 times. The highest rung it can reach is
  retried until it passes: G_j = g_j + (1 − p_j)·G_{j+1}, ending in g/p, with p the forecast P(VS).
- The lowest G_j wins, and ties go to the higher rung. With no finite G, it is the last rung.

**Config** (`[self_model]`):
- `start_rule = "expected_total"` (the default), or `"break_even"`, which restores today's rule exactly;
- `time_value_usd_per_hour` (λ, default 3). `0` weighs cost alone.

**Not in this item:** `after_failure` (6131) keeps E[c]/p, which is a follow-up. The replay keeps S04's rule, since
its matrix has no time.

## Done when

On Undertow's phase-1 forecasts for T3's first try (run `0bed5c58`), `break_even` and λ = 0 start on cheap. λ =
$3/hour starts on strong: $0.99 to a verified pass, against cheap's $1.08 and mid's $1.12. Checked by the test in
`[[verify]]`. The start-rung demo harness (round v6) passes the same conditions before and after, or the change is
explained.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/self-model-shadow-p0-p1.txt`. Phase 1's per-try times are in
  `tmp/hermes-roko-v7/findings/hermes-pilot.md`, section G (from `tmp/hermes-roko-v7/pilot/tooling/trytimes.py`).
- Both linked items bear on this: strong's P(VS) is understated (bug-a35286), and the fallback is the open finding
  (find-816b4b).
