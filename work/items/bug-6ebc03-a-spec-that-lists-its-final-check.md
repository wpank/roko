+++
id = "bug-6ebc03"
kind = "bug"
title = "A spec that lists its final check in its table and names it the plan's own in its notes gets two source checks for it, and coverage reads the table's as missing though [[meta.verify]] runs it: phase 1's plan showed 1 of 21 checks not in the plan, and paid for a regeneration"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "9d34a2e8b"
source = "round-v8 hermes-pilot-v8, phase 1 plan write op 19bfbbb8 (Undertow 19943c9, roko bd93521ee), tmp/hermes-roko-v8/pilot/evidence/p1-plan-writing-status.json"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::source_checks", "crates/roko-cli/src/plan_generate/source_checks.rs::plan_check_line", "crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-96115c", "bug-3cdc58", "bug-187c5c", "bug-aa93e9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_final_check_the_table_states_is_one_check' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_final_check_the_table_states_is_one_check"
+++

## Problem

A spec that lists its final check in its table, and also says in its notes that this check is the plan's own, gets
two source checks for it. Coverage then reads the table's copy as missing, though `[[meta.verify]]` runs it.

Round v8's phase 1 plan write (op 19bfbbb8, Undertow 19943c9, roko bd93521ee):

- The spec's table has `| P1.20 | `tools/check.sh --plugin` | The whole local check passes … |`.
- Its notes say "`tools/check.sh --plugin` (P1.20) is the plan's final check, not a task's."
- `source_checks` lifted both: SC20 (the row) and SC21 (the notes line, `plan_level`).
- The plan's `[[meta.verify]]` runs `tools/check.sh --plugin`, so SC21 was kept. SC20 is not plan-level, so only a
  task's step could keep it, and it read as missing.
- The status said 21 checks, 20 kept, 1 missing, and the Hermes card showed "1 of 21 checks your spec states is not in
  this plan". In fact all 20 commands run: 19 in tasks, P1.20 as the final check.
- The false "missing" also sent the draft back for a coverage regeneration: a second planner call, about half of the
  write's $0.258, and the call that took it over its $0.25 cap (bug-187c5c's report).

## Why it matters

- Every Undertow 2 phase spec lists its final check in its table and names it in its notes, so every phase pays for a
  regeneration it doesn't need, and shows a person a gap that isn't there.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::source_checks`: lifts the table row and the notes line
  separately (`Reader::line`, `plan_check_line`).
- `SourceCoverage::of`: a plan-level check is kept only by `[[meta.verify]]`, and any other only by a task's step.

## Plan

1. When a line names a check of the plan's own and also names a check the source states elsewhere, it is that check.
   It names it by that check's label (`P1.20`, the first cell of its row) or by the command the check quotes first.
   That check becomes plan-level, and the line adds no check of its own. The ids stay in source order: SC1 to SC20.
2. A line that names no stated check stays a check of its own, as bug-96115c lifted it.
3. Coverage is unchanged: `[[meta.verify]]` keeps the plan-level P1.20. A check the source does not call the plan's
   own still needs a task's step. A check run only as the final check can't fail the task whose work it checks, so
   coverage keeps asking for that task's step.
4. Test `a_final_check_the_table_states_is_one_check`, on the real phase 1 spec and its plan as written. It gives 20
   checks, P1.20 plan-level, 20 kept, none missing. The v7 phase 1 fixture, whose final check is in no table, keeps
   its 21.

## Done when

- The `[[verify]]` command passes.
- The plan as written reports 20 of 20 kept and asks for no regeneration.

## Notes

- Found by hermes-pilot-v8 (evidence: `tmp/hermes-roko-v8/pilot/evidence/p1-plan-writing-status.json`).
- Related: bug-96115c (plan-level checks), bug-3cdc58 (a check kept by its command), bug-187c5c (the cap).
