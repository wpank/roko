+++
id = "bug-eeadbb"
kind = "bug"
title = "CI's test step stops at the first failing test binary (cargo test --workspace without --no-fail-fast), so each run shows one Linux failure and hides every test after it"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["ci"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "2b4272ff4"
source = "round v8 lead: release PR #89, runs 44524474195, 44574043691 and 44589959537"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = [".github/workflows/ci.yml"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-623001", "bug-4debc8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/^  test:/{t=1;next} /^  [a-z-]+:/{t=0} t' .github/workflows/ci.yml | grep -q 'run: cargo test --workspace --no-fail-fast'"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T20:44:42Z"
commit = "2b4272ff4"
forced = false
evidence = "2b4272ff4 on work/bug-4debc8. ci.yml job test now runs cargo test --workspace --no-fail-fast (every test binary runs; the step still fails if any test fails), so one run shows every remaining Linux failure instead of one per run (plan_runner hand-back, immune_boundary factory, roko-gate clippy rung in a row; 18 packages after roko-gate had not run). The verify failed on the old workflow and passes now; the workflow parses as YAML."
+++

## Problem

CI's Test + Clippy runs `cargo test --workspace`, which stops at the first test binary that fails. On release PR #89
each run showed one Linux failure and hid every test binary after it:
- plan_runner's hand-back test, fixed by bug-b589fe;
- immune_boundary's factory test, fixed by bug-7d13f0;
- roko-gate's clippy rung test, bug-4debc8.

Each fix took a full 60 to 120 minute CI round to reveal the next failure.

## Why it matters

Goal release. One run should show every remaining failure, so they can be fixed together.

## Where

- `.github/workflows/ci.yml`, job `test`.

## Plan

1. `cargo test --workspace --no-fail-fast`. The step still fails if any test fails, and it lists them all.

## Done when

CI's test step runs every test binary and fails if any test fails. The `[[verify]]` checks the workflow.
