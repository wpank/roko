# IMPL-01 Rewrite: Runtime Extraction Overview

This folder replaces `../IMPL-01-RUNTIME.md` with smaller execution files.

## Objective

Extract the current runtime logic out of the monolithic CLI/orchestration path and move Roko toward an agent-runtime model with:

- explicit runtime state and event types;
- extension-based composition;
- domain-aware agent startup;
- a migration path that does not break the existing `roko plan run` loop.

## Current codebase reality

- Runtime infrastructure already exists in `crates/roko-runtime/src/`.
- The live execution loop still centers on `crates/roko-cli/src/orchestrate.rs`.
- Scheduling/execution state already exists in `crates/roko-orchestrator/src/`.
- Agent factory and execution entrypoints exist in `crates/roko-agent/`, `crates/roko-cli/src/agent_exec.rs`, `crates/roko-cli/src/agent_spawn.rs`, and `crates/roko-cli/src/chat.rs`.
- The original plan mentions target crates such as `roko-ext-core`, `roko-ext-code`, and `roko-ext-chain`. Those crates do not exist yet in the workspace.

## Relevant code and docs

- Runtime: `crates/roko-runtime/src/lib.rs`, `heartbeat.rs`, `event_bus.rs`, `process.rs`, `lifecycle.rs`
- CLI/orchestration: `crates/roko-cli/src/orchestrate.rs`, `run.rs`, `chat.rs`, `agent_spawn.rs`, `agent_exec.rs`
- Orchestrator: `crates/roko-orchestrator/src/lib.rs`, `executor/`, `plan_discovery.rs`
- Agent status/gaps: `docs/02-agents/15-status-gaps.md`
- Crate boundaries: `docs/00-architecture/15-crate-map.md`
- Original source plans: `../PRD-02-AGENT-RUNTIME.md`, `../IMPL-01-RUNTIME.md`

## Deliverable split in this folder

- `01-foundation-and-extraction-checklist.md`
- `02-migration-verification-and-cutover.md`
- `03-heartbeat-timescales-inference-gateway-and-ops.md`

## PRD coverage map

- PRD-02 sections 2-4 map to heartbeat pipeline, timescales, and concurrent mechanisms.
- PRD-02 sections 5-10 map to extensions, type-state, CorticalState, event fabric, and process supervision.
- PRD-02 sections 11-12 map to backwards compatibility and crate layout.
- PRD-02 sections 13-16 map to the unified tick narrative, performance targets, inference gateway, and updated extension payloads.

## Fresh-agent rules

- Preserve the working `roko plan run` path while refactoring.
- Prefer transitional adapters over big-bang replacement.
- Treat new extension crates as optional scaffolds until the dependency graph is explicit in `Cargo.toml`.
- If a new runtime type overlaps with existing orchestrator or runtime types, define the ownership boundary before coding.

## Done when

- there is a documented runtime core with explicit ownership boundaries;
- extension loading order and hook points are specified against existing code;
- migration steps preserve CLI compatibility;
- integration tests prove the old and new paths agree on lifecycle, routing, and event emission.
