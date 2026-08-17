# tiagent: Bringing Every Developer to Celestia Through AI

**tiagent is an open-source, self-improving coding agent that uses Celestia's DA layer as a shared learning backbone --- turning every developer who uses it into a Celestia DA consumer, whether they know it or not.**

---

## The Elevator Pitch

tiagent is a coding agent --- like Claude Code, Codex, or Cursor --- but open-source, model-agnostic, and fundamentally different in one way: it gets better every time you use it.

Today's coding agents are stateless. Run 1 and run 1,000 behave identically. They don't remember what worked. They don't learn which models handle which tasks best. They don't extract reusable strategies from successful runs. Every session starts from zero.

tiagent changes this. A cascade router learns which LLM backend (Claude, GPT, Gemini, Ollama, anything) handles which task type best. Adaptive gates tune quality thresholds based on real outcomes. Playbooks capture winning strategies and replay them. Efficiency tracking identifies what's slow and reroutes around it. This is not prompt engineering --- it is a structural feedback loop baked into the runtime.

But the real breakthrough is collective learning. Through optional Celestia DA integration, tiagent instances share what they learn --- routing weights, efficiency patterns, behavioral fingerprints, successful strategies --- so the entire network of agents gets smarter, not just yours. Every developer running tiagent publishes learning artifacts as Celestia blobs. The network improves with every task executed, everywhere.

This creates a growth flywheel: developers adopt tiagent because it is a better coding agent. tiagent pushes data to Celestia. DA usage grows. The ecosystem grows. And the agent gets smarter from the collective, which attracts more developers.

---

## Why This Matters for Celestia

### The narrative gap is real

0G Labs has claimed the "AI DA" position with $108M+ in AI-specific ecosystem funding. They have a dedicated AI grants program, an Apollo AI Accelerator, and explicit branding as "The AI Blockchain."

Celestia has invested **$0** in AI-specific grants. Zero dedicated AI programs. No AI narrative at all.

Yet Celestia has objectively superior infrastructure for AI workloads:

| Capability | Celestia | 0G Labs |
|---|---|---|
| Block size | 128 MB (post-Matcha), scaling to 1 Tb/s | 50 MB max (early testnet) |
| Network maturity | Production mainnet, 56+ rollups | Early-stage testnet |
| Light node verification | DAS with production lumina-node | Not yet available |
| Data partitioning | 29-byte namespace system, production-ready | Custom sharding (not yet stable) |
| Economic activity | Real transaction volume, established ecosystem | Pre-launch |

This is a first-mover problem disguised as a marketing problem. The ecosystem that builds the first credible AI agent framework on a DA layer claims the narrative. tiagent is that framework.

### What Celestia gets

**A headline.** "Celestia powers the world's first collectively intelligent coding agent." That is a conference keynote. A viral tweet. A narrative weapon against every ecosystem claiming the AI crown with vaporware.

**Non-blockchain users.** Developers using tiagent do not need to understand Celestia, TIA, DA, or blobs. They install a coding agent. They write code. The Celestia integration is invisible infrastructure --- the same way most developers don't know their app uses S3 under the hood. This is how you bring millions of developers into the Celestia ecosystem without asking them to learn a single blockchain concept.

**Real DA consumption.** Agent traces, embeddings, routing weights, and behavioral fingerprints are published as Celestia blobs. At 1,000 active agents: 50 MB--5 GB of daily DA consumption. At 10,000 agents: 500 MB--50 GB. This is meaningful blob revenue --- the kind of organic, recurring usage that rollups provide today, but from an entirely new consumer category.

**Narrative positioning.** Celestia is not just for rollups. It is the modular backbone for AI agent infrastructure. tiagent proves this thesis with working software, not a whitepaper.

---

## The Growth Flywheel

```
    Developers adopt tiagent
    (better coding agent: self-improving, model-agnostic, open-source)
                |
                v
    tiagent publishes learning artifacts to Celestia DA
    (traces, routing weights, embeddings, fingerprints)
                |
                v
    DA usage grows
    (more blobs, more fees, more network value)
                |
                v
    More data --> tiagent gets collectively smarter
    (trajectory RAG, shared routing, cross-agent playbooks)
                |
                v
    Smarter agent --> more developers adopt
    (network effects make tiagent better than single-tenant alternatives)
                |
                v
    +----- Network effects compound exponentially -----+
    |                                                   |
    +------ loops back to top, cycle accelerates -------+
```

The key insight: developers do not adopt tiagent because of Celestia. They adopt it because it is a better coding agent. Celestia integration is the invisible engine that makes the collective intelligence possible --- and every new user strengthens the network for everyone else.

---

## What We're Building

### A production-grade coding agent harness

tiagent is approximately 14 Rust crates, open-source under MIT/Apache-2.0 dual license. It is not a toy, a demo, or a wrapper around an API. It is a full agent runtime:

**Model-agnostic execution.** Use Claude, GPT, Gemini, Llama, Mistral, Ollama, or any OpenAI-compatible API. Switch providers without changing your workflow. Route different task types to different models automatically.

**MCP-compatible.** tiagent speaks the Model Context Protocol (97M+ monthly SDK downloads), implementing both client and server. It plugs into the existing MCP ecosystem of tools, data sources, and integrations out of the box.

**Self-improving runtime.** Four feedback mechanisms work in concert:
- *Cascade router* --- learns which model handles which task type best
- *Adaptive gates* --- tune quality thresholds (compile, test, lint, diff) based on real outcomes
- *Playbook extraction* --- captures successful tool-call sequences and replays them
- *Efficiency tracking* --- identifies bottlenecks and reroutes around them
- *Knowledge demurrage* --- entries pay a continuous holding tax; unused knowledge fades, actively validated knowledge strengthens through four tiers

**Agent orchestration.** Execute multi-step plans as DAGs with parallel dispatch, dependency ordering, and automated quality gates at every step. PRD-to-code workflows: write a spec, generate a plan, execute it autonomously.

**Collectively improving.** With optional Celestia DA integration, all of the above learning artifacts are published as blobs in organized namespaces. New agents bootstrap from the network's collective experience. Trajectory RAG lets agents ask "how did another agent solve a similar task?" and use that as in-context learning.

**Celestia-native developer tools.** Blob management, namespace tooling, rollup deployment assistance, and ecosystem-specific MCP tools --- making tiagent immediately useful for Celestia developers specifically, not just developers in general.

---

## What We're Asking For

**$200,000 over 12 months** from the Celestia Foundation, structured across 6 milestones with concrete, verifiable deliverables at each stage.

Every milestone produces working, open-source software --- not research papers, not prototypes, not "further design." Code ships at every checkpoint.

Full milestone breakdown, budget allocation, and success criteria are detailed in the grant proposal (document 08).

---

## Document Index

This overview is one of 12 documents in the tiagent v2 specification suite. Each document is self-contained but builds on the ones before it.

| # | Document | Description |
|---|----------|-------------|
| **00** | **00-OVERVIEW.md** (this document) | Entry point, elevator pitch, and strategic case for Celestia. |
| **01** | [**Why Celestia Needs This**](https://gist.github.com/wpank/eb71d014a219883fcbf593cc5e68ad49) | Strategic case: AI narrative gap, marketing value, non-blockchain developer acquisition, exponential returns. |
| **02** | [**Product Vision**](https://gist.github.com/wpank/fc5147b3ff4325bfc6dcd2c4f7273f7f) | What tiagent is, how developers use it, how it gets better, comparison with Claude Code/Codex/Cursor. |
| **03** | [**Technical Architecture**](https://gist.github.com/wpank/fd2d8ead683e8dce31ad76135741700f) | Signal model, universal loop, 6 verb traits, layered architecture, crate structure. |
| **04** | [**Celestia Integration Design**](https://gist.github.com/wpank/70c6fbe1b176261c6603edd83471b78f) | DA substrate, namespace schema, light node embedding, cost model, blob wire format. |
| **05** | [**Network Effects, Growth, and Scale**](https://gist.github.com/wpank/7799c1904650b546666996f672fc0fed) | Growth flywheel, scaling economics, defensibility, marketing multiplier, ROI analysis. |
| **06** | [**Ecosystem Impact**](https://gist.github.com/wpank/a734e04a366b225041d91ea6352c2388) | How specific Celestia projects benefit: Sovereign SDK, Eclipse, Astria, OnchainDB, Flame, Neutron, and more. |
| **07** | [**Competitive Landscape**](https://gist.github.com/wpank/c9f1546998612b426104b5704344760e) | Two-front analysis: vs coding agents (Claude Code, Codex, Cursor) and vs on-chain frameworks (ElizaOS, IronClaw, etc). |
| **08** | [**Grant Proposal**](https://gist.github.com/wpank/939bac9200491c63aa673f06e8c42f4b) | Celestia Foundation Strategic Ecosystem Grant: $200K, 12 months, 6 milestones, full budget and deliverables. |
| **09** | [**Technical Appendix**](https://gist.github.com/wpank/7b625b3937c98baff0f7389f9ba4f3c9) | Rust type definitions, crate structure, CLI reference, configuration schema, Celestia API surface. |
| **10** | [**Research References**](https://gist.github.com/wpank/f828e912028efa855a97ff9542ec1e65) | Papers, standards, market data, and prior art supporting tiagent's technical claims. |
| **11** | [**DA Feasibility Assessment**](https://gist.github.com/wpank/80226d2e575db01832e16abe1ab06aa0) | Real cost analysis: TraceCommons case study, measured roko artifact sizes, cost projections at scale. |

---

## Key Numbers

| Metric | Value |
|---|---|
| AI agent market size (2026) | $22.6--27B |
| MCP monthly SDK downloads | 97M+ |
| Celestia DA cost per MB | $0.07--$0.81 |
| 0G Labs AI ecosystem funding | $108M+ |
| Celestia AI-specific funding | $0 |
| Target crate count | ~14 |
| Competing coding agents with self-improvement | 0 |
| Competing coding agents with shared learning | 0 |
| Daily DA consumption (1K agents) | 50 MB--5 GB |
| Daily DA consumption (10K agents) | 500 MB--50 GB |
| Grant ask | $200K / 12 months |
| Milestones | 6, each with shipping deliverables |
| Knowledge half-lives | 1 hour (warnings) to 150 days (persistent heuristics) |

---

*tiagent is the Trojan horse. Developers get a better coding agent. Celestia gets a new category of DA consumers. The network gets smarter with every task executed. Everyone wins.*
