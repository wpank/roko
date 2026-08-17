# Gateway Adapter Opportunities

The gateway subsystem is roko's most adapter-rich surface. Every layer in the proxy pipeline
is a trait boundary. This document maps each layer to its adapter interface, existing state,
integration opportunities, and competitive positioning in the April 2026 market.

Last updated: 2026-04-29.

---

## Market Context: The Gateway Layer in April 2026

The standalone LLM gateway market has not produced a breakout company:

| Company | Model | Revenue | Assessment |
|---|---|---|---|
| **OpenRouter** | 5.5% credit fee | $30-50M ARR run rate | Dominant volume play but structurally capped |
| **Portkey** | $49/mo Pro | $5-10M ARR | Growing but competing on price |
| **Braintrust** | Eval + routing | <$10M ARR ($300M Series B, Casado-led) | Pivoting toward eval |
| **LiteLLM** | OSS + enterprise | Small | Commodity layer |
| **Cloudflare AI Gateway** | Bundled with Workers | Platform threat | Near-zero pricing |

**Strategic conclusion**: Standalone gateways cap at ~$50M ARR. Migration takes hours (just
change the endpoint URL). The gateway must be positioned as the **data-acquisition layer**
for the orchestration platform, not a standalone product.

But inside roko's architecture, the gateway is the richest adapter surface: 8 layers, each
a trait boundary, each composable with the rest of the system. When embedded in roko-serve,
the gateway layers feed learning loops, gate calibration, and routing intelligence. This is
the compound value that standalone gateways cannot access.

---

## Gateway Pipeline as Adapter Stack

```
Request -> Auth -> Format -> Safety -> Cache -> Router -> Optimize -> Billing -> Provider -> Response
            |        |        |        |       |          |          |          |
            v        v        v        v       v          v          v          v
        AuthAdapter  Format  Safety   Cache   Router   Optimizer  Billing   Provider
                    Adapter  Pipeline Layer            Pipeline   Provider
```

Every `v` is a trait boundary. Every trait boundary is an adapter surface.

---

## Layer-by-Layer Analysis

### 1. Provider (`Provider` trait) -- EXISTS

**Trait**: `Provider` with `id()`, `can_handle()`, `capabilities()`, `forward()`, `health_check()`

**Existing**: 6 protocol families (ClaudeCli, AnthropicApi, OpenAiCompat, CursorAcp,
PerplexityApi, GeminiApi)

**Planned**: 20+ provider implementations:
- Tier 1: Anthropic, OpenAI, Google, DeepSeek, Mistral (5)
- Tier 2: OpenRouter, HuggingFace, Bedrock, Azure (4)
- Tier 3: Groq, Together, Fireworks, Cerebras, SambaNova, Venice (6)
- Tier 4: Ollama, vLLM, llama.cpp, TGI, HF Endpoints (5)

**Why this matters (April 2026)**: Cursor depends on Anthropic. Codex CLI is locked to
OpenAI. Claude Code is Anthropic-only. Roko's Provider trait means any backend via TOML config.
Adding a new OpenAI-compat provider = 5 lines of TOML.

**Cost economics**: GPT-5.5 is $5/$30 per 1M tokens (1M context). Claude Sonnet 4.6 is
$3/$15. Gemini 2.0 Flash is $0.10/$0.40. CascadeRouter routes cheap models to easy tasks
-> 5-10x cost reduction vs single-model competitors.

---

### 2. Router (`Router` trait) -- EXISTS

**Trait**: `Router` with `route()`, `observe()`

**Existing**: CascadeRouter (3-stage: static -> Wald CI -> LinUCB bandit)

**Integration opportunities**:

| Integration | What It Does |
|---|---|
| HuggingFace Hub API | Dynamic model discovery -> auto-add CascadeRouter arms |
| OpenRouter metadata | Model capability/pricing -> inform routing decisions |
| A/B experiments | Route variants -> ExperimentStore convergence |
| External eval | SWE-bench, HumanEval -> benchmark routing quality |

**Competitive differentiator**: No competing product has bandit-based model routing that
learns from execution outcomes. LangGraph users manually specify models. Cursor users get
whatever Anysphere negotiates with Anthropic. Roko's Router trait makes model selection a
first-class concern.

**Generalization**: Router composition -- stacking routers (cost router -> quality router ->
availability router) via a `CompositeRouter`.

---

### 3. Cache Layer (`CacheLayer` trait) -- DESIGNED

**Trait**: `CacheLayer` with `get()`, `put()`, `stats()`

**Planned implementations**:
- L1: BLAKE3 hash + Moka LRU (in-memory, exact match)
- L2: SimHash semantic (configurable) or embedding-based (fastembed ONNX)
- L3: Anthropic `cache_control` prefix injection

**Integration opportunities**:

| Integration | What It Does |
|---|---|
| Redis | Shared cache across gateway instances |
| DiskCache | Persistent cache surviving restarts |
| Cloudflare Workers KV | Edge cache for distributed deployment |

**Cost impact**: Prompt cache alone contributes 1.5-2x cost reduction in HAL benchmark data.
Combined with L2 semantic caching (near-miss matching), estimated 2-3x on repetitive
workloads.

---

### 4. Safety Pipeline -- PARTIALLY EXISTS

**Current**: `AgentContract` system with 8 bundled contracts. Falls open on missing YAML.

**Adapter traits**:
- `PiiScanner`: detect and mask PII patterns
- `InjectionDetector`: detect prompt injection attempts
- `PrivacyClassifier`: classify request sensitivity for routing

**Integration opportunities**:

| Integration | What It Does |
|---|---|
| External PII service | Specialized PII detection beyond regex |
| ML injection classifier | ONNX model for prompt injection |
| EU AI Act compliance | Article 50 transparency checks |
| Audit logger | Every safety decision logged for compliance |

**Regulatory context**: EU AI Act Article 50 enforcement begins August 2, 2026. The safety
pipeline's audit log is compliance evidence. No competing gateway produces this.

---

### 5. Optimizer (`Optimizer` trait) -- DESIGNED

**Trait**: `Optimizer` with `optimize_request()`, `optimize_response()`

**Planned**: 12 optimizers from bardo architecture:
- Tool pruning, tool schema compression, tool result compression
- Context compression, loop detection, convergence detection
- Output budgeting, thinking cap, KV affinity
- Batch scheduling, context profiles, capability checks

**Integration opportunities**:

| Integration | What It Does |
|---|---|
| External tokenizer | Accurate token counting per model |
| Custom compression | Domain-specific context compression |
| External loop detector | ML-based loop/oscillation detection |

**User configuration**:
```toml
[gateway.optimizers]
enabled = ["tool_prune", "context_compress", "output_budget", "loop_detect"]

[gateway.optimizers.custom_compress]
kind = "external"
command = "my-compressor --format json"
```

---

### 6. Billing Provider (`BillingProvider` trait) -- DESIGNED

**Trait**: `BillingProvider` with `authorize()`, `record()`

**Planned**: Stripe (March 2026 LLM token billing native) + x402/USDC micropayments

**Market context**: Stripe's March 2026 update made LLM token billing a first-class product.
The BillingProvider adapter maps directly to Stripe's usage metering API.

---

### 7. Analytics / Observability -- PARTIALLY EXISTS

**Current**: `GatewayStats` (atomic counters), response headers with cost info. No export.

**Adapter traits**:
- `MetricsExporter`: export to Prometheus/Datadog
- `TraceExporter`: export to OTel/Jaeger
- `CostExporter`: export cost events

**`gen_ai.*` OTel Semantic Conventions**:

The gateway pipeline inserts `gen_ai.*` spans between Router and Provider:

```
Request -> Auth -> Format -> Safety -> Cache -> Router -> [gen_ai span start] -> Provider -> [gen_ai span end] -> Response
```

**Vendor support (April 2026)**:

| Vendor | Support Date |
|---|---|
| Datadog | March 2026 |
| Honeycomb | March 11, 2026 |
| Langfuse | 2025+ (ClickHouse since Jan 2026) |
| Arize Phoenix | 2025+ (EL 2.0, not Apache-2.0) |
| Laminar (lmnr.ai) | 2025+ (Apache-2.0) |
| Grafana | 2025+ |

**Gateway-specific attributes**:

| Attribute | Source | Value |
|---|---|---|
| `gen_ai.system` | Provider | "openai", "anthropic" |
| `gen_ai.request.model` | Router | Selected model |
| `gen_ai.usage.input_tokens` | Provider response | Token counts |
| `roko.gateway.cache_hit` | Cache layer | L1/L2/L3/miss |
| `roko.gateway.router_decision` | Router | CascadeRouter arm |
| `roko.gateway.cost_usd` | Billing | Computed cost |
| `roko.gateway.safety_flags` | Safety pipeline | PII/injection results |
| `roko.agent.chain_id` | Chain composition | Which chain (A-E) |
| `roko.agent.trigger_source` | Adapter | "slack" / "linear" / "github_label" |

**Implementation**: ~200 LOC for span builder + constants, ~80 LOC example. No Rust equivalent
of Python's `opentelemetry-instrumentation-openai` exists -- filling this gap gets roko
ecosystem-wide visibility.

---

### 8. Tenant Management -- DESIGNED

**Trait (implicit)**: `TenantResolver` with org/project/key hierarchy

**Integration opportunities**:

| Integration | What It Does |
|---|---|
| Auth0/WorkOS | OIDC-based tenant identity |
| LDAP/AD | Enterprise directory integration |

---

## Gateway-to-Roko Composition Matrix

How gateway adapters compound with the rest of the system:

| Gateway Layer | roko-learn | roko-gate | roko-compose | roko-neuro |
|---|---|---|---|---|
| **Provider** | CascadeRouter selects | - | - | - |
| **Router** | Bandit learns from outcomes | Gate results -> routing | - | Knowledge -> routing |
| **Cache** | Cache hit rate -> adaptation | - | Prefix cache -> prompt | - |
| **Safety** | - | - | Contracts -> prompt rules | - |
| **Optimizer** | Tool usage -> pruning | - | Compression -> budget | - |
| **Billing** | Cost -> budget degradation | - | - | - |
| **Analytics** | All learning -> export | Gate results -> export | - | Quality -> export |

**Key insight**: The gateway is both a standalone product (transparent LLM proxy) and a
subsystem of roko (embedded in roko-serve). Both modes use the same adapter traits.
Standalone mode uses config-driven defaults. Embedded mode uses roko's learning components
as adapter implementations -- and that is where the compound value lives.

---

## Business Model: Tiered Adapter Access

| Adapter | Free Tier | Pro Tier | Enterprise |
|---|---|---|---|
| Provider | 3 providers | All providers | Custom + dedicated |
| Router | Static routing | CascadeRouter | Custom routing logic |
| Cache | L1 only | L1 + L2 + L3 | Shared cache cluster |
| Safety | Basic PII | Full pipeline | Custom classifiers |
| Optimizer | 3 optimizers | All 12 | Custom optimizers |
| Billing | Free tier limits | Stripe metering | Custom invoicing |
| Analytics | Basic stats | Full OTel export | Custom dashboards |
| Tenant | 1 key | 5 keys | Unlimited + org hierarchy |

The adapter architecture naturally creates tier differentiation without code branching.
Free tier users get the same code with fewer adapter implementations enabled.

---

## Linear Wire Protocol (R8 Specification)

The gateway's most important adapter integration is Linear AgentSession. Corrected
specification from R8:

**Webhook header**: `Linear-Event: AgentSessionEvent` (not `AgentSession`)

**Two latency budgets**:
1. 5-second HTTP-200 acknowledgment (standard webhook SLA)
2. 10-second first `agentActivityCreate` mutation (before `stale` state)

**Architecture**: Spawn `tokio::task` for thought activity within 10s, drive LLM roundtrip
async, return HTTP 200 within 5s budget. This is the emit-then-async pattern that Rust
enables and Python competitors struggle with.

**Activity types**: `thought`, `elicitation`, `action`, `response`, `error` (server-validated)

**OAuth**: `actor=app` + `app:assignable` + `app:mentionable` scopes

**HMAC-SHA256**: Raw body, hex-encoded, `Linear-Signature` header. `Linear-Delivery` UUID
for dedup.

**graphql_client v0.14.0**: Introspect schema, commit, write 3 query files, derive types.
Custom scalars module required (DateTime, JSON, UUID, TimelessDate).

---

## Langfuse Partnership (Corrected R8)

**Langfuse acquired by ClickHouse January 16, 2026.** MIT license stays. 50K obs/month free
tier stays. 2027-2028 re-license watch.

**Do not use `opentelemetry-langfuse` crate** (bus factor 1, 3 stars). Use
`opentelemetry-otlp` directly with basic-auth header. Same code repoints at any vendor by
changing env vars.

**Partnership process**: GitHub Discussion -> PR to langfuse-docs -> co-marketing via Marc
Klingen (Berlin).

---

## Sources

- Gateway standalone market: OpenRouter, Portkey, Braintrust, LiteLLM, Cloudflare
- OTel gen_ai.*: semconv >=1.37, 6 vendor backends (Datadog, Honeycomb, Langfuse, Phoenix, Langtrace, Grafana)
- Langfuse: ClickHouse acquisition Jan 16, 2026 (Marc Klingen announcement)
- Arize Phoenix: Elastic License 2.0 (not Apache-2.0)
- Linear: AgentSession protocol, 11+ shipped agents, dual latency budget
- Stripe: March 2026 LLM token billing update
- HAL benchmark: 10-30x cost reduction from coordination-aware scaffolding
- Codex CLI: GPT-5.5 at $5/$30 per 1M tokens
- Gateway architecture docs: AUDIT.md, PROVIDERS.md, BUSINESS.md, INNOVATIONS.md
