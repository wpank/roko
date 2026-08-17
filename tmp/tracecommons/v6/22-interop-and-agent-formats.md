# Interoperability and Agent Trace Formats

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving registry of AI coding
agent session traces (~235K LOC, 6 crates, MIT/Apache-2.0). Contributors submit scrubbed
traces; TC scores quality and novelty inside TEEs (Trusted Execution Environments --
hardware-isolated encrypted compute enclaves); contributors earn NEAR blockchain credits via
the formula `q = f * g * a` (f = freshness, g = gate score, a = attestation weight). ~352
submissions, 3 contributors, 6 GitHub stars. TC's network effects depend on broad ingest: the
more agent runtimes that can submit traces without custom exporters, the faster the corpus
grows. This document maps the interoperability landscape as of August 2026, specifies how TC
ingests traces from each major source, and describes the hub architecture that prevents TC
from becoming yet another silo.

---

## 1. The Interop Landscape (August 2026)

The agent trace interchange substrate has consolidated around three distinct layers, each
operating at a different level of abstraction.

### 1.1 Wire Format: OTel OTLP

OpenTelemetry Protocol (OTLP) is the universal transport for observability data. Traces,
metrics, and logs are serialized as protobuf (preferred) or JSON and sent over gRPC or HTTP.
Every major observability backend (Datadog, Elastic, Honeycomb, Grafana) accepts OTLP.
Every major agent framework that instruments itself emits OTLP. OTLP is not an LLM-specific
standard -- it predates the current agent wave -- but it has become the de facto wire format
for agent telemetry by virtue of its ubiquity.

TC should expose an OTLP receiver endpoint (gRPC + HTTP) as its primary ingest path for
instrumented agent frameworks. An agent framework that already emits OTel spans to Datadog
can be redirected to TC with a single exporter configuration change.

### 1.2 Semantic Conventions: OTel GenAI and OpenInference

OTLP carries raw spans. The semantic conventions define what the attribute keys and values
mean. Two convention sets are relevant to TC:

**OTel GenAI (`gen_ai.*`)**: The CNCF-backed standard for LLM operations. Governed by the
OTel GenAI SIG. All conventions remain at "Development" status with no stable graduation
timeline (see doc 18 for full technical state). Adopted by Datadog (from OTel v1.37,
December 2025), Elastic, Honeycomb, and the OpenAI Python SDK. The OpenAI instrumentation
path is the most mature; Anthropic/Bedrock/Cohere are covered by community libraries of
varying quality.

**OpenInference**: A parallel, independently maintained convention set created by Arize AI
for their Phoenix observability platform. Not part of the OTel project. Defines richer
LLM-specific metadata than base OTel GenAI, including first-class RAG span types, retrieval
metadata, and custom span kinds (`LLM`, `CHAIN`, `TOOL`, `AGENT`, `EMBEDDING`, `RETRIEVER`,
`RERANKER`, `GUARDRAIL`). Arthur AI chose OpenInference over raw OTel GenAI specifically for
its richer RAG primitives. Adopted by LangChain (via `openinference-instrumentation-langchain`)
and LlamaIndex. Neither convention set has won. TC must support both.

### 1.3 Provenance Backbone: W3C PROV-DM / PROV-O

W3C PROV-DM (Data Model) and its OWL serialization PROV-O define the standard vocabulary for
expressing provenance: which entities were derived from which activities, which agents were
responsible. The standard is well-established and predates modern ML -- PROV-O is used in
government data catalogs, scientific reproducibility systems, and supply-chain compliance.

Wang et al. (arXiv:2606.04990, "From Agent Traces to Trust") survey the application of W3C
PROV to AI/ML pipelines and confirm that PROV-DM provides adequate expressivity for tracing
the full lifecycle of an agent session: from input documents through tool calls to output
artifacts, with agent identity and model version attached to each activity. TC should export
W3C PROV-mapped lineage to serve downstream provenance consumers: compliance workflows
(doc 15), downstream value tracking (doc 16), and external audit chains (doc 13).

The survey defines a six-dimensional taxonomy for evidence tracing in LLM agents that TC
should adopt as its metadata design checklist:

1. **Trace sources**: Where evidence originates -- user inputs, tool outputs, model
   generations, environment observations. TC's canonical envelope captures tool call
   sequences and model outputs but does not currently tag the source type per evidence unit.
2. **Evidence units**: The atomic elements of a trace -- individual tool calls, reasoning
   steps, retrieval results. TC's OTel span mapping already represents these at the span
   level; the gap is labeling each span with its evidence-unit role.
3. **Provenance relations**: The causal links between evidence units -- `wasDerivedFrom`,
   `wasGeneratedBy`, `wasInformedBy`. TC's planned PROV export (Section 5.3) covers this
   dimension directly.
4. **Tracing granularity and timing**: Whether provenance is captured per-token, per-step,
   or per-session, and whether tracing is online (during execution) or offline
   (post-hoc reconstruction). TC operates at per-session granularity with offline tracing.
   Finer per-step granularity is available from OTel-instrumented sources but not yet
   exploited in TC's scoring pipeline.
5. **Representation forms**: How provenance is serialized -- flat logs, typed graphs,
   knowledge graphs. The survey identifies typed graphs as the highest-fidelity
   representation. TC should adopt typed-graph representation as its target provenance
   format for NEAR lineage export, building on PROV-O's OWL graph serialization.
6. **Trust functions**: How provenance evidence is consumed -- verification, attribution,
   debugging, compliance. TC's existing trust surface (TEE attestation, doc 13) addresses
   verification; the credit formula (doc 16) addresses attribution; this document's PROV
   export addresses compliance. Debugging remains an unaddressed trust function -- TC does
   not currently provide tools for trace consumers to diagnose failures using provenance
   evidence.

TC's trace schema should explicitly address all six dimensions. Dimensions 1, 2, and 4
represent current gaps; dimensions 3, 5, and 6 are partially covered by existing or planned
features. The schema extension roadmap (Phase 3) should close these gaps incrementally.

### 1.4 Governance: AAIF and W3C AI Agent Protocol CG

Two governance bodies are shaping the agent interoperability standards space:

**Linux Foundation Agentic AI Foundation (AAIF)**: Established December 2025. Currently
hosting two adopted protocols -- MCP (Model Context Protocol, Anthropic origin) and A2A
(Agent-to-Agent, Google origin). MCP standardizes how LLMs connect to tools and data sources;
A2A standardizes inter-agent communication (v1.0.0, 150+ organizations). TC's cross-agent
session stitching and credit attribution work (doc 16) benefits from A2A identity support.

A comprehensive four-protocol taxonomy survey (arXiv:2505.02279, "A survey of agent
interoperability protocols") maps the full protocol landscape and identifies two additional
protocols that TC should track:

**ACP (Agent Communication Protocol)**: RESTful HTTP-based protocol for structured
asynchronous agent communication. ACP uses MIME-typed multipart messages, explicit session
management, and DID (Decentralized Identifier) integration for agent identity. For TC, ACP's
structured HTTP semantics and session management provide a natural transport for asynchronous
trace submissions: a contributor's agent could open an ACP session with TC's ingest endpoint,
submit traces as MIME-typed multipart payloads (trace envelope + metadata + attestation
artifacts), and receive quality scores as response messages within the same session context.
ACP's DID integration also aligns with TC's planned identity layer for contributor
verification.

**ANP (Agent Name Protocol)**: Open-network agent discovery protocol built on W3C DIDs and
JSON-LD semantic graphs. ANP enables agents to discover and verify each other without a
central registry. For TC, ANP addresses the contributor discovery problem: new contributors
could register their agent's capabilities and trace generation profiles as ANP-discoverable
JSON-LD descriptions, allowing TC to discover potential contributors programmatically rather
than relying solely on manual onboarding. ANP's W3C DID foundation also provides a
decentralized identity primitive that complements TC's NEAR-account-based contributor
identity.

The survey proposes a phased adoption roadmap: **MCP -> ACP -> A2A -> ANP**. These protocols
are complementary layers, not competing alternatives. MCP handles tool invocation (already
in TC's near-term roadmap as MCP server exposure). ACP adds structured async communication
(relevant for TC's ingest API design). A2A adds peer-to-peer task delegation with identity
(relevant for multi-agent credit splitting). ANP adds open-network discovery (relevant for
decentralized contributor onboarding). TC should adopt this phased sequence as its own
protocol integration roadmap, adding ACP ingest support in Phase 2 and ANP discovery in
Phase 3 alongside A2A identity.

**W3C AI Agent Protocol Community Group (CG)**: Active as of August 2026. Working toward
formal W3C specifications for agent communication. Specs expected 2026-2027. TC should
monitor but not block on W3C CG output -- the AAIF-hosted MCP and A2A are operational now
and have the ecosystem adoption.

---

## 2. OTel GenAI Convention Status

All `gen_ai.*` semantic conventions are still at "Development" status. Nothing is stable.

Key facts as of August 2026 (from doc 18):

- All GenAI conventions moved to the dedicated `open-telemetry/semantic-conventions-genai`
  repository at main-repo **v1.42.0** (June 12, 2026). The main repo now has zero GenAI
  definitions.
- The dedicated repository has **no tagged release**, no finalized schema URL, and no
  published Stable graduation timeline.
- The `gen_ai.system` attribute was renamed to `gen_ai.provider.name` at v1.39.0. This is
  a breaking change -- any code filtering on `gen_ai.system` silently stops matching traces
  emitted by frameworks that adopted the new name.
- The `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` flag enables dual-emission
  (both old and new attribute names) in instrumentation libraries that support it. However,
  per John Hodge's July 2026 analysis: "not every framework honors the variable identically."
  TC must validate real exported spans per SDK rather than trusting the flag.
- Datadog native OTel GenAI support shipped December 1, 2025 (OTel SDK/Collector v1.37),
  demonstrating production adoption of Development-status conventions.

TC's dual-emission strategy -- accepting both `gen_ai.system` and `gen_ai.provider.name` --
is architecturally correct. The alias shim (~50 LOC, described in doc 18 Section 7) is the
implementation vehicle. It must be validated against real span exports from each SDK, not
just trusted based on the stability flag.

---

## 3. OpenInference as Complement

OpenInference provides richer LLM-specific metadata than base OTel GenAI in two areas that
matter for TC:

**First-class RAG span types**: OpenInference defines `RETRIEVER` and `RERANKER` span kinds
with structured retrieval metadata (document IDs, scores, chunk boundaries). OTel GenAI has
no native RAG primitives -- retrieval steps are represented as generic spans with no semantic
structure. For TC's trajectory RAG feature (doc 12), this difference is significant: ingesting
OpenInference traces gives TC structured retrieval context that OTel GenAI traces lack.

**Span kind taxonomy**: OpenInference's eight span kinds (`LLM`, `CHAIN`, `TOOL`, `AGENT`,
`EMBEDDING`, `RETRIEVER`, `RERANKER`, `GUARDRAIL`) map more naturally to TC's internal trace
structure than OTel's generic span model. TC's structural embedding pipeline (doc 17) can
extract richer tool-call graphs from OpenInference traces.

Both convention sets travel on the OTLP wire format. They are not competing transports --
they are competing vocabularies on the same transport. TC detects which convention set a
trace uses by checking for sentinel attributes (`gen_ai.provider.name` / `gen_ai.system` for
OTel GenAI; `llm.provider` / `llm.model_name` for OpenInference) and applies the appropriate
normalization layer. See doc 18 Sections 5 and 8 for the full mapping tables.

---

## 4. Cross-Agent Session Format Matrix (August 2026)

The table below describes where session data lives, what format it takes, and whether it is
parseable without an OTel pipeline.

| Agent | Where session data lives | On-disk format | Parseable w/o OTel? | OTel GenAI emission |
|---|---|---|---|---|
| Claude Code | Local files | JSON transcript / JSONL | Yes | Beta |
| OpenAI Codex | Local | JSON/JSONL | Yes | Yes |
| Cursor | Local app state + cloud | SQLite/JSON (app-specific) | Partially | Unknown |
| GitHub Copilot | Mostly API/cloud; limited local | Proprietary | Rarely (API only) | Unknown |
| Gemini CLI | Local | JSON/JSONL | Yes | Likely (Google OTel investment) |
| IronClaw | NEAR AI runtime | TC-defined format | Yes (native integration) | N/A (native) |

**Important caveat**: This matrix is partially inferred from public documentation and community
reports as of August 2026. Agent client formats change across releases. Verify each agent's
exact current version before relying on these entries for integration work. In particular:
Cursor's local storage format is not publicly documented and has changed in past releases;
GitHub Copilot's local telemetry is gated behind enterprise agreements; Gemini CLI's OTel
emission is inferred from Google's broader OTel investment and is not confirmed from primary
sources.

### 4.1 Claude Code

Claude Code's `SessionEnd` hook (one of 30 hooks documented in the Claude Code integration
surface) fires on session completion and provides access to the session transcript as a JSON
structure. TC's primary acquisition path for Claude Code sessions (doc 01) uses this hook:

```
SessionEnd fires -> hook invokes tc submit -> transcript uploaded to TC ingest
```

This path does not require OTel -- TC reads the native transcript format directly. A
bespoke parser converts the transcript into TC's canonical trace envelope. This is simpler
than an OTel roundtrip and more reliable given that Claude Code's OTel emission is still
in beta. The `SessionEnd` hook integration is a Week 1 task (doc 00 priority item 5).

### 4.2 OpenAI Codex

Codex emits OTel GenAI spans via the official `opentelemetry-sdk-python` instrumentation,
which is the most mature OTel GenAI path. TC's OTLP receiver handles Codex traces directly
with no bespoke parser needed. The `gen_ai.provider.name` attribute carries the value
`openai`; the alias shim normalizes `gen_ai.system` from older Codex versions.

### 4.3 Cursor

Cursor stores session state in a combination of local SQLite (for editor state) and
proprietary cloud sync. The local SQLite schema is not publicly documented. Parsing Cursor
sessions without OTel requires reverse-engineering the local database format, which changes
across Cursor releases. TC should deprioritize a bespoke Cursor parser until Cursor ships
a stable export API or native OTel emission.

### 4.4 GitHub Copilot

GitHub Copilot's telemetry is primarily cloud-side and not accessible to third-party tools
without GitHub enterprise agreements. OTel emission status is unknown from public sources.
TC cannot build a reliable ingest path for Copilot without a partnership or API access.
Treat as out-of-scope until GitHub publishes a trace export API.

### 4.5 Gemini CLI

Gemini CLI is a newer entrant (Google I/O 2026). Session data is stored locally in JSON/JSONL
format analogous to Claude Code. Google's broader OTel investment (they are a founding CNCF
member and major contributor to OTel) makes native OTel GenAI emission likely, but this is
inferred, not confirmed from primary sources. TC should monitor Gemini CLI releases and add
a bespoke parser once the local format stabilizes.

### 4.6 IronClaw

IronClaw (NEAR AI's open-source agent runtime, 12.6K GitHub stars) is TC's primary
integration partner with 3 PRs merged and 20K+ lines of integration code. IronClaw sessions
arrive in TC's native trace envelope format -- no format conversion required. The
IronClaw-to-TC path is TC's most mature ingest path and the primary source of the current
~352 submissions. Issue #219 (redaction penalizing quality scores) disproportionately affects
IronClaw contributors because IronClaw's redaction is particularly thorough; fixing this
(doc 08) is prerequisite to healthy IronClaw growth.

### 4.7 Future Format Challenge: Latent-Space Multi-Agent Communication

The agent format matrix above assumes that inter-agent communication happens through
text-based messages that leave inspectable traces. LatentMAS (arXiv:2511.20639, Zou et al.,
ICML 2026 Spotlight, "Latent Collaboration in Multi-Agent Systems") demonstrates a
fundamentally different paradigm: agents collaborate through continuous latent space
representations (shared KV-cache segments) rather than text-based message passing.

LatentMAS evaluated across 9 benchmarks and achieved up to 14.6% higher accuracy while
reducing output tokens by 70.8-83.7% and achieving 4x-4.3x faster inference compared to
text-based multi-agent communication. These gains come from eliminating the
information-lossy text serialization step between agents -- agents share dense neural
representations directly.

This is a coverage gap for TC. When agents communicate through latent space, there is no
text-level trace of their inter-agent communication. TC's canonical trace envelope, OTel
spans, and OpenInference conventions all assume text-serializable interactions. A LatentMAS
agent cluster produces observable traces only at the boundaries: the initial user input and
the final aggregated output. The intermediate collaboration -- which agent contributed what
reasoning, how disagreements were resolved, what information was shared -- exists only as
continuous vector operations with no human-readable or schema-mappable representation.

**Near-term mitigation**: Treat a LatentMAS agent cluster as a single logical agent for TC
purposes. Capture the cluster's input and final output as a standard trace. Tag the trace
with a `collaboration_mode: latent` attribute to distinguish it from text-based multi-agent
traces. This preserves TC's ability to score the trace's quality and novelty while
acknowledging that per-agent attribution within the cluster is not possible.

**Longer-term**: TC's format working group should track whether OTel GenAI or OpenInference
will add semantic conventions for latent-space exchanges. Possible approaches include
logging latent-space dimensionality, KV-cache sharing topology, and per-agent contribution
magnitude (e.g., attention weight attribution). These conventions do not exist today and are
unlikely to emerge before 2027 given the current pace of GenAI convention development
(Section 2). Until they do, per-agent latent attribution remains outside TC's capture scope.

### 4.8 Scaffold Architecture and Trace Structure

Agent scaffolds -- the architectural patterns that organize an agent's internal processing
pipeline -- determine the structure of the traces an agent produces. AgentSpec
(arXiv:2606.14674, "AgentSpec: Understanding Embodied Agent Scaffolds Through Controlled
Composition," UC San Diego/JHU/UW/UIUC) provides a modular specification with typed
interfaces between five phases:

```
Perception -> Memory -> Reasoning -> Reflection -> Action
```

Each phase produces distinct trace artifacts. A Perception phase generates spans for input
parsing and context loading. Memory spans cover state retrieval and working-memory updates.
Reasoning spans contain the core LLM inference calls. Reflection spans capture
self-evaluation and error-checking steps. Action spans record tool invocations and output
generation. Critically, AgentSpec found that **scaffold compatibility matters more than
isolated module strength** -- a well-integrated mediocre scaffold outperforms a poorly
integrated stack of individually strong modules.

For TC, the implication is that trace quality is partly a function of scaffold architecture.
A trace from a scaffold that includes a Reflection phase is structurally more valuable than
one without, because the Reflection phase captures self-correction events -- a Reasoning
error caught by Reflection produces a trace segment showing the error, the detection, and
the correction, which is precisely the kind of evidence that TC's trajectory RAG (doc 12)
should surface to future agents.

TC's OTel mapping should capture scaffold phase metadata per span. A `scaffold.phase`
attribute (values: `perception`, `memory`, `reasoning`, `reflection`, `action`) would enable
phase-filtered retrieval from the trace corpus and enrich quality scoring. This attribute
is not part of any existing OTel GenAI or OpenInference convention -- TC would define it as
a custom attribute in its canonical envelope schema and map it from framework-specific
indicators where available (e.g., LangChain's chain types, CrewAI's agent roles).

---

## 5. TC's Interop Strategy: Hub, Not Silo

TC becomes a silo if it can only ingest one trace format. It becomes a hub by accepting any
trace format and exporting in formats that downstream systems (compliance tools, provenance
chains, research pipelines) can consume. The hub architecture has four components: ingest,
store, export, and identity.

### 5.1 Ingest Layer

TC's ingest layer accepts traces from three source categories:

**Instrumented OTel emitters** (Codex, any OTel-native agent framework): OTLP receiver
endpoint (gRPC + HTTP). The alias shim normalizes convention-version differences. Per-SDK
conformance tests validate real exported spans.

**Native integrations** (IronClaw, Claude Code via SessionEnd hook): Bespoke parsers that
read the agent's native format and produce TC's canonical trace envelope. These parsers are
simpler to maintain than OTel roundtrips and more reliable for formats that have not
stabilized around OTel yet.

**OpenInference emitters** (LangChain, LlamaIndex, LlamaTrace): OTLP receiver plus an
OpenInference normalization layer that maps `llm.*` / `embedding.*` / `retriever.*`
attributes to TC's internal schema. The detection logic (doc 18 Section 8.3) identifies
which convention set a trace uses and routes it to the appropriate normalizer.

### 5.2 Store: TC's Canonical Trace Envelope

Internally, TC stores all traces in its own canonical envelope format, independent of the
wire format they arrived on. The envelope captures:

- Provider and model identity (normalized from whatever convention was used)
- Tool call sequence with timestamps
- Token usage per LLM call
- Redacted content references
- Provenance tier (1/2/3, from doc 13)
- Source format tag (for diagnostics and migration)

The canonical envelope is TC's internal API surface. External formats are wire formats only
-- they are normalized at ingestion and not stored as-is. This decouples TC's internal
evolution from external convention churn.

#### 5.2.1 Externalization Taxonomy Mapping

Hu et al. (arXiv:2604.08224, "Externalization in LLM Agents: A Unified Review of Memory,
Skills, Protocols and Harness Engineering," 21 authors, SJTU/CMU/OPPO) define a
four-category taxonomy for what LLM agents externalize -- i.e., what they persist, share,
or delegate outside their immediate inference context. The taxonomy is: Memory (state
across time), Skills (procedural expertise), Protocols (interaction structure), and Harness
(governed execution). The survey's key insight is that traces flow from skill execution
into memory systems as an "evidence base for procedural guidance" -- meaning that well-
structured traces are not just observability artifacts but inputs to future agent behavior.

TC's canonical trace envelope fields map onto these four externalization dimensions as
follows:

| Externalization Dimension | TC Envelope Field(s) | Coverage |
|---|---|---|
| **Memory** (state across time) | Token usage per LLM call; redacted content references; provenance tier | Partial -- TC captures outcome-level memory (what happened) but not working-memory state (what the agent was considering) |
| **Skills** (procedural expertise) | *None* | **Gap** -- TC does not record which skill, strategy, or procedural template generated a given trace segment |
| **Protocols** (interaction structure) | Tool call sequence with timestamps; source format tag | Good -- tool call sequences capture the structural protocol of agent-tool interaction |
| **Harness** (governed execution) | Provider and model identity; provenance tier | Partial -- TC captures the execution environment identity but not harness-level governance metadata (safety constraints, execution policies, resource limits) |

The **Skill layer gap** is the most significant finding from this mapping. When a trace is
submitted to TC, there is no metadata indicating which skill or procedural strategy the
agent was executing. A trace segment showing a sequence of file-read and code-write tool
calls could have been generated by a "bug fix" skill, a "refactoring" skill, or an "add
feature" skill -- each with different quality expectations and different relevance for
trajectory RAG retrieval (doc 12). Adding a `skill.name` or `skill.type` attribute to TC's
canonical envelope would enable skill-filtered retrieval and more precise quality scoring.
This should be added to the schema extension roadmap alongside the `scaffold.phase`
attribute proposed in Section 4.8.

The **Harness layer gap** is secondary but relevant for compliance (doc 15): downstream
consumers may need to know what safety constraints governed the agent's execution, not just
what the agent did. TC's planned metadata extensions should include harness-level
governance attributes where the submitting framework provides them.

#### 5.2.2 Failure Annotation Standard: TRAIL Taxonomy

TRAIL (arXiv:2505.08638, "TRAIL: Trace Reasoning and Agentic Issue Localization") provides
a formal failure taxonomy for agent traces, validated on a dataset of 148 traces (118 from
GAIA, 30 from SWE-Bench) comprising 1,987 OTel spans, 575 error spans, and 841 annotated
errors. The dataset was collected using OTel and OpenInference instrumentation and stored
as structured JSON -- a wire format compatible with TC's OTLP ingest path.

TRAIL classifies agent failures into three top-level categories:

1. **Reasoning Errors**: The agent's LLM inference produced incorrect conclusions,
   hallucinated facts, or applied flawed logic. These errors originate in the Reasoning
   phase of the agent scaffold (Section 4.8) and are detectable through output validation
   or self-reflection.
2. **System Execution Errors**: Tool calls failed, APIs returned errors, file operations
   produced unexpected results. These errors originate in the Action phase and are
   detectable through return-code checking and output parsing.
3. **Planning/Coordination Errors**: The agent chose the wrong sequence of actions, failed
   to decompose a complex task, or miscoordinated with other agents in a multi-agent setup.
   These errors span the Reasoning and Reflection phases and are harder to detect
   automatically.

Each error annotation includes the error category, the evidence span(s) that exhibit the
error, and an impact level indicating whether the error caused task failure or was recovered
from.

TC should adopt TRAIL's taxonomy as its canonical failure classification schema. When a
submitted trace has a gate score of 0 (rejected by TC's quality gate), TC should annotate
the rejection with:

- **TRAIL error category** (Reasoning / System Execution / Planning-Coordination)
- **Evidence span references** (pointers to the span(s) within the trace that exhibit the
  error)
- **Impact level** (terminal failure vs. recovered error)

This annotation serves two purposes. First, it provides structured feedback to contributors
explaining why their trace was rejected, which is more actionable than a raw gate score.
Second, it enriches TC's corpus metadata for trajectory RAG (doc 12): a future agent
querying TC for relevant traces can filter by error category to find traces that demonstrate
recovery from specific failure types.

TC's current corpus (~352 submissions) is already larger than TRAIL's evaluation dataset
(148 traces), suggesting that TC has sufficient volume to validate the taxonomy's coverage
against real-world coding agent traces. Errors that do not fit the three TRAIL categories
should be logged as a fourth "Unclassified" category and reviewed periodically to determine
whether the taxonomy needs extension.

### 5.3 Export Layer: W3C PROV Lineage

For downstream provenance consumers -- compliance tools, audit systems, research pipelines --
TC exports traces as W3C PROV-DM entities. The PROV graph expresses:

- Each trace as a `prov:Entity` with identity anchored to the contributor's NEAR account
- Each LLM call as a `prov:Activity` with model and provider attributes
- Each tool invocation as a `prov:Activity` derived from the preceding LLM call
- `wasDerivedFrom` edges linking outputs to inputs across tool call boundaries
- `wasGeneratedBy` edges linking each artifact to the activity that produced it
- `wasAssociatedWith` edges linking activities to the agent identity

This PROV graph serves two use cases from other v6 documents:

**Compliance (doc 15)**: GPAI providers under EU AI Act Article 12 must maintain logs of
system behavior. TC's PROV export provides a machine-readable, standards-compliant lineage
record that compliance tooling can consume without a TC-specific integration.

**Downstream value tracking (doc 16)**: Usage-linked credit allocation requires tracing
which traces contributed to downstream value. PROV's `wasDerivedFrom` edges provide the
causal chain needed to attribute credit to original contributors when their traces are
retrieved and used (trajectory RAG, doc 12).

### 5.4 Identity and Protocol Integration: MCP -> ACP -> A2A -> ANP

TC's credit formula `q = f * g * a` attributes credits to a single NEAR account per
submission. Multi-agent workflows -- where one agent invokes another and the combined output
is submitted -- require cross-agent credit splitting. The four-protocol taxonomy from
arXiv:2505.02279 (Section 1.4) provides the layered integration path for TC's identity and
communication infrastructure:

**MCP (Phase 1 -- near-term)**: MCP standardizes how TC itself can be exposed as a tool to
agent frameworks. An agent using Claude Code's MCP integration can submit traces to TC as a
tool call, with TC's ingest endpoint exposed as an MCP server. This lowers the integration
friction to a single tool registration. MCP server exposure is a near-term task (days) once
the ingest endpoint is stable.

**ACP (Phase 2 -- structured async ingest)**: ACP's RESTful HTTP semantics, MIME-typed
multipart messages, and explicit session management provide a natural transport for
asynchronous trace submissions. A contributor's agent could open an ACP session with TC's
ingest endpoint, submit traces as multipart payloads (trace envelope + metadata +
attestation artifacts), and receive quality scores as response messages within the same
session context. ACP's DID integration also provides a verified identity primitive for
contributor authentication that complements NEAR account identity.

**A2A (Phase 3 -- multi-agent credit splitting)**: A2A (Agent-to-Agent protocol, v1.0.0,
150+ organizations, hosted by AAIF) provides the identity layer for cross-agent credit
attribution:

- A2A messages carry agent identity assertions
- When a trace is generated by a multi-agent workflow, A2A identity headers identify each
  contributing agent
- TC can split credits proportionally across contributing NEAR accounts

**ANP (Phase 3 -- decentralized contributor discovery)**: ANP's W3C DID and JSON-LD graph
foundation enables decentralized contributor onboarding. New contributors could register
their agent's capabilities and trace generation profiles as ANP-discoverable descriptions,
allowing TC to discover potential contributors programmatically rather than relying solely
on manual onboarding. ANP is the longest-horizon integration -- decentralized discovery
matters primarily when TC scales beyond its current small contributor base.

None of these protocol integrations require changes to TC's scoring pipeline -- they are
front-end integration concerns. The phased sequence (MCP -> ACP -> A2A -> ANP) follows the
survey's recommended adoption roadmap and aligns with TC's own maturity progression: tool
exposure first, then structured ingest, then multi-agent identity, then open discovery.

---

## 6. WASM-Sandboxed Scorer Plugins

Research finding C2 from the v6 deep research sweep establishes a pattern for composable,
attestable scoring that is directly relevant to TC's interop architecture. The scoring
pipeline is currently monolithic: one scorer binary, one attestation surface. As TC adds
new scoring dimensions -- structural embeddings (doc 17), trajectory RAG relevance (doc 12),
GPAI compliance scoring (doc 15) -- the monolithic design becomes a maintenance and
attestation problem.

The WASM-sandboxed plugin pattern resolves this:

**Each scorer plugin runs in a WASM sandbox**: The plugin cannot access raw trace data
outside its sandbox boundary. The sandbox enforces API contracts -- a plugin can only receive
the trace slice it is authorized to score.

**Each plugin is independently versioned**: Plugins are content-addressed artifacts. Deploying
a new version of one scorer does not require redeploying the full pipeline.

**Each plugin extends a TDX RTMR at load time**: When a plugin binary is loaded into the
scoring pipeline, its measurement (hash of the WASM binary) is extended into an RTMR
(Runtime Measurement Register) in the Intel TDX quote. The TDX quote produced at the end
of scoring enumerates exactly which plugin versions ran. External verifiers can inspect the
quote and confirm that the scoring was performed by specific, known plugin binaries.

**Plugins are hot-swappable**: A plugin can be updated and the new version loaded without
restarting the full pipeline. The next TDX quote reflects the new plugin measurement. This
is compatible with TC's continuous deployment model.

**Policy-as-code for gate thresholds**: The gate decision logic (accept/reject thresholds,
weights across scoring dimensions) is itself a plugin, independently versioned and attested.
Operators can update gate policy without touching the scorer implementations. An OPA/Rego-style
policy language provides a human-readable representation of gate logic that is also a versioned
artifact whose measurement extends into the TDX quote.

This architecture satisfies four constraints simultaneously:
- **Modularity**: New scorers can be added without redeploying the full pipeline
- **Attestation**: Per-plugin attestation via RTMR extension (doc 13, doc 08)
- **Safety**: WASM sandbox prevents plugin code from accessing raw trace data outside its scope
- **Versioning**: Each plugin version is independently measurable and auditable

The WASM scorer plugin framework is a Phase 3 item (2-4 months). It is not a prerequisite
for TC's core pipeline, but it is prerequisite for TC to scale its scoring surface beyond
the current monolithic design without losing attestation coherence.

---

## 7. ClickHouse/Langfuse Market Context

The LLM observability market context affects TC's competitive positioning. One correction
from prior research:

**ClickHouse acquired Langfuse on January 16, 2026** -- not Databricks, as earlier TC
research materials stated. The acquisition was accompanied by a $400M Series D that tripled
ClickHouse's valuation to approximately $15B. Langfuse was the leading open-source LLM
observability platform at the time of acquisition.

Market sizing (from The Business Research Company via MarkTechPost, August 2026): the LLM
observability market is valued at $1.97B in 2025 and projected at $9.26B by 2030 at 36.2%
CAGR. These figures are vendor estimates -- treat them as directional, not authoritative.

Implications for TC's interop positioning:

**Langfuse's open-source trajectory is uncertain**: Langfuse is now part of a commercial
database company. Its community edition maintenance pace and feature roadmap are no longer
governed solely by open-source incentives. This creates a gap for a genuinely independent
observability layer that TC can occupy.

**Existing tools are siloed**: Langfuse/ClickHouse, Braintrust, and LangSmith all operate
as closed data silos. None offer:
- Cross-user shared trace retrieval (trajectory RAG, doc 12)
- TEE-based scoring with verifiable attestation (doc 13)
- Contributor compensation for trace data
- W3C PROV lineage export for compliance consumers

TC's hub-not-silo architecture is a direct differentiator from this market. The interop
investment described in this document -- OTLP ingest, OpenInference support, PROV export,
A2A identity -- is what makes TC a hub rather than a smaller silo.

---

## 8. Implementation Roadmap

### Phase 1: Alias Shim, Hook Integration, and MCP Exposure (Weeks)

1. **OTel alias shim** (~50 LOC): `gen_ai.system` to `gen_ai.provider.name` normalization,
   per doc 18 Section 7. Must be implemented before any OTel ingest work.

2. **Per-SDK conformance test framework**: For each instrumented SDK (OpenAI Python, LangChain
   via OpenInference, Gemini CLI when format stabilizes), export a real span and validate that
   TC's normalization produces the expected canonical attributes. Do not trust the stability
   flag alone.

3. **Claude Code SessionEnd hook integration**: Per doc 01. The simplest ingest path -- no
   OTel required. Captures the session transcript on completion and submits to TC. Hours of
   engineering work.

4. **MCP server exposure**: TC ingest endpoint as an MCP server. Agent frameworks with MCP
   support can submit traces as tool calls without a dedicated TC integration. MCP is the
   first layer in the phased protocol roadmap (Section 5.4) and the lowest-friction
   integration path.

### Phase 2: OTLP Receiver, OpenInference Mapping, and ACP Ingest (1-2 Months)

5. **OTel OTLP receiver endpoint** (gRPC + HTTP): Accepts standard OTel spans from any
   instrumented agent framework. Routes to alias shim, then to canonical envelope
   serialization.

6. **OpenInference attribute mapping**: The normalization layer mapping `llm.*` /
   `embedding.*` / `retriever.*` to TC's internal schema (per doc 18 Section 8.1 tables).
   Convention detection logic (sentinel attribute check) routes traces to the correct
   normalizer.

7. **Bespoke parsers for Claude Code and Codex local formats**: Claude Code via SessionEnd
   hook (Phase 1 already covers this); Codex local JSONL as a fallback path alongside OTel
   emission.

8. **ACP session-based ingest**: Structured async trace submission via ACP's RESTful HTTP
   semantics. MIME-typed multipart payloads (trace envelope + metadata + attestation
   artifacts) with session context for multi-part submissions. ACP's DID integration
   provides contributor identity verification. Second layer in the phased protocol roadmap
   (Section 5.4).

9. **Schema extensions for externalization dimensions**: Add `skill.name`/`skill.type`
   and `scaffold.phase` attributes to TC's canonical envelope (per Sections 4.8 and
   5.2.1). Add `collaboration_mode` attribute for latent-space agent clusters (Section
   4.7). These attributes are custom to TC and not part of any existing OTel convention.

### Phase 3: PROV Export, WASM Plugins, A2A/ANP Identity, and Failure Annotation (2-4 Months)

10. **W3C PROV-DM/PROV-O lineage export**: Traces expressed as PROV entities, activities,
    and agents. `wasDerivedFrom` and `wasGeneratedBy` edges for downstream attribution.
    Enables compliance workflows (doc 15) and usage-linked credit (doc 16). Target
    typed-graph representation per evidence tracing survey (Section 1.3).

11. **WASM scorer plugin framework**: Independent plugin sandboxing with RTMR extension per
    load. Policy-as-code gate logic. Hot-swap support. Prerequisite for scaling the scoring
    surface without losing attestation coherence.

12. **A2A identity support**: Cross-agent credit attribution for multi-agent workflow traces.
    Credit splitting by contributing NEAR account. Builds on A2A v1.0.0 (AAIF hosted).
    Third layer in the phased protocol roadmap (Section 5.4).

13. **ANP contributor discovery**: Decentralized contributor onboarding via W3C DID and
    JSON-LD agent descriptions. Enables programmatic discovery of potential contributors.
    Fourth and final layer in the phased protocol roadmap (Section 5.4). Relevant
    primarily at scale beyond TC's current contributor base.

14. **TRAIL failure annotation pipeline**: Automatic classification of rejected traces
    (gate score = 0) using TRAIL's three-category taxonomy (Reasoning / System Execution /
    Planning-Coordination). Per-error evidence span references and impact levels stored in
    TC's canonical envelope. Requires the OTLP ingest path (item 5) to be operational for
    OTel-instrumented traces. See Section 5.2.2.

15. **Six-dimension provenance metadata**: Close the remaining gaps in the evidence tracing
    taxonomy (Section 1.3): per-evidence-unit source type tagging (dimension 1),
    evidence-unit role labeling per span (dimension 2), and finer per-step tracing
    granularity (dimension 4). These extend the canonical envelope schema and enrich the
    PROV export (item 10).

---

## 9. Verification Ledger

| # | Claim | Source | Status |
|---|---|---|---|
| 1 | All `gen_ai.*` semantic conventions are still at "Development" status, no Stable attributes | OTel GenAI SIG repo; John Hodge analysis, July 17 2026 (via doc 18) | Verified |
| 2 | GenAI conventions moved to `open-telemetry/semantic-conventions-genai` at main-repo v1.42.0 (June 12, 2026) | OTel release notes (via doc 18) | Verified |
| 3 | Dedicated `semantic-conventions-genai` repo has no tagged release and no finalized schema URL | OTel GenAI SIG repo state (via doc 18) | Verified |
| 4 | `gen_ai.system` renamed to `gen_ai.provider.name` at v1.39.0 | OTel registry (via doc 18) | Verified |
| 5 | `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` is honored inconsistently across SDKs | John Hodge, July 2026 (via doc 18) | Verified |
| 6 | Datadog native OTel GenAI support shipped December 1, 2025 (OTel SDK/Collector v1.37) | Datadog docs (via doc 18) | Verified |
| 7 | Arthur AI chose OpenInference over raw OTel GenAI for richer RAG primitives | Public blog post (via doc 18) | Verified |
| 8 | LangChain uses OpenInference by default via `openinference-instrumentation-langchain` | OpenInference repo (via doc 18) | Verified |
| 9 | W3C PROV-DM provides adequate expressivity for AI/ML pipeline provenance | arXiv:2606.04990 (Akhtar et al., PROV survey) | Verified |
| 10 | Agent-OSI L5 defines a provenance interface admitting TEE, ZK, and signed log attestations | arXiv:2602.13795 (Agent-OSI) | Verified |
| 11 | AAIF (Linux Foundation Agentic AI Foundation) established December 2025, hosting MCP and A2A | AAIF announcement (via doc 03) | Verified |
| 12 | A2A v1.0.0 has 150+ organizations | A2A repo (via doc 03) | Verified |
| 13 | ClickHouse acquired Langfuse on January 16, 2026 (not Databricks) | ClickHouse press release, January 2026 | Verified |
| 14 | ClickHouse $400M Series D accompanied the Langfuse acquisition, tripling valuation to ~$15B | ClickHouse press release, January 2026 | Verified |
| 15 | LLM observability market: $1.97B (2025) → $9.26B (2030) at 36.2% CAGR | The Business Research Company via MarkTechPost -- treat as vendor estimate | Unverified (vendor source) |
| 16 | TC has 3 PRs merged with IronClaw, 20K+ lines of integration code | TC repo metrics (doc 00) | Verified |
| 17 | W3C AI Agent Protocol Community Group is active, specs expected 2026-2027 | W3C CG page (via doc 03) | Verified |
| 18 | Cursor's local storage format is not publicly documented and has changed in past releases | Community reports; no primary source | Inferred, unverified |
| 19 | Gemini CLI OTel emission is likely given Google's OTel investment | Inferred from Google CNCF membership; no primary source from Gemini CLI docs | Inferred, unverified |
| 20 | GitHub Copilot telemetry is primarily cloud-side and not accessible without enterprise agreements | GitHub Copilot docs | Verified |
| 21 | IronClaw supports 26+ LLM providers, runs across CLI, Telegram, Slack, Discord, Signal | IronClaw repo (doc 00) | Verified |
| 22 | Issue #219: IronClaw contributors disproportionately affected by redaction quality penalty | TC GitHub Issues (doc 00, 08) | Verified |
| 23 | LatentMAS achieves up to 14.6% higher accuracy, 70.8-83.7% fewer output tokens, 4x-4.3x faster inference via shared KV-cache latent collaboration | arXiv:2511.20639 (Zou et al., ICML 2026 Spotlight) | Verified |
| 24 | Four-protocol taxonomy: MCP (JSON-RPC tool invocation), ACP (RESTful HTTP + DID), A2A (peer-to-peer task delegation, v1.0.0, 150+ orgs), ANP (W3C DID + JSON-LD discovery); phased adoption roadmap MCP -> ACP -> A2A -> ANP | arXiv:2505.02279 (agent interoperability protocols survey, May 2025, v2) | Verified |
| 25 | Four-category externalization taxonomy (Memory, Skills, Protocols, Harness); traces flow from skill execution into memory as "evidence base for procedural guidance" | arXiv:2604.08224 (Hu et al., 21 authors, SJTU/CMU/OPPO) | Verified |
| 26 | AgentSpec modular specification: Perception -> Memory -> Reasoning -> Reflection -> Action with typed interfaces; scaffold compatibility matters more than isolated module strength | arXiv:2606.14674 (UC San Diego/JHU/UW/UIUC) | Verified |
| 27 | Six-dimensional evidence tracing taxonomy: trace sources, evidence units, provenance relations, tracing granularity/timing, representation forms, trust functions; typed-graph as highest-fidelity provenance representation | arXiv:2606.04990 (Wang et al., "From Agent Traces to Trust") | Verified |
| 28 | TRAIL failure taxonomy: Reasoning Errors, System Execution Errors, Planning/Coordination Errors; dataset of 148 traces, 1,987 OTel spans, 575 error spans, 841 annotated errors collected via OTel/OpenInference | arXiv:2505.08638 (TRAIL) | Verified |
