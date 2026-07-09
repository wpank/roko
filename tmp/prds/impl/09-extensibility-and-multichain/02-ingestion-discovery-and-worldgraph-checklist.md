# Multi-Chain Ingestion, Discovery, And WorldGraph Checklist

## Scope

Use this file for canonical chain events, connector traits, discovery pipelines, predictive foraging, and WorldGraph integration.

## Implementation checklist

- [ ] Define the chain-ingestion contract first.
  - `ChainConnector`
  - canonical event schema
  - deterministic event ids
  - finality and reorg handling
- [ ] Start with one mature connector and one contrast connector.
  - EVM first;
  - one non-EVM or external-feed connector second only if needed.
- [ ] Build contract discovery as layered composition.
  - interface detection;
  - selector fingerprinting;
  - bytecode similarity;
  - transaction-pattern classification;
  - factory tracking;
  - optional shared insight lookup.
- [ ] Add predictive foraging only when it has real inputs.
  - candidate patches/assets/contracts;
  - expected value estimate;
  - switching rule;
  - attention budget output.
- [ ] Treat WorldGraph as a consumer of canonical events, not a parallel event store.
  - entity extraction;
  - relationship updates;
  - fingerprinting for retrieval or context bidding later.

## Relevant current files

- `crates/roko-chain/src/client.rs`
- `apps/mirage-rs/src/chain/`
- `crates/roko-learn/src/active_inference.rs`
- `crates/roko-learn/src/bandits.rs`
- `crates/roko-learn/src/model_router.rs`

## Verification checklist

- [ ] Canonical events survive reordering/replay in tests.
- [ ] Discovery layers can be benchmarked independently.
- [ ] Foraging decisions expose their score inputs.
- [ ] WorldGraph updates can be replayed from stored canonical events.

## Acceptance criteria

- Multi-chain ingestion has one stable canonical event schema.
- Discovery logic is layered and testable.
- WorldGraph is downstream of canonical events and can later bid into context cleanly.
