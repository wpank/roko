+++
id = "bug-3d0414"
kind = "bug"
title = "A criterion the planner writes that quotes a step's command is left uncovered when the step covers the source check's id instead: phase 5.2's write (op 453d4ba0) kept 9 of 9 checks but failed plan validate --strict with 5x PLAN_044"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-hermes-pilot#phase-5-2-write"
discovered_from = "agent:hermes-pilot (round v7, op 453d4ba0), the lead's ask"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::normalize_covers", "crates/roko-cli/src/plan_generate/source_checks.rs::criterion_naming"]
links = { depends_on = [], blocks = [], related = ["gap-8b1f4f", "gap-dc5d7c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_criterion_that_quotes_a_steps_command_is_covered_by_it' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_criterion_that_quotes_a_steps_command_is_covered_by_it"
+++

## Problem

Phase 5.2's plan write from chat (Undertow, op 453d4ba0, roko-main-00ecbd158, gpt-5.4-mini, $0.0686, 45.8 s) kept 9 of
9 source checks, but `plan validate --strict` exits 1 with 5× PLAN_044 (`tmp/hermes-roko-v7/pilot/logs/
validate-p5-2.log`; as written: `tmp/hermes-roko-v7/pilot/phase5-2/tasks.toml.as-written`).

- The planner wrote AC1-AC5 in its own words, each quoting one step's command: "AC2: `tools/spec.sh 5 presets` still
  passes on the base preset suite", through AC5 for `tools/render-demos.sh && tools/spec.sh 5 demos`.
- Its steps' `covers` named the source checks' ids (`SC1` … `SC8`), not those criteria.
- `source_checks::normalize_covers` points a step's `SCk` at a criterion only when the criterion names `SCk` itself.
  AC1-AC5 don't, so it added AC6-AC13 (`ACn: SCk: <the check>`) and pointed the covers there.
- AC1-AC5 were left with no step covering them, which is PLAN_044.
- The pilot fixed it by hand, adding each one to the covers of the step whose command it quotes.

**Not caused by the prompt.** For this spec the planner prompt differs between 2ca53cb61 and 00ecbd158 in one
sentence, gap-8b1f4f's "A check that comes with its own command is never dropped and never left to the plan's final
check: the task that owns it runs it." It says nothing about criteria or `covers`. Phase 4's plan (written before
gap-8b1f4f) has the same command-quoting criteria, but there the planner's covers named its own AC ids beside the
SC ids. So the variable is the planner's `covers`, not the text. Telling the two apart with live calls would take
several paired writes per binary, and the repair below makes the question moot.

## Why it matters

A plan that keeps every check still fails `--strict` and needs a hand edit before the person reviews it. Any
spec-sourced write whose planner quotes commands in its own criteria and covers by id can hit it.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::normalize_covers`.
- `crates/roko-cli/src/plan_generate/source_checks.rs::criterion_naming`, which already knows a criterion that names a
  check by id, label or command.

## Plan

1. `normalize_covers`: point a step's `SCk` at the criterion that names the check, by id, by its source label (P5.2.05)
   or by quoting its command whole. Add a new `ACn: SCk: …` criterion only when none does.
2. A safety net: a criterion that no step covers, and whose text quotes exactly one step's command whole, joins that
   step's `covers`, with a repair line ("T1: step 2 covers AC2, which quotes its command").
3. Test with phase 5.2's as-written draft: no PLAN_044, no added criteria, every check kept.

## Done when

5.2's as-written draft, run through the repairs, validates with `--strict` and no PLAN_044, keeping 9 of 9.
