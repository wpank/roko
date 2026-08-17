# PRD-04 — Judge Methodology: Pairwise Bradley-Terry, Disjoint Panels, and Goodhart Resistance

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Crate**: `roko-eval-judge` (new, depends on `roko-eval`)
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions)

---

## 0. Scope

This document specifies how LLM judges evaluate artifacts in the unified evaluation
framework. It covers the statistical model (Bradley-Terry MLE with Davidson ties), the
mandatory composition rules (disjoint-family panel, position-swap-and-discard), the
sampling strategy, aggregation, prompt design, rubric definitions, gate composition,
control-theory retry primitives, and the six anti-Goodhart safeguards.

The single most important architectural commitment: **pairwise comparison aggregated via
Bradley-Terry against a fixed anchor, not absolute Likert scoring.** Absolute scoring
is a fallback for bootstrapping only.

Everything in this document maps to types and traits defined in PRD-01. Judge panels are
`CriterionKind::JudgePanel` criteria that consume `EvidenceBag` entries (screenshots,
DOM snapshots, diffs) and produce `CriterionResult` with grounded `Finding`s.

---

## 1. Why Pairwise, Not Absolute

### 1.1 The Empirical Case

**MLLM-as-a-Judge** (Chen et al., ICML 2024 Oral, arXiv 2402.04788) is decisive on the
question of scoring mode. Their benchmark of GPT-4V, Gemini-1.5-Pro, Qwen-VL-Max, and
LLaVA-1.6 across 12 visual understanding tasks shows:

| Scoring Mode | Human Agreement | Notes |
|---|---|---|
| **Pairwise** | ~0.60-0.70 | Consistent across all model families |
| **Absolute (Likert)** | Pearson ~0.49 | Barely above chance for fine-grained distinctions |
| **Batch ranking** | Significant divergence | Models disagree on ordering of 4+ items |

Zheng et al. (NeurIPS 2024, arXiv 2306.05685, MT-Bench/Chatbot Arena) reach the same
conclusion for text: pairwise judgments from GPT-4 agree with humans >80% of the time,
while absolute ratings show systematic calibration drift across sessions.

### 1.2 Three Structural Advantages

**Advantage 1: Lower variance per comparison.** In absolute scoring, the model must
simultaneously estimate quality and map it to a fixed scale. In pairwise comparison,
the only question is ordinal: A > B, B > A, or A = B. Scale-mapping variance is
eliminated entirely.

**Advantage 2: Harder to Goodhart.** Under absolute scoring, an optimizing agent can
learn the target number and generate artifacts that score well on the proxy without
genuinely improving quality (Skalse et al., ICML 2022). With pairwise comparison
against a fixed anchor, the agent must demonstrably outperform a concrete baseline,
and the baseline itself can rotate (Section 3).

**Advantage 3: Composes natively with bandits.** LinUCB contextual bandits and
Thompson sampling consume preference signals (A > B given context x), not absolute
scores. Pairwise judgments feed directly into the self-improvement flywheel (PRD-05)
without lossy conversion.

### 1.3 Cost Mitigation

The cost of pairwise comparison is O(N^2) for N candidates. Three mitigations:

1. **Fixed anchor**: compare `new_candidate` vs `prev_best_release` only (Section 3).
   This reduces to O(1) comparisons per evaluation.
2. **Cheap absolute scores as coarse prior**: pre-filter a batch to top-K before
   running pairwise, reducing O(N^2) to O(N + K).
3. **Active sampling** (Chiang et al., Chatbot Arena, arXiv 2403.04132): concentrate
   comparisons among closely-rated candidates for ~40% fewer comparisons.

### 1.4 When to Fall Back to Absolute

Absolute scoring is used in exactly two cases:

1. **Bootstrapping**: No prior approved artifact exists. Use absolute rubric scoring
   to establish the first anchor. Once accepted, switch to pairwise permanently.
2. **No anchor available**: Anchor artifact is corrupted or inaccessible. Fall back
   to absolute scoring and emit a `Finding` noting the degraded mode.

---

## 2. Bradley-Terry MLE

### 2.1 The Model

Bradley-Terry (Bradley & Terry, 1952) finds skill parameters that maximize the
likelihood of observed pairwise outcomes. For candidates i and j:

```
P(i > j) = exp(theta_i) / (exp(theta_i) + exp(theta_j))
          = sigma(theta_i - theta_j)
```

Implementation: logistic regression with no intercept and high regularization C = 10^6.

```rust
/// BT MLE result for a set of pairwise comparisons.
///
/// File: crates/roko-eval-judge/src/bt_model.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtResult {
    /// Candidate ID -> Elo score (anchor = 1000).
    pub elo_scores: BTreeMap<String, f64>,
    /// BCa confidence intervals per candidate.
    pub confidence_intervals: BTreeMap<String, BtConfidenceInterval>,
    /// Davidson tie parameter (nu).
    pub tie_parameter: f64,
    /// Number of comparison triples used.
    pub comparison_count: u32,
    /// Log-likelihood of the fitted model.
    pub log_likelihood: f64,
}
```

### 2.2 Elo Scale Mapping

```
Elo_i = theta_i * 400 / ln(10)
```

A 400-point difference means the stronger candidate wins with probability ~0.909.

### 2.3 Davidson Model for Ties

The Davidson model (Davidson, 1970; arXiv 2412.18407) extends BT with a tie
parameter nu:

```
P(i > j) = exp(theta_i) / (exp(theta_i) + exp(theta_j) + nu * sqrt(exp(theta_i) * exp(theta_j)))
P(tie)   = nu * sqrt(...) / (same denominator)
```

When nu = 0, this recovers standard BT. Always use Davidson when any tie verdicts
are observed.

### 2.4 BCa Bootstrap Confidence Intervals

Every BT score must carry a confidence interval. BCa bootstrap (Efron, 1987) is
second-order accurate and handles skewed distributions:

1. Resample N comparison triples with replacement.
2. Fit BT MLE to bootstrap sample.
3. Repeat B = 1000 times.
4. Compute BCa-adjusted 95% CIs.

```rust
/// BCa bootstrap confidence interval for a BT Elo score.
///
/// File: crates/roko-eval-judge/src/bt_model.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BtConfidenceInterval {
    pub lower: f64,
    pub point: f64,
    pub upper: f64,
    pub n_bootstrap: u32,
    pub confidence: f64,
}
```

### 2.5 Minimum Sample Requirements

| Purpose | Minimum Comparisons | Source |
|---|---|---|
| Production pass/fail | 18 (3 judges x 2 positions x 3 runs) | API non-determinism |
| Stable Elo (+/- 50) | ~100 per candidate | Chatbot Arena empirics |
| Detecting 5% improvement | 200 scenarios x 5 runs | Princeton "AI Agents That Matter" |
| Bootstrapping first anchor | 5 absolute-score runs | Chen et al. recommended minimum |

---

## 3. The Fixed-Anchor Protocol

### 3.1 Core Rule

> Always compare `new_candidate` vs `prev_best_release`. Never score in isolation.

### 3.2 Anchor Selection

The anchor is the most recently human-approved artifact for this task/viewport/journey
combination. Approval means:

1. Human explicitly marked the artifact as "approved".
2. Artifact passed all hard gates AND panel AND was merged without revert.
3. Artifact was selected as winner in a manual arena.

```rust
/// File: crates/roko-eval-judge/src/anchor.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeAnchor {
    pub content_hash: ContentHash,
    pub established_at_ms: i64,
    pub provenance: AnchorProvenance,
    pub artifact: ArtifactRef,
    pub evidence_path: PathBuf,
    pub elo: f64,
    pub comparison_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorProvenance {
    HumanApproved { reviewer: String },
    GatePassed { merge_commit: String },
    ArenaWinner { arena_id: String },
    Bootstrapped,
}
```

### 3.3 Bootstrapping Protocol (No Prior Anchor)

When no prior approved artifact exists:

1. Run absolute rubric scoring with N=3 per judge, analyze-before-rate format.
2. If absolute score passes thresholds, candidate becomes anchor with
   `provenance: Bootstrapped`.
3. After 10 subsequent evaluations, auto-promote best-scoring candidate
   regardless of human approval (prevents weak bootstrap from persisting).

### 3.4 Anchor Rotation

Anchors rotate when:
- A human approves a new artifact (immediate).
- Anchor ages past `max_anchor_age_days` (default 30) and a candidate has >80%
  win rate over the last 20 evaluations.
- Anchor is invalidated (deleted, corrupted).

```rust
/// File: crates/roko-eval-judge/src/anchor.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorRotationConfig {
    #[serde(default = "default_30")]
    pub max_anchor_age_days: u32,
    #[serde(default = "default_080")]
    pub rotation_win_rate: f64,
    #[serde(default = "default_20")]
    pub rotation_min_evals: u32,
    #[serde(default = "default_10")]
    pub bootstrap_rotation_evals: u32,
}
```

---

## 4. Disjoint-Family Panel (Mandatory Composition)

### 4.1 The Self-Preference Problem

Wataoka et al. (NeurIPS Safe-GenAI 2024, arXiv 2410.21819) demonstrate that LLM
judges exhibit systematic self-preference: models rate their own outputs higher,
and this bias correlates with low perplexity-on-self. The mechanism is straightforward
-- models recognize their own stylistic patterns and conflate familiarity with quality.

A 2026 follow-up (arXiv 2604.22891) quantifies and proposes mitigation: self-preference
bias persists even across architectures within the same model family, making
cross-family diversity essential rather than merely useful.

Verga et al. (2024, arXiv 2404.18796, PoLL) show that a panel of diverse-family models
outperforms a single large judge at 7x lower cost.

### 4.2 The Critical Rule

> Never use the same model family as both generator and judge.

If the artifact was generated by Claude, exclude Claude from the judge panel.
This is non-negotiable.

### 4.3 Recommended Panel Composition

**Standard panel (3 judges):**

| Judge | Family | Role | Approx Cost |
|---|---|---|---|
| Claude Opus 4.6 | Anthropic | Closed frontier | ~$0.10/judgment |
| LLaVA-Critic-72B | LLaVA | Open multimodal | ~$0.02/judgment |
| Prometheus-Vision | KAIST | Rubric-conditioned | ~$0.01/judgment |

**Extended panel (5 judges, high-stakes):**

Add Gemini 2.5 Pro (Google, ~$0.08) and Qwen-VL-Max (Alibaba, ~$0.03).

### 4.4 Panel Construction Algorithm

```rust
/// File: crates/roko-eval-judge/src/panel.rs
///
/// Constructs a judge panel from available providers, excluding the
/// generator family. Returns Err if fewer than min_panel_size disjoint
/// families are available.
pub fn construct_panel(
    providers: &ProvidersConfig,
    generator_family: Option<&str>,
    config: &JudgePanelConfig,
) -> Result<Vec<JudgeSpec>, EvalError> {
    // 1. Collect available families, excluding generator.
    // 2. For each family, select strongest vision-capable model.
    // 3. Sort by priority: known high agreement > rubric-conditioned > open.
    // 4. Take top preferred_panel_size families.
    // 5. Error if < min_panel_size available.
    todo!()
}
```

### 4.5 Dynamic Family Exclusion with roko-learn Integration

The panel construction integrates with the existing cascade router in
`crates/roko-learn/src/cascade_router.rs`. When `CascadeRouter::select` returns
a model for generation, its family is recorded in the `AgentEfficiencyEvent`
and passed to the panel constructor as the exclusion family.

```rust
/// File: crates/roko-eval-judge/src/panel.rs
///
/// Wire into the existing AgentEfficiencyEvent from roko-learn.
/// The efficiency event records which model generated the artifact.
pub fn generator_family_from_efficiency_event(
    event: &AgentEfficiencyEvent,
    providers: &ProvidersConfig,
) -> Option<String> {
    let model_slug = &event.model;
    providers.family_for_model(model_slug)
}
```

---

## 5. Position Bias Mitigation (Mandatory)

### 5.1 The Problem

Wang et al. (ACL 2024, arXiv 2305.17926) and the systematic study at AACL-IJCNLP 2025
demonstrate that LLM judges exhibit position bias: verdicts depend on which artifact
is presented first. Position consistency drops below 0.5 when candidates are close
in quality.

Recent research confirms position bias is strongly affected by the quality gap between
solutions and weakly influenced by prompt component length. This means the bias is
worst precisely where accurate judgment matters most.

| Model Family | Position Bias Rate | Direction |
|---|---|---|
| GPT-4V | 15-25% | Favors position A |
| Gemini-1.5-Pro | 10-20% | Favors position A |
| Claude Opus | 8-15% | Slight position A |
| LLaVA-1.6 | 20-30% | Strong position A |
| Qwen-VL-Max | 12-22% | Favors position A |

### 5.2 Mandatory Swap-and-Discard Procedure

For **every** pairwise comparison by **every** judge:

1. Present `(A=candidate, B=anchor)`. Record verdict V_AB.
2. Present `(A=anchor, B=candidate)`. Record verdict V_BA.
3. Check consistency:
   - If both prefer the same underlying artifact regardless of position: keep.
   - If the verdict flipped with position: **discard** this judge's result.

```rust
/// File: crates/roko-eval-judge/src/position_swap.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSwapResult {
    pub judge: JudgeSpec,
    pub verdict_ab: PairwiseVerdict,
    pub verdict_ba: PairwiseVerdict,
    pub consistent: bool,
    pub effective_verdict: Option<PairwiseVerdict>,
}

impl PositionSwapResult {
    pub fn check_consistency(
        verdict_ab: PairwiseVerdict,
        verdict_ba: PairwiseVerdict,
    ) -> bool {
        match (verdict_ab, verdict_ba) {
            (PairwiseVerdict::PreferA, PairwiseVerdict::PreferB) => true,
            (PairwiseVerdict::PreferB, PairwiseVerdict::PreferA) => true,
            (PairwiseVerdict::Tie, PairwiseVerdict::Tie) => true,
            _ => false,
        }
    }
}
```

### 5.3 Advanced Mitigation: CalibraEval Debiasing

Beyond swap-and-discard, CalibraEval (2026) reformulates position bias as an
optimization problem over prediction distributions. The approach:

1. Collect raw pairwise verdicts across both orderings.
2. Model the position bias as a systematic shift in the logistic probability.
3. Solve for debiased skill parameters by jointly optimizing BT coefficients
   and a per-judge bias correction term.

This is implemented as a post-processing step on top of the standard BT MLE:

```rust
/// File: crates/roko-eval-judge/src/calibraeval.rs
///
/// CalibraEval debiasing: jointly estimate BT skill parameters
/// and per-judge position bias correction terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibraEvalResult {
    /// Debiased Elo scores.
    pub debiased_elo: BTreeMap<String, f64>,
    /// Estimated position bias per judge family.
    pub position_bias: BTreeMap<String, f64>,
    /// Improvement in log-likelihood vs uncorrected BT.
    pub ll_improvement: f64,
}
```

### 5.4 Intra-Pair Instability (IPI) Metric

From recent 2025 research, the IPI metric measures local pairwise consistency by
detecting positional bias at the individual comparison level. For each comparison
pair, IPI quantifies how much the verdict changes when the presentation order is
swapped:

```
IPI(pair) = |P(A>B | order=AB) - P(A>B | order=BA)|
```

Across a panel, aggregate IPI provides a single-number reliability score. Pairs
with IPI > 0.5 should be discarded. Pairs with IPI < 0.1 are highly reliable.

The panel-level IPI is tracked per evaluation and fed back to the learning
subsystem for judge selection optimization.

### 5.5 Cost Implications

Position swap doubles invocations. For a 3-judge panel with 3 runs per position:

```
18 invocations total = 3 judges x 3 runs x 2 positions
Cost: ~$0.78 per evaluation (Claude $0.60 + LLaVA $0.12 + Prometheus $0.06)
```

For a plan with 20 visual tasks x 2 viewports: ~$31.20 in judge costs.
This is a small fraction of agent generation cost.

---

## 6. Sampling Strategy

### 6.1 N=3 at T=0 Per Position

Per G-Eval methodology (Chiang & Lee, arXiv 2310.05657): N=5 at T=0 captures ~80%
of variance reduction vs N=20. We use N=3 as the practical minimum because:

1. At T=0, output is nearly deterministic. Residual API-level non-determinism
   introduces 1-3% variation from batching and kernel selection.
2. N=3 allows majority vote to overcome these artifacts.
3. With position swap, total per-judge invocations = 6, which is sufficient.

### 6.2 Adaptive Sampling Budget

When the panel disagrees (Section 7.3), increase the sampling budget adaptively:

```rust
/// File: crates/roko-eval-judge/src/sampling.rs
pub fn adaptive_sample_count(
    initial_agreement: f64,
    base_samples: u32,
    max_samples: u32,
) -> u32 {
    if initial_agreement >= 0.8 {
        base_samples // Panel agrees, no extra samples needed
    } else if initial_agreement >= 0.5 {
        (base_samples * 2).min(max_samples) // Moderate disagreement
    } else {
        max_samples // Severe disagreement, maximize samples
    }
}
```

---

## 7. Aggregation

### 7.1 Trimmed Mean (Default)

Aggregate panel scores using a trimmed mean with 10-20% trim. For a 3-judge panel,
no trimming occurs (too few scores). For a 5-judge panel with per-run scores
(5 x 3 = 15 scores), trim the single lowest and highest.

```rust
/// File: crates/roko-eval-judge/src/aggregation.rs
pub fn trimmed_mean(scores: &mut [f64], trim_fraction: f64) -> Option<f64> {
    if scores.is_empty() { return None; }
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = scores.len();
    let k = (trim_fraction * n as f64 / 2.0).floor() as usize;
    let trimmed = &scores[k..n - k];
    if trimmed.is_empty() { return None; }
    Some(trimmed.iter().sum::<f64>() / trimmed.len() as f64)
}
```

### 7.2 Learned Weights via Stacking

Once >= 500 human-rated canary examples accumulate, switch to learned weights:

1. Collect per-judge scores on the canary set.
2. Fit ridge regression: `human[i] ~ w_1 * scores_1[i] + ... + w_J * scores_J[i]`.
3. Learned weights replace uniform weights.

Stacking gains 3-8 correlation points over uniform trimmed mean (Verga et al., 2024).

```rust
/// File: crates/roko-eval-judge/src/aggregation.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedJudgeWeights {
    pub weights: BTreeMap<String, f64>,
    pub fit_at_ms: i64,
    pub n_canary: u32,
    pub r_squared: f64,
    pub active: bool,
}
```

### 7.3 Detecting Judge Disagreement

When judges disagree substantially, flag for human review:

1. **Agreement rate**: fraction of consistent judges agreeing on winner. < 0.5 = review.
2. **Score spread**: max - min > 0.3 on [0,1] scale = review.
3. **Krippendorff alpha**: alpha < 0.4 across all per-run verdicts = review.

```rust
/// File: crates/roko-eval-judge/src/disagreement.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelDisagreement {
    pub agreement_rate: f64,
    pub score_spread: f64,
    pub krippendorff_alpha: f64,
    pub needs_human_review: bool,
    pub reason: Option<String>,
}
```

### 7.4 Weak Total Order Violation (TOV)

Beyond local pairwise consistency, TOV (2025) assesses global logical coherence:
if A > B and B > C, does the judge also report A > C? Transitivity violations
indicate the judge is not reasoning about quality in a coherent manner.

TOV is computed across all evaluation traces within a session and tracked as a
panel health metric. High TOV (> 0.15) triggers panel recalibration.

---

## 8. Judge Prompt Design

### 8.1 Analyze-Before-Rate Format

From Zheng et al. (NeurIPS 2024, MT-Bench): prompts requiring detailed analysis
before verdict produce substantially more consistent judgments. The mechanism is
forcing the model to articulate reasoning before committing, reducing surface-level
heuristics (position, length, style).

All judge prompts use this format:
1. Analyze each artifact in detail (grounded observations with bounding boxes).
2. Compare on each rubric dimension.
3. Render verdict LAST.

### 8.2 UICrit Annotation Schema

From Liang et al. (ICML 2024, UICrit): bounding-box-grounded critiques increase
human agreement by ~15%. Every finding references a specific region:

- Visual: `[x, y, width, height]` normalized coordinates [0,1].
- Code: `file:line:col` source location.
- DOM: CSS selector identifying the element.

### 8.3 RISE-Judge Stepwise Analysis

From Yu et al. (2025, RISE-Judge): judgment is a general LLM competence, not a
task-specific skill. The RISE framework trains judges via two-stage approach:

1. **SFT stage**: Chain-of-thought analysis and verdict formats.
2. **DPO stage**: Pairwise discrimination power via explicit ranking objectives.

Key insight: ~40k high-quality synthetic examples (20k SFT, 20k DPO) are sufficient
to reach state-of-the-art on RewardBench. The data quality pipeline uses
LLM-in-the-loop synthesis with rigorous filtering for label consistency, order/length
bias minimization, and structured style.

We apply RISE-Judge principles to our prompt design: structured stepwise analysis
before any verdict, with explicit rubric dimension scoring as intermediate steps.

### 8.4 Visual Evaluation Prompt Template (Pairwise)

```
System:
You are a UI quality evaluator comparing two screenshots of a web interface.
Analyze both carefully, score on the rubric, ground findings with bounding boxes,
and determine which is better overall.

## Rubric (7 dimensions, score 0.0-1.0 each)

1. task_completion (0.25): Required workflow achieved, all elements present.
2. layout_integrity (0.20): No overlap, clipping, broken spacing.
3. responsive_quality (0.15): Correct viewport adaptation.
4. interaction_clarity (0.10): Controls identifiable, adequate targets.
5. visual_polish (0.10): Typography, spacing, hierarchy.
6. design_system_fit (0.10): Matches app conventions.
7. accessibility_affordance (0.10): Contrast, tap targets, labels.

## Instructions

1. Analyze Screenshot A (grounded observations with bounding boxes).
2. Analyze Screenshot B (same specificity).
3. Score each on all 7 dimensions.
4. List findings: severity, screenshot, bbox, problem, evidence, fix.
5. State preference: "A", "B", or "tie" (must follow from analysis).

## Output: valid JSON only (no markdown fences)
{ "analysis_a": "...", "analysis_b": "...",
  "rubric_a": {...}, "rubric_b": {...},
  "findings": [...],
  "preference": "A|B|tie", "confidence": 0.0,
  "reasoning": "..." }
```

### 8.5 Code Evaluation Prompt Template (Pairwise)

Analogous to visual, with 7 code-specific dimensions:
correctness (0.30), maintainability (0.20), safety (0.15), performance (0.10),
test_coverage (0.10), api_design (0.10), documentation (0.05).

Findings grounded with `file:line:col` locations and code snippets.

### 8.6 Prompt Design Principles

1. **Deterministic metrics as context, not judge**: APCA, token adherence, etc.
   are provided as context. Judge renders an independent subjective judgment.
2. **Structured JSON output only**: enables automated extraction and storage.
3. **Findings before verdict**: reduces hallucinated justifications.
4. **Normalized coordinates**: all bounding boxes use [0,1] for resolution independence.

---

## 9. The 7-Dimension Visual Rubric

### 9.1 Dimensions and Weights

| # | Dimension | Weight | What It Measures |
|---|---|---|---|
| 1 | task_completion | 0.25 | Required workflow visible, elements present |
| 2 | layout_integrity | 0.20 | No overlap, clipping, broken spacing |
| 3 | responsive_quality | 0.15 | Correct viewport adaptation |
| 4 | interaction_clarity | 0.10 | Controls identifiable, adequate targets |
| 5 | visual_polish | 0.10 | Typography, spacing, hierarchy, balance |
| 6 | design_system_fit | 0.10 | Matches existing app conventions |
| 7 | accessibility_affordance | 0.10 | Contrast, tap targets, labels, focus indicators |

### 9.2 Weight Customization

Weights are configurable per evaluation profile. A design-system-heavy team might
increase `design_system_fit` to 0.25 and reduce others. The marketplace (PRD-06)
lets users publish domain-specific weight configurations.

### 9.3 Absolute Scoring Thresholds (Bootstrapping Only)

When absolute scoring is used for bootstrapping:

| Dimension | Minimum Score | Rationale |
|---|---|---|
| task_completion | 0.6 | Must achieve core workflow |
| layout_integrity | 0.5 | No critical layout breaks |
| responsive_quality | 0.4 | Basic viewport adaptation |
| interaction_clarity | 0.4 | Controls must be findable |
| visual_polish | 0.3 | Relaxed for bootstrapping |
| design_system_fit | 0.3 | Relaxed for bootstrapping |
| accessibility_affordance | 0.5 | Non-negotiable floor |

---

## 10. Integration with Existing Gate Pipeline

### 10.1 Mapping to roko-gate

The judge methodology integrates into the existing 7-rung gate pipeline at
`crates/roko-gate/src/gate_pipeline.rs`. The LLM judge gate at
`crates/roko-gate/src/llm_judge_gate.rs` provides the `JudgeOracle` trait
that the panel-based judge implements.

```rust
/// File: crates/roko-eval-judge/src/gate_adapter.rs
///
/// Adapts the full panel-based judge to the existing JudgeOracle trait
/// from roko-gate, allowing panel evaluation to plug into the gate pipeline.
pub struct PanelJudgeOracle {
    panel_config: JudgePanelConfig,
    anchor_store: Arc<AnchorStore>,
    providers: ProvidersConfig,
}

#[async_trait]
impl JudgeOracle for PanelJudgeOracle {
    async fn judge(&self, prompt: &str) -> Result<f32, String> {
        // 1. Parse JudgePayload from prompt.
        // 2. Construct panel excluding generator family.
        // 3. Run pairwise evaluation against anchor.
        // 4. Aggregate with trimmed mean.
        // 5. Return normalized score.
        todo!()
    }
}
```

### 10.2 Rung Placement

The panel-based judge replaces the current single-model LLM judge at Rung 5.
The existing `LlmJudgeGate` remains available as a fast/cheap fallback for
low-stakes evaluations.

| Rung | Gate | Integration |
|---|---|---|
| 1 | Compile | Unchanged |
| 2 | Format (rustfmt/prettier) | Unchanged |
| 3 | Lint (clippy) | Unchanged |
| 4 | Test | Unchanged |
| 5 | **Panel Judge** (new) | Replaces single LLM judge |
| 6 | Security scan | Unchanged |
| 7 | Diff review | Unchanged |

### 10.3 Adaptive Thresholds Integration

The existing adaptive threshold system at
`crates/roko-gate/src/adaptive_threshold.rs` tracks per-rung pass rates via EMA.
The panel judge integrates by reporting its aggregate score as the rung-5 observation:

```rust
/// File: crates/roko-eval-judge/src/gate_adapter.rs
///
/// Report panel evaluation outcome to adaptive thresholds.
pub fn report_to_adaptive_thresholds(
    thresholds: &mut AdaptiveGateThresholds,
    panel_result: &PanelEvaluationResult,
) {
    let passed = panel_result.aggregate_score >= panel_result.threshold;
    thresholds.observe("llm_judge_panel", passed);
}
```

---

## 11. Novel Techniques: Confidence-Aware Judging

### 11.1 Trust or Escalate (ICLR 2025)

The "Trust or Escalate" framework provides LLM judges with calibrated abstention:
when the judge's confidence in its verdict falls below a threshold, it escalates
to human review rather than producing a low-confidence judgment.

Implementation: the judge's output includes a `confidence` field (0.0-1.0). Verdicts
with confidence below 0.6 are automatically routed to the human review queue.

### 11.2 JudgeBench Calibration

JudgeBench provides a benchmark for evaluating judge capability through human
agreement. We use JudgeBench-style calibration sets to validate our panel before
deployment:

1. Run the panel on a held-out set of human-rated comparisons.
2. Compute agreement rate per judge and per panel aggregate.
3. Only deploy panel configurations with agreement rate >= 0.65.
4. Track agreement rate over time as a drift detector.

### 11.3 Scoring Bias Awareness

Recent 2026 research (arXiv 2506.22316) shows that scoring stability is disrupted
by biases related to score rubrics, score IDs, and reference answer selection.
Countermeasures:

1. **Rubric phrasing rotation**: vary the phrasing of rubric anchors across runs.
2. **Score ID randomization**: avoid sequential score IDs that create ordinal bias.
3. **Reference independence**: never include a reference "correct answer" in the
   judge prompt; let the judge evaluate on merit, not proximity to a reference.

---

## 12. Anti-Goodhart Safeguards

Six operational tactics against the four Goodhart types (Manheim & Garrabrant 2018):

### 12.1 Disjoint-Family Panel with Trimmed Mean

Inner-loop bandit reward and held-out validation never share a judge. Panel diversity
prevents any single model's biases from dominating.

### 12.2 Frozen Human-Rated Canary Set

200-500 prompts with UICrit-style ratings (Krippendorff alpha >= 0.8). Re-evaluated
every release. If panel score rises but canary score does not, Goodharting detected.

### 12.3 Quarterly Rubric Rotation

Goodhart relies on a stable target. Rotate rubric emphasis quarterly. Per LiveBench
(White et al., ICLR 2025): monthly updates with fresh questions prevent contamination.

### 12.4 Adversarial Red-Team Eval

Test for gameable patterns: gradient bombs, oversaturated heroes, blur-everything,
copy-paste of approved screenshots, token-stuffing.

### 12.5 Reference-Conditioned BT Against Strong Baseline

Always pairwise against a strong anchor. BT against anchor is harder to inflate
because the anchor provides a moving target as the baseline improves.

### 12.6 Canary Correlation Monitor (Spearman rho)

Track Spearman correlation between inner-loop judge and expensive canary evaluation.
If rho drops below ~0.6, the inner judge has drifted and needs recalibration.

### 12.7 Preference As Reward (PAR) Integration

Recent 2025-2026 work on PAR achieves 5+ percentage point win rate improvements
over competing approaches by using pairwise preferences directly as reward signals
rather than training a separate scalar reward model. This eliminates a layer of
approximation that is vulnerable to reward hacking.

In our architecture, judge panel verdicts ARE the reward signal. No intermediate
reward model is trained. This is architecturally aligned with PAR.

---

## 13. Implementation Plan

### Phase 1: Core BT Model and Panel (Weeks 1-3)

| File | What |
|---|---|
| `crates/roko-eval-judge/src/lib.rs` | Crate root, re-exports |
| `crates/roko-eval-judge/src/bt_model.rs` | BT MLE, Davidson ties, BCa bootstrap |
| `crates/roko-eval-judge/src/panel.rs` | Panel construction, family exclusion |
| `crates/roko-eval-judge/src/position_swap.rs` | Swap-and-discard, IPI metric |
| `crates/roko-eval-judge/src/anchor.rs` | Anchor store, rotation, bootstrapping |
| `crates/roko-eval-judge/src/aggregation.rs` | Trimmed mean, learned weights |
| `crates/roko-eval-judge/src/sampling.rs` | Adaptive sampling budget |

### Phase 2: Prompt Templates and Rubrics (Weeks 3-5)

| File | What |
|---|---|
| `crates/roko-eval-judge/src/prompts/visual.rs` | Visual evaluation prompt template |
| `crates/roko-eval-judge/src/prompts/code.rs` | Code evaluation prompt template |
| `crates/roko-eval-judge/src/rubric.rs` | Rubric definition, weight customization |
| `crates/roko-eval-judge/src/findings.rs` | Finding types, grounding, severity |

### Phase 3: Gate Integration (Weeks 5-7)

| File | What |
|---|---|
| `crates/roko-eval-judge/src/gate_adapter.rs` | PanelJudgeOracle implementing JudgeOracle |
| `crates/roko-eval-judge/src/disagreement.rs` | Disagreement detection, human review escalation |
| `crates/roko-eval-judge/src/calibraeval.rs` | CalibraEval debiasing post-processing |

### Phase 4: Anti-Goodhart and Monitoring (Weeks 7-10)

| File | What |
|---|---|
| `crates/roko-eval-judge/src/canary.rs` | Canary set management, Spearman rho tracking |
| `crates/roko-eval-judge/src/anti_goodhart.rs` | Rubric rotation, drift detection |
| `crates/roko-eval-judge/src/metrics.rs` | TOV, IPI, panel health metrics |

### Integration Points with Existing Crates

| Existing Crate | Integration |
|---|---|
| `roko-gate` (`llm_judge_gate.rs`) | `PanelJudgeOracle` implements `JudgeOracle` trait |
| `roko-gate` (`adaptive_threshold.rs`) | Panel results feed rung-5 EMA observations |
| `roko-learn` (`cascade_router.rs`) | Generator family from `CascadeRouter::select` |
| `roko-learn` (`episode_logger.rs`) | Panel verdicts recorded in Episode's `gate_verdicts` |
| `roko-learn` (`feedback_service.rs`) | Panel outcomes feed `KnowledgeOutcome` scoring |
| `roko-learn` (`prompt_experiment.rs`) | Rubric rotation tracked as experiments |
| `roko-neuro` (`knowledge_store.rs`) | Canary set entries stored as knowledge engrams |

---

## 14. Open Questions

1. **Cost ceiling**: at $0.78/evaluation, 100 evaluations/day = $78/day. Is this
   acceptable, or do we need a cheaper fast-path for development iterations?

2. **Judge model versioning**: when a judge model is updated (e.g., Claude Opus 4 to
   Opus 5), historical Elo scores may not be comparable. Should we re-baseline?

3. **Visual embedding**: should we add CLIP/SigLIP visual embeddings as a fast
   pre-filter before running the full judge panel?

4. **Multi-turn evaluation**: the current framework evaluates single artifacts. How
   should we evaluate multi-turn agent interactions where quality emerges over a
   conversation?

5. **Judge fine-tuning**: should we fine-tune an open-weight judge (LLaVA-Critic)
   on our domain-specific canary set, or keep judges frozen to maintain independence?
