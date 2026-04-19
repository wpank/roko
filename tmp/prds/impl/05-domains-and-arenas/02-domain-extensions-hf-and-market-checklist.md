# Domain Extensions, HuggingFace, And Work Market Checklist

## Scope

Use this file for domain-specific extensions, HuggingFace integration, native benchmark adapters, and later work-market hooks.

## Implementation checklist

- [ ] Stage domain-specific extensions behind the same runtime extension contract used by IMPL-01.
  - chain subscriber/risk/ISFR hooks;
  - research source-watcher/synthesis hooks.
- [ ] Only create a dedicated HuggingFace crate when there is a minimal consumer.
  - dataset access for benchmarks;
  - model discovery for routing;
  - optional AutoTrain trigger after data export exists.
- [ ] Prefer a small vertical slice over broad stub coverage.
  - example: SWE-bench subset -> instance mapping -> execution -> score -> persisted result.
- [ ] Keep work-market integration behind explicit boundaries.
  - job submission schema;
  - verification result schema;
  - settlement/knowledge-futures logic deferred until there is a stable marketplace core elsewhere in the system.
- [ ] Track cross-arena transfer only when arenas emit comparable metrics.

## Relevant code and docs

- `crates/roko-chain/src/`
- `crates/roko-learn/src/model_router.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `docs/05-learning/20-research-to-runtime.md`
- `docs/14-identity-economy/`

## Verification checklist

- [ ] At least one domain extension runs through the shared extension chain.
- [ ] One benchmark dataset can be loaded end to end from source to score.
- [ ] Arena output is rich enough to later support reward, settlement, or training export.

## Acceptance criteria

- Domain-specific behavior is additive to the runtime, not a parallel architecture.
- HuggingFace integration exists because something uses it now.
- Market hooks remain explicit future boundaries unless there is real settlement logic to test.
