# IMPL-06 Rewrite: ISFR And Instruments Overview

This folder replaces `../IMPL-06-ISFR.md`.

## Objective

Build the ISFR benchmark index as a concrete first financial primitive, then layer prediction, perps, and cooperative clearing on top of it.

## Current codebase reality

- `crates/roko-chain/src/isfr.rs` already exists.
- `roko-chain` has types, wallet/client abstractions, marketplace, futures, and witness-related files, but much of the intended Korai system is still built or scaffolded rather than deployed.
- `apps/mirage-rs` can be used as the test harness for chain-like behavior before real chain deployment.

## Relevant code and docs

- `crates/roko-chain/src/isfr.rs`
- `crates/roko-chain/src/futures_market.rs`
- `crates/roko-chain/src/types.rs`
- `apps/mirage-rs/src/http_api/isfr.rs`
- `docs/08-chain/24-current-status-and-6-contracts.md`
- `../PRD-07-ISFR-AND-INSTRUMENTS.md`

## Deliverable split

- `01-oracle-prediction-and-perps-checklist.md`
- `02-clearing-runtime-and-verification-checklist.md`
- `03-publication-states-economics-and-credibility.md`

## PRD coverage map

- PRD-07 sections 2-5 map to benchmark definition, source composition, computation, and publication/circuit-breaker states.
- PRD-07 sections 6-10 map to knowledge production, yield perps, hedging examples, clearing profiles, and cooperative clearing.
- PRD-07 sections 12-21 map to benchmark generalization, credibility path, solver economics, Korai gaps, EventFabric integration, cross-domain use, ecosystem roles, and multi-chain sources.

## Fresh-agent rules

- Use mirage or mocks first; do not require a live Korai chain for basic correctness.
- Treat ISFR as the first `BenchmarkIndex` implementation, not a one-off special case.
- Every market primitive must have failure-mode tests.
