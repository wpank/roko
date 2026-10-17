+++
id = "bug-df9aff"
kind = "bug"
title = "Plan writing gives tasks allowed_tools = read_file, edit_file, grep for files that don't exist yet, so no try can create them (G1's 14-of-14 plan spent $0.47 of $0.60 with no changes on any rung); and it left T4 without its dependency on T1-T3"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-18
updated = 2026-10-18
source = "round-v6-dogfood-desktop#L4-g1-final"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L4, 2026-10-18)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::repair", "crates/roko-cli/src/plan_generate/runnable.rs::problems"]
links = { depends_on = [], blocks = [], related = ["bug-8fe462"], supersedes = [], duplicate_of = "" }
+++

## Problem

The plan roko wrote from pocket-synth's spec (round v6, take L4, 2026-10-18, roko 68528f4b2, planner gpt-oss-120b)
gives every task `allowed_tools = ["read_file", "edit_file", "grep"]` and `files = ["synth/<module>.py"]`. None of
those files exists yet: the project has only `synth/__init__.py`, and the tasks are there to write the modules. With
no `write_file`, no try can create its file. `edit_file` fails ("failed to read …/synth/svf.py: No such file or
directory"), and the agent stops: "I couldn't complete the implementation because the required file `synth/svf.py`
does not exist … so there was nothing to edit within the allowed scope."

Every try of every task ended with no changes, on every rung:

| Rung | What the tries did |
|---|---|
| cheap (gpt-oss-120b) | hit the 15-turn cap retrying `edit_file` |
| strong (gpt-5.4-mini, T2's failover) | gave up after 9.5 s |
| mid (glm-4.7) | T3's try ran 14 minutes |

In 15 minutes the run (`83c4222a-3523-4547-b587-1a37408b486f`, $0.60 cap) spent $0.469 and passed nothing, against a
plan estimate of "likely $0.02 · worst $0.16". The spec-check report was perfect, "14 of 14 kept", so the checks
were right and the tasks could not be done.

The same plan has a second defect. T4 ("Add render demo module and CLI") has `depends_on = []`, but its checks import
`synth.osc`, `synth.svf` and `synth.env`, which T1–T3 write. The plan shows "1 wave · 4 at once", so T4 runs before
the code it needs exists. The planner's earlier draft of the same spec (take l4-g1-prd-0043) had T4 after T1, T2
and T3.

## Why it matters

G1's pitch is "a prose spec becomes a runnable plan that keeps its checks". This plan keeps its checks and can't be
run: it spends its cap on tries that cannot succeed. Nothing tells the person before the run. `plan validate` and
the planner's own runnable pass (gap-ecafde) look at the checks, not at whether a task's tools can produce its files.

## Where

- Plan writing: `crates/roko-cli/src/plan_generate/` (the planner's per-task `allowed_tools`, `files` and
  `depends_on`; the runnable pass in `runnable.rs`).
- The plan: `/Users/Shared/pocket-synth-prd/plans/plan-ps-1-pocket-synth-oscillator-filter-envelop/tasks.toml`.

## Current state

- Nothing checks that a task can create the files it lists when they don't exist yet.
- Nothing checks that a task whose checks import another task's files depends on that task.

## Plan

- When a task's `files` don't exist on the base, its `allowed_tools` must allow creating them (`write_file`), or the
  plan leaves `allowed_tools` to the role's defaults. Repair it, or refuse the plan.
- When a task's verify imports a module another task's `files` will create, add `depends_on` (or flag it).
- Both are deterministic and could run in `runnable.rs`'s repair pass.

## Done when

Writing a plan from pocket-synth's spec gives T1–T4 tools that can create their modules, and T4 depends on T1–T3. A
run of it can pass.

## Notes

- Evidence: `tmp/hermes-roko-v6/findings/dogfood-desktop.md`.
  - The take is `tmp/hermes-roko-v6/media/pipeline/work/takes/l4-g1-prd-0117`, whose `plan-card.png` shows 14 of 14
    kept and "1 wave · 4 at once".
  - The run's events are in `/Users/Shared/pocket-synth-prd/.roko/events-by-run/` and `runs/83c4222a…/attempts.jsonl`.
- Related: bug-f0007b (T3's 14-minute mid try). On mid, z.ai rate-limited three tasks climbing at once. T1 ended
  after two `provider_error` tries without a real try on mid, and T2 failed over to strong.
