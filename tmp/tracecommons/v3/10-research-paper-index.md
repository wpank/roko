# Research Paper Index

TraceCommons (TC) is an open-source, privacy-preserving AI trace registry. Agents submit execution traces; TEE-attested rungs score quality and novelty; NEAR settles credit to contributors. This document indexes ~122 papers across 9 categories relevant to TC's design and implementation. Papers marked `[v3]` are new additions from the latest research sweep. Each entry follows a consistent format: citation, URL (where available), and a single-sentence TC relevance note.

---

## 1. Novelty Detection & Scoring

### Code Similarity & Clone Detection

**Song et al. (2024)** "Revisiting Code Similarity Evaluation with AST Edit Distance" ACL 2024. https://arxiv.org/abs/2404.08817
AST-based similarity captures hierarchical structure useful for scoring novelty of tool-call sequences.

**CSSG (2025)** "Measuring Code Similarity with Semantic Graphs" EMNLP 2025. https://arxiv.org/html/2601.04085v1
Program dependence graphs capture data flow, enabling detection of functionally equivalent but structurally different agent strategies.

**GraphCodeBERT (2024)** "Improving Source Code Similarity Measurement Through GraphCodeBERT" https://arxiv.org/html/2408.08903
Learned code embeddings incorporating data flow edges serve as a potential backbone for trace code-snippet fingerprinting.

**BDiff (2025)** "Block-aware Text-based Code Differencing" https://arxiv.org/abs/2510.21094
Block-level edit detection outperforms line-level diff for computing minimal deltas between trace revisions.

### Compression & Information Theory

**Li et al. (2004)** "The Similarity Metric" IEEE Transactions on Information Theory 50(12).
Defines Normalized Compression Distance (NCD), the foundation for TC's compression-based novelty pre-filter.

**Jiang et al. (2023)** "Less is More: Parameter-Free Text Classification with Gzip" ACL 2023. https://arxiv.org/pdf/2212.09410
NCD + kNN achieves competitive classification without model training, validating compression as a cheap first-pass novelty filter.

**Shannon (1948)** "A Mathematical Theory of Communication" Bell System Technical Journal 27(3).
Information entropy underpins every novelty metric TC uses: perplexity, compression ratios, and surprise.

**Itti & Baldi (2005)** "Bayesian Surprise Attracts Human Attention" NeurIPS 2005.
Formalizes surprise as KL divergence between prior and posterior, directly applicable to scoring how a new trace shifts beliefs.

### Embedding & Similarity Search

**Kanerva (2009)** "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation" Cognitive Computation 1(2).
Foundational HDC reference; TC uses MAP-B binary hypervectors (10,240 bits) with role-filler binding for O(1) novelty comparison.

**Kleyko et al. (2022)** "A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures" ACM Computing Surveys. https://arxiv.org/abs/2111.06077
Comprehensive HDC/VSA survey useful for choosing between MAP-B, MAP-C, and BSC representations for fingerprinting.

**Charikar (2002)** "Similarity Estimation Techniques from Rounding Algorithms" STOC 2002.
SimHash enables fast approximate near-duplicate detection as a dedup pre-filter before expensive novelty scoring.

**Malkov & Yashunin (2020)** "Efficient and Robust Approximate Nearest Neighbor Search Using HNSW Graphs" IEEE TPAMI. https://arxiv.org/abs/1603.09320
HNSW achieves O(log N) approximate nearest-neighbor search, the index structure TC needs for sub-millisecond similarity queries.

**Zhang et al. (2023)** "SPFresh: Incremental In-Place Update for Billion-Scale Vector Search" SOSP 2023.
Incremental vector index updates without full rebuild, critical for TC's continuously growing corpus.

**OpenAI (2022)** "Text and Code Embeddings by Contrastive Pre-Training" https://arxiv.org/pdf/2201.10005
Contrastive learning produces embeddings where cosine similarity correlates with semantic similarity, a baseline for trace content vectors.

### Perplexity & LLM-Based Scoring

**Zhang et al. (2025)** "Perplexity Predicts Scientific Surprise" https://arxiv.org/abs/2509.05591
Validates perplexity as a novelty signal but warns of bimodal distribution -- TC must handle the high-perplexity boilerplate case.

**Padmakumar et al. (2025)** "Measuring Novelty of LLM Outputs as the Frontier" https://arxiv.org/abs/2504.09389
Defines novelty as harmonic_mean(originality, quality), directly informing TC's two-axis scoring design.

**Farquhar et al. (2024)** "Detecting Hallucinations in Large Language Models Using Semantic Entropy" Nature. https://arxiv.org/abs/2302.09611
Semantic entropy groups paraphrases before measuring uncertainty, applicable to distinguishing genuine trace diversity from noisy reformulations.

### Novelty Metrics & Evaluation

**Ai et al. (2025)** "NovAScore: A New Automatic Metric for Evaluating Document-Level Novelty" COLING 2025. https://arxiv.org/abs/2409.09249
Decomposes documents into Atomic Content Units for per-unit novelty scoring, directly applicable to trace-level decomposition.

**Wang et al. (2025)** "Review of Novelty Measurements of Academic Papers" https://arxiv.org/pdf/2501.17456
Surveys ranking-based novelty annotation methods useful for designing TC's human-eval calibration protocol.

**Liu et al. (2026)** "Automated Creativity Evaluation of Large Language Models" https://arxiv.org/pdf/2606.11762
LLM-as-judge for novelty assessment, relevant to TC's consideration of LLMs as an upper-rung oracle.

### Data Valuation & Influence

**Choe et al. (2025)** `[v3]` "What is Your Data Worth to GPT? (LoGra)" NeurIPS 2025. https://arxiv.org/abs/2405.13954
Influence-function data valuation via LogIX, directly applicable to TC's per-trace credit allocation.

**Deng et al. (2026)** `[v3]` "For-Value: Forward-Only Influence Estimation" ACL 2026. https://arxiv.org/abs/2508.10180
Eliminates backward passes for data valuation, making influence estimation feasible at TC's per-trace scale.

**arXiv:2511.19803 (2025)** `[v3]` "Forward-Only Test-Time Inference for Data Attribution" https://arxiv.org/abs/2511.19803
Removes backward pass entirely for attribution, complementing For-Value to define a forward-only stack TC can run at ingest time.

### Multi-Module Pipeline Optimization

**EMNLP 2025** `[v3]` "Compound AI Systems Optimization" EMNLP 2025.
Multi-module pipeline optimization relevant to TC's multi-rung scoring pipeline where individual rungs interact.

### Process Mining & Trace Analysis

**van der Aalst et al. (2025)** "Detecting Anomalous Patterns in Process Executions"
Conformance checking techniques for scoring agent trace novelty against known execution patterns.

**Nolle et al. (2025)** "Control-flow Anomaly Detection by Process Mining" https://arxiv.org/pdf/2502.10211
Feature extraction from process models for identifying structurally unusual agent workflows.

**Cognition Labs (2025)** "TraceLab: Characterizing Coding Agent Workloads" https://arxiv.org/html/2606.30560v1
4,265 coding agent sessions with 357K LLM steps -- the largest public agent trace corpus and potential seed data for TC.

### Dedup at Scale

**Rensa** "Rust MinHash implementation" https://github.com/beowolx/rensa
608x faster than Python datasketch; production-grade MinHash for TC's Bloom filter dedup rung.

**FED (2025)** "GPU-accelerated Fuzzy Deduplication" https://arxiv.org/html/2501.01046v2
Processes 1.2T tokens in 6 hours on commodity GPUs, establishing the throughput ceiling TC should target for batch dedup.

---

## 2. Skill Extraction & Experience

**Ouyang et al. (2025)** `[v3]` "ReasoningBank: Distilling Generalizable Reasoning Strategies from Trajectories"
Distills reusable reasoning strategies from trajectories, validating TC's thesis that traces decomposed into transferable patterns are more valuable than raw logs.

**arXiv:2605.06614 (2025)** `[v3]` "SkillOS: Skill Curator with RL from Outcome/Judge Rewards" https://arxiv.org/abs/2605.06614
Curates skills using RL with outcome and judge rewards; TC can adopt SkillOS's criteria to rank extracted skills by utility.

**Suzgun et al. (2025)** `[v3]` "Dynamic Cheatsheet: Self-Curated Test-Time Memory of Reusable Strategies"
Validates agents building and consulting strategy libraries -- TC provides the cross-agent version of this pattern.

**arXiv:2606.01139 (2025)** `[v3]` "SkillRevise: Revising Reusable Skill Artifacts Under a Fixed Verifier" https://arxiv.org/abs/2606.01139
TC's gate pipeline can serve as the fixed verifier for iterative community-driven skill refinement.

**arXiv:2606.05922 (2025)** `[v3]` "RHO: Retrospective Harness Optimization" https://arxiv.org/abs/2606.05922
19% gain on SWE-Bench Pro via retrospective harness optimization, demonstrating that post-hoc trace analysis yields large practical gains.

**Agent Skills / SKILL.md (2025)** `[v3]` "Open Standard for Agent Skills" agentskills.io. Linux Foundation.
TC should align its skill metadata schema with SKILL.md for interoperability with the broader agent ecosystem.

**Snyk (2026)** `[v3]` "ToxicSkills: Security Analysis of Agent Skill Packages" Snyk.
36.82% of scanned skills have security flaws -- extracted skills must pass security screening before commons inclusion.

**Zhao et al. (2024)** "ExpeL: LLM Agents Are Experiential Learners" https://arxiv.org/abs/2308.10144
Cross-task experience extraction within one agent; TC enables this pattern across organizations.

**Shinn et al. (2023)** "Reflexion: Language Agents with Verbal Reinforcement Learning" NeurIPS 2023. https://arxiv.org/abs/2303.11366
TC's trace scoring can identify which self-reflection patterns correlate with improved downstream performance.

---

## 3. Agent Systems & Trace Formats

### Agent Memory & Architecture

**Park et al. (2023)** "Generative Agents: Interactive Simulacra of Human Behavior" https://arxiv.org/abs/2304.03442
Three-factor retrieval (recency, importance, relevance) informs TC's weighted model for surfacing historical traces.

**Packer et al. (2023)** "MemGPT: Towards LLMs as Operating Systems" https://arxiv.org/abs/2310.08560
Structured memory blocks with explicit management inform TC's hot/warm/cold trace storage tiers.

**Chhikara et al. (2025)** "Mem0: Building Production-Ready AI Agent Memory" https://arxiv.org/abs/2504.19413
Production memory patterns (extraction, dedup, conflict resolution) directly applicable to TC's contributor-side trace preprocessing.

**Xu et al. (2025)** "A-MEM: Agentic Memory for LLM Agents" https://arxiv.org/abs/2502.12110
Atomic notes with dynamic bi-directional links inform TC's knowledge graph approach to trace relationships.

**Liu et al. (2025)** "Memory in the Age of AI Agents: A Comprehensive Survey" https://arxiv.org/abs/2512.13564
Taxonomy of agent memory systems (episodic, semantic, procedural) useful for classifying what type of memory each trace represents.

**Han et al. (2026)** `[v3]` "LEGOMem: Modular Procedural Memory Units" AAMAS 2026. https://arxiv.org/abs/2510.04851
TC's trace decomposition can output LEGOMem-compatible units for plug-and-play memory sharing across architectures.

**Lin et al. (2025)** `[v3]` "Sleep-time Compute: Pre-filling Agent Context During Idle" Letta/Berkeley. https://arxiv.org/abs/2504.13171
TC traces retrieved during sleep-time can be pre-digested into compact representations, reducing retrieval latency.

### Multi-Agent Systems

**Zou et al. (2026)** `[v3]` "LatentMAS: Latent Inter-Agent Collaboration" ICML 2026 Spotlight. https://arxiv.org/abs/2511.20639
Agents collaborate via shared latent space; TC's cross-agent trace sharing is the externalized version of this pattern.

### Agent Frameworks & Scaffolding

**Lee et al. (2026)** "Meta-Harness: Towards Benchmarking LLM-based Coding Agents" https://arxiv.org/abs/2603.28052
Validates TC's bet that tooling around agent execution (tracing, scoring, sharing) is more valuable than the agents themselves.

**Anthropic (2025)** "Building Effective Agents" https://www.anthropic.com/engineering/building-effective-agents
Production agent patterns that define the execution traces TC ingests.

**Anthropic (2025)** "Context Engineering for Agents"
Context window management techniques informing TC's understanding of what information agents use from retrieved traces.

**arXiv:2604.08224 (2026)** `[v3]` "Externalization Review: Memory, Skills, Protocols, Harness Engineering Taxonomy" https://arxiv.org/abs/2604.08224
Comprehensive taxonomy providing the classification framework TC needs to organize its trace metadata schema.

**arXiv:2606.14674 (2026)** `[v3]` "AgentSpec: Controlled Scaffold Composition" https://arxiv.org/abs/2606.14674
Formal specification for composing agent scaffolds; TC's skill composition pipeline can adopt its type-safe model.

### Trace Formats & Observability Standards

**OpenTelemetry (2024-2025)** "GenAI Semantic Conventions" https://opentelemetry.io/docs/specs/semconv/gen-ai/
Emerging standard for AI/LLM observability spans; TC's trace schema should align with OTel GenAI conventions.

**OTel GenAI v1.42.0 (2026)** `[v3]` "gen_ai.* Semantic Conventions" OpenTelemetry.
Stabilized `gen_ai.*` conventions are now the de facto standard; TC's trace ingest must support them natively.

**OpenInference (2026)** `[v3]` "Parallel Span Conventions for AI Systems" Arize AI.
TC should support both OTel GenAI and OpenInference conventions since the ecosystem is split.

**Cognition Labs (2025)** "Agent Trace Specification" (informal).
Joint Cursor/Cognition effort to standardize agent trace formats; TC should track and align with this emerging standard.

**Langchain (2025)** "LangSmith Trace Format"
De facto trace format for the LangChain ecosystem; TC's ingest pipeline needs a LangSmith adapter given its market share.

### Interoperability Protocols

**A2A Protocol (2025)** `[v3]` "Agent-to-Agent Delegation Protocol" Linux Foundation.
TC traces spanning A2A delegations need first-class support for cross-agent provenance chains.

**arXiv:2505.02279 (2025)** `[v3]` "MCP/ACP/ANP Survey: Interoperability Protocol Stack" https://arxiv.org/abs/2505.02279
Surveys MCP, ACP, and ANP protocol layers, mapping where each TC component belongs and where integration points exist.

---

## 4. Privacy, Security & Verifiability

### Foundational Cryptography & Privacy

**Goldwasser, Micali & Rackoff (1985)** "The Knowledge Complexity of Interactive Proof Systems" STOC 1985.
Foundational ZK paper; TC uses ZK proofs so contributors can prove trace properties without revealing content.

**Dwork (2006)** "Differential Privacy" ICALP 2006.
TC's aggregate statistics (corpus novelty distributions, contributor rankings) require DP guarantees.

**Merkle (1987)** "A Digital Signature Based on a Conventional Encryption Function" CRYPTO 1987.
TC uses Merkle proofs in its SCITT append-only transparency log for trace provenance.

**SCITT / IETF RFC 9943 (2025)** "Supply Chain Integrity, Transparency, and Trust"
Append-only transparency ledger with Merkle inclusion proofs implementing TC's provenance layer.

### TEE & Hardware Security

**Van Bulck et al. (2024)** "TEE.Fail: TEE Vulnerability Analysis"
Systematic survey of TEE attacks (SGX, TrustZone, SEV) -- essential reading for TC's scoring enclave threat model.

**arXiv:2605.03213 (2026)** `[v3]` "TEE Survey for Agentic AI: SGX, TDX, SEV-SNP, CCA, H100 CC" https://arxiv.org/abs/2605.03213
Surveys TEE options for agentic AI; TC should target TDX/SEV-SNP for server-side and H100 CC for GPU-accelerated scoring.

### Verifiable Execution & Provenance

**arXiv:2512.15892 (2025)** `[v3]` "VET: Verifiable Execution Traces" https://arxiv.org/abs/2512.15892
Cryptographic verification that a trace was produced by a claimed agent run, essential for marketplace integrity.

**arXiv:2503.22573 (2025)** `[v3]` "Cryptographic AI Pipeline Framework: End-to-End Verifiability" https://arxiv.org/abs/2503.22573
End-to-end cryptographic verification for AI pipelines; TC can make its entire ingest-score-publish pipeline verifiable.

### Safety & Governance

**EU AI Act (2024)** Regulation 2024/1689.
Article 12 mandates logging for high-risk AI; Article 50 requires content marking -- TC is compliance infrastructure for both.

**Dalrymple et al. (2024)** "Towards Guaranteed Safe AI" https://arxiv.org/abs/2405.06624
Framework (world model + safety spec + verifier) where TC's gate pipeline implements the verifier role.

**Bai et al. (2022)** "Constitutional AI: Harmlessness from AI Feedback" https://arxiv.org/abs/2212.08073
Structural constraints via self-critique, relevant to TC embedding safety rules in the scoring pipeline rather than training.

**Gaurav et al. (2025)** `[v3]` "Governance-as-a-Service: Model-Agnostic Governance Layers"
TC can position its scoring and policy engine as a GaaS layer organizations plug into existing agent deployments.

**Tomasev et al. (2026)** `[v3]` "Privilege Attenuation: Sub-Agent Permission Narrowing"
TC's multi-agent trace provenance must track permission boundaries since attenuated-privilege traces have different trust properties.

### Identity & Consent

**W3C (2022)** "Decentralized Identifiers (DIDs) v1.0" W3C Recommendation.
TC's identity layer supports DIDs for cross-instance contributor verification without a central authority.

**Breuer et al. (2024)** "Data Donation: Best Practices and Ethical Considerations" https://link.springer.com/article/10.1007/s11135-024-01983-x
Progressive disclosure informs TC's contributor onboarding: start with metadata only, escalate to full traces with consent.

---

## 5. Failure Attribution & Debugging

**Zhu et al. (2026)** `[v3]` "AgentDebugX: Failure Attribution and Error Hub" https://arxiv.org/abs/2607.18754
Systematic failure attribution with an Error Hub pattern; traces demonstrating novel failure modes are high-value contributions.

**Zhang et al. (2026)** `[v3]` "AgenTracer-8B: Failure Attribution Model" ICLR 2026. https://arxiv.org/abs/2509.03312
Purpose-built 8B model for attributing agent failures; TC can use it as an automated scoring rung for failure-mode metadata.

**arXiv:2505.08638 (2025)** `[v3]` "TRAIL: Span-Level Failure Taxonomy" https://arxiv.org/abs/2505.08638
TC's trace schema should adopt TRAIL's span-level failure taxonomy for consistent cross-organization failure labeling.

**ICML 2025** `[v3]` "Who&When: Failure Attribution Benchmark" ICML 2025 Spotlight.
Benchmark for attributing failures to specific agents and timesteps; TC can use it to calibrate its failure attribution rung.

**arXiv:2606.04990 (2026)** `[v3]` "Evidence Tracing Survey: Provenance Schema with Typed Relations" https://arxiv.org/abs/2606.04990
Typed relations (supports, contradicts, refines) capture how traces relate beyond simple similarity.

---

## 6. Agent UX & Steering

**Zhao et al. (2026)** `[v3]` "AgentGUI: Trajectory Replay and Steering" ETH Zurich. https://arxiv.org/abs/2607.26300
Trace replay with branch-point steering is a compelling UX pattern for TC's trace browser.

**arXiv:2411.16627 (2024)** `[v3]` "Inference-Time Steering: Sampling Bias Toward Human Intent" https://arxiv.org/abs/2411.16627
When injecting retrieved traces into agent context, steering ensures the agent follows the retrieved strategy.

**arXiv:2604.00892 (2026)** `[v3]` "Interruptible Agents: Mid-Task Interruption Evaluation" https://arxiv.org/abs/2604.00892
Provides evaluation criteria for scoring partial traces and graceful degradation patterns from interrupted runs.

**arXiv:2505.00753 (2025)** `[v3]` "Human-Agent Collaboration Survey: Supervision and Steering Frameworks" https://arxiv.org/abs/2505.00753
TC's UX should support all four collaboration modes (supervision, steering, delegation, joint), capturing the type as trace metadata.

---

## 7. Incentive Design & Data Markets

**Ostrom (1990)** *Governing the Commons: The Evolution of Institutions for Collective Action* Cambridge University Press.
Eight design principles for sustainable common-pool resources -- TC is literally a commons, and this is the playbook.

**Vickrey (1961)** "Counterspeculation, Auctions, and Competitive Sealed Tenders" Journal of Finance 16(1).
**Clarke (1971)** "Multipart Pricing of Public Goods" Public Choice 11(1).
**Groves (1973)** "Incentives in Teams" Econometrica 41(4).
VCG mechanism: truthful bidding as dominant strategy, the foundation for TC's trace valuation auctions.

**Arrow (1962)** "Economic Welfare and the Allocation of Resources for Invention" NBER.
Information as a public good frames TC's core challenge: incentivizing contribution when sharing reduces scarcity.

**Glickman (1999)** "The Glicko-2 System" http://www.glicko.net/glicko/glicko2.pdf
Dynamic rating with reliability deviation and volatility; TC uses Glicko-2 for contributor reputation.

**Meritrank (2022)** "MeritRank: Sybil Tolerant Reputation for Merit-Based Token Economies" https://arxiv.org/abs/2207.09950
Graph-based reputation resistant to Sybil attacks, critical for TC's pseudonymous contributor model.

**Duetting et al. (2024)** "Mechanism Design for Large Language Models" https://arxiv.org/abs/2310.10826
Token auction model applicable to TC's credit allocation when multiple contributors provide overlapping traces.

**Agent Exchange (2025)** "RTB-Inspired Auction for Agent Tasks" https://arxiv.org/abs/2507.03904
Real-time bidding applied to agent task allocation, relevant to TC's marketplace for trace bundles.

**Kahneman & Tversky (1979)** "Prospect Theory: An Analysis of Decision Under Risk" Econometrica 47(2).
Loss aversion is 2.25x stronger than gains; TC's credit system should frame contributions as avoiding credit decay.

**Shapley (1953)** "A Value for N-Person Games" Contributions to the Theory of Games II.
Shapley values for fair credit attribution when multiple traces contribute to a composite insight.

---

## 8. User Acquisition & Developer Tools

**Sweller (1988)** "Cognitive Load Theory" Cognitive Science 12(2).
TC's `tc init` must require fewer than 3 decisions to produce a working configuration.

**NFX (2023)** "19 Tactics for Marketplace Cold Start" https://www.nfx.com/post/19-marketplace-tactics-for-overcoming-the-chicken-or-egg-problem
Tactics 3 (single player mode) and 7 (come for the tool, stay for the network) are most applicable to TC's bootstrap.

**PostHog** "How PostHog Grows: The Power of Being Contrarian" https://www.howtheygrow.co/p/how-posthog-grows-the-power-of-being
97% organic growth through radical transparency; TC should emulate open-source core and developer-first UX.

**Langfuse / YC W23** "How Langfuse Built in Public" https://posthog.com/spotlight/startup-langfuse
TC can seed the commons with traces from roko's own self-hosting runs as go-to-market.

**Sentry Founding Story** https://research.contrary.com/company/sentry
Open-source SDK with sub-5-minute install; TC's contributor SDK must achieve the same.

**Tea Protocol** "Proof of Contribution" https://tea.xyz/
Blockchain-based open-source contribution tracking, similar goals but TC focuses on AI traces, not package dependencies.

**Community-Led Growth for Developer Tools (2026)** https://www.idlen.io/blog/community-led-growth-developer-tools/
Framework for building developer communities before product-market fit.

**TraceLab Dataset** "4,265 Coding Agent Sessions" https://arxiv.org/html/2606.30560v1
Largest public agent trace dataset; TC can ingest as seed data to demonstrate novelty scoring before organic contributions.

---

## 9. Infrastructure

**Nygard (2007)** *Release It! Design and Deploy Production-Ready Software* Pragmatic Bookshelf.
Circuit breaker, bulkhead, timeout patterns; TC's scoring pipeline must degrade gracefully when individual rungs fail.

**Gray & Reuter (1992)** *Transaction Processing: Concepts and Techniques* Morgan Kaufmann.
ACID semantics: TC's ingest pipeline must guarantee accepted traces are durably stored and scored atomically.

**Kulkarni et al. (2014)** "Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases" https://cse.buffalo.edu/tech-reports/2014-04.pdf
Hybrid logical clocks for consistent causal ordering when TC federates across instances without synchronized clocks.

**Neubert et al. (2022)** "Introduction to Hyperdimensional Computing for Robotics" https://arxiv.org/abs/2106.05268
BSC hardware achieves 3-5ns per comparison, establishing the performance ceiling for TC's HDC fingerprint comparisons.

**Dageville et al. (2016)** "The Snowflake Elastic Data Warehouse" SIGMOD 2016.
Separation of storage and compute; TC's scoring compute should scale independently of trace storage.

**Lin et al. (2025)** `[v3]` "Sleep-time Compute: Pre-filling Agent Context During Idle" Letta/Berkeley. https://arxiv.org/abs/2504.13171
TC can pre-compute trace summaries and embeddings during off-peak hours, amortizing scoring costs across the diurnal cycle.

---

## Citation Format

Papers are listed as: **Author (Year)** "Title" Venue. URL. One-sentence TC relevance. Papers marked `[v3]` were added in the latest revision (34 new papers). URLs verified as of August 2026.
