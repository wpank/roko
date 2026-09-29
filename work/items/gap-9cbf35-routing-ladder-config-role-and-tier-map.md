+++
id = "gap-9cbf35"
kind = "gap"
title = "Routing ladder config: role and tier map to a model rung, with the D11 cascade as the default"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-core/config", "roko-cli/dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/DECISIONS.md (D11); tldr/05 §6 decisions 1 and 3; tldr/research/B3-routing-cost.md (Gaps 1)"
anchors = ["crates/roko-core/src/config/routing.rs::RoutingConfig", "crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter::route", "crates/roko-cli/src/dispatch/model_routing.rs::ModelChoiceSource", "crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::new", "roko.toml"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-8c0a20"], blocks = [], related = ["gap-460230", "gap-dbf2a6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_ladder_follows_d11' crates/roko-core/src/ && cargo test -p roko-core --lib default_ladder_follows_d11"

[[verify]]
command = "grep -rqw 'fn ladder_routes_by_role_and_tier' crates/roko-cli/src/ && cargo test -p roko-cli --lib ladder_routes_by_role_and_tier"
+++

## Problem

No setting says "run this role's mechanical tasks on the cheapest model, and move up from there".
`routing.{standard,complex}_task_model` are display-only on the Graph path (`graph_engine_inert_settings`),
`fast_task_model` serves only express mode (off), and plan runs ignore `[agent.roles.<role>].model` (B3). A run is
cheap only if the author pins `model_hint` on every task, or passes `--model` for every role.

## Why it matters

D11 fixes the executor cascade: gpt-oss-120b → GLM-4.7 → gpt-5.4-mini, at most 2 escalations; PLAN.md adds Sonnet as
the last production rung. tldr/05 decision 3 (default): a config ladder maps tier and role to models, and the router
learns only within it. Rung hints (gap-dbf2a6) and escalation (gap-460230) move along this ladder. Step 3 of epic
spec-98f76d.

## Where

- `crates/roko-core/src/config/routing.rs::RoutingConfig` (`deny_unknown_fields`): a `ladder` field and a resolver.
- `crates/roko-cli/src/dispatch/model_routing.rs`: a ladder stage in `ModelRouter::route` (:341) and a
  `ModelChoiceSource::Ladder` variant.
- `crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::new`, which builds the router, and `roko.toml`.

## Current state

Checked at `41c7ffbd6`. `ModelRouter::route` goes `force_backend`, `task_model_hint`, cascade router, then guards that
fall back to `claude-sonnet-4-6`. `ModelRegistry::upgrade_tier` (`config/registry.rs`) has no outside caller and picks
any model of the next capability class, not an ordered rung. `roko.toml` has no GLM-4.7 or gpt-5.4-mini model, though
the `zai` and `openai` providers exist.

## Plan

1. Config, with built-in defaults (rung names are illustrative):
   ```toml
   [routing.ladder]
   enabled = true
   rungs = [{ name = "cheap", model = "cerebras-gptoss" }, { name = "mid", model = "glm-4-7" },
            { name = "strong", model = "gpt-5-4-mini" }, { name = "top", model = "claude-sonnet" }]
   start = { mechanical = "cheap", focused = "cheap", integrative = "mid", architectural = "top" }
   # [routing.ladder.roles.<role>] may override `start` or `rungs` for one role.
   ```
2. `LadderConfig::resolve(role, TaskTier)` returns the rungs and the start rung, skipping rungs whose model has no
   configured provider (logged once). If none is left, the ladder is off for that task.
3. In `ModelRouter::route`, after `force_backend` and `task_model_hint`, return the start rung's model as
   `ModelChoiceSource::Ladder { rung }`, under the existing guards. Still log the cascade router's pick (shadow).
4. The routing observation sink records ladder outcomes like router outcomes, so the learner sees every rung.
5. `roko.toml`: add `glm-4-7` (`zai`) and `gpt-5-4-mini` (`openai`) with the providers' exact slugs, and the ladder
   block, in the same commit as the type. Mark the three `*_task_model` keys deprecated.
6. Tests: `default_ladder_follows_d11` (roko-core) and `ladder_routes_by_role_and_tier` (the tier's start rung; a role
   override; a skipped rung; a hint and `--model` still win).

## Done when

- [ ] With no hint, a mechanical implementer task routes to the first configured rung and is recorded as `ladder`.
- [ ] A workspace with only Claude keys routes as before.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Needs gap-8c0a20's `TaskTier`. Planning on Opus 5.5 is gap-853b31's `[authoring] planner_model`.
- Whether the ladder is on by default is the author's call (epic Notes).
- **Hot files:** `dispatch/model_routing.rs` and `dispatch/factory.rs`. S01.P0-8 edits `route_logged` in the same file.
- **Decided 2026-09-29 (Will):** the ladder is on by default. Rungs whose model has no configured provider are skipped.
