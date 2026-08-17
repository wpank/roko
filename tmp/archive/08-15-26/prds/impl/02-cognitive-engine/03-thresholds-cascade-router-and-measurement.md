# Adaptive Thresholds, Cascade Router, And Measurement

## Scope

Use this file for the parts of PRD-03 that go beyond raw gating: adaptive-threshold families, temperament/neuro priors, mortality-aware clocks, cascade-router integration, and formal validation.

## Implementation checklist

- [ ] Implement adaptive-threshold evolution as a pluggable policy family.
  - EWMA baseline;
  - CUSUM detector;
  - SPC ensemble;
  - joint anomaly option such as Hotelling-style multivariate detection if enough features exist.
- [ ] Tie threshold policy to domain profiles.
  - coding defaults;
  - blockchain defaults;
  - research defaults;
  - environment-specific overrides in config.
- [ ] Add temperament-aware adjustments.
  - define supported temperament modes;
  - specify which thresholds or escalation penalties they modify;
  - persist temperament choice in config and surface it in diagnostics.
- [ ] Add neuro-informed priors where current knowledge/history exists.
  - prior familiarity from knowledge store;
  - warning density or anti-knowledge boost;
  - prior failures around the same patch/entity/task class.
- [ ] Expand the PRD's three clocks model from the PRD.
  - task/tick clock;
  - budget/cost/vitality clock;
  - longer-horizon lifecycle or exhaustion clock.
- [ ] Clarify what “mortality integration” means in the current architecture.
  - map legacy mortality language onto budget pressure, confidence decay, deadlines, and operator limits;
  - avoid reintroducing removed death-specific semantics as implementation requirements unless explicitly wanted.
- [ ] Integrate with CascadeRouter by stage.
  - static routing when observations are scarce;
  - confidence-based routing after minimal evidence;
  - UCB or learned routing once enough outcomes exist;
  - cognitive-tier and somatic policy as inputs, not just raw model names.
- [ ] Add arena-backed validation.
  - compare gated vs ungated cost;
  - compare threshold variants;
  - compare routing stages;
  - measure gate pass rate lift and cost reduction.
- [ ] Add anomaly and regression dashboards or logs.
  - threshold drift;
  - over-escalation rate;
  - under-escalation failures;
  - somatic false positives/false negatives.

## Additional gap-closure tasks

- [ ] Add explicit detector-selection tasks.
  - when to use EWMA only;
  - when to enable CUSUM;
  - when multivariate detection is justified;
  - safe defaults per domain.
- [ ] Add a task for threshold warm-start behavior.
  - first-run defaults;
  - restore from prior workspace state;
  - cross-agent or cross-workspace seeding rules if ever allowed.
- [ ] Add a task for dishabituation edge cases.
  - repeated low-value signals turning urgent again after a real failure;
  - time-decay reset rules;
  - interaction with somatic caution.
- [ ] Add a task for “reason for escalation” UX.
  - exact features that pushed a tick from T0 -> T1 or T1 -> T2;
  - operator-facing explanation text;
  - machine-readable explanation for audit trails.
- [ ] Add a task for cost-regression guardrails.
  - fail CI or benchmark checks if gated execution becomes more expensive than prior baseline without quality lift.

## Agent-ready task sequence

1. `CE-GAP-01` Threshold policy selection matrix
   - Scope: define exactly which detector family is enabled by default per domain and confidence level.
   - Touches: threshold policy config, domain defaults, docs/tests.
   - Deliverable: one machine-readable selection matrix and default config wiring.
   - Done when: an engineer can tell from config which threshold policy is active and why.

2. `CE-GAP-02` Threshold warm-start and restore behavior
   - Scope: formalize first-run defaults, persisted restore, and safe fallback when prior state is corrupt or missing.
   - Touches: threshold persistence files, boot-time loader, regression tests.
   - Deliverable: deterministic boot behavior for fresh and resumed workspaces.
   - Depends on: `CE-GAP-01`.
   - Done when: warm-started thresholds and fresh thresholds both pass fixture tests.

3. `CE-GAP-03` Escalation reason surface
   - Scope: emit exact features behind each tier escalation or de-escalation.
   - Touches: gate decision type, logs, audit trail output, possibly TUI/serve payloads.
   - Deliverable: machine-readable and human-readable escalation explanations.
   - Depends on: `CE-GAP-01`.
   - Done when: one test can assert the explanation text/fields for a known escalation case.

4. `CE-GAP-04` Dishabituation and somatic reactivation rules
   - Scope: define when repeated noise becomes urgent again after a failure or new negative somatic signal.
   - Touches: habituation state, somatic query integration, gate policy tests.
   - Deliverable: explicit reset/reactivation rules with fixtures.
   - Depends on: `CE-GAP-03`.
   - Done when: a once-habituated signal can be shown to re-escalate under the documented conditions.

5. `CE-GAP-05` Cost-regression benchmark gate
   - Scope: add a benchmark or CI guard that catches threshold/routing changes that increase cost without lift.
   - Touches: benchmark harness, CI task, recorded baselines.
   - Deliverable: one benchmark-based regression check tied to cognitive-engine changes.
   - Depends on: `CE-GAP-01`.
   - Done when: a known synthetic regression can fail the benchmark gate.

## Relevant current files

- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/model_router.rs`
- `crates/roko-learn/src/provider_health.rs`
- `crates/roko-learn/src/drift.rs`
- `crates/roko-daimon/src/lib.rs`
- `crates/roko-cli/tests/e2e_domain.rs`

## Verification checklist

- [ ] Threshold policy choice is visible in config and logs.
- [ ] CascadeRouter stage transitions are deterministic under fixture data.
- [ ] Arena or replay benchmarks can compare threshold/routing variants.
- [ ] Mortality/vitality language is translated into current runtime concepts consistently across code and docs.

## Acceptance criteria

- The cognitive engine has explicit threshold policies and validation metrics.
- CascadeRouter and cognitive gating reinforce each other instead of competing.
- Measurement covers both cost savings and quality outcomes.
