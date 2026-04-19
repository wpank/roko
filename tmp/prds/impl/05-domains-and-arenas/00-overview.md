# IMPL-05 Rewrite: Domains And Arenas Overview

This folder replaces `../IMPL-05-DOMAINS.md`.

## Objective

Make domain specialization first-class without forking the runtime into separate products, and build arena-based evaluation loops that measure whether specialization actually helps.

## Current codebase reality

- Domain and role concepts already exist in agent/orchestrator config.
- `roko-chain` and `roko-cli/tests/e2e_domain.rs` show domain-aware behavior already exists in parts of the system.
- Arena work is mostly specified, not implemented.
- HuggingFace integration and work-market plumbing are not present as dedicated crates today.

## Relevant code and docs

- `crates/roko-cli/src/agent_config.rs`
- `crates/roko-cli/tests/e2e_domain.rs`
- `crates/roko-agent/src/` and provider/routing code
- `crates/roko-chain/src/`
- `docs/02-agents/16-domain-profiles.md`
- `docs/06-neuro/16-current-status-and-gaps.md`
- `../PRD-06-DOMAINS-AND-ARENAS.md`

## Deliverable split

- `01-domain-runtime-and-arenas-checklist.md`
- `02-domain-extensions-hf-and-market-checklist.md`
- `03-profile-catalog-custom-domains-and-scaling.md`

## PRD coverage map

- PRD-06 sections 1-6 map to one-runtime-many-domains, DomainProfile, blockchain/research/coding agents, and the arena framework.
- PRD-06 sections 7-10 map to the arena catalog, work markets, HuggingFace integration, and native SWE-bench execution.
- PRD-06 sections 11-13 map to custom domain creation, generalized benchmark indices, and scaling/network effects.

## Fresh-agent rules

- Domain profiles should configure one runtime, not create separate orchestration stacks.
- Arenas must be measurable and reproducible.
- Do not add marketplace/settlement behavior until the benchmark or arena loop exists to exercise it.
