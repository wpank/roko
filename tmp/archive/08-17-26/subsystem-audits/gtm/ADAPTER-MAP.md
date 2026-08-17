# Adapter Map: Every Subsystem Generalization Opportunity

This document maps every place in roko where hardcoded behavior can be replaced with an adapter
trait, what external integrations each adapter enables, and which combinations create
multiplicative value.

Last updated: 2026-04-29. Market context reflects the April 2026 competitive landscape.

---

## Market Context: Why Adapter Architecture Matters Now

The AI developer tool market reached $6.8B in 2025 and is projected to hit $8.5B by end of
2026 (24% CAGR through 2034). Seven companies have crossed $100M ARR: Cursor ($2B+),
Devin/Windsurf (~$150M combined), Lovable, Replit, and others. But every market leader is
architecturally coupled to a single inference provider:

- **Cursor**: Anthropic Claude (primary) + OpenAI (secondary), VS Code fork
- **Codex CLI**: OpenAI only, Apache-2.0 Rust, 75K+ stars but structurally locked to OpenAI auth
- **Devin/Windsurf**: Proprietary SWE-1.5 model, closed-source orchestration
- **GitHub Copilot**: GitHub/Microsoft infrastructure, multi-model but platform-locked
- **Claude Code**: Anthropic only, terminal-based, $2.5B estimated annualized revenue

Roko's adapter architecture is the structural answer to vendor lock-in. Every subsystem
boundary is a trait -- not hardcoded to a single provider. This is not aspirational
architecture; it is the competitive moat that survives the current consolidation wave.

---

## Subsystem-by-Subsystem Adapter Opportunities

### 1. Inference Dispatch: ModelCallService + Provider

**Current state**: 13+ LLM call sites across 8 crates, 4 spawning mechanisms, 4 copies of
stream-json parsing. `ProviderAdapter` trait exists with 6 implementations but is bypassed by
bare `Command::new("claude")` in 4 paths.

**Adapter traits**:
- `Provider` (gateway): forward HTTP proxy requests (Anthropic/OpenAI wire format)
- `ProviderAdapter` (agent dispatch): spawn and manage agent processes

**Generalization**: unify all 13 call sites through `ModelCallService` which delegates to
`ProviderAdapter`. Every call goes through routing -> provider -> feedback recording.

**Why this matters in April 2026**: Cursor's $2B ARR is built on Claude dependence -- Fortune
reported in March 2026 that Cursor's "very uncertain future" stems from Anthropic controlling
the model supply chain. Codex CLI is structurally locked to OpenAI. Roko's provider-agnostic
architecture means a user can swap from Claude to GPT-5.5 to Gemini to local Ollama models
via a TOML config change. No code modification. No vendor negotiation.

**Integrations unlocked**:

| Integration | Adapter | New Workflow | Market Relevance |
|---|---|---|---|
| OpenTelemetry | `ObservabilityExporter` | Every LLM call emits `gen_ai.*` spans | 97M monthly MCP SDK downloads need observability |
| CascadeRouter | `Router` | All paths get intelligent model selection | Bandit-based routing across 20+ providers |
| Cost tracking | `CostTracker` | All paths record spend | Critical at $2B+ ARR scale |
| HuggingFace | `Provider` (HF Inference) | Route to any HF-hosted model | 2M+ models on HF Hub |
| Groq/Cerebras | `Provider` (OpenAI-compat) | Ultra-fast inference for simple tasks | Sub-100ms TTFT for routing decisions |
| Bedrock/Azure | `Provider` (enterprise) | Enterprise compliance deployment | 61% of global VC went to AI in 2025 |
| Ollama/vLLM/llama.cpp | `Provider` (local) | Air-gapped/sovereign deployment | EU sovereignty requirements post-CRA |

**Exponential combination**: `Provider` x `Router` x `ObservabilityExporter` = every request
is routed optimally, executed reliably, and measured completely. CascadeRouter learns from
OTel-exported outcomes -> routing improves -> costs drop -> more headroom for harder tasks.

**Cost impact**: HAL benchmark data (arXiv:2510.11977) shows 10-30x cost reduction from
coordination-aware scaffolding vs naive baseline. The CascadeRouter alone contributes 5-10x
by avoiding expensive models on tasks that cheaper ones can handle.

---

### 2. Gate Pipeline: GateService + GateRung + SecurityScanner

**Current state**: 3 separate gate dispatch paths (run.rs: 4 hardcoded gates, ACP: 3 hardcoded
gates, orchestrate.rs: full 7-rung pipeline). `RungDispatcher` exists but only called from
limited paths. Adaptive thresholds in ACP (3 rungs) and orchestrate.rs.

**Adapter traits**:
- `GateRung` (existing): single verification step (compile, test, clippy, etc.)
- `SecurityScanner` (new): external security tools (Semgrep, Snyk, CodeQL)
- `QualityAnalyzer` (new): external code quality tools (SonarQube, ESLint)

**Why verification is roko's strongest differentiator**: No competing agent framework has a
verification pipeline. LangGraph executes agents and hopes for the best. CrewAI has no gate
concept. AutoGen has no verification. Cursor/Codex/Claude Code generate code but have zero
automated verification before the human reviews. Roko's 7-rung gate pipeline is a category
of one.

**Generalization**: `GateService` unifies all 3 dispatch paths. New gate rungs are
TOML-configured:

```toml
[[gates.rungs]]
name = "security-scan"
kind = "external"
command = "semgrep scan --config auto --sarif"
output_format = "sarif"
on_failure = "block"
adaptive = true
```

**Integrations unlocked**:

| Integration | Adapter | New Workflow | Competitive Delta |
|---|---|---|---|
| Semgrep | `SecurityScanner` | Agent code auto-scanned before commit | 30K+ community rules, sub-second scans |
| Snyk | `SecurityScanner` | Dependency vulnerabilities caught in pipeline | Post-Shai-Hulud demand is real |
| CodeQL | `SecurityScanner` | GitHub Advanced Security findings fed to agents | Enterprise procurement unblocker |
| SonarQube | `QualityAnalyzer` | Code quality metrics as gate pass/fail | Enterprise compliance requirement |
| GitHub Actions | `CiAdapter` | Run gates in real CI environment | Reusable workflow: `uses: nunchi/roko-gates@v1` |
| Buildkite | `CiAdapter` | Enterprise CI with agent-native workflows | Goldman Sachs, Shopify use Buildkite |

**Exponential combination**: `SecurityScanner` (Semgrep) + `CiAdapter` (GitHub Actions) +
`ObservabilityExporter` (OTel) = security findings in CI -> fed to agent as structured data ->
agent fixes -> re-gates -> security posture tracked over time. This is the "validated paths,
not assertions" narrative from a16z's Aubakirova.

---

### 3. Learning & Feedback: FeedbackService + ObservabilityExporter

**Current state**: 10 learning components (episodes, CascadeRouter, efficiency, experiments,
playbooks, conductor, budget, adaptive thresholds, cost, knowledge routing). All fully built,
wired from orchestrate.rs. `roko run` records episodes + cost.

**Adapter traits**:
- `ObservabilityExporter` (new): export learning signal to external systems
- `MLAdapter` (new): export to ML platforms for analysis
- `ExperimentTracker` (new): A/B test tracking + analysis

**Why learning compounds and competitors cannot replicate it**: Aubakirova's April 2026
essay with Bornstein draws the line: "retrieval is not learning." Every competing framework
treats each run as stateless. LangGraph runs start from zero. CrewAI has no memory across
executions. Roko's 4 compounding loops -- episodes to knowledge, knowledge to routing, routing
to prompts, prompts to gate pass rates -- create a flywheel that gets better with every
execution. This is genuinely defensible for 6-12 months because it requires real production
usage data to replicate.

**Generalization**: `FeedbackService` wraps all 10 components behind a single `record()` call.
Every execution path (run, chat, ACP, plan run) calls `FeedbackService::record()`.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| OpenTelemetry | `ObservabilityExporter` | Agent performance as distributed traces |
| Langfuse | via OTel OTLP | MIT-licensed, 50K obs/month free, ClickHouse-backed since Jan 2026 |
| W&B Weave | `MLAdapter` | Visualize prompt experiments, track quality over time |
| Grafana | via OTel | Real-time dashboards of agent cost, quality, latency |
| Honeycomb | via OTel | Deep-dive query agent performance with high cardinality |
| Laminar | via OTel | Apache-2.0, agent-first UI, OTel-native |

**Vendor-neutral config knob**:
```toml
[observability]
provider = "langfuse"   # or "phoenix" | "honeycomb" | "grafana" | "laminar" | "otlp-generic"
endpoint = "https://cloud.langfuse.com/api/public/otel/v1/traces"
protocol = "http/protobuf"
```

One config block, six backends. This is adapter-trait architecture made visible to the end user.

---

### 4. Task Management: WorkSource + ProjectManagementAdapter

**Current state**: Board -> Epic -> Task hierarchy designed but not yet built as a standalone
data model. No bidirectional sync with external trackers. Task state exists only in TOML files.

**Adapter traits**:
- `WorkSource` (designed): pull/push/write/enrich work items from external trackers
- `ProjectManagementAdapter`: bidirectional issue/task sync

**Why Linear is the gateway integration**: Cursor's Linear integration is publicly broken
(forum threads /134796, /158505, /144750, /158866 document duplicate sessions, unusability,
and cross-user identity bleed as of April 2026). Devin validated the magnitude -- 659 PRs
merged in its best week. But Devin is closed-source and $500/month. No Apache-2.0 Rust-native
runtime competes in the Linear AgentSession slot.

**Structural moats**:
1. Linear does not bill agents as seats -- free distribution channel
2. Linear's 5s/10s dual latency budget favors Rust's emit-then-async pattern
3. No usable Rust Linear SDK exists on crates.io -- `roko-linear` fills a vacuum

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Linear | `WorkSource` | Issue -> plan -> execution -> PR -> issue closed |
| GitHub Issues | `WorkSource` | Issue label triggers agent dispatch |
| Jira | `WorkSource` | Enterprise issue tracking sync |
| Plane | `WorkSource` | Self-hosted PM (AGPL, no user limits) |
| Notion | `WorkSource` | Database row -> task -> execution |

**The autonomous developer loop**:
```
WorkSource (Linear) + VersionControlAdapter (GitHub) + CiAdapter (GitHub Actions)
+ NotificationAdapter (Slack) + ObservabilityExporter (OTel)
```
**Loop**: Issue -> Plan -> Code -> PR -> CI -> Merge -> Close -> Notify -> Trace URL in Slack

---

### 5. Prompt Assembly: PromptAssemblyService + ContextProvider

**Current state**: 9-layer SystemPromptBuilder exists and works, called from orchestrate.rs.
`roko "prompt"` and `roko chat` use it. ACP uses inline format strings. Some bypass paths remain.

**Adapter traits**:
- `ContextProvider` (new): pluggable context sources for prompt injection
- `PromptTemplate` (existing `RolePromptTemplate`): role-specific identity and rules

**Why context assembly is an adapter concern**: The VCG auction for context allocation is
built but the greedy path dominates at runtime. When context windows are 200K+ tokens, the
question is not "can we fit everything?" but "what is the optimal composition?" This is
Aubakirova's "multi-agent architectures as a scaling strategy for context itself."

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Code Intelligence | `ContextProvider` | Relevant code symbols injected into prompts |
| Knowledge Store | `ContextProvider` | Past learnings injected per-task |
| Playbooks | `ContextProvider` | Proven action sequences injected |
| Figma MCP | `ContextProvider` | Design tokens injected for UI tasks |
| Sentry | `ContextProvider` | Error context injected for bug-fix tasks |
| External docs | `ContextProvider` (MCP Resource) | API docs fetched and injected |

---

### 6. Version Control: VersionControlAdapter

**Current state**: `roko-mcp-github` for MCP tool access. Orchestrator creates branches and
commits via `git` commands. No PR creation, review, or merge automation natively.

**Adapter traits**:
- `VersionControlAdapter` (new): branch, PR, review, merge, CI status

**Library choice**: `octocrab` 0.49.5 (1.2K stars, actively maintained). MVP: 350 LOC
webhook handler + issue-to-plan + PR creation. Production: 820 LOC + 16 lines TOML.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| GitHub | `VersionControlAdapter` | Full PR lifecycle: create -> review -> merge |
| GitLab | `VersionControlAdapter` | MR lifecycle with GitLab CI integration |
| Bitbucket | `VersionControlAdapter` | Enterprise Atlassian stack |

---

### 7. Communication: NotificationAdapter

**Current state**: `roko-mcp-slack` and webhook reception routes exist. No structured
notification system for plan completion, gate failures, or approval requests.

**Adapter traits**:
- `NotificationAdapter` (new): message, thread, approval request

**The killer demo**: Chain D -- Slack thread -> agent -> tool use -> Slack reply with trace
URL. No competitor pastes an inline observability trace URL back into the Slack thread that
triggered the agent run. Cursor pastes PR links. Devin posts progress. None show the human a
trace with `gen_ai.*` spans, token cost, and tool calls without leaving Slack.

**Library**: `slack-morphism` 2.18.0 (1.84M+ downloads, 217+ stars, MIT). Socket-mode
operation means no public webhook endpoint required.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Slack | `NotificationAdapter` | Plan completion -> Block Kit message with PR links, cost, gates |
| Discord | `NotificationAdapter` | Community/OSS team notifications |
| Teams | `NotificationAdapter` | Enterprise Microsoft stack |
| Email | `NotificationAdapter` | Digest of agent activity |
| Webhooks | `NotificationAdapter` | Generic HTTP callbacks for any system |

---

### 8. Safety & Contracts: SafetyProvider + ContractLoader

**Current state**: `AgentContract` system with 8 bundled JSON contracts. Falls open on
missing YAML (uses `permissive()` default). `dangerously_skip_permissions` defaults to `true`.

**Adapter traits**:
- `ContractLoader` (new): load safety contracts from any source
- `SafetyProvider` (new): pluggable safety checks (PII, injection, compliance)

**EU AI Act context**: Article 50 enforcement begins August 2, 2026. Signed gate results =
compliance artifacts. This is the Vanta playbook ($220M ARR at $4.15B valuation, built on
SOC 2 enforcement timing) applied to AI agent audit trails.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Auth0/WorkOS | `AuthProvider` | Agent identity with scoped permissions |
| EU AI Act compliance | `ContractLoader` | Article 50 transparency contracts |
| Sigstore/in-toto | `SafetyProvider` | Agent-action-time verification (not just build-time) |
| PII scanners | `SafetyProvider` | External PII detection services |

---

### 9. Knowledge & Storage: KnowledgeBackend + StorageAdapter

**Current state**: neuro knowledge store uses 4-tier file-based storage. Episode logs are
append-only JSONL. Cost data in SQLite (gateway) or in-memory (runtime).

**Adapter traits**:
- `KnowledgeBackend` (new): store/query/delete with vector similarity
- `StorageAdapter` (new): generic key-value + append-only log persistence

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| PostgreSQL + pgvector | `KnowledgeBackend` | Shared state across roko instances |
| Qdrant | `KnowledgeBackend` | High-performance vector search |
| Redis | `StorageAdapter` | Fast caching, session state |
| S3/R2 | `StorageAdapter` | Artifact archival (episodes, plan snapshots) |
| DuckDB | `StorageAdapter` | Analytics queries on cost/learning data |

---

### 10. Deployment: DeployAdapter

**Current state**: `roko deploy railway/fly/docker` exists in CLI. Daemon lifecycle
management built. No programmatic deployment from the orchestrator.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Railway | `DeployAdapter` | Auto-deploy on plan completion |
| Fly.io | `DeployAdapter` | Edge deployment for gateway |
| Vercel | `DeployAdapter` | Web dashboard deployment |
| AWS Lambda | `DeployAdapter` | Serverless agent execution |
| Docker | `DeployAdapter` | Container image build + push |

---

### 11. Billing & Payments: BillingProvider + PaymentGateway

**Current state**: cost tracking exists (in-memory CostTable + BudgetGuardrail). Gateway
design includes Stripe + MPP. No actual billing integration wired.

**Integrations unlocked**:

| Integration | Adapter | New Workflow |
|---|---|---|
| Stripe | `BillingProvider` | Usage-based billing (March 2026 LLM token billing native) |
| x402/USDC | `PaymentGateway` | Agents pay for external APIs autonomously |
| Credits system | `BillingProvider` | Pre-paid credit balance with burn-down |

---

## Multiplicative Combination Matrix

The highest-value combinations of adapters:

### "The Autonomous Developer" (P0 combination)
```
WorkSource (Linear) + VersionControlAdapter (GitHub) + CiAdapter (GitHub Actions)
+ NotificationAdapter (Slack) + ObservabilityExporter (OTel)
```
**Loop**: Issue -> Plan -> Code -> PR -> CI -> Merge -> Close -> Notify -> Trace URL
**Competitive delta**: Devin does this closed-source at $500/month. Roko does it open-source,
on-prem, multi-model, with full `gen_ai.*` observability.

### "The Security Pipeline" (P1 combination)
```
SecurityScanner (Semgrep) + ErrorFeedback (Sentry) + CiAdapter (GitHub Actions)
+ VersionControlAdapter (GitHub)
```
**Loop**: Code -> Scan -> Fix -> CI -> Ship -> Error -> Auto-fix PR
**Market signal**: Post-Shai-Hulud (Bitwarden CLI attack April 22-27, 2026), demand for
supply-chain security is no longer theoretical.

### "The Knowledge Flywheel" (P1 combination)
```
KnowledgeBackend (pgvector) + MLAdapter (Langfuse) + FeedbackService
```
**Loop**: Execute -> Learn -> Store -> Retrieve -> Execute better -> Learn more
**Competitive delta**: "retrieval is not learning" -- Aubakirova's exact thesis.

### "The Billable Platform" (P2 combination)
```
BillingProvider (Stripe) + ObservabilityExporter (OTel) + PaymentGateway (x402)
```
**Loop**: Usage -> Meter -> Bill -> Agents pay for APIs autonomously

### "The Design-to-Deploy Pipeline" (P3 combination)
```
DesignAdapter (Figma MCP) + VersionControlAdapter (GitHub)
+ DeployAdapter (Vercel) + CiAdapter (GitHub Actions)
```
**Loop**: Design -> Code -> PR -> CI -> Deploy -> Preview

---

## The Contrast Frame: Roko vs Every Competitor

| Axis | Cursor | Codex CLI | Devin | Claude Code | **Roko** |
|---|---|---|---|---|---|
| License | Proprietary | Apache-2.0 | Proprietary | Proprietary | Apache-2.0 |
| Language | TypeScript | ~95% Rust | Python | TypeScript | 100% Rust (18 crates, 177K LOC) |
| Models | Claude primary | OpenAI only | Proprietary SWE-1.5 | Claude only | Any (20+ providers via adapter trait) |
| Verification | None | None | None | None | 7-rung gate pipeline with adaptive thresholds |
| Learning | None | None | Unknown | None | 4 compounding loops (episodes, routing, prompts, gates) |
| Observability | None | None | None | None | Vendor-neutral `gen_ai.*` OTel |
| Integrations | IDE-embedded | CLI-only | Linear+GitHub (closed) | Terminal-only | Adapter traits: Linear, Slack, GitHub, Sentry, OTel |
| Self-hosting | No | Partial | No | No | Full: PRD -> plan -> execute -> gate -> learn -> iterate |
| Price | $20-400/mo | Pay-per-token | $500/mo | $20-200/mo | Open-source + support tiers |
| Valuation | $29-60B | N/A (OpenAI) | $10-25B | N/A (Anthropic) | Pre-revenue |

---

## Sources

- AI developer tools market: Virtue Market Research ($4.5B -> $10B by 2030, 17.32% CAGR)
- Cursor: Reuters ($2B ARR Feb 2026), Fortune (uncertain future, Mar 2026), CNBC ($29.3B valuation)
- Codex CLI: GitHub (75K+ stars, 640+ releases, 3M weekly active users)
- Devin/Cognition: SiliconANGLE ($25B valuation talks Apr 2026), TechCrunch ($400M at $10.2B Sep 2025)
- Claude Code: FindSkill.ai ($2.5B estimated annualized revenue), Anthropic ($14B ARR early 2026)
- MCP: 97M monthly SDK downloads, 17,468 servers indexed (Nerq Q1 2026 census)
- AI VC: OECD (61% of global VC = $258.7B went to AI in 2025)
- HAL benchmark: arXiv:2510.11977, 10-30x cost reduction from coordination-aware scaffolding
- Subsystem audit analysis: 10 AUDIT.md files across all roko subsystems
