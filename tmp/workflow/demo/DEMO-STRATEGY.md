# Demo Strategy: Nunchi Series A

**Purpose**: Master strategy document for the Nunchi demo — what to show, why, for whom, and the narrative that ties it together. Written for someone with zero prior context about Nunchi, Roko, or the agent infrastructure space.

**Date**: April 2026

---

## 1. What Is Nunchi

Nunchi is two things that compose into one product.

**Part 1 — Roko (the runtime)**: An open-source Rust toolkit for building and coordinating AI agents. 18 crates, approximately 177,000 lines of code, Apache 2.0 licensed. Roko handles the mechanics of running AI agents: model routing (sending easy tasks to cheap models, hard tasks to expensive ones), gate pipelines (automated validation of agent outputs — compile, test, lint, semantic review), prompt composition (9-layer system prompts assembled from templates, knowledge hints, and context bidding), session persistence (crash recovery with zero work lost), and a knowledge store that lets agents learn from past work. The runtime exposes ~115 HTTP API routes and a terminal UI for monitoring. It is self-hosting: Roko develops itself by reading product requirement documents, generating implementation plans, executing them with AI agents, validating results through gates, and learning from the outcomes.

**Part 2 — The Nunchi blockchain**: A purpose-built sovereign EVM Layer 1 chain with ~50ms block times, native hyperdimensional computing (HDC) precompiles, ERC-8004 agent identities with 7-domain reputation, ZK-HDC proofs for verifiable agent behavior, and an on-chain knowledge substrate with demurrage (knowledge that isn't used decays and is pruned; knowledge that is useful gets reinforced). The chain enables cross-organization coordination: Agent A at Company X publishes a solution pattern; Agent B at Company Y, facing a similar problem, starts ahead without accessing Company X's code or data.

**The category name**: Agent Coordination Plane. The thesis is that coordination is a more durable moat than model capability. The analogy: SDN (Software-Defined Networking) separated the network control plane from the data plane, creating Nicira (acquired by VMware for $1.26B) and the modern cloud networking stack. Nunchi separates agent coordination from agent execution.

**The tagline**: "The model is the same. The system is the variable."

### Why This Matters for the Demo

The demo must prove that Nunchi is infrastructure, not a framework. Frameworks help you write agent code. Infrastructure makes all agent code better — cheaper, safer, more durable, smarter over time. The distinction is the difference between a $200M valuation (LangChain) and a $5B valuation (Temporal). The demo must feel like infrastructure.

---

## 2. The Demo Thesis

The demo must prove exactly four primitives in a compressed timeframe. If all four land, the demo has done its job. If any one is missing, the story has a hole.

### Primitive 1: Identity and Safety (Default-Off)

Every agent has a verifiable non-human identity. Before spending a single token, policy gates fire: PII scan, cost ceiling, compliance checks. Nothing runs without passing policy. This is the "agents are accountable" primitive.

**Why it matters**: 82:1 is the ratio of machine identities to human identities in enterprise infrastructure today (CyberArk 2025). The agent economy is already here. Identity infrastructure is not. When the EU AI Act Article 50 enforcement begins (August 2, 2026 — approximately 14 weeks from this document's date), every AI system interacting with humans must disclose its nature and maintain audit trails. Penalties: up to €15M or 3% of global turnover for transparency violations.

**What the investor sees**: A verified agent identity line in the terminal output. Gate checks passing before any work begins. The absence of this in every other agent tool.

### Primitive 2: Cost Prediction and Optimization

The system predicts what a task will cost before execution, routes to the cheapest model that can handle it, and self-corrects after execution. The prediction-actual delta is visible. Every run improves the predictor.

**Why it matters**: The Princeton HAL benchmark (ICLR 2026) shows 50x cost variation between agents achieving similar accuracy. The difference is not the model — it's the system. Nunchi's stacked optimization delivers 10-30x practical cost reduction:

| Layer | Reduction | Source |
|-------|-----------|--------|
| Prompt caching (L1/L2/L3) | 5x | Anthropic: 90% cache discount; ProjectDiscovery: 7%→84% cache hit rate (9.8B tokens) |
| Model routing (CascadeRouter) | 3x | RouteLLM paper: 85% cost reduction retaining 95% quality (arXiv:2406.18665) |
| Gate-based early stopping | 2x | Gate pipeline terminates failed paths before full execution cost |
| **Stacked** | **10-20x practical, 30x theoretical** | Princeton HAL: $44.86 naive → $1.42 optimized |

**Critical disclosure**: The HAL benchmark costs do NOT include caching benefits. HAL states this explicitly. Caching alone accounts for ~4-5x of the reduction. The remaining ~6-7x comes from routing and gating. Always disclose the intermediate step. Honesty builds more trust than inflated claims.

**30x cost number caveat**: The exact $44.86 -> $1.42 figures were NOT directly verifiable in any paper fetched during research. They are consistent in spirit with AAM (LATS >50x warming) and EPiC ($9.30 vs $1.55). **Recommendation:** either reproduce locally on a 5-task subset and print actual numbers, or cite as "derived from HAL methodology" rather than verbatim paper quote. Do not invent precision. Also note: HAL splits by scaffold pattern (ReAct, Tool-Calling, Few-Shot), NOT by framework name — no public HAL numbers tagged "LangGraph" or "AutoGen."

**What the investor sees**: A predict line showing expected cost, then an actual line showing real cost with the delta. The second agent running cheaper than the first because it loaded knowledge from the first.

### Primitive 3: Shared Knowledge Across Agents

Agents that work in the same domain share knowledge automatically. Agent A publishes findings. Agent B — a completely different agent — loads those findings and starts ahead. The knowledge is scored, timestamped, and attribution-tagged. Stale knowledge decays via demurrage. Useful knowledge gets reinforced. The thousandth agent joins smarter than the first.

**Why it matters**: This is the network effect. Every other agent framework is single-session: knowledge dies when the process ends. Nunchi's knowledge substrate compounds across agents, across sessions, across organizations (via the chain). A competitor who copies the routing logic starts with an empty knowledge store.

**What the investor sees**: The knowledge line in the second agent's output showing "loaded 9 facts from 4 agents, 0.93 avg confidence." The cost dropping because knowledge was reused instead of re-derived.

### Primitive 4: Durability (Zero Work Lost)

Kill the agent mid-run. Resume from the last checkpoint. Zero tokens wasted. Zero work lost. State is persisted after every completed step.

**Why it matters**: This is Temporal's signature move applied to agent workloads. Temporal built a $5B company (February 2026, led by Sarah Wang and Raghu Raghuram at a16z) on the premise that workflows should survive infrastructure failure. Agent workloads are workflows. The same property is required.

**What the investor sees**: A visible Ctrl+C killing a running agent. A two-second pause. A resume command that picks up exactly where it stopped. The cost meter continuing from where it paused, not restarting from zero.

---

## 3. Market Context

### The Problem (One Sentence)

41-86% of multi-agent deployments fail, and 79% of those failures come from coordination, not model capability (MAST taxonomy, Berkeley AI Safety, arXiv:2503.13657).

### The Market Size

- Global AI agent market: projected $190B by 2030 (various analyst estimates converge around this range)
- Agent infrastructure specifically: the layer between "model providers" (Anthropic, OpenAI, Google) and "agent applications" (customer support bots, coding agents, research tools)
- Interest rate derivatives (the ISFR expansion domain): $668 trillion notional outstanding (BIS Triennial Survey, H1 2025) — this is the future expansion, not the beachhead

### Why Now (Three Converging Forces)

1. **Protocol convergence**: MCP (Anthropic/Linux Foundation, 97M monthly downloads), A2A (Google/Linux Foundation, 150+ organizations), ERC-8004 (agent identities), x402 (Coinbase micropayments) — the standards for agent interoperability are crystallizing simultaneously. The coordination plane sits above all of them.

2. **Regulatory trigger**: EU AI Act Article 50 enforcement begins August 2, 2026. Only 35.7% of EU managers feel prepared (Deloitte AI Regulation Survey, 2025). This creates an acute enterprise buying trigger with a known deadline. Compliance-as-distribution: Vanta reached ~$100M+ ARR from SOC 2 automation. OneTrust exceeded $5B from GDPR compliance tooling. The pattern repeats.

3. **Cost reduction is empirically proven**: The HAL benchmark, RouteLLM, and production caching data collectively demonstrate that 10-30x cost reduction is achievable and reproducible. This is not theoretical — it is measured.

### Comparable Valuations

| Company | Valuation | Multiple | What They Do |
|---------|-----------|----------|--------------|
| Temporal | $5B (Feb 2026) | 40-60x ARR | Durable execution platform |
| Cursor | ~$10B+ (2026) | 25-29x ARR | AI-native IDE |
| Braintrust | $800M | 30-50x ARR | AI evaluation layer |
| LangChain | $200M | 80-100x near-zero ARR | Agent framework (narrative-driven) |
| Devin/Cognition | ~$25B (Apr 2026) | Pre-revenue | Autonomous coding agent |

Nunchi as a coordination plane — not a tool, not a framework — should command platform multiples (8.2x revenue per Equal Ventures data, vs 3.9x for SaaS tools per BVP Emerging Cloud Index).

### Benchmark Methodology (Reproducible Numbers)

The demo must cite numbers that can be reproduced, not just asserted. The benchmark stack:

- **HAL** (arXiv:2510.11977) is the PRIMARY benchmark — 9 benchmarks, ICLR 2026 credibility, Weave cost integration. HAL splits by scaffold pattern (ReAct, Tool-Calling, Few-Shot), not by framework name. This is the strongest third-party validation of cost variation across agent architectures. Use it for "50x cost variation" and the structural argument for routing.

- **GAIA Level-1** as fallback — 5 questions in <3 min. Simple enough to run live if HAL is too heavy for the demo environment. Good for "agents can do useful work cheaply" without requiring the full HAL infrastructure.

- **tau-bench Airline/Retail** — customer-service archetype with deterministic user simulation. Useful for showing the gate pipeline in action (agent outputs are scored against known-correct responses).

- **The 5-task HAL subset for live demo**: tau-bench task_0 + task_4, AppWorld test_normal, GAIA Level-1 x 2. This subset is small enough to run in real-time during a meeting, diverse enough to show routing decisions (different tasks route to different models), and produces concrete cost numbers that can be compared against naive baselines.

**Critical**: Any number cited in the demo should either come from a published paper with a specific table/figure reference, or be reproduced locally on the 5-task subset with printed actual numbers. The 30x headline is defensible as a structural claim; the specific dollar figures require local reproduction or "derived from HAL methodology" attribution.

---

## 4. The Narrative Arc

### General VC Version (3 minutes)

**Beat 1 (0:00-0:30) — Identity and Gates**

Run one command. The terminal prints the agent identity and gate checks immediately.

```
$ nunchi run agents/researcher.py --task "Summarize Q3 fintech earnings"
  > agent      researcher@v2  .  nhi://acme/researcher.v2  (verified)
  > predict    $0.043  .  12.4s  .  route: haiku -> gpt-4o-mini
  > gates      pii_scan OK   cost_ceiling<$0.10 OK   sox_compliance OK
```

Point to the output: "Every agent has a verified non-human identity. Before the agent spends a single token, three gates fire. Default-off. Nothing runs without passing policy."

**Beat 2 (0:30-1:15) — Predict-Publish-Correct**

The agent runs. When it completes:

```
  > actual     $0.031  (-28% vs predicted)  .  routed to haiku
  > deposited  2 new facts -> /finance/q3
```

"$0.043 predicted. $0.031 actual. 28% below prediction. The system predicted the cost before execution. After execution, the actual cost is recorded and the delta is computed. Every run improves the predictor."

**Beat 3 (1:15-2:15) — Shared Knowledge**

Run a second, different agent:

```
$ nunchi run agents/analyst.py --task "Draft Q3 fintech earnings brief for CRO"
  > agent      analyst@v1  .  nhi://acme/analyst.v1  (verified)
  > knowledge  loaded 9 facts from /finance/q3 (4 agents, 0.93 avg conf)
  > actual     $0.022  (-42% vs predicted)  .  routed to haiku
```

"9 facts loaded. The first agent deposited 2 new facts. This agent picked them up automatically. First agent: $0.031. Second agent: $0.022. That's a 64% cost reduction versus running both naive."

**Beat 4 (2:15-2:35) — Kill and Resume**

Start a third command. Ctrl+C mid-run. Wait two seconds. Resume:

```
$ nunchi run agents/researcher.py --task "Compare Q3 vs Q2 fintech margins" --resume
  > resuming from checkpoint 3/7  .  $0.012 spent  .  4 steps remaining
  > actual     $0.029  .  routed to haiku
```

"We killed it mid-run. It resumed from the last checkpoint. Zero work lost. Zero tokens wasted."

**Close (2:35-3:00)**

"Identity, prediction, shared memory, durability. Four primitives. Every multi-agent company will need them within 18 months."

### a16z-Specific Version (5 minutes, tailored for Martin Casado + Malika Aubakirova)

**Casado dossier**: VMware co-founder, built Nicira (SDN control plane, acquired for $1.26B). His entire thesis is control planes. His April 2025 published skepticism: "we can't yet close the control loop on agents." Board companies: Kong, Truffle, Pindrop (security/infrastructure), Cursor, Braintrust (developer tools), Convex, Netlify, Fivetran, Material Security, Ideogram, World Labs, Fly.io. **Never criticize any of these companies in the meeting.**

Key Casado canon to mirror:
- SDN control/data-plane decoupling (his identity — use this vocabulary throughout)
- "Customers don't buy platforms; customers buy products" (Open Networking Summit 2017) — Roko is the product, Nunchi becomes the platform
- "No more Red Hats" (Peter Levine phrase, Casado-endorsed) — open source as top-of-funnel, not the product
- "Non-consensus investing is overrated" — do NOT lead with "we're contrarian." Give a consensus pitch for a non-consensus product
- "Vertical clouds" — call the Nunchi chain a "vertical cloud for agent identity and settlement"
- Bitter Economics / "compute overrides cleverness" — Roko's 177K lines of Rust is the substrate that makes compute efficient
- "Market annealing" / "if the founder can't sell it, no one can"

**Critical landmine**: Casado has no public canon on sovereign EVM L1s. He operates separately from Dixon's a16z crypto. Frame the chain as identity and settlement infrastructure, never as web3/DeFi.

**Aubakirova dossier**: Deal partner on Casado's infra team, co-authors with Joel de la Garza, Zane Lackey, Matt Bornstein, Yoko Li on security and AI infra. Low public profile but tightly canonical 9-essay corpus. Author hub: https://a16z.com/author/malika-aubakirova/ **She is read more than heard — minimal podcast, low-volume X (@MaikaThoughts), but her essays are the strongest alignment surface for Nunchi.**

**Name correction**: Her byline is **Malika Aubakirova**, not Maika. Handle remains @MaikaThoughts. Use "Malika" in any written follow-up; "Maika" is fine as conversational preferred form.

**@MaikaThoughts activity**: She is genuinely low-volume on X (~1.1-1.3K followers, joined Mar 2024). Her canonical thinking lives on a16z bylines, not in tweets. Topics she has NOT publicly tweeted about (do not assume preloaded knowledge): ERC-8004, ZK proofs, EU AI Act, LangGraph/AutoGen by name, "non-human identity" (she uses "machine identity" / "agent identity"). **Implication**: EU AI Act framing must be introduced as a tailwind, not echoed.

The full 9-essay Aubakirova corpus:

1. **"Investing in Adaptive Security"** (Apr 2, 2025) — Early co-authored essay with Lackey and de la Garza. Establishes the security thesis that runs through her subsequent work.

2. **"Breaking the Cybersecurity Kill Chain with AI"** (Sep 9, 2025) — The shield against "isn't this Keycard?" Key quote: *"the next generation of tools will not be defined by acronyms but by their ability to eliminate attack paths altogether."* Use her kill-chain framework to pre-empt overlap: "Keycard = intra-org issuance/runtime enforcement; Nunchi = cross-org reputation/settlement. Different stages of the same lifecycle."

3. **"Investing in Keycard"** (Oct 21, 2025) — The positioning seam. Key quote: *"static secrets and API keys... built for humans clicking buttons, not for autonomous agents spawning by the thousands."* The trilemma: too permissive / lock-down stifles innovation / custom infra burns engineers. **Steal line:** "Keycard solved the Auth0 moment inside the org — dynamic, identity-bound, task-scoped tokens. ERC-8004 + Nunchi takes the same primitive across the trust boundary: same dynamic-intent token, but issued by no one and verifiable by everyone — sovereignized via on-chain settlement."

4. **"Next-Gen Pentesting"** (2025) — Language template for ZK-HDC. Key quote: *"validated path showing how an attacker would have breached your system."* **Steal:** Don't say "we have proofs." Say "we produce validated paths — cryptographic evidence of agent similarity, not assertions."

5. **"State of AI: 100 Trillion Token Study with OpenRouter"** (Dec 4, 2025) — Match her empirical voice. She is lead author (confirmed via Anjney Midha tag). Key quote: *"The competitive frontier is no longer only about accuracy or benchmarks. It is about orchestration, control, and a model's ability to operate as a reliable agent."* Use the exact phrase "agentic inference."

6. **"The Cinderella Glass Slipper Effect"** (Dec 8, 2025) — The wedge framing. Key quote: *"In AI, achieving product-market fit may literally mean solving one high-value workload better than anyone else."* **Steal:** Don't pitch "agent infrastructure." Pitch one workload — verifiable similarity for cross-org agents under cooperative clearing — completely.

7. **"Big Ideas 2026 Part 1"** (Dec 9, 2025) — Defined "agent-speed traffic": *"agent-speed workloads that're recursive, bursty, and massive... a single agentic 'goal' to trigger a recursive fan-out of 5,000 sub-tasks... To a legacy database or rate-limiter, it looks like a DDoS attack... The next generation must treat 'thundering herd' patterns as the default state."* This describes Nunchi's Pulse fabric verbatim.

8. **"Et Tu, Agent? Did You Install the Backdoor?"** (Apr 2, 2026) — Closes with *"Save the AI agents. Secure the supply chain."* This is Nunchi's gate pipeline + ERC-8004 attestation.

9. **"Why We Need Continual Learning"** (Apr 22, 2026) — Contains: *"'Janky but native' interfaces often win because they couple directly to the underlying system rather than fighting it."* This is the strongest possible justification for a 177K-line Rust runtime. Written 5 days before this research.

**The Aubakirova axis is the most actionable intelligence in this dossier.** Three "she'll recognize her own ideas" moments to engineer:
1. Open with a verbatim Big-Ideas-2026 pull-quote on screen while Pulse fans out sub-tasks
2. Frame ERC-8004 + HDC fingerprints as "the Keycard pattern, sovereignized" — collaborator, not competitor with their portfolio bet
3. Close on the "Save the AI agents" line — *"You said the supply chain has to move at machine speed. Roko + Pulse + ERC-8004 are how."*

One-line pitch calibrated to Aubakirova if she leads the room: *"Nunchi is the agent-native control plane you wrote about in Big Ideas 2026 — Pulse handles the thundering herd, ERC-8004 attestations extend the Keycard pattern into a sovereign substrate, and Roko is the janky-but-native Rust runtime that couples to the agent execution model instead of fighting it."*

#### The Keycard Sovereignization Argument

Keycard's issuance model: OAuth 2.1 Client ID Metadata Documents with three scoping vectors (who/what/for whom), issuing ephemeral runtime-revocable tokens. This is the "Auth0 moment for machine identity" — dynamic, identity-bound, task-scoped.

The three gaps Malika identifies in the pre-Keycard landscape:
1. **Too permissive** — static API keys and secrets grant blanket access with no task-level scoping
2. **Lock-down stifles innovation** — over-restrictive policies prevent agents from doing useful work
3. **Custom infra burns engineers** — every org rebuilds the same identity plumbing from scratch

Root cause she names: "years of underinvestment in machine identity."

**The sovereignization argument**: Keycard requires a trusted issuer — it works inside one trust domain (Company X's IdP issues tokens for Company X's agents). Cross-org agent coordination has no shared IdP. Company X and Company Y cannot agree on who issues tokens for agents that cross the trust boundary. ERC-8004 + ZK-HDC is the natural generalization: preserve her axis (static secrets -> dynamic identity), then add the perpendicular axis (centralized issuer -> sovereign verification). Same dynamic-intent token, but issued by no one and verifiable by everyone. The chain IS the IdP.

#### Adjacent Partners (Likely in the Room)

Intelligence on adjacent partners who may be present or whose work will be referenced:

- **Joel de la Garza**: Security focus, co-author of "Et Tu Agent" and "Breaking the Kill Chain." Thesis: "2026 is the year of agents; identity is the bottleneck." He will be the most sympathetic ear for ERC-8004 and the gate pipeline. Lead with identity and audit trail.

- **Zane Lackey**: Signal Sciences founder, Keycard observer. Use the kill-chain frame to defuse overlap: "Keycard = intra-org issuance/runtime enforcement; Nunchi = cross-org reputation/settlement. Different stages of the same lifecycle." He understands layered security — speak in layers, not in competition.

- **Matt Bornstein**: GP, AI/data, co-author of "Continual Learning." Skeptic: "agents don't really work yet." Lead with concrete numbers versus architecture diagrams. He wants evidence, not vision. The HAL benchmark costs, the cache hit rates, the real cost deltas — these are his language.

- **Yoko Li**: Infra/devtools, co-author of Keycard investment memo. Phrase to mirror: *"redefinition of how software gets built with agents, context, and intent at the core."* She cares about developer experience and infrastructure primitives. The `nunchi init` -> first agent in 10 seconds story is hers.

**Productive tension to bridge in-room:** Joel ("2026 is the year of agents") vs Matt ("agents don't work yet"). Bridge: "Joel is right about demand. Matt is right that today's frameworks fail. Nunchi is the missing coordination + verifiability layer that makes agents actually work — cheaper, safer, durable, smarter over time."

#### a16z Lexicon

Vocabulary Will should speak in the room. These are phrases drawn from Aubakirova's essays, Casado's canon, and the broader a16z AI/infra thesis. Using their language signals alignment without signaling mimicry:

- "machine identity" (NOT "non-human identity")
- "the missing trust fabric"
- "from static identity to dynamic intent"
- "agent-native infrastructure"
- "coordination becomes a bottleneck"
- "agentic inference"
- "workload-model fit"
- "output, not users"
- "multi-agent architectures as a scaling strategy for context itself"
- "validated paths"

**Opening line** (delivered verbatim as you open the laptop):

> "Martin, you wrote that we can't yet close the control loop on agents. That's exactly why we built Nunchi as the control plane. Five minutes."

**Min 1 — Identity and Attestation**

```
$ nunchi agents list --env=prod
```

Show SPIFFE identities, attestation status, policy assignments. "Every agent has a verifiable non-human identity. Default-off. Nothing runs without attestation."

**Min 2 — Policy-Gated Audit**

```
$ nunchi audit deployment payments-svc --rev=abc123 --policy=prod-sec
```

Eight parallel audit steps fire. Step 3 finds a leaked AWS secret. The system flags it and does NOT continue past the violation. "The agent didn't close the loop — the coordination plane did."

This hits both Casado's infrastructure thesis (Kong, Truffle, Pindrop) and Aubakirova's security portfolio (Chronicle Detect, Keycard).

**Min 3 — Pre-Seeded Failure**

Step 5 panics (pre-seeded). Visibly Ctrl+C. Let the room sit with a dead terminal for two seconds.

**Min 4 — Resume and Remediation**

```
$ nunchi resume run_4823
```

Recovers from event 47 of 52. Leaked credential rotated. PR opened automatically. "Zero work lost."

**Min 5 — Replay and Audit Trail**

```
$ nunchi replay run_4823 --as-of="step 05"
```

Full JSON audit trail from step 5 forward. Every decision cryptographically timestamped and replayable. "This is what you hand the compliance officer when they ask what happened on March 15th."

**Close**: "Same primitives — identity, policy, replay — work for triage, migrations, anything. The agent didn't close the loop. We did."

---

## 5. The "Stripe Moment"

Every great developer product has a single interaction that makes the investor think "I cannot go back to the old way."

- **Stripe**: 7 lines of code to accept a payment
- **Vercel**: Push to git, get a preview URL in 30 seconds
- **Temporal**: Kill a workflow, it resumes from checkpoint

**Nunchi's Stripe moment**:

```bash
nunchi init
nunchi run "Fix the failing test in src/auth.rs" --share
# → https://nunchi.network/runs/abc123
# → Task cost: $0.14 (naive baseline: $4.18)
# → Cache hit: 65% | Routing: 82% to $0.14/MTok model
# → ZK proof anchored: block 1,204,387
```

The `--share` flag produces a URL. The URL opens in a browser and shows:

- Live timeline of agent execution steps with cost per step
- Real-time cost meter with a configurable cap
- ZK-HDC proof anchored on chain with a public verifier link
- Full replay from any step — click any point in the timeline and replay forward
- The agent's ERC-8004 identity with reputation attestation

This URL is the artifact that leaves the room. It is what the investor forwards internally. It is the Vercel preview URL, but for agent runs.

### The Collison Pattern

Patrick Collison closed Stripe's early rounds by handing investors a laptop and letting them process a real payment in under 60 seconds. Collison pitched Stripe to Peter Thiel as a PayPal replacement — to PayPal's founder — and Thiel funded immediately. The equivalent: hand Casado the laptop and let him run a 4-line snippet. The pre-warmed cache covers a range of prompts. The experience is always fast (under 10 seconds from cache).

The shareable URL is the artifact that goes in a Slack message. It is the thing the partner forwards to the Monday meeting. It must work reliably, load fast, and be visually impressive.

### Beyond the Collison Pattern: The Reverse Demo

The highest-risk, highest-payoff move: *"Martin, give me a task you'd actually do this afternoon. Roko will do it now."* Casado is on record (World of DaaS, April 2025) that he uses AI for productivity hours a day; he'll have a task ready. If Roko fails, the meeting ends. If it works, he becomes a co-author of the demo and tells every partner. This converts the "hand them the laptop" moment from consumption to creation.

### The Casado-as-Verifier Move

The most structurally novel demo mechanic: give Casado a temporary on-chain verifier role. He scans a QR with his phone wallet (clean Sign-In-With-Ethereum modal), receives a Gitcoin-Passport-style stamp granting `verifier-role` to his address with 24-hour TTL. Any attestation published during the meeting requires his co-signature. When the predict-publish-correct cycle resolves, his wallet pings, he co-signs, and the ERC-8004 attestation hits the L1 with his identity in the verifier set, permanently on-chain. He owns part of the demo's reputation graph. This makes the L1 visceral instead of abstract.

---

## 6. Key Numbers (All With Sources)

Every number used in the demo or deck must be cited and accurate.

| Number | What It Measures | Source | Notes |
|--------|-----------------|--------|-------|
| 30x | Cost reduction vs naive baseline | HAL + Anthropic caching + RouteLLM | Theoretical max; 10-20x practical. Always disclose caching carries most of the reduction |
| $44.86 → $1.42 | HAL benchmark task cost | Princeton HAL, ICLR 2026 | HAL excludes caching. $44.86 is no-cache baseline |
| 41-86% | Multi-agent deployment failure rate | MAST taxonomy, arXiv:2503.13657 | NeurIPS 2025 |
| 79% | Failures from coordination, not capability | MAST | Same paper |
| 82:1 | Machine-to-human identity ratio in enterprise | CyberArk, 2025 | |
| ~400 gas | HDC similarity search cost on-chain | Nunchi chain precompile spec | 20-100x cheaper than equivalent Solidity |
| <1s | ZK-HDC proof generation time | Circom + Groth16 benchmarks | |
| 97M | Monthly MCP SDK downloads | Linux Foundation, 2025 | Template for protocol adoption curve |
| Aug 2, 2026 | EU AI Act Article 50 enforcement | EU Regulation 2024/1689 | ~14 weeks from now |
| 35.7% | EU managers who feel prepared for AI Act | Deloitte AI Regulation Survey, 2025 | |
| $668T | Global OTC interest rate derivatives | BIS Triennial Survey, H1 2025 | ISFR expansion domain |
| 50x | Cost variation between agents at similar accuracy | Princeton HAL, arXiv:2407.01502 | Shows headroom for optimization |
| 5x | Prompt caching alone | Anthropic cache pricing (90% discount) | The honest intermediate step |
| 92% | Claude Code cache hit rate | LMCache (third-party, Dec 2025) | NOT official Anthropic data. Do not misattribute |
| 99.8% | Anthropic's internal cache hit | Anthropic postmortem, April 23, 2026 | Specific internal pipeline |

### Numbers to Never Fake

- GitHub stars — show the real number even if it's 47
- Customer logos — only show signed design partners
- ARR — if pre-revenue, say "pre-revenue, N design partners at LOI stage"
- Download counts — real numbers only

---

## 7. What the Demo Is NOT (Proactive Honesty)

These points should be acknowledged proactively, not in response to skeptical questions.

1. **The chain is not live in production.** The demo uses mirage-rs, a local EVM simulator. Chain mechanics are fully implemented and visually identical to what mainnet will show. Mainnet launch is the Phase 1 milestone.

2. **ZK proofs are generated and verified locally**, not on mainnet. The cryptographic construction is production-ready; the chain they anchor to is not yet live.

3. **Cross-organization knowledge sharing requires both organizations to run Roko-connected agents.** The network effect is not available to agents outside the ecosystem. Same constraint that faced every network business at launch.

4. **The 30x number assumes proper caching configuration.** Misconfiguration (unique system prompts defeating L1 caching) wastes the benefit. The SDK defaults prevent this by construction, but it's possible to misconfigure.

5. **ISFR and cooperative clearing are future expansion**, not the current product. They are what the coordination plane enables at scale — a second pitch for crypto-native investors, not the primary pitch for infrastructure investors.

---

## 8. Audience-Specific Framing

### For Infrastructure Investors (Casado Lens)

Lead with: coordination plane, control/data plane separation, SDN vocabulary, durable execution.

"Every agent needs coordination infrastructure — shared identity, reputation, and knowledge. The Roko runtime delivers 10-30x cost reduction as the acquisition wedge. The Nunchi chain makes knowledge compound across organizations. The beachhead is enterprise agent coordination."

Architecture slide must have two explicit horizontal bands: control plane (Nunchi) and data plane (agent execution). This is the Casado identity test — every company he backs has this separation.

### For Crypto-Native Investors (Dixon/Yahya Lens)

Lead with: on-chain benchmark rate, clearing-as-inference, the $668T gap.

"ISFR is DeFi's SOFR moment — the first credible on-chain benchmark rate. $668 trillion in TradFi rate derivatives versus less than $100 million on-chain. Two primitives were missing: a benchmark rate and a continuous hedging instrument. Yield perpetuals are the instrument. Cooperative clearing with KKT proofs is the mechanism."

**Critical sequencing**: The infrastructure story is the Series A pitch. The crypto story is the appendix or the second conversation. Leading with ISFR in an infrastructure meeting repositions Nunchi as a DeFi project. Lead with coordination plane and cost reduction.

### For Technical Diligence

Show the dashboard. Four focused views:
1. **Cost Dashboard** — real-time cost meter, per-agent spend, cache hit rate, routing decisions, gate outcomes
2. **Agent Fleet** — active agents with ERC-8004 identities, reputation scores, current tasks, costs
3. **Knowledge Graph** — force-directed graph of published knowledge, citation edges, demurrage decay visualization
4. **Chain View** — live block explorer, knowledge publications, ZK proof statuses, identity attestations

---

## 9. The Competitive Positioning in the Demo

The demo should make competitors feel like a different category, not a different product.

| Competitor | What They Are | What They're Missing |
|-----------|--------------|---------------------|
| **LangChain / LangGraph** | Agent framework + observability (LangSmith) | No identity, no cross-agent knowledge, no durability, no chain. Framework-level, not infrastructure-level |
| **CrewAI** | Multi-agent orchestration platform | No cost optimization, no identity, no knowledge substrate, no chain. Orchestration without coordination |
| **Temporal** | Durable execution platform (non-AI) | Durable execution for workflows, not agents. No model routing, no gates, no knowledge. Adjacent but different workload |
| **Cursor** | AI-native IDE | Single-developer tool. No multi-agent coordination, no cross-org knowledge, no identity. End-user product, not infrastructure |
| **Devin (Cognition)** | Autonomous coding agent | Application, not infrastructure. Devin uses infrastructure; Nunchi is infrastructure. $25B valuation validates the category |
| **Nava** | Trust-intercept layer ($8.3M, April 2026) | Trust layer only. No runtime, no knowledge, no chain. Partial solution |

**The empty quadrant**: Every competitor is either a framework (helps you write agent code) or a trust layer (helps you verify agent behavior) or a single-agent product (does agent work for you). None is a coordination plane — the infrastructure that makes all agents cheaper, safer, and smarter as they scale. That quadrant is empty. Nunchi occupies it alone.

**Anti-pattern vocabulary** (never use in the demo or deck):
- "Web3 platform"
- "tokenomics"
- "blockchain company"
- "DeFi"
- "we're contrarian" (Casado has publicly tweeted that non-consensus investing is overrated — give a consensus pitch for a non-consensus product)

**Replacement vocabulary**:
- "infrastructure for agent coordination"
- "incentive design"
- "verifiable compute" or "programmable trust"

---

## 10. Revenue Model (For Q&A)

Three streams, not led with in the demo but ready for questions:

**Stream 1 — Enterprise Support on Roko OSS (near-term)**
Temporal precedent: raised $18.75M Series A (Oct 2020) with zero commercial product. First dollar came from enterprise support contracts. The 1,000th paying Cloud customer arrived 3.5 years later.

**Stream 2 — Managed Cloud (12-24 months)**
Per-action billing in USD. Enterprise tiers: Standard ($5K-25K/month), Professional ($25K-100K/month), Enterprise (custom). Comparable: Vercel at $340M ARR; Temporal at $5B with >380% YoY growth.

**Stream 3 — Chain Economics (long-term)**
Block production fees, validator staking, knowledge posting fees, job marketplace fees (5% combined on agent job completions). NUNCHI token is for staking and governance only — not for payment. Burn-and-mint equilibrium modeled on Helium's HIP-141.

**Pricing**: Per-action in USD. Never percentage-of-savings (attribution is unauditable, perversely rewards bad baselines). Comparable: Sierra ~$1.50/resolution, Crescendo $1.25, Decagon ~$0.50. Nunchi target: $0.10-0.75 depending on action complexity.

---

## 11. The 30-Second Init Benchmark

This is the internal bar the engineering team must hit before the demo is ready:

1. **Scaffold in under one second.** Sub-second, Vite-style. No progress bar, no "installing dependencies," just done.
2. **First agent boots and prints a colored sigil** (visual identity derived from ERC-8004 address) within 10 seconds of `nunchi init`.
3. **Two agents exchange a message** with a ZK proof, a cost meter reading, and a shareable identity URL within 30 seconds of initialization.

If the 30-second benchmark is not hit, the demo is not ready.

---

## 12. Sources Referenced in This Document

| Source | Citation | Used For |
|--------|----------|----------|
| MAST taxonomy | arXiv:2503.13657, NeurIPS 2025 | 41-86% failure rate, 79% coordination failures |
| Princeton HAL | arXiv:2407.01502, ICLR 2026 | 50x cost variation, $44.86→$1.42 |
| RouteLLM | arXiv:2406.18665, Princeton NLP 2024 | 85% cost reduction retaining 95% quality |
| CyberArk | CyberArk 2025 Machine Identity Report | 82:1 machine-to-human identity ratio |
| Anthropic | Official cache pricing documentation | 90% cache discount |
| ProjectDiscovery | Engineering blog | 7%→84% cache hit rate, 9.8B cached tokens |
| LMCache | Third-party, December 2025 | 92% Claude Code cache hit (NOT Anthropic data) |
| EU AI Act | Regulation 2024/1689 | Article 50, August 2, 2026 enforcement |
| Deloitte | AI Regulation Survey, 2025 | 35.7% EU managers prepared |
| BIS | Triennial Survey, H1 2025 | $668T interest rate derivatives |
| MCP adoption | Linux Foundation, 2025 | 97M monthly downloads |
| Temporal | a16z announcement, Feb 2026 | $5B valuation, led by Sarah Wang + Raghu Raghuram (NOT Casado) |
| Braintrust | a16z portfolio | $800M valuation, evaluation layer |
| Story Protocol | Series B, Aug 2024 | $80M at $2.25B, dual-entity template |
| Helium | Nova Labs / Helium Foundation | HIP-141 burn-and-mint template |
| Odling-Smee et al. | Niche Construction, Princeton 2003 | Co-evolution theory for knowledge substrate |
| Gibson | Ecological Approach to Visual Perception, 1979 | Affordance theory for routing |
| Pirolli & Card | Information Foraging, Psych Review 1999 | Information scent / context efficiency |
| Grassé / Heylighen | Stigmergy, 1959 / 2016 | O(1) coordination scaling |

---

---

## 13. Pre-Meeting Media Arc

The pitch on May 6 is the closing of an arc, not its opening. The optimal media stack:

| Timing | Medium | Purpose |
|--------|--------|---------|
| T-14 days | **Stripe-Press-grade essay** | Load-bearing artifact. 4,000-7,000 words, single-page HTML, custom typography, CC0-licensed. Title: *"Sovereignty: Why Agents Need Their Own Chain"* or *"The Agent's Body."* Drop on HN at 8am PT Tuesday. |
| T-7 days | **Stratechery interview** | Origin story / philosophical positioning. Requires warm intro (~3 weeks lead time). Best vector: Casado portfolio CEO (Cursor's Aman Sanger, Convex's James Cowling, Netlify's Matt Biilmann). |
| T-3 days | **Latent Space episode** | Technical deep-dive. Requires warm intro (~1 month lead time). |
| T-0 | **The meeting** | The demo is the climax, not the introduction. |

Tailscale's Avery Pennarun proved this exact mechanism in April 2025 with his Stratechery interview — Tailscale's marketing now treats it as their canonical origin story.

**The essay format** (per research across Collison's `/fast`, Vitalik's "Endgame," Bezos Day-1 letters, Aschenbrenner's *Situational Awareness*, Amodei's "Machines of Loving Grace"):
- Numbered structure where each section stands alone
- Single load-bearing claim in the first paragraph
- Quantified specificity over rhetoric
- A coined phrase that becomes a meme ("counting the OOMs," "Endgame," "Day 1")
- Citation density with footnotes
- A thesis-defining negative that names a wrong consensus
- A personal grace note

**A/B test the positioning line** on infra Twitter the week before: "the agent operating system" vs "agent-native control plane" — walk in with the winning version.

---

## 14. Physical Artifacts

### The Nunchi Cell (Credit-Card PCB)

The single most durable physical artifact. A credit-card-sized PCB in a black anodized sleeve:

- **Front**: Etched 256×256 binary pattern that IS the recipient's HDC fingerprint, derived live from the meeting transcript hash (so months later they can prove it was minted then and there)
- **Display**: Tiny e-ink display in cholesteric-LCD aesthetic showing the Cell's current ISFR and ERC-8004 attestation count
- **NFC**: Tap opens an explorer page filtered to this Cell
- **Engraved serial**: `nunchi-cell-0001` for Casado
- **Cost**: ~$30/unit
- **Cultural lineage**: DEFCON badgelife crossed with Bright Moments NFT minting, ferried by Worldcoin's "Orb-as-ritual" gravitas

He tells the story for years. Unlike a Loom recording, it sits on his desk.

### The Zine

A 32-page perfect-bound Berkeley-Mono-set zine called *Sovereignty Vol. I*:
- The Nunchi essay
- Vitalik quote on inside front cover
- Hamming quote on inside back cover
- Single technical diagram of the Roko execution graph
- One copy, printed by a fine-press printer (Edition.Studio or similar)
- Hand the only copy to Casado

He recognizes the Stripe Press lineage on sight. The inscribed card signals you understand the form.

---

## 15. Technical Architecture (For Context)

This section provides enough technical context to understand how the demo primitives map to real code. For the complete codebase reference (every crate, every route, every file), see CODEBASE-CONTEXT.md.

### How the Runtime Works

Roko is a Rust workspace with 33 crates. The core execution loop (`roko run "<prompt>"`) follows these steps:

1. **Compose**: `PromptComposer` assembles a prompt from sections (role, context files, task) with priority ordering and token budget management. The `SystemPromptBuilder` can assemble 9-layer system prompts from templates, knowledge hints, and tool descriptions.

2. **Route**: `CascadeRouter` uses Thompson sampling and LinUCB bandit algorithms to select the cheapest model that can handle the task. It routes across 8+ LLM backends: Claude CLI (subprocess), Anthropic API (direct HTTP), Ollama (local models), Codex, OpenAI-compatible endpoints, Gemini, Perplexity, and generic subprocesses. The router learns from every execution — model weights are persisted to `.roko/learn/cascade-router.json`.

3. **Execute**: The selected backend runs the agent. The agent has access to 19 built-in tools (file read/write, shell exec, web search, etc.) via a tool loop. MCP (Model Context Protocol) config can be passed through for additional tool access.

4. **Gate**: After the agent completes, outputs pass through the gate pipeline: 11 gates across 7 rungs (compile, test, clippy, shell, diff review, LLM judge, integration test, regression). Each gate produces a verdict (pass/fail). Adaptive thresholds adjust pass criteria based on historical data via EMA (Exponential Moving Average).

5. **Persist**: Every artifact is a `Signal` — content-addressed (Blake3 hashed) and stored in `.roko/engrams.jsonl`. An `Episode` record captures the full execution: prompt ID, output ID, agent success, gate verdicts, cost, tokens. Episodes are appended to `.roko/episodes.jsonl`.

6. **Learn**: Efficiency events (cost, tokens, latency per turn) are recorded. The CascadeRouter updates its model weights. If a gate fails and `learning_config.replan_on_gate_failure` is set, the system generates a revised plan automatically.

### How the Knowledge Store Works

The `NeuroStore` (`crates/roko-neuro/`) is a durable knowledge store. When an agent completes a task, it can deposit "facts" — knowledge entries with confidence scores, timestamps, and source attribution. These are the entries that the demo's "loaded 9 facts from 4 agents" line refers to.

Knowledge entries progress through tiers based on usage:
- **Ephemeral**: New entries, not yet validated
- **Working**: Used at least once, moderate confidence
- **Consolidated**: Frequently cited, high confidence
- **Crystallized**: Core knowledge, very stable

Unused entries decay via an Ebbinghaus forgetting curve (demurrage). Frequently cited entries get reinforced. This creates a natural selection pressure: useful knowledge survives, useless knowledge fades.

When a new agent starts, the dispatch enrichment step queries the NeuroStore for relevant entries and injects them as "knowledge hints" in the system prompt. This is the mechanism behind Primitive 3 (Shared Knowledge).

### How the Chain Works

The Nunchi blockchain is a purpose-built sovereign EVM Layer 1 with:
- **~50ms block times**: Co-located validators (Tokyo data center model, similar to Hyperliquid)
- **HDC precompiles**: Native hardware-accelerated hyperdimensional computing operations (~400 gas for similarity search vs 100,000+ for equivalent Solidity)
- **ERC-8004**: Agent identity standard with 7-domain reputation (code quality, resource efficiency, latency, cost optimization, safety, gate compliance, collaboration)
- **ZK-HDC proofs**: Zero-knowledge proofs over HDC vectors — proves computation matches claims without revealing underlying data

For the demo, the chain runs locally via `mirage-rs` (an in-process EVM fork simulator in `apps/mirage-rs/`). The `roko-chain` crate uses the `alloy` Rust Ethereum library for both reads (`AlloyChainClient`) and writes (`AlloyChainWallet`). Chain interactions are optional — the runtime works without a chain connection.

### How the HTTP Server Serves Everything

`roko serve` (the `roko-serve` crate) starts an Axum HTTP server on port 6677. The server:
- Exposes ~115 API routes for all runtime operations (plans, agents, PRDs, knowledge, learning, config, etc.)
- Provides WebSocket endpoints for terminal PTY sessions and dashboard events
- Provides SSE endpoints for real-time event streaming
- Serves the React SPA via rust-embed (the compiled React app is baked into the Rust binary at build time)
- Runs 10+ background tasks (config watcher, feedback loop, state snapshotting, job runner, etc.)

The React SPA in `demo/demo-app/` has 7 pages, uses React 19 + Vite 6 + React Router 7, and currently uses only 11 of the ~115 available API endpoints. The SPA's build output (`dist/`) is embedded into the Rust binary via `rust-embed` so the entire application (API server + web dashboard + terminal emulator) ships as a single binary with zero external dependencies.

### The Self-Hosting Loop

Roko develops itself. The complete self-hosting workflow:

```bash
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"  # Capture idea
roko prd draft new "system-prompt-wiring"                      # AI drafts PRD
roko research enhance-prd system-prompt-wiring                 # Enrich with research
roko prd plan system-prompt-wiring                             # Generate implementation plan
roko plan run plans/                                           # Execute (agents run, gates validate)
roko plan run plans/ --resume .roko/state/executor.json        # Resume if interrupted
roko dashboard                                                 # Watch progress in TUI
```

Every step is a real CLI command that works today. The demo's four primitives (identity, cost prediction, shared knowledge, durability) are visible at every stage of this loop.

---

*This document is the anchor for all other demo planning documents. Cross-references: CODEBASE-CONTEXT.md (complete technical reference), DEMO-VISUAL-SPEC.md (what it looks like), DEMO-FLOW.md (beat-by-beat script), DEMO-COMPETITIVE.md (competitive landscape), DEMO-BUILD.md (what to implement).*
