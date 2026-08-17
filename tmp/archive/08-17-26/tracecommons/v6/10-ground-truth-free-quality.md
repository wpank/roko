# Ground-Truth-Free Quality Estimation

**Date**: August 2026 (v6)

**What is TraceCommons?** TraceCommons (TC) is an open-source, Rust-based, privacy-preserving
registry of AI coding agent session traces. Contributors submit scrubbed traces of what their
AI agents did (Claude Code, Codex, IronClaw, etc.); quality and novelty are scored inside TEEs
(Trusted Execution Environments); contributors earn NEAR blockchain credits. Built by Zaki
Manian (Cosmos SDK, IBC). ~235K LOC Rust, 6 crates. Pilot on GCP. ~352 submissions, ~13/week,
3 contributors, 6 GitHub stars.

**The bake-off confound**: PR #216 ran a scorer selection bake-off. Six trivial baselines
(paragraph count, line count, word count, byte count, distinct word count, mean word length) ALL
beat the winning model. Paragraph count achieved AUC=1.000 because every duplicate had exactly
1 paragraph while novel files had 7-163. The corpus builder entangled format with novelty class.
Without ground-truth labels, every automated metric validated against other automated metrics.
This circular reasoning let the confound go undetected.

---

## 0. The Problem

TC's scoring pipeline has no ground-truth labels. The bake-off (PR #216) was supposed to select
the best scorer, but the evaluation corpus was confounded -- format features (paragraph count,
byte length) predicted novelty class perfectly because the corpus builder generated duplicates as
single-paragraph files and novel files as multi-paragraph documents. The winning scorer was not
actually measuring quality or novelty; it was measuring paragraph count with extra steps.

Three consequences:

1. **The production scorer is uncalibrated.** Floor thresholds, embedding model choice, and
   scorer selection were downstream of a leaky evaluation. Every parameter tuned against this
   corpus is suspect.

2. **No human-annotated ground truth exists.** The synthetic paraphrase corpus has a length
   confound (median ratio 0.282). Building human annotations (doc 02, section A.4) requires
   40-80 person-hours and will take weeks.

3. **Circular validation is invisible.** When scorer A validates against scorer B, and scorer B
   was selected using scorer A's output, any shared confound is amplified rather than detected.
   The PR #216 confound survived because no external reference broke the circle.

The question this document addresses: **Can TC estimate trace quality and identify confounded
scorers without any ground-truth labels?** Four verified papers plus two classical methods say
yes, under specific conditions.

---

## 1. Judge-Aware Ranking (JAR)

**Source**: arXiv:2601.21817, ICML 2026.

Judge-Aware Ranking extends the Bradley-Terry-Luce (BTL) model with judge-specific
discrimination parameters. Standard BTL assumes all judges are equally reliable -- JAR does not.
It jointly estimates two things:

- **Latent trace quality** -- a scalar per trace, on a shared scale.
- **Judge reliability** -- a discrimination parameter per judge, indicating how well that judge
  separates high-quality from low-quality traces.

The key insight: judges that agree with each other AND with the emergent consensus get high
discrimination parameters. Judges that agree with each other but disagree with the consensus
(as confounded scorers do) get low discrimination parameters. This is exactly the PR #216
failure mode: paragraph count and perplexity agreed perfectly -- but only because both measured
format, not quality.

### TC Mapping

Each TC scorer is treated as a "judge." After collapsing the two shared-forward-pass scorers
(see Critical Warning below), TC has **4 independent judges**:

| TC Judge | Components | Role |
|---|---|---|
| `ForwardPassJudge` | Qwen 3.6 35B: perplexity + token rarity from a single forward pass | Quality judge (counts as ONE judge) |
| Embedding cosine distance | BGE-large-en-v1.5 against HNSW index | Novelty judge |
| MinHash Jaccard distance | Via Rensa (if wired per doc 02 A.2) | Novelty judge |
| NCD via zstd | Compression distance (if wired per doc 02 B.2) | Novelty judge |

JAR takes pairwise comparisons: for each pair of traces, each judge votes on which is
higher-quality. From these votes, JAR estimates the latent quality ranking and each judge's
discrimination parameter simultaneously.

### Critical Warning

The paper states: **"more data can make evaluation more confidently wrong under misspecified
aggregation."** If scorers share a systematic bias (as in the PR #216 confound), adding more
traces makes the JAR estimate converge faster -- to the wrong answer. JAR detects confounds
only when judges disagree. If all judges are confounded in the same direction, JAR cannot help.

This is why scorer diversity (section 4) is essential: at least one judge must measure something
genuinely different from the others.

**CRITICAL WARNING -- Correlated judges violate independence assumptions.**
`PerplexityScorer` and `TokenRarityScorer` both derive from the **same forward pass** through
Qwen 3.6 35B. Treating them as two independent judges violates the conditional independence
assumption that underpins Dawid-Skene, latent-class aggregation, and Hui-Walter. This
violation has two concrete consequences:

1. **CIs too narrow (overconfidence).** The aggregation model believes it has more independent
   evidence than it actually does, producing confidence intervals that are systematically
   too tight.
2. **Biased consensus estimates.** The shared covariance between the two scorers biases the
   latent-quality estimate toward whatever signal the shared forward pass happens to emphasize.

Reference: BT-sigma (Qian et al. 2026, via arXiv:2605.09702 -- note: 2605.09702 is "Calibrate,
Don't Curate" by Yanran Li, which cites BT-sigma); arXiv:2601.21817 follow-on 2605.05073
requires connected per-judge comparison graphs.

**Immediate fix**: Collapse `PerplexityScorer` + `TokenRarityScorer` into ONE judge
("ForwardPassJudge") for all judge-aware analyses (JAR, Hui-Walter, Dawid-Skene, IRT). They
share a forward pass and are not conditionally independent. The judge count for all
independence-sensitive analyses is **4**, not 5.

**Endgame fix**: Model the junction-tree covariance between the correlated scorers using
Snorkel-style dependency modeling (Ratner et al. 2018; see also arXiv:2511.13891). This
preserves both perplexity and token-rarity signals while correctly accounting for their
dependence -- neither signal is discarded, but the shared covariance is explicitly represented
rather than assumed away. (Note: arXiv:2111.14282 was previously cited here but has been found
by research6 to be a WRONG CITATION — it is a sentiment-analysis paper on customer-support
chat, not a Snorkel-related method. It has been removed.)

---

## 2. Rogan-Gladen Estimator

**Source**: arXiv:2605.06939, "Bias and Uncertainty in LLM-as-a-Judge."

The Rogan-Gladen (RG) estimator corrects for judge sensitivity and specificity without
ground-truth labels. Originally developed for medical diagnostic tests, it applies directly to
TC's problem: estimating the true prevalence of "high-quality" traces when each scorer has
unknown error rates.

### How It Works

For each scorer, estimate two parameters:

- **Sensitivity (Se)**: P(scorer says "high quality" | trace is truly high quality)
- **Specificity (Sp)**: P(scorer says "low quality" | trace is truly low quality)

The Rogan-Gladen formula corrects the apparent prevalence:

```
true_prevalence = (apparent_prevalence + Sp - 1) / (Se + Sp - 1)
```

When Se + Sp > 1 (i.e., the scorer is better than random), this correction is valid even under
distribution shift -- provided the scorer depends only on latent correctness and not on other
features of the test set.

### TC Application

Each TC scorer produces a binary accept/reject decision (after thresholding). Estimate Se and Sp
for each scorer by cross-referencing against other scorers using the Hui-Walter paradigm
(section 3). Then apply Rogan-Gladen to correct the pooled quality estimate.

The critical assumption is **conditional independence given latent quality**: the scorers must
make independent errors. **The shared forward pass between `PerplexityScorer` and
`TokenRarityScorer` means treating them as independent judges violates the Rogan-Gladen
assumption. Collapsing them into one composite judge is mandatory before applying RG
correction.** If this collapse is not performed, the RG estimator will treat correlated
errors as independent evidence, producing a corrected prevalence estimate that is
overconfident and biased. Mitigation: use the `ForwardPassJudge` (section 1) as a single
composite judge throughout all Rogan-Gladen calculations.

---

## 3. Hui-Walter Paradigm

**Classical method**: Bayesian estimation of test accuracy without a gold standard.

The Hui-Walter paradigm uses 2+ tests on 2+ populations with different prevalences to estimate
each test's sensitivity and specificity. The key: different populations provide different
"mixtures" of positive and negative cases, which disambiguates the test parameters from the
prevalence.

### TC Structure

TC satisfies the Hui-Walter requirements:

| Requirement | TC Mapping |
|---|---|
| 2+ tests | 4 independent judges: ForwardPassJudge (perplexity + token rarity collapsed to one), embedding cosine distance, MinHash Jaccard, NCD via zstd |
| 2+ populations | 3 contributors (core team, IronClaw users, brapse) |
| Different prevalences | Different contributors likely produce different quality distributions |

The minimum structure for identifiability:

```
2 tests x 2 populations = 4 unknowns (Se1, Sp1, Se2, Sp2) + 2 prevalences = 6 unknowns
2 tests x 2 populations = 4 observed cells (2x2 table per population) = 8 observations
8 observations > 6 unknowns -> identifiable
```

With 4 independent judges and 3 populations, the system is over-identified, which helps
with estimation stability. Note: using 5 raw scorer outputs (treating PerplexityScorer and
TokenRarityScorer as separate) would produce false over-identification -- the shared forward
pass means only 4 are conditionally independent.

### Minimum Sample Size

At least 30-50 traces per scorer per population for any reliability estimate. With ~352 total
submissions across 3 contributors, TC is at the lower bound. If one contributor has fewer than
30 submissions, pool the two smallest contributors.

---

## 4. Reliability Without Validity

**Source**: arXiv:2606.19544.

This paper demonstrates that raw exact-match agreement between judges overstates reliability.
Two judges can agree 90% of the time purely by chance if the class distribution is skewed
(e.g., 95% of traces are "novel"). The paper recommends **chance-corrected agreement** metrics.

### Cohen's Kappa

For any two TC scorers, compute:

```
kappa = (p_observed - p_chance) / (1 - p_chance)
```

Where `p_observed` is the fraction of traces where both scorers agree, and `p_chance` is the
expected agreement if both scorers were independent (computed from marginal rates).

**Interpretation thresholds**:

| Kappa | Interpretation | TC Action |
|---|---|---|
| < 0.2 | Slight agreement | Scorers measure different constructs -- do not pool |
| 0.2 - 0.4 | Fair agreement | Investigate what each scorer is actually measuring |
| 0.4 - 0.6 | Moderate agreement | Viable for ensemble, but check for shared confounds |
| 0.6 - 0.8 | Substantial agreement | Good ensemble candidates |
| > 0.8 | Near-perfect agreement | Likely redundant -- use the cheaper scorer |

**The PR #216 confound in kappa terms**: paragraph count and perplexity would have shown kappa
near 1.0 -- but both would have shown low kappa against a genuinely different signal (e.g.,
token rarity). Kappa between scorers is a confound detector, not just an agreement measure.

---

## 5. IRT for Judge Reliability

**Source**: arXiv:2602.00521, Item Response Theory Graded Response Model.

IRT models each trace as having a latent "quality" (analogous to student ability) and each
scorer as having:

- **Discrimination (a)**: How well the scorer separates high-quality from low-quality traces.
  A scorer with high discrimination produces sharply different scores for traces of different
  quality.
- **Difficulty (b)**: The quality threshold at which the scorer switches from reject to accept.
  A scorer with high difficulty rejects more traces.

### Relationship to JAR

IRT and JAR estimate similar quantities (latent quality, judge discrimination) but differ in
model structure:

| Property | JAR (BTL-based) | IRT (GRM-based) |
|---|---|---|
| Input | Pairwise comparisons | Item-level responses |
| Output | Latent quality ranking + discrimination | Latent quality + discrimination + difficulty |
| Identification | Needs comparison data | Needs multi-level response data |
| Advantage | Handles incomplete comparisons | Models threshold behavior directly |

For TC, IRT is a natural fit because scorers produce continuous scores that are thresholded
into accept/reject -- exactly the structure the Graded Response Model is designed for. If TC
retains raw scores (pre-thresholding), IRT can estimate where each scorer's effective threshold
lies and whether that threshold is appropriate.

---

## 6. Dawid-Skene Model

**Classical method**: Expectation-maximization for estimating annotator accuracy from
disagreement patterns.

Dawid-Skene alternates between:

1. **E-step**: Given current estimates of scorer accuracy, compute posterior probability that
   each trace belongs to each quality class.
2. **M-step**: Given current class assignments, update scorer accuracy estimates (confusion
   matrices).

Convergence yields both the most likely quality class for each trace and a full confusion matrix
for each scorer. This is the simplest approach in this document and serves as a baseline: if
Dawid-Skene cannot separate the scorers, more sophisticated methods will also struggle.

### When to Prefer Dawid-Skene Over JAR

- When the quality classification is naturally categorical (e.g., novel/duplicate/paraphrase)
  rather than continuous.
- When the number of traces is small (< 200) -- Dawid-Skene's EM converges faster than
  JAR's joint optimization.
- As a sanity check: if Dawid-Skene and JAR disagree substantially on scorer reliability, one
  model's assumptions are violated.

---

## 7. Forward-Pass Judge: Combined Perplexity + Token Rarity

`TokenRarityScorer` computes `exp(-mean(K rarest logprobs))` from the same forward pass that
produces perplexity. Implementation exists in the codebase. `EnclaveGateOrchestrator::evaluate`
does not call it.

**Judge-count clarification.** `TokenRarityScorer` is NOT a 5th independent judge -- it shares
a forward pass with `PerplexityScorer` (see section 1, Critical Warning). For all
independence-sensitive analyses (JAR, Hui-Walter, Dawid-Skene, IRT, Rogan-Gladen), the judge
count is **4**, not 5. The two forward-pass signals are combined into a single
`ForwardPassJudge`. However, combining them into one judge does not discard the diagnostic
information -- it simply treats the two signals as two outputs of a single measurement
instrument, which is what they actually are.

### Why the Forward-Pass Judge Is Still Critical

All judge-aware methods (JAR, Hui-Walter, IRT, Dawid-Skene) require **disagreement patterns**
to work. If all judges agree on every trace, the methods cannot distinguish quality from
confound. The `ForwardPassJudge` provides richer signal than perplexity alone because token
rarity and perplexity can diverge even within the same forward pass:

| Scenario | Perplexity | Token Rarity | Interpretation |
|---|---|---|---|
| High perplexity + low rarity | High | Low | Incoherent noise (random text) |
| High perplexity + high rarity | High | High | Genuinely rare tokens in coherent context |
| Low perplexity + low rarity | Low | Low | Common, predictable text |
| Low perplexity + high rarity | Low | High | Unusual: rare tokens in a predictable structure |

This diagnostic value makes the combined `ForwardPassJudge` richer than either signal alone.
When paragraph count and the forward-pass judge agree on a trace but the embedding distance
judge disagrees, the disagreement is evidence of format confounding. The key constraint is
that this combined signal counts as **one judge** for independence purposes -- not two.

### Implementation Cost

Minimal. `TokenRarityScorer` already exists. Wiring it into `EnclaveGateOrchestrator::evaluate`
is the work described in doc 02, section A.1 -- hours of effort. The combination into a
`ForwardPassJudge` is an analysis-level bookkeeping change, not an implementation change: both
scores are computed from the same forward pass and can be reported together as a single judge
with two component signals.

---

## 8. Implementation Sequence (9 Steps)

This sequence assumes TC's current state: 3 independent judges wired, 1 built but unwired
(`TokenRarityScorer`, which combines with `PerplexityScorer` into a 4th independent
`ForwardPassJudge`), ~352 traces, 3 contributors.

### Step 1: Wire TokenRarityScorer into the ForwardPassJudge

Wire `TokenRarityScorer` alongside `PerplexityScorer` in `EnclaveGateOrchestrator::evaluate`.
This is doc 02 section A.1. Hours of effort. Treat both outputs as components of a single
`ForwardPassJudge` in all downstream analyses (see section 1, Critical Warning). The effective
independent judge count moves from 3 to 4 -- not 5. Must happen first because steps 2-9 need
at least 4 independent judges for meaningful disagreement analysis.

### Step 2: Run All 4 Independent Judges on ~200 Traces

Score a stratified sample of ~200 traces (stratified by contributor, length quintile,
and current novelty score). Store raw scores, not just accept/reject. Every subsequent step
depends on this matrix. The `ForwardPassJudge` contributes two component scores (perplexity +
token rarity) that can be analyzed jointly or separately, but counts as one judge for
independence-sensitive analyses.

**Output**: A 200 x 5 score matrix (4 judges, with the ForwardPassJudge contributing 2 columns),
plus binarized versions at each scorer's current threshold. For JAR/Hui-Walter/Dawid-Skene,
collapse the ForwardPassJudge to a single aggregated column before fitting.

### Step 3: Compute Pairwise Chance-Corrected Agreement

For all 10 scorer pairs, compute Cohen's kappa (section 4).

**Decision gate**: If any pair has kappa > 0.9, those scorers are likely redundant -- flag
for investigation. If any pair has kappa < 0.2, those scorers measure different constructs --
they should not be pooled naively but are valuable as diverse judges.

### Step 4: Fit Judge-Aware BTL Model

Using the 200-trace score matrix, fit the JAR model (section 1). This produces:

- Latent quality estimate per trace (on a shared scale).
- Discrimination parameter per scorer (higher = more reliable).

**Decision gate**: If any scorer has discrimination < 0.5 (on normalized scale), it is adding
more noise than signal. Flag for possible exclusion.

### Step 5: Estimate Per-Scorer Sensitivity/Specificity via Rogan-Gladen

Using the Hui-Walter paradigm (section 3) with 3 contributor populations, estimate Se and Sp
for each scorer. Then apply the Rogan-Gladen correction (section 2) to estimate the true
prevalence of high-quality traces in the corpus.

**Decision gate**: If any scorer has Se + Sp < 1.2 (barely better than random), exclude it from
the ensemble. If Se + Sp < 1.0, the scorer is worse than random -- invert its decisions.

### Step 6: Identify Confounded Scorers

Compute correlation between each scorer's raw scores and trivial baselines (paragraph count,
line count, word count, byte count, distinct word count, mean word length). Any scorer with
Pearson |r| > 0.7 against any trivial baseline is confounded.

**Cross-check**: A confounded scorer should also have low discrimination in Step 4 and
anomalous Se/Sp in Step 5. If a scorer has high trivial-baseline correlation but high
JAR discrimination, the trivial baseline is actually measuring something real -- investigate
before excluding.

### Step 7: Build JAR-Weighted Ensemble

Construct a weighted ensemble excluding confounded scorers. Weights are proportional to
JAR discrimination parameters from Step 4.

```
ensemble_score(trace) = sum(w_j * score_j(trace)) / sum(w_j)
where w_j = discrimination_j for non-confounded scorers
```

### Step 8: Compare Ensemble vs Single-Scorer Performance

On a held-out set (split from the 200 traces), compare:

- JAR-weighted ensemble AUC
- Best single-scorer AUC
- Current production scorer AUC

If the ensemble does not beat the best single scorer, the additional complexity is not
justified. Use the best single scorer.

**Note**: Without ground-truth labels, "AUC" here means AUC against the JAR latent quality
ranking -- which is itself an estimate. This is better than no external reference but is not
a substitute for human annotation (doc 02, section A.4). When human labels become available,
re-run this comparison against true labels.

### Step 9: Replace Production Scorer with JAR Ensemble

If the ensemble wins in Step 8, deploy it as the production scorer. Monitor the
discrimination parameters over time -- if a scorer's discrimination drops below the exclusion
threshold (Step 4), automatically remove it from the ensemble.

---

## 9. When Judge-Aware Methods Fail

These methods have well-understood failure modes. Recognizing them early prevents wasted effort.

### Failure Mode 1: All Judges Share the Same Confound

If every scorer correlates with paragraph count (or any other format feature), JAR will
converge to a confident but wrong estimate. The consensus IS the confound.

**Detection**: Step 6 (trivial baseline correlation). If all scorers have |r| > 0.7 against
the same trivial baseline, no ensemble of these scorers will fix the problem.

**Remedy**: Add a scorer that does not share the confound. Token rarity (section 7) is the
immediate candidate. NCD (doc 02, section B.2) and MinHash (doc 02, section A.2) are
medium-term options. If no available scorer is confound-free, human annotation is the only
path forward.

### Failure Mode 2: Low Inter-Scorer Agreement (Kappa < 0.4)

If chance-corrected agreement between all scorer pairs is below 0.4, the scorers are measuring
different constructs. Pooling them into a single ensemble produces noise, not signal.

**Remedy**: Split into construct-specific ensembles. For example, one ensemble for "originality"
(embedding distance, MinHash, NCD) and another for "quality" (perplexity, token rarity). Score
each dimension separately. Report multi-dimensional quality, not a single number.

### Failure Mode 3: High Inter-Scorer Agreement (Kappa > 0.8)

If all scorer pairs agree almost perfectly, there is not enough disagreement for JAR/IRT to
estimate discrimination parameters. The methods degenerate to simple averaging.

**Remedy**: The scorers are likely redundant. Use the cheapest one. Or: the task is easy and
all scorers handle it well -- look for harder test cases (adversarial traces, edge cases) where
disagreement emerges.

### Failure Mode 4: Insufficient Data

With fewer than 30 traces per scorer per population, Hui-Walter estimates are unstable. With
fewer than 100 total traces, JAR's joint optimization may not converge.

**TC status**: ~352 traces across 3 contributors. Marginal for Hui-Walter if contributions
are unevenly distributed. If the smallest contributor has fewer than 30 traces, pool the
two smallest contributors into a single population (reducing from 3 populations to 2).

---

## 10. Minimum Requirements Summary

| Requirement | Threshold | TC Status |
|---|---|---|
| Independent judges | >= 3 (4 preferred) | 3 independent judges wired, 1 built (combined ForwardPassJudge makes 4 independent judges total) |
| Traces scored by all judges | >= 200 | ~352 total, need to score with all 4 independent judges |
| Distinct contributor populations | >= 2 (3 preferred) | 3 contributors |
| Traces per population | >= 30 | Depends on per-contributor distribution |
| Hui-Walter structure | 2+ tests x 2+ populations | Satisfied with 4 independent judges x 3 populations |
| Judge diversity | >= 1 judge orthogonal to others | ForwardPassJudge (perplexity + token rarity) is the quality-side candidate; embedding, MinHash, NCD are novelty-side |
| ForwardPassJudge collapsed | PerplexityScorer + TokenRarityScorer treated as ONE judge | Required before applying any independence-sensitive method (JAR, Hui-Walter, Dawid-Skene, RG) |

---

## 11. Relationship to Other v6 Documents

This document is a deep dive into doc 02's section C.1 (Label-Free Quality Scoring). It
provides the methods and implementation sequence that C.1 references but does not detail.

| v6 Doc | Relationship |
|---|---|
| 02 (Scoring Pipeline) | A.1 (TokenRarityScorer) is prerequisite. C.1 (JAR) is the parent reference. A.4 (human annotation) is the ultimate ground truth this document tries to postpone. |
| 02 Section A.3 | Fix bake-off corpus -- orthogonal but complementary. A fixed corpus validates the methods here. |
| 02 Section B.5 | Auto-optimization of gate thresholds -- consumes the per-scorer reliability estimates this document produces. |
| 06 (Paper Index) | All papers cited here appear in the paper index with verification status. |

---

## 12. Decision Framework

### "We have no human labels and cannot get them soon."

Use the full 9-step sequence (section 8). JAR-weighted ensemble with confound detection
provides the best available quality estimate without labels. Accept that the estimate has
unknown accuracy until human labels arrive.

### "We have 50-100 human labels."

Use the labels to validate JAR's latent quality estimates. If JAR's ranking correlates well
with human labels (Spearman rho > 0.7), the label-free ensemble is trustworthy. If not,
investigate which JAR assumptions are violated.

### "We have 200+ human labels."

Label-free methods are no longer the primary tool. Use human labels directly for scorer
evaluation. The methods in this document become a monitoring tool: run JAR periodically to
detect scorer drift, and flag when a scorer's discrimination parameter drops significantly.

### "We suspect a specific confound but cannot prove it."

Run Step 6 (trivial baseline correlation) immediately. This requires only the score matrix
from Step 2 -- no modeling. If the suspected confound shows |r| > 0.7 against a scorer, the
evidence is strong.

---

## 13. Verification Ledger

| Item | arXiv | Venue | Status |
|---|---|---|---|
| Judge-Aware Ranking | 2601.21817 | ICML 2026 | **Verified** |
| JAR follow-on: connected per-judge comparison graphs | 2605.05073 | -- | **Verified** |
| Calibrate, Don't Curate (Yanran Li; cites BT-sigma / Qian et al.) | 2605.09702 | -- | **Verified** (note: attributed to Qian et al. in some sources but paper is by Li) |
| Bias and Uncertainty in LLM-as-a-Judge (Rogan-Gladen) | 2605.06939 | -- | **Verified** |
| Reliability without Validity | 2606.19544 | -- | **Verified** |
| IRT Graded Response Model for Judge Reliability | 2602.00521 | -- | **Verified** |
| Hui-Walter Paradigm | Classical | Veterinary epidemiology | **Established method** |
| Dawid-Skene Model | Classical | Applied Statistics, 1979 | **Established method** |
| Rogan-Gladen Estimator | Classical | Am J Epidemiology, 1978 | **Established method** |

*9 methods. 6 verified arXiv papers, 3 established classical methods. Last updated August 2026.*
