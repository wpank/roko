# 07 -- TraceCommons Trace Quality and Trajectory RAG

**Date**: August 2026

Every time a coding agent successfully completes a task -- writes a function, fixes a bug,
refactors a module -- that execution trace is valuable learning data. TraceCommons is an
open commons that collects, quality-gates, and shares these traces so every agent gets
better from every other agent's experience. This is what makes tiagent collectively
self-improving.

This document describes how tiagent integrates with TraceCommons (TC). The integration
gives tiagent agents access to a shared corpus of scored, quality-assessed execution
traces -- enabling agents to learn from the successes and failures of other agents across
harnesses, models, and organizations.

---

## 1. What is TraceCommons?

TraceCommons is an open-source, Rust-based, privacy-preserving registry of AI coding
agent session traces. Built by Zaki Manian (co-creator of Cosmos SDK / IBC), it consists
of approximately 235K lines of Rust across 6 crates, dual-licensed MIT/Apache-2.0, with a
pilot deployment on GCP.

The core concept: developers using AI coding assistants (Claude Code, Codex, Cursor,
IronClaw, and others) generate session traces as a byproduct of their work -- records of
what the agent did, which tools it called, what worked, and what failed. TraceCommons
collects these traces (with privacy scrubbing), scores them for quality and novelty inside
Trusted Execution Environments (TEEs -- hardware-isolated encrypted compute enclaves
using Intel TDX and NVIDIA GPU TEE), and compensates contributors with credits on the
NEAR blockchain.

The goal is a shared, contributor-owned corpus of agent behavior data that no single
vendor controls. No existing LLM observability platform (Langfuse, Braintrust, LangSmith)
offers cross-user retrieval -- they all store traces per-organization. TraceCommons is
the only system that combines cross-user retrieval with TEE-based privacy guarantees and
contributor compensation.

### How the pipeline works

A submitted trace passes through a multi-stage gate pipeline:

1. **Redaction** -- PII and secret scrubbing
2. **Chunking** -- structural decomposition
3. **Embedding** -- BGE-large-en-v1.5 vector encoding
4. **Similarity** -- cosine similarity against an HNSW index for novelty detection
5. **Perplexity scoring** -- via Qwen 3.6 35B running on NEAR AI Cloud TEE
6. **Gate evaluation** -- accept/reject decision

The credit formula is `q = f * g * a` where f = quality factor, g = novelty factor,
a = anomaly penalty. Contributors earn NEAR-denominated credits for accepted traces.

### Current state (August 2026)

| Metric | Value |
|---|---|
| Submissions | ~352 |
| Weekly ingest | ~13 |
| Contributors | 3 |
| GitHub stars | 6 |
| Primary integration | IronClaw (NEAR AI agent runtime, 12.6K stars) |
| Ingest formats | IronClaw native, Claude Code SessionEnd hook, Codex OTel |

The corpus is early-stage. The network effects that make TraceCommons valuable --
more traces lead to better retrieval, which attracts more users, who submit more
traces -- have not yet kicked in. tiagent's integration is both a consumer of the
commons and a contributor that helps bootstrap the corpus.

---

## 2. Why TraceCommons for tiagent?

### Agents learn from past executions

tiagent agents execute tasks, interact with tools, encounter errors, and develop
strategies for solving problems. Each execution produces a trace -- a structured record
of the agent's behavior. Without a shared corpus, each agent learns only from its own
history. With TraceCommons, agents learn from the collective experience of every
contributing harness.

**Concrete example for regular development work:**

1. Developer A's tiagent successfully implements JWT authentication in a FastAPI app.
   The agent writes the middleware, configures token validation, adds tests, and all
   gates pass.
2. That execution trace -- the full sequence of tool calls, file edits, test results,
   and the reasoning that connected them -- is submitted to TraceCommons.
3. Developer B, working on a completely different project, asks their tiagent to add
   JWT auth to their own FastAPI service. Trajectory RAG retrieves Developer A's trace
   as a relevant prior execution.
4. Developer B's agent uses the retrieved trajectory as in-context learning: it sees
   which files Developer A created, what order they were edited in, which test patterns
   caught edge cases, and how the middleware was structured.
5. Result: Developer B's agent completes the task faster and with fewer errors, even
   though it never saw that specific codebase before. It learned from another
   developer's successful session, not from documentation or training data.

This works for any task, not just auth -- database migrations, CI pipeline setup,
API refactors, dependency upgrades, bug fixes. Every successful agent session becomes
a reusable template for the next developer who faces a similar problem.

Research validates this approach. Dynamic Cheatsheet (Suzgun et al., ICLR 2026,
arXiv:2504.07952) demonstrated that giving an agent a persistent, evolving memory of
strategies from prior attempts produces dramatic gains: Claude 3.5 Sonnet's AIME accuracy
more than doubled; GPT-4o on Game of 24 went from 10% to 99%. The mechanism is simple
-- a structured text blob appended to the context window -- but the effect is large
because the agent stops repeating mistakes and accumulates transferable strategies.

The limitation of Dynamic Cheatsheet is that it is per-agent and per-session. When the
session ends, the memory is gone. TraceCommons is the cross-agent, cross-organization
version of this. Where Dynamic Cheatsheet accumulates strategies within one session,
TraceCommons accumulates them across all contributors. Where the cheatsheet disappears
at session end, TraceCommons traces persist indefinitely with quality scores and outcome
labels. Where the cheatsheet has no quality control, TraceCommons scores every trace
in a TEE.

### Combined with Celestia DA

tiagent publishes execution traces to Celestia's Data Availability layer for
verifiability and permanence. TraceCommons adds a quality dimension to that publication.
A trace published to Celestia DA is verifiable (anyone can confirm it exists and was not
tampered with). A trace scored by TraceCommons is also quality-assessed (the gate
pipeline determines whether the trace represents a successful, novel contribution to the
corpus). Dual publication -- Celestia DA for verifiability, TraceCommons for
searchability and quality scoring -- gives tiagent traces both properties.

### Rising tide lifts all boats

The network effect is the key insight: every high-quality trace that tiagent contributes
to TraceCommons improves the retrieval corpus for all consumers. Every high-quality trace
from other contributors improves tiagent's agents. This is a positive-sum dynamic that
does not exist when traces are siloed per-organization.

Research on retrospective harness optimization (RHO, arXiv:2606.05922) demonstrates
that agents can self-improve from trajectory corpora: on SWE-Bench Pro, pass rates
improved from 59% to 78% in a single optimization round. The improvement scales with
corpus diversity -- an agent optimizing from a cross-organization corpus extracts more
generalizable strategies than one limited to its own history.

---

## 3. Trace Submission

### How tiagent formats traces

tiagent's internal episode format (JSONL records of agent turns, tool calls, gate
results, and outcomes) must be adapted to TraceCommons' canonical trace envelope.
The envelope captures:

- Provider and model identity
- Tool call sequence with timestamps
- Token usage per LLM call
- Redacted content references
- Provenance tier (1/2/3)
- Source format tag

tiagent produces a `TraceContributionEnvelope` from its episode data, mapping
fields as follows:

| tiagent field | TC envelope field |
|---|---|
| Agent model + backend | `provider`, `model` |
| Tool invocations (MCP calls) | `ToolCallEvent` sequence |
| Token counts per turn | `input_token_count`, `output_token_count` |
| Task outcome (pass/fail) | Outcome label |
| Gate results (compile, test, clippy) | Quality metadata |
| Episode timestamp | Trace timestamp |

### Schema compatibility

TraceCommons supports three ingest paths. tiagent uses a combination:

1. **Native integration** -- a bespoke parser that reads tiagent's episode format and
   produces the canonical trace envelope. This is the simplest path and does not
   require OTel instrumentation.

2. **OTel OTLP** -- tiagent can optionally emit OpenTelemetry spans for its agent
   executions. TraceCommons accepts OTLP via gRPC and HTTP/protobuf. The OTel GenAI
   semantic conventions (`gen_ai.*`) provide the attribute mapping, though all GenAI
   conventions remain at "Development" status as of August 2026 -- not stable. tiagent
   must pin attribute versions and handle the `gen_ai.system` to `gen_ai.provider.name`
   rename (breaking change at v1.39.0).

3. **OpenInference** -- for tiagent configurations using LangChain or LlamaIndex
   toolchains, OpenInference conventions (`llm.*`, `embedding.*`, `retriever.*`)
   provide richer RAG-specific metadata than base OTel GenAI. TraceCommons detects
   the convention set from sentinel attributes and normalizes accordingly.

### Quality thresholds for submission

Not every trace should be submitted. Submitting low-quality traces pollutes the commons
and wastes scoring compute. tiagent applies local quality gates before submission:

- **Minimum trace length** -- traces shorter than a meaningful threshold (e.g., single
  tool call with no outcome) are filtered out
- **Outcome requirement** -- traces must have a determinable outcome (success, failure
  with diagnosis, or partial completion with documented state)
- **Redaction verification** -- tiagent runs its own redaction pass before submission,
  removing PII, secrets, and project-specific identifiers. This is critical because
  TraceCommons' Issue #219 documented that thorough redaction is penalized by the
  perplexity scorer (redaction markers appear as incoherent noise). The fix (TEE raw
  scoring before redaction, confirmed viable via TDX attestation analysis) is in
  progress on the TC side. Until then, tiagent's pre-submission redaction should be
  calibrated to balance privacy protection against scoring impact.
- **Deduplication** -- traces that are near-duplicates of previously submitted traces
  (detected via MinHash or embedding similarity) are suppressed

### Dual publication: Celestia DA + TraceCommons

tiagent publishes traces through two channels simultaneously:

1. **Celestia DA** -- the trace blob is posted to Celestia's data availability layer.
   This provides a tamper-proof, timestamped record that the trace exists. The DA
   commitment serves as a provenance anchor -- anyone can verify that a specific trace
   was published at a specific time by a specific namespace.

2. **TraceCommons** -- the same trace (or a quality-filtered subset) is submitted to
   the TC ingest endpoint. TC scores it for quality and novelty, indexes it for
   retrieval, and awards credits if accepted.

The Celestia DA commitment can be included in the TC submission as a provenance
attestation artifact, strengthening the trace's provenance tier in TC's system. This
creates a complementary trust stack: Celestia provides data availability and ordering
guarantees; TraceCommons provides quality assessment and searchability.

**Note: TraceCommons works independently of Celestia.** Traces can be submitted directly
to TraceCommons without going through any DA layer. The dual-publication path (Celestia
DA + TraceCommons) is an optional configuration that adds verifiability on top of the
quality scoring and retrieval that TraceCommons provides on its own. Developers who have
no interest in blockchain-based provenance can use TraceCommons purely as a shared trace
corpus -- submit traces, earn quality scores, and retrieve other developers' traces via
trajectory RAG -- without touching Celestia at all.

---

## 4. Trajectory RAG (Retrieval-Augmented Generation from Trajectories)

Trajectory RAG is TraceCommons' killer feature for developers, and the primary reason
tiagent integrates as a consumer (not just a contributor). Think of it this way: your
coding agent has access to a library of successful coding sessions from other developers.
Not documentation. Not Q&A threads. Full execution traces -- the exact sequence of tool
calls, file edits, test runs, error recoveries, and reasoning that led to a working
result.

This is like Stack Overflow for agents, but instead of getting a code snippet and a
prose explanation, your agent gets the complete record of how another agent actually
solved the problem: what it tried, what failed, what worked, and in what order.

Traditional RAG retrieves text passages to answer factual questions. Trajectory RAG
retrieves structured records of agent behavior -- tool-call sequences, decision points,
error recovery strategies, multi-step reasoning chains -- to inform an agent's approach
to a new problem. A document tells you what is true; a trajectory shows you how to get
something done.

### How tiagent consumes traces

When a tiagent agent faces a task, it queries TraceCommons before (or during) execution:

```
Agent receives task
  -> tiagent constructs a structured query from the task context
  -> Query sent to TraceCommons retrieval API
  -> TC returns relevant, quality-filtered, diverse traces
  -> tiagent injects trajectory segments into the agent's prompt context
  -> Agent executes the task informed by past successful approaches
```

### Retrieval: finding similar past trajectories

The query is not a bare string. Research on reasoning-aware retrieval (AgentIR,
arXiv:2603.04384) demonstrates that agent-context-aware embedding dramatically
outperforms generic embedding: 68% accuracy vs 50% from conventional embedders twice
its size. tiagent sends structured queries that include the agent's full context:

```json
{
  "task": "Deploy a Rust service with health checks and readiness probes",
  "recent_actions": ["cargo build", "docker build", "kubectl apply"],
  "errors": ["CrashLoopBackOff: health check timeout"],
  "reasoning": "The readiness probe path might be wrong"
}
```

TraceCommons' retrieval pipeline processes this query through five stages:

1. **Query encoding** -- embed the full structured context, not just the task description
2. **Candidate retrieval** -- hybrid BM25 (lexical, catches error messages and tool
   names) plus dense retrieval (semantic similarity). Merged by reciprocal rank fusion.
   Top-100 candidates.
3. **Diversity re-ranking** -- cardinality-constrained Binary Quadratic Programming
   (BQP, arXiv:2604.02554) over the candidate set. BQP is 2.4-22.9x faster than MMR
   (Maximal Marginal Relevance) at the practically relevant similarity threshold and
   scales sublinearly in k. This prevents top-k collapse, where naive cosine similarity
   returns results from the densest cluster in embedding space regardless of actual
   relevance.
4. **Quality filtering** -- only traces above TC's gate quality threshold are returned
5. **Privacy filtering** -- all results are redacted; query embeddings computed in TEE;
   minimum 5 distinct contributors per result set to prevent de-anonymization

### Augmentation: injecting trajectory context

tiagent injects retrieved trajectories into the agent's prompt context. The injection
strategy must respect the context budget -- a single trace can be 10K+ tokens, and five
traces would consume 50K tokens before the agent starts working. tiagent uses several
strategies:

- **Summary-first** -- TraceCommons returns trace summaries (compressed via trajectory
  compression techniques that achieve 10-50x compression). The agent sees summaries and
  can request full traces for the most relevant ones.
- **Sub-trace fragments** -- rather than full traces, retrieve specific segments:
  planning spans (how the agent decomposed the task), execution spans (specific tool
  calls and error recovery), or decision points (where the agent chose between
  alternatives). This typed decomposition follows the LEGOMem architecture
  (arXiv:2510.04851, AAMAS 2026).
- **Multi-view matching** -- TraceCommons supports content embedding (text similarity),
  structural embedding (tool-call sequence similarity), and outcome embedding
  (success/failure pattern matching). The final relevance score uses the minimum
  similarity across views, preventing content-only matches from dominating when
  structural relevance is absent.

### Benefits

The research literature validates three key benefits of trajectory RAG:

1. **Learning without training** -- agents learn from successful patterns without
   fine-tuning or retraining. T3 (arXiv:2605.03344) demonstrates that RAG over
   thinking traces achieves +56.3% relative gains on AIME benchmarks. The retrieval
   corpus matters more than the retrieval method.

2. **Cross-agent knowledge transfer** -- ExpRAG (arXiv:2603.18272) shows that combining
   experience retrieval with fine-tuning "significantly improves generalization to
   unseen tasks." Neither alone matches the combined approach. TraceCommons' cross-user
   corpus makes this transfer happen across organizations, not just within a single
   agent's history.

3. **Error recovery** -- when a tiagent agent encounters an error, it can query
   TraceCommons for traces where other agents encountered and resolved similar errors.
   AgentDebugX (arXiv:2607.18754) demonstrated that sharing failure-diagnosis-repair
   bundles improved GAIA task accuracy from 55.8% to 63.6%.

---

## 5. Quality Scoring Integration

### TraceCommons quality signals

TraceCommons' gate pipeline produces several quality signals that tiagent uses for
trace filtering and weighting:

- **Gate score** -- the primary accept/reject decision from the multi-stage pipeline
  (redaction, chunking, embedding, similarity, perplexity scoring, gate evaluation)
- **Novelty score** -- how different the trace is from existing corpus entries, computed
  via cosine similarity against the HNSW index
- **Anomaly penalty** -- flags for suspicious patterns (potential Sybil submissions,
  automated low-quality dumps)
- **Contributor reputation** -- Glicko-2 rating for the submitting contributor, updated
  based on the quality of their submissions over time

### Conformal prediction gates

TraceCommons is adopting conformal prediction (ToolChain-CRC) for quality gate
calibration. Conformal prediction provides distribution-free coverage guarantees --
the gate can promise "95% of accepted traces are truly above quality threshold X"
without assumptions about the underlying score distribution. For small calibration
sets, SSBC (Small-Sample Bias Correction, arXiv:2509.15349) provides valid coverage
starting at n=47.

tiagent consumes these calibrated quality scores as trust signals. A trace that passed
a conformally-calibrated gate carries a stronger quality guarantee than one that passed
a heuristic threshold. tiagent can set its own consumption threshold based on the
coverage guarantee: "only retrieve traces where the conformal gate guarantees 95%
quality coverage."

### Quality-weighted RAG retrieval

Higher-quality traces get more influence in trajectory RAG results. tiagent weights
retrieved traces by their gate score when constructing prompt context:

- Traces at the top of the quality distribution are presented first and in more detail
- Traces near the quality threshold are presented as summaries only
- The diversity re-ranker (BQP) operates over the quality-filtered candidate set, so
  diversity is achieved within the high-quality subset, not at the expense of quality

This creates a virtuous cycle for contributors: higher-quality submissions earn more
NEAR credits (via the credit formula) AND get retrieved more frequently (via quality-
weighted RAG). The incentive alignment is that quality always dominates quantity.

### HDC fingerprinting

TraceCommons uses Hyperdimensional Computing (HDC) hypervectors for structural
fingerprinting. At the current corpus scale (~352 traces), brute-force exact scan
over HDC hypervectors is the recommended retrieval mechanism inside TEEs. This approach
is simultaneously side-channel-free (sequential memory access with no query-dependent
branching, unlike HNSW's graph traversal which leaks information via cache-timing
side channels) and deterministic (same query + same corpus = same results, making
attestation meaningful). tiagent's own HDC fingerprinting (used for episode
fingerprinting) is compatible with this representation, allowing a unified vector
space across local and commons-based retrieval.

---

## 6. Credit and Incentive System

### Trace producers earn credit

Contributors whose traces are accepted by TraceCommons' gate pipeline earn NEAR
blockchain credits. The credit formula `q = f * g * a` rewards:

- **f (quality factor)** -- higher-quality traces earn proportionally more
- **g (novelty factor)** -- novel traces that add new information to the corpus earn
  more than near-duplicates of existing entries
- **a (anomaly penalty)** -- traces flagged for suspicious patterns (potential gaming,
  automated generation without genuine agent interaction) receive reduced or zero
  credit

### Credit tracked on-chain

All credit settlements happen on the NEAR blockchain, providing transparency and
auditability. Every contributor can verify their credit history, and the settlement
logic is publicly inspectable. For tiagent, the Celestia DA commitment included with
each trace submission creates a cross-chain provenance link: the NEAR credit
references a trace whose existence is independently verifiable on Celestia.

### Incentive alignment: quality over quantity

The incentive design deliberately favors quality over quantity. Several mechanisms
reinforce this:

1. **Novelty scoring** -- submitting 100 near-identical traces earns less than
   submitting 10 diverse, high-quality traces. The HNSW similarity check penalizes
   redundancy.

2. **Gate pipeline** -- the multi-stage quality gate rejects traces that do not meet
   the quality threshold. Rejected traces earn zero credit regardless of volume.

3. **Retrieval-linked credit** (planned) -- when a trace is frequently retrieved via
   trajectory RAG, the contributor could earn additional credits. This creates a
   second economic incentive: traces that are useful to other agents are worth more
   than traces that sit unqueried in the corpus. VCG (Vickrey-Clarke-Groves) auction
   mechanisms are being evaluated for this, replacing Shapley-based valuation which
   has been proven gameable by three independent papers.

4. **Anti-Sybil measures** -- TEE-signed ingestion attestation, contributor reputation
   tracking (Glicko-2), and brute-force HDC scan for fingerprint deduplication prevent
   gaming the credit system through fake identities or synthetic traces. At the current
   contributor count (N=3), mechanism design research shows that N=3 collusion is
   impossible to resist via payment mechanism alone -- quality gates and provenance
   attestation carry the anti-manipulation load.

### tiagent as contributor

tiagent agents that execute tasks successfully and produce high-quality traces
contribute those traces to the commons. The credits earned flow back to the tiagent
operator, creating an economic incentive to run tiagent agents that produce genuinely
useful work (not just token-consuming busy work). The Celestia DA publication provides
an independent verifiability layer that TraceCommons' NEAR-based credit system can
reference for provenance.

---

## 7. Implementation Roadmap

### Phase 1: Local trace logging (every developer gets this out of the box)

**Scope**: tiagent logs agent executions in its native episode format. This phase
requires zero configuration, no accounts, and no external services. Every developer
running tiagent gets local trace logging automatically -- it is the foundation that
all subsequent phases build on.

- Structured JSONL recording of agent turns, tool calls, gate results, outcomes
- Episode fingerprinting via HDC hypervectors
- Local storage in tiagent's data directory
- No external submission yet -- this phase ensures tiagent produces traces that are
  rich enough to be useful

Even without TraceCommons, Phase 1 is immediately useful: developers can search their
own past agent sessions, review what worked and what failed, and debug agent behavior
from structured logs rather than scrolling through terminal output.

**Deliverables**:
- Episode logger with full tool-call sequence capture
- Outcome labeling (success / failure with error classification / partial completion)
- HDC fingerprint computation per episode
- Local query interface for searching past episodes

**Validation**: verify that episode records contain all fields required by TraceCommons'
canonical envelope schema. Run a manual mapping exercise on 10 sample episodes.

---

### Phase 2: TraceCommons schema adapter (opt-in)

**Scope**: build the adapter that converts tiagent episodes into TraceCommons'
canonical trace envelope format.

- Field mapping from tiagent episode format to TC envelope (provider, model, tool
  call sequence, token counts, outcome, timestamps)
- OTel attribute mapping with version pinning -- handle both `gen_ai.system` and
  `gen_ai.provider.name` via the alias shim pattern (~50 LOC)
- OpenInference detection and normalization for configurations using LangChain or
  LlamaIndex toolchains
- Scaffold phase metadata tagging (`scaffold.phase` attribute with values: perception,
  memory, reasoning, reflection, action) per the AgentSpec architecture
  (arXiv:2606.14674)
- TRAIL failure taxonomy annotation for failed traces: categorize failures as Reasoning
  Errors, System Execution Errors, or Planning/Coordination Errors (arXiv:2505.08638)

**Deliverables**:
- `TraceContributionEnvelope` builder from tiagent episodes
- Bidirectional format tests (round-trip tiagent episode -> TC envelope -> verify
  no data loss)
- Scaffold phase tagger
- Failure classifier using TRAIL taxonomy

**Validation**: submit 20 adapter-produced envelopes to a local TC instance and
verify they pass schema validation.

---

### Phase 3: Submission pipeline with quality gates (opt-in)

**Scope**: automated submission of quality-filtered traces to TraceCommons.

- Pre-submission quality gates (minimum length, outcome requirement, deduplication)
- Redaction pipeline -- PII scrubbing, secret removal, project-specific identifier
  stripping. Calibrated to balance privacy against the Issue #219 scoring penalty
  until TC ships the TEE raw-scoring fix.
- Dual publication: Celestia DA blob submission + TraceCommons ingest endpoint. The
  Celestia DA commitment is included in the TC submission as a provenance attestation
  artifact.
- Submission configuration: opt-in per project, configurable quality threshold,
  rate limiting to avoid overwhelming TC's ingest capacity
- Submission feedback loop: TC returns quality score + percentile + credits in the
  response. tiagent surfaces this feedback to the operator.

**Deliverables**:
- Submission CLI command (`tiagent trace submit`)
- Pre-submission quality gate pipeline
- Redaction module
- Dual publication coordinator (Celestia DA + TC)
- Submission result tracking (credits earned, acceptance rate)

**Validation**: end-to-end submission of 50 traces from real tiagent executions.
Measure acceptance rate, quality score distribution, and credit earned.

---

### Phase 4: Trajectory RAG retrieval (opt-in)

**Scope**: tiagent agents query TraceCommons for relevant traces during task execution.

- Structured query construction from task context (task description, recent actions,
  errors, agent reasoning)
- TraceCommons retrieval API integration -- consume the five-stage pipeline (query
  encoding, candidate retrieval, BQP diversity re-ranking, quality filtering, privacy
  filtering)
- Context injection strategy: summary-first presentation with on-demand expansion,
  sub-trace fragment retrieval for specific spans (planning, execution, decision
  points), context budget management to avoid consuming the entire context window
  with retrieved traces
- Quality-weighted presentation: higher-quality traces presented first and in more
  detail
- Local caching of retrieved traces to avoid redundant API calls for similar queries
  within a session
- Fallback to local episode search when TraceCommons is unavailable

**Deliverables**:
- Trajectory RAG query builder
- TC retrieval API client
- Context injection module with budget management
- Local trace cache with TTL
- Integration with tiagent's system prompt builder (retrieved trajectories as a
  prompt layer)

**Validation**: A/B test agent task completion rate with and without trajectory RAG.
Measure downstream agent performance on a held-out task set. Verify that context
budget stays within bounds (retrieved traces should not exceed 30% of available
context window).

---

### Phase 5: Credit system via DA (opt-in)

**Scope**: full credit tracking and incentive integration.

- NEAR credit tracking for submitted traces -- monitor acceptance rate, credit earned
  per trace, cumulative credits
- Celestia DA provenance linking -- each TC credit settlement references the
  corresponding Celestia DA commitment, creating a cross-chain audit trail
- Contribution analytics dashboard -- what types of traces earn the most credit,
  which quality dimensions tiagent traces score highest/lowest on, how tiagent's
  acceptance rate compares to the commons average
- Retrieval-linked credit tracking (when available) -- monitor whether tiagent's
  contributed traces are being retrieved by other consumers, creating a feedback
  signal about what kinds of traces the commons values most
- Economic optimization: use credit and retrieval feedback to tune tiagent's
  pre-submission quality gates, favoring trace types that earn more credit and
  are retrieved more frequently

**Deliverables**:
- NEAR credit monitor
- Cross-chain provenance linker (Celestia DA commitment <-> NEAR credit)
- Contribution analytics
- Quality gate auto-tuning from credit feedback

**Validation**: verify end-to-end credit flow from trace submission through TC
acceptance to NEAR credit settlement. Confirm Celestia DA commitments are correctly
referenced in TC provenance metadata. Demonstrate that quality gate auto-tuning
improves acceptance rate over time.

---

## Summary

TraceCommons integration gives tiagent agents access to a shared corpus of quality-
scored execution traces. The integration operates bidirectionally: tiagent submits
high-quality traces to grow the commons (earning NEAR credits and strengthening
provenance via Celestia DA), and tiagent agents consume traces from the commons via
trajectory RAG to improve their own task execution.

The key architectural decisions are:

- **Dual publication** (Celestia DA for verifiability + TraceCommons for quality and
  search) creates a complementary trust stack
- **BQP diversity re-ranking** prevents top-k collapse in trajectory retrieval,
  ensuring agents see diverse relevant traces rather than redundant results from the
  densest corpus cluster
- **Quality-weighted RAG** ensures that higher-quality traces have more influence on
  agent behavior, aligning the retrieval system's incentives with the credit system's
  incentives
- **Conformal prediction gates** provide distribution-free quality guarantees that
  tiagent can use as calibrated trust signals
- **HDC brute-force scan** inside TEEs provides side-channel-free, deterministic
  retrieval at current corpus scale, with a clear upgrade path to approximate methods
  at 10K+ traces

The five-phase roadmap moves from local trace logging through schema adaptation,
quality-gated submission, trajectory RAG consumption, and full credit integration.
Each phase delivers standalone value while building toward the full bidirectional
integration.
