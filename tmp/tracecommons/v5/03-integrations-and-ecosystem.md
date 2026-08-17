# Integrations & Ecosystem

**Date**: August 2026

TraceCommons needs to widen its ingest funnel from "IronClaw only" to "anything that produces agent traces." Five strategic integrations, ordered by leverage, plus the IronClaw integration status and deep research queries for finding more.

---

## 1. OTel-Native Ingest (Highest Leverage)

### Why This Is #1

Before OTel, every integration is bespoke — a Claude Code adapter, a Codex adapter, a Cursor adapter, each a maintenance burden. With OTel, the pattern is: source emits `gen_ai.*` spans (many already do), TC accepts them over OTLP, TC maps spans to envelopes via a version-pinned mapping layer.

OTel GenAI semantic conventions (v1.42.0, June 2026) are adopted by Langfuse, Datadog, Arize Phoenix, MLflow, and the standard instrumentation libraries. Any team already emitting these spans can pipe them to TC with a config change, not a code change. Integration cost drops from "learn our SDK and add trace collection calls" to "add an exporter endpoint to your OTel collector config."

### What to Build

**OTLP receiver (gRPC + HTTP/protobuf).** The `opentelemetry-proto` Rust crate provides generated protobuf types. `tonic` handles gRPC. HTTP/protobuf is a fallback for environments that can't do gRPC. The receiver is an Axum handler that deserializes `ExportTraceServiceRequest`, walks the span tree, and produces `TraceContributionEnvelope`s.

**Attribute mapping layer.** Version-pinned mapping from OTel GenAI attributes to TC envelope fields:

| OTel Attribute | TC Envelope Field |
|---|---|
| `gen_ai.system` | `provider` |
| `gen_ai.request.model` | `model` |
| `gen_ai.usage.input_tokens` | `input_token_count` |
| `gen_ai.usage.output_tokens` | `output_token_count` |
| `gen_ai.usage.total_tokens` | `total_token_count` |
| `gen_ai.request.temperature` | `temperature` |
| `gen_ai.prompt.*` | `messages` (conversation) |
| `gen_ai.completion.*` | `messages` (response) |
| Tool-call child spans | `ToolCallEvent` sequence |
| `server.address` / `server.port` | `endpoint` |
| W3C `traceparent` | `trace_id` (for stitching) |

**Span-to-envelope assembly.** Walk the span tree, identify agent root spans (spans with `gen_ai.system` attribute), group child spans into tool-call events, assemble into `TraceContributionEnvelope`. Multi-turn conversations produce multiple envelopes linked by `trace_id`.

**Redaction on ingest.** OTel spans carry raw content. The existing redaction pipeline runs identically — no special case for OTel-origin traces.

**Version pinning.** The conventions are pre-stable. Pin attribute strings behind a version constant so TC can update mappings as the spec evolves without breaking existing exporters.

**OpenInference support.** Arize/Phoenix uses a parallel span convention (`openinference.*`). Support both via a detection layer: if `gen_ai.*` attributes present → OTel path; if `openinference.*` → OpenInference path. Same envelope output.

### Coexistence With Custom Envelope

The custom `TraceContributionEnvelope` carries TC-specific fields without OTel equivalents: consent metadata, `redaction_hash`, device-key claims, quality gate inputs. Two paths coexist:

- OTel for trace data (spans, attributes, timing)
- TC metadata wrapper for consent, redaction, and claims

This means OTel-origin traces have slightly different metadata than IronClaw-origin traces. The gate pipeline operates identically on both.

### User Acquisition Impact

The target user journey:

```text
# Already using Langfuse? Add TC in 30 seconds:
$ tc auth
$ tc config set otel.endpoint http://localhost:4318   # your existing collector
# OR: add TC as an OTel exporter in your collector config:
#   exporters:
#     otlp/tracecommons:
#       endpoint: https://ingest.tracecommons.org:4317
```

Effort: 2-4 weeks.

---

## 2. Error Hub / Failure Commons

### What It Is

A searchable collection of scrubbed failure-diagnosis-repair bundles. When an agent fails, TC attributes root cause, bundles the failure context, scores for novelty, and publishes to commons. Developers search "my agent keeps failing on X" and get back structured bundles from others who hit the same issue.

### What to Build

**Failure-attribution gate extension.** New metadata on the envelope:

```rust
struct FailureAttribution {
    outcome: TraceOutcome,           // Success, Failure, Partial, Interrupted
    failure_type: Option<FailureType>,  // TestFailure, BuildError, RuntimeError, UserInterrupt, ContextExhausted
    root_cause_step: Option<u32>,    // Index of the step that caused the failure
    diagnosis_path: Vec<DiagnosisStep>, // How the failure was identified
    repair_diff: Option<String>,     // What was changed to fix it (if available)
    similar_failure_count: u32,      // How many similar failures exist in corpus
}
```

**Failure novelty scoring.** Per-tenant failure-mode frequency table. Score = rarity of failure type + rarity of tool/context combination + quality of diagnosis path. A "cargo build failed because of a missing feature flag" is common. A "cargo build failed because of a circular dependency introduced by the agent's own refactoring" is novel.

**Search interface.** CLI: `tc search-failures "cargo build failed"`. API: `POST /api/v1/failures/search`. Return ranked failure bundles with scrubbed context. Search by: failure type, tool involved, error message pattern, agent harness.

**Bundle display.** When a user views a failure bundle, they see: what happened, what the root cause was, what fixed it, how many others hit the same issue, and the novelty/diagnostic score. Not raw JSON.

### User Acquisition Impact

Failure commons is the top-of-funnel growth engine. Developer hits failure → searches TC → finds a fix → contributes own failures → enters the ecosystem. This is the same pattern that made Stack Overflow the default: people arrive when they're stuck.

Effort: 6-8 weeks for MVP.

---

## 3. Agent Skills Publishing

### What It Is

TC mines its corpus for recurring high-quality patterns and publishes them as SKILL.md files per the Agent Skills spec. ~40 compatible products: Claude Code, Codex, GitHub Copilot, Cursor, Gemini CLI, Windsurf, etc. Each skill credits TraceCommons and links to the corpus.

### What to Build

**Manual curation CLI (v1).** `tc skill publish --traces <id1>,<id2>,... --template <template>`. Takes trace IDs and a skill template, extracts the common procedure, formats as SKILL.md with YAML frontmatter. Output is markdown with metadata: source trace IDs, quality score, security scan result, provenance chain.

**Security scanner.** Before publication: injection detection (does the skill contain prompt injection patterns?), code execution analysis (does it instruct the agent to run untrusted code?), data exfiltration checks (does it instruct the agent to send data to external endpoints?). ToxicSkills found 36.82% of skills in the wild have security flaws — TC's scanner is the differentiator.

**Attribution tracker.** Map skills to contributing traces, flow credit back to trace contributors when skills are adopted. If Skill X was extracted from Traces A, B, C, and Skill X gets adopted by 100 developers, contributors of A/B/C earn ongoing credit.

**Automated extraction pipeline (v2).** RHO (arXiv 2606.05922): 19% absolute gain on SWE-Bench Pro via retrospective pass over unlabeled trajectories. Pipeline: cluster traces by task embedding → identify common sub-procedures via sub-trace decomposition → run retrospective extraction → format as SKILL.md → score through gate before publication. 12-16 weeks.

Effort: 1-2 weeks (manual v1), 12-16 weeks (automated v2).

---

## 4. Protocol-Level Integration

### MCP Tool-Call Events

MCP (Model Context Protocol) tool calls are the most common structured interaction in agent traces. First-class support means: MCP tool invocation spans are parsed and stored as typed events, tool parameters are captured (subject to redaction), tool results are captured (subject to redaction), and the tool-call graph (which tools called which) is preserved.

Ships alongside OTel ingest — MCP tool calls are already represented as child spans in OTel-instrumented agents.

Effort: 1-2 weeks.

### A2A Delegation Events

The A2A protocol (Agent-to-Agent, Linux Foundation, 50+ partners) defines cross-agent delegation. When Agent A delegates a subtask to Agent B, the delegation event is a new data type for TC — not a single-agent trace, but an inter-agent coordination record.

Model as parent-child span relationship (maps naturally to OTel). Parent span = delegating agent's decision, child span = delegated agent's execution. Linked by `trace_id` + parent `span_id`.

Build when multi-agent traces appear in meaningful volume. Effort: 4-6 weeks.

### W3C Trace Context

W3C trace context (`traceparent` header) enables cross-organizational trace stitching. If Organization A's agent delegates to Organization B's agent, and both propagate `traceparent`, TC can reconstruct the full trace.

Requires bilateral opt-in. Privacy implications: cross-org stitching reveals organizational relationships. Opt-in per trace, not per organization.

Effort: 2-3 weeks.

---

## 5. Trajectory Replay

### What It Is

Cross-harness replay interface: browse TC traces as navigable step-by-step timelines. Not flat JSON — interactive visualization showing tool calls, model responses, timing, scoring overlays, and branching points.

### What to Build

**Terminal-based viewer (v1).** `tc replay <trace-id>` — step through events, show tool calls with timing and token counts, highlight failure points. TUI using ratatui or similar. Works offline on local traces.

**SSE replay stream.** Server-side: send trace events over SSE at configurable speed. Client-side: any web UI can consume. Enables embedding replays in docs, blog posts, etc.

**Web-based viewer (v2).** Browser UI: timeline with expandable events, cost accumulation graph, scoring overlays, side-by-side comparison. Natural home for failure attribution results.

**Anonymization layer.** For network replay (sharing replays with others): heavy redaction by default, configurable per-field. Strip file paths, variable names, API keys. Leave tool-call structure, timing, scoring.

AgentGUI (ETH Zurich) validated that visual replay with branch-point steering helps users identify key trace elements 38% faster.

Effort: 8-10 weeks total (2-3 weeks for terminal viewer, 6-8 for web).

---

## 6. IronClaw Integration Status

### Shipped (Working End-to-End)

```
agent turn → capture → policy check → envelope → redact →
credential resolve → JWT → ingest → gate → credit
```

| PR | Summary | When |
|---|---|---|
| IronClaw #4559 | Agent-driven onboarding, 6 `builtin.trace_commons.*` capabilities, Ed25519 auth, standing consent, deterministic redaction | Jun 2026 |
| IronClaw #5280 | Instance enrollment, community profiles, trace inspection, credit tracking | Jun 2026 |
| IronClaw #5858 | Instance enrollment CLI (`traces enroll-instance`), hosted-user login links | Jul 2026 |
| TC server #152 | Per-user device-key subjects, namespaced `principal_ref` | Jun 2026 |

6 modules in `ironclaw_trace_commons` crate (capture, client, contribution, conversation_message, onboarding, redaction) consumed by 5 other IronClaw workspace crates.

### Critical Fixes (Trust Path — Before New Features)

**No TLS enforcement on credential HTTP clients.** The HTTP client builders in `ironclaw_trace_commons::client` do not validate that the configured endpoint uses HTTPS. If an operator sets `trace_commons_url = "http://..."`, bearer tokens and signed JWTs are sent over plaintext. In a coffee-shop or corporate-proxy scenario, the device key's JWT is exposed in transit; an attacker with the JWT can replay uploads or correlate contributor identity. **Fix:** Scheme check at client construction time. Accept `http://` only when an explicit `allow_insecure = true` flag is set (useful for local development against localhost). ~10 lines. (IronClaw #7144)

**Quarantine check keyed on prose substring.** The privacy gate checks quarantine status by looking for the substring `"quarantined"` in a server-returned status string. This is stringly-typed against what should be a typed enum. If TC changes the wording (e.g., `"quarantined_for_review"` to `"held_for_review"`), the check silently stops matching and quarantined traces are treated as accepted. This is a privacy boundary. **Fix:** Define a `TraceStatus` enum on the wire protocol, or at minimum match exhaustively and treat unknown values as quarantined-by-default. (IronClaw #7144)

**Empty-bytes `redaction_hash` on serialization failure.** If serialization fails, the error is swallowed and the hash is computed over an empty byte slice (`sha256(b"")` = `e3b0c44...`). Every serialization failure produces the same hash; TC cannot distinguish them. **Fix:** Propagate the serialization error. Reject the trace rather than submit with a meaningless hash. (IronClaw #7144)

**No behavioral tests for `ContributionHttpSink`.** HTTP layer fully mocked. No test has ever made a real HTTP request to a TC-compatible endpoint. The three defects above were in this path — the lack of integration tests is a compounding factor. **Fix:** `wiremock`-based test exercising full `ContributionHttpSink::submit()` path. (IronClaw #4940)

### High-Impact IronClaw Opportunities

**Immediate scoring feedback.** TC's ingest endpoint returning quality score + percentile + credits in the submission response (currently returns 202 with minimal body). IronClaw surfaces inline:

```text
Trace contributed to TraceCommons
Quality score: 92/100 (top 15% this week)
Credits earned: 3.2 TC
```

Three changes: (1) TC ingest returns score in response, (2) IronClaw `ContributionHttpSink` parses and surfaces, (3) display path in CLI and WebUI. The "top 15% this week" context turns an abstract number into a relative position.

**WASM fuel as quality signal.** IronClaw sandboxes tool execution in WASM with fuel metering. Fuel consumption is a direct, manipulation-resistant measure of computational work. An agent that accomplishes a task with fewer fuel units per tool call is producing a more efficient trace, and you cannot inflate fuel consumption without actually running more WASM instructions. Include per-tool-call fuel in the TC envelope, track separately at first, correlate with quality scores before incorporating into scoring.

**Cross-provider comparison analytics.** IronClaw supports 26 providers. TC has standardized quality scoring. The combination produces neutral, empirical provider comparison data: same quality metrics applied across dozens of providers on real-world agent tasks. Start with private contributor-only insights (not public rankings).

**Onboarding-time opt-in.** Surface TC contribution during IronClaw agent setup flow. Lowest-effort, highest-impact change for IronClaw users.

**Channel-specific interaction patterns.** IronClaw runs the same agent across CLI, Telegram, Slack, Discord, Signal. Channel tag in session metadata but not in TC envelope. Adding it (one field) enables "do CLI users produce higher-quality traces than chat users?" analytics.

### Open Questions

1. **OTel vs. custom envelope: coexistence or migration?** If TC adds OTLP ingest, does the custom envelope stay as a parallel path? Likely: OTel for trace data, custom wrapper for TC-specific metadata. But the details determine how much IronClaw integration code changes.

2. **TC #219: redaction penalizes quality scores.** IronClaw's redaction is thorough. If TC's gate penalizes redacted traces, IronClaw contributors are systematically disadvantaged vs. less privacy-conscious integrations. Most urgent TC-side fix for the integration.

3. **Credit settlement economics.** NEAR settlement is batched above a dust threshold. At pilot volume, most contributors may never hit the threshold. Pre-settlement balance display or lower pilot-phase threshold.

4. **Third contributor signal.** PR #250 from brapse is TC's first contribution outside the core team. Acquisition channel unknown (IronClaw? NEAR? Academic?). The third contributor's experience sets the template for every contributor after them.

---

## 7. Deep Research Queries: Integrations & Ecosystem

### Q-I1: One-Click AI Agent Integrations

```
"one click" OR "zero config" OR "auto-instrument" AI agent integration observability 2025 2026
```
**Looking for:** Which AI agent frameworks support auto-instrumentation (like Java agent auto-instrumentation in APM)? Can TC achieve zero-config trace collection for specific harnesses? What did Sentry, Datadog, or New Relic do to get "install our SDK and traces appear automatically"? Is there an equivalent for AI agent frameworks?

### Q-I2: OTel GenAI Adoption State

```
"OpenTelemetry" "gen_ai" semantic conventions adoption integrations 2026
```
**Looking for:** Who has shipped OTel GenAI convention support? Which agent frameworks, observability platforms, and SDKs emit `gen_ai.*` spans? How many developers are already emitting these spans? This determines how large the "add TC as an exporter" addressable market is.

### Q-I3: Agent Framework Plugin Architectures

```
"Claude Code" OR "Cursor" OR "Codex" OR "Copilot" plugin OR extension OR hook architecture 2025 2026
```
**Looking for:** Which major AI agent harnesses have extensible architectures (hooks, plugins, extensions, post-session callbacks)? Can TC integrate as a plugin/hook rather than requiring a separate install? Claude Code hooks, Cursor extensions, VS Code extension APIs — what's the easiest path to "install once, traces flow automatically"?

### Q-I4: Cross-Agent Trace Stitching

```
"multi-agent" trace stitching OR correlation OR "distributed tracing" A2A MCP 2025 2026
```
**Looking for:** How are multi-agent systems being traced today? When Agent A delegates to Agent B, how is the trace connected? Are there standards beyond W3C trace context? How does A2A handle observability? This determines whether TC needs its own stitching mechanism or can rely on existing standards.

### Q-I5: Agent Session Storage & Backup

```
"AI session" OR "agent session" backup OR storage OR history search 2025 2026
```
**Looking for:** Are there existing tools for backing up and searching AI agent sessions? If not, this validates TC's session backup hook as novel. If yes, how do they work and can TC integrate with them rather than building from scratch?

### Q-I6: LangSmith / LangChain Trace Format

```
LangSmith trace format OR schema export OR migration 2025 2026
```
**Looking for:** LangSmith has massive market share in the LangChain ecosystem. Can TC ingest LangSmith traces? Is there an export/migration path? Understanding the LangSmith trace format determines whether a LangSmith adapter is a high-leverage integration.

### Q-I7: Emerging Agent Harnesses

```
"AI coding agent" OR "AI agent" new OR emerging framework harness 2026
```
**Looking for:** What new AI agent harnesses have launched or are launching in 2026? Beyond Claude Code, Codex, Cursor, Copilot — what's coming? Each new harness is a potential integration partner. Which ones have open trace formats or OTel support?

### Q-I8: IDE Extension Marketplaces for AI Tools

```
"VS Code extension" OR "JetBrains plugin" AI agent observability analytics 2025 2026
```
**Looking for:** Is there an emerging category of IDE extensions focused on AI agent analytics? Could TC ship as a VS Code extension or JetBrains plugin that shows session scores, cost tracking, and replay inline? What's the competitive landscape for "AI coding assistant analytics in the IDE"?

### Q-I9: CI/CD Integration for Agent Quality

```
"CI/CD" OR "GitHub Actions" AI agent quality OR testing OR validation 2025 2026
```
**Looking for:** Are teams integrating AI agent quality checks into CI/CD? Could TC serve as a quality gate in CI — "this PR was generated by an agent whose session scored below threshold, flag for extra review"? Is there demand for this?

### Q-I10: Mobile and Web Agent Trace Collection

```
"mobile agent" OR "web agent" trace collection observability 2025 2026
```
**Looking for:** Agent frameworks increasingly operate in browsers and mobile apps (not just CLI). How are traces collected from these environments? Can TC's ingest accept traces from browser-based agents? What are the privacy implications of collecting traces from user-facing (not developer-facing) agents?
