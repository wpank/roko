+++
id = "bug-e37197"
kind = "bug"
title = "Prompt assembly resolves role `reviewer` to the implementer template"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-compose/prompt-assembly"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-compose/src/prompt_assembly_service.rs::resolve_role", "crates/roko-core/src/agent.rs::AgentRole"]
links = { depends_on = [], blocks = [], related = ["bug-db607b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "alias = \"reviewer\"" crates/roko-core/src/agent.rs || grep -q "\"reviewer\" =>" crates/roko-compose/src/prompt_assembly_service.rs'
+++

`resolve_role` matches a task's role against `AgentRole` labels and serde names and falls back to `AgentRole::Implementer`. `reviewer` is neither: the variant is `QuickReviewer`, labelled `quick-reviewer`, with no `reviewer` alias. A `role = "reviewer"` task is therefore assembled with the implementer template and section budget. The plan validator already parses `reviewer` as `QuickReviewer` (bug-db607b), so validation and prompt assembly disagree.

Fix: map `reviewer` to `QuickReviewer` in one place (a serde alias on the variant) and test that a reviewer task gets the reviewer template.

Rechecked 2026-09-29: still true. Scope: the defect is in roko-compose's PromptAssemblyService (built by crates/roko-serve/src/service_factory.rs:331 and :511). Graph plan dispatch uses crates/roko-cli/src/dispatch/prompt_builder.rs::parse_role_label, which already maps reviewer to QuickReviewer, so of the three role parsers (plan_validate.rs::parse_task_role, parse_role_label, resolve_role) only resolve_role disagrees.

## Notes

- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check.
  `AgentRole::QuickReviewer` gains `#[serde(alias = "reviewer")]`, so `resolve_role` gives a `reviewer` task the
  reviewer's template. Its label and serialization stay `quick-reviewer`. Tests:
  `resolves_role_labels_and_serde_names` (roko-compose) and `serde_kebab_case_roundtrip` (roko-core).
