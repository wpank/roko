# Learning & Feedback: Goals

## End State

Every model call, regardless of entry point (`roko run`, `roko chat`, ACP, `roko serve`),
automatically records episodes, updates routing, tracks cost, adjusts thresholds, and feeds
insights back into future prompts. The learning subsystem is structural -- not bolted on as
an afterthought. Agents get measurably better at their tasks over time, with quantifiable
evidence of improvement at each timescale.

---

## 1. Core Properties

### 1.1 Automatic Feedback (No Bypass)

`FeedbackService` runs on every model call. No entry point bypasses it. Every path through
the system that calls an LLM must emit:
- A `FeedbackEvent::ModelCall` with prompt_section_ids, knowledge_ids, model, cost, latency
- A `FeedbackEvent::GateResult` when gates run
- A `FeedbackEvent::WorkflowComplete` when the workflow finishes

The service handles the fan-out to all downstream learning components:
- CascadeRouter observation (model selection feedback)
- Knowledge score updates (which knowledge entries helped)
- Section effectiveness updates (which prompt sections improved pass rates)
- Episode logging (full structured record)
- Cost tracking (per-model, per-provider, per-session)

### 1.2 Four Feedback Loops at Increasing Timescales

| Loop | Timescale | What Adapts | Mechanism | Status |
|---|---|---|---|---|
| L1 | Per-tick | Gate thresholds, token budgets, anomaly detection | EMA on gate pass rates, section lift weights, prompt loop / cost spike detection | Built, partially wired |
| L2 | Per-task | Model selection, retry policy, prompt sections | CascadeRouter bandit (18-dim LinUCB), ConductorBandit (19-dim Thompson), section effectiveness | Built, partially wired |
| L3 | Per-session | Knowledge consolidation, playbook extraction, dream synthesis, regression detection | LearningRuntime.record_completed_run(), DreamCycle, TierProgression, pattern mining | Built, partially wired |
| L4 | Manual / cross-session | Structural changes, experiment conclusions, workflow amendments | ExperimentStore winners, operator overrides, dream routing advice | Built, not automated |

### 1.3 Section Effectiveness Tracking

Track which prompt sections correlate with success across roles and task types:

**Current implementation (`section_effect.rs`):**
- Per-section included/excluded trial counts with pass rates
- Lift calculation: `included_pass_rate - excluded_pass_rate`
- Budget weight: `(1.0 + lift).clamp(0.5, 1.5)`
- Priority recommendation at 20+ included / 5+ excluded trials
- Persisted at `.roko/learn/section-effects.json`

**Goal:**
- PromptAssemblyService reads section_effectiveness weights before assembly
- Sections with negative lift get deprioritized or dropped
- Sections with strong positive lift get expanded token budget
- Per-role section effectiveness (a section may help implementers but hurt reviewers)

### 1.4 Cross-Agent Learning

Insights from one agent's experience inform another's prompts:

**Current mechanisms:**
- Knowledge store: entries are agent-agnostic, queryable by any agent
- Playbook store: playbooks extracted from one agent's success are available to all
- Skill library: skills registered by one agent are discoverable by others
- Pattern discovery: trigrams mined across all episodes, not per-agent

**Goal:**
- C-Factor agent contributions identify which agents drag the collective down
- Dream cycle routing advice propagates cross-agent pattern insights to CascadeRouter
- Section effectiveness is role-specific -- different roles learn different section weights

### 1.5 Predict-Publish-Correct

Every component publishes predictions, reality publishes outcomes, calibration computes error:

**Current implementation:**
- `calibration_policy.rs`: bus-backed calibration loop for predict-publish-correct
- `prediction.rs`: prediction primitives
- CascadeRouter: alpha decay serves as implicit calibration (exploration -> exploitation)
- Conductor: Thompson posteriors self-calibrate via reward updates

**Goal:**
- Model routing publishes predicted success probability before dispatch
- Gate outcomes correct the prediction
- Calibration error feeds back into exploration rate (miscalibrated -> explore more)
- Dashboard shows calibration curves (predicted vs. actual success rate)

### 1.6 Agent Self-Adjustment Dimensions

| Dimension | Component | Adaptation Mechanism |
|---|---|---|
| Model routing | CascadeRouter | LinUCB bandit with 18-dim context, 3-stage cascade |
| Gate thresholds | Adaptive thresholds | EMA per rung, persisted to gate-thresholds.json |
| Prompt sections | Section effectiveness | Lift weights adjust token budget per section |
| Tool selection | Skill library | Usage/success tracking, prompt injection |
| Recovery strategy | Conductor bandit | Thompson + linear context over 7 interventions |
| Workflow selection | Playbook store | Proven sequences injected into Layer 6 |
| Knowledge sourcing | Knowledge store | Score-weighted retrieval, anti-knowledge gating |
| Cost management | Budget guardrails | 3-scope limits with graduated actions |

---

## 2. Novel Learning Approaches

### 2.1 Reinforcement from Gate Outcomes

The gate pipeline is the primary reward signal for all learning:

**Currently implemented:**
- Gate verdicts feed CascadeRouter observations: pass -> reward 1.0, fail -> reward 0.0
- Gate verdicts feed playbook confidence: success_count / failure_count
- Gate verdicts feed section effectiveness: included sections correlated with pass
- Gate verdicts feed conductor: Thompson posterior updates per intervention action
- Gate verdicts feed knowledge scores: knowledge entries present during passes get +1
- Adaptive gate thresholds: EMA updates per rung after each gate run

**Goal: tighter gate-to-dispatch coupling:**
- Gate failure signatures feed error_pattern_store, which informs conductor context
- Gate pass on retry feeds conductor reward (the intervention worked)
- Gate pass rate by model feeds CascadeRouter reward weighting
- Gate pass rate by prompt variant feeds experiment convergence
- Multi-gate compound verdicts: a task that passes compile but fails test gets
  different treatment than one that fails compile

### 2.2 Model Routing Optimization

**Current 3-stage cascade:**
1. Static (< 50 obs): role table -- safe but expensive (defaults to premium models)
2. Confidence (50-200 obs): Wald CI pass rates -- intermediate
3. UCB (> 200 obs): full LinUCB -- optimal allocation

**Optimization levers not yet active in live paths:**
- Pareto frontier: down-weight dominated models (built, integrated into UCB scoring)
- Cost pressure: budget guardrail state modulates tier selection
- Affect-adjusted thresholds: daimon behavioral state shifts tier boundaries
- Temperament exploration: cautious agents explore less, bold agents explore more
- Cache affinity: prefer models that share KV cache with previous task
- Provider health: circuit-breaker state deprioritizes unhealthy providers
- Dream routing advice: offline analysis recommends model changes for observed patterns
- Force-backend override learning: manual escalations update static table

### 2.3 Prompt A/B Testing

**Current ExperimentStore architecture:**
- UCB1 arm selection: `mean + sqrt(2 * ln(total) / trials)` -- unsampled arms get infinity
- Wilson 95% CI for convergence: `(p + z^2/2n) / (1 + z^2/n)` with z=1.96
- Experiment lifecycle: Running -> Concluded (winner applied as static override)
- Winners exported to `experiment-winners.json` for operator review

**What is being A/B tested:**
- Prompt section text variants (e.g., different constraint phrasings)
- Model selection variants (model A vs. model B for a specific role)
- Per-variant metric tracking (not just pass/fail -- also cost, duration)

**Goal: automated experiment creation:**
- Dream cycle identifies promising section variants from successful episodes
- System auto-creates experiments for new sections with unknown effectiveness
- Winning variants are auto-applied after sufficient evidence (Wilson CI non-overlapping)

### 2.4 Offline Knowledge Consolidation (Dream Cycle)

The dream cycle is the most novel learning mechanism:

**4-phase processing:**
1. Hypnagogia: liminal filtering (ThalamicGate relevance scoring, HomuncularObserver
   self-awareness tagging, ExecutiveLoosener generating variant encodings)
2. NREM: episode clustering by plan/task HDC similarity, structural consolidation via
   CrossEpisodeConsolidator, distillation into KnowledgeEntry candidates via Claude Haiku
3. REM: counterfactual episode generation (what if we used a different model? different
   prompt?), cross-domain hypothesis synthesis, threat rehearsal
4. Integration: tier progression (D1 insights -> D2 heuristics -> D3 playbook),
   StagingBuffer promotion (Raw -> Replayed -> Validated), routing advice generation

**Replay prioritization (Mattar-Daw):**
- Utility = information gain * task importance * recency
- Affect-weighted selection: emotionally salient episodes replayed more
- Budget-constrained: per-phase USD limits

**What dreams produce:**
- Durable knowledge entries (insights, warnings, causal links)
- New playbooks from successful clusters
- Routing recommendations for CascadeRouter
- C-Factor regression analysis
- Performance stall detection with actionable notes
- Strategy hypotheses from cross-domain structural similarity

---

## 3. Knowledge Store Integration

### 3.1 Knowledge-Informed Dispatch

**Current integration points:**
- `PromptAssemblyService.with_knowledge_store()`: queries store during prompt assembly
- Knowledge entries injected into Layer 6 of 9-layer prompt (domain context)
- `FeedbackService.record_knowledge_usage()`: tracks which entries were used, gate outcome
- Knowledge scores: cumulative +1/-1 per entry based on gate pass/fail

**Goal: knowledge-informed model routing:**
- CascadeRouter consults knowledge store for task-specific model recommendations
- Knowledge entries with model-preference tags influence routing context
- Anti-knowledge entries for specific model+task combinations deprioritize that pair

### 3.2 Admission and Quality Control

**Three-tier admission:**
1. LightAdmissionGate (fast path): confidence > 0.5, novelty > 0.3, source trust > 0.65
2. KnowledgeAdmissionStore (full evidence): min 0.72 confidence, multi-source corroboration
3. Anti-knowledge validation: min 0.65 confidence, HDC gating at 0.5/0.7/0.9 thresholds

**Tier progression (evidence-based):**
- Transient: initial admission, 3 passing verdicts needed for promotion
- Consolidated: promoted after gate-backed evidence, 2 failures trigger demotion
- Canonical: long-lived, high-confidence entries
- Expiry review: triggered at 2x half-life age

### 3.3 Knowledge Feedback Scoring

**File:** `crates/roko-learn/src/feedback_service.rs`

The knowledge feedback loop is the most complete closed loop in the system:

1. PromptAssemblyService includes knowledge entries in prompt, records entry IDs
2. FeedbackService receives ModelCall event with knowledge_ids
3. FeedbackService remembers provenance (run_id -> knowledge_ids mapping)
4. Gate runs, FeedbackService receives GateResult
5. FeedbackService resolves provenance, applies +1 (pass) or -1 (fail) to each entry
6. Scores persisted to `knowledge-scores.json`
7. On next prompt assembly, scores influence retrieval ranking (higher score -> more likely)
8. On restart, scores loaded from disk -- learning persists across sessions

**Verified in test:** `test_knowledge_loop_scoring` exercises the full cycle from
store -> assemble -> model_call -> gate -> score_update -> reload -> verify.

---

## 4. UX Data Feeds (from v2 Showcase)

### 4.1 EpisodeScrubber Panel
- Full timeline with 16 event markers (user/phase/tool/knowledge/perm/gate/done)
- Scrub slider with position readout ("6.9 / 11.5 min")
- "Branch from here" button to fork from any position
- Data: `EpisodeTimeline` events array with timestamp_fraction, kind, label

### 4.2 Learnings Extraction
- Auto-extracted insights from completed episodes
- Data: `LearningsExtraction` array of (finding, action, target) tuples

### 4.3 Post-Replay Router Update
- After replay, cascade router shows updated component scores
- Data: `PostReplayUpdate` per-component: name, score, action taken

### 4.4 Cumulative Savings
- "Cumulative session savings vs always-opus: 87%"
- Data: `CostSparkline` with last N turn costs and trend percentage

### 4.5 Override Recording
- Manual model escalations recorded and learned
- Data: `OverrideRecord` with task_pattern, from_model, to_model, frequency, learned_action

### 4.6 Pair Convergence
- "Pair sessions log convergence rate"
- Data: `ConvergenceRate` with rounds, must_fixes, nits, knowledge_overlap_factor

---

## 5. Measurable Success Criteria

### 5.1 Learning Effectiveness Metrics

The learning subsystem should demonstrate measurable improvement along these dimensions:

| Metric | Baseline | Target (30 days) | Target (90 days) | Source |
|---|---|---|---|---|
| CascadeRouter stage | Static | Confidence | UCB | cascade-router.json |
| Gate first-pass rate | Unknown | Tracked, stable | Improving trend | episodes.jsonl |
| Cost per successful task | Unknown | Tracked, stable | Decreasing trend | costs.jsonl |
| Section lift variance | Unknown | Measured | Sections converged | section-effects.json |
| Knowledge entry count | 0 | 50+ | 200+ | neuro/knowledge.jsonl |
| Playbook count | 0 | 10+ | 30+ | learn/playbooks/ |
| Conductor non-Continue rate | 0% | 15%+ | Stable, effective | conductor.json |
| Experiment winners applied | 0 | 3+ | 10+ | experiment-winners.json |
| Dream cycles completed | 0 | 7+ | 30+ | .roko/dreams/ |
| Regression alerts acted on | N/A | Logged | Auto-responded | regression alerts |

### 5.2 Coverage Metrics

Every model-calling entry point must be instrumented:

| Entry Point | FeedbackService | Episodes | Routing | Cost | Status |
|---|---|---|---|---|---|
| `roko run` | Partial | Yes | Partial | Yes | Phase 0 target |
| `roko chat` | No | No | No | No | Phase 0 target |
| ACP | No | No | No | No | Phase 0 target |
| `roko serve` (API) | No | No | No | No | Phase 1 target |
| `roko plan run` | Dead code | Dead code | Dead code | Dead code | Phase 1 target |
| `roko agent chat` | No | No | No | No | Phase 1 target |

### 5.3 Self-Improvement Velocity

The rate at which the system improves should itself accelerate:

- **Week 1-2:** Baseline established (all entry points instrumented, initial observations)
- **Week 3-4:** CascadeRouter transitions from Static to Confidence stage
- **Week 5-8:** First experiment conclusions, playbook extraction from dream cycles
- **Week 9-12:** UCB stage reached, conductor starts learned interventions
- **Week 13+:** Knowledge-informed routing, automated experiment proposals

Each milestone should be visible in `roko status` and `roko learn all` output.

### 5.4 Cost Reduction Targets

Model routing optimization should produce measurable cost savings:

- Static stage: premium model for everything (baseline cost = 1.0x)
- Confidence stage: simple tasks route to cheaper models (target: 0.7x)
- UCB stage: optimal allocation by task type (target: 0.5x)
- With budget enforcement: hard ceiling prevents runaway (target: never exceed 2x daily budget)
- With Pareto frontier: dominated models excluded (target: additional 10% saving)

The "cumulative savings vs. always-premium" metric should be visible in the dashboard
and reported by `roko learn efficiency`.

---

## 6. Architectural Invariants

These properties must hold as the learning subsystem evolves:

### 6.1 Append-Only Persistence

All learning data is append-only (JSONL). No in-place mutation of historical records.
This ensures:
- Auditability: every decision can be traced to its evidence
- Debuggability: no state corruption from partial writes
- Reproducibility: replaying the log reproduces the learned state

Exceptions: JSON snapshots (cascade-router.json, experiments.json, etc.) are full rewrites
via atomic temp+rename. These are derived state, reconstructable from the append-only logs.

### 6.2 Graceful Degradation

Every learning component must tolerate missing or corrupt persistence files:
- Missing file: start from clean state, log info
- Corrupt file: skip corrupt entries, log warning, continue with valid entries
- Missing FeedbackService: model calls still work, just without learning
- Missing CascadeRouter: fall back to config default model

No learning failure should prevent the system from dispatching work.

### 6.3 Frequency Gating

Expensive learning operations (skill mining, pattern discovery, distillation) are
frequency-gated via `UpdateFrequency` to prevent I/O bottlenecks:
- `router_every_n_episodes: 1` (cheap observation)
- `experiments_every_n: 1` (cheap stat update)
- `skill_mining_every_n: 10` (moderate)
- `pattern_discovery_every_n: 20` (moderate)
- `distiller_every_n: 50` (expensive, calls LLM)

### 6.4 Deterministic Replay

Given the same episode log and the same learning configuration, `LearningRuntime` must
produce the same learned state. This enables:
- Testing: unit tests with fixed episode sequences
- Debugging: replay a production log to reproduce a routing decision
- Migration: schema changes can be validated by replaying existing logs

---

## 7. Gap Summary

| Gap | Priority | Effort | Impact |
|---|---|---|---|
| Wire FeedbackService to `roko chat` | P0 | Small | All chat sessions feed learning |
| Wire FeedbackService to ACP | P0 | Small | Editor integration feeds learning |
| Full RoutingContext in `roko run` | P1 | Medium | Better model selection signal quality |
| Budget enforcement in live paths | P1 | Medium | Cost protection for production use |
| Conductor in live paths | P1 | Medium | Learned retry policy for all paths |
| Section effectiveness -> PromptAssemblyService | P1 | Small | Prompt sections adapt to evidence |
| Knowledge-informed model routing | P2 | Medium | Store insights inform CascadeRouter |
| Dream cycle cron trigger | P2 | Small | Offline learning runs automatically |
| Automated experiment creation | P2 | Large | System proposes its own A/B tests |
| L4 loop: agent-proposed structural changes | P3 | Large | Full autonomy in self-improvement |
| Episode replay streaming to ACP | P3 | Medium | Editor scrubber visualization |
| Cross-approach episode comparison | P3 | Medium | Parallel strategy evaluation |

| Gap | Priority | Effort | Impact |
|---|---|---|---|
| Wire FeedbackService to `roko chat` | P0 | Small | All chat sessions feed learning |
| Wire FeedbackService to ACP | P0 | Small | Editor integration feeds learning |
| Full RoutingContext in `roko run` | P1 | Medium | Better model selection signal quality |
| Budget enforcement in live paths | P1 | Medium | Cost protection for production use |
| Conductor in live paths | P1 | Medium | Learned retry policy for all paths |
| Section effectiveness -> PromptAssemblyService | P1 | Small | Prompt sections adapt to evidence |
| Knowledge-informed model routing | P2 | Medium | Store insights inform CascadeRouter |
| Dream cycle cron trigger | P2 | Small | Offline learning runs automatically |
| Automated experiment creation | P2 | Large | System proposes its own A/B tests |
| L4 loop: agent-proposed structural changes | P3 | Large | Full autonomy in self-improvement |
| Episode replay streaming to ACP | P3 | Medium | Editor scrubber visualization |
| Cross-approach episode comparison | P3 | Medium | Parallel strategy evaluation |
