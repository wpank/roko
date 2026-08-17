# Deep Research Queries v3

**Date**: August 2026

## What This Document Is

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces. TC scores traces for quality and novelty inside TEEs (Trusted Execution Environments) and compensates contributors via NEAR blockchain credits. This document contains copy-paste-ready queries for deep research tools (Perplexity Pro, Google Scholar, Semantic Scholar, Connected Papers). Each query includes what to look for, why it matters for TC, and what we already know from prior research.

The prior research corpus (~220+ papers) spans: novelty detection, agent failure attribution, conformal prediction, trajectory compression, skill extraction, process mining, data valuation, privacy/TEE, agent protocols (OTel, MCP, A2A), commons governance, and developer tools.

37 queries across 7 categories, generated from gaps identified by the 27-agent research sweep.

---

## Category 1: Scoring & Quality (8 queries)

### Q-S1: Conformal Prediction for Agent Quality

```
"conformal prediction" "agent" OR "LLM" quality scoring calibration coverage guarantee 2025 2026
```
**Looking for:** Production applications of conformal prediction to AI agent quality scoring. ToolChain-CRC and related methods. How do they handle exchangeability violations when contributor populations shift? What coverage guarantees are achievable with ~350-1000 calibration traces? TC needs to ship "92/100, 95% coverage" not just "92/100."

**What we already know:** ToolChain-CRC (arXiv:2606.18467) is directly applicable to TC's pipeline. Role-Stratified CRC (2607.24343) handles heterogeneous contributor populations. PASC (2605.18812) provides another angle. The finite-sample formula gives at most 0.33% overshoot at n=300, meaning 300-1000 calibration traces is sufficient for production-grade guarantees. No production deployment exists yet for trace quality scoring -- TC would be the first.

### Q-S2: Incentive-Compatible Data Pricing (Not Shapley)

```
"data valuation" "incentive compatible" OR "DSIC" OR "VCG" OR "Myerson" marketplace production deployment 2025 2026
```
**Looking for:** Has anyone deployed VCG or Myerson mechanisms for data marketplace pricing in production? What are the practical challenges? How do they handle the O(n^2) scaling? TC has `vcg_allocate` built but needs production patterns for approximation and deployment.

**What we already know:** Shapley gameability is definitively proven and TC must avoid it. arXiv:2504.05563 (Claim 3) shows formal gameability. arXiv:2605.07663 demonstrates Sybil attacks yielding 1.74x unfair payoff. arXiv:2506.12619 proves the entire semivalue class is gameable, not just Shapley. VCG is O(n log n) for homogeneous multi-unit auctions, which fits TC's trace batches. No production VCG deployment for data pricing exists. Q-MIA (2506.05379) offers a budget-balanced alternative worth investigating.

### Q-S3: Process Mining for Agent Traces

```
"process mining" "AI agent" OR "LLM agent" trace analysis "directly-follows graph" OR "petri net" conformance 2025 2026
```
**Looking for:** Applications of classical process mining to AI agent traces. STRACE, PrefixGuard, Agent Behavior Mining -- what other tools and libraries exist? What false-positive rates are typical for conformance-based novelty detection? Can process mining handle the variability of agent traces (unlike structured business processes)?

**What we already know:** Agent Behavior Mining (arXiv:2606.20669, BPM 2026) is the key paper for this space. AgentLTL (2607.02599) enables LTL verification of agent traces. PM4Py has an LLM module. Empirically, approximately 60% of sessions follow 5-7 canonical patterns. Fitness thresholding alone fails as an anomaly detector -- it is too coarse for trace populations with high variability.

### Q-S4: Causal Attribution Without Re-Execution

```
"causal" "failure attribution" agent trace "without re-execution" OR "offline" OR "counterfactual" 2025 2026
```
**Looking for:** Causal failure attribution methods that work on logged traces without re-executing the agent. CAR requires re-execution (expensive). Are there methods that estimate causal effects from observational data alone? What accuracy tradeoffs exist? TC processes historical traces -- re-execution may not be possible.

**What we already know:** CAR (2606.08275), CausalFlow (2605.25338), and AgenTracer-8B (2509.03312) are all verified. Zero-Replay Debugging (2606.14805) achieves Branch Recall@5 of 0.93 with zero LLM calls, making it the most practical approach for TC's offline pipeline. Correlational methods achieve only approximately 14% accuracy, confirming that causal methods are necessary, not optional.

### Q-S5: Joint Compression and Quality Scoring

```
"trajectory compression" "quality" OR "scoring" OR "evaluation" joint OR simultaneous agent 2025 2026
```
**Looking for:** Methods that compress traces AND score them in a single pass. What's the quality loss from scoring compressed vs. full traces? Can TC's pipeline compress-then-score without information loss on quality-relevant features?

**What we already know:** TRACE (2606.00611), ACE (2606.31564, SambaNova open-source), Slipstream (2605.08580, +8.8pp accuracy with -39.7% latency), and CompactionRL (2607.05378) are all verified. Critical safety finding: Governance Decay (2606.22528) shows that naive compaction causes safety violation rates to jump from 0% to 30-59%. Constraint Pinning (approximately 47 tokens overhead) restores 0% violation rate. TC must never compress without pinning safety-relevant spans.

### Q-S6: Trace Quality Prediction Without Ground Truth

```
"trace quality prediction" OR "data quality estimation" "without ground truth" OR "unsupervised" labels 2025 2026
```
**Looking for:** Methods to estimate data quality without labeled training data. TC's bake-off corpus was confounded (PR #216) -- scorer comparison relied on implicitly biased labels. Hui-Walter paradigm, agreement-based quality estimation, and proxy-based scoring.

**Why:** Ground-truth-free calibration lets TC bootstrap quality scoring from raw submissions alone, removing the chicken-and-egg problem of needing labeled traces to train the labeler.

### Q-S7: Contrastive Learning for Trace Embeddings

```
"contrastive learning" code execution trace embeddings representation 2025 2026
```
**Looking for:** Contrastive pre-training for code and execution traces. TC uses HDC fingerprints for structural similarity; learned embeddings would add semantic similarity. Hard negative mining where traces are superficially similar but functionally different.

**Why:** Two traces solving the same problem differently get different HDC fingerprints but should cluster semantically. Hard negative mining is especially relevant since TC traces are often superficially similar but functionally different.

**Citation correction (v6):** arXiv:2509.24291 was previously cited here as "hard-negative mining contrastive." The actual paper at that ID is **GIRCSE** (generative contrastive sentence embeddings) -- a different paper. The hard-negative mining claim is UNVERIFIED; the correct source is still needed.

### Q-S8: Concept Drift Detection for Evolving Trace Populations

```
"concept drift detection" production LLM systems "evolving distributions" OR "population shift" 2025 2026
```
**Looking for:** Methods for detecting when statistical properties of incoming data change. PSI, KS test, Wasserstein distance applied to production ML. Automatic recalibration triggers.

**Why:** TC's contributor population shifts as new models and frameworks emerge. When a new model family appears (e.g., Claude 4 vs. Claude 3.5), old calibrations go stale. Drift detection triggers automatic recalibration of quality and novelty scorers without manual intervention.

---

## Category 2: Integrations & Ecosystem (5 queries)

### Q-I1: OTel GenAI Convention Stability

```
"OpenTelemetry" "gen_ai" OR "generative AI" semantic conventions stability roadmap breaking changes 2026
```
**Looking for:** Timeline for gen_ai.* conventions reaching Stable status. What breaking changes are in the pipeline? How are early adopters (Langfuse ClickHouse, Datadog, Arize) handling instability internally? What's the recommended version pinning strategy? TC must build on OTel without being broken by schema changes.

**What we already know:** All gen_ai.* conventions are at Development status. The conventions moved to a dedicated repo on June 12, 2026. The gen_ai.system to gen_ai.provider.name rename is the most dangerous pending change for existing integrations. OpenInference (Arize/Phoenix) is a parallel, competing convention set.

### Q-I2: Claude Code Hook Integration Patterns

```
"Claude Code" hooks OR extensions OR plugins "post-session" OR "session end" integration telemetry 2026
```
**Looking for:** How are third-party tools integrating with Claude Code's hook system? Are there existing examples of post-session hooks for analytics or telemetry? What are the performance constraints (must complete in <N seconds)? Are there best practices for non-blocking hooks?

**What we already know:** 30 hook events are confirmed. SessionEnd has a 1.5s default timeout, which constrains what post-session hooks can do synchronously. Third-party examples exist: opentelemetry-hooks, claude_telemetry, and a Langfuse hook. TC's hook must either complete within 1.5s or defer work to a background daemon.

### Q-I3: Agent Skills Security & Trust Registries

```
"agent skills" OR "SKILL.md" security trust registry verification provenance ClawHavoc 2026
```
**Looking for:** Post-ClawHavoc security landscape. How are registries responding to the 341 malicious skills discovery? Are new trust mechanisms being developed? What makes SkillSieve/SkillSpector bypassable and what would make a scanner robust? Is there demand for a "verified skills" tier backed by provenance?

**What we already know:** ClawHavoc is verified (341 malicious skills, 300K+ affected). SkillSieve achieves F1=0.920. SkillSpector by NVIDIA covers 64 patterns. However, Trail of Bits bypassed all scanners in under 1 hour. NVIDIA Verified Agent Skills has 162 signed skills. OWASP AST10 is published. The bypass result means static analysis alone is insufficient -- behavioral analysis from actual execution traces (what TC collects) may be the missing layer.

### Q-I4: Cross-Agent Session Formats

```
"Claude Code" OR "Codex" OR "Cursor" OR "Copilot" session format log structure export API 2026
```
**Looking for:** Detailed session/log formats for major AI coding agents. Can TC parse these natively? Which agents expose session data via API vs. local files vs. neither? This determines which integrations are possible without OTel.

### Q-I5: A2A Observability & Multi-Agent Tracing

```
"A2A protocol" OR "agent-to-agent" observability tracing delegation monitoring distributed 2026
```
**Looking for:** How are A2A multi-agent systems being observed? When Agent A delegates to Agent B, what metadata is captured? Is there a standard for multi-agent trace correlation beyond W3C traceparent? TC needs to ingest cross-agent traces.

**What we already know:** A2A reached v1.0.0 with 150+ participating organizations. A Traceability Extension exists as a sample implementation. The AAIF Observability WG covers cross-protocol trace propagation. TC should align with the Traceability Extension rather than inventing a proprietary correlation scheme.

---

## Category 3: Growth & Distribution (7 queries)

### Q-G1: Developer CLI Auto-Update Mechanisms

```
"CLI tool" auto-update OR self-update mechanism developer "cargo-dist" OR "homebrew" 2025 2026
```
**Looking for:** How do developer CLI tools handle auto-updates? cargo-dist generates installers but what about updates? Does Homebrew tap auto-update? Are there patterns for "update available" notifications in CLIs? TC needs the first install to be <90 seconds AND subsequent updates to be zero-friction.

### Q-G2: Agent Failure Pattern Communities

```
"AI agent" OR "LLM" failure pattern community forum database debugging shared 2026
```
**Looking for:** Where do developers currently discuss AI agent failures? Reddit? Discord servers? GitHub Discussions? X threads? Stack Overflow tags? Understanding the current gathering places for failure debugging reveals the distribution channel for TC's Error Hub. If there's no centralized place, the Error Hub fills a real gap.

**What we already know:** r/ClaudeAI has 1M+ subscribers. Claude Code averages 291 issues/day on GitHub. Mozilla cq has 1,200 stars. Stack Overflow for Agents launched a beta in June 2026. No session-trace-first failure database exists anywhere. TC's Error Hub would be the first tool where developers search failures by trace structure rather than error message text.

### Q-G3: Open-Source Developer Tool Viral Mechanics

```
"open source" developer tool "viral" OR "product-led growth" artifact badge embed 2025 2026
```
**Looking for:** What artifacts do successful open-source developer tools produce that circulate virally? Sentry error pages, PostHog session replay links, Vercel deploy buttons, coverage badges. What's TC's equivalent? Skill files with provenance? Failure bundle links? Cost comparison screenshots with watermarks?

### Q-G4: Background Daemon Opt-In Best Practices

```
"background daemon" OR "background service" developer tool opt-in telemetry privacy 2025 2026
```
**Looking for:** How do developer tools that run background processes handle opt-in? Docker Desktop, VS Code telemetry, Homebrew analytics, npm audit. What opt-in rates do different framings achieve? TC's daemon (PR #244) needs the right opt-in UX.

**What we already know:** Go telemetry aimed for 10-20% opt-in rate and largely achieved it. Mission-first framing ("help improve the tool for everyone") outperforms privacy-first framing. GitHub CLI has the best payload transparency -- users can inspect exactly what is sent. TC should target a 5% initial contribution rate and grow from there.

### Q-G5: Cross-Tool AI Coding Comparison Demand

```
"Claude Code" vs "Cursor" vs "Codex" comparison benchmark developer demand 2026
```
**Looking for:** How hot is the "which AI coding tool is best?" discourse? What data do developers want that doesn't exist? If TC's cross-harness corpus can answer "Claude Code is 23% more cost-efficient for debugging tasks," is there a viral distribution channel for that insight?

### Q-G6: Synthetic Trajectory Generation for Corpus Seeding

```
"synthetic trajectory generation" LLM agent "training data" augmentation OR seeding 2025 2026
```
**Looking for:** Methods for generating synthetic agent trajectories to augment sparse real data. Open-SWE-Traces (arXiv:2606.16038) released 200K+ SE agent trajectories across 9 languages. GenEnv (2512.19682) co-evolves agents and simulators for diverse synthetic trajectories.

**Why:** TC faces a cold-start problem -- the registry needs traces to attract contributors, but contributors need a populated registry to see value. Synthetic seeding with verified trajectories bootstraps the corpus while real contributions ramp up.

### Q-G7: Developer Tool Community Building from Trace Data

```
"developer community" "open source" "shared debugging" OR "failure database" OR "error registry" growth 2025 2026
```
**Looking for:** How have projects like Sentry, Mozilla cq, and error-tracking platforms built communities around shared failure data? What community features drive retention vs. one-time use? TC needs contributors who come back, not just one-off uploads.

---

## Category 4: Strategy & Market (5 queries)

### Q-M1: GPAI Compliance Requirements (Live Now)

```
"GPAI" "general purpose AI" compliance requirements obligations transparency 2026 "EU AI Act"
```
**Looking for:** Specific GPAI provider compliance obligations that took effect August 2, 2026. What must providers do? What tools are they using? Is there a market for GPAI compliance infrastructure specifically (not just general AI Act compliance)? TC should position around LIVE obligations, not deferred ones.

**What we already know:** GPAI enforcement went live August 2, 2026. The Digital Omnibus defers Article 12 standalone obligations to December 2, 2027. The compliance market is estimated at EUR 7.6-38B by 2030. TC should focus on the obligations that are enforceable now, not the deferred ones.

### Q-M2: AI Compliance Market Landscape

```
"AI compliance" platform market "EU AI Act" OR "AI governance" Holistic AI OR "Credo AI" OR TrustArc pricing 2026
```
**Looking for:** Detailed competitive landscape for AI compliance platforms. What do they charge? What do they do? Where are the gaps that an open-source alternative fills? TC's "open-source compliance" positioning is stronger with specific competitor pricing data.

### Q-M3: NLnet Application Success Patterns

```
NLnet "NGI Zero" OR "Restack" application tips funded projects privacy AI 2025 2026
```
**Looking for:** What makes NLnet applications successful? Are there public examples of funded applications? What do reviewers look for? NLnet funds TC's category (Provability Fabric precedent) -- what framing and milestones resonated?

**What we already know:** Provability Fabric is a verified precedent for TC-category funding. The next relevant call is Restack, opening September 3 (not Commons Fund as previously assumed). Scoring weights: Relevance 40%, Technical feasibility 30%, Value/impact 30%. TC should emphasize the privacy-preserving angle and open-source commitment.

### Q-M4: Agent Quality Standards Emerging

```
"agent quality" standard benchmark certification evaluation framework 2026
```
**Looking for:** Are industry consortia or standards bodies developing agent quality standards? Is there a "safety certification for AI agents" initiative? Could TC's scoring methodology become an input to or implementation of an emerging standard? Positioning TC as "the scoring infrastructure for [emerging standard]" is powerful.

### Q-M5: Privacy-Preserving Data Sharing Market

```
"privacy preserving" "data sharing" OR "data commons" TEE OR "trusted execution" marketplace platform 2025 2026
```
**Looking for:** How are platforms like Vana DLP, Ocean Protocol, OPAL doing in 2026? What worked and what didn't? What can TC learn from their go-to-market? Are TEEs gaining traction as the standard privacy mechanism for data sharing? What's the competitive and collaborative landscape?

---

## Category 5: New Technical Frontiers (5 queries)

### Q-T1: Streaming Agent Trace Analysis

```
"streaming" OR "real-time" agent trace analysis processing online 2025 2026
```
**Looking for:** Methods for analyzing agent traces in real-time as they're generated, not post-hoc. Can quality scoring happen during the session? Can novelty be estimated from a partial trace? This enables "live quality feedback" -- showing contributors their quality score as the session progresses.

### Q-T2: Multi-Modal Agent Trace Analysis

```
"multi-modal" agent trace screenshot OR visual OR GUI interaction analysis 2025 2026
```
**Looking for:** As agents increasingly interact with GUIs (web browsing, app interaction), traces include screenshots and visual elements. How are multi-modal traces being analyzed? Can TC's scoring pipeline handle visual trace components? This is relevant for computer-use agents.

### Q-T3: Agent Trace Provenance Verification

```
"agent trace" provenance verification authenticity "proof of execution" 2025 2026
```
**Looking for:** How can TC verify that a submitted trace was actually produced by a real agent execution and not fabricated? VET (Verifiable Execution Traces) is one approach. Are there lighter-weight alternatives? This prevents Sybil attacks on the credit mechanism.

### Q-T4: Sleep-Time Trace Processing

```
"sleep-time compute" OR "idle-time processing" OR "off-peak" pre-computation agent 2025 2026
```
**Looking for:** Beyond Lin et al.'s sleep-time compute paper -- what can TC pre-compute during idle windows? Batch scoring, dedup index compaction, skill extraction, cross-submission similarity -- what's the priority for off-peak processing? Are there production patterns for managing idle-time workloads in single-instance systems?

### Q-T5: Skill Extraction from Agent Traces

```
"skill extraction" OR "skill discovery" agent traces "reusable" OR "transferable" patterns 2025 2026
```
**Looking for:** Methods for extracting reusable skills or behavioral patterns from raw agent execution traces. How to go from "agent did X, Y, Z" to "this is the debug-a-type-error skill."

**What we already know:** RHO (2606.05922, verified) improves skill extraction from 59% to 78%. Trace2Skill (2603.25158) achieves +57.65pp improvement. SkillAudit (2606.14239) provides audit-grade verification. Note: SPARK is not a real system -- do not reference it in any TC materials.

---

## Category 6: Under-Explored Directions (6 queries)

Areas TC has not investigated yet but could yield practical improvements. Each includes a search query and anchor papers.

### Q-U1: Federated Learning for Privacy-Preserving Scorer Updates

```
"federated learning" "model updates" privacy "data quality" scoring distributed 2025 2026
```
**Looking for:** Train/update TC's scorers across distributed contributors without centralizing raw traces. Adds a fourth privacy layer beyond DP/redaction/TEEs: traces never leave contributor infrastructure.

**Anchors:** FedAvg (McMahan 2017). Key open question: can federated averaging work for TC's heterogeneous scorer ensemble, or does non-IID contributor data cause convergence failures?

**Citation correction (v6):** arXiv:2504.17703 was previously cited here as a "federated learning survey." That paper has been **WITHDRAWN** due to disputed authorship. Do not cite. The federated learning anchor remains FedAvg (McMahan 2017).

### Q-U2: Graph Neural Networks for Tool-Call Sequences

```
"graph neural network" "tool call" sequence "agent trajectory" representation 2025 2026
```
**Looking for:** Agent traces have branching, loops, parallel calls, and complex dependencies -- not linear sequences. GNNs naturally model this structure for improved novelty detection and pattern discovery.

**Anchors:** GraphTracer (2026), NESTFUL (IBM Research). TC currently treats traces as linear sequences for HDC fingerprinting; GNN-based representations could capture structural patterns (e.g., retry loops, parallel tool calls) invisible to sequential methods.

### Q-U3: Reward Modeling from Pairwise Preferences

```
"reward model" "pairwise preference" data quality scoring "Bradley-Terry" 2025 2026
```
**Looking for:** Apply RLHF-style pairwise comparison ("which trace is more useful?") instead of absolute scoring. Bradley-Terry model trained on preferences avoids confounds from TC's bake-off (PR #216).

**Anchors:** Nathan Lambert's RLHF Book (Ch 5), DynaCF (arXiv:2606.09043), ConsistRM (2604.07484). Pairwise comparison is cognitively easier for human annotators than absolute scoring, potentially improving annotation quality while reducing annotator fatigue.

### Q-U4: Online Anomaly Detection for Streaming Intake

```
"online anomaly detection" "streaming data" "concept drift" "random cut forest" 2025 2026
```
**Looking for:** Detect anomalous traces at submission time with O(1) per-trace cost. Streaming anomaly detection that adapts to concept drift without full retraining.

**Anchors:** CALIBURN (arXiv:2605.24696) adds conformal risk control to streaming detection. MINAS and Online Isolation Forest are established baselines. TC's intake pipeline currently scores in batch -- streaming detection would enable immediate contributor feedback and junk rejection at the gate.

### Q-U5: RAG from Trace Corpora

```
"retrieval augmented generation" "execution traces" agent behavior "code retrieval" 2025 2026
```
**Looking for:** Use TC's registry as a retrieval source: an agent queries "show me traces from agents that solved similar problems." Harder than text RAG because traces are structured and multi-modal (code, NL reasoning, tool metadata, timing).

**Anchors:** cAST (arXiv:2506.15655), evidence tracing (2606.04990). This is a potential killer feature for TC -- instead of just storing and scoring traces, TC becomes a retrieval backend that makes agents better at solving problems they have not seen before.

### Q-U6: Active Learning for Annotation Budgets

```
"active learning" "efficient annotation" LLM "data labeling" "information gain" 2025 2026
```
**Looking for:** Methods for choosing which traces to label first for maximum information gain. TC plans human annotation (PR #173) but cannot afford to label everything.

**Why:** ACL 2025 survey (arXiv:2502.11767) shows 93%+ performance at approximately 6% annotation cost. DALL (2602.14102) combines data programming with active learning, mapping to TC's automated scoring + human validation mix. This makes PR #173's annotation pipeline 10x more cost-effective.

---

## Category 7: Cross-Domain Inspiration (6 queries)

Non-obvious domains whose methods transfer directly to TC's problems. The best ideas often come from fields that solved the same abstract problem in a different context.

### Q-X1: Network Intrusion Detection for Trace Novelty

```
"network intrusion detection" novelty streaming "zero-day" detection 2025 2026
```
**Why for TC:** Network IDS has decades of experience detecting novel patterns in structured event streams -- exactly TC's trace novelty problem. Methods like streaming autoencoders, isolation forests, and hierarchical temporal memory transfer directly.

### Q-X2: Prediction Markets for Contributor Incentives

```
"prediction market" "information aggregation" incentive mechanism design 2025 2026
```
**Why for TC:** Prediction markets solve the same problem as TC's credit mechanism: eliciting truthful information from self-interested participants. Proper scoring rules (logarithmic, Brier) and market scoring rules could replace or augment VCG for trace valuation.

### Q-X3: Video Summarization for Trace Compression

```
"keyframe extraction" "temporal summarization" importance scoring video 2025 2026
```
**Why for TC:** Video summarization selects representative frames from a temporal sequence -- analogous to selecting representative steps from a long agent trace. Importance scoring, diversity-aware selection, and temporal coverage guarantees all apply.

### Q-X4: Secure MPC for Multi-Contributor Queries

```
"secure aggregation" "multi-party computation" statistics privacy "function secret sharing" 2025 2026
```
**Why for TC:** When TC computes aggregate statistics across multiple contributors' traces, MPC techniques can ensure no individual trace is exposed. Lighter-weight than full TEE processing for simple aggregations.

### Q-X5: Program Synthesis for Pattern Vocabulary

```
"program synthesis" "library learning" abstraction "DreamCoder" 2025 2026
```
**Why for TC:** DreamCoder-style library learning discovers reusable abstractions from examples. Applied to traces, it could automatically discover the "pattern vocabulary" -- the set of reusable behavioral building blocks that compose into full agent sessions.

### Q-X6: Entity Resolution for Trace Deduplication

```
"entity resolution" "record linkage" scalable "blocking" "approximate matching" 2025 2026
```
**Why for TC:** Trace deduplication is structurally identical to entity resolution: determining whether two records (traces) refer to the "same thing" (same behavioral pattern). Blocking strategies, approximate matching, and transitive closure from the ER literature apply directly.

---

## Practical Search Strategy

### Start from anchor papers

Use Connected Papers (connectedpapers.com) seeded from these verified papers. Two hops from each anchor yields 40-60 papers that keyword search misses:

- "Revisiting Code Similarity" (Song 2024) -- code similarity baselines
- "Causal Agent Replay" (arXiv:2606.08275) -- causal attribution
- "TRACE" (arXiv:2606.00611) -- trajectory compression
- "From Agent Traces to Trust" (arXiv:2606.04990) -- evidence tracing
- "Open-SWE-Traces" (arXiv:2606.16038) -- trace corpora
- "CausalMix" (arXiv:2607.01104) -- data mixture optimization
- "ToolChain-CRC" (arXiv:2606.18467) -- conformal prediction
- "Agent Behavior Mining" (arXiv:2606.20669) -- process mining

### Use Semantic Scholar's "Highly Influential Citations"

Find a known paper, click Citations, filter by "Highly Influential," sort by date. This filters out 90% of passing mentions and surfaces papers that actually build on the anchor's methods.

### Set arXiv alerts

Subscribe to daily alerts for these categories: cs.CL (agents, trace analysis), cs.SE (debugging, testing), cs.AI (multi-agent, planning), cs.CR (DP, TEEs), cs.DB (data quality, lineage), cs.LG (representation learning, anomaly detection), cs.IR (similarity, retrieval). Use arxiv-sanity-lite or Semantic Scholar Research Feed for filtering.

### Prefer workshop papers

Workshop papers at ML conferences are more practical, more recent, and more honest about limitations than main-track papers. Priority workshops:

- **FMAI** (ICML) -- Failure Modes in Agentic AI
- **Agents in the Wild** (ICML) -- production agent systems
- **Lifelong Agents** (COLM) -- traces, observability, long-horizon
- **SynthAI** (SIGMOD) -- synthetic data quality
- **DCAI** (NeurIPS) -- Data-Centric AI

### Watch GitHub trending

Check weekly, filtering Rust + Python. Look for agent observability frameworks, trace processing libraries, DP tools, embedding/similarity implementations. Key repos: `yzhao062/anomaly-detection-resources`, `pengr/LLM-Synthetic-Data`, `tmgthb/Autonomous-Agents`.

### Cross-pollinate from non-obvious domains

| TC Problem | Non-Obvious Domain | Query |
|---|---|---|
| Trace novelty | Network intrusion detection | `"network intrusion detection" novelty streaming 2025` |
| Contributor incentives | Prediction markets | `"prediction market" "information aggregation" incentive 2025` |
| Trace compression | Video summarization | `"keyframe extraction" "temporal summarization" importance 2025` |
| Multi-contributor queries | Secure MPC | `"secure aggregation" "multi-party" statistics privacy 2025` |
| Pattern vocabulary | Program synthesis | `"program synthesis" "library learning" abstraction 2025` |
| Trace deduplication | Entity resolution | `"entity resolution" "record linkage" scalable 2025` |

---

## Quick-Start: Five Searches to Run Today

Highest-payoff queries targeting the biggest gaps in TC's current knowledge:

1. **Conformal prediction for LLM outputs** -- calibrated confidence intervals on novelty scores.
   `"conformal prediction" LLM uncertainty "coverage guarantee" 2025 2026`

2. **Trajectory compression with safety preservation** -- 10-50x storage savings without governance decay.
   `"trajectory compression" "safety-aware" agent "long-horizon" 2025 2026`

3. **Data mixture marginal value** -- core algorithm for TC's value attribution.
   `"data mixture" "marginal contribution" training optimization 2025 2026`

4. **Active learning for annotation budgets** -- makes PR #173 10x more efficient.
   `"active learning" annotation "information gain" "label budget" LLM 2025`

5. **Zero-replay failure attribution** -- offline causal debugging without re-execution.
   `"zero replay" OR "offline" "failure attribution" agent trace causal 2025 2026`

---

## How to Use These Queries

1. **Perplexity Pro** (best for current market intelligence): Use Q-M* and Q-G* queries
2. **Google Scholar** (best for academic papers): Use Q-S*, Q-T*, and Q-U* queries
3. **Semantic Scholar** (best for citation graphs): Use anchor papers from Practical Search Strategy, then "Highly Influential Citations"
4. **Connected Papers** (best for discovering related work): Seed with any anchor paper, explore two hops
5. **GitHub/X search** (best for tools and community): Use Q-I* queries
6. **Deep research tools** (Perplexity Spaces, Google DeepResearch): Use any query with the "Looking for" text as additional instruction

Each query is designed to be self-contained -- copy the quoted search string, add the "Looking for" text as context if the tool supports it.
