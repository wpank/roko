+++
id = "gap-d2d750"
kind = "gap"
title = "In active mode the self-model chooses the start rung through S03's route table and logs its propensity, task 6130"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "73a96794e"
source = "tmp/backlog/2026-10-02-complete-and-wire 6130 (deferred in wave 8, PK49, until PK43)"
discovered_from = "gap-7ec3ef"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs"]
lane = "rust-hot"
links = { depends_on = ["gap-c1d920"], blocks = [], related = ["gap-7ec3ef", "gap-c1d920"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn active_self_model_picks_start_rung_and_logs_propensity' crates/roko-cli/src/ && cargo test -p roko-cli active_self_model_picks_start_rung_and_logs_propensity"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T19:23:32Z"
commit = "73a96794e"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T17:08:28Z"
forced = false
evidence = "Gate 9b (work/backlog-batch-9b, merged into main as 73a96794e): cargo check --workspace --tests, roko-cli and roko-serve with fault-injection, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 5,854 tests over roko-cli, -learn and -serve, roko-cli bin 436 passed, the golden-path canaries and the loop-audit census run pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, 159 fault-injection lib tests pass; every [[verify]] passes. 6130: in active mode the self-model picks the start rung through S03's route table (held-out chains run the ladder's rung at h, epsilon explores, L-M3 rows log every rung's propensity)."
+++

## Problem

Task 6130 of PK49 (gap-7ec3ef, gated in wave 8): nothing lets M3 act. In active mode the self-model should choose
the start rung within the ladder (6101's option B) and log its propensity, through S03's route table, which PK43
(gap-c1d920) wires into `ModelRouter::decide`. PK49's worker stopped before it because the spec forbids editing
`crates/roko-cli/src/dispatch/model_routing.rs` in parallel with PK43, and a second L-M3 draw would duplicate the
table.

## Why it matters

Until it lands the self-model only forecasts (shadow mode); the M3 loop can't act, and its loop audit can't
measure an effect.

## Where

`crates/roko-cli/src/dispatch/model_routing.rs` (`ModelRouter::decide`, PK43's route table), the self-model
(`roko_learn::self_model`, PK47-PK49), and the `[self_model]` config section (6127).

## Current state

PK43 and PK49 are merged (gate 8c); the shadow hook (6128) and the outcome sink (6129) are live.

## Plan

1. Implement 6130 as its spec says, drawing through S03's route table (PK43) rather than a second L-M3 draw.
2. Write the test the verify names.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/6130-in-active-mode-let-the-self-model-choose-the-start-rung.md`.
- Left PK49's package item at gate 8c (2026-10-03).

## Progress

- 6130: implemented at 09cbdc04a (cargo verification deferred to the batch gate)
