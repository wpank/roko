+++
id = "bug-1e97dc"
kind = "bug"
title = "The audit strictness ladder steps up on sampling noise: it must ignore windows below a minimum effective sample"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-gate/audit", "roko-core/config"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "4783dbe91"
source = "status-quo-2026-10-15#finding-15"
anchors = ["crates/roko-gate/src/audit/feedback.rs::step", "crates/roko-gate/src/audit/feedback.rs::Signal", "crates/roko-gate/src/audit/estimate.rs", "crates/roko-core/src/config/audit.rs::AuditConfig"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["q-394b7f", "gap-d76b1b", "q-c02c36"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_holds_below_the_minimum_effective_sample' crates/roko-gate/ && cargo test -p roko-gate ladder_holds_below_the_minimum_effective_sample"

[[verify]]
command = "grep -rqw 'fn ladder_steps_up_below_the_floor_on_clear_evidence' crates/roko-gate/ && cargo test -p roko-gate ladder_steps_up_below_the_floor_on_clear_evidence"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T15:07:07Z"
commit = "4783dbe91"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:06:53Z"
forced = false
evidence = "crates/roko-gate/src/audit/feedback.rs (4783dbe91), per the q-394b7f decision: step() steps a task type up only when LCB95 > the limit at any n_eff, or when n_eff >= min_n_eff(max) = ceil(Z95^2*(1-max)/max) (73 for theta_max 0.05, 189 for gamma_max 0.02) and UCB95 > the limit; otherwise the level holds; step-down is unchanged. Signal now carries a Reading (LCB95, UCB95, Kish n_eff) per rate, and the audit.policy_change reason names the clause. No new [audit] key (the minimum is derived from theta_max/gamma_max) and no window pooling (the decision: pooled prior not now), so the Plan's config key and pooled case were dropped. Tests: ladder_holds_below_the_minimum_effective_sample (20 clean audited units hold window after window, also through close_due_windows; 1-2 false greens or 1 gaming finding in 20 hold; one false green holds at n_eff 72 and steps at 73; a window below the minimum never counts as quiet) and ladder_steps_up_below_the_floor_on_clear_evidence (8 false greens in 20 step once; 3 false greens or 2 gaming findings in 20 step). Both [[verify]] commands passed (1 passed each) at base 5e32264ce plus this change; the branch then fast-forwarded to 16c73f352, whose new commits touch only roko-agent, roko-serve and work items. cargo test -p roko-gate --lib audit:: 18 passed; cargo +nightly fmt --all and cargo clippy -p roko-gate --no-deps -- -D warnings clean. gap-d76b1b (audits on at 10%) can now go ahead."
+++

## Problem

DP3's strictness ladder steps a task type up on sampling noise. `step` (`crates/roko-gate/src/audit/feedback.rs:194`)
moves a rung deeper whenever a window's Wilson UCB95(θ) exceeds `theta_max` (0.05, `crates/roko-core/src/config/
audit.rs:79`), and `Signal` (:71) carries only the two UCBs, no sample size. With zero false greens, Wilson's UCB95 is
about 3.84 / (n + 3.84), which stays above 0.05 until roughly 73 audited units. At the defaults (`rho = 0.10`,
`window_units = 200`, `audit.rs:73`, `:77`) a window audits about 20, so every task type with a known label climbs one
level per window toward V4, and stepping down needs UCB95 < θ_max/2 for `QUIET_WINDOWS` windows in a row, which a
20-unit window can never show. At V4 every green attempt gets A2 re-runs, a B1 suite and B2 mutants: CPU and model spend
with no evidence behind them (q-394b7f; A08 §4 surprise 2).

## Why it matters

Goal `cybernetic` (S05 §4.6, DP3). NEW-FINDINGS finding 15. Decision 7103 (audits on at 10% by default, gap-d76b1b)
may only ship after this, per Will's decision 2 (2026-10-15). Stage 4 of the epic: the precondition for enforcing
anything with audits.

## Where

- `crates/roko-gate/src/audit/feedback.rs::step`, `Signal` (`Signal::of` from a `GroupEstimate`), `Ladder`.
- `crates/roko-gate/src/audit/estimate.rs` (Wilson at Kish n_eff; the estimate's `n_eff`).
- `crates/roko-core/src/config/audit.rs::AuditConfig` (`theta_max`, `rho`, `window_units`; add the floor here, with
  validation).

## Current state

Confirmed at HEAD: no n_eff floor and no pooled prior in the step logic. q-394b7f asks Will which shape to take; agent T1
is deciding it on 2026-10-15. This item is written for the floor; if the answer is the pooled prior, rewrite the Plan
and keep the tests' intent (no step on noise, a step on clear evidence).

## Plan

1. Carry the window estimate's `n_eff` (and LCB95(θ)) in `Signal`.
2. Add `[audit] min_n_eff`, defaulting to the n at which a zero-false-green Wilson UCB95 falls below `theta_max` (about
   73 at 0.05; compute it from `theta_max` or validate the pair).
3. In `step`, a window below the floor neither steps up on its UCB nor counts as quiet. Below the floor, step up only
   when LCB95(θ) > θ_max (clear evidence of false greens even at small n). Pool a task type's consecutive small windows
   until their combined n_eff reaches the floor, then judge the pooled estimate as today.
4. Tests: `ladder_holds_below_the_minimum_effective_sample` (20 audited units, 0 false greens: no step) and
   `ladder_steps_up_below_the_floor_on_clear_evidence` (20 units, 8 false greens: one step), plus a pooled case.

## Done when

Both `[[verify]]` commands pass. Then gap-d76b1b can flip `[audit] enabled`.

## Notes

- q-c02c36: a clippy-dirty crate can never step down; separate item.
- Do not change `eps_floor` (locked at 0.05, 7113) or ρ.

## Decision (q-394b7f, 2026-10-15)

Will delegated q-394b7f to the coordinator's recommendation (decision 9 in tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md); the decision taken: A minimum n_eff floor, derived per limit as ceil(Z95^2 * (1 - max) / max): 73 for theta_max = 0.05 and 189 for gamma_max = 0.02 at the defaults (no new config key). The ladder steps a task type up when LCB95(x) > x_max at any n_eff (evidence), or when n_eff(x) >= the floor and UCB95(x) > x_max (today's rule); below the floor with no evidence it holds the level. Step-down is unchanged (two windows with UCB95(theta) < theta_max/2, so n_eff >= 150). Pooled prior: not now.

Why: Will's decision 2 already picked a minimum-sample floor; this fixes its value and use. step() trips on UCB alone (crates/roko-gate/src/audit/feedback.rs:195-197); with zero observed events the Wilson UCB is Z95^2/(n+Z95^2) (Z95 at crates/roko-gate/src/audit/estimate.rs:29), above 0.05 below n_eff 73 and above 0.02 below 189, while a default window audits about 20 units across all task types (crates/roko-core/src/config/audit.rs:73,77-80). A pure floor would freeze the ladder at the defaults (n_eff never reaches 73 per task type), which raises the false-green risk; the LCB clause keeps escalation on real evidence (3 false greens or 2 gaming events in 20 audited units). Deriving the floor from theta_max/gamma_max is the cheapest reversible change; the LCB and n_eff are already computed (estimate.rs:132-142, window.rs:230-232).
