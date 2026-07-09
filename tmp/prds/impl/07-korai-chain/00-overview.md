# IMPL-07 Rewrite: Korai Chain Overview

This folder replaces `../IMPL-07-CHAIN.md`.

## Objective

Define the path from the current chain abstractions and mirage harness to a real Korai execution environment with precompiles, InsightStore, and token economics.

## Current codebase reality

- `roko-chain` is built enough to provide client/wallet/types, some market logic, and ISFR-related code.
- `apps/mirage-rs` is the practical test harness today.
- Consensus, execution engine, and many contracts/precompiles remain largely specified.
- This work should be treated as a staged chain-program effort, not a near-term dependency for the core self-hosting loop.

## Relevant code and docs

- `crates/roko-chain/src/`
- `apps/mirage-rs/src/chain/`
- `docs/08-chain/24-current-status-and-6-contracts.md`
- `docs/14-identity-economy/`
- `../PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md`

## Deliverable split

- `01-consensus-execution-and-precompiles-checklist.md`
- `02-insightstore-tokenomics-and-hdc-checklist.md`
- `03-identity-registries-proof-log-and-rollout.md`

## PRD coverage map

- Chain-core protocol work in this folder covers the precompile, InsightStore, and token/economic sides of PRD-07 and the chain-facing parts of PRD-09.
- Identity, reputation, validation, and proof-log registry work is staged as explicit chain backlog here even where contracts are still simulator-first.

## Fresh-agent rules

- Treat mirage as the first proving ground.
- Do not claim shipping chain features when only simulator support exists.
- Keep chain-core, precompile, and economic logic separated so each can be tested independently.
