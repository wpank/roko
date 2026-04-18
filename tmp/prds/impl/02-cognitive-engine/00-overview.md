# IMPL-02 Rewrite: Cognitive Engine Overview

This folder replaces `../IMPL-02-COGNITIVE-ENGINE.md`.

## Objective

Implement the cost-aware cognitive path that decides when work stays at a cheap reactive tier and when it escalates to model-backed reasoning.

## Current codebase reality

- `roko-learn` already contains cost, routing, provider health, and active-inference-related building blocks.
- `roko-daimon` already exposes PAD state, behavioral modulation, and somatic-landscape hooks.
- `roko-chain` already has `triage.rs` and `isfr.rs`.
- The main dispatch path still runs through `crates/roko-cli/src/orchestrate.rs`.
- The original plan assumes a cleaner runtime boundary than currently exists, so cognitive-gate work must be staged against the live CLI/orchestrator path first.

## Relevant code and docs

- `crates/roko-learn/src/active_inference.rs`
- `crates/roko-learn/src/cost_table.rs`
- `crates/roko-learn/src/provider_health.rs`
- `crates/roko-daimon/src/lib.rs`
- `crates/roko-chain/src/triage.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `docs/03-composition/13-current-status-and-gaps.md`
- `docs/09-daimon/13-current-status-and-gaps.md`
- `../PRD-03-COGNITIVE-ENGINE.md`

## Deliverable split

- `01-prediction-gating-and-triage-checklist.md`
- `02-native-harness-costs-and-verification.md`
- `03-thresholds-cascade-router-and-measurement.md`

## PRD coverage map

- PRD-03 sections 3-4 map to tiering and prediction error.
- PRD-03 sections 5-8 map to adaptive thresholds, habituation, somatic markers, and affect coupling.
- PRD-03 sections 9-12 map to triage, ISFR prediction, mortality/cost clocks, and cascade routing.
- PRD-03 sections 13-19 map to validation, native harness migration, blue-ocean features, inference gateway, implementation map, and summary.

## Fresh-agent rules

- Do not add a second routing brain if one can be expressed as policy over the existing dispatch path.
- Reuse Daimon and Learn signals instead of inventing separate affect/cost stores.
- Every new escalation rule must name its observable inputs and fallback behavior.
