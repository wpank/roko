# Context Mesh, Measurement, And Persistence

## Scope

Use this file for cross-agent context sharing, section-effect attribution, cache economics, persistence layout, and operator-visible diagnostics.

## Implementation checklist

- [ ] Implement or formalize `ContextMesh` only as a scoped shared surface.
  - plan-local or atelier-local scope first;
  - thread-safe publication/query model;
  - deduplication rules;
  - staleness eviction.
- [ ] Define publication entry types for mesh sharing.
  - error;
  - pattern;
  - caution/warning;
  - local discovery or excerpt;
  - source/provenance metadata.
- [ ] Prevent echo and duplication.
  - exclude self-publications by default;
  - overlap-based deduplication;
  - winner selection for near-duplicate entries.
- [ ] Record section-effect outcomes in a way compatible with causal analysis.
  - presence/absence of sections;
  - outcome labels;
  - domain and role;
  - confidence intervals or uncertainty estimates.
- [ ] Add optional leave-one-out or Shapley-style attribution only where the cost is justified.
- [ ] Persist learned context state exactly where the PRD expects it.
  - section effects;
  - influence data;
  - budget predictor state;
  - context policy;
  - attention-curve calibration;
  - experiments and efficiency traces.
- [ ] Add measurement tasks for cache economics.
  - local tier hit rates;
  - gateway-added cache tiers where applicable;
  - dollar/token savings from hits;
  - under-utilization or pathological churn.
- [ ] Surface diagnostics for operators.
  - budget utilization;
  - winning vs losing bidders;
  - chain-context lift;
  - cross-agent mesh contribution rate.

## Additional gap-closure tasks

- [ ] Add a task for mesh namespace scoping.
  - atelier-local vs plan-local vs future multi-agent/global scopes;
  - permissions on who can read or publish;
  - clean teardown semantics at end of run.
- [ ] Add a task for context-pack explainability snapshots.
  - save the assembled pack fingerprint;
  - section order;
  - winning bids and externality/payments if VCG accounting is enabled.
- [ ] Add a task for prefix-alignment calibration by provider/model.
  - which sections must be stable for KV reuse;
  - cache-break placement rules;
  - regression tests for accidental prefix churn.
- [ ] Add a task for social-foraging safeguards.
  - how chain or mesh popularity boosts are bounded;
  - preventing herd effects from swamping higher-quality local evidence.
- [ ] Add a task for persistence compaction and GC.
  - when learned context files are compacted;
  - stale experiment eviction;
  - corruption recovery for partial writes.

## Agent-ready task sequence

1. `CTX-GAP-01` Mesh namespace model
   - Scope: define atelier-local, plan-local, and future wider scopes plus read/publish permissions.
   - Touches: any `ContextMesh` type, mesh query API, docs.
   - Deliverable: explicit namespace and permission model with one default scope.
   - Done when: two concurrent plans cannot accidentally read each other’s mesh entries in tests.

2. `CTX-GAP-02` Context-pack explainability snapshot
   - Scope: persist the assembled pack fingerprint, section order, and winning bid summary.
   - Touches: pack assembly path, persistence layout, debug/export helpers.
   - Deliverable: one snapshot artifact per assembled context pack.
   - Depends on: `CTX-GAP-01`.
   - Done when: a failed task can be traced back to the exact context pack that was sent.

3. `CTX-GAP-03` Prefix-alignment calibration
   - Scope: encode stable-prefix rules per provider/model so cache reuse is deliberate.
   - Touches: prompt assembly ordering, cache-key logic, provider calibration data.
   - Deliverable: deterministic prefix policy and regression fixtures for accidental churn.
   - Depends on: `CTX-GAP-02`.
   - Done when: a no-op task rerun preserves the aligned prefix under test.

4. `CTX-GAP-04` Social-foraging safety bounds
   - Scope: bound popularity/collective-boost effects so mesh or chain popularity cannot swamp quality.
   - Touches: bidder scoring, context ranking, measurement logs.
   - Deliverable: capped social-boost term with diagnostic output.
   - Depends on: `CTX-GAP-01`.
   - Done when: contrived popularity spikes cannot override a higher-quality local result past the configured cap.

5. `CTX-GAP-05` Persistence compaction and corruption recovery
   - Scope: compact learned context files and recover safely from partial writes.
   - Touches: persistence loaders/writers under `.roko/learn`.
   - Deliverable: compaction policy, stale-entry eviction, corruption fallback path.
   - Depends on: `CTX-GAP-02`.
   - Done when: corrupted context state files fall back safely and compaction leaves semantics intact.

## Relevant current files

- `crates/roko-learn/src/section_effect.rs`
- `crates/roko-learn/src/context_pack_cache.rs`
- `crates/roko-learn/src/efficiency.rs`
- `crates/roko-compose/src/`
- `crates/roko-cli/src/tui/`
- `crates/roko-serve/src/routes/projections.rs`

## Verification checklist

- [ ] Mesh publications are queryable and evictable under test.
- [ ] Learned context state survives restart when persisted.
- [ ] Cache-hit metrics and section-effect records can be inspected without custom debugging.
- [ ] Cross-agent sharing remains scoped and does not leak unrelated context.

## Acceptance criteria

- The PRD’s context-mesh and persistence concepts are represented as concrete implementation work.
- Measurement is first-class, not an afterthought.
- Operators can see why the context system is helping or failing.
