+++
id = "gap-603aa4"
kind = "gap"
title = "roko-serve's estimate_cost_usd has no callers"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-serve/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-serve/src/bench.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'fn estimate_cost_usd' crates/roko-serve/src/ || grep -rn --include='*.rs' 'estimate_cost_usd(' crates/ | grep -v 'fn estimate_cost_usd' | grep -q ."
+++

## Problem

`estimate_cost_usd` (`crates/roko-serve/src/bench.rs:723`) has no callers.

## Why it matters

Hygiene (epic spec-9a3131): an unused cost estimator invites someone to use it for numbers that should be measured. p3.

## Plan

1. Delete it. If an estimate is ever needed, label it `estimated` wherever it's shown.

## Done when

- [ ] The function is gone or has a caller that labels its output as estimated.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `67364f9d2`; cargo verification deferred to the batch check. The verify is static and passes; left open only for the batch cargo check.
