# Pitch Intelligence: Adapter Architecture as Investor Thesis

How roko's adapter-first architecture maps to investor theses, competitive differentiation
in the April 2026 market, benchmark evidence, demo strategy, and identity/auth surfaces.

Last updated: 2026-04-29.

---

## 1. The April 2026 Market Thesis

### 1.1 The numbers that define the conversation

AI developer tools are the fastest-growing software category in history:

| Metric | Value | Source |
|---|---|---|
| AI coding tools market (2025) | $6.8B | Multiple research firms |
| Projected 2026 | $8.5B | 24% CAGR through 2034 |
| Cursor ARR (Feb 2026) | $2B+ | Reuters, fastest SaaS to $2B ever |
| Cursor valuation range | $29-60B | CNBC ($29.3B Nov 2025), xAI ($60B Apr 2026) |
| Devin/Cognition valuation | $25B (talks) | SiliconANGLE Apr 2026 |
| Claude Code est. annualized revenue | $2.5B | FindSkill.ai |
| Anthropic annualized revenue | $14B | Early 2026 |
| AI share of global VC (2025) | 61% = $258.7B | OECD |
| Companies crossing $100M ARR | 7+ | CB Insights |
| Developers using AI coding tools at work | 76%+ | JetBrains survey 2026 |
| Average tools used simultaneously | 2.3 | Industry surveys |

### 1.2 The structural problem investors see

Every market leader is architecturally coupled to a single inference provider:

- **Cursor** ($2B ARR): Built on Anthropic Claude. Fortune reported March 2026 its "very
  uncertain future" because Anthropic controls the supply chain. If Anthropic raises prices,
  restricts access, or ships a competing IDE, Cursor's margins collapse.
- **Codex CLI** (75K stars): Apache-2.0 Rust, structurally locked to OpenAI auth. Cannot
  route to Claude, Gemini, or local models without forking.
- **Devin** ($150M+ combined ARR): Closed-source, proprietary SWE-1.5 model. $500/month.
  Enterprise-only positioning.
- **Claude Code** ($2.5B est.): Anthropic-only. Terminal-based. Throttling issues since March
  2026.
- **GitHub Copilot** (24% market share): Microsoft/GitHub platform lock-in. Multi-model but
  platform-dependent.

**The gap**: No vendor-neutral, open-source agent orchestration platform exists with
verification, learning, and integration depth.

---

## 2. Investor Thesis Mapping

### 2.1 a16z vocabulary mapped to roko

Malika Aubakirova's 9-essay corpus provides borrowable terminology:

| Her vocabulary | Where she uses it | Roko adapter surface |
|---|---|---|
| "coordination becomes a bottleneck" | Big Ideas 2026 | Plan DAG + parallel executor + gate pipeline |
| "routing, locking, state management, policy enforcement" | Big Ideas 2026 | `Router`, `Policy`, `Substrate`, `Gate` traits |
| "5,000 sub-tasks fan-out" | Big Ideas 2026 | Plan DAG executor |
| "machine identity" | Keycard, Et Tu Agent | `AuthAdapter`, capability-based security |
| "from static identity to dynamic intent" | Keycard | Capability attenuation chains |
| "compounds in value over time" | Continual Learning | 4 compounding loops |
| "retrieval is not learning" | Continual Learning | Neuro knowledge store tier progression |
| "validated paths" | Pentesting | Gate pipeline: cryptographic evidence, not assertions |
| "workload-model fit" | Cinderella | CascadeRouter bandit-based selection |
| "output, not users" | Casado's framing | Usage-based billing via `BillingAdapter` |

### 2.2 The four investment pillars

**Pillar 1: Coordination plane.** Her Big Ideas 2026 essay is a spec for roko. The bottleneck
she identifies -- "routing, locking, state management, policy enforcement across massive
parallel execution" -- maps 1:1 to roko's 6 verb traits.

**Pillar 2: Thundering herd survival.** Platforms that "survive the deluge of tool execution."
Tower-style middleware layers: rate limiting, caching, batching, budget enforcement.

**Pillar 3: Machine identity.** From Keycard through Et Tu Agent: "years of underinvestment
in machine identity." One `AuthAdapter` trait, multiple implementations (OAuth, UCAN),
composable with any workflow.

**Pillar 4: Continual learning.** "Retrieval is not learning." Roko's 4 compounding loops
directly implement her thesis.

### 2.3 Partner vocabulary to mirror

| Partner | Their framing | Roko mapping |
|---|---|---|
| **Joel de la Garza** | "2026 is the year of agents; identity is the bottleneck" | `AuthAdapter` + `SecurityScanner` |
| **Zane Lackey** | Kill-chain stages differentiate, not overlap | Each gate rung = a kill-chain stage |
| **Matt Bornstein** | "agents don't really work yet" -- needs concrete numbers | Lead with benchmark data, not architecture |
| **Yoko Li** | "compressing coordination" / MCP marketplace | MCP as universal adapter transport |
| **Martin Casado** | "value scales with output, not users" | Usage-based pricing |

### 2.4 The productive tension to bridge

Joel: "2026 is the year of agents." Matt: "agents don't work yet." The bridge:
"Joel is right about demand. Matt is right that today's frameworks fail. Roko is the
missing coordination + verifiability layer that closes that gap."

---

## 3. Competitive Positioning via Adapter Architecture

### 3.1 The "missing columns" argument

| Capability | LangGraph | AutoGen | CrewAI | Temporal | Codex CLI | **Roko** |
|---|---|---|---|---|---|---|
| **Multi-model routing** | Manual | Manual | Manual | None | OpenAI only | CascadeRouter (bandit-based, 20+ providers) |
| **Verification pipeline** | None | None | None | None | None | 7-rung pipeline with adaptive thresholds |
| **Cost tracking** | None | Basic | None | None | None | Per-turn efficiency, `BillingAdapter`, gate-level attribution |
| **Composable middleware** | None | None | None | Interceptors | None | Tower-style `Layer<S>` on every pipeline |
| **Plugin ecosystem** | Python only | Python only | Python only | Go/Java/Python | Rust + MCP | Rust in-process OR any-language via process boundary |
| **Learning from execution** | None | None | None | None | None | 4 compounding loops |
| **Knowledge compounding** | None | None | None | None | None | 4-tier neuro store + distillation + federated sync |
| **Protocol interop** | MCP (basic) | None | MCP (basic) | None | MCP client+server | MCP + A2A + ACP + OTel + Webhooks |
| **Self-developing** | No | No | No | No | No | PRD -> plan -> execute -> gate -> learn -> iterate |

**The positioning is not "roko vs LangGraph." It is: roko provides capabilities that do
not exist in any competing product.**

### 3.2 Codex CLI collision response (R8)

OpenAI Codex CLI is Apache-2.0, ~95% Rust, 75K+ stars, 640+ releases, 3M weekly active users.
"Apache-2.0 Rust agent runtime" is no longer differentiating on its face.

**4-pillar differentiation that survives**:

1. **Adapter-trait architecture** -- 18-crate composition vs Codex's structural OpenAI coupling
2. **Model-agnostic from day one** -- 20+ providers via adapter trait vs OpenAI-only
3. **EU sovereignty** -- Berlin-built, no US-cloud control plane, CRA-aligned
4. **Integration depth** -- Linear AgentSession, Slack-to-trace, Sentry; Codex is a CLI, not
   an integration runtime

**Template**: "Codex CLI is Apache-2.0 Rust, 75K stars, and locked to OpenAI. Roko is
Apache-2.0 Rust, adapter-trait architecture, and speaks every inference backend from day one.
Same language, different philosophy -- one is a product, the other is a platform."

### 3.3 Process boundary as competitive moat

LangGraph/CrewAI/AutoGen are Python-only -- their plugin surface is a Python class. Roko's
adapter traits can be implemented in-process (Rust, fast) OR as external processes (any
language, crash-isolated). This is the Terraform/Kubernetes pattern that produced 2,700+
providers and 300+ operators.

---

## 4. Benchmark-Driven Integration Value

### 4.1 HAL benchmark data

HAL (arXiv:2510.11977, ICLR 2026): 9 benchmarks, 21,730 rollouts, $40K total cost.

| Metric | Naive baseline | Coordinated (roko-style) | Ratio |
|---|---|---|---|
| Cost per task (tau-bench) | $0.30 - $4.00 | $0.02 - $0.10 | 10-30x |
| Cost per task (AppWorld) | ~$4.00 | ~$0.10 | ~40x |
| Cost per task (GAIA L1) | $0.40 | $0.01 | ~40x |
| Aggregate (5-task mix) | ~$6-8 | ~$0.20 | ~30x |

**Which adapters drive the cost numbers**:

| Adapter | Contribution | Mechanism |
|---|---|---|
| **CascadeRouter** | ~5-10x | Routes cheap models to easy tasks |
| **Gate pipeline** (adaptive) | ~2-3x | Fails fast, skips redundant gates |
| **Prompt cache** (CAS) | ~1.5-2x | Deduplicates identical prompts |
| **Context bidding** (VCG) | ~1.5-2x | Avoids stuffing irrelevant context |
| **Episode-informed dispatch** | ~2-3x | Does not repeat known-bad strategies |

**Multiplicative**: 5 x 2 x 1.5 = 15x conservative. With episode + context: 45x upper bound.
The 30x claim sits comfortably in this range.

### 4.2 Presenting to Aubakirova's empirical voice

1. **Single ratio, plain English**: "30x less LLM spend per task."
2. **Crushed bar visualization**: $4.18 red at 100% width, $0.14 green at 3.3%.
3. **Live benchmark in the room**: Bloomberg Two-Tape corner widget during the meeting.

---

## 5. Demo-as-Product Strategy

### 5.1 Four visualization artifacts as product features

#### Artifact 1: Trace Page (/runs/{id})

Shareable URL showing execution trace, cost breakdown, agent timeline. Product version:
the `ObservabilityExporter` adapter surface rendered as UI.

| Element | Adapter Source | Value |
|---|---|---|
| Multi-track timeline | `ObservabilityExporter` | Debugging, cost attribution |
| Per-agent color coding | `Router` dispatch | Team visibility |
| Shade-by-cost | `BillingAdapter` | Budget monitoring |
| Log/timeline sync | Event sourcing | Root cause analysis |

#### Artifact 2: Computation Receipt

Hybrid dark-canvas/cream-receipt card. Product version: `Gate` trait + event sourcing
rendered as auditable artifact.

| Element | Adapter Source | Value |
|---|---|---|
| Run ID, cost, tokens | `BillingAdapter` + OTel | Invoice attachment |
| Gate pass/fail | Gate pipeline | Compliance evidence (SOC 2, EU AI Act Article 50) |
| Agent delegation chain | `AuthAdapter` | Audit trail |
| Downloadable PDF | Server-side render | "Thing that leaves the room" |

#### Artifact 3: Terrain Knowledge Graph

d3-contour terrain showing knowledge compounding over time. Product version: `roko-neuro`
knowledge store rendered as visualization.

#### Artifact 4: Pulse Globe

three-globe showing agent fan-out. Product version: `ProcessSupervisor` + `Router` rendered
as real-time topology monitor.

### 5.2 The key insight

Every demo visualization is a consumer of adapter outputs. Building the adapter infrastructure
IS building the product features. The pitch demo is the product, viewed through the
ObservabilityExporter adapter.

---

## 6. Commercial Strategy

### 6.1 3-Tier offering (shippable in 90 days)

| Tier | Offering | Price | Effort |
|---|---|---|---|
| **1** | Roko Production Support | $24K/year ($2K/month) per design partner | Zero engineering (Slack + calendar) |
| **2** | Custom Adapter Authoring | $10K-$25K fixed-fee per adapter | 4-8 week delivery; IP returns to OSS |
| **3** | Roko Cloud Early Access | $499-$1,499/month flat, single-tenant | Hand-deployed a la Temporal 2021 |

**Series A claim**: 2 Tier 1 + 1 Tier 2 = $63K bookings in 90 days, $48K+ ARR run-rate.
Structurally identical to Temporal's early-2021 position; they closed Series B at $1.5B
six months later.

### 6.2 Design partner agreement

Use Common Paper Design Partner Agreement v1.3 (CC BY 4.0, 30+ attorneys, Temporal/Snyk
precedent). 6-month term, $12K fee paid quarterly, 25% future discount, bi-monthly feedback.

**Key clause**: Provider-owns-Feedback IP (Section 1.3+6) is load-bearing. Never concede.

### 6.3 Why the fee should not be zero

A free pilot is psychologically a beta; a $12K contract is a commitment. Common Paper
explicitly recommends non-refundable fees. The Series A signal is "paying customers" not
"interesting LOIs."

### 6.4 Temporal as structural analog

| Dimension | Temporal (2019-2022) | Roko (2026 target) |
|---|---|---|
| Runtime | Go OSS workflow runtime | Rust OSS agent runtime |
| Cloud | Hand-deployed cells | Hand-deployed single-tenant VMs |
| Monetization onset | ~18-24 months post-founding | Month 0 via design partners |
| Series B trigger | Paying design partners in production | 2 signed Tier 1 + 1 Tier 2 |
| Current valuation | $5B (Aug 2025) | Pre-revenue |

### 6.5 Free tier design

Match Supabase's auto-pause-on-inactivity: 1 project, 1 deployment, auto-pause after 7 days
idle, no credit card. Allow OSS self-host of everything with no rug-pull risk. Gate
operational convenience, never functionality.

---

## 7. Non-Dilutive Capital (Berlin Advantage)

| Program | Amount | Deadline | Fit |
|---|---|---|---|
| **NLnet NGI Zero Commons Fund** | EUR 5K-50K | **June 1, 2026** | Strong: Rust projects routinely funded |
| **Sovereign Tech Fund** | EUR 23M+ across 60 OSS projects | Rolling | Plausible for infrastructure |
| **Sovereign Tech Fellowship** | EUR 64K-82K/year | ~Q1 2027 | Strong for maintainers personally |
| **Rust Foundation Community Grants** | From $100K allocation (2026) | Rolling | Modest stipends |

**Total realistic non-dilutive within 18 months**: EUR 80K-150K. Extends runway 6-12 months.

---

## 8. Conference & Event Strategy

| Event | Dates | Priority | Action |
|---|---|---|---|
| **AI Engineer World's Fair** | June 29-July 2, 2026 | Highest buyer density | Side-event: "Rust Agent Runtime Happy Hour" ($5-15K) |
| **EuroRust** | Oct 14-17, Barcelona | Highest Rust leverage | CFP submission: "Trait-based agent orchestration" |
| **MCPCon Europe** | Sep 17-18, Amsterdam | MCP ecosystem | SEP presentation |
| **MCPCon NA** | Oct 22-23 | MCP ecosystem | Partnership conversations |
| **RustConf** | Sep 8-11, Montreal | Rust community | Talk submission |
| **KubeCon NA** | Nov 9-12, Salt Lake City | CNCF K8s AI Conformance | Future (2027+) |

**Rust Berlin on location**: every 4 weeks at Ferrous Systems + Slint + KDAB office.
Will's home channel for early prototype demos.

---

## 9. VC Map (Corrected April 2026)

### Priority funds

| Fund | Thesis | Check Size | Notes |
|---|---|---|---|
| **468 Capital** | Berlin + SF, AI & Infra | $1.3B+ raised | Only Berlin-anchored fund with explicit AI infra thesis |
| **Air Street Capital** | $232M Fund III (Mar 2026) | $500K-$15M | Largest solo-GP fund in Europe. Berlin-friendly. |
| **Cherry Ventures** | Fund V $500M (Feb 2025) | EUR 2-7M | Backed Dash0 (observability) + Riplo (workflow) |

### Stage-appropriate

| Stage | Funds |
|---|---|
| Pre-seed Berlin | NAP (ex-Cavalry), Earlybird, Atlantic Labs |
| Series-A-grade | 468 Capital, Air Street Capital |

**Correction**: La Famiglia merged into General Catalyst 2024. Cavalry rebranded to NAP
February 2025.

---

## 10. Content Strategy

Priority-ordered content formats:

1. **Technical deep-dives** ("how we built X") -- highest conversion
2. **X-vs-Y comparisons** ("Roko vs LangChain") -- most SEO leverage
3. **Migration guides** ("Moving from $competitor to Roko in 30 minutes")
4. **Benchmarks** (reproducible methodology)
5. **Category-defining posts** ("Modern Agent Runtime Stack" -- own the search graph)

**This Week in Roko**: Start from week one, even with 50 readers. Modeled on
"This Week in Bevy" (thisweekinbevy.com).

**Crate of the Week**: Submit to This Week in Rust once 0.x ships.

---

## Sources

- Cursor: $2B ARR (Reuters Feb 2026), Fortune uncertain future (Mar 2026), xAI $60B deal (Apr 2026)
- Codex CLI: 75K+ stars, 3M WAU, Apache-2.0 Rust (GitHub Apr 2026)
- Devin/Cognition: $25B talks (SiliconANGLE Apr 2026), Windsurf acquisition ~$250M
- Claude Code: $2.5B est. annualized, 91% CSAT (FindSkill.ai, JetBrains 2026)
- AI VC: OECD (61% global VC = $258.7B to AI in 2025)
- HAL: arXiv:2510.11977, ICLR 2026
- Temporal: $5B at Series D (Reuters Aug 2025)
- MCP: 97M monthly SDK downloads, 17K+ servers (Nerq Q1 2026)
- Supabase: $5B Series E (Oct 2025), 4.5M developers
- Common Paper: DPA v1.3 (CC BY 4.0)
- JetBrains: AI coding tools developer survey (April 2026)
