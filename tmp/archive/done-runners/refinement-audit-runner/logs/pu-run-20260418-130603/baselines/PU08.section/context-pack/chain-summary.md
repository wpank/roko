# Chain Summary — Batch 08

Concise runtime picture for agents entering `08` without prior context.

## What Is Already Real

- `roko-chain` ships real client/wallet traits, a live alloy backend, wallet and tx-sim gates, mocks, and an attestation witness engine.
- `apps/mirage-rs/` ships a real revm-based simulator plus a large `chain/` scaffold and `roko_bridge` trait implementations.
- `apps/roko-chain-watcher/` ships a real long-running observer with rule-based reactions.
- `contracts/src/` ships 7 Solidity demo contracts with tests.
- `crates/roko-demo/` consumes the shipping chain/demo surface end-to-end.

## What Is Misleading Today

- Doc 24 under-claims several shipping Rust surfaces.
- the docs often imply no Solidity exists, which is false.
- the mirage chain scaffold is real but is not the same thing as the Korai registry/gossip/token design.
- Doc 21 over-claims a proxy-only ISFR surface.
- `ChainWitnessEngine` means two different things across docs and code.

## What Batch 08 Should Usually Do

1. make the shipping Rust surfaces explicit,
2. make the demo Solidity surfaces visible and correctly scoped,
3. inventory the mirage scaffold honestly,
4. mark the major frontier chapters as frontier,
5. leave later Tier-6 implementation work explicitly deferred.
