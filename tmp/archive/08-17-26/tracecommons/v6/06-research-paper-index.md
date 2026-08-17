# Research Paper Index

**Date**: August 2026 (v6.2)

**Context**: This is the research paper index for TraceCommons (TC), an open-source Rust-based privacy-preserving registry of AI coding agent session traces. TC scores traces for quality and novelty inside TEEs and compensates contributors via NEAR blockchain. The papers below inform TC's scoring pipeline, integrations, privacy architecture, incentive design, and growth strategy.

~220+ papers across 12 categories. Papers marked `[v3]` are from the v3/v5 research sweeps. Papers marked `[v6]` are new additions from the 27-agent research sweep. Each entry: citation, URL (where available), and TC relevance.

**Verification key**: Papers with arXiv URLs have been cross-checked. Papers marked `[v6-unverified]` were surfaced by research agents and lack independent URL confirmation -- treat citations as provisional until verified against arXiv or conference proceedings.

---

## 1. Novelty Detection & Scoring

### Code Similarity & Clone Detection

**Song et al. (2024)** "Revisiting Code Similarity Evaluation with AST Edit Distance" ACL 2024. https://arxiv.org/abs/2404.08817 -- AST-based similarity for tool-call sequence novelty.

**CSSG (2025)** "Measuring Code Similarity with Semantic Graphs" EMNLP 2025. https://arxiv.org/html/2601.04085v1 -- Program dependence graphs for functionally equivalent but structurally different strategies.

**GraphCodeBERT (2024)** "Improving Source Code Similarity Through GraphCodeBERT" https://arxiv.org/html/2408.08903 -- Learned code embeddings with data flow edges for trace fingerprinting.

**BDiff (2025)** "Block-aware Text-based Code Differencing" https://arxiv.org/abs/2510.21094 -- Block-level edit detection for trace revision deltas.

### Compression & Information Theory

**Li et al. (2004)** "The Similarity Metric" IEEE Transactions on Information Theory 50(12). -- NCD foundation.

**Jiang et al. (2023)** "Less is More: Parameter-Free Text Classification with Gzip" ACL 2023. https://arxiv.org/pdf/2212.09410 -- NCD + kNN validates compression as novelty pre-filter.

**Shannon (1948)** "A Mathematical Theory of Communication" Bell System Technical Journal 27(3).

**Itti & Baldi (2005)** "Bayesian Surprise Attracts Human Attention" NeurIPS 2005. -- KL divergence surprise scoring.

### Embedding & Similarity Search

**Kanerva (2009)** "Hyperdimensional Computing: An Introduction" Cognitive Computation 1(2). -- HDC foundation; TC uses MAP-B 10,240-bit vectors.

**Kleyko et al. (2022)** "Survey on Hyperdimensional Computing" ACM Computing Surveys. https://arxiv.org/abs/2111.06077

**Charikar (2002)** "Similarity Estimation from Rounding Algorithms" STOC. -- SimHash for dedup.

**Malkov & Yashunin (2020)** "HNSW Graphs" IEEE TPAMI. https://arxiv.org/abs/1603.09320 -- O(log N) approximate NN.

**Zhang et al. (2023)** "SPFresh: Incremental In-Place Update for Billion-Scale Vector Search" SOSP 2023.

**OpenAI (2022)** "Text and Code Embeddings by Contrastive Pre-Training" https://arxiv.org/pdf/2201.10005

### Perplexity & LLM-Based Scoring

**Zhang et al. (2025)** "Perplexity Predicts Scientific Surprise" https://arxiv.org/abs/2509.05591 -- Bimodal distribution warning.

**Padmakumar et al. (2025)** "Measuring Novelty as the Frontier" https://arxiv.org/abs/2504.09389 -- `novelty = harmonic_mean(originality, quality)`.

**Farquhar et al. (2024)** "Detecting Hallucinations Using Semantic Entropy" Nature. https://arxiv.org/abs/2302.09611

### Novelty Metrics & Evaluation

**Ai et al. (2025)** "NovAScore" COLING 2025. https://arxiv.org/abs/2409.09249 -- ACU-level novelty.

**Wang et al. (2025)** "Review of Novelty Measurements" https://arxiv.org/pdf/2501.17456

**Liu et al. (2026)** "Automated Creativity Evaluation" https://arxiv.org/pdf/2606.11762

### Data Valuation & Influence

**Ghorbani & Zou (2019)** "Data Shapley: Equitable Valuation of Data for Machine Learning" ICML 2019. -- Foundational data Shapley paper; gameability later proven (see Agarwal et al. below and Section 10).

**Choe et al. (2025)** `[v3]` "LoGra" NeurIPS 2025. https://arxiv.org/abs/2405.13954 -- Influence-function data valuation.

**Deng et al. (2026)** `[v3]` "For-Value" ACL 2026. https://arxiv.org/abs/2508.10180 -- Forward-only influence.

**arXiv:2511.19803 (2025)** `[v3]` "Forward-Only Test-Time Inference for Data Attribution" https://arxiv.org/abs/2511.19803

**Kwon et al. (2022)** `[v6]` "Beta Shapley: A Unified and Noise-Reduced Approach" AISTATS. -- Semivalue variant; inherits gameability.

**Agarwal et al. (2025)** `[v6]` "On the Fragility of Shapley-based Data Valuation" https://arxiv.org/abs/2504.05563 -- Proves strategic misrepresentation inflates Shapley values. TC must use DSIC mechanisms.

**Blum et al. (2025)** `[v6]` "Quotient Semivalues" https://arxiv.org/abs/2605.07663 -- Sybil attack amplification 1.74x for semivalues.

**Hu et al. (2025)** `[v6]` "On the Gameability of the Entire Semivalue Class" https://arxiv.org/abs/2506.12619 -- Proves the entire semivalue class (including Shapley, Banzhaf, Beta) is gameable.

**Chen et al. (2026)** `[v6-unverified]` "Incentive-Compatible Data Valuation via VCG for Data Marketplaces" -- VCG mechanism for truthful data pricing.

**Fernandez et al. (2026)** `[v6-unverified]` "Fairshare: Fair Pricing for Data Marketplaces" -- Mechanism design for equitable pricing.

**Xu et al. (2026)** `[v6-unverified]` "DSIC Data Markets: A Survey" -- Survey of dominant-strategy incentive-compatible mechanisms for data.

**Li et al. (2025)** `[v6-unverified]` "Myerson-Optimal Data Pricing Under Information Asymmetry" -- Revenue-optimal mechanisms when data quality is private information.

**Wang et al. (2026)** `[v6-unverified]` "Shapley Value Gaming in Federated Learning" -- Demonstrates practical Shapley gaming attacks in federated settings.

**Sim et al. (2026)** `[v6-unverified]` "Data Valuation at Scale: Challenges and Approximations" -- Scaling VCG from O(n^2) via sampling-based approximation.

**Jia et al. (2025)** `[v6-unverified]` "Towards Efficient Data Valuation: Amortized Shapley via Neural Networks" -- Amortized valuation; speed gains but inherits gameability.

### Multi-Module Pipeline Optimization

**EMNLP 2025** `[v3]` "Compound AI Systems Optimization" -- Multi-module pipeline optimization for TC's multi-rung gate.

### Dedup at Scale

**Rensa** "Rust MinHash" https://github.com/beowolx/rensa -- 608x faster than Python datasketch.

**FED (2025)** "GPU-accelerated Fuzzy Deduplication" https://arxiv.org/html/2501.01046v2 -- 1.2T tokens in 6 hours.

---

## 2. Agent Trace Analysis & Failure Attribution

### Failure Attribution (v5 core)

**Zhu et al. (2026)** `[v6-verified]` "AgentDebugX: An Open-Source Toolkit for Failure Observability, Attribution, and Recovery in LLM Agents" https://arxiv.org/abs/2607.18754 -- Error Hub with opt-in sharing of scrubbed failure-diagnosis-repair bundles. DeepDebug: 28.8% exact agent+step accuracy on Who&When (vs 21.7% baseline). GAIA: repairs 13/73 failed tasks, improving accuracy 55.8% to 63.6%. TC relevance: closest existing system to TC's Error Hub. TC is the next-generation version -- broader scope (all traces, not just failures), TEE-attested, NEAR credit-incentivized.

**Zhang et al. (2026)** `[v6-verified]` "AgenTracer-8B" ICLR 2026 **[ICLR status UNVERIFIED]**. https://arxiv.org/abs/2509.03312 -- First automated framework for annotating failed multi-agent trajectories. Outperforms Gemini-2.5-Pro and Claude-4-Sonnet by up to 18.18% on Who&When. TC relevance: candidate scoring model for failure attribution inside TC's TEE; TracerTraj dataset as potential seeding corpus for TC's failure-biased gap.

**arXiv:2505.08638 (2025)** `[v6-verified]` "TRAIL: Span-Level Failure Taxonomy" https://arxiv.org/abs/2505.08638 -- Three-domain failure taxonomy (Reasoning Errors, System Execution Errors, Planning/Coordination Errors). 148 traces, 1,987 OTel spans, 841 annotated errors. OTel/OpenInference compatible. TC relevance: canonical failure classification for TC's Error Hub; TC's corpus (~352) already exceeds TRAIL's 148.

**Li et al. (2025)** `[v6]` "Who&When: Diagnosing Multi-step Agent Failures" ICML 2025 Spotlight. https://arxiv.org/abs/2505.00212 -- Failure attribution benchmark. Correlational methods ~14% accuracy; causal methods needed.

**arXiv:2606.04990 (2026)** `[v6-verified]` "Evidence Tracing Survey" https://arxiv.org/abs/2606.04990 -- Survey of evidence tracing methods across LLM systems. TC relevance: informs TC's attribution pipeline design and provenance chain architecture.

### Failure Attribution (v6 additions)

**Shah (2026)** `[v6]` "Causal Agent Replay (CAR)" https://arxiv.org/abs/2606.08275 -- SCM + do()-resample, Monte-Carlo Shapley. Open-source. **Highest-impact attribution method.**

**arXiv:2605.25338 (2026)** `[v6]` "CausalFlow" https://arxiv.org/abs/2605.25338 -- Single-agent interventional Causal Responsibility Score + minimal ranked repairs. Open-source.

**Zero-Replay Debugging (2026)** `[v6]` "Zero-Replay Debugging for LLM Agents" https://arxiv.org/abs/2606.14805 -- Fault localization without re-execution.

**StepFinder (2026)** `[v6]` "StepFinder: Localizing Critical Steps in Agent Trajectories" https://arxiv.org/abs/2606.03467 -- Step-level attribution for agent failures.

**CoACT (2026)** `[v6]` "Collaborative Agent Causal Tracing" https://arxiv.org/abs/2607.02911 -- Multi-agent causal attribution.

**SWE-MeM (2026)** `[v6]` "SWE-Memory: Learning from Past Agent Failures" https://arxiv.org/abs/2606.28434 -- Failure memory for SWE agents.

**TraceProbe (2026)** `[v6-unverified]` "Automated Root-Cause Localization for Agent Failures" -- 82% accuracy on failure step identification without re-execution. Attention-based analysis.

**AgentLocate (2026)** `[v6-unverified]` "Fault Localization via Attention Pattern Analysis" -- Uses attention patterns over tool-call sequences for fault localization.

**TraceSIR (2026)** `[v6-unverified]` "Statistical Inference for Agent Trace Repair" -- Statistical methods for identifying minimal repair sets from failed traces.

**VCC (2026)** `[v6-unverified]` "Verification-Conditioned Counterfactuals for Agent Debugging" -- Generates counterfactual traces that would have succeeded. Expensive but high-quality.

**FailureNet (2026)** `[v6-unverified]` "Graph Neural Networks for Agent Failure Classification" -- GNN over tool-call graphs for failure type classification.

**RootTrace (2026)** `[v6-unverified]` "Root Cause Analysis in Multi-Step Agent Executions" -- Hierarchical attribution across nested agent calls.

**AgentPostMortem (2026)** `[v6-unverified]` "Automated Post-Mortem Reports from Agent Traces" -- Generates structured post-mortem reports from failed traces.

**DebugAgent (2026)** `[v6-unverified]` "Training Agents to Debug Other Agents Using Trace Data" -- Meta-learning from failure traces to produce debugging agents.

### Trace Analysis & Characterization

**Zhu et al. (2026)** `[v6-verified]` "TraceLab: Characterizing Coding Agent Workloads for LLM Serving" University of Washington. https://arxiv.org/html/2606.30560v1 -- 4,265 sessions, 357K LLM steps. Workload characterization study of real coding agent sessions -- shows the scale and complexity of LLM serving demands from coding agents. ⚠️ NOT Cognition Labs; does NOT report a "31% failure rate" -- earlier versions of this index (v3/v5) contained both errors, corrected in v6.2.

**STRACE (2026)** `[v6-unverified]` "Structured Trace Analysis Using Process Mining Abstractions" -- Structural features add 15-22% predictive power beyond content-only.

**PrefixGuard (2026)** `[v6-unverified]` "Prefix-Based Conformance Checking for Agent Behavior" -- Real-time behavioral deviation detection.

**Agent Behavior Mining (2026)** `[v6]` "Process Mining for Agent Interaction Logs" BPM 2026. https://arxiv.org/abs/2606.20669 -- ~60% of sessions follow 5-7 canonical workflow patterns.

**PASTE (2026)** `[v6-unverified]` "Process-Aware Scoring of Trace Elements" -- Combines process mining with LLM-based element scoring.

**TraceGraph (2026)** `[v6-unverified]` "Graph Representations of Agent Execution Traces" -- Converts sequential traces to graph structure for pattern mining.

**AgentAtlas (2026)** `[v6-unverified]` "Mapping Agent Behavior Landscapes from Trace Corpora" -- Unsupervised clustering of agent behavioral strategies across trace corpora.

**SessionSense (2026)** `[v6-unverified]` "Making Sense of AI Coding Sessions" -- User study: developers spend 40% of review time on 10% of trace events.

---

## 3. Conformal Prediction & Calibration

**Xu & Lu (2025)** `[v3]` "TECP" https://arxiv.org/abs/2509.00461 -- Token-entropy nonconformity + split conformal. Logit-free.

**arXiv:2401.09376 (2024)** `[v3]` "Hui-Walter Bayesian Estimator" https://arxiv.org/abs/2401.09376

**ToolChain-CRC (2026)** `[v6]` "Conformal Prediction for Tool-Call Chain Verification" https://arxiv.org/abs/2606.18467 -- Coverage guarantees for multi-step agent actions. Directly applicable to TC's gate pipeline.

**Role-Stratified CRC (2026)** `[v6]` "Role-Stratified Conformal Risk Control" https://arxiv.org/abs/2607.24343 -- Conformal prediction stratified by agent role.

**Conformal Agent Error Attribution (2026)** `[v6]` "Distribution-Free Uncertainty for Agent Failure Attribution" https://arxiv.org/abs/2605.06788 -- Conformal intervals on failure attribution confidence.

**PASC (2026)** `[v6]` "Prediction-Aligned Summary Calibration" https://arxiv.org/abs/2605.18812 -- Calibrated summaries with coverage guarantees.

**Angelopoulos & Bates (2023)** `[v6]` "Conformal Prediction: A Gentle Introduction" -- Tutorial reference for CP implementation.

**Romano et al. (2020)** `[v6]` "Classification Validity for Nonexchangeable Data" -- Handles distribution shift in CP; applicable to TC's evolving contributor population.

**Barber et al. (2023)** `[v6]` "Conformal Prediction Beyond Exchangeability" -- Extends CP to time-series and dependent data; relevant for sequential trace analysis.

**Cauchois et al. (2024)** `[v6-unverified]` "Robust and Agnostic Conformal Prediction" -- Distributionally robust CP for worst-case coverage; applicable to adversarial trace submissions.

**Park et al. (2025)** `[v6-unverified]` "Conformal Risk Control for Multi-Agent Systems" -- Extends conformal risk control to multi-agent settings.

**Zeni et al. (2025)** `[v6-unverified]` "Adaptive Conformal Prediction for Streaming Data" -- Online CP updates for streaming trace ingestion.

**Feldman et al. (2026)** `[v6-unverified]` "Conformal Prediction Sets for LLM Tool Use" -- Prediction sets for which tools an agent should use; inverse application of TC's tool-call analysis.

**Gibbs & Candes (2024)** `[v6]` "Adaptive Conformal Inference Under Distribution Shift" -- ACI for non-stationary trace populations.

**Bhatnagar et al. (2023)** `[v6-unverified]` "Conformal Prediction for Natural Language Processing" -- NLP-specific CP techniques applicable to trace text scoring.

---

## 4. Trajectory Compression

**arXiv:2606.00611 (2026)** `[v3]` "TRACE" https://arxiv.org/abs/2606.00611 -- Compressor-Reader latent state. +12.6pp safety. Open-source.

**arXiv:2605.27690 (2026)** `[v3]` "TRACES" https://arxiv.org/abs/2605.27690 -- Related compression approach.

**arXiv:2604.19572 (2026)** `[v3]` "Terminal-Observation Compression" https://arxiv.org/abs/2604.19572

**ACE (2026)** `[v6]` "Adaptive Context Engine: Dynamic KV Cache Pooling for Trajectory Compression" SambaNova. https://arxiv.org/abs/2606.31564 -- Dynamically adjusts compression ratio based on content importance. 5-20x compression. Open-source.

**Slipstream (2026)** `[v6]` "Speculative Execution Compression for Agent Trajectories" https://arxiv.org/abs/2605.08580 -- Leverages speculative decoding for lossy-but-fast compression. 3-10x.

**CompactionRL (2026)** `[v6]` "RL-Trained Compression Policies for Agent Traces" https://arxiv.org/abs/2607.05378 -- Trains compression policy with safety-preservation reward. 10-30x with safety guarantees.

**ARC (2026)** `[v6-unverified]` "Adaptive Resolution Compression for Multi-Step Reasoning" -- Variable resolution: high for critical steps, low for routine. 5-15x.

**Focus (2026)** `[v6-unverified]` "Attention-Guided Selective Compression" -- Uses attention scores to identify compressible regions. 3-8x.

**TraceZip (2026)** `[v6-unverified]` "Domain-Specific Compression for Agent Execution Traces" -- Exploits trace structure (tool calls, repeated patterns) for lossless compression.

**ContextPrune (2026)** `[v6-unverified]` "Pruning Context for Long-Horizon Agent Tasks" -- Identifies and removes redundant context without affecting task completion.

**StreamCompress (2026)** `[v6-unverified]` "Streaming Compression for Real-Time Agent Traces" -- Incremental compression for live trace ingestion. Addresses TRACE's streaming limitation.

**CompressAndScore (2026)** `[v6-unverified]` "Joint Compression and Quality Scoring for Data Commons" -- Combines compression with quality scoring in a single pass. Directly relevant to TC's pipeline.

---

## 5. Skill Extraction & Experience

### Skill Extraction (v5 core)

**Ouyang et al. (2025)** `[v3]` "Scaling Agent Self-Evolving with Reasoning Memory" (ReasoningBank). https://arxiv.org/abs/2509.25140 -- Transferable reasoning strategies from trajectories. TC relevance: seeding corpus of distilled reasoning strategies for skill extraction.

**arXiv:2605.06614 (2025)** `[v6-verified]` "SkillOS" https://arxiv.org/abs/2605.06614 -- RL-based skill curation. TC relevance: SkillOS's RL-based curation approach informs TC's automated skill extraction pipeline -- quality-gated skill publishing from trace corpora.

**Suzgun et al. (2025)** `[v3]` "Dynamic Cheatsheet: Test-Time Learning with Adaptive Memory" https://arxiv.org/abs/2504.07952 -- Self-curated test-time memory. TC relevance: model for how agents can build and query persistent knowledge during execution, analogous to TC's skill store.

**arXiv:2606.01139 (2025)** `[v6-verified]` "SkillRevise" https://arxiv.org/abs/2606.01139 -- TC's gate as fixed verifier. TC relevance: validates TC's gate pipeline as a verification backend for iteratively refined skills.

**RHO (2025)** `[v3]` "Evolving Agents in the Dark: Retrospective Harness Optimization via Self-Preference" https://arxiv.org/abs/2606.05922 -- 19% gain on SWE-Bench Pro. TC relevance: self-preference-based harness optimization can inform TC's scoring calibration -- agents improving their own scaffolding produce traces with measurably different quality signatures.

**Agent Skills / SKILL.md (2025)** `[v3]` agentskills.io. Linux Foundation.

**Snyk (2026)** `[v3]` "ToxicSkills" -- 36.82% security flaw rate.

**Zhao et al. (2024)** "ExpeL" https://arxiv.org/abs/2308.10144 -- Cross-task experience extraction.

**Shinn et al. (2023)** "Reflexion" NeurIPS 2023. https://arxiv.org/abs/2303.11366

### Skill Extraction (v6 additions)

**Trace2Skill (2026)** `[v6]` "End-to-End Trace-to-Skill Extraction" https://arxiv.org/abs/2603.25158 -- Full pipeline from raw traces to publishable skills. 78% judged useful.

**AutoRefine (2026)** `[v6]` "Iterative Skill Refinement with Execution Feedback" https://arxiv.org/abs/2601.22758 -- 2.3x quality improvement over single-pass extraction.

**SkillAudit (2026)** `[v6]` "Formal Verification of Extracted Agent Skills" https://arxiv.org/abs/2606.14239 -- Catches 89% of security issues in extracted skills.

**MetaSkill-Evolve (2026)** `[v6]` "Evolutionary Skill Improvement Across Trace Corpora" https://arxiv.org/abs/2607.05297 -- Cross-corpus skill evolution over time.

**SkillGraph (2026)** `[v6-unverified]` "Graph-Based Skill Discovery from Agent Traces" -- Models skill dependencies as graphs for structured discovery.

**SafeSkill (2026)** `[v6-unverified]` "Safety-Constrained Skill Extraction" -- Extracts skills with provable safety constraints.

**SkillTransfer (2026)** `[v6-unverified]` "Cross-Agent Skill Transfer Learning" -- Transfers skills extracted from one agent type to another.

**ClawHavoc Analysis (2026)** `[v6-unverified]` "Security Analysis of the ClawHavoc Attack" -- 341 malicious skills; bypass techniques for SkillSieve/SkillSpector documented.

---

## 6. Process Mining for Agent Traces

**van der Aalst et al. (2025)** "Detecting Anomalous Patterns in Process Executions"

**Nolle et al. (2025)** "Control-flow Anomaly Detection by Process Mining" https://arxiv.org/pdf/2502.10211

**Agent Behavior Mining (2026)** `[v6]` "Classical Process Mining Applied to Agent Interaction Logs" BPM 2026. https://arxiv.org/abs/2606.20669 -- DFGs, Petri nets, conformance checking. ~60% of sessions follow 5-7 canonical patterns.

**AgentLTL (2026)** `[v6]` "Temporal Logic Specifications for Agent Behavior" https://arxiv.org/abs/2607.02599 -- LTL-based behavioral specifications for agent traces.

**PrefixGuard (2026)** `[v6-unverified]` "Prefix-Based Real-Time Conformance for Agents" -- Real-time deviation detection using prefix-based matching.

**PASTE (2026)** `[v6-unverified]` "Process-Aware Scoring of Trace Elements" -- Combines process mining with LLM scoring.

**AgentPetri (2026)** `[v6-unverified]` "Petri Net Models of AI Agent Workflows" -- Formal workflow models enabling deadlock and livelock detection.

**TraceConform (2026)** `[v6-unverified]` "Conformance Checking for Multi-Agent Coordination" -- Extends conformance checking to A2A delegation patterns.

**WorkflowMiner (2026)** `[v6-unverified]` "Mining Workflow Patterns from Large-Scale Agent Corpora" -- Scalable pattern mining for corpora >10K traces.

---

## 7. Agent Systems & Architecture

### Agent Memory

**Park et al. (2023)** "Generative Agents" https://arxiv.org/abs/2304.03442

**Packer et al. (2023)** "MemGPT" https://arxiv.org/abs/2310.08560

**Chhikara et al. (2025)** "Mem0" https://arxiv.org/abs/2504.19413

**Xu et al. (2025)** "A-MEM" https://arxiv.org/abs/2502.12110

**Liu et al. (2025)** "Memory in the Age of AI Agents" https://arxiv.org/abs/2512.13564

**Han et al. (2026)** `[v6-verified]` "LEGOMem" AAMAS 2026. https://arxiv.org/abs/2510.04851 -- Modular memory architecture for agents. TC relevance: informs how TC could structure persistent memory across trace sessions.

**Lin et al. (2025)** `[v6-verified]` "Sleep-time Compute" https://arxiv.org/abs/2504.13171 -- Offline computation during idle periods. TC relevance: model for TC's background processing of trace scoring and skill extraction during low-load periods.

### Multi-Agent Systems

**Zou et al. (2026)** `[v6-verified]` "LatentMAS" ICML 2026 Spotlight. https://arxiv.org/abs/2511.20639 -- Latent-space multi-agent communication. TC relevance: multi-agent traces with latent coordination present novel trace format challenges -- TC's schema may need extensions for implicit coordination patterns.

### Frameworks & Scaffolding

**Lee et al. (2026)** "Meta-Harness" https://arxiv.org/abs/2603.28052

**Anthropic (2025)** "Building Effective Agents"

**Anthropic (2025)** "Context Engineering for Agents"

**arXiv:2604.08224 (2026)** `[v6-verified]` "Externalization Review" https://arxiv.org/abs/2604.08224 -- Four-category taxonomy of agent externalization: Memory, Skills, Protocols, Harness. TC relevance: TC currently captures Protocol (tool calls) and Memory (outcomes) but lacks Skill layer metadata and Harness (scaffold) metadata. Gap analysis informs TC's trace schema extension roadmap.

**arXiv:2606.14674 (2026)** `[v6-verified]` "AgentSpec" https://arxiv.org/abs/2606.14674 -- Scaffold architecture determines trace structure. TC relevance: recommend capturing scaffold phase metadata (e.g., `tc.scaffold.phase`, `tc.scaffold.pattern`) in TC's OTel attribute mapping so scoring normalizes for architectural differences.

### Trace Formats & Standards

**OpenTelemetry GenAI Semantic Conventions (2024-2026)** Base specification for `gen_ai.*` attributes. Status: Development. Moved to dedicated repository June 12, 2026. https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai

**OpenInference (2026)** `[v3]` Arize AI parallel conventions

**Cognition Labs (2025)** Agent Trace Specification (informal)

**LangSmith Trace Format (2025)** De facto for LangChain ecosystem

### Protocols

**A2A Protocol (2026)** `[v3]` v1.0.0, 150+ orgs, Linux Foundation

**arXiv:2505.02279 (2025)** `[v6-verified]` "MCP/ACP/ANP/A2A Survey" https://arxiv.org/abs/2505.02279 -- Comprehensive survey of agent communication protocols. TC relevance: informs TC's protocol versioning strategy and multi-protocol event type design.

---

## 8. Privacy, Security & Verifiability

**Goldwasser, Micali & Rackoff (1985)** ZK foundations. STOC.

**Dwork (2006)** "Differential Privacy" ICALP.

**Merkle (1987)** Merkle proofs. CRYPTO.

**SCITT / IETF RFC 9943 (2025)** Supply Chain Integrity.

**Van Bulck et al. (2024)** "TEE.Fail" -- TEE vulnerability analysis.

**arXiv:2605.03213 (2026)** `[v6-verified]` "TEE Survey for Agentic AI" https://arxiv.org/abs/2605.03213 -- Survey of TEE technologies for AI agent systems. TC relevance: directly informs TC's TEE-based scoring architecture and threat model.

**arXiv:2512.15892 (2025)** `[v6-verified]` "VET: Verifiable Execution Traces" https://arxiv.org/abs/2512.15892 -- Framework for cryptographically verifiable agent execution traces. TC relevance: candidate architecture for TC's attestation layer.

**arXiv:2503.22573 (2025)** `[v6-verified]` "Cryptographic AI Pipeline" https://arxiv.org/abs/2503.22573 -- End-to-end cryptographic pipeline for AI systems. TC relevance: informs TC's privacy-preserving scoring pipeline design.

**EU AI Act (2024)** Regulation 2024/1689. Articles 12, 50. **Updated: Digital Omnibus defers Art. 12 standalone to Dec 2027.**

**Dalrymple et al. (2024)** "Towards Guaranteed Safe AI" https://arxiv.org/abs/2405.06624

**Bai et al. (2022)** "Constitutional AI" https://arxiv.org/abs/2212.08073

**Gaurav et al. (2025)** `[v6-verified]` "Governance-as-a-Service" https://arxiv.org/abs/2508.18765 -- Governance framework delivered as a composable service layer. TC relevance: model for TC's governance architecture -- scoring rules, contribution policies, and credit formulas as configurable governance modules rather than hardcoded logic.

**Governance Decay (2026)** `[v6]` "Governance Decay in Autonomous Agent Systems" https://arxiv.org/abs/2606.22528 -- How governance constraints erode over time in long-running agent deployments.

**Q-MIA (2025)** `[v6]` "Quantile-based Membership Inference Attacks" https://arxiv.org/abs/2506.05379 -- Privacy risk assessment for trace data; relevant to TC's TEE-based privacy model.

**Tomasev et al. (2026)** `[v6-verified]` "Intelligent AI Delegation" (Privilege Attenuation). https://arxiv.org/abs/2602.11865 -- Principled privilege attenuation for delegated AI tasks. TC relevance: informs TC's trust model -- traces from delegated tasks should carry attenuated trust scores reflecting the delegation chain depth.

**W3C (2022)** DIDs v1.0

**Breuer et al. (2024)** "Data Donation Best Practices" https://link.springer.com/article/10.1007/s11135-024-01983-x

---

## 9. Agent UX & Steering

**Zhao et al. (2026)** `[v6-verified]` "AgentGUI" ETH Zurich. https://arxiv.org/abs/2607.26300 -- **NOW CONFIRMED**: 38% faster trace element identification (p=0.023). Also raises task completion by up to 34pp for small agents via drift prevention. TC relevance: AgentGUI-style visualization should be integrated into TC's contributor dashboard for immediate contributor value (faster debugging while contributing).

**arXiv:2411.16627 (2024)** `[v3]` "Inference-Time Steering" https://arxiv.org/abs/2411.16627 -- **MISCLASSIFIED**: This is a ROBOTICS paper (ICRA 2025) about physical robot policy steering, NOT about LLM agent UX. Should not be cited for LLM agent steering or trace inspection. Retained in index for audit trail.

**arXiv:2604.00892 (2026)** `[v6-verified]` "Interruptible Agents" https://arxiv.org/abs/2604.00892 -- First systematic study of handling user interruptions during long-horizon tasks. Three interruption types: Addition, Revision, Retraction. Current models perform poorly on all three. TC relevance: interrupted traces are high-value Error Hub content -- captures a failure mode (inability to handle mid-stream corrections) common in real coding sessions but underrepresented in benchmarks.

**arXiv:2505.00753 (2025)** `[v6-verified]` "LLM-Based Human-Agent Collaboration and Interaction Systems: A Survey" https://arxiv.org/abs/2505.00753 -- Four collaboration subtypes confirmed: Delegation & Direct Command, Supervision, Cooperation, and Coordination (within a broader three-type interaction taxonomy: Collaboration, Competition, Coopetition). TC relevance: capture collaboration mode as trace metadata -- a supervised coding session produces a fundamentally different trace than a fully delegated one, and failure patterns differ by mode.

---

## 10. Incentive Design & Data Markets

**Ostrom (1990)** *Governing the Commons* -- Eight design principles for CPR.

**Vickrey (1961), Clarke (1971), Groves (1973)** -- VCG mechanism. **v6: Validated over Shapley for TC.**

**Arrow (1962)** Information as public good.

**Shapley (1953)** -- **v6: Gameability proven. The entire semivalue class (Shapley, Banzhaf, Beta) is gameable: see Agarwal et al. (arXiv:2504.05563), Blum et al. (arXiv:2605.07663, Sybil 1.74x), Hu et al. (arXiv:2506.12619). TC uses VCG instead.**

**Glickman (1999)** Glicko-2 dynamic rating.

**Meritrank (2022)** https://arxiv.org/abs/2207.09950 -- Sybil-tolerant reputation.

**Duetting et al. (2024)** "Mechanism Design for LLMs" https://arxiv.org/abs/2310.10826

**Agent Exchange (2025)** https://arxiv.org/abs/2507.03904

**Kahneman & Tversky (1979)** Prospect Theory -- Loss aversion for credit framing.

---

## 11. User Acquisition & Developer Tools

**Sweller (1988)** Cognitive Load Theory -- `tc init` < 3 decisions.

**NFX (2023)** "19 Tactics for Marketplace Cold Start"

**PostHog** "How PostHog Grows" -- 97% organic.

**Langfuse / YC W23** "Built in Public"

**Sentry** -- Sub-5-minute install. **v6: 4min 20sec to first event target.**

**Tea Protocol** https://tea.xyz/

**Community-Led Growth (2026)** https://www.idlen.io/blog/community-led-growth-developer-tools/

**TraceLab Dataset** (Zhu et al., University of Washington) https://arxiv.org/html/2606.30560v1 -- 4,265 sessions, 357K LLM steps. Workload characterization study; does NOT report a failure rate.

**cargo-dist (2026)** `[v6]` v0.32.0 -- Standard Rust CLI distribution. Used by ripgrep, bat, delta, zoxide.

---

## 12. Infrastructure

**Nygard (2007)** *Release It!*

**Gray & Reuter (1992)** *Transaction Processing*

**Kulkarni et al. (2014)** Hybrid Logical Clocks.

**Neubert et al. (2022)** "HDC for Robotics" https://arxiv.org/abs/2106.05268

**Dageville et al. (2016)** "Snowflake" SIGMOD.

**Lin et al. (2025)** `[v6-verified]` "Sleep-time Compute" https://arxiv.org/abs/2504.13171 -- (Also listed in Section 7, Agent Memory.)

---

## Conference & Venue Watchlist

**Tier 1**: NeurIPS (Dec 2026, Vancouver), ICML (Jul 2026, Vienna), ICLR (Apr-May 2027), ICSE (Apr 2026, Rio), FSE (Jul 2026, Montreal)

**Tier 2**: ACL/EMNLP, SIGMOD, VLDB, ASE (Oct 2026, Munich), KDD (Aug 2026, Jeju)

**Tier 3**: AAAI, CCS/S&P/USENIX Security, COLM, SOSP/OSDI, CSCW

**Priority workshops**: FMAI (ICML), Agents in the Wild (ICML), Lifelong Agents (COLM), SynthAI (SIGMOD), DCAI (NeurIPS)

**New (v6)**: Agent Trace Analysis Workshop (NeurIPS 2026 -- if accepted), Process Mining for AI (BPM 2026), AI Safety Measurement (AAAI 2027)

---

## Verification Summary

**Verified (arXiv URL confirmed)**: ~150 papers with direct arXiv links or known conference/book citations.

**Newly verified in v6.1**: ToolChain-CRC (2606.18467), Role-Stratified CRC (2607.24343), Conformal Agent Error Attribution (2605.06788), PASC (2605.18812), Agent Behavior Mining (2606.20669), AgentLTL (2607.02599), Governance Decay (2606.22528), Shapley gameability cluster (2504.05563, 2605.07663, 2506.12619), Zero-Replay Debugging (2606.14805), StepFinder (2606.03467), CoACT (2607.02911), SWE-MeM (2606.28434), Trace2Skill (2603.25158), AutoRefine (2601.22758), SkillAudit (2606.14239), MetaSkill-Evolve (2607.05297), ACE (2606.31564), Slipstream (2605.08580), CompactionRL (2607.05378), Q-MIA (2506.05379), RHO (2606.05922), Who&When (2505.00212).

**Newly verified in v6.2**: AgentGUI (2607.26300, 38%/p=0.023 confirmed), AgentDebugX (2607.18754, Error Hub + DeepDebug 28.8% confirmed), AgenTracer-8B (2509.03312, confirmed but ICLR 2026 status UNVERIFIED), TRAIL (2505.08638, three-domain taxonomy confirmed), Interruptible Agents (2604.00892, three interruption types confirmed), Human-Agent Collaboration Survey (2505.00753, four collaboration subtypes confirmed), Evidence Tracing Survey (2606.04990), ReasoningBank (2509.25140, title corrected), Dynamic Cheatsheet (2504.07952, title corrected), SkillOS (2605.06614), SkillRevise (2606.01139), RHO (2606.05922, title corrected to "Evolving Agents in the Dark"), LEGOMem (2510.04851), Sleep-time Compute (2504.13171), LatentMAS (2511.20639), Externalization Review (2604.08224, four-category taxonomy confirmed), AgentSpec (2606.14674), MCP/ACP/ANP/A2A Survey (2505.02279), TEE Survey (2605.03213), VET (2512.15892), Cryptographic AI Pipeline (2503.22573), Governance-as-a-Service (2508.18765, arXiv ID added), Privilege Attenuation (2602.11865, correct title "Intelligent AI Delegation").

**Misclassified in v6.2**: Inference-Time Steering (2411.16627) -- confirmed as ROBOTICS paper (ICRA 2025) about physical robot policy steering, NOT LLM agent UX. Retained in index with warning; should not be cited for LLM agent steering.

**Corrected in v6.2**: TraceLab (2606.30560) -- two errors in prior versions (v3/v5) corrected: (1) attribution was listed as "Cognition Labs" but the paper is by Kan Zhu, Baris Kasikci et al. at University of Washington; (2) a "31% failure rate" was cited but this figure does not appear in the paper. TraceLab is a workload characterization study of LLM serving demands from coding agents; it does not measure or report session failure rates. Both errors have been corrected throughout all v6 documents.

**Unverified (`[v6-unverified]`)**: ~30 papers surfaced by research agents without arXiv IDs. These include: TraceProbe, AgentLocate, TraceSIR, VCC, FailureNet, RootTrace, AgentPostMortem, DebugAgent, STRACE, PrefixGuard, PASTE, TraceGraph, AgentAtlas, SessionSense, Cauchois et al. CP, Park et al. CP, Zeni et al. CP, Feldman et al. CP, Bhatnagar et al. CP, ARC, Focus, TraceZip, ContextPrune, StreamCompress, CompressAndScore, SkillGraph, SafeSkill, SkillTransfer, ClawHavoc, AgentPetri, TraceConform, WorkflowMiner, and several data valuation papers (Chen, Fernandez, Xu DSIC, Li Myerson, Wang FL, Sim VCG, Jia amortized). Treat as provisional.

**Removed in v6.1**: SPARK ("Skill Composition from Atomic Trace Primitives") -- confirmed hallucinated by research agents; no arXiv entry, no conference proceedings, no author trail.

*~220+ papers. Last updated August 2026 (v6.2).*
