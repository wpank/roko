+++
id = "q-e916e4"
kind = "question"
title = "The audit strictness ladder never steps down at the default window size: a quiet window needs n_eff near 150, a default window audits about 20"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-gate/audit"]
created = 2026-10-16
updated = 2026-10-18
last_verified = 2026-10-16
source = "bug-1e97dc worker 2026-10-16"
discovered_from = "bug-1e97dc"
anchors = ["crates/roko-gate/src/audit/feedback.rs::step", "crates/roko-core/src/config/audit.rs::AuditConfig"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-1e97dc", "q-394b7f", "gap-d76b1b", "q-c02c36"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_steps_down_once_the_pooled_run_is_quiet' crates/roko-gate/ && cargo test -p roko-gate --lib ladder_steps_down_once_the_pooled_run_is_quiet"
+++

## Problem

DP3's strictness ladder steps a task type down one level only after two windows in a row with UCB95(θ) < θ_max/2
(`crates/roko-gate/src/audit/feedback.rs::step`, `QUIET_WINDOWS = 2`; left unchanged by bug-1e97dc, per q-394b7f).
With no events, Wilson's UCB95 is z²/(n_eff + z²), so a quiet window needs n_eff above 149.8 at θ_max = 0.05. A
window holds `window_units = 200` green units or 24 hours, whichever comes first, and at the default ρ = 0.10 it
audits about 20 units, fewer still in θ's sample after phase B (`phase_b_rate = 0.5`). Even ρ = 0.5, the cap, audits
only about 100. So at the defaults a task type's level can go up but never comes back down. Since bug-1e97dc, a
level goes up on evidence (LCB95 above the limit: 3 false greens or 2 gaming findings in 20 audited units).

A one-way ratchet keeps every rare evidence window. The chance that the LCB95 clause fires at least once in 50
windows (binomial, n audited units per window) is:

- at θ = 0.02 (under half of θ_max): 0.30 at n = 20, 0.56 at n = 10;
- at γ = 0.01 (half of γ_max): 0.57 at n = 20, 0.19 at n = 10.

Each firing costs one level for good: V2 adds A2 re-runs, V3 a B1 hidden suite (model spend), and V4 B2 mutants.

## Why it matters

Goal `cybernetic`, epic spec-7a71d6 stage 4: gap-d76b1b turns audits on at 10% by default. bug-1e97dc stopped the
ladder climbing on noise. This is the remaining way it over-escalates at the defaults, more slowly but with no way
back. It is the same family as q-c02c36, where a clippy-dirty crate never steps down.

## Where

- `crates/roko-gate/src/audit/feedback.rs::step`: the quiet run (`Rung.quiet`, `QUIET_WINDOWS`) and the step down.
- `crates/roko-core/src/config/audit.rs::AuditConfig`: `window_units` 200, `window_hours` 24, `rho` 0.10,
  `phase_b_rate` 0.5, `theta_max` 0.05.

## Current state

At 4783dbe91 (bug-1e97dc), a rate steps its task type up when LCB95 is above its limit at any n_eff, or when n_eff
reaches `min_n_eff` (73 for θ, 189 for γ) and UCB95 is above the limit. Step-down is as before. Nothing pools quiet
windows or decays a level, so the only way down at the defaults is editing the vault's `ladder.json` by hand.

## Why this needs Will

Each option trades verify spend on healthy task types against how fast a level returns once a real problem is fixed:

1. Keep the ratchet: the operator lowers levels by hand.
2. Pool the quiet run: add up n_eff and events over the consecutive windows since the last step, and step down
   once the pooled UCB95(θ) < θ_max/2 (about 150 audited units, 8 to 15 default windows).
3. Step down after K windows in a row with no evidence (no LCB95 above either limit), for a fixed K such as 10.
4. Close windows per task type by n_eff rather than by green units.

## Notes

- q-394b7f's decision reads "Step-down is unchanged (two windows with UCB95(theta) < theta_max/2, so n_eff >= 150)".
  It saw the n_eff requirement. This item records that a default window never meets it.
- Do not change `eps_floor` (locked at 0.05, 7113) or ρ.
- Found while doing bug-1e97dc (2026-10-16).

## Decision (2026-10-18, Will)

Option 2: pool the quiet run. Add up n_eff and events over the consecutive windows since the last step, and step down once the pooled UCB95(θ) < θ_max/2 (about 150 audited units, 8 to 15 default windows). gap-d76b1b (audits on at 10%, decision 7103) may follow once this lands.

## Done when

- `Rung.run` pools each rate's n_eff and events (θ̂_H·n_eff) over the windows since the task type's last step, and the
  ladder steps down one once the pooled UCB95(θ) < θ_max/2 and, by the same rule (the coordinator's 2026-10-18 brief:
  "apply the same rule to γ"), the pooled UCB95(γ) < γ_max/2. A clean run is quiet at n_eff 150 for θ and 381 for γ,
  so γ sets the pace at the defaults: about 20 to 40 default windows rather than 8 to 15.
- bug-1e97dc's floor and step-up rules hold, and a window that steps up (at V4 too) starts a fresh run.
- The `[[verify]]` test passes: clean windows of 20 step down at the 20th, four false greens in the run push it to the
  21st and one gaming finding to the 29th, and the run lasts from one `close_due_windows` to the next.
