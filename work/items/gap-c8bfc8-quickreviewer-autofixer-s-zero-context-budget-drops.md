+++
id = "gap-c8bfc8"
kind = "gap"
title = "QuickReviewer/AutoFixer's zero context budget drops verify commands and gate feedback, not just cross-plan context"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-compose/budget", "roko-cli/prompt-builder"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK10)"
discovered_from = "PK10's reviewer-role prompt work"
anchors = ["crates/roko-compose/src/templates/common.rs", "crates/roko-compose/src/system_prompt_builder.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn quick_reviewer_prompt_includes_verify_commands_and_gate_feedback' crates/roko-cli/ && cargo test -p roko-cli quick_reviewer_prompt_includes_verify_commands_and_gate_feedback"
+++

## Problem

`QuickReviewer` and `AutoFixer`'s `PromptBudget` both set `context: 0`
(`crates/roko-compose/src/templates/common.rs:91-108`). That field caps the `"context_layer"` section
(`system_prompt_builder.rs:913`: `"domain_context" | "context_layer" | "pheromone_signals" => Some(budget.context)`),
and `context_layer` is not a narrow "cross-plan context" extra — it is the single block
`dispatch/prompt_builder.rs::build_runner_context` assembles, whose own doc comment says exactly what it carries:
"files-in-scope, acceptance criteria, verify commands, gate retry feedback, dependency outputs, workspace map,
tasks toml, workspace context, and C-factor context" (`prompt_builder.rs:1534-1538`), fed into the canonical prompt
via `TaskContext::with_context` (`prompt_builder.rs:2033`, confirmed by `assemble`'s own comment at
`:1971-1974`: "Runner-specific context (files in scope, acceptance criteria, verify commands, gate feedback,
dependency outputs, workspace map, etc.) is mapped into `TaskContext::with_context`").

A budget of 0 for that one field removes the *entire* block for QuickReviewer and AutoFixer, not just the parts
that might legitimately not apply to a review/fix role. The adjacent test comment
(`crates/roko-compose/src/budget.rs:259`, "QuickReviewer has no cross-plan context") suggests `context` was
designed to mean something narrower — domain/pheromone-style context from *other* plans — before
`build_runner_context`'s much richer, *this-task's-own* payload (verify commands, gate feedback, files in scope)
was routed through the same budget field. The two concerns now share one knob.

## Why it matters

Goal: a code reviewer or auto-fixer that cannot see which files are in scope, what the verify commands are, or
what the gate's retry feedback said has to re-derive or guess at exactly the information its job most depends on.
`file_context` (a separate budget field, also 0 for both roles) legitimately drops raw file contents for a
lighter-weight role, but `context_layer`'s verify-command and gate-feedback content is not duplicated anywhere
else in the prompt for these roles (confirmed: no other section/test references passing verify commands or gate
feedback to QuickReviewer/AutoFixer specifically). This likely degrades review and auto-fix quality silently —
nothing fails loudly, the reviewer or fixer just reasons without the acceptance criteria it is meant to check
against.

## Where

- `crates/roko-compose/src/templates/common.rs::budget_for` (or its match arms, ~lines 91-108): `PromptBudget`
  literals for `AgentRole::QuickReviewer` and `AgentRole::AutoFixer`, both `context: 0`.
- `crates/roko-compose/src/system_prompt_builder.rs:913`: the budget lookup mapping `context_layer` to
  `budget.context`.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context` (~line 1534) and `::assemble` (~line
  1971): what actually goes into the block, and where it is attached via `TaskContext::with_context`.
- `crates/roko-compose/src/budget.rs` tests (~205-270): `auto_fixer_trivial_drops_nothing_already_zero` and
  `per_role_budgets_differ`/`reviewer_gets_reviews_budget` pin the current (zero) values as intentional, without
  a comment distinguishing "cross-plan context" from "this task's own runner context."

## Current state

Unaddressed. The zero values are deliberate at the budget-table level (pinned by tests), but nothing suggests
anyone considered that `build_runner_context`'s payload would ride the same field once it was added.

## Plan

1. Either split the budget: a new field (e.g. `runner_context`) for `build_runner_context`'s payload (files in
   scope, verify commands, gate feedback, dependency outputs), separate from `context` (cross-plan
   domain/pheromone context), with QuickReviewer/AutoFixer getting a non-zero `runner_context` budget and keeping
   `context: 0`; or
2. Give QuickReviewer and AutoFixer a modest non-zero `context` budget (team-lead/Will's call on the number), if
   splitting the field is too large a change for this item's scope.
3. Add a test asserting QuickReviewer's/AutoFixer's assembled system prompt contains the verify commands and gate
   feedback for a task that has them, not just that `budget.context` is some non-zero number.

## Done when

- A QuickReviewer or AutoFixer system prompt for a task with verify commands and gate retry feedback actually
  contains them.
- The `[[verify]]` command passes.

## Notes

- Do not change `file_context` (raw file contents) for these roles without separate justification — this item is
  scoped to the structured `context_layer` block only (verify commands, gate feedback, files *in scope* as a
  list/summary, not full file bodies).
- `AutoFixer`'s `plan`, `workspace_map`, `brief` and `reviews` are also 0 — out of scope here unless the same
  "two concerns, one budget field" pattern applies to one of them too; this item only confirms `context`.

## Progress

- gap-c8bfc8: implemented at 0c9a3c96b, option 1. `PromptBudget` gains `runner_context`, the cap of the `context_layer` and `gate_feedback` sections; `context` now caps only cross-plan context (domain context, pheromone signals). Every role's `runner_context` equals its old `context`, except QuickReviewer and AutoFixer, which get 2,000 and keep `context: 0`; a trivial task keeps its runner context and a complex one doubles it. The CLI prompt builder gives a role with no cross-plan context only its verify commands and, on a retry, the failing gate's feedback; it still builds the full block, so a task's declared context is checked as before. The Notes' question: AutoFixer's `plan: 0` also drops the system prompt's `task_context` section, but the task's title, details and TSS sections still reach it in the user prompt, so nothing is lost there. The verify's grep passes; cargo verification is deferred to the batch gate (`quick_reviewer_prompt_includes_verify_commands_and_gate_feedback`, `review_and_fix_roles_keep_a_runner_context`, `trivial_keeps_the_runner_context`).
