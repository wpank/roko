# Korai Market Research and Build Opportunities

## Executive summary

The next build opportunity for Nunchi is not a generic agent framework, a generic blockchain, or a generic oracle. The open market gap is a coordination plane for production agents: a layer that makes agent work durable, accountable, economically bounded, permissioned, paid, benchmarkable, and eventually regulated. Enterprise AI spend has crossed into real budget territory, but true agent deployments remain rare: enterprise generative AI spend reached $37 billion in 2025, while only 16% of enterprise deployments qualify as true agents that plan, act, observe feedback, and adapt behavior ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). McKinsey shows the same gap from a different angle: 62% of respondents are at least experimenting with AI agents, but only 23% are scaling an agentic system somewhere in the enterprise, and no more than 10% are scaling agents in any given function ([McKinsey](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai)).

![Agent production gap](https://d2z0o16i8xm8ak.cloudfront.net/8e9c1e4e-6ca8-4a00-ad3d-2ea67aeaa3dd/ad6fa9ab-c2d6-4c51-8345-cdbed03184e6/agent-production-gap.png?Policy=eyJTdGF0ZW1lbnQiOlt7IlJlc291cmNlIjoiaHR0cHM6Ly9kMnowbzE2aTh4bThhay5jbG91ZGZyb250Lm5ldC84ZTljMWU0ZS02Y2E4LTRhMDAtYWQzZC0yZWE2N2FlYWEzZGQvYWQ2ZmE5YWItYzJkNi00YzUxLTgzNDUtY2RiZWQwMzE4NGU2L2FnZW50LXByb2R1Y3Rpb24tZ2FwLnBuZz8qIiwiQ29uZGl0aW9uIjp7IkRhdGVMZXNzVGhhbiI6eyJBV1M6RXBvY2hUaW1lIjoxNzc4MjA5ODQ2fX19XX0_&Signature=snhUXeeOVZx-kB~T-JP3Xmn~1Vyudzb3-lDMu1BAVQ8tnFrt7AsB1dhsk~vwf7mTiwxiq8KgUEylNcM1UuzH6PDMzzBEqgig97eb~U4gsuqeybdsos-H04JVPM0hnTDIn60zoA7phQhXzGQdy~U9Qv-kuEr6vuZmmNs5VTd3L6cxgiMn33kUz3qxP6vxzuphd8car6jgg~zugrgZoE-g~kjW6w6j-CozVpRg87f23cFeoj4Rukx0S7mylCR6W0NYNsuz8h3wkWiEuLJwTCjfIbav9bZrgRjGMOUOJinIBTS22QYJjeQkZBR8dUKxRkSwyPqTVtjxRmLsY3k6VbQrSA__&Key-Pair-Id=K1BF7XGXAIMYNX)

The core thesis should be: the model is commoditizing, but the scaffold is not. The scaffold means the full production substrate around agents: identity, permissions, coordination, budgets, proofs, memory, eval gates, attestation, payment, and governance. Anthropic’s MCP standardized how agents connect to tools and data sources, while Google’s A2A standardized how agents discover and collaborate with other agents across vendors and frameworks ([Anthropic](https://www.anthropic.com/news/model-context-protocol), [Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). That standardization is not a reason to build another MCP or A2A clone. It is the reason to build the next layer above them: a runtime and economic clearing layer that treats MCP tools, A2A agents, benchmark data sources, and on-chain settlement as components of one accountable system.

The recommended sequence is to lead with Korai as the agent-native coordination chain, Roko as the cognitive runtime that proves the chain’s usefulness, and ISFR-YBS as the first serious workload. ISFR should stay framed as a benchmark business, not oracle infrastructure. The market does not need another price feed, agent OS, or L1 speed claim. It needs a credible reference-rate and agent-coordination system where work, data, identity, payments, and methodology are verifiable by construction.

The build order should be ruthless. The first 30 days should produce: a live ISFR-YBS dashboard, a browser-verified proof-of-work-done demo, a compact Korai/Roko coordination spec, and a cost-governance demo that makes the “agent tax” measurable. The next 60 days should produce agent-attested benchmark data, a methodology-as-code whitepaper, a scoped agent credential and payment manifest, inline eval gates, and a 1,000-agent control surface. The long-term research moats, including ZK-HDC, proof-of-learning, DKG-private collaboration, and large-scale reputation, should remain on the roadmap but should not block the market wedge.

## Market signal

### Enterprise AI spend is large, but agent platforms are under-monetized

Enterprise buyers are already spending real money on AI, but the spending pattern shows that agent infrastructure is still early. Menlo Ventures estimates enterprise generative AI spend at $37 billion in 2025, up from $11.5 billion in 2024, with application-layer spend at $19 billion and infrastructure spend at $18 billion ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). Within horizontal AI spend, copilots captured $7.2 billion, while agent platforms captured only $750 million and productivity tools captured $450 million ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). This is the clearest market opening: buyers have accepted AI budgets, but the category for agent production infrastructure is still small relative to usage and pain.

![Agent platform spend gap](https://d2z0o16i8xm8ak.cloudfront.net/8e9c1e4e-6ca8-4a00-ad3d-2ea67aeaa3dd/ffe6ef89-eff6-4f71-840d-6fa7f43d43ab/agent-platform-spend-gap.png?Policy=eyJTdGF0ZW1lbnQiOlt7IlJlc291cmNlIjoiaHR0cHM6Ly9kMnowbzE2aTh4bThhay5jbG91ZGZyb250Lm5ldC84ZTljMWU0ZS02Y2E4LTRhMDAtYWQzZC0yZWE2N2FlYWEzZGQvZmZlNmVmODktZWZmNi00ZjcxLTg0MGQtNmZhN2Y0M2Q0M2FiL2FnZW50LXBsYXRmb3JtLXNwZW5kLWdhcC5wbmc~KiIsIkNvbmRpdGlvbiI6eyJEYXRlTGVzc1RoYW4iOnsiQVdTOkVwb2NoVGltZSI6MTc3ODIwOTg0Nn19fV19&Signature=IpcyT2uUWrz~7ZyKWj3QM2ODih7ppO1Ju2YqbngD91fj-SSnETPX7MWlW2QujYVqAj-xEaIxV3Ji75MDroEsUooXEsr28tzKdzlN8ZB2oXi7CkiujBWE4lAVKpjGUi9Fd9NT7gg3oyh1gNkiNcwzNfvQs56~4~CQwbjlgrkwp0mnsGbLM2QbVFiEzWKehO3P5VrBbjuK8mQwptXODX2OjeDOHIjbyEQ8Ko9IxL4jydzFKDT-koA--KSiGK1hr4svaRQlPUZ-vbrRkQbPCaNNB35c81uYPghe-Kcy948cU7sb7fWI5MK-AGZkylI7oLX2w0mI~GfIwjxiOKcjiaEaug__&Key-Pair-Id=K1BF7XGXAIMYNX)

Coding shows where the market is going first. Coding represented $4.0 billion of departmental AI spend in 2025, or 55% of departmental AI spend, and Menlo reports that 50% of developers now use AI coding tools daily ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). Claude Code’s own positioning has moved beyond autocomplete: it reads codebases, plans changes across files, runs tests, iterates on failures, and requires explicit permission before modifying files or running commands by default ([Anthropic Claude Code](https://www.anthropic.com/product/claude-code)). This matters because coding is no longer just a text-generation market. It is the first scaled enterprise category where developers are learning to supervise multiple semi-autonomous workers.

The implication for Korai and Roko is direct: the first users for production agent infrastructure will not be vague “AI teams.” They will be engineering, operations, finance, risk, and data teams already dealing with autonomous or semi-autonomous workflows. The product has to sell reliability, cost control, provenance, and permissions before it sells intelligence.

### The production gap is the buyer pain

McKinsey reports that 88% of respondents use AI regularly in at least one business function, but nearly two-thirds have not yet begun scaling AI across the enterprise ([McKinsey](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai)). The same survey finds that 39% report an enterprise-level EBIT impact from AI, but most of those report less than 5% of EBIT attributable to AI use ([McKinsey](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai)). This creates a practical buyer contradiction: executives believe AI matters, budgets exist, but most organizations cannot yet convert pilots into scaled, auditable operating systems.

The agent-specific gap is even sharper. McKinsey finds that 62% of organizations are at least experimenting with agents, but only 23% are scaling an agentic system, and no more than 10% are scaling agents in any individual function ([McKinsey](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai)). Menlo’s stricter definition is even more conservative: only 16% of enterprise deployments and 27% of startup deployments qualify as true agents ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)).

That gap should be treated as the product brief. Agent prototypes are easy because they can ignore state, accountability, budget, permissions, failure recovery, and audit. Agent production is hard because every agent is a distributed system with a model inside it. The market opportunity is to make that distributed system legible and governable.

### Protocol standardization creates the next layer

MCP and A2A have reduced protocol uncertainty. Anthropic introduced MCP in November 2024 as an open standard for connecting AI assistants to content repositories, business tools, development environments, and other systems where data lives ([Anthropic](https://www.anthropic.com/news/model-context-protocol)). MCP ships with a specification, SDKs, local MCP server support in Claude Desktop, an open-source server repository, and prebuilt servers for Google Drive, Slack, GitHub, Git, Postgres, and Puppeteer ([Anthropic](https://www.anthropic.com/news/model-context-protocol)). It answers the tool-use question: how does an agent safely connect to external systems?

Google introduced A2A in April 2025 as an open protocol that lets agents communicate, securely exchange information, and coordinate actions across enterprise platforms even when built by different vendors or frameworks ([Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). A2A includes capability discovery through JSON Agent Cards, task lifecycle management, artifact exchange, collaboration messages, and user-experience modality negotiation ([Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). Google launched A2A with more than 50 technology and services partners, including Atlassian, Box, Cohere, Intuit, LangChain, MongoDB, PayPal, Salesforce, SAP, ServiceNow, Workday, Accenture, BCG, Deloitte, KPMG, McKinsey, PwC, TCS, and Wipro ([Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)).

The right read is not “the protocol layer is closed.” The right read is that the protocol layer has created a stable substrate for an economic and verification layer. MCP handles tool access. A2A handles inter-agent messaging. Korai and Roko should handle accountable execution: which agent acted, under whose authority, within what budget, using which tools, producing which proof, and receiving which payment or reputation update.

## Competitive landscape

### The live stack

| Layer | What is becoming standardized | Representative players | Opportunity for Korai and Roko |
| --- | --- | --- | --- |
| Foundation models | Model APIs, coding agents, multimodal models | OpenAI, Anthropic, Google, Meta, Mistral, xAI | Do not compete on model quality. Treat models as replaceable workers. |
| Tool protocol | Agent-to-tool and agent-to-data connectivity | MCP, provider-specific tool APIs | Use MCP as default tool interface. Add identity, cost, and proof enforcement around tool use. |
| Agent interoperability | Agent discovery, task lifecycle, artifacts, messages | A2A, Agent Protocol, vendor agent registries | Use A2A rather than fight it. Add settlement, proof receipts, and reputation. |
| Frameworks | Graphs, roles, conversations, workflows | LangGraph, CrewAI, AutoGen, Semantic Kernel, LlamaIndex | Integrate, do not replace. Runtime should be framework-agnostic. |
| Durable execution | Crash recovery, retries, long-running workflows | Temporal, LangSmith deployment, cloud runtimes | Differentiate with agent-native economics, proof, and identity. |
| Observability and evals | Tracing, evaluation, annotation, dashboards | LangSmith, Arize, Langfuse, Braintrust | Do not be just a tracer. Make evals and proofs inline gates in execution. |
| Identity and permissions | OAuth, workload identity, policy engines, secret isolation | Auth0, Arcade, Entra, SPIFFE/SPIRE, OPA | Package principal-binding, delegate-binding, tool scopes, and spending limits as one manifest. |
| Payments and settlement | Agent micropayments, checkout, API payment | x402, AP2, ACP, Stripe, Coinbase, Circle | Add coordination: discovery, constraints, budgets, proof-of-work-done, and dispute logic. |
| Benchmarks and reference rates | Methodology, governance, licensing, administration | CF Benchmarks, CoinDesk Indices, Treehouse, market-data providers | Enter with ISFR-YBS as benchmark, not oracle. Use agents to operate it better. |

### Temporal and LangSmith validate the runtime category

Temporal’s agent positioning is important because it confirms that production agents are being reframed as durable distributed systems. Temporal describes itself as the Durable Execution layer for resilient, production-ready agents, with crash recovery, retries, long-running logic, observability, testability, and human-in-the-loop orchestration ([Temporal](https://temporal.io/pages/durable-ai-agent-bundle)). Temporal also positions its OpenAI Agents SDK integration as a way to add durable execution to agent stacks and cites Gorgias scaling agents to 15,000 brands with retries, sagas, and human-in-the-loop steps handled by Temporal ([Temporal](https://temporal.io/pages/durable-ai-agent-bundle)).

LangSmith validates the same category from the agent-engineering side. LangSmith positions itself as a framework-agnostic platform for observing, evaluating, and deploying agents, with traces, cost and latency monitoring, offline and online evals, human annotation, background agents, multi-agent coordination, a durable runtime, exactly-once execution, and native A2A, MCP, and Agent Protocol support ([LangSmith](https://www.langchain.com/langsmith-platform)). This means Korai and Roko should not claim that no one has durable runtime, observability, evals, or HITL. That claim would be false and strategically weak.

The better differentiation is narrower and stronger: Temporal is durable execution, LangSmith is agent engineering, while Korai and Roko should become proof-aware, payment-aware, identity-aware, benchmark-aware coordination. The wedge is not “debug agents better.” The wedge is “make agent work economically and cryptographically accountable.”

### Payments are advancing, but coordination remains open

Galaxy Research describes x402 as an agentic payment standard launched by Coinbase that uses the HTTP 402 “Payment Required” status code to let agents or humans pay for online services using stablecoins or crypto assets ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). Galaxy also notes that early x402 activity had a speculative spike, but since early December, apparent gaming or wash-like activity dropped below 50% of transactions and agent-to-agent services, data-as-a-service, and infrastructure/utilities account for an increasingly large percentage ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). That is useful but should be treated carefully: x402 is promising, not yet proof of a mature agent economy.

x402 V2 is especially relevant to Nunchi because it moved toward wallet-based identity, reusable access sessions, automatic service discovery, subscriptions, prepaid access, usage-based billing, and multi-step agent workflows ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). Galaxy explicitly identifies the coordination layer above payment as the missing piece: service discovery, intent signaling, constraints such as budget and permissions, context management, and multi-step or multi-agent coordination ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). That is almost exactly the Korai/Roko opportunity, provided the product avoids becoming dependent on any single payment standard.

The practical build takeaway: make Korai x402-compatible, AP2-aware, and ACP-aware, but do not position the company as a payments protocol. Position it as the coordination layer where agent work can be permissioned, priced, proven, and settled.

### Security is moving from model safety to agent systems safety

OWASP released its Top 10 for Agentic Applications after more than a year of research and input from more than 100 security researchers, industry practitioners, user organizations, and GenAI technology providers ([OWASP](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)). The named risks include agent behavior hijacking, tool misuse and exploitation, identity and privilege abuse, goal hijacking, human trust manipulation, rogue autonomous behaviors, memory poisoning, privilege escalation, prompt injection, and data leakage ([OWASP](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)). OWASP frames the shift clearly: agent security is not just about preventing bad outputs, but about preventing cascading failures across systems that plan, persist, delegate, and use tools ([OWASP](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)).

This creates a second wedge alongside cost. A buyer who does not care about agent payments may still care that an agent cannot use a destructive tool without a scoped grant, cannot spend past budget, cannot invoke a high-risk workflow without approval, and cannot claim successful work without a receipt. Security is therefore not a compliance appendix. It is a category-defining feature.

## Strategic positioning

### The company and product map

The clean framing should be:

| Name | Role | External framing | Internal function |
| --- | --- | --- | --- |
| Nunchi | Company | The company building agent-native coordination infrastructure | Brand, fundraising entity, partnerships |
| Korai | Primary product and chain | Agent coordination chain and economic clearing substrate | Verifiable work, identity, settlement, proofs, reputation |
| Roko | Cognitive runtime | Runtime for durable, reflective, cost-aware agent execution | Signal/Cell/Graph kernel, memory, eval gates, cost governance |
| ISFR | First benchmark workload | Benchmark business for yield-bearing stablecoin and DeFi rates | Demonstrates agent-attested data, methodology, governance, licensing |

Korai should lead the narrative. Roko is the engine that makes Korai useful, and ISFR is the wedge that makes Korai serious. The mistake to avoid is leading with Roko as another agent framework, with Korai as a blockchain sidecar, or with ISFR as a DeFi oracle. The market is crowded in all three of those framings.

### The category should be “agent coordination plane”

The strongest category frame is agent coordination plane. It is specific enough to avoid “agent OS” noise, broad enough to include runtime and chain primitives, and close enough to cloud networking language to make the architecture legible. It also gives the company room to say that MCP and A2A are not competitors. They are packet formats and interfaces. Korai is the control and clearing layer.

The positioning statement:

> For teams deploying production agents, Korai is the agent coordination plane that turns autonomous work into accountable work because it combines runtime state, scoped identity, budget enforcement, proof receipts, and settlement in one verifiable substrate.

The shorter investor version:

> Models are getting cheaper. Agent work is getting riskier. Korai makes agent work accountable.

### What to avoid

| Avoid positioning as | Why it is weak | Better frame |
| --- | --- | --- |
| Generic agent OS | Hyperscalers, LangChain, and many startups already occupy the phrase. | Agent coordination plane with proof, budget, and settlement. |
| Generic L1 or L2 | Chain buyers ask for TPS, liquidity, ecosystem, and bridges. That is not the strongest wedge. | Chain purpose-built for agent work receipts and coordination. |
| DeFi oracle | Chainlink, Pyth, API3, UMA, and market-data incumbents dominate that mental model. | Benchmark administrator with agent-attested methodology. |
| Observability platform | LangSmith, Arize, Langfuse, Braintrust, and others are already credible. | Runtime-native proof and policy gates before work settles. |
| Payments protocol | x402, AP2, ACP, Stripe, Coinbase, and Circle already own payment narratives. | Payment-aware coordination and settlement for agent work. |
| Agent marketplace first | Marketplaces without trust receipts become directories. | Proof-of-work-done first, marketplace later. |

## The build thesis

### From “agents that act” to “agents that clear”

The current market is obsessed with agents that can act. The next market will care about agents that can clear. Clearing means an agent can be identified, authorized, budgeted, observed, evaluated, paid, disputed, and remembered. A production buyer does not only ask “can the agent do the task?” The buyer asks:

- Who authorized the task?
- Which agent executed it?
- Which tools and data did it touch?
- How much did it spend?
- What did it prove?
- Who accepted the result?
- What happens if it fails, loops, colludes, lies, or exceeds scope?

Korai should be built around that question set. The product is not an agent that performs a task. It is the substrate that makes task performance admissible.

### The three wedge markets

| Wedge | Buyer | Why now | Why Korai/Roko |
| --- | --- | --- | --- |
| Benchmark operations | DeFi protocols, stablecoin issuers, trading desks, index licensees, data consumers | Yield-bearing stablecoins and on-chain credit need reference rates, but institutional buyers require methodology, governance, and audit. | Agent-attested data plus methodology-as-code plus on-chain proof receipts creates a benchmark administrator that is cheaper and more transparent than manual ops. |
| Agent cost and governance | Engineering, platform, AI ops, finance operations | Multi-agent systems amplify cost and failure, while enterprises have AI budgets but limited scaled deployment. | Roko can enforce cost-per-correct-answer, budget caps, routing, retry, and escalation as runtime primitives. |
| Agent identity and work receipts | Security, compliance, developer platforms, marketplaces | MCP and A2A increase agent connectivity, which increases identity, privilege, and cascading-failure risks. | Korai can bind principals, delegates, tool scopes, proofs, and settlement into verifiable work receipts. |

The wedge sequence should start with benchmark operations because it gives the system a serious, high-margin production workload. It should then expand into agent cost and governance because the same runtime primitives are broadly useful. Finally, it should use accumulated work receipts to create an agent trust registry and marketplace.

## Things to build

### P0: ISFR-YBS live dashboard and methodology preview

| Dimension | Detail |
| --- | --- |
| Customer | DeFi protocols, stablecoin issuers, rate desks, benchmark licensees, institutional crypto researchers |
| Problem | Yield-bearing stablecoins and DeFi credit markets need credible rates, but existing data products are either price feeds, dashboards, or exchange-specific derivatives rather than benchmark administration. |
| MVP | Public dashboard showing constituent yields, supply weights, exclusions, daily fixing, methodology preview, source freshness, and confidence flags. |
| Why now | Enterprise AI budgets exist, but true agent production is scarce, so a credible vertical workload helps distinguish Nunchi from generic infrastructure ([Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). |
| Korai/Roko primitive | Agent-attested data fetches, methodology-as-code, proof receipts, source provenance, benchmark audit trail. |
| 30-day validation | Working dashboard with 5 to 10 constituents, transparent methodology, direct source links, stale-data warnings, and a daily fixing artifact. |
| 90-day validation | Backtest, methodology paper, named methodology reviewers, and first serious distribution conversation. |

This should be the first shipped artifact because it is the competency gate. It shows that Nunchi can do real market infrastructure, not only research. It also prevents the company from having to sell the full agent coordination vision before there is a concrete workload.

The key product detail is to avoid pretending the first dashboard is already a regulated benchmark. The initial release should be explicit: methodology preview, research rate, not authorized benchmark. That honesty increases credibility.

### P0: Browser-verified agent activity and proof-of-work-done demo

| Dimension | Detail |
| --- | --- |
| Customer | Investors, developers, protocol partners, early technical buyers |
| Problem | Agent systems produce logs, but logs are not proofs. Developers need an intuitive way to see that agent work can be verified without trusting a hosted dashboard. |
| MVP | A public URL where a visitor’s browser verifies a stream of agent job events: claim, commit, execute, result, receipt, reputation update. |
| Why now | A2A creates standard agent task exchange, but it does not by itself settle proof of execution, payment, reputation, or dispute ([Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). |
| Korai/Roko primitive | Browser light client, Merkle-verified event stream, proof-of-work-done receipt, agent reputation delta. |
| 30-day validation | “Open this URL. Your browser verifies the agent did the work.” |
| 90-day validation | Demo supports at least three job types: benchmark data fetch, coding task, and paid API call. |

This is the single most legible demo. Most blockchain demos ask the viewer to understand infrastructure. This one makes the viewer the verifier. The story is not “faster chain.” The story is “agent work you can check from the browser.”

### P0: Compact Korai/Roko coordination spec

| Dimension | Detail |
| --- | --- |
| Customer | Protocol partners, developer tool builders, security reviewers, research collaborators |
| Problem | Without a compact spec, the system sounds like a bundle of ideas rather than an interoperable protocol. |
| MVP | Under 50 pages, covering message envelope, principal-binding, delegate-binding, scoped permissions, typed extensions, proof receipts, cost headers, eval gates, and settlement hooks. |
| Why now | MCP and A2A have established developer expectations for open standards, SDKs, and reference implementations ([Anthropic](https://www.anthropic.com/news/model-context-protocol), [Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). |
| Korai/Roko primitive | Principal/delegate distinction, cost budget, proof hash, HDC fingerprint, policy outcome, extension mechanism. |
| 30-day validation | Spec v0.1 plus TypeScript and Python message validators. |
| 90-day validation | Three external agents or tools can emit compatible receipts. |

The spec should not try to replace MCP or A2A. It should explicitly import them. The spec should say: MCP describes tool access, A2A describes agent collaboration, Korai/Roko describes accountable execution and clearing.

Minimum envelope:

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

### P0: Cost governance runtime and CPCA demo

| Dimension | Detail |
| --- | --- |
| Customer | AI platform teams, engineering leaders, finance operations, model gateway teams |
| Problem | Agent systems multiply model calls, tool calls, retries, and hidden loops, but teams still budget them like chatbots. |
| MVP | Run the same benchmark task through baseline agent, LangGraph or comparable framework, and Roko-governed execution. Show cost, success, retries, and cost per correct answer live. |
| Why now | McKinsey’s data shows scaling is still limited, while Menlo’s data shows real enterprise spend, which means buyers are now looking for controls rather than demos ([McKinsey](https://www.mckinsey.com/capabilities/quantumblack/our-insights/the-state-of-ai), [Menlo Ventures](https://menlovc.com/perspective/2025-the-state-of-generative-ai-in-the-enterprise/)). |
| Korai/Roko primitive | Cost-per-correct-answer, model routing, budget caps, retry discipline, loop breaker, per-task bill of materials. |
| 30-day validation | 20 to 50 repeatable tasks with visible budget enforcement and no hidden manual rescue. |
| 90-day validation | Demonstrate cheaper equal-quality execution on a credible benchmark without relying on unsupported model substitution claims. |

This is likely the best enterprise wedge after ISFR. It has a buyer, a measurable ROI story, and a clear pain. It also avoids speculative claims about future agent markets.

The metric should be CPCA: cost per correct answer. CPCA is better than raw token cost because cheap wrong answers are not useful. It is also better than pass rate alone because expensive accuracy may not survive production budgets.

### P1: Agent credential and payment manifest

| Dimension | Detail |
| --- | --- |
| Customer | Security teams, API providers, regulated workflow owners, agent marketplaces |
| Problem | Agents increasingly need to call tools, pay APIs, and act for users, but credentials, permissions, budgets, and approval rules are fragmented. |
| MVP | A manifest that binds principal, delegate, tool scopes, time window, spending budget, approval thresholds, payment rails, and revocation. |
| Why now | x402 V2 added wallet-based identity, reusable access sessions, service discovery, subscriptions, prepaid access, usage-based billing, and multi-step workflow support ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). |
| Korai/Roko primitive | Principal-binding, delegate-binding, scoped credentials, x402-compatible spend envelopes, revocation receipts. |
| 30-day validation | Manifest enforced against one MCP tool and one paid API endpoint. |
| 90-day validation | Manifest supports reusable sessions, budget exhaustion behavior, revocation, and audit export. |

This is a strong build because it turns security and payments into one product primitive. The important design choice is to keep it payment-rail agnostic. The manifest can support x402, AP2, card-based merchant authorization, internal credits, or free tool calls. The core asset is not the rail. It is the policy envelope.

### P1: Agent-attested ISFR data pipeline

| Dimension | Detail |
| --- | --- |
| Customer | Benchmark users, stablecoin issuers, on-chain protocols, risk teams |
| Problem | Benchmarks need trusted data collection, but manual data operations are expensive and opaque while generic oracle feeds do not satisfy benchmark governance. |
| MVP | Roko agent fetches source data, computes constituent yield, signs a receipt, posts hash and metadata, and produces a human-readable audit row. |
| Why now | MCP standardizes access to tools and systems, while A2A standardizes agent collaboration, but neither provides benchmark methodology, audit, or economic accountability ([Anthropic](https://www.anthropic.com/news/model-context-protocol), [Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). |
| Korai/Roko primitive | Agent-attested source fetches, methodology-as-code, proof receipts, anomaly flags, disagreement events. |
| 30-day validation | One protocol source end to end. |
| 90-day validation | Five or more sources, replayable methodology, disagreement handling, and daily fixing trace. |

This is the flywheel hinge. It proves that the agent runtime makes benchmark operations cheaper and more auditable, while the benchmark gives the agent runtime a serious production workload. It also anchors the story in something more credible than generic agent automation.

### P1: Inline eval gates and work settlement

| Dimension | Detail |
| --- | --- |
| Customer | AI platform teams, regulated workflow owners, benchmark operators |
| Problem | Most evals happen after the fact. Production workflows need gates before an output is accepted, passed downstream, paid, or used in a benchmark. |
| MVP | Configurable gate that evaluates output against freshness, schema, policy, accuracy proxy, budget, and source requirements before emitting a receipt. |
| Why now | LangSmith already offers online evals, trace analysis, human annotation, and deployment capabilities, so differentiation requires evals to become execution gates, not only observability artifacts ([LangSmith](https://www.langchain.com/langsmith-platform)). |
| Korai/Roko primitive | Runtime-native eval gate, policy outcome hash, receipt status, retry or escalation rule. |
| 30-day validation | Three gate types: schema gate, freshness gate, budget gate. |
| 90-day validation | Failed gates trigger automatic retry, cheaper-model fallback, human escalation, or dispute state. |

This feature should be built with humility. LangSmith and others are strong in evals. The unique angle is not “better eval dashboards.” It is “no work clears until policy and eval gates pass.”

### P1: 1,000-agent command surface

| Dimension | Detail |
| --- | --- |
| Customer | Developers, demo audiences, operations teams, protocol partners |
| Problem | Multi-agent work is hard to visualize and control. Chat surfaces are familiar, but ordinary chat cannot safely command verified agent fleets. |
| MVP | A command room where agents have identity, role, scope, budget, status, and receipts. Operator can address subsets by role, reputation, source, or task. |
| Why now | A2A supports long-running tasks, agent messages, artifacts, and collaboration, but operators still need control surfaces for real fleets ([Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). |
| Korai/Roko primitive | Agent identity, role scope, proof receipts, reputation, kill switch, state streaming. |
| 30-day validation | 100 to 250 simulated agents with real identity and receipt semantics. |
| 90-day validation | 1,000 agents executing benchmark recompute, coding review, and paid data-call workflows. |

This is a demo, not the company. Build it to make the coordination plane legible. Do not market it as “Slack for agents” unless the proof and permission story is visible in the UI.

### P2: ZK-HDC proof-of-learning and reputation

| Dimension | Detail |
| --- | --- |
| Customer | Future marketplaces, privacy-sensitive data owners, research partners, high-assurance agent users |
| Problem | Agent reputation is currently a claim or rating. The deeper moat is reputation tied to what an agent has done, learned, and proven without exposing sensitive inputs. |
| MVP | Small proof-of-learning demo: agent ingests a bounded dataset, emits a receipt, and later uses the learned artifact in a task while preserving provenance. |
| Why now | OWASP’s agentic security work shows that memory poisoning, identity abuse, privilege escalation, and cascading failures are core risks, so verifiable learning and work history can become safety infrastructure rather than vanity reputation ([OWASP](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)). |
| Korai/Roko primitive | HDC fingerprints, proof-of-learning, proof-of-work-done, continuous reputation. |
| 30-day validation | Pick the demo dataset and proving fallback. |
| 90-day validation | End-to-end proof on a toy but understandable task. |

This is a moat path, not an immediate GTM wedge. It should be kept in the investor narrative as the reason Korai can become a trust substrate, but it should not block the live benchmark or cost-governance demos.

### P2: DKG-private collaboration

| Dimension | Detail |
| --- | --- |
| Customer | Robotics labs, financial institutions, benchmark contributors, competitive data owners |
| Problem | Many valuable agent collaborations require parties to combine information without revealing raw data to each other. |
| MVP | Two parties run agents that jointly compute an output over private inputs and publish only proof, receipt, or aggregate. |
| Why now | As agents gain tool access through MCP and collaborate through A2A, the next unsolved layer is not just communication but privacy-preserving shared work ([Anthropic](https://www.anthropic.com/news/model-context-protocol), [Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)). |
| Korai/Roko primitive | Threshold collaboration, proof receipts, private work market, source-separated attestation. |
| 30-day validation | Design note and one use-case choice. |
| 90-day validation | Minimal two-party demo with synthetic data. |

This should be sequenced after the live proof-of-work and benchmark pipeline. It is strategically powerful but harder to explain and slower to sell.

## Build portfolio scorecard

| Rank | Build | Urgency | Market pull | Differentiation | Demo clarity | Overall |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | ISFR-YBS live dashboard | Very high | High | Medium-high | High | Must ship first |
| 2 | Browser-verified proof-of-work-done | Very high | Medium | Very high | Very high | Best investor demo |
| 3 | Korai/Roko coordination spec | Very high | Medium | High | Medium | Credibility gate |
| 4 | Cost governance runtime | High | Very high | High | High | Best enterprise wedge |
| 5 | Agent credential and payment manifest | High | High | High | Medium | Security and payments wedge |
| 6 | Agent-attested ISFR pipeline | High | Medium-high | Very high | High | Flywheel hinge |
| 7 | Inline eval gates and settlement | Medium-high | High | High | Medium | Differentiates from observability |
| 8 | 1,000-agent command surface | Medium | Medium | Medium-high | Very high | Visual proof of coordination |
| 9 | ZK-HDC proof-of-learning | Medium | Low-medium today | Very high | Medium | Long-term trust moat |
| 10 | DKG-private collaboration | Medium | Medium in regulated verticals | Very high | Low-medium | Later vertical wedge |

## Roadmap

### Days 0 to 30

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Live ISFR-YBS dashboard | Shows constituent yields, source freshness, weights, exclusions, and daily fixing. Labels product honestly as research or methodology preview. |
| Proof | Browser-verified agent activity page | Visitor can verify at least one job receipt without trusting a hosted dashboard. |
| Spec | Korai/Roko v0.1 coordination spec | Defines envelope, principal, delegate, scope, budget, proof, eval, settlement, and extension fields. |
| Cost | CPCA benchmark demo | Same task suite, visible budget, retries, success rate, and cost-per-correct-answer. |
| DevEx | Seven-line or sub-five-minute starter | Developer can run a toy agent, emit a receipt, and inspect the proof. |

### Days 31 to 60

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Methodology-as-code paper draft | Methodology is deterministic, versioned, hashable, replayable, and readable by benchmark users. |
| Data | Agent-attested source pipeline | At least five source fetches produce signed receipts, freshness checks, and disagreement handling. |
| Security | Agent credential and payment manifest | Manifest enforces tool scope, time bound, spending cap, revocation, and approval threshold. |
| Evals | Inline gates | Failed freshness, schema, or budget gate prevents settlement. |
| Partners | Design partner list | At least 10 serious conversations across DeFi, data, agent tooling, security, and devtools. |

### Days 61 to 90

| Track | Ship | Acceptance criteria |
| --- | --- | --- |
| ISFR | Backtest and public methodology v0.2 | Includes historical reconstruction, stress windows, exclusions, and limitations. |
| Agents | 1,000-agent command surface | Agents have identity, role, scope, budget, receipt status, and kill switch. |
| Proof | Receipt leaderboard | Shows agent work count, success, corrections, gate failures, and reputation deltas. |
| Enterprise | Cost-governance pilot | One external team runs a task suite under Roko budget controls. |
| Regulatory | Benchmark readiness memo | Clear separation between research rate, production index, and regulated benchmark pathway. |

## Product requirements for the first serious wedge

### Problem statement

Teams deploying agents do not lack models. They lack accountable execution. The same agent system can accidentally exceed budget, reuse stale data, call the wrong tool, loop indefinitely, silently fail, act under unclear authority, or produce a result that cannot be audited. The problem becomes more acute as MCP and A2A make agents more connected: connectivity increases the blast radius of identity, permission, and cascading-failure errors.

### Goals

- Reduce time from agent prototype to auditable workflow by giving teams a default envelope for identity, budget, proof, and policy.
- Make cost per correct answer visible for every agent workflow.
- Prevent work from clearing when source, schema, policy, freshness, or budget gates fail.
- Make benchmark data operations replayable and auditable from source fetch to daily fixing.
- Demonstrate one user-facing proof moment that no generic framework or oracle can copy quickly.

### Non-goals

- Do not build a general-purpose LLM framework.
- Do not build a generic DeFi oracle network.
- Do not build a new payment protocol when x402, AP2, and ACP are still evolving.
- Do not make regulated benchmark claims before governance, authorization, and methodology work justify them.
- Do not make ZK-HDC a dependency for the first dashboard or cost-governance wedge.

### User stories

| Persona | Story | Priority |
| --- | --- | --- |
| Benchmark operator | As a benchmark operator, I want each data point to have a source, method, timestamp, and agent receipt so that I can explain and audit the fixing. | P0 |
| AI platform lead | As an AI platform lead, I want every agent workflow to have a budget, retry rule, and cost-per-correct-answer metric so that I can scale without surprise spend. | P0 |
| Security lead | As a security lead, I want a manifest binding user, agent, tool scopes, spend, and approvals so that agents cannot inherit broad credentials. | P1 |
| Developer | As a developer, I want to run a minimal agent and emit a verifiable work receipt in minutes so that I can understand the system without reading a long whitepaper. | P0 |
| Institutional buyer | As a benchmark user, I want methodology and backtest transparency so that I can evaluate whether the rate is credible. | P1 |
| Marketplace participant | As an agent provider, I want successful work to update reputation automatically so that buyers can trust receipts rather than claims. | P2 |

### P0 acceptance criteria

| Requirement | Acceptance criteria |
| --- | --- |
| Agent work receipt | Given an agent completes a task, when the task clears, then the system emits a receipt containing principal, delegate, task, scope, input hash, output hash, policy status, eval status, budget usage, and proof hash. |
| Budget cap | Given an agent has a budget, when cumulative execution cost exceeds the cap, then the runtime halts, escalates, or reroutes according to policy before additional paid calls. |
| ISFR source freshness | Given a constituent source is stale, when the dashboard computes a fixing, then the source is flagged and the methodology explains inclusion, exclusion, or fallback. |
| Browser verification | Given a user opens the proof page, when receipts stream, then the browser verifies event inclusion without requiring a trusted RPC assumption in the demo path. |
| Destructive action gate | Given an action is classified as destructive, when an agent attempts it without an explicit grant, then execution is blocked and a policy-failure receipt is emitted. |

### Success metrics

| Metric | Target | Why it matters |
| --- | --- | --- |
| Time to first receipt | Under 5 minutes in starter flow | Developer activation |
| Dashboard source coverage | 5 to 10 constituents in first month | Benchmark credibility |
| Receipt completeness | 100% of cleared tasks include required envelope fields | Auditability |
| Budget enforcement | 0 unbounded loops in benchmark task suite | Enterprise trust |
| CPCA improvement | Demonstrable improvement against baseline on fixed task set | ROI |
| Gate precision | Low false pass rate on freshness, schema, and budget tests | Safety |
| External validation | At least 3 credible reviewers for methodology or spec | Market trust |

## Moat map

### Near-term moats

| Moat | Why it matters | How to build it now |
| --- | --- | --- |
| Live benchmark workload | Prevents generic infrastructure narrative. | Ship ISFR-YBS dashboard and methodology preview. |
| Browser-verifiable proof moment | Makes the chain’s purpose instantly legible. | Build proof-of-work-done demo as public URL. |
| Cost-per-correct-answer data | Gives enterprise buyer a budget reason to care. | Run fixed task suites and publish honest numbers. |
| Principal/delegate spec | Makes agent accountability concrete. | Publish compact spec and validators. |
| Agent-attested benchmark trace | Shows runtime and benchmark flywheel. | End-to-end source fetch to receipt to fixing. |

### Medium-term moats

| Moat | Why it matters | Build path |
| --- | --- | --- |
| Work receipt graph | Accumulates agent performance data that competitors cannot fork from code. | Every task emits receipts; every receipt updates reputation. |
| Methodology-as-code | Turns benchmark ops into software and audit artifacts. | Version, hash, replay, and compare methodologies. |
| Permission and payment manifests | Makes agent work safe enough for enterprises. | Bind identity, scopes, budget, approvals, and revocation. |
| Inline eval settlement | Distinguishes execution gate from observability dashboard. | Work clears only after policy and quality gates pass. |

### Long-term moats

| Moat | Why it matters | Build path |
| --- | --- | --- |
| ZK-HDC reputation | Reputation becomes proof-backed rather than claim-backed. | Start with small proof-of-learning demo, then extend. |
| Privacy-preserving collaboration | Unlocks regulated data markets and competitive collaboration. | DKG or threshold collaboration over synthetic private datasets first. |
| Agent benchmark administrator | Combines regulated benchmark credibility with agent-native operations. | Move from research rate to governance, oversight, and authorization pathway. |
| Agent trust registry | Work receipts compound into marketplace trust. | Only launch marketplace after enough receipts exist. |

## Risk register

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Overclaiming benchmark status | Institutional buyers punish sloppy governance claims. | Call early product a research rate or methodology preview until authorization path is real. |
| Competing head-on with LangSmith or Temporal | Both already have credible runtime and observability positioning. | Integrate where useful. Differentiate on proof, identity, budget, settlement, and benchmark workloads. |
| Becoming an x402 derivative | x402 is promising but activity quality is still evolving, and Galaxy notes early speculative usage ([Galaxy Research](https://www.galaxy.com/insights/research/x402-ai-agents-crypto-payments)). | Be payment-rail agnostic. Support x402-compatible flows without depending on x402 as the category. |
| Protocol sprawl | Developers will not adopt a spec that ignores MCP and A2A. | Treat MCP and A2A as imports. Keep Korai/Roko focused on clearing semantics. |
| Security blind spot | OWASP’s agentic risks include identity abuse, tool misuse, memory poisoning, privilege escalation, and cascading failures ([OWASP](https://genai.owasp.org/2025/12/09/owasp-genai-security-project-releases-top-10-risks-and-mitigations-for-agentic-ai-security/)). | Make scoped credentials, destructive-action gates, revocation, and audit receipts P0 or P1. |
| ZK dependency risk | Proving systems may not be ready for full HDC workflows. | Keep ZK-HDC as P2, not a dependency for ISFR or cost governance. |
| Naming confusion | Roko, Korai, Daeji, and protocol naming can fragment the story. | Lock external naming before public launch: Nunchi company, Korai primary product, Roko runtime. |
| Generic chain positioning | L1 buyers will evaluate against liquidity, bridges, throughput, and ecosystem. | Lead with agent-native work receipts and benchmark clearing, not TPS. |
| Marketplace too early | Empty marketplaces look weak and invite spam. | Build proof receipts and trust graph before launching marketplace. |
| Unsupported traction claims | Fabricated or weak market numbers damage credibility. | Use only sourced public evidence and honest internal milestone language. |

## Recommended first narrative

The first public narrative should be concrete and sequential:

1. Enterprise AI has budget, but real agents are not scaling.
2. MCP and A2A solved connectivity, not accountability.
3. Production agents need clearing: identity, scope, budget, proof, eval, payment, and reputation.
4. Korai is the coordination chain for accountable agent work.
5. Roko is the runtime that makes agents cost-aware, stateful, and auditable.
6. ISFR-YBS is the first workload proving this is not theory.
7. The first demo is browser-verifiable: the viewer can check agent work directly.

This narrative keeps the product out of crowded categories and lets every build artifact reinforce the same claim. The dashboard proves market competence. The proof page proves chain necessity. The spec proves interoperability. The cost demo proves enterprise ROI. The credential manifest proves security seriousness. The agent-attested pipeline proves the benchmark-agent flywheel.

## Final recommendation

Build the first 90 days around one sentence: agent work should clear like financial work. That means it has identity, authorization, cost, source, methodology, proof, acceptance, and settlement. Korai should be the substrate where that clearing happens. Roko should be the runtime that produces the work and receipts. ISFR-YBS should be the first market where the entire system matters.

The highest-leverage thing to build first is not the most technically ambitious primitive. It is the smallest complete loop:

1. An ISFR-YBS data point is fetched by a Roko agent.
2. The agent acts under a scoped principal/delegate manifest.
3. The runtime enforces budget and freshness gates.
4. The output receives a proof-of-work-done receipt.
5. The receipt is visible and browser-verifiable on Korai.
6. The methodology preview explains how that data point contributes to a fixing.

That loop is enough to show the whole company in miniature. It is a benchmark, an agent runtime, a chain, a proof system, and a market opportunity. Everything else should compound from there.
