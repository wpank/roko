+++
id = "bug-ad5487"
kind = "bug"
title = "Gate rows in verification.rs still record turn 1 when the attempt's turn count is unknown"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-55fd84"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gate_rows_carry_the_attempts_turns_or_unknown' crates/roko-cli/src/graph_task_dispatch/ && cargo test -p roko-cli --lib gate_rows_carry_the_attempts_turns_or_unknown"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 38521c19f. Gate rows carry the attempt's turns, or turns_unknown with turn_number 0. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

bug-31438d's branch marks unknown turn counts (`extra.turns_unknown`), in place of the old fallback to 1 (bug-55fd84). The gate rows that `graph_task_dispatch/verification.rs` writes still fall back to 1 (`.unwrap_or(1)` at :657 and :960 on the branch).

## Why it matters

One settled record per attempt (epic spec-b7303f): a gate row that says turn 1 contradicts the attempt's record, which says the turns are unknown.

## Where

The two fallbacks in `verification.rs`.

## Plan

1. Take the turn number from the attempt's record, and mark it unknown the same way the episode does.
2. Add `gate_rows_carry_the_attempts_turns_or_unknown`.

## Done when

- [ ] Gate rows agree with the attempt's turn count, known or unknown.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
- Implemented on `work/bug-b8af02` at `45570299f`; cargo verification deferred to the batch check. `gate_rows_carry_the_attempts_turns_or_unknown` (targeted `cargo test` passed at the branch head). The gate-pass and gate-fail rows take their turn count from `attempt::reported_turns`, which also feeds the verdict's `executed.turns`. When the count is unknown they write `turn_number` 0 and mark `turns_unknown` (via `roko_learn::efficiency::TurnsRow`), as the episode does. The verification.rs change is limited to those two row sites.
