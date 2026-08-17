# Research-Backed Innovations

*v3 -- August 2026*

---

TraceCommons (TC) is an open-source Rust AI trace registry. Traces are scored
for quality and novelty inside TEEs. The pipeline is built on four traits
(`PerplexityScorer`, `TokenRarityScorer`, `Embedder`, `VectorIndex`), credit is
computed as `q = f * g * a`, and the infrastructure -- TEE scoring, SSE
streaming, NEAR settlement, background workers, 17 operational drills -- is
operational.

The innovations below are drawn from 2025-2026 research with code, benchmarks,
and in several cases open-source implementations. All build on existing traits
and pipeline stages. None require redesigning TC's architecture.

Organized into three tiers by how soon they can ship.

---

## Tier 1: Integrate Now

Existing open-source tools and standards. Less than one month of work each.
These are integration tasks, not novel engineering.

### 1.1 OpenTelemetry GenAI Ingest

**Source:** OTel `gen_ai.*` semantic conventions (v1.42.0, June 2026) +
OpenInference conventions (Arize/Phoenix).

Make `TraceContributionEnvelope` ingest/export OTel-GenAI and OpenInference
spans over OTLP via a version-pinned adapter (conventions are pre-stable; pin
attribute strings behind a version constant). W3C context propagation stitches
multi-agent handoffs automatically.

Highest-leverage item on the list. Any developer already using Langfuse, Arize,
or Datadog pipes existing telemetry into TC without changing instrumentation.
The `opentelemetry-rust` crate provides the OTLP receiver. ~2-3 weeks.

### 1.2 SKILL.md Publishing from High-Scoring Traces

**Source:** Anthropic Agent Skills (October 2025; agentskills.io; Linux
Foundation Agentic AI Foundation, 146 member orgs). ToxicSkills (Snyk,
February 2026): 36.82% of 3,984 scanned skills had security flaws; 76
confirmed malicious.

When the corpus accumulates enough high-scoring traces for a capability (e.g.,
"Rust async refactoring"), extract the common procedure and publish it as a
SKILL.md file any compatible harness can consume. Initial version is manual
curation via `tc skill publish` taking trace IDs and a skill template. SKILL.md
is markdown with YAML frontmatter -- the output format is trivial.

Creates the "contribute traces, get back skills" feedback loop. Positions TC as
a security checkpoint -- "skills from TC are scored and scrubbed" -- in a market
where a third of community skills contain flaws. ~1-2 weeks.

### 1.3 Prometheus Metrics via tower-http

**Source:** `tower-http` (v0.6.x, Tokio project), `metrics` (v0.24.x,
metrics-rs), `metrics-exporter-prometheus` (v0.16.x). All MIT.

Add a `/metrics` Prometheus endpoint exposing counters and histograms for the
gate pipeline: traces received, traces gated out by reason, scoring latency per
stage, novelty score distribution, settlement events, redaction hits, error
rates.

Table stakes for production trust. Drills verify correctness at a point in
time; continuous metrics show trends. Required foundation for compound system
auto-tuning (2.6). `tower-http` layers wrap the existing Axum router; `metrics`
provides the macros; the Prometheus exporter is a single builder call at
startup. ~1-2 weeks.

### 1.4 MinHash Dedup via Rensa

**Source:** Rensa crate (Rust MinHash, MIT; reportedly 608x faster than Python
datasketch). Broder (1997) "On the resemblance and containment of documents."

MinHash generates a fixed-size fingerprint for a document's shingle set and
estimates Jaccard similarity in O(k) time. Two traces exceeding threshold
(0.85-0.90) are near-duplicates regardless of what the embedding model thinks.
Integrates as Layer 1 in the multi-layer novelty pipeline: catches verbatim and
near-verbatim copies before the expensive embedding comparison.

Addresses the core novelty problem (PR #216) with a layer whose false-positive
rate is analytically derivable -- a known-good pre-filter that reduces the
surface area the embedding model must cover. `rensa` provides `MinHasher` and
`MinHashIndex`. The orchestrator's `evaluate` method runs MinHash before the
embedding step; if Jaccard exceeds threshold, short-circuit with a
"near-duplicate" verdict. ~1-2 weeks.

---

## Tier 2: Build Next

Clear research foundations, defined scope. One to three months each. More design
work than Tier 1 but the research provides reference implementations.

### 2.1 Failure Attribution Scoring

**Source:** AgentDebugX (Zhu et al., arXiv:2607.18754): Detect-Attribute-
Recover-Rerun loop; repairs 13/73 failed GAIA tasks (accuracy 55.8% to 63.6%).
AgenTracer-8B (Zhang et al., arXiv:2509.03312, ICLR 2026): beats
Gemini-2.5-Pro/Claude-4-Sonnet on Who&When by up to 18.18%. TRAIL
(arXiv:2505.08638), Who&When (ICML 2025 Spotlight).

Add failure-attribution labels to outcome metadata: which step caused the
failure, what category (reasoning error, tool misuse, context loss,
hallucination, planning mistake). A trace demonstrating a novel failure mode is
valuable even if -- especially if -- the task outcome was bad.

Creates a second reason to contribute ("my failure pattern is useful to
others"). The lightweight version runs AgenTracer-8B over failed traces for
structured attribution; the full version includes an Error Hub API for querying
failure patterns by category, tool, and agent config. ~6-8 weeks for scoring;
~4-6 more for Error Hub.

### 2.2 Trajectory Replay UI

**Source:** AgentGUI (Zhao, Sohn, Zheng & Moor -- ETH Zurich;
arXiv:2607.26300; MIT). Users identify key trace elements 38% faster
(p=0.023); automated drift-prevention raises task completion by up to 34 pp
across 0.8B-9B models.

Interactive trajectory visualization over TC's existing SSE infrastructure.
Contributors see timelines with branching points, expandable tool-call cards,
and scoring overlays instead of flat JSON. Natural home for failure attribution
results (2.1). OTel ingest (1.1) helps by standardizing cross-harness formats.

Answers the "single-player value" question: show contributors their own agent
behavior -- where it was efficient, where it spun its wheels, how it compares to
high-scoring corpus traces. ~8-10 weeks.

### 2.3 NCD Compression Pre-Filter

**Source:** Li et al. (2004) "The Similarity Metric," IEEE Trans. Info. Theory.
Jiang et al. (2023) "Less is More: Parameter-Free Text Classification with
Gzip," ACL 2023. NCD + kNN achieves competitive classification without model
training.

Normalized Compression Distance: compress A, compress B, compress AB
concatenated, compute
`NCD(x,y) = (C(xy) - min(C(x),C(y))) / max(C(x),C(y))`. Using zstd, a single
comparison takes under 10ms. NCD catches structural similarity where exact
tokens differ but information content is the same -- two traces solving the same
problem with different variable names or comment styles.

Layer 2 in the multi-layer novelty pipeline, complementary to MinHash (1.4).
Like MinHash, NCD's behavior is well-understood from information theory -- you
are not depending on a model whose training distribution may not match your
corpus. Together, MinHash + NCD eliminate the two most common false-novelty
failure modes before the embedding model is consulted. ~2-3 weeks.

### 2.4 Sub-Trace Decomposition

**Source:** LEGOMem (Han et al. -- Microsoft Research; arXiv:2510.04851, AAMAS
2026). Orchestrator memory was critical for task decomposition/delegation;
fine-grained agent memory improved execution accuracy; small-model teams
benefited most.

A trajectory decomposes into orchestration decisions (which sub-tasks, what
order, what delegation), execution steps (tool calls, file edits, test runs),
and verification actions (checking results, comparing outputs). These have
different value for different consumers. Extend `TraceContributionEnvelope` with
sub-trace decomposition so consumers query by role and granularity:
"orchestration patterns for multi-file Rust refactoring" or "verification
strategies for database migrations."

A 200-step trace where 180 steps are boilerplate and 20 are genuinely novel
currently gets a single averaged score. Decomposition lets the pipeline
recognize the 20-step fragment is novel even when the 180-step execution is
routine. Enables finer-grained credit and feeds both failure attribution (2.1)
and skill extraction (3.2). Sub-units are stored as indexed children of the
parent envelope, each with their own novelty and quality scores. ~6-8 weeks.

### 2.5 Sleep-Time Batch Pre-Computation

**Source:** Sleep-time Compute (Lin et al. -- Letta/UC Berkeley;
arXiv:2504.13171, shipped in Letta 0.7.0). ~5x reduction in test-time compute;
accuracy gains up to 13% on Stateful GSM-Symbolic and 18% on Stateful AIME;
2.5x average cost per query drop.

During idle GPU windows, pre-compute embeddings, run NCD comparison matrices,
update MinHash indices, compute cluster centroids, and pre-score failure
attribution candidates. When a submission arrives, the scoring pipeline
references pre-computed artifacts instead of computing from scratch.

The 2.5x cost reduction translates directly to TC's economics. Background
worker becomes two-phase: continuous "sleep" phase pre-computing batch
artifacts, and a "wake" phase triggered by submissions. ~4-6 weeks.

### 2.6 Compound System Auto-Tuning for Gate Thresholds

**Source:** "Compound AI Systems Optimization" (EMNLP 2025): multi-module LLM
pipeline optimization via natural-language feedback and numerical signals. Also:
TextGrad (Yuksekgonul et al., 2024), DSPy (Khattab et al., 2024).

Replace hand-tuned gate thresholds with an auto-tuning loop optimizing for
diversity of accepted traces while minimizing false positives against human
annotations (PR #173 infrastructure). Uses gradient-free methods (Bayesian
optimization via the `argmin` Rust crate, evolutionary strategies).

Gate thresholds are the weakest link -- even with better models, the "novel
enough" vs. "too similar" cutoffs are arbitrary. Auto-tuning lets the system
calibrate itself as the corpus grows. Runs offline in the sleep-time batch
(2.5), publishes updated thresholds the live pipeline picks up on next tick.
Depends on PR #173. ~6-8 weeks.

---

## Tier 3: Differentiate Later

Research frontier. Three to six months each. Once built, these make TC
genuinely hard to replicate.

### 3.1 Influence-Function Valuation via LogIX

**Source:** LoGra (Choe et al. -- CMU/Toronto/Vector; arXiv:2405.13954,
NeurIPS 2025). LogIX library (Apache 2.0): up to 6,500x/5x compute/memory
improvements vs. EKFAC at Llama3-8B scale. For-Value (Deng et al.,
arXiv:2508.10180, ACL 2026): forward-only influence estimation.

Influence functions measure how much a training example affects model
predictions. LoGra makes this tractable at LLM scale via Kronecker-factored
gradient projection. Replace heuristic `q = f * g * a` with measured downstream
impact. Run LogIX in the offline worker to compute each trace's influence, then
use that as the true `quality_proxy` in the VCG credit mechanism.

No other trace marketplace prices by measured downstream impact -- Ocean and
Vana price by volume, Bittensor by mining difficulty. TC would be the first to
use influence-function valuation for contributor compensation. Requires
open-weight models (needs gradients); For-Value's forward-only variant reduces
cost. LogIX is Python/PyTorch; TC integration is a Python sidecar running batch
computation during sleep-time windows (2.5). ~10-14 weeks.

### 3.2 Automated Skill Extraction Pipeline

**Source:** RHO (arXiv:2606.05922): 19% absolute gain on SWE-Bench Pro without
validation labels via single retrospective pass over unlabeled trajectories.
Also: ReasoningBank (Ouyang et al., 2025), SkillOS (arXiv:2605.06614), Dynamic
Cheatsheet (Suzgun et al., 2025).

The automated version of 1.2. Instead of manual curation, a pipeline analyzes
the corpus to identify recurring high-value procedures and converts them to
SKILL.md artifacts. RHO's key insight: no labeled data needed. A retrospective
pass over past trajectories, comparing successful and failed attempts at similar
tasks, suffices to extract harness improvements.

Turns TC from a passive data lake into an active capability supplier. The pitch
becomes "your traces taught an extraction pipeline, which produced a skill
adopted by 47 developers this month, and you earn credit from that adoption."
Pipeline: cluster traces by task embedding, identify common sub-procedures via
decomposition (2.4), run retrospective extraction (RHO pattern), format as
SKILL.md, score through the gate pipeline before publication. ~12-16 weeks.

### 3.3 VET Composed Proofs for Verifiable Scoring

**Source:** VET (arXiv:2512.15892): Agent Identity Document (AID) with composed
proofs across SNARKs/STARKs, TEEs, and consensus re-execution. Also:
"Cryptographic Verifiability of End-to-End AI Pipelines" (arXiv:2503.22573),
"When Agents Handle Secrets" (arXiv:2605.03213).

Compose proofs from different families: TEE attestation for the environment,
succinct proof for the scoring computation, optionally consensus re-execution
for critical steps. Extends TC's existing IronClaw TEE bridge to a composed
proof covering pipeline correctness, input authenticity, and external
verifiability.

Offer "verifiable trace" as a premium tier. No competing trace platform offers
end-to-end verifiable scoring. Relevant for EU AI Act Article 12 enforcement
(began August 2, 2026) and safety researchers. Near-term target:
scoring-verification proofs (MinHash, NCD, threshold application) composed with
TEE attestation via `risc0` or `sp1` zkVM. Full ZKML for LLM inference is
longer-term. ~12-16 weeks.

### 3.4 Evidence Relation Graph

**Source:** "From Agent Traces to Trust: A Survey of Evidence Tracing and
Execution Provenance in LLM Agents" (arXiv:2606.04990). Proposes typed
relations -- Support, Depend-on, Contradict, Update, Invalidate -- linking
evidence units, tool calls, memory items, claims, and actions.

Add typed edges to the trace model: this tool call *depends on* that retrieval;
this assertion *supports* that claim; this action *invalidates* that decision.
Enables queries impossible with flat logs and supports taint tracking for prompt
injection via Depend-on edges.

Extends TC's provenance stack (C2PA, SCITT, DIDs/VCs) from content provenance
to reasoning provenance. Extraction via rule-based heuristics for deterministic
relations plus LLM-assisted analysis for semantic relations. ~8-12 weeks.

### 3.5 Inference-Time Steering Metadata Capture

**Source:** "Inference-Time Policy Steering" (arXiv:2411.16627): biases
sampling toward human intent without fine-tuning. "When Users Change Their Mind"
(arXiv:2604.00892): benchmarks mid-task interruptions; documents failure modes
(reasoning leakage, panic, self-doubt). "LLM-Based Human-Agent Collaboration"
survey (arXiv:2505.00753).

Capture human-intervention events as a distinct labeled trace type with
structured metadata: intervention point (which step), type (correction,
redirection, clarification, cancellation), and outcome (agent recovered,
degraded, failed). Steering events capture the gap between agent intent and
human intent -- exactly the signal needed to train more steerable agents.

No other trace platform distinguishes steering events from conversation.
Training better interruptibility requires examples of real human interventions
-- data that does not exist in any public corpus. TC is uniquely positioned to
collect it with privacy guarantees. ~6-8 weeks.

---

## Summary

| # | Innovation | Tier | Effort | Key Benefit |
|---|---|---|---|---|
| 1.1 | OTel GenAI Ingest | 1 | 2-3w | Standards-based onboarding |
| 1.2 | SKILL.md Publishing | 1 | 1-2w | Contributor feedback loop |
| 1.3 | Prometheus Metrics | 1 | 1-2w | Production observability |
| 1.4 | MinHash Dedup | 1 | 1-2w | Analytically sound novelty layer |
| 2.1 | Failure Attribution | 2 | 6-8w | New contribution motive |
| 2.2 | Trajectory Replay UI | 2 | 8-10w | Single-player value |
| 2.3 | NCD Pre-Filter | 2 | 2-3w | Structural similarity detection |
| 2.4 | Sub-Trace Decomposition | 2 | 6-8w | Fine-grained scoring and credit |
| 2.5 | Sleep-Time Batch | 2 | 4-6w | Faster scoring, lower cost |
| 2.6 | Auto-Tuning Gates | 2 | 6-8w | Self-calibrating thresholds |
| 3.1 | Influence Valuation | 3 | 10-14w | Impact-based pricing |
| 3.2 | Skill Extraction | 3 | 12-16w | Active capability supplier |
| 3.3 | VET Composed Proofs | 3 | 12-16w | Verifiable scoring |
| 3.4 | Evidence Relation Graph | 3 | 8-12w | Reasoning provenance |
| 3.5 | Steering Metadata | 3 | 6-8w | Human-intervention corpus |

Total estimated effort: ~18-24 person-months across three tiers. Tier 1 ships
in a month. Tier 2 picks 2-3 items for the next quarter. Tier 3 chooses one
strategic bet. Each tier builds on the last: MinHash feeds NCD feeds influence
valuation; SKILL.md publishing is the manual version of skill extraction;
sleep-time compute is the execution framework for all Tier 3 batch work.
