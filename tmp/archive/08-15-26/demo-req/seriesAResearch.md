# Nunchi Series A Research Sweep — April 26, 2026

Pitch date: May 6, 2026. Lead target: Martin Casado (a16z). Parallel: Sequoia/Sonya Huang, Lightspeed/Guru Chahal, Benchmark/Eric Vishria.

---

## 1. Key facts that change the pitch (must-update items)

**Klarna is May 2025, not May 2024.** Bloomberg/Fortune coverage is dated May 8–9, 2025. The Feb 2024 claim ("700 agents replaced") was 15 months earlier; the *reversal* was May 2025. Verbatim Siemiatkowski quote: *"Cost unfortunately seems to have been a too predominant evaluation factor… what you end up having is lower quality."* Headcount went 5,500 → 3,400. Saying "May 2024" anywhere is a credibility-kill.

**Two fresher reversal anchors exist that are stronger than Klarna alone.** (a) **Commonwealth Bank of Australia, August 21, 2025** — rehired 45 customer-service staff after voice bot failed; union demonstrated call volumes were *rising*, not falling. (b) **NYC MyCity chatbot, January 30, 2026** — Mamdani killed it citing $500K cost and "functionally unusable" performance. The MyCity one is the only verified Q1 2026 reversal we found; use it as proof the pattern is current, not a 2024 artifact.

**The "$44.86 → $1.42" derivation is partially defensible but needs surgical edits.** $44.86 is not a HAL row I could find; the closest published number is **TAU-bench Airline, HAL Generalist + Claude 3.7 Sonnet, $42.11/task** (hal.cs.princeton.edu/taubench_airline). Cite that specific row. Critically, **HAL explicitly excludes caching from its cost calculations** ("Costs are currently calculated without accounting for caching benefits") — applying caching on top is mathematically valid, but you must footnote it or diligence will catch the gap.

**Multipliers reality check:**
- **Caching 5×**: theoretical max is 10× (Anthropic and OpenAI both quote 0.1× input pricing on cache reads). Best-documented production case: ProjectDiscovery's Neo agent, **74% cache hit rate, 59% net cost reduction (~2.4×)**. DigitalOcean's "70–90%" is a blog claim with no single citation. **5× is a defensible mid-range, but lead with the documented Anthropic 10× cache-read multiplier as the primitive, not "5×" as a vague aggregate.**
- **Cascade routing 3×**: RouteLLM's own paper (Ong et al., arXiv:2406.18665) reports **up to 3.66× on MT-Bench matching GPT-4 quality**, but the safer "no quality loss" number is **2×**. FrugalGPT shows 50–98% (2×–50×) range. **3× is at the upper bound of what RouteLLM specifically supports.**
- **Gate pre-screening 2× — this is the weakest link in the entire chain.** No primary paper or production case study cleanly establishes "2× from gating" as an independent multiplier. CUARewardBench (arXiv:2510.18596) shows the best ORM verifier hits only **80.1% accuracy** on computer-using agents — i.e., a gate is *useful but not a clean multiplier*. **Drop this multiplier. Replace with a quality/reliability claim citing MAST.**

**MAST's 41–86.7% multi-agent failure rate is still the headline stat.** v3 of arXiv:2503.13657 (Cemri et al., Berkeley) was published Oct 26, 2025. No Q1 2026 paper has produced a sharper replacement. Several follow-ups (MP-Bench, GraphTracer, CHIEF, AgentV-RL, CUARewardBench) corroborate the prevalence. **This is the single most defensible number in the entire reliability story.**

**ERC-8004 launched on Ethereum mainnet March 17, 2026 ("8004 Launch Day")** with audited Identity and Reputation registries deployed at `0x8004A169...` and `0x8004BAa1...`. Phala (TEE-attested), Oasis ROFL, TRON (TRC-8004), Morph, and Filecoin Pin all have reference implementations. **Nunchi's deferred L1 should be ERC-8004-native at genesis** — don't duplicate identity registry work.

**x402 metrics (April 21, 2026, per Base announcement):** ~69,000 active AI agents, 165M+ transactions, $50M cumulative volume; 85% on Base. **x402 Foundation incubated under Linux Foundation, co-founded by Coinbase + Cloudflare.** Visa, Stripe, AWS, Google, Mastercard, Circle backing. Google's AP2 uses x402 for settlement.

**MCP scale:** Anthropic disclosed **97M monthly SDK downloads, ~10,000+ active public MCP servers** (Linux Foundation donation Dec 9, 2025). Nerq Q1 2026 census indexed 17,468 servers, only 12.9% high-trust. **MCP donated to the Agentic AI Foundation (AAIF)** under the Linux Foundation along with Block's goose and OpenAI's AGENTS.md.

**Temporal Series D = $300M at $5B valuation, Feb 17, 2026, a16z lead, Lightspeed participating.** Customer logos disclosed: OpenAI, Replit, Lovable, Nordstrom, ADP, Abridge, Yum!, Block, Snap, Netflix, JPMorgan. Disclosed metrics: >380% YoY revenue growth, 9.1T lifetime action executions, 1.86T from AI-native customers. **Temporal explicitly framed Series D narrative around agents:** *"Agentic AI doesn't fail because models aren't good enough — it fails because the systems around them can't handle real-world execution."* This is your business-model precedent and your *flank target* — they own durable execution, leaving coordination open.

**Casado's firm wrote your "why now" slide for you.** a16z's "Big Ideas 2026 Part 1" (Malika Aubakirova, Dec 2025) — "**Agent-native infrastructure becomes table stakes**." Verbatim quotes available: *"thundering herd of agent-speed workloads,"* *"the bottleneck becomes coordination: routing, locking, state management, and policy enforcement,"* *"to a legacy database or rate-limiter, it looks like a DDoS attack."* **Cite this on slide 2.**

**Highest single risk to the pitch window: Temporal Replay 2026 (May 5–7, SF, Moscone)**, which is the day before and overlapping with the May 6 pitch date. R&D pipeline disclosed includes **Temporal Nexus** (Durable Application Communication = agent-to-agent durable RPC). If Nexus ships the morning of May 6, Temporal eats half your coordination claim. **Read every Replay 2026 announcement on May 5 evening before the pitch.**

---

## 2. Weakest current claims, verified against data

User flagged 8 weaknesses. Verdict on each:

1. **"$44.86 → $1.42 / 30× compound multiplier"** — **WEAK as constructed**. The chained multiplier is mathematically dependent (caching savings shrink as cascade routes more queries to cheap models that already have lower absolute cache benefit), and the gate 2× has no primary citation. Replace with a single defensible compound: **"~5–10× cost reduction by combining Anthropic's documented 0.1× cache-read pricing with RouteLLM's documented 2–3.66× cascade reduction."**
2. **"5× from caching"** — **MID DEFENSIBILITY**. Theoretical 10×, production best-case documented 2.4× (ProjectDiscovery). Defensible if framed conservatively.
3. **"3× from cascade"** — **DEFENSIBLE AT UPPER BOUND**. RouteLLM specifically supports 3.66× on MT-Bench. On agent workloads (less benchmark-saturating), 2× is safer.
4. **"2× from gates"** — **WEAK; NO PRIMARY CITATION**. Drop or repurpose as quality claim.
5. **"Klarna May 2024"** — **WRONG**. May 2025. Already addressed.
6. **"41–86% multi-agent failures"** — **STRONG, STILL CURRENT**. Use it.
7. **HAL as the canonical baseline** — **PARTIALLY DEFENSIBLE**. HAL is real, peer-reviewed (ICLR 2026), and Princeton-credible. But HAL has paused leaderboard updates to focus on the Reliability Dashboard, and HAL excludes caching by design. Cite specific row, footnote the caching exclusion, point at hal.cs.princeton.edu/reliability as the current SOTA lens.
8. **Solo-founder framing** — implicit weakness given Casado's pattern of backing technical founders with deep systems pedigrees. Mitigation: front-load OSS traction (GitHub stars, downloads, contributors) before founder slide. The user has already cut slide 4; that's correct.

---

## 3. New evidence to strengthen the pitch

**Reliability evidence (use on the "why agents fail" slide):**
- **MAST 41–86.7% failure rate**, Cemri et al., arXiv:2503.13657v3 (Oct 26, 2025).
- **CUARewardBench**, arXiv:2510.18596 — best ORM verifier only 80.1% accuracy on computer-using agents; agent task success 25.9–50.8%. Frames "even SOTA verification can't catch all failures, so coordination layer must own the failure surface."
- **AgentV-RL**, arXiv:2604.16004 (Apr 17, 2026) — agentic verifier 4B variant beats SOTA ORM by +25.2% accuracy. Use as evidence that verifier-augmented coordination beats raw model scaling.
- **HAL Reliability Dashboard** at hal.cs.princeton.edu/reliability — Princeton paused cost leaderboard updates to focus on consistency/predictability/robustness/safety/self-awareness. Reliability is now the live frontier; cost is yesterday's framing.

**Production cost anchors (use on the "agent unit economics" slide):**
- **Devin: $2.25/ACU (~$9/hour)** — docs.devin.ai/admin/billing. Crisp, official, per-task.
- **GitHub Copilot: $0.04/premium request** = 1 cloud-agent session = 1 premium request. Rare officially-disclosed *per-action* number.
- **Anthropic Claude Managed Agents: $0.08/session-hour** (Apr 8, 2026, public beta). Freshest possible.
- **Cursor Background Agents: ~$4.63/PR** observed; Bugbot 78% resolution rate.
- **Sierra: $150M ARR by Feb 2026** (Bret Taylor blog), $100M ARR in 7 quarters, voice surpassed text Oct 2025; outcomes-based per-resolution pricing.
- **Harvey: 25,000+ custom agents in production, 400K+ agentic queries/day** (Mar 25, 2026 $11B raise blog) — **the strongest "production agents are scaling" stat in the application layer.**

**Reversal stories (use on the "AI-replaces-humans bet is breaking" slide):**
- Klarna (May 8–9, 2025), CBA (Aug 21, 2025), NYC MyCity (Jan 30, 2026). Pattern from May 2025 → April 2026, accelerating.

**Coordination-protocol momentum (use on the "standards convergence" slide):**
- ERC-8004 mainnet launch March 17, 2026.
- x402: 165M+ tx, $50M volume by April 21, 2026 (3× growth Dec 2025 → Apr 2026).
- A2A v1.2: 150+ organizations in production (one-year mark Apr 9, 2026).
- AAIF (Linux Foundation): Anthropic, OpenAI, Block, Cloudflare, Google, Microsoft, AWS as Platinum; **Temporal listed Gold** — important political signal.

**Karpathy autoresearch (March 7, 2026) — use on the "why now" slide.** 66K+ stars in <1 month. Tobi Lütke ran 35 agents on Hyperspace network, 333 unsupervised experiments, 19% perf gain. *"You spin up a swarm of agents… you promote the most promising ideas to increasingly larger scales."* This is the freshest cultural reference you can put in front of an infra GP in May 2026.

---

## 4. Competitor moves to address proactively

**Most urgent (must address in deck, in this order):**

1. **Temporal ($5B, a16z-led).** Frame: *Temporal solved durable execution for backend services. Nunchi solves coordination for fleets of agents at the edge — Rust runtime, model-agnostic, sovereign-chain optional. Same business model (Apache + enterprise + managed cloud), distinct primitive.* Cite their "systems around the model fail" framing approvingly. Be the *successor*, not the rival.

2. **Keycard ($38M, a16z + Acrew).** This is the politically sensitive one: a16z already holds the agent-identity chip. Frame: *Keycard authenticates the agent. Nunchi tells the agent what to do next.* Position cleanly above (coordination consumes identity) or below (runtime emits identity events). Do not appear to overlap.

3. **OpenAI Frontier + Agents SDK (Sandbox Agents beta, Apr 2026).** OpenAI is now competing on runtime layer (shell tool, hosted container workspace, context compaction). Frame: *Frontier is hosted-only and Claude/Gemini-locked-out at the runtime. Nunchi is the open, model-portable equivalent.*

4. **Cloudflare Agent Cloud (Apr 13, 2026: Replicate acquired, Dynamic Workers, Think SDK persistence).** This is the most strategically coherent hyperscaler agent-runtime competitor: A2A-adjacent, MCP-integrated, x402 Foundation co-founder, edge runtime. Position as *Cloudflare-locked at every layer; Nunchi is portable Rust runtime that runs on any edge or sovereign chain.*

5. **Tempo (Stripe + Paradigm payments L1, mainnet live Mar 18, 2026).** Visa, Standard Chartered, Stripe as validators. **Strongest sovereign-L1-for-machines precedent.** Frame for a deferred 2027 L1: *Tempo is a payments L1 that machines use. Nunchi's L1 is a coordination/reputation L1 that agents need. Distinct primitives.* Acknowledge them; don't duplicate their corporate-validator strategy unless you actually have those relationships.

6. **0G Aristotle Mainnet (live since Sept 2025), Olas (10M+ a2a tx but $10M token cap), Story Protocol (delayed token unlock, layoffs, ~$0/day on-chain revenue), ASI Alliance (FET -94% from ATH).** **Use this list defensively**: most "sovereign agent L1" narratives are token-fatigued or struggling. Nunchi's deferred-2027 L1 is credible *because* it's deferred — you're shipping the runtime first, accumulating real coordination metadata, then earning the right to a chain. Frame it that way explicitly to head off "another agent L1?" objections.

7. **Nava ($8.3M seed Apr 14, 2026, Polychain + Archetype, EigenLayer pedigree).** Verification layer + escrow + NavaUSD + Arbitrum L3. Closest *funded* competitor on agent-payment-trust narrative. They will ship before you. Differentiator: Nava is a verification *layer* (L3); Nunchi is runtime + (later) sovereign L1. Cite them respectfully; don't ignore.

8. **Microsoft Agent Framework 1.0 (Apr 6, 2026, .NET + Python).** Third framework in 18 months (Semantic Kernel → AutoGen → AF). Brand-damaged with devs. *"Stability commitment is a wedge"* — LangGraph 1.0 on Apr 16 explicitly committed "no breaking changes until 2.0." Nunchi should match this commitment publicly.

**Lower priority (mention only if asked):** CrewAI ($18M, Insight, $3.2M revenue per Latka — vulnerable), Letta ($10M seed, pivoted to Letta Code app — diluted infra positioning), Adept (effectively dissolved at Amazon), Rabbit (alive but distressed), Parlant (vertical to banking), Replit Agent, Augment, Sourcegraph Amp.

---

## 5. Casado / Huang / Vishria language patterns

### Casado — incorporate

- *"Every time you have a technical epoch, you have to redo everything, and we forget that every time."* (Six Five Pod, Mar 2026) — perfect setup for "agents are a new technical epoch; coordination must be redone."
- *"Coding is pretty much dead, but engineering is very much not."* (Six Five) — frame Nunchi as engineering infrastructure, not a coding tool.
- *"Build a non-consensus product, give a consensus pitch."* (Newcomer 2025) — lean into the consensus framing.
- *"It's unimportant if VCs understand your world. It's incredibly important they understand why your world has changed and how you're positioned to capture it."* (BI 2021, advice given to Temporal founders) — frame the inflection, not the tech.
- **Single highest-leverage move**: Quote Aubakirova's a16z Big Ideas 2026 Part 1 verbatim on slide 2. *"Agent-native infrastructure becomes table stakes… the bottleneck becomes coordination."*

### Casado — avoid
"AI-powered," "next-generation," "transformative," "democratize," "AGI" as feature, governance/safety as wedge, foundation-model-commoditization narratives (he disagrees), passive voice, hedging, "platform" without specificity.

### Huang — incorporate
- *"The AI applications of 2023 and 2024 were talkers. The AI applications of 2026 and 2027 will be doers."*
- *"Users will go from working as an IC to managing a team of agents."*
- *"It's time to ride the long-horizon agent exponential."*
- LangChain framing she wrote on her bio: *"chains of reasoning and tool use, not just single calls to a model, are what unlock complex behavior."* — Nunchi makes the chains durable.
- AGI-pilled, "tokenmaxxing," METR long-horizon doubling.

### Huang — avoid
Doomer/safety framing. She is "very AGI-pilled" and rolls eyes at hedged AI-risk talk.

### Vishria (Benchmark) — incorporate
Frame Nunchi as **"Confluent for agents"** — neutral, durable, stateful coordination substrate. He invested in Confluent at Series A on the picks-and-shovels thesis; the parallel is direct. His public quote: *"as the cost of generating code drops to near zero, the volume will explode, making human review impossible."* — perfect setup for Nunchi as the verification/orchestration layer.

### Lightspeed — Guru Chahal is the right partner
Their Paid investment (Mar 2026): *"missing economic infrastructure for AI agents."* Their Resolve AI thesis: "AI for production." Branding language to mirror: *"Depth is our center of gravity."* Lightspeed likes deep infra, not breadth.

### Gurley
**Don't pitch first.** He's publicly bearish on AI capex ("a bunch of people got rich quick and a reset is coming," Fortune Mar 16, 2026). Vishria is the Benchmark agent-infra contact.

---

## 6. Specific numbers — use vs. drop

**USE (high defensibility):**
- $42.11/task — TAU-bench Airline, HAL Generalist + Claude 3.7 Sonnet (cite hal.cs.princeton.edu/taubench_airline)
- 0.1× cache-read input pricing (Anthropic + OpenAI documented)
- 2–3.66× cascade routing (RouteLLM, arXiv:2406.18665)
- 41–86.7% multi-agent failure rate (MAST, arXiv:2503.13657v3)
- 80.1% verifier accuracy ceiling on computer-using agents (CUARewardBench, arXiv:2510.18596)
- $2.25/ACU Devin, $0.04 Copilot, $0.08/session-hour Anthropic Managed Agents
- 97M MCP SDK downloads/month, 10,000+ MCP servers
- 165M x402 transactions, $50M volume (Apr 21, 2026)
- 25,000+ Harvey production agents, 400K+ daily agentic queries
- Klarna May 2025, CBA Aug 21, 2025, NYC MyCity Jan 30, 2026
- Karpathy autoresearch 66K stars, Lütke 35-agent 333-experiment 19% lift

**DROP (or substantially weaken):**
- "$44.86" — not a HAL row I can find; replace with $42.11 + footnoted source
- "5× caching" as a standalone multiplier — replace with "Anthropic 0.1× cache-read pricing"
- "2× gate pre-screening" — drop or convert to quality claim
- 30× compound multiplier — replace with "5–10× compound, sourced to Anthropic + RouteLLM"
- Air Canada $880 figure — actual was C$812.02
- Klarna May 2024 — actual was May 2025
- Any "ChainUpAd 130k ERC-8004 agents" projection — unverified marketing blog
- BenchLM "Claude Mythos 89.2% TAU-bench" — self-flagged display-only
- Decagon "$50K platform fee" — secondary source only

**SINGLE NUMBER TO LEAD WITH (recommendation):**
*"Multi-agent systems fail 41–86% of the time in production (MAST, Berkeley, arXiv:2503.13657). Coordination — not raw model capability — is the bottleneck."* This is reliability-first, not cost-first, which sidesteps the multiplier-stack argument entirely and aligns with HAL's own pivot to the Reliability Dashboard.

---

## 7. Visual reference list — what to steal from each

| Source | What to steal |
|---|---|
| **Vercel** (vercel.com/geist) | Geist Sans + Geist Mono, tight `-0.04em` tracking, near-zero border-radius, live HTML code-editor hero |
| **Linear** | One bold accent on near-mono palette, tracked-out 12px uppercase eyebrow labels above section headers |
| **Anthropic** | Warm `#FAF9F5` cream + `#D97757` terracotta accent — break from "AI blue" sea. Sans-for-UI + serif-for-research pairing signals depth |
| **Cursor** | Embedded interactive code-editor demo in hero — visitors interact, don't watch a video |
| **Modal** | The canonical "code-as-hero" two-pane layout: real Rust code on left, streaming terminal trace on right |
| **Stripe** | 3-column docs IA (nav / prose / sticky right-side code), live syntax-highlighted snippets with tab support |
| **Tailscale** | Granular `--color-gray-0` through `--color-gray-1100` named scale — "expensive" look |
| **Warp** | Terminal-block UI with traffic-light window chrome — telegraphs "dev infra" instantly |
| **Fly.io** | Permission to be quirky/illustrated — but use sparingly |
| **Front Series A deck** (alexanderjarvis.com/front-series-saas-startup-pitch-deck) | "Boring, consistent, anyone-can-do-it-in-PowerPoint" — the explicit lesson. White bg, tight headlines, charts not mockups |
| **Pitch's own $85M Series B writeup** | Process: Notion (objectives) → Figma (visual) → Pitch (final). One bold image per slide, mono accent type |
| **Notion 2013 seed deck** (published as Notion page) | Strong narrative > strong design |

**Stack recommendation for Nunchi:** shadcn/ui with `--base-ui` flag + Geist Sans/Mono + Tailwind v4 `@theme` tokens. Near-black `#0A0A0A` bg, single accent — pick **Anthropic terracotta `#D97757`** if you want to differentiate from the "AI blue" cohort, or **Alephic blue `#1C3FFD`** if you want to look like cohort-default. Modal-style split-pane code-as-hero. `cargo install nunchi` copy-block under H1. Tokyo Night terminal theme for product demos. Hairline borders `rgba(255,255,255,0.08)`. One Rive animation, no aurora gradients.

**Avoid in 2026:** Inter (now reads as Tailwind starter default), bright `#2563EB` AI blue, aurora/spotlight gradients (Aceternity tell), glassmorphism, default Tailwind shadows, "Book a demo" primary CTA, scrolling logo carousels with no logos, Manrope/Poppins/Space Grotesk.

---

## 8. Positioning rewrite options — two distinct frames that beat the current

### Frame A — "The coordination primitive" (Casado-optimized)

> **Nunchi is the coordination primitive for agent fleets.**
>
> Workflows became a primitive when microservices made them inevitable. Coordination becomes a primitive when agents do. *(Citing Casado: every technical epoch you redo everything.)*
>
> Multi-agent systems fail 41–86% of the time (MAST, Berkeley). Production agent costs are real and disclosed: Devin $2.25/ACU, Copilot $0.04/action, Sierra $150M ARR. Harvey runs 25,000 custom agents and 400K daily agentic queries. The agents exist. The coordination layer doesn't.
>
> Roko is the open Rust runtime — Apache 2.0, MCP-native, A2A-native, ERC-8004-compatible. Same business model as Temporal: open core, enterprise contracts, managed cloud. The sovereign chain ships in 2027 once we've earned the right to one through real coordination metadata.

This is the **safest** frame. It maps directly onto Aubakirova's a16z essay, sits adjacent to (not on top of) Keycard, uses Temporal as precedent rather than rival, and front-loads disclosed numbers.

### Frame B — "The control plane for agent-led growth" (Huang-optimized)

> **Agent-led growth needs a control plane.**
>
> Sonya Huang's bet: users go from ICs to managers of agent teams. That future has a missing layer: when 10,000 agents share state, hand off work, and resolve conflicts at agent-speed, *what owns the substrate?*
>
> Karpathy's autoresearch (March 2026) showed what one agent can do overnight. Tobi Lütke ran 35 agents in parallel and got 19% perf lift on day one. The next 100× of that is coordination — not models.
>
> Roko: Rust agent runtime, open-source, durable, model-agnostic. The coordination plane that A2A and MCP need underneath them.

This frame is **bolder** — better fit for Sequoia/Huang. Aligns with her "long-horizon agents are AGI" thesis and her "managing a team of agents" framing. Risk: weaker for Casado, who is more skeptical of pure-thesis pitches and wants the primitive.

**Use Frame A as default.** Have Frame B ready as the closing slide (vision/future) for a Huang-led conversation.

---

## 9. Slide-by-slide recommendations (12 main slides, slide 4 founder cut)

**Slide 1 — Title.** "Nunchi: the coordination primitive for agent fleets." Geist 96pt. Single accent. No tagline soup.

**Slide 2 — Why now.** Quote Aubakirova/a16z verbatim: *"Agent-native infrastructure becomes table stakes… the bottleneck becomes coordination."* Add Karpathy autoresearch (Mar 7, 2026) and Lütke's 35-agent run as the cultural anchor. This is the consensus pitch for the non-consensus product.

**Slide 3 — Problem.** Three numbers, one chart: **41–86% multi-agent failure (MAST), 80.1% verifier accuracy ceiling (CUARewardBench), 25,000 production agents at Harvey alone.** Subhead: *"Production scale is here. Coordination isn't."* Don't lead with cost — lead with reliability.

*[Slide 4 founder — cut. Correct call.]*

**Slide 4 (was 5) — Why this fails today.** Three reversal anchors with dates: Klarna May 2025, CBA Aug 2025, NYC MyCity Jan 2026. Quote Siemiatkowski directly. Frame: *"The AI-replaces-humans bet broke publicly. Coordination is the layer that makes agents a credible substitute for managed work."*

**Slide 5 — What we built.** Roko: 18 Rust crates, Apache 2.0, MCP-native, A2A-native, ERC-8004-compatible. Show the architecture diagram. Modal-style split: Rust code defining an agent on left, streaming trace on right. **No marketing adjectives.**

**Slide 6 — The coordination primitive.** What it does that Temporal/Keycard/LangGraph can't: multi-agent state, cross-agent handoffs, conflict resolution at agent-speed, identity-aware routing, payment-aware execution. One diagram, three callouts.

**Slide 7 — Demo / proof.** Live demo or recorded clip. Tokyo Night terminal aesthetic. Show a real agent fleet doing real work with traces. **No fake `[INFO]` spam.** Real reasoning → action → observation blocks.

**Slide 8 — Numbers.** Revised cost claim: *"On TAU-bench Airline (Princeton HAL, ICLR 2026), the baseline is $42.11/task. Combining Anthropic's documented 0.1× cache-read pricing with RouteLLM's 2–3.66× cascade reduction (Berkeley, arXiv:2406.18665), Nunchi delivers 5–10× cost reduction on agent workloads — without the verification gap MAST documents."* Drop the gate multiplier. Footnote HAL's caching exclusion.

**Slide 9 — Why us / why now (technical).** OSS traction: GitHub stars, downloads, contributors, Discord. Show momentum, not credentials. *(Slot the technical-founder pedigree as a footer line — Casado fundability — but don't make it the slide.)*

**Slide 10 — Business model.** Temporal precedent. Apache 2.0 runtime + enterprise support contracts + BSL managed cloud (4-year Apache conversion, named explicitly to head off post-HashiCorp BSL skepticism). Disclose Temporal's $5B / $300M Series D / a16z lead as the precedent. **Be the successor.**

**Slide 11 — Standards & ecosystem.** MCP, A2A, ERC-8004, x402. Linux Foundation AAIF positioning (target Silver+ membership). One-line position on each. Frame: *"Nunchi is the coordination plane that ratifies these standards in code."*

**Slide 12 — Roadmap & ask.** Runtime GA path. Sovereign EVM L1 deferred to 2027 — explicitly framed as *"earned through real coordination metadata first."* This pre-empts the "another agent L1?" objection. The ask, the team plan, the milestones to next round.

**Closing slide (vision, optional Huang-frame).** Long-horizon agent exponential, METR doubling, "managing a team of agents." One sentence: *"What will you build when your plans are measured in centuries?"*

---

## 10. Red flags / objections at a16z

1. **"Why isn't this absorbed by the model labs / GPU clouds?"** Counter: cite Casado's "infrastructure inversion" — agents *choose* infra, decoupling buying from labs. MCP/A2A/ERC-8004 standardization is the proof.

2. **"How is this not LangGraph / Temporal / Cloudflare Workflows / Restate / Inngest?"** Casado is on the Convex board and a16z funded Keycard — he knows every adjacent tool. Differentiate on the *coordination primitive*: multi-agent state, hand-offs, conflict resolution at agent-speed vs. workflows (single-agent durability) and identity (Keycard/Descope).

3. **"Cloudflare or AWS could build this in a sprint."** Counter: Confluent (Vishria's Series A bet) proved neutral coordination beats hyperscaler messaging. Open standards make neutral infra valuable; Salesforce Agent Fabric and MS Agent 365 are vendor-locked, leaving the neutral coordination layer open.

4. **"a16z already invested in Keycard. How does this complement, not overlap?"** This is the **most politically important objection**. Pre-position cleanly: *"Keycard authenticates the agent. Nunchi tells it what to do next."* Make the orthogonality unmistakable on slide 6.

5. **"Where's the data moat?"** Casado emphasizes data + integration as defensibility. Counter: coordination metadata is the moat — once Nunchi sees agent traffic patterns, it becomes the routing layer. Reference Cloudflare's V2 control plane redesign as proof state-at-scale is itself a moat.

6. **"Pricing model?"** Casado has co-written extensively on consumption-based AI pricing. Counter: align with consumption + per-coordination-event metering; never pitch seat-based.

7. **"This feels like a feature."** Counter with Temporal's own counter: *"workflows looked like a feature until microservices made them a primitive. Agents make coordination a primitive."* Use Casado's *"every technical epoch you redo everything"* line back at him.

8. **"Are you actually non-consensus or riding the hype wave?"** Counter (Newcomer 2025 Casado quote): *"Build a non-consensus product, give a consensus pitch."* Lean into the consensus framing — *"a16z wrote 'agent-native infrastructure becomes table stakes.' We're the team to ship the coordination primitive Aubakirova described."* Don't pretend to be contrarian.

9. **"Bubble objection" (more from Sarah Wang growth team than Casado).** Position Nunchi as *deflationary* infra — coordination reduces wasted agent compute. Cite Karpathy/Lütke 19% lift, Chamber's 50% workload increase claim. Frame as the shovels for the rebound.

10. **"Why deferred L1 to 2027 and not now or never?"** Counter: *"Tempo earned a chain by getting Visa, Stripe, Standard Chartered as validators. 0G earned one with $250M token + Alibaba. Story Protocol launched a chain without earning it and is now pivoting to enterprise licensing. Nunchi earns a chain through accumulated coordination metadata first."* This pre-empts the "another agent L1?" objection by acknowledging the graveyard.

11. **Solo-founder objection.** Casado backs technical founders with deep systems pedigrees (his Nicira/SDN root). Mitigation: lead with OSS traction, technical depth on the architecture slide, and recruiting plan in the ask. Cut the founder slide as planned but ensure the technical depth shows in slides 5/6/7.

12. **Replay 2026 timing risk.** Temporal's conference May 5–7 overlaps the pitch. Read all May 5 announcements (especially anything Nexus-related) the night before May 6. Have a one-line framing ready: *"Temporal Nexus solves backend service-to-service durability. Nunchi solves agent-to-agent coordination at the edge. Both can be true."*

---

## What's missing or unverifiable

- **No public Temporal Series A deck**, but the **2020 $18.75M Series A deck IS public** at alexanderjarvis.com/temporal-pitch-deck-to-raise-18-75m-series-a-round — this is the single most relevant comp to study.
- **No public Cursor, Anthropic, Mistral (deck), Modal, Pinecone, LanceDB, Replicate, Decart, Mira, Lambda, RunPod Series A decks** found.
- **Linux Foundation "Agent UCID"** working group did not surface — likely nascent or renamed; verify via AAIF GitHub orgs before citing.
- **"Roko"** doesn't show up as a public competitor (it's Nunchi's own runtime name).
- **Rabbit financial distress** is from a single aggregator (digitalapplied.com), not primary disclosure — flag as unconfirmed.
- **Several tracker numbers** (CrewAI revenue from Latka, Sacra-derived Sierra/Decagon/Hebbia/Harvey pricing) are estimates, not company-disclosed. Cite as "per Sacra" or "reported."
- The user's "Sonya Huang Plan B thesis" likely refers to Cahn's $600B follow-up or Huang's "Act 2" framing, not a distinct named "Plan B" essay — couldn't locate one under that exact name.