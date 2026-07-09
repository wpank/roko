# Batch Execution Contract

Narrowed follow-up batches for the learning parity work. Each batch should be realistic for **one Codex agent in 45-90 minutes**, with clear deferrals when the work starts turning into new architecture.

---

## Batch Posture

- Default strategy: **use more of the learning runtime that already ships** before inventing new learning theory.
- Prefer code paths with existing callsites over research-only modules.
- If a task starts requiring a new router family, a new storage architecture, or a governance framework, stop and hand it off.
- Keep `docs/05-learning/` and `tmp/docs-parity/05/` honest about what is present tense and what is only a target-state.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning detail file(s)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Order

`L1 -> L2 -> L3 -> L5 -> L6 -> L4 -> L7 -> L8`

Why this order:

- `L1-L3` tighten the highest-value production seams first.
- `L5-L6` improve operator visibility and then resolve the remaining dormant learning scaffolding.
- `L7-L8` finish the doc-honesty work after the runtime contract is clear.

---

## Batch Overview

| Batch | Purpose | Primary Scope | Verify Focus | Time Box |
|------|---------|---------------|--------------|----------|
| L1 | Use the shipped playbook-rule and skill surfaces in a smaller learned-context path | `roko-cli`, `roko-learn`, `roko-compose` | `cargo test -p roko-cli -p roko-learn -p roko-compose` | 45-90 min |
| L2 | Expose slice-aware regressions and the existing iteration threshold | `roko-learn`, `roko-cli` | `cargo test -p roko-learn -p roko-cli` | 45-90 min |
| L3 | Canonicalize the shipped prediction and routing-log calibration path | `roko-learn`, `roko-core`, `roko-cli` | `cargo test -p roko-learn -p roko-core -p roko-cli` | 60-90 min |
| L4 | Decide whether subscriber and drift modules are live or explicitly demoted | `roko-learn`, startup wiring if needed | `cargo test -p roko-learn -p roko-cli` | 45-90 min |
| L5 | Make existing cost pressure visible in routing decisions without redesigning the router | `roko-learn`, `roko-cli` | `cargo test -p roko-learn -p roko-cli` | 45-90 min |
| L6 | Persist experiment outcomes in an operator-visible artifact | `roko-learn`, `roko-cli` | `cargo test -p roko-learn -p roko-cli` | 45-90 min |
| L7 | Align episode, pattern, and tier-progression docs with shipped behavior | docs only | `rg -n "EpisodeLogger|compact|PatternMiner|pattern_discovery|TierProgression|HeuristicRule" crates/roko-learn crates/roko-neuro docs/05-learning tmp/docs-parity/05` | 30-60 min |
| L8 | Demote demurrage, worldviews, and framework-heavy vision content to future work | docs only | `rg -n "demurrage|worldview|replication-ledger|FEP|Friston|Viable System Model|constitutional" docs/05-learning tmp/docs-parity/05 tmp/refinements-audit` | 30-60 min |

---

## Batch Details

### L1 — Learned-Context Activation

**Owns**: the real runtime gap in knowledge-tier docs

**Goal**: populate `MatchContext` with files, tags, category, and error signature where already available.

**Scope**:

1. keep the current prompt-injection path,
2. activate richer rule matching,
3. only widen skill retrieval if `SkillQuery::select` is a small, deterministic improvement.

**Out of scope**:

- new heuristic ontologies,
- demurrage,
- worldview clustering,
- redesigning prompt composition.

**Verify**:

```bash
cargo test -p roko-cli -p roko-learn -p roko-compose
rg -n "build_learned_context|MatchContext|search_by_tag|SkillQuery" crates/roko-cli crates/roko-learn crates/roko-compose
```

### L2 — Slice-Aware Regression Output

**Owns**: the highest-value metrics gap

**Goal**: make regression alerts match the slice-aware docs that already describe them.

**Scope**:

1. activate `iterations_increase`,
2. iterate `Baseline.slices`,
3. preserve the existing overall alerts.

**Out of scope**:

- advanced drift detectors,
- rollback automation,
- dashboards.

**Verify**:

```bash
cargo test -p roko-learn -p roko-cli
rg -n "detect_regressions|iterations_increase|slice: None|slice:" crates/roko-learn crates/roko-cli
```

### L3 — Predictive Calibration Canonicalization

**Owns**: predictive-calibration truth in advertising

**Goal**: make the routing-log replay path the explicit source of truth unless the direct `PredictionRecord` path is deliberately wired.

**Scope**:

1. pick one canonical path,
2. expose one real summary metric on that path,
3. align prompt/scorer consumers with it.

**Out of scope**:

- full predictive-foraging engine,
- Brier/reliability/foraging research unless it is genuinely small,
- cross-operator bus architecture.

**Verify**:

```bash
cargo test -p roko-learn -p roko-core -p roko-cli
rg -n "PredictionRecord|CalibrationTracker|PredictiveScorer|routing_log|adjust_prediction" crates/roko-learn crates/roko-core crates/roko-cli
```

### L4 — Drift / Subscriber Decision

**Owns**: dormant learning scaffolding

**Goal**: stop leaving `DriftDetector` and `run_learning_subscriber` in an ambiguous state.

**Scope**:

1. wire one real caller, or
2. explicitly demote the modules in docs/comments and leave them dormant on purpose.

**Out of scope**:

- new event-bus design,
- broad anomaly-system redesign,
- new calibration theory.

**Verify**:

```bash
cargo test -p roko-learn -p roko-cli
rg -n "run_learning_subscriber|DriftDetector" crates/roko-learn crates/roko-cli docs/05-learning tmp/docs-parity/05
```

### L5 — Operator-Facing Budget Visibility

**Owns**: the remaining practical loop cleanup

**Goal**: make cost pressure and experiment winners easier to inspect without building a new routing theory.

**Scope**:

1. expose budget action outcomes clearly,
2. make experiment winner materialization discoverable,
3. keep the current router stages intact.

**Out of scope**:

- cost-spectrum routing,
- router ensembles,
- new experiment frameworks.

**Verify**:

```bash
cargo test -p roko-learn -p roko-cli
rg -n "BudgetGuardrail|BudgetAction|on_experiment_concluded|ExperimentStore" crates/roko-learn crates/roko-cli
```

### L6 — Operator-Visible Experiment Outcomes

**Owns**: experiment materialization

**Goal**: keep the current prompt-experiment path, but persist the winning state somewhere a later agent or operator can inspect without reading router internals.

**Scope**:

1. keep the existing experiment store,
2. surface winner state in a durable artifact,
3. avoid redesigning the experiment system.

**Out of scope**:

- new experiment frameworks,
- router-family changes,
- theory-heavy prompt-optimization doctrine.

**Verify**:

```bash
cargo test -p roko-learn -p roko-cli
rg -n "ExperimentStore|prompt_experiment|model_experiment|cascade_router" crates/roko-learn crates/roko-cli
```

### L7 — Episodes / Patterns Docs Honesty

**Owns**: doc 00 and doc 05 parity cleanup

**Goal**: document the shipped episode and pattern story without implying tiered storage or DBSCAN already exist.

**Scope**:

1. align the `Episode` schema text,
2. make `EpisodeLogger::compact` the shipping retention story,
3. make `PatternMiner` / cross-episode consolidation / k-medoids the present-tense pattern story.

### L8 — Framework Demotion And Future Work

**Owns**: the overscoped theory sections

**Goal**: preserve the framework-mapping value while moving research claims into explicit future work.

**Scope**:

1. keep framework-to-code mappings that are genuinely helpful,
2. label worldview, replication-ledger, constitutional, demurrage, ADAS, EvoSkills, and scaling claims as deferred,
3. keep c-factor as a measurement surface, not an optimization doctrine.

---

## Carry-Forward Rule

If a batch discovers work in any of these buckets, defer it immediately:

- `Engram` HDC fingerprint bridge,
- new router families,
- new memory-economy models,
- worldview / replication-ledger systems,
- governance / constitutional layers,
- scaling-law or autocatalytic proofs.

Batch `05` should leave behind a tighter runtime contract, not a new research roadmap.
