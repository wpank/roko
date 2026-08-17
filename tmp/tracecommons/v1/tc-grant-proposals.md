# TraceCommons Grant Proposals

Three near-submission-ready grant proposals for TraceCommons development.
Each proposal is self-contained and assumes no prior reader context.

---

# Proposal 1: NLnet Foundation -- NGI Zero Restack

**Program**: NGI Zero Restack (NLnet Foundation)
**Opens**: September 3, 2026 | **Deadline**: November 3, 2026
**Requested amount**: EUR 48,000
**Duration**: 12 months, 4 milestones
**URL**: https://nlnet.nl/restack/

## Proposal title

Privacy-Preserving Collective AI Trace Scoring with Adaptive Thresholds

## Abstract

AI agents are rapidly becoming the primary interface through which
developers, knowledge workers, and researchers interact with software
systems. Every agent session produces a trace -- a structured record of
the tools called, the decisions made, the errors encountered, and the
result returned. These traces are the empirical record of what AI
actually does when deployed in the real world. Today, that record is
collected unilaterally by whichever company runs the model, on terms the
user never specifically agreed to, and it is never shared back.

TraceCommons is an open-source infrastructure project (MIT/Apache-2.0,
Rust, PostgreSQL) that reverses this dynamic. It is a user-owned
register of AI agent work. Capture and redaction happen on the
contributor's own machine; only the scrubbed envelope reaches a shared
server, where two independent gates -- novelty (is this genuinely
different from what is already filed?) and substance (is this real work
rather than template filler?) -- decide whether the record is worth
keeping. Accepted records are signed, dated, and filed. Frontier labs,
auditors, and regulators can query the register under selective
disclosure; they see what they need, and the rest stays encrypted.

This proposal funds four milestones that harden TraceCommons for
production deployment as EU AI Act Article 12 compliance infrastructure:
an adaptive scoring engine with EMA-based thresholds and CUSUM change
detection; a multi-stage gate pipeline (Bloom filter, LSH,
classifier, TEE); differential privacy for aggregate statistics; and a
consolidation engine with comprehensive documentation. All outputs are
open source under the existing dual MIT/Apache-2.0 license.

## 1. Problem statement

### 1.1 The AI accountability gap

The EU AI Act (Regulation 2024/1689), whose Article 12 requirements for
mandatory logging of high-risk AI systems took effect on August 2, 2026,
creates a legal requirement for auditable records of AI system behavior.
No open-source infrastructure exists today that satisfies this
requirement while keeping the underlying data under contributor control.
Commercial providers collect traces but do not share them, do not allow
independent audit, and do not compensate the users whose work sessions
generated the data.

### 1.2 The trust deficit in current approaches

Existing AI trace collection suffers from three structural problems:

1. **Unilateral collection.** Model providers capture session data
   without meaningful user consent or data sovereignty.
2. **No quality control.** Bulk corpus dumps contain enormous volumes of
   duplicative, trivial, or template-shaped content that is useless for
   training or evaluation.
3. **No privacy guarantees.** Traces routinely contain API keys, private
   file paths, database credentials, PII, and proprietary code. No
   deployed system today provides verifiable privacy guarantees over
   contributed traces.

### 1.3 Regulatory pressure without infrastructure

Article 12 requires "automatic recording of events" for high-risk AI
systems, with logs kept for "an appropriate period of time." But the
regulation does not mandate a specific technical infrastructure. Without
an open, auditable, privacy-preserving register, organizations will
default to proprietary, siloed logging that satisfies the letter of the
law while undermining its spirit.

## 2. Proposed solution: TraceCommons

### 2.1 Architecture overview

TraceCommons is a Rust workspace of six crates totaling approximately
62,000 lines of production code, currently in pilot deployment:

| Crate | Purpose |
|---|---|
| `trace-commons-protocol` | Shared DTOs, envelope schema, deterministic redaction helpers |
| `trace-commons-gate-api` | Public gate contracts: scorer traits, decision types, reference implementations |
| `trace-commons-gate-enclave` | Scoring orchestrator: perplexity scorer, embedder, vector index (HNSW via usearch) |
| `trace-commons-server` | Hosted control plane: ingest, review, retention, revocation, credit settlement, audit chain |
| `trace-commons-contributor` | Contributor CLI: local session discovery (Claude Code, Codex, Letta Trajectory), redaction, upload |
| `trace-commons-operator-client` | Operator-facing API client |

The system architecture follows a local-first, fail-closed design:

```
Contributor machine              Hosted server                 NEAR AI Cloud
+-------------------+    HTTPS   +------------------------+    +----------------+
| trace-commons-    | ---------> | trace-commons-ingest   | -> | TEE-hosted     |
| contributor CLI   |            |                        |    | vLLM scoring   |
| - session discovery            | - PostgreSQL (RLS)     |    | (Intel TDX +   |
| - local redaction |            | - encrypted artifacts  |    |  NVIDIA TEE)   |
| - envelope upload |            | - credit settlement    |    +----------------+
+-------------------+            | - audit chain          |
                                 +--------+--------+------+
                                          |        |
                                     GCS/Object    NEAR blockchain
                                     storage       (credit settlement)
```

### 2.2 The two-gate pipeline

Every submitted envelope passes two independent gates before entering
the register:

- **Novelty gate.** Exact canonical-summary hash matching for strong
  duplicate detection, then vector-backed nearest-neighbor scoring
  (HNSW index via usearch, fastembed embeddings, BAAI/bge-large-en-v1.5)
  for semantic similarity. A trace that is too similar to existing
  entries is rejected.

- **Substance gate.** Perplexity scoring via a 27B-class language model
  (currently Qwen 3.6 35B-A3B-FP8, running on NEAR AI Cloud TEE-hosted
  vLLM with Intel TDX attestation). The perplexity score distinguishes
  genuine reasoning from template-shaped filler. This was validated via a
  four-model bake-off (Llama-3.1-8B, Qwen3-8B-Base, Qwen 3.6 27B Dense,
  Gemma 4 31B) that demonstrated 27B-class models achieve AUC > 0.93
  while 8B-class models remain below chance (AUC < 0.35).

Both gates must pass. The `EnclaveGateOrchestrator` composes
`PerplexityScorer + Embedder + VectorIndex` behind trait objects, so
backends can be swapped without changing the host-side code. The
gate-api crate provides the stable contract surface; proprietary
backends implement the same traits.

### 2.3 Local-first privacy

Privacy is a structural invariant, not a feature flag:

- **Off by default.** Trace contribution requires explicit opt-in.
- **Local redaction.** Three-layer deterministic scrubbing runs entirely
  on the contributor's machine before anything reaches the network:
  (1) named-pattern regexes for known secret shapes (OpenAI/Anthropic
  `sk-`, GitHub `ghp_`/`gho_`, AWS `AKIA`, JWTs, PEM blocks, 15+
  provider patterns); (2) cue-gated high-entropy catch-all for unknown
  secret formats; (3) per-session fail-closed leaked-token guard that
  refuses the session rather than uploading partial redaction.
- **Optional NEAR AI PII filter.** A separate pass sends
  already-redacted message text through NEAR AI Cloud TEE-hosted PII
  detection. This is fail-closed: if unreachable, the batch is refused.
- **Server-side rescrub.** The server treats every upload as untrusted
  and re-runs deterministic redaction.
- **Hash-only audit.** All audit rows, error logs, and operational
  surfaces use only hashes and labels. Raw traces, URLs, bearer tokens,
  and contributor identity never appear in stored rows or log strings.
- **PostgreSQL RLS enforced.** Every Trace Commons table has row-level
  security forced via `trace_current_tenant_id()`.

### 2.4 Trace Credits and NEAR settlement

A Trace Credit is the signed, on-chain record that a contributor's
submission was accepted. Credits are non-transferable and settle on NEAR.
The credit lifecycle is deliberately staged:

1. Local pending estimate from a multi-factor scorecard (privacy risk,
   quality, replayability, novelty, duplicate penalty, coverage,
   difficulty, dependability, correction value).
2. Pending credit event recorded on acceptance.
3. Delayed credit appended through audited paths (benchmark conversion,
   ranker training utility, reviewer adjustments).
4. Settlement turns eligible pending credit into non-transferable NEAR
   account credit.

The settlement system supports three modes: `disabled` (safe default),
`dry_run` (synthetic tx hashes for testing), and `http` (external
signing adapter for production). Payout resolution is fail-closed:
ambiguous designations result in held credit, never guessed
destinations.

### 2.5 Current deployment status

TraceCommons is in pilot deployment as of May 2026. The server is
code-complete and smoke-validated. The scoring backend runs on NEAR AI
Cloud (TEE-hosted vLLM, Intel TDX + NVIDIA GPU TEE). The contributor CLI
supports Claude Code, Codex, and Letta Trajectory sessions. The database
schema spans 41 migrations covering ingest, credit settlement, ranking
evidence, gate decisions, contributor accounts (passkey/WebAuthn),
NEAR identity enrollment, and deduplication.

### 2.6 Relevance to NGI Zero Restack

TraceCommons directly addresses the Restack program's mission of
delivering, maturing, and scaling new internet commons:

- **Open internet infrastructure.** An open, auditable register of AI
  agent behavior is foundational infrastructure for an accountable AI
  ecosystem.
- **User data sovereignty.** The local-first design ensures traces are
  owned by contributors, not platforms.
- **EU regulatory compliance.** The system provides infrastructure for
  AI Act Article 12 compliance without requiring proprietary solutions.
- **Interoperability.** The protocol crate and gate-api contracts are
  designed for multi-implementation interoperability.

## 3. Technical approach: what this grant funds

This grant funds four specific engineering milestones that move
TraceCommons from pilot to production-ready infrastructure. Each
milestone builds on the existing codebase and delivers measurable,
independently verifiable outputs.

### Milestone 1: Adaptive scoring engine (months 1-3)

**Budget**: EUR 12,000

The current gate uses static floor thresholds. This milestone replaces
them with adaptive thresholds that track distributional shifts in
contributed traces.

**Deliverables:**

1. **EMA-based threshold adaptation.** Exponential moving average
   tracking of per-gate score distributions. As the corpus evolves, the
   novelty and substance floors adjust to maintain consistent selectivity
   without manual recalibration. Implementation as a new module in
   `trace-commons-gate-enclave` behind the existing `EnclaveGateOrchestratorConfig`
   surface.

2. **CUSUM change-point detection.** Cumulative sum control chart
   monitoring on score streams to detect regime changes -- sudden shifts
   in the quality or character of incoming traces that indicate either
   gaming attempts or legitimate shifts in the contributor population.
   Alerts feed the existing hash-only audit chain.

3. **Per-contributor score normalization.** Baseline establishment per
   contributor to distinguish genuinely novel contributions from
   contributor-specific patterns. This prevents gaming through
   stylistic variation that produces superficial novelty without
   substantive content.

4. **Regression test suite.** Property-based tests (via proptest or
   similar) exercising threshold convergence, change-point sensitivity,
   and the interaction between per-contributor normalization and
   corpus-wide thresholds.

**Verification criteria:** The adaptive engine must demonstrate
convergence on a held-out slice of the HuggingFace agent-traces corpus,
with selectivity within +/-5% of manually calibrated floors. Change-point
detection must fire within 50 traces of a synthetic regime shift.

### Milestone 2: Multi-stage gate pipeline (months 4-6)

**Budget**: EUR 12,000

The current gate evaluates every trace through the full perplexity +
embedding + vector pipeline. For large-scale deployment, the gate needs
early-exit stages that reject obvious duplicates and low-quality
submissions before invoking the expensive GPU-backed scorer.

**Deliverables:**

1. **Bloom filter pre-screen.** A probabilistic membership test on
   canonical-summary hashes that rejects exact and near-exact duplicates
   in O(1) with configurable false-positive rate. Persisted alongside
   the existing PostgreSQL schema; rebuilt on server restart from the
   gate-decision table.

2. **Locality-sensitive hashing (LSH) stage.** MinHash-based similarity
   estimation that catches semantic near-duplicates before the
   full vector pipeline runs. Configurable similarity threshold
   integrated into `EnclaveGateOrchestratorConfig`.

3. **Lightweight classifier stage.** A small, CPU-only classifier
   (logistic regression or small decision tree on envelope metadata
   features -- event count, tool diversity, session duration, redaction
   density) that screens out template-shaped submissions before the
   27B-class perplexity scorer runs. Trained on the existing corpus of
   accepted/rejected decisions.

4. **TEE attestation integration.** Preparation for Phase B's dstack
   migration: the multi-stage pipeline structured so each stage can run
   inside an attested enclave, with attestation evidence chained through
   the existing `attestation_chain_hash` in `OrchestrationDecision`.

**Verification criteria:** The multi-stage pipeline must reject at least
60% of duplicate/low-quality submissions before the GPU scorer runs,
with false-negative rate below 1% (no genuine high-quality trace
rejected by early stages). End-to-end latency for rejected traces must
be below 100ms.

### Milestone 3: Privacy enhancements (months 7-9)

**Budget**: EUR 12,000

This milestone adds privacy guarantees that go beyond redaction to
provide formal, verifiable privacy properties.

**Deliverables:**

1. **Differential privacy for aggregate statistics.** The community
   leaderboard, analytics summary, and per-contributor statistics
   endpoints currently return exact counts. This deliverable adds
   calibrated Laplace noise to all aggregate queries, with configurable
   epsilon budgets per query class. Implementation in the existing
   community API routes (`/api/v1/community/leaderboard`,
   `/api/v1/community/analytics/summary`).

2. **Zero-knowledge attestation of gate decisions.** A ZK proof system
   (likely Groth16 or PLONK via arkworks or similar Rust ZK library)
   that allows the gate to prove a trace passed both novelty and
   substance checks without revealing the scores or the trace content.
   This enables third-party audit of gate integrity without exposing
   the scoring parameters.

3. **Encrypted computation readiness.** Structural preparation for
   homomorphic encryption of vector similarity queries -- the most
   privacy-sensitive operation in the pipeline. This milestone delivers
   the trait abstractions and benchmarks; full HE deployment is a
   future milestone contingent on HE library maturity.

4. **Privacy budget accounting.** A per-contributor privacy budget
   tracker that limits the cumulative information leaked about any
   single contributor through aggregate queries, status responses, and
   credit settlement events.

**Verification criteria:** Differential privacy implementation must pass
a membership inference attack test suite (the attacker, given aggregate
statistics, cannot determine whether a specific trace is in the corpus
with advantage greater than epsilon). ZK attestation must verify in
under 500ms on commodity hardware.

### Milestone 4: Consolidation engine and documentation (months 10-12)

**Budget**: EUR 12,000

**Deliverables:**

1. **Trace consolidation engine.** An offline batch process that
   identifies clusters of related traces and produces consolidated
   training examples -- merging partial traces from the same workflow
   into coherent episodes while preserving provenance and credit
   attribution. Built on the existing chunker and embedder
   infrastructure in `trace-commons-gate-enclave`.

2. **Federation protocol specification.** A written protocol spec for
   multi-instance TraceCommons federation -- how two independent
   instances can exchange trace metadata (not content) for cross-corpus
   deduplication and novelty assessment. This is a spec deliverable,
   not an implementation, to establish the contract before building.

3. **Contributor SDK documentation.** Comprehensive documentation for
   the contributor integration surface: envelope schema, redaction
   contract, consent model, credit lifecycle, and the gate-api trait
   surface for custom scoring backends.

4. **Deployment guide and operator runbooks.** Production deployment
   documentation covering GCP, AWS, and self-hosted configurations.
   Building on the existing `docs/operator/` runbook collection (which
   currently covers calibration, bootstrap, settlement, backup/restore,
   and troubleshooting).

**Verification criteria:** All documentation must be reviewed by at
least one external contributor. The consolidation engine must produce
output that passes the existing gate pipeline when re-scored. The
federation spec must be implementable independently by a third party
given only the spec document.

## 4. Timeline

| Month | Milestone | Key deliverable |
|---|---|---|
| 1-3 | M1: Adaptive scoring | EMA thresholds, CUSUM detection, per-contributor normalization |
| 4-6 | M2: Multi-stage gates | Bloom filter, LSH, classifier, TEE prep |
| 7-9 | M3: Privacy | Differential privacy, ZK attestations, budget accounting |
| 10-12 | M4: Consolidation | Consolidation engine, federation spec, documentation |

## 5. Budget justification

| Item | Amount | Justification |
|---|---|---|
| M1 engineering | EUR 10,000 | ~400 hours at EUR 25/hr. Adaptive scoring touches gate-enclave internals, requires careful testing against real corpus data. |
| M1 infrastructure | EUR 2,000 | GPU compute for corpus-scale threshold validation. |
| M2 engineering | EUR 10,000 | Multi-stage pipeline requires new data structures (Bloom, MinHash), classifier training, and TEE integration preparation. |
| M2 infrastructure | EUR 2,000 | CI/CD pipeline for multi-backend gate testing. |
| M3 engineering | EUR 10,000 | Privacy engineering (DP, ZK) requires specialized cryptographic implementation and formal verification. |
| M3 infrastructure | EUR 2,000 | ZK proof generation benchmarking hardware. |
| M4 engineering | EUR 8,000 | Consolidation engine, federation spec, documentation. |
| M4 infrastructure | EUR 2,000 | Multi-cloud deployment testing. |
| M4 external review | EUR 2,000 | Compensating external reviewers for documentation and spec review. |
| **Total** | **EUR 48,000** | |

## 6. Team qualifications

*[Template -- to be filled by applicants]*

**Principal Investigator:** [Name], [Title/Role]. [Relevant experience
with open-source infrastructure, Rust systems programming, privacy
engineering, and/or AI systems.]

**Co-investigator:** [Name], [Title/Role]. [Relevant experience with
blockchain systems (NEAR specifically), cryptographic protocols, and/or
TEE-based computation.]

The TraceCommons codebase has been developed since early 2026 and
represents approximately 62,000 lines of production Rust code across
six crates, with 41 database migrations, a pilot deployment on GCP with
NEAR AI Cloud scoring, and a contributor CLI supporting three session
formats. The team has demonstrated the ability to ship production
infrastructure at this scale.

## 7. Broader impacts

### 7.1 EU AI Act compliance infrastructure

TraceCommons provides the first open-source infrastructure for Article 12
compliance that keeps data under contributor control. Organizations
deploying high-risk AI systems need auditable trace logs; TraceCommons
provides this without vendor lock-in.

### 7.2 AI safety research

A curated, quality-gated corpus of real agent traces is a prerequisite
for empirical AI safety research. Academic researchers currently lack
access to traces of real-world AI agent behavior at scale. TraceCommons
provides this access through scoped, audited API queries with formal
privacy guarantees.

### 7.3 Fair compensation

The Trace Credit system on NEAR provides a transparent, auditable
mechanism for compensating contributors when their data generates
downstream value. This is a concrete alternative to the current model
where AI companies capture user session data without compensation.

### 7.4 Open standard

The protocol crate and gate-api contracts are designed to be
implementable independently. Multiple TraceCommons instances, operated by
different organizations, could eventually interoperate through the
federation protocol, creating a decentralized network of AI trace
registers rather than a single centralized corpus.

## 8. Sustainability plan

TraceCommons is designed for long-term sustainability through three
mechanisms:

1. **Query fees.** Frontier labs and enterprise consumers pay to query
   the register. Revenue flows through the credit system to
   contributors.
2. **Hosted operation.** Organizations that want managed TraceCommons
   instances pay for hosted operation. The open-source codebase remains
   freely available for self-hosting.
3. **Grant-funded research.** The corpus enables academic research that
   attracts research funding (this proposal is one example).

Post-grant, the adaptive scoring and privacy enhancements become part of
the production infrastructure that query fees and hosted operation
sustain.

## 9. References

1. EU AI Act, Regulation (EU) 2024/1689, Article 12: Automatic recording of events.
2. Dwork, C. & Roth, A. (2014). "The Algorithmic Foundations of Differential Privacy." Foundations and Trends in Theoretical Computer Science.
3. Groth, J. (2016). "On the Size of Pairing-based Non-interactive Arguments." EUROCRYPT 2016.
4. TraceCommons source repository: https://github.com/zmanian/trace-commons-server (MIT/Apache-2.0).
5. NEAR AI Cloud TEE documentation: https://docs.near.ai (Intel TDX + NVIDIA GPU TEE attestation).
6. Indyk, P. & Motwani, R. (1998). "Approximate Nearest Neighbors: Towards Removing the Curse of Dimensionality." STOC 1998.
7. Page, E.S. (1954). "Continuous Inspection Schemes." Biometrika, 41(1/2), 100-115. (CUSUM method)

---

# Proposal 2: NEAR Foundation Developer Hub Grants

**Program**: NEAR Foundation Developer Hub Grants
**Applications**: Rolling
**Requested amount**: USD 120,000
**Duration**: 9 months, 3 phases

## Proposal title

TraceCommons: Decentralized AI Training Data Marketplace on NEAR

## Abstract

TraceCommons is an open-source infrastructure project that creates a
user-owned register of AI agent work -- a shared, quality-gated corpus
of real-world AI agent traces that contributors control and are
compensated for. It is built in Rust (MIT/Apache-2.0, ~62,000 LOC,
six crates, PostgreSQL-backed), currently in pilot deployment, and
already uses the NEAR blockchain as the settlement layer for Trace
Credits -- non-transferable, on-chain records that a contributor's
submission was accepted into the register and that enable compensation
when downstream consumers pay to query the evidence.

This proposal funds three phases of deeper NEAR integration that
transform TraceCommons from a centralized pilot into a decentralized
marketplace for AI training data on NEAR. Phase 1 enhances the existing
NEAR integration with smart contracts for trace provenance, credit
staking, and on-chain governance. Phase 2 builds federated trace sharing
across multiple TraceCommons instances, enabling cross-organization
trace exchange with NEAR as the settlement and coordination layer.
Phase 3 delivers a developer SDK and marketplace UI that makes it easy
for AI builders to discover, evaluate, and license trace data through
NEAR-mediated transactions.

The result is a NEAR-native marketplace where every accepted AI trace
has a verifiable provenance chain on NEAR, every contributor receives
transparent compensation through NEAR credits, and every downstream
consumer can license specific trace subsets through NEAR smart
contracts.

## 1. Problem statement

### 1.1 The AI training data crisis

AI agent builders need millions of real-world traces to train, evaluate,
and benchmark their systems. These traces currently live inside private
user sessions, collected unilaterally by model providers. There is no
marketplace where trace data can be contributed, quality-gated,
discovered, and licensed with verifiable provenance and fair
compensation.

### 1.2 Why NEAR

NEAR is uniquely positioned as the settlement and coordination layer for
a decentralized AI trace marketplace:

- **Account model.** NEAR's account model supports named accounts and
  access keys, which maps naturally to contributor identity and
  device-key enrollment in the TraceCommons protocol.
- **Low transaction costs.** Trace Credits settle individually; the
  per-trace transaction cost must be negligible relative to the credit
  value.
- **TEE ecosystem.** NEAR AI Cloud provides TEE-hosted inference
  (Intel TDX + NVIDIA GPU TEE) that TraceCommons already uses for
  privacy-preserving scoring.
- **Existing integration.** TraceCommons already uses NEAR for credit
  settlement. This grant deepens that integration rather than building
  from scratch.

### 1.3 Current NEAR integration

TraceCommons already has substantial NEAR integration:

- **Credit settlement outbox.** Hash-only utility attestations, dry-run
  and signed central-issuer-approved live batches, NEAR receipt outbox.
  Settlement supports three modes: `disabled`, `dry_run`, and `http`
  (external signing adapter).
- **NEAR identity enrollment.** Contributors enroll NEAR identities for
  payout designation. The system supports multiple enrolled identities
  per account with a single active payout designation, enforced by a
  partial-unique index.
- **NEAR AI Cloud scoring.** The substance gate runs on NEAR AI
  Cloud-hosted vLLM with Intel TDX attestation. The `NearAiPerplexityScorer`
  in `trace-commons-gate-enclave` is a production scorer backend.
- **Account outbox.** A separate NEAR account-operation outbox
  (migration V21) for credit-hold freeze/unfreeze calls.

This grant builds on this foundation.

## 2. Technical approach

### 2.1 Existing architecture

TraceCommons is a Rust workspace of six crates:

- **trace-commons-protocol** -- Shared DTOs, envelope schema,
  deterministic redaction helpers. The `ironclaw.trace_contribution.v1`
  envelope is the contract between client and server.
- **trace-commons-gate-api** -- Public gate contracts: `PerplexityScorer`,
  `Embedder`, `VectorIndex` traits, `OrchestrationDecision` type,
  `EnclaveGateOrchestratorConfig`. This is the stable seam for
  scoring backends.
- **trace-commons-gate-enclave** -- Scoring orchestrator composing
  perplexity + embedding + novelty into a single gate decision. Two
  production backends: local GPU (mistral.rs + candle) and NEAR AI Cloud
  HTTP. Vector index via usearch HNSW. Embedder via fastembed
  (BAAI/bge-large-en-v1.5).
- **trace-commons-server** -- The hosted control plane. Eight binaries:
  `trace-commons-ingest` (~61K LOC, the main API), upload-claim issuer
  (EdDSA/Ed25519), gate calibrator, pilot bootstrap harness, vector
  replay tool, review tool, admin tool, worker tool. 41 PostgreSQL
  migrations. Forced RLS on every tenant-scoped table. Encrypted
  artifact store (local-encrypted, filesystem-remote, GCS). Cloud KMS
  envelope encryption.
- **trace-commons-contributor** -- Contributor CLI. Subcommands: login,
  list, submit, status, whoami, logout, mint-grant. Discovers Claude
  Code, Codex, and Letta Trajectory sessions. Three-layer deterministic
  secret redaction. Optional NEAR AI PII filter. Scope-based consent
  model with interactive prompts.
- **trace-commons-operator-client** -- Operator-facing API client.

The community site at tracecommons.ai renders leaderboard, contributor
profiles, analytics, and cohort information from the ingest API.

### 2.2 Phase 1: Enhanced NEAR integration (months 1-3, USD 40,000)

**Smart contracts for trace provenance:**

Deploy a NEAR smart contract that records the provenance chain for every
accepted trace. The contract stores:
- Trace hash (the existing `attestation_chain_hash` from
  `OrchestrationDecision`)
- Gate policy version and gate version hash
- Contributor's pseudonymous identifier (never the real identity)
- Timestamp and block height of acceptance
- Credit amount and settlement status

This creates an immutable, publicly auditable record of what was
accepted and when, without revealing trace content.

**Credit staking mechanism:**

Extend the existing Trace Credit settlement with a staking contract
that allows downstream consumers to stake NEAR against specific
trace subsets. When a consumer stakes against a query result, the
stake is distributed to the contributors whose traces were included
in the result, proportional to their credit allocation. This replaces
the current central-issuer model with a decentralized, market-driven
compensation mechanism.

**On-chain governance of gate parameters:**

The gate's three floor parameters (`perplexity_floor_micros`,
`tail_fraction_floor_micros`, `novelty_floor_micros`) are currently
set by the operator. This deliverable adds a NEAR-based governance
contract where stakeholders (contributors, consumers, operators) can
propose and vote on parameter changes, with a time-lock for
safety.

**Deliverables:**
- NEAR smart contract for trace provenance (Rust, deployed on testnet
  then mainnet)
- Credit staking contract with proportional distribution
- Governance contract for gate parameters
- Integration with the existing `trace-commons-server` settlement
  outbox
- End-to-end test suite exercising the full provenance chain
- Documentation of contract interfaces and deployment procedures

### 2.3 Phase 2: Federated trace sharing (months 4-6, USD 40,000)

**Multi-instance sync protocol:**

Design and implement a federation protocol that allows multiple
independent TraceCommons instances to share trace metadata (not
content) for cross-corpus deduplication and novelty assessment. NEAR
serves as the coordination layer:

- Each instance registers on a NEAR registry contract with its
  public key, endpoint, and capabilities.
- When an instance receives a new trace, it queries the registry for
  other instances that might have seen similar content, using the
  Bloom filter and LSH hashes (never raw content or embeddings).
- Cross-instance deduplication results are recorded on-chain for
  auditability.

**Cross-organization trace exchange:**

Build on the federation protocol to enable controlled trace exchange
between organizations. A data licensing contract on NEAR mediates
access:

- Contributors specify which organizations may access their traces
  through the existing consent-scope system.
- Organizations request access through the NEAR contract, which
  enforces consent constraints.
- Access grants are recorded on-chain with time-bounded leases.
- Credit flows to contributors through the existing settlement
  system when their traces are accessed.

**NEAR-mediated trust anchoring:**

Use NEAR as the trust anchor for cross-instance identity. A
contributor enrolled in one instance can prove their identity to
another instance through their NEAR identity, without revealing
their identity to either instance's operator.

**Deliverables:**
- Federation protocol specification (written, versioned, implementable
  by third parties)
- Registry contract on NEAR for instance discovery
- Cross-instance deduplication via Bloom/LSH exchange
- Data licensing contract with consent enforcement
- Reference implementation of federation in `trace-commons-server`
- Integration tests exercising two-instance federation

### 2.4 Phase 3: Developer SDK and marketplace (months 7-9, USD 40,000)

**Developer SDK for trace submission:**

A language-agnostic SDK (starting with TypeScript and Python, the
languages of the AI agent ecosystem) that wraps the contributor CLI's
functionality:

- Session discovery for popular frameworks (Claude Code, Codex,
  LangChain, CrewAI, AutoGen, Letta)
- Local redaction using the same deterministic pipeline as the Rust CLI
- Envelope construction and upload
- Status and credit queries
- NEAR wallet integration for payout setup

**Quality dashboard:**

A web-based dashboard (building on the existing tracecommons.ai
community site) that shows:

- Real-time gate statistics (accept/reject rates, score distributions)
- Per-contributor quality metrics (acceptance rate, credit history,
  novelty scores)
- Corpus composition (tool categories, agent types, difficulty
  distribution)
- Gate parameter history (tracking governance votes and their effects)

**Marketplace UI:**

A NEAR-native marketplace interface where:

- AI builders can browse the corpus by category, quality tier, and
  coverage domain
- Consumers can preview trace metadata (never content) before
  purchasing access
- Licensing terms are enforced through the Phase 2 data licensing
  contract
- Credit settlement is visible and auditable through NEAR Explorer

**Deliverables:**
- TypeScript SDK with npm package
- Python SDK with PyPI package
- Quality dashboard (React, integrated with tracecommons.ai)
- Marketplace UI (React, NEAR wallet integration)
- API documentation and integration guides
- End-to-end demo: submit trace via SDK, view on dashboard, license
  through marketplace

## 3. Timeline

| Month | Phase | Key deliverables |
|---|---|---|
| 1-3 | Phase 1: NEAR contracts | Provenance contract, staking contract, governance contract |
| 4-6 | Phase 2: Federation | Federation protocol, registry contract, cross-instance dedup |
| 7-9 | Phase 3: SDK + marketplace | TypeScript/Python SDKs, quality dashboard, marketplace UI |

## 4. Budget justification

### Phase 1: USD 40,000

| Item | Amount | Notes |
|---|---|---|
| Smart contract development | USD 18,000 | Three contracts (provenance, staking, governance), ~2 months full-time equivalent |
| Server integration | USD 12,000 | Integrating contracts with existing settlement outbox, gate decision pipeline |
| Testing and audit | USD 6,000 | End-to-end tests, security review of contract logic |
| Infrastructure | USD 4,000 | NEAR testnet/mainnet deployment, CI/CD for contract builds |

### Phase 2: USD 40,000

| Item | Amount | Notes |
|---|---|---|
| Protocol design | USD 8,000 | Federation spec, threat modeling for cross-instance trust |
| Contract development | USD 12,000 | Registry contract, licensing contract, identity anchoring |
| Server implementation | USD 14,000 | Federation endpoints in trace-commons-ingest, Bloom/LSH exchange |
| Testing | USD 6,000 | Two-instance integration tests, adversarial dedup testing |

### Phase 3: USD 40,000

| Item | Amount | Notes |
|---|---|---|
| SDK development | USD 16,000 | TypeScript + Python SDKs wrapping the contributor CLI |
| Dashboard + marketplace UI | USD 16,000 | React apps, NEAR wallet integration, API integration |
| Documentation | USD 4,000 | API docs, integration guides, deployment guides |
| Infrastructure | USD 4,000 | Hosting, CDN, NEAR mainnet gas costs for demo |

## 5. Team qualifications

*[Template -- to be filled by applicants]*

**Project Lead:** [Name]. [Experience with NEAR ecosystem, Rust
development, AI infrastructure.]

**Smart Contract Developer:** [Name]. [Experience with NEAR smart
contract development, Rust, blockchain security.]

**Frontend Developer:** [Name]. [Experience with React, NEAR wallet
integration, dashboard development.]

The team has built and deployed the existing TraceCommons infrastructure
including the NEAR credit settlement system, the TEE-hosted scoring
pipeline on NEAR AI Cloud, and the contributor CLI. The project has
been under active development since early 2026.

## 6. Ecosystem impact

### 6.1 NEAR ecosystem growth

TraceCommons brings a new category of users to NEAR: AI developers and
the contributors who generate training data for them. Every contributor
enrolled in TraceCommons creates a NEAR identity. Every trace acceptance
produces a NEAR transaction. Every data licensing event flows through
NEAR contracts.

### 6.2 Novel use case for NEAR

AI trace provenance and compensation is a novel on-chain use case that
demonstrates NEAR's suitability for applications beyond DeFi and NFTs.
The combination of low transaction costs, named accounts, and the
NEAR AI Cloud TEE ecosystem makes this a showcase application.

### 6.3 Developer tooling

The TypeScript and Python SDKs lower the barrier for AI developers to
interact with NEAR. A developer who starts by submitting traces to
TraceCommons and receiving NEAR credits may go on to build other
NEAR-native applications.

### 6.4 Data commons on NEAR

TraceCommons establishes a pattern for NEAR-mediated data commons: a
quality-gated, contributor-owned data repository with on-chain
provenance and compensation. This pattern generalizes beyond AI traces
to any domain where contributed data has downstream value.

## 7. Sustainability plan

Post-grant sustainability comes from three revenue streams, all mediated
through NEAR:

1. **Query licensing fees.** Consumers pay NEAR tokens to access trace
   subsets. Fees are split between contributors (via credits) and
   infrastructure operation.
2. **Instance hosting fees.** Organizations that want managed
   TraceCommons instances pay for hosted operation, with settlement on
   NEAR.
3. **Federation fees.** Cross-instance queries pay a small coordination
   fee to the registry contract, funding ongoing development.

The NEAR Foundation's ongoing ecosystem support and the growing demand
for AI training data provide the market context for these revenue
streams.

## 8. References

1. TraceCommons source repository: https://github.com/zmanian/trace-commons-server (MIT/Apache-2.0).
2. NEAR Protocol documentation: https://docs.near.org
3. NEAR AI Cloud: https://docs.near.ai (TEE-hosted inference, Intel TDX + NVIDIA GPU TEE).
4. EU AI Act, Regulation (EU) 2024/1689, Article 12.
5. Letta Trajectory format: https://github.com/letta-ai/trajectory
6. TraceCommons community site: https://tracecommons.ai

---

# Proposal 3: NSF PESOSE Track 1

**Program**: NSF Pathways to Enable Open-Source Ecosystems (PESOSE)
**Track**: Track 1 (up to USD 300,000 over 2 years)
**Deadline**: Approximately September 1, 2026
**Requested amount**: USD 300,000
**Duration**: 24 months

## Proposal title

Building a Sustainable Open-Source Ecosystem for AI Agent Trace Commons

## Abstract (250 words)

AI agents are becoming the dominant interface for software development,
knowledge work, and scientific computing. Every agent session produces a
trace -- a structured record of tool calls, decisions, errors, and
outcomes. These traces are the empirical evidence of what AI does when
deployed in real systems. Today, this evidence is captured exclusively by
commercial model providers, creating an accountability gap that
undermines public trust, impedes independent safety research, and
concentrates economic value in a small number of companies.

TraceCommons is an open-source infrastructure project (MIT/Apache-2.0,
Rust, approximately 62,000 lines of code) that creates a user-owned
register of AI agent work. Contributors run a local CLI that discovers
session files from popular AI coding agents (Claude Code, Codex, and any
framework covered by the Letta Trajectory standard), redacts them using a
three-layer deterministic pipeline that is fail-closed against credential
leakage, and uploads only the scrubbed envelope to a shared server. There,
two independent quality gates -- novelty and substance -- decide whether
the record enters the register. Accepted records earn non-transferable
Trace Credits that settle on the NEAR blockchain, providing transparent
compensation when downstream consumers pay to query the evidence.

This PESOSE proposal funds the community infrastructure, governance
framework, and sustainability model needed to grow TraceCommons from a
single-team pilot into a self-sustaining open-source ecosystem. The
project serves as research infrastructure for AI safety, alignment, and
evaluation research, while providing practical AI accountability
infrastructure that aligns with emerging regulatory requirements
including EU AI Act Article 12.

## 1. Introduction and motivation

### 1.1 The open-source AI accountability gap

The open-source ecosystem has produced transformative tools for software
development (Git, Linux, GCC), data science (Jupyter, scikit-learn,
pandas), and AI model training (PyTorch, Hugging Face Transformers). But
a critical gap exists: there is no open-source infrastructure for
collecting, curating, and sharing evidence of what AI agents actually do
when deployed in the real world.

This gap matters for three reasons:

**Safety research.** Empirical AI safety research requires access to
traces of real-world AI agent behavior -- not just benchmark
performance, but actual tool calls, failure modes, recovery strategies,
and outcomes in production settings. Academic researchers currently have
no access to this data at scale.

**Accountability.** As AI agents take autonomous actions (editing code,
managing infrastructure, interacting with APIs), the question "what did
the AI actually do?" becomes a regulatory and legal requirement. The EU
AI Act Article 12, effective August 2, 2026, mandates automatic
recording of events for high-risk AI systems. No open-source
infrastructure exists to satisfy this requirement.

**Economic fairness.** Users whose AI sessions generate valuable training
data receive no compensation. The economic value of their work accrues
entirely to the companies that capture and use their sessions. A
transparent, user-owned data commons with verifiable compensation is a
prerequisite for fair participation in the AI economy.

### 1.2 TraceCommons: current state

TraceCommons addresses this gap with a working, deployed system:

**Six production crates** totaling ~62,000 lines of Rust:

- `trace-commons-protocol`: Shared DTOs, envelope schema
  (`ironclaw.trace_contribution.v1`), and a deterministic redaction
  pipeline covering 15+ secret pattern families (API keys, JWTs, PEM
  blocks, provider tokens), cue-gated high-entropy catch-all, and
  fail-closed leaked-token guard.
- `trace-commons-gate-api`: Public gate contracts. The `PerplexityScorer`,
  `Embedder`, and `VectorIndex` traits define the stable scoring seam.
  Reference implementations (byte-entropy perplexity, feature-hashed
  bag-of-tokens embedder) allow the open server to gate traces without a
  proprietary backend.
- `trace-commons-gate-enclave`: Scoring orchestrator composing perplexity
  + embedding + vector novelty into a single `OrchestrationDecision`.
  Production backends: mistral.rs (local CUDA GPU) and NEAR AI Cloud
  HTTP (TEE-hosted vLLM, Intel TDX + NVIDIA GPU TEE). HNSW vector index
  via usearch. Sentence embeddings via fastembed (BAAI/bge-large-en-v1.5).
  Chunked scoring for large traces (configurable target/max tokens,
  chunk cap, min-token floor).
- `trace-commons-server`: The hosted control plane. Eight binaries
  including `trace-commons-ingest` (~61K LOC) providing ingest, review,
  admin, worker, community, and credit APIs over Axum. 41 PostgreSQL
  migrations. Forced row-level security on every tenant-scoped table.
  Encrypted artifact store with three backends (local-encrypted,
  filesystem-remote, GCS). Cloud KMS envelope encryption with
  `KmsKeyWrapper` trait.
- `trace-commons-contributor`: Contributor-facing CLI with subcommands
  for login (device-key enrollment via Ed25519, scope-based consent),
  list (session discovery), submit (redaction + upload), status, whoami,
  logout, and mint-grant. Supports Claude Code, Codex, and Letta
  Trajectory session formats.
- `trace-commons-operator-client`: Operator API client.

**Pilot deployment** since May 2026. Server deployed on GCP. Scoring on
NEAR AI Cloud. Contributor CLI available for direct use.

**Database schema** covering 41 migrations: ingest, credit settlement
(hash-only utility attestations, settlement batches, NEAR receipt
outbox), ranking evidence (calibration, promotion, model-risk gates),
gate decisions (vector entry IDs, credit-withheld reasons), contributor
accounts (passkey/WebAuthn via webauthn-rs 0.5), NEAR identity
enrollment, deduplication, PII backstop, and per-contributor credit
caps.

**Community infrastructure** at tracecommons.ai: leaderboard, contributor
profiles, analytics summary, cohort management.

**Credit system** on NEAR: non-transferable credits, staged lifecycle
(pending, delayed utility, settlement), central-issuer ABAC, three
settlement modes (disabled/dry_run/http), fail-closed payout resolution.

### 1.3 Why PESOSE

TraceCommons has demonstrated technical viability. The missing piece is
the community and governance infrastructure needed to grow from a
single-team project into a sustainable open-source ecosystem. This is
precisely what PESOSE is designed to fund: not more code, but the
organizational, educational, and sustainability work that turns a
working project into a thriving commons.

## 2. Proposed activities

### Year 1: Core infrastructure and community (USD 150,000)

#### 2.1 Adaptive scoring infrastructure (months 1-6)

Extend the gate pipeline to handle the distributional shifts that come
with a growing, diverse contributor base:

- **Adaptive thresholds.** Replace static gate floors with EMA-tracked
  thresholds that maintain consistent selectivity as the corpus
  composition evolves. The current three floors
  (`perplexity_floor_micros`, `tail_fraction_floor_micros`,
  `novelty_floor_micros`) were calibrated against a single
  bake-off corpus; a production ecosystem needs thresholds that
  adapt.

- **Change-point detection.** CUSUM monitoring on score streams to
  detect gaming attempts (e.g., a contributor systematically
  generating traces that appear novel but contain no substantive
  content) and legitimate population shifts (e.g., a new AI framework
  producing traces with different structural characteristics).

- **Multi-stage early exit.** Bloom filter, LSH, and lightweight
  classifier stages that reject obvious duplicates and low-quality
  submissions before the expensive GPU-backed scorer runs. This is
  essential for scaling: the current per-trace scoring cost
  (27B-class model forward pass + embedding + vector search) is
  sustainable for a pilot but not for thousands of contributors.

#### 2.2 Privacy layer (months 3-9)

Formal privacy guarantees that go beyond redaction:

- **Differential privacy for aggregate statistics.** The community
  endpoints return exact counts; this adds calibrated noise with
  configurable epsilon budgets per query class.

- **Privacy budget accounting.** Per-contributor tracking of cumulative
  information leakage through aggregate queries, status responses,
  and credit events.

- **Encrypted computation preparation.** Trait abstractions and
  benchmarks for eventual homomorphic encryption of vector similarity
  queries.

#### 2.3 Contributor SDK and documentation (months 6-12)

- **Multi-language SDKs.** TypeScript and Python wrappers around the
  contributor CLI's functionality (session discovery, local redaction,
  envelope construction, upload, status/credit queries). These target
  the AI developer community's primary languages.

- **Comprehensive documentation.** Contributor guide, operator
  deployment guide, gate-api trait documentation for custom scoring
  backends, envelope schema specification.

- **Tutorial series.** Step-by-step tutorials for: submitting your
  first trace, setting up a local TraceCommons instance, building a
  custom scoring backend, integrating TraceCommons into a CI/CD
  pipeline for AI agent evaluation.

#### 2.4 Community building (months 1-12)

- **Community governance charter.** A written governance framework
  covering: who can modify gate parameters, how new scoring backends
  are approved, how disputes about trace quality are resolved, and
  how the project's technical direction is set.

- **Contributor onboarding pipeline.** Automated onboarding for new
  code contributors (not trace contributors): a "good first issues"
  label scheme, mentored contributions to the gate-api trait surface,
  and a contributor CLA process.

- **Monthly community calls.** Public calls to discuss project
  direction, review gate statistics, and coordinate development.

- **Academic partnership program.** Establish partnerships with 3-5
  AI safety or evaluation research groups to use TraceCommons as
  research infrastructure. Provide scoped API access and
  collaboration on research questions.

### Year 2: Ecosystem growth and sustainability (USD 150,000)

#### 2.5 Federation protocol (months 13-18)

- **Protocol specification.** A versioned, implementable specification
  for multi-instance TraceCommons federation: how independent instances
  exchange trace metadata (never content) for cross-corpus
  deduplication and novelty assessment.

- **Reference implementation.** Federation support in
  `trace-commons-server`, exercised between two independent test
  instances.

- **Cross-instance identity.** NEAR-mediated identity verification
  that allows contributors to prove enrollment across instances
  without revealing identity to either operator.

#### 2.6 Governance framework (months 13-18)

- **Technical steering committee.** Formalize a TSC with
  representation from contributors, operators, consumers, and
  academic researchers.

- **Gate parameter governance.** A transparent process for proposing
  and adopting changes to gate parameters, scorer models, and
  quality thresholds. Initial implementation as off-chain governance
  with on-chain recording; full on-chain governance is a future
  milestone.

- **Dispute resolution.** A process for contributors to contest gate
  decisions, with escalation to the TSC for policy questions.

#### 2.7 Sustainability model (months 19-24)

- **Business model validation.** Test three revenue models with pilot
  users: query licensing (per-access fees for trace subsets), hosted
  operation (managed instances), and research data access
  (institutional subscriptions for academic use).

- **Foundation planning.** If revenue models validate, prepare the
  organizational structure (likely a nonprofit foundation) to hold
  the project's assets, manage the commons, and sustain development
  post-grant.

- **Ecosystem metrics.** Establish and publish metrics for ecosystem
  health: contributor diversity, corpus growth rate, gate selectivity,
  consumer adoption, and revenue per contributor.

#### 2.8 Developer advocacy and education (months 13-24)

- **Conference presentations.** Present TraceCommons at 3-4 relevant
  conferences: NeurIPS (AI safety track), USENIX Security (privacy
  engineering), Open Source Summit, and a NEAR ecosystem event.

- **Workshop series.** Hands-on workshops for: AI safety researchers
  (using the corpus for empirical research), AI agent developers
  (contributing traces and building with the quality dashboard), and
  infrastructure engineers (deploying and operating TraceCommons
  instances).

- **Curriculum materials.** Course modules on AI accountability
  infrastructure suitable for graduate-level AI ethics or AI safety
  courses. Materials include: the design of quality gates for AI
  data, privacy engineering for contributed data, and economic models
  for data commons.

## 3. Intellectual merit

### 3.1 Novel quality gating

TraceCommons's two-gate pipeline (novelty + substance) represents a
novel approach to data quality control for contributed datasets. The
perplexity-based substance gate was validated through a rigorous
bake-off process that tested four candidate models and discovered a
fundamental finding: aggregate perplexity discriminates novel reasoning
from filler only at 27B-class model capacity (AUC > 0.93), while
8B-class models remain inverted below chance (AUC < 0.35). This
finding has implications for the broader field of LLM-based data
quality assessment.

### 3.2 Privacy-preserving data commons

The combination of local-first redaction, TEE-hosted scoring,
differential privacy for aggregates, and NEAR-settled compensation
creates a novel architecture for privacy-preserving data commons.
This architecture addresses an open problem: how to build a shared
resource from privacy-sensitive contributed data while maintaining
individual data sovereignty.

### 3.3 Adaptive quality thresholds

The proposed EMA + CUSUM adaptive threshold system addresses the
challenge of maintaining consistent data quality standards in a
growing, heterogeneous contributor population. This is a contribution
to the broader field of online quality control for crowd-sourced
datasets.

### 3.4 Federated data commons

The federation protocol addresses the tension between centralized
quality control and decentralized operation. The protocol's approach
-- exchange hashes and similarity estimates, never content -- is a
novel contribution to federated data management.

## 4. Broader impacts

### 4.1 AI accountability

TraceCommons provides the first open-source infrastructure for
evidence-based AI accountability. As AI agents take more autonomous
actions, the ability to audit their behavior through curated,
quality-gated trace logs is a prerequisite for responsible AI
deployment. The project directly supports compliance with EU AI Act
Article 12 and provides a model for similar regulations globally.

### 4.2 AI safety research

Academic AI safety researchers currently lack access to large-scale,
real-world AI agent traces. TraceCommons fills this gap with a curated,
privacy-preserving corpus accessible through scoped API queries.
The proposed academic partnership program (Section 2.4) ensures that
this resource reaches the research community.

### 4.3 Economic fairness

The Trace Credit system demonstrates a viable model for compensating
data contributors -- a direct response to the growing concern about
the uncompensated extraction of user data by AI companies. By building
compensation into the protocol rather than adding it as an afterthought,
TraceCommons establishes a pattern that other data commons can follow.

### 4.4 Workforce development

The proposed curriculum materials (Section 2.8) contribute to
workforce development in AI accountability -- a field with growing
demand but few educational resources. The hands-on workshop series
trains the next generation of engineers who will build and operate
AI accountability infrastructure.

### 4.5 Open-source ecosystem model

TraceCommons serves as a case study in building sustainable open-source
ecosystems around novel AI infrastructure. The governance framework,
sustainability model, and community building activities funded by this
grant are applicable to other open-source AI infrastructure projects
facing similar challenges.

### 4.6 Broadening participation

The contributor CLI's support for multiple session formats (Claude
Code, Codex, Letta Trajectory) and the proposed multi-language SDKs
(TypeScript, Python) lower the barrier for diverse contributors.
The credit system provides economic incentives for participation from
underrepresented communities whose AI usage patterns are
underrepresented in existing training corpora.

## 5. Timeline with milestones

### Year 1 (months 1-12)

| Month | Activity | Deliverables |
|---|---|---|
| 1-3 | Adaptive scoring engine | EMA thresholds, CUSUM detection, per-contributor normalization; open-source release |
| 3-6 | Multi-stage gate pipeline | Bloom filter, LSH, classifier; benchmarks showing 60%+ early-exit rate |
| 3-9 | Privacy layer | Differential privacy for aggregates, privacy budget accounting; formal analysis |
| 6-9 | Contributor SDKs | TypeScript and Python SDKs; published to npm and PyPI |
| 6-12 | Documentation | Contributor guide, operator guide, gate-api docs, tutorials |
| 1-12 | Community building | Governance charter, 12 monthly calls, 3-5 academic partnerships |

### Year 2 (months 13-24)

| Month | Activity | Deliverables |
|---|---|---|
| 13-18 | Federation protocol | Spec document, reference implementation, two-instance integration tests |
| 13-18 | Governance framework | TSC formation, gate parameter governance process, dispute resolution |
| 19-24 | Sustainability model | Revenue model validation, foundation planning, ecosystem metrics |
| 13-24 | Developer advocacy | 3-4 conference presentations, workshop series, curriculum materials |
| 13-24 | Ecosystem growth | SDK adoption metrics, contributor growth, consumer pilot programs |

## 6. Budget justification

### Year 1: USD 150,000

| Category | Amount | Justification |
|---|---|---|
| Senior developer (0.5 FTE, 12 mo) | USD 72,000 | Adaptive scoring, privacy layer, multi-stage pipeline. Requires Rust systems programming, privacy engineering, and ML expertise. Market rate for this combination is USD 180K+/yr; 0.5 FTE reflects shared effort with project maintainers. |
| Junior developer (0.5 FTE, 12 mo) | USD 42,000 | SDK development, documentation, tutorial creation. Market rate USD 105K+/yr for Rust+TypeScript+Python. |
| Community manager (0.25 FTE, 12 mo) | USD 18,000 | Monthly calls, contributor onboarding, academic partnership coordination. |
| Infrastructure and cloud | USD 10,000 | GPU compute for corpus-scale testing, CI/CD, staging environments, NEAR testnet gas. |
| Travel and conferences | USD 5,000 | One conference presentation (Year 1 serves as preparation for Year 2 advocacy). |
| Miscellaneous | USD 3,000 | External code review, security audit of privacy implementation. |

### Year 2: USD 150,000

| Category | Amount | Justification |
|---|---|---|
| Senior developer (0.5 FTE, 12 mo) | USD 72,000 | Federation protocol, governance infrastructure, sustainability model implementation. |
| Junior developer (0.25 FTE, 12 mo) | USD 21,000 | Dashboard improvements, SDK maintenance, documentation updates. |
| Community manager (0.5 FTE, 12 mo) | USD 36,000 | Increased community activity: TSC coordination, governance facilitation, workshop organization, ecosystem metrics. |
| Travel and conferences | USD 12,000 | 3-4 conference presentations, 2-3 workshops. |
| Infrastructure and cloud | USD 6,000 | Multi-instance federation testing, production staging. |
| Miscellaneous | USD 3,000 | Legal review of governance documents, foundation planning costs. |

## 7. Team qualifications

*[Template -- to be filled by applicants]*

**Principal Investigator (PI):** [Name], [Title] at [Institution].

*Relevant expertise:* [Describe experience with open-source ecosystem
building, Rust systems programming, AI infrastructure, privacy
engineering, and/or blockchain systems. Include relevant publications,
open-source projects, and prior NSF or similar funding.]

**Co-PI:** [Name], [Title] at [Institution].

*Relevant expertise:* [Describe complementary expertise -- e.g., AI
safety research, community governance, cryptographic protocols, or
software engineering education.]

**Senior Personnel:**

- [Name], [Role]. [Expertise in NEAR blockchain development, smart
  contract security, decentralized identity.]
- [Name], [Role]. [Expertise in differential privacy, formal
  verification, or privacy-preserving computation.]

**Evidence of capability:** TraceCommons has been under continuous
development since early 2026. The team has delivered: a six-crate Rust
workspace (~62,000 LOC); a 41-migration PostgreSQL schema covering
ingest, credit settlement, ranking evidence, accounts, and
deduplication; a production deployment on GCP with NEAR AI Cloud
scoring; a contributor CLI supporting three session formats; a community
site; and a credit settlement system on NEAR. The project has
demonstrated sustained engineering velocity, with 40+ pull requests
merged and eight CI-gated binaries shipping continuously.

## 8. Prior NSF support

*[Template -- to be filled by applicants]*

*If any PI or Co-PI has received prior NSF support, list the award
number, title, amount, period, and briefly summarize the outcomes.*

## 9. Sustainability plan

### 9.1 Short-term (during grant, months 1-24)

Grant funding supports the engineering and community work described
above. During this period, the project establishes the governance
framework and validates revenue models.

### 9.2 Medium-term (months 24-48)

Three revenue streams sustain the project post-grant:

1. **Query licensing.** Downstream consumers (AI labs, evaluation
   companies, benchmarking services) pay per-access fees for trace
   subsets. Fees are split between contributors (via NEAR credits) and
   infrastructure operation.
2. **Hosted operation.** Organizations that want managed TraceCommons
   instances pay for hosted operation. The open-source codebase remains
   freely available for self-hosting.
3. **Institutional subscriptions.** Academic institutions pay
   subsidized annual subscriptions for research access, providing a
   stable revenue base alongside commercial licensing.

### 9.3 Long-term (48+ months)

If revenue models validate during Year 2 (Section 2.7), the project
transitions to a nonprofit foundation structure that:

- Holds the intellectual property under the existing MIT/Apache-2.0
  dual license (irrevocable open source).
- Employs a small core team (2-3 engineers, 1 community manager)
  funded by revenue.
- Manages the commons (gate parameters, scoring model selection,
  federation registry) through the TSC governance framework.
- Distributes surplus revenue to contributors through enhanced NEAR
  credit rates.

### 9.4 Risk mitigation

| Risk | Mitigation |
|---|---|
| Insufficient consumer demand | The EU AI Act creates regulatory demand for auditable AI traces regardless of voluntary adoption. |
| Contributor attrition | Credit system provides ongoing economic incentive; community governance gives contributors voice in project direction. |
| Technical obsolescence | The gate-api trait surface is designed for backend substitution; new models and scoring methods slot in without protocol changes. |
| Competing proprietary solutions | The open-source, federated architecture is a structural advantage: no proprietary solution can offer the same transparency and contributor control. |

## 10. Data management plan

### 10.1 Types of data

- **Trace envelopes:** Redacted AI agent session records. Never contain
  raw user content. Stored encrypted with per-object envelope
  encryption (AES-GCM-256) in object storage. Retention governed by
  contributor consent scope and operator policy.
- **Gate decisions:** Per-trace scoring outcomes (perplexity, novelty,
  acceptance). Stored in PostgreSQL with forced RLS. No trace content.
- **Credit events:** Pending/settled credit amounts per contributor.
  Hash-only audit trail. Settled on NEAR blockchain.
- **Aggregate statistics:** Corpus composition, quality metrics,
  contributor statistics. Subject to differential privacy (post-M3).

### 10.2 Data sharing

Research access to the corpus is provided through scoped API queries
with formal privacy guarantees. Academic partners receive API
credentials with defined query budgets. No bulk export of trace
envelopes is supported; all access is mediated through the gate
pipeline's selective disclosure mechanisms.

### 10.3 Data preservation

The NEAR blockchain provides immutable, publicly auditable records of
trace acceptance and credit settlement. Database backups follow the
existing `docs/operator/backup-restore.md` runbook. The open-source
codebase is preserved on GitHub under MIT/Apache-2.0.

## 11. References

1. EU AI Act, Regulation (EU) 2024/1689, Article 12: Automatic recording of events. Official Journal of the European Union.
2. Dwork, C. & Roth, A. (2014). "The Algorithmic Foundations of Differential Privacy." Foundations and Trends in Theoretical Computer Science, 9(3-4), 211-407.
3. Vaswani, A., et al. (2017). "Attention is All You Need." NeurIPS 2017.
4. Bender, E.M., Gebru, T., McMillan-Major, A., & Shmitchell, S. (2021). "On the Dangers of Stochastic Parrots." FAccT 2021.
5. Bommasani, R., et al. (2021). "On the Opportunities and Risks of Foundation Models." arXiv:2108.07258.
6. Raji, I.D., et al. (2020). "Closing the AI Accountability Gap." FAT* 2020.
7. Shokri, R., Stronati, M., Song, C., & Shmatikov, V. (2017). "Membership Inference Attacks Against Machine Learning Models." IEEE S&P 2017.
8. Groth, J. (2016). "On the Size of Pairing-based Non-interactive Arguments." EUROCRYPT 2016.
9. Page, E.S. (1954). "Continuous Inspection Schemes." Biometrika, 41(1/2), 100-115.
10. Indyk, P. & Motwani, R. (1998). "Approximate Nearest Neighbors: Towards Removing the Curse of Dimensionality." STOC 1998.
11. TraceCommons source repository: https://github.com/zmanian/trace-commons-server (MIT/Apache-2.0).
12. NEAR Protocol documentation: https://docs.near.org
13. NEAR AI Cloud: https://docs.near.ai
14. Letta Trajectory format: https://github.com/letta-ai/trajectory

---

# Appendix A: Cross-Proposal Technical Summary

All three proposals build on the same TraceCommons codebase. This
appendix provides a unified technical reference.

## Codebase metrics

| Metric | Value |
|---|---|
| Language | Rust (edition 2024, MSRV 1.92) |
| License | MIT OR Apache-2.0 |
| Crates | 6 workspace members |
| Production LOC | ~62,000 (server ingest binary alone is ~61K) |
| Database migrations | 41 (PostgreSQL, forced RLS) |
| Binaries | 8 (ingest, upload-claim-issuer, gate-calibrate, pilot-bootstrap, vector-replay, review, admin, worker) |
| CI gates | 8 (fmt, clippy, 3x cargo check variants, test, pilot-bootstrap smoke, operator-binaries smoke) |
| Deployment | GCP (pilot), NEAR AI Cloud (scoring) |
| Community site | tracecommons.ai (Cloudflare Pages) |

## Key technical decisions

1. **Trait-object scoring seam.** The gate-api crate defines
   `PerplexityScorer`, `Embedder`, and `VectorIndex` as traits.
   Proprietary backends implement the same traits. This seam is the
   contract that makes open-source and commercial deployments
   interoperable.

2. **Hash-only audit.** No raw content, URL, bearer token, ARN, account
   reference, transaction hash, contributor identity, or trace body
   ever appears in a stored row or log string. This is a structural
   invariant enforced across the codebase.

3. **Fail-closed default.** When a required gate dependency is missing,
   the system refuses the path rather than falling back to a
   less-restricted mode. This applies to: KMS key wrapper (production
   trust boundary), PII filter (batch refused if unreachable), secret
   redaction (session refused if leakage detected), settlement (credit
   held if payout ambiguous).

4. **Chunked scoring.** Large traces are chunked into configurable
   segments (target 2048 tokens, max 4096, cap 16 chunks). Per-chunk
   perplexity is aggregated via weighted mean (excluding short chunks
   below a minimum token threshold). Per-chunk embeddings drive per-chunk
   novelty, with intra-trace deduplication at the insert threshold.

5. **Three-layer redaction.** (1) Named-pattern regexes for 15+ known
   secret families. (2) Cue-gated high-entropy catch-all for unknown
   formats. (3) Per-session fail-closed leaked-token guard. Validated
   against 992 real Claude Code sessions with zero survivors for
   pattern-shaped secrets.

## Existing contributor integrations

| Source | Discovery | Notes |
|---|---|---|
| Claude Code | `~/.claude/projects/<project>/*.jsonl` + subagent transcripts | Native support |
| Codex | `~/.codex/sessions/**/rollout-*.jsonl` | Native support |
| Letta Trajectory | Explicit `--trajectory <path>` | Covers Hermes, Letta Code, OpenClaw, OpenHands, Pi, Deep Agents |

## Consent model

Scope-based, not capped to a single default. The instance sets a
ceiling via its onboarding policy. The contributor chooses scopes at
login (interactive prompt or `--scopes` flag). The server clamps to the
intersection. Available scopes: `debugging_evaluation` (always on),
`benchmark_generation`, `ranking_model_training`, `model_training`,
`public_attribution`.

## Credit lifecycle

1. Client computes local pending estimate from trace value scorecard
   (9 components: privacy risk, quality, replayability, capped novelty,
   duplicate penalty, coverage, difficulty, dependability, correction
   value).
2. Server records pending credit event on acceptance.
3. Delayed credit events appended through audited paths (benchmark
   conversion, ranker training, reviewer adjustments).
4. Settlement step turns eligible pending credit into non-transferable
   NEAR account credit.
5. Payout resolves to a contributor-designated NEAR identity (fail-closed
   on ambiguity).

---

# Appendix B: Letters of Support Template

*[Each proposal benefits from letters of support. Template below.]*

**For NLnet (Restack):**
- Letter from an EU-based organization that would use TraceCommons for
  AI Act Article 12 compliance.
- Letter from an open-source AI project that would contribute traces.

**For NEAR Foundation:**
- Letter from the NEAR AI team confirming the existing technical
  relationship (TEE-hosted scoring).
- Letter from a potential marketplace consumer (AI lab, evaluation
  company).

**For NSF (PESOSE):**
- Letters from 2-3 academic research groups expressing interest in
  using TraceCommons as research infrastructure.
- Letter from an industry partner willing to pilot hosted operation.
- Letter from a university expressing interest in the proposed
  curriculum materials.
