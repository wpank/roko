+++
id = "gap-ecafde"
kind = "gap"
title = "Generated verify steps can keep a check but never pass: a runner that is not installed (pytest), a test name that does not exist, a test file no task writes"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "7ad5f98e5"
source = "roko-planner, round v6 re-run of plans-research's docs (gap-dc5d7c)"
discovered_from = "gap-dc5d7c"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_validate.rs::validate_tasks_file"]
links = { depends_on = [], blocks = [], related = ["gap-dc5d7c", "bug-6fa025", "gap-dde99e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generation_flags_verify_steps_that_cannot_pass' crates/roko-cli/src && cargo test -p roko-cli --lib -- generation_flags_verify_steps_that_cannot_pass"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T14:55:46Z"
commit = "e8e6eff0c"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-17T14:00:20Z"
forced = false
evidence = "plan_generate/runnable.rs checks every verify step of a draft: each program installed (each python3 -m module importable), each file it reads existing or written by its task or one before it, each pytest/unittest test it names defined in its file unless a task writes that file. Repairs: pytest over unittest files runs as python3 -m unittest when pytest is missing; a test file a task check needs joins its files. Prompt states the runners this machine has; steps that still cannot pass go back once (with missing source checks), and a check covered only by them is cannot_run, not kept (e8e6eff0c). Verify generation_flags_verify_steps_that_cannot_pass passes; roko-cli lib 3792 passed, 0 failed; clippy roko-cli/roko-learn clean. Live (pocket-synth, gpt-oss-120b, $0.0694, 2 calls, binary e8e6eff0c): 14 of 14 kept, every step python3 -m unittest <real test id> (no pytest, INV-5 names both real lowpass tests); plan validate --spec-quality --dynamic: 4 of 4 tasks red on the base, none cannot-run; each of the 18 steps exits 1 on the base and 0 on the plans-research reference implementation."
+++

## Problem

gap-dc5d7c's re-run of plans-research's docs (round v6) kept every source check as a verify step, but some of
those steps cannot pass as written. Generation never checks that a step's runner exists, that a test it names
exists, or that a test file it names is one some task writes:

- **The runner is not installed.** pocket-synth (gpt-oss-120b) ran each of its 14 checks with `pytest -q
  tests/test_x.py::Class::test_y`. pytest is not installed in that workspace, and the spec says "standard library
  only" and names its tests as unittest ids (`tests.test_osc.OscillatorTest.test_engine_0_produces_output`). Every
  step would exit 127 at run time; bug-6fa025 is that failure.
- **A named test does not exist.** Its INV-5 step runs `tests/test_svf.py::SVFTest::test_lowpass_low_gain`. The
  file exists and no task writes it, but the test is `test_lowpass_attenuates_highs` / `test_lowpass_passes_lows`.
- **A test file no task writes.** 08g (gpt-oss-120b) checks each source check with `grep -q 'test_scN'
  crates/…/tests/<module>_scN.rs && cargo test --test <module>_scN`, but each task's `files` lists only its `src/`
  module. A run that keeps an attempt to its declared files cannot write the test its own check needs.

Evidence: `tmp/hermes-roko-v6/findings/roko-planner.md` (the v6 re-run), which quotes each plan's checks.

## Why it matters

A check that cannot pass is not a quality gate. It fails every attempt, and the ladder escalates for nothing. The
pitch is that the person's acceptance criteria become checks the work can actually pass (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the checks a draft must pass before it is
  written, and the regenerations.
- `crates/roko-cli/src/plan_validate.rs::validate_tasks_file`: the static rules `plan validate` applies.
- Related: bug-6fa025 (exit 127 at run time), plans-research item 6 (`spec_red_on_base.rs::classify` counts 127 as
  red).

## Plan

At generation, and as `plan validate` warnings:
1. **Runner.** Flag a verify step whose first word is not on PATH and is not a file the plan writes.
2. **Named test.** Flag a `path::Class::test` (pytest) or `module.Class.test` (unittest) that names a test missing
   from a file that exists and no task writes.
3. **Test file.** Flag a step that greps or runs a file that does not exist and that no task (the step's own, or one
   it depends on) lists in `files`.

Send a draft with any of these back to the planner once, naming each step, as the other regenerations do.

## Done when

`[[verify]]` passes: a generated plan whose step runs a missing runner, names a missing test, or needs a test file
no task writes is sent back once with each named.
