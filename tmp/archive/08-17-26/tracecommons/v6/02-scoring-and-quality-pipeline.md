# Scoring & Quality Pipeline

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces. Quality and novelty are scored inside TEEs (Trusted Execution Environments -- hardware-isolated compute enclaves where code runs in encrypted memory). Gate pipeline: redaction -> chunking -> embedding (BGE-large-en-v1.5) -> cosine similarity vs HNSW index (VectorIndex, usearch) -> perplexity scoring (Qwen 3.6 35B-A3B-FP8, AUC > 0.93) -> gate evaluation. Key traits: `PerplexityScorer`, `TokenRarityScorer` (built but NOT wired into live gate), `Embedder`, `VectorIndex`. IronClaw: NEAR AI's agent runtime (12.6K stars), TC's primary integration partner (3 PRs merged). NEAR: Layer-1 blockchain for credit settlement and TEE-hosted scoring (NEAR AI Cloud provides Intel TDX + NVIDIA GPU TEE).

---

## 0. The Problem

TC prices traces using `q = f * g * a` where `f` is perplexity-derived quality, `g` is novelty-derived (embedding cosine distance against HNSW), and `a` is an anomaly penalty. If `q` falls below a floor, the contributor earns nothing. The scorer decides who gets paid.

The scorer selection is broken. PR #216 showed the A2.6 bake-off was confounded: six trivial baselines (paragraph count, line count, word count, byte count, distinct word count, mean word length) ALL beat the winning model. Paragraph count achieved AUC 1.000 because every duplicate had exactly 1 paragraph while novel files had 7-163. The corpus builder entangled format with novelty class.

Three consequences: (1) the production scorer is uncalibrated -- floor thresholds, embedding model choice, and scorer selection were downstream of a leaky evaluation; (2) no human-annotated ground truth exists to calibrate against -- the synthetic paraphrase corpus has a length confound (median ratio 0.282); (3) this is existential for TC -- if the scorer cannot distinguish novel from derivative, the credit mechanism rewards the wrong contributions.

The gate pipeline architecture is well-designed. The traits are clean plugin points. The problem is the measurement methodology, gaps in scoring dimensions, and absence of ground truth. Two additional bugs discovered since that analysis:

### Issue #210: "0 of 99 Sessions Would Be Accepted"

The gate rejects everything. A scoring logic inversion means 0 out of 99 test sessions pass the acceptance threshold. This is not a calibration problem -- it is a logic bug. Nothing downstream works until this is fixed. **Impact**: A developer who installs TC, scans 47 sessions, and sees "0 sessions accepted" will uninstall immediately. **Fix**: Identify and invert the logic gate. Hours of work once root cause is found.

### Issue #219: Redaction Penalizes Quality Scores

The perplexity scorer sees redaction markers as incoherent noise, lowering scores. This creates a perverse incentive: contributors who redact less get higher scores and more credits. IronClaw's redaction is particularly thorough, meaning IronClaw contributors are systematically disadvantaged. **Impact**: Undermines TC's privacy premise. **Fix**: Score on pre-redaction content (within TEE boundary), normalize by redaction density, or train scorer to treat redaction markers as neutral.

---

## A. Short-Term: Fix What's Broken (Weeks)

### A.1 Wire TokenRarityScorer

`TokenRarityScorer` computes `exp(-mean(K rarest logprobs))` from the same forward pass producing perplexity. Implementation exists. `EnclaveGateOrchestrator::evaluate` doesn't call it. Diagnostic value: high perplexity + low rarity = incoherent noise; high perplexity + high rarity = genuinely rare tokens in coherent context.

**Builds on**: `LocalPerplexityScorer` already produces per-token logprobs. Rarity is a sort + mean on the same vector -- no additional inference.

**Validation**: Run bake-off on a corrected corpus (A.3) with both perplexity and rarity. Compare AUC.

**Effort**: Hours.

### A.2 MinHash Dedup via Rensa

MinHash fingerprints from each trace's rendered text. Jaccard estimate above 0.85-0.95 = near-duplicate. Short-circuits before expensive embedding path. Rensa crate: 608x faster than Python datasketch. Shingle at paragraph/event level, not token level, to prevent boilerplate from dominating Jaccard estimates.

**Builds on**: Rensa crate (Rust MinHash). Fingerprint stored alongside vector index entry.

**Validation**: MinHash has analytically known false-positive rates. Verify on 100 production trace pairs: compute MinHash Jaccard and embedding cosine, plot correlation.

**Effort**: 1-2 days.

### A.3 Fix the Bake-Off Corpus

The current corpus conflates format with novelty class. Any new model comparison on it is meaningless. Rebuild with stratification as a design-time invariant: every class must have examples at every length quintile and paragraph-count range. The synthetic paraphrase generator must enforce length-matching: reject outputs where `|len_out/len_in - 1| > 0.2`.

Stratification invariants:

```
For each class C in {novel, duplicate, paraphrase, near-duplicate}:
  For each length quintile Q in {Q1..Q5}: count(C, Q) >= 10
  For each paragraph-count bin B in {1, 2-5, 6-20, 21+}: count(C, B) >= 5
  mean_length(C) within 20% of mean_length(overall)
```

**Validation**: Before running any scorer comparison, compute AUC of every trivial baseline on the new corpus. Every baseline AUC must be below 0.6. If any exceeds 0.7, the corpus has a leak.

**Effort**: ~1 week.

### A.4 Start Human Annotation (200+ Traces, 3+ Reviewers)

Without human judgment, every automated metric validates against other automated metrics. This circular reasoning let the confounded bake-off go undetected.

Minimum: 200+ production traces, stratified by length, event count, tenant, and current novelty score. 3+ independent reviewers. Task: "Have you seen a trace substantially similar?" The hard part is defining "novel" -- labeling guidelines need worked examples covering: same approach + new context, new approach + same context, structurally identical + semantically different.

**Builds on**: PR #173 Phase 2 (corpus map + trace triage). `render_event_text` already produces human-readable output.

**Validation**: Krippendorff's Alpha. Below 0.67 = task too ambiguous, revise guidelines. Above 0.8 = labels usable as ground truth.

**Effort**: 40-80 person-hours.

---

## B. Medium-Term: Build a Real Scoring Pipeline (Months)

Assumes A.1-A.4 has produced a corrected corpus and human-annotated ground truth.

### B.1 Multi-Layer Novelty Pipeline

```
Layer 1: MinHash / LSH dedup           (< 1ms, from A.2)
Layer 2: NCD via zstd                  (< 10ms, from B.2)
Layer 3: Embedding distance            (existing path, improved per B.7)
Layer 4: Structural / process mining   (from B.3)
Layer 5: LLM perplexity               (existing scorer)
```

Each layer short-circuits or passes through. Final decision is AND of all verdicts (fail-closed).

**Builds on**: `EnclaveGateOrchestrator` already sequences perplexity and embedding checks. Each new layer is a trait impl plugging into the existing orchestrator.

**Effort**: Trait definition + orchestrator changes: days. Each layer is independent work. Threshold calibration requires annotated corpus from A.4.

**Validation**: On annotated corpus: per-layer AUC, short-circuit rate, false-positive rate. Goal: maximize pipeline AUC while minimizing the fraction reaching expensive layers.

### B.2 NCD via zstd

`NCD(x,y) = (C(xy) - min(C(x),C(y))) / max(C(x),C(y))`. Parameter-free, O(n), TEE-compatible. zstd dictionary pre-training makes NCD more discriminative per tenant.

**Builds on**: `zstd` Rust crate. Slots in as Layer 2 before embedding path.

**Validation**: On annotated corpus: NCD AUC vs embedding cosine AUC. If they catch different duplicates, signals are complementary.

**Effort**: Days.

### B.3 Process Mining: Tool-Call DAG Conformance

The current pipeline embeds rendered text, discarding control flow. Two traces calling the same tools in the same order with different arguments are structurally identical but might score as "novel."

Research: Agent Behavior Mining (arXiv:2606.20669, BPM 2026) shows ~60% of sessions follow 5-7 canonical patterns. AgentLTL (arXiv:2607.02599) for LTL verification. PM4Py has LLM module.

**TC approach**: Extract `(event_type, tool_name)` pairs -> build frequency-weighted DFG per tenant -> score new traces by fraction of unseen transitions. Cold-start for new tenants: use global DFG until tenant has 50+ traces.

**Builds on**: Chunker already parses event structure via `parse_envelope_rendered_events`. Structural information is extracted but not used for scoring.

**Validation**: On annotated corpus: structural conformance correlation with human labels. Hypothesis: structural conformance adds predictive power beyond embedding distance.

**Effort**: 1-2 weeks.

### B.4 Failure Attribution as a Scoring Dimension

A trace that fails in a novel way is diagnostic. A trace that fails in a known way is redundant. Correlational LLM-judge attribution achieves only ~14% step-level accuracy (Who&When, arXiv:2505.00212, ICML 2025 Spotlight). Interventional approaches outperform:

| Method | Approach | Availability | arXiv |
|---|---|---|---|
| **Causal Agent Replay** | SCM + do()-resample + Shapley | Open-source | 2606.08275 |
| **CausalFlow** | Interventional CRS + minimal ranked repairs | Open-source | 2605.25338 |
| **AgenTracer-8B** | Trained attributor, beats Gemini-2.5-Pro | ICLR 2026 | 2509.03312 |

Zero-Replay Debugging (arXiv:2606.14805): Branch Recall@5 of 0.93 with zero LLM calls -- cheap pre-filter before causal methods.

**Builds on**: Trace envelope includes outcome metadata. Chunker segments into events.

**Validation**: 50+ failed traces, reviewers identify root-cause step, measure inter-annotator agreement.

**Extension**: Failed-trace bundles (scrubbed failure-diagnosis-repair sequences) are a natural TC product line, overlapping with skill extraction (C.11).

**Effort**: Heuristic: 1 day. LLM-based: 1-2 weeks.

### B.5 Auto-Optimization of Gate Thresholds

TC's gate pipeline is a compound AI system with hand-tuned thresholds per module. Simple: Bayesian optimization (GP surrogate via `argmin` crate) to maximize AUC on annotated corpus. Sophisticated: when a human reviewer disagrees with the gate, propagate that disagreement to the specific layer and adjust its threshold.

**Builds on**: Adaptive gate thresholds (EMA per rung) and LinUCB self-learning already exist. This generalizes them to joint optimization.

**Validation**: Train/test split on annotated corpus. Jointly-tuned vs hand-tuned vs independently-tuned per-module. If joint tuning wins on test set, it adds value.

**Effort**: Days for Bayesian opt; 1-2 weeks for feedback propagation.

### B.6 Sub-Trace Decomposition

Traces are not atomic -- they contain orchestration decisions (task decomposition, delegation) and execution tactics (tool use, error handling). Whole-trace embedding conflates these. Decompose each trace into sub-units by role (orchestration, execution, communication). Embed and score each fragment separately. Two traces sharing orchestration but differing in execution get partial novelty credit. A 200-step trace where 180 steps are boilerplate and 20 are genuinely novel currently gets a single averaged score -- decomposition lets the pipeline recognize the 20-step fragment.

**Builds on**: Chunker already segments into typed events. `Embedder` per-fragment. `VectorIndex` needs per-fragment-type namespaces.

**Validation**: Targeted annotation: for 50+ traces, reviewers identify which aspects are novel (orchestration, execution, or both). If reviewers can reliably distinguish, per-fragment scoring adds value.

**Effort**: 1-2 weeks. Start with rule-based classification; measure error rate before considering LLM classification.

### B.7 Improved Embeddings

Four improvements ordered by effort: (1) **Matryoshka embeddings** (days) -- truncated dims for coarse->fine filtering; (2) **Code-aware embeddings** (days) -- CodeBERT/GraphCodeBERT/StarEncoder; (3) **Contrastive fine-tuning** (1-2 weeks) -- SimCSE/CoSENT on annotated corpus, highest-leverage improvement; (4) **Multi-view embedding** (1-2 weeks) -- separate embeddings per trace view, final novelty = minimum across views.

**Builds on**: `Embedder` trait is the plugin point.

**Validation**: AUC on annotated corpus, new vs old embedding.

---

## C. Long-Term: Research-Backed Upgrades (Quarters)

Goal: move from "is this novel?" to "is this valuable?"

### C.1 Label-Free Quality Scoring (Judge-Aware Ranking)

**Source**: Judge-Aware Ranking (arXiv:2601.21817; ICML 2026). Extends BTL with judge-specific discrimination. Treat each candidate scorer as a "judge," weight ensemble by estimated reliability. Removes PR #216 confound at the root.

**Builds on**: Existing scorer trait implementations.

**Validation**: Single-scorer AUC vs JAR-weighted ensemble AUC on annotated corpus.

### C.2 Conformal Prediction on Scores

Distribution-free coverage guarantees. Directly applicable papers: ToolChain-CRC (arXiv:2606.18467) for tool-call chains; Role-Stratified CRC (arXiv:2607.24343) by agent role; Conformal Agent Error Attribution (arXiv:2605.06788) for failure attribution uncertainty; PASC (arXiv:2605.18812) for adaptive selective conformal. 300-1000 calibration traces sufficient (finite-sample formula: <= 0.33% overshoot at n=300).

**TC use**: Wrap scores in conformal intervals. Ship "top-15% novelty, 95% coverage" instead of bare 0.73.

**Builds on**: Existing scorer outputs. Conformal wrapper is post-processing.

**Validation**: If you claim 95% coverage, verify 95% of ground-truth labels fall within the interval.

### C.3 Causal Failure Attribution

Full causal version of B.4. Run CAR in the offline worker on failed traces. Store per-step causal-importance vector. Expose "decisive step" as a premium annotation.

**Builds on**: B.4 failure attribution output. Upgrades heuristic attribution to causal.

**Validation**: On 50+ annotated failed traces, compare causal attribution vs heuristic attribution vs human-identified root cause. Causal should match human judgment more often.

### C.4 NovAScore ACU Decomposition

Decompose traces into Atomic Content Units (ACUs) -- smallest meaningful claims or actions. Score each ACU for novelty against a historical ACU bank per tenant. Overall novelty = salience-weighted aggregate of ACU novelty scores.

For agent traces, an ACU might be: a tool call with specific arguments, a decision to use one approach over another, a novel tool combination, a recovery strategy after failure.

Addresses explainability: when the pipeline says "novelty score 0.73," ACU decomposition identifies which specific content is novel. This is the missing link between a numeric score and actionable feedback to contributors.

**Builds on**: Chunker segments into events (each event = one or more ACUs). `Embedder` for ACU-level comparison. `VectorIndex` stores historical ACU bank.

**Effort**: Weeks. ACU extraction (rule-based vs LLM-based) is the main challenge.

**Validation**: Explainability testing: for 50 traces, reviewers verify whether identified novel ACUs are actually novel. High agreement = genuine explainability. Synergy with B.4: ACU decomposition + failure attribution can pinpoint "the novel ACU that caused the failure" -- the most diagnostic unit.

### C.5 Verifiable Scoring

Chain execution attestation (IronClaw TEE), scoring attestation (proof pipeline ran correctly), and credit attestation (credit formula applied correctly) into an end-to-end verifiable proof. External consumers verify scoring without trusting TC's API.

**Builds on**: IronClaw TEE bridge (merged), NEAR on-chain settlement. ZKML for neural layers (335M params) impractical near-term; TEE-based alternative is practical now.

**Validation**: Can an external party verify the score given only the proof and public inputs? Ship "verifiable trace" premium tier with IronClaw TEE covering both execution and scoring.

### C.6 Marginal Value Scoring (CausalMix)

**Source**: CausalMix (arXiv:2607.01104). State-conditioned marginal value: "Given the current corpus, what is the marginal return of adding this trace?" Individually-excellent-but-redundant traces get low credit.

**Builds on**: Multi-layer pipeline (B.1) feature vectors as treatment covariates.

**Validation**: Compare marginal vs flat pricing on synthetic marketplace simulation.

### C.7 Data Valuation: Shapley Is Broken, Use VCG

Shapley-based data valuation is fundamentally gameable -- now definitively proven:
- **Shapley fragility** (arXiv:2504.05563, Claim 3): strategic misrepresentation inflates values.
- **Sybil attacks** (arXiv:2605.07663): splitting achieves 1.74x inflation.
- **Entire semivalue class** (arXiv:2506.12619): Banzhaf, beta-Shapley all inherit gameability.

VCG is DSIC -- truthful reporting is a dominant strategy. O(n log n) for homogeneous multi-unit. No production VCG deployment for data markets exists yet. Q-MIA (arXiv:2506.05379) provides budget-balanced alternative.

**TC implication**: `vcg_allocate` already built. Wire into credit settlement. Replace `q = f * g * a` with VCG where each trace's payment equals its externality.

**Builds on**: Existing `vcg_allocate`. **Validation**: Sybil test -- splitting one trace into two should not earn more under VCG.

**Scaling**: At 13/week, trivial. At 1000/week, requires sampled VCG or Myerson.

### C.8 Specification Mining -> Pattern Vocabulary

**Source**: Mining Beyond the Bools (arXiv:2603.06710). Mine temporal-logic invariant library from corpus. Novelty = "exhibits a pattern not yet in the vocabulary."

**Builds on**: Process mining DFG from B.3. **Validation**: Mine on 80%, test on 20%.

### C.9 Influence-Function Valuation (LoGra/LogIX)

**Source**: LoGra (arXiv:2405.13954, NeurIPS 2025). 6,500x/5x compute/memory improvements. Compute each trace's influence on eval set (e.g., SWE-Bench).

**Builds on**: Dream consolidation worker. **Validation**: Influence-priced top-K vs heuristic-priced top-K on downstream eval.

**Risk**: Highest risk of wasted effort. GPU-dependent, benchmark-sensitive. Do after VCG is working.

### C.10 Safety-Preserving Compression

| Method | Compression | Safety | arXiv |
|---|---|---|---|
| **TRACE** | 10-50x | +12.6pp safety bench | 2606.00611 |
| **ACE** (SambaNova, open-source) | 5-20x | Partial | 2606.31564 |
| **Slipstream** | +8.8pp, -39.7% latency | Not designed for safety | 2605.08580 |
| **CompactionRL** | 10-30x | Trained with safety reward | 2607.05378 |

**Critical finding**: Governance Decay (arXiv:2606.22528) shows compaction causes 0% -> 30-59% safety violations. Constraint Pinning (~47 tokens) restores 0%. Any compression TC deploys must include constraint pinning.

**Builds on**: Cold storage archival. Compressed alongside full traces, never replacing.

**Validation**: Compress 100 traces. Gate pipeline on compressed vs original. Safety signals must be preserved.

### C.11 Automated Skill Extraction

| Method | Result | arXiv |
|---|---|---|
| **RHO** | 59% -> 78% SWE-Bench Pro | 2606.05922 |
| **Trace2Skill** | +57.65pp | 2603.25158 |
| **AutoRefine** | Quality improvement via execution feedback | 2601.22758 |
| **SkillAudit** | Catches security issues | 2606.14239 |
| **MetaSkill-Evolve** | Long-horizon evolutionary improvement | 2607.05297 |

**TC pipeline**: Cluster -> identify sub-procedures -> RHO extraction -> SkillAudit scan -> SKILL.md -> gate.

**Builds on**: Sub-trace decomposition (B.6). Dream consolidation worker.

**Validation**: (a) Inject skills, measure benchmark completion. (b) Adoption rate. (c) Security scan.

---

## D. The Unifying Formula

`novelty = harmonic_mean(originality, quality)`

The harmonic mean prevents gaming: random noise is maximally "original" but zero quality (HM = 0). A templated high-quality trace is high quality but zero originality (HM = 0). Only traces that are both original AND impactful score high.

**Originality signals:**

| Signal | Source | Section |
|---|---|---|
| MinHash Jaccard distance | Rensa | A.2 |
| NCD distance | zstd | B.2 |
| Embedding cosine distance | BGE / fine-tuned | B.7 |
| DFG conformance deviation | Process mining | B.3 |
| LLM perplexity | PerplexityScorer | existing |
| Token rarity | TokenRarityScorer | A.1 |
| ACU-level novelty | NovAScore | C.4 |
| Sub-trace fragment novelty | Decomposition | B.6 |

**Quality signals:**

| Signal | Source | Section |
|---|---|---|
| Downstream model influence | LoGra / LogIX | C.9 |
| Skill extractability | RHO / Trace2Skill | C.11 |
| Failure diagnostic value | CAR / CausalFlow | B.4 |
| Trace outcome | Envelope metadata | existing |
| Marginal corpus value | CausalMix CATE | C.6 |

**Credit mechanism**: VCG (not Shapley) for incentive compatibility (C.7). **Uncertainty**: Conformal prediction intervals on all scores (C.2). Weights tuned jointly by compound-system optimization (B.5) against annotated corpus (A.4).

---

## E. Decision Framework

### E.1 What Signal Matters Most?

**"Stop paying for garbage":** Dedup layers (A.2, B.2, B.3). Known error rates, no labeled corpus needed.

**"Know whether our scorer works":** Measurement (A.3, A.4, B.5). Without annotations, every improvement is unmeasurable. If you can only do one thing, do A.4.

**"Price traces by actual value":** Influence functions (C.9) and skill extraction (C.11). Most infrastructure, most defensible.

**"Differentiate from competitors":** Verifiable scoring (C.5), failure attribution (B.4), process mining (B.3). Langfuse and Braintrust do not verify scoring cryptographically, attribute failures to steps, or model tool-call sequences as processes.

### E.2 Corpus Size Considerations

**Small (< 1K traces/tenant):** MinHash/NCD less useful (small comparison set). Focus on A.1, A.3, A.4.

**Medium (1K-100K):** MinHash, NCD, DFG all become useful. Focus on B.1-B.7.

**Large (> 100K):** Full pipeline essential for cost. Influence functions high-leverage. Sleep-time pre-computation essential for latency.

### E.3 Team Bandwidth

**One engineer, part-time:** A.1 (hours), A.2 (1-2 days), A.4 (incrementally, 20 traces/week). Skip B and C until annotation corpus exists.

**2-3 engineers, focused:** All of A in parallel, then B.1 as foundation, then pick one of B.2-B.7 based on what annotations reveal.

**Team with ML capacity:** A and B in parallel. C.9 for pricing, C.11 for product differentiation, C.5 for trust.

---

## F. Sequential Dependencies

```
#210 fix ──────────────────────────────────┐
#219 fix ──────────────────────────────────┤
                                           v
A.3 (fix corpus) ─────────────────────┐   ALL
                                      v
A.4 (annotations) ──────────────> B.5 (auto-optimize)
                                      |
A.1 (TokenRarity) ──┐                |
A.2 (MinHash) ─────>├──> B.1 (multi-layer) ──> B.7 (embeddings)
B.2 (NCD) ─────────>|        |
B.3 (process mining)┘        |
                              |
B.4 (failure attr) ──────────>├──> C.11 (skill extraction)
                              |
                              └──> C.9 (influence)
                                       |
C.7 (VCG mechanism) ──────────────────>|──> credit formula
                                       |
                                       v
                                  C.10 (compression)
                                       |
                                       v
                                  C.8 (spec mining)
```

**Critical path**: #210/#219 -> A.3/A.4 -> B.1 -> B.5 -> C.1 (label-free scoring).

---

## G. Open Questions

1. **Trace type distribution in production?** If 90% are short single-tool interactions, MinHash + token rarity suffices. If long multi-step sessions, the full pipeline is needed.
2. **Perplexity-embedding correlation?** If highly correlated, one is redundant. Testable on production data today.
3. **Can annotators reliably distinguish novel from not-novel?** If alpha < 0.5, "novelty" may be too subjective; fall back on objective proxies (influence, skill extractability).
4. **Influence computation cost at TC scale?** LoGra is 6,500x cheaper than EKFAC, but absolute cost depends on model size, corpus size, GPU capacity.
5. **Does structural novelty add power beyond content novelty?** If tool-call sequences are highly variable even for derivative traces, structural conformance is noise.
6. **Is sub-trace decomposition worth the complexity?** If most traces lack a clear orchestration/execution distinction, decomposition adds complexity without signal.
7. **What failure-mode rarity threshold makes a failed trace "valuable"?** The mapping from root cause to diagnostic value is itself a judgment call.

---

## H. Risk Assessment

| Idea | Risk of Wasted Effort | Risk of Not Doing It |
|---|---|---|
| **#210 gate logic fix** | **Near zero** | **Project-killing** |
| **#219 redaction penalty fix** | **Near zero** | **Trust-destroying** |
| A.1 Wire TokenRarity | Near zero | Free information left on table |
| A.2 MinHash dedup | Low | Expensive layers process easy duplicates |
| A.3 Fix corpus | Low | Every model comparison meaningless |
| A.4 Human annotation | Medium (low agreement?) | No ground truth, circular metrics |
| B.1 Multi-layer pipeline | Low (modular) | Single-layer scoring leaves gaps |
| B.2 NCD pre-filter | Low | Miss structural duplicates |
| B.3 Process mining | Medium | Miss procedural duplicates |
| B.4 Failure attribution | Medium | Failure traces unscored |
| B.5 Auto-optimization | Low | Hand-tuned thresholds |
| B.6 Sub-trace decomp | Medium | Whole-trace conflates dimensions |
| B.7 Better embeddings | Low-medium | Weak on code-heavy traces |
| C.2 Conformal prediction | Low | Scores lack uncertainty quantification |
| C.4 NovAScore ACU | Medium-high | Novelty not explainable |
| C.5 Verifiable scoring | High (ZKML immature) | Trust depends on TC API |
| C.6 Marginal value (CausalMix) | Medium | Heuristic pricing |
| C.7 VCG over Shapley | Low (already built) | Gameable credit mechanism |
| C.9 Influence functions | High (GPU, benchmark-dependent) | Heuristic pricing |
| C.10 Safety compression | Medium | Latency scales with layers |
| C.11 Skill extraction | Medium (adoption uncertain) | Corpus stays passive |

---

## I. Verification Ledger

| Item | arXiv | Status |
|---|---|---|
| Judge-Aware Ranking (ICML 2026) | 2601.21817 | **Verified** |
| TECP | 2509.00461 | **Verified** |
| Hui-Walter Bayesian | 2401.09376 | **Verified** |
| Causal Agent Replay | 2606.08275 | **Verified** + open-source |
| CausalFlow | 2605.25338 | **Verified** |
| TRACE | 2606.00611 | **Verified** + open-source |
| CausalMix | 2607.01104 | **Verified** |
| Mining Beyond the Bools | 2603.06710 | **Verified** |
| AgenTracer-8B (ICLR 2026) | 2509.03312 | **Verified** |
| Who&When (ICML 2025 Spotlight) | 2505.00212 | **Verified** |
| LoGra/LogIX (NeurIPS 2025) | 2405.13954 | **Verified** |
| ToolChain-CRC | 2606.18467 | **Verified** |
| Role-Stratified CRC | 2607.24343 | **Verified** |
| Conformal Agent Error Attribution | 2605.06788 | **Verified** |
| PASC | 2605.18812 | **Verified** |
| Shapley fragility | 2504.05563 | **Verified** (Claim 3) |
| Sybil attack on Shapley | 2605.07663 | **Verified** (1.74x) |
| Semivalue class gameability | 2506.12619 | **Verified** |
| Q-MIA | 2506.05379 | **Verified** |
| Agent Behavior Mining (BPM 2026) | 2606.20669 | **Verified** |
| AgentLTL | 2607.02599 | **Verified** |
| Zero-Replay Debugging | 2606.14805 | **Verified** (Recall@5 0.93) |
| ACE (SambaNova, open-source) | 2606.31564 | **Verified** |
| Slipstream | 2605.08580 | **Verified** (+8.8pp, -39.7%) |
| CompactionRL | 2607.05378 | **Verified** |
| Governance Decay | 2606.22528 | **Verified** (0%->30-59%) |
| RHO | 2606.05922 | **Verified** (59%->78%) |
| Trace2Skill | 2603.25158 | **Verified** (+57.65pp) |
| AutoRefine | 2601.22758 | **Verified** |
| SkillAudit | 2606.14239 | **Verified** |
| MetaSkill-Evolve | 2607.05297 | **Verified** |

---

## J. Deep Research Queries

### Q-S1: Conformal Prediction for Agent Quality Scores

```
"conformal prediction" "agent" OR "tool chain" OR "LLM" quality calibration 2025 2026
```
How do ToolChain-CRC and related methods handle the exchangeability assumption when agent populations shift? What are practical coverage guarantees achievable with TC's corpus size (~352 traces)?

### Q-S2: Incentive-Compatible Data Valuation

```
"data valuation" "incentive compatible" OR "DSIC" OR "VCG" OR "Myerson" marketplace 2025 2026
```
Production systems using VCG/Myerson for data marketplace pricing instead of Shapley? How do they handle O(n^2) scaling? What approximations work?

### Q-S3: Process Mining for Agent Traces

```
"process mining" "agent" OR "LLM" trace analysis workflow 2025 2026
```
Applications of classical process mining to AI agent traces. What tools/libraries exist beyond PM4Py? What false-positive rates are typical for conformance-based novelty detection?

### Q-S4: Agent Failure Root Cause Attribution

```
"agent failure" "root cause" OR "attribution" OR "causal" interventional 2025 2026
```
Beyond CAR and CausalFlow -- other interventional failure attribution methods? Computational cost of do()-resampling at TC's scale? Attribution without re-executing the agent?

### Q-S5: Safety-Preserving Trajectory Compression

```
"trajectory compression" OR "trace compression" safety preserving OR lossless 2025 2026
```
Methods that compress agent trajectories while provably preserving safety-relevant signals. Theoretical compression limit while maintaining safety detection? Streaming/incremental approaches?
