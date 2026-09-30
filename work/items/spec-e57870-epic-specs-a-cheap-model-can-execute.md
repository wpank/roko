+++
id = "spec-e57870"
kind = "spec"
title = "Epic: specs a cheap model can execute"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/prd", "roko-cli/plan_validate", "roko-cli/plan_policy", "benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #9-10; tldr/04 steps 1-3)"
anchors = ["crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_outcome", "crates/roko-cli/src/plan_policy.rs::validate_plan_budgets", "crates/roko-cli/src/plan_validate.rs::validate_tasks_file", "crates/roko-cli/src/task_parser.rs::TaskDef"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["gap-853b31", "gap-2623b2", "gap-d14a43", "gap-1cd8d3", "gap-b3fa0a", "find-70edcb", "gap-a8d786", "gap-1d1fa6", "gap-46ab3f", "bug-477ede", "gap-1b5636", "gap-ba4d01", "bug-b0fd73", "bug-019f02", "bug-05d1ac", "bug-c1b845"], blocks = [], related = ["find-84bfa8", "bug-8b1bf8", "gap-b3e513", "gap-3bea93", "gap-0f3980", "gap-25065c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin"
+++

## Problem

The golden path assumes that a frontier model writes tasks small, exact and checkable enough for a cheap model to
execute. Roko's authoring side works against that:

- The frontier planner can be selected only on CLI paths. The portal/serve path, `roko do`'s complex band and plan
  revision use the default model.
- Plans come from three prompts. The main one cuts the PRD at 8,000 characters and lets the planner read at most
  5 files, "for smaller models".
- `acceptance` is prompt text only. The pattern that worked, pinned tests in `accept/` (6 portal plans, 48 files),
  is a hand convention, and the agent can edit those files.
- Nothing proves that a verify step fails before the change, flags weak verify steps, checks that concurrent tasks
  write disjoint files, or bounds a task's size by its executor tier.

## Why it matters

- tldr/04 steps 1–3 (author, compile the spec, size and split) are PARTIAL or MISSING. They are P1 #9–10 in tldr/05.
- tldr/04 design rules 2, 3 and 7: split on independent outputs, the planner writes the gating checks, and ambiguity
  goes back into authoring.
- Cheap models game visible checks most (`zhao2026specbench`), so the checks must come from the planner and be
  proven red before the change.
- Epic E9 (the diff check) needs `[task.accept]` from this epic.

## Where

- Generation: `crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_outcome`, `plan_generate.rs`,
  `plan_authoring.rs::revise_plan_source`, and `commands/{plan,do_cmd}.rs`.
- Validation: `plan_validate.rs` (codes PLAN_001–036) and `plan_policy.rs` (`PlanExecutionPolicy`,
  `validate_plan_budgets`).
- Schema: `task_parser.rs` (`TaskDef`, `VerifyStep`).
- speclint (Python): `benchmarks/viabilitybench/speclint/`. The path follows decision D4 (dec-b78874).

## Current state

Checked at `41c7ffbd6`:

- No `planner_model` or `[authoring]` anywhere in `crates/`. `generate_plan_from_prd` and
  `generate_plan_from_prd_isolated` pass `model: None`; `revise_plan_source` uses `[agent] model`.
- `prd.rs:1398` still truncates the PRD at 8,000 characters; `prd.rs:1413` still says "read up to 5 codebase files".
- `TaskDef` has no accept, goal, hidden-test or coverage fields, and `VerifyStep` has no `expect`.
- `plan_validate.rs` has no overlap, tier-size or spec-quality rule. `plan_policy.rs` has only the serial-owner rule
  `PLAN_FRAGMENTED_OWNERSHIP`.
- No `benchmarks/viabilitybench/` tree exists; `benchmarks/` holds only `dev-audit/`. The S07 prototype linter
  lived in a scratchpad.
- **find-70edcb is mostly subsumed.** Its weak-verify patterns (file-scoped verifies, negative greps, verifies that
  pass on an untouched tree) are speclint rules SQ04 and SQ05 and hard fails HF2 and HF3. It has no `[[verify]]`.
  Proposal: close it as covered once gap-46ab3f and gap-b3fa0a land, and move its generator-prompt half to
  gap-2623b2.

## Plan

This is the implementation plan.

1. **Planner model** (gap-853b31): `[authoring] planner_model` on every generate and revise path. Can start now.
2. **One generator** (gap-2623b2, after 1): fold `roko plan generate` and `roko do`'s standard band into the PRD
   pipeline, and scale the PRD and file budgets to the planner's context window.
3. **Pinned acceptance tests** (gap-d14a43): `[task.accept]`, stored outside the agent's reach and checked by hash.
   Can start now; E9.1 (gap-abbd22) waits for it.
4. **speclint in Python** (gap-1cd8d3, then gap-b3fa0a): the static rules SQ01–SQ12 with hard fails (S07.1), then
   the red-on-base checker (S07.2). Both wait for decision D4.
5. **Rust lints in `plan_policy.rs`:** overlapping files between concurrent tasks (gap-a8d786, after E7.3
   gap-439794) and size limits per tier (gap-1d1fa6, after E5.1 gap-8c0a20).
6. **`plan validate --spec-quality`** (gap-46ab3f, after 4): the static rules in Rust, matched against speclint.
7. **Exit check,** written by whoever closes the last child: a fixture pair in
   `crates/roko-cli/tests/plan_validate.rs`.
   - The weak plan has a grep-only verify step, overlapping `files` between concurrent tasks, an oversized
     mechanical task and a missing `[task.accept]` source. `plan validate --strict --spec-quality` rejects it, with
     each code.
   - Its fixed twin passes.

## Done when

- [x] gap-853b31: Frontier planner everywhere: an [authoring] planner_model on every plan generate and revise path
- [ ] gap-2623b2: One plan generator instead of three prompts, without the 8,000-character PRD and 5-file caps
- [x] gap-d14a43: Planner-written acceptance tests in [task.accept], stored out of the agent's reach
- [x] gap-1cd8d3: speclint first slice: static spec-quality rules SQ01–SQ12 with hard fails (S07.1)
- [x] gap-b3fa0a: Red-on-base checker: prove each task's verify step fails on a clean base (S07.2)
- [ ] find-70edcb: Plan generation/validation does not flag weak verify gates (existing item)
- [ ] gap-a8d786: Plan lint: tasks that can run at the same time must not share files
- [x] gap-1d1fa6: Task size limits per executor tier in plan validate
- [x] gap-46ab3f: plan validate --spec-quality runs the speclint rules when a plan loads (S07.9)
- [x] bug-477ede: prd plan escalation drops a planner model outside the haiku/sonnet/opus chain to the cheapest model
- [x] gap-1b5636: Prompts paste each pinned acceptance script verbatim; show pinned steps by their header line only
- [ ] gap-ba4d01: Portal plans 08b–08e and 08g still hand-copy their acceptance tests instead of pinning them with [task.accept]
- [ ] bug-b0fd73: For accept plans, authored_plan_running reports that tasks.toml no longer matches on every run
- [x] bug-019f02: speclint and roko_gate::spec_quality ignore [task.accept], so plans that pin acceptance tests lose verify steps and acceptance credit
- [ ] bug-05d1ac: verification.rs quotes each step's full command in skipped-step lists, progress events and gate output, so a pinned step repeats its script in retry feedback
- [ ] bug-c1b845: speclint --dynamic runs only authored verify steps on the base, so SQ06 and HF3 ignore pinned acceptance tests
- [ ] The epic's `[[verify]]` command (the weak and fixed fixture pair) passes on the merged branch.

## Notes

- **Not filed here:**
  - S07.3–S07.5 (the degradation operator, the refiner, the critic) and S07.10–S07.12;
  - S07.8, the TSS v1 fields: `goal`, `non_goals`, `[task.hidden]`, and verify `covers` and `expect`;
  - open questions that block dispatch (design rule 7), and a plan diff in the portal;
  - failure-aware revision (gap-b3e513, gap-3bea93).

  gap-25065c (E14.6) imports the rest of the S07 checklist as unverified items.
- find-70edcb keeps its goal (`core`) and severity (p2). A move to `golden-path` is proposed in `PLAN.md` §5.
- Mostly cold files. gap-46ab3f adds one flag in `main.rs`. The plan-load gate in `plan_runner.rs` (S07.11) is later
  work.
- Task granularity is decision 2 in tldr/05 §6 (default: size by the executor tier's measured pass rate).
  gap-1d1fa6 encodes it.
