+++
id = "gap-3cd890"
kind = "gap"
title = "Audit tilt's inclusion_probability always gets risk=None, so risk_fg never reaches the selection draw"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch", "roko-gate/audit"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-10 follow-up reports 2026-10-04 (PK66 gap-414e56)"
discovered_from = "gap-414e56"
anchors = ["crates/roko-cli/src/graph_task_dispatch/audit_select.rs", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime", "crates/roko-gate/src/audit/policy.rs::inclusion_probability"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tilted_selection_uses_the_chain_s_risk_fg' crates/roko-cli/ && cargo test -p roko-cli tilted_selection_uses_the_chain_s_risk_fg"
+++

## Problem

`crates/roko-gate/src/audit/policy.rs::inclusion_probability(params, risk: Option<f64>, mean_risk: Option<f64>)`
takes a false-green-risk term specifically so the audit tilt can weight a selection toward riskier units — but
its only call site, `crates/roko-cli/src/graph_task_dispatch/audit_select.rs:344`, passes `(¶ms_for(...), None,
None)`: both `risk` and `mean_risk` are always `None`, so the tilt never actually tilts on risk.

The signal to feed it already exists: `SelfModelRuntime::risk_fg(chain_key)`
(`crates/roko-cli/src/graph_task_dispatch/self_model.rs:520-523`), "P(false green) of the chain's last pass that
stood, exported as `risk_fg` for S05's audit tilt" — its own doc comment says this is exactly what it's for.

## Why it matters

Goal: cybernetic, M4 deep audits / M3 self-model interface (S05). Once S05's tilted-selection rate is turned on
(it's presumably off or untilted by default today, given `risk` is always `None`), the audit lottery should
preferentially select units the self-model judges more likely to be false greens — that's the whole point of a
risk-weighted tilt. With `risk_fg` never threaded through, the tilt (when enabled) draws uniformly regardless of
M3's own risk estimate, missing the targeting it's designed for.

## Where

- `crates/roko-cli/src/graph_task_dispatch/audit_select.rs` (the call site to fix).
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime::risk_fg` (the value to thread
  through).
- `crates/roko-gate/src/audit/policy.rs::inclusion_probability` (the function that already accepts it).

## Current state

Confirmed: the plumbing is in place on both ends; only the call site itself doesn't connect them.

## Plan

1. At the `inclusion_probability` call site in `audit_select.rs`, pass the audited unit's chain's `risk_fg` (and
   whatever this run/window's `mean_risk` should be) instead of `None, None`.
2. Add a regression test: with a chain whose `risk_fg` is set, the audit tilt's inclusion probability for that
   chain differs from the untilted baseline.

## Done when

- `inclusion_probability` is called with the real `risk_fg` once it's available for a chain, not `None`.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK66's work (gap-414e56, done).
