# IMPL-03 Rewrite: Context Engineering Overview

This folder replaces `../IMPL-03-CONTEXT.md`.

## Objective

Turn the current prompt/context assembly pipeline into an explicit cognitive workspace with auctioned section selection, policy learning, caching, and optional chain/world-model inputs.

## Current codebase reality

- `roko-compose` is already wired into `orchestrate.rs`.
- `roko-neuro/src/context.rs` now contains the canonical `ContextAssembler`, re-exported by `roko-compose`.
- `roko-learn` already has `section_effect.rs`, `context_pack_cache.rs`, and active-inference-related code.
- The original plan talks about nine bidders, WorldGraph, and InsightStore integration. Some of that is specified only and not present in the current workspace yet.

## Relevant code and docs

- `crates/roko-compose/src/`
- `crates/roko-neuro/src/context.rs`
- `crates/roko-learn/src/section_effect.rs`
- `crates/roko-learn/src/context_pack_cache.rs`
- `docs/03-composition/13-current-status-and-gaps.md`
- `docs/06-neuro/16-current-status-and-gaps.md`
- `../PRD-04-CONTEXT-ENGINEERING.md`

## Deliverable split

- `01-workspace-bidders-and-policy-checklist.md`
- `02-caching-chain-and-worldgraph-checklist.md`
- `03-context-mesh-measurement-and-persistence.md`

## PRD coverage map

- PRD-04 sections 2-5.5 map to CognitiveWorkspace, VCG allocation, ContextPolicy, cross-agent aggregation, and WorldGraph injection.
- PRD-04 sections 6-12 map to section effects, cache architecture, placement, token dropping, affect modulation, HDC retrieval, and InsightStore integration.
- PRD-04 sections 13-16 map to context mesh, measurement, integration map, and persistence layout.

## Fresh-agent rules

- Start from the existing `ContextAssembler`; do not fork prompt assembly into a second competing implementation.
- Each bidder must produce an inspectable score and provenance.
- Cache keys must be deterministic and explainable.
