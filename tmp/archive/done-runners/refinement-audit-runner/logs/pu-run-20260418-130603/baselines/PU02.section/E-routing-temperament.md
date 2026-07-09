# E — Routing, Temperament, Harness (Docs 08, 10, 11)

Parity analysis of `docs/02-agents/08-harness-engineering.md`, `10-temperament-profiling.md`, `11-dual-process-routing.md` vs actual codebase.

---

## E.01 — ModelTier Enum (Doc 11, §Three Model Tiers)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Three-variant enum `ModelTier { Fast, Standard, Premium }` mapping to Haiku-class, Sonnet-class, and Opus-class models respectively. Each `AgentRole` has a default tier.

### What exists
`ModelTier` is defined at `crates/roko-core/src/agent.rs:445` with exactly the three variants: `Fast`, `Standard`, `Premium`. Derived `Serialize`/`Deserialize`/`Hash`/`Eq`.

`AgentRole::model_tier()` maps each role to a default tier at `crates/roko-core/src/agent.rs:790-824`:
- Fast: `Conductor`, `Watcher`, `RegressionDetector`, etc.
- Standard: `Implementer`, `Tester`, `Reviewer`, `ErrorDiagnoser`, etc.
- Premium: `Architect`, `FullLoopValidator`, etc.

Unit tests confirm at `crates/roko-core/src/agent.rs:1087-1089`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps. Fully matches spec.

### Verify
```bash
grep -n 'pub enum ModelTier' crates/roko-core/src/agent.rs
```

---

## E.02 — CascadeRouter Three-Stage Cascade (Doc 11, §CascadeRouter)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Multi-stage confidence cascade: Task arrives -> try Fast (System 1) -> if confidence >= threshold, accept; else try Standard; else try Premium (System 2). Three stages: Static (< 50 obs), Confidence (50-200 obs), UCB (> 200 obs). The router persists state to `.roko/learn/cascade-router.json`.

### What exists
`CascadeRouter` is a substantial implementation at `crates/roko-learn/src/cascade_router.rs:994`. The struct wraps a `LinUCBRouter`, `confidence_stats` (`Mutex<HashMap<String, ModelStats>>`), a `pareto_frontier`, `role_table`, `model_slugs`, and `stage_tracking`.

Three stages are defined by `CascadeStage` enum at line 63: `Static`, `Confidence`, `Ucb`. Stage selection is observation-count driven (line 1091+).

Route methods exist at lines 1391-1647: `route()`, `route_logged()`, `route_with_experiments()`, `route_with_health()`, `route_with_cfactor()`, `route_with_cfactor_among()`.

`save()` at line 2257 persists to JSON. `load_or_new()` at line 2315 restores from JSON. Persists confidence stats, model slugs, total observation count, and stage transitions.

Called from `orchestrate.rs` at lines 9686-9835 (adaptive model selection via CascadeRouter) and from `main.rs:1727` (`CascadeRouter::load_or_new`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps. The doc describes the architecture and the code implements it faithfully, including persistence and three-stage transitions.

### Verify
```bash
grep -n 'pub struct CascadeRouter' crates/roko-learn/src/cascade_router.rs
grep -n 'CascadeStage' crates/roko-learn/src/cascade_router.rs | head -5
grep -n 'cascade_router' crates/roko-cli/src/orchestrate.rs | head -5
```

---

## E.03 — LinUCB Contextual Bandit (Doc 11, §LinUCB Bandit)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
LinUCB contextual bandit (Li et al., 2010) for model selection. Context vector per task (type, complexity, history, role, budget). Each model is an "arm". UCB computed as `theta_a . x + alpha * sqrt(x' A_a^-1 x)`. Updates weight matrix after each observation. Exploration parameter controlled by temperament.

### What exists
`LinUCBRouter` at `crates/roko-learn/src/model_router.rs:655`. Implements full LinUCB: arms with A-matrix inverse, b-vectors, and theta estimates. Context dimension is constant (`CONTEXT_DIM`). Key methods:
- `select_features()` for arm selection with UCB scoring
- `update_features()` / `update_features_multi_objective()` for A/b matrix updates
- `current_alpha()` at line 714 with exponential decay: `alpha = ALPHA_MIN + (ALPHA_MAX - ALPHA_MIN) * exp(-obs/TAU)`
- Full matrix operations at lines 513-522 (`mat_vec_mul`, `dot`)

`CascadeRouter` wraps `LinUCBRouter` (line 996) and uses it for Stage 3 (UCB) routing. All observations are forwarded to LinUCB regardless of stage (line 1951-1952) so it is warm when stage transitions happen.

Persistence via `with_persist_path()` / `load()` / `save()` on LinUCBRouter itself.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.03a | Exploration parameter not controlled by temperament config | `crates/roko-learn/src/model_router.rs:714` | Low |

The exploration alpha decays purely as a function of observation count, not temperament. Doc 10 says "Exploratory temperament sets a high exploration parameter." This is not wired.

### Verify
```bash
grep -n 'pub struct LinUCBRouter' crates/roko-learn/src/model_router.rs
grep -n 'current_alpha' crates/roko-learn/src/model_router.rs
```

---

## E.04 — Pareto Frontier Pruning (Doc 11, §Pareto frontier pruning)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Before bandit selection, a Pareto frontier computation prunes dominated models (worse on both quality and cost dimensions). Implementation plans 2G.10 and 2G.11.

### What exists
Dedicated module at `crates/roko-learn/src/pareto.rs`. `ModelObservation` struct holds `pass_rate`, `cost_per_success`, `avg_latency_ms`, `observations` (lines 12-21). `compute_pareto_frontier()` at line 28 implements dominance checking.

In `CascadeRouter`:
- `ParetoFrontierState` cached at line 1000-1016
- `refresh_pareto_frontier_if_needed()` at line 2738 triggers every 50 observations
- `recompute_pareto_frontier()` at line 2836 builds observations from confidence stats using `pareto_cost_proxy()` and `pareto_latency_proxy()`
- `pareto_adjusted_alpha()` at line 2877 reduces exploration for dominated models (non-frontier gets 10% alpha)
- UCB route path at line 2094 applies `pareto_adjusted_alpha` per candidate

Pareto status exposed in route explanations: `on_pareto_frontier` field in `CascadeRouteCandidate` (line 168-169) and in `CascadeRouteExplanation` (line 189-190).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps. Fully implemented and wired into the UCB routing path.

### Verify
```bash
grep -n 'compute_pareto_frontier' crates/roko-learn/src/pareto.rs
grep -n 'pareto_adjusted_alpha' crates/roko-learn/src/cascade_router.rs
```

---

## E.05 — Thompson Sampling (Doc 11, §Thompson Sampling)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
For confidence-threshold decisions (escalate or accept?), CascadeRouter uses Thompson sampling: sample theta from Beta(successes, failures) and multiply by raw confidence. Introduces beneficial randomness.

### What exists
`ThompsonArm` at `crates/roko-learn/src/model_router.rs:449`. Beta posterior with discount:
- `alpha` / `beta` fields (lines 453-455)
- `sample()` at line 483 draws from Beta distribution via `sample_beta()` (line 524)
- `update()` at line 489 applies discount before increment, accumulates `sum_reward`

Thompson sampling is configurable via `RoutingAlgorithm::Thompson` in config schema (`crates/roko-core/src/config/schema.rs:1640`). Default discount factor is 0.99 (line 1791-1793).

Used in `ConductorPolicy` (`crates/roko-learn/src/conductor.rs:108-129`) where each `ConductorAction` has a `ThompsonArm`. Blended with context features at line 294.

The `RoutingConfig` supports switching between `LinUcb` and `Thompson` algorithm at `crates/roko-core/src/config/schema.rs:1636-1658`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.05a | Thompson not used as confidence-threshold decision in CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | Low |

The doc describes Thompson sampling for the _escalate-or-accept_ decision within the cascade. The actual CascadeRouter uses observation-count-based stage transitions (Static/Confidence/UCB), not Thompson-modulated confidence thresholds. Thompson is available for the conductor policy and as an alternate routing algorithm, but the cascade's internal confidence-to-escalation logic does not sample from Beta posteriors as the doc describes.

### Verify
```bash
grep -n 'ThompsonArm' crates/roko-learn/src/model_router.rs | head -5
grep -n 'RoutingAlgorithm' crates/roko-core/src/config/schema.rs
```

---

## E.06 — Anomaly Detection (Doc 11, §Anomaly Detection)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`AnomalyDetector` monitors model performance: sudden quality drops, latency spikes, cost anomalies. When detected, router de-prioritizes affected model and fires alert.

### What exists
Two `AnomalyDetector` implementations exist:

1. **roko-learn** at `crates/roko-learn/src/anomaly.rs:20`. Session-local detector with four check methods:
   - `check_prompt()` (line 52): repeated prompt loop detection (5 occurrences in 20-message window)
   - `check_cost()` (line 77): EWMA-based cost spike detection (z-score > 3.0)
   - `check_quality()` (line 95): sustained quality degradation (recent 5 vs earlier 10 scores)
   - `check_budget()` (line 122): budget exhaustion

   Four `Anomaly` variants: `PromptLoop`, `CostSpike`, `QualityDegradation`, `BudgetExhausted` (line 206-229).

2. **roko-agent** at `crates/roko-agent/src/task_runner.rs:149`. Simpler version for per-task detection within the agent task runner.

Wired into `orchestrate.rs`:
- Imported at line 70
- Field on runner at line 2194
- `drain_turn_learning_events()` calls `anomaly_detector.check_prompt()` and `check_cost()` at lines 582-609
- `detect_cost_anomaly_override()` at line 10834

Also wired into `event_subscriber.rs` (line 53), `dispatch.rs` (line 193).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.06a | No latency spike detection | `crates/roko-learn/src/anomaly.rs` | Low |

The doc mentions detecting "Response times exceed 2x the rolling average." The current detector checks prompt loops, cost spikes, quality degradation, and budget exhaustion, but does not have a latency spike check. Latency tracking exists separately in `roko-learn/src/latency.rs` but is not integrated into the anomaly detector.

### Verify
```bash
grep -n 'pub fn check_' crates/roko-learn/src/anomaly.rs
grep -n 'anomaly_detector' crates/roko-cli/src/orchestrate.rs | head -5
```

---

## E.07 — Three Cognitive Speeds / Operating Frequencies (Doc 11, §Three Cognitive Speeds)

- **Status**: DONE (with naming difference)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Three speeds: Gamma (~5-15s, fast reflexive), Theta (~75s, standard deliberation), Delta (hours, deep reasoning). CascadeRouter uses these as priors for tier selection.

### What exists
`OperatingFrequency` at `crates/roko-core/src/operating_frequency.rs:16` with three variants: `Gamma` (reactive ~10s), `Theta` (strategic ~2-5min), `Delta` (consolidation ~30min+).

Each frequency maps to:
- `inference_tier()` (lines 34-39): Gamma -> T0, Theta -> T1, Delta -> T2
- `turn_limit()` (lines 48-53): Gamma -> 0, Theta -> 20, Delta -> 50
- Task-based selection logic at line 56+

CascadeRouter integrates via `select_for_frequency()` at `crates/roko-learn/src/cascade_router.rs:1160`:
- Gamma -> `None` (no LLM turn)
- Theta -> cascade router selection
- Delta -> strongest available model

Called from `orchestrate.rs` at lines 709-711 and 9616+.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No significant gaps. The doc's description of three speeds matches implementation. Naming convention is slightly different (doc says "cognitive speeds", code says "operating frequencies") but the semantics align.

### Verify
```bash
grep -n 'pub enum OperatingFrequency' crates/roko-core/src/operating_frequency.rs
grep -n 'select_for_frequency' crates/roko-learn/src/cascade_router.rs | head -3
```

---

## E.08 — Active Inference Tier Selection (Doc 11, §Active Inference Connection; Doc 10, §Temperament in Active Inference)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Model routing grounded in the Free Energy Principle. Expected free energy minimization for tier selection. Epistemic value (exploration) vs pragmatic value (exploitation). Confidence threshold acts as precision parameter.

### What exists
`crates/roko-learn/src/active_inference.rs` implements this directly:
- `BeliefState` at line 17: 90-state factorized distribution (3 difficulty x 3 skill x 10 confidence)
- `select_tier()` at line 83: selects tier minimizing expected free energy
- `expected_free_energy()` at line 99: computes risk + 0.20*ambiguity + 0.10*evidence
- `observe()` at line 48: Bayesian update after observing outcome with cost/latency penalties

CascadeRouter exposes `select_tier_with_active_inference()` at `crates/roko-learn/src/cascade_router.rs:1213-1218`, delegating to the active inference module.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.08a | Active inference tier selection not called from orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` | Medium |

The `select_tier_with_active_inference()` method exists on CascadeRouter but `orchestrate.rs` uses the standard `select_for_frequency_among()` path (line 9835). The active inference belief state is not maintained or updated during plan execution. It exists as an alternative path but is not integrated into the main loop.

### Verify
```bash
grep -n 'active_inference' crates/roko-learn/src/cascade_router.rs
grep -n 'active_inference\|BeliefState' crates/roko-cli/src/orchestrate.rs
```

---

## E.09 — Router Persistence (Doc 11, §Persistence)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
CascadeRouter persists state to `.roko/learn/cascade-router.json`. Routing decisions improve across sessions.

### What exists
- `CascadeRouter::save()` at `crates/roko-learn/src/cascade_router.rs:2257` writes a `CascadeSnapshot` (model slugs, role table, confidence stats, total observations, stage transitions) as pretty-printed JSON with atomic write (tmp + rename).
- `CascadeRouter::load_or_new()` at line 2315 restores from JSON. Handles model version changes via `detect_version_changes()` and `migrated_confidence_stats()` which seed half the old stats into new slug names.
- `cascade_router_path()` at `crates/roko-cli/src/main.rs:2120-2123` returns `.roko/learn/cascade-router.json`.
- `RokoLayout::cascade_router_path()` at `crates/roko-fs/src/layout.rs:179` provides canonical path.
- `orchestrate.rs` saves at line 3698: `self.learning.save_cascade_router()`.
- Dashboard reads it at `crates/roko-cli/src/tui/dashboard.rs:574`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps. Persistence fully wired with version migration support.

### Verify
```bash
grep -n 'cascade-router.json' crates/roko-fs/src/layout.rs
grep -n 'save_cascade_router' crates/roko-cli/src/orchestrate.rs
```

---

## E.10 — Temperament Enum and Config (Doc 10, §The Temperament Concept, §Configuration)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~120
- **Dependencies**: None
- **Files to modify**: `crates/roko-core/src/config/schema.rs`, `crates/roko-agent/src/introspection.rs`

### What the doc says
Four temperaments: `Conservative`, `Balanced`, `Aggressive`, `Exploratory`. Configured via `roko.toml` under `[agent]` with `temperament = "balanced"`. Per-role overrides supported under `[agent.roles.implementer]`. Controls model parameters (temperature, top_p), tool selection, gate strictness, review depth, and model routing.

### What exists
**AgentIdentity** at `crates/roko-agent/src/introspection.rs:12-21` has a `temperament: String` field. It is a free-form string, not an enum. Constructed via `AgentIdentity::new(role, temperament)` at line 26 where temperament is `impl Into<String>`.

**AgentConfig** at `crates/roko-core/src/config/schema.rs:1256-1293` has **no** `temperament` field. The struct contains: `default_model`, `default_backend`, `default_effort`, `context_limit_k`, `bare_mode`, `command`, `args`, `timeout_ms`, `env`, `tier_models`, `fallback_model`, `roles`.

**RoleOverride** (referenced at line 1292 via `roles: HashMap<String, RoleOverride>`) also has no `temperament` field.

The temperament string is used in `MorphableAgent` at `crates/roko-agent/src/metamorphosis.rs:171` to tag output signals with `"temperament"`, but this is purely a metadata annotation - it does not affect any runtime behavior.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.10a | No `Temperament` enum in config | `crates/roko-core/src/config/schema.rs` | High |
| E.10b | No `temperament` field on `AgentConfig` | `crates/roko-core/src/config/schema.rs:1256` | High |
| E.10c | No per-role temperament override | `crates/roko-core/src/config/schema.rs` (RoleOverride) | Medium |
| E.10d | Temperament is a free-form string in AgentIdentity, not a validated enum | `crates/roko-agent/src/introspection.rs:18` | Medium |

### Verify
```bash
grep -n 'temperament' crates/roko-core/src/config/schema.rs
# Expected: no results
grep -n 'temperament' crates/roko-agent/src/introspection.rs
# Expected: free-form String field only
```

---

## E.11 — Temperament Propagation to Model Parameters (Doc 10, §What Temperament Controls — Model Parameters)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~80
- **Dependencies**: E.10
- **Files to modify**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-agent/src/`

### What the doc says
Temperament controls `temperature` (0.1-1.0), `top_p` (0.9-1.0), and `max_tokens` multiplier (1.0x-2.0x) depending on the temperament variant (Conservative through Exploratory).

### What exists
There are no `temperature`, `top_p`, or `top_k` fields anywhere in `crates/roko-core/src/config/`. The `AgentConfig` (schema.rs:1256) has `default_effort` ("low"/"medium"/"high"/"max") and `context_limit_k`, but no model sampling parameters.

When agents are spawned via Claude CLI in `orchestrate.rs`, model parameters are not passed. The `RenderedTools` at `crates/roko-agent/src/translate/mod.rs:131` handles tool format but not sampling parameters.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.11a | No temperature/top_p/max_tokens config fields | `crates/roko-core/src/config/schema.rs` | Medium |
| E.11b | No temperament-to-parameter mapping function | Nowhere | Medium |
| E.11c | Model parameters not passed to agent spawn | `crates/roko-cli/src/orchestrate.rs` | Medium |

### Verify
```bash
grep -rn 'temperature\|top_p\|top_k' crates/roko-core/src/config/ --include='*.rs'
# Expected: no results
```

---

## E.12 — Temperament Propagation to Gate Strictness (Doc 10, §What Temperament Controls — Gate Strictness)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~60
- **Dependencies**: E.10
- **Files to modify**: `crates/roko-gate/`, `crates/roko-cli/src/orchestrate.rs`

### What the doc says
Temperament sets per-gate behavior (Required/Warning/Skipped) for compile, test, clippy, diff-size, and review gates. Conservative requires all; Exploratory disables most.

### What exists
Gate pipeline is wired and called per-task from `orchestrate.rs`. Adaptive gate thresholds exist via EMA in `.roko/learn/gate-thresholds.json` (runtime_feedback.rs:110/134). However, gate strictness levels are determined by the pipeline configuration, not by temperament.

There is no code path that reads a temperament value and maps it to gate Required/Warning/Skipped behavior.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.12a | No temperament-to-gate-strictness mapping | Nowhere | Medium |
| E.12b | Gate pipeline does not read temperament config | `crates/roko-gate/` | Medium |

### Verify
```bash
grep -rn 'temperament' crates/roko-gate/ --include='*.rs'
# Expected: no results
```

---

## E.13 — Temperament Propagation to Tool Selection (Doc 10, §What Temperament Controls — Tool Selection)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~60
- **Dependencies**: E.10
- **Files to modify**: `crates/roko-agent/src/dispatcher/mod.rs`

### What the doc says
Temperament controls tool allowlists. Conservative restricts to read-only and blocks dangerous tools. Exploratory allows all tools including network and bash.

### What exists
`ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs:80` validates tool calls against JSON schema, checks permissions, and runs SafetyLayer policies. However, tool allowlists are controlled by role-based `ToolPermissions` (via `AgentRole::tool_permissions()`), not by temperament.

The `ToolDispatcher` is not called from `orchestrate.rs` at all (confirmed by grep returning no results). The primary execution path uses Claude CLI which has its own safety, bypassing Roko's `ToolDispatcher` pipeline. The doc itself notes this as gap #1 in its "Where gaps remain" section.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.13a | No temperament-to-tool-allowlist mapping | Nowhere | Medium |
| E.13b | ToolDispatcher not called from orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` | High |

### Verify
```bash
grep -n 'ToolDispatcher' crates/roko-cli/src/orchestrate.rs
# Expected: no results
grep -n 'ToolDispatcher' crates/roko-cli/src/run.rs
# Expected: used only in `roko run`, not in `plan run`
```

---

## E.14 — Temperament in CascadeRouter (Doc 10, §Temperament in the CascadeRouter)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~40
- **Dependencies**: E.10, E.02
- **Files to modify**: `crates/roko-learn/src/cascade_router.rs`

### What the doc says
CascadeRouter uses temperament to set: confidence threshold (Conservative: 0.9, Balanced: 0.7), UCB exploration parameter (Exploratory: high exploration), and cost weight in Pareto frontier (Aggressive: lower cost weight).

### What exists
CascadeRouter has no temperament field and accepts no temperament parameter in any of its constructors or route methods. The exploration alpha is purely observation-count-driven (`alpha_for_observations()` in model_router.rs). Confidence thresholds are not adjustable by temperament. Pareto cost weights use hardcoded proxies (`pareto_cost_proxy()` at cascade_router.rs:2885).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.14a | CascadeRouter has no temperament input | `crates/roko-learn/src/cascade_router.rs:994` | Medium |
| E.14b | Confidence threshold not adjustable by temperament | `crates/roko-learn/src/cascade_router.rs` | Medium |
| E.14c | Exploration parameter not temperament-modulated | `crates/roko-learn/src/model_router.rs:714` | Low |
| E.14d | Pareto cost weight not temperament-adjustable | `crates/roko-learn/src/cascade_router.rs:2877` | Low |

### Verify
```bash
grep -n 'temperament' crates/roko-learn/src/cascade_router.rs
# Expected: no results
```

---

## E.15 — Meta-Routing (Doc 11, §Meta-Routing: Routing the Router)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~200
- **Dependencies**: E.02
- **Files to modify**: New module in `crates/roko-learn/src/`

### What the doc says
`MetaRouter` that selects between `HeuristicRouter`, `CascadeRouter`, and `KnnRouter` based on task characteristics. `MetaRoutingPolicy` with confidence/observation/budget thresholds. Three-level hierarchy: heuristic for known patterns, kNN for warm data, learned router for exploration.

### What exists
None. No `MetaRouter`, `HeuristicRouter`, `KnnRouter`, or `MetaRoutingPolicy` structs exist anywhere in the codebase. The code only has `CascadeRouter` (which itself has a static-stage fallback for cold start, serving a similar purpose to a heuristic router within its own three-stage cascade).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.15a | No MetaRouter struct | Nowhere | Low |
| E.15b | No HeuristicRouter | Nowhere | Low |
| E.15c | No KnnRouter | Nowhere | Low |
| E.15d | No MetaRoutingPolicy | Nowhere | Low |

Note: This is low severity because the CascadeRouter's three-stage cascade (Static -> Confidence -> UCB) serves a similar role to meta-routing by using simple static routing when data is sparse and only engaging the full bandit when enough observations exist.

### Verify
```bash
grep -rn 'MetaRouter\|HeuristicRouter\|KnnRouter\|MetaRoutingPolicy' crates/ --include='*.rs'
# Expected: no results
```

---

## E.16 — CollapseAvoidance / Anti-Monoculture (Doc 11, §Avoiding Expert Collapse)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~80
- **Dependencies**: E.02
- **Files to modify**: `crates/roko-learn/src/cascade_router.rs`

### What the doc says
`CollapseAvoidance` struct with: `min_exploration_rate` (5%), `geometric_forgetting` (0.95), `max_consecutive_same_model` (20), `diversity_bonus_per_100` (0.1).

### What exists
No `CollapseAvoidance` struct exists. However, some anti-collapse mechanisms are present implicitly:
- LinUCB's UCB exploration bonus naturally explores underused models
- Thompson sampling's Beta posterior provides random exploration when used
- Pareto frontier pruning already reduces exploration of dominated models (but this is the opposite of anti-collapse for non-dominated models)

The `ThompsonArm` has a `discount` factor (model_router.rs:463, default 0.95 via `THOMPSON_DEFAULT_DISCOUNT`) which provides geometric forgetting, but it is per-arm, not a global collapse avoidance mechanism.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.16a | No `CollapseAvoidance` struct | Nowhere | Low |
| E.16b | No `min_exploration_rate` enforcement | `crates/roko-learn/src/cascade_router.rs` | Low |
| E.16c | No `max_consecutive_same_model` check | `crates/roko-learn/src/cascade_router.rs` | Low |
| E.16d | No diversity bonus | `crates/roko-learn/src/cascade_router.rs` | Low |

### Verify
```bash
grep -rn 'CollapseAvoidance\|min_exploration_rate\|diversity_bonus' crates/ --include='*.rs'
# Expected: no results
```

---

## E.17 — EMA-Based Routing Statistics (Doc 11, §Exponential Moving Average Adaptation)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: ~40
- **Dependencies**: None
- **Files to modify**: None (for existing), `crates/roko-learn/src/cascade_router.rs` (for gaps)

### What the doc says
`EmaStats` struct with smoothing factor 0.05 and `ModelRunningStats` per model tracking: EMA of pass rate, latency, cost per task, token efficiency, and observation count.

### What exists
No `EmaStats` or `ModelRunningStats` structs matching this spec. However, EMA-based statistics exist in related components:
- `EwmaState` in `crates/roko-learn/src/anomaly.rs:152` (used for cost spike detection, alpha=0.2)
- `ModelStats` in `crates/roko-learn/src/cascade_router.rs` tracks `trials`/`successes`/`total_cost_usd` per model (simple counters, not EMA)
- Adaptive gate thresholds use EMA per rung (persisted to `.roko/learn/gate-thresholds.json`)

The CascadeRouter's confidence stage uses simple pass-rate counters (`successes/trials`), not EMA-smoothed running statistics.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.17a | No `EmaStats` per-model running stats in CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | Low |
| E.17b | Pass rate is raw ratio, not EMA-smoothed | `crates/roko-learn/src/cascade_router.rs` | Low |

### Verify
```bash
grep -rn 'EmaStats\|ModelRunningStats' crates/ --include='*.rs'
# Expected: no results
```

---

## E.18 — Six Harness Principles Mapping (Doc 08, all sections)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: varies per gap
- **Dependencies**: Multiple
- **Files to modify**: Multiple

### What the doc says
Six harness engineering principles from Meta-Harness (Lee et al., 2026) mapped to Roko:
1. Design tools for the model (`ToolDef`, `Translator`, `RenderedTools`)
2. Right context, not more context (`SystemPromptBuilder` 6-layer)
3. Validate before executing (`ToolDispatcher` 7-step pipeline)
4. Compress history intelligently (`prune` submodule)
5. Graduate autonomy based on confidence (roles + CascadeRouter)
6. Close the feedback loop (EpisodeLogger, efficiency events, CascadeRouter persistence, adaptive gates)

### What exists

**Principle 1 (Tools for the model)**: `ToolDef` exists in `roko-core::tool`. `RenderedTools` at `crates/roko-agent/src/translate/mod.rs:131` with three variants: `JsonArray`, `CliFlag`, `SystemPromptBlock`. Multiple translators exist (`ClaudeTranslator`, `OllamaTranslator`, `OpenAiTranslator`). DONE.

**Principle 2 (Right context)**: `SystemPromptBuilder` exists in `roko-compose` with 6-layer construction. DONE.

**Principle 3 (Validate before executing)**: `ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs:80` with schema validation, permission checks, and SafetyLayer. But NOT called from `orchestrate.rs` (the primary plan-execution path). Only used in `roko run` via `crates/roko-cli/src/run.rs:498`. PARTIAL.

**Principle 4 (Compress history)**: `prune_if_needed()` at `crates/roko-agent/src/tool_loop/prune.rs:38`. Byte-based heuristic (bytes/4 for token estimate). Preserves head 2 and tail 3 messages. DONE but basic.

**Principle 5 (Graduate autonomy)**: Role-based permissions via `AgentRole::tool_permissions()`. CascadeRouter tier escalation. DONE.

**Principle 6 (Feedback loop)**: EpisodeLogger wired (`orchestrate.rs:82`). Efficiency events to `.roko/learn/efficiency.jsonl`. CascadeRouter persistence. Adaptive gate thresholds. DONE.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.18a | ToolDispatcher (Principle 3) not called from plan execution path | `crates/roko-cli/src/orchestrate.rs` | High |
| E.18b | Context pruning (Principle 4) is byte-based, not semantic | `crates/roko-agent/src/tool_loop/prune.rs` | Low |
| E.18c | No gate-failure feedback to agent for retry (Principle 6) | `crates/roko-cli/src/orchestrate.rs` | Medium |

### Verify
```bash
grep -n 'ToolDispatcher' crates/roko-cli/src/orchestrate.rs
# Expected: no results (gap E.18a)
grep -n 'prune_if_needed' crates/roko-agent/src/tool_loop/prune.rs
```

---

## E.19 — Routing Config in roko.toml (Doc 11 implicit, Doc 10 §Configuration)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Routing configurable in `roko.toml` with algorithm selection, model tiers, reward weights.

### What exists
`RoutingConfig` at `crates/roko-core/src/config/schema.rs:1744` with:
- `mode` (default `"auto_override"`)
- `algorithm`: `RoutingAlgorithm::LinUcb` or `Thompson` (line 1636-1658)
- `discount_factor` (default 0.99, line 1752)
- `fast_task_model` (default `"claude-haiku-4-5"`, line 1756)
- `standard_task_model` (default `"claude-sonnet-4-6"`, line 1758)
- `complex_task_model` (default `"claude-opus-4-6"`, line 1760)
- `weights`: `RoutingRewardWeightsConfig` with per-tier overrides (line 1698-1739)
- `context_strategy` (line 1767)

TUI config metadata at `crates/roko-cli/src/tui/config_meta.rs:264` lists `"linucb"` and `"thompson"` as valid values.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps for the routing config itself. The missing piece is temperament integration (covered in E.10-E.14).

### Verify
```bash
grep -n 'pub struct RoutingConfig' crates/roko-core/src/config/schema.rs
grep -n 'RoutingAlgorithm' crates/roko-core/src/config/schema.rs | head -5
```

---

## Summary

| ID | Title | Status | Priority |
|----|-------|--------|----------|
| E.01 | ModelTier enum | DONE | -- |
| E.02 | CascadeRouter three-stage cascade | DONE | -- |
| E.03 | LinUCB contextual bandit | DONE | -- |
| E.04 | Pareto frontier pruning | DONE | -- |
| E.05 | Thompson sampling | DONE | -- |
| E.06 | Anomaly detection | DONE | -- |
| E.07 | Three cognitive speeds | DONE | -- |
| E.08 | Active inference tier selection | DONE | -- |
| E.09 | Router persistence | DONE | -- |
| E.10 | Temperament enum and config | NOT DONE | P2 |
| E.11 | Temperament -> model parameters | NOT DONE | P2 |
| E.12 | Temperament -> gate strictness | NOT DONE | P2 |
| E.13 | Temperament -> tool selection | NOT DONE | P2 |
| E.14 | Temperament in CascadeRouter | NOT DONE | P2 |
| E.15 | Meta-routing | NOT DONE | P3 |
| E.16 | CollapseAvoidance | NOT DONE | P3 |
| E.17 | EMA routing statistics | PARTIAL | P3 |
| E.18 | Six harness principles | PARTIAL | P1 |
| E.19 | Routing config in roko.toml | DONE | -- |

**Overall**: 10/19 items DONE, 2 PARTIAL, 7 NOT DONE.

The routing infrastructure (CascadeRouter, LinUCB, Pareto, Thompson, anomaly detection, persistence) is fully built and wired. The major gap is the **temperament system** -- it exists as a concept and a free-form string in `AgentIdentity`, but the enum, config schema, and all five propagation paths (model parameters, gate strictness, tool selection, review depth, CascadeRouter tuning) are unimplemented. The doc itself acknowledges this at §Implementation Status: "Not wired -- The runtime does not yet read the temperament field."

The harness engineering gap (E.18a, ToolDispatcher not called from plan execution) is the highest-severity single item because it means the validate-before-executing pipeline is bypassed for the primary orchestration path.

---

## Agent Execution Notes

### E.10 — Temperament Foundation

Do not start with behavior tweaks. Start with type ownership.

Recommended slice:

1. add a shared `Temperament` enum,
2. add config support with a clear default,
3. replace free-form string plumbing where practical,
4. keep behavior unchanged until the type is stable.

Acceptance criteria:

- `Temperament` is a real shared type,
- config can express it,
- `AgentIdentity` and agent creation surfaces no longer rely on an unstructured string.

### E.11 / E.13 / E.14 — Temperament Propagation

Only execute the smallest meaningful propagation set.

Good outcomes:

- one model-parameter effect,
- one tool-behavior effect,
- one router-behavior effect.

Do not widen into:

- full gate-threshold semantics,
- meta-routing,
- anti-monoculture systems.

### E.18 — Harness Principles

For batch `02`, Principle 3 is the key executable target:

- route a plan-execution path through `ToolDispatcher`,
- reuse the existing safety stack,
- make the runtime path visibly closer to the doc claim.

`E.18c` can stay deferred unless it falls out naturally from the chosen path.
