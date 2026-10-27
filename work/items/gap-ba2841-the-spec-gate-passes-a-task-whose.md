+++
id = "gap-ba2841"
kind = "gap"
title = "The spec gate passes a task whose tests assert one call equal to two values (round_to_dollars(150) == 2 and == 3) as 'advise, band B', so an impossible task is paid for and then gamed"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-gate/spec_quality"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "find-2ba86d (hermes-plugin-roko dogfood 2026-10-13, run ff24ef90), split out 2026-10-27"
anchors = ["crates/roko-gate/src/spec_quality.rs::score_task", "crates/roko-gate/src/attempt_diff/gaming.rs::contradictions"]
links = { depends_on = [], blocks = [], related = ["find-2ba86d"], supersedes = [], duplicate_of = "" }
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
