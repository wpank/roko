+++
id = "gap-343a23"
kind = "gap"
title = "The planner prompt teaches a grep -q guard before every fail_on_base check (rule 6 and three example plans), so planners guard the spec's own check commands and run_exactly strips them on every spec-sourced plan (phase 3's re-plan: 7 of 7 steps)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "f45fc8ff6"
source = "round-v7-team-lead#phase-3-replan-guards"
discovered_from = "agent:roko-planner (round v7, phase 3 re-plan op 3b13af5f, the lead's ask)"
anchors = ["crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT", "crates/roko-cli/src/plan_generate/source_checks.rs::SELF_CONTAINED"]
links = { depends_on = [], blocks = [], related = ["bug-56a806", "gap-115c2a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_planner_is_told_not_to_guard_a_source_check' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_planner_is_told_not_to_guard_a_source_check"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T11:37:58Z"
commit = "1dcad4591"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 1dcad4591 (branch dca28c426; main merged in at 8a82040ff and 7de5836a0). Post-merge verify in main at 1dcad4591: the grep finds the test and the_planner_is_told_not_to_guard_a_source_check passes. Branch suites at 7de5836a0: roko-cli lib 3992 passed, 0 failed, 7 ignored (share test skipped); plan_generate 171/171; nightly fmt clean; clippy -p roko-cli -D warnings clean. The guard is now taught only for the planner's own checks, in rule 6, the first example plan (Rust and the CMake rewrite Undertow reads), the source-check block (the guard paragraph only for a check with no given command), the CMake note and the hidden-output re-ask. Live (10-21, gpt-5.4-mini, phase 3 at Undertow c9dc249): the CLI and serve MCP plan_generate {spec_path} each wrote the spec's 7 commands alone, with 0 grep -q and 0 guard repairs ($0.0359, $0.0139). The old prompt (dafb6316f) did the same ($0.0497, $0.0150), so the guard is intermittent and the pinned test is the proof; run_exactly stays the safety net."
+++

## Problem

Round v7, Undertow phase 3's re-plan from chat (op 3b13af5f, roko 79208f291, 2 planner calls, $0.0735): the planner
put a guard such as `grep -q 'P3.01' tests/spec/p3/sequencer_test.cpp && …` before the spec's command on all 7 verify
steps. e3b25962e's repair (bug-56a806, `source_checks::run_exactly`) stripped every one, and the plan sheet lists
seven repairs. Phase 2's rumble-3 write had 15 such guards, T4's for `AllocationCounter`, which could never pass.

roko's own planner prompt teaches the guard:

- `PLAN_GENERATOR_SYSTEM_PROMPT`, rule 6 ("Executable spec (TSS v1)", `crates/roko-cli/src/plan_generate.rs`): "a
  `fail_on_base` step checks what the task adds: a `grep -q` for the new item before the compile or the test run
  (`grep -q 'fn new_test' crates/x/src/lib.rs && cargo test -p x new_test`)".
- Three of the prompt's example plans do the same: `grep -q 'pub struct FundingRate' … && cargo check -p
  roko-core`, `grep -q 'fn status_shows_funding_rate' … && cargo test -p roko-cli …` and `grep -q 'fn
  health_endpoint' … && cargo test -p roko-serve --test api_integration health_endpoint`.
- The test `generator_prompt_runs_test_runners_plainly` pins the rule-6 example.
- The source-check block tells the planner a source check "must be self-contained and runs exactly as written"
  (gap-115c2a, `source_checks::SELF_CONTAINED`), but never says not to guard it. The planner follows rule 6.

## Why it matters

Each guard costs a repair on every spec-sourced plan, and a guard the repair misses can make a check impossible (T4's
`AllocationCounter`). The rule is right for a check the planner writes itself, where a bare compile or an unmatched
test filter passes on the base. A spec's own command already fails on the base: its tests are the new item.

## Where

- `crates/roko-cli/src/plan_generate.rs`: `PLAN_GENERATOR_SYSTEM_PROMPT`, rule 6 and the example plans.
- `crates/roko-cli/src/plan_generate/source_checks.rs`: `SELF_CONTAINED` and `planner_block`.

## Plan

1. Rule 6: the guard is for a check the planner writes itself. A check whose command the source gives runs alone,
   with nothing before it.
2. Say the same with the source's checks (`SELF_CONTAINED`): no `grep` or other command before it.
3. Keep `run_exactly` as the safety net, and count the guards it strips in the generation's repairs, so a
   regression shows.

## Done when

A test pins the prompt: rule 6 limits the guard to the planner's own checks, and the source-check block says to run
a source command with nothing before it.
