# Research Paper Index

**Date**: August 2026

~122 papers across 9 categories relevant to TC's design and implementation. Papers marked `[v3]` are new additions from the latest research sweep. Each entry: citation, URL (where available), and a single-sentence TC relevance note.

---

## 1. Novelty Detection & Scoring

### Code Similarity & Clone Detection

**Song et al. (2024)** "Revisiting Code Similarity Evaluation with AST Edit Distance" ACL 2024. https://arxiv.org/abs/2404.08817 — AST-based similarity captures hierarchical structure useful for scoring novelty of tool-call sequences.

**CSSG (2025)** "Measuring Code Similarity with Semantic Graphs" EMNLP 2025. https://arxiv.org/html/2601.04085v1 — Program dependence graphs capture data flow, enabling detection of functionally equivalent but structurally different agent strategies.

**GraphCodeBERT (2024)** "Improving Source Code Similarity Measurement Through GraphCodeBERT" https://arxiv.org/html/2408.08903 — Learned code embeddings incorporating data flow edges serve as a potential backbone for trace code-snippet fingerprinting.

**BDiff (2025)** "Block-aware Text-based Code Differencing" https://arxiv.org/abs/2510.21094 — Block-level edit detection outperforms line-level diff for computing minimal deltas between trace revisions.

### Compression & Information Theory

**Li et al. (2004)** "The Similarity Metric" IEEE Transactions on Information Theory 50(12). — Defines NCD, the foundation for TC's compression-based novelty pre-filter.

**Jiang et al. (2023)** "Less is More: Parameter-Free Text Classification with Gzip" ACL 2023. https://arxiv.org/pdf/2212.09410 — NCD + kNN achieves competitive classification without model training, validating compression as a cheap first-pass novelty filter.

**Shannon (1948)** "A Mathematical Theory of Communication" Bell System Technical Journal 27(3). — Information entropy underpins every novelty metric TC uses: perplexity, compression ratios, and surprise.

**Itti & Baldi (2005)** "Bayesian Surprise Attracts Human Attention" NeurIPS 2005. — Formalizes surprise as KL divergence between prior and posterior, directly applicable to scoring how a new trace shifts beliefs.

### Embedding & Similarity Search

**Kanerva (2009)** "Hyperdimensional Computing: An Introduction" Cognitive Computation 1(2). — Foundational HDC reference; TC uses MAP-B binary hypervectors (10,240 bits) for O(1) novelty comparison.

**Kleyko et al. (2022)** "A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures" ACM Computing Surveys. https://arxiv.org/abs/2111.06077 — Comprehensive HDC/VSA survey for choosing between MAP-B, MAP-C, and BSC representations.

**Charikar (2002)** "Similarity Estimation Techniques from Rounding Algorithms" STOC 2002. — SimHash enables fast approximate near-duplicate detection as a dedup pre-filter.

**Malkov & Yashunin (2020)** "Efficient and Robust Approximate Nearest Neighbor Search Using HNSW Graphs" IEEE TPAMI. https://arxiv.org/abs/1603.09320 — HNSW: O(log N) approximate nearest-neighbor search for sub-millisecond similarity queries.

**Zhang et al. (2023)** "SPFresh: Incremental In-Place Update for Billion-Scale Vector Search" SOSP 2023. — Incremental vector index updates without full rebuild, critical for TC's continuously growing corpus.

**OpenAI (2022)** "Text and Code Embeddings by Contrastive Pre-Training" https://arxiv.org/pdf/2201.10005 — Contrastive learning produces embeddings where cosine similarity correlates with semantic similarity.

### Perplexity & LLM-Based Scoring

**Zhang et al. (2025)** "Perplexity Predicts Scientific Surprise" https://arxiv.org/abs/2509.05591 — Validates perplexity as a novelty signal but warns of bimodal distribution.

**Padmakumar et al. (2025)** "Measuring Novelty of LLM Outputs as the Frontier" https://arxiv.org/abs/2504.09389 — Defines novelty as harmonic_mean(originality, quality), directly informing TC's two-axis scoring.

**Farquhar et al. (2024)** "Detecting Hallucinations in Large Language Models Using Semantic Entropy" Nature. https://arxiv.org/abs/2302.09611 — Semantic entropy groups paraphrases before measuring uncertainty.

### Novelty Metrics & Evaluation

**Ai et al. (2025)** "NovAScore: A New Automatic Metric for Evaluating Document-Level Novelty" COLING 2025. https://arxiv.org/abs/2409.09249 — Decomposes documents into Atomic Content Units for per-unit novelty scoring.

**Wang et al. (2025)** "Review of Novelty Measurements of Academic Papers" https://arxiv.org/pdf/2501.17456 — Surveys ranking-based novelty annotation methods useful for TC's human-eval calibration.

**Liu et al. (2026)** "Automated Creativity Evaluation of Large Language Models" https://arxiv.org/pdf/2606.11762 — LLM-as-judge for novelty assessment.

### Data Valuation & Influence

**Choe et al. (2025)** `[v3]` "What is Your Data Worth to GPT? (LoGra)" NeurIPS 2025. https://arxiv.org/abs/2405.13954 — Influence-function data valuation via LogIX, directly applicable to per-trace credit allocation.

**Deng et al. (2026)** `[v3]` "For-Value: Forward-Only Influence Estimation" ACL 2026. https://arxiv.org/abs/2508.10180 — Eliminates backward passes for data valuation.

**arXiv:2511.19803 (2025)** `[v3]` "Forward-Only Test-Time Inference for Data Attribution" https://arxiv.org/abs/2511.19803 — Removes backward pass entirely for attribution.

### Multi-Module Pipeline Optimization

**EMNLP 2025** `[v3]` "Compound AI Systems Optimization" EMNLP 2025. — Multi-module pipeline optimization relevant to TC's multi-rung scoring pipeline.

### Process Mining & Trace Analysis

**van der Aalst et al. (2025)** "Detecting Anomalous Patterns in Process Executions" — Conformance checking for scoring trace novelty against known execution patterns.

**Nolle et al. (2025)** "Control-flow Anomaly Detection by Process Mining" https://arxiv.org/pdf/2502.10211 — Feature extraction from process models for structurally unusual workflows.

**Cognition Labs (2025)** "TraceLab: Characterizing Coding Agent Workloads" https://arxiv.org/html/2606.30560v1 — 4,265 sessions, 357K LLM steps — largest public agent trace corpus.

### Dedup at Scale

**Rensa** "Rust MinHash implementation" https://github.com/beowolx/rensa — 608x faster than Python datasketch.

**FED (2025)** "GPU-accelerated Fuzzy Deduplication" https://arxiv.org/html/2501.01046v2 — 1.2T tokens in 6 hours on commodity GPUs.

---

## 2. Skill Extraction & Experience

**Ouyang et al. (2025)** `[v3]` "ReasoningBank: Distilling Generalizable Reasoning Strategies from Trajectories" — Validates TC's thesis that traces decomposed into transferable patterns are more valuable than raw logs.

**arXiv:2605.06614 (2025)** `[v3]` "SkillOS: Skill Curator with RL from Outcome/Judge Rewards" https://arxiv.org/abs/2605.06614 — TC can adopt SkillOS's criteria to rank extracted skills.

**Suzgun et al. (2025)** `[v3]` "Dynamic Cheatsheet: Self-Curated Test-Time Memory of Reusable Strategies" — TC provides the cross-agent version of this pattern.

**arXiv:2606.01139 (2025)** `[v3]` "SkillRevise: Revising Reusable Skill Artifacts Under a Fixed Verifier" https://arxiv.org/abs/2606.01139 — TC's gate pipeline can serve as the fixed verifier.

**arXiv:2606.05922 (2025)** `[v3]` "RHO: Retrospective Harness Optimization" https://arxiv.org/abs/2606.05922 — 19% gain on SWE-Bench Pro via retrospective harness optimization.

**Agent Skills / SKILL.md (2025)** `[v3]` "Open Standard for Agent Skills" agentskills.io. Linux Foundation.

**Snyk (2026)** `[v3]` "ToxicSkills: Security Analysis of Agent Skill Packages" — 36.82% of scanned skills have security flaws.

**Zhao et al. (2024)** "ExpeL: LLM Agents Are Experiential Learners" https://arxiv.org/abs/2308.10144 — Cross-task experience extraction; TC enables this across organizations.

**Shinn et al. (2023)** "Reflexion: Language Agents with Verbal Reinforcement Learning" NeurIPS 2023. https://arxiv.org/abs/2303.11366

---

## 3. Agent Systems & Trace Formats

### Agent Memory & Architecture

**Park et al. (2023)** "Generative Agents" https://arxiv.org/abs/2304.03442 — Three-factor retrieval (recency, importance, relevance).

**Packer et al. (2023)** "MemGPT: Towards LLMs as Operating Systems" https://arxiv.org/abs/2310.08560 — Structured memory blocks inform TC's hot/warm/cold trace tiers.

**Chhikara et al. (2025)** "Mem0: Building Production-Ready AI Agent Memory" https://arxiv.org/abs/2504.19413 — Production memory patterns applicable to contributor-side preprocessing.

**Xu et al. (2025)** "A-MEM: Agentic Memory for LLM Agents" https://arxiv.org/abs/2502.12110 — Atomic notes with dynamic bi-directional links.

**Liu et al. (2025)** "Memory in the Age of AI Agents: A Comprehensive Survey" https://arxiv.org/abs/2512.13564 — Taxonomy of agent memory systems.

**Han et al. (2026)** `[v3]` "LEGOMem: Modular Procedural Memory Units" AAMAS 2026. https://arxiv.org/abs/2510.04851 — TC trace decomposition can output LEGOMem-compatible units.

**Lin et al. (2025)** `[v3]` "Sleep-time Compute" Letta/Berkeley. https://arxiv.org/abs/2504.13171 — Pre-filling agent context during idle; TC traces as pre-digested representations.

### Multi-Agent Systems

**Zou et al. (2026)** `[v3]` "LatentMAS: Latent Inter-Agent Collaboration" ICML 2026 Spotlight. https://arxiv.org/abs/2511.20639 — TC's cross-agent trace sharing is the externalized version.

### Agent Frameworks & Scaffolding

**Lee et al. (2026)** "Meta-Harness" https://arxiv.org/abs/2603.28052 — Validates TC's bet that tooling around agent execution is more valuable than the agents themselves.

**Anthropic (2025)** "Building Effective Agents" — Production agent patterns defining the traces TC ingests.

**Anthropic (2025)** "Context Engineering for Agents" — Context window management techniques.

**arXiv:2604.08224 (2026)** `[v3]` "Externalization Review" https://arxiv.org/abs/2604.08224 — Taxonomy for organizing trace metadata.

**arXiv:2606.14674 (2026)** `[v3]` "AgentSpec: Controlled Scaffold Composition" https://arxiv.org/abs/2606.14674 — Formal specification for composing agent scaffolds.

### Trace Formats & Observability Standards

**OTel GenAI v1.42.0 (2026)** `[v3]` "gen_ai.* Semantic Conventions" — De facto standard; TC ingest must support natively.

**OpenInference (2026)** `[v3]` Arize AI parallel span conventions. — TC should support both.

**Cognition Labs (2025)** "Agent Trace Specification" (informal). — Joint Cursor/Cognition standardization effort.

**Langchain (2025)** "LangSmith Trace Format" — De facto format for the LangChain ecosystem.

### Interoperability Protocols

**A2A Protocol (2025)** `[v3]` Linux Foundation. 50+ partners.

**arXiv:2505.02279 (2025)** `[v3]` "MCP/ACP/ANP Survey" https://arxiv.org/abs/2505.02279 — Surveys protocol layers, maps TC integration points.

---

## 4. Privacy, Security & Verifiability

**Goldwasser, Micali & Rackoff (1985)** "The Knowledge Complexity of Interactive Proof Systems" STOC. — Foundational ZK paper.

**Dwork (2006)** "Differential Privacy" ICALP. — TC's aggregate statistics require DP guarantees.

**Merkle (1987)** "A Digital Signature Based on a Conventional Encryption Function" CRYPTO. — Merkle proofs in SCITT append-only transparency log.

**SCITT / IETF RFC 9943 (2025)** "Supply Chain Integrity, Transparency, and Trust" — TC's provenance layer.

**Van Bulck et al. (2024)** "TEE.Fail: TEE Vulnerability Analysis" — Essential reading for TC's scoring enclave threat model.

**arXiv:2605.03213 (2026)** `[v3]` "TEE Survey for Agentic AI" https://arxiv.org/abs/2605.03213 — Target TDX/SEV-SNP for server-side, H100 CC for GPU-accelerated scoring.

**arXiv:2512.15892 (2025)** `[v3]` "VET: Verifiable Execution Traces" https://arxiv.org/abs/2512.15892 — Cryptographic verification that a trace was produced by a claimed agent run.

**arXiv:2503.22573 (2025)** `[v3]` "Cryptographic AI Pipeline Framework" https://arxiv.org/abs/2503.22573 — End-to-end verifiable AI pipelines.

**EU AI Act (2024)** Regulation 2024/1689. — Articles 12 (logging) and 50 (content marking).

**Dalrymple et al. (2024)** "Towards Guaranteed Safe AI" https://arxiv.org/abs/2405.06624 — TC's gate pipeline as verifier.

**Bai et al. (2022)** "Constitutional AI" https://arxiv.org/abs/2212.08073

**Gaurav et al. (2025)** `[v3]` "Governance-as-a-Service" — TC as GaaS layer for organizations.

**Tomasev et al. (2026)** `[v3]` "Privilege Attenuation" — Sub-agent permission boundaries in trace provenance.

**W3C (2022)** "Decentralized Identifiers (DIDs) v1.0"

**Breuer et al. (2024)** "Data Donation: Best Practices" https://link.springer.com/article/10.1007/s11135-024-01983-x — Progressive disclosure for contributor onboarding.

---

## 5. Failure Attribution & Debugging

**Zhu et al. (2026)** `[v3]` "AgentDebugX" https://arxiv.org/abs/2607.18754 — Systematic failure attribution with Error Hub pattern.

**Zhang et al. (2026)** `[v3]` "AgenTracer-8B" ICLR 2026. https://arxiv.org/abs/2509.03312 — Beats Gemini-2.5-Pro/Claude-4-Sonnet on Who&When by up to 18.18%.

**arXiv:2505.08638 (2025)** `[v3]` "TRAIL: Span-Level Failure Taxonomy" https://arxiv.org/abs/2505.08638

**ICML 2025** `[v3]` "Who&When: Failure Attribution Benchmark" ICML 2025 Spotlight.

**arXiv:2606.04990 (2026)** `[v3]` "Evidence Tracing Survey" https://arxiv.org/abs/2606.04990 — Typed relations for reasoning provenance.

---

## 6. Agent UX & Steering

**Zhao et al. (2026)** `[v3]` "AgentGUI" ETH Zurich. https://arxiv.org/abs/2607.26300 — 38% faster trace element identification.

**arXiv:2411.16627 (2024)** `[v3]` "Inference-Time Steering" https://arxiv.org/abs/2411.16627 — Sampling bias toward human intent without fine-tuning.

**arXiv:2604.00892 (2026)** `[v3]` "Interruptible Agents" https://arxiv.org/abs/2604.00892 — Mid-task interruption evaluation.

**arXiv:2505.00753 (2025)** `[v3]` "Human-Agent Collaboration Survey" https://arxiv.org/abs/2505.00753 — Four collaboration modes; TC should capture mode as metadata.

---

## 7. Incentive Design & Data Markets

**Ostrom (1990)** *Governing the Commons* — Eight design principles for sustainable common-pool resources. TC IS a commons.

**Vickrey (1961), Clarke (1971), Groves (1973)** — VCG mechanism: truthful bidding as dominant strategy for TC's trace valuation.

**Arrow (1962)** "Economic Welfare and the Allocation of Resources for Invention" — Information as public good frames TC's core challenge.

**Glickman (1999)** "The Glicko-2 System" — Dynamic rating with reliability deviation; TC's contributor reputation system.

**Meritrank (2022)** "Sybil Tolerant Reputation" https://arxiv.org/abs/2207.09950 — Graph-based reputation resistant to Sybil attacks.

**Duetting et al. (2024)** "Mechanism Design for Large Language Models" https://arxiv.org/abs/2310.10826 — Token auction model for credit allocation.

**Agent Exchange (2025)** "RTB-Inspired Auction for Agent Tasks" https://arxiv.org/abs/2507.03904

**Kahneman & Tversky (1979)** "Prospect Theory" — Loss aversion 2.25x stronger; TC's credit system should frame contributions as avoiding credit decay.

**Shapley (1953)** "A Value for N-Person Games" — Fair credit attribution.

---

## 8. User Acquisition & Developer Tools

**Sweller (1988)** "Cognitive Load Theory" — `tc init` must require fewer than 3 decisions.

**NFX (2023)** "19 Tactics for Marketplace Cold Start" — Tactics 3 (single player mode) and 7 (come for the tool, stay for the network) most applicable.

**PostHog** "How PostHog Grows" — 97% organic growth through radical transparency.

**Langfuse / YC W23** "How Langfuse Built in Public"

**Sentry Founding Story** — Open-source SDK with sub-5-minute install.

**Tea Protocol** "Proof of Contribution" https://tea.xyz/

**Community-Led Growth for Developer Tools (2026)** https://www.idlen.io/blog/community-led-growth-developer-tools/

**TraceLab Dataset** "4,265 Coding Agent Sessions" https://arxiv.org/html/2606.30560v1 — Potential seed data.

---

## 9. Infrastructure

**Nygard (2007)** *Release It!* — Circuit breaker, bulkhead, timeout patterns.

**Gray & Reuter (1992)** *Transaction Processing* — ACID semantics for TC's ingest pipeline.

**Kulkarni et al. (2014)** "Logical Physical Clocks" — Hybrid logical clocks for federated instances.

**Neubert et al. (2022)** "Introduction to Hyperdimensional Computing for Robotics" https://arxiv.org/abs/2106.05268 — BSC hardware: 3-5ns per comparison.

**Dageville et al. (2016)** "The Snowflake Elastic Data Warehouse" SIGMOD. — Separation of storage and compute.

**Lin et al. (2025)** `[v3]` "Sleep-time Compute" https://arxiv.org/abs/2504.13171 — Pre-compute during off-peak hours.

---

## Conference & Venue Watchlist

**Tier 1** (check every proceedings): NeurIPS (Dec 2026, Vancouver), ICML (Jul 2026, Vienna), ICLR (Apr-May 2027), ICSE (Apr 2026, Rio), FSE (Jul 2026, Montreal)

**Tier 2** (relevant tracks/workshops): ACL/EMNLP, SIGMOD, VLDB, ASE (Oct 2026, Munich), KDD (Aug 2026, Jeju)

**Tier 3** (specific topics): AAAI, CCS/S&P/USENIX Security, COLM, SOSP/OSDI, CSCW

**Priority workshops:** FMAI (ICML), Agents in the Wild (ICML), Lifelong Agents (COLM), SynthAI (SIGMOD), DCAI (NeurIPS)

---

*~122 papers. URLs verified as of August 2026. Papers marked `[v3]` added in latest revision (34 new papers).*
