# Research Foundations & Paper Index

**Date**: August 2026

This document consolidates TC's research grounding: verified findings from deep research, the paper index (~122 papers), and research queries for continued exploration.

---

## 1. Highest-Impact Research Findings

Seven verified, directly implementable findings from the August 2026 deep research rounds.

### 1.1 Label-Free Quality Scoring

**Judge-Aware Ranking Framework** (arXiv 2601.21817, ICML 2026 -- verified). Jointly estimates latent trace quality and scorer reliability from pairwise comparisons without reference labels. Proves identifiability + consistency.

**Hui-Walter Bayesian estimator** (arXiv 2401.09376 -- verified). Estimates classifier sensitivity/specificity with no gold standard from cross-classified agreement across 2+ prevalence-differing populations.

**TC use**: Treat each candidate scorer as a "judge." Feed pairwise trace comparisons into judge-aware BTL. Weight ensemble by estimated reliability. Partition submissions into prevalence-differing populations (e.g., by contributor cohort or model family) for Hui-Walter. Removes the PR #216 confound at the root.

### 1.2 Conformal Prediction on Scores

**TECP** (arXiv 2509.00461 -- verified). Token-entropy nonconformity + split conformal → prediction sets with finite-sample coverage. Logit-free, works black-box.

**TC use**: Wrap novelty/quality scores in conformal intervals ("95% coverage"). Recalibrate per contributor population. Caveat: split CP assumes exchangeability; pair with covariate-shift-aware CP for drifting populations.

### 1.3 Causal Failure Attribution

**Causal Agent Replay** (arXiv 2606.08275 -- verified, open-source). SCM + do()-resample + Monte-Carlo Shapley. Correlational LLM-judge attribution scores only ~14% step-level accuracy; CAR replaces it.

**CausalFlow** (arXiv 2605.25338 -- verified). Single-agent interventional Causal Responsibility Score + minimal ranked repairs.

**TC use**: Run in offline worker on failed traces; store per-step causal-importance vector; expose "decisive step" as premium annotation.

### 1.4 Safety-Preserving Compression

**TRACE** (arXiv 2606.00611 -- verified, open-source). Compressor-Reader latent evidence state, +12.6pp safety-detection accuracy, robust as context grows.

**TC use**: Store full traces cold; index/query TRACE latent state hot. 10-50x storage savings while retaining safety signal. Caveat: streaming/incremental compression is future work.

### 1.5 Marginal Value Scoring

**CausalMix** (arXiv 2607.01104, Tsinghua -- verified). Mixture-as-treatment, CATE estimation, explicitly survives pool shift. Beats RegMix and generalizes across shifting/unseen pools.

**TC use**: Marginal-value scoring as state-conditioned CATE over corpus features. Feed into VCG credit function. Directly prices redundancy.

### 1.6 Specification Mining → Pattern Vocabulary

**Mining Beyond the Bools** (arXiv 2603.06710 -- verified). Synthesizes temporal + relational invariants from traces.

Related: AgentSpec (ICSE 2026), VeriGuard, Causal Past Logic (arXiv 2605.20923 -- verified), TraceFix (arXiv 2605.07935 -- verified).

**TC use**: Mine temporal-logic invariants from corpus; tag each trace with patterns it satisfies/violates; novelty = "exhibits a pattern not yet in the vocabulary."

### 1.7 Cold-Start Acquisition Playbook

Canonical tactics (Andrew Chen *The Cold Start Problem*, CRV network-effects guide):
- Seed supply side first (high-quality trace contributors)
- Ship single-player utility valuable at zero network size (local debugger/quality linter)
- Manufacture density in one atomic niche (e.g., Claude-Code Rust traces)
- Bounded subsidy/prizes to cross critical mass

---

## 2. Research-Backed Innovations by Ship Readiness

### Tier 1: Integrate Now (< 1 month each)

| Innovation | Source | TC Benefit | Effort |
|---|---|---|---|
| OTel GenAI Ingest | OTel v1.42.0, OpenInference | Standards-based onboarding | 2-3w |
| SKILL.md Publishing | Agent Skills spec, ToxicSkills | Contributor feedback loop, viral distribution | 1-2w |
| Prometheus Metrics | tower-http, metrics-rs | Production observability | 1-2w |
| MinHash Dedup | Rensa crate (608x faster than datasketch) | Analytically sound novelty layer | 1-2w |

### Tier 2: Build Next (1-3 months each)

| Innovation | Source | TC Benefit | Effort |
|---|---|---|---|
| Failure Attribution | AgentDebugX, AgenTracer-8B, TRAIL | New contribution motive, Error Hub | 6-8w |
| Trajectory Replay UI | AgentGUI (38% faster comprehension) | Single-player value, retention | 8-10w |
| NCD Pre-Filter | Li et al. 2004, Jiang et al. 2023 | Structural similarity detection | 2-3w |
| Sub-Trace Decomposition | LEGOMem (AAMAS 2026) | Fine-grained scoring and credit | 6-8w |
| Sleep-Time Batch | Lin et al. (Letta/Berkeley) | 2.5x cost reduction | 4-6w |
| Auto-Tuning Gates | Compound AI Systems (EMNLP 2025) | Self-calibrating thresholds | 6-8w |

### Tier 3: Differentiate Later (3-6 months each)

| Innovation | Source | TC Benefit | Effort |
|---|---|---|---|
| Influence Valuation | LoGra/LogIX (NeurIPS 2025) | Impact-based pricing (unique) | 10-14w |
| Skill Extraction Pipeline | RHO (19% SWE-Bench Pro gain) | Active capability supplier | 12-16w |
| VET Composed Proofs | VET (arXiv 2512.15892) | Verifiable scoring (unique) | 12-16w |
| Evidence Relation Graphs | arXiv 2606.04990 | Reasoning provenance | 8-12w |
| Steering Metadata Capture | arXiv 2411.16627 | Human-intervention corpus (unique) | 6-8w |

---

## 3. Key References for Grant Applications

| # | Citation | Relevance |
|---|---|---|
| 1 | Choe et al. "What is Your Data Worth to GPT?" NeurIPS 2025. arXiv:2405.13954 | Data valuation via influence functions |
| 2 | OTel GenAI semantic conventions v1.42.0 (June 2026) | Interoperability standard |
| 3 | Compound AI Systems Optimization survey. EMNLP 2025 | Pipeline optimization |
| 4 | VET: Verifiable Execution Traces. arXiv:2512.15892 | Cryptographic verification |
| 5 | AgentDebugX. arXiv:2607.18754 | Failure attribution + Error Hub |
| 6 | AgenTracer-8B. ICLR 2026. arXiv:2509.03312 | Trained failure attribution model |
| 7 | EU AI Act, Regulation (EU) 2024/1689, Articles 12 and 50 | Regulatory compliance |
| 8 | For-Value. ACL 2026. arXiv:2508.10180 | Forward-only influence estimation |
| 9 | TRAIL / Who&When. ICML 2025 Spotlight | Failure attribution benchmark |
| 10 | LEGOMem. AAMAS 2026. arXiv:2510.04851 | Modular procedural memory |
| 11 | Judge-Aware Ranking. ICML 2026. arXiv:2601.21817 | Label-free quality scoring |
| 12 | TECP. arXiv:2509.00461 | Conformal prediction for scores |
| 13 | Causal Agent Replay. arXiv:2606.08275 | Causal failure attribution |
| 14 | TRACE. arXiv:2606.00611 | Safety-preserving compression |
| 15 | CausalMix. arXiv:2607.01104 | Marginal value scoring |

---

## 4. Paper Index Summary (~122 Papers, 9 Categories)

Full index in v3/10-research-paper-index.md. Categories:

1. **Novelty Detection & Scoring** (40+ papers): code similarity (AST, GraphCodeBERT, CSSG), compression/NCD, embedding/similarity search (HDC, HNSW, SimHash), perplexity scoring, novelty metrics (NovAScore), data valuation (LoGra, For-Value), process mining, dedup (Rensa, FED)

2. **Skill Extraction & Experience** (10 papers): ReasoningBank, SkillOS, Dynamic Cheatsheet, SkillRevise, RHO, ExpeL, Reflexion, SKILL.md spec, ToxicSkills

3. **Agent Systems & Trace Formats** (20+ papers): agent memory (MemGPT, Mem0, A-MEM, LEGOMem), multi-agent (LatentMAS), frameworks (Meta-Harness, AgentSpec), trace formats (OTel GenAI, OpenInference, Agent Trace Spec), protocols (A2A, MCP/ACP/ANP)

4. **Privacy, Security & Verifiability** (15+ papers): ZK proofs, DP, TEE survey, VET, cryptographic pipelines, EU AI Act, governance-as-a-service, privilege attenuation, DIDs, data donation ethics

5. **Failure Attribution & Debugging** (5 papers): AgentDebugX, AgenTracer-8B, TRAIL, Who&When, evidence tracing survey

6. **Agent UX & Steering** (4 papers): AgentGUI, inference-time steering, interruptible agents, human-agent collaboration survey

7. **Incentive Design & Data Markets** (10+ papers): Ostrom (commons), VCG, Glicko-2, MeritRank, mechanism design for LLMs, prospect theory, Shapley values

8. **User Acquisition & Developer Tools** (8 papers): cognitive load theory, marketplace cold-start tactics, PostHog/Sentry/Langfuse growth patterns, TraceLab dataset

9. **Infrastructure** (7 papers): circuit breakers, ACID, hybrid logical clocks, HDC hardware, sleep-time compute

---

## 5. Deep Research Queries (For Continued Exploration)

### Highest-Payoff Queries

1. `"conformal prediction" LLM uncertainty "coverage guarantee" 2025 2026` -- calibrated confidence on scores
2. `"trajectory compression" "safety-aware" agent "long-horizon" 2025 2026` -- storage savings
3. `"data mixture" "marginal contribution" training optimization 2025 2026` -- core value attribution algorithm
4. `"active learning" annotation "information gain" "label budget" LLM 2025` -- efficient annotation for PR #173
5. `"specification mining" "temporal logic" "execution traces" patterns 2025` -- decomposable pattern vocabulary

### Under-Explored Directions

- Federated scorer updates (traces never leave contributor infrastructure)
- GNNs for tool-call graphs (branching, loops, parallel calls)
- Reward modeling from pairwise human preferences (Bradley-Terry)
- Online anomaly detection for streaming intake (CALIBURN-style)
- RAG from trace corpora (structured + multi-modal retrieval)
- Tighter DP composition (Rényi/Gaussian DP)

### Venue Watchlist

**Tier 1** (check every proceedings): NeurIPS, ICML, ICLR, ICSE, FSE
**Tier 2** (relevant tracks): ACL/EMNLP, SIGMOD, VLDB, ASE, KDD
**Tier 3** (specific topics): AAAI, CCS/S&P/USENIX Security, COLM, SOSP/OSDI, CSCW

---

## 6. Verification Ledger

| Item | Status |
|---|---|
| 2601.21817 Judge-Aware Ranking (ICML 2026) | Verified |
| 2509.00461 TECP | Verified |
| 2401.09376 Hui-Walter Bayesian | Verified |
| 2606.08275 Causal Agent Replay | Verified + open-source |
| 2605.25338 CausalFlow | Verified |
| 2606.00611 TRACE | Verified + open-source |
| 2607.01104 CausalMix (mixtures) | Verified (distinct from 2603.03587) |
| 2603.06710 Mining Beyond the Bools | Verified |
| 2605.20923 Causal Past Logic | Verified |
| 2605.07935 TraceFix | Verified |
| 2510.05566 domain-shift CP | NOT verified -- deep-sweep target |
| 2506.08628 Logic Mining | NOT verified -- deep-sweep target |
| 2606.16038 Open-SWE-Traces | NOT verified -- deep-sweep target |

### Caveats

- Several headline numbers (+12.6pp, ~14% baseline, CausalMix vs RegMix) come from single papers, not independently replicated.
- Influence/causal-replay methods need model internals or re-execution ability -- apply to open-weight/self-hosted targets, not black-box-API-only traces.
- Label-free methods (Hui-Walter, judge-aware BTL) rest on assumptions (conditional independence, 2+ populations, exchangeability) that TC must validate on real submissions.
