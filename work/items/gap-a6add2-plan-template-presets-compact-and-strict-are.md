+++
id = "gap-a6add2"
kind = "gap"
title = "Plan template presets Compact and Strict are unreachable since PRD frontmatter went"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/workflow-audit (roko-7d migration, 2026-10-02)"
discovered_from = "audit:tmp/workflow-audit/"
anchors = ["crates/roko-cli/src/plan_generate.rs::PlanTemplateKind"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-3eef6b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q -- '--template' crates/roko-cli/src/commands/plan.rs || ! grep -qw Compact crates/roko-cli/src/plan_generate.rs"
+++

## Problem

The plan-generation presets `PlanTemplateKind::Compact` and `PlanTemplateKind::Strict`
(`crates/roko-cli/src/plan_generate.rs`) are unreachable from any command. A PRD's frontmatter picked them; the
PRD pipeline went on 2026-10-02 (`tmp/workflow-audit/`), so production generation always uses `Default`, and
only `DefaultPlanGenerator`'s tests name the others.

## Why it matters

The presets carry real behaviour (task budget, gate strictness guidance, default model tier) that a user cannot
select. Either expose it or delete it.

## Where

- `crates/roko-cli/src/plan_generate.rs::PlanTemplateKind` (`resolve`, the preset values)
- `crates/roko-cli/src/plan_generate/pipeline.rs::PlanRequest` (where a selection would be passed)
- `crates/roko-cli/src/commands/plan.rs` (`roko plan generate` flags) and
  `crates/roko-cli/src/commands/run_cmd.rs` (`roko run --plan`)

## Plan

Pick one:
1. Expose: `roko plan generate --template compact|strict` (and `roko run --plan --template`), passed through
   `PlanRequest`; test that the generated prompt carries the preset's budget.
2. Delete `Compact` and `Strict`, keeping `Default`'s values inline.

## Done when

- A command selects the presets, or they are gone. Verify:
  `grep -q -- '--template' crates/roko-cli/src/commands/plan.rs || ! grep -qw Compact crates/roko-cli/src/plan_generate.rs`

## Notes

- Found by the workflow-audit migration (session roko-7d, 2026-10-02). Coordinate with gap-3eef6b (the dead
  `DefaultPlanGenerator`), whose tests are the presets' only callers.
