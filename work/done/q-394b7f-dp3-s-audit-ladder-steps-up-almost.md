+++
id = "q-394b7f"
kind = "question"
title = "DP3's audit ladder steps up almost everywhere at low n_eff: a minimum n_eff floor, or a pooled prior?"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-gate/audit"]
created = 2026-10-03
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b883db4e6"
source = "wave-9 follow-up reports 2026-10-03 (PK60 gap-940e44)"
discovered_from = "gap-940e44"
anchors = ["crates/roko-gate/src/audit/feedback.rs", "crates/roko-gate/src/audit/estimate.rs", "crates/roko-core/src/config/audit.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:30:09Z"
commit = "b883db4e6"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = true
evidence = "2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): A minimum n_eff floor, derived per limit as ceil(Z95^2 * (1 - max) / max): 73 for theta_max = 0.05 and 189 for gamma_max = 0.02 at the defaults (no new config key). The ladder steps a task type up when LCB95(x) > x_max at any n_eff (evidence), or when n_eff(x) >= the floor and UCB95(x) > x_max (today's rule); below the floor with no evidence it holds the level. Step-down is unchanged (two windows with UCB95(theta) < theta_max/2, so n_eff >= 150). Pooled prior: not now. Why: Will's decision 2 already picked a minimum-sample floor; this fixes its value and use. step() trips on UCB alone (crates/roko-gate/src/audit/feedback.rs:195-197); with zero observed events the Wilson UCB is Z95^2/(n+Z95^2) (Z95 at crates/roko-gate/src/audit/estimate.rs:29), above 0.05 below n_eff 73 and above 0.02 below 189, while a default window audits about 20 units across all task types (crates/roko-core/src/config/audit.rs:73,77-80). A pure floor would freeze the ladder at the defaults (n_eff never reaches 73 per task type), which raises the false-green risk; the LCB clause keeps escalation on real evidence (3 false greens or 2 gaming events in 20 audited units). Deriving the floor from theta_max/gamma_max is the cheapest reversible change; the LCB and n_eff are already computed (estimate.rs:132-142, window.rs:230-232). Follow-up: bug-1e97dc. If vetoed: Audits stay off: decision 2 holds gap-d76b1b until a floor exists. With a pooled prior instead, the follow-up becomes a larger statistics item; with no fix, every task type climbs to V4 within a few windows once audits run."
+++

## Problem

DP3's strictness ladder (`crates/roko-gate/src/audit/feedback.rs`, module doc comment lines 7-8: steps up when
"UCB95(θ) > θ_max or UCB95(γ) > γ_max," down only after two windows in a row with UCB95(θ) < θ_max/2) has no
minimum effective-sample-size floor and no pooled prior. At the default `theta_max = 0.05`
(`crates/roko-core/src/config/audit.rs:79`), the Wilson UCB at an observed rate of p = 0 stays above 0.05 until
roughly 73 effective audited units accumulate in a window — below that, a task type with *zero* observed false
greens still trips the step-up condition purely from small-sample uncertainty, not evidence of a real problem.

At the defaults (`rho = 0.10`, `window_units = 200`, `audit.rs:73,77`), a 200-unit window audits only about 20
units — well under the ~73 needed — so in practice the ladder climbs to the deepest verify checks for almost
every task type, and only steps back down after roughly 150 quiet units accumulate in a window (since stepping
down needs UCB95 < θ_max/2, an even tighter bound, over two consecutive windows).

## Why it matters

Goal: cybernetic, M4 deep audits (S05). If the ladder is meant to reserve deep verification for task types that
show real evidence of trouble, as designed, it currently can't tell "genuinely risky" apart from "not enough
audited units yet" — it treats low-n uncertainty as risk, running expensive deep checks on types this scanty
evidence can neither confirm nor rule out as a false-green source.

## Where

- `crates/roko-gate/src/audit/feedback.rs` (the step-up/step-down logic, θ_max/γ_max comparison).
- `crates/roko-gate/src/audit/estimate.rs` (Wilson at Kish n_eff, the CI method this feeds).
- `crates/roko-core/src/config/audit.rs` (the defaults: `rho = 0.10`, `window_units = 200`, `theta_max = 0.05`).

## Current state

Confirmed as described; no n_eff floor or pooled-prior mechanism exists in the step-up/down logic today.

## Why this needs Will

Two shapes of fix, each a real design choice:

1. **A minimum effective n before the ladder is allowed to step up at all** (e.g. require n_eff ≥ some floor,
   below which the ladder stays at its current level regardless of UCB95) — simple, but means a genuinely risky
   task type with few audited units yet goes unescalated until enough accumulate.
2. **A pooled prior** (borrow strength across task types, or across the whole workspace, to stabilize the
   estimate at low n) — avoids the above trade-off but is more statistical machinery to design and verify.

Both change DP3's actual behavior in ways that trade off false escalations against slow-to-detect real ones;
not a fact to verify in code.

## Notes

- Discovered during PK60's work (gap-940e44, done).
- If Will decides on an approach, it becomes a regular `gap`/`bug` item with its own implementation and verify.
