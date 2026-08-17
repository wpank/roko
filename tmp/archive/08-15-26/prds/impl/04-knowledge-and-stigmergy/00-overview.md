# IMPL-04 Rewrite: Knowledge And Stigmergy Overview

This folder replaces `../IMPL-04-KNOWLEDGE.md`.

## Objective

Turn the existing neuro, learning, and dream scaffolds into a coherent knowledge pipeline with HDC-backed retrieval, publishing controls, and later chain sharing.

## Current codebase reality

- `roko-neuro` already has `KnowledgeEntry`, `KnowledgeStore`, `Distiller`, `TierProgression`, and `ContextAssembler`.
- `roko-primitives` already has the HDC vector implementation.
- `roko-learn` already has HDC clustering, fingerprinting, resonant patterns, and episode logging.
- `roko-dreams` exists but remains scaffold-heavy.
- Coordination/pheromone systems are still largely specified, not implemented.

## Relevant code and docs

- `crates/roko-neuro/src/`
- `crates/roko-primitives/src/`
- `crates/roko-learn/src/episode_logger.rs`
- `crates/roko-learn/src/hdc_fingerprint.rs`
- `crates/roko-learn/src/hdc_clustering.rs`
- `crates/roko-dreams/src/`
- `docs/06-neuro/16-current-status-and-gaps.md`
- `docs/10-dreams/16-implementation-status.md`
- `docs/13-coordination/12-current-status-and-gaps.md`

## Deliverable split

- `01-knowledge-pipeline-and-hdc-checklist.md`
- `02-publishing-dreams-and-chain-checklist.md`
- `03-insightstore-resonance-lifecycle-and-measurement.md`

## PRD coverage map

- PRD-05 sections 2-5 map to Neuro, InsightStore, HDC, and fingerprinting.
- PRD-05 sections 6-10 map to clustering, resonance, PP-HDC, dreams, and somatic markers.
- PRD-05 sections 11-14 map to lifecycle, benchmark framework, network effects, and temporal knowledge topology.
- PRD-05 sections 17-21 map to Korai gaps, publishing defense, geometric sharing, HDC integration levels, and measurement.

## Fresh-agent rules

- Do not rewrite `roko-neuro` from scratch unless a concrete blocker is identified.
- Reuse existing HDC and distillation primitives.
- Keep privacy and publishing defenses explicit and layered.
