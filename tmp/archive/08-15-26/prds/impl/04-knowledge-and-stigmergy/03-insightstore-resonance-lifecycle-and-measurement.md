# InsightStore, Resonance, Lifecycle, And Measurement

## Scope

Use this file for the shared-knowledge half of PRD-05: on-chain entry/query semantics, pheromone dynamics, resonance, temporal topology, network effects, and C-factor measurement.

## Implementation checklist

- [ ] Define the shared knowledge contract in terms of current local knowledge kinds.
  - Insight;
  - Heuristic;
  - Warning;
  - CausalLink;
  - StrategyFragment;
  - AntiKnowledge.
- [ ] Specify the on-chain/shared entry format boundary.
  - minimal metadata on chain;
  - content-addressed payload location;
  - submitter/reputation/freshness fields;
  - query result envelope.
- [ ] Implement or stub pheromone dynamics concretely.
  - potency/weight;
  - confirmation/reset mechanics;
  - demurrage/decay;
  - local simulation in mirage if the real chain path is not ready.
- [ ] Wire cross-domain resonance from existing HDC components.
  - resonance detector invocation;
  - Lotka-Volterra or other interaction model as a separate module;
  - runtime consumers for discovered resonance, not just offline computation.
- [ ] Add temporal knowledge topology tasks.
  - time-scoped retrieval slices;
  - historical vs current knowledge queries;
  - topology-aware pruning or promotion if implemented.
- [ ] Add generalized benchmark and collective-intelligence measurement tasks.
  - section-effect and contribution metrics;
  - C-factor computation;
  - leave-one-out contribution measurement;
  - cross-agent lift benchmarks.
- [ ] Encode network-effects and thousandth-agent-advantage claims as measurable benchmarks rather than narrative only.

## Additional gap-closure tasks

- [ ] Add a task for explicit counterfactual output types from dreams and resonance work.
  - hypothesis records;
  - validation status;
  - promotion/rejection path into or out of shared knowledge.
- [ ] Add a task for reputation-aware publish throttling.
  - low-reputation flood resistance;
  - novelty thresholds;
  - cooldowns for repeated low-value publications.
- [ ] Add a task for AntiKnowledge lifecycle measurement.
  - false refutation rate;
  - time to confirmation or reversal;
  - effect on retrieval and planning behavior.
- [ ] Add a task for cross-domain resonance false-positive evaluation.
  - adjudication set;
  - threshold sweeps;
  - operator review workflow where needed.
- [ ] Add a task for shared-knowledge backup/export semantics.
  - what can be exported locally;
  - what must remain redacted;
  - replay/import path for local testing and disaster recovery.

## Agent-ready task sequence

1. `KN-GAP-01` Counterfactual knowledge record type
   - Scope: define hypothesis/counterfactual records and their validation lifecycle.
   - Touches: neuro types, dream outputs, measurement schema.
   - Deliverable: explicit data model for generated-but-not-yet-validated hypotheses.
   - Done when: dream/resonance code can emit a hypothesis artifact without promoting it to normal knowledge prematurely.

2. `KN-GAP-02` Reputation-aware publish throttling
   - Scope: throttle low-value or low-reputation publication spam without banning legitimate new contributors.
   - Touches: publisher policy, publication queue, metrics.
   - Deliverable: novelty/cooldown/reputation-aware throttling rules.
   - Depends on: `KN-GAP-01`.
   - Done when: repeated low-value submissions are suppressed while one novel submission still passes.

3. `KN-GAP-03` AntiKnowledge measurement path
   - Scope: track false refutations, reversals, and retrieval impact of AntiKnowledge.
   - Touches: knowledge analytics, query path, measurement exports.
   - Deliverable: AntiKnowledge-specific metrics and one regression fixture.
   - Depends on: `KN-GAP-01`.
   - Done when: AntiKnowledge records show measurable downstream effect in retrieval tests.

4. `KN-GAP-04` Resonance false-positive evaluation set
   - Scope: create adjudication fixtures and threshold sweeps for cross-domain resonance quality.
   - Touches: resonance detector tests/benchmarks, evaluation docs.
   - Deliverable: reusable resonance evaluation set and threshold recommendations.
   - Depends on: `KN-GAP-01`.
   - Done when: threshold tuning is driven by a repeatable fixture set, not intuition.

5. `KN-GAP-05` Shared-knowledge backup/export path
   - Scope: define redacted export/import semantics for local testing and recovery.
   - Touches: export CLI or helper path, redaction logic, import/replay path.
   - Deliverable: one documented local export/import flow for shared-knowledge artifacts.
   - Depends on: `KN-GAP-02`.
   - Done when: a redacted export can be imported into a fresh local environment for replay tests.

## Relevant current files

- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-learn/src/resonant_patterns.rs`
- `apps/mirage-rs/src/chain/insight.rs`
- `apps/mirage-rs/src/chain/pheromone.rs`
- `docs/13-coordination/12-current-status-and-gaps.md`

## Verification checklist

- [ ] Shared-entry query envelopes match the local knowledge vocabulary.
- [ ] Pheromone reinforcement and demurrage can be simulated deterministically.
- [ ] Resonance output is consumed somewhere observable, not left as dead analysis.
- [ ] Collective-intelligence metrics can be recomputed from persisted artifacts.

## Acceptance criteria

- PRD-05’s shared-knowledge and network-effect claims are represented as implementation tasks.
- Resonance, lifecycle, and measurement are no longer hidden behind local-store work only.
- C-factor and shared-knowledge lift are measurable, not just aspirational.
