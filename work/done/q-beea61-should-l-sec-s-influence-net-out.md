+++
id = "q-beea61"
kind = "question"
title = "Should L-sec's influence net out an A/A floor? The section bandit is a random policy, so the floor would read it inert by construction"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-16
updated = 2026-10-18
last_verified = 2026-10-18
last_verified_rev = "b2496bc06"
source = "gap-6f31c3"
discovered_from = "gap-6f31c3 (worker, 2026-10-16): measuring L-sec from its sections rows"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::content_opportunity", "crates/roko-learn/src/telemetry/records.rs::ContentProposals", "crates/roko-cli/src/dispatch/prompt_builder.rs::draw_sections", "crates/roko-learn/src/loop_audit/state.rs::Auditor::rule"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-6f31c3", "spec-7a71d6", "gap-03fcd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn l_sec_influence_reads_flat_and_learned_bandits_differently' crates/roko-learn/ && cargo test -p roko-learn --lib l_sec_influence_reads_flat_and_learned_bandits_differently"

[[verify]]
command = "grep -rqw 'fn sections_rows_log_the_bandits_exclusion_odds_on_both_arms' crates/roko-cli/ && cargo test -p roko-cli --lib sections_rows_log_the_bandits_exclusion_odds_on_both_arms"

[closed]
at = 2026-10-18
at_ts = "2026-10-18T18:22:33Z"
commit = "b2496bc06"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-18T17:10:11Z"
forced = false
evidence = "b2496bc06, Will's option (b): L-sec's ι is read from the section bandit's exclusion odds against the floor policy (p_ex at SECTION_EXCLUSION_FLOOR on every drawn section), not from single draws. Writer: decision_log.rs section_proposals logs proposals.exclusion (records.rs ExclusionProposals::of_draws: each drawn section's p_ex, and the floor) on both arms, from the draws the prompt applied (learned arm) or only logged (the arm that runs π⁰). Census: census.rs gives a sections row's included sets to ε and the arm counts, and its ι is the ExclusionOdds distance (exposure.rs: the mean gap in odds) between the bandit's odds and the floor policy's, with no A/A floor (the odds never fall below the floor, so the distance is linear in them); rows from before the odds count for ε alone; CENSUS_STATE_SCHEMA 6 refolds kept censuses; state.rs's inert rule counts ι's own opportunities toward N_ι; loops.toml's L-sec note says how ι is read. Checks at b2496bc06: both [[verify]] commands pass, and both are red on main (neither test exists there). l_sec_influence_reads_flat_and_learned_bandits_differently, 400 chains each at α/13: flat posteriors (Beta(11,11) on both arms, odds at the 0.2 early cap) read ι_net 0.150 [0.114, 0.176]; a bandit that learned every section helps (odds at the 0.05 floor) reads 0.000 [-0.032, 0.032], and the audit tick flags it inert while the flat one stays on probation; on single draws netted of a second, independently seeded draw (option a) both read below 0 (-0.092 and -0.041). roko-learn lib 1384/1384; roko-cli decision_log 21/21 plus the loop_audit and learn_loops lib tests; loop_audit_census_run (real binary, 200 chains, fake provider) asserts every L-sec opportunity row logs the odds on both arms, with ι_net between 0 and the cap's excess; nightly fmt clean; clippy -p roko-learn -p roko-cli --no-deps -D warnings clean. Not run: E1 (loop_audit_faults, fault-injection build). Filed spec-dc29d3 (S03 §4.2 and §4.4 text)."
+++

## Problem

S03 §4.4 nets every loop's influence of an A/A floor: I^AA = d(A^L, A^L'), from a second independent draw of the
learned policy ("the A/A rerun floor is mandatory"; 0 only for a deterministic reader). The section bandit (L-sec) is
a random policy: each droppable section is left out with p_ex in [0.05, 0.5]
(`crates/roko-learn/src/section_effect.rs::SectionBandit::decide`). Since gap-6f31c3 the census measures L-sec from
its sections rows, but the rows log one draw only (`ContentProposals` has no A/A field, unlike a route row's
`proposals.aa`), so `census.rs::content_opportunity` sets `learned_again: None` and L-sec's floor is 0.

So L-sec's ι_net is mostly its own exploration. With the floor it would be the opposite: for one section left out at
p against π⁰'s "keep", d(L, D) is about p while d(L, L') is about 2p(1 − p), which is larger for every p below 0.5.
Netted of the floor, L-sec's ι_net is at or below 0 whatever its posteriors say, so the state machine would flag it
`inert` by construction, and with enforce on, an inert loop runs π⁰ (no section is ever left out).

## Why it matters

Goal `cybernetic` (S03 §4.4, §4.6). `inert` and, after gap-03fcd0, β's guard 2 rest on ι_net. Today L-sec's ι is
overstated; with the floor S03 prescribes, a loop whose job is to explore can never show net influence. Someone has
to decide which reading S03 means for an exploration policy before enforce is turned on (D10).

## Where

- `crates/roko-learn/src/loop_audit/census.rs::content_opportunity` (`learned_again: None` for content rows).
- `crates/roko-learn/src/telemetry/records.rs::ContentProposals` (no A/A draw).
- `crates/roko-cli/src/dispatch/prompt_builder.rs::draw_sections` (one draw per prompt).
- `crates/roko-learn/src/loop_audit/state.rs::Auditor::rule` (the `inert` rule).
- S03 §4.4 (`tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md`).

## Current state

gap-6f31c3 gave sections rows S03's fields: on both arms, the included sets under the bandit and under π⁰. No second
draw is logged, so L-sec's floor is 0.

## Plan

1. Decide (Will, or the S03 author): (a) log a second, independently seeded bandit draw as the A/A proposal and accept
   that L-sec reads `inert` while its p_ex stay below 0.5; (b) measure L-sec's influence on its exclusion
   probabilities against the floor policy (p_ex at the floor everywhere) instead of on single draws; or (c) exempt
   exploration loops from the floor and say so in S03.
2. Implement the choice in the writer and the census, with a test that a bandit with flat posteriors and one with a
   learned preference read differently.

## Done when

The decision is recorded here, and both `[[verify]]` commands pass:

- the census (`census.rs`) reads L-sec's ι from the odds the bandit left each droppable section out at, against the
  floor policy (every section at `SECTION_EXCLUSION_FLOOR`), so that over the same rows a bandit with flat posteriors
  (odds at the cap) and one that learned every section helps (odds at the floor) read differently: ι_net at the
  cap's excess and 0, the audit tick reading only the second `inert`
  (`l_sec_influence_reads_flat_and_learned_bandits_differently`);
- the writer (`decision_log.rs::section_proposals`) logs those odds and the floor on every sections row, on both
  arms (`sections_rows_log_the_bandits_exclusion_odds_on_both_arms`).

## Notes

Measurement only; enforce stays off (D10). Found while closing gap-6f31c3.

## Decision (2026-10-18, Will)

Option (b): measure L-sec's influence on its exclusion probabilities against the floor policy (p_ex at the floor everywhere), instead of on single draws. Test that a bandit with flat posteriors and one with a learned preference read differently.
