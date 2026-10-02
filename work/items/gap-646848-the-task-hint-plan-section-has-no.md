+++
id = "gap-646848"
kind = "gap"
title = "The task hint plan_section has no consumer since the PRD excerpt was removed"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-core/task"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/workflow-audit (roko-7d migration, 2026-10-02)"
discovered_from = "audit:tmp/workflow-audit/"
anchors = ["crates/roko-core/src/task.rs", "crates/roko-cli/src/task_parser.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qw plan_section crates/roko-core/src/task.rs || grep -qw plan_section crates/roko-cli/src/dispatch/prompt_builder.rs"
+++

## Problem

The task hint `plan_section` has no consumer. It narrowed the `# PRD Requirements` excerpt of the dispatch prompt to
one section of the PRD; that excerpt (`prd_excerpt`) went with the PRD pipeline on 2026-10-02
(`tmp/workflow-audit/`). Plans can still set it and the parser accepts it, but nothing reads it.

## Why it matters

A plan author (or the planner model) who sets `plan_section` expects the agent's context to change; it does not.

## Where

- `crates/roko-core/src/task.rs`: `plan_section` field of the hint structs (around :547 and :725)
- `crates/roko-cli/src/task_parser.rs`: `plan_section` in the known hint keys (around :1023) and test literals

## Current state

The only reader was `load_prd_excerpt` in `crates/roko-cli/src/dispatch/prompt_builder.rs`, removed with the
`prd_excerpt` section.

## Plan

Pick one:
1. Give it a use: narrow the plan's `plan.md` context the prompt builder includes to the named section.
2. Drop it: remove the field and the known-key entry. Keep parsing old plans that set it: an unknown hint key
   is only a warning, so they still load (check `TasksFile::parse_str` on a fixture).

## Done when

- `plan_section` is either gone from `roko-core`'s task hints or read by the prompt builder. Verify:
  `! grep -qw plan_section crates/roko-core/src/task.rs || grep -qw plan_section crates/roko-cli/src/dispatch/prompt_builder.rs`

## Notes

- Found by the workflow-audit migration (session roko-7d, 2026-10-02).
