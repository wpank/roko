# SOURCE-INDEX — Current Code Anchors For 08-Chain

Refreshed source anchors for the PU08 post-audit parity pack.

Generated: 2026-04-18

---

## Important Corrections First

- This pack now anchors only to the **narrow shipped chain surface**.
- Do **not** use broader scaffold, watcher, or proxy code to imply Korai runtime parity.
- `ChainWitnessEngine` is an **attestation witness helper**, not the full witness-observer design described elsewhere in the docs.
- The Solidity contracts under `contracts/src/` are **demo contracts with tests**, not the final Korai contract suite.

---

## crates/roko-chain/src/

### Core trait surface

| Anchor | Why it matters |
|---|---|
| `crates/roko-chain/src/client.rs:22-63` | canonical `ChainClient` trait |
| `crates/roko-chain/src/wallet.rs:17-35` | canonical `ChainWallet` trait |
| `crates/roko-chain/src/types.rs:122-157` | `ChainError` and `ChainResult` used by the narrow chain API |

### Live backend

| Anchor | Why it matters |
|---|---|
| `crates/roko-chain/src/alloy_impl.rs:63-88` | `AlloyChainClient` construction and provider exposure |
| `crates/roko-chain/src/alloy_impl.rs:91-224` | `ChainClient` implementation for the Alloy-backed live backend |
| `crates/roko-chain/tests/alloy_live.rs:21-80` | live-backend smoke tests for block reads, wallet reads, and tx submission |

### Gates

| Anchor | Why it matters |
|---|---|
| `crates/roko-chain/src/gate/wallet_gate.rs:31-77` | `WalletGateConfig`, `WalletGate`, and `WalletCheck` definitions |
| `crates/roko-chain/src/gate/wallet_gate.rs:106-180` | wallet-balance / nonce preflight logic |
| `crates/roko-chain/src/gate/tx_sim_gate.rs:33-77` | `SimulationOutcome` and `TxSimulator` |
| `crates/roko-chain/src/gate/tx_sim_gate.rs:79-143` | `TxSimGateConfig` and `TxSimGate` |
| `crates/roko-chain/src/gate/tx_sim_gate.rs:146-180` | gate verification path for simulation results |

### Witness helper

| Anchor | Why it matters |
|---|---|
| `crates/roko-chain/src/witness.rs:11-18` | minimal witness primitives: marker, topic, sink address, `ChainWitnessEngine` |
| `crates/roko-chain/src/witness.rs:20-100` | `ChainWitnessEngine` witness / verify methods |
| `crates/roko-chain/src/witness.rs:103-147` | free helper functions and witness tx construction |

---

## contracts/src/

The current tree contains additional Solidity files beyond the narrow demo
suite below. For this parity pack, the stable anchor set is the seven-contract
demo surface that is already wired into the Rust demo path.

### Shipping Solidity demos

| Anchor | Why it matters |
|---|---|
| `contracts/src/AgentRegistry.sol:4-73` | minimal agent identity / heartbeat demo |
| `contracts/src/WorkerRegistry.sol:6-233` | bonded worker + EMA reputation demo |
| `contracts/src/BountyMarket.sol:7-136` | bounty lifecycle / escrow demo |
| `contracts/src/ConsortiumValidator.sol:7-114` | 3-member committee validation demo |
| `contracts/src/FeeDistributor.sol:6-103` | fee-splitting demo contract |
| `contracts/src/InsightBoard.sol:6-78` | on-chain insight / pheromone demo |
| `contracts/src/MockERC20.sol:6-23` | demo token used by the suite |

### Matching Forge tests

| Anchor | Why it matters |
|---|---|
| `contracts/test/AgentRegistry.t.sol:7-63` | `AgentRegistry` registration / liveness coverage |
| `contracts/test/WorkerRegistry.t.sol:8-80` | worker registration, EMA, and slashing coverage |
| `contracts/test/BountyMarket.t.sol:9-80` | funded-post / assign / resolve coverage |
| `contracts/test/ConsortiumValidator.t.sol:10-80` | committee assembly and vote-path coverage |
| `contracts/test/FeeDistributor.t.sol:8-80` | fee-split behavior coverage |
| `contracts/test/InsightBoard.t.sol:8-78` | posting / confirm / claim coverage |
| `contracts/test/MockERC20.t.sol:7-30` | token metadata and mint coverage |

---

## Stale Anchors To Avoid In This Pack

- wide `apps/mirage-rs/src/chain/*` inventories as if they were the core shipped chain story
- watcher / observer anchors as proof of a broader witness runtime
- settlement / ISFR / privacy files as proof of shipped chain economics
- roadmap-only doc anchors presented as if they described current code

Those surfaces may matter elsewhere in the repo, but they are not the source contract for this narrowed PU08 parity pack.
