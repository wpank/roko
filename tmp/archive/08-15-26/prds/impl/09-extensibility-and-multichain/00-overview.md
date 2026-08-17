# IMPL-09 Rewrite: Extensibility And Multi-Chain Overview

This folder replaces `../IMPL-09-EXTENSIBILITY-AND-MULTICHAIN.md`.

## Objective

Add an extension/package system, multi-profile composition, multi-chain ingestion, contract discovery, predictive foraging, and world-model hooks without destabilizing the current runtime.

## Current codebase reality

- There is no shipping `roko-ext-registry`, `roko-quickjs`, `roko-chain-ingest`, `roko-foraging`, or `roko-worldgraph` crate yet.
- The current workspace already has plugin/MCP surfaces, runtime event plumbing, chain abstractions, and strong learning/context primitives.
- This means the first deliverable is boundary-setting, not mass crate creation.

## Relevant code and docs

- `crates/roko-plugin/`
- `crates/roko-agent/`
- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-chain/src/`
- `docs/15-code-intelligence/10-current-status-and-gaps.md`
- `docs/13-coordination/12-current-status-and-gaps.md`
- `../PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md`

## Deliverable split

- `01-package-and-runtime-loading-checklist.md`
- `02-ingestion-discovery-and-worldgraph-checklist.md`
- `03-finetuning-integration-and-acceptance.md`
- `04-attention-allocation-publishing-and-ecosystem-completion.md`

## PRD coverage map

- PRD-09 sections 2-8 map to packages, manifests, loaders, QuickJS, and multi-domain composition.
- PRD-09 sections 9-16 map to chain ingestion, temporal resolution, finality, connectors, foraging, discovery, WorldGraph, and active inference.
- PRD-09 sections 17-22 map to publishing, HuggingFace loop, arenas-as-packages, prior-PRD integration, scaling properties, and phased rollout.

## Fresh-agent rules

- Create new crates only when the boundary and first consumer are both clear.
- Prefer manifest-first/package-first design over ad hoc loader logic.
- Use current runtime, chain, and learn code as the initial consumers of any new surface.
