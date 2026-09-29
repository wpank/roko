+++
id = "gap-853b31"
kind = "gap"
title = "Frontier planner everywhere: an [authoring] planner_model on every plan generate and revise path"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/model_selection", "roko-cli/prd", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/research/B1-plan-authoring.md (planner model choice; tldr/05 P1 #9)"
anchors = ["crates/roko-core/src/config/schema.rs::RokoConfig", "crates/roko-cli/src/model_selection.rs::resolve_effective_model_key", "crates/roko-cli/src/prd.rs::generate_plan_from_prd_isolated", "crates/roko-cli/src/plan_authoring.rs::revise_plan_source", "crates/roko-cli/src/commands/do_cmd.rs::run_complex_path"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["bug-8b1bf8", "gap-b3e513", "gap-3bea93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn planner_model_precedence_cli_then_authoring_then_role' crates/roko-cli/src/ && cargo test -p roko-cli --lib planner_model_precedence_cli_then_authoring_then_role"

[[verify]]
command = "grep -q 'resolve_planner_model' crates/roko-cli/src/prd.rs && grep -q 'resolve_planner_model' crates/roko-cli/src/plan_authoring.rs && grep -rq 'resolve_planner_model' crates/roko-cli/src/commands/"
+++

## Problem

Which model writes a plan depends on where you ask:

- `roko prd plan`, `roko plan generate` and `roko do`'s standard band take `--model` or the strategist role's model,
  through `model_selection::resolve_effective_model_key`.
- These paths pass no model and fall back to `[agent] model` (`claude-sonnet` in this repo):
  - the portal/serve path, `generate_plan_from_prd_isolated`;
  - `roko do`'s complex band, `run_complex_path` → `generate_plan_from_prd`;
  - plan revision, `plan_authoring::revise_plan_source` (the portal's "revise").

So the author's main surface, the portal, never plans with the frontier model.

## Why it matters

The golden path starts with "a frontier model plans" (tldr/04 step 1, PARTIAL). The best plans so far were written
outside Roko, by a frontier chat session (research note B1). tldr/05 P1 #9. Part of epic spec-e57870; gap-2623b2
builds on it.

## Where

- `crates/roko-core/src/config/schema.rs::RokoConfig`: add `authoring: AuthoringConfig` (new
  `config/authoring.rs`) with `planner_model: Option<String>`, a key from `[models]`.
- `crates/roko-cli/src/model_selection.rs`: a new `resolve_planner_model(workdir, cli_model)` next to
  `resolve_effective_model_key`.
- Callers:
  - `prd.rs`: `generate_plan_from_prd`, `generate_plan_from_prd_isolated`, and `generate_plan_from_prd_with_outcome`
    where `model` is `None`;
  - `plan_authoring.rs::revise_plan_source`;
  - `commands/do_cmd.rs` (both bands), and the CLI paths in `commands/prd.rs` and `commands/plan.rs`.

## Current state

Checked at `41c7ffbd6`: no `planner_model` or `[authoring]` in `crates/`. `prd.rs:1443` falls back to
`resolved.config.agent.model`, and `revise_plan_source` uses the same key. bug-8b1bf8 (open) reports that
`POST /api/prds/{slug}/plan` still sends a hand-built prompt through `run_once`.

## Plan

1. Add `[authoring] planner_model`.
2. `resolve_planner_model` precedence: `--model`, then `[authoring] planner_model`, then the strategist role's model,
   then `[agent] model`. Print the choice to stderr, as the existing selection does.
3. Call it on every generate and revise path listed above, including serve's generate and revise.

## Done when

- [ ] With `[authoring] planner_model` set, `roko prd plan`, `roko plan generate`, both bands of `roko do`, serve
      generate and serve revise all use it, unless `--model` overrides it.
- [ ] The test `planner_model_precedence_cli_then_authoring_then_role` covers the precedence.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Revision also lacks failure context (gap-3bea93) and has a single retry (gap-b3e513). Both are out of scope.
- bug-8b1bf8's `run_once` path should go through the generator. If it is still open, make it at least call
  `resolve_planner_model`.
- tldr/05 decision 1's default planner is Opus 5.5. Leave the key unset in this repo's `roko.toml` until the author
  sets it.
