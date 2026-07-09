# Integration Research: What Creates Exponential Returns

Comprehensive research into external integrations for roko, ranked by ROI, with adapter
interface designs, multiplicative combination analysis, and updated competitive intelligence
from the April 2026 market.

Last updated: 2026-04-29.

---

## The Integration Thesis in April 2026

The AI coding tools market hit $6.8B in 2025 with 7 companies crossing $100M ARR. But every
leader is an island: Cursor is an IDE, Codex CLI is a terminal, Claude Code is a terminal,
Devin is a SaaS agent, Copilot is an extension. None of them are integration runtimes.

Roko occupies a different slot: it is not a coding assistant. It is the orchestration layer
that connects coding agents to the systems developers already use -- Linear for issues,
GitHub for code, Slack for communication, Sentry for errors, OTel for observability. The
integration depth is the product, not the model.

The dominant template shape from n8n (9,487 templates as of April 2026) confirms this:
**trigger -> enrich -> write+notify** is the three-system minimum for retention. Single-
integration utilities do not retain users. Roko's headline recipe must chain at least three
systems.

---

## Priority Ranking (Updated April 2026)

### Tier 0: Foundation (enables everything else)

| # | Integration | Effort | Why Tier 0 |
|---|---|---|---|
| 1 | **OpenTelemetry OTLP** | Medium | One exporter unlocks six observability backends. Makes everything measurable. The single highest-compounding integration. |

### Tier 1: Core Loop (closes the autonomous developer workflow)

| # | Integration | Effort | Why Tier 1 |
|---|---|---|---|
| 2 | **GitHub** (octocrab) | Medium | Every roko user is a GitHub user. 100M+ developers. Bidirectional PR lifecycle. |
| 3 | **Linear** (AgentSession) | Medium | The gateway integration. Cursor broken. Devin validates magnitude. No Rust SDK exists. |
| 4 | **Slack** (slack-morphism) | Low | The killer demo: Slack thread -> agent -> trace URL inline. No competitor does this. |
| 5 | **GitHub Actions** | Low | Run gates in real CI. Reusable workflow template. |
| 6 | **Semgrep** | Low | Security gate rung with SARIF output. Sub-second scans. 30K+ rules. |

### Tier 2: Revenue & Intelligence

| # | Integration | Effort | Why Tier 2 |
|---|---|---|---|
| 7 | **Sentry** (MCP) | Medium | Production errors -> auto-fix PR. Seer Autofix is partial competitor but does not close Linear issues or emit gen_ai.* spans. |
| 8 | **Stripe** | Medium | Usage-based billing (March 2026 LLM token billing native). |
| 9 | **A2A Protocol** | High | 150+ orgs, Linux Foundation. Network effect with LangGraph/CrewAI agents. |
| 10 | **pgvector** | Medium | Shared knowledge store across instances. Vector search for episodes + knowledge. |
| 11 | **OAuth/OIDC** | Medium | Required for enterprise adoption. Agent identity. |

### Tier 3: Ecosystem Expansion

| # | Integration | Effort | Why Tier 3 |
|---|---|---|---|
| 12 | **x402/USDC** | Low | Agent-to-agent autonomous payments. 100M+ payments since May 2025. |
| 13 | **Plane** | Low | Self-hosted PM alternative to Linear (AGPL, no user limits). |
| 14 | **Figma MCP** | Low | Design-to-code pipeline via design tokens. |
| 15 | **W&B Weave** | Medium | Experiment visualization and team collaboration. |
| 16 | **GitLab** | Medium | Enterprise VCS with integrated CI/CD. |
| 17 | **Jira** | Medium | Enterprise PM (Atlassian stack). |
| 18 | **Buildkite** | Low | Enterprise CI (Goldman Sachs, Shopify). |

---

## 90-Day Shipping Sequence (Empirically Ordered)

R5 proposed GitHub -> OTel -> Linear -> Sentry -> Slack. R6 reordered based on named
competitor evidence. R8 corrected the Linear latency budget and observability default.

| Phase | Weeks | Integration | Role | Rationale |
|---|---|---|---|---|
| Foundation | 1-3 | **GitHub** (octocrab) | Substrate | Every user is a GitHub user. PR lifecycle is the substrate. 350 LOC MVP. |
| Gateway | 3-5 | **Linear AgentSession** | Gateway | Cursor broken, Devin validates magnitude, no Rust SDK exists. 400 LOC. |
| Advocacy | 5-7 | **Slack** (slack-morphism) | Killer demo | Slack thread -> agent -> trace URL inline. 250 LOC. |
| Connective tissue | 7-9 | **OTel / Langfuse** | Observability | gen_ai.* spans via `opentelemetry-otlp` directly. 200 LOC core. |
| Differentiation | 9-12 | **Sentry** | Error-to-fix loop | Demoted because Seer is partial competitor. 250 LOC via `rmcp`. |

---

## Detailed Integration Profiles

### 1. OpenTelemetry (OTLP Exporter)

**What it is**: The universal telemetry standard. GenAI semantic conventions (`gen_ai.*`)
at semconv >=1.37, actively supported by 6 vendors.

**Why it is the highest-compounding integration**: One integration unlocks six observability
backends simultaneously. No Rust equivalent of Python's `opentelemetry-instrumentation-openai`
exists -- shipping this fills a gap in the Rust ecosystem.

**Vendor support (verified April 2026)**:

| Vendor | Support Date | Mode |
|---|---|---|
| Datadog | March 2026 | Maps gen_ai schema to LLM-Obs |
| Honeycomb | March 11, 2026 | Native gen_ai attribute support |
| Langfuse | 2025+ (acquired by ClickHouse Jan 2026) | OTel-native, MIT license retained |
| Arize Phoenix | 2025+ | Elastic License 2.0 (not Apache-2.0) |
| Langtrace | 2025+ | OTel-native |
| Grafana | 2025+ | OTel-native |
| Laminar (lmnr.ai) | 2025+ | Apache-2.0, agent-first UI |

**Adapter interface**:
```rust
#[async_trait]
pub trait ObservabilityExporter: Send + Sync {
    async fn export_turn(&self, turn: &AgentTurn) -> Result<()>;
    async fn export_gate_result(&self, result: &GateResult) -> Result<()>;
    async fn export_cost_event(&self, event: &CostEvent) -> Result<()>;
    async fn flush(&self) -> Result<()>;
}
```

**Implementation**: Use `opentelemetry-otlp` directly (not the `opentelemetry-langfuse`
wrapper crate -- bus factor 1, 3 stars, breaks on every OTel minor bump). Construct basic-auth
header from env vars, target any OTLP endpoint with `http/protobuf`. Vendor-neutral by design.

**Effort**: ~200 LOC core + ~80 LOC example. MVP one week (chat span). Target two weeks (full
agent/retrieval/tool span hierarchy + token metric histogram + opt-in content capture).

**Vendor-neutral config**:
```toml
[observability]
provider = "langfuse"  # or "phoenix" | "honeycomb" | "grafana" | "laminar" | "otlp-generic"
endpoint = "https://cloud.langfuse.com/api/public/otel/v1/traces"
protocol = "http/protobuf"
auth = "basic"
semconv_opt_in = "gen_ai_latest_experimental"
```

---

### 2. GitHub (octocrab)

**Library**: `octocrab` 0.49.5 (1.2K stars, actively maintained). Covers Issues, Pulls,
Actions, GraphQL, GitHub App auth.

**Auth**: ~30-40 LOC: `OctocrabBuilder::app(app_id, key).build().installation(id)`

**Effort**: MVP 350 LOC (webhook handler + issue-to-plan + PR creation). Production 820 LOC +
16 lines TOML.

**Adapter interface**:
```rust
#[async_trait]
pub trait VersionControlAdapter: Send + Sync {
    async fn create_branch(&self, repo: &str, branch: &str, from: &str) -> Result<()>;
    async fn create_pr(&self, repo: &str, pr: &PullRequest) -> Result<PrId>;
    async fn add_review_comment(&self, pr: PrId, comment: &ReviewComment) -> Result<()>;
    async fn get_pr_diff(&self, pr: PrId) -> Result<String>;
    async fn get_ci_status(&self, pr: PrId) -> Result<CiStatus>;
    async fn merge_pr(&self, pr: PrId, strategy: MergeStrategy) -> Result<()>;
    fn capabilities(&self) -> VcsCapabilities;
}
```

**Side project**: octocrab's webhook event typing is beta. A `roko-github-webhooks` crate
with exhaustive typed events is a publishable two-week project that advertises Roko.

---

### 3. Linear AgentSession

**Why Linear is the gateway**: Cursor's Linear integration is broken (forum threads /134796,
/158505, /144750, /158866). Devin validated the magnitude (659 PRs in best week). No
Apache-2.0 Rust-native runtime competes.

**Structural moats**:
1. Linear does not bill agents as seats -- free distribution channel
2. Two simultaneous latency budgets: 5s HTTP-200 ack + 10s first activity mutation
3. No usable Rust Linear SDK on crates.io (v0.0.1 from 2022, abandoned)

**Wire protocol (corrected in R8)**:
- Webhook header: `Linear-Event: AgentSessionEvent`
- Two action values: `created` and `prompted`
- Stop signal: `prompted` with `agentActivity.signal: "stop"`
- 5 server-validated activity types: `thought`, `elicitation`, `action`, `response`, `error`
- `promptContext` is XML-formatted markup, not plain text
- HMAC-SHA256 verification over raw body, hex-encoded, `Linear-Signature` header
- `Linear-Delivery` UUID for dedup (Comment + AgentSessionEvent fire for same mention)

**Ecosystem (April 2026)**: 11+ shipped agents including Cursor (broken), Devin, OpenAI
Codex, GitHub Copilot coding agent, ChatPRD, Codegen, Sentry/Seer, Warp Oz, Factory.ai,
Charlie Labs, Reflag. Plus Linear's own first-party "Linear Agent" (public beta March 2026).

**Implementation**: `graphql_client` v0.14.0 + `reqwest`. Custom scalars module for
DateTime/JSON/UUID/TimelessDate (known issues #524, #538, #559). 10-14 working days for v1.

**Effort**: ~400 LOC for schema codegen + 5 mutations + 2 queries + webhook receiver +
HMAC verification (~120 LOC using hmac 0.12).

---

### 4. Slack (slack-morphism)

**Library**: `slack-morphism` 2.18.0 (1.84M+ downloads, 217+ stars, MIT). axum integration
via feature flag. Web API + Events API + Socket Mode + Block Kit with signature verification.

**The killer demo -- Chain D**: Slack thread -> agent -> tool use -> Slack reply with trace URL.
No competitor does this. Cursor pastes PR links. Devin posts progress. None show the human a
trace with `gen_ai.*` spans, token cost, and tool calls without leaving Slack.

**Narrative**: "Cursor and Devin show you what happened. Roko shows you why, with receipts."

**User flow**: `/roko fix #ENG-123` -> ack -> threaded progress updates -> final PR link ->
emoji-reaction approval gate -> merge.

**Socket-mode advantage**: No public webhook endpoint required. Simplifies deployment behind
firewalls.

**Effort**: ~250 LOC + 6 lines TOML.

**Compounding mechanism**: Humans approve agent actions in Slack; approval signals become
feedback data via OTel attributes on the span, closing the human-feedback loop.

---

### 5. GitHub Actions

**Adapter interface**:
```rust
#[async_trait]
pub trait CiAdapter: Send + Sync {
    async fn trigger_pipeline(&self, config: &PipelineConfig) -> Result<RunId>;
    async fn get_run_status(&self, id: RunId) -> Result<RunStatus>;
    async fn get_run_logs(&self, id: RunId, step: Option<&str>) -> Result<String>;
    async fn cancel_run(&self, id: RunId) -> Result<()>;
    fn capabilities(&self) -> CiCapabilities;
}
```

**New workflows**:
- Agent opens PR -> GitHub Actions runs roko gate pipeline -> results as PR check
- Reusable workflow: `uses: nunchi/roko-gates@v1` with configurable rungs
- CI failure -> structured error fed back to agent -> auto-fix -> re-push

---

### 6. Semgrep

**Why security gates matter now**: Post-Shai-Hulud (npm supply-chain worm Sep-Nov 2025,
Bitwarden CLI attack Apr 22-27 2026), demand for supply-chain security is no longer theoretical.
Sigstore adoption has 101M+ Rekor entries, 33K+ projects signing.

**Adapter interface**:
```rust
#[async_trait]
pub trait SecurityScanner: Send + Sync {
    async fn scan(&self, target: &ScanTarget) -> Result<Vec<Finding>>;
    async fn get_rules(&self) -> Result<Vec<Rule>>;
    fn output_format(&self) -> OutputFormat;
    fn capabilities(&self) -> ScannerCapabilities;
}
```

**New gate rung**: `security-scan` between `test` and `review`. Findings parsed from SARIF ->
injected into agent context -> agent patches -> re-scans.

---

### 7. Sentry (Seer/MCP)

**Status**: Seer Autofix relaunched January 27, 2026. $40/active-contributor/month, unlimited
usage. Ships through GitHub but does not close Linear issues and does not emit gen_ai.* spans.

**Roko's delta**: The agent's plan/tool-use itself becomes a span attached to the originating
trace ID, closing the loop with "this plan fixed this exact span." Multi-model and multi-VCS
where Seer is GitHub-only and Sentry-runtime-only.

**Effort**: ~250 LOC using `rmcp` (official Rust MCP crate).

**Closed loop**: Sentry issue -> MCP `getIssueDetails` -> Roko plan -> PR via octocrab ->
Sentry `resolveIssue` on merge.

---

### 8. Stripe

**March 2026 update**: LLM token billing is now native in Stripe. Works with AI gateways.

**Adapter interface**:
```rust
#[async_trait]
pub trait BillingAdapter: Send + Sync {
    async fn record_usage(&self, event: &UsageEvent) -> Result<()>;
    async fn get_usage_summary(&self, period: &DateRange) -> Result<UsageSummary>;
    async fn check_budget(&self, key: &str) -> Result<BudgetStatus>;
    fn capabilities(&self) -> BillingCapabilities;
}
```

---

### 9. A2A Protocol

**Status (April 2026)**: Donated to Linux Foundation. 150+ organizations, v1.0 with
Microsoft/AWS/Salesforce/SAP/ServiceNow in production. IBM ACP folded in August 2025.

**Adapter interface**:
```rust
#[async_trait]
pub trait AgentProtocolAdapter: Send + Sync {
    async fn publish_agent_card(&self) -> Result<AgentCard>;
    async fn send_task(&self, target: &str, task: &A2ATask) -> Result<TaskResult>;
    async fn receive_task(&self, task: &A2ATask) -> Result<TaskResult>;
    fn capabilities(&self) -> AgentProtocolCapabilities;
}
```

---

## Protocol Layer: MCP + A2A as Universal Connectors

**MCP (April 2026)**: 97M monthly SDK downloads, 17,468+ servers (Nerq Q1 2026 census),
AAIF governance (170+ organizations in <4 months). 52% abandonment quality crisis in registries
-- strengthens the case for roko's verification badge approach.

**A2A (April 2026)**: 150+ orgs, Linux Foundation governance since April 2026. v1.0 with
major enterprise adoption. The higher-leverage play is MCP SEP authorship, not A2A working
group attendance.

**Both together**: MCP for tool/resource integration, A2A for agent-to-agent delegation.
Roko speaks both = maximum interoperability.

---

## Five Named Integration Chains

| Chain | Flow | Named Precedent | Roko Delta |
|---|---|---|---|
| **A** | Sentry -> plan -> GitHub PR -> OTel trace -> Linear closed | Sentry Seer Autofix | Agent plan as span; closes Linear (Seer does not) |
| **B** (lead demo) | Linear webhook -> plan -> GitHub PR -> CI -> Linear status | Devin (659 PRs/week best) | Open-source, on-prem, sub-10s via Rust |
| **C** | GitHub label -> plan -> PR -> Slack approval -> merge | Sweep (7.4K stars, dormant) | Label trigger + Slack approval gate |
| **D** (killer) | Slack thread -> agent -> tool use -> Slack reply with trace URL | **No competitor ships this** | Inline observability in Slack |
| **E** | recipe.toml composition | terraform-aws-modules/eks (139.9M downloads) | One import -> 5+ adapters wired |

**Chain compounding**: A team that wires Chain B gets Chain C at near-zero marginal cost
because GitHub and CI adapters are already configured. Each chain lowers the marginal cost
of the next.

---

## Recipe Schema (recipe.toml)

```toml
[recipe]
name = "security-aware-dev-loop"
title = "Triage GitHub PRs with Semgrep findings into Linear, traced via OTel"
version = "0.1.0"
roko_compat = "^0.4"

[recipe.metadata]
capabilities = ["agent::code-review", "integration::issue-tracker", "observability::tracing"]
verified = false

[adapters]
github  = { version = "^1.2", features = ["webhooks", "pr-comments"] }
linear  = { version = "^0.5", features = ["issue-create"] }
semgrep = { version = "^0.3" }
otel    = { version = "^1.0", features = ["traces", "metrics"] }

[[parameters]]
name = "github_repo"
type = "string"
required = true

[[steps]]
id = "scan"
adapter = "semgrep"
inputs = { repo = "${{ parameters.github_repo }}", pr = "${{ event.pr_number }}" }

[telemetry]
provider = "otel"
trace_all_steps = true
```

Cargo-style `[adapters]` table with SemVer constraints -- Rust developers already know how
to read it.

---

## Integration Capability Matrix

| Integration | Pull | Push | Write | Enrich | MCP | A2A |
|---|---|---|---|---|---|---|
| OpenTelemetry | - | - | Export | - | - | - |
| GitHub | Issues | Webhooks | PRs | Code context | Existing | - |
| Linear | Issues | Webhooks | Status | Issue context | Native | - |
| GitHub Actions | Status | Webhook | Trigger | Logs | - | - |
| Semgrep | Scan | - | - | Findings | Native | - |
| Slack | History | Events | Messages | - | Existing | - |
| Stripe | Usage | Webhooks | Meter | - | - | - |
| Sentry | Issues | Webhooks | Resolve | Root cause | - | - |
| A2A | Discover | Tasks | Results | - | - | Native |
| pgvector | Query | - | Store | Similarity | - | - |

---

## Langfuse Partnership Strategy

**Status**: Langfuse acquired by ClickHouse January 16, 2026. MIT license stays. 50K obs/month
free tier stays. German GmbH + US C-corp retained. ClickHouse marketing money behind co-launches.

**Watch**: 2027-2028 re-license risk if ClickHouse pulls a re-license.

**Partnership process**: Not formal. No application. Mechanism:
1. GitHub Discussion to signal demand
2. PR to `langfuse-docs` creating integration page once code exists
3. Co-marketing via Marc Klingen (Berlin, ex-Google)

**Asset**: co-published blog post pinning `roko` + OTel + `slack-morphism` in a single recipe.
Cost: zero. Leverage: Langfuse's 50M+ SDK-installs-per-month traffic.

---

## Sources

- AI developer tools market: $6.8B in 2025, $8.5B projected 2026 (multiple research firms)
- MCP: 97M monthly SDK downloads, 17,468 servers (Nerq Q1 2026)
- A2A: 150+ orgs, Linux Foundation (April 2026)
- Linear: AgentSession protocol, 11+ shipped agents, Cursor broken (forum threads)
- Sentry Seer: $40/active-contributor/month (Jan 27, 2026 relaunch)
- Stripe: March 2026 LLM token billing update
- OTel gen_ai.*: semconv >=1.37, 6 vendor backends
- Langfuse: ClickHouse acquisition Jan 16, 2026
- n8n: 9,487 templates, 3-system minimum dominant shape
- octocrab: 0.49.5 (Aug 2025, 1.2K stars)
- slack-morphism: 2.18.0 (Feb 2026, 1.84M+ downloads)
- Sigstore: 101M+ Rekor entries, Bitwarden CLI attack Apr 22-27, 2026
