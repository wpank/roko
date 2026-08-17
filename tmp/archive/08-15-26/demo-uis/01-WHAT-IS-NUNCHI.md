# What Is Nunchi — The Big Picture

## One Sentence

Nunchi is the **Agent Coordination Plane** — the infrastructure layer that sits between AI models and the applications they power, solving the coordination problem that causes 41–86% of production agent deployments to fail.

## The Thesis

> *The model is commoditizing. The scaffold is the moat.*

Frontier model capability per dollar trends down. GPT-3.5-class capability fell from $20.00/M tokens (Nov 2022) to $0.04 (Apr 2026) — a 500x collapse in 41 months. Switching costs between model providers approach zero. The variable that separates a production agent from a prototype is no longer the model — it is the **system around the model**: identity, memory, calibration, policy, settlement, audit.

Nunchi builds that system.

## Two Components, One Architecture

Nunchi is two things that work together:

### Roko — The Cognitive Runtime

An open-source Rust runtime (Apache 2.0) that turns a model call into a governed event. 18 crates, ~177K lines of code. Functional and self-hosting today.

**What it does:** Plan, route, gate, persist, replay. Every agent run — whether a research session, a code refactor, or a multi-step trade — passes through the same deterministic 8-step loop. The model is swappable. The loop is not.

**What it ships with:**

- **CascadeRouter** — cost-aware model routing across tiers (Haiku → Sonnet → Opus), with confidence-bid promotion. Each rung carries a measured confidence interval from prior calls; the router bids the lowest tier whose interval covers the task
- **Gate pipeline** — 11 verifiers, 7 rungs between every model call and every side effect. Schema, PII, cost, latency, policy, jurisdiction, idempotent, provenance, budget, consent, audit
- **EpisodeLogger** — deterministic replay across non-deterministic LLMs. Every tool call, model call, decision, and result captured with structured metadata
- **NeuroStore** — HDC-indexed knowledge store that persists across runs. 10,000-dimensional binary vectors, sub-millisecond similarity search. The thousandth agent learns from the first
- **SystemPromptBuilder** — 9-layer prompt assembly with role templates, context bidding, and playbook injection
- **Predict-publish-correct** — every Cell predicts outcomes before acting, records predictions and errors, enabling continuous learning from residuals

### Korai — The Verifiable Substrate

An agent-native sovereign EVM L1 blockchain where Roko episodes settle. Where local memory becomes shared memory across organizations. Where reputation is an attestation, not a marketing claim.

**What it does:** Provides the settlement layer for cross-organization agent coordination — identity, attestation, clearing — with cryptographic guarantees no runtime alone can offer.

**What it ships with:**

- **ERC-8004** — transferable cryptographic agent identities with SPIFFE SVIDs, anchored on chain. 7-domain EMA reputation that decays over time
- **HDC precompile** at address 0xA01 — native hyperdimensional computing at consensus layer, ~~400 gas for top-K similarity search (~~170μs at 100K vectors)
- **ZK-HDC proofs** — prove behavioral fingerprints without revealing them (Circom + Groth16, <1s proving, ~250K gas verify)
- **Simplex consensus** — BFT-family with ~50ms blocks, seconds-level finality, co-located Tokyo validators
- **Cooperative clearing** — KKT-verified payout proofs for multi-tenant agent work settlement without a trusted clearinghouse
- **ISFR** — Internet Secured Funding Rate, a composite DeFi benchmark computed by validators every 10 seconds

**The relationship:** Roko runs alone. Korai runs alone. Together, local traces compound into shared memory, attestable reputation, and verifiable settlement across organizational trust boundaries.

---

## The Problem Nunchi Solves

### Production Agents Fail at Coordination, Not Intelligence

The Berkeley MAST taxonomy ran 1,642 multi-agent traces across seven popular frameworks. Results:


| Framework    | Failure Rate |
| ------------ | ------------ |
| MetaGPT      | 41%          |
| ChatDev      | 51%          |
| HyperAgent   | 57%          |
| OpenManus    | 63%          |
| AppWorld     | 71%          |
| Magentic-One | 78%          |
| AG2          | 86.7%        |


**79% of failures came from coordination, not capability.** 42% from system design, 37% from inter-agent misalignment, 21% from verification breakdowns. Not one was a model intelligence problem.

Additional research confirms this:

- **Princeton NLP:** Solo agents beat multi-agent ensembles 64% of the time. Adding agents adds failure surface, not capability
- **Google DeepMind:** Beyond ~45% single-agent accuracy, more agents *hurt*. Coordination errors compound geometrically — naive multi-agent setups show 17.2x error amplification

### Four Named Failure Modes

1. **Routing (Unrouted):** A million inferences per second. Every framework picks one model and prays. No calibrated routing based on task difficulty and cost
2. **Locking (Unguarded):** Two agents pulling the same row. Two agents emailing the same customer. Distributed locking on a substrate never built for it
3. **State (Unremembered):** Session amnesia. Every conversation starts at zero. Every solved bug is solved again. The thousandth agent learns nothing from the first
4. **Policy (Unchecked):** SOC 2, FINRA, EU AI Act. The auditor wants attestation per call. "The model decided" is no longer an acceptable answer

### Real-World Reversals Confirm the Pattern

- **Klarna** (Feb→May 2024): Replaced 700 CS roles with AI. Reversed in 3 months. "Quality of human service was too important."
- **CBA** (Dec 2024→Aug 2025): Laid off 45 staff for AI voice bot. Reversed. Customers escalated to humans by default
- **Cursor** (Jul 2025): Coding agent leaked private repo content into public sessions through shared cache. Forum #134796
- **NYC MyCity** (Mar 2024): City chatbot told users to break the law — steal tips, refuse disabled customers. n=53 prompts
- **Devin** (May 2024→Apr 2026): First autonomous engineer. $2B raised at $9.8B. Internal benchmarks: 13.86% vs claimed; community reproduction near zero

> *These didn't fail at modeling. They failed at coordination, audit, and accountability — the layer no model provider sells.*

---

## The Category: The Empty Cell

Every adjacent layer of the agent stack is funded. The plane between them is empty.


| Layer                  | Players                                  | Capital         |
| ---------------------- | ---------------------------------------- | --------------- |
| Vertical Agents        | Cognition, Decagon, Sierra               | $2B+ valuations |
| Frameworks             | LangChain, LlamaIndex, CrewAI            | $30M Series A   |
| **COORDINATION PLANE** | **Cross-org probabilistic coordination** | **EMPTY**       |
| Durable Execution      | Temporal, Inngest, Orkes                 | $5B Series E    |
| Identity               | Keycard, Stytch, WorkOS                  | $38M Series A   |
| Payments               | x402, Skyfire                            | $50M+ volume    |
| Models                 | OpenAI, Anthropic, Google, DeepSeek      | $157B+          |


**Frameworks** orchestrate one team. **Durable execution** remembers within one process. **Identity** attests one principal. **Payments** settle one transaction. **Cross-organization probabilistic coordination** is unbuilt.

### The Architectural Precedent

In 2007, networks separated the data plane (forwarding) from the control plane (decisions about what to forward). Martin Casado's Ethane paper at Stanford became Software-Defined Networking. Nicira commercialized it. VMware acquired it for $1.26B.

Production agents need the same separation. Frameworks describe what an agent does. Models execute. The plane in the middle decides *which agent, which memory, which model, at what cost, under which policy.*

### The Stripe Parallel

Stripe was not the bank. It was the API to the bank. Seven lines of code, one charge. Banks held the money. Stripe held the relationship between the developer and the money.

Nunchi is not the model. It is the plane. Nine lines of code, one coordinated run:

```python
from nunchi import session
with session.open(
    agent="researcher@v2",
    identity="spiffe://acme/research",
    budget_usd=0.10,
    gates=["pii", "cost_ceiling", "sox"],
) as s:
    result = s.run("Summarize Q3 earnings")
```

Those nine lines produce: identity attestation, cost prediction, cascade routing, three gate passes, actual cost measurement, knowledge deposit to NeuroStore, and a tamper-evident audit record anchored on chain.

---

## The Regulatory Catalyst

### EU AI Act Article 50

Article 50 of Regulation (EU) 2024/1689 imposes binding transparency obligations on August 2, 2026. Penalties: EUR 15M or 3% of global turnover, whichever is higher. Higher tiers reach EUR 35M or 7%.

**Only 26.2% of EU enterprises have begun concrete compliance activity.** 73.8% have not.

What the law requires:

1. Transparency — disclose AI involvement
2. Risk classification — identify high-risk use
3. Logging — maintain technical documentation and event logs
4. Data quality — bias, representativeness, error handling
5. Human oversight — demonstrable intervention paths
6. Post-market monitoring — continuous performance and incident reporting

What Nunchi ships by default:

1. AI-involvement marker in every attestation envelope
2. Per-tenant risk-tier metadata bound to each call
3. Every call's hash, gates, and verdicts on chain in seconds
4. Source-attested inputs through tool-call gates
5. Gate 11 enforces opt-in human review on flagged paths
6. NeuroStore retains events; chain anchors tamper-evident

> *Vanta turned SOC 2 into $100M ARR. OneTrust turned GDPR into $5B+. Article 50 is the next compliance category. The audit trail is native, not bolted on.*

---

## Key Numbers


| Metric                       | Value                   | Source                         |
| ---------------------------- | ----------------------- | ------------------------------ |
| Multi-agent failure rate     | 41–86%                  | MAST, Berkeley, NeurIPS 2025   |
| Failures from coordination   | 79%                     | MAST taxonomy                  |
| Cost reduction (compound)    | 10–30x                  | HAL benchmark + Nunchi stack   |
| HAL baseline cost per task   | $42.11–$44.86           | Princeton TAU-bench            |
| Nunchi optimized cost        | $1.42                   | Full stack applied             |
| GPT-3.5 price collapse       | 500x in 41 months       | Public API pricing             |
| EU AI Act enforcement        | August 2, 2026          | Regulation (EU) 2024/1689      |
| EU penalty ceiling           | EUR 35M / 7% turnover   | Article 99                     |
| Machine:human identity ratio | 80:1 to 144:1           | CyberArk/Entro/Gartner 2025–26 |
| MCP SDK downloads            | 97M/month               | Anthropic/AAIF                 |
| ERC-8004 registrations       | 22.9K in 3 days         | Ethereum mainnet               |
| x402 payment volume          | $50M cumulative         | Coinbase + Cloudflare          |
| Roko codebase                | 177K LOC, 18 crates     | Internal                       |
| HDC vector dimension         | 10,000 bits             | Internal                       |
| HDC similarity search        | <1μs                    | Internal                       |
| On-chain HDC query           | ~400 gas                | Precompile spec                |
| ZK-HDC proof time            | <1s                     | Circom + Groth16               |
| Gateway cache hit rate       | 58%                     | Production tenant, Mar 2026    |
| NHI market size              | $10.71B→$25.65B by 2033 | Frost & Sullivan / Gartner     |


