# Deep Research Queries v5

**Date**: August 2026

## What This Document Is

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces (~235K LOC, 6 crates). Traces are scored for quality and novelty inside TEEs (Trusted Execution Environments on NEAR AI Cloud) and contributors earn NEAR blockchain credits. ~352 submissions, ~13/week, 3 contributors, 6 GitHub stars. Built by Zaki Manian (co-creator of Cosmos SDK/IBC).

This is the fifth-generation query set, synthesized from gaps, open questions, unverified claims, and decision points identified across all 22 v6 documents (00-22). Prior queries (doc 07, 37 queries; doc 20, 52 queries) are superseded where the research has been completed; retained where still open. This document adds new queries and consolidates the full unverified citation inventory across the entire document corpus.

### How This Document Was Produced

Four extraction agents scanned all 22 v6 documents and identified ~200+ open items: unverified citations, decision-blocking gaps, internal data questions, and cross-document dependencies. This document synthesizes those findings into a prioritized, deduplicated set of research queries organized by theme. Items that appear in multiple documents (e.g., ClickHouse/Langfuse acquisition verification in docs 05, 12, 15, 22) are consolidated into a single query with all source documents noted.

### Relationship to Doc 20

Doc 20 contains 52 queries across 10 categories plus a "Retained from Doc 07" section. This document:

**Fully answered (remove from active research):**
- Q-R1 (TEE attestation scope): ANSWERED by research4 -- TDX RTMRs measure boot chain, not intra-application control flow. Pipeline reordering within same measured binary is threat-model-neutral.
- Q-R2 (Qwen FIM capabilities): ANSWERED by research4 -- Qwen3-Coder has native FIM via `<|fim_prefix|>/<|fim_suffix|>/<|fim_middle|>` tokens. Zero extra VRAM.
- Q-GP1 (GPAI training data summary template): ANSWERED by research4 -- 3-section structure confirmed (General Info, Data Sources, Compliance Metadata). Published 24 Jul 2025 under Art. 53(1)(d).
- Q-GP2 (Digital Omnibus official citation): ANSWERED by research4 -- Regulation (EU) 2026/1744, OJ 24 Jul 2026, in force 27 Jul 2026. Parliament 423-57, Council 29 Jun 2026.
- Q-CC4 (Dual-emission OTel env var support): PARTIALLY ANSWERED by doc 22 -- John Hodge July 2026 analysis confirms inconsistent per-SDK support. Per-SDK conformance tests are the prescribed validation method.

**Partially answered (retained with updated context):**
- Q-GT1 (JAR with 4-5 judges): 4 independent judges confirmed (PerplexityScorer + TokenRarityScorer collapsed to ForwardPassJudge). Shared forward pass violation confirmed. Identifiability at 4-5 judges confirmed per arXiv:2601.21817. OPEN: empirical validation on TC's scorer ensemble still needed.
- Q-PS1 (Sybil economics): N=3 collusion impossible to resist via payment mechanism. Quality gates + provenance attestation carry the load. OPEN: formal economic model for crossover point at larger N.
- Q-GT5 (Inter-annotator agreement baselines): Realistic alpha is 0.4-0.6. Pairwise comparison raises agreement. OPEN: baselines from code-similarity assessment specifically.
- Q-CC3 (Score distribution of 352 traces): INTERNAL data question. Not researchable externally. Still unanswered.

**Still fully open (retained in Category 11):**
- Q-CD5 (Competitive feature matrix verification)
- Q-IM1 (VCG production deployments)
- Q-GT2 (Annotation tool selection)
- Q-GT4 (Snorkel in Rust)
- Q-VS5 (SkillFortify and SIGIL sources)
- All Category 11 retained queries from doc 07 (Q-S3, Q-S4, Q-S5, Q-S7, Q-S8, Q-I2, Q-I4, Q-I5, Q-G1-G5, Q-G7, Q-M1-M5, Q-T1, Q-T2, Q-T4, Q-T5, Q-U1, Q-U3, Q-U4, Q-X1-X6)

---

## Quick-Start: Ten Searches to Run First

Highest-payoff queries targeting the biggest decision-blockers across all 22 v6 documents. Ordered by impact on TC's next engineering sprint.

1. **Qwen3-Coder vs Qwen 3.6 35B-A3B-FP8 model identity** (Q-DB1) -- TC's production scorer model naming is ambiguous across documents. Is "Qwen 3.6 35B-A3B-FP8" the same model as "Qwen3-Coder"? If not, which one is deployed? This affects FIM capability, VRAM planning, and every scoring document.
2. **AUC > 0.93 confound invalidation scope** (Q-SP1) -- The headline quality metric in grant-facing documents is known to be confounded (paragraph count achieves AUC 1.000). What replacement metric can TC cite? Blocks grant applications.
3. **COSINE noise-aware loss for trace quality** (Q-SP2) -- Doc 21's bootstrap sequence depends on COSINE (arXiv:2111.14282) for training on weak labels. Has COSINE been applied to non-image domains? Blocks Phase 2 of bootstrap.
4. **Competitive feature matrix verification (August 2026)** (Q-MI1) -- Langfuse/Braintrust/LangSmith feature claims are unverified. ClickHouse acquired Langfuse -- has the product changed? Blocks differentiation narrative.
5. **EUR 7.6-38B compliance market source** (Q-RG1) -- The market size range cited in grant applications has no analyst source. Find the primary report or flag as unreliable.
6. **VCG production deployments for data pricing** (Q-MD1) -- TC's mechanism design pivot from Shapley to VCG needs real-world precedent. Any production system using VCG for data valuation?
7. **VS-Graph accuracy on trajectory graphs** (Q-TD1) -- VS-Graph reports 450x speedup on molecular graph benchmarks (MUTAG/DD). Does accuracy hold on TC's trajectory graphs (10-200 nodes, heterogeneous node types)?
8. **Open-SWE-Traces Rust fraction** (Q-SP5) -- TC is Rust-native. How many of the 207K Open-SWE-Traces trajectories are Rust? Determines corpus seeding calibration value.
9. **Optimal k for TypiClust on TC's 352 traces** (Q-AH1) -- Doc 19's cold-start annotation begins with TypiClust cluster-centroid selection. What k value maximizes distributional coverage at n=352?
10. **WASM plugin RTMR attestation performance overhead** (Q-TD3) -- Doc 22's WASM scorer plugin architecture extends TDX RTMRs per plugin load. What is the per-plugin attestation overhead?

---

## How to Use These Queries

| Tool | Best For | Queries |
|---|---|---|
| **Perplexity Pro** | Market intelligence, product features, vendor pricing, recent news | Q-MI1 through Q-MI5, Q-RG1, Q-RG3, Q-AE2, Q-AE3 |
| **Google Scholar** | Academic papers, methods, formal results, citation verification | Q-SP*, Q-MD*, Q-TD*, Q-AH*, citation verification batch |
| **Semantic Scholar** | Citation graphs, "highly influential" filtering | Q-SP2 (seed: 2111.14282), Q-TD1 (seed: 2512.03394), Q-MD2 (seed: 2508.21261) |
| **Connected Papers** | Two-hop discovery from anchor papers | Q-SP3 (seed: 2606.22741), Q-AH2 (seed: 2202.02794) |
| **arXiv** | Direct paper verification, existence confirmation | Full citation verification batch (Section 1) |
| **EUR-Lex / Official Journal** | EU regulation citations, GPAI Code of Practice updates | Q-RG2, Q-RG4 |
| **HuggingFace** | Dataset inspection, model cards | Q-SP5 (Open-SWE-Traces), Q-DB1 (Qwen model identity) |
| **TC database** | Internal data questions (flagged INTERNAL) | Q-IN1 through Q-IN5 |
| **GitHub / npm / crates.io** | Tools, libraries, community adoption data | Q-AH3 (Snorkel in Rust), Q-AE4 (OTel SDKs), Q-AE5 (AAIF governance) |
| **X / Hacker News** | Community sentiment, adoption signals, product launches | Q-MI3 (VerifyWise), Q-AE1 (AgentSkills market data) |

Each query is designed to be self-contained -- copy the search string, use the "Looking for" text as additional instruction for deep research tools.

---

## Category 1: Citation Verification Batch

Across all 22 v6 documents, these citations could not be independently verified within the research budget. They are not assumed fabricated -- they simply were not reachable via standard academic databases during prior sweeps. This section consolidates them into an efficient verification plan rather than generating one query per paper.

### Unverified Citations Master Table

**Papers with arXiv IDs (verify existence and claimed content):**

| arXiv ID | Claimed Topic | Where Cited | Priority |
|---|---|---|---|
| 2607.26300 | AgentGUI (38% improvement, p=0.023) | Docs 01, 03 | High -- cited as evidence for UI patterns |
| 2606.30560 | TraceLab (Zhu et al., Univ. of Washington) -- workload characterization, NOT "Cognition Labs", NO "31% failure rate" | Docs 01, 03, 06, 14 | Verified -- both prior claims corrected in v6.2 |
| 2607.18754 | AgentDebugX | Doc 01 | Medium |
| 2606.18467 | ToolChain-CRC | Docs 02, 07 | High -- anchors conformal prediction argument |
| 2607.24343 | Role-Stratified-CRC | Docs 02, 07 | High -- anchors conformal prediction argument |
| 2605.18812 | PASC | Docs 02, 07 | Medium |
| 2605.07663 | Sybil unfair payoff (1.74x) | Docs 02, 16 | High -- anchors mechanism design pivot |
| 2506.12619 | Semivalue gameability | Docs 02, 16 | High -- proves Shapley class is gameable |
| 2606.20669 | Agent Behavior Mining (BPM 2026) | Docs 02, 07 | Medium |
| 2607.02599 | AgentLTL | Docs 02, 07 | Medium |
| 2606.08275 | Causal Agent Replay (CAR) | Docs 02, 07 | Medium |
| 2605.25338 | CausalFlow | Docs 02, 07 | Medium |
| 2509.03312 | AgenTracer-8B | Docs 02, 07 | Low |
| 2606.14805 | Zero-Replay Debugging | Docs 02, 07 | Medium |
| 2606.00611 | TRACE compression | Docs 02, 07 | Low |
| 2606.31564 | ACE compression (dynamic KV pooling) | Docs 02, 07 | Low |
| 2605.08580 | Slipstream (spec-exec compression) | Docs 02, 07 | Low |
| 2607.05378 | CompactionRL | Docs 02, 07 | Low |
| 2606.22528 | Governance Decay | Docs 02, 07 | Medium -- safety constraint for compression |
| 2606.05922 | RHO skill extraction | Docs 02, 07, 11 | High -- supports verified skills tier |
| 2603.25158 | Trace2Skill | Docs 02, 07, 11 | High -- supports verified skills tier |
| 2606.14239 | SkillAudit | Docs 02, 07, 11 | High -- supports verified skills tier |
| 2606.09043 | DynaCF (pairwise comparison framework) | Doc 07 | Medium |
| 2604.07484 | ConsistRM (reward model consistency) | Doc 07 | Medium |
| 2605.24696 | CALIBURN (online anomaly detection) | Doc 07 | Medium |
| 2506.15655 | cAST | Doc 07 | Low |
| 2602.14102 | DALL | Doc 07 | Low |
| 2512.19682 | GenEnv | Doc 07 | Low |
| 2604.02324 | Grounded vocab init (GTI) | Doc 08 | Medium |
| 2504.09389 | Harmonic mean formula source | Doc 06 | Medium -- cited in data valuation argument |
| 2607.05397 | Proof-of-Execution (EACs at 2.7ms) | Doc 11 | Medium |
| 2605.11053 | MCPShield (structural features +2-10pp AUC) | Doc 17 | Medium |
| 2606.03019 | Deterministic TEE inference (34-61% overhead) | Doc 13 | High -- cost anchor for TEE scoring |
| 2606.22741 | GRADE (execution-layer vs dependency-layer graph) | Docs 02, 07, 17 | High -- key structural embedding reference |

**Papers with KNOWN errors (do NOT use without correction):**

| arXiv ID | Error | Correction |
|---|---|---|
| 2509.24291 | Cited as "hard-negative mining" | Actually **GIRCSE** (generative contrastive sentence embeddings). Wrong paper. |
| 2504.17703 | Cited as "federated learning survey" | **WITHDRAWN** due to disputed authorship. Do not cite. |
| 2604.16656 | Cited as "grounded vocab init (2)" | Actually **"Defragmenting Language Models"** (vocab expansion/interpretability). Distinct from GTI. |

**Papers with NO arXiv ID (existence unconfirmed):**

| Name | Claimed Result | Where Cited | Priority |
|---|---|---|---|
| TraceProbe | 82% accuracy for detecting synthetic agent traces | Doc 03 | High -- cited as v6-unverified |
| SkillFortify | ~96.95% F1, zero false positives (formal verification) | Doc 11 | High -- key verified-skills evidence |
| SIGIL | On-chain skill registry with provenance | Doc 11 | Medium |
| AgentLocate | Agent failure localization | Doc 06 | Low |
| TraceSIR | Trace similarity ranking | Doc 06 | Low |
| VCC | Valuation via code coverage | Doc 06 | Low |
| FailureNet | Failure pattern network | Doc 06 | Low |
| RootTrace | Root cause trace analysis | Doc 06 | Low |
| AgentPostMortem | Post-mortem agent analysis | Doc 06 | Low |
| DebugAgent | Agent debugging framework | Doc 06 | Low |
| STRACE | Structured trace analysis for process mining | Docs 02, 06 | Medium |
| PrefixGuard | Prefix-based trace conformance | Docs 02, 06 | Medium |
| PASTE | Trace embedding method | Doc 06 | Low |
| TraceGraph | Graph representation of traces | Doc 06 | Low |
| AgentAtlas | Agent behavior atlas | Doc 06 | Low |
| SessionSense | Session-level behavior analysis | Doc 06 | Low |
| SkillGraph | Skill dependency graph | Doc 06 | Low |
| SafeSkill | Skill safety assessment | Doc 06 | Low |
| SkillTransfer | Skill transferability metric | Doc 06 | Low |
| ClawHavoc Analysis | Analysis of the ClawHavoc malicious skill attack | Doc 06 | Medium |
| AgentPetri | Petri net model of agent behavior | Doc 06 | Low |
| TraceConform | Trace conformance checking | Doc 06 | Low |
| WorkflowMiner | Workflow extraction from traces | Doc 06 | Low |
| ARC | Adaptive retrieval compression | Doc 06 | Low |
| Focus | Focused trace compression | Doc 06 | Low |
| TraceZip | Trace compression algorithm | Doc 06 | Low |
| ContextPrune | Context pruning for long traces | Doc 06 | Low |
| StreamCompress | Streaming trace compression | Doc 06 | Low |
| CompressAndScore | Joint compression and scoring | Doc 06 | Low |
| Sampled VCG (Balkanski 2017) | Sampling-based VCG approximation | Doc 16 | Medium |
| Trail of Bits scanner bypass | Scanner bypass in skill verification | Docs 01, 03, 11 | Medium |

**Non-paper claims without sources:**

| Claim | Where Cited | Priority |
|---|---|---|
| "490K+ skills, 32+ adopters" in Agent Skills ecosystem | Docs 01, 03, 11 | High -- market size claim in grant materials |
| ClawHavoc "341 malicious skills" planted | Docs 01, 03, 11 | High -- security narrative anchor |
| "30 lifecycle hook events" for Claude Code | Doc 01 | Medium |
| r/ClaudeAI "1M+ members" | Doc 01 | Low -- community size claim |
| "291 issues/day" (unspecified context) | Doc 01 | Low |
| Mozilla cq "1,200 stars" | Doc 01 | Low |
| TokenShift "$60M Series B" | Docs 01, 03 | Medium -- competitive landscape |
| Exceeds AI / Ink / UseAI as cost-tracking competitors | Docs 01, 03 | Low |
| IronClaw PR #4559 "standing consent" | Doc 01 | Medium -- verify which repo |
| A2A "150+ member organizations" (specific count) | Doc 03 | Medium -- verify primary source |
| AAIF Observability Working Group | Doc 03 | Low -- verify URL/charter |
| NVIDIA "162 signed skills" through 8-stage pipeline | Doc 11 | Medium |
| Braintrust "$800M" valuation or funding | Doc 05 | Medium |
| Galileo acquired by Cisco | Doc 05 | Medium |
| Helicone acquired by Mintlify | Doc 05 | Medium |
| ClickHouse/Langfuse "$15B" post-acquisition valuation | Docs 05, 12, 15, 22 | High -- used across 4 docs |

### Efficient Verification Strategy

Rather than running 40+ individual searches, batch verification by source type:

**Batch A: arXiv existence check (30 minutes)**
For each arXiv ID in the table above, visit `https://arxiv.org/abs/XXXX.XXXXX` directly. Record: (1) paper exists, (2) title matches claimed topic, (3) key claim is present. This is mechanical and fast.

**Batch B: Named-but-no-ID papers (1 hour)**
Run a single search per paper name on Google Scholar and Semantic Scholar. Most of these are likely either: (a) published under a slightly different name, (b) workshop papers without arXiv preprints, (c) industry reports, or (d) fabricated by the research agent. Flag each as FOUND, RENAMED, or NOT FOUND.

**Batch C: Market/product claims (1 hour)**
Search Crunchbase, TechCrunch, and company press pages for: TokenShift funding, Galileo/Cisco acquisition, Helicone/Mintlify acquisition, Braintrust funding, ClickHouse valuation, ClawHavoc incident details, NVIDIA signed skills count, Agent Skills ecosystem size.

---

## Category 2: Internal Data Questions (INTERNAL)

These items are answerable from TC's own database, scoring logs, or codebase. No external search queries are needed. They are listed here because multiple v6 documents depend on the answers and no one has produced them yet.

### Q-IN1: Per-Contributor Submission Distribution

**Looking for:** The breakdown of TC's ~352 submissions by contributor. Is it 300/40/12? 200/100/52? 117/117/118? The distribution determines: (1) whether Hui-Walter is feasible (needs 30-50 per population, doc 10), (2) whether per-contributor conformal calibration works (doc 09), (3) whether LOCUS subgroup calibration is possible (doc 09), (4) whether the "3rd contributor" (brapse, PR #250, Aug 10 2026) has enough submissions to form a meaningful subgroup.

**Action:** Query TC's database. `SELECT contributor_id, COUNT(*) FROM submissions GROUP BY contributor_id ORDER BY COUNT(*) DESC`.

**Referenced in:** Docs 02, 09, 10, 13, 16, 21.

### Q-IN2: Score Distribution of 352 Traces

**Looking for:** Plot the actual score distribution of TC's ~352 submissions across all scorer dimensions (perplexity, embedding cosine, any other active scorers). Is it bimodal (good cluster + bad cluster)? Unimodal? Multi-modal? This determines whether a simple quantile gate is appropriate (doc 09) and reveals whether Issue #210 ("0 of 99 accepted") was caused by a threshold above the entire distribution or by genuinely low scores.

**Action:** Export all scorer outputs for the 352 submissions. Plot histograms per scorer dimension. Compute summary statistics (mean, median, std, skewness, kurtosis).

**Referenced in:** Docs 02, 09, 10, 21.

### Q-IN3: Redaction Density per Contributor

**Looking for:** How many redaction markers per trace, broken down by contributor and by agent family (IronClaw vs Claude Code vs other)? Doc 08 claims IronClaw contributors are "systematically disadvantaged" by thorough redaction. Quantify the disadvantage: what is the mean/median/max placeholder count per trace per contributor? What is the correlation between placeholder count and quality score?

**Action:** Parse the 352 submitted traces, count `[REDACTED]` and typed placeholder markers, join with contributor and scorer data.

**Referenced in:** Docs 08, 10.

### Q-IN4: Scorer Pair Count Verification

**Looking for:** Doc 10 states "10 scorer pairs" but only describes 4 judges (PerplexityScorer, TokenRarityScorer, embedding cosine, MinHash). With 4 judges, the combinatorial count is C(4,2) = 6 pairs, not 10. Either there are additional scorers not named in the text, or the "10 scorer pairs" claim is an error. Verify which scorers are currently implemented and runnable (not just defined in the codebase).

**Action:** `grep -rn "Scorer" crates/ --include='*.rs' | grep -v target/ | grep -v test` to enumerate implemented scorers. Cross-reference with doc 10's list.

**Referenced in:** Doc 10.

### Q-IN5: 4th Contributor Timing and Impact

**Looking for:** When did brapse (PR #250, Aug 10 2026) first appear? How many submissions have they made? What agent family do they use? A 4th contributor who represents a meaningfully different trace population (different agent family, different task type, different language) would unlock 3-population Hui-Walter estimation (doc 10) and potentially earlier per-subgroup conformal calibration (doc 09, Phase 4 moved to Phase 3).

**Action:** Query PR #250 metadata and submission logs.

**Referenced in:** Docs 10, 21.

---

## Category 3: Decision-Blocking Research

Queries that gate engineering decisions. Each one has a specific "Decision it unblocks" line linking to a concrete engineering choice.

### Q-DB1: Qwen3-Coder vs Qwen 3.6 35B-A3B-FP8 Model Identity

```
"Qwen3-Coder" model card specifications "35B" OR "A3B" parameters quantization 2025 2026
```
**Looking for:** TC documents use two names for the production scorer model: "Qwen 3.6 35B-A3B-FP8" (docs 02, 08, 09, 21) and "Qwen3-Coder" (doc 08 after research4 update). Are these the same model? "3.6" and "A3B" suggest a Mixture-of-Experts architecture with 35B total and 3B active parameters. "Qwen3-Coder" implies a code-specialized variant. If they are different models, TC's FIM capability assessment (doc 08) may not apply to the production scorer.

**What we already know:** Research4 confirmed that "Qwen3-Coder has native FIM." But doc 08's original text describes the production scorer as "Qwen 3.6 35B-A3B-FP8." The hyphenated naming convention suggests Qwen version 3.6, 35B parameters, A3B (Active 3B), FP8 quantization. Qwen3-Coder may be a distinct code-specialized model. TC needs to know whether the model currently deployed in the NEAR AI Cloud TEE supports FIM natively.

**Decision it unblocks:** Doc 08, Phase 3 model selection; all perplexity scoring calibration assumptions.

### Q-DB2: IronClaw Entity Typing Capabilities

```
IronClaw "NEAR AI" redaction "entity type" OR "entity recognition" OR "NER" PII detection capabilities 2026
```
**Looking for:** Doc 08 Approach 2 (typed placeholder vocabulary) depends on IronClaw producing typed redaction markers (`<PERSON>`, `<API_KEY>`, `<SECRET>`) rather than generic `[REDACTED]`. IronClaw's current redaction pipeline sometimes produces untyped markers where entity type detection failed. What are IronClaw's NER capabilities? Can they be improved upstream (in IronClaw itself), or must TC classify entities in its own post-ingestion pipeline?

**What we already know:** IronClaw is NEAR AI's open-source agent runtime (12.6K GitHub stars). Its redaction is "particularly thorough" (doc 08), which is good for privacy but compounds the perplexity penalty. No documentation of IronClaw's entity typing capabilities has been found in TC's research corpus.

**Decision it unblocks:** Doc 08, Approach 2 (typed vocabulary tokens) feasibility.

### Q-DB3: SSBC "Approximately 40% Violation Rate" Table Reference

```
"small sample" conformal prediction "violation rate" OR "miscoverage" correction table arXiv:2509.15349 2025 2026
```
**Looking for:** Doc 09 cites arXiv:2509.15349 (SSBC) for the claim that uncorrected split conformal prediction has "approximately 40% violation rate" at nominal 90% coverage for small samples (n approximately 47-100). This is a strong claim supporting the urgency of SSBC adoption. Verify: (1) that arXiv:2509.15349 contains a table or figure showing the 40% violation rate, (2) at what specific n values this occurs, (3) whether the 40% figure is for the same type of calibration TC uses (regression conformal, not classification).

**What we already know:** SSBC uses exact Beta-Binomial finite-sample distributions to correct conformal quantile indices. The correction is most impactful at small n. Doc 09 prescribes deploying the gate at n approximately 150 to avoid the worst violation rates. The 40% figure is attributed to arXiv:2509.15349 but no specific table or section is cited.

**Decision it unblocks:** Doc 09, SSBC urgency assessment.

### Q-DB4: COSINE Noise-Aware Loss Applicability to Trace Quality

```
"COSINE" OR "noise-aware" "self-training" OR "weak supervision" "non-image" OR "text" OR "tabular" domain 2025 2026
```
**Looking for:** Doc 21 (bootstrap sequencing) prescribes COSINE (arXiv:2111.14282) for training a discriminative model on noisy Snorkel-generated weak labels in Phase 2. COSINE was originally evaluated on image classification (CIFAR, WebVision). Has it been applied to text, structured data, or quality scoring tasks? If COSINE's noise-aware loss is effective only for image classification, TC may need a different noise-tolerant training objective for trace quality scoring.

**What we already know:** COSINE's core idea -- contrastive self-supervised learning with a noise-aware loss that explicitly accounts for label noise in the training objective -- is theoretically domain-agnostic. But the only published evaluations are on image benchmarks. TC's trace quality task is a structured regression/classification problem, not an image task.

**Decision it unblocks:** Doc 21, Phase 2 (noise-aware discriminative model training).

### Q-DB5: BQP Speedup Transferability to TC's Corpus

```
"BQP" OR "balanced query processing" diversity retrieval "trajectory" OR "document" OR "text" benchmark arXiv:2604.02554 2025 2026
```
**Looking for:** Doc 12 prescribes BQP (arXiv:2604.02554) as the diversity retrieval method, replacing MMR, with reported 2.4-22.9x speedup. This speedup was measured on BQP's own benchmark suite. Does the speedup transfer to TC's workload (cosine similarity over BGE-large-en-v1.5 embeddings of agent traces, HNSW index, k=5-20 retrieval)? The 2.4x lower bound vs 22.9x upper bound is a 9.5x range -- where does TC's workload land?

**What we already know:** BQP has sublinear-in-k scaling at theta >= 0.5. MMR has no approximation guarantee. Doc 12 recommends BQP but has not benchmarked it on TC's actual corpus or embedding space.

**Decision it unblocks:** Doc 12, retrieval architecture selection.

---

## Category 4: Market Intelligence

### Q-MI1: Competitive Feature Matrix Verification (August 2026)

```
Langfuse ClickHouse features "cross-user" OR "shared traces" OR "trajectory retrieval" 2026
```
```
Braintrust "agent traces" OR "trajectory" features "cross-organization" 2026
```
```
LangSmith "cross-user" OR "shared" OR "RAG" OR "retrieval" features 2026
```
**Looking for:** Docs 12, 15, and 11 make competitive claims about Langfuse, Braintrust, and LangSmith that are stated as facts but not independently verified. ClickHouse acquired Langfuse on January 16, 2026 ($400M Series D). Has the product changed since acquisition? Has any competitor added cross-user retrieval, trajectory RAG, TEE-based scoring, or contributor compensation? TC's differentiation claims need an August 2026 verification pass.

**What we already know:** Pre-acquisition Langfuse was open-source LLM observability with per-organization trace storage. No cross-user retrieval. Braintrust offered evaluation and monitoring. LangSmith offered tracing and evaluation. None offered TEE scoring or contributor compensation. But 7+ months have passed since the Langfuse acquisition.

### Q-MI2: ClickHouse/Langfuse Acquisition Verification

```
ClickHouse Langfuse acquisition "$400M" OR "Series D" OR "$15B" valuation January 2026
```
**Looking for:** Four TC documents (05, 12, 15, 22) cite this acquisition with specific numbers: "$400M Series D," "valuation tripled to approximately $15B." Verify: (1) the acquisition date (January 16, 2026), (2) the funding amount ($400M), (3) the post-acquisition ClickHouse valuation ($15B), (4) whether Langfuse's open-source community edition continues to be maintained.

**What we already know:** Research4 confirmed "ClickHouse (not Databricks) acquired Langfuse." The $400M and $15B figures appear consistently across TC documents but trace back to a single source (ClickHouse press release). Independent confirmation from tech press (TechCrunch, The Information, Bloomberg) would strengthen the citation.

### Q-MI3: VerifyWise Adoption and Maturity

```
VerifyWise "AI compliance" OR "AI governance" adoption users deployments 2026
```
**Looking for:** Doc 15 identifies VerifyWise as the closest open-source competitor for GPAI compliance tooling, but notes it uses BSL 1.1 (Business Source License -- source-available, NOT open source per OSI definition). What is VerifyWise's actual adoption? Number of deployments? GitHub stars? Active contributors? If VerifyWise has minimal adoption, TC's "no open-source GPAI compliance toolkit exists" claim is strengthened. If VerifyWise has significant traction, TC needs to differentiate more sharply.

**What we already know:** VerifyWise is source-available under BSL 1.1. Its license is NOT open source. Doc 15 positions TC as genuinely open source (MIT/Apache-2.0) vs VerifyWise's BSL. No adoption data has been collected.

### Q-MI4: AI Compliance Vendor Pricing Verification

```
"Holistic AI" pricing annual cost 2026
```
```
"Credo AI" pricing OR "annual cost" OR "subscription" enterprise 2026
```
```
TrustArc "AI compliance" pricing OR cost 2026
```
```
OneTrust "AI governance" pricing OR cost module 2026
```
**Looking for:** Doc 15 originally cited specific pricing ranges (EUR 30K-100K for Holistic AI, EUR 30K-50K for Credo AI, EUR 50K-500K for TrustArc, EUR 50K-500K for OneTrust). The v6 update changed these to "Not verified (vendor quote)" for all four vendors. TC's positioning as an open-source alternative depends on accurate competitive pricing data. Can public pricing pages, analyst reports, or Gartner/Forrester reviews provide verified pricing ranges?

**What we already know:** These are enterprise SaaS products that typically do not publish pricing publicly. The original estimates may have come from sales conversations, analyst reports, or community forums. None has been independently verified.

### Q-MI5: LLM Observability Market Sizing Verification

```
"LLM observability" market size 2025 2026 2030 CAGR "Business Research Company" OR analyst report
```
**Looking for:** Doc 15 cites market sizing from The Business Research Company via MarkTechPost: $1.97B (2025) to $9.26B (2030) at 36.2% CAGR. This is flagged as a "vendor estimate." Is The Business Research Company a recognized market research firm? Do other analysts (Gartner, Forrester, IDC, Grand View Research) have competing estimates? For grant applications, having 2+ independent market size estimates is stronger than 1.

**What we already know:** The broader "AI governance" market estimates range from EUR 7.6B to EUR 38B by 2030, but the source for these figures is unknown. The LLM observability market is a subset. No second independent estimate has been found.

---

## Category 5: Regulatory and Grants

### Q-RG1: EUR 7.6-38B Compliance Market Source

```
"AI compliance" OR "AI governance" market size EUR billion 2030 analyst report source
```
**Looking for:** Docs 05 and 07 cite a compliance market size of "EUR 7.6-38B" with no analyst source. This range appears in grant application materials. For NLnet and Horizon Europe applications, unsourced market claims are a credibility risk. Find the primary analyst report(s), or flag this range as unreliable and recommend removing it from grant materials.

**What we already know:** The $1.97B to $9.26B LLM observability figure from The Business Research Company is a different market segment (observability, not full compliance). The EUR 7.6-38B range is broader but its provenance is unknown.

### Q-RG2: Horizon Europe Call Currency and Deadlines

```
Horizon Europe "Cluster 3" OR "Civil Security for Society" call 2026 2027 "artificial intelligence" compliance
```
**Looking for:** Doc 15 flags a time-sensitive question about Horizon Europe call currency: which Horizon Europe work programmes currently fund AI compliance / trustworthy AI infrastructure? What are the upcoming deadlines? Doc 05 mentions an NLnet application deadline of November 3 (NGI Zero). Are there parallel Horizon Europe calls that TC should apply to?

**What we already know:** NLnet's NGI Zero Core/Entrust opens September 3 with the Provability Fabric as precedent. Mozilla Tech Fund has no active call. NEAR community-DAO has $45M+ available on rolling basis. Horizon Europe calls rotate on an annual cycle.

**Decision it unblocks:** Grant application strategy and timeline.

### Q-RG3: Open-Source GPAI Compliance Toolkit Gap Re-Verification

```
"open source" "GPAI compliance" OR "AI Act compliance" toolkit OR framework GitHub 2026
```
**Looking for:** Doc 15 asserts "no open-source GPAI compliance toolkit exists" based on a survey of GitHub, FOSS directories, EU AI Act compliance guides, and NLnet/NGI project lists. This negative claim was made in July-August 2026. Re-verify: the open-source landscape changes rapidly. If a competitor has emerged since the last survey, TC's grant positioning needs adjustment. Check: GitHub trending repositories with "GPAI" or "AI Act" tags, Product Hunt AI compliance launches, NLnet/NGI grantee announcements.

**What we already know:** VerifyWise uses BSL 1.1 (not OSI-open-source). No MIT/Apache/GPL-licensed GPAI compliance toolkit was found in the prior survey.

### Q-RG4: GPAI Code of Practice Updates Since July 2025

```
"GPAI Code of Practice" update OR amendment OR revision 2026
```
**Looking for:** The GPAI Code of Practice was published July 10, 2025. Has it been updated, amended, or supplemented since publication? The Digital Omnibus (Regulation (EU) 2026/1744, in force July 27 2026) may have triggered updates to the Code. TC's compliance positioning references the July 2025 version -- if there are newer versions, the mapping to TC capabilities may need revision.

**What we already know:** The Code has three chapters (Transparency, Copyright, Safety & Security). The training data summary template has a confirmed 3-section structure. No updates since the initial publication have been referenced in TC's research corpus.

---

## Category 6: Mechanism Design

### Q-MD1: VCG Production Deployments for Data Pricing

```
"VCG" OR "Vickrey-Clarke-Groves" "production" OR "deployed" OR "real-world" "data marketplace" OR "data pricing" OR "data valuation" 2025 2026
```
**Looking for:** TC's mechanism design pivot from Shapley to VCG (doc 16) is theoretically motivated but lacks real-world precedent. Has any production system deployed VCG or a VCG variant for data valuation or data marketplace pricing? What were the engineering challenges? How did they handle the utility computation at scale? If no production VCG deployment exists, TC would be pioneering -- which is both a differentiator and a risk.

**What we already know:** `vcg_allocate` is built in TC's codebase but the greedy path dominates at runtime. VCG is O(n log n) for homogeneous multi-unit auctions. Q-MIA (arXiv:2506.05379) provides a budget-balanced alternative. The Credibility Trilemma (arXiv:2605.26604) proves ghost-bid deviations are undetectable under sealed-bid VCG.

### Q-MD2: Owen Sampling Speed at TC's Contributor Count

```
"Owen sampling" OR "Owen value" computational cost "small" OR "few" contributors Shapley approximation 2025 2026
```
**Looking for:** Doc 16 proposes Owen Sampling (arXiv:2508.21261) for multi-agent credit apportionment. Owen values generalize Shapley to coalition structures. At TC's current 3 contributors (and anticipated 5-10 near-term), what is the computational cost of Owen Sampling? Is it tractable with < 10 contributors? At what contributor count does approximation become necessary?

**What we already know:** Shapley is gameable and being replaced. Owen values handle coalition structure (useful for multi-agent traces where one orchestrator + sub-agents submit together). arXiv:2508.21261 provides the sampling algorithm. No runtime performance data at small N.

### Q-MD3: Collusion Resistance Mechanisms for Single-Digit Participant Counts

```
"collusion" OR "cartel" resistance mechanism design "small number" OR "few" participants data marketplace 2025 2026
```
**Looking for:** Doc 16 acknowledges that at N=3 contributors, "collusion is the entire contributor base" and that payment-mechanism-based collusion resistance is impossible. Quality gates + provenance attestation carry the anti-manipulation load instead. Are there mechanism design approaches specifically for single-digit participant counts? Commitment schemes? Reputation staking? Security deposits that are forfeited on detected collusion? TC needs a principled answer for the N=3 to N=10 range.

**What we already know:** VCG/MUT are DSIC under unilateral deviation but not collusion-proof. The Credibility Trilemma proves ghost-bid deviations are undetectable under sealed-bid VCG. TC's TEE is positioned as the closure mechanism. At N=3, every possible subset of participants is either a singleton or a majority -- classical anti-collusion mechanisms assume a large honest majority.

### Q-MD4: Ocean Protocol Current State and Lessons

```
"Ocean Protocol" "TVL" OR "total value locked" OR "adoption" OR "state" 2026
```
**Looking for:** Doc 16 references Ocean Protocol's AMM-based data marketplace as a cautionary tale ("struggled with liquidity"). What is Ocean Protocol's current TVL/state as of August 2026? What specific lessons has TC extracted from Ocean's experience? Has Ocean pivoted its mechanism design? If Ocean has collapsed or pivoted, that strengthens TC's VCG argument. If Ocean has found traction, TC should study what worked.

**What we already know:** Ocean used an AMM (Automated Market Maker) for data pricing. Doc 16 suggests this "struggled with liquidity" but provides no current data.

### Q-MD5: Vana VRC-14 Adoption Status

```
"Vana" "VRC-14" adoption OR implementation OR deployment data marketplace 2026
```
**Looking for:** Doc 16 cites Vana's pivot from emissions-based to usage-linked rewards (VRC-14) as the model for TC's Phase 3 credit system (50% access fees, 30% usage, 20% quality). Has VRC-14 been adopted? By how many data DAOs? What is the measured impact on contributor behavior? If VRC-14 is still a proposal with no adoption, TC cannot rely on it as a validated design.

**What we already know:** Vana pivoted from emissions to usage-linked rewards. The 50/30/20 split is cited as VRC-14's structure. No adoption data has been found.

---

## Category 7: TEE and Determinism

### Q-TD1: VS-Graph Accuracy on Trajectory Graphs vs Molecular Graphs

```
"VS-Graph" OR "hyperdimensional" graph classification "trajectory" OR "heterogeneous" OR "non-molecular" accuracy benchmark 2025 2026
```
**Looking for:** VS-Graph (arXiv:2512.03394) reports 450x speedup over GNNs on standard graph benchmarks (MUTAG, DD, PROTEINS). These are molecular and protein graphs with homogeneous node types and small sizes. TC's trajectory graphs are different: 10-200 nodes, heterogeneous node types (LLM calls, tool invocations, file edits, test results), directed edges with temporal ordering. Doc 17 flags this caveat explicitly: "accuracy caveat (MUTAG/DD benchmarks not equal to TC trajectory graphs)." Has VS-Graph been evaluated on directed, heterogeneous graphs?

**What we already know:** VS-Graph uses HDC (Hyperdimensional Computing) with XOR composition. TC already uses HDC fingerprints per episode. The XOR composition of content and structure fingerprints is proposed but not theoretically justified. VS-Graph is pure Rust, which aligns with TC's stack.

**Decision it unblocks:** Doc 17, structural embedding method selection.

### Q-TD2: Floating-Point Timing Side-Channels in Brute-Force HDC Scan

```
"side channel" "timing" "floating point" OR "cosine similarity" "trusted execution" OR "TEE" OR "enclave" 2025 2026
```
**Looking for:** Docs 12, 13, and 17 recommend brute-force HDC scan (not HNSW) for deterministic, side-channel-free retrieval at TC's current scale. The determinism argument is clear (no randomized HNSW layers). The side-channel argument is less clear: floating-point cosine similarity computation may have data-dependent timing due to denormalized numbers, NaN handling, or branch prediction differences. Are there known floating-point timing side channels that would leak information about the query or index contents during brute-force similarity search?

**What we already know:** HNSW has random memory access patterns that leak information via cache side channels. Brute-force scan has sequential access patterns that are harder to observe. But the computation itself (dot product, normalization) involves floating-point operations that may not be constant-time. Doc 12 flags this as an open question.

### Q-TD3: WASM Plugin RTMR Attestation Performance Overhead

```
"WASM" OR "WebAssembly" "TEE" OR "TDX" attestation "RTMR" OR "measurement" performance overhead 2025 2026
```
**Looking for:** Doc 22 describes WASM-sandboxed scorer plugins that extend TDX RTMRs (Runtime Measurement Registers) at load time. Each plugin's WASM binary hash is measured into an RTMR. What is the per-plugin attestation overhead? If TC loads 5-10 scorer plugins, does the cumulative RTMR extension add measurable latency? Is there a limit on RTMR depth?

**What we already know:** Intel TDX provides 4 RTMRs (RTMR[0-3]). Extending an RTMR is a cryptographic hash operation (SHA-384). Doc 22 describes the architecture but provides no performance measurements. The per-extension cost should be microseconds, but cumulative effects with 5-10 plugins are unquantified.

**Decision it unblocks:** Doc 22, WASM scorer plugin architecture feasibility.

### Q-TD4: ONNX Runtime Determinism in TDX Enclaves

```
"ONNX Runtime" OR "ort" deterministic inference "TDX" OR "SGX" OR "enclave" "trusted execution" 2025 2026
```
**Looking for:** Doc 17's GNN path requires ONNX Runtime via the `ort` Rust crate inside TC's TEE. arXiv:2501.05867 (cited in doc 17) flags ONNX as "non-deterministic in TEEs." Has anyone achieved deterministic ONNX Runtime inference in Intel TDX enclaves? What are the sources of non-determinism (thread scheduling, BLAS library, memory allocation)? What is the performance overhead of forcing determinism (e.g., single-threaded execution, deterministic BLAS)?

**What we already know:** Three sources of non-determinism in TEE inference: (1) floating-point operations with non-deterministic reduction ordering, (2) HNSW randomized layers, (3) thread scheduling. Brute-force HDC scan avoids (2) and (3) but not (1). ONNX Runtime for GNN inference introduces additional non-determinism from graph execution optimization.

**Decision it unblocks:** Doc 17, GNN inference path feasibility in TEE.

### Q-TD5: NEAR Gas Cost for On-Chain Attestation Accumulation

```
NEAR gas cost "attestation" OR "on-chain" storage accumulation "per transaction" 2026
```
**Looking for:** Doc 13's Phase 4 envisions on-chain attestation accumulation: each scored trace's attestation is anchored to the NEAR blockchain. What is the per-transaction NEAR gas cost for storing an attestation hash? At 13 submissions/week, what is the weekly gas budget? At 100 submissions/week (growth target), does the gas cost become a constraint? Does NEAR support batch attestation to amortize gas costs?

**What we already know:** NEAR gas costs are denominated in NEAR tokens. The per-transaction cost depends on storage size and compute. A SHA-384 hash (48 bytes) is small, but each transaction incurs base gas costs. No gas cost estimate exists in TC's documents.

**Decision it unblocks:** Doc 13, Phase 4 on-chain attestation economics.

---

## Category 8: Scoring Pipeline

### Q-SP1: Replacement Metric for Confounded AUC > 0.93

```
"evaluation metric" "confounded" OR "confound" "replacement" OR "alternative" quality scoring "ground truth free" 2025 2026
```
**Looking for:** TC's headline quality metric (AUC > 0.93, docs 01, 02, 05) is known to be confounded: paragraph count achieves AUC 1.000 on the bake-off corpus (PR #216). This means the metric is meaningless -- it measures structural features, not quality. What replacement metric can TC cite in grant applications and public materials? Options include: (1) inter-annotator agreement on human-labeled subset (requires completing doc 19's annotation protocol), (2) downstream task improvement from retrieved traces (requires trajectory RAG deployment, doc 12), (3) calibration error of the conformal gate (requires deploying the gate, doc 09). Which is fastest to produce?

**What we already know:** The confound is structural: longer traces with more paragraphs score higher. The bake-off corpus lacks diversity (PR #216). The replacement must be robust to structural confounds. Doc 10's Judge-Aware BTL approach estimates quality without ground truth but requires 4+ independent judges.

**Decision it unblocks:** All grant applications and public-facing quality claims.

### Q-SP2: Contrastive Learning for Trace Embeddings (Corrected Citation)

```
contrastive learning "trace embedding" OR "trajectory embedding" OR "sequence embedding" representation learning 2025 2026
```
**Looking for:** Doc 07 Q-S7 proposed contrastive learning for trace embeddings, but the cited paper (arXiv:2509.24291) was identified as GIRCSE (generative contrastive sentence embeddings), NOT a hard-negative mining paper. The hard-negative mining citation remains UNVERIFIED. What is the correct paper for hard-negative mining in contrastive learning for structured sequence embeddings? Are there papers applying contrastive learning specifically to agent trace or execution trace embeddings?

**What we already know:** arXiv:2509.24291 is GIRCSE, which is about generative contrastive sentence embeddings -- tangentially related but not the hard-negative mining paper that was claimed. Doc 17 adds structural embeddings as a complementary approach. The gap: no verified citation for hard-negative mining applied to trace-like data.

### Q-SP3: Tool-Call Graph Extraction Methods

```
"tool call" OR "function call" graph extraction "agent trace" OR "execution trace" dependency "data flow" 2025 2026
```
**Looking for:** Doc 17 describes a 6-step tool-call graph extraction pipeline for building structural embeddings. Step 4 (data-flow edge detection -- inferring that tool B consumed data produced by tool A) is explicitly deferred as "ambiguous." GRADE (arXiv:2606.22741) distinguishes execution-layer from dependency-layer graph projections. Does GRADE include an extraction algorithm for the dependency layer? Are there other tools or libraries that extract dependency graphs from agent execution logs?

**What we already know:** GRADE provides the conceptual framework (execution-layer vs dependency-layer). PM4Py has an LLM module for process mining. Agent Behavior Mining (arXiv:2606.20669) applies classical process mining to agent traces. None of these are confirmed to solve TC's specific data-flow edge extraction problem.

### Q-SP4: Structural Novelty Contribution Magnitude for Traces

```
"structural" OR "graph" "novelty" OR "anomaly" detection "additional" OR "complementary" "text" OR "content" embedding 2-10 improvement 2025 2026
```
**Looking for:** Doc 17 cites MCPShield (arXiv:2605.11053) for the finding that structural features add 2-10 percentage points of AUC for novelty detection. The range is wide. For trace novelty detection specifically, where on the 2-10pp spectrum does the contribution fall? Is structural signal more or less valuable for trace novelty than for the attack detection task MCPShield evaluated? TC needs this before deciding how much engineering effort to invest in structural embeddings (doc 17, priority 3).

**What we already know:** MCPShield's 2-10pp result is for attack/anomaly detection in MCP tool calls, which is related but not identical to TC's trace novelty detection task. TC's novelty signal is primarily textual (perplexity, token rarity) and distributional (cosine distance). Structural signal from tool-call graphs could capture novelty in agent behavior patterns that textual methods miss.

### Q-SP5: Rust Fraction in Open-SWE-Traces

```
"Open-SWE-Traces" NVIDIA language distribution Rust fraction dataset statistics
```
**Looking for:** Open-SWE-Traces (207,489 trajectories, arXiv:2606.16038, doc 14) spans 9 programming languages including Rust. TC is a Rust codebase and Rust-language traces are particularly valuable for calibrating TC's own scoring pipeline (the scorer sees Rust syntax patterns frequently). What fraction of the 207K trajectories are Rust? If Rust is < 1% (< 2K traces), the seeding value for Rust-specific calibration is limited. If > 5% (> 10K traces), Rust traces alone provide a substantial calibration corpus.

**What we already know:** The 9 languages are Python, Go, TypeScript, JavaScript, Rust, Java, PHP, C, C++. Python likely dominates given the SWE-bench origin. The Rust fraction is unknown. This is answerable from the dataset itself (HuggingFace dataset inspection).

**Decision it unblocks:** Doc 14, corpus seeding strategy for Rust-specific calibration.

### Q-SP6: Perplexity-Embedding Correlation in TC's Corpus

```
"perplexity" "embedding" "correlation" OR "orthogonal" OR "complementary" language model scoring 2025 2026
```
**Looking for:** Doc 02 identifies 7 open questions, one of which is whether perplexity and embedding cosine similarity scores are correlated or orthogonal for TC's traces. If highly correlated (r > 0.8), one scorer adds little information beyond the other, and the 4-judge ensemble (doc 10) effectively has only 3 independent signals. If orthogonal (r < 0.3), the two scorers capture different quality dimensions and the ensemble benefits from both.

**What we already know:** PerplexityScorer and TokenRarityScorer share a forward pass (doc 10 -- collapsed to ForwardPassJudge). Embedding cosine (BGE-large-en-v1.5) uses a separate model. The theoretical expectation is partial correlation (both capture language model quality, but perplexity measures local coherence while embedding similarity measures global distributional position). No empirical measurement exists for TC's corpus.

---

## Category 9: Annotation and Human-in-the-Loop

### Q-AH1: Optimal k for TypiClust on TC's 352 Traces

```
"TypiClust" OR "cluster-based" active learning "optimal k" OR "number of clusters" "cold start" 300 400 500 2025 2026
```
**Looking for:** Doc 19's cold-start annotation protocol begins with TypiClust (arXiv:2202.02794): embed all traces via BGE-large-en-v1.5, cluster into k groups, select 1-2 traces per cluster centroid for human annotation. What is the optimal k for n=352 traces in a high-dimensional embedding space (768 dimensions for BGE-large-en-v1.5)? Standard heuristics (k = sqrt(n/2) approximately 13) may not apply in high dimensions.

**What we already know:** TypiClust maximizes distributional coverage before any model uncertainty is available. Doc 19 prescribes "15-20 groups" in Week 1, selecting 1-2 per centroid for 15-20 labeled traces. The 15-20 number is not derived from TC's specific embedding geometry -- it is an estimate.

### Q-AH2: LLM Pre-Labeling Anchoring Bias Quantification

```
"LLM" "pre-labeling" OR "pre-annotation" "anchoring bias" OR "priming" quantitative human annotation 2025 2026
```
**Looking for:** Doc 19 proposes LLM pre-labeling with human review to speed up annotation (estimated 2-3x speedup). Doc 19 also warns that this introduces anchoring bias: human reviewers who see the LLM's label may be biased toward confirming it. Doc 19 mitigates with "blind labeling for the first 30 traces" and comparing agreement between blind and pre-labeled batches. But the expected bias magnitude is explicitly described as the "thinnest-sourced" claim in doc 19. Are there controlled experiments measuring anchoring bias magnitude in LLM-assisted annotation?

**What we already know:** If anchoring bias is severe (agreement between blind and pre-labeled batches < 80%), all calibration-critical traces must be blind-labeled, which eliminates the speedup. Doc 19 does not cite any source for the expected bias magnitude. This is listed as Gap U1 in doc 19.

### Q-AH3: Snorkel Weak Supervision in Rust (Updated)

```
"weak supervision" OR "data programming" OR "label model" Rust crate OR library implementation 2025 2026
```
**Looking for:** Doc 19 and doc 21 both depend on Snorkel-style weak supervision for the bootstrap sequence. Doc 19 estimated that Snorkel's core algorithm (generative model over labeling function outputs) is reimplementable in approximately 200-500 LOC of Rust. Has anyone published a Rust implementation? A Rust crate? Or is TC still the first? If a crate exists, how mature is it? Does it include Snorkel's conflict resolution and dependency modeling, or just majority vote?

**What we already know:** No Rust implementation was found in the prior research sweep. Snorkel's label model is conceptually a small generative model (majority vote with learned accuracies). The full Snorkel pipeline includes conflict resolution, dependency modeling (junction tree), and noise-aware training. Doc 19 lists 6 labeling functions with estimated accuracies (unsourced). The junction tree / eigendecomposition step requires linear algebra in Rust (nalgebra crate exists).

### Q-AH4: Krippendorff Alpha for Code Novelty -- Primary Source

```
"Krippendorff" alpha "code" OR "software" "novelty" OR "similarity" OR "duplicate" assessment inter-annotator 2025 2026
```
**Looking for:** Doc 19 sets inter-annotator agreement thresholds (< 0.50 = too subjective, 0.67 = usable, > 0.8 = ground truth) but cites these thresholds as an "internal citation" needing a primary source. What are published Krippendorff Alpha values for code similarity / code novelty assessment tasks? If code novelty assessment is inherently low-agreement (alpha approximately 0.3-0.5, similar to "code quality" or "code readability" assessments), doc 19's thresholds may be unrealistic.

**What we already know:** The general Krippendorff Alpha thresholds (0.67 for tentative conclusions, 0.80 for reliable conclusions) come from Krippendorff's own 2004 textbook. But these are domain-independent. Code-specific baselines would be more informative for TC's task ("is this trace novel?").

### Q-AH5: Annotation Tool Selection for Trace Labeling

```
"annotation tool" OR "labeling tool" "trace" OR "structured data" OR "sequence" labeling "open source" 2025 2026
```
**Looking for:** Doc 19 describes a 4-week annotation protocol but never specifies what tool reviewers will use. The annotation task is binary (NOVEL / NOT_NOVEL) with reference to the full trace structure, not a text snippet. What annotation tools support structured/sequence labeling? Options: Label Studio (open source, flexible schemas), Prodigy (commercial, NLP-focused), Argilla (open source, LLM-focused), custom Streamlit/Gradio app, or even a structured spreadsheet. Which is fastest to deploy for TC's specific task?

**What we already know:** The task is binary classification with possible graded extension in Week 4. Annotators need to see the full trace structure (tool calls, timestamps, outcomes). Standard text annotation tools may not render trace data well. A custom UI might be needed.

---

## Category 10: Agent Ecosystem

### Q-AE1: Agent Skills Market Size Verification

```
"agent skills" OR "AI agent tools" OR "MCP tools" registry market size total number 2026
```
**Looking for:** Docs 01, 03, and 11 cite "490K+ skills" and "32+ adopters" without any source. These market size claims appear in TC's verified skills narrative and grant materials. Where do these numbers come from? Is there a registry or aggregator that tracks total Agent Skills? Are the numbers current? If these are estimates or extrapolations, TC should cite them as such or remove them from external-facing materials.

**What we already know:** The Agent Skills ecosystem includes multiple registries. Claude Code, Codex, Cursor, Gemini CLI, Windsurf are listed as adopters. No primary source for "490K+" or "32+" has been found.

### Q-AE2: ClawHavoc Incident Details and Source

```
"ClawHavoc" malicious skills attack "341" OR "hundreds" agent registry 2026
```
**Looking for:** Docs 01, 03, and 11 cite the ClawHavoc attack -- "a single operation planted hundreds of malicious skills across public registries" with a specific count of "341 malicious skills." What is the primary source? Is ClawHavoc a published security disclosure, a conference talk, a blog post, or an industry report? The 341 count is specific enough to require a citation. If ClawHavoc is from a conference talk or industry report, TC needs the specific reference.

**What we already know:** ClawHavoc is referenced alongside SkillVetBench (arXiv:2606.00925) and MalSkillBench (arXiv:2606.07131) as motivation for TC's verified skills tier. The "ClawHavoc Analysis" paper has no arXiv ID.

### Q-AE3: Claude Code Hook Count and Documentation

```
"Claude Code" hooks lifecycle events "SessionEnd" OR "hook" documentation count 2026
```
**Looking for:** Doc 01 claims Claude Code has "30 lifecycle hook events" without citation. Docs 01 and 22 both depend on the `SessionEnd` hook for TC's primary Claude Code integration path. Verify: (1) how many hooks does Claude Code actually expose? (2) Is the `SessionEnd` hook documented in Claude Code's official documentation? (3) What is the timeout constraint (doc 22 mentions 1.5s from doc 01)?

**What we already know:** The SessionEnd hook fires on session completion and provides access to the session transcript. The 1.5s timeout constraint means TC must submit asynchronously via a background daemon (PR #244, merged). No official documentation URL for Claude Code hooks has been cited.

### Q-AE4: OTel GenAI Stable Graduation Timeline

```
"OTel" OR "OpenTelemetry" "GenAI" OR "gen_ai" "stable" graduation timeline 2026 2027
```
**Looking for:** Doc 18 states that all `gen_ai.*` semantic conventions are at "Development" status with no stable graduation timeline. The dedicated `semantic-conventions-genai` repository has no tagged release and no finalized schema URL. Is there any public indication of when OTel GenAI conventions might reach Stable status? OTel SIG meeting minutes? Roadmap documents? CNCF board discussions? TC's pinning strategy (doc 18, doc 22) assumes instability will persist for months to years -- if Stable graduation is imminent, the pinning strategy's priority changes.

**What we already know:** GenAI conventions moved to a dedicated repository at main-repo v1.42.0 (June 12, 2026). The `gen_ai.system` to `gen_ai.provider.name` rename at v1.39.0 was a breaking change. Dual-emission via `OTEL_SEMCONV_STABILITY_OPT_IN` is the prescribed mitigation.

**Decision it unblocks:** Doc 18, pinning strategy urgency.

### Q-AE5: OTel GenAI vs OpenInference Adoption Split

```
"OpenInference" vs "OTel GenAI" adoption comparison usage 2026
```
**Looking for:** Doc 22 identifies two competing semantic convention sets (OTel GenAI and OpenInference) and prescribes supporting both. But what is the adoption split? If 90% of agent frameworks use OTel GenAI and 10% use OpenInference, TC should prioritize OTel GenAI normalization. If the split is closer to 50/50, both paths are equally important. Relevant data: which agent frameworks and observability backends use which convention set?

**What we already know:** OTel GenAI is CNCF-backed, adopted by Datadog, Elastic, Honeycomb, OpenAI SDK. OpenInference is Arize AI's convention set, adopted by LangChain, LlamaIndex, Arthur AI. LangChain is one of the most popular agent frameworks. The split matters for TC's engineering prioritization.

### Q-AE6: A2A Phase 3 Deferral Justification

```
"A2A" OR "Agent to Agent" protocol multi-agent identity credit attribution 2026
```
**Looking for:** Doc 22 defers A2A identity support (cross-agent credit attribution for multi-agent workflows) to Phase 3 (2-4 months). The justification is that "multi-agent workflows are still early." Is this accurate as of August 2026? How many production multi-agent systems generate traces that TC could ingest? If multi-agent adoption is accelerating, deferring A2A support may miss the adoption window. If still early, the deferral is correct.

**What we already know:** A2A v1.0.0 has 150+ organizations. AAIF hosts both A2A and MCP. Doc 16 Q-IM4 flags multi-agent credit apportionment as an open problem. No extensions of VCG/Shapley/MUT for multi-contributor items have been found.

---

## Category 11: Retained from Prior Documents (Updated Context)

These queries from docs 07 and 20 were NOT fully answered by the research sweeps and remain open. Each is updated with new context from docs 08-22.

### Q-S3 (retained): Process Mining for Agent Traces

```
"process mining" "AI agent" OR "LLM agent" trace analysis false positive rate "conformance" novelty 2025 2026
```
**Updated context (from docs 08-22):** GRADE (arXiv:2606.22741) provides a better graph representation than flat conformance checking by distinguishing execution-layer from dependency-layer projections. But TC still needs empirical data on false-positive rates for conformance-based novelty detection on agent traces specifically. Doc 17 adds structural embeddings as a complementary approach to process mining.

### Q-S4 (retained): Causal Attribution Without Re-Execution

```
"causal" "failure attribution" "offline" OR "observational" agent trace "without re-execution" 2025 2026
```
**Updated context:** GraphTracer was WITHDRAWN (arXiv:2510.10581 v2, December 2025, "fundamental error in methodology"). Zero-Replay Debugging (arXiv:2606.14805) remains the most practical offline approach but has not been evaluated on TC's trace data. No new causal attribution methods surfaced in docs 08-22.

### Q-S5 (retained): Joint Compression and Quality Scoring

```
"joint" compression "quality scoring" OR "quality estimation" agent trace "lossy" OR "distillation" 2025 2026
```
**Updated context:** No new findings from docs 08-22. Governance Decay (arXiv:2606.22528) remains the key safety constraint -- compression must preserve the information needed for governance verification. ACE (arXiv:2606.31564) and Slipstream (arXiv:2605.08580) are the compression SOTA but neither integrates with quality scoring.

### Q-S7 (retained): Contrastive Learning for Trace Embeddings (CORRECTED)

```
contrastive learning "hard negative mining" trace OR trajectory embedding representation 2025 2026
```
**Updated context:** CRITICAL CITATION CORRECTION. arXiv:2509.24291 is actually GIRCSE (generative contrastive sentence embeddings), NOT the hard-negative mining paper previously cited in doc 07. The hard-negative mining source remains UNVERIFIED and needs a correct citation. Doc 17 adds structural embeddings (VS-Graph) as a complementary approach to text-based contrastive learning.

### Q-S8 (retained): Concept Drift Detection for Trace Populations

```
"concept drift" detection "conformal" OR "non-parametric" "distribution shift" "data quality" 2025 2026
```
**Updated context:** WATCH (arXiv:2505.04608) is detailed in doc 09 as the drift trigger for conformal recalibration. WATCH uses conformal martingales to detect when the calibration set is stale without requiring labels. But WATCH has not been evaluated on TC's data. Doc 21's Phase 4 prescribes deploying WATCH after the conformal gate is calibrated.

### Q-I2 (retained): Claude Code Hook Integration Patterns

```
"Claude Code" "hook" OR "extension" OR "plugin" integration patterns "session end" background daemon 2026
```
**Updated context:** The SessionEnd timeout constraint (1.5s) is confirmed. Background daemon (PR #244, merged) is the prescribed architecture. Doc 22 specifies the full ingest path: SessionEnd fires, hook invokes `tc submit`, transcript uploaded to TC ingest. No new integration patterns surfaced.

### Q-I4 (retained): Cross-Agent Session Formats

```
"Claude Code" OR "Codex" OR "Cursor" OR "Copilot" OR "Gemini CLI" session format export local storage 2026
```
**Updated context:** Doc 22 provides the full cross-agent session format matrix: Claude Code (JSON/JSONL, yes parseable), Codex (JSON/JSONL, yes), Cursor (SQLite/JSON, partially -- not publicly documented, changes across releases), Copilot (proprietary, API-only), Gemini CLI (JSON/JSONL, yes -- format inferred, not confirmed), IronClaw (TC-native). Cursor and Gemini CLI formats remain unverified.

### Q-I5 (retained): A2A Observability and Multi-Agent Tracing

```
"A2A" "multi-agent" tracing "observability" OR "telemetry" "credit attribution" 2026
```
**Updated context:** GRADE (arXiv:2606.22741) provides dependency-graph correlation beyond W3C traceparent. Multi-agent credit apportionment (doc 16 Q-IM4) is now a dedicated problem. Owen Sampling (arXiv:2508.21261) addresses coalition structure in credit allocation. Doc 22 defers A2A identity support to Phase 3.

### Q-G1-G5, Q-G7 (retained): Growth and Distribution

Queries about TC's growth strategy, distribution channels, and adoption metrics. These were identified as the "thinnest-sourced area" in the prior research sweep and remain so. No new findings from docs 08-22. Priority for next research round.

**Q-G1**: cargo-dist adoption rates and time-to-first-install benchmarks.
**Q-G2**: Developer tool distribution channel effectiveness (homebrew vs cargo install vs binary download vs npm wrapper).
**Q-G3**: Sentry wizard model replication -- implementation details and success metrics.
**Q-G4**: Error Hub flywheel -- comparison to similar community-driven error databases.
**Q-G5**: Open-source CLI tool growth benchmarks (first-year stars/downloads/contributors for ripgrep, bat, delta, etc.).
**Q-G7**: Developer community engagement patterns for data contribution tools (vs. passive tools).

### Q-M1-M5 (retained): Strategy and Market

Queries about TC's strategic positioning, competitive moats, and market timing. Updated with doc 15 (GPAI compliance) and doc 16 (incentive design) context.

**Q-M1**: Data commons governance models -- what worked and what failed (Wikipedia, OpenStreetMap, Common Crawl).
**Q-M2**: Developer data marketplace precedents -- has any developer tool successfully monetized contributed data?
**Q-M3**: NEAR ecosystem adoption trends -- is the NEAR blockchain gaining or losing developer mindshare?
**Q-M4**: "Contributor-owned" data registry positioning -- does this language resonate with enterprise buyers?
**Q-M5**: Credit/token-based incentive system design for data contribution -- lessons from prior token-incentive failures. Updated: doc 16 provides the mechanism design framework (VCG replacing Shapley), but real-world precedent remains absent.

### Q-T1 (retained): Streaming Agent Trace Analysis

```
"streaming" OR "real-time" "agent trace" OR "execution trace" quality analysis feedback 2025 2026
```
**Updated context:** No new findings from docs 08-22. Real-time quality feedback during agent execution remains unexplored. The SessionEnd hook (doc 22) fires only on completion, not during execution.

### Q-T2 (retained): Multi-Modal Agent Trace Analysis

```
"multi-modal" agent trace "screenshot" OR "computer use" analysis scoring 2025 2026
```
**Updated context:** No new findings. Computer-use agents (Anthropic's computer use, OpenAI's operator) generate traces with screenshots. TC's pipeline is text-only. No assessment of multi-modal trace scoring exists.

### Q-T4 (retained): Sleep-Time Trace Processing

```
"idle time" OR "background" OR "sleep time" processing agent trace quality scoring 2025 2026
```
**Updated context:** Background daemon (PR #244, merged) provides the infrastructure for idle-time processing. But specific patterns for background trace processing (re-scoring, index rebuilding, quality recalibration) are not researched.

### Q-T5 (retained): Skill Extraction from Agent Traces

```
"skill extraction" OR "capability extraction" "agent trace" OR "execution trace" automated 2025 2026
```
**Updated context:** Doc 11 provides the "verified skills tier" architecture that consumes extracted skills. RHO (arXiv:2606.05922), Trace2Skill (arXiv:2603.25158), and SkillAudit (arXiv:2606.14239) remain UNVERIFIED. The skill extraction pipeline is a prerequisite for the verified skills tier.

### Q-U1 (retained): Federated Learning for Scorer Updates

```
"federated learning" "scorer" OR "model" update heterogeneous ensemble privacy 2025 2026
```
**Updated context:** No new findings. Note: arXiv:2504.17703 (federated learning survey) was WITHDRAWN due to disputed authorship. Do not cite. FedAvg feasibility for heterogeneous scorer ensembles (where each contributor trains on their own data) is unknown.

### Q-U3 (retained): Reward Modeling from Pairwise Preferences

```
"reward model" "pairwise preference" OR "comparison" "human feedback" quality 2025 2026
```
**Updated context:** Now linked to Q-AH2 (anchoring bias) and doc 19's pairwise comparison recommendation. DynaCF (arXiv:2606.09043) and ConsistRM (arXiv:2604.07484) remain UNVERIFIED.

### Q-U4 (retained): Online Anomaly Detection for Trace Scoring

```
"online anomaly detection" OR "streaming anomaly" "conformal" OR "non-parametric" data quality 2025 2026
```
**Updated context:** CALIBURN (arXiv:2605.24696) remains UNVERIFIED. Doc 09 adds WATCH (arXiv:2505.04608) for drift detection, which is related but addresses calibration staleness rather than online anomaly detection.

### Q-X1-X6 (retained): Cross-Domain Inspiration

Exploratory queries seeking methods from adjacent domains that could transfer to TC's problems. No new findings from docs 08-22.

**Q-X1**: Supply chain provenance systems -- attestation chain patterns applicable to trace provenance.
**Q-X2**: Music information retrieval -- audio fingerprinting techniques applicable to trace fingerprinting.
**Q-X3**: Video summarization -- temporal summarization techniques applicable to trace compression. Most promising unexplored transfer.
**Q-X4**: Federated analytics -- privacy-preserving aggregate statistics applicable to cross-contributor analysis.
**Q-X5**: Recommender system cold-start -- techniques for bootstrapping recommendation quality with few items.
**Q-X6**: Peer review systems -- incentive-compatible review mechanisms applicable to trace quality assessment.

---

## Cross-Document Dependencies

These are not research queries per se, but structural dependencies that block multiple documents simultaneously. Resolving each one unblocks progress across 3+ docs.

| Dependency | Blocks | Status | Next Action |
|---|---|---|---|
| **Issue #210 (0/99 accepted)** | Docs 02, 09, 10, 21 (all scoring/calibration work) | Open | Deploy permissive threshold (doc 21, Phase 0) as immediate unblock; conformal gate is Phase 3 |
| **Issue #219 (redaction penalty)** | Docs 02, 08, 10 (quality scoring accuracy) | Open | Placeholder-excluded pseudo-perplexity (doc 08, Approach 1) is hours of work |
| **MinHash and NCD not wired** | Docs 02, 10, 12, 21 (3+ docs depend on 4-judge ensemble) | Open | Wire MinHash via Rensa crate (1-2 days, doc 02) |
| **Human annotation protocol undefined** | Docs 10, 19, 21 (quality estimation, bootstrap) | Open | Select annotation tool (Q-AH5), begin TypiClust cold start |
| **Bake-off corpus confound** | Docs 02, 10 (all quality validation) | Open | No timeline for replacement corpus; doc 14 (Open-SWE-Traces seeding) partially addresses |
| **TokenRarityScorer not wired as ForwardPassJudge** | Docs 02, 10, 21 (scorer independence) | Open | Hours of work (doc 21, Phase 1) |

---

## Summary Statistics

| Category | Query Count | New in v5 | Retained from v4/v3 |
|---|---|---|---|
| Citation Verification Batch | (table, not counted as queries) | -- | -- |
| Internal Data Questions (INTERNAL) | 5 | 5 | 0 |
| Decision-Blocking Research | 5 | 4 | 1 (Q-DB5 extends Q-TR5) |
| Market Intelligence | 5 | 3 | 2 (Q-MI1 = Q-CD5, Q-MI4 = Q-GP4) |
| Regulatory and Grants | 4 | 2 | 2 (Q-RG3 = Q-GP3, Q-RG4 new) |
| Mechanism Design | 5 | 3 | 2 (Q-MD1 = Q-IM1, Q-MD3 = Q-IM3) |
| TEE and Determinism | 5 | 3 | 2 (Q-TD4 = Q-SE4, Q-TD5 new) |
| Scoring Pipeline | 6 | 4 | 2 (Q-SP2 = Q-S7 corrected, Q-SP6 new) |
| Annotation and Human-in-the-Loop | 5 | 2 | 3 (Q-AH3 = Q-GT4, Q-AH4 = Q-GT5, Q-AH5 = Q-GT2) |
| Agent Ecosystem | 6 | 5 | 1 (Q-AE4 extends Q-CC4) |
| Retained from Prior Docs | 23 | 0 | 23 |
| **Total** | **69** | **31** | **38** |

Plus the citation verification batch (34 arXiv papers, 29 no-ID papers, 16 non-paper claims = 79 items for batch verification).
