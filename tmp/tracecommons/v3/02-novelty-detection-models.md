# Novelty Detection Models for TraceCommons

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving register of AI coding
agent session traces. Quality and novelty are scored inside TEEs. The gate pipeline:
chunk -> per-chunk perplexity (`PerplexityScorer`) -> embedding (BGE-large-en-v1.5) ->
cosine similarity vs HNSW index (`VectorIndex`, usearch) -> gate decision. Credit formula:
`q = f * g * a`. Key traits: `PerplexityScorer`, `TokenRarityScorer` (built but NOT wired
into the live gate), `Embedder`, `VectorIndex`.

Technical brainstorming. Ideas organized by time horizon: short-term fixes (weeks),
medium-term pipeline (months), long-term valuation (quarters).

---

## 0. The Problem

TC prices traces using `q = f * g * a` where `f` is perplexity-derived, `g` is
novelty-derived (embedding cosine distance against HNSW), and `a` is an anomaly penalty.
If `q` falls below a floor, the contributor earns nothing. The scorer decides who gets paid.

The scorer selection is broken. PR #216 showed the A2.6 bake-off was confounded: six trivial
baselines (paragraph count, line count, word count, byte count, distinct word count, mean
word length) ALL beat the winning model. Paragraph count achieved AUC 1.000 because every
duplicate had exactly 1 paragraph while novel files had 7-163. The corpus builder entangled
format with novelty class.

Three consequences: (1) the production scorer is uncalibrated -- floor thresholds, embedding
model choice, and scorer selection were downstream of a leaky evaluation; (2) no human-annotated
ground truth exists to calibrate against -- the synthetic paraphrase corpus has a length confound
(median ratio 0.282); (3) this is existential for TC -- if the scorer cannot distinguish novel
from derivative, the credit mechanism rewards the wrong contributions.

The gate pipeline architecture is well-designed. The traits are clean plugin points. The problem
is the measurement methodology, gaps in scoring dimensions, and absence of ground truth.

---

## A. Short-Term: Fix What's Broken (Weeks)

Changes using existing code, traits, and infrastructure. No new models or research dependencies.

### A.1 Wire TokenRarityScorer into the Live Gate

`TokenRarityScorer` computes `exp(-mean(K rarest logprobs))` from the same forward pass that
produces perplexity. The trait, implementation, mock scorer, and `global_rarity_micros_across_chunks`
aggregation all exist. `EnclaveGateOrchestrator::evaluate` does not call it.

**Builds on:** `LocalPerplexityScorer` already produces per-token logprobs. Rarity is a
sort + mean on the same vector -- no additional inference.

**Effort:** Hours. Plumbing in `evaluate` to call the aggregation and propagate the result.

**Validation:** Run bake-off on a corrected corpus (A.3) with both perplexity and rarity.
Compare AUC. Even without a corrected corpus, having rarity in production audit rows enables
retrospective analysis once human annotations exist.

The diagnostic value is independent of whether rarity improves the gate: high perplexity +
low rarity = incoherent noise. High perplexity + high rarity = genuinely rare tokens in
coherent context. This decomposition distinguishes novelty from noise.

### A.2 MinHash Dedup Layer via Rensa

MinHash fingerprints (a few hundred bytes) from each trace's rendered text. Jaccard estimate
above 0.85-0.95 = near-duplicate. Catches verbatim and near-verbatim copies before the
expensive embedding path.

**Builds on:** Rensa crate (Rust MinHash). Fingerprint stored alongside vector index entry.
`evaluate` gets a pre-filter before the embedding step: short-circuit on near-duplicate or
pass through.

**Effort:** 1-2 days for prototype.

**Validation:** MinHash has analytically known false-positive rates. Verify on a sample of
100 production trace pairs: compute MinHash Jaccard and embedding cosine, plot correlation.
If they catch different classes of duplicates, the signals are complementary.

**Concern:** Agent traces share boilerplate (system prompts, standard tool invocations).
Shingle at the paragraph/event level rather than token level to prevent boilerplate from
dominating Jaccard estimates.

### A.3 Fix the Bake-Off Corpus

The current corpus conflates format with novelty class. Any new model comparison on it is
meaningless.

Rebuild with stratification as a design-time invariant: every class (novel, duplicate,
paraphrase, near-duplicate) must have examples at every length quintile and paragraph-count
range. The synthetic paraphrase generator must enforce length-matching: reject outputs where
`|len_out/len_in - 1| > 0.2`.

Stratification invariants (must hold before any scorer comparison runs):

```
For each class C in {novel, duplicate, paraphrase, near-duplicate}:
  For each length quintile Q in {Q1..Q5}: count(C, Q) >= 10
  For each paragraph-count bin B in {1, 2-5, 6-20, 21+}: count(C, B) >= 5
  mean_length(C) within 20% of mean_length(overall)
```

**Effort:** ~1 week. Hard part is sourcing traces that satisfy constraints.

**Validation:** Before running any scorer comparison, compute AUC of every trivial baseline
on the new corpus. Every baseline AUC must be below 0.6. If any exceeds 0.7, the corpus
has a leak. This is PR #216's insight operationalized as a pre-flight check.

### A.4 Start Human Annotation (200+ Traces, 3+ Reviewers)

Without human judgment on "is this trace novel?", every automated metric validates against
other automated metrics. This is the circular reasoning that let the confounded bake-off go
undetected.

Minimum: 200+ production traces, stratified by length, event count, tenant, and current
novelty score. 3+ independent reviewers per trace. Task: "Have you seen a trace substantially
similar to this one?" Options: yes (link it), no, skip.

**Builds on:** PR #173 Phase 2 (corpus map + trace triage). `render_event_text` already
produces human-readable output.

**Effort:** 40-80 person-hours for 200 traces across 3 reviewers.

**Validation:** Krippendorff's Alpha. Below 0.67 = task too ambiguous, revise guidelines.
Above 0.8 = labels usable as ground truth.

Ranking-based annotation ("is A more novel than B?") produces higher inter-annotator agreement
than absolute scoring but costs more labor per comparison. Use ranking if the goal is a
continuous novelty score; absolute labels if the goal is binary classification.

The hard part is defining "novel." Labeling guidelines need worked examples covering: same
approach + new context, new approach + same context, structurally identical + semantically
different.

---

## B. Medium-Term: Build a Real Scoring Pipeline (Months)

These require new implementations or dependencies. They assume A.1-A.4 has produced a
corrected bake-off corpus and human-annotated ground truth.

### B.1 Multi-Layer Novelty Pipeline

Layer cheap fast filters before expensive ones. Each layer is a separate trait implementation
with early short-circuiting.

```
Layer 1: MinHash / LSH dedup           (< 1ms, from A.2)
Layer 2: NCD via zstd                  (< 10ms, see B.2)
Layer 3: Embedding distance            (existing path, improved per B.6)
Layer 4: Structural / process mining   (see B.3)
Layer 5: LLM perplexity               (existing scorer)
```

Each layer either short-circuits ("duplicate, stop") or passes through. Final decision is
AND of all verdicts (fail-closed). Orchestrator sorts layers by estimated latency.

**Builds on:** `EnclaveGateOrchestrator` already sequences perplexity and embedding checks.
Each new layer is a trait impl plugging into the existing orchestrator.

**Effort:** Trait definition + orchestrator changes: days. Each layer is independent work
(B.2-B.6). Threshold calibration requires the annotated corpus from A.4.

**Validation:** On annotated corpus: per-layer AUC, short-circuit rate, false-positive rate.
Goal is maximizing pipeline AUC while minimizing the fraction reaching expensive layers.

### B.2 NCD via zstd as Pre-Filter

NCD uses a compressor as a Kolmogorov complexity proxy:
`NCD(x,y) = (C(xy) - min(C(x),C(y))) / max(C(x),C(y))`. Parameter-free, O(n), TEE-compatible.

For each incoming trace, compute `NCD(trace, sample_i)` against sampled corpus entries. Minimum
NCD across samples is a cheap novelty signal.

**Dictionary pre-training angle:** zstd supports dictionaries pre-trained on representative data.
A per-tenant dictionary makes NCD more discriminative -- the compressor "knows" the tenant's
common patterns, so genuinely novel traces compress poorly relative to the dictionary. Rebuild
periodically in the offline worker.

**Builds on:** `zstd` Rust crate. Slots in as Layer 2 before embedding path.

**Effort:** Days for prototype. Engineering: sampling strategy, dictionary management, threshold
selection.

**Validation:** On annotated corpus: NCD AUC vs embedding cosine AUC. If NCD catches different
duplicates than embeddings, they are complementary. If redundant, NCD adds only speed.

### B.3 Process Mining: Tool-Call DAG Conformance

Agent traces are structured processes. The current pipeline embeds rendered text, capturing
content but discarding control flow. Two traces calling the same tools in the same order with
different arguments are structurally identical but might score as "novel."

Approach: extract `(event_type, tool_name)` pairs, build a frequency-weighted directly-follows
graph (DFG) per tenant from historical traces, score new traces by the fraction of transitions
unseen in the DFG.

**Builds on:** The chunker already parses event structure via `parse_envelope_rendered_events`.
Structural information is extracted but not used for scoring.

**Effort:** 1-2 weeks. DFG construction = counting bigrams. Questions: cold-start for new
tenants, weighting structural vs content novelty, handling variable-length traces.

**Validation:** On annotated corpus: structural conformance correlation with human labels.
Hypothesis: structural conformance adds predictive power beyond embedding distance.

### B.4 Failure Attribution as a Scoring Dimension

The pipeline scores quality and originality but has no mechanism for failure traces. A trace
that fails in a novel way is diagnostic (reveals failure modes others can avoid). A trace that
fails in a known way is redundant.

Approach: (1) classify trace as succeeded/failed from outcome metadata (already available),
(2) for failed traces, attribute the failure step (heuristic: last tool call before error;
or LLM-based; or AgenTracer-style trained model), (3) score failure novelty as rarity of the
attributed failure mode against a per-tenant failure-mode frequency table.

This adds a third dimension: content-novel, structurally-novel, and/or failure-novel.

**Builds on:** Trace envelope includes outcome metadata. Chunker segments into events.

**Effort:** Heuristic attributor: 1 day. LLM-based: more expensive but more accurate.

**Validation:** Separate annotation effort: 50+ failed traces, reviewers identify root-cause
step, measure inter-annotator agreement on attribution.

**Extension:** Failed-trace bundles (scrubbed failure-diagnosis-repair) are a natural TC
product line, overlapping with skill extraction (C.2).

### B.5 Compound System Auto-Optimization of Gate Thresholds

TC's gate pipeline is a compound AI system with hand-tuned (or EMA-adapted) thresholds per
module. The compound-system optimization literature (EMNLP 2025) shows how to auto-optimize
such pipelines jointly rather than per-module.

Simple implementation: treat the pipeline as a function from trace to gate decision with k
thresholds. Use Bayesian optimization (GP surrogate) to maximize AUC on the annotated corpus.

Sophisticated implementation: when a human reviewer disagrees with the gate, propagate that
disagreement to the specific layer and adjust its threshold.

**Builds on:** Adaptive gate thresholds (EMA per rung) and LinUCB self-learning already exist.
This generalizes them to joint optimization.

**Effort:** Bayesian optimization: days (using `argmin` crate). Feedback propagation: 1-2 weeks.

**Validation:** Train/test split on annotated corpus. Jointly-tuned thresholds vs
hand-tuned vs independently-tuned per-module. If joint tuning wins on test set, it adds value.

### B.6 Sub-Trace Decomposition for Fine-Grained Scoring

Traces are not atomic -- they contain orchestration decisions (task decomposition, delegation)
and execution tactics (tool use, error handling). Whole-trace embedding conflates these.

Approach: decompose each trace into sub-units by role (orchestration, execution, communication),
embed and score each fragment type separately, dedup at appropriate granularity. Two traces
sharing orchestration but differing in execution get partial novelty credit.

**Builds on:** Chunker already segments into typed events. Decomposition is classification of
events into categories by event type (tool_call = execution, user_message = communication,
etc.). `Embedder` called per-fragment. `VectorIndex` needs per-fragment-type namespaces.

**Effort:** 1-2 weeks. Fragment boundary detection is ambiguous for mixed events; start with
rule-based classification and measure error rate before considering LLM classification.

**Validation:** Targeted annotation: for 50+ traces, reviewers identify which aspects are
novel (orchestration, execution, or both). If reviewers can reliably distinguish, per-fragment
scoring adds value.

### B.7 Improved Embeddings

BGE-large-en-v1.5 is general-purpose. Agent traces contain code, shell commands, API calls,
structured data. Four improvements ordered by effort:

**Matryoshka embeddings (days).** Models like nomic-embed-text-v1.5 support truncated
dimensions -- 64 dims for coarse filter in early layers, full 768 for final score. No model
training required.

**Code-aware embeddings (days).** Replace BGE with CodeBERT, GraphCodeBERT, or StarEncoder.
Better duplicate detection for semantically equivalent code across languages.

**Contrastive fine-tuning on TC data (1-2 weeks).** SimCSE/CoSENT on the annotated corpus
to maximize distance between novel pairs and minimize distance between duplicate pairs.
Probably the single highest-leverage embedding improvement. Requires labeled data from A.4.

**Multi-view embedding (1-2 weeks).** Embed each trace view (NL content, code, tool-call
structure, temporal ordering) separately. Final novelty = minimum across views. More expensive
(N embeddings per trace) but catches traces novel in text but derivative in structure.

**Builds on:** `Embedder` trait is the plugin point. All four are different implementations.

**Validation:** AUC on annotated corpus, new embedding vs old. Highest AUC wins. If a simpler
approach matches contrastive fine-tuning, prefer it for lower complexity.

---

## C. Long-Term: Impact-Based Valuation (Quarters)

These require new infrastructure or computational resources. They assume the medium-term
pipeline produces calibrated novelty scores. Goal: move from "is this novel?" to "is this
valuable?"

### C.1 Influence-Function Valuation (LoGra/LogIX)

Score traces by measured downstream impact instead of intrinsic properties: "this trace,
when included in training data, improved eval performance by X on benchmark Y."

LoGra makes influence-function data valuation tractable at LLM scale via Kronecker-factored
projection (6,500x compute, 5x memory improvement over EKFAC). LogIX library (Apache 2.0)
provides the implementation. For-Value (Deng et al., ACL 2026) reduces cost further via
single forward pass (less accurate, orders of magnitude cheaper).

**Workflow:** Select target eval set (e.g., SWE-Bench for coding traces). Fine-tune a small
model on TC corpus sample. Use LogIX to compute each trace's influence on the eval set.
Use influence score as `quality_proxy` in credit formula and VCG mechanism.

**Builds on:** Dream consolidation worker for offline computation. Credit formula's quality
factor (`f`) is the replacement target.

**Effort:** Weeks to months. Fine-tuning pipeline, LogIX integration, GPU-hours for corpus,
validation.

**Validation:** Compare two credit strategies on same corpus. Strategy A: `q = f * g * a`.
Strategy B: influence-based pricing. Fine-tune models on top-K traces by each. Measure
downstream eval. If influence-priced traces produce better models per dollar, the pricing
is justified.

Influence-based pricing ties value to a specific model and benchmark. The credit formula
may need `quality = mean(influence_across_targets)` or `max(...)` across a representative set.

### C.2 Skill Extraction from High-Influence Traces

Traces are raw material; skills are refined product. A trace yielding a reusable skill
(procedure, code pattern, debugging strategy) is more valuable than one that does not.

Approaches: ReasoningBank (distill strategies from successes), SkillOS (RL-trained curator),
RHO (improve the harness from retrospective pass over unlabeled trajectories -- 19% gain on
SWE-Bench Pro). Publish in SKILL.md format (Agent Skills open standard, Linux Foundation).

Creates a second credit surface: contributors earn when skills extracted from their traces
are adopted downstream.

**Builds on:** Dream consolidation worker for offline extraction. TC provenance can extend
to skills (quality scoring, toxic skill detection -- ToxicSkills found 36.82% of scanned
Agent Skills had at least one security flaw).

**Effort:** Weeks to months. RHO is the most practical starting point (unlabeled trajectories,
concrete implementation).

**Validation:** (a) Extraction quality: inject skills into agent context, measure task
completion on benchmark. (b) Adoption rate: product metric.

### C.3 Sleep-Time Pre-Computation

Move expensive operations from the synchronous gate path to offline idle windows: embeddings
(async on ingest), NCD dictionary rebuilds (periodic), DFG model updates (incremental),
influence scores (batch), skill candidate flagging (lightweight heuristic).

**Builds on:** Dream consolidation worker already runs offline corpus computations.

**Effort:** Per-component: ~1 day each. Cache invalidation is the tricky part.

**Validation:** Gate latency before vs after. Goal: synchronous path reduces to vector index
lookup + threshold check (milliseconds) regardless of scoring layer count.

### C.4 NovAScore ACU Decomposition

Decompose traces into Atomic Content Units (ACUs) -- smallest meaningful claims or actions.
Score each ACU for novelty against a historical ACU bank per tenant. Overall novelty =
salience-weighted aggregate of ACU novelty scores.

For agent traces, an ACU might be: a tool call with specific arguments, a decision to use
one approach over another, a novel tool combination, a recovery strategy after failure.

Addresses explainability: when the pipeline says "novelty score 0.73," ACU decomposition
identifies which specific content is novel.

**Builds on:** Chunker segments into events (each event = one or more ACUs). `Embedder`
for ACU-level comparison. `VectorIndex` stores historical ACU bank.

**Effort:** Weeks. ACU extraction (rule-based vs LLM-based) is the main challenge.

**Validation:** Explainability testing: for 50 traces, reviewers verify whether identified
novel ACUs are actually novel. High agreement = genuine explainability.

Synergy with B.4: ACU decomposition + failure attribution can pinpoint "the novel ACU that
caused the failure" -- the most diagnostic unit.

### C.5 Verifiable Scoring (VET-Style Composed Proofs)

Chain execution attestation (IronClaw TEE), scoring attestation (proof pipeline ran correctly),
and credit attestation (credit formula applied correctly) into an end-to-end verifiable proof.

External consumers verify scoring without trusting TC's API.

**Builds on:** IronClaw TEE bridge (merged), NEAR on-chain settlement, ZK range proof
infrastructure. Composed proof links these components.

**Effort:** Months. Individual proof components exist; composition requires protocol design.
ZKML for neural layers (BGE-large, 335M params) is impractical near-term. TEE-based
alternative (run scoring inside TEE) is more practical with different trust assumptions.

**Validation:** Binary: can an external party verify the score given only the proof and
public inputs? If yes, it works.

**Practical path:** Ship "verifiable trace" premium tier with IronClaw TEE covering both
execution and scoring. Full ZK composed proofs when ZKML matures.

---

## D. Novelty = f(Originality, Quality)

The unifying formula: `novelty = harmonic_mean(originality, quality)`.

**Originality signals (multi-layer pipeline):**

| Signal | Source | Section |
|--------|--------|---------|
| MinHash Jaccard distance | Rensa | A.2 |
| NCD distance | zstd | B.2 |
| Embedding cosine distance | BGE / fine-tuned | B.7 |
| DFG conformance deviation | Process mining | B.3 |
| LLM perplexity | PerplexityScorer | existing |
| Token rarity | TokenRarityScorer | A.1 |
| ACU-level novelty | NovAScore | C.4 |
| Sub-trace fragment novelty | LEGOMem decomposition | B.6 |

**Quality signals:**

| Signal | Source | Section |
|--------|--------|---------|
| Downstream model influence | LoGra / LogIX | C.1 |
| Skill extractability | RHO / ReasoningBank | C.2 |
| Failure diagnostic value | AgentDebugX | B.4 |
| Trace outcome | Envelope metadata | existing |

The harmonic mean prevents gaming: random noise is maximally "original" but zero quality
(HM = 0). A templated high-quality trace is high quality but zero originality (HM = 0).
Only traces that are both original AND impactful score high.

Weights are the thresholds that compound-system optimization (B.5) tunes jointly. The
annotated corpus (A.4) is the calibration target.

---

## E. Decision Framework

### E.1 What Signal Matters Most?

**"Stop paying for garbage":** Focus on dedup layers (A.2, B.2, B.3). MinHash and NCD
have known error rates, no labeled corpus needed.

**"Know whether our scorer works":** Focus on measurement (A.3, A.4, B.5). Without a
corrected corpus and annotations, every pipeline improvement is unmeasurable. If you can
only do one thing, do A.4.

**"Price traces by actual value":** Focus on influence functions (C.1) and skill extraction
(C.2). Most infrastructure, most defensible pricing.

**"Differentiate from competitors":** Focus on verifiable scoring (C.5), failure attribution
(B.4), process mining (B.3). Langfuse and Braintrust do not verify scoring cryptographically,
attribute failures to steps, or model tool-call sequences as processes.

### E.2 Corpus Size Considerations

**Small (< 1K traces/tenant):** MinHash/NCD less useful (small comparison set). Focus on
A.1, A.3, A.4.

**Medium (1K-100K):** MinHash, NCD, DFG all become useful. Focus on B.1-B.7.

**Large (> 100K):** Full pipeline essential for cost. Influence functions high-leverage.
Sleep-time pre-computation essential for latency. Focus on C.1-C.5.

### E.3 Team Bandwidth

**One engineer, part-time:** A.1 (hours), A.2 (1-2 days), A.4 (incrementally, 20 traces/week).
Skip B and C until annotation corpus exists.

**2-3 engineers, focused:** All of A in parallel, then B.1 as foundation, then pick one of
B.2-B.7 based on what annotations reveal.

**Team with ML capacity:** A and B in parallel. C.1 for pricing, C.2 for product
differentiation, C.5 for trust.

### E.4 Sequential Dependencies

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
B.4 (failure attr) ──────────>├──> C.2 (skill extraction)
                              │
                              └──> C.1 (influence)
                                       │
                                       v
                                  C.3 (sleep-time)
                                       │
                                       v
                                  C.4 (NovAScore)
                                       │
                                       v
                                  C.5 (verifiable scoring)
```

Critical path: A.3/A.4 -> B.1 -> B.5 -> C.1.

### E.5 Risk Assessment

| Idea | Risk of Wasted Effort | Risk of Not Doing It |
|------|----------------------|---------------------|
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
| C.1 Influence functions | High (GPU, benchmark-dependent) | Heuristic pricing |
| C.2 Skill extraction | Medium (adoption uncertain) | Corpus stays passive |
| C.3 Sleep-time compute | Low | Latency scales with layers |
| C.4 NovAScore ACU | Medium-high | Novelty not explainable |
| C.5 Verifiable scoring | High (ZKML immature) | Trust depends on TC API |

---

## F. Open Questions

1. **Trace type distribution in production?** If 90% are short single-tool interactions,
   MinHash + token rarity suffices. If long multi-step sessions, the full pipeline is needed.

2. **Perplexity-embedding correlation?** If highly correlated, one is redundant. Testable
   on production data today.

3. **Can annotators reliably distinguish novel from not-novel?** If Krippendorff's alpha
   < 0.5, "novelty" may be too subjective, and TC falls back on objective proxies (influence,
   skill extractability).

4. **Influence computation cost at TC scale?** LoGra is 6,500x cheaper than EKFAC, but
   absolute cost depends on model size, corpus size, GPU capacity.

5. **Does structural novelty add power beyond content novelty?** If tool-call sequences are
   highly variable even for derivative traces, structural conformance is noise.

6. **Is sub-trace decomposition worth the complexity?** If most traces lack a clear
   orchestration/execution distinction, decomposition adds complexity without signal.

7. **What failure-mode rarity threshold makes a failed trace "valuable"?** The mapping from
   root cause to diagnostic value is itself a judgment call.

---

## References

### Prior (not repeated in full)

- Li et al. (2004). NCD / Similarity Metric. IEEE TIT.
- Jiang et al. (2023). NCD with gzip. ACL 2023.
- Padmakumar et al. (2025). Novelty = harmonic_mean(originality, quality). ICLR 2025.
- Ai et al. (2025). NovAScore ACU decomposition. COLING 2025.
- van der Aalst (2016). Process Mining. Springer.
- Gao et al. (2021). SimCSE. EMNLP 2021.
- Rensa: Rust MinHash. github.com/beowolx/rensa

### New

- Choe et al. LoGra / LLM-Scale Data Valuation with Influence Functions. NeurIPS 2025.
  arXiv:2405.13954. LogIX: github.com/logix-project/logix
- Deng et al. For-Value: Forward-Only Influence. ACL 2026. arXiv:2508.10180.
- Ouyang et al. (2025). ReasoningBank. arXiv.
- SkillOS. arXiv:2605.06614.
- Suzgun et al. (2025). Dynamic Cheatsheet.
- SkillRevise. arXiv:2606.01139.
- RHO: Retrospective Harness Optimization. arXiv:2606.05922.
- Han et al. LEGOMem: Modular Procedural Memory. AAMAS 2026. arXiv:2510.04851.
- Compound AI Systems Optimization Survey. EMNLP 2025.
- Lin et al. Sleep-time Compute. Letta/Berkeley. arXiv:2504.13171.
- VET: Verifiable Execution Traces. arXiv:2512.15892.
- Zhu et al. AgentDebugX. arXiv:2607.18754.
- Zhang et al. AgenTracer-8B. ICLR 2026. arXiv:2509.03312.
- TRAIL: Span-Level Failure Taxonomy. arXiv:2505.08638.
- Who&When: Step-Level Attribution. ICML 2025 Spotlight.
- ToxicSkills. Snyk, Feb 2026.
- Agent Skills open standard. agentskills.io. Linux Foundation.
