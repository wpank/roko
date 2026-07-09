# Repo Map — Shared Chain Context

Quick reference for agents working on `08` chain parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 08 |
|------|------|----------------------------|
| Chain crate | `crates/roko-chain/src/` | canonical Rust chain surface |
| HDC primitive | `crates/roko-primitives/src/hdc.rs` | canonical HDC owner |
| Mirage simulator | `apps/mirage-rs/` | real simulator + scaffold + bridge surface |
| Chain watcher | `apps/roko-chain-watcher/src/` | shipping observer precursor |
| Demo contracts | `contracts/src/` | 7 shipping Solidity demo contracts |
| Demo contract tests | `contracts/test/` | evidence that the Solidity demos are not hypothetical |
| Demo consumer | `crates/roko-demo/src/` | end-to-end consumption of the shipping chain surface |
| Chain docs | `docs/08-chain/` | source material being checked |
| Parity batch | `tmp/docs-parity/08/` | execution contract and findings |

## Important Corrections

Use these instead of older or misleading assumptions:

- `AlloyChainClient`, `WalletGate`, `TxSimGate`, and `roko_bridge` already ship.
- 7 Solidity demo contracts already ship.
- the mirage `chain/` modules are real, but they are not literally the Korai v1 contract/gossip/runtime stack.
- Doc 21’s shipping repo surface is proxy-only.
- `ChainWitnessEngine` in Rust is an attestation anchor, not the Doc 15 block-observer engine.
- `roko-primitives/src/hdc.rs` is the canonical HDC owner for chain docs.

## Search Priorities

Before editing, search these first:

```bash
rg -n "AlloyChainClient|WalletGate|TxSimGate|ChainWitnessEngine|roko_bridge" crates/roko-chain apps/mirage-rs docs/08-chain
rg -n "contracts/src|AgentRegistry.sol|WorkerRegistry.sol|BountyMarket.sol|ConsortiumValidator.sol|FeeDistributor.sol" docs/08-chain contracts
rg -n "korai_|chain-extensions|legacy-api|ISFR_SERVICE_URL|localhost:8546" docs/08-chain apps/mirage-rs
rg -n "InsightBus|PheromoneBus|roko-chain-watcher|WitnessEngine|ChainWitnessEngine" docs/08-chain apps/mirage-rs apps/roko-chain-watcher crates/roko-chain
rg -n "Implementation: Built|Design — Phase 2\\+|Proxy-only|Scaffold" docs/08-chain/*.md
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
```

## Practical Rules

1. Make shipping surfaces visible before refining frontier theory.
2. Keep “demo”, “scaffold”, and “built” distinct.
3. Prefer one canonical status doc and one canonical source index.
4. If the work turns into actual chain implementation, stop and defer it.
