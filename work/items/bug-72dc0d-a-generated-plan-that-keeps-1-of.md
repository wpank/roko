+++
id = "bug-72dc0d"
kind = "bug"
title = "A generated plan that keeps 1 of the spec's 20 source checks is still written: the regeneration that asked for the 19 missing ones is dropped without a word, a cargo step in a repo with no Cargo.toml is neither flagged as unrunnable (runnable.rs checks the program, not the manifest) nor run red-on-base (skipped: cargo), and spec quality grades the plan A (Undertow phase 2, op 9fa7849f)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate", "roko-cli/plan_validate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#E-p2-plan2"
discovered_from = "agent:hermes-pilot (round v7 phase 2, op 9fa7849f)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_generate/runnable.rs::problems", "crates/roko-cli/src/plan_generate/source_checks.rs::apply_runs"]
links = { depends_on = [], blocks = [], related = ["bug-8fe462", "bug-3cdc58", "bug-b59d8e", "bug-8723fc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_spec_that_names_each_tasks_checks_gets_its_commands_or_no_plan' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_spec_that_names_each_tasks_checks_gets_its_commands_or_no_plan"
+++

## Problem

Round v7, Undertow phase 2, second plan write: roko 5de243ec8, planner gpt-5-4-mini, plan `p2-undertow-rumble`,
operation `9fa7849f`, from a Hermes chat that passed the spec exactly. The spec's acceptance-checks table names 19
commands (`tools/spec.sh 2 lfo 'P2.01*'` …), and its notes say which task owns which checks. roko found all 20 source
checks ("source checks in the planner prompt checks=20").

The first draft kept 1 of them, the `[[meta.verify]]` that roko's own repair adds. Every task's check was cargo, in a
C++/CMake repo with no `Cargo.toml`:

```
T1: test -f core/include/undertow/lfo.h && test -f core/src/lfo.cpp && grep -q 'file(GLOB' core/CMakeLists.txt && grep -q 'CONFIGURE_DEPENDS' core/CMakeLists.txt && cargo check -p undertow_core
T2: test -f core/include/undertow/rumble.h && … && cargo test -p undertow_core rumble
T3: test -f core/include/undertow/output_stage.h && … && cargo test -p undertow_core output_stage
T4: cargo check --workspace          (added by roko's own repair: bug-8723fc)
```

roko saw the gap and asked once more: "19 of the source's 20 checks are not kept as verify steps; asking the planner
once more". The regenerated draft (10,064 bytes of TOML) was dropped without a word, because a regeneration is kept
only if it validates and misses fewer checks. Nothing says which test failed. roko then wrote the first draft:

```
📋 Spec quality: mean 92.2, lowest task 78.1; bands A 3, B 1, C 0, D 0
📋 Source checks: 1 of 20 kept as verify steps, 19 in no verify step
📋 Wrote tasks.toml (8517 bytes) to …/plans/p2-undertow-rumble
```

Every later check passes it:
- `roko plan validate --spec-quality`: "0 diagnostics", 3 A and 1 B.
- `roko plan validate --spec-quality --dynamic`, after 0 s: "red on base (tasks) unknown: skipped: cargo (batch
  gate) 4".
- `runnable::problems` doesn't flag `cargo … -p undertow_core` in a workspace without a `Cargo.toml`, so bug-8fe462's
  `apply_runs`, which replaces a step that can't run with the source's command, never fires.

Run, the plan fails every task on every try, all the way up the ladder.

## Why it matters

The person's spec named every check and its command, and roko had them all in hand. roko still wrote a plan whose
checks can never pass, and graded it A. The person only learns this by reading the operation's report line, or by
paying for a run that can't succeed.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: `generate_plan`, at the source-check regeneration (`missing_of`)
  and the write that follows.
- `crates/roko-cli/src/plan_generate/runnable.rs`: `problems` (a cargo step in a workspace without `Cargo.toml`).
- `crates/roko-cli/src/plan_generate/source_checks.rs`: `apply_runs`.

## Plan

1. When the source names each task's checks (by id, as the spec's notes do) and the plan keeps few of them, roko
   assigns the source's commands itself. `apply_runs` already does this for steps that can't run.
2. Treat a cargo step in a workspace with no `Cargo.toml` as one that can't run. The same goes for any build tool whose
   manifest is absent.
3. Never drop a regeneration without a reason: log why it wasn't kept.
4. If most source checks are still missing, don't write the plan silently: fail, or write it with a blocking finding
   that `plan validate` and the plan's page show.

## Done when

A spec whose table names a check per task yields a plan that runs those commands, or no plan, never a written plan
with 1 of 20. Checked by the test in `[[verify]]`.

## Notes

- Evidence is in `tmp/hermes-roko-v7/pilot/evidence/p2-plan2/`: the plan, the operation, serve's printed report, the
  log lines, the chat's tool call and both validate outputs.
- The first write of the same spec (op `c31152b2`) kept all 19 commands, behind `grep -q … &&` guards. The planner's
  choice varies; roko's handling should not.
- The third write of the same spec (op `87df1c1b`, plan `p2-undertow-rumble-2`, 12:19) again had cargo steps in all
  four tasks and none of the 19 `tools/spec.sh` commands. This time the planner quoted every check, command included,
  in the tasks' `acceptance` lines ("AC1: SC1: P2.01 | `tools/spec.sh 2 lfo 'P2.01*'` | …"), with the cargo steps
  `covers` those ACs. roko's report counted that as coverage: "Source checks: 10 of 20 kept as verify steps, 10 checked
  only by a grep, a file test or a build", and spec quality A 4. Yet no step runs a single source command. The count
  should come from what a step runs, not from the AC text a step claims to cover. Evidence is in
  `tmp/hermes-roko-v7/pilot/evidence/p2-plan3/`.
