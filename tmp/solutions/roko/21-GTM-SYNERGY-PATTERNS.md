# Synergy & Composability Patterns

21 patterns for how roko's components create value greater than the sum of their parts.
Each pattern is grounded in quantitative evidence from real platforms, with concrete
application to roko's architecture and prioritized by implementation feasibility. Updated
with April 2026 competitive landscape data and roko-specific differentiation analysis.

Last updated: 2026-04-29.

---

## April 2026 Market Context: Why Synergies Matter

The AI developer tools market ($6.8B in 2025, projected $8.5B in 2026) is consolidating
around two architectures: monolithic bundles (Cursor, Devin, Windsurf) and composable
platforms (roko, LangGraph, open-source agent frameworks). The synergy patterns below
explain why composable platforms win in the long run -- each component makes every other
component more valuable, creating compound returns that monolithic architectures cannot match.

| Product | Architecture | Synergy Ceiling |
|---|---|---|
| **Cursor** ($2B ARR) | Monolithic IDE fork | Components are tightly coupled. Adding a new feature requires modifying the monolith. |
| **Codex CLI** (75K stars) | Monolithic binary | Single-purpose tool. No subsystem boundaries = no composition surface. |
| **Devin** ($25B val talks) | Closed-source cloud | Proprietary everything. No user-extensible composition. |
| **Claude Code** ($2.5B est.) | Anthropic-only CLI | Single-vendor. MCP support adds tools but no learning/routing/gating composition. |
| **Roko** (18 crates) | Adapter-trait composition | Every trait boundary is a composition surface. 11 adapter categories with 3+ impls each = 177K+ possible configurations. |

---

## Pattern 1: Network Effects in Developer Tools

**Evidence**: NFX research shows 70% of tech company value comes from network effects.
GitHub (100M -> 180M+ developers 2023-2026), npm (2.1M -> 3M+ packages 2023-2026),
VS Code (marketplace drives 80% of stickiness). MCP reached 97M monthly SDK downloads
and 17,468+ servers in 16 months -- the fastest protocol adoption in developer tools history.

**Types of network effects in roko**:

| Type | Mechanism | Example |
|---|---|---|
| **Direct** | More users -> more shared knowledge | 100 roko instances -> 100x routing data |
| **Indirect** | More users -> more adapters -> more users | Linear adapter -> PM teams adopt -> more adapters built |
| **Data** | More usage -> better models -> better product | CascadeRouter improves from every model call |
| **Platform** | More adapters -> more integrations -> more workflows | GitHub + Linear + Slack = autonomous developer loop |

**Roko application**:

The federated learning pattern (ADVANCED-PATTERNS.md Pattern 12) is the primary network effect
mechanism. Every roko instance contributes to collective intelligence:

```
Instance count    | Federated benefit
1                 | None (local learning only)
10                | Routing data covers 10x more task types
100               | Statistical significance for model routing
1,000             | Near-optimal routing for all common task types
10,000            | Tail-case coverage, rare language/framework data
```

**Competitive delta**: Cursor's 250K+ paying users generate data that stays inside
Anysphere's servers. Codex CLI's 3M WAU generate data that stays inside OpenAI. Roko's
federated learning means the data network effect is user-owned and composable.

**Priority**: P1 -- federated learning infrastructure enables the most valuable network effect.

---

## Pattern 2: Composable Architecture (MACH Pattern)

**Evidence**: Microservices + API-first + Cloud-native + Headless. Shopify (composable
commerce, $7.7B revenue). Contentful (headless CMS, $200M+ ARR). Stripe (API-first,
embedded in 3.1M sites).

**The MACH formula applied to roko**:

K traits x N implementations each = K^N possible configurations.

Roko's 6 core verb traits (Substrate, Scorer, Gate, Router, Composer, Policy) with
even 3 implementations each = 729 possible configurations. With the 11 adapter categories
from ADAPTER-MAP.md:

```
11 adapter categories x 3 avg implementations = 177,147 possible configurations
```

**The power of composability**:
- Customer A: Linear + GitHub + Semgrep + Slack + OTel = security-focused dev team
- Customer B: Jira + GitLab + SonarQube + Teams + Datadog = enterprise Java team
- Customer C: Plane + Gitea + custom scanner + Discord + self-hosted = open-source project
- Same roko, different adapter configs, three different products.

**Profile-based configuration bundles**:
```toml
# profiles/security-team.toml
[adapters]
work_source = "github-security-alerts"
vcs = "github"
ci = "github-actions"
scanner = ["semgrep", "snyk", "codeql"]
notification = "slack"
observability = "datadog"
gates = ["compile", "test", "security-scan", "license-check"]
```

**Competitive delta**: Cursor is one product, one configuration. Codex CLI is one tool,
one model vendor. Roko is 177K+ configurations from the same codebase -- each customer
gets a product tailored to their stack via TOML config, not code changes.

**Priority**: P0 -- the adapter trait system IS composable architecture. Implementing
clean adapter interfaces directly enables this pattern.

---

## Pattern 3: Platform Engineering & Internal Developer Platforms

**Evidence**: Gartner predicts 80% of large engineering orgs will have platform engineering
teams by 2026. Backstage (Spotify -> CNCF, 3,400+ orgs, 89% developer portal market share,
200+ plugins). Port, Cortex, Humanitec all growing 100%+ YoY.

**The platform engineering thesis**: developer self-service reduces cognitive load.
Instead of learning 15 tools, developers interact with one platform that orchestrates
everything behind the scenes.

**Roko as IDP backend**:

```
Developer -> roko -> [ GitHub, CI, Security, Observability, Deploy ]
                     All behind adapter traits
                     All configured per-team
                     All observable via OTel
```

| Platform Engineering Need | roko Feature |
|---|---|
| Service scaffolding | Plan generation from PRD |
| CI/CD orchestration | Gate pipeline + CI adapter |
| Security scanning | Security scanner adapter |
| Observability setup | OTel exporter adapter (gen_ai.* spans) |
| Documentation | Auto-generated from code |
| Compliance checks | Policy adapter + gate rungs |
| Cost management | CascadeRouter + budget guards |

**Roko application**: Position roko as the "agent-powered IDP." Unlike static platforms
(Backstage = dashboard + links), roko actively does the work. The platform doesn't just
show you what to do -- it does it for you, with verification via 7-rung gate pipeline.

**Competitive delta**: Backstage is a portal (read-only). Roko is an executor (read-write).
The difference is between "here's a link to your CI" and "I ran CI, here are the results,
and I fixed the failures."

**Priority**: P2 -- requires adapter infrastructure (P0) + several integrations (P1) first.

---

## Pattern 4: The API Economy

**Evidence**: API economy $45.3B by 2033 (Grand View Research). Twilio, Stripe, Plaid,
SendGrid -- all API-first companies. Postman: 30M+ developers, 500K+ API collections.

**Every roko adapter is a potential API**:

The adapter-first architecture means every roko capability is accessible via API:

| roko Capability | API Surface | External Value |
|---|---|---|
| Gate pipeline | `POST /api/v1/gate/run` | Any CI can use roko's verification |
| Code review | `POST /api/v1/review` | Review-as-a-service |
| Plan generation | `POST /api/v1/plan/generate` | Planning-as-a-service |
| Model routing | `POST /api/v1/route` | Routing-as-a-service |
| Knowledge query | `POST /api/v1/knowledge/query` | Knowledge-as-a-service |
| Cost estimation | `POST /api/v1/simulate` | Prediction-as-a-service |

**The API flywheel**:
1. roko exposes capabilities as APIs (~85 routes already exist in roko-serve)
2. External tools integrate with roko APIs
3. External tools become roko adapters (bidirectional via MCP dual-exposure)
4. More integrations -> more data -> better roko -> more integrations

**Competitive delta**: Cursor has no API. Codex CLI has no server mode. Devin has a
proprietary API. Roko-serve already exposes ~85 routes and can add OpenAPI spec generation
for self-documenting, discoverable APIs.

**Priority**: P1 -- the routes exist, the adapter architecture enables it. Need OpenAPI
spec generation + authentication.

---

## Pattern 5: Marketplace Dynamics

**Evidence**: Agent marketplace projected $52.62B by 2030 (Precedence Research).
Terraform Registry: 3,500+ providers, 800% growth for top integrations in single
years. npm: 3M+ packages, 200B+ downloads/month. VS Code: tens of thousands of
extensions, marketplace drives adoption. MCP: 17,468+ servers in registries, but
52% abandonment rate (Rapid Claw audit) -- quality, not quantity, is the bottleneck.

**Roko marketplace tiers**:

| Tier | What | Trust Level | Revenue |
|---|---|---|---|
| 1. Prompts | Markdown + TOML front-matter | View source | Free / tip |
| 2. Config profiles | TOML bundles (roles, workflows, gates) | View source | Free / paid |
| 3. Declarative tools | `connector.toml` manifests | View source | Free / paid |
| 4. WASM plugins | Sandboxed computation | Capability-gated | Paid |
| 5. Native Rust | Full performance | In-tree only | Enterprise |

**The critical insight**: trust by evidence, not authority. Unlike app stores
(trust by review), roko can use its own gate pipeline to verify marketplace
submissions. A role config's effectiveness is measured by real gate pass rates.
This directly addresses the MCP quality crisis -- roko positions as the quality
layer for the agent ecosystem.

**Marketplace revenue model**: Follow GitHub Actions' model (0% revenue share, pure
distribution-as-marketing). GitHub Actions: 5M workflow runs/day, zero revenue share,
enormous impression count. Vendors publish adapters for marketing alone if publishing
is trivial. Roko should run the same play.

**Priority**: P3 -- requires adapter infrastructure + federated learning + significant
user base first. But design marketplace-ready interfaces from P0.

---

## Pattern 6: Compounding Learning Systems

**Evidence**: Cursor ($100M -> $2B ARR in 14 months, driven by learning from usage).
Tesla Autopilot (network learning from fleet). Google Search (20+ years of ranking
signal accumulation). AlphaFold (each protein structure improves the model).

**Roko's 4 compounding loops** (all built and wired):

```
Loop 1: Task Learning
  Agent executes -> episode recorded -> similar future tasks benefit
  Compounds: logarithmically (each new episode adds less, but never zero)
  Storage: .roko/episodes.jsonl

Loop 2: Model Routing
  CascadeRouter observes outcomes -> bandit updates -> better model selection
  Compounds: linearly until convergence, then logarithmically
  Storage: .roko/learn/cascade-router.json

Loop 3: Prompt Optimization
  Section effectiveness scored -> budget adjusted -> better prompts -> better outcomes
  Compounds: linearly (each experiment improves allocation)
  Storage: .roko/learn/experiments.json

Loop 4: Gate Calibration
  Adaptive thresholds -> fewer false positives -> faster pipeline -> more tasks -> more data
  Compounds: exponentially (faster pipeline -> more tasks -> more calibration data)
  Storage: .roko/learn/gate-thresholds.json
```

**Cross-loop synergy**:
- Loop 1 feeds Loop 2 (episode outcomes -> routing signals)
- Loop 2 feeds Loop 3 (better models -> better prompt experiments)
- Loop 3 feeds Loop 4 (better prompts -> higher first-pass gate rates)
- Loop 4 feeds Loop 1 (faster gates -> more tasks completed -> more episodes)

**The flywheel speed indicator**: the ratio of successful first-pass gate completions.
When this ratio increases, all four loops accelerate simultaneously.

**Competitive delta**: No competitor has compounding learning loops. Cursor learns from
aggregate usage data (controlled by Anysphere). Codex CLI learns nothing between sessions.
Claude Code learns nothing between invocations. Roko's 4 loops are the strongest
compounding advantage in the market.

**Priority**: P0 -- already implemented. Focus on measuring and visualizing the
compound effect to demonstrate value.

---

## Pattern 7: Interoperability as Moat

**Evidence**: Linux (kernel interfaces are the moat -- 90%+ server market share).
Kubernetes (API compatibility is the moat -- every cloud implements it). Stripe
(API stability is the moat -- switching cost increases with integration depth).

**roko's interoperability surface**:

| Protocol | Direction | What It Connects | Status |
|---|---|---|---|
| MCP | Both (expose + consume) | Tools, resources, context from any MCP server | 97M monthly SDK downloads |
| A2A | Both (expose + consume) | Cross-framework agent collaboration | 150+ organizations, Linux Foundation |
| ACP | Expose | Editor integration (Cursor, VS Code, etc.) | Partial |
| OpenAPI | Expose | Any HTTP client | ~85 routes in roko-serve |
| OTel | Export | Any observability backend | gen_ai.* semconv >=1.37 |
| Webhooks | Both | Any event-driven system | Linear AgentSession, GitHub, Slack |

**The interoperability thesis**: roko doesn't need to build everything. By speaking
every protocol, roko becomes the orchestration layer that connects everything.

```
External MCP servers -> roko (orchestration) -> External A2A agents
External webhooks    ->                       -> External APIs
External CI          ->                       -> External deploy targets
```

**Critical insight**: MCP + A2A together make roko a universal agent connector.
MCP handles tool/resource integration. A2A handles agent-to-agent delegation.
roko sits in the middle, providing the plan-execute-verify loop that neither
protocol provides on its own.

**Competitive delta**: Cursor speaks ACP only. Codex CLI speaks OpenAI API only.
Claude Code speaks MCP. Roko speaks all of them -- the universal adapter.

**Priority**: P1 -- MCP infrastructure exists. A2A is new but maps to existing
JSON-RPC patterns. Being early to A2A creates first-mover advantage.

---

## Pattern 8: Unbundling/Rebundling Cycle

**Evidence**: Jim Barksdale: "There are only two ways to make money in business:
bundling and unbundling." Craigslist unbundled -> Zillow, Indeed, Tinder.
Microsoft Office bundled -> Google unbundled -> Microsoft rebundled (365).

**roko's unbundling opportunity**:

Current AI coding tools are bundled monoliths (Cursor, Windsurf, Devin). They
bundle: editor, AI, deployment, testing, review into one product. This creates
lock-in but also rigidity -- you cannot use Cursor's code review without using
Cursor's editor.

**roko's position**: unbundled components that CAN be rebundled:

| Component | Standalone Value | Bundled Value |
|---|---|---|
| Gateway | LLM proxy (OpenRouter competitor) | Embedded in roko-serve, feeds learning |
| Gate pipeline | CI verification service | Embedded in orchestrator, adaptive thresholds |
| Code review | Review bot for GitHub | Embedded in workflow, feeds episodes |
| Model routing | Cost optimization service | Embedded in dispatch, bandit-driven |
| Knowledge store | Knowledge base product | Embedded in prompts, tier progression |

**The adoption sequence**:
```
Stage 1: Use roko-gateway as LLM proxy (standalone, immediate value)
Stage 2: Add gate pipeline to CI (standalone, verification value)
Stage 3: Use roko for plan execution (bundled, orchestration value)
Stage 4: Full self-hosting workflow (fully bundled, compound value)
```

Each stage has independent value. Each stage makes the next stage lower-friction.
By Stage 3, the switching cost is significant. By Stage 4, it's prohibitive.

**Competitive delta**: Cursor bundles everything -> lock-in but rigidity. Roko
unbundles into composable pieces -> each piece lands independently, rebundles
naturally as usage grows.

**Priority**: P1 -- the crate structure already supports this. Gateway is designed
as both standalone and embedded. Gate pipeline can be extracted similarly.

---

## Pattern 9: Developer Experience as Growth Engine

**Evidence**: Stripe (7-minute integration), Vercel (zero-config deploy), Railway
(infrastructure from Git push). Time-to-first-value is the #1 predictor of
developer tool adoption. Stripe grew 60% YoY by obsessing over DX.

**roko's time-to-first-value**:

| Action | Current | Target | How |
|---|---|---|---|
| Install | `cargo install roko-cli` | Same | Already works |
| Initialize | `roko init` | Same | Already works |
| First run | `roko run "fix the typo"` | Same | Already works |
| First plan | `roko prd idea "..." && roko prd plan ...` | `roko plan "..."` | One command |
| First gate | Implicit (in plan run) | `roko gate run` | Standalone gate |
| First dashboard | `roko dashboard` | Same | Already works |
| First integration | Edit roko.toml + restart | `roko connect linear` | CLI wizard |

**Activation keystone** (Supabase model): The activation event is "first successful
agent invocation that hit a real model API and returned a trace" -- not `cargo install`,
not signup, not first adapter scaffolded. All Day 2-7 nudges tie to this single event.
Supabase re-oriented all funnel metrics around initialization as the single activation
milestone and grew to 4.5M developers and $5B valuation (Series E, Oct 2025).

**Scaffolding benchmarks** (time-to-first-adapter):

| Platform | No-code | Low-code | Full-code |
|---|---|---|---|
| Airbyte CDK | <10 min | <30 min | ~3 hours |
| Backstage | N/A | `yarn backstage-cli new` | ~1 hour |
| **Roko target** | connector.toml (<10 min) | `cargo generate` (<15 min) | Full trait (<1 hour) |

**Competitive delta**: Codex CLI has zero-config UX (great). Cursor has IDE UX (great).
But neither has configurable workflows. Roko must match their initial UX simplicity while
offering the configuration depth that power users need. The `roko connect <service>`
wizard is the bridge.

**Priority**: P1 -- DX improvements have immediate ROI. Every minute saved in setup
= more users complete onboarding = more adoption = more data = better product.

---

## Pattern 10: Emergent Behaviors from Simple Rules

**Evidence**: Conway's Game of Life (4 rules -> Turing-complete). Ant colonies
(3 pheromone rules -> optimal foraging). Bitcoin (simple consensus rules -> global
financial network). Stigmergy: indirect coordination through environment modification.

**roko's emergent behaviors**:

The signal system is designed for emergence:
```
Signal = { kind, payload, hash, parent_hash, timestamp }
```

Simple rules that create complex behaviors:
1. **Signal decay**: old signals lose weight -> system forgets what's no longer relevant
2. **Signal reinforcement**: signals that lead to gate passes get amplified
3. **Signal composition**: signals with shared parents cluster -> emergent categories
4. **Cross-agent signaling**: Agent A's output signal becomes Agent B's input

**What emerges**:
- Agents naturally specialize in what they're good at (via routing feedback from
  CascadeRouter bandit arms)
- Workflows self-optimize (steps that don't help get skipped via adaptive thresholds)
- Knowledge self-organizes (tier progression from working -> episodic -> semantic -> durable
  via roko-neuro)
- Tool preferences emerge (tool usage correlates with success -> more usage)

**The stigmergy pattern**: agents don't communicate directly -- they modify the
shared environment (signals, episodes, knowledge store), and other agents react
to those modifications. This is exactly how ant colonies achieve optimal foraging
without central coordination.

**Competitive delta**: No competing product has emergent self-optimization. Cursor's
behavior is deterministic per configuration. Codex CLI behaves identically on every
run. Roko gets better with use because simple rules produce emergent optimization.

**Priority**: P0 -- the signal system exists. The learning loops exist. The emergent
behaviors are already happening. Focus: make emergence visible in the dashboard so
users trust the self-optimization.

---

## Pattern 11: The Data Flywheel

**Evidence**: Google Search (data -> better rankings -> more users -> more data).
Netflix (viewing data -> better recommendations -> more viewing -> more data).
Amazon (purchase data -> better suggestions -> more purchases -> more data).
Tesla (driving data -> better autopilot -> more customers -> more driving data).

**roko's 4 interconnected data flywheels**:

```
Flywheel 1: Episode -> Knowledge
  More tasks -> more episodes -> more knowledge -> better prompts -> better task outcomes
  -> more tasks (because users trust roko more)

Flywheel 2: Gate -> Routing
  More gate results -> better threshold calibration -> fewer false positives ->
  faster pipeline -> more tasks completed -> more gate results

Flywheel 3: Routing -> Cost
  More routing observations -> better model selection -> lower cost per task ->
  higher ROI -> more tasks assigned -> more routing observations

Flywheel 4: Community -> Adapters
  More users -> more adapter contributions -> more integrations -> more use cases ->
  more users
```

**Cross-flywheel connections**:
- Flywheel 1 (episodes) feeds Flywheel 2 (gate calibration uses episode data)
- Flywheel 2 (gates) feeds Flywheel 3 (gate results inform routing decisions)
- Flywheel 3 (routing) feeds Flywheel 1 (better routing -> better outcomes -> richer episodes)
- Flywheel 4 (community) amplifies all other flywheels (more adapters -> more data sources)

**Cost flywheel specifics**: Gemini 2.0 Flash is $0.10/$0.40 per 1M tokens. Claude
Sonnet 4.6 is $3/$15. GPT-5.5 is $5/$30. CascadeRouter learns which tasks can use cheap
models without quality degradation. Over time, the average cost per task drops while quality
stays constant or improves. This is a 30-150x cost spread that routing intelligence captures.

**Competitive delta**: Cursor users pay the same per-token rate whether the task is trivial
or complex. Roko's routing flywheel means trivial tasks route to cheap models automatically.

**Priority**: P0 for Flywheels 1-3 (already operational). P2 for Flywheel 4
(requires marketplace infrastructure).

---

## Pattern 12: Standards vs Innovation Tension

**Evidence**: Rust editions (stable foundation + opt-in evolution). Web standards
(HTML5 took 10 years but won). USB-C (standard connector, diverse implementations).
TCP/IP (stable protocol, infinite applications).

**roko's stability/innovation split**:

| Layer | Stability Requirement | Innovation Speed |
|---|---|---|
| Signal format | Very stable (breaking = data loss) | Slow (semver major) |
| Adapter traits | Stable (breaking = ecosystem disruption) | Medium (semver minor) |
| Core loop | Stable (users depend on it) | Medium |
| Adapter implementations | Unstable OK (isolated) | Fast |
| Prompt templates | Unstable OK (A/B tested) | Very fast |
| Learning parameters | Unstable OK (self-adjusting) | Continuous |

**The critical insight**: stability at the trait level enables innovation at the
implementation level. Users don't care how CascadeRouter works internally -- they
care that the `Router` trait interface doesn't change. This is why the <=5 methods
rule matters -- small interfaces are easier to keep stable.

**Versioning strategy**: Rust editions model. Core traits are versioned. Adapters
declare which trait version they implement. Multiple versions can coexist. The
Schema Registry pattern (ADVANCED-PATTERNS.md Pattern 5) provides the mechanism.
crates.io trusted publishing (RFC 3691, shipped July 2025) provides the distribution.

**Competitive delta**: Cursor's API changes break extensions without warning. MCP's
rapid evolution caused 52% server abandonment. Roko's stability guarantee makes it
safe to build on.

**Priority**: P1 -- version the adapter traits from the start. It's much harder to
add versioning retroactively.

---

## Pattern 13: Cursor Unbundling Thesis -- Three Standalone Products

**Evidence**: Cursor crossed $2B ARR by February 2026 (fastest SaaS to $100M ARR ever,
then fastest to $1B). But the architecture is single-vendor (Anthropic supply chain risk
per Fortune), opaque routing, and weak gating. Reddit /r/cursor (~80k members) and Cursor
Forum (~150k posts) surface three recurring pain points.

**The three unbundled products**:

| Product | Cursor Pain Point | Roko Component | Buyer |
|---|---|---|---|
| **Model Router** | Cursor's auto-mode is opaque ("why did it pick GPT here?") | CascadeRouter (auditable, bandit-driven) | Any team using multiple LLMs |
| **Gate Pipeline** | Cursor's review feature weak ("PRs look right but don't compile") | 7-rung gate pipeline as CI service | Teams wanting pre-commit verification |
| **Cost Optimizer** | Forum full of "burned $30 in 2 hours" posts | Caching + routing + budget guard | Any team with LLM cost concerns |

**Why this pattern works**:
- Roko doesn't compete with Cursor head-on (different buyer, different price point).
- Each standalone product has independent value.
- Each standalone product is an entry point to the full platform (land -> expand).

**The entry sequence**:
```
Stage 1: Team adopts gate pipeline as CI service (standalone)
Stage 2: Team adds model router for cost optimization (standalone)
Stage 3: Team connects both to roko orchestration (bundled)
Stage 4: Team moves to full plan-execute-verify workflow (fully bundled)
```

**Pitch utility**:
> "Cursor is at $2B ARR but their architecture is single-vendor, opaque routing, and
> weak gating. We don't compete with Cursor -- we sell the three pieces of Cursor that
> the developers themselves are asking for as standalone tools: the router, the gate
> pipeline, and the cost optimizer."

**Priority**: P1 -- the crate structure already supports this. Gateway is designed as
both standalone and embedded. Gate pipeline can be extracted similarly.

---

## Pattern 14: Continuous Compliance Attestation -- Gate Pipeline as Product

**Evidence**: Vanta hit $220M ARR at $4.15B valuation (TechCrunch, July 2025), built
entirely on SOC 2 enforcement timing. OneTrust ($5.3B last private round) was built on
GDPR enforcement timing. EU AI Act Article 50 enforcement begins August 2, 2026 --
14 weeks from the research date.

**The novel workflow**: Today, SOC 2 / ISO 27001 / EU AI Act compliance is a point-in-time
audit (annual or biennial). The novel pattern: **continuous attestation as a stream**,
where every agent action is gated, signed, and emitted as a real-time compliance event.

**Why this is genuinely novel**:
- **Vanta/OneTrust** ship "evidence collection" -- they pull data into their platform.
- **Roko** ships "evidence emission" -- the runtime IS the evidence stream.
- The buyer (Chief Risk Officer) gets a real-time compliance dashboard, not a quarterly report.

**Roko's unique position**:
- The gate pipeline already produces structured verification results for every agent action.
- Adding cryptographic signing (via in-toto attestations, ~100 LOC adapter) turns gate
  results into compliance attestations.
- The compliance buyer pays for **the stream of signed gate results**, not for the agent
  execution. The gate pipeline IS the product, not QA overhead.

**Revenue model shift**:
```
Traditional: Pay for agent -> gates are QA overhead
Attestation: Pay for verified gate stream -> agent execution is the delivery mechanism
```

**Market timing**: The Vanta playbook on Article 50 -- regulatory deadline + 18-24 month
build window = $3-6B outcome. The window is closing fast.

**Priority**: P1 -- requires gate pipeline to emit structured compliance events (close
to existing gate output format) + signing infrastructure. The gate pipeline already exists;
the compliance framing is positioning, not engineering.

---

## Pattern 15: Bandit-Driven Prompt Experiment Promotion -- Production IS the Experiment

**Evidence**: Braintrust ($300M Series B, Casado-led, May 2025), PromptLayer, LangSmith --
all separate eval from production. A team evaluates prompt variants offline, picks a winner,
deploys it. This creates a gap: offline eval performance does not match production
performance, and the handoff is manual.

**The novel pattern**: A team has 5 candidate prompt variants. They run each through Roko's
CascadeRouter for 1,000 invocations. The bandit (Thompson sampling) automatically promotes
the winner. **Production routing IS the experiment**, and convergence to the optimal
variant happens online without a separate eval framework.

**Why this is genuinely novel**:
- **Braintrust** separates eval from production.
- **PromptLayer** separates eval from production.
- **LangSmith** separates eval from production.
- **Roko** unifies them. The ExperimentStore + CascadeRouter already implement this --
  the bandit allocates traffic across prompt variants and converges to the best performer
  based on real gate pass rates, not offline metrics.

**Technical requirements** (all already built):
- `ExperimentStore` (`.roko/learn/experiments.json`) -- stores prompt variants with
  performance data
- `CascadeRouter` (`.roko/learn/cascade-router.json`) -- implements bandit routing
  across model/prompt combinations
- Adaptive gate thresholds (`.roko/learn/gate-thresholds.json`) -- adjusts gate
  sensitivity based on historical performance

**The workflow-embedding moat**:
Once a team is using bandit-promoted prompts in production, leaving requires both
rebuilding eval AND rebuilding routing. This is compound lock-in -- two systems
that must be replaced simultaneously.

**Pitch utility**:
> "Every eval platform separates testing from production. We don't. Production routing
> IS the experiment -- the bandit converges to the winning prompt variant online. Once
> a team is using bandit-promoted prompts in production, leaving means rebuilding both
> eval and routing simultaneously."

**Priority**: P0 -- the infrastructure is already built and wired. The differentiator
is framing and marketing, not engineering.

---

## Pattern 16: Recipe Compression (1 Import -> 5+ Adapters)

**Evidence**: Terraform modules outnumber providers 4:1 (14,000+ modules vs ~3,500
providers). The terraform-aws-modules/eks module has 96.3M+ all-time downloads and
internally calls 30+ resources from `hashicorp/aws`, `hashicorp/kubernetes`, and
`hashicorp/helm`. One HCL block; behind it sits four providers and 200 resources.

n8n has 9,487 published templates (April 2026) using verb-first naming convention.
Top templates: Personalized LinkedIn outreach (323 uses), QuickBooks-from-Sheets (315),
QuickBooks-receipts-from-Stripe (291). Each chains 3+ systems.

**The compression ratio is the mechanism**: 1 import -> 5+ adapters wired correctly.

**Roko application**:

```toml
# recipes/code-fix-loop/recipe.toml
[recipe]
name = "code-fix-loop"
title = "Canonical code-fix loop with trigger variants"
version = "0.1.0"

[sub-recipes]
sentry-trigger = "recipes/code-fix-loop/sentry-trigger.toml"
linear-trigger = "recipes/code-fix-loop/linear-trigger.toml"
github-label-trigger = "recipes/code-fix-loop/github-label-trigger.toml"

[shared-steps]
plan = { adapter = "planner", version = "^0.1" }
execute = { adapter = "agent", version = "^0.1" }
gate = { adapter = "gate-pipeline", version = "^0.1", rungs = ["compile", "test", "clippy"] }
notify = { adapter = "slack", version = "^0.1", template = "completion-with-trace" }
```

The user writes 3 lines of TOML; behind it sits the entire chain.

**The n8n 3-system-minimum finding**: top templates are dominated by three-system minimum
workflows (trigger -> enrich -> write+notify). Single-integration utilities do not retain.
The templates that retain are ones where the user reaches "automated handoff to a system
they don't actively check." Roko's headline recipe should touch at least 3 systems.

**Priority**: P1 -- requires the verified adapter set (Pattern 17) to exist first. Recipes
ship publicly only when 10-20 verified reference adapters exist to compose them from.

---

## Pattern 17: Verification Badge Gravity

**Evidence**: Terraform's February 2018 RedMonk audit: out of 376 total modules, **42 were
HashiCorp-verified -- and those 42 verified modules accounted for >95% of all downloads**.
AWS modules alone were >94% of total downloads.

**The verification badge is doing nearly all the discovery work.** This inverts the usual
"build the long tail" advice.

**Additional evidence**:
- n8n Verified Community Nodes (May 2025): ~25 verified out of ~2,000 community nodes,
  requiring GitHub Actions + npm provenance for publishing
- OTel Collector Contrib: per-signal stability levels (Stable/Alpha/Development) act as
  graduated verification badges
- Airbyte: graduated-by-quality bounty payouts (more for alpha->GA promotion than for new
  shipping) structurally align the badge with maintenance burden

**Roko application**:

Plan v1 around 10-20 "Roko Verified" reference adapters, gated by a real review process:
1. Pass `roko-conformance` crate in CI
2. Publish via GitHub Actions with trusted publishing (crates.io RFC 3691)
3. Maintain CODEOWNERS entry for the adapter
4. Declare per-capability stability levels in `metadata.toml`

Composite quality scoring is wasted effort below ~1,000 adapters; a binary `verified: true`
field is sufficient and far more legible to users.

**Competitive delta**: MCP has 17,468+ servers with no verification system, resulting in
52% abandonment. Roko's verification badge positions it as the quality layer for the agent
ecosystem.

**Priority**: P0 -- curate 10-20 verified reference adapters before encouraging the long
tail. The badge is the mechanism that does discovery work.

---

## Pattern 18: Compounding Integration Chains

**Evidence**: Five named chains, each with production analogs and roko's specific delta.
The empirical finding from n8n (9,487 templates) is that the dominant template shape is
**trigger -> enrich -> write+notify** -- a three-system minimum. Zapier's Mike Knoop
confirmed multi-step Zaps were the largest single product upgrade in Zapier history.

**The Five Chains**:

| Chain | Flow | Closest Analog | Roko Delta |
|---|---|---|---|
| **A** | Sentry error -> plan -> GitHub PR -> OTel trace -> Linear closed | Sentry Seer Autofix | Agent plan becomes a span; closes Linear issue |
| **B** (lead demo) | Linear webhook -> plan -> GitHub PR -> CI -> Linear status | Devin (closed-source, 659 PRs/week) | Open-source, on-prem, sub-10s via Rust |
| **C** | GitHub label -> plan -> PR -> Slack approval -> merge | Sweep (7.4k stars) | Combines label trigger + Slack approval gate |
| **D** (killer) | Slack thread -> agent -> tool use -> Slack reply with trace URL | **No competitor ships this** | Inline observability in the Slack thread |
| **E** | Recipe-as-template composition | terraform-aws-modules/eks (139.9M downloads) | One recipe ships sub-recipe trigger variants |

**Chain composition mechanics**: Chains A-D share adapters and compound. A team that wires
Chain B (Linear -> PR -> CI -> Linear) gets Chain C (GitHub label -> PR -> Slack -> merge)
at near-zero marginal cost because the GitHub and CI adapters are already configured.

**Priority**: P1 -- requires GitHub + Linear + Slack adapters. Chain B is the lead demo;
Chain D is the killer differentiation.

---

## Pattern 19: Event-Triggered Shareable Artifacts

**Evidence**: Supabase, Vercel, and Clerk all converge on the same Day-2 pattern: the user
produces a publicly shareable artifact. Supabase's activation keystone is "create a database,"
not signup (Craft Ventures, Oct 2025). Vercel's is `vercel --prod` producing a public
preview URL.

**The Trace URL as Viral Artifact**:

Roko's shareable artifact is the **trace URL**. When a user runs an agent against a real
GitHub repo, the resulting Langfuse public-share trace URL shows the entire agent decision
tree: which tools were called, which model, how many tokens, latency per span.

Properties of the trace URL:
1. **Shareable** -- one click to post to Twitter/LinkedIn with pre-filled template
2. **Self-explanatory** -- a non-Roko-user can read the trace and understand the value
3. **Proof of work** -- contains concrete metrics (tokens, cost, latency, pass/fail)
4. **Differentiated** -- no competitor ships inline traces in Slack

**k-Factor Mechanics**: SaaS typical k = 0.2; consumer 0.45 median. Each retained Roko user
should produce ~1 new install per 5 users via word of mouth. The trace URL converts a private
event (agent run) into a public signal (shareable artifact).

**Priority**: P1 -- requires Langfuse partnership + slack adapter. The pre-filled tweet
template is ~50 LOC in the CLI.

---

## Pattern 20: Design Partner Revenue Loop

**Evidence**: Common Paper Design Partner Agreement v1.3 (CC BY 4.0, written by 30+
attorneys, used by Temporal/Snyk). Temporal's Cadence-graduation customers (Snap, Box,
Coinbase, Checkr) were the foundation for their $1.5B Series B.

**Important correction**: Temporal did not run a structured cold-pitch design-partner motion
in 2019-2020; they inherited customers from Cadence at Uber. The directly portable tactic is
the **services-attached-to-software model**: offer free implementation and integration
engineering as part of the design-partner package.

**The Loop**:
```
Common Paper v1.3 contract -> paid design partner ($24k/yr) ->
  adapter contribution back to OSS -> ecosystem growth ->
  next partner (lower friction) -> repeat
```

**90-Day Revenue Target**: 2 Tier 1 contracts ($48k ARR) + 1 Tier 2 adapter contract ($15k)
= $63k bookings in 90 days, zero CAC (all inbound from OSS).

**3-Tier Commercial Offering**:

| Tier | Offering | Price | Engineering Effort |
|---|---|---|---|
| **1** | Roko Production Support | $24,000/year ($2,000/month) | Zero -- Slack + calendar only |
| **2** | Custom Adapter Authoring | $10,000-$25,000 fixed-fee per adapter | 4-8 week delivery; IP returns to OSS |
| **3** | Roko Cloud Early Access | $499-$1,499/month flat | Defer 60-90 days; hand-deployed |

**Common Paper DPA v1.3 Gotchas**:
1. Only modify the Cover Page -- Standard Terms are incorporated by URL reference
2. Term + Fees default "none" if blank -- explicitly populate both
3. Provider-owns-Feedback IP (Section 1.3+6) is load-bearing -- never concede this clause
4. No exclusivity / no ROFR -- the standard agreement contains neither
5. Add non-refundable fees + GDPR DPA add-on for Berlin
6. Governing law: Berlin courts for EU, Delaware for US

**Ferrous Systems pricing precedent**: EUR 25/seat/month for binary distributions + LTS +
qualified compiler. Rust enterprise contracts trend lower-volume but higher-touch than
TypeScript/Python equivalents.

**Priority**: P0 -- requires zero engineering. The first Tier 1 contract can be signed
tomorrow using Common Paper's template.

---

## Pattern 21: AAIF Governance Leverage

**Evidence**: The Linux Foundation Agentic AI Foundation (AAIF) surpassed CNCF in membership
at the same stage -- 170+ organizations in <4 months, fastest-growing LF foundation. Platinum
members: AWS, Anthropic, Block, Bloomberg, Cloudflare, Google, Microsoft, OpenAI.

**The pattern**: SEP authorship + AAIF Technical Committee participation as the
under-represented EU voice in protocol governance -> protocol-level influence -> adapter
standard alignment -> Roko adapters become reference implementations.

**Why this is structurally novel**: there is no European Platinum member visible in MCP
governance. A Berlin-based maintainer occupying this seat through SEP authorship is
higher-leverage than chasing working group calls where the table is already set.

**Key events**:
- MCPCon Europe: Amsterdam, Sep 17-18, 2026
- MCPCon NA: Oct 22-23, 2026

**Multiplicative value**:
- Protocol-level influence shapes the standard toward adapter-trait patterns Roko implements
- Reference implementation status in a 170+ org foundation is permanent credibility
- No monetary cost to participate -- only engineering time on SEP authorship
- Each SEP accepted makes Roko's adapter architecture the de facto example

**Priority**: P0 -- zero engineering cost, only requires SEP document authorship.

---

## Synergy Composition Map

How all 21 synergy patterns reinforce each other:

| Pattern | Enables | Enabled By |
|---|---|---|
| 1. Network Effects | 5 (Marketplace), 6 (Learning), 11 (Flywheel) | 4 (API), 7 (Interop) |
| 2. Composable Arch | 3 (Platform), 4 (API), 8 (Unbundling) | 12 (Standards) |
| 3. Platform Eng | 9 (DX) | 2 (Composable), 7 (Interop) |
| 4. API Economy | 1 (Network), 5 (Marketplace) | 2 (Composable), 12 (Standards) |
| 5. Marketplace | 1 (Network), 4 (API) | 6 (Learning), 9 (DX) |
| 6. Compounding Learning | 1 (Network), 10 (Emergence), 11 (Flywheel) | 11 (Flywheel) |
| 7. Interoperability | 1 (Network), 3 (Platform), 8 (Unbundling) | 12 (Standards) |
| 8. Unbundling | 9 (DX) | 2 (Composable), 7 (Interop) |
| 9. DX | 1 (Network), 3 (Platform), 5 (Marketplace) | 8 (Unbundling) |
| 10. Emergence | 6 (Learning) | All (emergent from system) |
| 11. Data Flywheel | 1 (Network), 6 (Learning) | All data-producing patterns |
| 12. Standards | 2 (Composable), 4 (API), 5 (Marketplace), 7 (Interop) | (Design choice) |
| 13. Cursor Unbundling | 8 (Unbundling), 9 (DX) | 2 (Composable), 4 (API) |
| 14. Compliance Attestation | 1 (Network), 7 (Interop) | 6 (Learning), 12 (Standards) |
| 15. Bandit Experiments | 6 (Learning), 11 (Flywheel) | 6 (Learning), 10 (Emergence) |
| 16. Recipe Compression | 2 (Composable), 5 (Marketplace), 9 (DX) | 12 (Standards), 17 (Badge) |
| 17. Verification Badge | 5 (Marketplace), 11 (Flywheel), 16 (Recipes) | 12 (Standards) |
| 18. Compounding Chains | 2 (Composable), 9 (DX), 16 (Recipes) | 12 (Standards), 17 (Badge) |
| 19. Event-Triggered Artifacts | 1 (Network), 9 (DX), 18 (Chains) | 16 (Recipes), 17 (Badge) |
| 20. Design Partner Revenue | 16 (Recipes), 17 (Badge), 18 (Chains), 19 (Artifacts) | 2 (Composable), 12 (Standards) |
| 21. AAIF Governance | 7 (Interop), 12 (Standards), 17 (Badge), 20 (Revenue Loop) | 2 (Composable), 12 (Standards) |

**The master synergy**: Standards (12) enable Composability (2), which enables
the API Economy (4) and Marketplace (5), which drive Network Effects (1), which
power the Data Flywheel (11), which feeds Compounding Learning (6), which creates
Emergent Behaviors (10). Each layer amplifies the ones above it.

**Compound synergies across pattern groups**:
- Patterns 13-15 (Cursor Unbundling + Compliance + Bandit Experiments): the gate pipeline
  sold standalone IS the compliance product, AND the bandit experiment system makes the
  gate pipeline self-improving.
- Patterns 16-17 (Recipes + Badges): verified recipes that compose verified adapters create
  a trust-cascading system where the recipe inherits the trust of its constituent adapters.
- Patterns 18-20 (Chains + Artifacts + Revenue): each design partner produces an adapter
  (verified), which becomes a recipe component (compressed), which creates a chain variant
  (compounding), generating a showcase entry (artifact). The revenue loop accelerates all
  four preceding patterns.
- Pattern 21 (AAIF Governance) is the meta-pattern: protocol-level influence ensures that
  the evolving MCP standard aligns with Roko's adapter-trait architecture, reducing future
  compatibility cost to near-zero for all other patterns.

---

## Implementation Priority Summary

| Priority | Patterns | What to Build | Why First |
|---|---|---|---|
| **P0** | 2, 6, 10, 11 (partial), 15, 17, 20, 21 | Adapter traits, learning visualization, signal dashboard, bandit experiment framing, verification badge on 10-20 reference adapters, design partner contracts, AAIF SEP authorship | Already partially built or zero engineering needed |
| **P1** | 1, 4, 7, 8, 9, 12, 13, 14, 16, 18, 19 | Federated protocol, API specs, interop (A2A), DX improvements, versioned traits, standalone gate/router/cost products, compliance attestation stream, recipe.toml, integration chains, trace URL artifacts | Creates the growth foundation |
| **P2** | 3, 5 | Platform engineering positioning, marketplace infrastructure | Requires P0 + P1 user base |
| **P3** | 11 (full) | Community flywheel, cross-instance aggregation | Requires significant adoption |

---

## Sources

- Network effects: NFX research, "70% of value" statistic, GitHub/npm growth data
- MACH pattern: MACH Alliance, Shopify composable commerce architecture
- Platform engineering: Gartner "80% of large orgs by 2026," Backstage 3,400+ orgs, 89% share
- API economy: Grand View Research, $45.3B by 2033
- Marketplace dynamics: Precedence Research, $52.62B agent marketplace by 2030
- Compounding learning: Cursor ARR trajectory ($100M -> $2B in 14 months)
- Interoperability: MCP (97M monthly SDK downloads), A2A (150+ orgs), AAIF (170+ orgs)
- Unbundling/rebundling: Jim Barksdale theory, Craigslist -> verticals case study
- DX as growth: Stripe 7-minute integration, Vercel zero-config, Railway Git-push deploy
- Emergent behaviors: stigmergy research, ant colony optimization
- Data flywheel: Google, Netflix, Amazon, Tesla fleet learning
- Standards: Rust editions model, TCP/IP stability
- Cursor: $2B ARR (Feb 2026), $29-60B valuation, Fortune supply chain concern
- Codex CLI: 75K+ stars, 3M WAU, Apache-2.0 Rust, OpenAI-locked
- Devin/Cognition: $25B valuation talks, ~$150M combined ARR
- Claude Code: $2.5B estimated annualized revenue, 91% CSAT
- Terraform: 3,500+ providers, $6.4B IBM acquisition, 42 verified = >95% downloads
- n8n: 6,234 nodes, 9,487 templates, 13.6 nodes/day growth (April 2026)
- MCP: 97M monthly SDK downloads, 17,468+ servers, 52% abandonment (Rapid Claw audit)
- Supabase: 4.5M developers, $5B Series E (Oct 2025), activation keystone model
- Vanta: $220M ARR, $4.15B valuation (July 2025)
- Common Paper DPA v1.3: 30+ attorneys, CC BY 4.0, Temporal/Snyk precedent
- Temporal: $1.5B Series B, Cadence-graduation customer model
- Ferrous Systems: EUR 25/seat/month, training-as-funnel model
- EU AI Act: Article 50 enforcement August 2, 2026
- LLM pricing: GPT-5.5 $5/$30, Claude Sonnet 4.6 $3/$15, Gemini 2.0 Flash $0.10/$0.40
