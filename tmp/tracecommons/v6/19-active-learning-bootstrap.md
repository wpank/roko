# Active Learning Bootstrap for Annotation Corpus

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores
AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution
Environments) on NEAR AI Cloud, compensating contributors with NEAR blockchain credits.
~352 submissions, 3 contributors, 6 GitHub stars. TC has no human-annotated ground truth.
Without annotations, every automated metric validates against other automated metrics -- the
circular reasoning that let the PR #216 bake-off confound go undetected. Paragraph count
achieved AUC 1.000 because format was entangled with novelty class, and no external reference
broke the circle. This document addresses how to bootstrap TC's annotation corpus using active
learning to maximize label quality while minimizing the annotation budget that a 2-person
team with 3 contributors can realistically afford.

---

## 1. The Problem: Annotation Is Essential but Expensive

TC needs human-annotated traces for three purposes:

1. **Calibrate the gate pipeline (doc 09).** Conformal prediction requires labeled calibration
   sets. Without labels, the conformal threshold is computed from scores that are themselves
   uncalibrated -- producing a well-calibrated gate to the wrong quantity.

2. **Validate scorers (doc 10).** The ground-truth-free methods (JAR, Hui-Walter, Rogan-Gladen,
   IRT) estimate scorer reliability from disagreement patterns, but when all scorers share the
   same confound, they converge confidently to the wrong answer. Human labels break this
   degeneracy.

3. **Train the bake-off corpus (doc 02, section A.4).** The confounded corpus needs replacement.
   A new corpus requires ground-truth novelty labels to validate that trivial baselines no
   longer achieve AUC > 0.6.

Doc 02 section A.4 estimates 40-80 person-hours for 200+ traces with 3+ reviewers. With a
2-person team and 3 contributors (one arrived days ago via PR #250), that budget represents
weeks of effort diverted from development. Random annotation wastes most of this budget on
traces that are easy to classify and provide little calibration signal.

Active learning inverts this: instead of labeling random traces, it selects the traces that
teach the most per label.

---

## 2. The Key Insight: 93% Performance at ~6% Cost

arXiv:2502.16892 (Zhang & Takada, "Applying LLMs to Active Learning," IJIS 2025) demonstrates
that LLM-assisted active learning achieves 93% of full-annotation performance at approximately
6% of the cost.

**Important nuance.** The "93%" is relative performance retention: 85.42% accuracy vs GPT's
94.63% baseline. The "6%" refers to computational cost compared to running GPT directly -- NOT
6% of human annotation budget. The paper compares LLM-assisted active learning against full LLM
labeling, not against full human labeling.

The principle transfers: active learning's efficiency gain -- dramatically fewer labels needed
for a given performance level -- is a property of the selection strategy, not the labeling
source. For TC, the oracle is a human reviewer, and the selection strategy determines which
of the ~352 traces that reviewer spends time on.

**Source correction.** Earlier internal notes attributed this figure to arXiv:2502.11767 (a
survey paper). The correct source is arXiv:2502.16892.

---

## 3. Verified Active Learning Methods

### 3.1 ActPRM: Active Process Reward Model Training

**Source**: arXiv:2504.10559

ActPRM applies active learning to process reward model (PRM) training. Key result: SOTA 75.0%
accuracy on ProcessBench at 6% of full annotation cost. PRM training faces the same bottleneck
as TC: labeling intermediate steps is expensive, most labels are uninformative, and strategic
selection reduces cost by an order of magnitude.

**Mechanism.** Uncertainty sampling: select process steps where the current PRM is most uncertain,
label them, retrain, repeat. Each batch resolves the model's most pressing ambiguities.

### 3.2 TypiClust: Cold-Start Selection

**Source**: Hacohen et al., arXiv:2202.02794

Before any labels exist, there is no model uncertainty to exploit. TypiClust addresses the
cold-start problem by selecting the most "typical" examples -- those closest to cluster centroids
in embedding space. Random selection can oversample dense regions and miss sparse ones; TypiClust
ensures each cluster contributes at least one representative.

**TC path.**

1. Embed all ~352 traces via BGE-large-en-v1.5 (TC's existing embedding model).
2. Cluster with k-means, k=15-20 (one cluster per ~18-23 traces).
3. Select the trace closest to each centroid.
4. Annotate these 15-20 traces as the first batch.

### 3.3 Hui-Walter Diagnostic (Without Gold Standard)

**Source**: Classical (veterinary epidemiology); TC application in doc 10, section 3.

The Hui-Walter paradigm estimates test accuracy without a gold standard using 2+ tests on 2+
populations. For TC: use multiple scorers across contributor cohorts to estimate sensitivity and
specificity without labeled data.

Hui-Walter is not an active learning method, but it sets the floor: approximately 30-50 labels
are needed before any reliability estimate is meaningful. Fewer than 30 labels and no estimation
method can produce stable scorer calibration.

---

## 4. Four-Week Bootstrap Protocol

Total budget: ~35 person-hours for 100-140 labeled traces. Compare to the naive approach:
~80 person-hours for 200+ random traces (doc 02, section A.4).

### Week 1: Cold Start via TypiClust

**Goal.** First labeled batch with maximum distributional coverage.

1. Embed all ~352 traces using BGE-large-en-v1.5.
2. Cluster into 15-20 groups (k-means on embedding vectors).
3. Select 1-2 traces per cluster, choosing the trace closest to each centroid.
4. Annotate with a binary label: "Is this trace novel relative to the corpus?" Binary at this
   stage because finer-grained labels require guidelines that do not yet exist.
5. Record annotator rationale (1-2 sentences per label) to seed annotation guidelines.

**Output.** 20-30 labeled traces spanning the full embedding space.
**Effort.** ~5 person-hours.

### Week 2: Uncertainty Sampling

**Goal.** Label traces where scorers disagree most -- highest calibration signal per label.

1. Run all available scorers (perplexity, token rarity, embedding cosine, MinHash Jaccard,
   NCD if wired) on the full corpus.
2. Compute disagreement score per trace: variance of binarized scorer decisions across all
   scorers. A trace where 3 scorers say "novel" and 2 say "not novel" has high disagreement.
3. Select the top 30-40 traces by disagreement score.
4. Annotate with binary label and rationale.
5. Re-calibrate gate thresholds using cumulative labels (Week 1 + Week 2).

**Output.** 50-70 cumulative labeled traces.
**Effort.** ~10 person-hours.

Traces where scorers agree are easy -- labeling confirms what scorers already know. Traces
where scorers disagree are the decision boundary: labels directly inform which scorer is right
and which is confounded.

### Week 3: Boundary Refinement

**Goal.** Focus annotation on traces near the accept/reject boundary.

1. Using cumulative labels from Weeks 1-2, fit a preliminary JAR-weighted ensemble (doc 10,
   Step 7).
2. Score all unlabeled traces with the ensemble.
3. Identify traces whose ensemble scores fall within +/- 0.1 of the gate threshold tau
   (doc 09, section 4.1).
4. Select 30-40 boundary traces.
5. Annotate with binary label and rationale.
6. Refit ensemble and recompute tau.

**Output.** 80-110 cumulative labeled traces.
**Effort.** ~10 person-hours.

Labels far from the boundary provide almost no information about threshold placement. Boundary
traces are harder to classify but each label directly improves threshold accuracy -- marginal
value per label is highest at the decision boundary.

### Week 4: Stratified Completion and Agreement Measurement

**Goal.** Fill coverage gaps and measure annotation quality.

1. Identify underrepresented strata: contributor, trace length, agent family, language.
2. Select 20-30 traces from underrepresented groups.
3. Annotate with binary label and rationale.
4. Compute inter-annotator agreement (Krippendorff's Alpha) across all labels.
5. **If Alpha < 0.67**: revise annotation guidelines using recorded rationales. Re-annotate
   the Week 1 batch with revised guidelines. Recompute Alpha.
6. **If Alpha >= 0.67 and < 0.80**: labels usable for calibration with uncertainty estimates.
7. **If Alpha >= 0.80**: labels usable as ground truth. Proceed to scorer validation (doc 10)
   and bake-off corpus construction (doc 02, section A.3).

**Output.** 100-140 cumulative labeled traces with measured inter-annotator reliability.
**Effort.** ~10 person-hours.
**Decision gate.** Alpha < 0.50 after guideline revision triggers the fallback in section 8.

---

## 5. Hybrid Approach: LLM Pre-Label + Human Review

LLM pre-labeling accelerates annotation by giving reviewers a starting point.

1. **LLM pre-labels all ~352 traces** with a structured prompt producing NOVEL, NOT_NOVEL,
   or UNCERTAIN plus a 1-2 sentence rationale.
2. **Triage by confidence.** High-confidence NOVEL/NOT_NOVEL: human validates (fast).
   UNCERTAIN: human labels from scratch (active learning directs effort here).
3. **Effort allocation.** Validation is 2-3x faster than labeling from scratch. A reviewer
   who labels 4 traces/hour from scratch can validate 8-12 pre-labeled traces/hour.
4. **Estimate LLM accuracy via Rogan-Gladen (doc 10).** After 30+ human labels, compute
   LLM sensitivity and specificity.

**Expected speedup.** 2-3x compared to human-only labeling.

**Risk: Anchoring bias.** Research (research4 finding C5) establishes that LLM pre-labeling
induces measurable anchoring bias -- humans over-accept LLM suggestions, inflating apparent
agreement and depressing true reliability. The effect is real: reviewers seeing a pre-label tend
to validate it rather than evaluate independently, which raises observed agreement without
raising actual labeling quality. The exact published magnitude of this bias is the
thinnest-sourced item in the research base -- frame as "measurable but magnitude requires TC's
own measurement."

**Mitigation.**

1. **Withhold LLM suggestions on the entire calibration subset** (not just 30 traces). Have
   annotators label the calibration batch blind, then run the same traces with LLM pre-labels
   visible. The agreement delta between the two conditions is TC's reliability penalty: it
   quantifies exactly how much apparent agreement is an artifact of anchoring rather than genuine
   label quality.
2. If the agreement delta exceeds 10 percentage points (a rough threshold for "material"), treat
   the blind labels as authoritative for all calibration-critical traces.
3. For non-calibration traces where speed matters, pre-labels remain useful -- but report the
   measured penalty alongside any reliability metrics derived from pre-label-assisted annotation.

---

## 6. Snorkel Weak Supervision: Annotation Multiplier

Labeling functions (LFs) generate weak labels at scale. Snorkel's label model combines them
into probabilistic labels more accurate than any individual LF.

**TC labeling functions.**

| LF | Signal | Confidence | Est. Accuracy |
|---|---|---|---|
| MinHash Jaccard > 0.95 | Near-duplicate | High | ~95% |
| All 5 scorers agree: accept | Likely novel | Moderate | ~80% |
| All 5 scorers agree: reject | Likely not novel | Moderate | ~75% |
| New contributor + new tools | Likely novel | Low | ~65% |
| Trace length < 10 events | Likely trivial | Moderate | ~70% |
| Embedding cosine < 0.3 to nearest neighbor | Likely novel | Moderate | ~80% |

**Protocol.**

1. Run all LFs on the full corpus.
2. Fit Snorkel's label model (estimates LF accuracies and correlations from agreement
   patterns).
3. Label model produces P(novel | LF outputs) per trace.
4. Use probabilistic labels as training signal; use human labels from the 4-week protocol
   as the validation set only -- never as training data, to avoid circularity.

**Limitation.** Traces where no LF has signal get probabilistic labels near 0.5. These are
exactly the traces that should be routed to human annotation via the active learning protocol.

**Rust implementation note.** The Snorkel label model is reimplementable in approximately
200-500 LOC Rust -- there is no fundamental Python dependency. The full algorithm:

1. Assemble the LF matrix Λ (K items × m labeling functions, with an abstain class).
2. Specify the LF dependency graph (which LFs share signal sources and may be correlated).
3. Compute the inverse generalized covariance of the junction tree formed by that dependency
   graph.
4. Matrix-complete to recover conditional LF accuracies P(LF output | true label Y).
5. Emit probabilistic labels P(novel | LF outputs) per trace for use in a noise-aware loss.

The non-trivial parts are conflict resolution and correlation modeling. For independent LFs --
the common case in TC's early deployment -- the model collapses to a simple generative model
that is straightforward to implement. This is the endgame for TC's Rust-native goal: no
Python subprocess, no interop boundary, weak supervision runs entirely in-enclave.

**Source attribution.** Ratner et al., "Snorkel: Rapid Training Data Creation with Weak
Supervision," VLDB 2018, p223 (the primary algorithm reference). arXiv:2511.13891 is an
application paper that uses Snorkel in its pipeline; it confirms the method is actively used
but is not an independent confirmation of the algorithm itself. (Correction from research4 C8
verification.) arXiv:2111.14282 was previously listed here as a Snorkel application paper but
has been found by research6 to be a WRONG CITATION: it is a sentiment-analysis paper on
customer-support chat (RoBERTa + labeling functions), not related to COSINE or Snorkel's core
algorithm. It has been removed from this reference list.

---

## 7. Minimum Requirements

### 7.1 Label Count

- **30-50 labels**: Minimum for any reliability estimate (Hui-Walter floor).
- **100-140 labels**: 4-week protocol target. Sufficient for gate calibration (doc 09),
  preliminary scorer validation (doc 10), and bake-off corpus seed (doc 02).
- **200+ labels**: Full validation threshold (doc 02, A.4). Achievable with additional
  uncertainty sampling rounds beyond Week 4.

### 7.2 Annotator Count

Minimum 3 annotators. Krippendorff's Alpha requires at least 2, but 2 gives only pairwise
agreement with no robustness to individual bias. With TC's 3 contributors + 2 core team
members, 3 is feasible.

### 7.3 Agreement Thresholds

| Krippendorff's Alpha | Interpretation | TC Action |
|---|---|---|
| < 0.50 | Poor | "Novelty" may be too subjective (section 8). Revise guidelines. |
| 0.50 - 0.67 | Fair | Usable for development, not calibration. Revise guidelines. |
| 0.67 - 0.80 | Good | Labels usable for gate calibration with uncertainty estimates. |
| > 0.80 | Excellent | Labels usable as ground truth. |

**Calibration note.** These thresholds are not from a single authoritative citation. Research4
finding C6 establishes that realistic Krippendorff's Alpha for subjective novelty judgments is
often 0.4-0.6 -- the same range as code-quality annotation. Treat Alpha >= 0.67 as aspirational,
not a hard gate. TC should report the observed Alpha alongside any derived metrics, and consider
pairwise comparison (section 7.5) as the mechanism for raising it.

### 7.4 Annotation Guidelines

Guidelines must include worked examples from doc 02, section A.4:

- Same approach, new context: novel or not?
- New approach, same context: novel or not?
- Structurally identical, semantically different: novel or not?
- Partial overlap: how much shared content disqualifies novelty?

Week 1 rationales seed the guidelines; Week 4 disagreements refine them.

### 7.5 Pairwise Comparison for Higher Agreement

**Source.** Research4 finding C7; well-established in the k-DPP/Bradley-Terry-Luce (BTL)
literature.

Absolute scoring ("is this trace novel? yes/no") asks annotators to apply an internal scale,
which is highly personal. Pairwise comparison ("which of these two traces is more novel?") is a
relative judgment that produces significantly higher inter-annotator agreement for subjective
quality dimensions. This is well-established in the human-preference literature that underlies
RLHF and DPO datasets.

**Why this matters for TC.** Novelty and quality are both subjective. If absolute labeling
yields Alpha in the 0.4-0.6 range (section 7.3 calibration note), switching to pairwise
comparison is the most direct available lever for raising it -- before investing additional
effort in guideline revision.

**Avoiding O(n²) cost.** Pairwise comparison on n traces naively requires O(n²) comparisons.
Adaptive/tournament designs reduce this dramatically:

- **Sorting-based active comparison.** Run a merge-sort tournament on embedding-clustered traces.
  Each comparison resolves a boundary between adjacent novelty tiers. With n=352 traces, a
  tournament requires approximately n log n = ~2,800 comparisons -- but only the comparisons near
  the boundary matter, so active sampling reduces this further to the ~50-150 range needed for
  calibration.
- **BTL model with sequential updating.** After each pairwise comparison, update the
  Bradley-Terry-Luce model and select the next pair at maximum expected information gain (the
  comparison whose outcome would most reduce uncertainty in the current ranking).

**Integration with judge-aware BTL (doc 10).** Pairwise labels feed directly into the
judge-aware BTL model described in doc 10. This creates a coherent pipeline: pairwise
comparisons during annotation produce a calibrated novelty ranking; the BTL model aggregates
across annotators accounting for individual biases; the resulting scores feed the conformal gate
(doc 09).

**Recommendation.** If absolute labeling in Week 4 yields Alpha < 0.67 and guideline revision
does not close the gap, switch novelty and quality labeling to pairwise with adaptive tournament
sampling. Do not switch mid-protocol -- complete one full approach before comparing.

---

## 8. When Active Learning Is Not Enough

If inter-annotator agreement stays below 0.50 after guideline revision, "novelty" as a single
binary label is too subjective. No annotation infrastructure fixes a label that humans cannot
consistently apply.

**Fallback: objective proxies.**

1. **Influence on downstream tasks (doc 02, C.9).** A trace is valuable if removing it degrades
   downstream model performance. Train a small model on the corpus, remove each trace, measure
   performance delta.

2. **Skill extractability (doc 02, C.11).** A trace is valuable if a reusable skill can be
   extracted from it. Run the skill extraction pipeline, check whether the extracted skill
   generalizes beyond the source trace.

3. **Structural conformance (doc 17).** A trace is valuable if it conforms to well-formed
   session structure: has tool calls, has outcomes, has a coherent task arc, is not trivially
   short. Fully automatable.

These proxies are individually weaker than a well-defined "novelty" label but are objective,
measurable, and reproducible. Switching to them is not a retreat -- it is an acknowledgment
that the pipeline should measure what can be measured reliably.

---

## 9. Budget Summary

| Phase | Traces | Cumulative | Hours | Method |
|---|---|---|---|---|
| Week 1 | 20-30 | 20-30 | ~5 | TypiClust cold start |
| Week 2 | 30-40 | 50-70 | ~10 | Uncertainty sampling |
| Week 3 | 30-40 | 80-110 | ~10 | Boundary refinement |
| Week 4 | 20-30 | 100-140 | ~10 | Stratified completion |
| **Total** | | **100-140** | **~35** | |

Naive baseline: 200+ random traces at 40-80 person-hours. The active learning protocol
achieves usable calibration at ~44% of the naive cost, with guaranteed distributional coverage
(TypiClust), targeted scorer disagreement resolution (Week 2), and focused boundary precision
(Week 3).

---

## 10. Integration with Other v6 Documents

| v6 Doc | Relationship |
|---|---|
| 02 (Scoring Pipeline) | A.4 defines the labeling task this document optimizes. A.1-A.3 (scorer wiring, MinHash, corpus fix) are prerequisites for uncertainty sampling. |
| 09 (Conformal Gate) | Consumes the labeled set. Section 4.1 (quantile gate) needs calibration data; this document provides it. Section 7.2 (epsilon dial) depends on calibrated labels. |
| 10 (Ground-Truth-Free Quality) | Complementary. Doc 10 estimates scorer reliability without labels; this document provides labels that validate those estimates. Section 12 ("50-100 human labels") references this protocol's output. |
| 14 (Corpus Seeding) | Seeded traces join the active learning pool but are labeled separately (known provenance influences novelty assessment). |

---

## 11. Implementation Sequence

1. **Wire TokenRarityScorer** (doc 02, A.1). Week 2 needs 5+ scorers for meaningful
   disagreement. Hours.
2. **Embed all ~352 traces** via BGE-large-en-v1.5. Export from HNSW index if available.
   Hours.
3. **Cluster and select seed batch** (Week 1). k-means, select centroids. Minutes of
   compute, ~5 hours of annotation.
4. **Score full corpus with all scorers** (Week 2 prerequisite). Store the full score matrix.
   Hours.
5. **Execute Weeks 2-4.** Each week depends on prior labels.
6. **Compute Krippendorff's Alpha** (Week 4). Below 0.67: iterate guidelines before
   proceeding.
7. **Feed labels to doc 09 and doc 10 pipelines.** Gate calibration and JAR validation
   consume the labeled set directly.

---

## 12. Verification Ledger

| Item | ID / Venue | Claim Used | Status |
|---|---|---|---|
| Zhang & Takada, "Applying LLMs to Active Learning" | arXiv:2502.16892 (IJIS 2025) | 93% relative performance at ~6% cost | **Verified** |
| ActPRM | arXiv:2504.10559 | SOTA 75.0% ProcessBench at 6% annotation cost | **Verified** |
| TypiClust | arXiv:2202.02794 | Cluster-centroid selection for cold start | **Verified** |
| Hui-Walter paradigm | Classical (veterinary epidemiology) | Gold-standard-free estimation | **Established method** |
| Rogan-Gladen estimator | Classical (Am J Epidemiology 1978) | Prevalence correction via Se/Sp | **Established method** |
| Snorkel weak supervision | Ratner et al., "Snorkel: Rapid Training Data Creation," VLDB 2018, p223 | LF combination via generative model; reimplementable in ~200-500 LOC Rust | **Established method** |
| Krippendorff's Alpha | Classical (Content Analysis 2004) | Inter-annotator reliability | **Established method** |
| arXiv:2502.11767 | arXiv:2502.11767 | NOT source of 93%/6% figure (correction) | **Verified (negative)** |
| arXiv:2111.14282 | **WRONG CITATION** | This is a sentiment-analysis paper on customer-support chat, NOT the COSINE noise-aware loss. Was previously listed as a Snorkel application reference. Correct COSINE citation: arXiv:2010.07835 (Yu et al., NEEDS RE-VERIFICATION). | **Misidentified (research6 correction)** |
| arXiv:2511.13891 | Application paper | Uses Snorkel in pipeline; NOT an independent confirmation of the Snorkel algorithm | **Application paper (research4 C8 correction)** |
| LLM pre-labeling anchoring bias | Research4 finding C5 | Humans over-accept LLM suggestions, inflating agreement and depressing true reliability; magnitude measurable but unquantified -- requires TC's own calibration experiment | **Directional (thinnest-sourced item; TC measurement needed)** |
| Pairwise comparison for IAA | Research4 finding C7; k-DPP/BTL literature | Pairwise comparison yields higher inter-annotator agreement than absolute scoring for subjective quality | **Established (well-sourced in RLHF/DPO literature)** |

*5 verified sources, 3 established methods, 3 source corrections/clarifications, 1 directional finding requiring TC measurement. Last updated August 2026 (v6).*
