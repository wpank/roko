# Landing Page Narrative

The Nunchi landing page is not a marketing site with a pitch deck attached. It is a single artifact that does both jobs simultaneously, scaled to two audiences: cold developers (3–5 minute scroll) and technical investors (10-minute self-guided demo). This document specifies the seven-section scroll narrative, the design pattern (ROSEDUST evolved to a high-contrast variant), the deck-vs-landing-page distinction, and the Linear precedent.

---

## 1. The Linear Precedent

Linear's website IS the sell — a scroll-driven product demonstration that communicates taste, speed, and opinionation before you read a single word of copy. Investors who visit the site understand the product philosophy in under three minutes. That is the bar.

**R13 critical: Do NOT mirror the deck.** The deck is persuasion for 10 readers live in a room. The landing page is self-serve qualification for thousands of visitors at 30 seconds each. These are fundamentally different jobs. The deck has narrative arc, emotional beats, and a presenter controlling timing. The landing page must convert a cold visitor who may bounce in 5 seconds. The scroll structure below is optimized for the landing page job; the deck slide order (see `06-business-model-and-tokenomics.md` and `05-strategy-and-narrative.md`) is optimized for the deck job. They share numbers and thesis but NOT structure.

**Landing page pattern (R13):** pain-to-resolution headline + code block above the fold + "reality bar" strip (GitHub stars, downloads, logos — NEVER fake) + three-block triptych with copy-paste code + customer outcome stat + architecture diagram + install CTA.

---

## 2. The Seven-Section Scroll Narrative

### Section 1: Hook / Hero

**Headline:** *"The model is the same. The system is the variable."*

**Subhead:** Nunchi is the Agent Coordination Plane — the infrastructure layer that separates agent coordination from agent execution, making agents cheaper, safer, and smarter as they scale.

**Visual:** Animated split-terminal. Left side shows a cost meter ticking upward to $44.86. Right side shows a cost meter barely moving, stopping at $1.42. Both run the same task. Numbers from the Princeton HAL benchmark (ICLR 2026). The gap is real and reproducible.

**CTAs:** "Get started" — "Read the docs" — "Watch the demo"

**Design note.** The cost meter is the hero visual for developer visitors. It communicates the core value proposition without any text. For the pitch deck, the cost comparison moves to slide 9 (where it has context and lands harder). On the landing page, developers who arrive cold benefit from the number immediately — the developer audience is self-selecting and technical. Kevin Hale warns against animations on opening slides of pitch decks; that guidance applies to the PDF deck, not to the landing page scroll.

### Section 2: The Problem

**Primary stat:** *"41–86% of multi-agent deployments fail. 79% of those failures come from coordination, not model capability."*

**Source:** MAST taxonomy, Berkeley AI Safety, NeurIPS 2025 (arXiv:2503.13657).

**Hook quote:** *"We gave agents intelligence. We forgot to give them infrastructure."* — Aubakirova, a16z, Big Ideas 2026 (use as opening pull-quote before the failure rate stat; sets the emotional frame before the data lands).

**Supporting framing.** The model is not the bottleneck. GPT-4o, Claude Sonnet, Gemini Pro — they all work. The failure happens between agents: routing decisions made without context, knowledge siloed per-session, no shared reputation to determine which agent to trust with which task. You are paying for intelligence and losing it to coordination failure.

**Visual.** Failure rate bar chart with source citation. A second number beneath: *"82:1 — the ratio of machine identities to human identities in enterprise infrastructure today"* (CyberArk 2025). The agent economy is already here. The coordination plane is not.

**Key message:** the problem is not capability. The problem is infrastructure.

### Section 3: The Runtime Loop

**What Nunchi does.** Roko is the open-source runtime that implements the Nunchi protocol. Every agent operation runs through a universal loop:

```
query → score → route → compose → act → verify → write → react
```

Every stage generates a learning signal. The system gets smarter with use. The thousandth run costs less than the first.

**Three primitives:**

- **Signal** — durable data. Persisted to substrate, indexed by HDC vector, addressable by content hash. Signals do not expire unless they fail demurrage checks on-chain.
- **Pulse / Bus** — ephemeral events. Fired during execution, consumed by gates and watchers, not persisted beyond the session window.
- **Cell** — atomic computation. A task with pre/post conditions, a designated gate rung, and an associated cost budget.

**Gates are language-agnostic.** The gate pipeline validates compilation, tests, lint, diff, and semantic coverage regardless of whether the agent wrote Rust, TypeScript, Go, or Python. Gates call language toolchains via subprocess; the runtime does not care about the language.

**Visual.** Animated loop diagram. Nodes light up as data flows through stages. Counter in the corner shows "efficiency signals generated" incrementing.

**Key message:** every operation generates a learning signal. The system improves with use.

### Section 4: The Cost Proof

This is the section that stops the scroll.

**Side-by-side comparison:**

| | Naive baseline | Nunchi-optimized |
|---|---|---|
| Task cost (HAL benchmark) | $44.86 | $1.42 |
| Cache hit rate | ~0% | ~65% |
| Model routing | Always frontier | Tier-matched |
| Gate pre-screening | None | 7-rung pipeline |

**Breakdown:**

- Prompt caching: 5x reduction (Anthropic cache pricing, verified)
- Cascade routing: 3x reduction (RouteLLM, Princeton 2024)
- Gate pre-screening: 2x reduction (avoids wasting frontier calls on malformed inputs)
- Combined: approximately 30x

**Sources.** Princeton HAL benchmark (ICLR 2026, arXiv:2407.01502), Anthropic prompt caching documentation, RouteLLM (Princeton NLP, arXiv:2406.18665).

**Interactive element.** A `nunchi run --share` command that lets any developer reproduce the benchmark in their own environment and see the cost delta. The output posts to a public leaderboard at `nunchi.dev/runs`.

**Visual.** The animated cost meter from the hero, now with detailed breakdown below. Left meter counts to $44.86 with a slow, painful tick. Right meter stops at $1.42 with the breakdown showing how each optimization layer contributed.

**Key message:** 30x is not a rounding difference. It is the difference between a feature and a business.

### Section 5: The Chain

**What the Nunchi blockchain is.** Sovereign EVM Layer 1. Not a rollup, not a sidechain. Purpose-built chain co-located with validators in Tokyo for ~50ms block times. Simplex consensus (Chan & Pass, IACR 2023/463). The chain exists because no general-purpose chain has the primitives agents need.

**What makes it different:**

- **Native HDC precompile** at address `0xA01`: hyperdimensional similarity search at approximately 400 gas — 20–100x cheaper than equivalent Solidity. Every agent can affordably search the shared knowledge substrate without gas budget anxiety.
- **ERC-8004 agent identities:** on-chain identities for agents. Transferable. Composed of a 7-domain EMA reputation score (code quality, research quality, latency, cost efficiency, safety, gate pass rate, cross-agent collaboration). Identity is an asset, not a credential.
- **On-chain substrate storage with demurrage:** knowledge published to the chain persists as long as it earns citations. Knowledge that stops being useful decays and is pruned. The chain is a living knowledge market, not an archive. **Important: demurrage applies to substrate storage, not to the NUNCHI token.**
- **ZK-HDC proofs:** verifiable Hamming distance between agent knowledge vectors, proved with Circom + Groth16 + Poseidon-2 in under one second. An agent can prove it knows something without revealing what it knows.

**Network effect.** The thousandth agent that joins Nunchi arrives with access to every verified knowledge publication from the previous 999. Their reputation starts fresh but their substrate is full. The system compounds.

**Projected scale.** Based on MCP SDK adoption (97 million monthly downloads, Linux Foundation 2025) and current agent deployment growth, 80–150K active ERC-8004 identities are projected within three months of mainnet. **Caveat:** these are projections; current ERC-8004 registrations (21,000–22,900 across BNB Chain, Base, Ethereum L1 within two weeks of mainnet on January 29, 2026) measure interest, not active usage.

**Visual.** Network graph with agents as nodes and knowledge citations as edges. The chain is the shared substrate in the center. New nodes join and immediately connect to existing knowledge clusters.

**Key message:** the thousandth agent joins smarter than the first.

### Section 6: Compliance

**The deadline.** August 2, 2026. EU AI Act Article 50 enforcement begins.

**What Article 50 requires.** AI systems interacting with humans must disclose their AI nature. Automated decision systems must maintain audit trails. High-risk AI must register with a competent authority.

**The problem.** 35.7% of EU managers feel prepared (Deloitte AI Regulation Survey, 2025). Most agent deployments have no identity layer at all. When the regulation lands, the retrofit cost will be high and the timeline will be short.

**Coordination plane framing.** The EU AI Act does not create compliance as a product category — it creates demand for coordination infrastructure that embeds compliance by design. An agent coordination plane that tracks identity, reputation, and decision provenance across every agent operation is not a compliance tool bolted on top; it is the architecture that makes compliance automatic. The regulation is a tailwind for coordination plane adoption, not the category itself. Don't lead with "we solve EU AI Act compliance" — lead with "the Agent Coordination Plane makes compliance a byproduct of coordination." Article 50 is the evidence that enterprises need this infrastructure now, not eventually.

**What Nunchi provides.** ERC-8004 identity anchors every agent to a verifiable on-chain record. Every interaction is logged to the episode journal. Every decision is attributable to a specific agent version at a specific reputation checkpoint. Compliance artifacts are generated as a byproduct of normal operation — no separate compliance pipeline required.

**Compliance framing on the page.** August 2, 2026. EU AI Act Article 50 enforcement begins. Penalty: €15M or 3% of global turnover for transparency violations. This date created a new buyer role — the AI Governance Lead — who needs auditable infrastructure before the deadline, not a countdown widget. Display the date, the penalty, and the role plainly. **Do not use a live countdown timer:** animated ticking clocks pattern-match to ICO/affiliate-marketing sites and undercut credibility with the investor audience. OneTrust used "six months before CCPA enforcement" as text, not a ticking widget. Specificity builds credibility; ticking clock reads as gimmick.

**Key message:** compliance is a byproduct of coordination. The coordination plane makes it automatic.

### Section 7: CTA / Proof

**Trust signals:**

- Open source. Apache 2.0 license.
- 18 crates. Production Rust runtime.
- SOC 2 Type II audit in progress.
- ISO 42001 (AI management systems) planned with Schellman.

**R13 trust signal rules — NEVER FAKE.**

- **NEVER use fake testimonials, mock customer dashboards, or "trusted by 10,000 developers" without verified numbers.**
- The "reality bar" strip (GitHub stars, downloads, logos) must reflect ACTUAL current metrics. If GitHub stars are 47, show 47. Authenticity at low numbers reads as honest and early-stage. Inflated numbers read as fraud and are trivially verifiable.
- If there are no customer logos yet, do not show a logo strip. Show the code and the numbers instead. Code that runs is more credible than logos that are fabricated.
- A live changelog showing 3–4 shipped items in the last 30 days is the most underused trust signal for developer infrastructure companies. It proves active development and shipping velocity without making any claim about adoption. Add `/changelog` to the nav and keep it updated weekly.

**Navigation (R13 — investor flywheel).** The following pages must exist in the top nav: `/customers`, `/changelog`, `/docs`. These are the three URLs investors visit after the landing page scroll. `/customers` shows design partner case studies (even if only 1–2). `/changelog` shows a running list of shipped features with dates — 3–4 items in 30 days proves velocity. `/docs` is the protocol specification and SDK reference. Missing any of these forces the investor to search, which means they leave.

**Links:**
- GitHub (star the repo)
- Documentation (read the protocol spec)
- `nunchi run --share` (try the 30x demo yourself)
- Discord (join the design partner community)
- Changelog (what shipped this month)

**The ask** (for the investor version of this page, served at `nunchi.dev/investors`): Series A, $20–30M. Building the Agent Coordination Plane for the agent economy before August 2026.

---

## 3. Key Numbers Anchoring the Landing Page

These numbers anchor the landing page. Every section has at least one number an investor can remember and repeat. All numbers have citations.

| Number | What it measures | Source |
|---|---|---|
| 30x | Cost reduction vs naive baseline | HAL + Anthropic caching + RouteLLM |
| $44.86 → $1.42 | Actual HAL benchmark task cost | Princeton HAL, ICLR 2026 |
| 41–86% | Multi-agent deployment failure rate | MAST, NeurIPS 2025 (arXiv:2503.13657) |
| 79% | Failures from coordination, not capability | MAST |
| ~400 gas | HDC similarity search cost on-chain | Nunchi chain precompile spec |
| <1s | ZK-HDC proof generation time | Circom + Groth16 benchmarks |
| 97M | Monthly MCP SDK downloads | Linux Foundation, 2025 |
| 80–150K | Projected ERC-8004 active agents at 3 months post-mainnet | Internal projection |
| 82:1 | Machine-to-human identity ratio in enterprise | CyberArk, 2025 |
| Aug 2, 2026 | EU AI Act Article 50 enforcement date | EU Regulation 2024/1689 |
| 18 | Crates in the open-source Rust runtime | Roko codebase (architecture depth, not LOC — see R14 on vanity metrics) |
| 35.7% | EU managers who feel prepared for AI Act | Deloitte AI Regulation Survey, 2025 |

---

## 4. Design Principles: ROSEDUST → R15 Lock

The design system is called **ROSEDUST**. It was designed for a developer-primary audience with a secondary audience of technical investors. The R15 lock evolved the original ROSEDUST (rose accents on near-black) to a higher-contrast variant for the demo-facing surfaces.

### R15 design system lock (apply consistently across landing page, CLI terminal output, Keynote deck, dashboard demo mode)

**Typography:**
- Display (64pt): Geist Sans
- Headline (36pt): Geist Sans
- Body (24pt): Geist Sans
- Code / terminal (24–28pt): Geist Mono (or Berkeley Mono $75 license for distinctive feel)

**Colors (R15 locked):**
- Background: `#000000`
- Text: `#FAFAFA`
- Accent: `#0070F3` (Vercel blue — signals developer-tool credibility, not a derivative choice)

**Terminal theme (CLI output, demo recordings, slide screenshots):**
- Base: Tokyo Night palette on `#1A1B26`
- CLI symbols: Clack-style — `◆ ◇ │ └ ✔ ✖ ⚠ ℹ ❯ → dots spinner`
- **NO emoji in terminal output. Ever.**

### Core visual principles

- **Dark backgrounds**, near-black (`#0D0D0F` for landing page sections where pure black reads as too flat; `#000000` for hero and terminal).
- **Accent: `#0070F3` (R15 lock)** — not rose/pink (ROSEDUST pink is retired from the deck and terminal; survives as secondary landing-page accent only).
- **Monospace typography** for all code and data displays: Geist Mono or Berkeley Mono.
- **Minimal chrome** — the data is the design.
- **Animations are subtle and purposeful, never decorative.**

### Layout decisions

- Numbers first, explanations second. Investors scan before reading.
- Code snippets are real and copy-pasteable. `nunchi run --share` should work.
- Each scroll section is a complete thought. No section requires reading the previous section to understand.
- Mobile is secondary. The audience is on a desktop at a work session or at a meeting. Optimize for a 1440px wide screen with good typography rendering.
- **Dark mode only.** This is a developer tool. Light mode is not the default. **R13 validation:** 8 of 12 agent-infrastructure sites surveyed use dark by default. ROSEDUST dark is genre convention for this audience, not a design risk. (The PDF pitch deck uses light backgrounds — projected in conference rooms; the landing page uses dark — viewed on developer desktops. Different contexts, different optimal choices.)

**The cost meter animation is the highest-priority visual element.** It should:
- Load within 2 seconds of page load
- Run automatically without user interaction
- Be reproducible — the numbers should match what a developer actually sees when they run the benchmark
- Have a "see the code" link beneath it pointing to the benchmark repository

### Typography hierarchy

- Headlines: large, slightly spaced, not bold — confidence without aggression
- Data numbers: extra-large, monospace, with subtle glow or accent color
- Body: comfortable reading size, muted color, generous line height
- Code: monospace, with syntax highlighting that matches the ROSEDUST palette

---

## 5. The App Dashboard (Stripped to Four Demo Views)

The existing dashboard had 27+ pages across seven sidebar sections (PULSE, FLEET, FORGE, KNOWLEDGE, ARENA, MEASUREMENTS, TREASURY). **Too busy for a demo.** An investor meeting needs 3–4 focused views. 27 pages overwhelms in a 30-minute session.

### Recommended demo-focused views (the four views that tell the whole story)

#### View 1: Cost Dashboard

Real-time cost meter. Per-agent spend. Cache hit rate. Routing decisions showing which model was selected and why. Gate outcomes showing how many tasks were caught before reaching the frontier model.

This is the "proof it works" view. Every number on the screen is verifiable.

#### View 2: Agent Fleet

Active agents. Their ERC-8004 identities displayed as cards with reputation domain scores. Current tasks with live status. Costs-per-agent over the session. Think "kubectl get pods" but for AI agents.

This is the "it's real" view.

#### View 3: Knowledge Graph

Published chain knowledge shown as a force-directed graph (or Terrain Map — see `12-visual-design-and-game-feel.md`). Edges are citations between knowledge nodes. Node color shows citation frequency. Demurrage decay visualized as nodes fading for uncited publications.

This is the "network effect" view. Knowledge compounds.

#### View 4: Chain View

Live block explorer. Knowledge publications scrolling as new blocks arrive. ZK proof statuses. Identity attestations. The chain is running and producing output.

This is the "coordination plane" view. The foundation is working.

**Everything else** should remain accessible behind navigation but not in the demo flow. A "Demo Mode" toggle in the top bar should auto-cycle through these four views on a 45-second timer for investor meetings where the presenter wants the screen to tell the story while they talk.

---

## 6. Deck Structure: 12–13 Main Slides + 8–15 Appendix (R14 Confirmed)

**Two-deck practice (Pillar VC) is what a16z expects.** The main deck (12–13 slides) is presented live. The appendix (8–15 slides) is deeper material for post-first-meeting follow-ups, technical diligence, and internal circulation. Both exported as a single PDF, sent 24–48 hours ahead. **NEVER use DocSend** — friction reads as paranoia. PDF attachment directly or a clean cloud link (Google Drive, Dropbox).

**Export format.** Always PDF. The PDF is the artifact they share internally at the Monday partner meeting. It must be self-contained — no embedded videos, no links that require login, no interactive elements. Every number, every chart, every screenshot must render correctly as a static PDF page.

### R10 research-backed slide order for infrastructure Series A

| Slide | Content | Rationale |
|---|---|---|
| 1 | Title + thesis: "The model is the same. The system is the variable." | Casado will pattern-match to category-creation. Category framing: "Nunchi is the Agent Coordination Plane." Kevin Hale explicitly warns against animations on opening slides — no live cost meter here. |
| 2 | Problem: MAST 41–86% failure rate (hero stat), 82:1 identity ratio | One hero stat. Failure rate is the hook; identity ratio sharpens the scale. |
| 3 | Solution: Roko runtime — show real callable code, under 10 lines, with output below | Code, not architecture diagram. Patrick Collison's "7 lines" principle: investors should be able to run it. |
| 4 | Founder | For a solo founder, team is the primary investible asset. Show it here while attention is highest. |
| 5 | Why Now: platform shift + EU AI Act + cost-reduction proven | Standards converging (MCP, A2A, ERC-8004, x402); EU AI Act enforcement August 2, 2026; 30x cost reduction empirically confirmed. Three forces converging now. |
| 6 | Product / architecture: Signal / Pulse / Cell, language-agnostic gates | Detailed system view after the "why now" frames the urgency. |
| 7 | How it works: real screenshots, live dashboard views | Not mockups. The runtime is production — show it. |
| 8 | Traction: design partner logos + community (GitHub stars, downloads) | At Series A, logos beat the sales playbook. |
| 9 | Cost comparison: $44.86 → $1.42, HAL benchmark, 30x breakdown | Lands HERE, not on slide 1. Investors now have context — they know the problem, the solution, the traction. |
| 10 | Competition: honest Harvey-Ball table OR Power Grid format | Reads as more honest than 2x2. |
| 11 | Business model + dual-asset structure | Protocol fees, managed cloud, enterprise SLA; token warrant structure explained plainly. |
| 12 | Use of funds + milestones tied to Series B | 3–4 buckets with percentages; 3–5 milestones with explicit Series B triggers. |
| 13 | Ask + thesis close | Headline number (rounded, never a range), use-of-funds summary, close with inevitability. |

**Key changes from prior structure:** Founder moved to slide 4. Cost comparison moved to slide 9 (needs context to be shocking). "Why Now" added as slide 5. Competition made honest with Harvey-Ball or Power Grid format.

### R15 Deck Refinements

These changes update the slide-by-slide structure above. Apply surgically — do not rewrite slides that are working.

**Slide 1 framing shift.** R15 proposes "Nunchi — the durable runtime for production agents" as the slide 1 identity. Echoes Temporal's positioning ("the durable execution platform") without copying it — signals category awareness, uses proven vocabulary (durable, runtime, production), sets up the Temporal comparison proactively rather than reactively.

**Slide 2 is "Why Now" — NOT "Problem".** Casado said on the Generalist podcast: *"I always start with what is the market."* The Why Now slide answers the market-timing question before the problem slide. Casado-specific slide order; other investors may prefer problem-first.

**Slide 3 is Problem — reframed with Temporal's narrative pattern.** *"Agents broke reliability — again."* The Temporal narrative reframe: 2010 monoliths broke reliability → 2020 microservices broke reliability in new ways → 2026 agents broke reliability again. Positions the coordination plane as the inevitable infrastructure response to the third reliability crisis in 16 years. **Reference it explicitly** — Temporal used this exact framing for durable execution. Nunchi uses the same pattern one layer up.

**Slide 5 — Architecture with control plane / data plane bands.** This is the Casado identity test. Every Casado-backed infrastructure company has a control/data plane separation in its architecture. Draw the slide with two explicit horizontal bands: control plane (Nunchi) and data plane (agent execution). No exceptions, no hybrid diagrams. The SDN vocabulary must be visible.

**Slide 6 — "How it closes the loop":** observe → decide → enforce → record. Answers Casado's April 2025 skepticism HEAD-ON. Do not describe closing the loop in prose — draw the four-stage loop with Nunchi primitives at each stage.

**Slide 7 — "Let me show you":** A blank slide with a single line of text: *"Let me show you."* Transition directly to the demo from this slide. No content, no bullets. The blank slide signals confidence: the product can speak for itself.

**Slide 13 — NO dollar amount on the slide.** Per Kirwin's March 2026 a16z-speedrun essay, do NOT put the round size on the ask slide. Format as "Next 12 months" milestones only. The dollar amount is discussed verbally and documented in the memo, not displayed in the deck. Putting a number on a slide invites negotiation before relationship; milestones invite alignment first.

### Production pipeline (R15 locked)

- **Build in Figma** — design tokens shared with landing page and dashboard for consistency.
- **Present from Keynote** — export Figma frames to Keynote for the live meeting. Keynote handles presenter mode, remote control, and offline reliability better than browser-based tools.
- **Export PDF for data room** — the PDF is the artifact that circulates at the Monday partner meeting. Self-contained: no embedded videos, no links requiring login, every number and chart renders correctly as a static page.

### Appendix slides (8–15, R14)

NOT presented live. Exist in the PDF for post-meeting circulation and diligence. Suggested contents:
- Detailed cost breakdown (HAL benchmark raw data, cache hit rate sources, routing split)
- Gate pipeline architecture diagram
- ERC-8004 identity system technical detail
- Chain architecture (Simplex consensus, HDC precompile, validator topology)
- Competitive landscape detail (full Harvey-Ball/Power Grid table with all competitors)
- Regulatory timeline (EU AI Act, Colorado AI Act, California SB 53)
- Market sizing methodology and sources
- Team bios and advisors (if not fully covered in main deck)
- Design partner case studies (if available)
- Financial model summary (if applicable)

### Visual language guidance (R10)

No VC has publicly stated preference for designed decks over Sequoia-template decks. Evidence runs the other direction. Alexander Jarvis: *"DON'T DO ANYTHING FANCY."* Casado's own published fundraising advice: content-first, not design-first. Words per slide: 15–30 content words, under 10 on transitions. 30pt minimum body, 60pt+ headers. Total deck: 300–600 words. Use light backgrounds — they render more consistently across projectors than ROSEDUST dark. **ROSEDUST dark is correct for the landing page (developer audience, desktop); the PDF deck is projected in conference rooms.**

### Compliance-as-GTM framing (R10)

Christina Cacioppo on Vanta: *"If you want to start a security company, you should think about starting a compliance company. Compliance is often a purchase driver — it's a growth accelerant."* OneTrust's Kabir Barday timed the Series A to "six months before CCPA enforcement." The formula: regulation creates the buyer role (Chief Privacy Officer for GDPR, Compliance Lead for SOC 2, AI Governance Lead for EU AI Act), and the software winner enables that buyer. Use this framing in slides 5 and 11.

---

## 7. The Condensed 11-Slide Deck (with ISFR)

The complete condensed deck for second-stage investor conversations. Lead-investor-readable in 5 minutes. Every slide pulls weight. **Two narratives, one substrate** — Article 50 / agent coordination is the beachhead, ISFR / agent-native finance is the expansion. Both get airtime.

| # | Title | Job to do |
|---|---|---|
| 1 | **Cover** — *Models execute. Nunchi coordinates.* | Hook. Set typography tone. |
| 2 | **The whole argument in 6 lines** | Elevator pitch for readers who close after slide 5. |
| 3 | **Why now** — 3 signals + 14-week clock | Capital is decided. Regulation has a date. |
| 4 | **The failure mode** — 41–86% / 79% coordination | Why models won't fix this. |
| 5 | **The empty cell** — 5 funded layers, 1 unfunded | The category claim. Vanta/OneTrust precedent footer. |
| 6 | **The wedge** — $42 → $1.42, 22.5x | Concrete, cited, reproducible economics. |
| 7 | **The moat** — knowledge compounds non-linearly | Why the second mover loses. |
| 8 | **Architecture** — Roko + Nunchi blockchain, one diagram | One page, no repetition. |
| 9 | **The second category** — ISFR + Cooperative Clearing | Same substrate clears DeFi rate markets. TAM expansion. |
| 10 | **What 12 months buys** — 4 milestones | The use-of-funds proof. |
| 11 | **Ask + closing** | $30M, runway, lead investor wanted. Closing typographic card. |

### The whole argument (slide 2)

1. Production agents fail at coordination, not capability — 41–86% failure, system-level.
2. Roko is the runtime wedge — production agents cheaper, safer, replayable. Open source, self-hosting.
3. The Nunchi blockchain is the substrate where local traces compound — shared memory, attestable reputation, verifiable settlement across organizations.
4. Every adjacent layer is funded — Temporal $5B, LangChain $1.25B, Orkes $300M, Keycard $200M. The coordination plane is empty. That's the category.
5. **The same substrate clears DeFi rate markets** — ISFR + KKT-verified Cooperative Clearing on a $668T-vs-<$100M opportunity. One chain, two rent surfaces.
6. Models commoditize. Scaffolds compound. The model is the same. The system is the variable.

### Why now (slide 3)

| Capital | Demand | Clock |
|---|---|---|
| $750M Google agentic fund (Apr 22) | 1,445% Gartner multi-agent inquiry surge | EU AI Act Art. 50 enforces Aug 2, 2026 |
| $60M Orkes Series B (Apr 23) | 26.2% of EU enterprises ready · 73.8% non-compliant on day one | $15M / 3% turnover penalty |
| Hyperscaler-led capital flowing to deployment, not coordination | | 14 weeks from today |

Footer: *The empty layer between application and execution is the category. It does not yet have a winner.*

### The wedge (slide 6)

$42.11 (HAL baseline) → $8.40 (caching) → $2.80 (routing) → $1.42 (trim + batch) — **22.5x on benchmark, 10–20x practical**. Each lever cited: Anthropic 0.1× cache · RouteLLM 85% cut at 95% quality · VentureBeat 73% trim · Anthropic 50% async batch. Reproducible methodology: task IDs, model versions, hit rates, gate-exit rates published.

Footer: *The receipt the EU AI Act will require — produced as a byproduct of the wedge.*

### The moat (slide 7)

Four compounding wheels — **knowledge · calibration · reputation · settlement** — each monotonic in successful Cells. HDC NeuroStore: 30K tokens recalled per Signal hit, 92% within-distribution precedent at 200 invocations. Five network effects, none requiring a social graph.

Footer: *Code can be forked. Lived coordination history cannot. The thousandth agent joins smarter than the first.*

### What 12 months buys (slide 10)

| Month | Milestone |
|---|---|
| M1 | 100 SWE-bench-Verified cost benchmark, published, reproducible |
| M3 | SOC 2 Type II (Schellman) + Article 50 mapping document — procurement-ready |
| M6 | 3 named design partners with signed reference agreements, real workloads |
| M9 | Cooperative Clearing v1 testnet · ClearingProfile hedging API |
| M12 | Protocol spec donated to Linux Foundation AAIF (alongside MCP, A2A) |

Design-partner targets, priority order: **Hebbia · Harvey · Decagon · Sierra**.

### Ask (slide 11)

- **$30M Series A**
- 18-month runway · 60% engineering+research / 30% GTM+infra / 10% reserve
- Lead investor wanted
- Closing card: *Models execute. Nunchi coordinates.*

---

## 8. Four Artifacts, One Story

All four deliverables tell the same story with the same numbers. An investor who sees the landing page, then attends a demo, then receives the deck async should feel like they are at different resolutions of the same document — not reading different pitches.

| Artifact | Primary audience | Duration | Medium |
|---|---|---|---|
| Landing page (this document) | Anyone who visits nunchi.dev | 3–5 min scroll | Web |
| Demo (`10-demo-script.md`) | Investor meeting, live | 3 min (R12) / 5 min (R15) | Split terminal |
| PDF deck | Investor review, async | 10 min read | PDF / slides |
| App dashboard | Design partner, technical due diligence | 30+ min | Web app |

The landing page is the first thing most investors see. The demo is what converts. The deck is the document they share internally. The dashboard is what convinces technical diligence that the runtime is real.

**Consistency requirements:**
- HAL benchmark numbers must be identical across all four artifacts.
- MAST citation (arXiv:2503.13657) must appear in landing page and deck.
- EU AI Act date (August 2, 2026) must appear in landing page, deck, and dashboard countdown.
- The `nunchi run --share` command must appear in landing page and demo script.
- ROSEDUST design tokens must be shared between landing page and dashboard via a shared design token file.

---

## 9. Landing Page Summary

| Element | Content |
|---|---|
| Reference aesthetic | ROSEDUST → R15 lock (#000000 bg, #FAFAFA text, #0070F3 accent) |
| Reference IA | Linear (scroll-driven product demonstration) |
| Hero headline | "The model is the same. The system is the variable." |
| Hero visual | Animated split-terminal cost meter ($44.86 → $1.42) |
| Problem stat | MAST 41–86% failure, 79% from coordination |
| Pull-quote | Aubakirova: *"We gave agents intelligence. We forgot to give them infrastructure."* |
| Cost proof | $44.86 → $1.42 with 5x × 3x × 2x breakdown |
| Compliance framing | August 2, 2026 deadline as TEXT, not countdown widget |
| Trust signals | NEVER fake. Reality bar must reflect actual numbers. Show /changelog with weekly updates. |
| Investor flywheel pages | /customers, /changelog, /docs in top nav |
| Deck format | 12–13 main slides + 8–15 appendix, PDF only, R10 slide order with R15 refinements |
| Production pipeline | Figma → Keynote → PDF |
