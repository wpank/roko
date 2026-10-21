+++
id = "bug-a8a787"
kind = "bug"
title = "A task's declared files share the 4,000-token context layer with its structured context and come first, so a whole spec read (phase 4's, 21 KB numbered) cuts off the task's other reads, Files in scope, acceptance and verify commands"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#N-context-cap"
discovered_from = "agent:hermes-pilot (round v7, phase 4 plan, captest-proof)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context", "crates/roko-compose/src/system_prompt_builder.rs::section_budget_cap"]
links = { depends_on = [], blocks = [], related = ["bug-60ad88"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_whole_spec_and_its_headers_fit_the_file_context_or_say_what_is_cut' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_whole_spec_and_its_headers_fit_the_file_context_or_say_what_is_cut"
+++

## Problem

A task's pre-loaded context, the system prompt's "## Relevant Context" section (`context_layer`), is capped at the
role's `runner_context` budget: 4,000 tokens for an implementer (`roko-compose` `templates/common.rs::budget_for`;
`system_prompt_builder.rs::section_budget_cap`). `PromptSection::enforce_hard_cap` (`prompt.rs`) keeps the first
cap x 4 bytes and appends "…[truncated N tokens]".

`build_runner_context` (`roko-cli` `dispatch/prompt_builder.rs`) puts the declared files first (`[task.context]
read_files`, rendered by `plan_policy::render_declared_context`), then Files in scope, acceptance criteria, verify
commands, gate feedback, Prior Task Outputs and Sibling Tasks. So a large read crowds out everything after it:

- hermes-pilot's $0 proof (`tmp/hermes-roko-v7/pilot/captest-proof/`, bodies/05-request.json): phase 4's spec
  `docs/specs/phase-4-ui.md` (16,938 B, 264 lines, about 21 KB with line numbers), read whole, is cut at "212 | ##
  Rules every tas…[truncated 2603 tokens]". The task's other declared file (Theme.h), Files in scope, the acceptance
  criteria and the verify commands are all gone.
- In Undertow's phases 0-3, 39 of the 57 tries that recorded a context layer sat at 4,006-4,007 tokens: the cap plus
  the marker.
- bug-60ad88 now puts the plan's source document first in every task's reads, which makes this worse unless the
  layer changes.

The role budget already has room meant for files, `file_context` (8,000 tokens for an implementer, more for a
Complex task), but no section uses it.

## Why it matters

The structured parts of the context say what the task must do and how it is checked. Losing them, or the spec's
rules, silently, is how a cheap model ends up guessing.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context`, and its caller that builds the role's
  TaskContext.
- `crates/roko-cli/src/plan_policy.rs::render_declared_context`.
- `crates/roko-compose/src/role_prompts.rs` (TaskContext), `system_prompt_builder.rs` (sections, budget caps),
  `prompt.rs::enforce_hard_cap`.

## Plan

1. Order: the context layer holds only the small structured parts, so a cut never reaches them.
2. Budget: declared file contents get a section of their own after it, under the role's `file_context` budget. A role
   whose `file_context` is 0 keeps them at the end of the context layer.
3. Fit in roko-cli before the cap: the plan's `[meta] sources` documents come first and are cut last; the other reads
   keep their order; a read cut at a line boundary, or left out, is named: "[docs/specs/phase-4-ui.md lines 212-264
   not shown: read_file them]". Nothing reaches `enforce_hard_cap` over the cap.

## Done when

The test in `[[verify]]` passes: phase 4's real spec and two editor headers, under an implementer's budget, arrive
whole or with each cut named, and the structured parts always arrive.
