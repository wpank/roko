# PRD-05 — Self-Improvement Flywheel: Compounding Value from Every Evaluation

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Crates**: `roko-eval` (trace capture), `roko-learn` (storage + bandits), `roko-neuro` (pattern persistence)
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions), PRD-04 (Judge Methodology)
**Supersedes**: `tmp/visual-gate/prd/PRD-04-Self-Improvement-Flywheel.md`

---

## 0. Scope

This document specifies how every evaluation run produces compounding value. The
framework generates traces, mines preferences, discovers patterns, synthesizes
curricula, optimizes prompts, and trains a post-processor -- seven flywheel steps
that turn evaluation from a cost center into the system's primary appreciating asset.

The document also covers five autonomous eval generation mechanisms, six anti-Goodhart
safeguards, integration with the existing `roko-learn` subsystem (cascade router,
episode logger, experiment store, feedback service, playbook store), the learning
event schema, and the phased implementation timeline.

---

## 1. Core Thesis

**Spend fine-tune budget on the post-fixer first, not the judge.**

Three reasons:

1. **Crisper signal.** Repair has a near-binary verification surface: broken code
   does not compile, broken layouts fail visual regression. Judge quality requires
   human calibration data that accumulates slowly. From Song et al. (ICLR 2025):
   self-improvement only works when verification difficulty <= generation difficulty.

2. **More abundant data.** Every error log, compile failure, console exception,
   and visual regression diff is a training pair. The flywheel generates
   `(broken_output, fixed_output)` pairs from normal operation. Judge training
   data requires expensive human annotation.

3. **Savings compound.** A post-fixer that learns common failure classes reduces
   frontier API spend on every subsequent generation. Fewer retries, fewer tokens,
   shorter wall-clock time. A better judge only saves by reducing false positives.

The judge stays frontier-API for 12+ months. APIs achieve ~70-80% human agreement
on pairwise judgments (Chen et al., ICML 2024). Cost (~$0.05-0.20/judgment) is
real but judging is parallelizable and infrequent compared to generation.

**One exception -- train a tiny router classifier.** DistilBERT-class on ~1k labeled
pairs. Gates questions like "does this prompt need clarification?" Cheap, high-signal.

---

## 2. The Seven Flywheel Steps

Every evaluation run must emit labeled, structured, retrievable artifacts, or
the system does not compound.

```
 +-----------------------------------------------------------+
 |              Evaluation Completes                          |
 +---------------------------+-------------------------------+
                             |
         +-------------------v-------------------+
         |  Step 1: Trace Capture (EvalTrace)    |
         |  Full execution record -> JSONL       |
         +-------------------+-------------------+
                             |
         +-------------------v-------------------+
         |  Step 2: Auto-Grade                   |
         |  Panel + deterministic + diff         |
         +-------------------+-------------------+
                             |
         +-------------------v-------------------+
         |  Step 3: Preference Mining            |
         |  Emit PreferenceTriple records        |
         +-------------------+-------------------+
                             |
     +-----------------------+-----------------------+
     |                       |                       |
     v (nightly)             v (nightly)             v (weekly)
 +-----------+       +--------------+        +--------------+
 | Step 4:   |       | Step 5:      |        | Step 6:      |
 | Pattern   |       | Curriculum   |        | MIPROv2      |
 | Extraction|       | from Failures|        | Optimization |
 +-----------+       +--------------+        +--------------+
                             |
         +-------------------v-------------------+
         |  Step 7: RFT Post-Processor (month 7+)|
         |  Train eval-autofixer-01              |
         +---------------------------------------+
```

### 2.1 Step 1: Trace Capture

Every evaluation emits a complete `EvalTrace` (PRD-01, Section 5). The trace is
the atomic unit of flywheel data.

#### 2.1.1 What the Trace Captures

| Field | What | Flywheel Use |
|---|---|---|
| `artifact` | ArtifactRef (URL, path, diff, screenshot) | Links trace to evaluated thing |
| `profile` | ProfileRef (which evaluation ran) | Groups traces by strategy |
| `evidence_phase` | Per-collector timing + success | Infrastructure bottleneck detection |
| `criterion_phase` | Per-criterion timing + results | Per-dimension preference mining |
| `verdict` | Aggregate result (pass/fail + findings) | Curriculum generation |
| `pipeline_context` | Bandit arm, model, retrieval-k, prompt variant | Bandit attribution |
| `cost` | LLM tokens, compute time, API dollars | Cost optimization |
| `preference_triples` | Emitted during evaluation | Direct training signal |

#### 2.1.2 Pipeline Context

The trace captures the full pipeline configuration that produced the artifact.
This is what makes bandits possible -- attribute outcomes to specific arms.

```rust
/// File: crates/roko-eval/src/trace.rs
///
/// The pipeline configuration that produced the artifact being evaluated.
/// Populated from AgentEfficiencyEvent fields at trace emission time.
/// See crates/roko-learn/src/efficiency.rs for source fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub model: String,
    pub backend: String,
    pub prompt_variant: Option<String>,
    pub retrieval_k: Option<u32>,
    pub used_clarifying_turn: bool,
    pub used_post_fixer: bool,
    pub cascade_stage: Option<String>,
    pub agent_role: String,
    pub generation_cost_usd: f64,
    pub generation_tokens: u64,
}
```

#### 2.1.3 Integration with Existing EpisodeLogger

The `EvalTrace` is a sibling of, not a replacement for, the existing `Episode`
from `crates/roko-learn/src/episode_logger.rs`.

| Record | Purpose | Storage |
|---|---|---|
| `Episode` | Agent turn accounting (tokens, cost, gate pass/fail) | `.roko/episodes.jsonl` |
| `EvalTrace` | Full evaluation record (evidence, scores, findings) | `.roko/eval/traces.jsonl` |

Cross-referenced by `task_id` and timestamp. The episode's `gate_verdicts` carries
the summary; the `EvalTrace` carries full detail.

#### 2.1.4 Storage

Traces are JSONL at `.roko/eval/traces.jsonl`. Append-only, matching the
`EpisodeLogger` pattern. Crash mid-write corrupts at most one line. Reader
is tolerant of malformed lines.

Traces are also promoted to engrams via `Engram::builder(Kind::EvalTrace)` for
integration with the signal DAG and the `roko-neuro` knowledge store.

---

### 2.2 Step 2: Auto-Grade

Auto-grading combines three signal sources:

1. **Deterministic criteria**: compile, test, lint, a11y, console errors.
   Binary and authoritative.
2. **Disjoint-family judge panel**: pairwise comparison from PRD-04. Three
   judges, trimmed-mean aggregation, position swap. Always pairwise against
   fixed anchor.
3. **Screenshot-diff against success bank**: structural similarity (DSSIM)
   against curated human-approved screenshots.

#### 2.2.1 Disagreement Triggers Human Review

When 2 of 3 panel judges disagree with the trimmed-mean aggregate, flag for
human review. This is the primary mechanism for collecting high-quality human
labels that calibrate the judge panel.

```rust
/// File: crates/roko-eval/src/auto_grade.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReviewDecision {
    pub requires_review: bool,
    pub reason: HumanReviewReason,
    pub priority: u32,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HumanReviewReason {
    JudgeDisagreement { disagreeing_count: u32, total_count: u32 },
    NearThreshold { score: f64, threshold: f64, dead_band: f64 },
    CanaryItem { canary_id: String },
    NewCriterion { criterion: String },
}
```

#### 2.2.2 Connection to ExperimentStore

Auto-grade results feed the existing `ExperimentStore` from
`crates/roko-learn/src/prompt_experiment.rs`. When a trace carries a
`pipeline_context.prompt_variant`, the pass/fail outcome is reported to
`VariantStats`:

```rust
// Wiring auto-grade into ExperimentStore
if let Some(variant_id) = trace.pipeline_context.prompt_variant.as_ref() {
    experiment_store.record_outcome(variant_id, trace.verdict.passed);
}
```

This closes the loop: `ExperimentStore`'s UCB1 bandit uses pass/fail outcomes
to converge on the best prompt variant. Auto-grade provides those outcomes
automatically, without human labeling.

---

### 2.3 Step 3: Pairwise Preference Mining

Every preference signal is logged as a `PreferenceTriple`. This collection
is a private WebDev Arena.

#### 2.3.1 PreferenceTriple

```rust
/// File: crates/roko-eval/src/preference.rs
///
/// A pairwise preference observation. Core training signal for BT models,
/// RLHF reward models, and the RFT post-processor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceTriple {
    pub id: String,
    pub prompt: String,
    pub candidate_a: String,
    pub candidate_b: String,
    pub preferred: PreferenceChoice,
    pub source: PreferenceSource,
    pub trace_id_a: Option<String>,
    pub trace_id_b: Option<String>,
    pub task_id: String,
    pub timestamp: String,
    pub viewport: Option<String>,
    pub criterion_deltas: Vec<CriterionDelta>,
    pub confidence: f64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceSource {
    /// User edited the output. Strongest signal (confidence 1.0).
    UserEdit,
    /// User selected one variant from multiple.
    UserSelection,
    /// Judge panel pairwise comparison.
    JudgePanel,
    /// External benchmark data.
    ExternalBenchmark,
    /// Previous passing vs current failing.
    RegressionComparison,
}
```

#### 2.3.2 Sources of Preference Signals

**Judge panel verdicts.** Every pairwise comparison from PRD-04 produces a triple.
When trace A and trace B evaluate the same artifact and scores differ beyond the
dead-band, a triple is emitted.

**User edits (strongest signal).** Pre-edit = negative, post-edit = positive.
Confidence 1.0 because the user demonstrated exactly what "correct" looks like.

**User variant selection.** When the system presents N variants, the user's
selection produces N-1 triples.

#### 2.3.3 Bootstrapping with External Data

Before internal data accumulates, bootstrap with
`lmarena-ai/webdev-arena-preference-10k` (commercial-use licensed). Tagged with
`source: ExternalBenchmark` and exponentially downweighted (half-life = 1,000
internal triples).

#### 2.3.4 Integration with EpisodeLogger

Preference triples append to `.roko/learn/preferences.jsonl` using the same
pattern as `EpisodeLogger`. The `Episode` record gains an `extra` field entry:

```rust
pub const EPISODE_PREFERENCE_COUNT_KEY: &str = "preference_triples_emitted";
```

#### 2.3.5 RLAIF and Self-Feedback Integration

Recent research on RLAIF (arXiv 2309.00267) shows AI feedback can match or
exceed human feedback quality when the labeler model is the same size as the
policy. For our use case, this means judge panel verdicts are a valid reward
signal for pipeline optimization, even without human labels.

RLSF (Reinforcement Learning from Self-Feedback, arXiv 2507.21931) goes further:
the model's own confidence serves as an intrinsic reward. We apply this by
tracking the judge panel's confidence across evaluations and using low-confidence
verdicts as exploration signals for the cascade router.

---

### 2.4 Step 4: Pattern Extraction via AST Diffing (Nightly)

A nightly batch job clusters successful evaluations and extracts reusable patterns.

#### 2.4.1 Cluster Successful Components

1. For each passing trace, extract AST, DOM structure, and screenshot.
2. Cluster by: DOM shape, CSS class bag, component prop signature,
   visual embedding (CLIP/SigLIP).
3. Emit cluster centroids as pattern library entries.

#### 2.4.2 Diff Successful-vs-Rejected Pairs

For each `(passing, failing)` pair sharing the same `artifact.id`:
1. AST diff: structural changes.
2. CSS diff: style changes.
3. DOM diff: element additions/removals.

Patterns in rejected-but-not-approved = anti-patterns.
Patterns in approved-but-not-rejected = positive patterns.

#### 2.4.3 Feed as Few-Shots via DSPy

Both positive and negative patterns feed prompt optimization:
- Positive: "Successful pricing tables use `grid-cols-3` with `gap-6`."
- Negative: "Rejected pricing tables frequently used inline styles."

Incorporated via DSPy `BootstrapFewShotWithRandomSearch`.

#### 2.4.4 Pattern Library

```rust
/// File: crates/roko-eval/src/pattern_library.rs
///
/// Patterns are stored in roko-neuro knowledge store as engrams with
/// Kind::Pattern. Queried at dispatch time via PlaybookStore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub fingerprint: Option<String>,
    pub polarity: PatternPolarity,
    pub support_count: u32,
    pub avg_score: f64,
    pub template: Option<String>,
    pub reference_screenshot: Option<String>,
    pub anti_pattern_description: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternPolarity {
    Positive,
    Negative,
}
```

#### 2.4.5 Connection to roko-neuro

Patterns persist into the `roko-neuro` knowledge store
(`crates/roko-neuro/src/knowledge_store.rs`) as engrams. The neuro store provides:

- **Tier progression**: useful patterns promote from working to episodic to
  semantic memory.
- **Similarity search**: HDC fingerprint (from `roko_primitives::hdc`)
  enables fast approximate nearest-neighbor queries at dispatch time.
- **Decay**: stale patterns naturally decay without manual cleanup.

At dispatch time, the orchestrator
(`crates/roko-cli/src/orchestrate.rs`) queries the neuro store for patterns
matching the current task category and injects top-K into the system prompt
via `RoleSystemPromptSpec` enrichment.

---

### 2.5 Step 5: Curriculum-from-Failures (WebRL Pattern)

**Source**: WebRL (Qi et al., arXiv 2411.02337).

Failed evaluations are raw material for synthetic training tasks targeting
the system's specific weaknesses.

#### 2.5.1 Cluster Failed Runs by Judge Rationale

1. Extract judge rationale text from failing traces.
2. Embed using the same model as the neuro store.
3. Cluster (k-means or HDBSCAN). Each cluster = a failure mode.

Example clusters:
- "pricing card layout overflows on mobile viewport"
- "dashboard sparkline charts not rendering"
- "form validation error messages not accessible"

#### 2.5.2 Synthetic Training Tasks

For each cluster with 3+ members:
1. Extract common failure description from centroid.
2. Generate synthetic task prompt exercising the failure mode.
3. Add edge-case variations (viewport extremes, long text, RTL).

```rust
/// File: crates/roko-eval/src/curriculum.rs
///
/// Integration: curriculum tasks are injected into the plan execution
/// loop via existing `roko plan run` infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumTask {
    pub id: String,
    pub source_cluster_id: String,
    pub cluster_size: u32,
    pub prompt: String,
    pub acceptance_criteria: Vec<String>,
    pub eval_profile: String,
    pub variants: Vec<CurriculumVariant>,
    pub priority: f64,
    pub generated_at: String,
    pub status: CurriculumStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurriculumStatus {
    Pending,
    InProgress,
    Promoted,   // Succeeded -> joins regression set
    Retained,   // Failed again -> stays in curriculum
    Superseded,
}
```

#### 2.5.3 Integration with Existing Curriculum Module

The existing `crates/roko-learn/src/curriculum.rs` provides curriculum
generation infrastructure. The flywheel extends this with visual-domain
specific failure clustering and variant generation.

#### 2.5.4 Promote Successes to Regression Set

When a curriculum task succeeds:
1. Task + passing trace join the regression eval set.
2. Future pipeline changes are tested against this set.
3. Status becomes `Promoted`.

Per LiveCodeBench (Jain et al., 2024): continuously collect new problems;
do not rely on a fixed set. The regression set grows organically.

#### 2.5.5 Integration with Post-Gate Reflection

The existing `crates/roko-learn/src/post_gate_reflection.rs` records agent
reflections after gate failures. These reflections provide structured failure
analysis that supplements the judge rationale for curriculum clustering:

```rust
/// Wire post-gate reflections into curriculum clustering.
/// Reflections contain structured failure analysis from the agent's
/// perspective, complementing the judge panel's external evaluation.
pub fn enrich_cluster_with_reflections(
    cluster: &mut FailureCluster,
    reflections: &[ReflectionInput],
) {
    for reflection in reflections {
        cluster.agent_hypotheses.push(reflection.agent_hypothesis.clone());
        cluster.attempted_fixes.push(reflection.attempted_fix.clone());
    }
}
```

---

### 2.6 Step 6: MIPROv2 Optimization (Weekly)

**Source**: MIPROv2 (Opsahl-Ong et al., EMNLP 2024).

Weekly optimization over pipeline surfaces with the best signal-to-noise ratio.

#### 2.6.1 Optimization Targets

| Target | Why | Mechanism |
|---|---|---|
| Retrieval queries for components | Better context = better generation | Optimize query templates |
| Clarifying-question prompt | Cheapest improvement | Optimize clarification prompt |
| Judge rubrics | Keeps judges calibrated | Optimize rubric phrasing |
| AutoFix repair prompts | Better feedback = faster convergence | Optimize repair prompts |

**Do NOT optimize the base generator prompt directly.** Too many degrees of
freedom, too little signal per evaluation.

#### 2.6.2 Bandit Routing over Pipeline Arms

In addition to prompt optimization, run bandits over discrete pipeline choices.
Integrates with the existing cascade router at
`crates/roko-learn/src/cascade_router.rs`.

The cascade router already implements three-stage routing (Static -> Confidence
-> UCB1). The flywheel extends this to additional dimensions:

```rust
/// File: crates/roko-eval/src/pipeline_arm.rs
///
/// Integration: arms register in CascadeRouter as additional context
/// dimensions. Existing LinUCBRouter handles bandit math.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineArm {
    pub model: String,
    pub retrieval_k: u32,
    pub with_clarifying_turn: bool,
    pub with_post_fixer: bool,
}

impl PipelineArm {
    pub fn to_features(&self) -> Vec<f64> {
        vec![
            self.retrieval_k as f64 / 10.0,
            if self.with_clarifying_turn { 1.0 } else { 0.0 },
            if self.with_post_fixer { 1.0 } else { 0.0 },
        ]
    }
}
```

#### 2.6.3 Integration with Existing CascadeRouter

The `CascadeRouter` provides:
- `LinUCBRouter`: contextual bandit for exploration/exploitation.
- `CascadeStage`: three-stage maturity (Static -> Confidence -> UCB1).
- `RoutingDecisionLog`: audit log for explainability.
- `compute_routing_reward_v2`: combines pass rate, cost, latency.

#### 2.6.4 Integration with Existing ExperimentStore

Weekly MIPROv2 creates experiments in `ExperimentStore`:

```rust
let experiment = PromptExperiment {
    id: format!("miprov2-{}-{}", target, week_number),
    section_name: target.section_name(),
    variants: miprov2_generated_variants,
    // UCB1 bandit selection via VariantStats
};
experiment_store.add_experiment(experiment);
```

When experiment concludes, winning variant is promoted via `ExperimentWinner`
and persisted to `.roko/learn/static-overrides.json`.

#### 2.6.5 BetterTogether Integration

BetterTogether (Soylu et al., EMNLP 2024) shows alternating prompt optimization
and weight fine-tuning achieves up to 60% gains over weight-only optimization.
This validates the two-phase strategy: prompt optimization months 1-6, then
weight fine-tuning (RFT post-processor) month 7+.

---

### 2.7 Step 7: RFT Post-Processor Training (Month 7+)

**Source**: v0's `vercel-autofixer-01` (Fireworks RFT).

#### 2.7.1 When to Train

Training triggered when:
- 10,000+ labeled `(broken, fixed)` pairs from Steps 2 and 4.
- Pairs span >= 20 distinct failure classes.
- Average per-class count >= 100.

#### 2.7.2 Training Recipe

1. Base model: Llama-3.1-8B or Qwen2.5-Coder-7B.
2. Method: Rejection Fine-Tuning (RFT) on Fireworks or Together.
3. Input: failing screenshot + error evidence.
4. Output: code patch in unified diff format.
5. Model name: `eval-autofixer-01`.

#### 2.7.3 Why This Compounds

1. **Every fix reduces frontier spend forever.** Previous frontier retry ->
   single 7B inference.
2. **Survives model swaps.** Targets failure patterns, not model quirks.
3. **Gets better over time.** Each cycle adds repair pairs. Training signal
   is near-binary (broken -> working).
4. **Composes with everything.** Runs after any generator, any model, any
   pipeline config. Orthogonal to the rest of the stack.

#### 2.7.4 Integration with Existing Skill Library

The existing `crates/roko-learn/src/skill_library.rs` provides template pattern
generation from successful episodes. The RFT post-processor training pipeline
consumes skill library entries as additional training data:

```rust
/// Wire skill library patterns into RFT training data.
///
/// Successful skill library templates represent agent-discovered
/// solutions. The (problem, solution) pairs extracted from templates
/// augment the autofixer training set.
pub fn extract_training_pairs_from_skills(
    skill_lib: &SkillLibrary,
) -> Vec<(String, String)> {
    skill_lib.templates()
        .filter(|t| t.success_count >= 3)
        .map(|t| (t.problem_description.clone(), t.solution_template.clone()))
        .collect()
}
```

---

## 3. Five Autonomous Eval Generation Mechanisms

### 3.1 Property-Based Eval Generation

Mine component interfaces to generate property-based assertions:

1. Extract props/types from TypeScript/Rust definitions.
2. Generate assertions: "Button with `variant='primary'` must have
   `background-color` matching design token."
3. Generate fuzz variants: random props, extreme content, edge viewports.
4. Self-curation: high-variance properties gain weight, always-pass are pruned.

Use Benchmark Self-Evolving (Wang et al., COLING 2025) six reframing operations
to expand each assertion into 6+ variants.

### 3.2 Scenario Mining from Production

- **Regression replay**: production bug -> before/after test scenario.
- **New component analysis**: new design system component -> exercise scenarios.
- **A/B test mining**: clear A/B winners -> reference/negative pair.

Per Re-Evaluating EVMBench (Storhaug & Meling 2026): curated benchmarks
overestimate capability. Continuously mine NEW scenarios from real deployments.

### 3.3 Predictive Foraging as Evaluation

**Source**: Friston's Free Energy Principle.

Before implementing, the agent predicts outcomes. After, compare prediction
to reality. The residual IS an evaluation metric:

- Render time: predicted vs actual.
- Bundle size: predicted vs actual.
- Journey success: predicted vs actual.

Agents with consistently small residuals are better calibrated, not just
more successful. This integrates with the existing
`crates/roko-learn/src/prediction.rs` module.

### 3.4 Red Team Agents

Specialized agents that find failures the system currently misses:

1. Select target: a highly-rated component or pattern.
2. Generate hypothesis: "This breaks with RTL text."
3. Construct test scenario.
4. Execute through the evaluation framework.
5. If counterexample succeeds: agent earns reward, scenario joins eval suite.
6. If fails: pattern is strengthened, agent pays cost.

Per AGENTPOISON (NeurIPS 2024): poisoning <0.1% of knowledge base achieves
63% attack success. The playbook library, pattern library, and judge prompts
are all attack surfaces. Red team agents should specifically test these.

### 3.5 Meta-Loop Optimization (Autoresearch Pattern)

**Source**: Karpathy autoresearch (March 2026), DSPy/MIPROv2.

The autoresearch pattern applied to evaluation pipeline configuration:

```
prepare.py equivalent:
  - Eval suite from Mechanisms 1-4
  - 500+ property assertions + 1000+ scenarios + 200+ adversarial cases
  - Fixed. Never modified by the optimizer.

train.py equivalent:
  - Pipeline config parameters (thresholds, retrieval-k, model selection)
  - The ONLY thing the optimizer changes.

Loop:
  1. Propose config change
  2. Run 100 eval scenarios with new config
  3. Measure: success rate, FP rate, FN rate, cost
  4. Compare against baseline
  5. Keep if improved, discard if not
```

---

## 4. Six Anti-Goodhart Safeguards

### 4.1 Disjoint-Family Panel with Trimmed Mean

Panel diversity prevents single-model bias domination. Inner-loop reward and
held-out validation never share a judge.

### 4.2 Frozen Human-Rated Canary Set

200-500 prompts, Krippendorff alpha >= 0.8. Re-evaluated every release.
Panel score rises + canary stalls = Goodharting detected.

### 4.3 Quarterly Rubric Rotation

Rotate rubric emphasis quarterly. Per LiveBench (ICLR 2025): monthly updates
prevent contamination.

### 4.4 Adversarial Red-Team Eval

Test for: gradient bombs, oversaturated images, blur-everything, screenshot
copy-paste, token-stuffing.

### 4.5 Reference-Conditioned BT Against Strong Baseline

Always pairwise against anchor. BT is harder to inflate because the anchor
is a moving target.

### 4.6 Canary Correlation Monitor

Track Spearman rho between inner-loop judge and canary. rho < 0.6 = drift.
Implemented as a scheduled check integrated with
`crates/roko-learn/src/drift.rs`.

### 4.7 Preference As Reward (PAR)

Recent work (2025-2026) shows using pairwise preferences directly as rewards
outperforms separate scalar reward models by 5+ percentage points. Our
architecture is natively aligned: judge panel verdicts ARE the reward signal.

---

## 5. Gate-to-Agent Feedback Loop

### 5.1 The Core Feedback Circuit

The self-improvement flywheel closes three feedback loops between the gate
pipeline and agent behavior:

```
+----------+     fail + findings     +-------------+
|   Gate   |------------------------>| Curriculum  |
| Pipeline |                         | Generator   |
+----+-----+                         +------+------+
     |                                      |
     | pass/fail                            | synthetic tasks
     v                                      v
+----+-----+                         +------+------+
| Cascade  |                         |   Agent     |
| Router   |<------------------------| (retrained  |
+----+-----+   performance metrics   |  or re-     |
     |                                |  prompted)  |
     | model selection                +------+------+
     v                                      ^
+----+-----+                                |
| Agent    |                                |
| Dispatch |------- generates artifacts ----+
+----------+
```

### 5.2 Integration with FeedbackService

The existing `FeedbackService` at `crates/roko-learn/src/feedback_service.rs`
provides the recording infrastructure. It already tracks:

- `KnowledgeOutcome`: Success/Failure/Partial per knowledge entry.
- Per-section effectiveness via `SectionEffectivenessRegistry`.
- Knowledge scores that feed prompt section selection.

The flywheel extends this with evaluation-specific feedback:

```rust
/// File: crates/roko-eval/src/feedback_bridge.rs
///
/// Bridge between EvalTrace and the existing FeedbackService.
/// Converts evaluation verdicts into KnowledgeOutcome records.
pub fn eval_trace_to_feedback(
    trace: &EvalTrace,
    feedback_service: &FeedbackService,
) -> Result<(), FeedbackError> {
    let outcome = if trace.verdict.passed {
        KnowledgeOutcome::Success
    } else {
        KnowledgeOutcome::Failure
    };

    // Record outcome for each knowledge entry used in generation
    for knowledge_id in &trace.pipeline_context.knowledge_ids {
        feedback_service.record_knowledge_outcome(knowledge_id, outcome)?;
    }

    // Record outcome for each prompt section used
    for section_id in &trace.pipeline_context.prompt_section_ids {
        feedback_service.record_section_outcome(section_id, outcome)?;
    }

    Ok(())
}
```

### 5.3 Integration with PlaybookStore

The existing `PlaybookStore` at `crates/roko-learn/src/playbook.rs` captures
successful action sequences. The flywheel creates new playbooks from
evaluation-verified successful patterns:

```rust
/// When a curriculum task succeeds, extract the agent's action
/// sequence as a new playbook entry.
pub fn promote_curriculum_to_playbook(
    task: &CurriculumTask,
    episode: &Episode,
    playbook_store: &PlaybookStore,
) -> Result<Playbook, PlaybookError> {
    let playbook = extract_playbook_from_episode(
        &task.id,
        &task.prompt,
        &episode.tool_calls,
    )?;
    playbook_store.save(&playbook)?;
    Ok(playbook)
}
```

### 5.4 Integration with Pattern Discovery

The existing `PatternMiner` and `CrossEpisodeConsolidator` from
`crates/roko-learn/src/pattern_discovery.rs` discover recurring patterns
across episodes. The flywheel feeds evaluation traces as additional episodes,
enabling cross-evaluation pattern discovery.

### 5.5 Closed-Loop Model Selection

The cascade router learns from evaluation outcomes. When a model consistently
produces artifacts that fail evaluation:

1. The `compute_routing_reward_v2` function incorporates eval pass/fail.
2. The UCB1 stage deprioritizes the failing model.
3. The confidence stage updates empirical pass rates.
4. The static stage is not affected (hardcoded baseline).

This creates a closed loop: poor model performance -> lower routing priority ->
fewer assignments -> system quality improves.

---

## 6. Learning Event Schema

### 6.1 Unified Event Types

All flywheel events share a common envelope:

```rust
/// File: crates/roko-eval/src/events.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelEvent {
    pub id: String,
    pub timestamp: String,
    pub event_type: FlywheelEventType,
    pub trace_id: Option<String>,
    pub task_id: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlywheelEventType {
    TraceEmitted,
    AutoGradeCompleted,
    PreferenceMined,
    PatternExtracted,
    CurriculumGenerated,
    ExperimentCreated,
    PostFixerTrained,
    CanaryEvaluated,
    AnchorRotated,
    DriftDetected,
}
```

### 6.2 Integration with RuntimeEvent

Flywheel events are emitted through the existing runtime event bus at
`crates/roko-core/src/runtime_event.rs`. This allows the TUI dashboard,
HTTP SSE endpoint, and WebSocket listeners to observe flywheel activity.

---

## 7. Data Flywheel Economics

### 7.1 Cost Per Flywheel Cycle

| Step | Per-Evaluation Cost | Frequency | Monthly Cost (100 evals/day) |
|---|---|---|---|
| Trace capture | ~$0.00 | Every eval | $0 |
| Auto-grade | ~$0.78 | Every eval | $2,340 |
| Preference mining | ~$0.00 | Every eval | $0 |
| Pattern extraction | ~$0.50 | Nightly | $15 |
| Curriculum gen | ~$1.00 | Nightly | $30 |
| MIPROv2 | ~$50.00 | Weekly | $200 |
| RFT training | ~$500 | Monthly | $500 |
| **Total** | | | **~$3,085/month** |

### 7.2 Expected Savings

Based on v0's published metrics and internal projections:

| Improvement | Monthly Savings | Source |
|---|---|---|
| 20% fewer retries (post-fixer) | ~$4,000 | Frontier API cost reduction |
| 10% better model routing | ~$1,500 | Cheaper models for easy tasks |
| 15% faster convergence | ~$800 | Fewer iterations per task |
| **Total savings** | **~$6,300/month** | |

The flywheel is ROI-positive by month 3 and compounds thereafter.

---

## 8. Implementation Plan

### Phase 1: Trace Capture + Auto-Grade (Weeks 1-3)

| File | What |
|---|---|
| `crates/roko-eval/src/trace.rs` | EvalTrace type, PipelineContext |
| `crates/roko-eval/src/auto_grade.rs` | Auto-grading with panel + deterministic |
| `crates/roko-eval/src/feedback_bridge.rs` | Bridge to existing FeedbackService |
| `crates/roko-eval/src/events.rs` | FlywheelEvent types |

### Phase 2: Preference Mining + Pattern Extraction (Weeks 3-5)

| File | What |
|---|---|
| `crates/roko-eval/src/preference.rs` | PreferenceTriple, mining logic |
| `crates/roko-eval/src/pattern_library.rs` | PatternEntry, extraction |
| `crates/roko-eval/src/neuro_bridge.rs` | Pattern -> neuro engram promotion |

### Phase 3: Curriculum + MIPROv2 (Weeks 5-8)

| File | What |
|---|---|
| `crates/roko-eval/src/curriculum.rs` | CurriculumTask, failure clustering |
| `crates/roko-eval/src/pipeline_arm.rs` | PipelineArm bandit integration |
| `crates/roko-eval/src/miprov2.rs` | Weekly optimization orchestration |

### Phase 4: Post-Processor + Anti-Goodhart (Weeks 8-12)

| File | What |
|---|---|
| `crates/roko-eval/src/post_processor.rs` | RFT training spec, data curation |
| `crates/roko-eval/src/canary.rs` | Canary set, Spearman rho tracking |
| `crates/roko-eval/src/anti_goodhart.rs` | Rubric rotation, drift detection |

### Integration Points with Existing Crates

| Existing Crate | File | Integration |
|---|---|---|
| `roko-learn` | `episode_logger.rs` | Cross-reference via `EPISODE_TRACE_ID_KEY` |
| `roko-learn` | `cascade_router.rs` | Pipeline arm features extend routing context |
| `roko-learn` | `prompt_experiment.rs` | MIPROv2 creates experiments, auto-grade closes loop |
| `roko-learn` | `feedback_service.rs` | Eval verdicts -> KnowledgeOutcome |
| `roko-learn` | `playbook.rs` | Curriculum success -> new playbooks |
| `roko-learn` | `pattern_discovery.rs` | Eval traces as additional episodes |
| `roko-learn` | `post_gate_reflection.rs` | Reflections enrich curriculum clusters |
| `roko-learn` | `skill_library.rs` | Skill templates -> RFT training data |
| `roko-learn` | `drift.rs` | Canary correlation as drift signal |
| `roko-gate` | `adaptive_threshold.rs` | Panel results feed rung EMA |
| `roko-gate` | `llm_judge_gate.rs` | Panel implements JudgeOracle |
| `roko-neuro` | `knowledge_store.rs` | Patterns stored as engrams with tier progression |
| `roko-core` | `runtime_event.rs` | Flywheel events on the event bus |

---

## 9. Open Questions

1. **Training infrastructure**: RFT training requires GPU infrastructure. Do we
   self-host or use Fireworks/Together APIs?

2. **Pattern library size**: how many patterns before retrieval becomes a
   bottleneck? Should we implement approximate nearest neighbor (HNSW)?

3. **Human review queue**: who reviews flagged evaluations? Is this a
   standalone UI or integrated into the existing TUI dashboard?

4. **Cross-project learning**: should the flywheel operate per-project or
   across all projects using roko? Per-project is safer but slower to converge.

5. **Evaluation freshness**: how quickly should old traces be decayed?
   The neuro store has natural decay, but the raw JSONL logs do not.
