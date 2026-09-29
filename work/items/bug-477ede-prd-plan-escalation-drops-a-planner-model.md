+++
id = "bug-477ede"
kind = "bug"
title = "prd plan escalation drops a planner model outside the haiku/sonnet/opus chain to the cheapest model"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-planner's report on gap-853b31)"
anchors = ["crates/roko-cli/src/prd.rs::next_tier_model", "crates/roko-cli/src/prd.rs:1780"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-853b31", "bug-12153c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn next_tier_model_never_downgrades_an_unknown_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib next_tier_model_never_downgrades_an_unknown_model"
+++

## Problem

When `roko prd plan` produces TOML that fails extraction or validation, it retries up to twice. With `[agent.escalation] escalate_model` on (the default, roko-cli `config.rs`), each retry calls `next_tier_model(current, tier_models, configured_models)`. The chain is `agent.tier_models`' `haiku`, `sonnet` and `opus` entries, or `DEFAULT_ESCALATION_CHAIN` (`claude-haiku-4-5`, `claude-sonnet-4-6`, `claude-opus-4-6`). If the current model is not in the chain, `position` is `None`, the candidates are the whole chain, and the function returns the first configured entry: the cheapest model. A frontier planner such as `claude-opus-5-5`, or any model key outside the chain, is "escalated" to Haiku on the retry.

## Why it matters

Specs a cheap model can execute (epic spec-e57870) depend on a frontier model writing the plan. gap-853b31 sends every plan generate and revise path through a configured planner model, which makes this the normal case: one validation failure silently hands plan authoring to the weakest model, and the retry is more likely to fail again.

## Where

- `crates/roko-cli/src/prd.rs::next_tier_model` (:1285) and `DEFAULT_ESCALATION_CHAIN` (:1273).
- The retry loop in `generate_plan_from_prd_with_outcome` (about :1760-1800).

## Current state

At BASE the starting model is `--model` or `agent.model`; on `work/gap-853b31` it is the planner model. The existing tests (`next_tier_model_escalates_with_empty_configured_set`, `next_tier_model_skips_unconfigured`) only use models that are in the chain.

## Plan

1. When the current model is not in the chain, return `None` and retry on the same model. Or rank models by tier or cost and only move upward. Never return a model ranked below the current one.
2. Add `next_tier_model_never_downgrades_an_unknown_model`: `next_tier_model(Some("claude-opus-5-5"), …)` with Haiku configured returns `None` or a model at least as strong.

## Done when

- [ ] A retry never moves plan authoring to a cheaper tier than the model that failed.
- [ ] The `[[verify]]` command passes.

## Notes

- `agent.tier_models` is dropped on load today (bug-12153c), so the chain in practice is always the default one.
