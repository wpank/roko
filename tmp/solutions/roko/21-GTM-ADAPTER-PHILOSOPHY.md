# Design Principle: Adapter-First Extensibility

Everything in roko is generalizable, pluggable, and composable through adapter traits. This is
the foundational design principle that separates "a tool" from "a platform" -- and in the
April 2026 market, it is the structural answer to vendor lock-in that every competitor lacks.

Last updated: 2026-04-29.

---

## The Principle

**Every subsystem boundary is an adapter surface.** Where roko connects to anything external
(LLM providers, project trackers, CI pipelines, storage backends, observability, payments,
communication channels), that connection point is defined by a trait -- not hardcoded to a
single implementation.

This is not aspirational architecture. It is the competitive moat that explains why roko can
exist next to Cursor ($29-60B), Codex CLI (75K stars), and Claude Code ($2.5B est. revenue).

---

## Why This Creates Exponential Returns

### 1. Publisher-Consumer Asymmetry (100:1 to 1000:1)

A small number of publishers create adapter implementations. A much larger number of consumers
use them. Terraform has ~200 provider publishers serving millions of consumers. Each new adapter
immediately benefits every existing user.

### 2. Compounding Network Effects

Each adapter makes every other adapter more valuable:
- Adding a Linear adapter means issues become tasks. Adding a GitHub adapter means tasks become
  PRs. Together: **issue -> plan -> code -> PR -> merge -> issue closed** -- a closed loop
  worth more than the sum of its parts.
- Adding an OTel exporter means every subsequent integration is measurable from day one.
- Adding a Sentry adapter means errors feed back into the task queue. Combined with GitHub
  Actions: **error -> task -> fix -> CI -> deploy** -- autonomous remediation.

### 3. Lowered Barrier to Entry

If adding a new LLM provider requires modifying roko's core code, the contributor pool is
limited to people who understand 177K LOC across 18 crates. If it requires implementing a
3-method trait, anyone can contribute.

### 4. Vendor Independence as Product Feature

In the April 2026 market, this is the most important reason:

- **Cursor** depends on Anthropic Claude. Fortune reported its "very uncertain future" because
  Anthropic controls the model supply chain.
- **Codex CLI** is structurally locked to OpenAI auth. Cannot route to Claude, Gemini, or
  local models without forking.
- **Devin** is closed-source with a proprietary model. $500/month, no self-hosting.
- **Claude Code** is Anthropic-only. Throttling issues since March 2026.

Roko's adapter architecture means: swap providers via TOML config change. No code modification.
No vendor negotiation. No supply chain risk.

---

## The 7 Patterns That Drive Ecosystem Flywheels

Distilled from research across MCP (97M monthly SDK downloads), Terraform (3,500+ providers,
$6.4B acquisition), Kubernetes (300+ certified operators), n8n (6,234 nodes), Airbyte (400+
connectors), OTel (200+ components), VS Code (tens of thousands of extensions), Backstage
(3,400+ organizations), LangChain (30K+ GitHub stars), and Zed (WASM extensions).

### Pattern 1: Minimal Interface, Maximum Composability

The smallest possible trait definition. Every successful ecosystem keeps it under 5 methods:
- MCP: 3 primitives (Tools, Resources, Prompts)
- OTel: 3 pipeline stages (Receiver -> Processor -> Exporter)
- Terraform: Schema + CRUD per resource
- Airbyte: 4 commands (spec, check, discover, read)
- Bevy: 1 required method (`build`), 5 optional with defaults

**Roko target**: each adapter trait has <=5 required methods. A developer should understand
the entire interface in under 10 minutes.

### Pattern 2: Process Boundary > Code Boundary

Every platform that achieved massive ecosystem growth defined its plugin interface at the
process boundary (gRPC, stdio, HTTP, Docker, WASM), not at the code boundary:

| Platform | Original | Migration | Result |
|---|---|---|---|
| K8s | In-tree plugins | CSI/CNI over gRPC | 300+ plugins |
| Terraform | Built-in | gRPC `go-plugin` | 2,700+ providers -> 3,500+ |
| VS Code | In-process | Extension Host | Tens of thousands |
| Airbyte | Python SDK | Docker containers | 400+ in any language |
| MCP | (started with stdio) | stdio + Streamable HTTP | 17,468+ servers |

**Roko target**: adapter traits implementable either in-process (Rust trait impl) OR
out-of-process (stdio/gRPC/WASM). This unlocks contributions in any language.

### Pattern 3: Declarative Path for 80%, Programmatic Path for 20%

n8n's declarative nodes. Airbyte's low-code CDK. Terraform's HCL. 80% of REST API
integrations need zero imperative code -- just a TOML declaration.

**Roko target**: `connector.toml` manifest that generates adapter implementations from
REST API declarations without writing Rust:

```toml
[connector]
name = "linear-issues"
kind = "rest"

[auth]
type = "bearer"
token_env = "LINEAR_API_KEY"

[endpoints.fetch_candidates]
method = "POST"
url = "https://api.linear.app/graphql"
body = '{ "query": "{ issues { nodes { id title } } }" }'
response_path = "data.issues.nodes"
```

### Pattern 4: Registry + Auto-Discovery

Terraform Registry (`terraform init` auto-downloads). VS Code Marketplace. npm for MCP.

**Roko target**: `roko.toml` declares required adapters; `roko init` downloads them.
`roko-contrib` repository (OTel-contrib model) for community adapters.

### Pattern 5: The Escape Hatch

n8n's HTTP Request node. LangChain's `@tool` decorator. MCP's raw JSON-RPC.

**Roko target**: generic HTTP, webhook, and MCP adapters that work with any service.
Specific adapters are optimizations, not requirements. The platform is useful on day zero.

### Pattern 6: Catalog as Gravity Well

Backstage's Software Catalog (3,400+ orgs, 89% market share in developer portals).
Terraform's State. K8s's etcd.

**Roko target**: `.roko/` is the catalog. Signals, episodes, plans, PRDs, knowledge -- all
entity types queryable through a unified interface. Every adapter that puts data in benefits
every adapter that takes data out.

### Pattern 7: Lazy Activation

VS Code loads extensions only when their activation event fires. No startup cost for unused
adapters.

**Roko target**: adapters declare activation events in TOML:
```toml
[adapter]
name = "semgrep-gate"
activate_on = ["gate:security-scan"]
```

---

## Roko's Adapter Surface Map (April 2026)

### Tier 1: Core Pipeline (exist today, need unification)

| Surface | Trait | Existing? | Impls |
|---|---|---|---|
| LLM Provider | `Provider` / `ProviderAdapter` | Yes | 6 (Claude, OpenAI, Gemini, Cerebras, Perplexity, Ollama) |
| Model Router | `Router` (CascadeRouter) | Yes | 1 (bandit-based, 20+ provider arms) |
| Gate Rung | `GateRung` / `RungDispatcher` | Yes | 7 rungs |
| Language Provider | `LanguageProvider` | Yes | 3 (Rust/TS/Go) |
| Code Search | `CodeIndex` | Yes | 5 strategies |
| Role Template | `RolePromptTemplate` | Yes | 11 impls |
| Composition | `CompositionStrategy` | Yes | 2 (Greedy/VCG) |

### Tier 2: External Integration (partially exist)

| Surface | Trait | Status |
|---|---|---|
| Project Tracker | `WorkSource` | Designed (Linear, Jira, GitHub Issues) |
| Version Control | `VersionControlAdapter` | Partial (MCP) |
| CI/CD | `CiAdapter` | Not built |
| Notification | `NotificationAdapter` | Partial (MCP) |
| Security Scanner | `SecurityScanner` | Not built |
| Observability | `ObservabilityExporter` | Not built |
| Storage Backend | `KnowledgeBackend` | Partial (file only) |
| Auth | `AuthAdapter` | Not built |
| Billing | `BillingProvider` | Designed |

### Tier 3: Ecosystem Connectors (new)

| Surface | Trait | Status |
|---|---|---|
| Design Source | `DesignAdapter` (Figma MCP) | Not built |
| ML Platform | `MLAdapter` (W&B, HF) | Not built |
| Deploy Target | `DeployAdapter` | Partial (CLI) |
| Workflow Trigger | `WorkflowTrigger` (n8n, Zapier) | Not built |
| Agent Protocol | `AgentProtocol` (A2A, MCP) | Partial (MCP) |

---

## Adapter Trait Design Rules

### Rule 1: <=5 Required Methods

```rust
// GOOD: 3 required methods
#[async_trait]
pub trait WorkSource: Send + Sync {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>>;
    async fn update_state(&self, id: &str, state: &str) -> Result<()>;
    fn capabilities(&self) -> WorkSourceCapabilities;
}
```

### Rule 2: Capabilities Struct, Not Boolean Methods

```rust
pub struct WorkSourceCapabilities {
    pub pull: bool,
    pub push: bool,
    pub write: bool,
    pub enrich: bool,
    pub pipeline_role: PipelineRole,  // Trigger | Enrichment | WriteNotify | Multi
}
```

### Rule 3: Default Implementations for Optional Methods

Bevy's Plugin trait: 1 required method (`build`), 5 optional with defaults. Roko adopts
the same pattern.

### Rule 4: Trait Object Safe

Every adapter trait must be `dyn`-safe. Use `Box<dyn Trait>` in registries.

### Rule 5: Configuration via TOML

Every adapter instance configurable via TOML without code changes.

### Rule 6: MCP Dual-Exposure

Every adapter is both consumable as MCP (exposed as MCP server) and implementable via MCP
(external MCP servers consumed as adapter implementations).

---

## The Bevy Plugin Trait as Foundation

Bevy's `Plugin` trait is the closest existing analog to roko's adapter design:

```rust
pub trait Plugin: Downcast + Any + Send + Sync {
    fn build(&self, app: &mut App);           // required
    fn ready(&self, _app: &App) -> bool { true }
    fn finish(&self, _app: &mut App) {}
    fn cleanup(&self, _app: &mut App) {}
    fn name(&self) -> &str { /* type_name */ }
    fn is_unique(&self) -> bool { true }
}
```

**The masterstroke**: The blanket impl that makes any `fn(&mut App)` automatically a `Plugin`.
The simplest plugin is a five-line function. You only reach for a struct when you need
configuration.

**Roko adopts this**: `fn(&mut RokoBuilder)` automatically becomes a `RokoAdapter`.

**`#[derive(RokoAdapter)]` macro**: Ship a derive macro for boilerplate methods, modeled on
serde's `#[derive(Serialize, Deserialize)]`.

---

## Conformance Testing

A `roko-conformance` crate exposing:

```rust
assert_adapter_conforms::<MyAdapter>();
```

Gated by `ROKO_ACC=1` for real-network tests. Exercises:
1. Lifecycle conformance (build/ready/cleanup)
2. Schema conformance (input/output types match declarations)
3. Capability conformance (declared capabilities match behavior)
4. Idempotency conformance (repeated calls produce same result)
5. Error conformance (structured errors, not panics)

Earning "Verified" badge requires passing this crate in CI.

---

## The Codex CLI Contrast Frame

Every external piece should contain this contrast:

| Axis | Codex CLI | Roko |
|---|---|---|
| License | Apache-2.0 | Apache-2.0 |
| Language | ~95% Rust | 100% Rust (18 crates, 177K LOC) |
| Auth | OpenAI only | Any provider via adapter trait |
| Models | GPT-5.5 default, OpenAI-only | Any model via CascadeRouter |
| Observability | None shipped | Vendor-neutral OTel config knob (6 backends) |
| Verification | None | 7-rung gate pipeline |
| Learning | None | 4 compounding loops |
| Integrations | CLI-only | Linear, Slack, GitHub, Sentry, OTel |
| Architecture | Monolithic | 18-crate adapter-trait composition |
| Self-developing | No | PRD -> plan -> execute -> gate -> learn |

---

## Implementation Path

### Phase 0: Unify Existing Traits (now)

Route all callers through existing trait-based adapters instead of bypassing them.

| Service | What It Unifies | Bypass Paths Eliminated |
|---|---|---|
| ModelCallService | 13+ LLM invocation sites | 4 stream parsers, 3 dispatch functions |
| PromptAssemblyService | 6+ prompt paths | Inline format strings in ACP/run.rs |
| GateService | 3 gate dispatch paths | run.rs (4), ACP (3), orchestrate.rs (7) |
| FeedbackService | 10 learning components | Partial wiring in various entry points |

### Phase 1: External Integration Traits (Tier 2)

Define and implement the 9 Tier 2 adapter traits. Priority:
1. `ObservabilityExporter` (OTel) -- makes everything measurable
2. `WorkSource` (Linear, GitHub Issues) -- closes issue-to-code loop
3. `VersionControlAdapter` (GitHub) -- deepens PR workflow
4. `CiAdapter` (GitHub Actions) -- validates in real CI
5. `SecurityScanner` (Semgrep) -- adds security to gate pipeline

### Phase 2: Ecosystem + Distribution

1. `roko-contrib` monorepo for community adapters
2. `connector.toml` declarative format for REST API adapters
3. Registry protocol for auto-discovery
4. WASM sandboxing for untrusted extensions

---

## Sources

- MCP: 97M monthly SDK downloads, 17,468 servers (Nerq Q1 2026), AAIF 170+ orgs
- Terraform: 3,500+ providers, $6.4B IBM acquisition (Feb 2025)
- Kubernetes: 300+ certified operators
- n8n: 6,234 nodes (400 official + 5,834 community), 13.6 nodes/day
- Airbyte: 400+ connectors, Docker containers, low-code CDK
- OTel: 200+ community components, gen_ai.* semconv >=1.37
- VS Code: Tens of thousands extensions, Extension Host isolation
- Zed: WASM sandboxing, WIT versioning
- Backstage: 3,400+ orgs, 89% developer portal market share
- Bevy: Plugin trait design, function-as-plugin blanket impl
- Cursor: $2B ARR, Fortune uncertain future (Mar 2026)
- Codex CLI: 75K+ stars, Apache-2.0 Rust, OpenAI-locked
- Codex CLI: 3M weekly active users, 640+ releases (Apr 2026)
