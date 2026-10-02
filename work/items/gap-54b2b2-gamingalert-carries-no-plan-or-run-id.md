+++
id = "gap-54b2b2"
kind = "gap"
title = "GamingAlert carries no plan or run id, so roko diagnose attributes alerts by model and timestamp alone"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/gate-gaming", "roko-cli/diagnose"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK08)"
discovered_from = "gap-a0043b (task 2125 built run_gaming_alerts with this approximation already named in its own doc comment, no follow-up filed)"
anchors = ["crates/roko-learn/src/gate_gaming.rs::GamingAlert", "crates/roko-learn/src/gate_gaming.rs::detect", "crates/roko-cli/src/commands/diagnose.rs::run_gaming_alerts"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gaming_alerts_are_attributed_by_plan_id_not_model_alone' crates/roko-cli/ && cargo test -p roko-cli gaming_alerts_are_attributed_by_plan_id_not_model_alone"
+++

## Problem

`GamingAlert` (`crates/roko-learn/src/gate_gaming.rs:65-80`) has no plan or run id:

```rust
pub struct GamingAlert {
    pub model_slug: String,
    pub pass_rate_delta: f64,
    pub quality_delta: f64,
    pub first_half_pass_rate: f64,
    pub second_half_pass_rate: f64,
    pub first_half_quality: f64,
    pub second_half_quality: f64,
    pub timestamp: DateTime<Utc>,
}
```

`roko diagnose`'s `run_gaming_alerts` (`crates/roko-cli/src/commands/diagnose.rs:519-540`, added by backlog task
2125, part of PK08's own open package `gap-a0043b`) therefore can only approximate which run an alert belongs to:
its own doc comment says so outright — "An alert names a model, not a plan, so this is as near as the log can
say" — and the filter is exactly that: an alert in `.roko/learn/gate-gaming-alerts.jsonl` is attributed to the run
being diagnosed when its `model_slug` is one of the models *any* attempt of that run used, and its `timestamp` is
at or after the run's start (`since_ms`). Two runs that overlap in time and share a model (a common case: the same
cheap-model pool, two plans running concurrently, or a resumed run started shortly after a previous one) will both
match the same alert.

## Why it matters

Goal: truth (diagnostic correctness). `roko diagnose <plan>` is meant to explain *this* plan's gate-gaming
signal. Under concurrent or closely-spaced runs sharing a model, an operator can be shown a gaming alert that
actually came from a different plan's attempts, or have a real alert for their own plan masked among reported
false positives attributed to it. This degrades trust in the one diagnostic surface gate-gaming has
(`roko diagnose`'s `gate_gaming_alerts` field, `commands/diagnose.rs:100`).

## Where

- `crates/roko-learn/src/gate_gaming.rs::GamingAlert` (struct definition, :65; `summary()`, :85).
- Whatever writes `.roko/learn/gate-gaming-alerts.jsonl` (the detector that constructs `GamingAlert` values — find
  its call site; it has the plan/run context available at detection time and simply doesn't carry it through).
- `crates/roko-cli/src/commands/diagnose.rs::run_gaming_alerts` (:519-540), the model+timestamp approximation this
  item makes exact.

## Current state

Alerts are keyed by `model_slug` + `timestamp` only. The detector that produces `GamingAlert`s runs with plan/run
context in scope (it has to, to read that model's recent gate-pass/quality history for one run), so the data to
add is available; it is just not threaded into the struct.

## Plan

1. Add `plan_id: String` (and `run_id: String`, if the detector's scope spans more than one run) to `GamingAlert`,
   filled at construction from the same context the detector already has.
2. Update `summary()` to include it.
3. Change `run_gaming_alerts` to filter by exact `plan_id` (and `run_id` if added) first, falling back to the
   model+timestamp heuristic only for alerts written before this change (a migration/compat path, or simply accept
   that pre-existing alerts in `gate-gaming-alerts.jsonl` stay approximate).
4. Add a regression test: two concurrent runs/plans sharing a model each get only their own alerts back from
   `run_gaming_alerts`.

## Done when

- `run_gaming_alerts` (or its successor) attributes an alert to the run it actually came from by id, not by
  model+timestamp alone, for every alert written after this change.
- The `[[verify]]` command passes.

## Notes

- Keep the model+timestamp fallback for alerts already on disk in existing workspaces (`gate-gaming-alerts.jsonl`
  is append-only and not rewritten retroactively elsewhere in this codebase's conventions).
- Related: gap-a0043b (PK08's own open package; task 2125 built `run_gaming_alerts` with this approximation
  already documented in its own doc comment, but filed no follow-up to make it exact).
