# OTel GenAI Semantic Conventions: Precise Technical State

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving registry of AI coding
agent session traces (~235K LOC, 6 crates, MIT/Apache-2.0). Contributors submit scrubbed
traces; TC scores quality and novelty inside TEEs (Trusted Execution Environments --
hardware-isolated encrypted compute enclaves); contributors earn NEAR blockchain credits.
~352 submissions, 3 contributors. TC plans OTel-native ingest to accept traces from agent
frameworks that emit OpenTelemetry GenAI spans, enabling any instrumented agent framework
to submit traces without a custom exporter. The GenAI semantic conventions are NOT stable --
every span, event, metric, and attribute remains at "Development" status with no graduation
timeline. This document captures the exact technical state as of August 2026 to prevent
building on shifting ground.

---

## 1. Key Correction from v5

v5 (document 03-integrations-and-ecosystem.md) stated "OTel GenAI v1.42.0 is the de facto
standard." This was wrong in two important ways:

1. **There is no GenAI v1.42.0.** v1.42.0 is the semantic-conventions main repo release that
   *deprecated and removed* all GenAI conventions. The GenAI conventions were moved to a
   dedicated repository that has no tagged release at all.

2. **Nothing is stable.** Every GenAI semantic convention -- every span kind, every event
   type, every metric name, every attribute key -- remains at "Development" status. There is
   no finalized schema URL. There is no Stable graduation timeline.

The correction matters because TC's OTel-native ingest design must account for conventions
that can and will change without backward-compatibility guarantees.


## 2. Timeline of Events

### 2.1 Deprecation and Move (June-July 2026)

- **June 12, 2026** -- semantic-conventions **v1.42.0** released. All GenAI conventions
  (spans, events, metrics, attributes) marked DEPRECATED in the main
  `open-telemetry/semantic-conventions` repository. Conventions moved to the dedicated
  `open-telemetry/semantic-conventions-genai` repository.

- **July 3, 2026** -- semantic-conventions **v1.43.0** released. Contains zero GenAI
  conventions. The main repo no longer ships GenAI definitions.

- **As of July 17, 2026** -- per John Hodge's analysis of the dedicated repository: no
  GenAI-specific span, event, metric, or attribute is marked Stable. Every definition
  remains at Development status.

### 2.2 Current State of the Dedicated Repository

The `open-telemetry/semantic-conventions-genai` repository:

- Has **no tagged release** (no v0.1.0, no v1.0.0, nothing)
- Has **no finalized schema URL** (no `https://opentelemetry.io/schemas/genai/...`)
- Contains only Development-status definitions
- Has **no published timeline for Stable graduation**
- Is governed by the OTel GenAI SIG, which meets regularly but has not announced
  stabilization milestones

### 2.3 The Breaking Rename (v1.39.0)

The most dangerous change for TC's ingest pipeline:

```
gen_ai.system  -->  gen_ai.provider.name
```

This rename was confirmed in the opentelemetry.io registry as of v1.39.0. The old attribute
name `gen_ai.system` was replaced by `gen_ai.provider.name`.

**Why this is critical for TC**: any code that filters, routes, indexes, or queries on
`gen_ai.system` will silently stop matching traces emitted by frameworks that adopted the
new name. There is no error, no warning -- the attribute simply does not exist under the
old key. Traces pass through but are unclassifiable by provider.

TC must build an alias shim before any OTel ingest work begins (see Section 7).


## 3. Datadog Native Support

**Date correction**: v5 research materials (research3.md) stated Datadog shipped native
GenAI support in 2026. The correct date is **December 1, 2025**. Datadog shipped native
OTel GenAI semantic convention support in OTel SDK/Collector v1.37.

Datadog auto-maps the following GenAI attributes into its APM/LLM Observability product:

| Attribute | Mapping |
|---|---|
| `gen_ai.request.model` | Model name in LLM Observability |
| `gen_ai.usage.input_tokens` | Input token metrics |
| `gen_ai.usage.output_tokens` | Output token metrics (implied by pipeline) |
| `gen_ai.provider.name` | Provider classification |
| `gen_ai.operation.name` | Operation type classification |

This matters for TC because Datadog's adoption demonstrates that production systems are
already consuming these attributes despite their Development status. TC's ingest must be
compatible with what Datadog emits and expects.


## 4. Complete GenAI Attribute Reference (All Development Status)

Every attribute listed below is at Development status. None have Stable guarantees. Any
can be renamed, restructured, or removed without a deprecation period.

### 4.1 Span Attributes

These attributes are set on spans representing GenAI operations:

| Attribute | Type | Description |
|---|---|---|
| `gen_ai.operation.name` | string | The GenAI operation being performed: `chat`, `text_completion`, `embeddings`, `create_agent`, `execute_tool` |
| `gen_ai.provider.name` | string | Provider identifier (e.g., `openai`, `anthropic`, `cohere`). Was `gen_ai.system` before v1.39.0 |
| `gen_ai.request.model` | string | The model name as specified in the request (e.g., `gpt-4o`, `claude-sonnet-4-20250514`) |
| `gen_ai.response.model` | string | The model name as reported in the response. May differ from `request.model` if the provider aliases or upgrades models |
| `gen_ai.request.max_tokens` | int | Maximum number of tokens the model should generate |
| `gen_ai.request.temperature` | double | Sampling temperature |
| `gen_ai.request.top_p` | double | Nucleus sampling parameter |
| `gen_ai.request.top_k` | int | Top-k sampling parameter |
| `gen_ai.request.stop_sequences` | string[] | Stop sequences |
| `gen_ai.request.frequency_penalty` | double | Frequency penalty |
| `gen_ai.request.presence_penalty` | double | Presence penalty |
| `gen_ai.response.id` | string | Provider-assigned response identifier |
| `gen_ai.response.finish_reasons` | string[] | Reasons the model stopped generating (e.g., `stop`, `length`, `tool_calls`) |

### 4.2 Metric Attributes and Instruments

| Metric / Attribute | Type | Description |
|---|---|---|
| `gen_ai.usage.input_tokens` | int | Number of tokens in the input/prompt |
| `gen_ai.usage.output_tokens` | int | Number of tokens in the output/completion |
| `gen_ai.client.token.usage` | Histogram | Client-side histogram of token usage, bucketed by operation and token type |
| `gen_ai.client.operation.duration` | Histogram | Duration of GenAI operations |
| `gen_ai.server.request.duration` | Histogram | Server-side request duration (if server-instrumented) |
| `gen_ai.server.time_per_output_token` | Histogram | Time per output token (server-side) |
| `gen_ai.server.time_to_first_token` | Histogram | Time to first token (server-side) |

### 4.3 Event Attributes

Events are attached to GenAI spans as log records with specific bodies:

| Event Name | Description |
|---|---|
| `gen_ai.content.prompt` | Captures the prompt/input content sent to the model. Body contains the message array. |
| `gen_ai.content.completion` | Captures the completion/output content from the model. Body contains the response message array. |
| `gen_ai.choice` | Represents a single choice/completion from the model, including finish reason and message content. |
| `gen_ai.tool.message` | Tool call or tool result message event. |
| `gen_ai.system.message` | System message event (the system prompt). |
| `gen_ai.user.message` | User message event. |
| `gen_ai.assistant.message` | Assistant message event. |

### 4.4 Attributes TC Must Ingest

For TC's OTel-native ingest pipeline, the minimum viable attribute set is:

```
Required (must be present for trace acceptance):
  gen_ai.provider.name      -- classify by provider
  gen_ai.request.model      -- classify by model
  gen_ai.operation.name     -- classify by operation type

Required (for scoring):
  gen_ai.usage.input_tokens   -- cost estimation, efficiency scoring
  gen_ai.usage.output_tokens  -- cost estimation, efficiency scoring

Optional (enrich if present):
  gen_ai.response.model       -- detect model aliasing
  gen_ai.response.id          -- deduplication
  gen_ai.response.finish_reasons -- completion quality signal
  gen_ai.request.temperature  -- reproducibility metadata
  gen_ai.content.prompt       -- content analysis (if not redacted)
  gen_ai.content.completion   -- content analysis (if not redacted)
```


## 5. OpenInference (Arize/Phoenix) -- Parallel Convention Set

OpenInference is a separate, independently maintained semantic convention set created by
Arize AI and used by their Phoenix observability platform. It is NOT part of the
OpenTelemetry project.

### 5.1 Key Differences from OTel GenAI

| Aspect | OTel GenAI | OpenInference |
|---|---|---|
| Governance | CNCF / OTel GenAI SIG | Arize AI (open-source, single-vendor origin) |
| Attribute prefix | `gen_ai.*` | `llm.*`, `embedding.*`, `retriever.*` |
| Span kinds | Uses standard OTel span kinds + semantic attributes | Defines custom span kinds: `LLM`, `CHAIN`, `TOOL`, `AGENT`, `EMBEDDING`, `RETRIEVER`, `RERANKER`, `GUARDRAIL` |
| Model attribute | `gen_ai.request.model` | `llm.model_name` |
| Provider attribute | `gen_ai.provider.name` | `llm.provider` |
| Token usage | `gen_ai.usage.input_tokens` | `llm.token_count.prompt` |
| Content capture | Events (`gen_ai.content.prompt`) | Span attributes (`llm.input_messages`, `llm.output_messages`) |
| Status | Development (no Stable) | In production use, versioned, but no CNCF backing |

### 5.2 Adoption

Both convention sets have real-world adoption:

- **OTel GenAI**: Datadog, Elastic, Honeycomb, AWS, Azure (via official OTel instrumentation libraries)
- **OpenInference**: Arize, Phoenix, LangChain (via OpenInference instrumentation), LlamaIndex, CrewAI

### 5.3 Implication for TC

If TC wants broad ingest compatibility -- accepting traces from any major agent framework
without requiring a custom exporter -- it must support both convention sets. This doubles
the attribute mapping surface but is necessary because:

1. Neither convention set has won. Both are actively used in production.
2. Agent frameworks pick one or the other. LangChain traces use OpenInference by default
   through the `openinference-instrumentation-langchain` package. OpenAI's SDK uses OTel
   GenAI conventions.
3. Asking contributors to re-instrument is a non-starter for adoption.

TC's ingest pipeline should normalize both convention sets into a common internal
representation (TC's own span schema).


## 6. TC Pinning Strategy

Given that all GenAI conventions are Development-status and subject to change, TC must
pin to a specific snapshot and manage drift explicitly.

### 6.1 Version Pin

Pin to **main-repo v1.42.0 attribute names** -- the last versioned cut that included GenAI
conventions before deprecation and removal. This provides:

- A concrete, immutable reference point (the v1.42.0 tag in `open-telemetry/semantic-conventions`)
- Known attribute names that existing instrumentation libraries emit
- A schema URL that was valid at time of release

### 6.2 Dual-Emission Support

Enable the `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` environment variable
for frameworks that support dual-emission. This causes instrumentation libraries to emit
both old and new attribute names simultaneously, giving TC time to migrate.

### 6.3 Migration Triggers

TC should plan migration from v1.42.0 pinned names to newer conventions when ANY of these
conditions are met:

1. The dedicated `semantic-conventions-genai` repo publishes its first tagged release
2. Any GenAI attribute reaches Stable status
3. A major instrumentation library (OpenAI SDK, LangChain, etc.) drops support for
   v1.42.0-era attribute names

### 6.4 Monitoring

- Subscribe to `open-telemetry/semantic-conventions-genai` releases and PRs
- Monitor OTel GenAI SIG meeting notes for stabilization discussions
- Track Datadog/Elastic/Honeycomb changelog for convention version bumps


## 7. Alias Shim Design

The alias shim is TC's defense against the `gen_ai.system` to `gen_ai.provider.name`
rename and any future renames. It sits at the front of the ingest pipeline.

### 7.1 Behavior

```
Input span arrives with attributes
  |
  v
Alias shim checks each attribute key against known aliases
  |
  v
If old name found (gen_ai.system):
  - Map to canonical name (gen_ai.provider.name)
  - Store canonical name internally
  - Preserve original name in metadata for provenance
  |
  v
If new name found (gen_ai.provider.name):
  - Store as-is (already canonical)
  |
  v
If both found:
  - Use new name value
  - Log warning for diagnostics
  |
  v
On export / query response:
  - Emit both old and new names for backward compatibility
  - Configurable: strict mode emits only canonical names
```

### 7.2 Alias Table

```
Old Name                    Canonical Name                  Since
-----------------------------------------------------------------------
gen_ai.system               gen_ai.provider.name            v1.39.0
```

The table is intentionally small today. As the dedicated repo evolves, additional renames
will be added here. The shim is designed to be a lookup table, not a transformation engine.

### 7.3 Implementation Scope

Approximately 50 lines of Rust in the ingest pipeline:

- A `HashMap<&str, &str>` for old-to-canonical mapping
- A normalization pass over incoming span attributes
- A reverse mapping for export compatibility
- Unit tests covering: old-name-only, new-name-only, both-present, neither-present

This must be implemented before any OTel ingest work begins. It is not optional.


## 8. OpenInference Normalization

For OpenInference traces, TC needs a parallel normalization layer that maps OpenInference
attributes to TC's internal schema.

### 8.1 Core Mapping

```
OpenInference                    TC Internal (OTel-canonical)
-----------------------------------------------------------------
llm.model_name                   gen_ai.request.model
llm.provider                     gen_ai.provider.name
llm.token_count.prompt           gen_ai.usage.input_tokens
llm.token_count.completion       gen_ai.usage.output_tokens
llm.input_messages               gen_ai.content.prompt (event body)
llm.output_messages              gen_ai.content.completion (event body)
```

### 8.2 Span Kind Mapping

```
OpenInference Span Kind          TC Classification
-----------------------------------------------------------------
LLM                              gen_ai operation (chat/completion)
CHAIN                            orchestration span
TOOL                             tool execution span
AGENT                            agent session span
EMBEDDING                        gen_ai operation (embeddings)
RETRIEVER                        retrieval span
RERANKER                         scoring span
GUARDRAIL                        safety/gate span
```

### 8.3 Detection

TC detects which convention set a trace uses by checking for sentinel attributes:

- If `gen_ai.provider.name` or `gen_ai.system` is present: OTel GenAI conventions
- If `llm.provider` or `llm.model_name` is present: OpenInference conventions
- If both are present: prefer OTel GenAI, log diagnostic

This detection happens once per trace, not per span.


## 9. Risk Assessment

### 9.1 High Risk: Building on Unstable Conventions

The GenAI conventions have no Stable attributes. Any attribute can be renamed (as
`gen_ai.system` was), restructured, or removed. Building a production ingest pipeline on
Development-status conventions means accepting that the wire format will change.

### 9.2 Mitigation Stack

| Layer | Mitigation | Effort |
|---|---|---|
| Alias shim | Absorbs renames without code changes | ~50 LOC, implement first |
| Version pinning | Fixes TC's expected attribute set to v1.42.0 snapshot | Configuration only |
| Dual-emission env var | Receives both old and new names during transitions | Configuration only |
| Internal canonical schema | TC stores its own attribute names; OTel names are wire format only | Architecture decision |
| OpenInference support | Reduces dependency on any single convention set | ~100 LOC mapping layer |

### 9.3 Decision Points

- **If dedicated repo reaches Stable by Q4 2026**: migrate pinned names to Stable definitions.
  This is the happy path.

- **If dedicated repo does not reach Stable by Q1 2027**: evaluate whether OpenInference
  should become TC's primary convention set, with OTel GenAI as secondary. OpenInference is
  less prestigious (no CNCF backing) but more pragmatically stable (Arize ships production
  releases with versioned schemas).

- **If OTel GenAI SIG announces a v2 restructuring**: freeze TC's OTel ingest at the
  pre-restructuring attribute set and wait for the dust to settle.

### 9.4 Monitoring Checklist

- [ ] Subscribe to `open-telemetry/semantic-conventions-genai` GitHub releases
- [ ] Monitor OTel GenAI SIG meeting notes (bi-weekly)
- [ ] Track `gen_ai.*` attribute changes in opentelemetry.io registry
- [ ] Watch Datadog, Elastic, Honeycomb changelogs for convention version bumps
- [ ] Monitor LangChain/LlamaIndex for OpenInference version changes
- [ ] Check instrumentation library release notes for dual-emission support


## 10. Summary

The OTel GenAI semantic conventions are in an awkward intermediate state: widely adopted by
observability vendors (Datadog shipped support in December 2025), referenced as "the
standard" by multiple agent frameworks, but technically unstable with no Stable-status
attributes and no tagged release in the dedicated repository.

TC's approach:

1. **Pin** to v1.42.0 attribute names as the reference snapshot
2. **Shim** the `gen_ai.system` to `gen_ai.provider.name` rename immediately
3. **Support** both OTel GenAI and OpenInference convention sets for broad compatibility
4. **Monitor** the dedicated repo for stabilization signals
5. **Decide** by Q1 2027 whether to shift primary convention allegiance if OTel remains unstable

The alias shim (~50 LOC) and convention detection logic are prerequisites for any OTel
ingest implementation. They must be built first, tested independently, and deployed before
TC accepts its first OTel GenAI span.
