# Verified Chain Layer — Design & Implementation Docs

## Quick Start for Implementing Agents

1. Read **05-IMPLEMENTATION-PLAN.md** — understand the parallel work unit structure and dependency graph
2. Pick the lowest-layer unimplemented WU whose dependencies are complete
3. Read the WU file (e.g., `06-WU1-core-types.md`) — it contains everything: context, exact code, files to modify, verification checklist
4. Implement it mechanically — every item is a concrete code change
5. Run the verification commands at the bottom of each WU
6. Move to the next available WU

**Multiple agents can work in parallel** on any WUs in the same layer (see plan for parallelism guide).

### Build and test

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build (default features — no alloy)
cargo build -p roko-chain

# Build with alloy backend
cargo build -p roko-chain --features alloy-backend

# Test
cargo test -p roko-chain
cargo test -p roko-chain --features alloy-backend

# Lint
cargo clippy -p roko-chain --no-deps -- -D warnings

# Full workspace check (after changes)
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Integration tests (requires running RPC)
ROKO_TEST_TEMPO_RPC_URL=https://rpc.moderato.tempo.xyz \
  cargo test -p roko-chain --features alloy-backend -- --ignored
```

### Key crate path

`/Users/will/dev/nunchi/roko/roko/crates/roko-chain/`

---

## Documents

### Design Docs (architecture & rationale)

| Doc | What |
|-----|------|
| [01-architecture.md](01-architecture.md) | Core architecture: `ConsensusVerifier` trait, adapter registry, `VerifiedState<T>`, config |
| [02-tempo.md](02-tempo.md) | Tempo integration: BLS threshold certs, EVM `eth_getProof` proofs, MPP payments |
| [03-adapters.md](03-adapters.md) | Adapter catalog: Tempo, Ethereum, daeji, Cosmos/IBC patterns |
| [04-agent-surface.md](04-agent-surface.md) | Agent-facing surface: tool handlers, sidecar routes, MCP tools |

### Implementation Plan

| Doc | What |
|-----|------|
| [05-IMPLEMENTATION-PLAN.md](05-IMPLEMENTATION-PLAN.md) | Parallel work unit graph, dependency layers, scope decisions, file inventory |

### Work Units (Layer 0 — no dependencies, start immediately)

| Doc | WU | Layer | Effort | What |
|-----|-----|-------|--------|------|
| [06-WU1-core-types.md](06-WU1-core-types.md) | WU-1 | 0 | 2-3h | `ConsensusVerifier` trait, `TrustLevel`, `TrustedHeader`, `VerifiedState<T>`, `ChainAdapter` |
| [07-WU2-playback.md](07-WU2-playback.md) | WU-2 | 0 | 1-2h | `PlaybackVerifier` — deterministic replay from JSONL for demos/tests |
| [08-WU3-chain-header.md](08-WU3-chain-header.md) | WU-3 | 0 | 1h | Add `state_root` to `ChainHeader`, update mock + alloy + all construction sites |

### Work Units (Layer 1 — depends on Layer 0)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [09-WU4-rpc-verifier.md](09-WU4-rpc-verifier.md) | WU-4 | 1 | WU-1, WU-3 | 1-2h | `RpcOnlyVerifier` — baseline verifier that trusts the RPC |
| [10-WU5-state-proofs.md](10-WU5-state-proofs.md) | WU-5 | 1 | WU-1, WU-3 | 2-3h | MPT state proof verification via alloy-trie (feature-gated) |
| [11-WU6-tool-defs.md](11-WU6-tool-defs.md) | WU-6 | 1 | WU-1 | 1h | 5 new tool definitions: verified_balance, verified_storage, verify_transfer, head, backends |

### Work Units (Layer 2 — depends on Layer 1)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [12-WU7-verified-client.md](12-WU7-verified-client.md) | WU-7 | 2 | WU-4, WU-5 | 2-3h | `VerifiedChainClient` wrapping ChainClient + ConsensusVerifier |
| [13-WU8-threshold-bls.md](13-WU8-threshold-bls.md) | WU-8 | 2 | WU-4 | 3-4h | `ThresholdBlsVerifier` for Tempo (commonware BLS12-381, feature-gated) |
| [14-WU9-tool-handlers.md](14-WU9-tool-handlers.md) | WU-9 | 2 | WU-6 | 2-3h | Extend existing `ChainToolHandler` with 5 new dispatch arms |

### Work Units (Layer 3 — depends on Layer 2)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [15-WU10-config.md](15-WU10-config.md) | WU-10 | 3 | WU-7, WU-8 | 2-3h | Multi-backend config (`[chain.backends.*]`), `BackendPool` factory |
| [16-WU11-watcher.md](16-WU11-watcher.md) | WU-11 | 3 | WU-7 | 2-3h | `ChainWatcherTask` async loop driving `BlockObserver`, event bus bridge |
| [17-WU12-sidecar.md](17-WU12-sidecar.md) | WU-12 | 3 | WU-7, WU-9 | 2h | Sidecar `chain` feature flag + routes, roko-serve verified routes |

### Work Units (Layer 4 — depends on Layer 3)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [18-WU13-orchestrator.md](18-WU13-orchestrator.md) | WU-13 | 4 | WU-10, WU-9 | 1-2h | Wire `BackendPool` + `VerifiedChainClient` into PlanRunner resolver |
| [19-WU14-integration.md](19-WU14-integration.md) | WU-14 | 4 | WU-10, WU-11 | 2-3h | Mock tests + live Tempo tests + watcher integration tests |

### Work Units (Layer 4 — MPP, depends on Layer 3)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [22-WU17-mpp-client.md](22-WU17-mpp-client.md) | WU-17 | 4 | WU-7, WU-10 | 3-4h | `MppClient` wrapping `mpp-rs` with LC settlement verification, `VerifiedPayment`, session support |
| [23-WU18-mpp-tools.md](23-WU18-mpp-tools.md) | WU-18 | 4 | WU-17, WU-9, WU-12 | 2-3h | MPP tool defs (`chain.mpp_pay/session/discover`), dispatch handlers, sidecar + serve routes, MCP entries |
| [24-WU19-mpp-budget.md](24-WU19-mpp-budget.md) | WU-19 | 4 | WU-17 | 2-3h | `MppBudgetPolicy` — per-agent spending caps (hourly/daily/total), per-service allowlist, hard deny on exceed |
| [25-WU20-payment-ledger.md](25-WU20-payment-ledger.md) | WU-20 | 4 | WU-17 | 2h | Append-only payment ledger (`.roko/payments.jsonl`), `roko learn payments` CLI, spending analytics |

### Work Units (Layer 5 — Discovery + Demo, depends on Layer 4)

| Doc | WU | Layer | Depends | Effort | What |
|-----|-----|-------|---------|--------|------|
| [20-WU15-demo-scenario.md](20-WU15-demo-scenario.md) | WU-15 | 5 | WU-13, WU-12, WU-14, WU-18 | 4-5h | Scripted /demo scenario: 2 agents, verified queries, MPP payment, transfer verification on Tempo Moderato |
| [21-WU16-dashboard-chain.md](21-WU16-dashboard-chain.md) | WU-16 | 5 | WU-12, WU-11, WU-13 | 4-5h | Interactive /dashboard/chain page: backend cards, live head, verified balance/storage queries, trust badges |
| [26-WU21-mpp-discovery.md](26-WU21-mpp-discovery.md) | WU-21 | 5 | WU-17, WU-18 | 2-3h | `MppDiscovery` client — OpenAPI + `x-payment-info` parsing, vendored service catalog (~50 services), LRU cache |
| [27-WU22-mpp-server.md](27-WU22-mpp-server.md) | WU-22 | 5 | WU-17, WU-18, WU-12 | 3-4h | Agent-as-MPP-server — `MppGateLayer` Axum middleware, 402 challenge-response, auto OpenAPI, revenue recording |

### Archived (superseded by WU files)

| File | Status |
|------|--------|
| `_old-06-TASK-CHECKLIST.md` | Superseded by individual WU files |
| `_old-07-DEPENDENCY-REFERENCE.md` | Content absorbed into WU files |
| `_old-08-EXISTING-CODE-REFERENCE.md` | Content absorbed into WU files |

---

## Dependency Graph

```
LAYER 0 (no dependencies — start 3 agents in parallel)
├── WU-1: Core Types & Traits           → 06-WU1-core-types.md
├── WU-2: Playback Verifier             → 07-WU2-playback.md
└── WU-3: Extend ChainHeader            → 08-WU3-chain-header.md

LAYER 1 (depends on Layer 0)
├── WU-4: RPC-Only Verifier             → 09-WU4-rpc-verifier.md     [needs WU-1, WU-3]
├── WU-5: State Proof Verification      → 10-WU5-state-proofs.md     [needs WU-1, WU-3]
└── WU-6: Verified Tool Defs            → 11-WU6-tool-defs.md        [needs WU-1]

LAYER 2 (depends on Layer 1)
├── WU-7: VerifiedChainClient           → 12-WU7-verified-client.md  [needs WU-4, WU-5]
├── WU-8: Threshold BLS (Tempo)         → 13-WU8-threshold-bls.md    [needs WU-4]
└── WU-9: Chain Tool Handlers           → 14-WU9-tool-handlers.md    [needs WU-6]

LAYER 3 (depends on Layer 2)
├── WU-10: Config Integration           → 15-WU10-config.md          [needs WU-7, WU-8]
├── WU-11: Block Watcher                → 16-WU11-watcher.md         [needs WU-7]
└── WU-12: Sidecar Chain Routes         → 17-WU12-sidecar.md         [needs WU-7, WU-9]

LAYER 4 (depends on Layer 3 — verification + MPP)
├── WU-13: Orchestrator Wiring          → 18-WU13-orchestrator.md    [needs WU-10, WU-9]
├── WU-14: Integration Tests            → 19-WU14-integration.md     [needs WU-10, WU-11]
├── WU-17: MPP Client                   → 22-WU17-mpp-client.md      [needs WU-7, WU-10]
├── WU-18: MPP Tool Handlers            → 23-WU18-mpp-tools.md       [needs WU-17, WU-9, WU-12]
├── WU-19: MPP Budget Controller        → 24-WU19-mpp-budget.md      [needs WU-17]
└── WU-20: Payment Ledger               → 25-WU20-payment-ledger.md  [needs WU-17]

LAYER 5 — DEMO + DISCOVERY (depends on Layer 4 — can run in parallel)
├── WU-15: Verified Chain Demo Scenario  → 20-WU15-demo-scenario.md  [needs WU-13, WU-12, WU-14, WU-18]
├── WU-16: Dashboard Chain Sub-Page      → 21-WU16-dashboard-chain.md [needs WU-12, WU-11, WU-13]
├── WU-21: MPP Service Discovery         → 26-WU21-mpp-discovery.md   [needs WU-17, WU-18]
└── WU-22: Agent-as-MPP-Server           → 27-WU22-mpp-server.md      [needs WU-17, WU-18, WU-12]
```

### Parallelism Guide

| Time slice | Agents working | Work units |
|------------|---------------|------------|
| T0 | 3 agents | WU-1, WU-2, WU-3 |
| T1 | 3 agents | WU-4, WU-5, WU-6 |
| T2 | 3 agents | WU-7, WU-8, WU-9 |
| T3 | 3 agents | WU-10, WU-11, WU-12 |
| T4 | 3 agents | WU-13, WU-14, WU-17 |
| T5 | 3 agents | WU-18, WU-19, WU-20 |
| T6 | 4 agents | WU-15, WU-16, WU-21, WU-22 |

**Total estimated effort**: ~50-68 hours across 22 work units.
**With 3 parallel agents**: ~20-25 hours wall clock.

---

## Design Principles

1. **Wire, don't build.** Activate existing chain tool defs, `chain_client` slot, and `BlockObserver` before building new abstractions.
2. **EVM-first.** State proofs are `eth_getProof` MPT proofs. Don't invent a new format.
3. **Consensus is the extension point.** What varies is how you verify a block header is finalized.
4. **`VerifiedState<T>` is the universal return type.** Every chain query returns data wrapped in verification metadata.
5. **Offline playback is mandatory.** Every adapter supports deterministic replay for demos and testing.
6. **Backward compatible.** Existing `[chain]` config with just `rpc_url` continues to work.

---

## Corrections from Prior Docs

| Error | Correction |
|-------|-----------|
| Tempo uses QMDB for state proofs | **Wrong.** Tempo uses Reth SDK → standard EVM → MPT proofs via `eth_getProof`. QMDB is a Commonware storage primitive, not used by Tempo's execution layer. |
| ~500-byte QMDB proofs | **Wrong.** State proofs are standard MPT (~1-5 KB). Consensus certs are ~240 bytes BLS threshold sig. |
| Placeholder chain IDs | Tempo mainnet chain ID is **4217**. Testnet (Moderato) is **42431**. |
| No mention of MPP | Tempo's Machine Payments Protocol is the payment primitive for agent commerce. |
| Treated chain tools as greenfield | roko-chain already has 17 tool definitions. We're extending, not creating. |
| Treated ChainToolHandler as new | `ChainToolHandler` already exists in `chain_handler.rs` with 17 dispatch arms. |
