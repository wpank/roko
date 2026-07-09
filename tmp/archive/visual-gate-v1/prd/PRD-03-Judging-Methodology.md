# PRD-03: Judging Methodology — Pairwise Bradley-Terry, Disjoint Panels, and Statistical Rigor

**Prerequisites**: PRD-00 (research foundations), PRD-01 (data model), PRD-02 (metrics).

---

## 1. Overview

This document specifies the visual judging system. The single most important architectural commitment per the research: **pairwise comparison aggregated via Bradley-Terry against a fixed anchor, not absolute Likert scoring.** Run by a panel of disjoint-family judges with mandatory (A,B)/(B,A) swap-and-discard, evaluated against a frozen human canary set with Krippendorff α monitoring and quarterly rubric rotation.

---

## 2. Why Pairwise, Not Absolute

**MLLM-as-a-Judge** (Chen et al., ICML 2024 Oral, arXiv 2402.04788) is decisive:
- GPT-4V/Gemini/Qwen-VL/LLaVA-1.6 align with humans on pairwise at ~0.6–0.7+ agreement.
- On absolute scoring: Pearson ~0.49 — barely above chance for fine-grained distinctions.
- On batch ranking: significant divergence.

Pairwise has three structural advantages:
1. **Lower variance per comparison.** No fixed scale to anchor on.
2. **Harder to Goodhart.** The agent cannot learn a single target number to imitate.
3. **Composes natively with bandits.** TensorZero/LinUCB fundamentally consume preference signals, not absolute scores.

The cost is O(N²) comparisons. Mitigate via:
- **Active sampling** (Chiang et al., Chatbot Arena, arXiv 2403.04132): information-gain sampling concentrates comparisons among closely-rated candidates.
- **Cheap absolute scores as coarse prior**: pre-filter to top-K before pairwise.
- **Always compare new vs fixed anchor**: new_candidate vs prev_best_release, never score in isolation.

---

## 3. Bradley-Terry Aggregation

### 3.1 The Model

Bradley-Terry (BT) is the maximum likelihood estimator under static-skill assumption. Chatbot Arena switched from online Elo to BT MLE in 2024 because BT is statistically cleaner.

**Implementation**: Logistic regression over `(model_a, model_b, win)` triples with no intercept and high regularization C. Map coefficients to Elo scale via `coef × 400 / ln(10)`.

```python
# Pseudocode for BT MLE
from sklearn.linear_model import LogisticRegression

# wins[i] = 1 if candidate i was preferred, 0 otherwise
# features: one-hot encoding of (candidate_a, candidate_b) pairs
X, y = encode_pairwise_results(comparison_results)
model = LogisticRegression(C=1e6, fit_intercept=False)
model.fit(X, y)
bt_scores = model.coef_[0] * 400 / np.log(10)  # Elo scale
```

### 3.2 Handling Ties

Use the **Davidson model** (arXiv 2412.18407) when judges return ties. The Davidson extension adds a tie parameter to the BT likelihood.

### 3.3 Bootstrap Confidence Intervals

Use **BCa bootstrap** (Efron 1987) — second-order accurate, handles skewed aesthetic scores better than percentile bootstrap. Compute 95% CI on the BT score.

### 3.4 The Fixed Anchor

Always evaluate `new_candidate vs prev_best_release` against a fixed anchor. Never score in isolation. BT against an anchor is harder to inflate than absolute scoring and gives progress in interpretable Elo points.

The anchor is the most recent human-approved screenshot for this task/viewport/journey combination. If no anchor exists (first run), use the task's reference screenshot or skip pairwise and fall back to absolute rubric scoring for bootstrapping.

---

## 4. Disjoint-Family Judge Panel

### 4.1 Composition

**Source**: PoLL (Verga et al., arXiv 2404.18796) — a panel of smaller diverse-family models outperforms a single large judge at 7× lower cost.

**Mandatory composition** (3–4 judges from disjoint families):

| Judge | Family | Role | Cost | Source |
|---|---|---|---|---|
| Claude Opus 4.6 | Anthropic | Closed frontier | ~$0.10/judgment | API |
| LLaVA-Critic-72B | LLaVA | Open multimodal critic | ~$0.02/judgment | Self-hosted or API. Fine-tuned on 113k pointwise+pairwise judgments, matches GPT-4o alignment. |
| Prometheus-Vision | KAIST | Rubric-conditioned specialist | ~$0.01/judgment | Self-hosted. Pearson ~0.5–0.6 with humans at fraction of API cost. |

**Critical rule**: Never use the same model family as both generator and judge. Self-preference is well-documented (Wataoka et al., NeurIPS Safe-GenAI WS 2024, arXiv 2410.21819) — correlates with low perplexity-on-self.

### 4.2 Aggregation

**Trimmed mean** (10–20% trim) across panel scores. Robust to one judge going off-rails while using more information than median.

Once ≥500 human-rated examples accumulate, switch to **learned weights via stacking** on held-out human labels — gains another 3–8 correlation points.

### 4.3 Position Bias Mitigation

**Source**: Wang et al. (ACL 2024, arXiv 2305.17926). Position consistency drops below 0.5 with 3–4 candidates when quality gaps are small.

**Mandatory procedure**:
1. Present (A=candidate, B=anchor) to all judges.
2. Present (B=candidate, A=anchor) to all judges (position swap).
3. For each judge, if the verdict changed with the swap, discard that judge's result for this comparison.
4. The 2× cost is non-negotiable.

### 4.4 Sampling

Per the G-Eval methodology (adapted): N=5 samples per judgment at T=0. This captures ~80% of N=20's variance reduction (Chiang & Lee, arXiv 2310.05657; Chen et al. 2024). Use majority vote or median across samples. Always force analyze-before-rate output format (reasoning before score, not score-only).

Run each judgment ≥3× even at T=0 — API non-determinism produces 1–3% disagreement on identical inputs.

---

## 5. Judge Prompt Template

Based on UICrit (Duan et al., UIST 2024) annotation schema — bounding-box-grounded critiques with multi-axis ratings.

```
System:
You are a UI quality evaluator. You will compare two screenshots of a web interface.
Analyze each screenshot carefully, then determine which is better.

For each screenshot, evaluate on these dimensions (0.0 to 1.0):
1. task_completion (weight 0.25): Is the required workflow visibly achieved?
2. layout_integrity (weight 0.20): No overlap, clipping, broken spacing, bad alignment?
3. responsive_quality (weight 0.15): Works well at this viewport size?
4. interaction_clarity (weight 0.10): Controls, focus, states are clear?
5. visual_polish (weight 0.10): Typography, spacing, hierarchy, balance?
6. design_system_fit (weight 0.10): Fits existing app conventions?
7. accessibility_affordance (weight 0.10): Contrast, tap targets, labels?

First, analyze both screenshots in detail.
Then, for each finding, provide a bounding box [x, y, width, height] grounding it.
Finally, state your preference: "A", "B", or "tie".

Return ONLY valid JSON:
{
  "analysis_a": "...",
  "analysis_b": "...",
  "rubric_a": { "task_completion": ..., ... },
  "rubric_b": { "task_completion": ..., ... },
  "findings": [
    { "severity": "high", "area": "modal footer", "bbox": [x, y, w, h],
      "problem": "...", "evidence": "...", "suggested_fix": "..." }
  ],
  "preference": "A" | "B" | "tie",
  "confidence": 0.0-1.0,
  "reasoning": "..."
}

User:
Task: {task_id} — {task_title}
Visual goal: {visual_goal}
Viewport: {viewport.name} ({width}×{height})

Computational metrics summary:
- Token adherence: {score}
- APCA pass rate: {score}
- Element density: {count}/{threshold}
- Grid adherence: {score}
- Colorfulness: {M} (optimal: 15-35)

Hard gate results: {summary}

Screenshot A:
[Image: screenshot_a]

Screenshot B:
[Image: screenshot_b]
```

---

## 6. Gate Composition: Conjunctive Hard, Pareto Soft

### 6.1 The Principle

**Source**: Moskovitz et al. (ICLR 2024) — constrained RLHF with per-RM thresholds outperforms weighted-sum because correlated proxy RMs amplify Goodhart.

**Translation**: A strong saliency score must not compensate for an APCA failure. Each dimension must independently pass its threshold.

### 6.2 Hard Gates (Conjunctive — ALL must pass)

All five tiers from PRD-00 Section 3.1. Any single tier failure blocks the gate regardless of other scores.

### 6.3 Soft Gates (Pareto Frontier)

Soft metrics (colorfulness, element density, text/whitespace ratio, visual balance, saliency, layout pattern, LPIPS, AIM metrics, judge panel score) contribute to a Pareto frontier. The controller is a multi-armed bandit (Thompson sampling over `(loop, fix-prompt-template)` posteriors).

**Not weighted-sum.** Each soft metric has an independent threshold band. A UI is on the Pareto frontier if no single metric is below its floor AND the overall panel score exceeds the target.

### 6.4 Control-Theory Primitives for Retry Policy

**Dead-band**: Don't retry if all soft scores within ε of target. Prevents oscillation on marginal improvements. ε per metric: 2 ΔE for color, 0.05 for density, 0.03 for saliency.

**Hysteresis**: Require 2 consecutive passes to accept, 1 strong fail to reject. Prevents flapping under stochastic judge noise.

**Anti-windup**: Cap retries to 4 per loop and 12 per session. If cumulative error fails to decrease over 2 retries on the same loop, switch controller — escalate prompt template, swap models, or accept-with-warning.

**Derivative term**: Track Δscore between renders. If Δ<0 (regression), revert to previous candidate.

**Feedforward**: Run static lints (token usage, contrast on specified colors) on generated source pre-render. Short-circuit obvious failures cheaply before launching the browser.

---

## 7. Statistical Rigor Requirements

### 7.1 Minimum Runs

| Purpose | Runs | Source |
|---|---|---|
| Production judgment | ≥3 per judge | API non-determinism |
| Optimization experiment | ≥5 per configuration | Princeton "AI Agents That Matter" |
| Detecting 5% improvement with 80% power, medium variance | 200 scenarios × 5 runs | Power analysis table (doc-17) |
| Canary set calibration | ≥200 human-rated examples | Chatbot Arena empirics (~400 for ±10 Elo) |

### 7.2 Significance Testing

For deciding whether a pipeline change is real:
1. **Paired-by-prompt design**: Same prompts, old vs new pipeline.
2. **BCa bootstrap** confidence intervals.
3. **Sign-flip permutation test**: p < 0.05.
4. Claim significance only if BCa excludes 0 AND permutation p < 0.05.

For multi-dimensional rubric:
- **Benjamini-Hochberg FDR** for exploratory dimension-by-dimension analysis.
- **Bonferroni** for the headline metric.

### 7.3 Clustered Standard Errors

Per Evan Miller (Anthropic, 2024): clustered standard errors can be 3× larger than naive estimates. Cluster by task type, viewport, or journey when computing CIs.

### 7.4 Canary Set Protocol

**Frozen human-rated canary set** of 200–500 prompts with UICrit-style ratings (α ≥ 0.8). Re-evaluated every release.

Monitoring:
- If panel score on production data rises but canary score doesn't → Goodharting detected.
- If Spearman correlation between cheap inner-loop judge and expensive canary drops below ~0.6 → inner judge has drifted, retrain or replace.

Maintain **Krakovna's specification-gaming list** as continuously-updated red-team prompt set. Every quarter, run agents adversarially against current eval and add new gaming patterns.

---

## 8. Retry Feedback Format

When gate fails, emit structured feedback. Quality determines whether agent can fix the problem.

### 8.1 Template

```markdown
## UI Gate Failure (attempt {n}/{max})

Task: {task_id} {task_title}

### Hard gate failures (Tiers 1–4)
{for each tier failure:}
- **Tier {n} ({name})**: {viewport}/{journey}: {description}
  Rule: {rule_id} | WCAG: {wcag_sc} | Tool: {source_tool}
  Fix hint: {fix_hint}

### Computational metric violations
{for each metric outside threshold:}
- **{metric_name}**: {actual_value} (threshold: {threshold})
  {specific elements if available}

### Visual judge findings
{for each finding from panel, top 3:}
- **{severity}** ({dimension}): {area} [{bbox}]
  Problem: {problem}
  Evidence: {evidence}
  Fix: {suggested_fix}
  Screenshot: {path}

### Panel score
{bt_score} Elo (threshold: {threshold})
Preferred over anchor: {yes/no} ({judges_preferring}/{total_judges})

### Browser evidence
- Console errors: {count}
- Failed requests: {count} ({top 3})
- Layout overflow: {yes/no}
- Token adherence: {score}
- APCA violations: {count}

### What to fix
{1–3 concrete, actionable items. Hard failures first. Reference specific selectors.}
```

### 8.2 Context Placement

Per Liu et al. "Lost in the Middle" (TACL 2024): models perform best when relevant information is at beginning or end. Put "what to fix" at the BEGINNING of the retry prompt. Evidence in the middle. Unchanged task context at the end.

Per Chroma "Context Rot" (July 2025): keep feedback concise. Max 3 findings. Do not dump entire result.json.

---

## 9. Verdict Score Normalization

```
if infrastructure_failed:
    score = 0.0, passed = false
elif any hard gate tier failed:
    score = min(0.49, hard_pass_ratio × 0.49), passed = false
elif panel_result.passed == false:
    score = panel_bt_score_normalized, passed = false
else:
    score = panel_bt_score_normalized, passed = true
```

BT score normalized: map from Elo range to 0.0–1.0 using the anchor as 0.5 baseline.

---

## 10. Design2Code Floor Metrics

**Source**: Design2Code (Si et al., Stanford, arXiv 2403.03163).

When reference screenshots are provided, compute these automatic floor metrics before the judge panel runs:

- **Block-Match**: Visual block detection via edge-based segmentation, Jonker-Volgenant assignment between candidate and reference blocks, area-overlap score.
- **Position similarity**: 1 − normalized centroid distance between matched blocks. **Most robust signal** — hard to fake without actually placing elements correctly.
- **Text similarity**: OCR both screenshots, compare extracted text. **Robust if OCR-based**, not DOM-based.
- **Color similarity**: CIEDE2000 mean color per matched block.
- **CLIP cosine**: Sanity check only, never primary reward. **Most gameable** — right gestalt color/density wins with wrong content.

**Defensible auto-score**: geometric mean of Block-Match + Position + Text.

**Gameability countermeasures**:
- Color: weight blocks by area variance (prevent single dominant palette from winning).
- Block-Match recall: don't use as ranking signal between two valid decompositions.
- CLIP: sanity check only, never reward signal.

Use **Design2Code-HARD** (80-page subset) as held-out eval set.
