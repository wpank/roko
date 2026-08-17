# tmp/light-clients/ -- What is this?

> **Status**: ARCHIVED -- Phase 2+ chain verification design (not implemented)
> **Last updated**: 2026-08-13

This directory contains detailed design and implementation documents for a **verified chain
layer** -- light client verification, state proofs, consensus adapters, and MPP (Machine
Payments Protocol) integration for the `roko-chain` crate.

## What is this for?

The goal was to let roko agents verify on-chain state (balances, storage, transfers) through
light client consensus verification rather than trusting an RPC endpoint. The docs define
22 work units (WU-1 through WU-22) organized in a 6-layer dependency graph, covering:

- Core verification traits (`ConsensusVerifier`, `VerifiedState<T>`)
- Tempo-specific BLS threshold signature verification
- EVM state proof verification via `eth_getProof` MPT proofs
- MPP client/server for agent-to-agent payments
- Dashboard and demo integration

## Implementation status

**None of these work units have been implemented.** The ISFR vertical in `roko-chain` is
wired (sources/keeper/oracle/bootstrap/serve), but the 16 remaining chain modules described
here are shelved as Phase 2+ pending the daeji devnet. See `.roko/GAPS.md` for per-module
WIRE/SHELVE verdicts.

## Contents

| File | What |
|---|---|
| `00-INDEX.md` | Master index with dependency graph, parallelism guide, design principles |
| `01-architecture.md` | Core architecture: ConsensusVerifier trait, adapter registry, VerifiedState |
| `02-tempo.md` | Tempo integration: BLS threshold certs, EVM proofs, MPP |
| `03-adapters.md` | Adapter catalog: Tempo, Ethereum, daeji, Cosmos/IBC |
| `04-agent-surface.md` | Agent-facing surface: tool handlers, sidecar routes, MCP tools |
| `05-IMPLEMENTATION-PLAN.md` | Parallel work unit graph, dependencies, scope |
| `06-27` (WU-1 through WU-22) | Individual work unit specs with exact code, file lists, verification checklists |
| `_old-*` | Superseded predecessors of the WU files |

## Naming note

These docs use "Signal" consistently. No Engram references remain.

## Do NOT modify the WU files

These are detailed design specs. If/when chain verification work resumes, the WU files
should be used as implementation guides, not rewritten.
