# docs/v1 cognition source coverage

This ledger records coverage for the v1 cognition and learning source corpus
encapsulated by `tmp/status-quo/backlog/plans/DOC-v1-cognition/tasks.toml`.

## Summary

- Source corpus: `docs/v1/05-learning`, `docs/v1/06-neuro`,
  `docs/v1/07-conductor`, `docs/v1/09-daimon`, `docs/v1/10-dreams`,
  `docs/v1/16-heartbeat`, and `docs/v1/17-lifecycle`.
- Source markdown files covered: 119.
- Authored task count: 7.
- Coverage rule: every source path below must also appear in at least one
  `[task.context].read_files` entry in the plan file.
- Validation command: `cargo run -q -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/DOC-v1-cognition`.

## Task Mapping

| Task id | Source directory | Docs covered | Local dependencies | Plan prerequisites |
|---|---:|---:|---|---|
| DOC-V1-COG-T01-learning | `docs/v1/05-learning` | 22 | none | E01, E05, E06, E07 |
| DOC-V1-COG-T02-neuro | `docs/v1/06-neuro` | 18 | DOC-V1-COG-T01-learning | E02, E03, E07 |
| DOC-V1-COG-T03-conductor | `docs/v1/07-conductor` | 17 | DOC-V1-COG-T01-learning, DOC-V1-COG-T02-neuro | E01, E05, E08 |
| DOC-V1-COG-T04-daimon | `docs/v1/09-daimon` | 15 | DOC-V1-COG-T01-learning, DOC-V1-COG-T02-neuro, DOC-V1-COG-T03-conductor | E01, E07, E08 |
| DOC-V1-COG-T05-dreams | `docs/v1/10-dreams` | 19 | DOC-V1-COG-T01-learning, DOC-V1-COG-T02-neuro, DOC-V1-COG-T04-daimon | E01, E07, E08 |
| DOC-V1-COG-T06-heartbeat | `docs/v1/16-heartbeat` | 14 | DOC-V1-COG-T01-learning, DOC-V1-COG-T02-neuro, DOC-V1-COG-T03-conductor, DOC-V1-COG-T04-daimon, DOC-V1-COG-T05-dreams | E01, E05, E07, E08, E09 |
| DOC-V1-COG-T07-lifecycle | `docs/v1/17-lifecycle` | 14 | DOC-V1-COG-T02-neuro, DOC-V1-COG-T04-daimon, DOC-V1-COG-T05-dreams, DOC-V1-COG-T06-heartbeat | E02, E07, E10, E17, E18 |

## Coverage Ledger

| Source path | Task id | Status |
|---|---|---|
| docs/v1/05-learning/00-episode-logger.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/01-playbook-system.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/02-skill-library-voyager.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/03-bandits-ucb-thompson-linucb.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/04-cascade-router.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/05-pattern-discovery-trigram.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/06-task-metrics-and-baselines.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/07-regression-detection.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/08-cost-normalization.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/09-provider-health-circuit-breaker.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/10-pareto-frontier-pruning.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/11-thompson-sampling-drift.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/12-self-improvement-frameworks.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/13-8-missing-feedback-loops.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/14-stability-mechanisms.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/15-collective-calibration-31x.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/16-predictive-foraging.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/17-adas-and-autocatalytic.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/18-self-learning-cybernetic-loops.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/19-heuristics-worldviews-and-falsifiers.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/20-research-to-runtime.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/05-learning/INDEX.md | DOC-V1-COG-T01-learning | covered |
| docs/v1/06-neuro/00-vision-and-grimoire-rename.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/01-six-knowledge-types.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/02-four-validation-tiers.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/03-type-half-lives.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/04-hdc-vsa-foundations.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/05-hdc-operations.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/06-hdc-knowledge-encoding.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/07-ebbinghaus-decay-with-tier.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/08-cross-domain-hdc-transfer.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/09-false-positive-math.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/10-knowledge-query-api.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/11-antiknowledge-challenge.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/12-4-tier-distillation-pipeline.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/13-somatic-integration.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/14-library-of-babel.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/15-knowledge-backup-restore.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/16-current-status-and-gaps.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/06-neuro/INDEX.md | DOC-V1-COG-T02-neuro | covered |
| docs/v1/07-conductor/00-conductor-architecture.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/01-watcher-ensemble.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/02-circuit-breaker.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/03-graduated-interventions.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/04-diagnosis-engine.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/05-stuck-detection.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/06-health-monitors.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/07-ooda-cybernetic-loop.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/08-good-regulator-self-model.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/09-cognitive-signals.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/10-adaptive-timeouts-state-machine.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/11-anomaly-detection-learning.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/12-yerkes-dodson-pressure.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/13-process-supervision-wiring.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/14-production-failure-catalog.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/15-conductor-learning-federation.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/07-conductor/INDEX.md | DOC-V1-COG-T03-conductor | covered |
| docs/v1/09-daimon/00-vision-and-mortality-incompatibility.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/01-pad-vector.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/02-alma-three-layer-temporal.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/03-occ-scherer-appraisal.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/04-six-behavioral-states.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/05-behavioral-state-to-tier-routing.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/06-somatic-markers-damasio.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/07-15-percent-contrarian-retrieval.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/08-8-dimensional-strategy-space.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/09-mood-congruent-memory.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/10-integration-points.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/11-coding-agent-integration.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/12-collective-emotional-contagion.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/13-current-status-and-gaps.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/09-daimon/INDEX.md | DOC-V1-COG-T04-daimon | covered |
| docs/v1/10-dreams/00-vision-and-dream-as-death-reframe.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/01-three-phase-cycle.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/02-nrem-replay.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/03-rem-imagination.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/04-consolidation-and-staging.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/05-dream-evolution.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/06-hdc-counterfactual-synthesis.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/07-hypnagogia-engine.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/08-divergence-and-alpha.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/09-threat-simulation.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/10-hauntology-in-dreams.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/11-inner-worlds-and-rendering.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/12-sleep-time-compute.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/13-scheduling-and-triggers.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/14-oneirography.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/15-cross-system-integration.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/16-implementation-status.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/17-advanced-dream-concepts.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/10-dreams/INDEX.md | DOC-V1-COG-T05-dreams | covered |
| docs/v1/16-heartbeat/00-coala-9-step-pipeline.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/01-universal-loop-mapping.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/02-chain-heartbeat-variant.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/03-three-cognitive-speeds.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/04-gamma-reactive-loop.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/05-theta-reflective-loop.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/06-delta-consolidation-loop.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/07-adaptive-clock.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/08-dual-process-t0-t1-t2.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/09-16-t0-probes.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/10-active-inference-compute-allocation.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/11-active-inference-state-space.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/12-attention-auction-and-gating.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/16-heartbeat/INDEX.md | DOC-V1-COG-T06-heartbeat | covered |
| docs/v1/17-lifecycle/00-vision-and-mortality-replaced.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/01-agent-creation.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/02-provisioning.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/03-configuration-and-operator-model.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/04-funding-and-budgets.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/05-knowledge-backup-export.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/06-agent-deletion.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/07-new-agent-creation.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/08-selective-restore.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/09-knowledge-transfer-via-mesh.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/10-ebbinghaus-for-knowledge-not-agents.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/11-knowledge-demurrage.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/12-academic-foundations.md | DOC-V1-COG-T07-lifecycle | covered |
| docs/v1/17-lifecycle/INDEX.md | DOC-V1-COG-T07-lifecycle | covered |
