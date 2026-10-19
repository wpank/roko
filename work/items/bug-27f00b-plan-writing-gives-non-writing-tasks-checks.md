+++
id = "bug-27f00b"
kind = "bug"
title = "Plan writing gives non-writing tasks checks that no task can change: a one-line synth request from Hermes got 5 strategist tasks whose 11 checks grep the hello-world demo's plan.md (pass_on_base, all red on the base)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#B-baseline"
discovered_from = "agent:hermes-pilot (round v7 baseline, 2026-10-19, op 5366ffa9)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::problems", "crates/roko-cli/src/plan_generate/runnable.rs::step_problem", "crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt"]
links = { depends_on = [], blocks = [], related = ["gap-d21da9", "gap-ff78e1", "bug-df9aff", "bug-8fe462", "gap-143b5c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_no_task_can_change_is_settled_on_the_base' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_check_no_task_can_change_is_settled_on_the_base"
+++

## Problem

Round v7 baseline (hermes-pilot, 2026-10-19; roko 3b34aed92; plugin 0.6.0; Hermes 0.22.2, chat model gpt-5.4-mini):

- The workspace is `/Users/Shared/undertow-oneliner`: a git repo with only a README, then `roko init` and
  `hermes roko init`. The latter adds `plans/demos/demo-hello-world`, so its `plan.md` and `tasks.toml` are the only
  files under `plans/`.
- The request, in a Hermes desktop chat: "Use roko to write a plan for this, and don't run anything: a vst3
  synthesizer plugin with the aesthetics from bardo/prd/18-interfaces for hypnotic techno basslines / sub / rumbles,
  for macOS. Cap the plan writing at $0.40."
- Following roko:plan's step 1, the chat model sent `roko_plan_generate` this text: "Create a roko plan for a macOS
  VST3 synthesizer plugin aimed at hypnotic techno basslines, sub, and rumbles, with aesthetics inspired by
  bardo/prd/18-interfaces. The plan should cover product definition, sound engine architecture, UI/UX direction,
  plugin format and macOS packaging, implementation milestones, test/verification strategy, and risks. Constraints:
  do not run anything now; only write the plan. Cap planning spend at $0.40 total."
- Operation `5366ffa9` wrote `plans/create-a-roko-plan-for-a-macos-vst3-synthesizer`. It took 30 s and 3 planner calls
  on gpt-5-4-mini (the chat-host planner rule's strong rung), for $0.044. The plan:
  - has 5 tasks, all `role = "strategist"`, each with `files = []` and `allowed_tools = []`;
  - has 11 verify steps, and every one is `grep -q '<phrase>' plans/demos/demo-hello-world/plan.md` (one greps that
    plan's `tasks.toml`), with `phase = "structural"` and `expect = "pass_on_base"`.
- **All 11 fail on the base.** The phrases ("product definition", "sound engine architecture", "UI/UX direction" …)
  are not in the demo's plan.md, which has 16 lines.
- **No task can change them.** The tasks write nothing, and the file belongs to another plan. Run as written, every
  task fails on every rung. The only way an agent could pass is to edit the hello-world demo's plan.md.
- **Nothing flagged it.**
  - The generation reported "succeeded · 5 tasks".
  - The roko page showed a plausible plan: "5 tasks · 4 waves · likely $0.03 · worst $0.43".
  - `roko plan validate --spec-quality --dynamic` printed 0 diagnostics and grades of 4 B and 1 C (see the related
    validate item).

## Why it matters

The golden path's first step in a fresh project is "ask for a plan". This plan looks runnable and cannot pass. It is
the "no spec" contrast in round v7's guide: one sentence in, an unrunnable plan out, at $0.054 (roko) + $0.021 (chat).

## Where

- `crates/roko-cli/src/plan_generate.rs`, the generator system prompt (around line 380), says: "architect/researcher/
  strategist: MUST have only structural checks on files that already exist (e.g. `grep -q ...`). These roles cannot
  write …". A role that cannot write can never turn a check from red to green. So such a check either already passes
  on the base, and checks nothing, or fails for ever. With only the demo plan's files in the repo, the planner
  pointed the greps there.
- `crates/roko-cli/src/plan_generate/runnable.rs::problems` and `step_problem`, the generation-time check from
  gap-ecafde. It asks whether a step's program is installed, whether its files exist or are written before it runs,
  and whether the tests it names are defined. A grep of an existing file passes all three.

## Plan

1. At generation, find each verify step whose files all exist and that no task of the plan writes (no task's
   `files`, per `written_before_and_by`). Such a step's result is fixed by the base, so run it once there. It is
   cheap: skip it if it runs cargo.
   - Failing: it is `cannot_run` ("no task writes `plans/demos/demo-hello-world/plan.md`, so this fails on the base
     and always will"). Ask the planner again, as for the other cannot-run reasons.
   - Passing: it is shallow.
2. A step that reads another plan's directory (`plans/<other>/…`) or roko's own state (`.roko/`) is never a task's
   check. Refuse it at generation, and warn about it in `plan validate`.
3. In the prompt, a request to build software gets implementer tasks. Design notes go in a file that a task writes,
   for example a scribe task's doc that its grep then checks, so the check can move. Also say that a host's "create a
   plan for X … only write the plan" means the plan roko writes is that plan, and its tasks build X.
4. The plugin side (roko:plan's step 1 invites the rewording) is reported separately to its owners.

## Done when

Generation replays this plan through a fake planner, reports its 11 steps as cannot-run, and asks again. A named test
covers it: see the `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/baseline/`, with the plan, the validate output, the chat transcript,
  the operation record and costs.jsonl.
- Related:
  - gap-d21da9: `validate --dynamic` runs none of these steps;
  - gap-ff78e1: validate lacks the generation's runnable checks;
  - bug-df9aff and bug-8fe462: earlier plan-writing defects;
  - gap-143b5c: the fourth planner call, $0.0096 after the operation closed, is outside the operation's $0.044.
