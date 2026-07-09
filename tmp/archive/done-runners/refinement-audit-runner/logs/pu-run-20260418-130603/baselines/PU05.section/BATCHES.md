# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to "cover the learning docs", but to let an agent turn learning-parity findings into bounded work that can run overnight without guessing which learning surfaces are already real.

---

## Batch Posture

- Default strategy: **promote already-shipped learning surfaces into clearer production contracts before adding more learning theory**.
- Treat `crates/roko-cli/src/orchestrate.rs` and `crates/roko-learn/src/runtime_feedback.rs` as the primary conflict hotspots.
- Treat `playbook_rules.rs`, `skill_library.rs`, `baseline.rs`, `regression.rs`, `prediction.rs`, `cascade_router.rs`, and `routing_log.rs` as the key contract modules.
- If a task starts requiring novel routing research, new storage architecture, or governance / constitutional systems, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any newly clarified runtime contract.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`L1 -> L2 -> L3 -> L5 -> L6 -> L4 -> L7 -> L8`

This order first makes the main learned-context and regression contracts real, then clarifies predictive calibration, then tightens the remaining partial feedback loops, and only after that resolves dead scaffolding and docs-heavy truth-in-advertising work.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| L1 | B.08, B.12 | Activate richer learned-context matching for playbook rules and skills | `roko-cli`, `roko-learn`, `roko-compose` | `cargo test -p roko-cli -p roko-learn -p roko-compose` | 220 |
| L2 | D.07, D.08 | Make regression detection slice-aware and activate iteration regressions | `roko-learn` regression/baseline, `roko-cli` consumers | `cargo test -p roko-learn -p roko-cli` | 180 |
| L3 | E.16, E.17 | Make predictive calibration use one canonical data path and add real metrics | `roko-learn`, `roko-core`, `roko-cli` | `cargo test -p roko-learn -p roko-core -p roko-cli` | 240 |
| L4 | E.18 | Resolve dead learning subscriber / drift scaffolding | `roko-learn`, runtime startup integration | `cargo test -p roko-learn -p roko-cli` | 180 |
| L5 | E.07, D.13 | Turn budget pressure into a clearer routing input instead of a narrow override | `roko-learn`, `roko-cli` | `cargo test -p roko-learn -p roko-cli` | 180 |
| L6 | E.09 | Materialize experiment winners into a durable operator-facing artifact | `roko-learn`, `roko-cli` experiment/config surfaces | `cargo test -p roko-learn -p roko-cli` | 160 |
| L7 | A.01, A.07, A.09, B.10 | Make episode-storage / clustering / monotonic-growth docs match the real contract | docs + small learning contract helpers if needed | `rg -n "compact|prune_stale|EpisodeStorageConfig|EpisodeCluster" crates tmp/docs-parity docs/05-learning` | 80 |
| L8 | F.07, F.08 | Demote prescriptive improvement-measurement and safety blocks to explicit handoff status | docs / comments / handoff notes | `rg -n "ImprovementScoreCard|SafetyInvariants|GateGamingDetector|ConstitutionalConstraints" crates docs/05-learning tmp/docs-parity/05` | 60 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| L1 | — |
| L2 | — |
| L3 | — |
| L4 | L3 |
| L5 | — |
| L6 | L5 |
| L7 | — |
| L8 | — |

Why `L4 -> L3`:

- resolving dead subscriber / drift scaffolding is easier after the predictive-calibration contract is clearer.

Why `L6 -> L5`:

- experiment materialization is easier to reason about once router cost-pressure behavior is less ambiguous.

Parallel-safe groups:

- `{L1, L2, L3, L5, L7, L8}` can start immediately.
- `L4` should wait for `L3`.
- `L6` should wait for `L5`.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| orchestrate-learning | `crates/roko-cli/src/orchestrate.rs` | L1, L3, L5, L6 |
| learned-context | `crates/roko-learn/src/playbook_rules.rs`, `skill_library.rs`, `crates/roko-compose`, `orchestrate.rs` | L1 |
| regression | `crates/roko-learn/src/baseline.rs`, `regression.rs`, `runtime_feedback.rs`, consumer logs | L2 |
| calibration | `crates/roko-learn/src/prediction.rs`, `routing_log.rs`, `crates/roko-core/src/prediction.rs`, `orchestrate.rs` | L3 |
| subscriber | `crates/roko-learn/src/event_subscriber.rs`, `drift.rs`, runtime wiring | L4 |
| router-budget | `crates/roko-learn/src/cascade_router.rs`, `budget.rs`, `orchestrate.rs` | L5, L6 |
| docs-contract | `docs/05-learning/*`, `tmp/docs-parity/05/*` | L7, L8 |

---

## Batch Details

### L1 — Learned-Context Trigger Activation

**Owns**: `B.08`, `B.12`

**Read first**:
- [B-knowledge-tiers.md](B-knowledge-tiers.md)
- [E-feedback-calibration.md](E-feedback-calibration.md)

**Problem**: the main production learned-context path uses only `role` for playbook-rule matching and only `search_by_tag(role)` for skills, leaving most of the richer selection surface dormant.

**Scope**:
1. Populate `MatchContext` with real task files, tags, category, and last error signature where available.
2. Decide whether the main learned-skill path should continue using `search_by_tag` or move to `SkillQuery::select`.
3. Keep the fallback deterministic when metadata is absent.
4. Add tests or production evidence that non-role triggers can now fire.

**Out of scope**:
- inventing `ToolUsageProfile` or `ToolSequencePattern` systems,
- redesigning playbook confidence math,
- changing prompt-composition architecture beyond what the learned context needs.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-learn/src/playbook_rules.rs`
- `crates/roko-learn/src/skill_library.rs`
- `crates/roko-compose` only if prompt forwarding needs small adjustments

**Verify**:
```bash
cargo test -p roko-cli -p roko-learn -p roko-compose
rg -n "build_learned_context|MatchContext|search_by_tag|SkillQuery" crates/roko-cli crates/roko-learn crates/roko-compose
```

**Acceptance criteria**:
- file/tag/category/error-signature triggers can fire on a production path,
- learned skill retrieval is richer or explicitly justified as role-only fallback,
- later agents do not need to infer why most rule triggers never match.

---

### L2 — Slice-Aware Regression Detection

**Owns**: `D.07`, `D.08`

**Read first**:
- [D-metrics-cost-health.md](D-metrics-cost-health.md)

**Problem**: slice-aware baselines exist, but regression detection only emits overall alerts and ignores iteration regression thresholds entirely.

**Scope**:
1. Iterate `baseline.slices` against the current baseline.
2. Populate `RegressionAlert::slice` when slice-specific regressions or improvements are found.
3. Activate the `iterations_increase` threshold.
4. Preserve the existing overall alerts.
5. Add tests covering both overall and slice-specific cases.

**Out of scope**:
- advanced drift detectors,
- dashboard redesign,
- policy decisions about automatic rollback.

**Files**:
- `crates/roko-learn/src/regression.rs`
- `crates/roko-learn/src/baseline.rs` only if helper APIs are needed
- `crates/roko-cli/src/orchestrate.rs` only if alert consumers need updates

**Verify**:
```bash
cargo test -p roko-learn -p roko-cli
rg -n "detect_regressions|iterations_increase|slice: None|slice:" crates/roko-learn crates/roko-cli
```

**Acceptance criteria**:
- some production or test alerts carry `slice: Some(...)`,
- `iterations_increase` is no longer dead,
- overall regression detection still works as before.

---

### L3 — Predictive Calibration Canonicalization

**Owns**: `E.16`, `E.17`

**Read first**:
- [E-feedback-calibration.md](E-feedback-calibration.md)
- [C-routing-bandits.md](C-routing-bandits.md)

**Problem**: predictive calibration currently mixes two stories: routing-log replay is the real runtime source for prompt/scoring policies, while the direct `PredictionRecord::register/resolve` path exists but is unused. Doc 16 overstates what ships.

**Scope**:
1. Make one prediction/calibration data path the explicit source of truth.
2. Prefer building on the routing log if it already contains the needed fields, rather than adding a second parallel prediction store.
3. Add at least one real calibration metric surface beyond mean bias, such as Brier-style scoring, reliability bins, or equivalent summary output.
4. Keep prompt / scorer consumers aligned with that canonical path.

**Out of scope**:
- full predictive-foraging engine design,
- complex forecasting models,
- routing research beyond calibration of the current system.

**Files**:
- `crates/roko-learn/src/prediction.rs`
- `crates/roko-learn/src/routing_log.rs`
- `crates/roko-core/src/prediction.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-learn -p roko-core -p roko-cli
rg -n "PredictionRecord|CalibrationTracker|PredictionPolicy|PredictiveScorer|routing_log|brier|reliability" crates/roko-learn crates/roko-core crates/roko-cli
```

**Acceptance criteria**:
- there is one obvious calibration source of truth,
- at least one non-test calibration metric exists on that path,
- docs no longer imply a second nonexistent primary pipeline.

---

### L4 — Dead Subscriber / Drift Resolution

**Owns**: `E.18`

**Read first**:
- [E-feedback-calibration.md](E-feedback-calibration.md)

**Problem**: `run_learning_subscriber` and `DriftDetector` are large enough to matter, but currently ambiguous enough that later agents cannot tell whether they are intended runtime surfaces or dead scaffolding.

**Scope**:
1. Decide whether the subscriber path is going live or being explicitly demoted.
2. If it goes live, wire it to one real runtime event-bus path.
3. If it stays out of path, leave an explicit demotion / deletion note rather than silent dead code.
4. Apply the same treatment to `DriftDetector`.

**Out of scope**:
- building a new global event architecture,
- feature-expanding the subscriber before it has one runtime owner,
- broad anomaly-system redesign.

**Files**:
- `crates/roko-learn/src/event_subscriber.rs`
- `crates/roko-learn/src/drift.rs`
- runtime startup / orchestrator wiring if activation is chosen

**Verify**:
```bash
cargo test -p roko-learn -p roko-cli
rg -n "run_learning_subscriber|DriftDetector" crates/roko-learn crates/roko-cli
```

**Acceptance criteria**:
- dead-module ambiguity is resolved,
- either there is a production caller or there is an explicit demotion path,
- later agents do not have to guess whether these modules are meant to be live.

---

### L5 — Budget Pressure As Routing Input

**Owns**: `E.07`, `D.13`

**Read first**:
- [E-feedback-calibration.md](E-feedback-calibration.md)
- [D-metrics-cost-health.md](D-metrics-cost-health.md)

**Problem**: cost pressure is currently more of a pre-dispatch guardrail than a graded routing signal, which is narrower than the loop docs imply.

**Scope**:
1. Turn budget pressure into a clearer input to router selection, stage choice, or candidate scoring.
2. Preserve the hard safety override as a backstop.
3. Make the pressure model more expressive than a single boolean if practical.
4. Add tests that show routing behavior changes under budget pressure.

**Out of scope**:
- inventing a full economic planner,
- redesigning the entire cascade router,
- changing pricing sources or tokenizer normalization.

**Files**:
- `crates/roko-learn/src/budget.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-learn -p roko-cli
rg -n "BudgetGuardrail|BudgetAction|apply_cost_pressure|RouteToCheaper|BlockNewSessions" crates/roko-learn crates/roko-cli
```

**Acceptance criteria**:
- budget pressure affects routing in a real, inspectable way,
- hard blocking still exists as safety backstop,
- the loop is no longer accurately described as “routing” when it is really only override logic.

---

### L6 — Experiment Winner Materialization

**Owns**: `E.09`

**Read first**:
- [E-feedback-calibration.md](E-feedback-calibration.md)

**Problem**: experiment conclusions update router state, but there is no clean durable operator-facing artifact or apply step that makes the result obvious.

**Scope**:
1. Leave a durable artifact when an experiment winner is concluded.
2. Make that artifact operator-usable without reverse-engineering internal JSON state.
3. Prefer a small explicit apply / export path over hidden side effects.
4. Add a minimal verification path showing a concluded experiment changes persisted state in an inspectable way.

**Out of scope**:
- full experiment dashboard tooling,
- broad config schema redesign,
- shadow-testing framework expansion.

**Files**:
- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-learn/src/prompt_experiment.rs` and/or `model_experiment.rs`
- `crates/roko-cli` command surface if an apply/export command is added

**Verify**:
```bash
cargo test -p roko-learn -p roko-cli
rg -n "on_experiment_concluded|apply-experiments|experiments.json|cascade_router" crates/roko-learn crates/roko-cli
```

**Acceptance criteria**:
- a concluded experiment leaves a durable and inspectable winner artifact,
- operators do not need to infer results from raw router state,
- the batch stops short of a full experiment platform.

---

### L7 — Episode / Storage / Clustering Truth In Advertising

**Owns**: `A.01`, `A.07`, `A.09`, `B.10`

**Read first**:
- [A-episodes-patterns.md](A-episodes-patterns.md)
- [B-knowledge-tiers.md](B-knowledge-tiers.md)

**Problem**: several learning docs still describe storage, clustering, and monotonic-growth contracts that do not match the actual shipped code.

**Scope**:
1. Align the episode schema description with the real struct.
2. Make `EpisodeLogger::compact` the explicit retention story if that is the current contract.
3. Mark tiered storage and DBSCAN clustering as design-only unless a minimal runtime stub already exists.
4. Align the skill-growth docs with the existence of `prune_stale(days)`.

**Out of scope**:
- building tiered storage,
- implementing DBSCAN,
- redesigning retention policies.

**Files**:
- `docs/05-learning/00-episode-logger.md`
- `docs/05-learning/02-skill-library-voyager.md`
- `tmp/docs-parity/05/*` as needed

**Verify**:
```bash
rg -n "compact|prune_stale|EpisodeStorageConfig|CompressedEpisodeSummary|EpisodeCluster|incremental_dbscan" crates docs/05-learning tmp/docs-parity/05
```

**Acceptance criteria**:
- docs describe the real storage / retention / pruning contract,
- design-only sections are explicit,
- later agents are not pushed toward nonexistent storage or clustering systems.

---

### L8 — Improvement Measurement And Safety Handoff

**Owns**: `F.07`, `F.08`

**Read first**:
- [F-frameworks-vision.md](F-frameworks-vision.md)

**Problem**: doc 12 contains strong prescriptive measurement and safety blocks that read more “shipped” than the code supports.

**Scope**:
1. Remove ambiguity about whether improvement scorecards and safety-invariant systems exist.
2. Add explicit handoff notes for later eval / governance work.
3. Keep the doc valuable as a framework mapping without implying missing code is already present.

**Out of scope**:
- implementing significance-test pipelines,
- constitutional policy engines,
- new gate-gaming detectors.

**Files**:
- `docs/05-learning/12-self-improvement-frameworks.md`
- `tmp/docs-parity/05/*` as needed

**Verify**:
```bash
rg -n "ImprovementScoreCard|PeriodMetrics|SignificanceTests|SafetyInvariants|GateGamingDetector|ConstitutionalConstraints" crates docs/05-learning tmp/docs-parity/05
```

**Acceptance criteria**:
- prescriptive nonexistent systems are clearly labeled as deferred,
- later agents get an explicit owning batch / pass for follow-up,
- the framework-mapping docs remain useful without overstating shipping status.
