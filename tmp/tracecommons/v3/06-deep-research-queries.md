# Deep Research Queries for TraceCommons

> **Date**: 2026-08-10
> **Purpose**: Copy-paste queries for finding papers relevant to TC, plus under-explored directions and venues to watch.

## What is TraceCommons?

TraceCommons (TC) is an open-source Rust AI trace registry. Agents submit execution traces; TC scores them for quality and novelty inside TEEs (privacy-preserving), then compensates contributors with credit via NEAR. The core problems are: scoring trace quality without ground truth, detecting novelty in a shifting population, compressing traces without losing safety evidence, and designing contributor incentives that suppress junk.

## What research has already been covered

The existing corpus (~88 papers) spans: novelty detection (NCD, MinHash, perplexity), influence functions (LoGra, LogIX, Shapley), agent memory (MemGPT, Mem0, A-MEM, LEGOMem), failure attribution (AgentDebugX, AgenTracer, TRAIL, Who&When), trace formats (OTel GenAI, OpenInference), privacy (DP, ZK, TEEs, SCITT, C2PA, DIDs), agent skills (SKILL.md), code similarity (AST edit, GraphCodeBERT, CSSG), developer tools (Sentry, PostHog, Langfuse), commons governance (Ostrom, VCG, mechanism design), and process mining.

---

## 1. Research Query Templates

Each query targets Perplexity Pro, Semantic Scholar, Google Scholar, or Connected Papers. Phrased to surface 2024-2026 work.

### Q1: Trace quality prediction without ground truth

**Query:** `"trace quality prediction without ground truth labels 2025 2026"`
**Looking for:** Methods to estimate data quality without labeled training data. TC's bake-off corpus was confounded (PR #216) -- scorer comparison relied on implicitly biased labels.
**Why:** Ground-truth-free calibration (e.g., Hui-Walter paradigm, arxiv 2601.19862) could let TC bootstrap quality scoring from raw submissions alone.

### Q2: Conformal prediction for LLM agent outputs

**Query:** `"conformal prediction uncertainty quantification LLM agent outputs 2025 2026"`
**Looking for:** Conformal prediction methods providing distribution-free coverage guarantees. Prediction sets instead of point estimates.
**Why:** TC could say "this trace is in the top 15% of novelty with 95% coverage guarantee" instead of a bare score. TECP (arxiv 2509.00461) and domain-shift-aware CP (arxiv 2510.05566) apply directly.

### Q3: Causal agent replay and counterfactual attribution

**Query:** `"causal agent replay counterfactual attribution LLM failures 2025 2026"`
**Looking for:** Methods going beyond correlation to causal analysis of agent failures. The causal replay literature has exploded in mid-2026.
**Why:** TC's evidence_chain records WHAT happened but not WHY. Causal Agent Replay (arxiv 2606.08275) and CausalFlow (arxiv 2605.25338) enable "if the agent had used a different tool at step 3, would the outcome change?" annotations.

### Q4: Trajectory compression preserving safety evidence

**Query:** `"trajectory compression long-horizon agent context safety 2025 2026"`
**Looking for:** Methods that compress long agent trajectories while preserving safety-critical information. Traces can be thousands of tool calls.
**Why:** TRACE (arxiv 2606.00611) compresses trajectories while preserving risky behavior evidence. ACON and FoldAct offer complementary approaches. Potential 10-50x storage savings.

### Q5: Specification mining from execution logs

**Query:** `"specification mining temporal constraints execution logs LLM 2025"`
**Looking for:** Automated extraction of behavioral specs (ordering constraints, temporal patterns, invariants) from logs.
**Why:** TC's novelty scorer compares traces holistically. Spec mining could decompose traces into patterns like "read-modify-verify" or flag temporal constraint violations. See arxiv 2506.08628 (Logic Mining from Process Logs).

### Q6: Contrastive learning for trace embeddings

**Query:** `"contrastive learning code execution trace embeddings 2025 2026"`
**Looking for:** Contrastive pre-training for code and execution traces. TC uses HDC fingerprints for structural similarity; learned embeddings would add semantic similarity.
**Why:** Two traces solving the same problem differently get different HDC fingerprints but should cluster semantically. Hard negative mining (arxiv 2509.24291) is especially relevant since TC traces are often superficially similar but functionally different.

### Q7: Data mixture optimization

**Query:** `"data mixture optimization training data curation LLM 2025 2026"`
**Looking for:** Methods for optimizing what mix of training data produces the best outcomes. TC is a data curation system that must score marginal value.
**Why:** CausalMix (arxiv 2607.01104) frames marginal value as causal inference. DUET (arxiv 2502.00270) optimizes for unseen downstream tasks. Exactly TC's question: "given 10K traces, is trace 10,001 worth adding?"

### Q8: Model diffing for behavioral comparison

**Query:** `"model diffing behavioral comparison LLM auditing 2025 2026"`
**Looking for:** Systematic behavioral comparison of LLM-based systems. How traces from Claude, GPT, Gemini, and open-source differ.
**Why:** TC could annotate "this trace demonstrates a pattern unique to Claude-family agents." LessWrong's model-diffing work shows this is tractable with scaffolded LLM auditors.

### Q9: Concept drift detection for evolving trace populations

**Query:** `"concept drift detection production LLM systems evolving distributions 2025 2026"`
**Looking for:** Methods for detecting when statistical properties of incoming data change. TC's population shifts as new models and frameworks emerge.
**Why:** When a new model family appears, old calibrations go stale. Drift detection (PSI, KS test, Wasserstein distance) triggers automatic recalibration. 2026 production ML literature treats drift as a system-level metric.

### Q10: Synthetic trajectory generation

**Query:** `"synthetic trajectory generation LLM agent training data augmentation 2025"`
**Looking for:** Methods for generating synthetic agent trajectories to augment sparse real data.
**Why:** Open-SWE-Traces (arxiv 2606.16038) released 200K+ SE agent trajectories across 9 languages -- a potential seeding corpus for TC. GenEnv (arxiv 2512.19682) co-evolves agents and simulators for diverse synthetic trajectories.

### Q11: Decentralized data marketplace incentives

**Query:** `"decentralized data marketplace incentive mechanism data quality 2025 2026"`
**Looking for:** Incentive mechanism designs for decentralized data-sharing platforms. TC is a data marketplace; getting incentives right is existential.
**Why:** TC uses VCG-inspired value attribution, but the literature (e.g., arxiv 2512.10372) has evolved to include staking, quality-weighted rewards, and RL-based supply/demand balancing.

### Q12: ML pipeline provenance and lineage

**Query:** `"provenance tracking ML pipeline lineage attestation 2025"`
**Looking for:** Systems tracking full provenance of ML artifacts from data through transformations to outputs.
**Why:** Atlas (arxiv 2502.19567) and OpenLineage track provenance outside TC -- connecting TC traces to upstream pipelines and downstream training runs. Enables broader ML supply chain verification.

### Q13: Self-supervised learning on structured/tabular data

**Query:** `"self-supervised learning structured tabular data pretext tasks 2025"`
**Looking for:** Self-supervised pre-training for tabular data. Agent traces are structured (tool_name, parameters, duration, token_count, error_code), not free text.
**Why:** Tabular methods (T-JEPA, TST-LLM) could learn representations respecting trace structure, potentially outperforming text-based embeddings for novelty detection.

### Q14: Implicit execution tracing from incomplete outputs

**Query:** `"implicit execution tracing multi-agent attribution provenance 2025 2026"`
**Looking for:** Methods for reconstructing execution provenance from incomplete traces. Not all frameworks produce structured OTel-level traces.
**Why:** "When Only the Final Text Survives" (arxiv 2603.17445) shows how to reconstruct provenance from minimal outputs. Lets TC accept low-fidelity traces, expanding the contributor base.

### Q15: Active learning for annotation budgets

**Query:** `"active learning efficient annotation LLM data labeling 2025 2026"`
**Looking for:** Methods for choosing which items to label first for maximum information gain. TC plans human annotation (PR #173).
**Why:** ACL 2025 survey (arxiv 2502.11767) shows 93%+ performance at ~6% annotation cost. DALL (arxiv 2602.14102) combines data programming with active learning, mapping to TC's automated scoring + human validation mix.

---

## 2. Under-Explored Research Directions

Areas TC hasn't investigated yet but could yield practical improvements. Each includes a search query and anchor papers.

### 2.1 Federated Learning for Privacy-Preserving Scorer Updates

Train/update TC's scorers across distributed contributors without centralizing raw traces. Adds a fourth privacy layer beyond DP/redaction/TEEs: traces never leave contributor infrastructure.

**Query:** `"federated learning" "model updates" privacy "data quality" scoring distributed 2025 2026`
**Anchors:** FedAvg (McMahan 2017), survey arxiv 2504.17703

### 2.2 Causal Inference for Trace-Outcome Attribution

Use do-calculus and counterfactual reasoning to determine which trace patterns CAUSE good outcomes, not just correlate. Enables "traces with pattern X lead to 15% better completion."

**Query:** `"causal inference" "treatment effect" agent trace "counterfactual" tool selection 2025 2026`
**Anchors:** CausalFlow (arxiv 2605.25338), Causal Agent Replay (arxiv 2606.08275)

### 2.3 Graph Neural Networks for Tool-Call Sequences

Agent traces have branching, loops, parallel calls, and complex dependencies -- not linear sequences. GNNs naturally model this structure for improved novelty detection.

**Query:** `"graph neural network" "tool call" sequence "agent trajectory" representation 2025 2026`
**Anchors:** GraphTracer (2026), NESTFUL (IBM Research)

### 2.4 Reward Modeling from Human Preferences

Apply RLHF-style pairwise comparison ("which trace is more useful?") instead of absolute scoring. Bradley-Terry model trained on preferences avoids confounds from the bake-off (PR #216).

**Query:** `"reward model" "pairwise preference" data quality scoring "Bradley-Terry" 2025 2026`
**Anchors:** Nathan Lambert's RLHF Book (Ch 5), DynaCF (arxiv 2606.09043), ConsistRM (arxiv 2604.07484)

### 2.5 Online Anomaly Detection for Streaming Intake

Detect anomalous traces at submission time with O(1) per-trace cost. CALIBURN (arxiv 2605.24696) adds conformal risk control to streaming detection.

**Query:** `"online anomaly detection" "streaming data" "concept drift" "random cut forest" 2025 2026`
**Anchors:** CALIBURN (arxiv 2605.24696), MINAS, Online Isolation Forest

### 2.6 Curriculum Learning for Progressive Trace Difficulty

Present traces to consumers in learning-optimal order. SPaCe (arxiv 2508.05015) demonstrates self-paced curriculum; SPARD (arxiv 2604.07837) integrates reward dynamics.

**Query:** `"curriculum learning" "difficulty progression" data curation training order LLM 2025 2026`
**Anchors:** SPaCe (arxiv 2508.05015), SPARD (arxiv 2604.07837)

### 2.7 RAG from Trace Corpora

Use TC's registry as a retrieval source: an agent queries "show me traces from agents that solved similar problems." Harder than text RAG -- traces are structured and multi-modal.

**Query:** `"retrieval augmented generation" "execution traces" agent behavior "code retrieval" 2025 2026`
**Anchors:** cAST (arxiv 2506.15655), evidence tracing (arxiv 2606.04990)

### 2.8 Data Mixture Marginal Value

Not "is this trace good?" but "does it improve the mix?" DoReMi, REGMIX, CausalMix (arxiv 2607.01104), and D3 (arxiv 2605.31164) formalize this as optimization.

**Query:** `"data mixture" "marginal value" optimization corpus composition training 2025 2026`
**Anchors:** DoReMi (2023), CausalMix (arxiv 2607.01104), D3 (arxiv 2605.31164)

### 2.9 Multi-Modal Trace Analysis

Treat trace components (code, NL reasoning, tool metadata, timing, errors) as separate modalities. Multi-modal fusion captures cross-modal patterns invisible to any single modality.

**Query:** `"multi-modal" "agent traces" code "natural language" "tool calls" fusion representation 2025 2026`
**Anchors:** Open-SWE-Traces (arxiv 2606.16038)

### 2.10 Tighter DP Composition for Multi-Contributor Queries

Concentrated DP, Renyi DP, and Gaussian DP give tighter composition bounds than naive sequential composition. Same privacy guarantees, more utility -- free improvement.

**Query:** `"differential privacy composition" tight bounds "concentrated DP" "Renyi DP" "privacy budget" 2025 2026`
**Anchors:** Bun & Dwork concentrated DP, Mironov Renyi DP, Google DP Accounting library

---

## 3. Conference & Venue Watchlist

### Tier 1: Check every proceedings

| Venue | Next Date | TC Relevance |
|-------|-----------|--------------|
| **NeurIPS** | Dec 2026 (Vancouver) | Agents, safety, data-centric AI workshops. Datasets & Benchmarks track. |
| **ICML** | Jul 2026 (Vienna) | Failure Modes in Agentic AI (FMAI), Agents in the Wild, Mechanistic Interp workshops. |
| **ICLR** | Apr-May 2027 (TBD) | Representation learning, contrastive methods, embeddings. |
| **ICSE** | Apr 2026 (Rio de Janeiro) | SE trace analysis, debugging, AI4SE / SE4AI workshops. |
| **FSE** | Jul 2026 (Montreal) | Agent-assisted development, trace-based debugging. |

### Tier 2: Check relevant tracks/workshops

| Venue | Next Date | TC Relevance |
|-------|-----------|--------------|
| **ACL / EMNLP** | Jul / Nov 2026 | Agent evaluation, code generation, trace analysis. |
| **SIGMOD** | May-Jun 2026 (Bengaluru) | SynthAI, aiDM workshops. Data curation. |
| **VLDB** | Aug 2026 (TBD) | Scalable data management, query processing, data quality. |
| **ASE** | Oct 2026 (Munich) | Agent-driven development, automated testing. |
| **KDD** | Aug 2026 (Jeju) | Anomaly detection, pattern mining, applied data science. |

### Tier 3: Scan for specific topics

| Venue | Next Date | TC Relevance |
|-------|-----------|--------------|
| **AAAI** | Jan 2026 (Singapore) | Causal reasoning, planning, multi-agent systems. |
| **CCS / S&P / USENIX Security** | Various 2026 | DP advances, TEE research, supply chain security. |
| **COLM** | 2026 (TBD) | Lifelong Agents workshop: traces, observability, long-horizon. |
| **SOSP / OSDI** | Oct 2026 | Distributed systems, telemetry infrastructure. |
| **CSCW** | 2026 (TBD) | Commons governance, incentive design. |

---

## 4. Practical Search Strategy

- **Start from anchor papers.** Use Connected Papers (connectedpapers.com) seeded from: "Revisiting Code Similarity" (Song 2024), "Causal Agent Replay" (arxiv 2606.08275), "TRACE" (arxiv 2606.00611), "From Agent Traces to Trust" (arxiv 2606.04990), "Open-SWE-Traces" (arxiv 2606.16038), "CausalMix" (arxiv 2607.01104). Two hops from each anchor yields 40-60 papers keyword search misses.
- **Use Semantic Scholar's "Highly Influential Citations."** Find a known paper, click Citations, filter by "Highly Influential," sort by date. Filters out 90% of passing mentions.
- **Set arXiv alerts** for: cs.CL (agents, trace analysis), cs.SE (debugging, testing), cs.AI (multi-agent, planning), cs.CR (DP, TEEs), cs.DB (data quality, lineage), cs.LG (representation learning, anomaly detection), cs.IR (similarity, retrieval). Use arxiv-sanity-lite or Semantic Scholar Research Feed.
- **Prefer workshop papers** over main-track at ML conferences. They are more practical, more recent, and more honest about limitations. Priority workshops: FMAI (ICML), Agents in the Wild (ICML), Lifelong Agents (COLM), SynthAI (SIGMOD), DCAI (NeurIPS).
- **Watch GitHub trending** (weekly, Rust + Python). Look for agent observability frameworks, trace processing libraries, DP tools, embedding/similarity implementations. Key repos: `yzhao062/anomaly-detection-resources`, `pengr/LLM-Synthetic-Data`, `tmgthb/Autonomous-Agents`.
- **Cross-pollinate from non-obvious domains:**

| TC Problem | Non-Obvious Domain | Query |
|-----------|-------------------|-------|
| Trace novelty | Network intrusion detection | `"network intrusion detection" novelty streaming 2025` |
| Contributor incentives | Prediction markets | `"prediction market" "information aggregation" incentive 2025` |
| Trace compression | Video summarization | `"keyframe extraction" "temporal summarization" importance 2025` |
| Multi-contributor queries | Secure MPC | `"secure aggregation" "multi-party" statistics privacy 2025` |
| Pattern vocabulary | Program synthesis | `"program synthesis" "library learning" abstraction 2025` |
| Trace deduplication | Entity resolution | `"entity resolution" "record linkage" scalable 2025` |

---

## 5. Quick-Start: Five Searches to Run Today

Highest-payoff queries targeting the biggest gaps in TC's current knowledge:

1. **Conformal prediction for LLM outputs** -- calibrated confidence intervals on novelty scores.
   `"conformal prediction" LLM uncertainty "coverage guarantee" 2025 2026`
2. **Trajectory compression with safety preservation** -- 10-50x storage savings.
   `"trajectory compression" "safety-aware" agent "long-horizon" 2025 2026`
3. **Data mixture marginal value** -- core algorithm for TC's value attribution.
   `"data mixture" "marginal contribution" training optimization 2025 2026`
4. **Active learning for annotation budgets** -- makes PR #173 10x more efficient.
   `"active learning" annotation "information gain" "label budget" LLM 2025`
5. **Specification mining from execution logs** -- decompose traces into reusable patterns.
   `"specification mining" "temporal logic" "execution traces" patterns 2025`
