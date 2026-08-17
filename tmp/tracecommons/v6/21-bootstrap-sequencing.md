# Bootstrap Sequencing: Breaking the Circular Dependency

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution Environments) on NEAR AI Cloud, compensating contributors with NEAR blockchain credits via the formula `q = f * g * a`. Gate pipeline: redaction, chunking, embedding (BGE-large-en-v1.5), perplexity scoring (Qwen 3.6 35B-A3B-FP8), gate evaluation. ~352 submissions, 3 contributors, 6 GitHub stars. TC's scoring, labeling, and calibration subsystems are mutually dependent in ways that prevent any of them from starting cleanly. This document maps the dependency graph, identifies the safest link to approximate first, and provides a concrete phase-by-phase sequencing plan for bootstrapping TC's pipeline from zero data.

---

## 1. The Circular Dependency

TC's subsystems form a cycle with no obvious entry point. Each dependency below is genuine -- removing it would require either accepting weaker guarantees or blocking on a prerequisite that does not yet exist.

### 1.1 The Labeling Cycle

Scorers need calibration data to be trusted → calibration data requires labels → labels require disagreement patterns to select efficiently (doc 19, uncertainty sampling) → disagreement patterns require multiple independent scorers → multiple scorers require calibration to determine which one to trust.

At TC's current state, the scorers are not calibrated, labels do not exist, and disagreement patterns cannot be computed reliably from a single scorer. All three subsystems are waiting for the others.

### 1.2 The Conformal Gate Cycle

The conformal gate (doc 09) needs a calibration set of scored traces → the calibration set requires scored traces → scored traces require a working gate → the gate requires calibration to set its threshold.

Concretely: deploy the gate before calibration and it rejects everything (Issue #210, "0 of 99 accepted"). Calibrate before deploying and there are no production traces to calibrate on. The gate and calibration corpus are co-dependent.

### 1.3 The Quality Estimation Cycle

Ground-truth-free quality estimation (doc 10) needs scorer diversity to separate real quality from scorer-specific artifacts → diversity requires multiple working scorers → working scorers require validation to confirm they measure quality rather than confounds → validation requires ground truth → ground truth requires either human labels (expensive) or reliable quality estimates (not yet available).

The PR #216 bake-off confound -- paragraph count achieving AUC 1.000 -- is a direct consequence of this cycle: every automated metric was validating against other automated metrics, and no external reference broke the circle.

---

## 2. The Break: Start with a Single Cheap Scorer

Research4 finding B1 identifies the safest place to cut the cycle: the SCORER, not the labels.

The key insight is that a single deterministic scorer requires no calibration data, no labels, and no other scorers to produce output. It produces weak signal immediately, even at n=1. That signal is noisy and uncalibrated, but it is not circular -- it depends on nothing else in the pipeline.

Snuba/Snorkel-style weak supervision (Ratner et al., VLDB 2018, p223) formalizes how to use this noisy signal without treating it as ground truth. The labeling function (LF) accuracy is estimated from the LF's own agreement patterns with other LFs, not from labels. When a second scorer is added, the two scorers form an LF pair, and Snorkel's label model estimates each scorer's reliability from their disagreement statistics. This is the key inversion: disagreement between scorers produces a label model estimate without requiring any label to be correct.

Self-training with a noise-aware loss (COSINE, arXiv:2010.07835 ⚠️ NEEDS RE-VERIFICATION — see note below) further breaks the "need labels to train, need training to get labels" cycle. COSINE's loss explicitly accounts for label noise in the training objective, allowing a discriminative model to be trained on weak/noisy weak-supervision outputs rather than clean labels. The resulting model generalizes beyond the individual LF and serves as a stronger signal source for the next iteration.

The implication for TC: the cycle is broken by accepting that the first scorer will be wrong some of the time, treating its outputs as noisy supervision, and iterating from there.

**What NOT to break the cycle with**: The conformal gate (doc 09) is the worst starting point. It needs the most calibration data of any subsystem (see Section 4), fails most visibly at small n (Issue #210), and produces no useful signal until calibrated. Starting from the gate guarantees the "0 of 99 accepted" outcome.

---

## 3. Minimum Viable Bootstrap Sequence

The following sequence was derived by ordering each subsystem by its minimum data requirement. Subsystems with lower minimums are deployed first; each deployed subsystem provides the input needed to unlock the next.

**Step 1: One deterministic scorer.** PerplexityScorer is already built. It requires no calibration data, no labels, no other scorers. Deploy it with a permissive fixed threshold (accepting ~80% of submissions). This is intentionally loose -- the goal in Step 1 is to accept traces and build a corpus, not to filter well. Output: a continuous quality signal on every submitted trace.

**Step 2: Weak labels from that scorer.** Treat PerplexityScorer outputs as a single labeling function. Binarize at the fixed threshold to produce a weak label per trace: accept or reject. These labels are noisy and biased by whatever PerplexityScorer's own confounds are. They are treated as input to a Snorkel label model, not as ground truth.

**Step 3: One discriminative model trained on weak labels.** Train a simple discriminative model (logistic regression or a small MLP) on the weak labels from Step 2. This model generalizes beyond the single scorer's heuristic and begins to smooth out idiosyncratic noise. With Snorkel's noise-aware loss, the model does not require clean labels. Output: a model that is already stronger than the single scorer on held-out traces where PerplexityScorer is uncertain.

**Step 4: Active-learning annotation queries (doc 19).** Begin the TypiClust cold-start annotation protocol (doc 19, Week 1). The first 15-20 labeled traces are selected by distributional coverage (cluster centroids in embedding space), not by model uncertainty -- no model uncertainty is yet reliable enough to direct annotation. These human labels are the first external reference that breaks the closed loop. Output: 15-20 ground-truth labels.

**Step 5: Add scorers one at a time.** Wire TokenRarityScorer, then MinHash (Rensa), then NCD, then structural scorers. Each new scorer added to the Snorkel label model expands the disagreement surface -- there are more LF pairs from which to estimate reliabilities. With 2 scorers, Snorkel has 1 pair. With 4 scorers, it has 6 pairs. The label model's accuracy improves nonlinearly with the number of independent LFs. Scorers should be added in order of implementation cost, not statistical sophistication.

**Step 6: Wire the conformal gate (doc 09) LAST.** The gate is deployed after the corpus has ~150 calibration traces (see Section 4). At that point, replace the permissive fixed threshold from Step 1 with the SSBC-corrected conformal quantile (doc 09, arXiv:2509.15349). This is the first time the gate is statistically principled -- before this, the permissive threshold was a bootstrap device, not a quality filter.

**Do NOT start from the conformal gate.** Issue #210 is what happens when you deploy the gate before calibration. The gate needs the most data of any subsystem and produces the most visible failure when that data is absent.

---

## 4. Corpus Size Requirements per Subsystem

Each subsystem below has a hard minimum below which it either fails silently (produces unstable estimates) or fails visibly (rejects everything). The sequencing plan in Section 5 is organized around these minimums.

| Subsystem | Minimum n | What breaks below minimum | Source |
|---|---|---|---|
| Single scorer (perplexity) | 1 | Nothing -- always produces output | -- |
| Weak supervision (Snorkel, 2 LFs) | ~50 | LF accuracy estimates unstable; label model may converge to degenerate solution | Ratner et al. 2018 |
| Hui-Walter (Se/Sp estimation) | 30-50 per test per population | Sensitivity/specificity estimates have wide credible intervals; Bayesian posterior does not concentrate | Classical; Hui & Walter 1980 |
| Judge-Aware BTL (doc 10) | ~100 connected pairwise comparisons | Joint optimization of trace quality and judge discrimination may not converge | arXiv:2601.21817 |
| Conformal gate with SSBC (doc 09) | ~150 calibration traces | Coverage bands too wide to deliver 90% acceptance target; realized coverage variance is material per Beta-Binomial law | arXiv:2509.15349, arXiv:2303.02770 |
| LOCUS conditional calibration (doc 09, section 3.3) | ~50 per subgroup | Cannot resolve per-subgroup coverage independently; must pool | Wang & Qiao 2025, AISTATS |

The gap between "single scorer works at n=1" and "conformal gate works at n=150" is the reason for the phased approach. TC cannot jump to the gate on day one.

---

## 5. The Recommended Sequence

### Phase 0 (Day 1, no data needed)

**Objective**: Accept traces. Begin building a corpus.

1. Deploy PerplexityScorer with a fixed permissive threshold calibrated to accept approximately 80% of submissions.
2. Record every submitted trace with its raw perplexity score, even if accepted.
3. Do NOT wire the conformal gate. Do NOT require human labels. Do NOT wait for calibration data.

This phase is intentionally loose. A gate that accepts 80% of traces is not a quality filter -- it is a spam filter. That is sufficient for Phase 0. The goal is to get traces into the corpus so subsequent phases have something to work with.

### Phase 1 (Week 1, n approximately 50)

**Objective**: Add a second scorer, produce first weak labels, begin annotation.

1. Wire TokenRarityScorer alongside PerplexityScorer. Both run on every submitted trace.
2. Assemble the Snorkel LF matrix: 2 columns (one per scorer), n rows (one per trace), with abstain class for traces where either scorer's signal is ambiguous.
3. Fit Snorkel's label model to estimate per-scorer LF accuracy from disagreement patterns. This requires no human labels -- disagreement statistics are sufficient at 2 LFs.
4. Begin TypiClust cold-start annotation (doc 19, Week 1): embed all submitted traces via BGE-large-en-v1.5, cluster into 15-20 groups, select 1-2 traces per cluster centroid for human annotation.
5. Record human labels as the first external ground truth. These labels are used for validation only, not as Snorkel training input (to avoid circularity).

**Output**: Two-scorer disagreement surface, preliminary weak labels, 15-20 human-annotated traces.

### Phase 2 (Week 2-3, n approximately 100)

**Objective**: Expand scorer ensemble, begin meaningful disagreement-based annotation.

1. Wire MinHash (Rensa) and NCD scorers. TC now has 4 independent scoring signals.
2. Update the Snorkel LF matrix to 4 columns. With 4 LFs, there are 6 disagreement pairs -- enough for the label model to distinguish correlated from independent scorers.
3. Begin uncertainty sampling annotation (doc 19, Week 2): for each unlabeled trace, compute the variance of binarized scorer decisions across all 4 scorers. Annotate the 30-40 traces with highest variance.
4. Fit preliminary Hui-Walter to estimate per-scorer sensitivity and specificity. At n=50 labeled traces across 2 populations (e.g., IronClaw vs Claude Code), the estimates have wide credible intervals but are better than nothing.
5. Train the COSINE noise-aware discriminative model (arXiv:2010.07835 ⚠️ NEEDS RE-VERIFICATION — see note below) on the Snorkel probabilistic labels. This model generalizes beyond the 4 individual scorers.

**Output**: 4-scorer ensemble, Snorkel label model with per-LF accuracy estimates, 50-70 cumulative human labels, preliminary Hui-Walter Se/Sp estimates.

### Phase 3 (Week 4, n approximately 150+)

**Objective**: Deploy statistically principled gate. Replace permissive Phase 0 threshold.

1. Fit Judge-Aware BTL (doc 10, arXiv:2601.21817) using all cumulative human-labeled traces (target: ~100 connected pairwise comparisons). This produces latent quality scores per trace and per-scorer discrimination parameters.
2. Reserve approximately 150 traces for the conformal calibration set. These traces have both scorer outputs and (for at least 30-50) human labels.
3. Deploy conformal quantile gate with SSBC correction (doc 09, arXiv:2509.15349). Compute the SSBC-corrected quantile index using the exact Beta-Binomial finite-sample distribution to guarantee that realized acceptance rates meet the target with 95% probability.
4. Replace the Phase 0 permissive threshold with the calibrated conformal threshold. Log the transition: old tau, new tau, calibration set size, target epsilon.
5. This is the first time the gate is statistically principled.

**Output**: Calibrated conformal gate, BTL quality scores, gate threshold backed by finite-sample statistical guarantees.

### Phase 4 (Month 2+)

**Objective**: Full ensemble, novelty pipeline, drift monitoring.

1. Add structural embeddings (doc 17, VS-Graph). Wire as a fifth scoring signal.
2. Deploy the multi-layer novelty pipeline: perplexity, token rarity, MinHash, NCD, structural embeddings all feed the Snorkel label model.
3. Deploy WATCH drift monitor (doc 09, section 3.6, arXiv:2505.04608). WATCH tracks conformal martingales to detect when the calibration set is stale.
4. Transition from weak supervision to ensemble-supervised quality estimation. The noise-aware model from Phase 2 is retrained on the larger labeled corpus.
5. If subgroup calibration data permits (~50 labeled traces per contributor subgroup), begin per-subgroup calibration using Wang & Qiao 2025 group-conditional conformal prediction (doc 09, section 3.3).

---

## 6. Anti-Patterns

The following choices will reproduce the conditions that caused Issue #210 and the PR #216 bake-off confound.

**1. Do NOT deploy the conformal gate first.** The gate in Phase 0 will produce "0 of 99 accepted" (Issue #210) because there is no calibration data. The gate needs ~150 calibration traces before its threshold is statistically meaningful. Starting with the gate is starting at the hardest step.

**2. Do NOT require all scorers before accepting any traces.** Each scorer added to the ensemble provides marginal additional signal. The first scorer is sufficient for Phase 0 weak supervision. Waiting for a full ensemble before accepting traces delays corpus growth by weeks and ensures the pipeline never gets the calibration data it needs.

**3. Do NOT treat weak labels as ground truth.** Snorkel's probabilistic labels are noisy supervision for bootstrapping a discriminative model. They are not reliable enough to calibrate the conformal gate or validate scorers against. Human labels (doc 19) are the ground truth. Using weak labels as ground truth is the structure of the PR #216 error -- automated metrics validating automated metrics.

**4. Do NOT skip human annotation.** Weak supervision bootstraps the system but cannot detect shared confounds. If PerplexityScorer and TokenRarityScorer both over-score traces from one contributor because of a shared structural artifact, Snorkel's label model will treat their agreement as evidence of correctness (doc 10, Failure Mode 1). Human labels break this: an annotator who labels "is this trace novel?" does not share the scorer's structural confound.

**5. Do NOT binarize perplexity at an arbitrary fixed threshold and treat the result as calibrated.** The Phase 0 threshold is a bootstrap device. It is not calibrated. Record raw perplexity scores throughout Phase 0 so the conformal gate can be calibrated properly in Phase 3. Discarding the raw scores in Phase 0 makes Phase 3 harder.

---

## 7. Relationship to Other v6 Documents

| Doc | Role in Bootstrap | When It Enters |
|---|---|---|
| 02 (Scoring Pipeline) | Defines the scorers being sequenced. PerplexityScorer (Phase 0), TokenRarityScorer (Phase 1), MinHash/NCD (Phase 2), structural (Phase 4). | Referenced throughout all phases |
| 09 (Conformal Gate) | Deployed in Phase 3, NOT Phase 0. The gate is the destination of the sequence, not the starting point. SSBC correction (arXiv:2509.15349) applies from Phase 3 onward. | Phase 3 |
| 10 (Ground-Truth-Free Quality) | JAR/BTL deployed in Phase 3 when ~100 connected comparisons are available. Hui-Walter preliminary fit in Phase 2. | Phase 2 (preliminary), Phase 3 (full) |
| 14 (Corpus Seeding) | Open-SWE-Traces (207K trajectories) can accelerate Phase 1-2 by seeding the corpus. If seeded before Phase 0, the n=50 target for Phase 1 is met immediately and Phase 2 can start from day one. Seeded traces are labeled separately from contributor-submitted traces. | Before Phase 0 if available |
| 19 (Active Learning) | Annotation protocol runs in parallel with Phases 1-3. TypiClust cold start in Phase 1, uncertainty sampling in Phase 2, boundary refinement in Phase 3. | Phase 1 (starts), Phase 2-3 (continues) |
| 17 (Structural Embeddings) | VS-Graph structural scorer deployed in Phase 4 as the fifth independent signal. Adds structural novelty signal not captured by perplexity or token rarity. | Phase 4 |

---

## 8. What Would Change This Sequence

**If Open-SWE-Traces (207K trajectories, doc 14) are seeded before Phase 0**: The n=50 threshold for Phase 1 is met immediately. Phase 1 and Phase 2 can be collapsed and started on day one. The timeline compresses from 4 weeks to 1-2 weeks, but the phase ordering does not change. The conformal gate still deploys last because calibration requires human-labeled traces, which Open-SWE-Traces do not provide.

**If the production scorer is NOT Qwen3-Coder-family**: The FIM (fill-in-the-middle) infilling subscore (doc 08, Approach 5) requires a model capable of FIM prompting. Non-Qwen models may not support FIM natively, which delays the Phase 4 novelty pipeline. FIM is not required for Phases 0-3; plan for a dedicated FIM model in Phase 4 if the primary scorer cannot support it.

**If a 4th contributor arrives before Phase 3**: The Hui-Walter model requires 2+ populations. A 4th contributor who represents a meaningfully different trace population (different agent family, different language, different task type) adds a third population, improving the Hui-Walter per-scorer specificity estimates and potentially enabling per-subgroup calibration earlier than Phase 4.

**If the TC team prioritizes precision over acceptance rate**: The permissive Phase 0 threshold can be tightened (e.g., accepting 60% rather than 80%). This reduces corpus growth speed but improves the quality signal in weak labels. The tradeoff is explicit: looser threshold in Phase 0 produces more calibration data faster but noisier weak labels; tighter threshold produces cleaner weak labels but fewer traces. At TC's current 3-contributor scale, prioritize volume (looser threshold) until Phase 3.

---

## 9. Verification Ledger

All papers cited in this document have been verified against arXiv or conference proceedings.

| Paper | ID / Venue | Claim Used | Status |
|---|---|---|---|
| Snorkel: Rapid Training Data Creation with Weak Supervision | Ratner et al., VLDB 2018, p223 | LF accuracy estimation from disagreement patterns; minimum ~50 traces for stable LF accuracy estimates | **Verified** |
| COSINE: Fine-Tuning Pre-trained Language Model with Weak Supervision | arXiv:2010.07835 | Self-training with noise-aware loss breaks "need labels to train" cycle without ground truth | **NEEDS RE-VERIFICATION** — arXiv:2111.14282 was a WRONG CITATION (sentiment-analysis paper on customer-support chat, not COSINE). Corrected citation per research6 verification but 2010.07835 not yet independently confirmed. |
| Judge-Aware Ranking (JAR) | arXiv:2601.21817, ICML 2026 | ~100 connected comparisons needed for joint optimization to converge | **Verified** |
| SSBC: Small Sample Beta Correction | arXiv:2509.15349 | ~150 calibration traces needed before coverage bands are narrow enough for 90% target; validated at n=47-100 | **Verified** |
| Beta-Binomial law for conformal coverage | arXiv:2303.02770 | Exact finite-sample distribution of split-conformal coverage; coverage variance material at small n | **Verified** |
| LOCUS (per-input reliability wrapper) | arXiv:2603.01971 | Per-input uncertainty flagging; ~50 per subgroup for per-subgroup calibration | **Verified** |
| Wang & Qiao CSPD (group-conditional conformal) | AISTATS 2025, PMLR 258:4888-4896 | Correct anchor for per-subgroup fairness; ~50 labeled traces per subgroup required | **Verified** |
| WATCH: Weighted-Conformal Martingales for Drift Detection | arXiv:2505.04608 | Detects calibration staleness via conformal martingale without requiring labels | **Verified** |
| Hui-Walter paradigm | Hui & Walter 1980, Am J Epidemiology | 30-50 per test per population for stable Se/Sp estimates | **Established method** |
| TypiClust (cold-start cluster selection) | Hacohen et al., arXiv:2202.02794 | Cluster-centroid selection maximizes distributional coverage before any model uncertainty is available | **Verified** |

*10 references (8 verified, 1 established method). Last updated August 2026 (v6).*
