+++
id = "gap-4aaa3b"
kind = "gap"
title = "A plan cannot carry its own rubric: judge rungs are workspace-wide, and workspace_rungs = false drops them"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
size = "M"
subsystem = ["roko-cli/graph-dispatch", "roko-cli/plan_validate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round v6 plans-research: tmp/hermes-roko-v6/findings/plans-research.md, section 2.4 and proposed item 8"
discovered_from = "agent:plans-research (round v6)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskMeta", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::kind_rungs", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::rubric_criteria", "crates/roko-cli/src/graph_task_dispatch/verification.rs::plan_runs_workspace_rungs", "crates/roko-core/src/config/gates.rs::rung_files"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-f4b935", "spec-e57870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_rubric_feeds_a_plan_scoped_judge_rung' crates/roko-cli/src && grep -rqw 'fn plan_rubric_missing_file_fails_validation' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_rubric_"
+++

## Problem

A plan cannot carry its own rubric, and no judge rung can be scoped to one plan.

mori shipped a `rubric.md` with 139 of its 162 plan dirs (`/Users/will/dev/uniswap/bardo/.mori/plans/<plan>/rubric.md`).
Each was the plan's "single source of truth": the implementer self-checked every item, and reviewers raised only
blocking issues on that list.

roko has the machinery: a `judge` gate rung takes a `rubric`, and a helper model of another family scores each
criterion and must quote the artefacts. The rung is advisory by default. But:

- **Rungs come only from the workspace.** They come from `[[gates.rungs]]` or a domain pack (`[gates.packs.<domain>]`),
  so every plan in a workspace shares one rubric.
- **`[meta]` has no rubric or rungs key.** `META_KEYS` has neither, so `[meta] rubric = "rubric.md"` draws
  `PLAN_043 [meta] has unknown key `rubric`, which plan run ignores`, and `plan validate --strict` exits 1 (checked
  2026-10-16 at 47b3cdd20).
- **Opting out of workspace rungs drops judges too.** A plan that sets `[meta] workspace_rungs = false` gets no judge
  rung at all: `kind_rungs` returns nothing. A plan needs that opt-out when its acceptance tests are committed up front,
  because the workspace's whole-suite test rung would hold the other tasks' unwritten modules against each task.
  Imported mori plans need it for the same reason.
- **A mori rubric parses badly.** `rubric_criteria` makes every non-empty line a criterion. For 08f's `rubric.md`, 9 of
  the 19 "criteria" are headings (`### Tests`) or prose ("Single source of truth for all agents…").

## Why it matters

- Round v6's deliverable 2 is that roko accepts and runs mori-style plans, rubric and reviews included, from Hermes
  (`tmp/hermes-roko-v6/BRIEF.md`).
- A rubric holds the checks that tests cannot express: fidelity to a formula, code clarity, no global RNG. A judge
  from another model family is the cheap way to check them. That is part of the pitch: "you know what gets built
  passes your quality checks".
- Today such a plan loses its rubric on import (`tmp/hermes-roko-v6/plans-research/mori2roko.py` keeps only the
  rubric's command lines, as `[[meta.verify]]`).
- Related: gap-f4b935 (reviewer verdicts do not gate; decided "advisory first" on 2026-10-15), spec-e57870 (specs a
  cheap model can execute), and the mori-import item filed from the same findings.

## Where

- `crates/roko-cli/src/task_parser.rs::TaskMeta` and `META_KEYS`: the plan's `[meta]` table, where a `rubric` key
  would live.
- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::kind_rungs`: the judge, citations, schema and confirm rungs
  an attempt runs. They come only from `pack_rungs` and are dropped when the plan opts out of workspace rungs.
- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::rubric_criteria`: rubric text or file to criteria, one per
  non-empty line; falls back to the task's acceptance criteria.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs::plan_runs_workspace_rungs`: the `[meta] workspace_rungs`
  opt-out.
- `crates/roko-core/src/config/gates.rs::rung_files`: rubric and schema files the pre-verify screen protects from
  edits (bug-d5d55f); a plan rubric needs the same protection.
- Entry point: `roko plan run <dir>`, through `check_kind_rungs`, which runs once an attempt's verify steps pass.

## Current state

At 47b3cdd20:
- A `judge` rung needs `artefacts` (globs, relative to the task's workspace) and fails when none matches. Its model is
  `routing_context::select_judge_model_key`, preferring a family other than the executor's; with no helper model it
  is skipped. It is advisory unless `advisory = false`, and an advisory verdict is recorded on the attempt and shown
  on the dashboard, never failing it.
- The judge reads the attempt's diff as context (`roko_gate::evidence_judge`).
- `plan validate` has no rubric check.
- Nothing shows a rubric to the implementer: the prompt has `## Acceptance` and `## Verification Commands`, but no
  rubric section.

## Plan

1. `[meta] rubric = "<path>"`, relative to the plan directory: add it to `TaskMeta` and `META_KEYS`. `plan validate`
   fails when the file is missing, and warns when it yields no criterion.
2. A plan-scoped judge: each task of the plan that declares `files` faces an advisory `judge` rung named
   `plan-rubric`. Its `rubric` is the plan's file and its `artefacts` the task's `files`. It runs whatever
   `workspace_rungs` says, since it is the plan's own check and not a workspace rung. Advisory first, as gap-f4b935
   decided for review verdicts. Gating can follow once the records show how often it fails.
3. Criteria from Markdown: when the rubric is Markdown, take list items (`- [ ] …`, `- …`, `* …`) as criteria and
   skip headings, prose and emphasis lines. Keep today's line rule for a plain-text rubric or an inline `rubric`
   string.
4. Show the rubric to implementer tasks as a `## Rubric (self-check)` prompt section, as mori did. The rubric is
   advisory, so this only informs.
5. Protect the file: add the plan's rubric path to the paths an attempt may not edit, as `GatesConfig::rung_files`
   does for workspace rubric files.

Design choice: `[[meta.rungs]]` entries taking any `GateRungConfig` (judge, schema, citations, command) would be more
general, and would also give plans their own schema checks. Start with `rubric`, which covers mori's need with one
key, and add `[[meta.rungs]]` when a second kind is wanted.

## Done when

- A plan with `[meta] rubric = "rubric.md"` validates under `--strict` with no PLAN_043, and one naming a missing
  file fails validation.
- A run of that plan records a `plan-rubric` judge verdict for each implementer task, with
  `workspace_rungs = false` too. The verdict never fails the task.
- A Markdown rubric's headings and prose are not criteria; its list items are.
- `[[verify]]`: `grep -rqw 'fn plan_rubric_feeds_a_plan_scoped_judge_rung' crates/roko-cli/src && grep -rqw 'fn plan_rubric_missing_file_fails_validation' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_rubric_`

## Notes

- Do not change what `workspace_rungs = false` does to `[[gates.rungs]]` and the packs.
- Leave reviewer verdicts to gap-f4b935; this item is about rubric criteria judged against artefacts.
- Never run `share_run_scrubs_output_text_and_prompt` (it publishes a public gist); the verify's filter does not
  match it.
- Evidence and mori numbers: `tmp/hermes-roko-v6/findings/plans-research.md` sections 1.2 and 2.4. A mori rubric
  to test with: `/Users/will/dev/uniswap/bardo/.mori/plans/08f-sonification-oscillator-filter-envelope/rubric.md`
  (read-only).
