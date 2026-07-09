# Prompt: 05-learning

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/05-learning/`. This topic covers episodes, playbooks, skill library (Voyager), bandits (UCB1/Thompson/LinUCB), cascade routing, pattern discovery, task metrics, baselines, cost database, provider health, the 8 missing cybernetic feedback loops, stability mechanisms, collective calibration heuristic (31.6×), predictive foraging, ADAS meta-architecture search, autocatalytic thesis.

## Step 1 — Context pack (MANDATORY, in order)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §5 Cybernetic Self-Learning Architecture, §6 conceptual feedback loops
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §VI Collective Calibration (31.6× with caveats), §VII Predictive Foraging + CalibrationTracker, §X EvoSkills, §XI ADAS
3. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 1M (**8 missing feedback loops**) — full table
4. `/Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md` §Autocatalytic Improvement (compound math 0.9^4 = 0.656)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`

## Step 3 — SOURCE-INDEX entry `## 05-learning.md`

Read every file. Key legacy:
- `bardo-backup/tmp/mori-agents/07-self-improvement.md`, `19-practical-self-learning.md`, `22-efficiency-monitoring.md`, `23-model-routing-optimization.md`
- `bardo-backup/tmp/mori-refactor-plan/06-phase-5-cybernetic.md`, `09-exponential-roadmap.md`, `11-cybernetic-learning-dashboard.md`
- `bardo-backup/tmp/death/22-cybernetic-learning.md` (extract mechanism, drop mortality framing)
- `bardo-backup/tmp/agent-chain/self-improvement-frameworks.md`, `09-exponential-flywheels.md`
- `bardo-backup/tmp/agent-chain-new/10-self-improvement.md`

## Step 4 — implementation-plans

- `05-learning-wiring.md` — completed reference
- `modelrouting/08-learning-loops.md` (20 tasks: provider health, latency, Pareto, anomaly detection)
- `modelrouting/09-cost-normalization.md` (CostTable, blended cost 3:1, budget guardrails)
- `modelrouting/10-model-experiments.md` (Thompson Sampling Beta distribution + discount factor for drift)
- `modelrouting/11-research-context.md` (RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC — 23 sections)
- `modelrouting/12-advanced-patterns.md` (Thompson, PF, gate feedback, skills, contracts, drift)
- `modelrouting/17-meta-learning-and-corrections.md` — **8 missing cybernetic feedback loops** + stability mechanisms (hysteresis, frequency separation) + compound optimization
- `12a-cognitive-layer.md` §D (distillation pipeline, half-life values)

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/**/*.rs`
- Read: `episode_logger.rs`, `playbook.rs`, `playbook_rules.rs`, `pattern_discovery.rs`, `skill_library.rs`, `baseline.rs`, `regression.rs`, `runtime_feedback.rs`, `cascade_router.rs`, `efficiency.rs`, `costs_db.rs`, `costs_log.rs`, `task_metric.rs`, `hdc_clustering.rs`

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/05-learning
```

Write **18 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-episode-logger.md` | JSONL episode log. Retention/compaction. Full Rust schema. `.roko/episodes.jsonl`. |
| 01 | `01-playbook-system.md` | Playbook rules with globset matching. Rule extraction. Human-readable + machine-parseable PLAYBOOK.md. |
| 02 | `02-skill-library-voyager.md` | Voyager-style skill growth (Wang et al. 2023, arXiv:2305.16291). Named capabilities. Skill bundles. |
| 03 | `03-bandits-ucb-thompson-linucb.md` | UCB1, Beta Thompson Sampling, LinUCB contextual bandit with feature vectors. TrackAndStop. BanditBank. When to use each. |
| 04 | `04-cascade-router.md` | 3-stage CascadeRouter. Tier escalation logic. Persistence to `.roko/learn/cascade-router.json`. Configurable models. How it maps to T0/T1/T2. |
| 05 | `05-pattern-discovery-trigram.md` | Trigram miner. Surface recurring patterns from episodes. Feed into distillation. |
| 06 | `06-task-metrics-and-baselines.md` | Task metrics pipeline. Per-role/per-complexity baselines. Efficiency grading. Runtime feedback crate. |
| 07 | `07-regression-detection.md` | Regression detection algorithm. Historical performance comparison. Trigger points. |
| 08 | `08-cost-normalization.md` | CostTable per provider. Blended cost (3:1 input:output, per Artificial Analysis). Standard tokenizer ratios. Multi-level budget guardrails (per-task/session/day: 80% downgrade, 95% block, 100% hard stop). |
| 09 | `09-provider-health-circuit-breaker.md` | 3-state circuit breaker (Closed → Open → HalfOpen). Error-type-specific cooldowns (RateLimit 5s, Timeout 10s, ServerError 30s, Auth 5min). Health metrics collection. |
| 10 | `10-pareto-frontier-pruning.md` | Pareto frontier for provider selection. Non-dominated subsets. Multi-objective optimization (cost × quality × speed × reliability). |
| 11 | `11-thompson-sampling-drift.md` | Thompson Sampling with discount factor for concept drift. UCB1 fallback. Model-level A/B experiments extending ExperimentStore. |
| 12 | `12-self-improvement-frameworks.md` | Reflexion (Shinn et al. 2023, arXiv:2303.11366). ExpeL (Zhao et al.). DSPy (Khattab et al.). Meta-Harness (Lee et al. 2026, arXiv:2603.28052, +7.7 pp on classification, +4.7 on IMO math at 4× fewer tokens). RouteLLM, MixLLM, FrugalGPT (Chen et al. 2023, arXiv:2305.05176), GEPA, SAGE, ABC. |
| 13 | `13-8-missing-feedback-loops.md` | The 8 cybernetic feedback loops from modelrouting/17-meta-learning-and-corrections.md. For each: source → target, what it does, why it matters: (1) Health → Routing (filter unhealthy providers), (2) Conductor → Routing (penalize on abort/stuck), (3) Section → Scaffold (lift > 0.05 for prompt sections), (4) Failure → Replanning (failed patterns inform re-planning), (5) Skills → Prompts (confidence-ordered skill injection), (6) Cost → Routing (force cheaper tier on cost spikes), (7) Latency → Reward (observed latency not static normalized), (8) Experiments → Static (update router cold-start table). |
| 14 | `14-stability-mechanisms.md` | Hysteresis (10% score delta to switch — prevents thrashing). Frequency separation (router updates every episode, thresholds every 5, patterns every 20). Why stability mechanisms matter for self-learning systems. |
| 15 | `15-collective-calibration-31x.md` | **The 31.6× heuristic with explicit caveats.** `accuracy(t) = 1 - 1/sqrt(t)` for solo. `accuracy(t) = 1 - 1/sqrt(N × t)` for collective. At N=1000, sqrt(1000) = 31.6× speedup. **But this is CLT-inspired, not a theorem.** Assumes agent predictions are approximately independent, observations equally informative, no distribution shift across agents. Actual speedup less than 31.6× due to correlation, distribution shift, coordination overhead. Projected table: solo 20 preds/day / 15% success → 10K 1M preds/day / 86% at $0.40/task. Network flywheel: O(N) insights → O(N²) value (Reed's Law, Metcalfe). |
| 16 | `16-predictive-foraging.md` | Falsifiable predictions as learning signal. PredictionClaim → execute → external verifier → residual (predicted - actual) → arithmetic corrector (~50ns). CalibrationTracker per (model, task_category). `adjusted_prediction = raw - mean_bias(model, category)`. On-chain: all agents read collective calibration. New agents start at collective calibration, not zero — mechanism behind 31.6× heuristic. |
| 17 | `17-adas-and-autocatalytic.md` | ADAS (Hu et al., ICLR 2025) — meta-agent iteratively programs new agent architectures in code. Programming languages are Turing-complete → can theoretically learn any agentic system. Results: +14% ARC, +13.6 F1 reading comp, +14.4% math vs hand-designed. Transfer across dissimilar domains. Integration with Roko's 6 Synapse traits as search dimensions. Autocatalytic thesis (Kauffman) — no single component self-improves, the network catalyzes. 5 levels of self-improvement (foundation → autocatalytic loops). Compound math: 0.9^4 = 0.656 (4 independent 10% improvements → 34% fewer failures). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per `context-pack/04-writing-rules.md`. ≥200 lines per sub-doc, ≥5000 total. Citations: Reflexion, ExpeL, DSPy, Voyager, Meta-Harness, FrugalGPT, RouteLLM, MixLLM, Kauffman, Reed's Law, Metcalfe's Law, ADAS Hu et al. ICLR 2025, EvoSkills, Chen et al. 2023, Wang et al. 2023.

Cross-reference 00-architecture, 02-agents (Router and bandits), 03-composition (section effectiveness feedback), 04-verification (gate verdicts are the primary learning signal), 07-conductor, 16-heartbeat.

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- **The 8 missing cybernetic feedback loops** are the organizing concept — these are ALREADY identified but NOT YET wired. Make this prominent.
- The 31.6× calibration speedup is a **heuristic model with explicit caveats**, not a theorem. Be honest about assumptions.
- Apply naming map: golem→agent, mori→Roko Orchestrator.
- No death framing (even though "cybernetic learning" and "dead reckoning" sound like they could go there — they are computational, not biological).
- Use Write tool. Absolute paths. Don't ask questions.
