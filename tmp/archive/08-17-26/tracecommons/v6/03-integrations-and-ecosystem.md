# Integrations & Ecosystem

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, MIT/Apache-2.0).
Contributors submit scrubbed traces of AI coding agent sessions. TC scores them for
quality and novelty inside TEEs (Trusted Execution Environments -- hardware-isolated
encrypted compute), and contributors earn NEAR blockchain credits. The project has ~352
submissions from 3 contributors. Its primary integration partner is IronClaw, NEAR AI's
open-source agent runtime (12.6K GitHub stars, 26+ LLM providers, runs across CLI,
Telegram, Slack, Discord, Signal) -- 3 PRs merged, 20K+ lines.

TC's value scales with corpus breadth. The current ingest funnel is narrow: IronClaw,
a contributor CLI with TraceSource implementations for Claude Code/Codex/Trajectory,
and NEAR for identity and credit settlement. Everything below widens that funnel.

---

## 1. OTel-Native Ingest (Highest Leverage)

OpenTelemetry (OTel) is the open standard for telemetry data (traces, metrics, logs).
The `gen_ai.*` semantic conventions define how AI/LLM operations are traced.

### Stability Status (Confirmed August 2026)

All `gen_ai.*` conventions remain at **"Development" status** -- not "Stable." The
conventions moved to a dedicated repository on June 12, 2026 -- a signal of investment
but also instability. v1.42.0 is the last versioned reference. The most dangerous
breaking change in progress is the `gen_ai.system` to `gen_ai.provider.name` rename.
The schema URL field is still a TODO placeholder. OpenInference (Arize/Phoenix) is a
parallel namespace, not a competing standard -- both need support.

**TC must not claim "OTel-native."** Say "supports OTel GenAI draft conventions
(pinned to [date])" and plan for breakage.

### What to Build

**OTLP receiver (gRPC + HTTP/protobuf).** `opentelemetry-proto` + `tonic`. Axum
handler deserializes `ExportTraceServiceRequest`, walks span tree, produces
`TraceContributionEnvelope`s. Authenticate via existing device-key or
instance-enrollment flow (API key in metadata headers, mapped to a TC principal).

**Attribute mapping layer (version-pinned):**

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

Pin the mapping behind a version constant. Fail explicitly on unrecognized attributes
rather than silently dropping them. When conventions change, ship a new mapping version
and support both during a transition period.

**OpenInference support**: Detect `openinference.*` attributes via a parallel mapping
path producing the same envelope output.

**Span-to-envelope assembly.** An OTel trace is a tree of spans; a TC envelope is a
structured document with turns, tool calls, and outcomes. The assembler walks the span
tree, identifies the root agent span, collects child tool-call and LLM-call spans, and
constructs the envelope. This is non-trivial for multi-agent traces where the span tree
has multiple agent roots -- the assembler needs heuristics to distinguish true agent
roots from intermediate orchestration spans (framework middleware, routing layers,
retry wrappers all create spans that look like agent roots but are not). Start with
one envelope per agent to match TC's per-session contribution model. Revisit when
multi-agent traces appear in volume.

**Redaction on ingest.** OTel spans carry raw content. Run TC's existing gate pipeline
(redaction, chunking, embedding, similarity, perplexity scoring, gate evaluation) on
OTLP-sourced envelopes identically to IronClaw-sourced ones. Add rate limiting at the
receiver level to avoid wasting compute on bulk low-quality automated dumps.

### User Acquisition Impact

```text
# Already using OTel? Two commands:
$ tc auth
$ tc config set otel.endpoint http://localhost:4318
```

A team already sending Langfuse traces to Datadog adds TC as a second OTLP destination.
No new SDK. No new CLI. **Effort**: 2-4 weeks.

### Trace Schema Gap Analysis

Two recent papers expose gaps in what TC's OTel mapping captures:

**Externalization Review** (arXiv:2604.08224, confirmed): Proposes a four-category taxonomy
of what agents externalize -- Memory (persistent state), Skills (reusable procedures),
Protocols (tool calls and API interactions), and Harness (scaffold/framework behavior). TC
currently captures Protocol metadata (tool calls via `ToolCallEvent`) and Memory outcomes
(trace results), but lacks a Skill layer (what reusable procedures the agent applied or
learned) and Harness metadata (which scaffold phases were active, what framework-level
decisions were made). Adding skill and harness dimensions to TC's envelope schema would
make traces more useful for downstream skill extraction and cross-framework comparison.

**AgentSpec** (arXiv:2606.14674, confirmed): Shows that scaffold architecture determines
trace structure -- the same logical task produces different trace shapes depending on
whether the scaffold uses ReAct, Plan-and-Execute, or other patterns. TC should capture
scaffold phase metadata in its OTel attribute mapping (e.g., `tc.scaffold.phase`,
`tc.scaffold.pattern`) so that trace scoring can normalize for architectural differences
rather than penalizing traces for being structurally different from the majority pattern.

---

## 2. Error Hub / Failure Commons

Searchable collection of scrubbed failure-diagnosis-repair bundles extracted from TC's
trace corpus. When an agent fails, the failure trace goes through standard ingest plus a
failure-attribution stage that identifies root cause, diagnosis process, and (if the
trace includes a retry) the repair that worked.

Developers gather where debugging happens -- Stack Overflow's entire growth model was
error-message Google hits. An Error Hub that answers "has anyone seen this before?" is a
community magnet. Once a developer is using it to debug, they are one click from
contributing their own traces.

### Validating Research

**TraceLab** (Zhu et al., University of Washington, arXiv:2606.30560): 4,265 coding agent
sessions, 357K LLM steps -- largest public agent trace corpus. A workload characterization
study showing that coding agent sessions involve complex, multi-step LLM interactions at
scale. The breadth and complexity of real-world coding agent workloads validates the Error
Hub as top-of-funnel: developers need a place to share and search failure traces precisely
because these workloads are long, tool-heavy, and hard to debug in isolation.

**AgentDebugX** (arXiv:2607.18754, confirmed): Opt-in Error Hub for sharing scrubbed
failure-diagnosis-repair bundles. DeepDebug achieves 28.8% exact agent+step accuracy on
Who&When (vs 21.7% baseline) and repaired 13/73 failed GAIA tasks, improving accuracy
from 55.8% to 63.6%. AgentDebugX's Error Hub is the closest existing system to TC's
planned failure commons -- but scoped to GAIA benchmarks, not coding agents. TC is the
next-generation version: broader (all scored traces, not just failures), TEE-attested,
NEAR credit-incentivized.

**AgenTracer-8B** (arXiv:2509.03312, confirmed; ICLR 2026 status UNVERIFIED): First
automated framework for annotating failed multi-agent trajectories. Outperforms
Gemini-2.5-Pro and Claude-4-Sonnet by up to 18.18% on Who&When. Its TracerTraj dataset
of structured failed trajectory annotations is a potential seeding corpus for TC if
open-sourced -- complementing TC's success-biased corpus with high-quality failure data.

**TRAIL** (arXiv:2505.08638, confirmed): Three-domain failure taxonomy (Reasoning Errors,
System Execution Errors, Planning/Coordination Errors) across 148 traces, 1,987 OTel
spans, and 841 annotated errors. OTel/OpenInference compatible. TRAIL's taxonomy is the
natural candidate for TC's canonical failure classification. TC's corpus (~352 submissions)
already exceeds TRAIL's 148 traces in volume.

**TraceProbe** (2026): Automated root-cause localization for agent failures using trace
structure. 82% accuracy on failure step identification without re-execution.

**AgentLocate** (2026): Fault localization using attention pattern analysis over
tool-call sequences.

### What to Build

**Failure-attribution stage.** A new gate extension on traces with negative outcomes
that identifies: failure type (compilation error, test failure, incorrect output,
timeout, tool misuse, hallucination), root-cause span, diagnosis path, and repair diff.

**Bundle schema:**

```rust
struct FailureAttribution {
    outcome: TraceOutcome,
    failure_type: Option<FailureType>,
    root_cause_step: Option<u32>,
    diagnosis_path: Vec<DiagnosisStep>,
    repair_diff: Option<String>,
    similar_failure_count: u32,
}
```

**Search interface** -- CLI (`tc search-failures "cargo build failed"`) and API endpoint
that agent harnesses call automatically on failure. Returns bundles ranked by relevance,
each showing root cause, fix, and novelty rating.

**Failure novelty scoring.** TC's existing novelty scoring gains a failure dimension: a
failure trace documenting a previously unseen failure mode is more valuable than the
50th "agent tried to import a nonexistent module." This creates targeted contribution
incentives -- the system can tell contributors which failure types the corpus needs more
of, turning the Error Hub into a directed contribution engine.

**Separate failure-trace consent.** Failure traces often contain the exact code and
prompts that triggered the failure. The consent model needs a separate opt-in for
failure-trace contribution, distinct from general trace contribution.

**Effort**: 6-8 weeks for MVP.

---

## 3. Agent Skills Publishing

The Agent Skills ecosystem uses SKILL.md, an open standard from the Linux Foundation's
Agentic AI Foundation. Skills are reusable capability descriptions that any compatible
agent can discover and execute.

### Ecosystem Scale (August 2026)

| Metric | Status |
|---|---|
| Compatible products | **32+** (Claude Code, Codex, Cursor, Gemini CLI, Windsurf, etc.) |
| Largest registries | **SkillsMP**: 1.5M indexed; **skills.sh**: 83K skills, 8M installs |
| Total across registries | **490,000+** skills (**⚠️ UNSOURCED** — this figure cannot be traced to a primary source; official agentskills.io lists ~40 adopters, catalog sizes vary by directory) |

### Security Landscape

**ClawHavoc incident**: 341 malicious skills discovered in public registries affecting
300K+ users. Attack patterns include prompt injection, code execution, and data
exfiltration via tool-call redirection.

Security scanners exist but are insufficient:
- **SkillSieve**: F1 = 0.920 (best available, still misses ~8% of malicious skills)
- **SkillSpector** (NVIDIA): 64 detection patterns across 16 categories
- **Trail of Bits** bypassed all existing scanners in under 1 hour
- **NVIDIA Verified Agent Skills**: 162 signed skills through an 8-stage pipeline
- **OWASP AST10** published February 2026 (first formal threat taxonomy)

**TC positioning**: The only system that can produce skills with provenance-verified
quality scores AND security scanning backed by a real trace corpus. TC-published skills
carry: quality score from the gate pipeline, security scan result, provenance chain
linking to source traces, and contributor reputation (Glicko-2).

### What to Build

**Manual curation CLI (v1)**: `tc skill publish --traces <id1>,<id2>,...` -- extract
common procedure from selected traces, format as SKILL.md, run security scan, attach
provenance chain. **1-2 weeks.**

**Automated extraction (v2)**: Offline consolidation on clusters of similar traces.
Identifies recurring patterns across traces sharing a task category with positive
outcomes, abstracts into model-agnostic skill descriptions, validates on held-out
tasks, scores for security. **12-16 weeks.**

**Weighted-influence attribution.** Map each published skill to its contributing traces.
When a skill earns credit (downloads, adoption, endorsements), flow credit back to
contributors weighted by influence -- traces that contributed more to the pattern earn
proportionally more. This solves the dilution problem: extracting from 500 traces
without weighting gives each contributor a meaningless 1/500th share. Without influence
weighting, skill extraction actively discourages contribution.

---

## 4. Protocol-Level Integration

### MCP Tool-Call Events

MCP (Model Context Protocol) is the standard for tools that LLMs invoke during sessions.
Tool calls are the most common structured interaction in agent traces. Promote MCP
`tools/call` JSON-RPC exchanges to a schema-level `ToolCallEvent` type (tool name,
server identity, redacted I/O, timing, errors) instead of embedding them in
conversation turns. Enables tool-use quality scoring, tool-use profiles, and
tool-specific search in the Error Hub. Ships alongside OTel ingest. **1-2 weeks.**

### A2A Protocol

A2A (Agent-to-Agent Protocol): Google-initiated, Linux Foundation-housed, v1.0.0
released, 150+ member organizations. Complementary to MCP -- MCP is vertical (tools),
A2A is horizontal (agent-to-agent delegation).

Add a `DelegationEvent` type capturing delegator identity (anonymized), delegatee
capability card, redacted task description, result, and chain depth. Parent span =
delegating agent, child span = delegated agent, linked by `trace_id` + parent
`span_id`. A2A defines three trace data channels: `TaskStatusUpdateEvent` for progress,
a Traceability metadata extension (sample, not normative), and OTel `traceparent`
propagation. The AAIF Observability Working Group is the standards body to watch.

Build when multi-agent traces appear in meaningful volume. **4-6 weeks.**

### W3C Trace Context

Cross-organizational trace stitching via `traceparent`. Store alongside TC envelope ID,
query by shared trace ID. Privacy constraint: cross-org stitching reveals
inter-organizational workflow details and needs bilateral opt-in. **2-3 weeks.**

### Protocol Versioning Strategy

MCP, A2A, ACP, and ANP are all young protocols. Treat protocol-specific event types as
versioned extensions to the core schema (not core fields) so protocol changes do not
require schema migrations. Start with MCP tool calls only (most common,
best-standardized), add A2A delegation when multi-agent traces appear, defer ACP/ANP
until there is demonstrated demand.

---

## 5. Cross-Agent Cost Tracking

### Emerging Tools (2026)

Three tools emerged for cross-agent cost tracking, none combining cost with quality:

| Tool | Focus | Scale |
|---|---|---|
| **TokenShift** (PointFive) | Token-level cost attribution, per-task breakdown | $60M Series B |
| **Exceeds AI / Exceeds Ink** | Multi-provider billing, code-level provenance | Code-level cost attribution |
| **UseAI** | Usage analytics across AI tools | Free, open-source |

No existing tool combines cost with quality scoring. TC adds three dimensions:
quality scoring (was the output good?), failure attribution (why did it fail?), and
quality-normalized cross-agent comparison (quality per dollar per task type).

**Opportunity**: Partner rather than compete. TC's gate pipeline enriches their cost
data with quality signals. OTel ingest makes this natural -- cost tools export to TC
as another OTel destination.

---

## 6. Trajectory Replay

### What to Build

**Terminal viewer (v1)**: `tc replay <trace-id>` -- step through events, show tool calls
with timing and token counts, highlight failure points. TUI using ratatui. Works
offline. **2-3 weeks.**

**SSE replay stream**: Server-side event stream at configurable speed (real-time, 2x,
5x, step-by-step). Any web UI can consume. Enables embedding replays in docs/blog
posts. **1-2 weeks.**

**Web viewer (v2)**: Timeline with expandable events, cost accumulation graph, scoring
overlays, side-by-side comparison of two traces on similar tasks. **6-8 weeks.**

**AgentGUI** (ETH Zurich, arXiv:2607.26300): Demonstrated 38% faster trace element
identification with visual replay. Validates the trajectory viewer investment.

### Anonymization for Network Replay

Network replay (viewing anonymized traces from other contributors) needs stronger
anonymization than standard redaction. Beyond PII and secrets, strip identifiable coding
patterns, project-specific identifiers, and content that could de-anonymize the
contributor. Start conservative: heavy redaction, metadata-only replay for sensitive
content. Relax as privacy techniques improve.

Contributors must be able to preview what their replayed trajectory looks like before
opting in. Start with a metadata-and-summary view (what happened at each step, without
full content) -- rendering full code diffs, terminal output, and tool interactions is
substantially more work.

---

## 7. Claude Code Integration

### 30 Hook Lifecycle Events

Claude Code exposes hooks at 30 lifecycle events. Key events for TC:

| Event | When | TC Relevance |
|---|---|---|
| **SessionEnd** | After session ends | **Primary**: Post-session trace archival |
| **PreToolUse** | Before tool execution | Real-time tool-call telemetry |
| **PostToolUse** | After tool execution | Tool result capture with timing |
| **Stop** | Response interrupted | User-interrupt pattern tracking |
| **SubagentSpawn** | Sub-agent created | Multi-agent trace stitching |
| **Notification** | System notification | Error/warning capture |

### SessionEnd Integration

```jsonc
// .claude/hooks.json
{
  "hooks": {
    "SessionEnd": [{
      "matcher": {},
      "hooks": [{
        "type": "command",
        "command": "tc scan --last --quiet --auto-submit-if-opted-in"
      }]
    }]
  }
}
```

**Constraints**: SessionEnd has a 1.5-second default budget (hardcoded). Since v2.1.74,
respects a custom `timeout` field. Use `nohup`+`disown` for network calls to avoid
blocking exit. Third-party examples: `opentelemetry-hooks`, `claude_telemetry`, Langfuse
hook. Must be a silent no-op if TC is not configured.

### Distribution via SKILL.md

A SKILL.md that teaches Claude Code to install TC creates a self-bootstrapping
distribution mechanism: user discovers skill -> agent installs TC -> TC captures
sessions -> TC publishes more skills.

---

## 8. IronClaw Integration Status

### Shipped

Full pipeline works end-to-end: agent turn -> capture -> policy check -> envelope ->
redact -> credential resolve -> JWT -> ingest -> gate -> credit.

| PR | Summary | When |
|---|---|---|
| IronClaw #4559 | Agent-driven onboarding, Ed25519 auth, standing consent, deterministic redaction | Jun 2026 |
| IronClaw #5280 | Instance enrollment, community profiles, trace inspection, credit tracking | Jun 2026 |
| IronClaw #5858 | Instance enrollment CLI, hosted-user login links | Jul 2026 |
| TC #152 | Per-user device-key subjects, namespaced `principal_ref` | Jun 2026 |

No TC-specific activity in recent IronClaw development (current focus: channels, WebUI,
model routing). **TC must drive the next round of integration improvements.**

### Critical Fixes (Trust Path)

**1. No TLS enforcement on credential HTTP clients.** Bearer tokens and JWTs sent over
plaintext if operator sets `http://` endpoint. **Fix**: Scheme check at client
construction. `allow_insecure = true` for localhost only. ~10 lines.

**2. Quarantine check keyed on prose substring.** Stringly-typed `"quarantined"` match
against what should be a typed enum. TC wording change silently breaks the privacy
boundary. **Fix**: `TraceStatus` enum or exhaustive match with quarantined-by-default
for unknown values.

**3. Empty-bytes `redaction_hash` on serialization failure.** Error swallowed, hash
computed over empty slice. Every failure produces same hash. **Fix**: Propagate error,
reject trace.

**4. No behavioral tests for `ContributionHttpSink`.** HTTP layer fully mocked. No test
has made a real HTTP request to a TC-compatible endpoint. **Fix**: `wiremock`-based
integration test.

### High-Impact Opportunities

- **Immediate scoring feedback**: Return quality score + percentile + credits in
  submission response; IronClaw surfaces inline.
- **WASM fuel as quality signal**: Manipulation-resistant computational work measure.
- **Cross-provider comparison**: 26 providers x standardized scoring = neutral data.
- **Channel-specific patterns**: One field addition enables quality comparison across
  CLI, Telegram, Slack, Discord, Signal.

---

## 9. Quarterly Roadmap

```
Q1:  OTel ingest + MCP tool-call events
Q2:  Error Hub (failure attribution + search)
Q3:  Skill extraction + SKILL.md publishing
Q4:  Trajectory replay + A2A delegation events
```

Each quarter's work makes the next quarter's feature more valuable: OTel brings volume,
Error Hub makes it useful, Skills distribute it, Replay turns it into a learning tool.

OTel ingest first because it eliminates the per-harness integration tax. Error Hub vs.
Skills is a toss-up: Error Hub is the stronger community magnet, Skills the stronger
distribution channel. Trajectory replay matters for retention but is less urgent than
widening ingest. Protocol integrations are background work -- MCP ships with OTel.
