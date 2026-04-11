# Demo Competitive Landscape: Nunchi Series A

**Purpose**: Comprehensive competitive analysis — every relevant competitor's demo, product experience, visual design, strengths, and gaps. Written for someone with zero prior context. This document informs what Nunchi must match, exceed, and differentiate against in its demo.

**Date**: April 2026

---

## 1. The Competitive Map

The agent infrastructure space is organized into five layers. Nunchi's thesis is that the coordination plane — a sixth layer that sits above and connects the others — is structurally missing and structurally necessary.

```
Layer 5: Applications        (Devin, Cursor, Replit Agent)
Layer 4: Evaluation/Trust    (Braintrust, Nava, Capsule, LangSmith)
Layer 3: Orchestration        (CrewAI, AutoGen, LangGraph)
Layer 2: Frameworks           (LangChain, Mastra, Vercel AI SDK)
Layer 1: Models               (Anthropic, OpenAI, Google, open-weight)

Layer 6: Coordination Plane   (Nunchi — EMPTY QUADRANT)
```

Every competitor occupies one or two layers. None spans the full stack from identity through execution through knowledge through settlement. The coordination plane is the gap.

---

## 1b. Casado Portfolio No-Go List

**NEVER criticize these companies in any meeting with Casado or Aubakirova** — they are board seats or close a16z portfolio companies:

Cursor, Convex, Netlify, Kong, Truffle, Material Security, Pindrop, Fivetran, Ideogram, World Labs, Fly.io, Imply, Braintrust

**Direct teardown energy at**: LangChain, AutoGen, CrewAI, or Anthropic's MCP runtime instead.

**Special portfolio integrations to pre-build before the meeting**:
- **Braintrust** ($800M, Casado/a16z): AI evaluation platform. *"Braintrust evaluations can feed directly into our gate pipeline as a custom gate."* Pre-build one integration.
- **Cursor**: *"Cursor agents running through Nunchi's CascadeRouter cost 3-5x less because routine substeps get routed to cheaper models automatically."* Complementary, never competitive. Casado is a board member.

---

### Keycard — Agent Identity Infrastructure (a16z Portfolio)

**What they are**: Dynamic, identity-bound, task-scoped tokens for agent access control. First production implementation of OAuth 2.1 Client ID Metadata Documents. Extends MCP, WIMSE, OAuth-for-agents. Tokens are bound to attested workload identity, ephemeral, just-in-time, and runtime-revocable.

**Three scoping vectors**: (1) agent identity (who), (2) action permissions (what), (3) delegation chain (for whom).

**The overlap concern**: This is the single most likely "isn't this the same thing?" objection from the room. Malika co-authored the Keycard investment memo. It must be handled cleanly.

**The defusal framework** (from her own Kill-Chain essay):
- Keycard = intra-org issuance / runtime enforcement (works inside one trust domain, requires a trusted issuer)
- Nunchi/ERC-8004 = cross-org reputation / settlement (no shared IdP needed, sovereign verification)
- Same axis (static identity -> dynamic intent), perpendicular extension (centralized issuer -> sovereign verification)
- Preserve her vocabulary: "from static identity to dynamic intent"

**What to say**: "Keycard solved the Auth0 moment inside the org. ERC-8004 takes the same primitive across the trust boundary — same dynamic-intent token, but issued by no one and verifiable by everyone."

**What NOT to say**: Anything negative about Keycard. It's a16z portfolio. Position as collaborator, next-stage evolution.

**Integration pre-build**: Show that Nunchi can consume Keycard tokens as a trust signal in the ERC-8004 attestation flow. Pre-build before the meeting.

---

## 2. Competitor-by-Competitor Analysis

### Temporal — Durable Execution Platform

**Valuation**: $5B (February 2026, led by Sarah Wang and Raghu Raghuram at a16z — NOT Casado)
**What they are**: Durable execution for workflows. If your code crashes, the workflow resumes from the last checkpoint.

**Their demo experience**:
- Homepage walks through 9 progressive steps as diagrams: SDK support, workflow orchestration, retry flows, infrastructure deployment, worker scaling, CLI, web UI
- The aha moment: spin up a local server in under 2 seconds via CLI, show a workflow surviving a crash and resuming exactly where it left off
- The replay mechanism (re-executing workflow history without re-running completed activities) is the technical magic

**Their Web UI** (redesigned 2024):
- Three visualization modes for workflow execution:
  1. **Compact View**: Left-to-right linear progression, identical events collapse into single lines with counts
  2. **Timeline View**: Event groups stacked vertically, with elapsed time encoded as the physical length of connecting lines. Updates real-time for running workflows. This is their signature visual.
  3. **Full History View**: Git-tree style with a thicker main workflow line and event groups branching outward
- Visual language: dots = individual events, dashed lines = pending, dashed-red = retrying, green = completed, red = failed
- Dark mode added as "Night Mode"

**Strengths**:
- "Durable execution" is now an established category — they defined it
- The kill-and-resume demo is iconic. Temporal's "Snakes" pattern (named after an internal demo) has been copied by every durable execution pitch
- Real enterprise traction: Snap, Box, Coinbase, Checkr as customers
- $5B valuation proves infrastructure orchestration commands premium multiples

**Gaps (where Nunchi differentiates)**:
- Temporal is for workflows, not for AI agents specifically. No model routing. No gate pipeline. No knowledge store. No agent identity.
- No cost optimization — Temporal doesn't know or care what an LLM call costs
- No cross-agent knowledge sharing — each workflow is isolated
- No on-chain coordination layer
- Homepage is explanation-heavy with diagrams — the aha moment requires running code locally (a demo weakness Nunchi should avoid)

**What to steal**: The kill-and-resume pattern. The Timeline View as a visualization concept. The category-defining confidence of "we built this category."

**What to say in the pitch**: "Temporal proved that durable execution is a billion-dollar category. Agent workloads are workflows. The same durability property is required, plus identity, routing, knowledge, and cost optimization. We're the Temporal for agents."

---

### LangChain / LangSmith — Framework + Observability

**Valuation**: $200M (raised on narrative with near-zero ARR)
**What they are**: LangChain is the most popular agent framework. LangSmith is their observability and evaluation layer.

**LangSmith's demo experience**:
- Tagline: "Know what your agents are really doing"
- Three core pillars: Tracing, Monitoring, Insights
- **Tracing**: Waterfall/timeline visualization. Each trace is a tree: root run → child runs for tool invocations, LLM calls. Shows inputs, outputs, latency, cost per step.
- **Monitoring**: Real-time dashboards with cost per run, latency percentiles, quality scores via LLM-as-judge, alert routing
- **Insights**: Automatic trace clustering that discovers usage patterns and error templates from unsupervised analysis
- **Playground**: Version-controlled prompt experimentation with images, PDFs, audio. Prompt comparison across versions. Auto-sync to GitHub.

**Visual design**:
- Framework-agnostic positioning (works with OpenAI SDK, Anthropic, Vercel AI, LlamaIndex, custom code)
- Hover-triggered radial gradient masks on hero imagery
- Scroll-triggered letter animations
- Navbar switches light/dark based on section backgrounds
- Modern SaaS polish, not developer-spartan

**Strengths**:
- Massive developer adoption — LangChain is the default starting point
- LangSmith's trace view is the reference implementation for agent observability
- Automatic trace clustering is genuinely useful for production debugging
- "Find failures fast" is a clear, compelling value proposition

**Gaps**:
- Framework-level, not infrastructure-level. LangChain helps you write agent code. It doesn't make all agent code better.
- No agent identity. No cross-agent knowledge. No durability (crash = restart from zero). No cost optimization beyond what the user manually configures.
- No on-chain coordination. No ZK proofs. No reputation.
- $200M valuation on narrative alone demonstrates the market appetite but also the fragility — they need to justify it with revenue

**What to steal**: LangSmith's trace waterfall visualization. The automatic trace clustering concept. The drill-down path: aggregate → trace → span → raw prompt/response.

**What to say in the pitch**: "LangChain is the most popular framework. LangSmith is the best observability layer. Neither is coordination infrastructure. We integrate with both — LangSmith traces can feed Nunchi's gate pipeline. We're not replacing them; we're the layer above them."

---

### CrewAI — Multi-Agent Orchestration

**Valuation**: Private, significant enterprise traction
**What they are**: Multi-agent orchestration platform. Agents work as "crews" with defined roles.

**Their demo experience**:
- Homepage: "The Leading Multi-Agent Platform." Dark, modern (Lenis smooth scroll). Dual CTAs: "Build a crew" (devs) + "Request a demo" (enterprise).
- Three build modes via interactive carousel: visual editor with AI copilot (no-code), full API control (code-first), hybrid
- Tab-based product explorer: Orchestrate, Build, Observe, Manage

**Enterprise dashboard (CrewAI Enterprise + AMP)**:
- Real-time dashboards: throughput, latency, error rates, estimated cost per agent/per task
- Streaming execution logs and deployment history
- Role-based access control across teams
- AG-UI protocol (May 2025): event-based streaming with state sync, tool visualization, human-in-the-loop approval

**Social proof**: 16+ enterprise logos. Concrete metrics: 450M+ workflows/month, 60% of Fortune 500. Case studies with quantified outcomes: "75% faster lead contact at DocuSign," "90% development time reduction at General Assembly."

**Strengths**:
- The "crew" metaphor is intuitive — agents as team roles working together
- Enterprise traction with real numbers (450M workflows/month)
- AG-UI protocol provides real-time visibility into agent behavior
- Visual editor + API code = serves both technical and non-technical users

**Gaps**:
- No cost optimization (CascadeRouter equivalent). No model routing.
- No durable execution (crash = restart). No session persistence.
- No cross-organization knowledge sharing. No on-chain identity.
- No gate pipeline for validation. No learning from past runs.
- Orchestration without coordination — they help you run multiple agents, but don't make the agents smarter or cheaper over time

**What to steal**: The crew metaphor for multi-agent visualization. The quantified social proof format ("450M workflows/month"). The tab-based product explorer for the landing page.

**What to say in the pitch**: "CrewAI orchestrates agents. Nunchi coordinates them. Orchestration tells agents what to do. Coordination makes them cheaper, safer, and smarter. CrewAI agents running through Nunchi's coordination plane cost 30x less and share knowledge across runs."

---

### Cursor — AI-Native IDE

**Valuation**: ~$10B+ (2026)
**What they are**: AI-native code editor. Fork of VS Code with deep AI integration.

**Their demo experience**:
- **Cursor 2.0** (early 2025): The magic was codebase-wide context. Composer feature: describe changes in natural language, edits multiple files simultaneously. Developer goes from code writer to code reviewer.
- **Cursor 3.0** (April 2, 2026): Major architectural shift.
  1. **Agents Window**: Standalone workspace showing all agents — local and cloud — in a single sidebar. Includes agents from Slack, GitHub, Linear, mobile, web.
  2. **Cloud Handoff**: Start a task locally, push to cloud mid-execution, pull back to local for iteration. Cloud agents produce screenshots and demos of their work.
  3. **Simplified Diffs View**: Cleaner interface for reviewing agent changes.

**The aha moment**:
- Cursor 2.0: Type a description of a multi-file change and watch the codebase update correctly
- Cursor 3.0: Close your laptop, come back to find a completed PR from a cloud agent with screenshots
- Performance claim: 30-40% faster coding, cloud agents 4x faster problem solving

**Strengths**:
- The product experience is genuinely magical — it's what developers wish coding was like
- Cursor 3.0's Cloud Handoff is the specific pattern no other tool had
- ~$10B+ valuation validates the AI developer tools category
- Casado is a board member — pitching against Cursor is a strategic error

**Gaps**:
- Single-developer tool. No multi-agent coordination. No cross-org knowledge.
- No cost optimization (Cursor uses whatever model it wants at whatever price)
- No agent identity or reputation. No gate pipeline. No audit trail.
- End-user product, not infrastructure

**What to say in the pitch**: "Cursor is the best AI IDE. It's a Casado portfolio company. Nunchi makes Cursor agents cheaper and more reliable. Cursor agents running through Nunchi's CascadeRouter cost 3-5x less because routine substeps get routed to cheaper models automatically. We're complementary, not competitive."

---

### Devin (Cognition) — Autonomous Software Engineer

**Valuation**: ~$25B (April 2026, in talks to raise hundreds of millions)
**What they are**: Fully autonomous coding agent.

**Their original demo** (March 2024):
- Video showing a sandboxed compute environment with three panels: terminal, code editor, browser — all controlled by the AI
- Four specific tasks demonstrated:
  1. Read a blog post and implement what it described (learned ControlNet on Modal, produced images with concealed messages)
  2. Build a full Game of Life website with incremental features, deploy to Netlify
  3. Clone the SymPy library, analyze the entire codebase, find a logarithm division error, fix it, run tests
  4. Complete a real Upwork freelance task (computer vision/Python), including diagnosing a PyTorch version error

**Why it was viral**: SWE-Bench score 13.86% vs 1.96% previous SOTA (7x improvement). The SymPy example was the magic moment — navigating a massive, unfamiliar codebase, finding a subtle mathematical error, verifying the fix.

**Current state (2026)**: 67% of Devin's PRs merged (up from 34%). Running in Goldman Sachs, Santander, Nubank. Massive valuation growth.

**Strengths**:
- The demo was genuinely impressive and went viral
- Real enterprise adoption at major financial institutions
- The three-panel sandbox (terminal, editor, browser) is an intuitive visualization
- $25B valuation validates autonomous agents as a category

**Gaps**:
- Application, not infrastructure. Devin uses infrastructure; Nunchi is infrastructure.
- No multi-agent coordination. Devin is one agent doing one task.
- No cost optimization or knowledge sharing across runs.
- No open-source component — fully proprietary.

**What to say in the pitch**: "Devin at $25B validates that autonomous agents are a massive category. Devin is one agent doing one task. When enterprises run 10, 100, 1000 Devins, they need coordination infrastructure — identity, routing, knowledge, audit trails. That's Nunchi."

---

### Nava — Trust Intercept Layer

**Funding**: $8.3M (April 14, 2026)
**What they are**: Trust layer for AI agent actions. Intercepts agent actions and validates them against policies before execution.

**Strengths**:
- Fresh funding validates the "agent trust" category
- Clean positioning: "the firewall for AI agents"
- Simple, clear value proposition

**Gaps**:
- Trust layer only. No runtime, no knowledge, no chain, no cost optimization, no durability.
- Partial solution — validates actions but doesn't improve them.
- Nunchi's gate pipeline subsumes this functionality as one of 11 gates across 7 rungs.

**What to say in the pitch**: "Nava's $8.3M raise validates that agent trust is a real buyer concern. Nunchi's gate pipeline does what Nava does — and more. But we don't lead with trust. We lead with cost reduction and build trust into the architecture. Trust is a byproduct of coordination, not a standalone product."

---

### Other Relevant Comparisons

**AutoGen (Microsoft)**: Multi-agent conversation framework. Strong research backing. But: research-oriented, not production infrastructure. No cost optimization, no durability, no identity.

**Mastra**: Newer agent framework gaining traction. Focuses on developer experience. Framework-level, same gaps as LangChain.

**Vercel AI SDK**: Integration layer for LLM calls in web applications. Not agent infrastructure — it's a convenience library for making API calls.

**Braintrust**: AI evaluation platform ($800M valuation, Casado/a16z). Evaluates agent outputs. Could feed into Nunchi's gate pipeline. Complementary, not competitive. Pre-build one integration before the Casado meeting.

---

## 3. The Competitive Landscape Visualization

### Power Grid Format (For the Pitch Deck)

Per research, a Power Grid format (named competitors as rows, capabilities as columns, filled/half/empty indicators) beats a 2x2 quadrant for competition slides. The 2x2 always puts the startup in the upper right — investors see through it.

```
                  Identity  Cost     Knowledge  Durability  Gates  Chain
                           Routing  Sharing
Nunchi            ●         ●        ●          ●           ●      ●
Temporal          ○         ○        ○          ●           ○      ○
LangChain/Smith   ○         ○        ○          ○           ◐      ○
CrewAI            ○         ○        ○          ○           ○      ○
Nava              ◐         ○        ○          ○           ◐      ○
Devin             ○         ○        ○          ○           ○      ○

● = full capability  ◐ = partial  ○ = absent
```

**Critical honesty**: Acknowledge competitor strengths. Temporal's durability is world-class — they invented the category. LangSmith's observability is the reference implementation. Devin's autonomous coding is impressive. CrewAI has real enterprise traction. The honest framing: "These are all excellent products. None of them is a coordination plane. The coordination plane sits above all of them and makes all of them better."

---

## 4. Two Category Traps to Exit

Before the demo, it's essential to understand which categories Nunchi explicitly exits. Staying in either constrains valuation multiples and investor audience.

### Trap 1: The "Trust Layer" Trap

Companies: Nava ($8.3M), Capsule, t54.

The trust layer category is real but narrow. These companies intercept agent actions and validate them against policies. The problem: trust alone is not a platform. It's a feature that every coordination system includes. Nunchi's gate pipeline (11 gates, 7 rungs, adaptive thresholds, gate failure replan) is a superset of what any trust layer does. But leading with "we're a better trust layer" caps the valuation at trust-layer multiples (~$50-200M) instead of coordination-plane multiples ($1B+).

**Exit strategy**: Never say "trust layer" in the demo. Say "coordination plane" with trust as a built-in property.

### Trap 2: The "Agent Framework" Trap

Companies: LangChain ($200M), CrewAI, Mastra, AutoGen.

The framework category is crowded and commoditizing. Frameworks help you write agent code. They are developer convenience layers. The problem: frameworks are replaceable. Developers switch frameworks easily. The switching cost is low, so the defensibility is low.

**Exit strategy**: Never compare Nunchi to frameworks feature-by-feature. The comparison is structural: "Frameworks help you write agent code. We make all agent code — regardless of framework — cheaper, safer, and smarter. We integrate with LangChain, CrewAI, and any other framework. We're the layer above them."

---

## 5. What Each Competitor's Demo Looks Like vs. Nunchi's

| Product | Demo Format | Time to Aha | The Aha | What's Missing |
|---------|------------|-------------|---------|---------------|
| **Temporal** | CLI → web UI | ~60s (must run code) | Kill process, it resumes | No cost awareness, no AI agent specifics |
| **LangSmith** | Dashboard (screenshot-heavy) | ~30s (trace view) | See exactly where agent failed | No knowledge sharing, no durability |
| **CrewAI** | Video + dashboard | ~45s (crew working) | Agents collaborating as team | No cost optimization, no learning |
| **Cursor** | Live IDE | ~15s (edit appears) | Multi-file edit from description | Single-user, not infrastructure |
| **Devin** | Video (3-panel) | ~120s (SymPy fix) | Navigate massive codebase, fix bug | Single agent, not coordination |
| **Nunchi** | CLI → shareable URL | ~10s (from cache) | Four primitives in 3 minutes | The chain is simulated (mirage-rs) |

### Nunchi's Unfair Advantage in the Demo

1. **Speed**: Pre-warmed cache means any demo prompt completes in under 10 seconds. Most competitor demos take 30-120 seconds for the interesting part to happen.

2. **Four primitives in one output**: Identity, cost prediction, knowledge sharing, and durability are all visible in a single terminal output. Competitors show one primitive per demo.

3. **The shareable URL**: No other product produces a URL from a CLI command that shows a full execution timeline with cost breakdown and ZK proof. This is the artifact that leaves the room.

4. **The cost delta**: Seeing $0.031 vs $4.18 happen in real time is viscerally more compelling than a benchmark number on a slide. LangSmith shows you what happened. Nunchi shows you what it cost and why it was cheap.

5. **"Hand them the laptop"**: The pre-warmed cache enables the Collison pattern. No other infrastructure product in this space has a "type your own prompt and see it work in 10 seconds" moment.

6. **Empirical proof in-meeting**: No other agent infrastructure company runs a live side-by-side benchmark during the pitch. Bloomberg Two-Tape widget (400x300px corner overlay) shows Roko vs LangGraph on 5-task HAL subset in real-time. Statistical significance declared at p<0.01. This matches Aubakirova's empirical standards — her State-of-AI paper analyzed 100 trillion tokens; she expects data, not slideware.

### Benchmark Landscape (For the Side-by-Side Demo)

The live demo references specific benchmarks. Know the provenance of each number:

- **HAL** (Princeton, ICLR 2026): 9 benchmarks, Weave cost integration. PRIMARY benchmark for the live demo. $40K / 21,730 rollouts headline. HAL splits by scaffold pattern (ReAct, Tool-Calling, Few-Shot), NOT by framework name.
- **HumanEval/AAM** (arXiv:2407.01502): Source of the 30x intuition. LATS=$9.30 vs warming=$1.54 from EPiC paper.
- **Critical caveat**: No public HAL numbers tagged "LangGraph." To get apples-to-apples, wrap LG/AG as agent_fn and run hal-harness. The demo must run this live — not reference someone else's table.

---

## 6. Competitive Responses (Pre-Built for Q&A)

### "How is this different from just using prompt caching?"

Caching alone is 5x. That's one layer. Nunchi stacks four mechanisms: caching (5x), routing (3x), gate-based early stopping (2x), and batch scheduling (variable). Stacked, they multiply to 10-30x. But the qualitative differentiator is the knowledge substrate: cross-organization learning. Agent A at Company X publishes a solution pattern. Agent B at Company Y starts ahead. No other system does this.

### "Why won't Anthropic / OpenAI just build this?"

Anthropic builds models. Nunchi orchestrates across models, providers, and organizations. The CascadeRouter routes across Anthropic, open-weight, and third-party models simultaneously. Anthropic has no incentive to route away from its own models. Nunchi does — the chain economics reward accurate routing, not loyalty to any single provider. The more models exist, the more valuable the router becomes.

### "How is this different from Temporal?"

Temporal is durable execution for general workflows. Nunchi is coordination infrastructure for AI agents specifically. Temporal doesn't know what an LLM costs, doesn't route between models, doesn't share knowledge across workflows, doesn't validate outputs through a gate pipeline. Agent workloads need all of these. Temporal's $5B valuation proves the category; Nunchi is the agent-specific version.

### "What if CrewAI adds cost optimization?"

CrewAI would need to build: CascadeRouter (model routing with Thompson sampling / LinUCB bandit), 3-tier inference cache (L3 deterministic, L2 semantic with HDC similarity, L1 prefix), gate pipeline (11 gates, 7 rungs, adaptive thresholds), knowledge store (Ebbinghaus decay, tier progression, HDC fingerprinting), session persistence (checkpoint/resume), and a chain layer for cross-organization coordination. Each is 6-12 months of engineering. Together they require deep architectural coupling that can't be bolted on. The coordination plane is not a feature — it's the architecture.

### "Isn't this just LangSmith with a chain?"

LangSmith is observability — it tells you what happened after the fact. Nunchi is a runtime — it optimizes what happens during execution. LangSmith's traces can feed Nunchi's gate pipeline (they're complementary), but LangSmith doesn't route between models, doesn't share knowledge, doesn't persist sessions, and doesn't reduce cost. The chain is the least of the differences; the runtime is the most.

---

## 7. Visual Design Comparison

### How Competitor UIs Compare

| Product | Primary Colors | Typography | Data Density | Overall Feel |
|---------|---------------|------------|--------------|-------------|
| **Temporal** | Dark + green/red status | System fonts | Medium | Industrial, functional |
| **LangSmith** | Light + dark modes | Modern SaaS | High (trace depth) | Polished, information-dense |
| **CrewAI** | Dark, gradient accents | Modern, bold | Medium | Marketing-forward |
| **Cursor** | Dark (VS Code base) | System + monospace | Very high (IDE) | Familiar, productivity-focused |
| **Devin** | Dark three-panel | Monospace-heavy | High | Terminal-first, raw |
| **Linear** (reference) | Dark, minimal chrome | Geist/custom, precise | Balanced | Best-in-class craft |
| **Vercel** (reference) | Pure black + white | Geist Sans/Mono | Clean | Mathematical precision |

### Where Nunchi Should Sit

Nunchi's visual design should sit between **Linear** (craft, restraint) and **Vercel** (mathematical precision) — never touching CrewAI's marketing-forward aesthetic or Temporal's industrial functional look.

The key visual differentiation: **the knowledge graph**. No competitor has a force-directed visualization of cross-agent knowledge with citation edges, demurrage decay, and live updates. This is the visual that says "this is a network, not a tool."

---

---

## 8. How Nunchi's Architecture Maps to Competitive Claims

This section maps each competitive differentiation claim to real code in the Roko codebase. Every claim must be demonstrable, not theoretical. For the complete codebase reference, see CODEBASE-CONTEXT.md.

### "Nunchi has agent identity" — vs LangChain, CrewAI, Cursor

**Current state**: ERC-8004 agent identity is defined in the `roko-chain` crate and can be registered on-chain via `AlloyChainWallet`. The `roko-serve` server has `/api/chain/agents` to list on-chain identities. However, the CLI does not currently print identity information in the `roko run` output — this is a build item (T0.1 in DEMO-BUILD.md).

**What exists**: The chain layer, the identity standard, the API endpoint. **What's missing**: CLI formatting that surfaces it.

### "Nunchi has cost prediction and optimization" — vs Temporal, CrewAI

**Current state**: The CascadeRouter in `roko-learn` routes tasks to the cheapest capable model using Thompson sampling / LinUCB bandits. Model weights are persisted to `.roko/learn/cascade-router.json`. The router is queried during agent dispatch in `run.rs` / `orchestrate.rs`. The prediction (estimated cost, selected model) is computed internally but NOT surfaced in CLI output.

**What exists**: The routing algorithm, the persistence, the per-turn efficiency telemetry. **What's missing**: CLI formatting that surfaces the prediction vs actual delta.

### "Nunchi has shared knowledge across agents" — vs everyone

**Current state**: The NeuroStore in `roko-neuro` stores knowledge entries with confidence scores, timestamps, attribution, and HDC fingerprints. Entries are queried during dispatch enrichment in `orchestrate.rs` and injected as knowledge hints into the system prompt. The Ebbinghaus decay curve and tier progression are implemented.

**What exists**: The knowledge store, the query mechanism, the decay/reinforcement logic. **What's missing**: CLI formatting that surfaces "loaded N facts from M agents" and explicit fact deposition after agent completion.

### "Nunchi has durability (zero work lost)" — matches Temporal's core claim

**Current state**: The plan executor saves state snapshots to `.roko/state/executor.json` after each completed task. The `--resume` flag loads the snapshot and continues. For single `roko run` commands, the signal substrate provides implicit checkpointing (each persisted Signal is a checkpoint). The mechanisms work but the CLI output doesn't format the resume experience prettily.

**What exists**: Checkpoint/resume for plan execution, signal-based recovery for single runs. **What's missing**: Clack-style formatted output showing "resuming from checkpoint 3/7."

### "Nunchi has a gate pipeline" — vs Nava's trust layer

**Current state**: 11 gates across 7 rungs in `roko-gate`. Called per-task from `orchestrate.rs`. Adaptive thresholds in `.roko/learn/gate-thresholds.json`. Gate failure can trigger automatic replanning. The gate pipeline is fully wired and works today.

**What exists**: Everything. The gate pipeline is one of the most mature subsystems. **What might need**: Better formatting in CLI output (Clack-style ✔/✖ symbols).

### "Nunchi has a chain layer" — unique to Nunchi

**Current state**: The `roko-chain` crate uses the `alloy` Rust Ethereum library. `AlloyChainClient` for reads, `AlloyChainWallet` for writes. The `mirage-rs` app provides a local EVM simulator. `roko-serve` has `/api/chain/status`, `/api/chain/agents`, `/api/chain/bounties` endpoints. The `roko-chain-watcher` app observes on-chain events.

**What exists**: Chain client, wallet, local simulator, API endpoints. **What's missing**: CLI formatting for chain interactions, the shareable URL page with ZK proof display, the Chain View dashboard.

### "30x cheaper" — verifiable via HAL harness

**Current state**: The CascadeRouter stacks four cost reduction mechanisms: caching (5x), routing (3x), gate-based early stopping (2x), and batch scheduling (variable). The 30x claim comes from EPiC paper data (LATS=$9.30 vs warming=$1.54 on HumanEval/AAM, arXiv:2407.01502). Verifiable via HAL harness on tau-bench + AppWorld + GAIA tasks. Live reproducible, not a slide assertion.

**What exists**: The routing algorithm, the cache tiers, the gate pipeline. **What's needed for the demo**: Run hal-harness with Roko as agent_fn alongside LangGraph as agent_fn on the same 5-task subset. Show results in the Bloomberg Two-Tape widget overlay.

### "Zero work lost" — extends Temporal's core claim

**Current state**: Checkpoint/resume works for plan execution (`--resume`), signal substrate provides implicit checkpointing for single runs. This matches Temporal's signature move. **What Nunchi adds**: cost continuation — the meter doesn't restart from zero on resume. When an agent resumes from checkpoint 3/7, the cost attribution continues from the accumulated total, not from scratch. No competitor tracks cost across resume boundaries.

### "Knowledge compounds" — unique to Nunchi, no competitor equivalent

**Current state**: NeuroStore queries at dispatch time inject knowledge hints into the system prompt. Entries decay via Ebbinghaus curve and progress through confidence tiers. **The moat**: No competitor has inter-agent knowledge sharing. LangGraph runs are isolated. CrewAI crews don't learn from past crews. Every agent run in Nunchi makes every future agent run cheaper and more accurate. This is the compounding advantage that widens over time.

### "Validated paths, not assertions" — mirrors Aubakirova's Pentesting essay

**Current state**: The gate pipeline (11 gates, 7 rungs) produces structured evidence for every agent output. ZK-HDC produces cryptographic evidence of agent similarity — the receipt page is the competitive differentiator. This mirrors the language in Aubakirova's Pentesting essay: validated paths, not model assertions. The receipt page shows proof of work, proof of cost, proof of correctness — not a dashboard that says "trust us."

---

*Cross-references: CODEBASE-CONTEXT.md (complete technical reference), DEMO-STRATEGY.md (what and why), DEMO-VISUAL-SPEC.md (detailed design), DEMO-FLOW.md (beat-by-beat script), DEMO-BUILD.md (what to implement).*
