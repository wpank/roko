# Oracle, Prediction, And Yield Perps Checklist

## Scope

Use this file for ISFR data-source ingestion, aggregation, confidence, precompile shape, CRPS scoring, and yield perpetual mechanics.

## Implementation checklist

- [ ] Define the `ISFRSource` trait around the current chain abstractions.
  - quote fetch;
  - timestamp/freshness;
  - per-source confidence;
  - error classification.
- [ ] Implement or stub the initial sources behind the trait.
  - Aave V3
  - Compound V3
  - Ethena sUSDe
  - ETH beacon/native staking proxy
- [ ] Implement aggregation with adversarial assumptions in mind.
  - dual median or equivalent robust aggregation;
  - stale-source rejection;
  - minimum quorum rules.
- [ ] Compute confidence explicitly.
  - freshness;
  - source agreement;
  - source coverage;
  - circuit-breaker state.
- [ ] Define the precompile or RPC interface only after the in-crate index representation is stable.
- [ ] Add CRPS-based prediction scoring as a separate module, not mixed into index publication.
- [ ] Implement yield perp math behind pure functions first.
  - mark price;
  - funding rate;
  - margin requirements;
  - P&L.

## Concrete file touchpoints

- `crates/roko-chain/src/isfr.rs`
- `crates/roko-chain/src/futures_market.rs`
- `crates/roko-chain/src/types.rs`
- `apps/mirage-rs/src/http_api/isfr.rs`

## Verification checklist

- [ ] Source failures do not produce a false valid index.
- [ ] Flash-loan or outlier resistance has an adversarial test.
- [ ] Perp pricing and funding are tested with deterministic fixtures.
- [ ] Prediction scoring is independent from settlement logic.

## Acceptance criteria

- ISFR can be computed from multiple sources with a visible confidence value.
- The index API is stable enough to become a `BenchmarkIndex` implementation.
- Perp math is unit-tested before any runtime wiring or UI work.
