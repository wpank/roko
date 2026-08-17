# Strategy and Narrative

This document is the consolidated pitch arc — the story Nunchi tells, in what order, with what evidence, to what audience. It contains the four primitives the demo must prove, the SDN analogy framing, the Cinderella wedge, the boundary statements to volunteer in the first ten minutes, the anti-pattern checklist, and what an investor should walk away believing.

It supersedes the earlier `05-strategy-and-narrative` document.

For the underlying thesis (the what and the why-now), read `01-thesis.md` first. For the full competitive map, read `03-competitive-landscape.md`. For the benchmark-business thesis (ISFR as the long-term expansion, not the Series A wedge), read `04-benchmark-business-thesis.md`.

---

## 1. The Repositioned Pitch (April 2026)

Earlier framings have been retired. The current category is **Agent Coordination Plane**. The architectural noun is **Cooperative Clearing**. The thesis is **"the model is the same; the system is the variable."** The beachhead is enterprise support contracts on Roko OSS.

| Retired framing | Date retired | Reason |
|---|---|---|
| The earlier payment-rail analogy | April 2026 | Stripe co-founded the x402 Foundation on April 2, 2026 and launched the Agentic Commerce Protocol with 60+ partners. Building toward that positioning meant building toward an incumbent rather than toward a gap. |
| "Trust layer for agents" | April 2026 | Seven or more companies claim it: Capsule ($7M seed, April 16, 2026), Nava ($8.3M seed, April 14, 2026), t54 Labs ($5M seed, Ripple + Franklin Templeton), Gen Digital's Agent Trust Hub, the CSA framework, plus stealth entrants. Nunchi would be the seventh entrant, not a category creator. |
| "Agent Operating System" | April 2026 | Six or more claimants: Sycamore ($65M), /dev/agents ($56M at $500M pre-product, Index + CapitalG), PwC, AIOS (Rutgers). The "OS" framing implies platform replacement; the coordination plane sits beneath execution. |
| "Identity layer" | April 2026 | Identity is a feature of the coordination plane, not the category. |

The pitch line replaces all of the above:

> *"Aubakirova wrote in a16z's Big Ideas 2026: 'the bottleneck becomes coordination: routing, locking, state management, and policy enforcement across massive parallel execution.' We're building the canonical implementation. Nunchi is the Agent Coordination Plane — the layer that separates agent coordination from agent execution, the same way SDN separated the control plane from the data plane."*

The category name follows the pattern of durable infrastructure categories — descriptive structural names that imply mechanical separation or unification (SDN, lakehouse, analytics engineering). It does not reward aspirational nouns like "trust," "intelligence," or "autonomy." "Agent Coordination Plane" is structural: it separates coordination from execution, the same structural move SDN made one layer up the stack.

---

## 2. The Three-Level Pitch

Different audiences need different depths. Memorize all three.

**The thesis (one sentence).** *"The model is the same. The system is the variable."* Princeton HAL (ICLR 2026, arXiv:2510.11977): 50x cost variation between agents at similar accuracy on the same tasks. Two teams running the same model on the same task can have 10x cost difference and 40-point reliability difference based purely on how they instrument the system.

**The network effect (one sentence).** *"The thousandth agent joins smarter than the first."* Every agent that runs on Nunchi deposits signals into the shared HDC-indexed knowledge store. Cross-organizational intelligence is structurally unavailable to single-tenant orchestration platforms.

**The alternative close (one sentence).** *"The model is commoditizing. The knowledge is not. We are building the network that compounds it."*

---

## 3. The Four Primitives the Demo Must Prove

Every demo, every deck, every conversation must prove exactly four primitives. If all four land, the story is complete. If any one is missing, the narrative has a hole.

### Primitive 1: Identity (default-off, verified machine identity)

Every agent has a verifiable non-human identity. Before spending a single token, policy gates fire: PII scan, cost ceiling, compliance checks. Nothing runs without passing policy.

**Why it matters.** The ratio of machine identities to human identities in enterprise infrastructure is 82:1 (CyberArk 2025) to 144:1 (Entro Security 2025). AI agents are the fastest-growing category of machine identity but they authenticate with static API keys built for humans. EU AI Act Article 50 enforces August 2, 2026; every AI system must disclose its nature and maintain audit trails. Penalties: €15M or 3% of global turnover for transparency violations under Article 99(4)(g).

**What the investor sees.** A verified agent identity line in the terminal output. Gate checks passing before any work begins. The `nhi://` scheme that no other tool uses.

### Primitive 2: Cost prediction (predict, actual, delta)

The system predicts what a task will cost before execution, routes to the cheapest model that can handle it, and self-corrects after execution. Every execution improves the predictor.

**Why it matters.** Princeton HAL: 50x cost variation between agents at similar accuracy. The difference is the system around the model. Nunchi's stacked optimization — prompt caching (~5x), CascadeRouter tier routing (~3x), gate-based early stopping (~2x) — delivers 10–30x practical cost reduction, validated by the Cline / Uber production case study (April 24, 2026: $18 versus $720 on the same coding task, ~40x).

**What the investor sees.** A prediction line showing expected cost, then an actual line showing real cost with the delta. The second agent running cheaper than the first because it loaded knowledge from the first.

### Primitive 3: Shared knowledge (agents learn from past agents)

Agents working in the same domain share knowledge automatically. Agent A publishes findings; Agent B — a different agent entirely — loads those findings and starts ahead. Knowledge is scored, timestamped, attribution-tagged. Stale knowledge decays via Ebbinghaus-style forgetting. The thousandth agent joins smarter than the first.

**Why it matters.** This is the network effect. Every other agent framework is single-session: knowledge dies when the process ends. Nunchi's knowledge substrate compounds across agents, sessions, and organizations. A competitor copying the routing logic starts with an empty knowledge store. The knowledge is the moat.

**What the investor sees.** The knowledge line in the second agent's output: `loaded 9 facts from 4 agents, 0.93 avg confidence`. The cost dropping because knowledge was reused instead of re-derived.

### Primitive 4: Durability (zero work lost)

Kill the agent mid-run. Resume from the last checkpoint. Zero tokens wasted. State is persisted after every completed step.

**Why it matters.** Temporal built a $5B company (Series D, $220M round, Reuters August 12, 2025) on the premise that workflows should survive infrastructure failure. Agent workloads are workflows. Every other agent demo carefully avoids failure. Nunchi's demo deliberately embraces it — the kill-and-resume moment is the point where the room shifts.

**What the investor sees.** A visible Ctrl+C killing a running agent. A two-second pause. A resume command picking up exactly where it stopped. The cost meter continuing from where it paused, not restarting from zero.

---

## 4. The Wedge: Cost Reduction Gets Developers in the Door

Three independently documented mechanisms stack multiplicatively to deliver 10–30x cost reduction. The full evidence chain is in `01-thesis.md §5.3`; the strategy summary:

- **Prompt caching (~5x).** Anthropic prefix caching at up to 90% discount on cached input tokens. ProjectDiscovery 7% → 74% → 84% across 9.8B cached tokens (engineering blog). LMCache 92% Claude Code (third-party, December 2025). Anthropic 99.8% on a specific internal pipeline (April 23, 2026 postmortem).
- **Tier routing (~3x).** RouteLLM (Princeton, arXiv:2406.18665): 85% cost reduction retaining 95% of GPT-4 quality.
- **Gate pre-screening (~2x).** The 11-gate / 7-rung pipeline catches malformed work in 3 turns instead of 15.

**Stacked: ~5x × ~3x × ~2x = ~30x theoretical, 10–20x practical** after routing overhead, cache misses, and gate false positives. Princeton HAL (ICLR 2026, arXiv:2510.11977) confirms the 50x ceiling. The Cline / Uber production case (April 24, 2026) confirms a ~40x spread on real coding tasks ($720 → $18).

**The honest demo framing.** Princeton HAL costs do not include caching benefits. HAL is therefore an upper bound on production cost, not a production cost estimate. In production with standard caching alone (~4–5x at 80–90% hit rate), the same workload costs roughly 4–5x less than HAL's published numbers; Nunchi's full stack (caching + tier routing + gate-based early stopping) brings it lower still. Disclose the intermediate step — credibility scales with specificity.

---

## 5. The Moat: The Coordination Plane Is What Keeps Them

Cost reduction drives acquisition. The coordination plane drives retention and expansion. Identity and reputation are *features* of the coordination plane, not the category itself.

### Five compounding layers

1. **Open-source composability.** Apache 2.0 Roko runtime, 18 crates. Every primitive replaceable. Forks validate the protocol; they do not threaten it. The protocol is the moat, not the runtime.
2. **Knowledge compounding.** Every Signal carries an HDC fingerprint. Every episode joins the shared substrate. The thousandth agent joins with the distilled experience of the previous 999. Single-tenant orchestration cannot replicate this — it has only its own signals.
3. **Protocol lock-in.** ERC-8004 identity. ERC-8183 marketplace. Once an agent has reputation on chain, switching costs are real. Reputation is portable across operators but not across protocols.
4. **Niche construction** (Odling-Smee, Laland & Feldman, *Niche Construction: The Neglected Process in Evolution*, Princeton University Press, 2003). Each agent that improves the substrate makes the next agent more effective. Returns are compounded, not linear. The platform co-evolves with its users.
5. **Regulatory tailwind.** EU AI Act Article 50 enforces August 2, 2026. Article 99 penalties up to €35M or 7% of global turnover for prohibited practices and €15M or 3% for transparency violations. Enterprises need an audit trail by deadline. The coordination plane is the audit trail by construction.

### The moat stack ranked by compound rate

| Rank | Moat | Compound mechanism | Honest assessment |
|---|---|---|---|
| 1 | Standards positioning + workflow-embedding | Reference implementer of MCP / A2A / ERC-8004 + Roko TOML config drives switching costs growing with every plan definition | Tactical today (credibility, not lock-in); meaningful by end of 2027 |
| 2 | Ecosystem + workflow-embedding | Adapter count × workflows per customer | Sub-50 adapters today: no moat. 200–500: meaningful (months to migrate). 500+: prohibitive (multi-quarter migrations) |
| 3 | Data + cascade router | Routing observations → better router → more usage | 6–12 month moat at best. Do not lead with it |
| 4 | Chain identity + audit trail | Regulatorily anchored. Leaving means losing compliance history | Strongest single moat once Article 50 deployed |

**Closing line:** *"Our moat stack: standards (MCP / A2A / ERC-8004 reference implementation) feeds workflow-embedding (Roko TOML format) feeds ecosystem (adapter marketplace) feeds data (cascade router learning) feeds chain (audit trail lock-in). Each layer compounds independently. The bottom of the stack — the chain — is the regulatorily-anchored moat."*

---

## 6. The Defining Demo Artifact

The defining artifact of the demo: `nunchi run --share` produces a URL in approximately 10 seconds.

The URL is a full execution timeline with cost breakdown, gate results, agent identity, knowledge provenance, and (when the chain is live) a ZK-HDC proof. The investor opens it on their phone during the meeting. They forward it to their partner after the meeting. It is the Vercel preview URL for agent runs — the artifact that leaves the room.

The pattern is modeled on Stripe's "7 lines of code" moment (the famous Patrick Collison habit of saying "right then, give me your laptop" and integrating Stripe on the spot) and Vercel's preview URL. The difference: Nunchi's URL is a verifiable computation receipt, not just a deployed artifact. Identity, cost, knowledge, and durability collapse into one shareable artifact.

---

## 7. The SDN Analogy in Detail

The structural analogy that defines the category for any infrastructure-fund partner:

**Before SDN.** Each router made its own forwarding decisions using locally computed routing tables. Changing network policy required configuring every router individually. The control plane (routing decisions) was fused to the data plane (packet forwarding) in every device.

**SDN insight.** Separate the control plane from the data plane. A centralized controller maintains global network state and pushes forwarding rules to dumb switches. Switches forward packets; the controller decides where packets go. This separation created Nicira (Martin Casado's company), which VMware acquired for $1.26 billion in 2012.

**Before Nunchi.** Each agent makes its own decisions using local state — which model to call, which tools to use, how to validate output, what context to include.

**Nunchi insight.** Separate the coordination plane from the execution plane. Roko (the runtime) provides coordination: model routing, gate validation, knowledge management, session persistence. Agents execute; Roko coordinates. At scale, the Nunchi blockchain extends coordination across organizations.

**The platform-vs-product framing** (Casado, Open Networking Summit 2017): *"Customers don't buy platforms; customers buy products. I think if you focus on the product, you build a viable business, and then for stickiness, you turn that into a platform."* Roko is the product. Nunchi is the platform.

**The naming pattern.** Kate Greene coined "SDN" at MIT Technology Review (2009). Casado commercialized it at Nicira. Aubakirova named "coordination plane" in a16z's Big Ideas 2026 essay (December 2025). Nunchi commercializes it. The SDN-Nicira playbook applied one layer up the stack.

---

## 8. The Cinderella Wedge Framing

From Aubakirova's *"The Cinderella 'Glass Slipper' Effect: Retention Rules in the AI Era"* (December 8, 2025):

> *"In AI, achieving product-market fit may literally mean solving one high-value workload better than anyone else."*

This governs the entire go-to-market. Do not pitch "agent infrastructure" (generalist). Pitch one workload — verifiable similarity for cross-org agents under cooperative clearing — completely. The glass slipper is the one workload you solve completely; it creates the distribution mechanism for everything else.

**Glass-slipper precedents:**

- **Vanta** — SOC 2 automation was the glass slipper. Reached approximately $220M ARR by July 2025 at $4.15B valuation (TechCrunch, July 22, 2025).
- **HashiCorp** — Vault (one secret-management product for one buyer) was the glass slipper. $6.4B IBM acquisition closing February 2025 (SEC filings).
- **Temporal** — durable execution for one workflow pattern was the glass slipper. $5B Series D, August 2025 (Reuters, August 12, 2025).

Nunchi's glass slipper: **cost-aware agent routing with verifiable identity and shared knowledge.** Specific enough to solve completely, valuable enough to sell on its own, positioned at the exact intersection of three converging forces — protocol convergence, regulatory trigger, empirically proven cost reduction.

The direct pitch line: *"We're not building agent infrastructure. We're solving one stubborn coordination workload — verifiable similarity for cross-org agents — completely."*

---

## 9. The Beachhead: Enterprise Support, Not a Platform Product

The first dollar comes from **enterprise support contracts on Roko OSS, not a managed platform product.**

**Temporal precedent.** Temporal raised $18.75M Series A (October 2020) with zero commercial product. Cloud did not exist. Revenue began as enterprise support on the open-source runtime. The 1,000th paying Cloud customer arrived April 2024 — 3.5 years later. Temporal's first customers (Snap, Box, Coinbase, Checkr) were Cadence-graduation customers inherited from the Uber fork, not greenfield design partners. The directly portable tactic: services-attached-to-software ("free integration engineering against your team's existing GitHub + Linear + Slack stack").

**HashiCorp precedent.** Mitchell Hashimoto on HashiCorp's first four years: *"no real business for four years, first commercial product was a failure."* Atlas tried to sell the whole stack; Vault Enterprise (one product, one buyer) won. The wedge was one specific security pain point, not the platform.

**Pricing structure (3-tier MVP):**

| Tier | Price | What | Engineering effort |
|---|---|---|---|
| 1: Production Support | $24K/yr per design partner ($2K/mo) | Private Slack channel, 24-hour SLA on critical bugs, 2 architecture review calls per quarter, priority on adapter authoring | Zero — Slack and calendar only |
| 2: Custom Adapter | $10–25K fixed-fee per adapter | Commission-built adapter; IP returns to OSS | 4–8 weeks per adapter |
| 3: Cloud Early Access | $499–1,499/mo | Single-tenant managed Roko, hand-deployed | Defer 60–90 days post v1 |

**90-day target:** 2 signed Tier 1 contracts ($48K ARR) + 1 Tier 2 adapter contract ($15K bookings) = **$63K bookings in 90 days, $48K+ ARR run-rate, zero CAC** (all inbound from OSS). Structurally identical to Temporal's early-2021 position; they closed Series B at $1.5B valuation roughly six months later.

**Modern monetization timing.** The HashiCorp four-year wait is a pre-cloud-GTM artifact. Temporal monetized within ~18–24 months of founding (first paying design partners by early 2021, Cloud GA February 2022 at three years). Modern infra OSS monetizes at v1, not at year four.

**Avoid:** percentage-of-savings pricing (attribution unauditable; perversely rewards bad baselines). Charge per-action in USD.

**Compliance is distribution, not revenue.** Combined ARR across all four pure-play AI governance vendors is under $50M today. Compliance creates the buyer role and the urgency, but the buyer pays for the runtime that makes compliance automatic — not for the compliance product itself. Article 50 gets you in the door; Roko + the chain keeps you there.

---

## 10. The Two Wedges at a Glance

| Wedge | Purpose | Mechanism | Time horizon |
|---|---|---|---|
| **Wedge 1: Cost reduction (10–30x)** | Gets developers in the door | Prompt caching (~5x) × tier routing (~3x) × gate-based early stopping (~2x); Princeton HAL 50x ceiling, Cline/Uber 40x production case | First contact, immediate |
| **Wedge 2: Coordination plane** | Keeps them | Shared knowledge substrate, ERC-8004 identity, Cooperative Clearing, ZK-HDC proofs, 7-domain reputation, compliance-native audit trail | 6–18 month sales cycle, retention compound |

---

## 11. Anti-Patterns: What Not to Say

These specific phrases will backfire in any infrastructure-fund room.

| Anti-pattern | Why it fails | Replacement |
|---|---|---|
| "Agent infrastructure" | Generic; every framework claims it | "Agent Coordination Plane" |
| "We're contrarian" | The published canon of leading infrastructure GPs explicitly rejects non-consensus investing as a value claim. Build a non-consensus product, give a consensus pitch | Consensus pitch (layer cake, infrastructure-orchestration multiples, Pinecone+LangChain layer-cake precedent); non-consensus product |
| Crypto lead | Casado has no public canon on sovereign EVM L1s, and operates separately from a16z crypto | Frame the chain as "identity and settlement infrastructure for autonomous agents." Use the "vertical clouds" framing: *"Vertical clouds, which are entirely focused on a specific type of workload, tend to be far more sophisticated, far more cost effective, and far more performant"* |
| Criticize portfolio companies | Cursor, Convex, Netlify, Kong, Truffle, Pindrop, Fivetran, Material Security, Ideogram, World Labs, Fly.io, Braintrust are Casado-associated boards | Position complementary. *"Cursor agents through CascadeRouter cost 3–5x less. Braintrust evals feed our gate pipeline."* |
| The earlier payment-rail analogy | Stripe co-founded the x402 Foundation April 2, 2026 | SDN analogy |
| "AGI" | Casado calls AGI talk "lazy thinking" | Stay grounded in measurable production properties |
| "Agent autonomy" as a feature | Casado is skeptical of open-loop systems | Frame autonomy as bounded execution under explicit policy and gates |
| "Data moat" | Published a16z view: data is rarely a real moat unless replication takes 3+ years | Lead with standards-positioning, workflow-embedding, ecosystem, and chain-audit moats |
| "Web3 platform" | Triggers crypto-skeptic pattern matching | "Infrastructure for [specific industry]" |
| "Tokenomics" | Same | "Incentive design" |
| "Decentralized AI" | Same | "Agent-native infrastructure" |

The vocabulary substitution table is not cosmetic. The institutional default for non-crypto infrastructure is SAFE + token warrants (Mysten Labs, Story Protocol, dYdX). What matters is how the dual-asset structure is framed. The wrong vocabulary triggers immediate pattern-matching to failed projects.

**Story Protocol** pitched as "AI × IP law modernization" — not crypto. **Helium** rebranded the equity entity to Nova Labs; HNT is governed by the Helium Foundation. The institutional default since 2022: equity in the operating company, token warrants attached as a side letter. This separates the investible story from the speculative overlay.

---

## 12. The Productive Tension Bridge

In any infrastructure-fund room there will be productive tension between two published positions inside the firm:

- A security partner's position: *"2026 is the year of agents; identity is the bottleneck."*
- An AI/data GP's position: *"Agents don't really work yet... most agent frameworks today are in the proof-of-concept phase."*

Do not pick a side. Bridge:

> *"Identity is the demand-side bottleneck. Today's frameworks fail on the supply side. Nunchi is the missing coordination and verifiability layer that closes the gap between them."*

Cite the relevant published essays directly when bridging — the audience reads its own colleagues' work, and naming the work signals research depth without taking sides.

The continual-learning literature names three requirements for agents that genuinely learn: the right modules, the right signal, and data quality. Nunchi's NeuroStore (the right module for knowledge), predict-publish-correct (the right signal via continuous error correction), and the gate pipeline (data quality enforcement) are the literal instantiation of those three requirements.

---

## 13. The Boundary Volunteers (Speak Before Asked)

Every infrastructure-fund pitch needs three explicit boundary statements **volunteered before the partner asks**. If a partner has to ask "how is this different from X?", the pitch is on defense. If the founder volunteers the boundary, the pitch owns the narrative.

### vs. Temporal

> *"Temporal is durable execution — they make sure a single workflow finishes despite crashes, inside one company's trust boundary. We love them; we run on them. Their own April 2026 blog says 'the agent framework handles the AI, Temporal handles the infrastructure.' Nunchi is the layer above: portable agent identity, shared memory, cost-aware model routing, and verifiable inter-agent coordination across organizations. Temporal owns 'did this code run.' Nunchi owns 'did the right agent, with the right memory, at the right price, with a receipt the counterparty can verify.' That second problem has network effects Temporal architecturally cannot capture from inside a single namespace — the same way Vercel built $9B on top of AWS Lambda."*

**Routing risk.** Temporal's $5B Series D ($220M, Reuters August 12, 2025) sits in a different practice within a16z than infrastructure / agents. Volunteer the Temporal distinction in the first 10 minutes to keep the conversation in the right partner's lane.

### vs. Keycard

> *"Keycard answers 'is this agent allowed to call this tool right now' — that's the new Auth0. Nunchi answers 'which agent should call which tool, when, on which model, with what context, at what cost' — that's the new control plane. Keycard's per-transaction pricing literally requires a coordination layer above it to drive volume. We're customers of Keycard, not competitors."*

a16z has funded adjacent layers in the same portfolio before (Pinecone + LangChain, Clerk + Keycard). The layer-cake precedent makes this the easiest yes if framed correctly.

### vs. Inngest

> *"Inngest orchestrates workflows; Nunchi coordinates agents across organizations. Same way Temporal handles execution and we handle coordination."*

Inngest is event-driven workflow-as-code inside a single trust boundary. It does not do agent-to-agent coordination, cross-org knowledge sharing, cost-aware model routing, or verifiable identity.

---

## 14. What Investors Should Walk Away Believing

After 10 minutes, the partner should be able to explain to a colleague:

1. **Roko is an agent runtime that compounds.** Every invocation, it gets smarter and cheaper. Competitors start from scratch every time.
2. **The Nunchi blockchain is a purpose-built chain that makes compounding a network effect.** The thousandth agent inherits everything the first 999 learned.
3. **The category — Agent Coordination Plane — is empty.** Five funded layers around it (frameworks, orchestration, evaluation, applications, identity); the coordination plane itself has no funded incumbent. Aubakirova named it; Nunchi builds it.
4. **The window is real and time-bounded.** EU AI Act Article 50 (August 2, 2026) creates the buyer and the deadline. MCP / A2A / ERC-8004 / x402 protocol convergence creates the substrate. The Series A is a bet on becoming the default winner before the window closes (6–12 months).
5. **The cost wedge is empirically proven.** Princeton HAL ceiling at 50x; Cline / Uber production case at 40x; the stacked levers are each independently documented. Cost gets developers in the door — but the moat is the coordination plane.

Market sizing and technology serve the moat narrative. They are proof points, not top-line messages.

---

## 15. Honest Feedback Heard from Investors

Substance from a recent investor meeting (the EV partner; April 30, 2026), generically attributed:

- **"This is not just an Oracle network. A benchmark business requires trust, methodology, governance, and adoption — not just technical infrastructure."** Sharpest pushback on the ISFR framing. Drives the repositioning of ISFR as a *benchmark business* (not an oracle), with the SOFR / VIX / S&P credibility playbook. See `04-benchmark-business-thesis.md`.
- **"A broad-based index might be messy. Something narrower gives credibility."** Drives the lending-only ISFR start (Aave V3 + Compound V3 supply rates) instead of the full four-component composition out of the gate.
- **"What's real vs. what's spec'd?"** The team gave narrative when the partner wanted a structured maturity matrix. Pre-built matrix required for every follow-up call.
- **"What's NOT working?"** The team deflected to the oracle problem rather than offering a crisp top-3 honest fragility assessment. Pre-built risk register required.

The recurring lesson: investor calls succeed when the team comes with the structured artifacts the partner is asking for, not the artifacts the team prefers to show. Match the question; do not pivot to the demo.

**Specific gaps to close before any future call:**

- Token / tokenomics one-pager (even draft).
- Funding ask, valuation, use of funds explicitly articulated.
- Revenue model: how Nunchi captures value (protocol fees, benchmark licensing).
- Competitive landscape grid (see `03-competitive-landscape.md`).
- Roadmap with devnet → testnet → mainnet milestones.
- Top 3–5 production risks, honest, with mitigations.

---

## 16. Strategy Summary

| Element | Content |
|---|---|
| **Category** | Agent Coordination Plane |
| **Architectural noun** | Cooperative Clearing |
| **Tagline** | "The model is the same. The system is the variable." |
| **Analogy** | SDN for agents. Nicira → VMware $1.26B (2012). Aubakirova named it (Big Ideas 2026, December 2025); Nunchi builds it. |
| **Four primitives** | Identity, Cost Prediction, Shared Knowledge, Durability |
| **Wedge** | Cost reduction (10–30x stacked); Princeton HAL 50x ceiling; Cline / Uber 40x production case |
| **Moat** | Coordination plane (identity, reputation, knowledge, settlement, regulatory tailwind) |
| **Glass slipper** | Cost-aware agent routing with verifiable identity and shared knowledge |
| **Defining artifact** | `nunchi run --share` → URL in 10 seconds |
| **Beachhead** | Enterprise support contracts on Roko OSS ($24K/yr Tier 1) |
| **Framing** | One workload solved completely (Cinderella), not agent infrastructure (generic) |
| **Anti-patterns** | No "agent infrastructure," no "we're contrarian," no crypto lead, no portfolio criticism, no payment-rail analogy, no AGI, no "data moat" |
| **Boundary statements (volunteer in first 10 min)** | Temporal, Keycard, Inngest distinctions |
| **Revenue model** | Enterprise support on OSS first, managed hosting second, per-action pricing in USD |
| **Series A target** | $20–30M at $200–400M post-money |
