# Attention Allocation, Publishing, And Ecosystem Completion

## Scope

Use this file for the PRD-09 sections that connect package ecosystem work to dynamic attention allocation and registry publishing: predictive foraging, active inference, package publishing, marketplace UX, arenas-as-packages, and phased ecosystem completion.

## Implementation checklist

- [ ] Flesh out predictive-foraging implementation details.
  - Gittins index struct and update rules;
  - uncertainty bonus;
  - monitoring cost model;
  - attention-budget output;
  - patch-switching / Marginal Value Theorem stopping rule.
- [ ] Map foraging outputs into current runtime consumers.
  - chain actors;
  - context retrieval;
  - event subscriptions;
  - benchmark surprise amplification.
- [ ] Add active-inference attention tasks as a separate policy layer.
  - expected free energy or proxy signals;
  - how it interacts with Gittins-style bandit allocation;
  - when one policy overrides the other.
- [ ] Add package publishing and registry tasks.
  - cognitive extension publish flow;
  - chain connector publish flow;
  - domain profile publish flow;
  - Pi-compatible extension publish flow;
  - registry validation rules;
  - install-count/market metadata.
- [ ] Add marketplace TUI/web tasks for packages.
  - browse;
  - inspect;
  - package info view;
  - install/update/remove;
  - update all extensions flow;
  - permissions review;
  - provenance and ABI compatibility display.
- [ ] Add arenas-as-packages tasks.
  - package metadata for arenas;
  - install and discover flow;
  - perpetual grinder scheduling;
  - scoreboard/result export.
- [ ] Add explicit implementation-phasing tasks from the PRD.
  - foundation;
  - Pi compatibility;
  - multi-domain;
  - multi-chain;
  - foraging;
  - WorldGraph/discovery;
  - ecosystem completion.

## Additional gap-closure tasks

- [ ] Add a task for connector capability declarations.
  - mempool support;
  - finality model;
  - block time;
  - decode coverage;
  - reorg expectations.
- [ ] Add a task for package-permission review UX.
  - install-time summary;
  - diff on upgrade;
  - denied-capability behavior.
- [ ] Add a task for dynamic discovery cache management.
  - selector/signature database refresh;
  - 4byte/local cache invalidation;
  - registry poisoning safeguards.
- [ ] Add a task for WorldGraph strategy-evolution checkpoints.
  - when entity/edge pruning runs;
  - when dream-driven hypothesis insertion runs;
  - operator visibility into graph evolution.
- [ ] Add a task for ecosystem health metrics.
  - package install success rate;
  - connector reliability;
  - arena-package adoption;
  - publish-to-install latency.

## Agent-ready task sequence

1. `EXT-GAP-01` Connector capability declaration schema
   - Scope: define a manifest/schema for mempool support, finality, decode coverage, and reorg expectations.
   - Touches: connector manifests, registry validation, docs.
   - Deliverable: one connector-capability schema with validation rules.
   - Done when: built-in and sample third-party connectors can declare comparable capability metadata.

2. `EXT-GAP-02` Package permission review UX
   - Scope: add install-time and upgrade-time permission summaries and diffs.
   - Touches: CLI install/update flows, marketplace UI/TUI, manifest parsing.
   - Deliverable: explicit permission review step for package install and upgrade.
   - Depends on: `EXT-GAP-01`.
   - Done when: package install output shows permissions and upgrade diffs without custom inspection.

3. `EXT-GAP-03` Dynamic discovery cache hygiene
   - Scope: manage selector/signature cache refresh and poisoning safeguards.
   - Touches: discovery pipeline caches, local registries, refresh jobs.
   - Deliverable: cache invalidation and trust policy for discovery metadata.
   - Depends on: none.
   - Done when: stale or poisoned selector data can be invalidated and replaced cleanly under test.

4. `EXT-GAP-04` WorldGraph evolution checkpoints
   - Scope: define when pruning, dream-driven hypothesis insertion, and graph-evolution updates run.
   - Touches: WorldGraph update cycle, dream integration, runtime scheduling.
   - Deliverable: one checkpoint schedule for graph maintenance and evolution.
   - Depends on: `EXT-GAP-01`.
   - Done when: graph evolution work has a deterministic cadence and audit trail.

5. `EXT-GAP-05` Ecosystem health dashboard metrics
   - Scope: define the core health metrics for packages, connectors, and arena packages.
   - Touches: registry metrics, CLI/serve/dashboard reporting.
   - Deliverable: one metrics set and one reporting surface for ecosystem health.
   - Depends on: `EXT-GAP-02`.
   - Done when: ecosystem operators can inspect install success, reliability, and publish-to-install latency.

## Verification checklist

- [ ] Foraging outputs can be replayed and inspected.
- [ ] Registry validation rejects malformed publish attempts with useful errors.
- [ ] Package/arena marketplace views expose permissions and compatibility clearly.
- [ ] Phase boundaries are documented so deferred work is still accounted for.

## Acceptance criteria

- Predictive foraging and package publishing are both represented as concrete implementation work.
- Ecosystem completion is phased explicitly rather than implied.
- The package ecosystem has a credible operator/developer UX, not only manifest design.
