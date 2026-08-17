# Trajectory RAG

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding
agent session traces (~235K LOC, 6 crates, MIT/Apache-2.0). Contributors submit scrubbed
traces of AI agent sessions; quality and novelty are scored inside TEEs (Trusted Execution
Environments -- hardware-isolated encrypted compute on NEAR AI Cloud, Intel TDX + NVIDIA
GPU TEE). Contributors earn NEAR blockchain credits. ~352 submissions, 3 contributors,
6 GitHub stars. TC stores scored, quality-assessed agent session traces -- the full record
of what agents did, which tools they called, what worked, and what failed. The natural next
step: let agents query "show me traces that solved similar problems" to learn from the
corpus. This is TC's potential killer feature.

---

## 1. The Vision

An agent facing a complex task queries TC: "show me how other agents solved this kind of
problem." TC returns relevant, high-quality, diverse traces. The agent learns from past
solutions without retraining.

This is RAG applied to agent experience -- not document retrieval, but trajectory retrieval.
Traditional RAG retrieves text passages to answer factual questions. Trajectory RAG retrieves
structured records of agent behavior -- tool-call sequences, decision points, error recovery
strategies, multi-step reasoning chains -- to inform an agent's approach to a new problem.

The difference matters: a document tells you *what* is true; a trajectory shows you *how*
to get something done. TC already has the corpus. The question is how to make it queryable.

---

## 2. Research Foundations

Six papers establish that trajectory retrieval is feasible, beneficial, and requires different
techniques than standard document RAG.

### 2.1 LRAT -- Learning to Retrieve from Agent Trajectories

**Zhou et al. (2026)** "Learning to Retrieve from Agent Trajectories"
arXiv:2604.04949

Trains a retriever specifically for agent trajectory retrieval. Key finding: trajectories
have temporal structure and tool-call sequences that standard retrievers do not exploit --
trajectory retrieval requires different optimization than document retrieval. LRAT extracts
supervision from agent interaction data (browsing actions, ignored documents, reasoning
traces) and improves evidence recall, task success, and execution efficiency.

**TC relevance**: TC's scored traces with outcome labels are precisely the training signal
LRAT consumes. TC could train a trajectory retriever on its own corpus.

### 2.2 ExpRAG -- Retrieval-Augmented LLM Agents: Learning to Learn from Experience

**Ferraz et al. (2026)** "Retrieval-Augmented LLM Agents: Learning to Learn from Experience"
arXiv:2603.18272

Retrieval of *experience* for policy generalization. Key result: combining experience
retrieval with fine-tuning "significantly improves generalization to unseen tasks" --
neither alone matches the combined approach. Provides detailed analysis of retrieval design
choices (storage, query formulation, trajectory selection).

**TC relevance**: TC is an experience store. ExpRAG validates the value proposition and
provides design guidance for the retrieval interface.

### 2.3 RISE -- Towards Retrieving Interaction Spaces for Agentic Search

**Zhuang et al. (2026)** "Towards Retrieving Interaction Spaces for Agentic Search"
arXiv:2606.06880

Retrieves interaction spaces -- the full context of how an agent interacted with tools and
environment, not just text. Combines BM25 for bounded corpus subsetting with pre-processed
documents for navigation. 81% accuracy on 1M-doc corpora (competitors degrade), ~75% cost
reduction.

**TC relevance**: TC traces *are* interaction spaces. RISE's bounded-exploration architecture
maps directly to TC: retrieve a bounded set of relevant traces, let the agent explore.

### 2.4 "Beyond RAG for Agent Memory" -- Retrieval by Decoupling and Aggregation

**Hu et al. (2026)** "Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation"
arXiv:2602.02007

Note on naming: this paper is sometimes referred to internally as "xMemory," but that
shorthand does not appear in the paper itself. The actual title is "Beyond RAG for Agent
Memory: Retrieval by Decoupling and Aggregation."

**Critical warning for TC**: naive top-k cosine similarity over trajectory memory collapses
into redundant dense regions. The top-k results all come from the densest cluster in
embedding space, missing diverse relevant trajectories. The paper addresses this via
"decoupling before aggregation" -- extracting distinct facts from similar histories and
organizing them hierarchically before retrieval.

If 60% of TC's traces involve Python web development, naive top-k returns Python web
traces for every query regardless of actual relevance. TC MUST use diversity-aware
retrieval, not vanilla cosine top-k. This is the single most important implementation
lesson from the trajectory RAG literature. See Section 3 below.

### 2.5 AgentIR-4B -- Reasoning-Aware Retrieval for Deep Research Agents

**Chen et al. (2026)** "AgentIR: Reasoning-Aware Retrieval for Deep Research Agents"
arXiv:2603.04384

A 4B-parameter retrieval model designed specifically for agent contexts. AgentIR jointly
embeds the agent's reasoning trace alongside its query, exploiting the explicit natural
language reasoning that agents generate before each search. On BrowseComp-Plus: 68%
accuracy vs 50% from conventional embedders twice its size and 37% from BM25.

**TC relevance**: AgentIR demonstrates that agent-context-aware retrieval dramatically
outperforms generic embedding models. TC's retrieval endpoint should accept the agent's
current reasoning context (not just a bare query string) to improve result quality.

### 2.6 T3 -- RAG over Thinking Traces Can Improve Reasoning Tasks

**Arabzadeh et al. (2026)** "RAG over Thinking Traces Can Improve Reasoning Tasks"
arXiv:2605.03344

T3 transforms thinking traces -- intermediate reasoning trajectories from problem-solving
-- into structured, retrieval-friendly representations. Results: on AIME 2025-2026,
RAG with Gemini-2-thinking traces achieves +56.3%, +8.6%, and +7.6% relative gains
across different models. The key insight: the retrieval corpus matters more than the
retrieval method. Replacing standard web corpora with traces yields surprising gains.

**TC relevance**: T3 validates TC's core thesis -- that a corpus of agent traces is
a valuable retrieval target. TC's traces are richer than T3's thinking traces because
they include tool calls, environment interactions, and outcome labels, not just reasoning
text.

---

## 3. The Top-K Collapse Problem

Hu et al. (arXiv:2602.02007, "Beyond RAG for Agent Memory") documents this clearly: when
you do naive top-k cosine similarity over a trajectory corpus, results collapse into the
densest region of embedding space.

**Why this happens**: embedding models map similar content to nearby vectors. If the corpus
has an uneven distribution -- and real corpora always do -- the densest cluster dominates
all queries. If 60% of TC traces involve Python web development, every query returns
Python web traces regardless of actual relevance. At TC's current ~352 traces this may
not bite yet. At 1,000+ traces, it will.

**Fix: BQP (Binary Quadratic Programming) Diversity Retrieval**

The recommended diversity solution is cardinality-constrained BQP re-ranking over an
expanded candidate set (arXiv:2604.02554):

1. Retrieve top-100 candidates by raw cosine similarity
2. Formulate a BQP that maximizes a joint relevance-diversity objective subject to the
   cardinality constraint |R| = k
3. Solve the BQP (efficient solvers exist; the problem structure exploits sublinear-in-k
   scaling)
4. Return the selected set R

**Why BQP over MMR**: research (arXiv:2604.02554) demonstrates that MMR has **no
approximation guarantee** -- it is a greedy algorithm applied to a non-monotone submodular
objective, so worst-case quality is unbounded. MMR runs O(knd) -- linear in k, n, and
dimensionality. BQP is **2.4-22.9x faster than MMR** at the practically-relevant
similarity threshold theta >= 0.5 and scales sublinearly in k. TC's prior prescription
of lambda=0.7 MMR was untested and should be replaced.

**Why not DPP**: Determinantal Point Processes offer theoretical elegance but are slower
than MMR in practice, and the log-det term in the DPP objective is unbounded and hard to
tune. DPP adds complexity without the speed or interpretability advantages of BQP.

**MMR as fallback**: MMR remains a valid starting point for teams that want the simplest
possible initial implementation (O(kn) greedy, no solver required). The key correction
to prior guidance: do not treat MMR's lambda=0.7 as a tuned default -- it is untested
on TC's corpus. Measure result diversity (mean pairwise embedding distance) and tune
against observed cluster collapse. Migrate to BQP once the basic pipeline is validated.

**Interpretable trade-off**: BQP exposes a bounded, interpretable objective compared to
MMR's greedy heuristic, making it easier to reason about the relevance-diversity tradeoff
and to explain retrieval behavior to users and contributors.

---

## 4. Architecture for TC

### 4.1 Retrieval Pipeline

```
Query                                                   Results
  |                                                        ^
  v                                                        |
[1. Query Encoding] -----> [2. Candidate Retrieval] --> [3. BQP Diversity Re-ranking]
                                                            |   (MMR as fallback)
                                                            v
                                                     [4. Quality Filter]
                                                            |
                                                            v
                                                     [5. Privacy Filter]
                                                            |
                                                            v
                                                        Response
```

**Stage 1: Query Encoding.** Embed the agent's full context -- not a bare query string.
AgentIR (Section 2.5) shows reasoning-aware embedding dramatically outperforms bare queries.
TC's API should accept structured input:

```json
{
  "task": "Deploy a Rust service to Kubernetes with health checks",
  "recent_actions": ["cargo build", "docker build", "kubectl apply"],
  "errors": ["CrashLoopBackOff: health check timeout"],
  "reasoning": "The readiness probe path might be wrong"
}
```

**Stage 2: Candidate Retrieval.** Hybrid BM25 + dense retrieval. BM25 catches lexical
matches (error messages, tool names) that embedders miss. Dense catches semantic similarity.
Merge by reciprocal rank fusion: `RRF(d) = sum_i 1/(60 + rank_i(d))`. Top-100 candidates.

**Stage 3: BQP Diversity Re-ranking.** Cardinality-constrained BQP (arXiv:2604.02554)
over the top-100. Return top-10 (configurable). BQP is 2.4-22.9x faster than MMR at
theta >= 0.5 and scales sublinearly in k. For teams implementing incrementally, MMR is an
acceptable initial fallback -- but measure result diversity and migrate. See Section 3.

**Stage 4: Quality Filtering.** Only return traces above TC's gate quality threshold.
Virtuous cycle: the gate pipeline that determines credit payments also determines retrieval
eligibility. Higher-quality submissions earn more AND get retrieved more.

**Stage 5: Privacy Filtering.** All results redacted (existing pipeline). Query embeddings
computed in TEE. Rate limiting per API key. Minimum k-anonymity: do not return traces from
categories with fewer than 5 distinct contributors.

### 4.2 Embedding Strategy

TC's existing embedder is BGE-large-en-v1.5 (used for the novelty scoring HNSW index).
Reuse for Phase 1, then evolve:

- **Content embedding** (immediate): BGE-large-en-v1.5 over rendered trace text. Already
  computed for novelty scoring. Reuse the HNSW index -- zero additional embedding cost.
- **Structural embedding** (next): encode tool-call sequence separately. Two traces using
  the same tools in the same order are structurally similar regardless of content. Aligns
  with doc 02 B.3 (process mining DFG).
- **Outcome embedding** (next): cluster by success/failure + error type. When an agent
  queries with a specific error, prioritize traces that resolved that error class.
- **Multi-view** (later): combine all three. Final relevance = minimum similarity across
  views. The minimum prevents content-only matches from dominating when structural
  relevance is absent.

### 4.3 Scale Thresholds

TC's architecture decisions should be driven by corpus size:

| Traces per Category | Retrieval Approach | Notes |
|---|---|---|
| < 100 | Insufficient for genuine utility | Return all traces in category, sorted by quality. Retrieval adds no value over browsing. |
| 100 - 1,000 | Basic dense retrieval + BQP diversity | BGE-large + BQP re-ranking. MMR acceptable as initial fallback. BM25 optional. |
| 1,000 - 10,000 | Hybrid BM25 + dense, BQP essential | Lexical and semantic signals complement. BQP diversity re-ranking critical to avoid cluster collapse; 2.4-22.9x faster than MMR at this scale. |
| 10,000+ | Hierarchical retrieval | Cluster -> within-cluster retrieval -> cross-cluster MMR re-rank. Pre-compute cluster centroids. Consider approximate methods (ScaNN, FAISS IVF). |

TC is at ~352 total submissions -- most categories have far fewer than 100 traces.
Trajectory RAG is *growth-dependent*. Build the pipeline now, do not over-invest in scale
optimizations, and use RAG access as a growth incentive ("submit traces, get access to the
knowledge base").

---

## 5. Synergies with Existing TC Infrastructure

Trajectory RAG layers on what TC already has:

| Existing Component | Trajectory RAG Reuse |
|---|---|
| BGE-large-en-v1.5 embeddings | Content embedding for retrieval (already computed) |
| HNSW vector index (usearch) | Candidate retrieval (already built) |
| Redaction pipeline | Privacy filtering (already runs on all traces) |
| Gate pipeline quality scores | Quality filtering threshold |
| TEE infrastructure (NEAR AI Cloud) | Query processing in secure enclave |
| Chunker + event parser | Sub-trace decomposition for fragment retrieval |
| MinHash dedup (planned, doc 02 A.2) | Pre-filter near-duplicates from results |
| Process mining DFG (planned, doc 02 B.3) | Structural embedding for tool-call similarity |

New components needed: (1) query API endpoint, (2) BQP diversity re-ranker (MMR as
initial fallback), (3) BM25 index via tantivy + RRF fusion, (4) trace summarizer. Items
(1) and (2) are days of work. (3) uses the mature tantivy crate. (4) can reuse TC's LLM
infra or the compression methods from doc 02 (TRACE achieves 10-50x compression with
+12.6pp safety, arXiv:2606.00611).

**Context budget**: a retrieved trace may be 10K+ tokens. Five traces consume 50K tokens
before the agent starts working. Solutions: trace summarization (doc 02 C.10), hierarchical
presentation (summaries first, expand on request), or sub-trace fragment retrieval
(doc 02 B.6).

---

## 6. Privacy Architecture

Trajectory RAG introduces new risks: trace content now moves *outward* (TC -> querying
agent), not just inward.

- **Query privacy**: query context (task, errors, code) is sensitive. Compute query
  embeddings inside the TEE -- raw query text never leaves the enclave.
- **Trace privacy**: all results redacted (existing pipeline). New attack surface: repeated
  queries can enumerate the corpus. Mitigate with rate limits, minimum result set sizes,
  and query bucketing (round similar queries to the same embedding).
- **Contributor privacy**: categories with 1-2 contributors risk de-anonymization. Enforce
  minimum 5 distinct contributors per returned result set.
- **Cross-user retrieval**: TC's differentiator. Langfuse/Braintrust store traces per-org.
  TC enables cross-user retrieval because TEEs + redaction make it viable. The privacy
  architecture is load-bearing for this feature.

### 6.1 Brute-Force HDC for Side-Channel-Free Retrieval

At TC's current ~352 traces, deterministic brute-force scan over HDC (Hyperdimensional
Computing) hypervectors is the recommended retrieval mechanism inside TEEs, for two
compounding reasons.

**The HNSW side-channel problem**: HNSW's graph-traversal algorithm follows random memory
access patterns -- it walks edges, fetches non-contiguous nodes, and branches based on
similarity comparisons. Inside an enclave (Intel TDX), these irregular memory access
patterns leak information via cache-timing side-channels even though memory contents are
encrypted. An attacker observing page faults or cache evictions can reconstruct the
traversal path, and from the traversal path, infer which regions of the index were
explored -- revealing structural information about the corpus and the query. This is a
well-known class of enclave side-channel attack, and it applies to any graph-based ANN
index.

**The determinism problem for attestation**: HNSW introduces randomness at index-build time
(layer assignment uses a random number generator) and at query time (thread scheduling
affects traversal order in parallel builds). Non-deterministic execution undermines
attestation: if two runs of the same query on the same corpus can produce different results,
the enclave quote does not bind to a specific retrieval outcome. Attestation becomes
meaningless for retrieval integrity purposes.

**Brute-force exact scan solves both**: at 352 traces, computing exact cosine or Hamming
similarity between the query hypervector and every stored hypervector is:

- **Side-channel-free**: sequential memory access with no query-dependent branching in the
  access pattern. Every scan touches every stored vector in the same order, regardless of
  the query. No information about the query or corpus structure leaks through memory access
  patterns.
- **Deterministic**: given the same query and corpus, brute-force always returns the same
  result. The retrieval outcome is fully determined by the enclave binary and the stored
  vectors -- both of which are reflected in the attestation measurement.
- **Fast enough**: at 352 traces with hypervectors of D=128 to D=10,000 dimensions, a
  full scan completes in microseconds on modern hardware. The latency argument for HNSW
  does not apply until the corpus is orders of magnitude larger.

**HDC advantage**: HDC hypervectors support efficient Hamming distance computation via
popcount, which is faster than floating-point cosine on CPU and remains accurate. VS-Graph
(arXiv:2512.03394) demonstrates HDC graph classification at D=128 with competitive
accuracy and up to 450x training speedup vs GNNs -- the same HDC representation used for
novelty scoring can be reused for retrieval, providing a unified representation across
TC's pipeline.

**This is a rare two-birds solution**: brute-force HDC scan simultaneously resolves the
side-channel privacy risk (attestable retrieval for TEE integrity) and the HNSW
determinism problem (meaningful attestation). It requires no ORAM (Oblivious RAM) schemes,
no PIR (Private Information Retrieval) protocols, and no additional cryptographic machinery
-- just a sequential scan.

**When to revisit**: switch to HNSW or an oblivious ANN index only when corpus scale
makes brute-force latency unacceptable, at approximately 10,000+ traces per category.
At that point, evaluate ORAM-based oblivious indexes (expensive but side-channel-free) or
accept HNSW with documented side-channel risk and compensating controls. The current
recommendation is: do not pay that complexity cost at 352 traces.

---

## 7. Beyond Retrieval: Trajectory Memory, Distillation, and Pre-Computation

The papers in Section 2 establish that trajectory retrieval works. The papers in this
section ask the next question: what do you *do* with retrieved trajectories? Five recent
results show that verbatim retrieval is a starting point, not an endpoint. Agents that
distill, decompose, and pre-process trajectories outperform agents that paste them into
context. TC's scored, outcome-labeled traces are exactly the input these systems consume --
and TC's cross-organization corpus makes the gains compound across agents and teams.

### 7.1 Dynamic Cheatsheet -- The Single-Agent Version of TC

**Suzgun et al. (2026)** "Dynamic Cheatsheet: Test-Time Learning with Adaptive Memory"
arXiv:2504.07952 (ICLR 2026)

Dynamic Cheatsheet gives black-box LLMs a persistent, evolving memory that updates at
inference time without any weight changes. The agent maintains a "cheatsheet" of strategies,
heuristics, and lessons learned from prior attempts. As it encounters new problems, it
consults the cheatsheet, attempts solutions, and updates the cheatsheet with what worked
and what failed.

Results are striking: Claude 3.5 Sonnet's AIME accuracy more than doubled; GPT-4o on
Game of 24 went from 10% to 99%. The mechanism is simple -- the cheatsheet is just a
structured text blob appended to the context window -- but the effect is large because
the agent stops repeating mistakes and accumulates transferable strategies.

**The limitation**: Dynamic Cheatsheet is per-agent and per-session. When the session ends,
the cheatsheet is gone. A new agent instance starts from zero. There is no mechanism for
sharing cheatsheets across agents, across sessions, or across organizations.

**TC is the cross-agent, cross-organization Dynamic Cheatsheet.** This is the sharpest
framing of TC's value proposition in the recent literature. Suzgun et al. demonstrate that
a curated memory of strategies produces dramatic gains for a single agent. TC provides the
infrastructure for those gains to propagate across the entire population of agents that
query the corpus:

- Where Dynamic Cheatsheet accumulates strategies within one session, TC accumulates them
  across all contributors -- ~352 submissions from 3 contributors today, scaling with
  growth.
- Where Dynamic Cheatsheet's memory disappears at session end, TC's traces persist
  indefinitely with quality scores and outcome labels.
- Where Dynamic Cheatsheet has no quality control (the agent decides what to remember),
  TC's gate pipeline scores every trace in a TEE -- only gate-approved traces enter the
  retrieval corpus.
- Where Dynamic Cheatsheet has no privacy story (the agent sees its own traces only),
  TC's TEE + redaction infrastructure enables cross-user retrieval without exposing
  contributor identities or proprietary code.

The Dynamic Cheatsheet result validates TC's thesis that a curated corpus of agent
strategies improves downstream performance. TC externalizes and scales the mechanism.

### 7.2 ReasoningBank -- Distilling Memory from Trajectories

**Google DeepMind/UIUC (2026)** "ReasoningBank: Scaling Agent Self-Evolving with Reasoning
Memory" arXiv:2509.25140 (ICLR 2026)

ReasoningBank distills generalizable reasoning strategies from agent experiences --
both successful AND failed. Rather than retrieving raw trajectories and pasting them into
context, ReasoningBank processes trajectories into compact, reusable "reasoning memories":
abstracted strategies that capture *why* an approach worked or failed, stripped of
task-specific details.

The key mechanism is MaTTS (Memory-Aware Test-Time Scaling): the system allocates more
compute at test-time to produce richer contrastive signals -- comparing what worked against
what failed -- and distills the contrast into higher-quality memory entries. Failure traces
are first-class signals, not discarded noise. A failed trace that exposes a subtle bug is
as valuable as a successful trace that demonstrates the fix, because the contrast between
them yields a more robust strategy than either alone.

**TC relevance**: TC's outcome-labeled traces with quality scores are exactly the training
signal ReasoningBank-style systems consume. TC already stores both successful and failed
traces, and the gate pipeline scores them -- providing the quality signal that determines
which traces are worth distilling from.

The implication for TC's trajectory RAG pipeline: once TC retrieves relevant traces
(Section 4), a distillation layer that processes retrieved traces into compact reasoning
memories would produce better downstream agent performance than verbatim retrieval.
ReasoningBank shows the gains from distillation are significant, and TC's corpus breadth
compounds the value -- an agent distilling from a diverse, quality-scored, cross-organization
corpus will extract more generalizable strategies than one distilling from its own limited
history.

This positions TC not just as a retrieval endpoint but as a reasoning memory service:
agents query TC, TC returns distilled strategies (not raw traces), and agents improve
without retraining. The network effect strengthens: more traces from more contributors
produce more diverse contrastive signals, yielding higher-quality distilled memories.

### 7.3 LEGOMem -- Typed Decomposition of Trajectory Memory

**Han et al. (2026)** "LEGOMem: Modular Procedural Memory for Multi-agent LLM Systems
for Workflow Automation" arXiv:2510.04851 (AAMAS 2026)

LEGOMem decomposes past trajectories into typed, reusable memory units and allocates them
across different levels of a multi-agent system. The key finding: orchestrator-level memory
(planning strategies, task decomposition patterns) and agent-level memory (tool usage,
error recovery steps) serve fundamentally different roles and should be stored, indexed,
and retrieved separately.

Even smaller models narrow the performance gap with larger models when given access to
well-structured procedural memory -- the memory compensates for model capacity.

**TC design insight**: TC currently treats traces as monolithic units. A trace is submitted,
scored, and indexed as a single artifact. LEGOMem suggests that traces should be decomposed
into typed units *before* indexing:

- **Planning spans**: how the agent decomposed the task, what sub-goals it identified, how
  it sequenced work. Useful for orchestrator-level retrieval ("show me how agents planned
  similar projects").
- **Execution spans**: specific tool calls, error recovery sequences, API interactions.
  Useful for agent-level retrieval ("show me how agents handled this specific error").
- **Decision points**: moments where the agent chose between alternatives. Useful for
  strategy retrieval ("what options did agents consider for this kind of problem?").

This decomposition is a Phase 2+ enhancement (Section 9, Phase 3). TC's existing chunker
and event parser (Section 5) provide the machinery to split traces into typed spans. The
multi-view embedding strategy in Section 4.2 already contemplates structural and outcome
embeddings -- LEGOMem adds the argument that these views should correspond to typed spans,
not just different embedding spaces over the same monolithic trace.

Typed decomposition also improves context budget management (Section 5): instead of
retrieving a full 10K+ token trace when the agent only needs the error recovery sequence,
TC can return just the relevant execution span.

### 7.4 Sleep-time Compute -- Pre-Processing Traces During Idle

**Lin et al. (2026)** "Sleep-time Compute: Beyond Inference Scaling at Test-time"
arXiv:2504.13171 (Letta/UC Berkeley)

Sleep-time compute extends the scaling paradigm beyond test-time: LLM agents "think" about
their persistent context *offline*, pre-computing useful quantities during idle periods
before any query arrives. When a query does arrive, the agent has already pre-processed
relevant context and can respond faster and more accurately. On Stateful GSM-Symbolic, this
reduces test-time compute by approximately 5x while maintaining or improving accuracy.

**TC relevance**: TC traces are natural candidates for sleep-time pre-computation. An agent
runtime integrated with TC could pre-fetch and pre-process relevant traces during idle --
extracting strategies, indexing error patterns, distilling reasoning memories (per
Section 7.2) -- so that when the next task arrives, the agent already has trajectory
context loaded and processed.

TC's NEAR credit mechanism could extend to incentivize hosting pre-computation services:
contributors who run sleep-time processing infrastructure on TC's corpus earn additional
credits, similar to how contributors currently earn credits for submitting traces. This
is speculative and depends on corpus growth, but the economic model aligns.

### 7.5 RHO -- Retrospective Optimization from Trajectory Corpora

**City University of Hong Kong + Microsoft Research Asia (2026)** "Evolving Agents in the
Dark: Retrospective Harness Optimization via Self-Preference"
arXiv:2606.05922

RHO (Retrospective Harness Optimization) demonstrates a self-supervised loop for improving
agent test harnesses: the system selects diverse past tasks from a trajectory corpus,
re-solves them, self-validates the results, and generates harness updates -- all without
human labels. On SWE-Bench Pro, this improves pass rate from 59% to 78% (a 19 percentage
point improvement) in a single optimization round.

The mechanism is "retrospective" because the agent looks backward at its own history, and
"self-preference" because it uses its own judgment (not external labels) to assess which
solutions are better. The trajectory corpus is the input; the improved harness is the
output.

**TC relevance**: TC traces are exactly the "past trajectories" that RHO-style systems
optimize from. An agent runtime integrated with TC could run retrospective optimization
cycles using TC's corpus, with TC's quality scores serving as the self-preference signal
(replacing or supplementing the agent's own judgment). This is particularly powerful
because TC's corpus spans multiple contributors and organizations -- the agent optimizes
from a broader set of experiences than its own history alone.

The 59% to 78% improvement on SWE-Bench Pro in one round suggests that even modest
trajectory corpora can produce meaningful gains. At TC's current ~352 traces, there may
already be enough data for domain-specific RHO optimization (e.g., within the Python web
development cluster), though broader gains require corpus growth.

---

## 8. Product Implications

### 8.1 From Passive Registry to Active Knowledge Base

Without trajectory RAG, TC is a one-directional repository: contributors submit, TC scores,
credits are paid, non-contributors have no reason to use TC. With trajectory RAG, value
flows both directions: contribute data AND query the accumulated knowledge of the corpus.

### 8.2 Monetization

- **API access**: per-query or per-trace pricing. Usage-based aligns incentives.
- **Tiered access**: free (basic retrieval), paid (BQP diversity re-ranking, structural +
  outcome matching, sub-trace retrieval, higher rate limits).
- **Revenue sharing**: portion of API revenue flows to contributors whose traces are
  retrieved, creating a second economic incentive beyond credits.

### 8.3 Network Effects

`More traces -> better retrieval -> more users -> more traces.` TC's gate pipeline adds
a quality dimension most data network effects lack: more traces means more signal, not
more noise.

### 8.4 Competitive Moat

| Capability | TC | Langfuse | Braintrust | LangSmith |
|---|---|---|---|---|
| Trace storage | Yes | Yes | Yes | Yes |
| Quality scoring | TEE-based | No | No | No |
| Cross-user retrieval | Yes (redacted, TEE) | No (per-org) | No (per-org) | No (per-org) |
| Trajectory RAG | Planned | No | No | No |
| Privacy guarantees | TEE + redaction | Standard | Standard | Standard |
| Contributor compensation | NEAR credits | No | No | No |

Note on Langfuse: ClickHouse acquired Langfuse on January 16, 2026 (not Databricks, as
sometimes reported), alongside a $400M Series D led by Dragoneer that tripled ClickHouse's
valuation to approximately $15B. Langfuse remains the leading open-source LLM observability
platform; the acquisition gives it ClickHouse's analytical database infrastructure as a
backend. This does not change TC's competitive position: Langfuse is per-org trace storage
with no cross-user retrieval, no TEE-based scoring, and no contributor compensation.

Cross-user retrieval + TEE privacy + quality scoring is TC's structural advantage. Any
competitor can build trace storage. The trust infrastructure for cross-user retrieval --
TEEs, redaction, quality gate -- is TC's existing stack. None of the competitors (Langfuse,
Braintrust, LangSmith) offer cross-user trajectory retrieval regardless of their corporate
backing.

---

## 9. Implementation Roadmap

### Phase 1: Basic Retrieval (Days)

- Expose TC's existing HNSW index via a query API endpoint
- Accept structured queries (task + recent_actions + errors)
- Return top-10 quality-filtered traces
- No MMR yet -- just raw cosine similarity with quality floor

**Validation**: can a user query TC and get back relevant traces? Manual evaluation on
10-20 synthetic queries.

### Phase 2: Diversity + Hybrid (1-2 Weeks)

- Add BQP diversity re-ranking (arXiv:2604.02554); MMR acceptable as initial fallback
- Add BM25 index via tantivy
- Implement RRF fusion of BM25 + dense results
- Add rate limiting and contributor diversity minimum

**Validation**: compare top-10 results with and without diversity re-ranking on 50 queries.
Measure result diversity (mean pairwise distance in embedding space). BQP should increase
diversity over vanilla cosine top-k without catastrophically reducing relevance. Measure
BQP vs MMR latency to confirm the 2.4-22.9x speedup at TC's theta >= 0.5 operating point.

### Phase 3: Multi-View + Compression + Typed Decomposition (1-2 Months)

- Add structural embedding (tool-call sequence)
- Add outcome embedding (success/failure + error type)
- Implement multi-view minimum-similarity ranking
- Add trace summarization for context budget management
- Sub-trace fragment retrieval
- Typed span decomposition (LEGOMem, Section 7.3): split traces into planning spans,
  execution spans, and decision points before indexing. Reuse existing chunker + event
  parser (Section 5). Enables span-level retrieval for better context budget efficiency.

**Validation**: A/B test multi-view vs content-only retrieval. Measure downstream agent
task completion rate when given retrieved traces. For typed decomposition: compare span-level
retrieval against full-trace retrieval on context budget utilization and relevance.

### Phase 4: Scale + Distillation + Optimization (Corpus-Dependent)

- Hierarchical retrieval for 10K+ trace categories
- Pre-computed cluster centroids
- Approximate nearest neighbor (ScaNN/FAISS IVF) if latency requires
- Learned retrieval model (LRAT-style, trained on TC's own query-trace pairs)
- Distillation layer (ReasoningBank, Section 7.2): process retrieved traces into compact
  reasoning memories (abstracted strategies) rather than returning verbatim traces.
  Requires sufficient corpus diversity for contrastive distillation to produce meaningful
  abstractions -- revisit at 1,000+ traces per category.
- Retrospective optimization endpoint (RHO, Section 7.5): expose TC corpus as input for
  agent runtimes running self-supervised harness optimization cycles

**Validation**: latency benchmarks at projected corpus sizes. p95 query latency < 200ms.

---

## 10. Open Questions

1. **Unit of retrieval?** Full traces vs sub-traces vs tool-call sequences. Start with full
   traces; measure whether users need finer granularity (doc 02 B.6).

2. **How to evaluate retrieval quality?** TC cannot directly measure downstream task
   improvement. Proxy metrics: click-through, explicit feedback (thumbs up/down), A/B
   testing with IronClaw.

3. **Train a custom retriever?** LRAT shows trajectory-specific retrievers outperform
   generic models. At ~352 traces, too small to train. Revisit at 5,000+.

4. **Credit integration?** If a trace is frequently retrieved, should its contributor earn
   additional credits? VCG (doc 02 C.7) could incorporate retrieval frequency.

5. **Query pattern distribution?** Hypotheses: error-driven most common, task-driven most
   valuable, exploration queries rarest but highest-signal for diversity re-ranking.

6. **Context budget?** T3 suggests 1-3 high-quality traces produce gains. More is not
   necessarily better. Current frontier models have 128K-200K windows but effective
   utilization degrades well before the limit.

7. **Verbatim retrieval vs distillation?** ReasoningBank (Section 7.2) shows distilled
   reasoning memories outperform raw trajectories. Should TC return verbatim traces,
   distilled strategies, or both? Distillation requires compute (LLM calls to abstract
   strategies from traces) and a sufficiently diverse corpus for contrastive signals.
   At ~352 traces, verbatim retrieval is the pragmatic starting point. Revisit distillation
   at 1,000+ traces per category.

8. **Span-level vs trace-level retrieval?** LEGOMem (Section 7.3) argues for typed
   decomposition: planning spans, execution spans, decision points. This is more
   precise but requires upfront decomposition of all traces. Does the precision gain
   justify the indexing cost? Measure at Phase 3.

9. **Sleep-time pre-computation economics?** Sleep-time compute (Section 7.4) suggests
   agents should pre-process TC traces during idle. Who pays for idle-time compute?
   TC's NEAR credit model could extend to pre-computation providers, but the demand
   signal is unclear at current corpus size.

---

## 11. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Corpus too small for useful retrieval** | High (current state) | Trajectory RAG is growth-dependent. Build pipeline, but primary investment should be in corpus growth (doc 01). |
| **Top-k collapse degrades results** | High (at scale) | BQP diversity re-ranking from Phase 2. Monitor result diversity metrics. MMR as initial fallback. |
| **Privacy breach via retrieval** | Critical | TEE query processing, redacted-only results, rate limiting, contributor diversity minimums. |
| **Retrieval latency too high** | Medium | Phase 1 reuses existing HNSW index. Add approximate methods only if needed. |
| **Retrieved traces mislead agents** | Medium | Quality filtering ensures only gate-approved traces are returned. Add explicit "this trace may not apply to your situation" disclaimer. |
| **Low adoption** | Medium | Trajectory RAG is novel. Developers may not know to query TC. Integrate into agent frameworks (IronClaw, Claude Code hooks) so retrieval happens automatically. |
| **Issue #210 blocks everything** | Critical | The gate pipeline must work (0/99 accepted = no quality-scored traces = nothing to retrieve). Fix #210 first. |

---

## 12. Verification Ledger

| Paper | arXiv | Status |
|---|---|---|
| LRAT: Learning to Retrieve from Agent Trajectories (Zhou et al.) | 2604.04949 | **Verified** -- title, authors, and content confirmed via arXiv |
| ExpRAG: Retrieval-Augmented LLM Agents: Learning to Learn from Experience (Ferraz et al.) | 2603.18272 | **Verified** -- title, authors, and key findings confirmed via arXiv; note "ExpRAG" is an internal shorthand, not the paper's title |
| RISE: Towards Retrieving Interaction Spaces for Agentic Search (Zhuang et al.) | 2606.06880 | **Verified** -- title, authors confirmed; 81% on 1M docs, 75% cost reduction confirmed |
| "Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation" (Hu et al.) | 2602.02007 | **Verified** -- full title confirmed; top-k collapse finding confirmed. Note: "xMemory" is an internal shorthand used in earlier drafts of TC docs; this name does NOT appear in the paper. The correct title is "Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation." |
| AgentIR-4B: Reasoning-Aware Retrieval for Deep Research Agents (Chen et al.) | 2603.04384 | **Verified** -- 4B params confirmed; 68% vs 50% (2x larger embedder) vs 37% (BM25) on BrowseComp-Plus confirmed; open-source on HuggingFace (Tevatron/AgentIR-4B) |
| T3: RAG over Thinking Traces Can Improve Reasoning Tasks (Arabzadeh et al.) | 2605.03344 | **Verified** -- title confirmed; +56.3% on AIME 2025-2026 confirmed; authors: Arabzadeh, Ma, Min, Zaharia |
| MMR: Maximal Marginal Relevance (Carbonell & Goldstein, 1998) | N/A | **Verified** -- SIGIR 1998 paper, foundational diversity re-ranking method. Noted limitation: no approximation guarantee (non-monotone submodular). Retained in ledger as MMR remains an acceptable initial fallback. |
| BQP diversity retrieval (cardinality-constrained Binary Quadratic Programming) | 2604.02554 | **Verified** -- 2.4-22.9x faster than MMR at theta >= 0.5; sublinear scaling in k confirmed; MMR approximation-guarantee absence confirmed; DPP slower than MMR confirmed |
| Dynamic Cheatsheet: Test-Time Learning with Adaptive Memory (Suzgun et al.) | 2504.07952 | **Verified** -- ICLR 2026; Claude 3.5 Sonnet AIME accuracy more than doubled; GPT-4o Game of 24: 10% to 99%; per-agent per-session memory with no cross-agent sharing |
| ReasoningBank: Scaling Agent Self-Evolving with Reasoning Memory (Google DeepMind/UIUC) | 2509.25140 | **Verified** -- ICLR 2026; distills reasoning strategies from both successful and failed experiences; introduces MaTTS (Memory-Aware Test-Time Scaling) for contrastive distillation |
| LEGOMem: Modular Procedural Memory for Multi-agent LLM Systems (Han et al.) | 2510.04851 | **Verified** -- AAMAS 2026; typed memory decomposition (orchestrator vs agent level); smaller models narrow gap with larger models via procedural memory |
| Sleep-time Compute: Beyond Inference Scaling at Test-time (Lin et al.) | 2504.13171 | **Verified** -- Letta/UC Berkeley; ~5x test-time compute reduction on Stateful GSM-Symbolic via offline pre-computation of persistent context |
| RHO: Evolving Agents in the Dark: Retrospective Harness Optimization via Self-Preference (City Univ. HK + MSRA) | 2606.05922 | **Verified** -- self-supervised harness optimization from trajectory corpus; SWE-Bench Pro: 59% to 78% pass rate (19pp) in one round. Note: NOT "Retrieval-augmented Hierarchical Orchestration" -- full title is "Evolving Agents in the Dark: Retrospective Harness Optimization via Self-Preference" |

All six trajectory RAG papers (Section 2), both diversity retrieval references (Section 3),
and five beyond-retrieval papers (Section 7) verified against arXiv as of August 2026.
No unverified citations in this document.
