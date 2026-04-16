# Coordination Plane Positioning, Build Portfolio, and the 90-Day Wedge

This document distills the strategic positioning, competitive map, build portfolio, and 90-day execution sequence required to convert Nunchi's research surface into a defensible go-to-market posture. It is written from scratch for a reader with no prior exposure to the project, the agent infrastructure landscape, or the prior research rounds. Every market figure, competitor positioning, and build artifact is sourced. The orientation: stop describing capabilities, start describing a category.

Source: `research/research6.md` ("Korai Market Research and Build Opportunities"), grounded against synthesis docs 18--21 on protocol adoption, agent economics, regulation, and measurement.

---

## 1. The Market Gap That Defines the Wedge

Three numbers establish the shape of the opportunity.

**Enterprise generative AI spend reached $37 billion in 2025**, up from $11.5 billion in 2024, with $19 billion at the application layer and $18 billion at infrastructure ([Menlo Ventures, 2025 State of Generative AI in the Enterprise](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). Within horizontal AI spend, copilots captured $7.2 billion. Agent platforms captured only $750 million. Productivity tools captured $450 million. The category is real but under-monetized: buyers have accepted AI budgets, but the spending pattern proves agent-specific infrastructure is still in the early adopter phase.

**Only 16% of enterprise deployments qualify as "true agents"** that plan, act, observe feedback, and adapt behavior (Menlo's strict definition). Startups do slightly better at 27%. McKinsey's parallel survey reports that 62% of organizations are at least experimenting with agents, but only 23% are scaling an agentic system, and no individual function exceeds 10% scaled adoption ([McKinsey, The State of AI](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai)). The implication: prototypes are everywhere, production agents are rare.

**88% of organizations use AI in at least one business function**, but nearly two-thirds have not begun scaling. 39% report enterprise-level EBIT impact, but most of those report less than 5% of EBIT attributable to AI use. Executives believe AI matters and budgets exist, but pilots refuse to convert into auditable operating systems.

The gap between "budget exists" and "production scales" is the product brief. Agent prototypes are easy because they can ignore state, accountability, budget, permissions, failure recovery, and audit. Agent production is hard because every agent is a distributed system with a model inside it. The buyer pain is not "we cannot build agents." The buyer pain is "we cannot operate them."

---

## 2. The Locked Substrate: MCP, A2A, x402, ERC-8004

Four protocols have effectively closed the lower layers of the stack. New entrants must compose with them, not replace them.

**MCP (Model Context Protocol)** -- Anthropic, November 2024. Standardizes agent-to-tool and agent-to-data connections. Shipped with reference servers for filesystem, GitHub, Drive, Postgres, Slack, Puppeteer. Inflection: Sam Altman's March 26, 2025 endorsement. By April 2026: 16,000+ public servers, 97-110M monthly SDK downloads. Donated to Linux Foundation December 9, 2025. **Status: locked-in tool layer.**

**A2A (Agent-to-Agent Protocol)** -- Google, April 9, 2025. Standardizes agent discovery, task lifecycle, artifact exchange, and collaboration messages. Launched with 50+ partners. Donated to Linux Foundation 75 days post-launch. AgentCard's typed `extensions` field absorbed IBM's competing ACP. **Status: locked-in coordination layer for agent-agent communication.**

**x402** -- Coinbase. Uses HTTP 402 "Payment Required" status code for stablecoin/crypto micropayments. V2 added wallet-based identity, reusable access sessions, automatic service discovery, subscriptions, prepaid access, usage-based billing, and multi-step workflow support ([Galaxy Research, x402 AI Agents Crypto Payments](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). Galaxy notes that since early December the apparent gaming/wash fraction dropped below 50% of transactions, with agent-to-agent services and data-as-a-service growing. **Status: emerging payment layer; remains payment-rail-agnostic-friendly.**

**ERC-8004** -- Ethereum standard for on-chain agent identity. Soulbound NFT passport with capability bitmask, system prompt hash, TEE attestation, reputation vector. Mainnet January 29, 2026. Within two weeks: 21,000-22,900 registrations across BNB Chain, Base, Ethereum L1. Caveat: registration counts measure interest, not active usage. **Status: emerging identity layer.**

The strategic reading is unambiguous: **the protocol layer has created a stable substrate for an economic and verification layer above it.** MCP handles tool access. A2A handles inter-agent messaging. x402 handles payment. ERC-8004 handles identity. None of them handles **accountable execution**: which agent acted, under whose authority, within what budget, using which tools, producing which proof, and receiving which payment or reputation update.

That is the unfilled layer. The category is **agent coordination plane**.

---

## 3. The Category Frame: "Agent Coordination Plane"

The category name matters because it determines the comparison set. The wrong name puts the company in a crowded fight; the right name creates white space.

| Avoid positioning as | Why it is weak | Better frame |
| --- | --- | --- |
| Generic "agent OS" | Hyperscalers, LangChain, and dozens of startups already occupy the phrase. | Agent coordination plane with proof, budget, and settlement. |
| Generic L1 or L2 | Chain buyers ask for TPS, liquidity, ecosystem, bridges. Not the strongest wedge. | Chain purpose-built for agent work receipts and coordination. |
| DeFi oracle | Chainlink, Pyth, API3, UMA, market-data incumbents dominate the mental model. | Benchmark administrator with agent-attested methodology. |
| Observability platform | LangSmith, Arize, Langfuse, Braintrust already credible. | Runtime-native proof and policy gates before work settles. |
| Payments protocol | x402, AP2, ACP, Stripe, Coinbase, Circle own payment narratives. | Payment-aware coordination and settlement for agent work. |
| Agent marketplace first | Marketplaces without trust receipts become directories. | Proof-of-work-done first, marketplace later. |

**Positioning statement.** For teams deploying production agents, Korai is the agent coordination plane that turns autonomous work into accountable work because it combines runtime state, scoped identity, budget enforcement, proof receipts, and settlement in one verifiable substrate.

**Investor short form.** Models are getting cheaper. Agent work is getting riskier. Korai makes agent work accountable.

The category frame "coordination plane" is specific enough to avoid "agent OS" noise, broad enough to include runtime and chain primitives, and close enough to cloud networking language (control plane / data plane) to make the architecture legible to senior engineering buyers.

---

## 4. The Three-Product Map

The clean external framing:

| Name | Role | External framing | Internal function |
| --- | --- | --- | --- |
| **Nunchi** | Company | The company building agent-native coordination infrastructure | Brand, fundraising entity, partnerships |
| **Korai** | Primary product and chain | Agent coordination chain and economic clearing substrate | Verifiable work, identity, settlement, proofs, reputation |
| **Roko** | Cognitive runtime | Runtime for durable, reflective, cost-aware agent execution | Signal/Cell/Graph kernel, memory, eval gates, cost governance |
| **ISFR** | First benchmark workload | Benchmark business for yield-bearing stablecoin and DeFi rates | Demonstrates agent-attested data, methodology, governance, licensing |

**Korai leads the narrative. Roko is the engine that makes Korai useful. ISFR is the wedge that makes Korai serious.** The mistake to avoid: leading with Roko as another agent framework, with Korai as a blockchain sidecar, or with ISFR as a DeFi oracle. The market is crowded in all three of those framings.

---

## 5. The Competitive Map (May 2026)

| Layer | What is becoming standardized | Representative players | Opportunity for Korai/Roko |
| --- | --- | --- | --- |
| Foundation models | Model APIs, coding agents, multimodal | OpenAI, Anthropic, Google, Meta, Mistral, xAI, DeepSeek | Treat models as replaceable workers. Do not compete. |
| Tool protocol | Agent-to-tool, agent-to-data connectivity | MCP, provider-specific tool APIs | Use MCP as default. Wrap with identity, cost, and proof. |
| Agent interoperability | Agent discovery, task lifecycle, artifacts | A2A, Agent Protocol, vendor registries | Use A2A. Add settlement, receipts, reputation. |
| Frameworks | Graphs, roles, conversations, workflows | LangGraph, CrewAI, AutoGen, Semantic Kernel, LlamaIndex | Integrate, do not replace. Be framework-agnostic. |
| Durable execution | Crash recovery, retries, long-running workflows | **Temporal**, LangSmith deployment, cloud runtimes | Differentiate with agent-native economics, proof, identity. |
| Observability + evals | Tracing, eval, annotation, dashboards | **LangSmith**, Arize, Langfuse, Braintrust | Make evals inline gates, not just dashboards. |
| Identity + permissions | OAuth, workload identity, policy engines | Auth0, Arcade, Entra, SPIFFE/SPIRE, OPA | Package principal-binding, delegate-binding, scopes, spend caps in one manifest. |
| Payments + settlement | Agent micropayments, checkout, API payment | x402, AP2, ACP, Stripe, Coinbase, Circle | Add coordination: discovery, constraints, budgets, proof, dispute. |
| Benchmarks + reference rates | Methodology, governance, licensing | CF Benchmarks, CoinDesk Indices, Treehouse, market-data incumbents | Enter with ISFR-YBS as benchmark, not oracle. |

**Temporal** describes itself as "Durable Execution" for resilient production agents -- crash recovery, retries, long-running logic, observability, testability, HITL ([Temporal](https://temporal.io/pages/durable-ai-agent-bundle)). Cites Gorgias scaling agents to 15,000 brands. **Confirms the category exists.** Korai/Roko cannot claim "no one has durable runtime."

**LangSmith** offers framework-agnostic observability, online evals, human annotation, deployment, background agents, multi-agent coordination, exactly-once execution, native A2A/MCP/Agent Protocol support ([LangSmith Platform](https://www.langchain.com/langsmith-platform)). **Confirms the category exists.** Korai/Roko cannot claim "no one has agent eval infrastructure."

The differentiation is narrower and stronger: Temporal is durable execution, LangSmith is agent engineering, while Korai/Roko is **proof-aware, payment-aware, identity-aware, benchmark-aware coordination**. The wedge is not "debug agents better." The wedge is "make agent work economically and cryptographically accountable."

---

## 6. The Three Wedge Markets

| Wedge | Buyer | Why now | Why Korai/Roko |
| --- | --- | --- | --- |
| **Benchmark operations** | DeFi protocols, stablecoin issuers, trading desks, index licensees | Yield-bearing stablecoins and on-chain credit need reference rates; institutional buyers require methodology, governance, audit. | Agent-attested data + methodology-as-code + on-chain proof receipts = benchmark administrator that is cheaper and more transparent than manual ops. |
| **Agent cost and governance** | Engineering, platform, AI ops, finance | Multi-agent systems amplify cost and failure; enterprises have AI budgets but limited scaled deployment. | Roko enforces cost-per-correct-answer, budget caps, routing, retry, escalation as runtime primitives. |
| **Agent identity and work receipts** | Security, compliance, devplatforms, marketplaces | MCP and A2A increase agent connectivity, which increases identity, privilege, and cascading-failure risks. | Korai binds principals, delegates, tool scopes, proofs, settlement into verifiable work receipts. |

**Sequence:** start with benchmark operations because it gives the system a serious, high-margin production workload; then expand into agent cost and governance because the same runtime primitives are broadly useful; finally use accumulated work receipts to create an agent trust registry and marketplace.

---

## 7. Security Wedge: OWASP Agentic Top 10 as Buyer Brief

OWASP released its Top 10 for Agentic Applications December 9, 2025, after >1 year of research with 100+ practitioners ([OWASP GenAI Security Project](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)). Named risks include: agent behavior hijacking, tool misuse, identity and privilege abuse, goal hijacking, human trust manipulation, rogue autonomous behaviors, memory poisoning, privilege escalation, prompt injection, data leakage.

This creates a second wedge alongside cost. A buyer who does not care about agent payments may still care that:
- An agent cannot use a destructive tool without a scoped grant
- An agent cannot spend past budget
- An agent cannot invoke a high-risk workflow without approval
- An agent cannot claim successful work without a receipt

**Security is not a compliance appendix. It is a category-defining feature.** Every enterprise call now asks "how does your protocol prevent the Replit incident?" (1,206 records deleted by an unbounded coding agent) and "how does it prevent GTG-1002?" (state-actor jailbreak via task decomposition).

---

## 8. Build Portfolio (Ranked, Scored)

| Rank | Build | Urgency | Market pull | Differentiation | Demo clarity | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **ISFR-YBS live dashboard** | Very high | High | Medium-high | High | **Must ship first** |
| 2 | **Browser-verified proof-of-work-done** | Very high | Medium | Very high | Very high | **Best investor demo** |
| 3 | **Korai/Roko coordination spec** | Very high | Medium | High | Medium | **Credibility gate** |
| 4 | **Cost governance runtime + CPCA demo** | High | Very high | High | High | **Best enterprise wedge** |
| 5 | Agent credential and payment manifest | High | High | High | Medium | Security + payments wedge |
| 6 | Agent-attested ISFR pipeline | High | Medium-high | Very high | High | Flywheel hinge |
| 7 | Inline eval gates and settlement | Medium-high | High | High | Medium | Differentiates from observability |
| 8 | 1,000-agent command surface | Medium | Medium | Medium-high | Very high | Visual proof of coordination |
| 9 | ZK-HDC proof-of-learning | Medium | Low-medium today | Very high | Medium | Long-term trust moat |
| 10 | DKG-private collaboration | Medium | Medium in regulated verticals | Very high | Low-medium | Later vertical wedge |

The top four are the P0 surface. Five through seven are P1 (days 31--60). Eight is a demo, not a product. Nine and ten are roadmap signals, not gating dependencies.

---

## 9. The Minimum Viable Receipt Envelope

The compact spec must define a message envelope that is backwards-compatible with MCP/A2A and carries the additional fields that make execution accountable. Minimum surface:

```json
{
  "protocol": "korai.work.v0",
  "principal": "did:org:nunchi:...",
  "delegate": "agent:roko:...",
  "task": "isfr.fetch.aave.v3.usdc",
  "scope": ["read:onchain", "write:receipt"],
  "budget": {"currency": "USD", "max": "0.25"},
  "policy": {"destructive": false, "hitl_required": false},
  "inputs_hash": "sha256:...",
  "output_hash": "sha256:...",
  "evals": [{"name": "freshness", "status": "pass"}],
  "proof": {"type": "merkle_receipt", "hash": "..."},
  "settlement": {"type": "x402-compatible", "status": "pending"}
}
```

Required design choices baked into the envelope:
- **Principal/delegate distinction.** Who authorized vs. who executed. Two distinct identity fields.
- **Scope.** Tool capabilities the delegate may exercise.
- **Budget.** Hard ceiling in dollars (or cost units), enforced at runtime.
- **Policy.** Destructive flag, HITL flag, retry rule.
- **Hashes.** Input and output content-addressed.
- **Eval status.** Inline gate outcomes (freshness, schema, accuracy proxy).
- **Proof.** Merkle receipt or equivalent verification artifact.
- **Settlement.** Payment rail and state, x402-compatible but rail-agnostic.

Spec target: under 50 pages, with TypeScript and Python validators shipped at v0.1, three external agents/tools emitting compatible receipts by Day 90.

---

## 10. The 90-Day Roadmap

### Days 0--30: Wedge artifacts

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Live ISFR-YBS dashboard | Constituent yields, source freshness, weights, exclusions, daily fixing. Honestly labeled "research rate" / "methodology preview." |
| Proof | Browser-verified agent activity page | Visitor verifies at least one job receipt without trusting a hosted dashboard. |
| Spec | Korai/Roko v0.1 coordination spec | Defines envelope, principal, delegate, scope, budget, proof, eval, settlement, extension fields. |
| Cost | CPCA benchmark demo | Same task suite, visible budget, retries, success rate, CPCA -- against bare ReAct and LangGraph. |
| DevEx | Sub-5-minute starter | Run a toy agent, emit a receipt, inspect the proof. |

### Days 31--60: Flywheel ignition

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Methodology-as-code paper draft | Deterministic, versioned, hashable, replayable, readable. |
| Data | Agent-attested source pipeline | At least 5 source fetches with signed receipts, freshness checks, disagreement handling. |
| Security | Agent credential and payment manifest | Tool scope, time bound, spending cap, revocation, approval threshold. |
| Evals | Inline gates | Failed freshness/schema/budget gate prevents settlement. |
| Partners | Design partner list | 10+ serious conversations across DeFi, data, agent tooling, security, devtools. |

### Days 61--90: Compounding

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Backtest + public methodology v0.2 | Historical reconstruction, stress windows, exclusions, limitations. |
| Agents | 1,000-agent command surface | Identity, role, scope, budget, receipt status, kill switch. |
| Proof | Receipt leaderboard | Work count, success, corrections, gate failures, reputation deltas. |
| Enterprise | Cost-governance pilot | One external team runs a task suite under Roko budget controls. |
| Regulatory | Benchmark readiness memo | Clear separation: research rate vs. production index vs. regulated benchmark. |

---

## 11. The "Smallest Complete Loop"

The single most important sentence in this synthesis: **build the first 90 days around one demonstrable loop.**

1. An ISFR-YBS data point is fetched by a Roko agent.
2. The agent acts under a scoped principal/delegate manifest.
3. The runtime enforces budget and freshness gates.
4. The output receives a proof-of-work-done receipt.
5. The receipt is visible and browser-verifiable on Korai.
6. The methodology preview explains how that data point contributes to a fixing.

That loop is the company in miniature. It is a benchmark (ISFR), an agent runtime (Roko), a chain (Korai), a proof system (browser-verifiable), and a market opportunity (yield-bearing stablecoin reference rate). Everything else compounds from there.

---

## 12. Risk Register (10 named risks)

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Overclaiming benchmark status | Institutional buyers punish sloppy governance claims. | Call early product a research rate or methodology preview until UK BMR Cat-6 path is real. |
| Competing head-on with LangSmith/Temporal | Both have credible runtime/observability positioning. | Integrate where useful. Differentiate on proof, identity, budget, settlement, benchmark workloads. |
| Becoming an x402 derivative | Galaxy notes early speculative usage on x402. | Be payment-rail agnostic. Support x402-compatible flows without depending on x402 as the category. |
| Protocol sprawl | Devs will not adopt a spec that ignores MCP/A2A. | Treat MCP and A2A as imports. Keep Korai/Roko focused on clearing semantics. |
| Security blind spot | OWASP risks include identity abuse, tool misuse, memory poisoning, privilege escalation. | Make scoped credentials, destructive-action gates, revocation, audit receipts P0/P1. |
| ZK dependency risk | Proving systems may not be ready for full HDC workflows. | Keep ZK-HDC as P2, not a dependency for ISFR or cost governance. |
| Naming confusion | Roko, Korai, Daeji, protocol naming can fragment story. | Lock external naming before public launch: Nunchi company, Korai primary product, Roko runtime. |
| Generic chain positioning | L1 buyers evaluate against liquidity, bridges, throughput, ecosystem. | Lead with agent-native work receipts and benchmark clearing, not TPS. |
| Marketplace too early | Empty marketplaces look weak and invite spam. | Build proof receipts and trust graph before launching marketplace. |
| Unsupported traction claims | Fabricated/weak market numbers damage credibility. | Use only sourced public evidence and honest internal milestone language. |

---

## 13. Recommended First Narrative (Sequential, 7 Lines)

1. Enterprise AI has budget, but real agents are not scaling.
2. MCP and A2A solved connectivity, not accountability.
3. Production agents need clearing: identity, scope, budget, proof, eval, payment, reputation.
4. **Korai is the coordination chain for accountable agent work.**
5. **Roko is the runtime that makes agents cost-aware, stateful, auditable.**
6. **ISFR-YBS is the first workload proving this is not theory.**
7. The first demo is browser-verifiable: the viewer can check agent work directly.

This narrative keeps the product out of crowded categories and lets every build artifact reinforce the same claim. The dashboard proves market competence. The proof page proves chain necessity. The spec proves interoperability. The cost demo proves enterprise ROI. The credential manifest proves security seriousness. The agent-attested pipeline proves the benchmark-agent flywheel.

---

## 14. Final Recommendation

Build the first 90 days around one sentence: **agent work should clear like financial work**. That means it has identity, authorization, cost, source, methodology, proof, acceptance, and settlement. Korai is the substrate where that clearing happens. Roko is the runtime that produces the work and receipts. ISFR-YBS is the first market where the entire system matters.

The highest-leverage thing to build first is not the most technically ambitious primitive. It is the smallest complete loop (Section 11). That loop ships in 30 days. Everything in Sections 8--10 then layers on top of it.

The tactical sequence is:
1. **Compete on accountability, not capability.** The model market is commoditizing; the scaffold is not.
2. **Compose with MCP/A2A/x402/ERC-8004.** Do not fork the substrate; build the layer above it.
3. **Lead with ISFR-YBS as the workload.** Generic infrastructure is a fundraising death spiral.
4. **Make proof browser-verifiable.** It is the single legible demo no oracle or framework can copy.
5. **Sell CPCA, not throughput.** Cost-per-correct-answer translates AI into CFO language.

Everything else is downstream of these five choices.
