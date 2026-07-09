# 05-Learning Parity Analysis

Gap analysis of `docs/05-learning/` vs the actual Roko learning stack: `crates/roko-learn/`, the orchestrator callsites in `crates/roko-cli/src/orchestrate.rs`, and the composition / prompt consumers that already read learning outputs.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **runtime learning-loop hardening + contract cleanup**, not as a license to implement every advanced routing, analytics, or self-improvement idea from the later learning docs.

- Prefer making the current production learning loops use the richer library surfaces they already have.
- Prefer one runtime seam per batch: learned-context matching, regression slices, predictive calibration, routing pressure, experiment materialization, dead-module resolution.
- If a task starts requiring new research-grade routing algorithms, storage architecture, or governance frameworks, record the seam and defer it.
- For overnight runs, every batch should be able to stop with a clear `PASS`, `FAIL`, or `BLOCKED` result and leave behind evidence: files changed, commands run, outputs, and explicit deferrals.

Recommended single-agent serial order inside batch `05`:

`L1 -> L2 -> L3 -> L5 -> L6 -> L4 -> L7 -> L8`

Reasoning:

- `L1` activates the richest low-risk learned-context seam first.
- `L2` fixes the biggest observability blind spot in the learning pipeline.
- `L3` makes the predictive-calibration contract explicit before more routing changes pile on.
- `L5` and `L6` close the remaining partial feedback loops that already have real runtime artifacts.
- `L4` resolves dead subscriber / drift scaffolding once the core feedback contracts are clearer.
- `L7` and `L8` are mostly truth-in-advertising and handoff cleanup after the runtime seams are settled.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-episodes-patterns.md](A-episodes-patterns.md) | 00, 05 (episodes + patterns) | A.01-A.14 | 9 DONE / 2 PARTIAL / 3 NOT DONE |
| [B-knowledge-tiers.md](B-knowledge-tiers.md) | 01, 02 (playbooks + skills) | B.01-B.13 | 9 DONE / 3 PARTIAL / 1 NOT DONE |
| [C-routing-bandits.md](C-routing-bandits.md) | 03, 04, 10, 11 (bandits + routing) | C.01-C.20 | 13 DONE / 0 PARTIAL / 7 NOT DONE |
| [D-metrics-cost-health.md](D-metrics-cost-health.md) | 06, 07, 08, 09 (metrics + regression + cost + health) | D.01-D.18 | 11 DONE / 5 PARTIAL / 2 NOT DONE |
| [E-feedback-calibration.md](E-feedback-calibration.md) | 13, 14, 15, 16 (feedback loops + stability + calibration) | E.01-E.19 | 11 DONE / 6 PARTIAL / 2 NOT DONE |
| [F-frameworks-vision.md](F-frameworks-vision.md) | 12, 17 (framework mappings + vision) | F.01-F.12 | 7 DONE / 0 PARTIAL / 5 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 60/96 items DONE (63%)

The learning stack is in a different state from the earlier parity batches:

- the **core runtime feedback hub is real and already widely used**,
- several subsystems are **present in both library code and production callers**,
- but some of the most useful matching / regression / calibration surfaces are **narrower in production than the library supports**,
- and some later docs still describe prescriptive systems as if they already ship.

### Tier 1 — Should Exist Now (self-hosting relevant)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.08 | `build_learned_context()` populates only `role`; file/tag/category/error-signature rule triggers never fire | PARTIAL | HIGH |
| D.08 | `detect_regressions()` never iterates `baseline.slices`; per-slice analysis is unreachable | NOT DONE | HIGH |
| D.07 | `iterations_increase` threshold is declared but never checked | PARTIAL | HIGH |
| E.16 | predictive calibration has a split contract: routing-log replay is real, direct `register/resolve` is unused | PARTIAL | HIGH |
| E.17 | Brier / reliability / arithmetic-corrector pieces from doc 16 are absent | NOT DONE | HIGH |
| F.08 | improvement-safety prescriptive surface is absent despite strong doc framing | NOT DONE | HIGH |

### Tier 2 — Should Exist Soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.12 | main learned skill retrieval path still uses `search_by_tag`, not richer `SkillQuery` matching | PARTIAL | MEDIUM |
| A.07 | doc 00 still implies tiered storage / clustering are active while the real retention story is `EpisodeLogger::compact` | NOT DONE | MEDIUM |
| B.10 | `prune_stale(days)` contradicts the docs' monotonic-growth claim | PARTIAL | MEDIUM |
| E.07 | `BudgetGuardrail` acts as pre-dispatch override, not as graded router input | PARTIAL | MEDIUM |
| E.09 | experiment winners update router state, but no durable operator-facing materialization path exists | PARTIAL | MEDIUM |
| E.18 | `DriftDetector` and `run_learning_subscriber` remain ambiguous dead scaffolding | NOT DONE | MEDIUM |
| F.07 | improvement-measurement scorecard/significance types are still absent while docs imply more | NOT DONE | MEDIUM |

### Tier 3 — Future / Design-Heavy

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.09 | DBSCAN-based episode clustering | NOT DONE | LOW |
| C.08-C.10 | contextual Thompson / NeuralUCB / bandit ensembles | NOT DONE | LOW |
| C.14-C.16 | lookahead routing / cost-spectrum routing / router calibration | NOT DONE | LOW |
| C.18 | 4-dimensional Pareto routing | NOT DONE | LOW |
| D.09 | advanced drift detectors | NOT DONE | LOW |
| F.09-F.12 | ADAS, EvoSkills, autocatalytic framing beyond current C-Factor oracle | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.03 | crash-safe append-only episode logging | DONE |
| A.10-A.14 | trigram mining, `EpisodeView`, consolidator, k-medoids, frequency separation | DONE |
| B.01-B.07 | playbooks, rules, persistence, confidence dynamics | DONE |
| B.09 | rich `Skill` schema and library persistence | DONE |
| C.01-C.07 | UCB, Track-and-Stop, LinUCB, Thompson, cascade routing core | DONE |
| C.17, C.19, C.20 | 2D Pareto, learning-rate schedule, active-inference tier selector | DONE |
| D.01-D.06, D.10-D.18 | metrics, baselines, cost tables, health, latency, anomaly wiring | DONE / PARTIAL where noted |
| E.01-E.15, E.19 | runtime feedback hub, 8-loop foundations, stability and C-Factor surfaces, local reward | DONE / PARTIAL where noted |
| F.01-F.06, F.10 | framework mappings plus C-Factor falsification-oracle framing | DONE |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside batch `05`:

| Item | Better Home | Why |
|------|-------------|-----|
| tiered episode storage, cold-tier compression, HDC-superposition archives | later storage / analytics hardening pass | not required to make current learning loops honest |
| DBSCAN clustering and advanced mining pipelines | later analytics pass | current k-medoids + trigram path already ships |
| advanced routing research (`NeuralUCB`, ensembles, lookahead, calibration stacks) | post-parity routing research pass | batch `05` should focus on the current shipped routers |
| full predictive-foraging engine beyond calibration summaries | later routing / forecasting pass | current need is one honest calibration contract |
| scorecards, constitutional safety, significance-test governance systems | later eval / governance pass | these are prescriptive layers, not current runtime dependencies |
| ADAS / evolutionary skill optimization | post-parity research pass | not needed to harden current learning runtime |

Batch `05` should usually produce:

- richer production use of already-shipped learning signals,
- clearer and more actionable regression / calibration outputs,
- fewer ambiguous dead learning subsystems,
- and explicit handoffs for the research or governance-heavy designs.

---

## Critical Learning Issues

1. **The learned-context production path is narrower than the rule and skill systems it sits on top of.** Most non-role matching signals are dropped before selection.
2. **Regression detection is currently overall-only despite slice-aware baselines already existing.** The headline per-slice analysis story is not real in production.
3. **Predictive calibration has a split reality.** Routing-log replay and prompt/scoring consumers are real, but doc 16's direct prediction-record pipeline and calibration metrics are not.
4. **Some partial loops still end in internal state rather than operator-visible artifacts.** Cost pressure and experiment winners are the clearest examples.
5. **A few sizable learning modules are still ambiguous dead code.** That makes overnight-agent work riskier because later agents cannot tell which scaffolding is meant to be live.

---

## Key Insight

Batch `05` does **not** primarily need more learning subsystems.

It needs a tighter contract between:

- the **rich learning library surfaces**,
- the **thinner production callsites that currently use them**,
- and the **docs that sometimes describe planned governance / calibration systems as if they already ship**.

That means the highest-value work here is usually:

1. make production selection paths use more of the metadata they already have,
2. make slice-aware and calibration-aware outputs real and inspectable,
3. resolve dead or ambiguous learning scaffolding,
4. keep research-heavy routing and governance work explicitly deferred.

---

## Batch 05 Success Definition

Batch `05` is successful when:

- learned-context matching is meaningfully richer than role-only lookup,
- regression outputs can distinguish overall regressions from slice-specific ones,
- predictive calibration has one obvious source of truth and at least one real metric surface,
- partial loops around cost and experiments leave clearer runtime artifacts,
- and the design-heavy safety / ADAS / advanced-routing sections are cleanly deferred instead of implied to be live.
