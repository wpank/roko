# Identity, Registries, Proof Log, And Rollout

## Scope

Use this file for the on-chain identity and verification surfaces that the PRDs rely on repeatedly: Agent Passport, Reputation Registry, Validation Registry, `PROOF_LOG`, and staged rollout through simulator-first contract work.

## Implementation checklist

- [ ] Define the Agent Passport backlog explicitly.
  - identity payload;
  - capability manifest;
  - stake tier;
  - reputation pointers;
  - service endpoints;
  - runtime fingerprint.
- [ ] Define the Reputation Registry backlog explicitly.
  - per-track scores;
  - update semantics;
  - EMA or rolling-window behavior;
  - authorized feedback sources;
  - slashing and discipline hooks where applicable.
- [ ] Define the Validation Registry backlog explicitly.
  - work proof payload;
  - gate result attachment;
  - clearing certificate attachment;
  - retrieval/query shape.
- [ ] Define the `PROOF_LOG` backlog explicitly.
  - prediction commitments;
  - scoring writeback path;
  - query API for downstream scorers and surfaces.
- [ ] Stage each registry through mirage or equivalent first.
  - simulator address allocation;
  - fixtures and event emission;
  - read/write client stubs in `roko-chain`.
- [ ] Tie these registries back into product consumers.
  - jobs/marketplace;
  - passport-based discovery;
  - epistemic reputation in ISFR;
  - operator/dashboard identity views.

## Additional gap-closure tasks

- [ ] Add a task for Agent Passport rotation and update semantics.
  - endpoint changes;
  - runtime fingerprint updates;
  - delegated capability changes;
  - timelock or governance constraints where required.
- [ ] Add a task for pseudonymous vs disclosed identity handling.
  - what surfaces show by default;
  - opt-in disclosure fields;
  - privacy-preserving public views.
- [ ] Add a task for registry read-model projection.
  - cache or projection layer in `roko-serve`;
  - frontend-friendly aggregation;
  - stale-data policy.
- [ ] Add a task for cross-registry consistency checks.
  - passport exists before reputation updates;
  - validation records point to known agents/jobs;
  - proof-log references resolve cleanly.
- [ ] Add a task for simulator genesis fixtures.
  - predeployed registry addresses;
  - seeded sample agents;
  - repeatable state for end-to-end tests.

## Agent-ready task sequence

1. `CHAIN-ID-01` Agent Passport update model
   - Scope: define how endpoint, fingerprint, and capability updates happen safely.
   - Touches: passport type/contracts, client update path, docs.
   - Deliverable: one update/rotation contract and client flow.
   - Done when: passport metadata changes can be modeled without ambiguous side effects.

2. `CHAIN-ID-02` Pseudonymous/disclosed identity projection
   - Scope: define what is public by default and what is opt-in.
   - Touches: registry read model, serve projection layer, UI-facing types.
   - Deliverable: one projection policy for pseudonymous vs disclosed views.
   - Depends on: `CHAIN-ID-01`.
   - Done when: the same registry record can be rendered safely in public and operator contexts.

3. `CHAIN-ID-03` Registry read-model projection
   - Scope: create backend-friendly projections for passports, reputation, validation, and proof logs.
   - Touches: `roko-serve` projections/state layer, chain client adapters.
   - Deliverable: one projection cache/schema for registry-backed UI/API reads.
   - Depends on: `CHAIN-ID-02`.
   - Done when: frontends no longer need to understand raw registry layout to consume the data.

4. `CHAIN-ID-04` Cross-registry consistency checks
   - Scope: validate references across passport, reputation, validation, and proof records.
   - Touches: chain-client validation helpers, projection layer, tests.
   - Deliverable: integrity checks and failure diagnostics.
   - Depends on: `CHAIN-ID-03`.
   - Done when: inconsistent registry references are caught in tests and surfaced clearly.

5. `CHAIN-ID-05` Simulator genesis fixture pack
   - Scope: seed repeatable registry state for end-to-end development and UI tests.
   - Touches: mirage setup, fixture generators, test docs.
   - Deliverable: one deterministic genesis fixture set with sample agents, scores, and proofs.
   - Depends on: `CHAIN-ID-03`.
   - Done when: local end-to-end scenarios can boot with consistent registry state.

## Verification checklist

- [ ] Contract or simulator interfaces are versioned and testable.
- [ ] Client code can read/write the registry envelopes without depending on undeployed chain infrastructure.
- [ ] Proof-log scoring data can be consumed by ISFR or prediction-scoring code paths.

## Acceptance criteria

- The PRD’s recurring registry concepts are represented as concrete implementation backlog.
- Identity, reputation, and proof artifacts have a chain-facing home in the plan tree.
- Simulator-first rollout is explicit instead of assumed.
