# Integration Opportunities

TraceCommons (TC) is an open-source Rust AI trace registry with
privacy-preserving scoring inside TEEs. Current integration surface: IronClaw
(substantially done, 3 merged PRs, ~20K lines), a contributor CLI with
TraceSource implementations for ClaudeCode/Codex/Trajectory, and NEAR for
identity + credit settlement.

That's a narrow ingest funnel for a project whose value scales with corpus
breadth. Everything below is about widening that funnel -- five specific
things to build and why each one brings new contributors into the commons.

---

## 1. OTel-Native Ingest

**The single highest-leverage integration.**

### What

An OTLP receiver endpoint (gRPC + HTTP/protobuf) that accepts OpenTelemetry
GenAI and OpenInference spans, maps them into `TraceContributionEnvelope`,
and feeds them through the existing gate pipeline. A version-pinned attribute
mapping layer translates `gen_ai.request.model`, `gen_ai.usage.*`,
`gen_ai.agent.name`, tool-call spans, and retrieval spans into TC's internal
schema, handling both GenAI and OpenInference namespaces.

### Why It Gets Users

Every observability platform already speaks OTel. Langfuse, Datadog, MLflow
Tracing, and Arize Phoenix all export OTLP. If TC can receive OTLP, anyone
who has already instrumented their agent stack can contribute traces without
writing a single line of TC-specific code -- just an endpoint URL and an auth
token.

This flips the integration model from "build a TraceSource per harness" to
"point your existing exporter at TC." A team already sending Langfuse traces
to Datadog can add TC as a second OTLP destination and start contributing in
an afternoon. No new SDK. No new CLI. The contributor CLI still exists for
uninstrumented harnesses (raw Claude Code sessions, local Codex runs), but
OTLP becomes the primary ingest channel for production deployments.

### What to Build

**OTLP receiver.** A gRPC + HTTP/protobuf endpoint accepting
`ExportTraceServiceRequest`. Rust has `opentelemetry-proto` for protobuf
types and `tonic` for gRPC -- both mature. Authenticate via the existing
device-key or instance-enrollment flow (API key in metadata headers, mapped
to a TC principal).

**Attribute mapping layer.** This is where the real work is. Map OTel GenAI
conventions and OpenInference variants into TC's envelope schema:

```
gen_ai.request.model          -> envelope.model
gen_ai.system                 -> envelope.provider
gen_ai.usage.input_tokens     -> envelope.token_counts.input
gen_ai.usage.output_tokens    -> envelope.token_counts.output
gen_ai.agent.name             -> envelope.agent_name
gen_ai.tool.name              -> tool_call_event.tool_name
gen_ai.tool.call.id           -> tool_call_event.call_id
```

OpenInference uses a different namespace (`message.tool_call_results`,
`tool_call_result.*`); the mapping layer handles both. Version-pin the
mapping -- the OTel GenAI conventions (v1.42.0, still experimental as of
mid-2026) are pre-stable and attribute names can change. Declare which
convention version is targeted (e.g., `gen_ai.semconv = v0.12.0`) and fail
explicitly on unrecognized attributes rather than silently dropping them.
When conventions change, ship a new mapping version and support both during
a transition period.

**Span-to-envelope assembly.** An OTel trace is a tree of spans; a TC
envelope is a structured document with turns, tool calls, and outcomes. The
assembler walks the span tree, identifies the root agent span, collects child
tool-call and LLM-call spans, and constructs the envelope. Non-trivial for
multi-agent traces where the span tree has multiple agent roots -- the
assembler needs heuristics to distinguish true agent roots from intermediate
orchestration spans. Start with one envelope per agent to match TC's
per-session contribution model.

**Redaction on ingest.** OTel spans carry raw content
(`gen_ai.content.prompt` includes the full user message). Run TC's existing
redaction pipeline (PII scrubbing, secret detection, content filtering) on
OTLP-sourced envelopes identically to IronClaw-sourced ones -- the existing
stages operate on text content regardless of source format. Add rate limiting
and pre-screening at the receiver level to avoid wasting scoring compute on
bulk low-quality automated dumps.

**Who benefits immediately:** Langfuse users (already OTel-native, just add
a second exporter), Datadog AI Observability users (OTLP export as side
channel), Phoenix/Arize users (OpenInference maps directly), MLflow Tracing
users (OTLP export path exists), and any custom harness with OTel
instrumentation.

---

## 2. Agent Skills as Output Channel

### What

TC mines its trace corpus for recurring, high-quality procedural patterns,
extracts them as standalone skill descriptions, and publishes them in the
SKILL.md format -- the open standard (stewarded by the Linux Foundation's
Agentic AI Foundation) that ~40 products can already consume, including
Claude Code, Codex, GitHub Copilot, Cursor, and Gemini CLI. Each published
skill carries attribution metadata linking back to the contributing traces.

### Why It Gets Users

Skills are a distribution channel TC doesn't currently have. Right now, the
only reason to visit TC is if you already know what a trace commons is.
Skills circulate in ecosystems TC can't otherwise reach -- Copilot's
marketplace, Cursor's rules directory, Claude Code's project config. Every
skill published from TC is a backlink into the commons.

Contributors whose traces informed a skill earn credit when it's adopted,
creating a viral loop: contribute traces -> skills get extracted -> skills
circulate -> new developers discover TC -> they contribute their own traces.

There's also a quality angle. Snyk's ToxicSkills research found 36.82% of
skills in the wild have at least one security flaw, with 76 confirmed
malicious payloads. TC is positioned to offer skill quality scoring because
it already has provenance tracking, adversarial review, and multi-round
validation. A TC-published skill with a quality score and provenance chain
is meaningfully more trustworthy than an anonymous SKILL.md from a public
registry.

### What to Build

**Skill extraction pipeline.** Runs in the offline consolidation worker on
clusters of similar traces (not individual sessions). The pipeline:

1. Identifies recurring tool-use and reasoning patterns across traces that
   share a task category and achieved positive outcomes
2. Abstracts them into model-agnostic descriptions (the skill)
3. Validates that a fresh agent can reproduce similar outcomes on held-out
   tasks
4. Scores for quality and security (injection vectors, arbitrary code
   execution, data exfiltration)

The extraction can use TC's existing LLM scoring infrastructure or
lighter-weight pattern mining (process mining over tool-call sequences,
followed by LLM summarization of discovered patterns).

**SKILL.md formatter.** Format extracted skills per the agentskills.io spec:
name, description, trigger conditions, full instructions with progressive
disclosure (only name/description loaded until triggered). The Agent Skills
standard is young -- ~40 compatible products today, but the spec could
evolve. TC should version its published skills and be prepared to regenerate
them as the standard matures.

**Security scanner.** Given the ToxicSkills findings, every skill published
from TC goes through automated security review: prompt injection detection,
code execution analysis, data exfiltration checks. Automated (static
analysis + LLM red-teaming) with human review for edge cases.

**Attribution tracker.** Map each published skill to the set of contributing
traces. When a skill earns credit (downloads, adoption signals,
endorsements), flow credit back to contributing traces and their submitters
via TC's existing credit settlement system. Weight by influence so traces
that contributed more to the pattern earn proportionally more credit -- this
avoids the dilution problem where extracting from 500 traces gives each
contributor a meaningless 1/500th share.

---

## 3. Error Hub / Failure Commons

### What

A searchable collection of scrubbed failure-diagnosis-repair bundles
extracted from TC's trace corpus. When an agent fails, the failure trace goes
through standard ingest plus a failure-attribution stage that identifies root
cause, diagnosis process, and (if the trace includes a retry) the repair that
worked. Bundles are searchable by failure type, tool involved, error message
pattern, and task category.

### Why It Gets Users

Developers gather where debugging happens -- Stack Overflow's entire growth
model was error-message Google hits. Agent failures are currently opaque: you
get a session transcript and no way to know if your failure is common (with a
known workaround) or genuinely novel.

An Error Hub that answers "has anyone seen this before?" is a community
magnet. Once a developer is using it to debug, they're one click from
contributing their own traces -- especially if the system says "we don't have
many traces of this failure type yet, your contribution would be especially
valuable."

TC already has the privacy infrastructure (redaction, consent, TEE scoring)
that makes developers comfortable sharing failure data. Failure traces are
sensitive -- they often contain the exact code and prompts that triggered the
failure -- but TC's redaction pipeline handles this the same way it handles
any other trace content. The consent model may need a separate opt-in for
failure-trace contribution, distinct from general trace contribution, given
the heightened sensitivity.

### What to Build

**Failure-attribution stage.** A new gate (or gate extension) on traces with
negative outcomes that identifies:

- Failure type (compilation error, test failure, incorrect output, timeout,
  tool misuse, hallucination)
- Root-cause span (which step caused the failure)
- Diagnosis path (if the trace includes debugging steps, the sequence)
- Repair diff (if the trace includes a successful retry, what changed)

The AgentDebugX Detect-Attribute-Recover-Rerun framework is a reasonable
model -- their DeepDebug core repairs 13 of 73 failed GAIA tasks in a single
rerun, suggesting the attribution is meaningful enough to act on.

**Bundle schema.** Extend `TraceContributionEnvelope` with failure metadata:

```
failure_type: enum (compilation, test, output, timeout, tool_misuse, ...)
root_cause_span_id: string
diagnosis_steps: [span_id]
repair_diff: Option<string>  // what changed between failure and success
related_bundles: [bundle_id]  // similar failures in the corpus
```

**Search interface.** Developers need to find relevant failure bundles fast.
Search by error message pattern, failure type, tool involved,
language/framework, task category. Start with a CLI command (`tc-contributor
search-failures "cargo build failed"`) and an API endpoint that agent
harnesses call automatically on failure. A web UI can follow.

**Novelty scoring extension.** TC's existing novelty scoring gains a failure
dimension: a failure trace documenting a previously unseen failure mode is
more valuable than the 50th "agent tried to import a nonexistent module."
This creates targeted contribution incentives -- the system can tell
contributors which failure types the corpus needs more of.

---

## 4. Cross-Harness Trajectory Replay

### What

A replay interface that renders TC traces as navigable, step-by-step
trajectories regardless of which harness generated them. Two modes:

- **Single-player replay**: view your own submitted traces as step-by-step
  trajectories. See what your agent did, when it used tools, where it spent
  tokens, where it went wrong.
- **Network replay**: view anonymized traces from other contributors.
  Compare tool-use patterns, prompting strategies, and success rates across
  different agent configurations.

### Why It Gets Users

Single-player replay is the onboarding hook -- submit a few sessions via the
contributor CLI and immediately get a replay viewer for your own traces.
That's instant value before you care about the commons. Network replay is
what keeps you coming back: see how experienced developers' agents handle the
same class of task and learn from the trajectory.

AgentGUI (ETH Zurich, 2026) demonstrated that a unified trajectory viewer
across harnesses helps users identify key trace elements 38% faster, and
that automated drift-prevention raises task completion rates by up to 34
percentage points for small models. TC is uniquely positioned because it
already holds the cross-harness corpus (Claude Code, Codex, IronClaw, Cursor)
in a unified schema. No other system collects traces from all of these in a
single store with a common format.

### What to Build

**Trace-to-trajectory normalizer.** Convert any TC envelope into a common
trajectory format:

```
Trajectory {
  steps: [{
    actor: Agent | User | Tool | System,
    action: String,
    content: Option<String>,  // redacted
    tool_calls: Vec<ToolCall>,
    timing: StepTiming,       // wall clock, token counts
    outcome: Option<Outcome>,
    annotations: Vec<Annotation>,  // quality scores, flags
  }]
}
```

Each TraceSource implementation already parses its respective format. The
normalizer is a second pass producing the unified representation.

**SSE replay stream.** TC already has SSE infrastructure. The replay stream
emits trajectory steps at configurable speed (real-time, 2x, 5x,
step-by-step). Start with a terminal-based viewer (`tc-contributor replay
<trace-id>`) -- simpler than a full interactive web UI and validates the
format. Add a web UI later. Add a comparison mode for side-by-side replay of
two traces on similar tasks once task categorization is good enough to find
comparable traces.

**Anonymization layer.** Network replay needs stronger anonymization than
standard redaction. Beyond PII and secrets, strip identifiable coding
patterns, project-specific identifiers, and content that could de-anonymize
the contributor. Start conservative (heavy redaction, metadata-only replay
for sensitive content) and relax as privacy techniques improve. Contributors
should be able to preview what their replayed trajectory looks like before
opting in to network replay.

Agent sessions involve code diffs, terminal output, file creation, and tool
interactions. Rendering all of that -- even in a text-based viewer -- is
more work than it sounds. Starting with a metadata-and-summary view (what
happened at each step, without full content) is more tractable than trying to
render everything from day one.

---

## 5. Protocol-Level Integrations

### What

Capture MCP tool-call spans, A2A delegation events, and W3C trace context as
first-class event types in TC's trace schema. This isn't a new product
surface -- it makes the ingest pipeline aware of the protocol-level events
that structure modern agent interactions, so traces from protocol-aware
harnesses arrive with richer metadata.

### Why It Gets Users

Teams that have adopted MCP and A2A have the most sophisticated agent
deployments and produce the most interesting traces. If TC can ingest their
protocol-level events without flattening them into opaque text, those teams
get more value from the corpus and are more likely to contribute. Richer
protocol metadata also makes every user-facing feature (Error Hub, replay,
skill extraction) more useful.

W3C trace context enables a new class of contribution: cross-organizational
traces. If two companies collaborate via A2A-connected agents and both opt
in to TC, the linked trace captures the full workflow in a way neither
company's internal observability can.

### What to Build

**MCP tool-call events.** Promote MCP `tools/call` JSON-RPC exchanges to a
schema-level `ToolCallEvent` type (tool name, server identity, redacted I/O,
timing, errors) instead of embedding them in conversation turns. IronClaw
already captures tool calls at the agent event stream level -- this promotes
them to a schema-level concept. Enables tool-use quality scoring independent
of conversation quality, tool-use profiles (which tools get used for which
tasks, which have high failure rates), and tool-specific search in the Error
Hub. Ship alongside OTel ingest since MCP tool calls are a natural part of
the span schema.

**A2A delegation events.** Add a `DelegationEvent` type capturing delegator
identity (anonymized), delegatee capability card, redacted task description,
result, and chain depth. For harnesses using A2A natively, map directly from
the protocol; for others (IronClaw's internal delegation, roko's plan
execution), provide lightweight adapters. Multi-agent workflows are the
fastest-growing segment of agent usage but the least well-represented in
existing trace corpora -- a TC corpus rich in delegation events becomes the
canonical source for understanding how multi-agent systems actually behave.
Build when multi-agent traces appear in the corpus in meaningful volume.

**W3C trace context propagation.** Store `traceparent` alongside the TC
envelope ID and build a query path that retrieves all envelopes sharing a
trace ID, enabling cross-agent trace stitching across contributors. The
privacy constraint: linking traces from different organizations reveals
inter-organizational workflow details and needs bilateral opt-in, not just
individual contributor consent.

**Versioning strategy.** MCP, A2A, ACP, and ANP are all young. Treat
protocol-specific event types as versioned extensions to the core schema (not
core fields) so protocol changes don't require schema migrations. Start with
MCP tool calls only (most common, best-standardized), add A2A delegation
when multi-agent traces appear, defer ACP/ANP until there's demonstrated
demand.

---

## Prioritization

**OTel ingest** first -- it's the fastest path to more contributors because
it eliminates the per-harness integration tax. Every other integration
benefits from the larger, more diverse corpus OTel brings.

**Error Hub** vs **Skills** is a toss-up. Error Hub is the stronger
community magnet (developers actively seek debugging tools). Skills is the
stronger distribution channel (skills circulate in ecosystems TC can't
currently reach). Both are ideal but not in the same quarter.

**Trajectory replay** matters for retention but is less urgent than widening
ingest. Build it once the corpus is large enough that replay is interesting.

**Protocol integrations** are background work. MCP tool-call events ship
with OTel ingest; A2A and W3C context can wait for demand.

```
Q1:  OTel ingest + MCP tool-call events
Q2:  Error Hub (failure attribution + search)
Q3:  Skill extraction + SKILL.md publishing
Q4:  Trajectory replay + A2A delegation events
```

Each quarter's work makes the next quarter's feature more valuable: OTel
brings volume, Error Hub makes it useful, Skills distribute it, Replay turns
it into a learning tool.
