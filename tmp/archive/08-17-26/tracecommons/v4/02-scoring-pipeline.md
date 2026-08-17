# Scoring Pipeline: Fixing What's Broken & Building What's Next

**Date**: August 2026

TC's gate pipeline scores traces for quality and novelty via `q = f * g * a`. The pipeline architecture is clean (four traits: `PerplexityScorer`, `TokenRarityScorer`, `Embedder`, `VectorIndex`). The problem is the measurement methodology: the A2.6 bake-off was confounded (PR #216 -- six trivial baselines beat the winning model because paragraph count was entangled with novelty class). The production scorer is uncalibrated, and no human-annotated ground truth exists.

This is existential: if the scorer can't distinguish novel from derivative, the credit mechanism rewards the wrong contributions.

---

## 1. Immediate Fixes (Weeks)

### 1.1 Wire TokenRarityScorer

`TokenRarityScorer` computes `exp(-mean(K rarest logprobs))` from the same forward pass as perplexity. The trait, implementation, mock, and aggregation all exist. `EnclaveGateOrchestrator::evaluate` doesn't call it.

High perplexity + low rarity = incoherent noise. High perplexity + high rarity = genuinely rare tokens in coherent context. This decomposition distinguishes novelty from noise for free.

**Effort**: Hours.

### 1.2 MinHash Dedup via Rensa

MinHash fingerprints from each trace. Jaccard >0.85 = near-duplicate. Catches verbatim copies before the expensive embedding path. Analytically known false-positive rates -- no model dependency.

Shingle at paragraph/event level (not token level) to prevent boilerplate from dominating Jaccard.

**Effort**: 1-2 days.

### 1.3 Fix the Bake-Off Corpus

Rebuild with stratification as a design-time invariant: every class (novel, duplicate, paraphrase, near-duplicate) must have examples at every length quintile and paragraph-count range.

Pre-flight check: compute AUC of every trivial baseline on the new corpus. Every baseline AUC must be below 0.6. If any exceeds 0.7, the corpus has a leak.

**Effort**: ~1 week.

### 1.4 Human Annotation (200+ Traces, 3+ Reviewers)

Without human judgment, every automated metric validates against other automated metrics. Minimum: 200+ production traces, stratified by length/event count/tenant/current score. 3+ independent reviewers per trace.

Target: Krippendorff's Alpha > 0.67 (usable labels). Below 0.5 = "novelty" is too subjective, fall back on objective proxies.

**Effort**: 40-80 person-hours.

---

## 2. Multi-Layer Pipeline (Months)

Layer cheap fast filters before expensive ones. Each layer short-circuits or passes through.

```
Layer 1: MinHash / LSH dedup           (< 1ms)
Layer 2: NCD via zstd                  (< 10ms)
Layer 3: Embedding distance            (existing path)
Layer 4: Structural / process mining   (new)
Layer 5: LLM perplexity               (existing)
```

### 2.1 NCD Compression Pre-Filter

NCD uses zstd as a Kolmogorov complexity proxy. Parameter-free, O(n), TEE-compatible. Catches structural similarity where exact tokens differ but information content is the same.

Per-tenant zstd dictionaries make NCD more discriminative. **Effort**: Days.

### 2.2 Process Mining: Tool-Call DAG Conformance

Agent traces are structured processes. Two traces calling the same tools in the same order with different arguments are structurally identical but might score as "novel."

Extract `(event_type, tool_name)` pairs → frequency-weighted directly-follows graph per tenant → score new traces by fraction of unseen transitions. **Effort**: 1-2 weeks.

### 2.3 Failure Attribution Scoring

A trace that fails in a novel way is diagnostic (reveals failure modes others can avoid). A trace that fails in a known way is redundant. This adds a third dimension: content-novel, structurally-novel, and/or failure-novel.

Classify succeeded/failed from outcome metadata → attribute failure step → score failure novelty against per-tenant failure-mode frequency table. **Effort**: Heuristic 1 day, LLM-based 1-2 weeks.

### 2.4 Compound System Auto-Optimization

Replace hand-tuned gate thresholds with Bayesian optimization (GP surrogate via `argmin` crate) maximizing AUC on annotated corpus. When a human reviewer disagrees with the gate, propagate disagreement to the specific layer.

Depends on annotated corpus (1.4). **Effort**: Days (Bayesian opt), 1-2 weeks (feedback propagation).

---

## 3. Research-Backed Upgrades (Quarters)

### 3.1 Label-Free Quality Scoring (Judge-Aware Ranking)

The Judge-Aware Ranking Framework (arXiv 2601.21817, ICML 2026) jointly estimates latent trace quality and scorer reliability from pairwise comparisons **without reference labels**. Treats each candidate scorer as a "judge," feeds pairwise trace comparisons into judge-aware BTL, weights ensemble by estimated reliability.

Complemented by Hui-Walter Bayesian estimator (arXiv 2401.09376): estimates classifier sensitivity/specificity with no gold standard across 2+ prevalence-differing populations.

**Impact**: Removes the PR #216 confound at the root. Produces defensible, calibrated quality numbers without human labels.

### 3.2 Conformal Prediction on Scores

TECP (arXiv 2509.00461) turns cumulative token-entropy into a conformal nonconformity score with finite-sample coverage. TC can ship "top-15% novelty, 95% coverage" instead of a bare 0.73.

**Impact**: Credibility upgrade for grant reviewers and trace consumers.

### 3.3 Causal Failure Attribution

Causal Agent Replay (arXiv 2606.08275, open-source) intervenes (do-resample a step, measure outcome-distribution shift) + Shapley credit-splitting. Correlational LLM-judge attribution scores only ~14% step-level accuracy; CAR replaces it.

**Impact**: Per-step causal-importance vector as a premium annotation.

### 3.4 Safety-Preserving Compression

TRACE (arXiv 2606.00611, open-source): Compressor-Reader latent evidence state, +12.6pp safety-detection accuracy. Store full traces cold; index/query the compressed state hot.

**Impact**: 10-50x storage savings while retaining safety signal.

### 3.5 Marginal Value Scoring

CausalMix (arXiv 2607.01104): mixture-as-treatment, CATE estimation, generalizes across shifting/unseen pools. Moves from "is this trace good?" to "given the current corpus, what is the marginal return of adding it?"

**Impact**: Directly prices redundancy. Individually-excellent-but-redundant traces get low credit.

### 3.6 Specification Mining → Pattern Vocabulary

Mining Beyond the Bools (arXiv 2603.06710): synthesizes temporal + relational invariants from traces. Tag each trace with patterns it satisfies/violates. Novelty = "exhibits a pattern not yet in the vocabulary."

**Impact**: Explainable, auditable novelty. Natural bridge to EU AI Act Art. 12 logging.

---

## 4. Improved Embeddings

Ordered by effort:

| Approach | Effort | What |
|---|---|---|
| Matryoshka embeddings | Days | 64 dims for coarse filter, full 768 for final score |
| Code-aware embeddings | Days | CodeBERT/GraphCodeBERT/StarEncoder instead of BGE |
| Contrastive fine-tuning | 1-2 weeks | SimCSE/CoSENT on annotated corpus -- highest leverage |
| Multi-view embedding | 1-2 weeks | Embed NL, code, tool-call structure separately |

---

## 5. Decision Framework

**"Stop paying for garbage"**: Focus on dedup layers (1.2, 2.1, 2.2). No labeled corpus needed.

**"Know whether our scorer works"**: Focus on measurement (1.3, 1.4, 2.4). Without annotations, every pipeline improvement is unmeasurable. If you can only do one thing, do 1.4.

**"Price traces by actual value"**: Focus on marginal value scoring (3.5) and influence functions. Most infrastructure, most defensible pricing.

**"Differentiate from competitors"**: Focus on label-free scoring (3.1), failure attribution (3.3), process mining (2.2). No competitor does these.

### Critical Path

```
1.3 (fix corpus) + 1.4 (annotations)
          ↓
    2.4 (auto-optimize)
          ↓
    3.1 (label-free scoring)
          ↓
    3.5 (marginal value)
```

### Team Bandwidth Guide

| Bandwidth | Focus |
|---|---|
| 1 engineer, part-time | 1.1 (hours), 1.2 (1-2 days), 1.4 (incrementally) |
| 2-3 engineers | All of §1 in parallel, then multi-layer pipeline |
| Team with ML capacity | §1 + §2 in parallel, pick one from §3 |
