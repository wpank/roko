# F — Built Foundation

This is the only section in topic `08` that should read as materially shipped.
Even here, the scope stays small.

## Minimal Shipped Foundation

- `ChainClient` in `crates/roko-chain/src/client.rs`
- `ChainWallet` in `crates/roko-chain/src/wallet.rs`
- `AlloyChainClient` in `crates/roko-chain/src/alloy_impl.rs`
- `WalletGate` in `crates/roko-chain/src/gate/wallet_gate.rs`
- `TxSimGate` in `crates/roko-chain/src/gate/tx_sim_gate.rs`
- `ChainWitnessEngine` in `crates/roko-chain/src/witness.rs`
- Solidity demo contracts under `contracts/src/`

## How To Describe The Solidity Surface

Describe the Solidity files as demo or precursor contracts. The audit baseline
called out seven demo contracts; regardless of exact file count in the tree at
edit time, the parity material should keep the claim narrow:

- demo contracts ship,
- they are useful evidence of exploration,
- they do not mean a full Korai deployment exists.

## What This Foundation Does Not Prove

- a running Korai node
- a complete token economy
- a gossip mesh
- a production market or settlement system
- privacy or futures infrastructure

## Handoff

Later execution work can build on this section, but this file itself should
stay short and factual. It is an inventory, not a roadmap.
