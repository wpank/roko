+++
id = "bug-d758f6"
kind = "bug"
title = "The plan generator's examples teach every task a restricted tool set (read_file, grep, write_file; no bash, no edit_file), so planners copy it and implementers in compiled projects can't build or run their own checks"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#C-p0-tools"
discovered_from = "agent:hermes-pilot (round v7 phase 0, 2026-10-19, plan writes 50608b6f and 437486f8)"
anchors = ["crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT", "crates/roko-cli/src/plan_generate/runnable.rs::repair_write_tools", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::effective_agent_contract"]
links = { depends_on = [], blocks = [], related = ["gap-770cb5", "bug-df9aff", "bug-3cdc58", "gap-6754ab"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generated_implementers_whose_checks_build_keep_a_shell' crates/roko-cli/src && cargo test -p roko-cli --lib -- generated_implementers_whose_checks_build_keep_a_shell"
+++

## Problem

Round v7, Undertow phase 0 (hermes-pilot, 2026-10-19, roko 3b34aed92, chat-host planner gpt-5-4-mini). Two plans
were written from `docs/specs/phase-0-foundation.md`, a C++/CMake/JUCE project: plan writes `50608b6f` and
`437486f8`. Every task of both has:

```toml
allowed_tools = ["read_file", "write_file", "grep"]
```

None has bash, edit_file, multi_edit, apply_patch, run_tests or glob.

- Dispatch narrows each task's agent to exactly that list. So an implementer that must write a CMake build, a
  57-row parameter table, an engine and a JUCE plugin can't run `cmake`, can't run its own check (`tools/spec.sh 0
  params 'P0.01*'`), and can't patch a file, only rewrite it whole.
- Every compile error therefore costs a whole try: the model's tokens, then the gate's check, then a retry prompt.
  The plugin preset's first build alone is about 77 s.
- v6's G1 plan (pocket-synth, Python, planner gpt-oss-120b) had the same shape: `["read_file", "edit_file",
  "grep"]`. bug-df9aff's repair then added write_file, but never a shell.

## Why it matters

The pitch is cheap models doing the work and climbing only when checks fail. A cheap model that can't compile,
can't see a compiler error and can't run its check fails more tries and climbs sooner, so the ladder pays for a
missing tool, not a hard task. This hits every compiled project roko plans for: C++, Rust, Go, TypeScript with a
build step. It blocks round v7's Undertow build: the lead chose to re-plan with bash allowed rather than run as is.

## Where

- `crates/roko-cli/src/plan_generate.rs`, `PLAN_GENERATOR_SYSTEM_PROMPT`. Every example task carries a three-tool
  list:
  - `["read_file", "grep", "edit_file"]` (the minimum structure, about line 177, and the field reference, about
    260);
  - `["read_file", "grep", "write_file"]` (about 299);
  - `["read_file", "write_file", "grep"]` (the /health example, about 488).

  The role section says "Each role has a default tool permission set. Tasks can further restrict via
  `allowed_tools`/`denied_tools`, never widen it." Planners copy the example list into every task.
- `crates/roko-cli/src/graph_task_dispatch/routing_context.rs::effective_agent_contract` narrows the role contract by
  `allowed_tools`. The implementer contract (`crates/roko-agent/src/safety/contracts/implementer.yaml`) only forbids
  web_fetch and web_search. Without a list, an implementer gets the coding domain's builtins
  (`crates/roko-std/src/roles.rs`, `CODING_DOMAIN_PROFILE`): bash, run_tests, edit_file, multi_edit, apply_patch,
  glob and the rest.
- `crates/roko-cli/src/plan_generate/runnable.rs::repair_write_tools` (bug-df9aff) adds write_file where a task must
  create a file, but nothing adds a shell.
- Not the cause: runs started from Hermes set `outbound = stage`, which holds only outbound MCP and plugin calls.
  Built-ins are never outbound (`crates/roko-agent/src/safety/effects.rs`).

## Plan

Options, not exclusive:
1. Drop `allowed_tools` from the prompt's examples, and say "leave `allowed_tools` out unless the task must be kept
   from a tool (a reviewer that must not write, a docs task that must not run commands)".
2. Repair at generation, like `repair_write_tools`: an implementer whose verify steps build or run code (cmake,
   cargo, make, npm, go, python, a `tools/*.sh` that does) gets bash and edit_file added, and they are taken out of
   `denied_tools`.
3. Have `plan validate` warn about an implementer whose checks build code but who has no shell.

## Done when

A generated plan for a compiled project gives each implementer a shell. Either it has no `allowed_tools`, or the
list includes bash and edit_file. Checked by the test in `[[verify]]` (a fake planner that copies the three-tool
list, then generation's output).

## Notes

- Owner: roko-planner-2, at the top of its queue (the lead's decision, 2026-10-19).
- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-plan1/plan/tasks.toml` and `p0-plan2/plan/tasks.toml`.
- Builds an agent runs with bash go through the round's cmake/cargo shims, so they take the same heavy.sh lanes as
  the checks.
- Related:
  - bug-df9aff: write_file repair;
  - gap-770cb5: run_tests has no CMake (filed with this one);
  - gap-6754ab: cold worktree builds.
