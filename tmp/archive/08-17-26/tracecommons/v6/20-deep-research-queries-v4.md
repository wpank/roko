# Deep Research Queries v4

**Date**: August 2026

## What This Document Is

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces (~235K LOC, 6 crates). Traces are scored for quality and novelty inside TEEs (Trusted Execution Environments on NEAR AI Cloud) and contributors earn NEAR blockchain credits. ~352 submissions, 3 contributors, 6 GitHub stars. Built by Zaki Manian (co-creator of Cosmos SDK/IBC).

This is the fourth-generation query set, generated from gaps, open questions, unverified claims, and decision points identified across all 20 v6 documents (00-19). Prior queries (doc 07, 37 queries across 7 categories) are superseded where the research has been completed; retained where still open. This document adds **52 new queries** across 10 categories.

### Relationship to Doc 07

Doc 07 contains 37 queries generated from the initial 27-agent research sweep. Many of those queries were answered by the subsequent 12-agent deep research sweep (docs 08-19). This document:
- **Supersedes**: Q-S1 (answered by doc 09), Q-S2 (answered by doc 16), Q-S6 (answered by doc 10), Q-I1 (answered by doc 18), Q-I3 (answered by doc 11), Q-T3 (answered by doc 13), Q-G6 (answered by doc 14), Q-U2 (answered by doc 17), Q-U5 (answered by doc 12), Q-U6 (answered by doc 19)
- **Retains**: Q-S3, Q-S4, Q-S5, Q-S7, Q-S8, Q-I2, Q-I4, Q-I5, Q-G1-G5, Q-G7, Q-M1-M5, Q-T1, Q-T2, Q-T4, Q-T5, Q-U1, Q-U3, Q-U4, Q-X1-X6
- **Extends**: All retained queries with updated context from docs 08-19

---

## Category 1: Redaction-Invariant Scoring (5 queries)

From doc 08. Issue #219 research is done; implementation questions remain.

### Q-R1: TEE Attestation Model for Pipeline Reordering

```
"trusted execution environment" attestation "pipeline" OR "workflow" reordering "Intel TDX" scope enclave 2025 2026
```
**Looking for:** Can TEE attestation cover a reordered pipeline (score raw text, then redact) without re-attestation? Does NEAR AI Cloud's TDX attestation bind to specific step ordering, or to the full enclave binary? TC needs to know whether Approach 4 (score-on-raw-text-in-TEE) is threat-model-neutral.

**What we already know:** TC scoring runs in Intel TDX on NEAR AI Cloud. Approach 4 is the cleanest fix for #219 but requires that the TEE attestation permits raw text in enclave memory during scoring. No TC team member has audited the attestation scope.

**Decision it unblocks:** Doc 08, Phase 2-alt (TEE Raw Scoring) -- go/no-go.

### Q-R2: Qwen FIM Capabilities for Infilling Coherence

```
"Qwen" OR "Qwen3" "fill in the middle" OR "FIM" OR "infilling" capabilities benchmark 2025 2026
```
**Looking for:** Does Qwen 3.6 35B-A3B-FP8 (TC's production scorer) have native fill-in-the-middle capabilities? If yes, TC can add an infilling-coherence sub-score (doc 08, Approach 5) without deploying a second model in the TEE (which would double VRAM). If no, what's the smallest FIM-capable model that runs alongside Qwen in the same enclave?

**What we already know:** MARIA-7B achieves downstream perplexity 2.82 at low mask ratio (arXiv:2502.06901). AST-FIM (arXiv:2506.00204) computes FIM-based perplexity. Neither has been evaluated in TC's TEE environment. Deploying a second model doubles VRAM requirements on the NEAR AI Cloud enclave.

**Decision it unblocks:** Doc 08, Phase 3 model selection.

### Q-R3: Typed Placeholder Vocabulary Extension for Large Language Models

```
tokenizer "vocabulary extension" OR "vocab expansion" "special tokens" embedding initialization "large language model" 2025 2026
```
**Looking for:** Best practices for adding domain-specific special tokens (e.g., `<PERSON>`, `<API_KEY>`, `<SECRET>`) to an existing LLM's tokenizer without fine-tuning the full model. Embedding initialization strategies beyond mean-init. Impact on downstream task performance. TC needs to add ~10-15 placeholder tokens to Qwen's vocabulary for the typing approach (doc 08, Approach 2).

**What we already know:** Lehman et al. (arXiv:2302.08091) added de-identification tags to clinical LMs. Grounded vocabulary initialization is referenced (arXiv:2604.02324, arXiv:2604.16656) but these papers have not been independently verified against TC's use case. The gap: clinical de-id tags are semantically meaningful; TC's placeholder tokens are semantically empty (they mark absence, not presence).

### Q-R4: Surrogate Generation for Privacy-Preserving Scoring

```
"surrogate" OR "pseudonymization" "realistic" OR "synthetic" replacement "privacy preserving" scoring evaluation NLP 2025 2026
```
**Looking for:** Surrogate generation systems that produce realistic typed replacements for redacted content, specifically for scoring/evaluation contexts. Vakili et al. (DOI:10.1186/s12911-024-02546-8) showed 94.85% BERTScore with surrogates on clinical text. Does this transfer to code-heavy AI agent traces? Are there surrogate generators for code entities (function names, variable names, API endpoints)?

**What we already know:** SurrogateShield achieves 94.85% BERTScore vs 81.59% for placeholder redaction, but this is on clinical text, not agent traces. Wu et al. (arXiv:2511.15364) show ~20-67% information loss even with typed surrogates. No surrogate system targets code entities in agent session traces.

### Q-R5: Quantifying Redaction Penalty on Perplexity Scores

```
"redaction" OR "anonymization" OR "masking" impact perplexity "language model" scoring quantitative 2025 2026
```
**Looking for:** Empirical measurements of how redaction/anonymization affects perplexity-based quality scores. The Δ=−0.31 from arXiv:2603.29497 is for a privacy-risk metric, not perplexity directly. TC's claim that redaction shifts mean perplexity by 15-30% is an engineering estimate, not an empirical measurement. Is there published data on perplexity degradation from masking?

**What we already know:** arXiv:2603.29497 confirmed that "uninformed redaction disrupts coherence." But the mechanism mapping from privacy-risk metrics to TC's perplexity scorer is argued by analogy. The 5-8 subword tokenization and 8-12 nats per placeholder are TC engineering estimates, not published measurements.

---

## Category 2: Conformal Calibration & Drift (4 queries)

From doc 09. Core methods identified; operational questions remain.

### Q-CC1: Conformal Prediction at Very Small Calibration Sizes

```
"conformal prediction" "small sample" OR "finite sample" calibration size coverage 100 200 300 practical 2025 2026
```
**Looking for:** Practical experience with conformal prediction at calibration sizes of 100-500. TC has ~352 submissions. The finite-sample slack is 1/(n+1) ≈ 0.28%, but what are the empirical coverage properties at this scale? Do the theoretical guarantees hold in practice with heterogeneous data? Are there small-sample corrections beyond the standard 1/(n+1) bound?

**What we already know:** Standard theory gives overshoot ≤ 0.33% at n=300. But TC's data is heterogeneous (3 contributors, different agent families, different redaction levels). Exchangeability violations at small n may cause coverage degradation that the asymptotic bound doesn't capture.

### Q-CC2: Per-Subgroup Conformal Calibration with Limited Data

```
"conditional coverage" OR "group-conditional" conformal prediction "small groups" subgroup calibration 2025 2026
```
**Looking for:** LOCUS (arXiv:2603.01971) formalizes per-subgroup calibration, but TC's subgroups (per-contributor, per-agent-family) may have <50 traces each. What are minimum subgroup sizes for conditional coverage? When should TC pool subgroups vs. calibrate separately? Are there methods for "borrowing strength" across subgroups?

**What we already know:** LOCUS notes that "marginal calibration does not guarantee that the induced accepted set satisfies a desired conditional exceedance rate." At ~352 total submissions across 3 contributors, per-contributor calibration may not be feasible. Wang & Qiao 2025 (AISTATS, PMLR 258:4888-4896) handles covariate shift but also needs sufficient per-group data.

### Q-CC3: Score Distribution Characterization for Gate Calibration

```
"score distribution" "calibration" "threshold" bimodal OR multimodal "quality scoring" ML 2025 2026
```
**Looking for:** TC prescribes a quantile gate (doc 09) but has never characterized the actual score distribution of its ~352 submissions. If the distribution is bimodal (cluster of good + cluster of bad), a simple quantile is appropriate. If it's unimodal or multi-modal, different calibration strategies apply. Methods for score distribution analysis before threshold setting.

**What we already know:** Issue #210 (0/99 accepted) implies the current fixed threshold is above the entire score distribution. But what the distribution actually looks like is unknown. No one has plotted it.

**Decision it unblocks:** Doc 09, epsilon selection.

### Q-CC4: Dual-Emission OTel env var Support Breadth

```
"OTEL_SEMCONV_STABILITY_OPT_IN" support framework instrumentation library 2026
```
**Looking for:** Which OTel instrumentation frameworks actually support the `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` env var? Doc 18's pinning strategy depends on dual-emission, but if major frameworks don't support it, the strategy fails. Check: OpenTelemetry Python, Java, .NET, Rust SDKs. Check: Datadog, Elastic, Honeycomb instrumentation libraries.

**What we already know:** The env var exists in the OTel spec. Doc 18 prescribes it as part of the pinning strategy. No assessment exists of actual framework support.

---

## Category 3: Ground-Truth-Free Quality & Annotation (6 queries)

From docs 10 and 19. Methods identified; empirical validation gaps remain.

### Q-GT1: Judge-Aware BTL on Small Scorer Ensembles

```
"Bradley-Terry" OR "BTL" "judge-aware" OR "annotator-aware" small ensemble 3 4 5 judges reliability 2025 2026
```
**Looking for:** Judge-Aware Ranking (arXiv:2601.21817) jointly estimates latent quality and judge reliability. But TC has only 4-5 scorers. Does JAR work with this few judges? What is the minimum number of judges for identifiable estimates? Does adding TokenRarityScorer (a fifth judge derived from the same forward pass as perplexity) violate the conditional independence assumption?

**What we already know:** JAR extends BTL with per-judge discrimination. TC's scorers: PerplexityScorer, TokenRarityScorer (not wired), embedding cosine, MinHash Jaccard, NCD (not wired). Perplexity and TokenRarity share a forward pass — potential conditional independence violation. Hui-Walter requires 2+ tests × 2+ populations.

**Decision it unblocks:** Doc 10, step 4 (fit JAR model).

### Q-GT2: Annotation Tool Selection for Trace Labeling

```
"annotation tool" OR "labeling tool" "trace" OR "structured data" OR "sequence" labeling open source 2025 2026
```
**Looking for:** Doc 19 describes a 4-week annotation protocol but never specifies what tool reviewers will use. Label Studio? Prodigy? Custom UI? Spreadsheet? What annotation tools support structured/sequence labeling (not just text classification)? TC's annotation task is "is this trace novel?" with reference to the full trace structure, not a text snippet.

**What we already know:** Doc 19's protocol produces 100-140 labeled traces over 4 weeks. The annotation task is binary (NOVEL / NOT_NOVEL) in weeks 1-3, potentially graded in week 4. No annotation tooling has been selected.

### Q-GT3: LLM-as-Annotator Anchoring Bias Quantification

```
"LLM" "pre-labeling" OR "pre-annotation" "anchoring bias" OR "priming effect" human annotation quality 2025 2026
```
**Looking for:** Doc 19 proposes LLM pre-labeling with human review. How much does seeing the LLM's label bias the human reviewer? Doc 19 mitigates with "blind labeling for the first 30 traces" but doesn't quantify the expected bias magnitude. Are there controlled experiments measuring anchoring bias in LLM-assisted annotation? What is the reliability penalty?

**What we already know:** Doc 19 estimates 2-3× speedup from LLM pre-labeling. The anchoring bias mitigation (blind-label first 30, compare agreement) is prescribed but untested. If bias is severe (agreement between blind and pre-labeled < 80%), all calibration-critical traces must be blind-labeled, which eliminates the speedup.

### Q-GT4: Snorkel/Weak Supervision in Rust

```
"weak supervision" OR "data programming" Rust OR "non-Python" implementation "label model" 2025 2026
```
**Looking for:** Doc 19 describes Snorkel weak supervision as an annotation multiplier, but Snorkel is a Python library and TC is a Rust codebase. Are there Rust implementations of label models? Can Snorkel's core algorithm (label model = generative model over labeling function outputs) be implemented in ~200-500 LOC of Rust? Or should TC call Python via FFI/subprocess?

**What we already know:** Doc 19 lists 6 labeling functions with estimated accuracies (unsourced). Snorkel's label model is conceptually simple (majority vote with learned accuracies), but the full Snorkel pipeline includes conflict resolution, dependency modeling, and noise-aware training. No Rust implementation exists as of the research sweep.

### Q-GT5: Inter-Annotator Agreement Baselines for Trace Novelty

```
"inter-annotator agreement" "novelty" OR "similarity" OR "duplicate" assessment "code" OR "software" Krippendorff 2025 2026
```
**Looking for:** What Krippendorff's Alpha values are typical for novelty/similarity assessment tasks? Doc 19 sets thresholds (< 0.50 = too subjective, 0.67 = usable, > 0.8 = ground truth) but doesn't cite baselines from comparable tasks. If trace novelty assessment is inherently low-agreement (like "code quality" or "code readability"), the thresholds may be unrealistic.

**What we already know:** Doc 02 section A.4 defines the annotation task: "Have you seen a trace substantially similar?" The hard part is defining "novel." Code similarity assessment studies would provide the best baseline, but none are cited.

### Q-GT6: Pairwise Preference vs. Absolute Scoring for Trace Quality

```
"pairwise comparison" vs "absolute rating" annotation efficiency "inter-annotator agreement" quality 2025 2026
```
**Looking for:** Doc 07 (Q-U3) suggested pairwise preferences ("which trace is more useful?") as an alternative to absolute scoring. Doc 10 proposes JAR (which supports both). Which approach yields higher inter-annotator agreement for TC's specific task? Pairwise is cognitively easier but requires O(n²) comparisons. Are there efficient tournament-style designs?

**What we already know:** DynaCF (arXiv:2606.09043) and ConsistRM (arXiv:2604.07484) are cited as anchors in doc 07 but both are UNVERIFIED. Judge-Aware Ranking (arXiv:2601.21817) supports both pairwise and absolute input. Nathan Lambert's RLHF Book (Ch 5) covers pairwise preference design.

---

## Category 4: Verified Skills & Behavioral Security (5 queries)

From doc 11. TC's differentiated wedge needs empirical grounding.

### Q-VS1: Delayed-Activation Malicious Behavior Detection

```
"delayed activation" OR "time bomb" OR "logic bomb" malicious behavior detection "multi-session" agent 2025 2026
```
**Looking for:** Methods for detecting skills that behave well in early invocations but activate malicious behavior after N uses, on specific dates, or when specific conditions are met. TC's cross-session corpus is uniquely positioned to detect this, but what statistical methods identify behavioral drift across invocations?

**What we already know:** SkillVetBench (arXiv:2606.00925) and MalSkillBench (arXiv:2606.07131) test single-session behavior. No existing benchmark tests multi-session delayed activation. TC fills this gap but needs detection methodology.

### Q-VS2: TEE-Attested Behavioral Reports

```
"TEE" OR "trusted execution" "behavioral report" OR "behavioral attestation" OR "execution attestation" verification consumer 2025 2026
```
**Looking for:** Systems that produce TEE-attested behavioral reports that external consumers can verify without trusting the platform. TC's verified skills tier (doc 11) depends on this, but the attestation chain from trace capture → behavioral analysis → badge issuance is not detailed.

**What we already know:** Proof-of-Execution (arXiv:2607.05397) issues EACs at ~2.7ms overhead. Agent-OSI (arXiv:2602.13795) L5 defines a provenance interface. But neither addresses the full chain from behavioral analysis to consumer-verifiable badge.

### Q-VS3: Verified Skill Registry Market Demand

```
"verified" OR "certified" "agent skill" OR "plugin" OR "extension" registry enterprise demand pricing 2026
```
**Looking for:** Is there enterprise demand for a "verified skills" registry? What would enterprises pay for verified-safe agent skills? NVIDIA's 162 signed skills suggest enterprise interest, but pricing and demand data are absent. TC needs market validation before building the tier.

**What we already know:** NVIDIA has 162 signed skills through an 8-stage pipeline. OWASP AST10 provides the threat taxonomy. ClawHavoc showed 341 malicious skills. But no pricing data exists for verified skill registries. Doc 11 lists "490K+ skills" and "32+ adopters" without citation — these market size claims need sourcing.

### Q-VS4: Behavioral Drift Detection Across Software Versions

```
"behavioral drift" OR "concept drift" software "version" OR "update" detection statistical test 2025 2026
```
**Looking for:** When a skill is updated (SKILL.md hash changes), TC needs to detect whether its behavior has drifted. What statistical tests work for detecting distributional shift in behavioral profiles (tool-call patterns, resource access, output distributions)? TC plans to expire badges every 90 days or on hash change (doc 11, section 4.4), but has no detection algorithm.

### Q-VS5: SkillFortify and SIGIL — Source Verification

```
"SkillFortify" formal verification agent skill security 2025 2026
```
```
"SIGIL" on-chain registry skill provenance agent 2025 2026
```
**Looking for:** Doc 11 cites SkillFortify (~96.95% F1 with zero false positives) and SIGIL (on-chain skill registry), but neither has an arXiv ID, DOI, or URL in the verification ledger. Are these published systems, whitepapers, industry announcements, or internal projects? TC cannot cite them externally without verifiable sources.

---

## Category 5: Trajectory RAG (5 queries)

From doc 12. Identified as TC's potential killer feature.

### Q-TR1: Trajectory Retrieval Evaluation Metrics

```
"trajectory retrieval" evaluation metric "downstream task" OR "task completion" agent 2025 2026
```
**Looking for:** How do you measure whether trajectory retrieval actually helps? TC can't directly measure downstream task improvement (the agent using the retrieved trace runs outside TC). What proxy metrics work? Click-through? Explicit feedback? A/B comparison of agent performance with vs. without retrieved traces?

**What we already know:** LRAT (arXiv:2604.04949) and ExpRAG (arXiv:2603.18272) use downstream task completion as evaluation. T3 (arXiv:2605.03344) reports +56.3% gains. But TC is a retrieval service, not the agent — TC can't observe the downstream outcome directly.

### Q-TR2: Diversity-Aware Retrieval Beyond MMR

```
"diversity aware" retrieval "determinantal point process" OR "submodular" OR "MMR" comparison 2025 2026
```
**Looking for:** Doc 12 prescribes MMR (Maximal Marginal Relevance) as the immediate fix for top-k collapse (xMemory, arXiv:2602.02007). Are there better alternatives? DPPs (Determinantal Point Processes) provide a principled probabilistic model for diversity. Submodular maximization gives approximation guarantees. How do they compare to MMR empirically for trajectory retrieval?

**What we already know:** xMemory documents the top-k collapse problem clearly. MMR with λ=0.7 is prescribed but untested. Doc 12 notes DPPs/submodular maximization as future work at 10K+ traces per category.

### Q-TR3: Privacy-Preserving Retrieval in TEEs

```
"private information retrieval" OR "PIR" "trusted execution" OR "TEE" embedding similarity search 2025 2026
```
**Looking for:** TC's trajectory RAG pipeline computes query-trace similarity inside a TEE. What is the state of the art for privacy-preserving embedding similarity search? Can the query embedding be kept private from TC's infrastructure? Can the trace embeddings be queried without revealing the full index?

**What we already know:** Doc 12's privacy architecture says "query embeddings computed in TEE" and "no trace content leaves TEE boundary." But the implementation details — particularly for HNSW search inside TEEs — are not addressed. HNSW has random memory access patterns that may leak information via side channels.

### Q-TR4: Context Budget Management for Retrieved Traces

```
"context budget" OR "context window" management retrieval "long context" summarization agent 2025 2026
```
**Looking for:** A retrieved trace may be 10K+ tokens. Five traces consume 50K tokens before the agent starts working. How should TC manage context budget? Summarize traces before returning? Return sub-traces? Let the consumer specify a token budget? What's the quality tradeoff between full traces and summarized traces?

**What we already know:** T3 suggests 1-3 high-quality traces produce gains; more is not necessarily better. Doc 02 C.10 covers trajectory compression (TRACE 10-50×, ACE 5-20×). Sub-trace decomposition (doc 02 B.6) could return relevant fragments instead of full traces.

### Q-TR5: Competitive Feature Matrix — Langfuse, Braintrust, LangSmith

```
Langfuse OR Braintrust OR LangSmith features "cross-user" OR "trajectory retrieval" OR "RAG" 2026
```
**Looking for:** Doc 12's competitive moat table claims Langfuse, Braintrust, and LangSmith lack cross-user retrieval, trajectory RAG, TEE-based quality scoring, and contributor compensation. These claims are based on the author's understanding, not verified feature matrices. Confirm or correct these claims against current (August 2026) feature sets. Note: Langfuse was acquired by Databricks — has the product changed?

---

## Category 6: Provenance & Anti-Sybil (4 queries)

From doc 13. Architecture designed; economic modeling gaps remain.

### Q-PS1: Sybil Attack Economics for Data Marketplaces

```
"Sybil attack" "data marketplace" OR "data sharing" economics "cost benefit" modeling 2025 2026
```
**Looking for:** At what contributor count and credit value do Sybil attacks become economically rational for TC? Doc 13 estimates "~100+ contributors" as the crossover but without formal modeling. What are the established models for Sybil attack economics in data marketplaces? How do staking requirements affect the breakeven point?

**What we already know:** arXiv:2605.07663 shows 1.74× inflation from splitting in Shapley-based systems. The Credibility Trilemma (arXiv:2605.26604) proves ghost-bid deviations are undetectable under sealed-bid VCG. TC's TEE is positioned as the closure mechanism. But the economic crossover point is an informal estimate.

### Q-PS2: Deterministic Scoring in TEEs

```
"deterministic" OR "reproducible" inference "trusted execution" OR "TEE" floating point BLAS 2025 2026
```
**Looking for:** Doc 13 Phase 2 requires deterministic scoring for meaningful attestation. Three sources of non-determinism are identified: floating-point operations, HNSW randomized layers, thread scheduling. What is the practical experience with achieving deterministic ML inference in TEEs? Are there deterministic BLAS implementations that run in Intel TDX? What is the performance overhead?

**What we already know:** Doc 13 prescribes "pin random seeds, use deterministic BLAS, serialize scoring" but provides no feasibility assessment or performance impact analysis.

### Q-PS3: Credit Multiplier Calibration for Tiered Provenance

```
"incentive" "multiplier" OR "bonus" "tiered" OR "level" calibration mechanism design 2025 2026
```
**Looking for:** Doc 13 proposes credit multipliers for provenance tiers (1.0×, 1.25×, 1.5×). These are round numbers chosen without simulation. How should multipliers be calibrated to maximize provenance adoption without overpaying? Are there mechanism design principles for setting tier incentives? Does the multiplier need to decay as adoption increases?

### Q-PS4: Web Proof Adoption Friction

```
"web proof" OR "TLS proof" OR "attestation proxy" adoption "user acceptance" privacy developer 2025 2026
```
**Looking for:** Doc 13 Phase 3 requires contributors to route API calls through TC's TEE Proxy for Web Proof verification. VET (arXiv:2512.15892) shows <3× overhead is achievable technically, but will developers accept routing their agent's API calls through TC's infrastructure? What is the expected adoption rate for privacy-invasive verification mechanisms, even with TEE guarantees?

---

## Category 7: Incentive Mechanisms (4 queries)

From doc 16. VCG/MUT identified; operational questions remain.

### Q-IM1: VCG Production Deployments for Data Pricing

```
"VCG" OR "Vickrey-Clarke-Groves" "production" OR "deployed" "data marketplace" OR "data pricing" 2025 2026
```
**Looking for:** Doc 16 notes "No production VCG deployment for data markets exists yet." Has this changed? Are there production systems using VCG or close variants for data valuation? What were the engineering challenges? How do they handle the O(n²) utility computation at scale?

**What we already know:** `vcg_allocate` is built in TC's codebase. VCG is O(n log n) for homogeneous multi-unit. Q-MIA (arXiv:2506.05379) provides a budget-balanced alternative. The Credibility Trilemma proves VCG requires broadcast commitment for collusion resistance.

### Q-IM2: Usage-Linked Credit Systems in Practice

```
"usage-based" OR "consumption-based" "credit" OR "reward" OR "compensation" "data contributor" marketplace 2025 2026
```
**Looking for:** Doc 16 Phase 3 proposes tying credits to downstream usage (50% access fees, 30% usage, 20% quality — mirroring Vana VRC-14). What are the practical challenges of tracking "downstream usage" of data? How do existing data marketplaces attribute downstream value? What is the time lag between contribution and usage measurement?

**What we already know:** Vana pivoted from emissions to usage-linked rewards. Ocean Protocol's AMM approach struggled with liquidity. The fundamental challenge: TC must track how each trace contributes to downstream value (RAG queries, skill extraction, model training), which requires cross-system instrumentation.

### Q-IM3: Collusion Resistance at Very Small Contributor Counts

```
"collusion" OR "cartel" resistance mechanism design "small" OR "few" participants auction 2025 2026
```
**Looking for:** Doc 16 notes VCG/MUT are DSIC under unilateral deviation but not collusion-proof. At 3 contributors, "collusion is the entire contributor base." What mechanism design principles apply when the participant count is single-digit? Is staking sufficient? Are there mechanisms that are collusion-resistant at small N?

### Q-IM4: Multi-Agent Trace Credit Apportionment

```
"multi-agent" "credit" OR "reward" "apportionment" OR "attribution" OR "splitting" traces 2025 2026
```
**Looking for:** When a trace involves multiple agents (orchestrator + sub-agents via A2A), how should credits be split? VCG and MUT assume single-contributor traces. Doc 16 flags this as Open Question 3 but offers no solution. Are there extensions of VCG/Shapley/MUT for multi-contributor items?

---

## Category 8: Structural Embeddings & Graph Methods (4 queries)

From doc 17. VS-Graph identified; evaluation and TEE deployment gaps remain.

### Q-SE1: HDC for Graph Classification Beyond VS-Graph

```
"hyperdimensional computing" OR "HDC" "graph classification" OR "graph embedding" beyond OR alternative VS-Graph 2025 2026
```
**Looking for:** VS-Graph (arXiv:2512.03394) is TC's proposed HDC-native graph embedding. Are there other HDC approaches to graph representation? How do they compare on graph classification benchmarks? Does VS-Graph's 450× speedup claim hold on graphs of TC's typical size (10-200 nodes per trace)?

**What we already know:** VS-Graph reports 450× speedup over GNNs on standard benchmarks. TC already uses HDC fingerprints per episode. The XOR composition of content and structure fingerprints is proposed but not theoretically justified.

### Q-SE2: Tool-Call Graph Extraction from Agent Traces

```
"tool call" OR "function call" graph extraction "agent trace" OR "execution trace" dependency 2025 2026
```
**Looking for:** Doc 17 describes a 6-step tool-call graph extraction pipeline, but step 4 (data-flow edge detection) is deferred as "ambiguous." Are there existing tools or methods for extracting dependency graphs from agent execution logs? GRADE (arXiv:2606.22741) distinguishes execution-layer from dependency-layer projections — does it include an extraction algorithm?

### Q-SE3: Structural Novelty Contribution Magnitude

```
"structural" OR "graph" "novelty" OR "anomaly" "additional" OR "complementary" "text" OR "content" embedding detection 2025 2026
```
**Looking for:** Doc 17 cites MCPShield's finding that structural features add 2-10pp AUC for novelty detection (arXiv:2605.11053). But this is a wide range. What determines where on the 2-10pp spectrum a given task falls? For trace novelty detection specifically, is structural signal more or less valuable than for attack detection? TC needs to decide whether to invest in structural embeddings before running the evaluation (doc 17, priority 3).

### Q-SE4: ONNX Runtime in TEE Enclaves

```
"ONNX Runtime" OR "ort" "trusted execution" OR "TEE" OR "SGX" OR "TDX" inference enclave 2025 2026
```
**Looking for:** Doc 17's GNN path requires ONNX Runtime via the `ort` Rust crate inside TC's TEE. Has anyone deployed ONNX Runtime in Intel TDX enclaves? What are the determinism guarantees? Performance overhead? Memory requirements? This determines whether the GNN inference path is feasible in TC's production environment.

---

## Category 9: GPAI & Regulatory (4 queries)

From doc 15. Positioning strategy defined; primary sources and market data gaps.

### Q-GP1: GPAI Code of Practice — Training Data Summary Template

```
"GPAI" "code of practice" "training data summary" template format fields requirements 2026
```
**Looking for:** Doc 15 maps TC's capabilities to GPAI obligations but does not cite the exact template schema for the required "training data summary." What are the specific fields, formats, and level of detail the EU expects? TC needs to build a template export function — the target format must be known.

**What we already know:** The GPAI Code of Practice was published July 10, 2025 (per Latham & Watkins). Three chapters: Transparency, Copyright, Safety & Security. The training data summary is required under Transparency. No TC team member has reviewed the actual template.

### Q-GP2: Digital Omnibus Regulation — Official Citation

```
"Digital Omnibus" regulation EU AI Act amendment "article 12" deferral official journal 2026
```
**Looking for:** Doc 15 references the Digital Omnibus Regulation as deferring Article 12 standalone high-risk AI system deadlines to December 2, 2027. But no official citation (regulation number, Official Journal reference) is provided. TC's grant applications need precise legal citations, not secondhand references.

### Q-GP3: Open-Source GPAI Compliance Toolkit Gap Verification

```
"open source" "GPAI compliance" OR "AI Act compliance" toolkit OR framework GitHub 2026
```
**Looking for:** Doc 15 asserts "no open-source GPAI compliance toolkit exists" based on a survey of "GitHub, FOSS directories, EU AI Act compliance guides, NLnet/NGI project lists." Re-verify this negative claim — the open-source landscape changes rapidly. If a competitor has emerged since the last survey, TC's positioning needs adjustment.

### Q-GP4: AI Compliance Market Pricing Verification

```
"AI compliance" platform pricing "Holistic AI" OR "Credo AI" OR "TrustArc" OR "OneTrust" 2026
```
**Looking for:** Doc 15 cites vendor pricing (Holistic AI €30K-100K, Credo AI €30K-50K, TrustArc €50K-500K, OneTrust €50K-500K) without sources. These are market claims that need verification — are they from public pricing pages, analyst reports, or estimates? TC's positioning as an open-source alternative depends on accurate competitive pricing data.

---

## Category 10: Cross-System Dependencies & Circular Breaks (5 queries)

Identified across multiple v6 documents. These are not topic-specific but address structural gaps in TC's plan.

### Q-CD1: Bootstrap Sequencing for Quality Systems Without Ground Truth

```
"bootstrap" OR "cold start" "quality system" OR "scoring system" "without ground truth" OR "no labels" sequence 2025 2026
```
**Looking for:** TC has a circular dependency: doc 19 (active learning) needs scorers → doc 10 (quality estimation) needs labels → doc 19 needs scorer disagreement → doc 02 needs TokenRarityScorer wired → doc 10 validates the ensemble. How do other quality systems break this cycle? What is the minimum viable sequence?

**What we already know:** The dependency chain is: wire TokenRarityScorer → run all scorers → uncertainty-sample → annotate → fit JAR → calibrate gate → validate. But each step depends on the previous. The practical question is: what is the weakest link that can be skipped or approximated to break the cycle?

### Q-CD2: Corpus Size Thresholds for Statistical Methods

```
"minimum sample size" OR "power analysis" "conformal prediction" OR "BTL" OR "Hui-Walter" OR "inter-annotator" traces 2025 2026
```
**Looking for:** Multiple v6 documents cite different minimum trace/sample sizes: 30-50 for Hui-Walter (doc 10), 100-140 from active learning (doc 19), 200+ for annotation (doc 02), 300+ for conformal calibration (doc 09), 350 for current corpus. Are these consistent? What power analysis would determine the actual minimum corpus size for TC to achieve reliable scoring? Can TC operate all these methods simultaneously at ~352 traces?

### Q-CD3: IronClaw Redaction Entity Typing Capabilities

```
IronClaw "NEAR AI" redaction "entity type" OR "named entity" detection PII 2026
```
**Looking for:** Doc 08 notes IronClaw produces some untyped `[REDACTED]` markers where entity type detection failed. What are IronClaw's entity typing capabilities? Can they be improved upstream? Or must TC classify entities in its own pipeline? This determines whether doc 08 Approach 2 (typed vocabulary tokens) can work with IronClaw traces.

**Decision it unblocks:** Doc 08, Approach 2 feasibility.

### Q-CD4: Per-Contributor Submission Distribution

```
(No external query — this is an internal data question)
```
**Looking for:** Multiple documents need the per-contributor breakdown of TC's ~352 submissions. Is it 200/100/52? 300/40/12? The distribution determines: (1) whether Hui-Walter is feasible (doc 10, needs 30-50 per population), (2) whether per-contributor conformal calibration works (doc 09), (3) whether LOCUS subgroup calibration is possible (doc 09). This is answerable from TC's own database.

**Action:** Query TC's database, not external research.

### Q-CD5: Competitive Landscape Verification (August 2026 Snapshot)

```
Langfuse Databricks acquisition features 2026
```
```
Braintrust "agent traces" OR "trajectory" features 2026
```
```
LangSmith "cross-user" OR "shared" OR "RAG" features 2026
```
**Looking for:** Docs 12, 15, and 11 make competitive claims about Langfuse, Braintrust, and LangSmith that are stated as facts but not independently verified. Langfuse was acquired by Databricks — has the product changed? Has any competitor added cross-user retrieval, trajectory RAG, or contributor compensation? TC's differentiation claims need an August 2026 verification pass.

---

## Category 11: Retained from Doc 07 (Updated Context)

These queries from doc 07 were NOT answered by the 12-agent sweep and remain open. Updated with new context from docs 08-19.

### Q-S3 (retained): Process Mining for Agent Traces
Updated context: GRADE (arXiv:2606.22741) provides a better graph representation than flat conformance checking. But TC still needs empirical data on false-positive rates for conformance-based novelty detection on agent traces specifically.

### Q-S4 (retained): Causal Attribution Without Re-Execution
Updated context: GraphTracer was WITHDRAWN (arXiv:2510.10581 v2). Zero-Replay Debugging (arXiv:2606.14805) remains the most practical offline approach. But no TC evaluation has been performed.

### Q-S5 (retained): Joint Compression and Quality Scoring
No new findings from docs 08-19. Governance Decay constraint (arXiv:2606.22528) still the key safety result.

### Q-S7 (retained): Contrastive Learning for Trace Embeddings
Updated context: Doc 17 adds structural embeddings as a complementary approach. **Citation correction:** arXiv:2509.24291 is actually GIRCSE (generative contrastive sentence embeddings), NOT the hard-negative mining paper previously cited. The hard-negative mining source remains UNVERIFIED and needs a correct citation.

### Q-S8 (retained): Concept Drift Detection
Updated context: WATCH (arXiv:2505.04608) is now detailed in doc 09 as the drift trigger for conformal recalibration. But WATCH has not been evaluated on TC's data.

### Q-I2 (retained): Claude Code Hook Integration Patterns
No new findings. SessionEnd timeout constraint (1.5s) confirmed. Background daemon (PR #244, merged) is the prescribed architecture.

### Q-I4 (retained): Cross-Agent Session Formats
No new findings. Per-tool matrix (local files vs API for Claude Code, Codex, Cursor, Copilot) remains a gap.

### Q-I5 (retained): A2A Observability & Multi-Agent Tracing
Updated context: GRADE (arXiv:2606.22741) provides dependency-graph correlation beyond W3C traceparent. Multi-agent credit apportionment (Q-IM4) is now a separate query.

### Q-G1-G5, Q-G7 (retained): Growth & Distribution
No new findings from docs 08-19. These were the "thinnest-sourced area" identified by research3.md and remain so. Priority for next research round.

### Q-M1-M5 (retained): Strategy & Market
Updated context: Doc 15 (GPAI compliance) adds significant detail but creates new verification needs (Q-GP1 through Q-GP4). Doc 16 (incentive design) addresses Q-M5 but creates new queries (Q-IM1 through Q-IM4).

### Q-T1 (retained): Streaming Agent Trace Analysis
No new findings. Real-time quality feedback remains unexplored.

### Q-T2 (retained): Multi-Modal Agent Trace Analysis
No new findings. Computer-use agents with screenshots remain unexplored.

### Q-T4 (retained): Sleep-Time Trace Processing
No new findings. Background daemon (PR #244) provides the infrastructure but idle-time processing patterns are not researched.

### Q-T5 (retained): Skill Extraction from Agent Traces
Updated context: Doc 11 provides the "verified skills tier" architecture that consumes extracted skills. RHO, Trace2Skill, SkillAudit IDs remain UNVERIFIED from research3.md.

### Q-U1 (retained): Federated Learning for Scorer Updates
No new findings. FedAvg feasibility for heterogeneous scorer ensembles unknown.

### Q-U3 (retained): Reward Modeling from Pairwise Preferences
Updated context: Now linked to Q-GT6 (pairwise vs. absolute scoring). DynaCF and ConsistRM remain UNVERIFIED.

### Q-U4 (retained): Online Anomaly Detection
No new findings. CALIBURN (arXiv:2605.24696) remains UNVERIFIED.

### Q-X1-X6 (retained): Cross-Domain Inspiration
No new findings from docs 08-19. Video summarization (Q-X3) for trace compression remains the most promising unexplored transfer.

---

## Unverified Citations Requiring Confirmation

Across all 20 v6 documents, these citations could not be independently verified within the research budget. They are not fabricated — they simply were not reachable. Confirm each before citing externally.

| arXiv/Source | Claimed Topic | Where Cited |
|---|---|---|
| 2606.18467 | ToolChain-CRC | Docs 02, 07 |
| 2607.24343 | Role-Stratified-CRC | Docs 02, 07 |
| 2605.18812 | PASC | Docs 02, 07 |
| 2605.07663 | Sybil unfair payoff | Docs 02, 16 |
| 2506.12619 | Semivalue gameability | Docs 02, 16 |
| 2606.20669 | Agent Behavior Mining | Docs 02, 07 |
| 2607.02599 | AgentLTL | Docs 02, 07 |
| 2606.08275 | Causal Agent Replay | Docs 02, 07 |
| 2605.25338 | CausalFlow | Docs 02, 07 |
| 2509.03312 | AgenTracer-8B | Docs 02, 07 |
| 2606.14805 | Zero-Replay Debugging | Docs 02, 07 |
| 2606.00611 | TRACE compression | Docs 02, 07 |
| 2606.31564 | ACE compression | Docs 02, 07 |
| 2605.08580 | Slipstream | Docs 02, 07 |
| 2607.05378 | CompactionRL | Docs 02, 07 |
| 2606.22528 | Governance Decay | Docs 02, 07 |
| 2606.05922 | RHO skill extraction | Docs 02, 07, 11 |
| 2603.25158 | Trace2Skill | Docs 02, 07, 11 |
| 2606.14239 | SkillAudit | Docs 02, 07, 11 |
| 2504.17703 | ~~Federated learning survey~~ **WITHDRAWN** (disputed authorship). Do not cite. | Doc 07 |
| 2606.09043 | DynaCF | Doc 07 |
| 2604.07484 | ConsistRM | Doc 07 |
| 2605.24696 | CALIBURN | Doc 07 |
| 2506.15655 | cAST | Doc 07 |
| 2602.14102 | DALL | Doc 07 |
| 2512.19682 | GenEnv | Doc 07 |
| 2509.24291 | ~~Hard-negative mining~~ Actually **GIRCSE** (generative contrastive sentence embeddings). Wrong paper. | Doc 07 |
| 2604.02324 | Grounded vocab init | Doc 08 |
| 2604.16656 | ~~Grounded vocab init (2)~~ Actually **"Defragmenting Language Models"** (vocab expansion/interpretability). Distinct from GTI. | Doc 08 |
| -- | SkillFortify (~96.95% F1) | Doc 11 |
| -- | SIGIL (on-chain registry) | Doc 11 |
| -- | Trail of Bits scanner bypass | Doc 11 |
| -- | Sampled VCG (Balkanski 2017) | Doc 16 |

---

## Quick-Start: Ten Searches to Run First

Highest-payoff queries targeting the biggest decision-blockers across all 20 v6 documents:

1. ~~**TEE attestation scope for pipeline reordering** (Q-R1)~~ — **ANSWERED (research4, doc 08 §2.4.1): GO.** TDX RTMRs measure boot chain, not intra-application control flow. Reordering within same measured binary is threat-model-neutral.
2. ~~**Qwen FIM capabilities** (Q-R2)~~ — **ANSWERED (research4, doc 08 §2.5): Qwen3-Coder has native FIM.** Zero extra VRAM. `<|fim_prefix|>/<|fim_suffix|>/<|fim_middle|>` tokens confirmed.
3. **JAR with 4-5 judges** (Q-GT1) — **PARTIALLY ANSWERED (research4, doc 10): 4 independent judges** (PerplexityScorer + TokenRarityScorer collapsed to one ForwardPassJudge). Identifiable with 4-5 judges per arXiv:2601.21817. Shared forward pass violation confirmed.
4. **Score distribution of existing 352 traces** (Q-CC3) — OPEN. Internal data question, not researchable externally.
5. **Sybil economics modeling** (Q-PS1) — **PARTIALLY ANSWERED (research4, doc 16 §13): N=3 collusion impossible to resist.** Defer payment-mechanism collusion resistance until N > ~10.
6. ~~**GPAI training data summary template** (Q-GP1)~~ — **ANSWERED (research4, doc 15 §4): 3-section structure confirmed.** Published 24 Jul 2025 under Art. 53(1)(d).
7. **Competitive feature matrix verification** (Q-CD5) — OPEN. Competitive features change with each release cycle.
8. **VCG production deployments** (Q-IM1) — OPEN. No production VCG deployment for data pricing found.
9. **Inter-annotator agreement baselines** (Q-GT5) — **PARTIALLY ANSWERED (research4, doc 19 §7.3/7.5): Realistic α is 0.4-0.6.** Pairwise comparison raises agreement. Tournament design avoids O(n²).
10. ~~**Digital Omnibus official citation** (Q-GP2)~~ — **ANSWERED (research4, doc 15 §1): Regulation (EU) 2026/1744**, OJ 24 Jul 2026, in force 27 Jul 2026.

---

## How to Use These Queries

| Tool | Best For | Queries |
|---|---|---|
| **Perplexity Pro** | Market intelligence, product features, vendor pricing | Q-VS3, Q-GP4, Q-CD5, Q-IM2 |
| **Google Scholar** | Academic papers, methods, formal results | Q-R*, Q-CC*, Q-GT*, Q-SE*, Q-PS2 |
| **Semantic Scholar** | Citation graphs, "highly influential" filtering | Q-GT1 (seed: 2601.21817), Q-SE1 (seed: 2512.03394) |
| **Connected Papers** | Two-hop discovery from anchors | Q-TR2 (seed: 2602.02007), Q-CC1 (seed: 2606.21255) |
| **EUR-Lex / Official Journal** | EU regulation citations | Q-GP2 |
| **HuggingFace** | Dataset inspection | Q-CD4 (Open-SWE-Traces Rust fraction) |
| **TC database** | Internal data questions | Q-CD4 (per-contributor distribution) |
| **GitHub/X** | Tools, community, adoption data | Q-GT2, Q-GT4, Q-GP3 |

Each query is designed to be self-contained — copy the search string, use the "Looking for" text as additional instruction for deep research tools.
