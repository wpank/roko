# Caching, Chain Context, And WorldGraph Checklist

## Scope

Use this file for section-effect tracking, cache tiers, InsightStore-backed retrieval, and future WorldGraph injection.

## Implementation checklist

- [ ] Add section-effect tracking where the current pipeline can observe outcomes.
  - record section/category presence;
  - record gate/outcome lift;
  - store per-domain measurements in `roko-learn`.
- [ ] Implement deterministic cache keys.
  - include task identity, domain, relevant workspace state, and selected sources;
  - explicitly exclude unstable fields that should not bust the cache.
- [ ] Define cache tiers with a concrete purpose.
  - local in-process memoization;
  - filesystem cache or persisted pack cache;
  - semantic similarity cache only when scoring is deterministic enough;
  - provider-side cache hints where current model APIs support them.
- [ ] Wire chain-sourced context conservatively.
  - start with a clear client boundary for InsightStore queries;
  - reputation-weight chain-derived entries;
  - preserve source and freshness metadata.
- [ ] Treat WorldGraph as future-facing unless a minimal implementation exists.
  - if no `roko-worldgraph` crate exists yet, create an interface boundary, not a fake full implementation;
  - define the bidder contract and the event/input sources it will need later.
- [ ] Add U-shaped placement and complexity-based token scaling only through the canonical prompt path.
  - include deterministic prefix alignment rules for cache reuse;
  - include any social foraging boost logic only as an explicit scoring term, not an undocumented bias.

## Relevant current files

- `crates/roko-learn/src/context_pack_cache.rs`
- `crates/roko-learn/src/section_effect.rs`
- `crates/roko-chain/src/isfr.rs`
- `crates/roko-chain/src/types.rs`
- `apps/mirage-rs/src/chain/insight.rs`
- `docs/08-chain/24-current-status-and-6-contracts.md`

## Verification checklist

- [ ] Cache hits and misses are visible in logs or metrics.
- [ ] Cache invalidation is deterministic under test.
- [ ] Chain-sourced entries are labeled and scoreable separately from local knowledge entries.
- [ ] If WorldGraph is not implemented yet, the code compiles with a stubbed interface and explicit TODO boundary.

## Acceptance criteria

- The context pipeline can explain both what it selected and what it reused.
- Chain context is additive, provenance-preserving, and reputation-aware.
- WorldGraph integration is staged behind a real interface boundary, not hand-wired into unrelated modules.
