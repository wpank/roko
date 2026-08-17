# 10 -- Research References and Prior Art

**Date**: August 2026

This document collects the research papers, technical standards, ecosystem projects,
and market data that inform tiagent's design. Each entry includes the source, the key
finding, and how it connects to tiagent's architecture.

tiagent is not speculative. Every major design decision -- self-improvement loops,
trajectory retrieval, cascade routing, quality gates, shared learning commons -- has
published research validating the approach. This document maps those connections.

---

## 1. Self-Improving Agent Research

These papers establish that optimizing the harness around a model produces gains
comparable to or exceeding model improvements alone. tiagent's entire architecture
is built on this insight.

### RHO: Retrieval-augmented Harness Optimization

- **Source**: arXiv:2606.05922
- **Key finding**: SWE-Bench Pro accuracy jumped from 59% to 78% -- not by changing the
  model, but by optimizing the harness (prompts, tool configurations, retrieval strategy).
  The wrapper around the model matters as much as the model itself.
- **Relevance to tiagent**: This is tiagent's foundational premise. tiagent treats the
  entire execution environment -- system prompts, tool availability, model routing,
  context assembly -- as parameters to optimize. The prompt experiment store A/B tests
  configurations; the cascade router learns which models work best for which tasks; the
  playbook extractor captures strategies that succeeded. RHO validates that this approach
  produces measurable, substantial gains.

### Dynamic Cheatsheet

- **Source**: arXiv:2504.07952 (ICLR 2026)
- **Key finding**: Agents that maintain a persistent, cross-session strategy memory --
  accumulating rules and heuristics from past tasks -- outperform agents that start
  fresh each session. The "cheatsheet" evolves as the agent encounters new situations.
- **Relevance to tiagent**: tiagent's playbook extraction implements this concept
  directly. After each successful task execution, the system distills reusable strategies
  ("when migrating a database schema in Django, always run makemigrations before migrate
  and check for circular dependencies") into a persistent playbook store. These playbooks
  are retrieved at dispatch time and injected into the agent's system prompt. The agent
  accumulates institutional knowledge across sessions.

### Sleep-Time Compute

- **Source**: Google DeepMind (2025)
- **Key finding**: Processing and consolidating information during idle periods (rather
  than at inference time) achieves approximately 5x reduction in active inference cost.
  The model "thinks" when compute is cheap and retrieves pre-computed results when
  it matters.
- **Relevance to tiagent**: tiagent's dream consolidation cycle is inspired by this.
  During idle periods, the system processes accumulated execution traces: distilling
  playbooks, updating the cascade router's learned weights, computing trajectory
  embeddings, and pruning low-value entries from the knowledge store. Active task
  execution stays fast because the heavy learning work happens offline.

### HarnessX Foundry Pattern

- **Source**: Derived from RHO and related harness optimization work
- **Key finding**: Treat the harness itself as a parameter to optimize. Instead of
  hand-tuning prompts and tool configurations, run systematic A/B tests across harness
  configurations and promote winners automatically.
- **Relevance to tiagent**: tiagent's `ExperimentStore` implements this pattern. The
  system runs controlled experiments on prompt templates, tool configurations, and
  model routing strategies. Each experiment tracks success rates, cost, and latency.
  Winners are promoted to default configurations. The harness evolves based on evidence,
  not intuition.

### EvoRoute: Evolutionary Model Routing

- **Source**: Evolutionary optimization research applied to LLM routing
- **Key finding**: Genetic algorithms over routing configurations discover
  non-obvious routing strategies that outperform hand-designed rules. The search
  space of possible model assignments is too large for manual optimization.
- **Relevance to tiagent**: Informs tiagent's cascade router design. The router uses
  multi-armed bandit algorithms (Thompson Sampling) to explore model assignments, with
  task features as context. Over time, the router converges on routing strategies that
  minimize cost while maintaining quality thresholds -- the same objective EvoRoute
  addresses with evolutionary methods.

---

## 2. Trajectory RAG Research

These papers address the specific problem tiagent solves: how to retrieve and use
past agent execution traces to improve future performance.

### T3: Trajectory-based Task Transfer

- **Source**: arXiv:2605.03344
- **Key finding**: Transferring structured execution trajectories between tasks
  improves agent performance more than transferring unstructured text summaries.
  The sequential structure of trajectories -- which tools were called in what order,
  what intermediate states looked like -- carries information that flat summaries lose.
- **Relevance to tiagent**: tiagent stores full execution traces (JSONL format, one
  event per line) including tool calls, LLM responses, gate results, and timing data.
  Trajectory RAG retrieves these structured traces and provides them as in-context
  examples, preserving the sequential structure that T3 shows is important.

### ExpRAG: Experience-Augmented Retrieval

- **Source**: arXiv:2603.18272
- **Key finding**: Augmenting retrieval with agent execution experiences -- not just
  documents but records of what agents actually did -- produces better downstream
  task completion. The retrieved experiences serve as executable templates, not just
  information.
- **Relevance to tiagent**: tiagent's integration with TraceCommons implements
  experience-augmented retrieval at scale. Retrieved traces are not passive context;
  they are patterns the agent can follow. When an agent retrieves a trace of another
  agent successfully implementing JWT auth in FastAPI, it gets an executable template:
  which files to create, which order to edit them, which tests to write.

### AgentIR: Information Retrieval for Agent Trajectories

- **Source**: arXiv:2603.04384
- **Key finding**: Standard document retrieval methods (BM25, dense retrieval) are
  suboptimal for agent trajectories. Trajectory-specific retrieval methods that account
  for tool call sequences, state transitions, and outcome signals produce significantly
  better retrieval quality.
- **Relevance to tiagent**: Validates tiagent's multi-signal retrieval approach. The
  TraceCommons integration uses BGE-large-en-v1.5 embeddings for semantic similarity,
  but also incorporates structural matching (same tools used), outcome filtering
  (successful traces prioritized), and task-type classification as retrieval signals.

### LEGOMem: Typed Memory Decomposition

- **Source**: arXiv:2510.04851
- **Key finding**: Decomposing agent memory into typed components (episodic, semantic,
  procedural) and retrieving from each type independently outperforms a single
  monolithic memory store. Different types of recall serve different purposes.
- **Relevance to tiagent**: tiagent's memory architecture reflects this decomposition.
  The episode log (`.roko/episodes.jsonl`) stores episodic memory -- what happened during
  each task. The playbook store holds procedural memory -- distilled strategies. The
  knowledge store (`roko-neuro`) maintains semantic memory -- durable facts and
  relationships. Each is queried independently at dispatch time and composed into
  the agent's system prompt.

---

## 3. Agent Quality and Safety

Research on calibrating agent output quality and ensuring safe tool use.

### ToolChain-CRC: Conformal Prediction for Tool Chains

- **Source**: Research on conformal prediction applied to LLM tool use
- **Key finding**: Conformal prediction can provide calibrated confidence intervals for
  tool chain outcomes. Instead of binary pass/fail, you get a prediction set that
  contains the correct outcome with guaranteed probability.
- **Relevance to tiagent**: Informs tiagent's adaptive gate thresholds. The gate
  pipeline uses exponential moving averages (EMA) per rung to adjust pass/fail
  thresholds based on recent history. Conformal prediction provides the theoretical
  grounding for these adaptive thresholds: as the agent improves, thresholds tighten;
  during exploration or model changes, thresholds relax.

### SSBC: Small-Sample Bayesian Calibration

- **Source**: Research on Bayesian methods for small-sample quality assessment
- **Key finding**: Bayesian calibration methods can produce useful quality estimates
  even with very small samples (tens of observations). This is critical for settings
  where data arrives slowly and decisions must be made early.
- **Relevance to tiagent**: tiagent's cascade router faces the classic cold-start
  problem: when a new model is added, there is no historical data to inform routing.
  Bayesian calibration enables the router to form useful estimates from a small number
  of trial runs, reducing the exploration cost of adding new models.

### BQP: Diversity Re-ranking for Retrieval

- **Source**: arXiv:2604.02554
- **Key finding**: Re-ranking retrieved results for diversity -- not just relevance --
  improves downstream task performance. Retrieving five highly similar traces is less
  useful than retrieving five traces that cover different strategies for the same task.
- **Relevance to tiagent**: Applied to trajectory RAG retrieval. When tiagent retrieves
  past traces for a new task, diversity re-ranking ensures the agent sees multiple
  approaches rather than variations of the same approach. This is especially important
  for tasks where the best strategy is not obvious.

### TRAIL: Failure Taxonomy for Agent Traces

- **Source**: Research on classifying and learning from agent failures
- **Key finding**: Structured failure taxonomies (categorizing why agents fail, not just
  that they failed) enable targeted improvements. Common failure modes include: wrong
  tool selection, premature termination, context window overflow, and hallucinated
  file paths.
- **Relevance to tiagent**: tiagent's gate failure replan system uses structured failure
  information. When a gate fails, the system generates a `build_gate_failure_plan_revision`
  that includes the specific failure category, the failing rung, and the relevant context.
  The replanned task targets the specific failure mode rather than retrying blindly.

---

## 4. On-Chain Agent Standards

Standards and protocols that define how AI agents interact with blockchains, identity
systems, and each other. tiagent positions Celestia within this emerging landscape.

### ERC-8004: Agent Identity on Ethereum

- **Source**: Ethereum Improvement Proposal (proposed standard)
- **Key finding**: Proposes a standard for on-chain agent identity, enabling agents
  to have verifiable, persistent identities tied to their execution history. Includes
  agent registration, capability attestation, and reputation tracking.
- **Relevance to tiagent**: tiagent agents that publish traces to Celestia create a
  verifiable execution history. ERC-8004 provides a complementary identity layer:
  the traces prove what the agent did (data availability), while the identity standard
  proves who the agent is (authentication). Cross-chain interop between Celestia DA
  and Ethereum identity is a future integration path.

### A2A: Google Agent-to-Agent Protocol

- **Source**: Google (v1.0.0, 2026); 150+ participating organizations
- **Key finding**: Standardized protocol for agents to discover each other's capabilities,
  negotiate task handoffs, and exchange structured results. Version 1.0.0 indicates
  production readiness.
- **Relevance to tiagent**: A2A defines how agents communicate; tiagent + Celestia
  define what they learn from each other. These are complementary: A2A handles
  real-time agent coordination, while Celestia-backed trace sharing handles
  asynchronous knowledge transfer across time and organizations.

### MCP: Model Context Protocol

- **Source**: Anthropic (2024-2026); 97M+ monthly SDK downloads
- **Key finding**: Standardized protocol for connecting LLMs to external tools and
  data sources. Rapidly adopted across the ecosystem: Claude, Cursor, Windsurf,
  Cline, and dozens of other clients support MCP.
- **Relevance to tiagent**: tiagent uses MCP as its tool integration layer. The
  `agent.mcp_config` in `roko.toml` passes MCP server configurations through to
  agent processes. tiagent includes purpose-built MCP servers for code intelligence
  (`roko-mcp-code`), GitHub integration (`roko-mcp-github`), and script execution
  (`roko-mcp-scripts`). MCP adoption numbers demonstrate the ecosystem's readiness
  for standardized agent tooling.

### AITP: NEAR Agent Interaction and Transaction Protocol

- **Source**: NEAR Protocol
- **Key finding**: Protocol for agents to interact with blockchain transactions,
  including payment authorization, data access, and cross-chain operations. Designed
  for the NEAR ecosystem but applicable to multi-chain agent workflows.
- **Relevance to tiagent**: TraceCommons (tiagent's shared trace commons) compensates
  contributors with NEAR-denominated credits via AITP. tiagent agents that submit
  high-quality traces earn credits, creating an economic incentive for participation
  in the shared learning commons.

### x402: HTTP 402 Payment Protocol

- **Source**: Coinbase (2025-2026)
- **Key finding**: Uses the HTTP 402 status code to enable pay-per-request API access
  for agents. Agents can autonomously pay for API calls using cryptocurrency,
  removing the need for pre-negotiated billing agreements.
- **Relevance to tiagent**: x402 enables tiagent agents to autonomously pay for
  Celestia DA submissions, TraceCommons queries, and third-party API access. Instead
  of requiring pre-configured API keys and billing accounts, agents can pay per
  request using on-chain funds.

---

## 5. Celestia Technical References

Primary sources for tiagent's Celestia integration.

### Celestia Documentation

- **Source**: docs.celestia.org
- **Key topics**: Data Availability Sampling (DAS), blob submission, namespace design,
  light node operation, gas pricing model
- **Relevance to tiagent**: Primary reference for blob submission costs, namespace
  schema design, and light node resource requirements. tiagent's blob pricing estimates
  ($0.07-0.81/MB) are derived from Celestia's current fee market.

### celestia-types / celestia-rpc / celestia-grpc (v1.0)

- **Source**: github.com/eigerco/lumina (Rust ecosystem)
- **Key topics**: Rust type definitions for Celestia data structures, RPC client for
  blob submission and retrieval, gRPC bindings for node communication
- **Relevance to tiagent**: tiagent's Celestia integration layer depends on these
  crates for blob encoding, submission, and retrieval. The v1.0 release signals
  production readiness for Rust-native Celestia applications.

### lumina-node

- **Source**: github.com/eigerco/lumina
- **Key topics**: Rust implementation of a Celestia light node. Performs Data
  Availability Sampling, blob retrieval, and header verification.
- **Relevance to tiagent**: Candidate for embedded light node operation. Running
  lumina-node inside the tiagent process would eliminate dependency on external RPC
  endpoints and enable offline-first trace verification. Resource overhead (CPU,
  memory, bandwidth) determines feasibility.

### nmt-rs: Namespaced Merkle Tree

- **Source**: Rust implementation of Celestia's Namespaced Merkle Tree
- **Key topics**: Namespace-ordered data commitment, inclusion proofs, namespace
  range queries
- **Relevance to tiagent**: NMT proofs enable tiagent to verify that a specific trace
  was included in a specific Celestia block without downloading the entire block.
  This is the cryptographic primitive that makes "verifiable trace history" practical.

### Celestia Vision 2.0

- **Source**: Celestia Foundation blog (2026)
- **Key topics**: Expanded vision for Celestia as infrastructure beyond rollups.
  Explicitly mentions AI agent payments and data availability for non-financial
  use cases.
- **Relevance to tiagent**: Official acknowledgment from the Celestia Foundation
  that AI agent use cases are part of Celestia's strategic direction. tiagent is
  a concrete implementation of the use cases described in the vision post.

### Sovereign Labs Acquisition

- **Source**: Celestia Foundation announcement (July 2026)
- **Key topics**: Acquisition of the team behind the Sovereign SDK, a framework
  for building sovereign rollups on Celestia.
- **Relevance to tiagent**: Signals Celestia's investment in developer tooling
  and Rust-native infrastructure. The Sovereign SDK team's expertise in Rust
  rollup frameworks aligns with tiagent's architecture (Rust workspace, modular
  crate design, trait-based composition).

---

## 6. Market Data

Quantitative context for tiagent's market positioning.

| Metric | Value | Source | Date |
|---|---|---|---|
| AI agent market size | $22.6-27B | Industry reports | Mid-2026 |
| MCP SDK monthly downloads | 97M+ | npm/PyPI registry data | Mid-2026 |
| Celestia DA cost per MB | $0.07-0.81 | Celestia fee market | Mid-2026 |
| Celestia rollups on mainnet | 56+ | Celestia ecosystem tracker | Mid-2026 |
| 0G Labs AI ecosystem fund | $88.88M | 0G Labs announcement | 2026 |
| 0G Labs accelerator fund | $20M | 0G Labs announcement | 2026 |
| TraceCommons submissions | ~352 | TraceCommons telemetry | August 2026 |
| TraceCommons weekly ingest | ~13 | TraceCommons telemetry | August 2026 |
| TraceCommons contributors | 3 | TraceCommons telemetry | August 2026 |
| IronClaw GitHub stars | 14K+ | GitHub | August 2026 |

### What these numbers mean for tiagent

**Market opportunity**: The $22-27B AI agent market is growing rapidly, but current
agent tooling (Claude Code, Codex, Cursor) does not include self-improvement or shared
learning. tiagent addresses an unserved segment of this market.

**Ecosystem readiness**: 97M+ monthly MCP SDK downloads demonstrate that developers
are already building tool-augmented agents. tiagent's MCP-native architecture meets
them where they are.

**Cost feasibility**: Celestia DA at $0.07-0.81/MB means a typical compressed agent
trace (10-50KB after zstd compression) costs less than $0.01 to store on-chain. At
hundreds of traces per day, the DA cost is negligible compared to the LLM inference
cost it helps optimize.

**TraceCommons bootstrap**: The corpus is early-stage (352 submissions, 3 contributors).
This is both a risk (limited retrieval value today) and an opportunity (tiagent can
become a major contributor and shape the commons). Network effects have not yet
kicked in, making this the right time to integrate and help bootstrap.

---

## 7. Related Projects

Projects in the ecosystem that tiagent relates to, competes with, or integrates with.

### TraceCommons

- **Source**: github.com/tracecommons (Zaki Manian / Cosmos SDK co-creator)
- **What it is**: Open commons for AI agent execution traces. Privacy-preserving
  (TEE-based scrubbing and scoring via Intel TDX + NVIDIA GPU TEE), contributor-
  compensated (NEAR credits), quality-gated (6-stage pipeline: redaction, chunking,
  embedding, similarity, perplexity scoring, gate evaluation).
- **Architecture**: ~235K LOC Rust, 6 crates, dual-licensed MIT/Apache-2.0, pilot
  deployment on GCP. Uses BGE-large-en-v1.5 for embeddings, Qwen 3.6 35B for
  perplexity scoring (running on NEAR AI Cloud TEE).
- **Relationship to tiagent**: tiagent is both a consumer (retrieves traces for
  trajectory RAG) and a contributor (submits execution traces after task completion).
  tiagent's Celestia integration adds a verifiable data availability layer that
  TraceCommons does not currently have. The two projects are complementary:
  TraceCommons handles quality scoring and retrieval; Celestia handles immutable
  storage and verifiability.

### IronClaw (NEAR AI)

- **Source**: NEAR AI (14K+ GitHub stars)
- **What it is**: Rust-based agent runtime with NEAR blockchain integration. Primary
  integration partner for TraceCommons. Provides agent lifecycle management, tool
  dispatch, and session recording.
- **Relationship to tiagent**: Peer project. IronClaw and tiagent both integrate
  with TraceCommons but have different strengths. IronClaw is NEAR-native with strong
  blockchain integration; tiagent is model-agnostic with self-improvement loops
  (cascade routing, prompt experiments, adaptive gates) that IronClaw does not have.
  Both submit traces to TraceCommons, expanding the shared corpus.

### roko

- **Source**: github.com/nunchi/roko (~177K LOC Rust, 35 workspace members)
- **What it is**: Rust toolkit for building agents that build themselves. Includes:
  11 LLM provider kinds, plan DAG execution, 16-gate quality pipeline, cascade model
  routing, prompt experiments, dream consolidation, interactive TUI, and HTTP control
  plane (~317 routes).
- **Relationship to tiagent**: roko is tiagent's engine. tiagent packages roko's
  capabilities as a product with Celestia DA integration and TraceCommons connectivity.
  roko provides the agent runtime, learning infrastructure, and quality gates; tiagent
  adds the shared learning commons and on-chain trace storage.

### polkagent

- **Source**: 90-crate Rust workspace for Polkadot agent integration
- **What it is**: Agent framework targeting the Polkadot ecosystem. Large codebase
  indicating serious investment in Rust-native agent tooling for blockchain.
- **Relationship to tiagent**: Demonstrates market demand for blockchain-native agent
  frameworks in Rust. Validates that the "Rust workspace + blockchain integration"
  architecture tiagent uses is not idiosyncratic -- other teams are making the same
  structural choices. polkagent targets Polkadot; tiagent targets Celestia.

---

## 8. Research-to-Implementation Mapping

How each research area maps to specific tiagent components.

| Research Area | tiagent Component | Implementation Status |
|---|---|---|
| RHO (harness optimization) | Prompt experiment store | Wired (`ExperimentStore`) |
| Dynamic Cheatsheet | Playbook extraction + retrieval | Wired (queried at dispatch) |
| Sleep-Time Compute | Dream consolidation cycle | Partial (built, no cron trigger) |
| EvoRoute | Cascade router | Wired (`CascadeRouter`) |
| T3 (trajectory transfer) | JSONL trace storage + retrieval | Wired (episode log) |
| ExpRAG | TraceCommons integration | Design phase |
| AgentIR | Multi-signal retrieval | Partial (embeddings + structural) |
| LEGOMem | Typed memory decomposition | Wired (episodes + playbooks + neuro) |
| ToolChain-CRC | Adaptive gate thresholds | Wired (EMA per rung) |
| SSBC | Cascade router cold-start | Wired (Thompson Sampling priors) |
| BQP | Diversity re-ranking | Design phase |
| TRAIL | Gate failure replan | Wired (`build_gate_failure_plan_revision`) |
| MCP | Tool integration layer | Wired (`agent.mcp_config`) |
| NMT proofs | Trace verification | Design phase (depends on Celestia integration) |

---

## 9. Further Reading

For deeper technical detail on specific topics covered in this document:

- **Celestia integration architecture**: See `04-celestia-integration.md`
- **Network effects and TraceCommons**: See `05-network-effects.md`
- **Competitive positioning**: See `07-competitive-landscape.md`
- **Technical architecture**: See `03-architecture.md` and `09-technical-appendix.md`
- **Grant proposal with milestones**: See `08-grant-proposal.md`

---

*This document is maintained as part of the tiagent proposal for Celestia Foundation.
All arXiv references were verified as of August 2026. Market data reflects mid-2026
estimates and should be updated as newer figures become available.*
