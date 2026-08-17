# 05 — Implementation Plan

## How This Plan Is Organized

Tasks are grouped into **independent work units** that can run in parallel. Each unit has:
- **Depends on**: which units must complete first (empty = can start immediately)
- **Blocks**: which units are waiting on this one
- A detailed checklist file with mechanical task items

```
LAYER 0 (no dependencies — start immediately, in parallel)
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

**Maximum parallelism**: 3-4 agents per layer.

| Time slice | Agents working | Work units |
|------------|---------------|------------|
| T0 | 3 agents | WU-1, WU-2, WU-3 |
| T1 | 3 agents | WU-4, WU-5, WU-6 |
| T2 | 3 agents | WU-7, WU-8, WU-9 |
| T3 | 3 agents | WU-10, WU-11, WU-12 |
| T4 | 3 agents | WU-13, WU-14, WU-17 |
| T5 | 3 agents | WU-18, WU-19, WU-20 |
| T6 | 4 agents | WU-15, WU-16, WU-21, WU-22 |

---

## Scope Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Ethereum sync committee | **Not included** | Focus on Tempo only; trait supports future Ethereum adapter |
| Chain tool wiring | **Included** | "Wire, don't build" — existing handlers need extending |
| Test strategy | **Mocks + live integration** | Live tests feature-gated, gracefully skip if no RPC |
| MPP integration | **Included (WU-17–22)** | `mpp-rs` v0.9+ is live on crates.io; full client, tools, budget, ledger, discovery, and server |
| Demo | **Included** | Scripted /demo scenario + /dashboard/chain page, live Tempo Moderato testnet, real MPP payments |

---

## Critical Codebase Discovery

**Chain tool handlers already exist.** The implementation is NOT starting from zero:

| What | File | Status |
|------|------|--------|
| `ChainToolHandler` struct | `crates/roko-cli/src/chain_handler.rs` | EXISTS — dispatches 17 tools to ChainClient/ChainWallet |
| `chain_handler_map()` | `crates/roko-cli/src/chain_registry.rs` | EXISTS — builds HashMap<name, handler> |
| `chain_aware_resolver()` | `crates/roko-cli/src/chain_registry.rs` | EXISTS — resolver that checks chain handlers first |
| Orchestrator wiring | `crates/roko-cli/src/orchestrate.rs` | EXISTS — constructs AlloyChainClient from config, wires resolver |
| Sidecar chain routes | `crates/roko-serve/src/routes/chain.rs` | EXISTS — 3 routes (/chain/agents, /bounties, /status) |
| Agent sidecar feature flags | `crates/roko-agent-server/src/lib.rs` | EXISTS — but no chain feature flag yet |

**What's new**: ConsensusVerifier trait, VerifiedChainClient wrapper, state proof verification, threshold BLS, and the verified_* tool variants.

---

## Files Created Per Work Unit

### New files in `crates/roko-chain/src/`
| File | Work Unit | What |
|------|-----------|------|
| `consensus.rs` | WU-1 | ConsensusVerifier trait, TrustLevel, ConsensusProof, TrustedHeader, ConsensusError |
| `verified_state.rs` | WU-1 | VerifiedState<T> universal return type |
| `adapter.rs` | WU-1 | ChainAdapter trait, ChainBackendConfig, adapter_for_consensus() |
| `playback.rs` | WU-2 | PlaybackVerifier, PlaybackEntry, JSONL parsing |
| `state_proof.rs` | WU-5 | verify_account_proof(), verify_storage_proof() — feature-gated |
| `verified_client.rs` | WU-7 | VerifiedChainClient implementing ChainClient |
| `threshold_bls.rs` | WU-8 | ThresholdBlsVerifier — feature-gated |
| `watcher.rs` | WU-11 | ChainWatcherTask async loop |
| `mpp_client.rs` | WU-17 | MppClient wrapping mpp-rs + LC verification, VerifiedPayment, MppSession — feature-gated |
| `mpp_budget.rs` | WU-19 | MppBudgetPolicy, per-agent spending caps, allowlist — feature-gated |
| `payment_ledger.rs` | WU-20 | Append-only PaymentLedger, PaymentRecord, spending analytics |
| `mpp_discovery.rs` | WU-21 | MppDiscovery client, OpenAPI + x-payment-info parsing, LRU cache — feature-gated |
| `mpp_catalog.rs` | WU-21 | Vendored static catalog of ~50 MPP services |

### Modified files
| File | Work Unit | Change |
|------|-----------|--------|
| `crates/roko-chain/src/types.rs` | WU-3 | Add state_root to ChainHeader |
| `crates/roko-chain/src/mock.rs` | WU-3 | Populate state_root in mock headers |
| `crates/roko-chain/src/alloy_impl.rs` | WU-3 | Extract state_root from alloy block |
| `crates/roko-chain/src/lib.rs` | WU-1,2,5,7,8,11 | Register new modules |
| `crates/roko-chain/src/tools.rs` | WU-6 | Add 5 verified tool defs |
| `crates/roko-chain/Cargo.toml` | WU-2,5,8,17 | Add hex, alloy-trie, commonware, mpp, blst deps |
| `crates/roko-cli/src/chain_handler.rs` | WU-9, WU-18 | Add verified_* + MPP tool dispatch arms |
| `crates/roko-cli/src/chain_registry.rs` | WU-9, WU-18 | Include verified + MPP tool names |
| `crates/roko-core/src/config/chain.rs` | WU-10 | Add ChainBackendsConfig |
| `crates/roko-cli/src/orchestrate.rs` | WU-13 | Construct VerifiedChainClient, wire into resolver |
| `crates/roko-agent-server/src/lib.rs` | WU-12, WU-22 | Add chain + mpp_server feature flags |
| `crates/roko-agent-server/src/features/mod.rs` | WU-12, WU-18 | Add chain + MPP modules |
| `crates/roko-serve/src/routes/chain.rs` | WU-12, WU-18 | Add verified state + MPP routes |
| `crates/roko-serve/src/events.rs` | WU-11 | Add ChainEvent to ServerEvent |
| `crates/roko-chain/src/tools.rs` | WU-6, WU-18 | Add verified + MPP tool definitions |
| `crates/roko-chain/src/lib.rs` | WU-1,2,5,7,8,11,17,19,20,21 | Register all new modules |

### New files in `demo/demo-app/`
| File | Work Unit | What |
|------|-----------|------|
| `src/lib/scenario-runners/verified-chain.ts` | WU-15 | Scripted demo scenario (8 steps, 2 agents) |
| `src/pages/dashboard/ChainDashboard.tsx` | WU-16 | Interactive chain monitoring + query page |
| `src/pages/dashboard/ChainDashboard.css` | WU-16 | ROSEDUST-styled CSS for chain dashboard |
| `fixtures/verified-chain-roko.toml` | WU-15 | Workspace config fixture for demo |

### Modified files in `demo/demo-app/`
| File | Work Unit | Change |
|------|-----------|--------|
| `src/lib/scenario-runners/index.ts` | WU-15 | Add verifiedChain to allScenarios |
| `src/pages/Demo/index.tsx` | WU-15 | Add 'verified-chain' to TAB_CATEGORY |
| `src/pages/dashboard/Layout.tsx` | WU-16 | Add Chain to VIEWS array |
| `src/main.tsx` | WU-16 | Add /dashboard/chain lazy route |
| `src/transport/types.ts` | WU-16 | Add chain SSE event types |
| `src/app/DataHub.ts` | WU-16 | Handle chain events in store |

### New files in `crates/roko-agent-server/`
| File | Work Unit | What |
|------|-----------|------|
| `src/features/chain.rs` | WU-12, WU-18 | Chain + MPP sidecar routes |
| `src/middleware/mpp_gate.rs` | WU-22 | MppGateLayer Axum middleware (402 challenge-response) |
| `src/features/mpp_openapi.rs` | WU-22 | Auto-generated OpenAPI with x-payment-info for gated routes |

### New test files
| File | Work Unit |
|------|-----------|
| `crates/roko-chain/tests/verified_client.rs` | WU-14 |
| `crates/roko-chain/tests/tempo_live.rs` | WU-14 |
| `crates/roko-chain/tests/watcher_integration.rs` | WU-14 |
| `crates/roko-chain/tests/backend_pool.rs` | WU-14 |
| `crates/roko-chain/src/testdata/demo-playback.jsonl` | WU-2 |

---

## Verification After All Work Units

```bash
# No features (core types + playback + rpc-only + payment ledger)
cargo test -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings

# With alloy (state proofs + verified client)
cargo test -p roko-chain --features alloy-backend

# With threshold BLS
cargo test -p roko-chain --features threshold-bls

# With MPP (client + budget + discovery)
cargo test -p roko-chain --features mpp
cargo test -p roko-chain --features mpp,alloy-backend  # full MPP + LC verification

# Full workspace (no breakage)
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Integration tests (requires running RPC)
ROKO_TEST_RPC_URL=https://rpc.moderato.tempo.xyz cargo test -p roko-chain --features alloy-backend -- --ignored

# MPP integration tests (requires wallet + funded testnet account)
ROKO_TEST_MPP_WALLET_KEY=... cargo test -p roko-chain --features mpp -- --ignored

# Demo app (frontend)
cd demo/demo-app && npm run build
cd demo/demo-app && npm run dev  # then navigate to /demo → Verified Chain tab, /dashboard/chain
```
