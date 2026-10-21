+++
id = "bug-60ad88"
kind = "bug"
title = "A plan written from a spec never gives its tasks the spec: phase 3's T1 tries worked from the tests and the PRD, never read docs/specs/phase-3-sequencer.md, and got the step rule wrong"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-undertow-spec#phase-3-T1-failures"
discovered_from = "agent:undertow-spec (round v7, phase 3 run bdc4461e)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation"]
links = { depends_on = [], blocks = [], related = ["gap-fe4225", "gap-ac6b9b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_task_of_a_spec_sourced_plan_reads_the_spec' crates/roko-cli/src && cargo test -p roko-cli --lib -- every_task_of_a_spec_sourced_plan_reads_the_spec"
+++

## Problem

Round v7, Undertow phase 3 (run bdc4461e; undertow-spec's findings, "Phase 3's T1 failures"). The plan
`p3-undertow-hypnotic-sequencer` was written from `docs/specs/phase-3-sequencer.md` through `spec_path` (op
1e9ab2e7). Yet:

- T1's `[[task.context.read_files]]` lists only `tests/spec/p3/sequencer_test.cpp` and PRD lines 60-170, and T2's
  lists engine.h, engine.cpp and the PRD. Neither task reads the spec.
- plan.md leaves the request out, since a chat request is untrusted.
- Of the six tries that reached the gate, four never opened the spec, and two opened it only in their last minute.
  They worked from the tests and the PRD, which don't state the step rule (seq_rate's step length), and got it wrong
  (16 note-ons where 8 were wanted).
- The spec-quality lint scored T1 100 (band A).

Nothing in roko gives a task the document its plan was written from: the planner decides each task's context files,
and the source path is lost on the way to the pipeline (`spec_path` and `--from-file` both arrive as text).

## Why it matters

The spec is where the person states the rules. A cheap model that never reads it can only guess from the tests.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: the draft's repairs; `PlanRequest`.
- `crates/roko-serve/src/routes/plans/authoring.rs`, `routes/mcp.rs`: `spec_path`; `crates/roko-serve/src/runtime.rs`:
  `generate_plan_reporting`; `crates/roko-cli/src/serve_runtime.rs`; `crates/roko-cli/src/commands/plan.rs`
  (`--from-file`).
- `crates/roko-cli/src/plan_validate.rs`: a lint.

## Plan

1. Carry the source document's path to the pipeline: `PlanRequest.source_path`, set by `spec_path` (through the
   serve runtime) and `--from-file`. The documents a request names (gap-ac6b9b) are sources too.
2. A deterministic repair, logged: every task's `read_files` includes each source document, the whole file with an
   explicit range, since a file without a range is inlined only to line 100. A partial range of it widens.
3. The plan records its sources in `[meta] sources`, and `plan validate` warns about a task that does not read one
   (a new PLAN rule).

## Done when

The test in `[[verify]]` passes, and a $0 replay of phase 3's write (op 1e9ab2e7) shows every task reading the spec.
