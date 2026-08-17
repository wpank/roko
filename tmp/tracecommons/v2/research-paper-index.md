# Research Papers Relevant to TraceCommons

A curated reference library of ~90 papers and resources most relevant to TraceCommons'
design and implementation. Drawn from the roko citation index (~270 papers) plus
additional research. Organized around TC's three priorities: novelty detection,
production hardening, and user acquisition.

Use this as source material for deep research queries, grant writing, and implementation
decisions. Each entry includes a citation, URL where available, and a 1-2 line relevance
note explaining why it matters for TC specifically.

---

## 1. Novelty Detection & Scoring

### Code Similarity & Clone Detection

**Song et al. (2024)** "Revisiting Code Similarity Evaluation with AST Edit Distance"
ACL 2024. https://arxiv.org/abs/2404.08817
AST-based similarity (TSED) captures hierarchical structure that token-level metrics miss.
Relevant to scoring novelty of tool-call sequences in agent traces.

**CSSG (2025)** "Measuring Code Similarity with Semantic Graphs"
EMNLP 2025. https://arxiv.org/html/2601.04085v1
Program dependence graph (PDG) based approach captures data flow, not just syntax.
Useful for detecting functionally equivalent but structurally different agent strategies.

**GraphCodeBERT (2024)** "Improving Source Code Similarity Measurement Through GraphCodeBERT"
https://arxiv.org/html/2408.08903
Learned code embeddings that incorporate data flow edges. Potential embedding backbone
for trace code-snippet fingerprinting.

**BDiff (2025)** "Block-aware Text-based Code Differencing"
https://arxiv.org/abs/2510.21094
Block-level edit detection outperforms line-level diff for understanding what changed.
Applicable to computing minimal deltas between trace revisions.

### Compression & Information Theory

**Li et al. (2004)** "The Similarity Metric"
IEEE Transactions on Information Theory 50(12).
Defines Normalized Compression Distance (NCD) -- parameter-free, universal similarity
via Kolmogorov complexity approximation. Foundation for TC's compression-based pre-filter.

**Jiang et al. (2023)** "Less is More: Parameter-Free Text Classification with Gzip"
ACL 2023. https://arxiv.org/pdf/2212.09410
NCD + kNN achieves competitive classification without any model training. Validates
compression-based approach as a viable cheap first-pass novelty filter.

**Shannon (1948)** "A Mathematical Theory of Communication"
Bell System Technical Journal 27(3).
Foundational. Information entropy underpins every novelty metric -- perplexity,
compression ratios, and surprise all derive from Shannon's framework.

**Itti & Baldi (2005)** "Bayesian Surprise Attracts Human Attention"
NeurIPS 2005.
Formalizes surprise as KL divergence between prior and posterior. Directly applicable
to scoring how much a new trace shifts beliefs about agent behavior.

### Embedding & Similarity Search

**Kanerva (2009)** "Hyperdimensional Computing: An Introduction to Computing in
Distributed Representation with High-Dimensional Random Vectors"
Cognitive Computation 1(2).
Foundational HDC reference. TC's fingerprinting uses MAP-B binary hypervectors
(10,240 bits) with role-filler binding for O(1) novelty comparison.

**Kleyko et al. (2022)** "A Survey on Hyperdimensional Computing aka Vector Symbolic
Architectures"
ACM Computing Surveys. https://arxiv.org/abs/2111.06077
Comprehensive survey of HDC/VSA variants. Useful for choosing between MAP-B, MAP-C,
and BSC representations for different fingerprint use cases.

**Charikar (2002)** "Similarity Estimation Techniques from Rounding Algorithms"
STOC 2002.
SimHash -- locality-sensitive hashing for binary codes. Fast approximate near-duplicate
detection as a dedup pre-filter before expensive novelty scoring.

**Malkov & Yashunin (2020)** "Efficient and Robust Approximate Nearest Neighbor
Search Using Hierarchical Navigable Small World Graphs"
IEEE TPAMI. https://arxiv.org/abs/1603.09320
HNSW achieves O(log N) approximate nearest-neighbor search. The index structure
TC needs for sub-millisecond similarity queries at scale.

**Zhang et al. (2023)** "SPFresh: Incremental In-Place Update for Billion-Scale
Vector Search"
SOSP 2023.
Incremental vector index updates without full rebuild. Critical for TC where the
corpus grows continuously and re-indexing is prohibitive.

**OpenAI (2022)** "Text and Code Embeddings by Contrastive Pre-Training"
https://arxiv.org/pdf/2201.10005
Contrastive learning produces embeddings where cosine similarity correlates with
semantic similarity. Baseline embedding approach for trace content vectors.

### Perplexity & LLM-Based Scoring

**Zhang et al. (2025)** "Perplexity Predicts Scientific Surprise"
https://arxiv.org/abs/2509.05591
Validates perplexity as a novelty signal for scientific text, but distribution is
bimodal -- very novel and very boilerplate both score high. TC must handle this.

**Padmakumar et al. (2025)** "Measuring Novelty of LLM Outputs as the Frontier"
https://arxiv.org/abs/2504.09389
Defines novelty = harmonic_mean(originality, quality). Low-quality garbage is not
novel. Directly informs TC's two-axis scoring design.

**Farquhar et al. (2024)** "Detecting Hallucinations in Large Language Models Using
Semantic Entropy"
Nature. https://arxiv.org/abs/2302.09611
Semantic entropy groups paraphrases before measuring uncertainty. Applicable to
distinguishing genuine trace diversity from noisy reformulations.

### Novelty Metrics & Evaluation

**Ai et al. (2025)** "NovAScore: A New Automatic Metric for Evaluating Document-Level
Novelty"
COLING 2025. https://arxiv.org/abs/2409.09249
Decomposes documents into Atomic Content Units and scores each for novelty against a
reference set. Directly applicable to trace-level novelty decomposition.

**Wang et al. (2025)** "Review of Novelty Measurements of Academic Papers"
https://arxiv.org/pdf/2501.17456
Surveys ranking-based annotation methods for novelty. Useful for designing TC's
human-eval calibration protocol.

**Liu et al. (2026)** "Automated Creativity Evaluation of Large Language Models"
https://arxiv.org/pdf/2606.11762
LLM-as-judge for novelty assessment. Relevant to TC's consideration of using LLMs
as an upper-rung novelty oracle.

### Process Mining & Trace Analysis

**van der Aalst et al. (2025)** "Detecting Anomalous Patterns in Process Executions"
Conformance checking techniques for identifying novel execution traces. Maps directly
to TC's problem of scoring agent trace novelty against known patterns.

**Nolle et al. (2025)** "Control-flow Anomaly Detection by Process Mining"
https://arxiv.org/pdf/2502.10211
Feature extraction from process models for anomaly detection. Applicable to TC's
gate pipeline for identifying structurally unusual agent workflows.

**Cognition Labs (2025)** "TraceLab: Characterizing Coding Agent Workloads"
https://arxiv.org/html/2606.30560v1
Dataset of 4,265 coding agent sessions with 357K LLM steps. Largest public agent
trace corpus -- potential seed data for TC's initial index and benchmarks.

### Dedup at Scale

**Rensa** "Rust MinHash implementation"
https://github.com/beowolx/rensa
608x faster than Python datasketch. Production-grade MinHash for TC's Bloom filter
dedup rung. Rust native, easy to integrate.

**FED (2025)** "GPU-accelerated Fuzzy Deduplication"
https://arxiv.org/html/2501.01046v2
Processes 1.2T tokens in 6 hours on commodity GPUs. Establishes the throughput
ceiling TC should target for batch dedup operations.

---

## 2. Privacy & Security

**Goldwasser, Micali & Rackoff (1985)** "The Knowledge Complexity of Interactive
Proof Systems"
STOC 1985.
Foundational ZK paper. TC's privacy architecture uses ZK proofs so contributors can
prove trace properties (novelty score, authorship) without revealing content.

**Van Bulck et al. (2024)** "TEE.Fail: TEE Vulnerability Analysis"
Systematic survey of TEE attacks (SGX, TrustZone, SEV). Essential reading for TC's
TEE-based scoring enclave design -- know what you're defending against.

**EU AI Act (2024)** Regulation 2024/1689
Article 12 mandates logging for high-risk AI systems (effective Aug 2, 2026).
Article 50 requires AI-generated content marking. TC is positioned as compliance
infrastructure for both requirements.

**Dalrymple et al. (2024)** "Towards Guaranteed Safe AI"
https://arxiv.org/abs/2405.06624
Framework: world model + safety specification + verifier. TC's gate pipeline
implements the verifier role for agent trace safety.

**Bai et al. (2022)** "Constitutional AI: Harmlessness from AI Feedback"
https://arxiv.org/abs/2212.08073
Structural constraints on AI behavior via self-critique. Relevant to TC's approach
of embedding safety rules in the scoring pipeline rather than relying on training.

**Merkle (1987)** "A Digital Signature Based on a Conventional Encryption Function"
CRYPTO 1987.
Merkle trees for tamper-evident data structures. TC uses Merkle proofs in its SCITT
append-only transparency log for trace provenance.

**Breuer et al. (2024)** "Data Donation: Best Practices and Ethical Considerations"
https://link.springer.com/article/10.1007/s11135-024-01983-x
Progressive disclosure for data sharing programs. Directly informs TC's contributor
onboarding: start with metadata only, escalate to full traces with informed consent.

**Dwork (2006)** "Differential Privacy"
ICALP 2006.
Formal framework for privacy-preserving data analysis. TC's aggregate statistics
(corpus novelty distributions, contributor rankings) require DP guarantees.

**SCITT / IETF RFC 9943 (2025)** "Supply Chain Integrity, Transparency, and Trust"
Append-only transparency ledger with Merkle inclusion proofs. TC's provenance layer
implements SCITT for immutable trace audit trails.

**W3C (2022)** "Decentralized Identifiers (DIDs) v1.0"
W3C Recommendation.
Decentralized identity for contributors without central authority. TC's identity
layer supports DIDs for cross-instance contributor verification.

---

## 3. Agent Systems & Trace Formats

**Park et al. (2023)** "Generative Agents: Interactive Simulacra of Human Behavior"
https://arxiv.org/abs/2304.03442
Three-factor retrieval: recency, importance, relevance. TC's trace ranking borrows
this weighted retrieval model for surfacing relevant historical traces.

**Packer et al. (2023)** "MemGPT: Towards LLMs as Operating Systems"
https://arxiv.org/abs/2310.08560
Structured memory blocks with explicit management. Informs TC's trace storage
architecture: hot/warm/cold tiers with promotion/demotion policies.

**Chhikara et al. (2025)** "Mem0: Building Production-Ready AI Agent Memory"
https://arxiv.org/abs/2504.19413
Production patterns for agent memory: extraction, dedup, conflict resolution.
Directly applicable to TC's contributor-side trace preprocessing.

**Xu et al. (2025)** "A-MEM: Agentic Memory for LLM Agents"
https://arxiv.org/abs/2502.12110
Atomic notes with dynamic bi-directional links. TC's knowledge graph approach to
trace relationships draws from this linked-note architecture.

**Liu et al. (2025)** "Memory in the Age of AI Agents: A Comprehensive Survey"
https://arxiv.org/abs/2512.13564
Taxonomy of agent memory systems (episodic, semantic, procedural). Useful for
classifying what type of memory each trace represents.

**Lee et al. (2026)** "Meta-Harness: Towards Benchmarking LLM-based Coding Agents"
https://arxiv.org/abs/2603.28052
"The scaffold IS the product." Validates TC's bet that tooling around agent execution
(tracing, scoring, sharing) is more valuable than the agents themselves.

**Zhao et al. (2024)** "ExpeL: LLM Agents Are Experiential Learners"
https://arxiv.org/abs/2308.10144
Cross-task experience extraction -- agents learn transferable insights from past
episodes. TC enables this across organizations, not just within one agent.

**Shinn et al. (2023)** "Reflexion: Language Agents with Verbal Reinforcement Learning"
NeurIPS 2023. https://arxiv.org/abs/2303.11366
Self-reflection as a learning signal. TC's trace scoring can identify which reflection
patterns correlate with improved downstream performance.

**Anthropic (2025)** "Building Effective Agents"
https://www.anthropic.com/engineering/building-effective-agents
Production patterns from Anthropic's agent engineering. Context window management,
tool design, and orchestration patterns that define the traces TC ingests.

**OpenTelemetry (2024-2025)** "GenAI Semantic Conventions"
https://opentelemetry.io/docs/specs/semconv/gen-ai/
Emerging standard for AI/LLM observability spans. TC's trace schema should align
with OTel GenAI conventions to ease adoption from existing instrumented systems.

**Cognition Labs (2025)** "Agent Trace Specification" (informal)
Joint effort between Cursor and Cognition (Jan 2026) to standardize agent trace
formats. TC should track and align with this emerging de facto standard.

**Anthropic (2025)** "Context Engineering for Agents"
Techniques for managing context windows in production agents. Informs TC's
understanding of what information agents actually use from retrieved traces.

**Langchain (2025)** "LangSmith Trace Format"
De facto trace format for LangChain ecosystem. TC's ingest pipeline needs a
LangSmith adapter given the framework's market share.

---

## 4. Incentive Design & Data Markets

**Ostrom (1990)** *Governing the Commons: The Evolution of Institutions for
Collective Action*
Cambridge University Press.
THE reference for commons governance. Eight design principles for sustainable
common-pool resources. TC is literally a commons -- this book is the playbook.

**Vickrey (1961)** "Counterspeculation, Auctions, and Competitive Sealed Tenders"
Journal of Finance 16(1).
**Clarke (1971)** "Multipart Pricing of Public Goods"
Public Choice 11(1).
**Groves (1973)** "Incentives in Teams"
Econometrica 41(4).
VCG mechanism: truthful bidding is a dominant strategy. Foundation for TC's
trace valuation auctions where contributors price their data honestly.

**Arrow (1962)** "Economic Welfare and the Allocation of Resources for Invention"
NBER.
Information as a public good -- once revealed, non-excludable. Frames TC's core
economic challenge: incentivizing trace contribution when sharing reduces scarcity.

**Glickman (1999)** "The Glicko-2 System"
http://www.glicko.net/glicko/glicko2.pdf
Dynamic rating with reliability deviation and volatility. TC uses Glicko-2 for
contributor reputation that adapts quickly to changing contribution quality.

**Meritrank (2022)** "MeritRank: Sybil Tolerant Reputation for Merit-Based
Token Economies"
https://arxiv.org/abs/2207.09950
Graph-based reputation resistant to Sybil attacks. Critical for TC where
pseudonymous contributors could create sock puppets to inflate scores.

**Duetting et al. (2024)** "Mechanism Design for Large Language Models"
https://arxiv.org/abs/2310.10826
Token auction model where LLM outputs are allocated via mechanism design.
Applicable to TC's credit allocation when multiple contributors provide
overlapping traces.

**Agent Exchange (2025)** "RTB-Inspired Auction for Agent Tasks"
https://arxiv.org/abs/2507.03904
Real-time bidding applied to agent task allocation. Relevant to TC's marketplace
where consumers bid for access to trace bundles.

**Kahneman & Tversky (1979)** "Prospect Theory: An Analysis of Decision Under Risk"
Econometrica 47(2).
Loss aversion is 2.25x stronger than equivalent gains. TC's credit system should
frame contributions as "avoiding credit decay" rather than "earning credits."

**Shapley (1953)** "A Value for N-Person Games"
Contributions to the Theory of Games II.
Shapley values for fair credit attribution when multiple traces contribute to a
composite insight. Computationally expensive but theoretically optimal.

---

## 5. User Acquisition & Developer Tools

**Sweller (1988)** "Cognitive Load Theory"
Cognitive Science 12(2).
Minimize extraneous cognitive load in onboarding. TC's `tc init` must require
fewer than 3 decisions to produce a working configuration.

**NFX (2023)** "19 Tactics for Marketplace Cold Start"
https://www.nfx.com/post/19-marketplace-tactics-for-overcoming-the-chicken-or-egg-problem
Marketplace bootstrapping playbook. TC faces the classic chicken-and-egg: no
contributors without consumers, no consumers without traces. Tactics 3 (single
player mode) and 7 (come for the tool, stay for the network) are most applicable.

**PostHog** "How PostHog Grows: The Power of Being Contrarian"
https://www.howtheygrow.co/p/how-posthog-grows-the-power-of-being
97% organic growth through radical transparency and developer-first UX. TC should
emulate: open-source core, public roadmap, no sales team initially.

**Langfuse / YC W23** "How Langfuse Built in Public"
https://posthog.com/spotlight/startup-langfuse
Used their YC batch as first users. TC can seed the commons with traces from
roko's own self-hosting runs -- eat your own dog food as go-to-market.

**Sentry Founding Story**
https://research.contrary.com/company/sentry
Open-source SDK with sub-5-minute install became the wedge. TC's contributor
SDK must achieve the same: `cargo add tc-contributor && tc init` in under 5 minutes.

**Tea Protocol** "Proof of Contribution"
https://tea.xyz/
Blockchain-based open-source contribution tracking. Relevant to TC's on-chain
credit layer -- similar goals but TC focuses on AI traces, not package dependencies.

**Community-Led Growth for Developer Tools (2026)**
https://www.idlen.io/blog/community-led-growth-developer-tools/
Framework for building developer communities before product-market fit.
TC should invest in community (Discord, office hours, showcases) early.

**TraceLab Dataset** "4,265 Coding Agent Sessions"
https://arxiv.org/html/2606.30560v1
Largest public agent trace dataset. TC can ingest this as seed data to demonstrate
novelty scoring and attract researchers before organic contributions arrive.

---

## 6. Infrastructure

**Nygard (2007)** *Release It! Design and Deploy Production-Ready Software*
Pragmatic Bookshelf.
Circuit breaker, bulkhead, timeout patterns. TC's scoring pipeline must degrade
gracefully when individual rungs fail -- not block the entire ingest path.

**Gray & Reuter (1992)** *Transaction Processing: Concepts and Techniques*
Morgan Kaufmann.
ACID semantics for trace processing. TC's ingest pipeline must guarantee that
accepted traces are durably stored and scored atomically.

**Kulkarni et al. (2014)** "Logical Physical Clocks and Consistent Snapshots in
Globally Distributed Databases"
https://cse.buffalo.edu/tech-reports/2014-04.pdf
Hybrid logical clocks for distributed trace ordering. When TC federates across
instances, traces need a consistent causal ordering without synchronized clocks.

**Neubert et al. (2022)** "Introduction to Hyperdimensional Computing for Robotics"
https://arxiv.org/abs/2106.05268
BSC hardware achieves 3-5ns per comparison. Establishes the performance ceiling
for TC's HDC fingerprint comparisons on specialized hardware.

**Dageville et al. (2016)** "The Snowflake Elastic Data Warehouse"
SIGMOD 2016.
Separation of storage and compute. TC's architecture should allow scoring compute
to scale independently of trace storage, especially for batch re-scoring.

---

## Citation Format Notes

Papers are listed as: **Author (Year)** "Title", Venue/Publisher. URL.
Followed by 1-2 lines of TC-specific relevance.

For papers without URLs, the citation should be sufficient for lookup via
Google Scholar or Semantic Scholar. All URLs were verified as of August 2026.

---

## Cross-Reference to TC Documents

| TC Document | Most Relevant Papers |
|---|---|
| [Implementation Roadmap](tc-implementation-roadmap.md) | Kanerva 2009, Li 2004, Malkov 2020, Zhang 2023 |
| [Grant Proposals](tc-grant-proposals.md) | Ostrom 1990, EU AI Act, Dwork 2006, Park 2023 |
| [Privacy & Security](tc-privacy-security.md) | Goldwasser 1985, Merkle 1987, Dwork 2006, Van Bulck 2024 |
| [Novel Research Ideas](tc-novel-research-ideas.md) | Itti 2005, Kahneman 1979, Shapley 1953, Li 2004 |
| [Competitive Landscape](tc-competitive-landscape.md) | NFX 2023, PostHog, Langfuse, OTel GenAI |
| [UX Design](tc-ux-design.md) | Sweller 1988, Breuer 2024, Sentry |
| [IronClaw Integration](tc-ironclaw-integration.md) | Van Bulck 2024, OTel GenAI, Kulkarni 2014 |
