+++
id = "gap-603aa4"
kind = "gap"
title = "roko-serve's estimate_cost_usd has no callers"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-serve/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-serve/src/bench.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'fn estimate_cost_usd' crates/roko-serve/src/ || grep -rn --include='*.rs' 'estimate_cost_usd(' crates/ | grep -v 'fn estimate_cost_usd' | grep -q ."

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 89028c96f. estimate_cost_usd is deleted. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: static check passes on MAIN."
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
