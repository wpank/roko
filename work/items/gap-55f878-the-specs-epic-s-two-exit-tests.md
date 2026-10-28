+++
id = "gap-55f878"
kind = "gap"
title = "The specs epic's two exit tests were never written: spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin (backlog 3200) and spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin (spec-e57870)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_validate", "roko-gate/spec_quality"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-27"
anchors = ["crates/roko-cli/tests/plan_validate.rs", "crates/roko-cli/src/commands/plan.rs::validate_before_run", "crates/roko-gate/src/spec_quality.rs::score_task"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-e57870", "find-70edcb", "dec-50192e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin"

[[verify]]
command = "grep -rqw 'fn spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin"
+++

## Problem

Two epics about specs a cheap model can execute end on an exit test that "whoever closes the last child" writes.
Nobody wrote either (status-quo assessment 2026-10-15 at 7b6a5af44, finding 27;
`tmp/status-quo/2026-10-15/backlog/A03-31-32-defaults-and-specs.md` section 3, row 3200):

- Backlog epic 3200 (`tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`): "an exact,
  test-backed fixture plan scores band B or better, and `roko plan run` refuses its twin whose verify step cannot
  fail". Its verify names `spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin`, which exists nowhere.
- Work item spec-e57870, step 7: a fixture pair in `crates/roko-cli/tests/plan_validate.rs`. The weak plan has a
  grep-only verify step, overlapping `files` between concurrent tasks, an oversized mechanical task and a missing
  `[task.accept]` source, and `plan validate --strict --spec-quality` rejects it with each code; its fixed twin
  passes. Its verify names `spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin`, which exists nowhere.

## Why it matters

Goal `golden-path`: the spec gate and spec lints are what keep a vacuous plan away from cheap models. Each child was
tested alone; nothing checks the whole gate end to end on one realistic pair. spec-e57870 cannot close until its
test exists.

## Where

- `crates/roko-cli/tests/plan_validate.rs`: where both tests go (53 tests today).
- `crates/roko-cli/src/commands/plan.rs::validate_before_run`: the plan-load spec gate `roko plan run` applies.
- `crates/roko-gate/src/spec_quality.rs::score_task`: the band score, rules SQ01-SQ12.

## Current state

A03 found all of 3200's code in place at HEAD (29 tasks done, 3234 partial, 6 blocked on spend). spec-e57870's open
children are find-70edcb (flag weak verify gates) and dec-50192e (CI `--strict`; Will's 2026-10-15 default: switch
CI to `--strict` in the change that fixes the last warning).

## Plan

1. Write a fixture pair under `crates/roko-cli/tests/fixtures/` (or inline): an exact, test-backed plan and its
   twin whose only verify step cannot fail (`|| true`, or a grep for a string the task writes).
2. `spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin`: the exact plan scores band B or better and passes
   the plan-load gate; the twin is refused before any dispatch, with the rule code in the message.
3. `spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin`: spec-e57870 step 7's pair, checking each expected code.
4. If find-70edcb's weak-verify flag is not in yet when you start, write test 2 first and leave test 3 to land with
   find-70edcb.

## Done when

Both `[[verify]]` commands pass.

## Notes

Parent: spec-e57870. Tests only; if a test exposes a gate bug, file it rather than widening this item.
