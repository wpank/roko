# Deck and Memo Build Checklist

> **Context**: Nunchi is pitching a16z on May 6, 2026 for a Series A ($20-30M). Two written artifacts must be ready: a 13-slide pitch deck (PDF) and a 2,000-word pre-read memo. Both go out by Friday May 1.
>
> **Category**: Agent Coordination Plane — the infrastructure layer that separates agent coordination from agent execution (SDN analogy).
> **Target**: Martin Casado (infra fund) via warm intro through Malika Aubakirova.
> **Design system**: Geist Sans + Geist Mono, `#000000` bg, `#FAFAFA` text, `#0070F3` accent. Build in Figma, present from Keynote, export PDF.

---

## The Deck (13 main slides + 8-15 appendix)

### Production Pipeline
- [ ] Build in **Figma** (design) or **Keynote** (presentation)
- [ ] Export to **PDF** for data room and email
- [ ] NEVER put behind DocSend (friction reads as paranoia — partners need to forward internally)
- [ ] Total visible text: under 400 words across all 13 slides
- [ ] Font sizes: 64pt display, 36pt headlines, 24pt body, 24-28pt code (Kawasaki 30pt floor)
- [ ] 15-20 words per content slide, under 10 for transitions

### Slide-by-Slide Content

**Slide 1 — Title**: "Nunchi — the durable runtime for production agents." Thesis: "The model is the same. The system is the variable." No animation, no number. Logo + thesis only.

**Slide 2 — Why Now**: Three tailwinds: production agents shipped at scale (Cursor 35%+ internal PRs), MCP/A2A standardized the substrate, EU AI Act makes auditability a procurement requirement. Casado says "I always start with what is the market" — lead here.

**Slide 3 — Problem**: "Agents broke reliability — again." Temporal's narrative reframe: 2010 monoliths → 2020 microservices → 2026 agents force same boilerplate around tool calls, state, policy. One giant stat: "41-86% of multi-agent deployments fail. 79% from coordination." Below: Aubakirova quote.

**Slide 4 — Solution as Code**: ≤10 lines side-by-side. LEFT: 50+ lines of retry/timeout/audit/identity boilerplate. RIGHT: same result in Nunchi. Headline: "Agents-as-code. Reliability primitives, not boilerplate." Tokyo Night code block, no fake terminal chrome.

**Slide 5 — Architecture**: Three labeled bands: agent (data plane) → Nunchi (control plane — policy, identity, durable state, audit) → enterprise systems. THE Casado identity test. "The agent never touches your APIs directly. We mediate every tool call."

**Slide 6 — How It Closes the Loop**: observe → decide → enforce → record. Answers his April 2025 skepticism head-on. One example primitive per word.

**Slide 7 — "Let me show you"**: Blank slide. Center text. Cmd-Tab to terminal.

**Slide 8 — Traction**: 4-6 named logos. One usage-depth metric. One velocity number with comp. NO LOC (Casado disqualified it). NO star count. Format: Temporal's "Snap, Box, Coinbase, Checkr" pattern.

**Slide 9 — Competition**: Primitive-comparison matrix (rows: LangChain, OpenAI Agents SDK, Anthropic Managed Agents, Temporal, Nunchi. Columns: model-agnostic, durable state, identity/policy, audit/replay, OSS). Nunchi all-green. "Walled gardens are incompatible with multi-model enterprise reality. We are Switzerland."

**Slide 10 — Why It Compounds**: Three flywheels: eval/trace data every run produces, integration breadth (N×M×K compounds switching cost), system of record (6 months of audit = institutional memory).

**Slide 11 — Business Model**: Apache 2 core, cloud-hosted commercial. "Open project = PMF speedrun. Monetize multi-tenant managed runtime + RBAC, SSO, audit, compliance." Cite Casado's own data: "Cloud offerings churn ~10% vs open-core ~15%."

**Slide 12 — Team**: Founder bio (hyperlink LinkedIn). Named #2 closing in 90 days. 5 senior hires in pipeline. 2-3 advisors. Preempt solo-founder risk before it's asked. Reference Pinecone: "Edo Liberty brought on Bob Wiederhold at scale — that's the path."

**Slide 13 — Ask**: "Next 12 months" with three milestone tracks: design partners in production, OSS/SDK developer count, early Cloud ARR signal. DO NOT put dollar amount on slide (Kirwin guidance). End on milestones, not money.

### Appendix (6-10 slides, dense, not presented live)
- [ ] Architecture deep-dive (control plane internals, 6-stage pipeline)
- [ ] Security model (CaMeL-style IFC, gate pipeline)
- [ ] Pricing detail (per-action, enterprise tiers)
- [ ] Hiring plan (first 10 roles, timeline)
- [ ] Customer case studies (if available)
- [ ] Full competitive matrix (expanded Harvey-Ball/Power Grid)
- [ ] Token graveyard: "Why we defer the token" (VIRTUAL -86%, ELIZAOS -99.98%, FET -94%)
- [ ] Regulatory timeline (EU AI Act, FINRA, Colorado AI Act)
- [ ] ISFR expansion thesis (DeFi SOFR moment — for Dixon/crypto fund)

---

## The Pre-Read Memo (2,000 words / 4-5 pages)

### Format and Delivery
- [ ] Write as **Google Doc** (shareable, commentable)
- [ ] Also export as **PDF** backup
- [ ] Send **Friday May 1, 6:00 PM PT**
- [ ] Send via 5-line email with Doc link + PDF attachment + deck as separate PDF
- [ ] Subject line under 60 characters: "Series A: Agent Coordination Plane — [traction stat]"
- [ ] First sentence of email LITERALLY says what the product does

### 10-Section Structure (per Kirwin's a16z-speedrun template)

**Section 1 — One-liner and TL;DR (100 words)**
"Nunchi is the control plane for production agents — durable execution, policy, identity, and audit as first-class primitives." Three bullets: what, why now, next 12 months.

**Section 2 — Why This Team (250 words)**
LEAD WITH TEAM, NOT MARKET. Hyperlink LinkedIn. Specific technical credentials. Address solo-founder risk in opening paragraph: named co-founder-track hire's profile and timing, 5 senior-hire pipeline, advisor bench.

**Section 3 — Why Now (300 words)**
Three tailwinds with citations: model-capability inflection (cite Aubakirova's OpenRouter 100T-token study), MCP/A2A standardization, regulatory shift. Cite "Et Tu, Agent?" as evidence the firm sees supply-chain risk.

**Section 4 — Why This Market (300 words)**
Avoid hand-wave TAM. Concrete personas: "Fortune 500 fintech compliance teams blocked from deploying agents because they can't pass SR 11-7 / SOC 2 / FINRA review." Anchor to Auth0 ($6B), Datadog ($40B), Snowflake.

**Section 5 — Product and Wedge (350 words)**
Architecture in plain language. Aubakirova's litmus test: "Are you fundamentally shifting the workflow — or wrapping something old in AI?" Cite MCP, WIMSE, OAuth-for-agents, SPIFFE.

**Section 6 — Distribution and GTM (250 words)**
Casado's #1 founder failure: "thinking what they created has intrinsic value." Show bottoms-up (OSS adoption) + top-down (named design partners, founder-led sales through ~$4M ARR per his Market Annealing essay).

**Section 7 — Traction (250 words)**
Use a16z metric vocabulary. Distinguish ARR vs annualized run-rate. Show logo retention by cohort (Aubakirova's "Cinderella Glass Slipper" essay made retention curves her PMF signal).

**Section 8 — Competitive Landscape (150 words)**
Acknowledge LangChain, OpenAI Agents SDK, Anthropic Managed Agents, Temporal. Differentiate by architectural primitive. "We are Switzerland."

**Section 9 — Five-Year Vision (200 words)**
Wedge-to-category: Casado's "find the white space, worry about defensibility later."

**Section 10 — Next 12 Months (150 words)**
Specific milestones. NO round size — "that should be a live discussion."

---

## Verification

- [ ] Deck and memo tell the SAME story with the SAME numbers
- [ ] No "trust layer" language in either
- [ ] No LOC claims in traction
- [ ] No fake data
- [ ] Casado quotes cited correctly (verify against primary sources)
- [ ] Aubakirova quotes cited verbatim from Big Ideas 2026
- [ ] Keycard differentiation is sharp and explicit
- [ ] Temporal boundary is volunteered, not defensive
- [ ] PDF renders correctly on MacBook, iPhone, and when printed
