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

## More evidence: phase 4 (hermes-pilot, 2026-10-21)

Plan `p4-undertow-phosphor-editor` (op fe5ffce6, roko 5a9884c95 + bug-5e597a) was written from
`docs/specs/phase-4-ui.md`, a 264-line spec, by the gpt-5.4 planner.
- The spec's planner note asks for the spec first among every task's reads. The planner did that, but gave T4–T7
  ranges that leave out all of their own "What to build" item:

  | Task | Spec lines read | Its item |
  |---|---|---|
  | T4 (unit array) | 86-179 | 81-85 |
  | T5 (look) | 97-179 | 87-92 |
  | T6 (CHEST displays) | 105-179 | 94-96 |
  | T7 (status, budget, images) | 129-223 | 98-106 |

  - T3 read 63-179 and missed the rules (212-219).
  - T1 and T2 read 1-223 and missed the planner notes.
  - roko only clamps a range that runs past the end of a file.
  - The person reviewing the plan set every spec read to `1-264` by hand (plan.md, "Review edits (person)").
- **For the fix: a whole spec may still not reach the agent whole.**
  - The declared files open the task's runner context (`build_runner_context`: declared context first, then Files in
    scope, acceptance, verify, Prior Task Outputs, Sibling Tasks, workspace map).
  - The system prompt's `context_layer` is capped at the role's `runner_context` budget: 4,000 tokens for an
    implementer (`section_budget_cap`). `enforce_hard_cap` keeps the first 16,000 bytes and appends "…[truncated N
    tokens]".
  - In Undertow's phases 0–3, 39 of the 57 tries that recorded a `context_layer` had 4,006 or 4,007 tokens: the
    cap plus the marker. That is every phase's plan, 0 to 3.
  - Phase 4's spec, numbered, is about 20.7 KB. A whole read is cut at line 212, just before "Rules every task
    keeps", and every read file after it is cut too.
  - A source read may need its own budget, or a place outside the capped layer.
