# Competitive Landscape

Every named competitor relevant to Nunchi's positioning. What they build. Their funding and traction (verified). Where they overlap, where they don't, and what to say when they come up in conversation. Updated through April 2026.

This document supersedes the earlier `08-competitive-landscape` document. For positioning at the category level, read `02-strategy-and-narrative.md` first; for the underlying market thesis, read `01-thesis.md`.

---

## 1. The Five-Layer Map and the Empty Layer

The agent infrastructure market has five established layers. Nunchi occupies an empty sixth layer that no funded company has claimed.

| Layer | Who | What they do |
|---|---|---|
| **L1: Models** | Anthropic, OpenAI, Google, Meta, Mistral, Cohere, DeepSeek, xAI | Foundation models. |
| **L2: Frameworks** | LangChain, Mastra, Vercel AI SDK | Toolkits for building one agent. |
| **L3: Orchestration** | CrewAI, AutoGen (Microsoft), LangGraph | Multi-agent coordination frameworks. Define topology. |
| **L4: Evaluation / Trust** | Braintrust, Nava, LangSmith, Patronus AI, Arize | Evaluation, observability, tracing, scoring. |
| **L5: Applications** | Devin (Cognition), Cursor, Replit Agent, Lovable, Bolt, Harvey, Hebbia, Decagon, Sierra | End-user products. |
| **L6: Coordination Plane (Nunchi)** | — | Identity, routing, gates, knowledge, durability, cost prediction, policy enforcement, settlement. |

L6 does not exist as a product today. Every company in L3–L5 has built fragments internally. None has extracted it as reusable infrastructure.

---

## 2. The Empty Quadrant and the Honest 2x2

Mapped against two axes investors care about — **open infrastructure vs proprietary platform** (horizontal) and **execution focus vs coordination focus** (vertical):

| | Open infrastructure | Proprietary platform |
|---|---|---|
| **Coordination** | **Nunchi (Roko + the Nunchi blockchain)** — alone | Capsule, Nava, t54 Labs, Keycard, Astrix |
| **Execution** | LangChain, CrewAI, Mastra | Temporal, /dev/agents, Sycamore |

The top-right (open infrastructure focused on coordination) is empty. Frameworks are open but execution-focused. Trust intercepts and identity products are proprietary and coordination-adjacent. **No company combines a production runtime, a sovereign chain with HDC primitives, ERC-8004 identities with 7-domain reputation and ZK-HDC verification, and stigmergic coordination.**

**The Hunter Walk problem with 2x2s.** *"I've never been presented a 2x2 where the startup isn't in the upper right."* For any actual slide, prefer a **Harvey-Ball feature table** (filled / half-filled / empty circles) over the 2x2. Reads as more honest because it acknowledges competitors are strong on some dimensions. Recommended columns: open-source runtime, sovereign L1, ERC-8004 full stack (7-domain reputation + ZK-HDC), production code shipped, Series A capital. Nunchi wins on all five. Nava is strong on EigenLayer pedigree and press, weak on runtime and ZK-HDC.

---

## 3. Per-Competitor Analysis

### Temporal — $5B valuation

**What they are.** Durable execution platform. Long-running workflows that survive crashes, restarts, and infrastructure failures. Activities and workflows defined in code; Temporal guarantees exactly-once execution with automatic retry and state persistence.

**Their demo.** A workflow processes an order, calls three APIs, gets killed mid-execution, restarts on a different machine, completes from exactly where it left off. The "kill-and-resume" demo is iconic in infrastructure sales.

**Verified funding.** $300M Series D at $5B valuation (Reuters, August 12, 2025). Earlier: $146M Series C at $1.7B (November 2024); $18.75M Series A (October 2020). The Series D round was led by Sarah Wang and Raghu Raghuram at a16z (NOT Martin Casado) — agent durability/orchestration sits in Wang's practice. If Casado feels overlap with Temporal during a pitch, his likely action is to route the deal to Wang. Volunteer the Temporal distinction in the first 10 minutes to keep the conversation in Casado's lane.

**Production usage.** OpenAI runs Codex on Temporal; Replit uses Temporal for agent orchestration; Lovable for AI web-dev agent; Snap and Datadog use it for backend pipeline orchestration.

**Their gaps relative to Nunchi.**
- Not agent-specific. Workflows are general-purpose. No concept of model routing, prompt optimization, gate validation, agent identity, or knowledge sharing.
- No cost awareness. Tracks workflow state, not inference cost. Cannot predict cost or route to a cheaper model.
- No learning. Workflows execute the same way every time.
- No identity. No concept of verified agent identity or per-agent policy enforcement.

**What to steal.** The kill-and-resume demo pattern. Single most compelling live demo in infrastructure. Nunchi's durability (checkpoint, resume, cost continuation) should be demoed with the same Ctrl+C theatrics.

**The 30-second answer (memorize):**

> *"Temporal is durable execution — they make sure a single workflow finishes despite crashes, inside one company's trust boundary. We love them; we run on them. Their own April 2026 blog says 'the agent framework handles the AI, Temporal handles the infrastructure.' Nunchi is the layer above: portable agent identity, shared memory, cost-aware model routing, and verifiable inter-agent coordination across organizations. Temporal owns 'did this code run.' Nunchi owns 'did the right agent, with the right memory, at the right price, with a receipt the counterparty can verify.' That second problem has network effects Temporal architecturally cannot capture from inside a single namespace — same way Vercel built $9B on top of AWS Lambda."*

---

### LangChain / LangSmith — $1.25B valuation (Series B, October 2025)

**What they are.** Most popular framework for building LLM applications. LangSmith is observability and evaluation: tracing, prompt playground, dataset management, evaluation runs.

**Verified funding.** $125M Series B at $1.25B valuation, October 2025 (Crunchbase; Forbes). Series A (February 2024) traction: 75–80K GitHub stars, 7M monthly downloads, 70K LangSmith closed-beta signups, named logos including Rakuten, Elastic, Moody's, Retool. LangGraph reaches roughly 90M monthly downloads.

**Their demo.** Waterfall trace of an agent execution showing each step, latency, token usage, output. Prompt playground for iterating on system prompts. Evaluation runs comparing model outputs against golden datasets.

**Their gaps.**
- No cross-agent knowledge. Traces individual agent runs. Does not share learned knowledge between agents or across runs.
- No model routing. LangChain connects to one model per chain.
- Framework lock-in. Building on LangChain means building in LangChain's abstractions. Migration is expensive.
- Most popular, valued lowest. Despite the largest developer community in the agent ecosystem, LangChain's valuation is lower than less-adopted competitors. Signals that the market does not believe frameworks capture durable value.

**What to steal.** LangSmith's trace visualization is clean and widely understood. Nunchi's dashboard trace waterfall should be at least as legible.

**What to say.** *"LangChain is a framework. Nunchi is a runtime. The difference: when a LangChain agent crashes, you start over. When a Nunchi-coordinated agent crashes, it resumes from the last checkpoint with its cost meter intact. Frameworks help you build agents. We help agents work together."*

**Cautionary tale.** LangSmith retained value by becoming a generic LLM observability product, not by being tied to LangChain. **Do not let the commercial product depend on the OSS framework's continued popularity.** LangChain bet the OSS on framework lock-in (chains, agents, prompts as proprietary abstractions). Nunchi bets on standards (MCP, A2A, ERC-8004) and verifiability (gate pipeline, signed receipts).

---

### CrewAI — $18M total

**What they are.** Multi-agent orchestration framework. Define "crews" of agents with roles, goals, task assignments. Agents collaborate by passing messages and delegating subtasks. Approximately 150 beta enterprise customers at Series A.

**Their gaps.**
- Crews do not learn from past crews. Each crew execution starts from zero.
- No durability. If a crew crashes, all progress is lost.
- No identity or policy enforcement. Any agent in a crew can do anything.
- Python-only. Performance ceiling for production workloads.
- **Known scaling ceiling at 5–6 agents** in complex workflows; token overhead of role-based prompting grows non-linearly with agent count.

**What to say.** *"CrewAI defines who talks to whom. Nunchi provides the substrate they talk through: verified identity, validated output, shared knowledge, and durable state."*

---

### Mastra — $22M Series A (2026)

**What they are.** Newer agent framework with a focus on developer experience.

**Verified traction (per learnings2 source).** Approximately 22K GitHub stars (1.5K to 7.5K in one Hacker News week), ~300K weekly npm downloads matching Gatsby's 4-year peak in 1 year. Named logos: Replit Agent 3, SoftBank, PayPal, Brex.

**Why they matter.** Developer-experience ratings place Mastra at 9/10 versus LangChain's 5/10 (NextBuild December 2025 survey). Rising fast and likely to displace LangChain for new greenfield projects.

**Why complementary.** Mastra applications need trust verification. Roko sits underneath as the runtime substrate.

---

### Cursor / Anysphere — $1B+ ARR by mid-2025

**Critical: a16z portfolio company (Casado-associated boards). Never compete. Never criticize. Always position complementary.**

**What they are.** AI-native code editor. Fork of VS Code. The fastest-growing developer tool in history.

**Verified traction.** $1B+ ARR by mid-2025 (Reuters, June 5, 2025; founder disclosure). Fastest SaaS to $100M ARR ever (~12 months from launch). Pricing: Free, Pro $20/mo, Business $40/seat/mo, Ultra $200/mo, Enterprise custom. Composer (agent mode) released March 2025.

**Their gaps (frame as opportunities to help, not competitive criticism).**
- Cost. Agent mode uses frontier models for everything, including simple tasks. Nunchi's CascadeRouter can route simple completions to smaller models, reducing inference cost by 3–5x.
- No cross-session knowledge. Each Cursor session starts fresh.
- Cursor's auto-mode is opaque ("why did it pick GPT here?" recurring forum complaint). CascadeRouter is auditable.
- **Cursor's Linear integration is broken** — current canonical complaint thread (Cursor Forum /158505): *"It used to preserve the session and only create a new one with @cursor agent. Right now, I see a session per @cursor in the same issue. This breaks the context usage... Cursor is unusable."* Adjacent threads: /144750 ("Linear + Cloud agents = Useless waste of time"), /158866 (cross-user identity bleed). Cursor's changelog through April 14, 2026 contains no acknowledgment of the Linear-specific failure.

**What to say.** *"Cursor agents through Nunchi's CascadeRouter cost 3–5x less. We make your portfolio company more efficient."* Only framing. Cursor is a customer, not a competitor.

**The unbundling thesis.** Developers are explicitly asking for three pieces of Cursor's bundle as standalone tools: just the model router (auditable vs. Cursor's opaque auto-mode), just the gate pipeline (7-rung as a standalone CI service), just the cost optimizer (caching + routing + budget guard). Pitch line: *"Cursor is at $1B+ ARR but their architecture is single-vendor, opaque routing, and weak gating. We don't compete with Cursor — we sell the three pieces of Cursor that the developers themselves are asking for as standalone tools. Plus the parts Cursor will never build: cross-org agent identity and on-chain audit."*

---

### Braintrust — $300M valuation (Series B, May 2025; Casado-led)

**Critical: Casado portfolio. Position as pre-built integration, not competitor.**

**What they are.** AI evaluation and monitoring platform. Logging, tracing, evaluation datasets, scoring functions, experiment tracking for LLM applications.

**Verified.** $125M total funding. **$300M Series B valuation** (TechCrunch, May 2025). Casado-led at a16z. Iconiq led the larger Series B round. Sub-$10M ARR.

**Their demo.** Side-by-side comparison of model outputs against golden datasets, automated scoring, human review, drift detection.

**What to say.** *"Braintrust evaluations feed into our gate pipeline. A Braintrust eval score becomes a gate rung. We are not replacing Braintrust. We are making Braintrust evaluations enforceable at runtime."* Positions Nunchi as additive to the a16z portfolio.

---

### Devin / Cognition — ~$73M ARR at $10.2B post-money (March 2025)

**What they are.** First widely-demonstrated autonomous software engineering agent. Operates in a sandboxed environment with code editor, terminal, browser.

**Verified facts.**
- $500M Series B in March 2025 at ~$10.2B post-money with reportedly ~$73M ARR (The Information, March 24, 2025) — a 140x ARR multiple.
- Windsurf was acqui-hired by Google for $2.4B in July 2025 (CEO + leadership team + non-exclusive license); Cognition acquired remaining Windsurf assets for ~$220M shortly after (TechCrunch, July 14, 2025).
- Pricing: $20/mo Core + $2.25/ACU and $500/mo Team + $2.00/ACU after the April 3, 2025 Devin 2.0 reset.
- Answer.AI January 2025 evaluation: Devin completed 3 of 20 tasks (15%). Not publicly refuted.
- Cognition self-report (Substack, accessed April 28, 2026): **659 Devin PRs merged in their best week**, up from 154 in best week 2025.

**Their demo.** Three-panel visual (code editor + terminal + browser) showing Devin solving SWE-bench tasks. Viral number: 7x improvement on SWE-bench resolution rate.

**Their gaps.**
- Single-agent. Does not coordinate with other agents, share knowledge, or learn from past sessions.
- No cost visibility. You do not know what a Devin task costs until it is done.
- No gate validation. Output checked by running tests, but no configurable policy enforcement.
- Opaque. Cannot inspect Devin's reasoning or audit its decisions cryptographically.

**The bear case (which infra investors will raise).**
- Devin standalone has not delivered on the autonomous-agent promise. Cognition is now a holding company for Devin + Windsurf.
- The $10.2B valuation was based on the autonomous-agent thesis; actual revenue is closer to "AI IDE + agent assist," which is the Cursor/Copilot category.
- Competitive position is now "behind Cursor, ahead of fast-followers" — not the unique-asset story the early valuation implied.

**What to steal.** Three-panel visual layout (editor + terminal + output) — now the expected format for agent demos. The viral number pattern: a single concrete multiplier on a recognized benchmark. Nunchi's analog: "Princeton HAL benchmark, 50x cost variation; Cline / Uber 40x production case."

**What to say.** *"The Devin story shows what doesn't work: a black-box autonomous agent with no audit layer. Roko is the opposite — every agent action is gated and signed. We're not betting on capability; we're betting on coordination and verification."*

**Anti-customer.** Cognition (Walden Yan's "Don't Build Multi-Agents" post). Philosophically opposed to multi-agent coordination. Do not pursue as design partner.

---

### Nava Labs — $8.3M seed (April 14, 2026)

The closest funded analog to Nunchi.

**Verified facts.**
- $8.3M seed co-led by Polychain Capital and Archetype.
- Stealth for 12 days before public announcement (effective disclosure date April 26, 2026).
- Additional backers: Sreeram Kannan (EigenLayer founder), FalconX, Hack VC, Eskender Abebe (Eliza Labs).

**Press coverage.** Fortune ran the exclusive (April 14, 2026). CoinDesk, The Block, TechCrunch, Decrypt all declined to cover. Rest is Chainwire syndication. Narrative is contestable, not yet locked in.

**Product.** Three components:
- **Execution Escrow** — SDK + MCP Server. Agents call `agent.propose(trade)` with TypeScript / Python SDKs integrating LangChain, CrewAI, OpenAI Agents. Initial verticals DeFi-first: prediction markets, swaps, perps, options.
- **Arbiter** — verification middleware. "Graph of Thoughts verification framework." Intercepts transactions after agent constructs them but before blockchain execution. Verifies each proposed action against policy specification. MPC custody — neither agent nor Arbiter alone can move funds.
- **NavaChain** — Arbitrum L3 settlement / coordination layer with planned parallel Tempo presence.

**No public codebase as of April 26, 2026.** No technical papers published. Private beta with undisclosed enterprise design partners.

**Nunchi advantages over Nava.**
1. **Sovereign L1 capturing sequencer revenue.** Nava is L3 on Arbitrum — pays rent to the Arbitrum sequencer. The Nunchi blockchain is native EVM L1 with co-located Tokyo validators and Simplex consensus, capturing sequencer revenue and controlling the economic model from day one.
2. **Full ERC-8004 stack with 7-domain reputation and ZK-HDC verification.** Nava's Arbiter requires an external policy store and has no published identity primitive. Nunchi uses ERC-8004 to its fullest extent.
3. **ZK-HDC as L1 precompile.** Nava has no published similarity verification scheme.
4. **Production Rust runtime already shipped.** Roko (18 crates, ~177K LOC) in production. Nava has no public codebase. Engineering lead conservatively 10x and likely larger.
5. **Series A capital advantage.** $20–30M target dwarfs Nava's $8.3M seed. Nava's round is promising but insufficient to build out a full L1, ZK proving stack, and enterprise sales motion simultaneously.

**Nava advantages over Nunchi.**
1. EigenLayer pedigree and Polychain conviction.
2. Press cycle head start.
3. Simpler architecture (arbiter intercept is easier to explain and integrate than full L1 with HDC precompiles).
4. Tempo positioning for payments.

**Reframe.** Nava validates that agent verification matters. They sit in the trust layer; Nunchi sits underneath as the coordination plane. Nava's Arbiter would eventually need a coordination plane underneath it. Nava and Nunchi are adjacent, not competitive — they operate at different layers of the stack.

**What to say.** *"Nava confirms the market. They are building the audit trail. We are building the full coordination plane: the runtime that produces the work, the gates that validate it, the proofs that verify it, and the chain that settles it."*

---

### Tempo — $500M at $5B (October 2025; mainnet March 18, 2026)

**Most commonly confused entity in investor conversations. NOT a competitor.**

**What Tempo is.** Payment rail. Simplex consensus delivers ~200ms block times and 300–500ms finality. EVM-compatible. Charges fees in stablecoins (no native gas token). Validator set: Visa (validator node, April 14, 2026), Stripe (Machine Payments Protocol), DoorDash (stablecoin payouts, April 21, 2026), Felix, Fifth Third Bank, Howard Hughes, ARK, OnePay. A payments-industry roster.

**Why Tempo is not a competitor.** Tempo moves money. Nunchi verifies who is allowed to act and proves they acted within policy. Analogy: Nunchi is to Tempo what Visa's network rules and chargeback infrastructure are to Visa's payment rails.

**Structural evidence for complementarity.** Nava itself. Nava's parallel chain on Tempo shows the natural architecture: agent trust verification (Nunchi / Nava layer) ABOVE payment rails (Tempo layer). Note: Nava is NOT a Tempo design partner — the parallel Tempo deployment is a positioning move, not an integrated partnership.

**Risk.** Tempo's permissioned validator set could theoretically adopt KYA (Know Your Agent) standards directly, foreclosing the agent identity trust layer if they build it natively. Real but low probability given Tempo's commercial focus on moving money, not verifying behavior.

**Investor FAQ answer.** *"Tempo is below us in the stack. We are the policy enforcement layer that sits between the agent and the payment. Nava validates this architecture — they are building the same layer on top of Tempo."*

---

### OpenAI Codex CLI — Apache 2.0, ~95% Rust

**The flagship-product existential repositioning trigger.**

**Verified facts.** Apache-2.0 license. ~95% Rust. 75K+ GitHub stars. 640+ releases. Apple Seatbelt + Linux Landlock + seccomp sandboxing. Native MCP client + server. **14.5M npm monthly downloads. 3M weekly active users.** OpenAI confirmed on March 19, 2026 that ChatGPT + Codex + Atlas are merging into a single desktop superapp under Fidji Simo.

**Existential impact.** OpenAI now ships the canonical Apache-2.0 Rust agent runtime as a flagship product. *"Apache-2.0 Rust agent runtime" alone no longer defines Roko's market position.* Roko must lead with the **4-pillar differentiation**:

1. **Adapter-trait architecture** — Roko's 18-crate Bevy-style design lets users swap inference, queue, observability, and integration layers. Codex CLI is structurally coupled to OpenAI auth and OpenAI models.
2. **Model-agnostic from day one** — Roko speaks OpenAI-compatible HTTP (vLLM, SGLang, LiteLLM, mistral.rs), `ollama-rs` for local, and Anthropic / Vertex independently. Codex is OpenAI-only.
3. **EU sovereignty and self-hostability** — Berlin-built, no US-cloud control plane required, CRA-aligned. Codex requires OpenAI infrastructure.
4. **Integration depth** — Linear AgentSession with 5 / 10s budget hardened, Slack-thread-to-trace-URL via `slack-morphism`, Sentry adapter. Codex is an IDE / CLI, not an integration runtime.

**These four pillars must appear in the same paragraph for every external piece going forward.**

**Apache-2.0 niche uncontested for runtimes.** Among credible Rust agent frameworks, rig-core (0xPlaygrounds) is MIT, swiftide is MIT, llm-chain is MIT and effectively dormant. The Apache-2.0 niche is uncontested for runtimes (Codex CLI occupies it for IDE-attached coding agents — structurally different). Real legal-procurement moat for enterprise customers whose legal teams require explicit patent grants.

**What to say.** *"Codex is the platform threat. Our answer: we're the open-source runtime they can't ship and the on-chain identity layer they won't ship. Every Codex user who needs cross-org coordination is a Roko user."*

---

### Linear Agent Ecosystem (April 2026)

**11+ named third-party agents have shipped AgentSession integrations** plus Linear's own first-party "Linear Agent" (public beta March 24, 2026):

| Agent | Ship date | Notes |
|---|---|---|
| Cursor | August 2025 | **Broken** — Cursor Forum /158505 |
| Devin | April 8, 2025 | Most feature-complete: real-time activity, plan-tracking, auto PR-URL, 3 triggers, Playbook labels |
| OpenAI Codex | December 4, 2025 | |
| GitHub Copilot | — | Coding agent |
| ChatPRD | May 20, 2025 | Launch partner |
| Codegen | May 20, 2025 | Launch partner |
| Sentry / Seer | — | Partial competitor for Sentry → PR loop |
| Warp Oz | — | |
| Factory.ai | — | |
| Charlie Labs | May 29, 2025 | |
| Reflag | — | |
| Linear Agent (first-party) | March 24, 2026 | Public beta |

**Confirmed absent from Linear:** Aider, Continue.dev, Tabnine, Augment Code, Magic.dev, Tabby, Replit Agent, Cosine Genie 2, Sweep, Claude Code.

**The "Linear-as-gateway" insight.** Three converging data points:
- Cursor's Background Agents Linear integration is broken — clean opening for Roko.
- Sweep AI (7.4k stars, Apache-2.0) opened issue #3669 to add Linear webhook support and pivoted to JetBrains before shipping it. Gap remains unfilled.
- Cosine Genie 2 ships GitHub + Linear together because GitHub-alone retention was insufficient. Empirical evidence that GitHub-only is not enough.

**Two structural amplifiers.**
1. Linear does not bill agents as seats (`linear.app/docs/agents-in-linear`). Roko users get a free distribution channel that does not tax their Linear bill. Pricing-structure moat, not a feature.
2. Linear's AgentSession protocol enforces a 10-second response window before marking the agent unresponsive (Hookdeck blog on Linear agents, accessed April 28, 2026). Roko's Rust runtime can emit a `thought` activity sub-100ms then offload async work — measurable performance moat over Python-based Devin / Cosine.

**The pitch shifts** from "first Rust agent in Linear" to **"Cursor's Linear integration breaks predictably under load, no Rust-native alternative — Roko fills that gap."**

**Side-by-side demo.** Create two identical Linear issues. Trigger `@roko` on one, `@cursor` on the other. Roko emits a `thought` activity within 10s, drives the LLM round-trip, updates the issue cleanly. Cursor spawns duplicate sessions, loses context across follow-ups. Record both side-by-side. Single most effective competitive artifact for enterprise procurement conversations.

---

### Cosine Genie 2 — closed (TypeScript front + Python model)

Ships GitHub + Linear together because GitHub-alone retention was insufficient. Empirical evidence the gateway product surface is Linear, not GitHub. Cosine is closed-source. Roko's delta: open-source, on-prem-able, zero seat cost in Linear.

---

### Sweep AI — Apache 2.0, 7.4k stars, 419 forks

`sweepai/sweep`. Reached 7.4k stars on GitHub-label-trigger alone. Opened issue #3669 to add Linear webhook support and pivoted to JetBrains before shipping it (last commit 2024). Linear gap in the OSS agent ecosystem remains unfilled. Roko's `roko-linear` crate fills exactly this gap.

---

### Sentry Seer Autofix — closed, paid add-on

Ships through GitHub but does NOT close Linear and does not emit `gen_ai.*` spans for its own decisions. Continue.dev's "Sentry Mission Control" cookbook is the documented end-to-end reference. Seer pricing: $40 / active-contributor / month (Sentry, January 2026 relaunch).

Seer's existence is the reason to **demote Sentry integration to last** in the 90-day sequence — integrating last avoids appearing to depend on a competitor's API that may close. Roko's delta: the agent's plan / tool-use itself becomes a span attached to the originating trace ID, closing the loop with "this plan fixed this exact span."

---

### Langfuse — MIT, acquired by ClickHouse January 16, 2026

**Partner, not competitor.**

- 50K observations / month free tier, no card required.
- Ecosystem partner page already lists Cursor, n8n, Spring AI, AutoGen, OpenAI Agents SDK, Mastra.
- Coexistence guide for Sentry / Datadog / Honeycomb — Roko + Langfuse + Sentry recipe is structurally welcomed.
- 20K+ GitHub stars. 26M+ SDK installs / month. 6M+ Docker pulls. 19 of Fortune 50 customers.
- Acquired by ClickHouse January 16, 2026 (NOT a Series A as prior rounds held). Licensing remained MIT, free tier intact.
- **2027–2028 re-license risk watch item** a la Elastic / MongoDB / HashiCorp. Mitigation: vendor-neutral OTLP plumbing means Roko's instrumentation is trivially portable if ClickHouse re-licenses.

**Lock the Langfuse partnership in the next 30 days** — co-published blog post, inclusion in their "agent runtimes" partner page. Cost: zero. Leverage: Langfuse's 26M+ SDK-installs-per-month traffic as free distribution channel.

---

### Arize Phoenix — Elastic License 2.0 (NOT Apache-2.0)

License correction: Arize Phoenix is **ELv2 + US patents**, not Apache-2.0. Source-available, no-managed-service restriction. The OpenInference instrumentation libraries at `github.com/Arize-ai/openinference` are separately Apache-2.0. **The "license symmetry with Roko Apache-2.0" argument for Phoenix collapses.**

Phoenix remains viable as backup observability partner for sovereign / on-prem buyers but flips the recipe default decision back to Langfuse with vendor-neutral OTLP.

---

### Helicone — out of consideration

Acquired by Mintlify March 3, 2026, now in maintenance mode. Langfuse has published a Helicone-to-Langfuse migration guide. **Laminar** (lmnr.ai, Apache-2.0, OTel-native, agent-first UI) is the credible smaller alternative.

---

### Vibe Coders — Lovable, Bolt, Replit, v0 — DIFFERENT CATEGORY

**Verified ARR data:**

- **Lovable** hit $100M+ ARR within ~6 months of launch (The Information, August 2025) — the fastest 0-to-$100M in SaaS history before Cursor surpassed it.
- **Replit** crossed $50M+ ARR in 2024; shifted heavily toward usage-based agent invocations.
- **Bolt.new** (StackBlitz) publicly disclosed $20M+ ARR in 2025.
- **v0** by Vercel is bundled with Vercel Pro; revenue not disclosed separately.

**Why this is NOT Roko's market.** Vibe-coders are non-developers building landing pages, MVPs, internal tools. They explicitly do not want gates, verification, or routing — they want one-shot generation. Different buyer, different price point. **Skip explicitly in the pitch.**

---

### Factory / Cosine / Augment / Poolside / Magic — funded ahead of revenue

| Company | Raised | Estimated ARR (Q1 2026) | Notes |
|---|---|---|---|
| Factory.ai (Browser-Use) | $19M Series A (Sequoia, rumored) | <$10M | Positioning shifted 3x in 18 months |
| Cosine (Genie) | $25M total (rumored) | <$5M | SWE-bench results contested |
| Augment | $227M Series B (Sutter Hill) | $20–40M | Strongest of cohort, enterprise-only |
| Poolside | $500M+ | Pre-revenue at last round | Pivoted from foundation model to enterprise agent |
| Magic.dev | $465M at $1.58B (Forbes) | Product not GA | Major credibility issue in dev community |

**Aggregate bear case:** ~$1.5B+ raised across the cohort, likely <$100M aggregate ARR. The single biggest bear case Roko should pre-empt: *"The AI coding agent space is funded ahead of revenue."*

**Pitch line.** *"We're not betting on the autonomous agent thesis. We're betting on the coordination layer. The autonomous agent companies fight Cursor and Copilot for the same buyer. We sell to a different buyer (the platform engineer / FDE / compliance officer) with a different value prop (verification, audit, multi-agent coordination)."*

---

### Adjacent: vertical agents that win

Funded vertical agents validate that agent infrastructure is a real category. They are not direct competitors — different layer.

- **Harvey** (legal AI). Step-up trajectory $715M → $11B over 28 months.
- **Decagon** (customer-support agents). $650M → $1.5B → $4.5B over 15 months.
- **Sierra** (Bret Taylor). $1B → $4.5B → $10B over 19 months.
- **Hebbia** — a16z portfolio (Casado-led Series B). Sivulka publicly claims >2% of OpenAI daily volume. Matrix orchestrates o1 / o3 / GPT-4o in parallel.

These step-ups reflect the current market's willingness to pay for enterprise AI infrastructure with demonstrated revenue. The framing for investors: Nunchi's infrastructure position is orthogonal to any single vertical AI product, so it captures value across the entire portfolio of enterprise AI deployments rather than from one specific use case.

---

### Adjacent identity layer

- **Keycard** — $38M (October 2025; co-led at a16z including Aubakirova). Identity authentication ("Auth0 moment for agent access"); identity-bound, task-scoped tokens via OAuth 2.1 Client ID Metadata Documents inside one trust domain. Nunchi extends Keycard's pattern across the trust boundary via ERC-8004. *"Same axis (static identity → dynamic intent), perpendicular extension (centralized issuer → sovereign verification)."* Customer, not competitor — this is the layer-cake play.
- **Astrix** — $85M (rumored Cisco acquisition at $250–350M). Enterprise NHI for humans. Different layer.
- **Catena Labs** — $18M (May 2025). Adjacent.
- **Oasis Security** — $120M.
- **GitGuardian** — $50M Series C (February 2026), pivoting to AI agent NHI security.

The market is not speculative — capital is already moving toward agent identity infrastructure at significant scale. Nunchi's positioning seam: Keycard handles intra-organizational machine identity; Nunchi handles cross-organizational reputation and settlement.

---

### Other agentic chains

- **Olas / Autonolas** — $13.8M (February 2025). Off-chain agent framework on Gnosis. **>9.9 million lifetime Mech requests, ~400 daily active agents, sub-cent per-request fees, top mechs earning $10s–$100s per month in sustained revenue.** Demonstrates that on-chain agent infrastructure works; revenue numbers show the market is early-stage, not mature. Integration partner.
- **Bittensor (TAO)** — ~$2.5B market cap. Decentralized ML training. Not coordination. Use as a reputation-corrosion case study.
- **0G Labs** — $359M committed. AI compute / storage / DA. Complementary ("AWS for AI"), not coordination.

---

### a16z portfolio — design partner targets (priority ordered)

| Rank | Customer | Why | Access |
|---|---|---|---|
| 1 | **Hebbia** | a16z portfolio (Casado-led Series B). >2% of OpenAI daily volume. Matrix orchestrates multiple frontier models in parallel | Casado warm intro |
| 2 | **Harvey** | Public job req for "Context Engineering & Agent Infrastructure." High monthly LLM spend | Pereyra direct (LinkedIn-active) |
| 3 | **Decagon** | a16z portfolio. Hiring Staff SWE Agent Orchestration. Outcome-based pricing makes cost reduction existential | Casado warm intro |
| 4 | **Sierra** | $10B valuation. Outcome-based pricing makes cost reduction existential. Multi-model supervisor architecture | Casado-to-Taylor warm path |

**Anti-customer.** Cognition (Walden Yan's "Don't Build Multi-Agents" post). Do not pursue.

**Unexpected leverage.** Replit (gross margins reported 36% to –14% from agent costs) and Cursor (moved to credit-based pricing because the request model did not reflect compute costs). Both bleed from exactly what Nunchi fixes. Not design partners — potential integration targets where Roko's CascadeRouter directly addresses margin compression. Strong "why-now" proof points for investor conversations.

---

## 4. The Killer Demo Gap

Across Cursor Background Agents, Devin, Cosine Genie 2, and Sweep — **none currently paste an inline observability trace URL back into the Slack thread that triggered the agent run.** They paste PR links, post progress messages, but none show the human a trace with `gen_ai.*` spans, token cost, and tool calls without leaving Slack.

This is Roko's single most differentiated demo. The narrative: *"Cursor and Devin show you what happened. Roko shows you why, with receipts."*

Implementation: ~20 minutes of recipe configuration using `slack-morphism` (Rust, MIT, 1.84M+ downloads on crates.io) socket-mode bot + the `genai-rs/opentelemetry-langfuse` Rust crate (or `opentelemetry-otlp` directly with basic-auth header from env vars). Langfuse partnership makes this free to demo (50K observations / month free tier, MIT-licensed).

---

## 5. Macro Funding Context

**$2.66B across 44 rounds YTD 2026** versus $1.09B / 71 rounds in the same period 2025 — a 143% dollar increase in fewer, larger rounds. Capital is consolidating into conviction bets. Partners at the table will know this number. It means: fewer companies are getting funded, but each is getting more capital. Favors Nunchi's Series A timing if conviction is there — investors are writing bigger checks into fewer companies, which means fewer competing deals but higher bars for traction.

**Google Cloud $750M agentic partner fund (April 22, 2026)** validates the investment thesis at the hyperscaler level. Google is funding the ecosystem around agents, not just building agents themselves. Distribution tailwind for coordination infrastructure.

**A2A v1.0 stable (April 9, 2026)** — Signed Agent Cards are now a primitive every coordination pitch should assume. A2A provides discovery and communication; Nunchi provides the coordination, routing, and verification layer that A2A explicitly lacks.

---

## 6. Three Strategic Implications

The competitive reality in April 2026 is that no single competitor has the coordination-plane positioning. The market has bifurcated into two camps:

1. **IDE-native agents** (Cursor, Copilot, Codex) — bundled with the editing environment, competing on UX and model quality. $1B+ ARR territory but closed ecosystems.
2. **Standalone autonomous agents** (Devin, Factory, Cosine, Poolside, Magic) — funded ahead of revenue (~$1.5B raised, <$100M aggregate ARR), competing on capability claims.

Nunchi / Roko occupies neither camp. The coordination plane sits underneath both, providing the routing, verification, and identity infrastructure both camps need but neither will build. The unbundling thesis (router + gates + cost optimizer as standalone services) is the near-term revenue play; the full coordination plane is the platform play.

---

## 7. How to Present the Competitive Slide

**The Hunter Walk problem with 2x2s** (above): never show a 2x2 where the startup is in the upper right.

**Two formats are defensible:**

**Option A: Harvey-Ball feature table** (recommended). Filled / half-filled / empty circles across specific capabilities. Reads as more honest than a 2x2 because it does not claim a single "winner" position. Acknowledges competitors are strong on some dimensions. Recommended columns: open-source runtime, sovereign L1, ERC-8004 full stack (7-domain reputation + ZK-HDC), production code shipped, Series A capital. Nunchi wins on all five. Nava is strong on EigenLayer pedigree and press, weak on runtime and ZK-HDC.

**Option B: 2x2 with non-obvious axes.** Works only if the axes name something genuinely orthogonal that the investor has not seen before. Strong axis example: "cross-org knowledge sharing" vs. "ZK behavioral verification" — axes that describe technical architecture and on which Nunchi's position is provable, not asserted.

**The Snowflake precedent.** Snowflake acknowledged AWS / GCP as both partners and competitors in the S-1, then differentiated on multi-cloud + decoupled storage / compute. Acknowledgment of competitor strengths increased credibility.

**Power Grid format.** A grid with named competitors as rows and capabilities as columns, with filled / half / empty indicators, reads as more honest and information-dense than a 2x2 quadrant where the startup is always in the upper right.

---

## 8. Deck Comparables — Traction Slides That Worked

| Company | Round | Traction shown at raise | What worked |
|---|---|---|---|
| **Temporal** | Series A (October 2020) | Snap, Box, Coinbase, Checkr (named non-paying production) + v1.0 GA + AWS SWF / Cadence pedigree | Logos, not revenue or stars. *"These sophisticated engineering teams chose us voluntarily."* |
| **LangChain** | Series A (February 2024) | 75–80K stars, 7M downloads, 70K LangSmith signups, Rakuten / Elastic / Moody's / Retool | Combined community scale + named enterprise + closed-beta conversion |
| **Mastra** | Seed (2026) | 22K stars (1.5K → 7.5K in one HN week), 300K weekly npm downloads matching Gatsby's 4-year peak in 1 year, Replit / SoftBank / PayPal / Brex | Velocity anchored to a named comparable |
| **E2B** | Series A ($21M Insight, May 2025) | "Hundreds of millions of sandboxes," 88% Fortune 100, HuggingFace / Perplexity / Groq / Manus | Usage-depth metric + Fortune 100 penetration |

**Pattern.** Every successful deck leads with named logos and usage depth, NOT lines-of-code, GitHub stars alone, or self-referential technical milestones. Stars only work when the slope is genuinely steep AND anchored to a comparable.

---

## 9. Summary

| Competitor | Funding | Direct overlap | Position |
|---|---|---|---|
| Temporal | $5B (Series D, $220M, Reuters Aug 12 2025; Sarah Wang led) | Durable execution | Build on, not replace. Volunteer the boundary |
| LangChain | $1.25B (Series B, October 2025) | Framework | Underneath. Migration target |
| CrewAI | $18M | Multi-agent orchestration | Underneath. Topology compatible with Roko runtime |
| Mastra | $22M Series A (2026) | Agent framework | Underneath. Roko is the runtime substrate |
| Cursor | $1B+ ARR (Reuters June 5, 2025) | a16z portfolio. NOT competitor | CascadeRouter makes Cursor cheaper |
| Braintrust | $300M Series B (TechCrunch May 2025; Casado-led) | a16z portfolio. NOT competitor | Eval feeds gate pipeline |
| Devin / Cognition | ~$73M ARR at $10.2B post-money (The Information March 2025) | Autonomous coding | Not a design partner. Bear case to pre-empt |
| Nava | $8.3M seed (April 14, 2026; Polychain + Archetype) | Trust layer (HIGH) | Adjacent category. Validates market. Differentiate on full L1, ZK-HDC, runtime, capital |
| Tempo | $500M at $5B (October 2025; mainnet March 18, 2026) | Payments rail | NOT competitor. Below Nunchi in stack |
| OpenAI Codex CLI | flagship | Apache-2.0 Rust runtime | Existential collision. Lead with 4-pillar differentiation |
| Capsule | $7M seed (April 16, 2026) | Trust layer | Adjacent. Acknowledge by name |
| t54 Labs | $5M seed (Ripple + Franklin Templeton) | Trust layer | Adjacent. Same category Nunchi exits |
| Keycard | $38M (October 2025; a16z, Aubakirova co-led) | Identity layer | Customer, not competitor. Sovereignization argument |
| Astrix | $85M (rumored Cisco acquisition $250–350M) | Enterprise NHI for humans | Different layer |
| Sycamore | $65M | Agent OS | Different category Nunchi exits |
| /dev/agents | $56M @ $500M pre-product (Index + CapitalG) | Agent OS | Different category |
| Olas / Autonolas | $13.8M (February 2025) | Off-chain agent framework on Gnosis | Integration partner |
| Bittensor | ~$2.5B mcap | Decentralized ML training | Not coordination. Reputation-corrosion case study |
| 0G Labs | $359M committed | AI compute / storage / DA | Complementary. "AWS for AI" |

The empty quadrant is unfilled. The window is 6–12 months. Series A capital advantage ($20–30M target vs. Nava's $8.3M seed) plus production runtime (~177K LOC of Rust shipped, devnet validated) plus the Aubakirova-named-the-category positioning = the credible Series A path to occupying the coordination plane category before it locks in.
