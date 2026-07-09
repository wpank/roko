# Knowledge Pipeline And HDC Checklist

## Scope

Use this file for episode clustering, resonance detection, fingerprint enrichment, and PP-HDC groundwork.

## Implementation checklist

- [ ] Inventory current write paths into `roko-neuro`.
  - episode distillation;
  - direct knowledge writes;
  - context retrieval paths;
  - any existing dream or orchestrator writeback.
- [ ] Wire the knowledge pipeline stages explicitly.
  - episode completion;
  - clustering;
  - resonance detection;
  - heuristic/insight promotion;
  - somatic tagging or provenance transfer where already supported.
- [ ] Strengthen fingerprints using existing learn primitives.
  - task description encoding;
  - tool-call sequence encoding;
  - support for similarity threshold tests.
- [ ] Reuse `roko-primitives` HDC operations for encoding.
  - bind
  - bundle
  - similarity
  - deterministic fingerprinting
- [ ] Add PP-HDC only behind a clearly named module boundary.
  - encode;
  - role unbind;
  - quality gate;
  - distance-preservation tests.
- [ ] Keep local retrieval quality measurable.
  - compare raw confidence vs combined ranking;
  - confirm HDC-enriched retrieval improves at least one benchmarked query class.

## Concrete file touchpoints

- `crates/roko-neuro/src/lib.rs`
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-neuro/src/distiller.rs`
- `crates/roko-neuro/src/tier_progression.rs`
- `crates/roko-learn/src/hdc_fingerprint.rs`
- `crates/roko-learn/src/hdc_clustering.rs`
- `crates/roko-learn/src/resonant_patterns.rs`
- `crates/roko-primitives/src/hdc.rs`

## Verification checklist

- [ ] Similar episodes cluster together in deterministic tests.
- [ ] Fingerprint enrichment improves or at least preserves retrieval quality on regression fixtures.
- [ ] PP-HDC encoding has explicit round-trip or distance-preservation tests.
- [ ] Local knowledge queries remain bounded in latency.

## Acceptance criteria

- The knowledge pipeline is a documented series of real stages, not disconnected helpers.
- Fingerprints reflect both task semantics and process history.
- HDC work is shared across subsystems instead of reimplemented ad hoc.
- Privacy-preserving encoding is introduced with quantitative validation, not only prose claims.
