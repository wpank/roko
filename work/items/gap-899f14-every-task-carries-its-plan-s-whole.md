+++
id = "gap-899f14"
kind = "gap"
title = "Every task carries its plan's whole spec in its prompt: phase 5's 25.4 KB spec was 55-67% of every first prompt and about 19% of the run's spend ($0.27 of $1.39); a per-task excerpt (its section, its checks and the rules every task keeps, marked by the spec's author) is the safe shape, measured before it's adopted"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_policy"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 10, run 1f677933), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/source_reads.rs::repair_source_reads", "crates/roko-cli/src/plan_policy.rs::render_declared_files"]
links = { depends_on = [], blocks = [], related = ["bug-a8a787", "bug-60ad88", "gap-744bd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_reads_its_section_its_checks_and_the_marked_rules' crates/roko-cli/src && cargo test -p roko-cli --lib a_task_reads_its_section_its_checks_and_the_marked_rules"
+++

## Problem

Every task of a spec-sourced plan carries the whole spec in its prompt. In phase 5 that cost about 19% of the run's
spend.

**Phase 5** (Undertow, run 1f677933, plan `p5-ship`):
- `docs/specs/phase-5-ship.md` is 274 lines, 22.8 KB on disk and 25.4 KB with the prompt's line numbers.
- It is whole in all 13 tries' prompts, 55–67% of each first prompt (37.8–51.8 KB).
- The run made 227 model calls, and each re-sends it: once fresh on each try's first call, then as cached input.
- **About $0.27 of the run's $1.39, 19%.** That's an estimate at the logs' 3.0 bytes a token, with gpt-5.4-mini's
  and gpt-5.4's prices per call (`tmp/hermes-roko-v7/run-analyst/prompt_logs.py 1f677933 p5-ship`).
- Phase 4.3's 12 KB spec was 26–28% of each prompt.

A task needs much less. T4, for example, needed:
- its own item under "What to build" (about 1 KB);
- its check rows (about 0.3 KB);
- "Rules every task keeps" (0.8 KB).

## Why it matters

**The trade-off is real; this is not to be done blind.** The whole spec is in every prompt for two reasons:
- **bug-60ad88:** in phase 3, four of six tries never opened the spec, so `repair_source_reads` now makes every task
  read its plan's sources whole.
- **bug-a8a787:** in phase 4, a rule at the spec's line 218 was cut from every prompt by a 4k context cap, and the
  plan check failed on it. Since then the declared files are their own section, sources first and cut last.

A per-task excerpt would cut most of this cost again. The failure it risks is phase 4's: a rule that sits outside
the excerpt.

Goal: golden-path, cheaper at equal quality.

## Where

- `crates/roko-cli/src/plan_generate/source_reads.rs::repair_source_reads`: widens every task's read of a source to
  the whole file. `plan validate` holds tasks to it (PLAN_061).
- `crates/roko-cli/src/plan_policy.rs::render_declared_files`: renders sources first and cuts them last.
- **Entry point:** `crates/roko-cli/src/dispatch/prompt_builder.rs::assemble`.

## Current state

- Undertow's specs already have the needed shape:
  - numbered items under "What to build", one per task;
  - an "Acceptance checks" table;
  - a "Rules every task keeps" section;
  - "Notes for the planner", which maps tasks to items and checks.
- Nothing in roko reads that shape.

## Plan

1. **The safe shape is per task:** the task's own section, the rows of its checks, and a section the spec's author
   marks as the rules every task keeps. For example, a heading convention or a marker such as
   `<!-- roko: every task -->`. Without the mark, keep the whole spec, as now.
2. **The plan names each task's sections** in its `read_files` (`lines` ranges or a `section` key). plan validate
   checks:
   - that the marked rules are in every task's reads;
   - that each task's checks are in its reads.
3. **Measure before adopting.**
   - Replay phase 4's plan, whose plan check failed on a cut rule, and phase 5's, with the per-task shape. Compare
     plan-check outcomes and spend.
   - Adopt only if the plan checks still pass.
4. **Test:** `a_task_reads_its_section_its_checks_and_the_marked_rules`. A spec with a marked rules section gives
   each task those three parts, and one without the mark gives each task the whole spec.

## Done when

- The `[[verify]]` passes.
- A replay of phases 4 and 5 shows the plan checks unchanged at a smaller spec share. Check the prompt logs with
  `prompt_logs.py`.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 10, "The retry prompts on 74c3594bc"; Undertow
  `.roko/prompt-logs/p5-ship-*.txt`.
- Related: bug-a8a787 and bug-60ad88 (why the whole spec is there), gap-744bd0 (a declared file sent twice).
