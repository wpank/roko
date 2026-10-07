+++
id = "gap-a342a2"
kind = "gap"
title = "Audit tilt's lambda is never computed; policy::tilt has no caller and InclusionParams.lam stays 0"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "2539f2c77"
source = "wave-14 follow-up reports 2026-10-04 (gap-3cd890, gate 14b)"
discovered_from = "gap-3cd890 (in flight, work/gap-3cd890; its risk/mean_risk fix doesn't touch lambda)"
anchors = ["crates/roko-gate/src/audit/policy.rs::tilt", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_tilt_lambda_rises_with_measured_false_green_ece' crates/ && cargo test -p roko-cli audit_tilt_lambda_rises_with_measured_false_green_ece"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T08:38:27Z"
commit = "2539f2c77"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T06:54:01Z"
forced = false
evidence = "Gate 15b (merged 2539f2c77): verify audit_tilt_lambda_rises_with_measured_false_green_ece passes. The lottery computes λ at open from the vault ledger: audit.selection logs risk_r, each vs.label with a known VS scores its unit's risk against false green with weight 1/π, and the IPW-ECE (10 equal-mass bins, latest 100 labels; λ = 0 below 50) goes to policy::tilt, per S05 §4.2 (λ falls as ECE grows). Window choice: 100 labels (S04's calibration window)."
+++

## Problem

Nothing computes the audit tilt's λ, so even once the risk/mean_risk arguments are threaded
through (`gap-3cd890`, in flight on `work/gap-3cd890`, not yet merged), the tilt still has no
effect: `InclusionParams::default()` sets `lam: 0.0` (`crates/roko-gate/src/audit/policy.rs:131-136`),
and `inclusion_probability`'s risk-weighted branch only applies `if lam > 0.0 && mean > 0.0`
(`policy.rs:211-212`) — with `lam` always 0, π is always ρ (the untilted base rate) in
production, regardless of what risk value is passed in. `policy::tilt`
(`policy.rs:168-179`), the function that computes a real λ — "λ_max·max(0, 1 − ECE/ECE_ref), and
0 without an ECE or below `MIN_TILT_LABELS` [= 50] labels" — has zero call sites anywhere in
`crates/` (confirmed by grepping every `.rs` file on `main` and on `work/gap-3cd890`); its own
unit tests are the only thing that ever calls it.

The ECE `tilt` needs is specifically M3's calibration of P(false green) on audited labels — not
the ECE the self-model's existing calibration gate already tracks. `CalibrationGate::evaluate`
(`crates/roko-learn/src/self_model/gate.rs:164-170`) computes `ece(&scored, ECE_BINS)` over a
window built from `WindowOutcome { p: candidate.p_gate, y: unit.label.y_gate == Some(true), ...
}` (`crates/roko-cli/src/graph_task_dispatch/self_model.rs:320-329`) — the gate-*pass* head's
calibration. `p_fg` (false-green risk) is tracked separately, but only fed to an LCB update
(`record_outcome`, `self_model.rs:355-358`: `self.lcb.lock().update(None, outcome.y, p_fg)`),
never into its own calibration window against a ground-truth false-green label (which would
need S05's late `vs.label`). So no ECE for P(false green) exists anywhere to pass to `tilt`,
even once a caller is written.

## Why it matters

Goal: cybernetic, M4 deep audits / M3 self-model interface (S05). S05's whole design for the
audit tilt is to ramp λ up as the self-model's false-green predictions prove miscalibrated
(more τ-worthy risk signal → more aggressive tilting) and ramp it down as they prove reliable.
With λ pinned at 0, the tilt mechanism (even after `gap-3cd890` lands) stays permanently
inactive — audits keep drawing uniformly regardless of any risk signal, which is the same
practical outcome as if neither fix existed.

## Where

- `crates/roko-gate/src/audit/policy.rs::tilt`, `::InclusionParams`, `::MIN_TILT_LABELS` (the
  λ-computing function, with no caller).
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime::record_outcome`,
  `::settle` (where a false-green calibration window would need to be built and scored).
- `crates/roko-learn/src/self_model/gate.rs::CalibrationGate` (the existing gate-pass
  calibration; a sibling for P(false green) does not exist).

## Current state

`tilt()` and `InclusionParams.lam` are implemented and unit-tested in isolation. No production
code builds the P(false green) ECE this function needs, or calls it with one, or feeds the
result into `inclusion_probability`'s `lam` parameter. `gap-3cd890`'s in-flight fix (risk/
mean_risk threading) is a different, narrower piece of the same mechanism and doesn't touch
this.

## Plan

1. Build a calibration window for P(false green) specifically: forecast `p_fg` against the
   eventual ground truth (S05's late `vs.label`, once an audited unit settles), parallel to
   the existing gate-pass `CalibrationWindow`.
2. Compute that window's ECE with `MIN_TILT_LABELS` (50) audited labels, and call
   `policy::tilt(ece, n_labels, lambda_max, ece_ref)` to get a real λ.
3. Thread the result into `InclusionParams.lam` at the `inclusion_probability` call site
   (`audit_select.rs`, once `gap-3cd890` lands), replacing the default 0.0.
4. Add a regression test: with ≥50 audited labels and a nonzero measured ECE, `lam` is nonzero
   and the resulting π differs from the untilted ρ.

## Done when

- `policy::tilt` has a real caller, fed a real P(false green) ECE once ≥50 audited labels exist.
- `InclusionParams.lam` is nonzero in production once that threshold is met.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-3cd890, gate 14b not yet merged): confirmed at main HEAD
  `b42c34ff9` that `policy.rs` (unchanged by `work/gap-3cd890`'s diff) already has `tilt`/`lam`
  with zero callers there too — this gap exists independent of whether `gap-3cd890` has merged.
  Checked `gap-3cd890`'s own item text and branch diff (`audit_select.rs`, `labels.rs`,
  `worker.rs`, `attempt.rs`, `b1.rs`-`b3.rs`, `verify_depth.rs`): none mention `lam` or `tilt`,
  confirming this is genuinely outside its scope, not a duplicate.
