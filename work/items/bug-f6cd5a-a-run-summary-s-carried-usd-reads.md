+++
id = "bug-f6cd5a"
kind = "bug"
title = "A run summary's carried_usd reads -0.0 on a run that carried nothing: an empty f64 sum starts from -0.0, so a raw JSON reader sees a negative zero spend"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/runs"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery, the P-51/P-53 scratch serve (2026-10-26 22:18), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, roko-main-2ca53cb61)"
anchors = ["crates/roko-serve/src/routes/runs.rs::run_summary"]
links = { depends_on = [], blocks = [], related = ["bug-b7f6a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumed_runs_summary_says_what_its_plans_carried' crates/roko-serve/src && ! grep -q 'carried.values().sum::<f64>()' crates/roko-serve/src/routes/runs.rs && cargo test -p roko-serve --lib -- a_resumed_runs_summary_says_what_its_plans_carried"
+++

## Problem

A run summary's `carried_usd` (bug-b7f6a8) reads `-0.0` on a run that carried nothing. `run_summary` sums the
latest `budget_carried` value of each plan with `carried.values().sum::<f64>()`, and Rust's float `Sum` starts from
−0.0. So with no `budget_carried` event, `GET /api/runs/{id}/summary` and MCP `run_status` answer
`"carried_usd": -0.0`.
- **Seen 2026-10-26 22:18:** on the P-51/P-53 scratch serve (roko-main-2ca53cb61), run 6e62dcbd (failed, nothing
  carried) and run 0a7e3e7b (interrupted).

## Why it matters

The value is right (−0.0 == 0.0), but a raw JSON reader sees a sign: Python's `f"{-0.0:.2f}"` prints "-0.00".
- The Hermes plugin is safe: rundata.py takes carried only when it is `> 0` or truthy, and views.py and tools.py only
  test `is not None`.
- Any other client that formats the number shows a negative zero spend.

## Where

`crates/roko-serve/src/routes/runs.rs::run_summary`: the `"carried_usd": carried.values().sum::<f64>()` line.

## Plan

Fold from 0.0 (`carried.values().fold(0.0, |total, usd| total + usd)`), so an empty sum is +0.0. Extend
`a_resumed_runs_summary_says_what_its_plans_carried`:
- a run that carried nothing reports `0.0`;
- the serialized summary holds `"carried_usd":0.0`, with no sign.

## Done when

The test passes, and a run that carried nothing serializes `"carried_usd":0.0`.
