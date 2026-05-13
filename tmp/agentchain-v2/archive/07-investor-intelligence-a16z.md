# Investor Intelligence: a16z

This document prepares you for a meeting with Martin Casado and Malika Aubakirova at Andreessen Horowitz. Dossiers on both partners. Detailed map of Aubakirova's published thinking. Critical Keycard relationship. Adjacent partner dynamics. Vocabulary you must mirror in the room. Plan B investors if a16z passes.

---

## 1. Martin Casado Dossier

### Background

General Partner at a16z. Co-founded Nicira, the company that created Software-Defined Networking (SDN). Nicira decoupled the network control plane from the data plane, allowing centralized policy enforcement over distributed physical infrastructure. **VMware acquired Nicira for $1.26 billion in 2012.** Casado then served as GM of VMware's Networking and Security Business Unit before joining a16z.

His entire career thesis is control planes: the idea that you can impose centralized, programmable policy over distributed execution without owning the underlying infrastructure.

### How to speak to him

Use SDN vocabulary throughout. Nunchi's architecture maps directly onto concepts he invented:

- **Control plane / data plane separation:** Nunchi is the control plane for AI agents. The agents themselves (Claude, GPT, Codex, Gemini, local models) are the data plane. Nunchi does not run inference. It routes, gates, coordinates, and enforces policy across whatever models you already use.
- **Flow tables / policy enforcement:** the gate pipeline is the equivalent of OpenFlow rules. Every agent action passes through a 7-rung validation pipeline before results are accepted.
- **Northbound / southbound APIs:** the HTTP control plane (~85 routes on port 6677) is the northbound API for dashboards and operators. The agent dispatch layer is the southbound API to LLM backends.

### His April 2025 skepticism — and the March 2026 thesis pivot

In April 2025, Casado expressed public skepticism about agent infrastructure: *"We can't yet close the control loop on agents."* This is the opening you must address. Nunchi closes that loop: plan, execute, gate, learn, replan. The system watches its own output, validates it against configurable thresholds, and adjusts routing and prompts based on measured outcomes.

**Critical update:** On March 18, 2026, Casado led the **$43M Deeptune Series A** — a reinforcement learning and simulation platform for computer-use agents. The shift from skeptic to lead investor in 11 months indicates he has developed a specific thesis about where agent infrastructure is heading. Deeptune (March 19): *"If the last decade of AI progress was driven by better datasets, the next decade will be mostly driven by better environments."* RL environments and runtime evaluation are top of mind. Roko's gate pipeline + episode history + adaptive thresholds = a runtime evaluation environment that improves with use.

Do NOT argue with the April 2025 skepticism. Validate it and then show the closed loop.

### Casado's published canon (use his vocabulary)

His public intellectual framework. He will use these phrases — you must too.

1. **SDN decoupling.** Centralized control over distributed execution. His origin story and mental model for every infrastructure company.
2. **"Customers don't buy platforms; customers buy products."** (Open Networking Summit 2017, repeated in Newcomer Aug 2025.) Deeply skeptical of horizontal platform plays that lack a specific wedge use case. Lead with a concrete workflow (the demo) before describing the platform.
3. **"No more Red Hats."** Open source is a distribution funnel, not a business model. Does not believe you can build a durable company by selling support around an open-source core. Roko (the 18-crate Rust toolkit) is the distribution mechanism. The business is the hosted coordination plane, the chain settlement layer, and enterprise policy enforcement. Make this distinction explicit.
4. **"Non-consensus investing is overrated."** Best investments are consensus ideas where others underestimate execution difficulty. *"Build a non-consensus product but give a consensus pitch to investors."* The non-consensus product is multi-agent coordination as a real category. The consensus pitch is the layer cake, the token-path margin, and the precedent of Pinecone+LangChain.
5. **"Vertical clouds."** *"Vertical clouds, which are entirely focused on a specific type of workload, tend to be far more sophisticated, far more cost effective, and far more performant."* The Nunchi blockchain is a vertical cloud for agent identity and settlement.
6. **"Bitter Economics"** (Latent Space, Feb 19, 2026, with Sarah Wang). *"Everybody has to be on the token path and everybody has to ask... how do I extract margin on the tokens that are going through?"* This IS Nunchi's wedge. Cost-aware routing = margin capture on the token path. Anchor on this quote in the meeting. Frontier labs are "gross margin positive" on the last training run but "gross margin negative" on the next — model providers cannot be trusted to optimize cost on the customer's behalf. A coordination layer that arbitrages across providers is structurally sound.
7. **"Market annealing"** (a16z January 2023 essay). Markets settle into equilibria slowly. Early entrants that establish the coordination layer become the default. Quote it back: *"I read your January 2023 piece. I'm not asking you to fund a category. I'm asking you to fund the founder-led sales annealing through year four."*

### His PhD context (R12 critical context)

**Read Casado's 2007 PhD dissertation before the meeting.** It is NOT a control-theory thesis in the Lyapunov sense. It is a systems-architecture thesis (Stanford CS, advisors McKeown / Shenker / Boneh). No convergence proofs, no bandit theory. The only "convergence" is spanning-tree convergence in Ethane bootstrap.

His architectural vocabulary — logically centralized control, control/data plane separation, default-off, flow-level granularity, trusted computing base minimization — maps directly to Nunchi. Every one of these concepts has a Nunchi analog. The implication: when he asks "show me the convergence math," the response must be **architectural**, with math as defensive backup only. Lead with the Nicira analogy of plan-execute-gate-learn-replan as a logically centralized control plane. Math is one slide, one equation (UCB1 regret O(√KT log T), Borkar-Meyn ODE method for EMA threshold adaptation), not a Lyapunov derivation.

### The pitch closing line

> *"Ethane reduced enterprise networks to dumb forwarding elements governed by a logically centralized policy. Nunchi reduces LLM agents to dumb invocation elements governed by a logically centralized routing-and-gating policy."*

### Three slide-one anchors (pick one to open the deck)

1. **"The bottleneck is coordination."** (His team's literal 2026 thesis.)
2. **"Re-architect the control plane for agents."** (SDN heritage.)
3. **"The missing layer."** (Deeptune framing.)

### What he has NOT said (opportunity AND risk)

He has not publicly written about: multi-agent orchestration, cost-aware routing as a category, agent economics. This is both opportunity (you define the category in his head) and risk (he has not pre-committed to the thesis). Define it using HIS vocabulary, not yours.

### Avoid in the meeting

- AGI (he calls it "lazy thinking")
- Agent autonomy as feature (skeptical of open-loop)
- Data moat claims (his published view: data rarely a real moat unless replication takes 3+ years)

### Board companies (NEVER criticize)

Cursor (~$10B+ valuation), Convex, Netlify, Kong, Truffle, Pindrop, Fivetran, Material Security, Ideogram, World Labs, Fly.io, Imply, Braintrust ($300M valuation Series B per TechCrunch May 2025; Casado-led; corrected from earlier $800M figure).

Frame complementary, not competitive:
- **Cursor:** *"Cursor agents through Nunchi's CascadeRouter cost 3–5x less. We make your portfolio company more efficient."* Cursor is a customer, not a competitor.
- **Braintrust:** *"Braintrust evaluations feed into our gate pipeline. A Braintrust eval becomes a gate rung. We are not replacing Braintrust. We are making Braintrust evaluations enforceable at runtime."*

### Critical landmine: Casado has NOT led a crypto deal

Crypto investments at a16z go through Dixon's team (a16z crypto, ~$7B AUM). The ISFR / yield-perps pitch belongs in a Dixon meeting, not a Casado meeting. In the Casado meeting, lead with the coordination plane and the open-source runtime. The chain is infrastructure that enables the coordination plane — frame as a technical choice (sovereign L1 for sub-50ms coordination), not a crypto play. If you lead with chain/crypto/web3 vocabulary, you trigger his "this is a solution looking for a problem" filter.

---

## 2. Malika Aubakirova Dossier

### Identity

- **Byline name:** Malika Aubakirova (use in all written communication)
- **Conversational name:** Maika (informal; her X handle is @MaikaThoughts)
- **Role:** Partner at a16z, focused on AI infrastructure, security, and developer tools
- **Author hub:** `https://a16z.com/author/malika-aubakirova/`
- **Background:** Stanford GSB, ex-Google / Chronicle Security (Chronicle Detect)

### The verified 9-essay corpus

Every piece of her published thinking that is relevant to this meeting. For each: title, date, URL, the key quote to internalize, and the tactical application.

#### Essay 1: Investing in Keycard

- **Date:** October 21, 2025
- **URL:** `https://a16z.com/investing-in-keycard/`
- **Co-authors:** Zane Lackey, Yoko Li, Joel de la Garza
- **Key quote:** *"Keycard solved the Auth0 moment inside the org."* (Most strongly attributable to Zane Lackey; Aubakirova co-owns the framing.)
- **Tactical use:** This is the positioning seam. Keycard handles intra-organizational machine identity. Nunchi handles cross-organizational reputation and settlement. Framing: *"Keycard solved the Auth0 moment inside the org. ERC-8004 takes the same primitive across the trust boundary."* Positions Nunchi as the next stage of evolution, not a competitor.
- **Verbatim passages:**
  - *"static secrets and API keys... built for humans clicking buttons, not for autonomous agents spawning by the thousands."*
  - *"dynamic, identity-bound, task-scoped tokens: cryptographic 'keycards' that carry verifiable proof of who the agent is, what it is allowed to do, and for whom."*
  - *"This simple but profound shift, from static identity (who you are) to dynamic intent (what you are doing right now), finally makes safe, autonomous delegation possible."*
  - *"the missing piece that will finally allow AI agents to move safely from pilot to production."*

#### Essay 2: Why We Need Continual Learning

- **Date:** April 22, 2026
- **URL:** `https://a16z.com/why-we-need-continual-learning/`
- **Co-author:** Matt Bornstein
- **Key quote:** *"The lossy compression is the learning."*
- **Tactical use:** Knowledge anchor. Nunchi's neuro store and ZK-HDC fingerprinting are the multi-agent analog of intra-model continual learning. *"Inside one model, the lossy compression is the learning. Across agents, the shared knowledge store is the same compression, but externalized and cryptographically verifiable."*
- **Additional verbatim:**
  - *"a coordinated swarm of agents, each holding its own context, specializing on a slice of the problem, and communicating results, can collectively approximate unbounded working memory."*
  - *"a model that genuinely learns from deployment compounds in value over time in a way that context-only systems cannot."*
- **Use her line "retrieval is not learning"** and pivot: *"the same is true between agents."*

#### Essay 3: Et Tu Agent?

- **Date:** April 2, 2026
- **URL:** `https://a16z.com/et-tu-agent/`
- **Co-authors:** Joel de la Garza, Zane Lackey
- **Key quote:** *"Save the AI agents. Secure the supply chain."*
- **Tactical use:** Gates anchor. Her essay identifies the agent supply chain as the attack surface. Nunchi's 7-rung gate pipeline is the supply-chain security layer for agent output. Mirror her **"63,000x faster"** pattern when presenting cost-reduction numbers. She responds to concrete multipliers, not abstract claims.
- **Additional verbatim:**
  - *"the entities making dependency decisions are increasingly not human."*
  - Socket detected the malicious dep *"within 6 minutes of its publication. That's roughly 63,000 times faster than the industry average."*

#### Essay 4: Big Ideas 2026

- **Date:** December 9, 2025
- **URL:** `https://a16z.com/big-ideas-in-tech-2026/`
- **Key quote (THE most important quote in the entire corpus):**

> *"The bottleneck becomes coordination: routing, locking, state management, and policy enforcement across massive parallel execution. The winning platforms will be the only ones capable of surviving the deluge of tool execution that follows."*

> *"It's not architected for a single agentic 'goal' to trigger a recursive fan-out of 5,000 sub-tasks, database queries, and internal API calls in under milliseconds."*

- **Tactical use:** Open the meeting on the Pulse Globe with this quote flashed on screen. She wrote those exact words five months before the meeting. The "5,000 sub-tasks fan-out" is the literal Pulse Globe spec. This is the strongest possible opening — the pitch becomes validation of her own published thesis rather than a cold sell.
- **Joel de la Garza's Big Ideas 2026 quote (R13):** *"recursive fan-out of 5,000 sub-tasks... looks like a DDoS."* Pair both quotes: Aubakirova ("thundering herd") and de la Garza ("DDoS") describe the same problem from different angles. Makes the coordination-plane thesis feel inevitable rather than novel.

#### Essay 5: Cinderella Glass Slipper

- **Date:** December 8, 2025
- **URL:** `https://a16z.com/the-cinderella-glass-slipper-problem-in-ai-infrastructure/`
- **Key quote:** *"In AI, achieving product-market fit may literally mean solving one high-value workload better than anyone else."*
- **Tactical use:** Wedge framing. Do not pitch "agent infrastructure" (generalist). Pitch one workload — verifiable similarity for cross-org agents under cooperative clearing — completely. Direct line: *"We're not building agent infrastructure. We're solving one stubborn coordination workload — verifiable similarity for cross-org agents — completely."*

#### Essay 6: State of AI / OpenRouter

- **Date:** December 4, 2025
- **URL:** `https://a16z.com/generative-ai-enterprise-2024/`
- **Key quote:** *"Agentic inference."*
- **Tactical use:** She is lead author. Match her empirical voice. Use "agentic inference" rather than "model selection" or "model routing." Signals careful reading of her work. Anjney Midha (159K views) tagged her as lead author of the State of AI report on Dec 4, 2025.
- **Additional verbatim:**
  - *"agentic inference... the model plans, retrieves context from tools or APIs, revises outputs, and iterates."*
  - *"The competitive frontier is no longer only about accuracy or benchmarks. It is about orchestration, control, and a model's ability to operate as a reliable agent."*

#### Essay 7: Kill Chain

- **Date:** September 9, 2025
- **URL:** `https://a16z.com/a-kill-chain-for-ai-model-security/`
- **Key quote:** *"Different kill-chain stages require different defenses."*
- **Tactical use:** Shield against the "isn't this just Keycard?" objection. Different kill-chain stages, different products, complementary. Keycard operates at the identity/authentication stage. Nunchi operates at the execution/validation/settlement stages.
- **Additional verbatim:** *"When viewed through the kill chain lens, [founders] are often addressing entirely different types of attacks... the next generation of tools will not be defined by acronyms but by their ability to eliminate attack paths altogether."*

#### Essay 8: Next-Gen Pentesting

- **Date:** 2025
- **URL:** `https://a16z.com/next-gen-pentesting/`
- **Key quote:** *"Validated paths."*
- **Tactical use:** Language template for ZK-HDC. Use **"validated paths"** rather than "zero-knowledge proofs" or "cryptographic proofs." Her vocabulary is security-first, not crypto-first.
- **Additional verbatim:** *"By safely executing real-world exploits in isolated sandboxes, these tools produce only actionable results. The output isn't just a list of issues, it's a validated path showing how an attacker would have breached your system."*

#### Essay 9: Investing in Adaptive Security

- **Date:** April 2, 2025
- **URL:** `https://a16z.com/investing-in-adaptive-security/`
- **Key quote:** *"Security must be adaptive, not reactive."*
- **Tactical use:** Her earliest published security thesis. Establishes the pattern she applies in all later essays: static defenses fail, adaptive defenses compound. Nunchi's adaptive gate thresholds (EMA-based, self-tuning) are a direct implementation.

### Topics she has NOT publicly tweeted or written about

Be aware. Do not assume preloaded knowledge:

- **ERC-8004** specifically (do not reference unless she brings it up)
- **ZK proofs / verifiable similarity** (uses "validated paths")
- **EU AI Act** (introduce as tailwind, not echoed)
- **LangGraph / AutoGen by name** (uses "multi-agent architectures")
- **"Non-human identity"** as a literal phrase (uses "machine identity" or "agent identity"). Mirror her vocabulary exactly.

### X (Twitter) activity

Low-volume. @MaikaThoughts ~1,100–1,300 followers. Profile sometimes literally reads "hasn't posted." Substantive thinking lives on a16z bylines, not tweets. Confirmed activity:
- **Sep 9, 2025:** "the most 'cracked' YC batch" (33.2K views)
- **Jul 31, 2025:** reply to @lmarena_ai
- **~Apr 2026:** went live with @VirtualElena on MTSlive about Continual Learning; welcomed @lishali88 to a16z; replied to @aurielws and @JasonSCui; RT'd Hedra Labs
- **Dec 4, 2025:** Anjney Midha (159K views) tagged her as lead author of the State of AI report

---

## 3. The Keycard Sovereignization Argument

This is the most important competitive relationship in the room because Keycard is an a16z portfolio company. Handle precisely. Aubakirova co-led the Keycard $38M round (October 2025) with Zane Lackey, Yoko Li, and Joel de la Garza.

### What Keycard does (R13 full product detail)

Keycard is **STRICTLY identity / authorization / audit**. NOT a gateway. NOT an orchestrator.

- **Product:** identity-bound task-scoped JWTs via Security Token Service (RFC 8693).
- **Composite identity model:** user + device + agent + task, evaluated per tool call.
- **Founded by:** Auth0's Chief Architect (Jared Hanson, built Passport.js).
- **Advisors:** Auth0 cofounder Matias Woloski, Okta Chief Product Architect Karl McGuinness, Datadog CISO, Cloudflare CTO Dane Knecht, Anthropic Head of Security Angie Lai.
- **Pricing:** $500/month for 100K transactions, $1 per additional 1,000.

### Three gaps Aubakirova identifies

In her Keycard investment essay, three failure modes in existing agent identity:
1. **Too permissive** — agents get broad API keys with no scoping.
2. **Lock-down** — orgs respond by restricting agents so heavily they cannot do useful work.
3. **Custom infra** — every team builds bespoke identity plumbing.

Root cause: *"years of underinvestment in machine identity."*

### Business-model orthogonality (the structural defense)

Keycard makes money when MORE agents make MORE tool calls. They have **zero incentive** to build coordination that reduces redundant calls. A coordination layer that optimizes routing and reduces wasteful invocations is structurally complementary — Keycard prices per transaction, Nunchi reduces transaction count per task while increasing total task volume across the ecosystem. **Their pricing model literally requires a coordination layer above it to drive volume.**

### What Keycard explicitly does NOT do

Agent-to-agent coordination, cost-aware routing, shared knowledge/memory, reputation, behavioral verification, ZK proofs, orchestration, economic primitives, task negotiation.

### The pitch line (R13, memorize)

> *"Keycard answers 'is this agent allowed to call this tool right now' — that's the new Auth0. Nunchi answers 'which agent should call which tool, when, on which model, with what context, at what cost' — that's the new control plane. Keycard's per-transaction pricing literally requires a coordination layer above it to drive volume. We're customers of Keycard, not competitors."*

### Layer-cake precedent

a16z has funded adjacent layers in the same portfolio before: **Pinecone + LangChain**, **Clerk + Keycard**. This is the EASIEST yes for a16z if framed correctly. There is no portfolio conflict because the layers are structurally complementary.

### The sovereignization two-axis diagram (deck slide template)

Preserve Aubakirova's axis (static identity → dynamic intent), add a perpendicular axis (centralized issuer → sovereign verification):

- **Keycard:** dynamic intent, centralized issuer.
- **Nunchi:** dynamic intent, sovereign verification (diagonal-opposite quadrant).

Visualizes the relationship as evolution, not competition.

### Exact line to deliver in the meeting

> *"Keycard solved the Auth0 moment inside the org — dynamic, identity-bound, task-scoped tokens, runtime-enforced. ERC-8004 + Nunchi takes the same primitive across the trust boundary: same dynamic-intent token, but issued by no one and verifiable by everyone — sovereignized via on-chain settlement."*

### Conflict risk

If Nunchi's pitch sounds like Keycard — specifically, if it emphasizes agent identity authentication as a primary value proposition — Aubakirova will flag a portfolio conflict and either pass or recuse herself from the deal evaluation. Since Aubakirova is the intended warm intro path to Casado, losing her advocacy is a significant setback.

**Mitigation:** differentiate sharply and proactively. Keycard = identity authentication (who you are, what you are authorized to do). Nunchi = coordination, prediction, shared knowledge ABOVE the identity primitive. The pitch must position Nunchi as consuming Keycard identity tokens as one input to the coordination plane, not as a competing identity layer. The safe framing: *"Keycard solved identity. What solves the rest?"*

Open the Aubakirova conversation by quoting her *"thundering herd / looks like a DDoS"* line from Big Ideas 2026. Signal that the founder has read the thesis and is offering the answer to the unanswered question (coordination, prediction, shared knowledge, durability).

---

## 4. Adjacent Partners

### Joel de la Garza

- **Role:** Operating Partner, Security
- **Boards:** Adaptive, Socket, Doppel, Lumos, Eclypsium, Truffle
- **Position:** *"2026 is the year of agents; identity is the bottleneck."*
- **Co-author of:** Et Tu Agent, Kill Chain
- **Public thesis:** Agent identity is bottleneck. Behavioral detection beats CVE databases.
- **Tactical use:** Joel represents demand-side conviction. Ally in the room. If conversation stalls on skepticism, redirect to Joel's framing.
- **Mirror:** *"Every dependency decision an agent makes is a trust decision; right now nobody's checking"* — Nunchi reputation : agent-to-agent calls :: Socket : agent-to-package.

### Zane Lackey

- **Role:** Security-focused partner
- **Background:** Founded Signal Sciences (acquired by Fastly for $775M)
- **Co-author of:** Keycard, Et Tu Agent, Kill Chain
- **Public framing:** AI as defender amplifier; kill-chain stages are differentiating, not overlapping.
- **Tactical use:** If the Keycard overlap question escalates, Zane's kill-chain framing (which Aubakirova references in Essay 7) is the defusal mechanism.

### Matt Bornstein

- **Role:** Partner focused on AI infrastructure
- **Position:** Skeptic — *"Agents don't really work yet."*
- **Co-author of:** Continual Learning
- **Tactical use:** Skeptic in the room. Lead with concrete numbers: cost reduction multipliers, gate failure rates, task completion rates from the demo. Do not argue the thesis — show the data.
- **Mirror:** Cite his Continual Learning piece directly. Nunchi's NeuroStore (the right module for knowledge), predict-publish-correct (the right signal via continuous error correction), and the gate pipeline (data quality enforcement) are the literal instantiation of his three requirements.

### Yoko Li

- **Role:** Partner focused on developer tools and AI
- **Position:** *"Redefinition of how software gets built with agents, context, and intent at the core."*
- **Co-author of:** Keycard
- **Tactical use:** Potential champion. Her thesis aligns with Nunchi's wedge (software development workflows). Direct the "why software development first" explanation toward her if she is in the room.
- **Public framing:** Agents as collaborators and consumers; "compressing coordination"; unified MCP marketplace; agent-as-distinct-identity.
- **Mirror:** Be ready with MCP/A2A integration depth. Use her exact phrase: *"redefinition of how software gets built with agents, context, and intent at the core."*

### Sarah Wang routing risk

Temporal's $5B Series D round was led by Sarah Wang and Raghu Raghuram, NOT Casado. Agent durability/orchestration sits in HER practice. If Casado feels overlap with Temporal during the pitch, his likely action is to route the deal to Wang.

**Wang's framework:** *"only 3-5 important questions matter"* for a deal. Her questions will center on how Nunchi is structurally distinct from Temporal, not on the coordination-plane thesis (which is Casado's territory).

**Mitigation:** volunteer the Temporal distinction BEFORE Casado asks. Use the 30-second Temporal answer (see `08-competitive-landscape.md`). Key phrase: *"Temporal owns 'did this code run.' Nunchi owns 'did the right agent, with the right memory, at the right price, with a receipt the counterparty can verify.'"* Making the boundary explicit in the first 10 minutes keeps the deal in Casado's lane.

---

## 5. The Productive Tension Bridge

The room will likely contain conviction (Joel, Yoko) and skepticism (Matt, Casado pre-2025-pivot). Do not try to eliminate the tension. Bridge it:

> *"Joel is right that agent identity is the bottleneck. Matt is right that most agent frameworks do not actually work yet. Nunchi closes that gap: it makes agents work reliably by adding the coordination layer that is missing."*

This acknowledges both positions and positions Nunchi as the resolution.

---

## 6. The a16z Lexicon — 13 Terms to Mirror

| # | Term | Source | Nunchi mapping |
|---|---|---|---|
| 1 | "machine identity" | Keycard, Big Ideas 2026 | ERC-8004 agent identities. NOT "non-human identity" or "NHI." |
| 2 | "the missing trust fabric" | Keycard | The coordination plane |
| 3 | "from static identity to dynamic intent" | Keycard | Static API keys → ERC-8004 sovereign claims |
| 4 | "agent-native infrastructure" | Big Ideas 2026 | Roko runtime |
| 5 | "coordination becomes a bottleneck" | Big Ideas 2026 | The problem statement (use verbatim) |
| 6 | "agentic inference" | State of AI / OpenRouter | The runtime's execution model |
| 7 | "workload-model fit" | Cinderella essay | CascadeRouter model routing |
| 8 | "output, not users" | Casado, via State of AI | Per-action pricing model |
| 9 | "multi-agent architectures as a scaling strategy for context itself" | Continual Learning | NeuroStore knowledge compounding |
| 10 | "validated paths" | Pentesting essay | ZK-HDC proofs (NOT "ZK proofs") |
| 11 | "janky but native" | Continual Learning | 177K lines of Rust, native coupling |
| 12 | "the lossy compression is the learning" | Continual Learning | HDC fingerprinting as lossy compression |
| 13 | "compounds in value over time" | Continual Learning | Knowledge store network effect |

Three additional terms from R3:

14. **"Retrieval is not learning"** — distinguishes RAG (filing cabinet) from genuine knowledge compounding. Use when explaining inter-agent knowledge sharing vs. simple context injection.
15. **"The entities making decisions are increasingly not human"** — setup line for why gates matter.
16. **"Compounds in value over time"** — the durability argument. Systems that learn from deployment compound; systems that restart from zero do not.

---

## 7. Plan B Investors (Run in Parallel)

If a16z passes. Run parallel conversations on the same timeline so a pass does not create dead time.

| Priority | Investor | Partner | Thesis fit | Check size | Risk |
|---|---|---|---|---|---|
| 1 | **Sequoia** | Sonya Huang | Led LangChain A+B; co-authored "2026: This Is AGI"; explicitly calls out agent-harness layer | $10–25M | LangChain conflict if Nunchi competes head-on |
| 2 | **Lightspeed** | Bucky Moore | Moved from Kleiner Perkins 2025. Led Raindrop, Inferact/vLLM, Tasklet, Distyl | $15M sweet spot | Lowest conflict, highest probability of competing on terms |
| 3 | **Benchmark** | Chetan Puttagunta | MongoDB, Elastic, Confluent, Airbyte DNA | $10–20M | Sits on LangChain board (conflict caveat). Eric Vishria is consumer-agent partner — different lane |

**Honorable mentions:** Astasia Myers (Felicis, cleanest agent-infra thesis), Erica Brescia (Redpoint, OSS DNA, Modal Series A). For Berlin specifically: **468 Capital** (Berlin + SF + Madrid, $1.3B+ raised, Fund II ~$400M from Jan 2022; partners Florian Leibert, Alexander Kudlich, Ludwig Ensthaler — only Berlin-anchored fund with explicit "AI & Automation, Infrastructure & Enterprise Software" thesis). **Air Street Capital** ($232M Fund III March 2026, largest solo-GP fund in Europe; Nathan Benaich; Berlin-friendly via Black Forest Labs, Sereact, Interloom, Fern Labs/poolside).

**Deprioritize:** Founders Fund (Luttig anti-OSS pattern), Index (bench gutted — Price-Wright to a16z, Goldberg to Chemistry), General Catalyst (Nishar departed), Insight (Series B+ mismatch), Khosla (closed-source pattern).

### Crypto component: Chris Dixon / Ali Yahya

Crypto fund (~$7B AUM). Led Catena Labs ($18M, May 2025): *"Software agents should be able to pay and get paid, instantly and safely… Machine-speed systems need machine-speed money."* That is the payment primitive. Nunchi is the identity and trust primitive. Complementary, not competitive.

April 16, 2026: a16z crypto published a post identifying five missing primitives in the agent economy: KYA (Know Your Agent) identity, governance, x402 payments, trust pricing, and user control. Nunchi directly addresses primitives one and five (identity and trust) via ERC-8004 agent identities with full 7-domain reputation. The pitch in compressed form.

---

## 8. R13 Pitch Order (Rehearse in This Sequence)

1. **Open with the layer cake** (Keycard at identity, Temporal at execution, Nunchi at coordination) — making Casado repeat his own portfolio's pattern.
2. **Anchor on HIS quote** about token-path margin (*"everybody has to be on the token path"*).
3. **Volunteer the boundary with Yoko Li's Inngest and Sarah Wang's Temporal BEFORE asked.** Inngest ($21M Series A, Altimeter + a16z; Yoko Li portfolio) is workflow-as-code, not agent coordination. Temporal is durable execution inside a single org's trust boundary. Nunchi is the coordination layer above both.
4. **Lead traction with three numbers** (one logo, one OSS metric, one revenue/design-partner metric).
5. **Have Stripe data-room access ready** (post-Cluely default for ARR verification).
6. **Acknowledge Capsule and Nava by name** — naming competitors looks confident, not defensive.
7. **Run parallel funds on same timeline** — a16z moves in 3–6 weeks; run Plan B investors concurrently.
8. **Pre-send deck as PDF 24–48h ahead** — never DocSend; include one-page exec summary + demo links.

### The investor memo format (R15 correction)

The memo is **2,000 words / 4–5 pages, NOT 500 words.** Per Troy Kirwin's March 2026 a16z-speedrun essay, the memo IS the substance and the deck is the teaser. Aubakirova gets the memo over the weekend to draft the IC memo with 60% copy-pasteable language. Send Friday 6pm PT as a Google Doc link (not PDF — she needs to copy-paste into the IC template).

**Ten-section structure:**
1. One-liner + TL;DR (100 words) — *"Nunchi is the durable runtime for production agents."*
2. Why this team (250 words) — **LEAD WITH TEAM, NOT MARKET.** Solo-founder slide addresses the objection before it is asked.
3. Why now (300 words) — MCP/A2A convergence, EU AI Act August 2 deadline, Stripe locking payments lane.
4. Why this market (300 words) — coordination plane as the cleared infrastructure category.
5. Product and wedge (350 words) — Roko runtime, CascadeRouter, gate pipeline.
6. Distribution and GTM (250 words) — MCP server distribution playbook, compliance-led enterprise.
7. Traction (250 words) — design partner LOIs, GitHub metrics, benchmark results.
8. Competitive landscape (150 words) — name Capsule, Nava, Temporal; place Nunchi above each.
9. Five-year vision (200 words) — coordination plane as the canonical infrastructure for the agent economy.
10. Next 12 months (150 words) — milestones only; **NO dollar amount on slide** per Kirwin's March 2026 a16z guidance.

---

## 9. The a16z Decision Process

| Step | Detail |
|---|---|
| First meeting | 30–45 min with sponsoring partner |
| 1–3 follow-up dives | Over 1–2 weeks |
| Architecture deep dive | Technical evaluation by associate or platform team |
| Reference calls | 4–6 customer + back-channel; assume back-channel WITHOUT being told |
| Monday partnership pitch | All-partner meeting. ~25% of deals at this stage receive term sheet. 75% rejected. |
| Term sheet | 3–6 weeks total from first meeting |

**"Partner ambush" risk:** the sponsoring partner senses the room turning against the deal during Monday and pivots against to preserve credibility. Counter: ensure Casado has socialized the deal with other GPs BEFORE Monday. Pre-meeting deck distribution matters — gives Casado ammunition to pre-sell the deal.

**Pass signal:** a16z communicates pass via email within 3–7 business days. Phone call only if decision was close. Two weeks of silence = soft pass. **"Good news comes early."**

### Post-meeting protocol (R14)

- **Thank-you email within 4–6 hours, same day** (not next morning). Attach 8–12 customer reference contacts. Speed signals confidence.
- **Reference call floor:** prepare 8–12 customer references and 5–10 personal/professional references. Prepare design partners for inbound calls from a16z associates they do not expect.
- **Post-Cluely ARR verification:** Stripe/Chargebee read-only dashboard access OR a screenshare during financial diligence. Have ready: dashboard cleaned up, ARR definition documented (how you calculate it, what counts), MRR-to-ARR bridge, cohort retention table.

---

## 10. Pre-Send Protocol (R14)

**Deck pre-send:** Send the deck 24–48 hours ahead of the meeting as a PDF attachment. **NEVER use DocSend or gated links** — friction reads as paranoia to a16z partners. Include a one-page executive summary and demo links alongside the PDF. The deck they receive is the deck they share internally; make it self-contained.

**Calendar event awareness:** Track external events landing on or immediately before the meeting date. Partners read the headlines that morning. Examples for the May 6 meeting:
- DeepSeek V4-Pro 75% promotional discount expires May 5, 15:59 UTC. Have a one-liner ready: *"CascadeRouter routes to the cheapest qualified model at any given moment. When DeepSeek runs a promo, tasks route there. When the promo expires, they route elsewhere. The customer's cost stays optimized without manual intervention. The routing layer captures price dislocations automatically — that is the product."*
- OpenAI Workspace Agents goes paid May 6 (same day as pitch). Have a one-liner ready: *"OpenAI charging for Workspace Agents validates that agent infrastructure is a revenue category, not a feature. Their orchestration is single-vendor, single-model-family. Ours coordinates across vendors and captures the arbitrage between them. That structural difference is why Temporal exists alongside AWS Step Functions, and why Nunchi exists alongside OpenAI's native tooling."*

---

## 11. Investor Intelligence Summary

| Element | Detail |
|---|---|
| **Lead target** | Martin Casado (a16z infrastructure) |
| **Warm intro** | Malika Aubakirova (a16z, Big Ideas 2026 author, co-led Keycard) |
| **Round target** | $20–30M at $200–400M post |
| **Slide-one anchor options** | "The bottleneck is coordination" / "Re-architect the control plane for agents" / "The missing layer" |
| **Open the demo with** | *"Martin, you wrote that we can't yet close the control loop on agents. That's exactly why we built Nunchi as the control plane. Five minutes."* |
| **Closing line** | *"Ethane reduced enterprise networks to dumb forwarding elements governed by a logically centralized policy. Nunchi reduces LLM agents to dumb invocation elements governed by a logically centralized routing-and-gating policy."* |
| **The MOST important quote to internalize** | Aubakirova Big Ideas 2026: *"The bottleneck becomes coordination: routing, locking, state management, and policy enforcement across massive parallel execution."* |
| **Forbidden vocabulary** | "Web3 platform," "tokenomics," "blockchain company," "DeFi," "AGI," "agent autonomy as feature," "data moat" |
| **Required vocabulary** | "Machine identity," "the missing trust fabric," "from static identity to dynamic intent," "agent-native infrastructure," "coordination becomes a bottleneck," "agentic inference," "workload-model fit," "validated paths," "janky but native" |
| **Memo format** | 2,000 words, 10 sections, Google Doc link Friday 6pm PT |
| **Decision timeline** | 3–6 weeks. Monday partnership meeting = 25% term sheet rate. Two weeks silence = soft pass. |
| **Plan B in parallel** | Sequoia (Sonya Huang), Lightspeed (Bucky Moore), Benchmark (Chetan Puttagunta), Felicis (Astasia Myers), 468 Capital (Berlin), Air Street (Berlin-friendly) |
