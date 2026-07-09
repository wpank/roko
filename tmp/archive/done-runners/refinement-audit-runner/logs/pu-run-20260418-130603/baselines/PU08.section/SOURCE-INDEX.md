# SOURCE-INDEX — Code Anchors for 08-Chain Parity

Verified code references for batch `08`, organized around the status seams an agent is most likely to touch.

Generated: 2026-04-16

---

## Important Corrections First

Use these before trusting the chain docs literally:

- `AlloyChainClient`, `WalletGate`, `TxSimGate`, and `roko_bridge` are real shipping surfaces, not stubs.
- 7 Solidity demo contracts ship under `contracts/src/`; the docs currently understate or hide them.
- `apps/mirage-rs/src/chain/*` is real and substantial, but it is not the same thing as the Korai registry/token/gossip design described by many PRDs.
- `apps/mirage-rs/src/http_api/isfr.rs` is a proxy to an upstream service at `http://localhost:8546`; the solver and KKT logic are not in this repo.
- `ChainWitnessEngine` in `crates/roko-chain/src/witness.rs` is an attestation anchor, not the frontier block-observer engine described in Doc 15.
- The canonical HDC primitive for chain docs is `crates/roko-primitives/src/hdc.rs`, not the stale `bardo-primitives` path and not the mirage wrappers.

---

## crates/roko-chain/src/

### Core crate surface

| File | What | Section |
|------|------|---------|
| `lib.rs:1-31` | module declarations and re-exports | F.01 |
| `client.rs:22-64` | `ChainClient` trait (8 methods) | F.02 |
| `wallet.rs:17-36` | `ChainWallet` trait (6 methods) | F.03 |
| `types.rs:121-157` | `ChainError` and `ChainResult` | F.05 |
| `mock.rs:48-408` | `MockChainClient`, `MockChainWallet`, `paired_mocks` | F.06 |

### Live backend, gates, witness

| File | What | Section |
|------|------|---------|
| `alloy_impl.rs:1-327` | `AlloyChainClient` live backend | F.07 |
| `gate/wallet_gate.rs:1-523` | `WalletGate` and `WalletCheck` | F.09 |
| `gate/tx_sim_gate.rs:1-448` | `TxSimGate`, `TxSimulator`, `SimulationOutcome` | F.10 |
| `witness.rs:17-112` | `ChainWitnessEngine`, `witness_on_chain`, `verify_on_chain` | F.08, E.13 |
| `tests/alloy_live.rs` | live-backend tests | F.07 |

---

## crates/roko-primitives/src/

### Canonical HDC primitive

| File | What | Section |
|------|------|---------|
| `hdc.rs:24-26` | `HdcVector { bits: [u64; 160] }` | B.01 |
| `hdc.rs:107-218` | bind, bundle, permute, similarity | B.01 |
| `hdc.rs:242-255` | fingerprint helpers | B.01 |

---

## apps/mirage-rs/

### Feature matrix and core simulator

| File | What | Section |
|------|------|---------|
| `Cargo.toml:88-111` | feature matrix; `chain`, `legacy-api`, `roko` | F.16 |
| `src/lib.rs` | core simulator surface | F.11 |
| `src/chain_rpc.rs:1-2085` | `chain_*` RPC namespace; no `korai_*` methods | F.15 |

### `src/chain/` scaffold

| File | What | Section |
|------|------|---------|
| `src/chain/mod.rs:24-57` | 9 submodule declarations and re-exports | F.12 |
| `src/chain/agent.rs` | agent-registry-like surface | F.12 |
| `src/chain/hdc_index.rs` | exact HDC search wrapper over canonical primitive | B.02, F.12 |
| `src/chain/hnsw.rs` | approximate HDC index wrapper | B.02, F.12 |
| `src/chain/insight.rs` | insight store types | F.12 |
| `src/chain/knowledge.rs` | knowledge store types | F.12 |
| `src/chain/pheromone.rs` | pheromone field/store types | F.12 |
| `src/chain/prediction.rs` | prediction/calibration store types | F.12 |
| `src/chain/projection.rs` | projection helpers | F.12 |
| `src/chain/task.rs` | task store types | F.12 |

### Legacy REST and bridge layers

| File | What | Section |
|------|------|---------|
| `src/http_api/isfr.rs:1-58` | ISFR proxy-only HTTP routes | G.04 |
| `src/http_api/ws.rs` | websocket surface over internal buses | C.08, F.17 |
| `src/roko_bridge/simulation_gate.rs:1-330` | `SimulationGate` (`impl Gate`) | F.13 |
| `src/roko_bridge/hdc_substrate.rs:1-289` | `HdcSubstrate` (`impl Substrate`) | F.13 |
| `src/roko_bridge/chain_substrate.rs:1-394` | `ChainSubstrate` (`impl Substrate`) | F.13 |
| `src/roko_bridge/subscription/*` | `InsightBus`, `PheromoneBus`, backpressure, sinks | C.08 |

---

## apps/roko-chain-watcher/src/

### Shipping observer precursor

| File | What | Section |
|------|------|---------|
| `watcher.rs:1-60` | main poll loop | E.04 |
| `block_observer.rs:1-60` | HTTP block polling + dedup state | E.04 |
| `reactions.rs:1-40+` | 5 hand-written reaction rules | E.06 |
| `rpc_client.rs` | HTTP RPC client | E.02, E.04 |
| `known_addresses.rs` | known contract address catalog | E.04 |

---

## contracts/src/

### Shipping Solidity demo contracts

| File | What | Section |
|------|------|---------|
| `AgentRegistry.sol:1-73` | minimal identity registry precursor | A.10, B.05 |
| `WorkerRegistry.sol:1-233` | single-domain EMA reputation + tier/decay | A.10, D.05-D.06 |
| `BountyMarket.sol:1-136` | 6-state bounty market precursor | A.10, D.01 |
| `ConsortiumValidator.sol:1-114` | committee validator precursor | D.11 |
| `FeeDistributor.sol:1-103` | fee-split contract, not wired into the market | A.10, D.09 |
| `InsightBoard.sol:1-78` | insight-posting contract | A.10 |
| `MockERC20.sol:1-23` | test token; not KORAI, not ERC-3009 | A.06, A.10 |

### Contract tests

| File | What | Section |
|------|------|---------|
| `contracts/test/*.t.sol` | ~43 test functions across the 7 demo contracts | A.10, D.01, D.05, D.11 |

---

## crates/roko-demo/src/

### End-to-end consumer of the shipping chain/demo surface

| File | What | Section |
|------|------|---------|
| `main.rs` | demo entry and bindings usage | F.20 |
| `scenarios/job_board.rs` | `BountyMarket` / `WorkerRegistry` consumer | F.20, D.01 |
| `scenarios/consortium.rs` | `ConsortiumValidator` consumer | F.20, D.11 |
| `scenarios/yield_routing.rs` | broader contract/demo usage | F.20 |

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in this repo:

### Korai v1 chain and contract surfaces

| Absent Feature | Search | Section |
|----------------|--------|---------|
| Korai node binary / runtime | `rg -n "korai node|KoraiNode|korai_runtime" crates apps contracts` | A.05 |
| KORAI token contract | `rg -n "contract KORAI|transferWithAuthorization|demurrage" contracts crates apps` | A.06, G.01 |
| Passport ERC-721 | `rg -n "ERC-721|soulbound|Passport" contracts crates apps` | B.05 |
| ERC-8004 three-registry deployment | `rg -n "0xA100|0xA200|0xA300|Validation Registry|Reputation Registry" contracts crates apps` | B.10-B.13 |

### Gossip / p2p / market frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| libp2p / gossipsub / iroh | `rg -n "libp2p|gossipsub|iroh|FABRIC|MiroFish" crates apps` | C.01-C.07 |
| Sparrow / power-of-two-choices / VRF hiring | `rg -n "Sparrow|power.?of.?two|VRF" contracts crates apps` | D.02 |
| Vickrey auction | `rg -n "Vickrey|commit-reveal|blind auction" contracts crates apps` | D.03-D.04 |
| C-factor aggregation in chain layer | `rg -n "C-factor|CFactor" contracts apps/mirage-rs/src/chain docs/08-chain` | D.08 |

### Witness / settlement / privacy frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| Binary Fuse / Roaring witness engine | `rg -n "Binary Fuse|Roaring|WitnessEngine" crates apps` | E.01-E.03 |
| MIDAS-R / curiosity scoring | `rg -n "MIDAS|curiosity|Bayesian curiosity" crates apps` | E.06-E.08 |
| ISFR solver / KKT verifier | `rg -n "KKT|Quadratic Programming|bisection|market-clearing" crates apps` | G.05-G.07 |
| Valhalla privacy tiers / TEE / ZK / Binius | `rg -n "Valhalla|TEE|Binius|zk|fraud proof" crates apps contracts` | G.08-G.10 |
| knowledge futures | `rg -n "futures market|knowledge futures" crates apps contracts` | G.11 |

---

## Runtime Negatives That Matter For Batch 08

These matter because the code exists, but the docs still misrepresent it:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| Doc 24 still treats several shipping Rust surfaces as missing/stub | `AlloyChainClient`, `WalletGate`, `TxSimGate`, `roko_bridge` all exist | A.09, F.07, F.09, F.10, F.13 |
| demo Solidity surface is easy to miss | 7 contracts + tests in `contracts/` with limited doc visibility | A.10, D.01, D.05, D.11 |
| mirage scaffold is real but semantically different | `apps/mirage-rs/src/chain/*` implements broader agent-coordination stores | F.12 |
| ISFR shipping surface is proxy-only | `http_api/isfr.rs` just forwards to upstream | G.04 |
| witness naming collision remains easy to misread | Rust attestation engine vs Doc 15 block-observer engine | E.13 |

---

## Practical Search Priorities

Before editing, search these first:

```bash
rg -n "AlloyChainClient|WalletGate|TxSimGate|ChainWitnessEngine|roko_bridge" crates/roko-chain apps/mirage-rs docs/08-chain
rg -n "contracts/src|AgentRegistry.sol|WorkerRegistry.sol|BountyMarket.sol|ConsortiumValidator.sol|FeeDistributor.sol" docs/08-chain contracts
rg -n "korai_|chain-extensions|legacy-api|ISFR_SERVICE_URL|localhost:8546" docs/08-chain apps/mirage-rs
rg -n "InsightBus|PheromoneBus|roko-chain-watcher|WitnessEngine|ChainWitnessEngine" docs/08-chain apps/mirage-rs apps/roko-chain-watcher crates/roko-chain
rg -n "Implementation: Built|Design — Phase 2\\+|Proxy-only|Scaffold" docs/08-chain/*.md
```

## Working Rule

If a chain task requires:

- new Solidity,
- a real p2p/network layer,
- a real solver, privacy, or settlement implementation,

then batch `08` should normally implement the smallest honest documentation/status contract and defer the rest.
