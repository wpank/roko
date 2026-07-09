# Gap Inventory — 08 Chain

Concise gap list for agents working on chain parity batches.

## Focus Now

These are the gaps batch `08` should actively try to close:

### 1. The Canonical Status Doc Still Under-Claims Shipping Code — HIGH

- Doc 24 still mislabels several shipping Rust surfaces,
- it also hides the demo-contract surface,
- later agents will keep getting the chain status wrong if this stays drifted.

### 2. The Demo Solidity Surface Is Real But Poorly Integrated Into The Docs — HIGH

- 7 contracts and tests ship,
- several implement meaningful precursors to the PRD design,
- the docs still read too close to “zero Solidity exists”.

### 3. The Mirage Chain Scaffold Is Large And Real, But Mis-Scoped — HIGH

- the scaffold ships under `apps/mirage-rs/src/chain/*`,
- but it is broader and different from the Korai-specific registry/gossip story the docs imply.

### 4. Doc 21 Still Overstates The ISFR Surface — HIGH

- the repo ships a proxy,
- not the solver / KKT / aggregation system the chapter mostly describes.

### 5. Witness Naming Is Easy To Misread — MEDIUM

- Doc 15’s witness engine and Rust’s `ChainWitnessEngine` are different systems,
- without explicit disambiguation later agents will keep conflating them.

## Defer From Batch 08

These are valid findings, but they should usually be documented and handed off:

- Korai v1 Solidity contracts -> later Solidity pass
- `MirageChainClient` -> later mirage integration pass
- libp2p / GossipSub / mesh -> later network pass
- x402 / channels / micropayments -> later payments pass
- ISFR solver / KKT -> later settlement pass
- TEE / ZK / Binius -> later privacy pass
- futures market -> later market pass

## Working Rule

If a chain task requires:

- new Solidity,
- a real p2p/network layer,
- or a real solver/privacy/settlement implementation,

then batch `08` should normally implement the smallest honest status contract and defer the rest.
