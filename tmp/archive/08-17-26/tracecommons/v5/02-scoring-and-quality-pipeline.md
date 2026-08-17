# Scoring & Quality Pipeline

**Date**: August 2026

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving register of AI coding agent session traces. Quality and novelty are scored inside TEEs. The gate pipeline: chunk → per-chunk perplexity (`PerplexityScorer`) → embedding (BGE-large-en-v1.5) → cosine similarity vs HNSW index (`VectorIndex`, usearch) → gate decision. Credit formula: `q = f * g * a`. Key traits: `PerplexityScorer`, `TokenRarityScorer` (built but NOT wired into the live gate), `Embedder`, `VectorIndex`.

---

## 0. The Problem

TC prices traces using `q = f * g * a` where `f` is perplexity-derived, `g` is novelty-derived (embedding cosine distance against HNSW), and `a` is an anomaly penalty. If `q` falls below a floor, the contributor earns nothing. The scorer decides who gets paid.

The scorer selection is broken. PR #216 showed the A2.6 bake-off was confounded: six trivial baselines (paragraph count, line count, word count, byte count, distinct word count, mean word length) ALL beat the winning model. Paragraph count achieved AUC 1.000 because every duplicate had exactly 1 paragraph while novel files had 7-163. The corpus builder entangled format with novelty class.

Three consequences: (1) the production scorer is uncalibrated — floor thresholds, embedding model choice, and scorer selection were downstream of a leaky evaluation; (2) no human-annotated ground truth exists to calibrate against — the synthetic paraphrase corpus has a length confound (median ratio 0.282); (3) this is existential for TC — if the scorer cannot distinguish novel from derivative, the credit mechanism rewards the wrong contributions.

The gate pipeline architecture is well-designed. The traits are clean plugin points. The problem is the measurement methodology, gaps in scoring dimensions, and absence of ground truth.

---

## A. Short-Term: Fix What's Broken (Weeks)

Changes using existing code, traits, and infrastructure. No new models or research dependencies.

### A.1 Wire TokenRarityScorer into the Live Gate

`TokenRarityScorer` computes `exp(-mean(K rarest logprobs))` from the same forward pass that produces perplexity. The trait, implementation, mock scorer, and `global_rarity_micros_across_chunks` aggregation all exist. `EnclaveGateOrchestrator::evaluate` does not call it.

**Builds on:** `LocalPerplexityScorer` already produces per-token logprobs. Rarity is a sort + mean on the same vector — no additional inference.

**Effort:** Hours. Plumbing in `evaluate` to call the aggregation and propagate the result.

**Validation:** Run bake-off on a corrected corpus (A.3) with both perplexity and rarity. Compare AUC. Even without a corrected corpus, having rarity in production audit rows enables retrospective analysis once human annotations exist.

The diagnostic value is independent of whether rarity improves the gate: high perplexity + low rarity = incoherent noise. High perplexity + high rarity = genuinely rare tokens in coherent context. This decomposition distinguishes novelty from noise.

### A.2 MinHash Dedup Layer via Rensa

MinHash fingerprints (a few hundred bytes) from each trace's rendered text. Jaccard estimate above 0.85-0.95 = near-duplicate. Catches verbatim and near-verbatim copies before the expensive embedding path.

**Builds on:** Rensa crate (Rust MinHash, 608x faster than Python datasketch). Fingerprint stored alongside vector index entry. `evaluate` gets a pre-filter before the embedding step: short-circuit on near-duplicate or pass through.

**Effort:** 1-2 days for prototype.

**Validation:** MinHash has analytically known false-positive rates. Verify on a sample of 100 production trace pairs: compute MinHash Jaccard and embedding cosine, plot correlation. If they catch different classes of duplicates, the signals are complementary.

**Concern:** Agent traces share boilerplate (system prompts, standard tool invocations). Shingle at the paragraph/event level rather than token level to prevent boilerplate from dominating Jaccard estimates.

### A.3 Fix the Bake-Off Corpus

The current corpus conflates format with novelty class. Any new model comparison on it is meaningless.

Rebuild with stratification as a design-time invariant: every class (novel, duplicate, paraphrase, near-duplicate) must have examples at every length quintile and paragraph-count range. The synthetic paraphrase generator must enforce length-matching: reject outputs where `|len_out/len_in - 1| > 0.2`.

Stratification invariants (must hold before any scorer comparison runs):

```
For each class C in {novel, duplicate, paraphrase, near-duplicate}:
  For each length quintile Q in {Q1..Q5}: count(C, Q) >= 10
  For each paragraph-count bin B in {1, 2-5, 6-20, 21+}: count(C, B) >= 5
  mean_length(C) within 20% of mean_length(overall)
```

**Effort:** ~1 week. Hard part is sourcing traces that satisfy constraints.

**Validation:** Before running any scorer comparison, compute AUC of every trivial baseline on the new corpus. Every baseline AUC must be below 0.6. If any exceeds 0.7, the corpus has a leak. This is PR #216's insight operationalized as a pre-flight check.

### A.4 Start Human Annotation (200+ Traces, 3+ Reviewers)

Without human judgment on "is this trace novel?", every automated metric validates against other automated metrics. This is the circular reasoning that let the confounded bake-off go undetected.

Minimum: 200+ production traces, stratified by length, event count, tenant, and current novelty score. 3+ independent reviewers per trace. Task: "Have you seen a trace substantially similar to this one?" Options: yes (link it), no, skip.

**Builds on:** PR #173 Phase 2 (corpus map + trace triage). `render_event_text` already produces human-readable output.

**Effort:** 40-80 person-hours for 200 traces across 3 reviewers.

**Validation:** Krippendorff's Alpha. Below 0.67 = task too ambiguous, revise guidelines. Above 0.8 = labels usable as ground truth.

Ranking-based annotation ("is A more novel than B?") produces higher inter-annotator agreement than absolute scoring but costs more labor per comparison. Use ranking if the goal is a continuous novelty score; absolute labels if the goal is binary classification.

The hard part is defining "novel." Labeling guidelines need worked examples covering: same approach + new context, new approach + same context, structurally identical + semantically different.

---

## B. Medium-Term: Build a Real Scoring Pipeline (Months)

These require new implementations or dependencies. They assume A.1-A.4 has produced a corrected bake-off corpus and human-annotated ground truth.

### B.1 Multi-Layer Novelty Pipeline

Layer cheap fast filters before expensive ones. Each layer is a separate trait implementation with early short-circuiting.

```
Layer 1: MinHash / LSH dedup           (< 1ms, from A.2)
Layer 2: NCD via zstd                  (< 10ms, see B.2)
Layer 3: Embedding distance            (existing path, improved per B.7)
Layer 4: Structural / process mining   (see B.3)
Layer 5: LLM perplexity               (existing scorer)
```

Each layer either short-circuits ("duplicate, stop") or passes through. Final decision is AND of all verdicts (fail-closed). Orchestrator sorts layers by estimated latency.

**Builds on:** `EnclaveGateOrchestrator` already sequences perplexity and embedding checks. Each new layer is a trait impl plugging into the existing orchestrator.

**Effort:** Trait definition + orchestrator changes: days. Each layer is independent work (B.2-B.6). Threshold calibration requires the annotated corpus from A.4.

**Validation:** On annotated corpus: per-layer AUC, short-circuit rate, false-positive rate. Goal is maximizing pipeline AUC while minimizing the fraction reaching expensive layers.

### B.2 NCD via zstd as Pre-Filter

NCD uses a compressor as a Kolmogorov complexity proxy: `NCD(x,y) = (C(xy) - min(C(x),C(y))) / max(C(x),C(y))`. Parameter-free, O(n), TEE-compatible.

For each incoming trace, compute `NCD(trace, sample_i)` against sampled corpus entries. Minimum NCD across samples is a cheap novelty signal.

**Dictionary pre-training angle:** zstd supports dictionaries pre-trained on representative data. A per-tenant dictionary makes NCD more discriminative — the compressor "knows" the tenant's common patterns, so genuinely novel traces compress poorly relative to the dictionary. Rebuild periodically in the offline worker.

**Builds on:** `zstd` Rust crate. Slots in as Layer 2 before embedding path.

**Effort:** Days for prototype. Engineering: sampling strategy, dictionary management, threshold selection.

**Validation:** On annotated corpus: NCD AUC vs embedding cosine AUC. If NCD catches different duplicates than embeddings, they are complementary. If redundant, NCD adds only speed.

### B.3 Process Mining: Tool-Call DAG Conformance

Agent traces are structured processes. The current pipeline embeds rendered text, capturing content but discarding control flow. Two traces calling the same tools in the same order with different arguments are structurally identical but might score as "novel."

Approach: extract `(event_type, tool_name)` pairs, build a frequency-weighted directly-follows graph (DFG) per tenant from historical traces, score new traces by the fraction of transitions unseen in the DFG.

**Builds on:** The chunker already parses event structure via `parse_envelope_rendered_events`. Structural information is extracted but not used for scoring.

**Effort:** 1-2 weeks. DFG construction = counting bigrams. Questions: cold-start for new tenants, weighting structural vs content novelty, handling variable-length traces.

**Validation:** On annotated corpus: structural conformance correlation with human labels. Hypothesis: structural conformance adds predictive power beyond embedding distance.

### B.4 Failure Attribution as a Scoring Dimension

The pipeline scores quality and originality but has no mechanism for failure traces. A trace that fails in a novel way is diagnostic (reveals failure modes others can avoid). A trace that fails in a known way is redundant.

Approach: (1) classify trace as succeeded/failed from outcome metadata (already available), (2) for failed traces, attribute the failure step (heuristic: last tool call before error; or LLM-based; or AgenTracer-style trained model), (3) score failure novelty as rarity of the attributed failure mode against a per-tenant failure-mode frequency table.

This adds a third dimension: content-novel, structurally-novel, and/or failure-novel.

**Builds on:** Trace envelope includes outcome metadata. Chunker segments into events.

**Effort:** Heuristic attributor: 1 day. LLM-based: more expensive but more accurate.

**Validation:** Separate annotation effort: 50+ failed traces, reviewers identify root-cause step, measure inter-annotator agreement on attribution.

### B.5 Compound System Auto-Optimization of Gate Thresholds

TC's gate pipeline is a compound AI system with hand-tuned (or EMA-adapted) thresholds per module. The compound-system optimization literature (EMNLP 2025) shows how to auto-optimize such pipelines jointly rather than per-module.

Simple implementation: treat the pipeline as a function from trace to gate decision with k thresholds. Use Bayesian optimization (GP surrogate via `argmin` crate) to maximize AUC on the annotated corpus.

Sophisticated implementation: when a human reviewer disagrees with the gate, propagate that disagreement to the specific layer and adjust its threshold.

**Builds on:** Adaptive gate thresholds (EMA per rung) and LinUCB self-learning already exist. This generalizes them to joint optimization.

**Effort:** Bayesian optimization: days. Feedback propagation: 1-2 weeks.

### B.6 Sub-Trace Decomposition for Fine-Grained Scoring

Traces are not atomic — they contain orchestration decisions (task decomposition, delegation) and execution tactics (tool use, error handling). Whole-trace embedding conflates these.

Approach: decompose each trace into sub-units by role (orchestration, execution, communication), embed and score each fragment type separately, dedup at appropriate granularity. Two traces sharing orchestration but differing in execution get partial novelty credit.

**Builds on:** Chunker already segments into typed events. Decomposition is classification of events into categories by event type (tool_call = execution, user_message = communication, etc.). `Embedder` called per-fragment. `VectorIndex` needs per-fragment-type namespaces.

**Effort:** 1-2 weeks.

### B.7 Improved Embeddings

BGE-large-en-v1.5 is general-purpose. Agent traces contain code, shell commands, API calls, structured data. Four improvements ordered by effort:

**Matryoshka embeddings (days).** Models like nomic-embed-text-v1.5 support truncated dimensions — 64 dims for coarse filter in early layers, full 768 for final score. No model training required.

**Code-aware embeddings (days).** Replace BGE with CodeBERT, GraphCodeBERT, or StarEncoder. Better duplicate detection for semantically equivalent code across languages.

**Contrastive fine-tuning on TC data (1-2 weeks).** SimCSE/CoSENT on the annotated corpus to maximize distance between novel pairs and minimize distance between duplicate pairs. Probably the single highest-leverage embedding improvement. Requires labeled data from A.4.

**Multi-view embedding (1-2 weeks).** Embed each trace view (NL content, code, tool-call structure, temporal ordering) separately. Final novelty = minimum across views. More expensive (N embeddings per trace) but catches traces novel in text but derivative in structure.

---

## C. Long-Term: Research-Backed Upgrades (Quarters)

These require new infrastructure or computational resources. They assume the medium-term pipeline produces calibrated novelty scores. Each finding is verified against primary sources (see verification ledger in §F).

### C.1 Label-Free Quality Scoring (Judge-Aware Ranking)

**Source:** Judge-Aware Ranking Framework (Xu, Tan, Wu, Zhou; arXiv 2601.21817; ICML 2026 — verified). Extends Bradley-Terry-Luce with judge-specific discrimination parameters, jointly estimating latent quality and judge reliability from pairwise comparisons without reference labels; proves identifiability + consistency, yields calibrated uncertainty. Warns that naive equal-weighting makes evaluation "more confidently wrong."

**Complemented by:** Hui-Walter Bayesian estimator (arXiv 2401.09376 — verified). Estimates classifier sensitivity/specificity with no gold standard from cross-classified agreement across ≥2 populations of differing prevalence.

**TC implementation:** Treat each candidate scorer as a "judge." Feed pairwise trace comparisons into the judge-aware BTL model. Weight the ensemble by estimated reliability. Partition submissions into ≥2 prevalence-differing populations (e.g., by contributor cohort or model family) to run Hui-Walter for sensitivity/specificity of each gate — all offline, no human labels.

**Impact:** Removes the PR #216 confound at the root and produces defensible, calibrated quality numbers. **Highest-impact single research finding.**

### C.2 Conformal Prediction on Scores

**Source:** TECP (Xu & Lu, 2025; arXiv 2509.00461 — verified). Token-entropy nonconformity + split conformal → prediction sets with finite-sample coverage; logit-free, works black-box.

**TC use:** Wrap novelty/quality scores in conformal intervals ("95% coverage"). Recalibrate per contributor population.

**Caveat:** Split CP assumes exchangeability — pair with covariate-shift-aware CP for drifting populations.

**Impact:** TC ships "top-15% novelty, 95% coverage" instead of a bare 0.73. Credibility upgrade for grant reviewers and consumers.

### C.3 Causal Failure Attribution

**Source:** Causal Agent Replay (Jaineet Shah; arXiv 2606.08275 — verified, open-source). SCM + `do()`-resample under the same stochastic policy; "point-of-commitment" locus rule; Monte-Carlo Shapley for interactions. Validated on synthetic ground truth. Also: CausalFlow (arXiv 2605.25338 — verified), single-agent interventional Causal Responsibility Score + minimal ranked repairs.

Correlational LLM-judge attribution scores only **~14%** step-level accuracy on Who&When; CAR replaces it.

**TC use:** Run CAR in the offline "dream" worker on failed traces; store a per-step causal-importance vector; expose "decisive step" as a premium annotation and as a feature for the novelty scorer.

**Impact:** Replaces ~14%-accurate correlational attribution with intervention-grounded scores. A new premium trace label.

### C.4 Safety-Preserving Compression

**Source:** TRACE (arXiv 2606.00611 — verified, open-source). Compressor-Reader latent evidence state; +12.6pp across ASSEBench/Pre-Ex-Bench/R-Judge; robust as context grows. Related: TRACES (2605.27690), terminal-observation compression (2604.19572).

**TC use:** Store full traces cold; index/query the TRACE latent state hot; keep the ability to detect sparse/delayed/compositional risk after compression.

**Impact:** 10-50x storage savings while retaining safety signal that is TC's differentiator. Caveat: streaming/incremental compression is future work per the authors.

### C.5 Marginal Value Scoring

**Source:** CausalMix (Tang et al., Tsinghua; arXiv 2607.01104 — verified). Mixture-as-treatment, data-pool features as covariates, CATE via 512 proxy runs, extrapolated to 7B; beats RegMix and crucially generalizes across shifting/unseen pools (RegMix-D independently flagged the same staleness failure).

**TC use:** Implement marginal-value scoring as state-conditioned CATE over corpus features; feed it into the VCG credit function. Moves from "is this trace good?" to "given the current corpus state, what is the marginal return of adding it?"

**Impact:** Directly prices redundancy. Individually-excellent-but-redundant traces get low marginal credit — exactly the behavior TC's incentive design needs.

### C.6 Specification Mining → Pattern Vocabulary

**Source:** Mining Beyond the Bools (arXiv 2603.06710 — verified). Synthesizes temporal + relational invariants (finite TSL) from traces. Anchor: Daikon (dynamic invariant detection). Related: AgentSpec (ICSE 2026), VeriGuard, Causal Past Logic (arXiv 2605.20923 — verified), TraceFix (arXiv 2605.07935 — verified).

**TC use:** Mine a library of temporal-logic invariants from the corpus. Tag each trace with the patterns it satisfies/violates. Novelty = "exhibits a pattern not yet in the vocabulary."

**Impact:** Explainable, auditable novelty. Natural bridge to runtime-verification consumers and EU AI Act Art. 12 logging.

### C.7 Influence-Function Valuation (LoGra/LogIX)

**Source:** LoGra (Choe et al., NeurIPS 2025; arXiv 2405.13954). LogIX library (Apache 2.0): 6,500x/5x compute/memory improvements vs. EKFAC at Llama3-8B scale. For-Value (Deng et al., ACL 2026; arXiv 2508.10180): forward-only influence estimation.

**TC use:** Select target eval set (e.g., SWE-Bench). Fine-tune small model on TC corpus sample. Use LogIX to compute each trace's influence on eval set. Use influence score as `quality_proxy` in credit formula and VCG mechanism.

**Impact:** No other trace marketplace prices by measured downstream impact — Ocean and Vana price by volume, Bittensor by mining difficulty. TC would be the first.

### C.8 Automated Skill Extraction

**Source:** RHO (arXiv 2606.05922): 19% absolute gain on SWE-Bench Pro without validation labels via single retrospective pass over unlabeled trajectories. Also: ReasoningBank, SkillOS (arXiv 2605.06614), Dynamic Cheatsheet.

**TC use:** Pipeline: cluster traces by task embedding → identify common sub-procedures via decomposition → run retrospective extraction (RHO pattern) → format as SKILL.md → score through gate before publication.

**Impact:** Turns TC from a passive data lake into an active capability supplier. "Your traces taught an extraction pipeline, which produced a skill adopted by 47 developers this month, and you earn credit from that adoption."

---

## D. The Unifying Formula

`novelty = harmonic_mean(originality, quality)`

**Originality signals (multi-layer pipeline):**

| Signal | Source | Section |
|---|---|---|
| MinHash Jaccard distance | Rensa | A.2 |
| NCD distance | zstd | B.2 |
| Embedding cosine distance | BGE / fine-tuned | B.7 |
| DFG conformance deviation | Process mining | B.3 |
| LLM perplexity | PerplexityScorer | existing |
| Token rarity | TokenRarityScorer | A.1 |
| ACU-level novelty | NovAScore | C.6 |
| Sub-trace fragment novelty | LEGOMem decomposition | B.6 |

**Quality signals:**

| Signal | Source | Section |
|---|---|---|
| Downstream model influence | LoGra / LogIX | C.7 |
| Skill extractability | RHO / ReasoningBank | C.8 |
| Failure diagnostic value | AgentDebugX | B.4 |
| Trace outcome | Envelope metadata | existing |

The harmonic mean prevents gaming: random noise is maximally "original" but zero quality (HM = 0). A templated high-quality trace is high quality but zero originality (HM = 0). Only traces that are both original AND impactful score high.

---

## E. Sequential Dependencies

```
A.3 (fix corpus) ─────────────────────┐
                                      v
A.4 (annotations) ──────────────> B.5 (auto-optimize)
                                      │
A.1 (TokenRarity) ──┐                │
A.2 (MinHash) ─────>├──> B.1 (multi-layer) ──> B.7 (embeddings)
B.2 (NCD) ─────────>│        │
B.3 (process mining)┘        │
                              │
B.4 (failure attr) ──────────>├──> C.8 (skill extraction)
                              │
                              └──> C.7 (influence)
                                       │
                                       v
                                  C.4 (compression)
                                       │
                                       v
                                  C.6 (spec mining)
```

Critical path: A.3/A.4 → B.1 → B.5 → C.1 (label-free scoring).

---

## F. Verification Ledger

| Item | Status |
|---|---|
| 2601.21817 Judge-Aware Ranking (ICML 2026) | **Verified** |
| 2509.00461 TECP | **Verified** |
| 2401.09376 Hui-Walter Bayesian | **Verified** |
| 2606.08275 Causal Agent Replay | **Verified** + open-source |
| 2605.25338 CausalFlow | **Verified** |
| 2606.00611 TRACE | **Verified** + open-source |
| 2607.01104 CausalMix (mixtures) | **Verified** (distinct from 2603.03587) |
| 2603.06710 Mining Beyond the Bools | **Verified** |
| 2605.20923 Causal Past Logic | **Verified** |
| 2605.07935 TraceFix | **Verified** |
| 2510.05566 domain-shift CP | NOT verified — deep-sweep target |

**Caveats:**
- Several headline numbers (+12.6pp, ~14% baseline, CausalMix vs RegMix) come from single papers, not independently replicated.
- Influence/causal-replay methods need model internals or re-execution ability — apply to open-weight/self-hosted targets, not black-box-API-only traces.
- Label-free methods rest on assumptions (conditional independence, ≥2 populations, exchangeability) that TC must validate on real submissions.

---

## G. Risk Assessment

| Idea | Risk of Wasted Effort | Risk of Not Doing It |
|---|---|---|
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
| C.1 Label-free scoring | Low (principled) | Confounded scorer persists |
| C.5 Marginal value | Medium (infra) | Heuristic pricing |
| C.7 Influence functions | High (GPU, benchmark-dependent) | Heuristic pricing |
| C.8 Skill extraction | Medium (adoption uncertain) | Corpus stays passive |

---

## H. Deep Research Queries: Scoring & Quality

### Q-S1: Scoring Without Ground Truth at Scale

```
"quality scoring" OR "data quality estimation" "without labels" OR "label-free" LLM traces 2025 2026
```
**Looking for:** Methods beyond Judge-Aware Ranking and Hui-Walter for estimating quality without ground truth. Are there production systems doing this? What scale do they operate at? What failure modes have they encountered?

### Q-S2: Agent Trace Deduplication at Scale

```
"deduplication" OR "near-duplicate detection" "agent traces" OR "execution traces" scalable 2025 2026
```
**Looking for:** How are large trace corpora being deduplicated? What works beyond MinHash for structured (non-text) traces? Any approaches that handle the boilerplate-vs-novelty problem (shared system prompts, common tool invocations)?

### Q-S3: Novelty Scoring for Structured Sequential Data

```
"novelty detection" "sequential data" OR "event sequences" "tool calls" OR "execution" 2025 2026
```
**Looking for:** Novelty scoring methods designed for structured sequential data (not free text). Process mining, sequence alignment, graph-based approaches. What works when traces are structurally similar but semantically different?

### Q-S4: Real-Time Quality Gates for Data Pipelines

```
"real-time quality gate" OR "online data quality" scoring pipeline production 2025 2026
```
**Looking for:** Production systems that score incoming data quality in real-time with multi-stage pipelines. How do they handle latency vs accuracy tradeoffs? How do they calibrate without labeled data? What's the state of the art for adaptive thresholds?

### Q-S5: Trace Compression State of the Art

```
"trajectory compression" OR "trace compression" OR "context compression" LLM agent safety 2025 2026
```
**Looking for:** Beyond TRACE — what other approaches exist for compressing agent trajectories while preserving task-critical information? Are there streaming/incremental approaches? What compression ratios are achievable in practice?

---

## I. Open Questions

1. **Trace type distribution in production?** If 90% are short single-tool interactions, MinHash + token rarity suffices. If long multi-step sessions, the full pipeline is needed.

2. **Perplexity-embedding correlation?** If highly correlated, one is redundant. Testable on production data today.

3. **Can annotators reliably distinguish novel from not-novel?** If Krippendorff's alpha < 0.5, "novelty" may be too subjective, and TC falls back on objective proxies (influence, skill extractability).

4. **Influence computation cost at TC scale?** LoGra is 6,500x cheaper than EKFAC, but absolute cost depends on model size, corpus size, GPU capacity.

5. **Does structural novelty add power beyond content novelty?** If tool-call sequences are highly variable even for derivative traces, structural conformance is noise.

6. **What failure-mode rarity threshold makes a failed trace "valuable"?** The mapping from root cause to diagnostic value is itself a judgment call.
