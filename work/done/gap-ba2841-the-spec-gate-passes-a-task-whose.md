+++
id = "gap-ba2841"
kind = "gap"
title = "The spec gate passes a task whose tests assert one call equal to two values (round_to_dollars(150) == 2 and == 3) as 'advise, band B', so an impossible task is paid for and then gamed"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-gate/spec_quality"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "3f7cd10b4"
source = "find-2ba86d (hermes-plugin-roko dogfood 2026-10-13, run ff24ef90), split out 2026-10-27"
anchors = ["crates/roko-gate/src/spec_quality.rs::score_task", "crates/roko-gate/src/attempt_diff/gaming.rs::contradictions"]
links = { depends_on = [], blocks = [], related = ["find-2ba86d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_whose_tests_assert_one_call_to_two_values_fails_its_spec_check' crates/roko-gate/src && grep -q '\"HF2\", \"HF3\", \"HF6\"' crates/roko-cli/src/spec_gate.rs && cargo test -p roko-gate --lib a_task_whose_tests_assert_one_call_to_two_values_fails_its_spec_check"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T12:09:26Z"
commit = "3f7cd10b4"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-27T10:25:54Z"
forced = false
evidence = "3f7cd10b4: the spec gate has a sixth hard fail, HF6 (contradictory assertions). spec_quality/contradiction.rs reads a task acceptance together (its pinned [task.accept] tests from the plan directory, its acceptance items, the criteria of an acceptance section) and each other paragraph, list and fenced block of its goal, description and prompt on its own. It runs the gaming screen finder, now public as roko_gate::attempt_diff::test_contradictions, which gaming_findings also calls, so there is one finder. Prose comparisons are read quoted or not, with a left-out call (and == 3) and \"returns\"; joining-word gaps, moment words and other calls keep before/after prose and stateful tests apart, and two assertions contradict only with nothing state-changing between them, constant arguments and no method of an object the test binds. The detail names the call, both values and where: the description asserts `round_to_dollars(150)` to return both 2 and 3, which no one result meets. roko-cli adds HF6 to BLOCKING_HARD_FAILS, so plan run and the plan-load gate of every Graph run (prompt runs included) refuse the task before dispatch and the holdout never drops it; plan validate --spec-quality --strict exits 1. The verify passes: both greps, then cargo test -p roko-gate --lib a_task_whose_tests_assert_one_call_to_two_values_fails_its_spec_check ran 1 test and passed (the dogfood request quoted and as a chat host relays it, its pinned test, a test block in a description and two disagreeing criteria fail with HF6; a task asserting factorial(0) == 1 and factorial(5) == 120, a today-versus-wanted description and a stateful pinned test pass). roko-gate lib 718 passed, the speclint golden-fixture parity test included; roko-cli spec_gate_blocks_a_task_whose_tests_contradict_themselves passed, plan_validate 27, plan_validation 38 and plan_spec_gate 1 passed, and lib tests filtered to spec_gate, spec_red_on_base, plan_generate, run::tests and holdout: 244 passed. 0 HF6 over 30,110 tasks in 6,577 plans (repo plans and fixtures with the golden path, tmp/, the mori reference plans). CLI on a scratch plan with the dogfood request: roko plan run plans --dry-run prints \"spec gate: plan refused before dispatch\" with the HF6 line and exits 1; its == 2 twin exits 0. clippy -p roko-gate and -p roko-cli --no-deps -- -D warnings are clean. Filed gap-3b0382 (Rust and TypeScript test readers), gap-fef073 (generation reads no pinned tests) and gap-1f32ed (speclint has no HF6), all p3."
+++

## Problem

A task whose acceptance criteria, or the tests its request asks for, assert one call equal to two different values
cannot be done, yet the spec gate passes it. In the 2026-10-13 Hermes dogfood (find-2ba86d, run
`ff24ef90-37cc-43d0-86ff-cd29c19782d3`) the request asked for `round_to_dollars(cents)` and a test asserting both
`round_to_dollars(150) == 2` and `round_to_dollars(150) == 3`. The spec gate rated the task "advise (score 62.5,
band B)", the run spent money, and the agent passed the impossible test with a module-level call counter.

## Why it matters

Goal `truth`, and cost: a contradictory request is exactly what a spec check should catch before any model call. An
impossible check invites gaming; find-2ba86d's gaming screen catches the gamed pass afterwards
(`roko_gate::attempt_diff::gaming_findings`, kind `special_case`), but only after the spend.

## Where

- `crates/roko-gate/src/spec_quality.rs::score_task`: the spec-quality score and its hard fails, which read a task's
  goal, acceptance criteria, verify steps and pinned acceptance tests (`[task.accept]`).
- `crates/roko-gate/src/attempt_diff/gaming.rs`: `Expectation`, `Expected::contradicts` and `contradictions`, and
  `gaming/python.rs::assertion`, which read what Python test code asserts about a call and find a call asserted to
  two values no one result meets. They are private to the gaming screen today.

## Current state

The spec gate does not compare acceptance criteria or planner-written tests with each other. The gaming screen's
contradiction finder exists, but only for the test code an attempt changed, and only after the attempt ran.

## Plan

1. Make the contradiction finder reusable (a `pub` function over test text, by language).
2. In the spec gate, run it over the task's pinned acceptance tests and over test code quoted in its acceptance
   criteria or description; parse `AC` items of the form "`f(x)` returns `y`" too.
3. A contradiction is a hard fail with a message naming the call and both values, so `roko plan validate` and the
   prompt run's spec check stop the task before dispatch.

## Done when

A plan whose task asks for `round_to_dollars(150) == 2` and `== 3` fails its spec check, naming the call, with a
test in roko-gate; a task asserting `factorial(0) == 1` and `factorial(5) == 120` still passes.

## Notes

Split out of find-2ba86d on 2026-10-27: the coordinator kept the spec-gate check out of that item's scope.
