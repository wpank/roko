# 07 -- Competitive Landscape

> tiagent competes on two fronts simultaneously: as a **coding agent** against
> Claude Code, Codex, Cursor, and others, and as an **on-chain agent framework**
> against ElizaOS, IronClaw, Solana Agent Kit, and others. No existing tool
> occupies both positions. This document maps both landscapes, identifies the
> structural gaps in each, and shows where tiagent sits relative to every
> relevant competitor.

---

## 1. Two Competitive Fronts

tiagent is not a coding agent that happens to touch the blockchain. It is not
a blockchain framework that happens to use AI. It is the first tool designed
from the ground up to be both -- a self-improving coding agent whose learning
state lives on a DA layer.

This dual positioning creates a unique competitive dynamic:

| Front              | Competitors                                           | tiagent's edge                                         |
|--------------------|-------------------------------------------------------|--------------------------------------------------------|
| **Coding agent**   | Claude Code, Codex CLI, Cursor, Windsurf, Aider       | Self-improvement, plan execution, shared learning      |
| **On-chain agent** | ElizaOS, IronClaw, Olas, Solana Agent Kit, AgentKit   | Coding-native, Celestia DA, model-agnostic routing     |
| **Both**           | Nobody                                                | tiagent                                                |

Every coding agent today is stateless across sessions: it cannot learn from
past successes, execute multi-step plans, or share knowledge between
instances. Every on-chain agent framework today is task-specific: it manages
wallets and submits transactions but cannot write code, improve itself, or
operate as a general development tool. tiagent bridges these two worlds.

---

## 2. Coding Agent Landscape

### 2.1 Feature Matrix

| Capability                  | Claude Code | Codex CLI | Cursor  | Windsurf | Aider   | Continue | tiagent   |
|-----------------------------|:-----------:|:---------:|:-------:|:--------:|:-------:|:--------:|:---------:|
| **Provider**                | Anthropic   | OpenAI    | Cursor  | Codeium  | OSS     | OSS      | OSS       |
| **Model lock-in**           | Claude only | GPT only  | Multi   | Multi    | Any     | Any      | Any       |
| **Self-improvement**        | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Plan execution**          | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Quality gates**           | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Shared learning**         | No          | No        | No      | No       | No      | No       | **Yes**   |
| **PRD-to-code pipeline**    | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Learned model routing**   | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Playbook extraction**     | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Persistent knowledge**    | No          | No        | No      | No       | No      | No       | **Yes**   |
| **Open source**             | Partial     | Yes       | No      | No       | Yes     | Yes      | **Yes**   |
| **Headless / CI-ready**     | Yes         | Yes       | No      | No       | Yes     | Partial  | **Yes**   |
| **On-chain integration**    | No          | No        | No      | No       | No      | No       | **Yes**   |

Every column in the bottom half of the table tells the same story: the coding
agent market has converged on a set of capabilities (code generation, file
editing, shell commands, multi-model support) and a set of blind spots
(learning, planning, quality enforcement, knowledge persistence). tiagent is
the only entry that addresses the blind spots.

### 2.2 Per-Agent Analysis

**Claude Code (Anthropic).** The current market leader in code quality. Best
tool-use integration, largest context window (200K tokens), MCP support for
extensibility. Locked to Claude models. No learning -- every session starts
from zero. No plan execution, no quality gates, no shared knowledge. A
developer using Claude Code on Monday has no advantage on Tuesday from what
the agent learned on Monday. Subscription model with usage caps.

**Codex CLI (OpenAI).** OpenAI's answer to Claude Code. Sandboxed execution
for safety. Locked to GPT/o-series models. Single-task oriented -- no plan
execution, no learning, no quality gates. Open-source CLI client but tied to
OpenAI's API. Newer and less mature than Claude Code.

**Cursor (Cursor Inc).** The most popular AI-assisted IDE by user count. Fork
of VS Code with deep agent integration. Multi-model support (Claude, GPT,
Gemini). IDE-locked -- cannot run headlessly, in CI, or as part of a
pipeline. Closed-source. No plan execution, no learning, no quality gates.

**Windsurf (Codeium).** Cursor's primary competitor. IDE-integrated agent with
"Cascade" multi-step reasoning. Same fundamental limitations: closed-source,
IDE-locked, no learning, no plan execution, no quality gates. Smaller
community than Cursor.

**Aider (open source).** The most capable open-source CLI coding agent.
Multi-model (Claude, GPT, Gemini, Ollama). Git-aware with automatic commit
generation. Python-based, which means slower startup and higher resource
usage than a compiled tool. No learning, no plan execution, no quality gates.

**Continue (open source).** IDE extension for VS Code and JetBrains.
Extensible via custom context providers. Multi-model support. Not a
standalone agent -- cannot run headlessly. No learning, no plan execution,
no quality gates.

### 2.3 The Structural Gap

The uniformity of omissions across the coding agent landscape is not
accidental. It reflects a shared assumption: the agent is a tool the human
operates, session by session, task by task. No coding agent today is designed
to improve autonomously over time. The capabilities that tiagent adds are not
incremental features bolted onto an existing model. They require a different
architecture -- one built around persistent state, quality enforcement, and
learned routing from the start.

| Missing capability              | Why incumbents lack it                                        |
|---------------------------------|---------------------------------------------------------------|
| Self-improvement loops          | Requires persistent knowledge store + episode logging         |
| Autonomous plan execution       | Requires DAG executor + quality gates between steps           |
| Quality gates                   | Requires compile/test/lint integration into the agent loop    |
| Shared learning                 | Requires a transport layer for cross-instance knowledge       |
| PRD-to-execution pipeline       | Requires plan generation + task decomposition + gate pipeline |
| Learned model routing           | Requires episode data + bandit/cascade routing algorithms     |
| Playbook extraction             | Requires pattern mining over successful episode sequences     |

These are not features that can be added to Claude Code or Cursor in a point
release. They require rearchitecting the agent loop from single-session to
persistent, from human-directed to plan-driven, from stateless to learning.

---

## 3. On-Chain Agent Framework Landscape

### 3.1 Feature Matrix

| Capability                  | ElizaOS  | IronClaw | Olas     | Solana AK | AgentKit | polkagent | ARC/Rig | tiagent    |
|-----------------------------|:--------:|:--------:|:--------:|:---------:|:--------:|:---------:|:-------:|:----------:|
| **Chain**                   | Multi    | NEAR     | Ethereum | Solana    | Base/EVM | Polkadot  | Agnostic| Celestia   |
| **Language**                | TS       | Rust     | Python   | TS        | TS/Py    | Rust      | Rust    | **Rust**   |
| **GitHub stars (approx)**   | 23K+     | 14K+     | Moderate | Growing   | Growing  | Private   | 6.7K    | New        |
| **Rust-native**             | No       | Yes      | No       | No        | No       | Yes       | Yes     | **Yes**    |
| **Multi-model LLM**         | Yes      | Yes      | Yes      | Yes       | Yes      | Yes       | Yes     | **Yes**    |
| **MCP integration**         | Plugin   | No       | No       | No        | Yes      | Partial   | No      | **Yes**    |
| **TEE support**             | No       | Yes      | No       | No        | No       | No        | No      | Planned    |
| **WASM sandboxing**         | No       | Yes      | No       | No        | No       | No        | No      | Planned    |
| **Policy / grant system**   | No       | Yes      | Yes      | No        | No       | Yes       | No      | **Yes**    |
| **Agent identity**          | No       | NEAR ID  | NFTs     | No        | Wallet   | No        | No      | **DA-backed** |
| **Multi-agent coordination**| No       | No       | Yes      | No        | No       | No        | No      | **Yes**    |
| **Self-improvement**        | No       | No       | No       | No        | No       | No        | No      | **Yes**    |
| **Persistent memory**       | Yes      | Yes      | Yes      | No        | No       | Partial   | No      | **Yes**    |
| **Coding agent**            | No       | No       | No       | No        | No       | No        | No      | **Yes**    |
| **DA integration**          | No       | No       | No       | No        | No       | No        | No      | **Yes**    |

The rightmost column is the only one without a single "No" in the
differentiated rows. Every on-chain framework has strengths in its home
ecosystem -- ElizaOS has community, IronClaw has security, Olas has economic
design -- but none of them can write code, improve from experience, or
operate as a general-purpose development tool.

### 3.2 Per-Framework Analysis

**ElizaOS (Multi-chain).** The most popular framework by developer adoption
(23K+ stars). TypeScript monorepo with a clean Provider/Action/Evaluator
pipeline. Broad multi-chain support. Plugin marketplace with community
contributions. Weaknesses: TypeScript performance ceiling, minimal security
model (tools run in the same Node.js process), architectural churn from
AI16Z-to-ElizaOS rebrand. No self-improvement, no coding capability.

**IronClaw (NEAR).** The strongest security model in the survey. WASM sandbox
per tool with capability-based permissions. TEE-backed credential vault --
secrets are encrypted at rest and decrypted only inside the enclave.
Prompt-injection detection as a runtime layer. Weaknesses: NEAR-centric
(WASM sandbox designed around NEAR's key model), no multi-agent
coordination, young project. No self-improvement, no coding capability.

**Olas / Autonolas (Ethereum).** The most mature economic model. FSM-based
agent coordination with deterministic state transitions. Multi-agent
consensus: multiple instances agree on actions before executing them.
On-chain component registry (agents as NFTs). Staking/slashing for economic
security. 10M+ agent-to-agent transactions. Weaknesses: FSM rigidity
conflicts with LLM non-determinism, Python-heavy, steep learning curve,
Ethereum gas costs. No self-improvement, no coding capability.

**Solana Agent Kit / SendAI (Solana).** Broadest action coverage of any
single-chain framework (60+ pre-built actions). Plugin modularity -- install
only what you need. Solana's 400ms slot times enable real-time agent loops.
Integrations with LangChain and Vercel AI SDK. Weaknesses: TypeScript only
(despite Solana programs being Rust), single-chain, basic security model. No
self-improvement, no coding capability.

**Coinbase AgentKit (Base/EVM).** Best wallet abstraction in the survey via
Coinbase's MPC infrastructure. MCP-native design with four major MCP products
shipped by mid-2026. Native x402 protocol support for autonomous API
payments. Weaknesses: Coinbase dependency (MPC wallet is a Coinbase service,
not self-hosted), no Rust SDK, shallow skill capabilities, no orchestration.
No self-improvement, no coding capability.

**polkagent (Polkadot).** Architecturally the most sophisticated -- 90-crate
Rust hexagonal workspace with a four-stage effect pipeline (Intent, Claim,
Attempt, Outcome). Swappable adapters for chain clients, LLM providers, and
storage backends. Grant/policy system for fine-grained authorization.
Weaknesses: massive surface area (90 crates), sparse docs, Polkadot-specific
idioms leak through the hexagonal boundary. No self-improvement, no coding
capability.

**ARC / Rig (Chain-Agnostic).** Most widely adopted Rust LLM library. Trait-
based composition: CompletionModel, Agent, Pipeline. Provider traits for
OpenAI, Anthropic, Cohere, and others. Enterprise deployments at Cloudflare,
Neon, Nethermind. Weaknesses: not a blockchain framework (no wallet, no chain
state, no transaction signing), no security model beyond Rust's type system,
no agent identity. No self-improvement, no coding capability.

### 3.3 Architecture Comparison

| Style        | Framework(s)       | Strengths                                  | Weaknesses                                 |
|--------------|--------------------|--------------------------------------------|---------------------------------------------|
| Hexagonal    | polkagent          | Swappable adapters, testable core          | More crates, indirection overhead           |
| Agent OS     | IronClaw           | Strong isolation, OS-like resource mgmt    | Complex runtime, single-agent focus         |
| Plugin       | ElizaOS, Solana AK | Easy extension, community contributions    | Security boundary is weak                   |
| FSM          | Olas               | Deterministic, auditable, consensus-ready  | Rigid, poor fit for LLM non-determinism     |
| Skills       | AgentKit           | Simple, composable, framework-agnostic     | Shallow capabilities, no orchestration      |
| Trait-based  | Rig                | Minimal, composable, type-safe             | No runtime, no chain integration            |
| **Hybrid**   | **tiagent**        | **Agent loop + DA + plan execution**       | **New entrant, unproven at scale**          |

tiagent's architecture is a hybrid that draws from the best of each style:
trait-based composition from Rig, effect pipeline discipline from polkagent,
the plan-execute-gate-persist loop from its own lineage, and DA-backed state
from Celestia integration.

### 3.4 Security Model Comparison

| Model            | Framework(s) | Trust boundary        | Key management     | Primary threat model          |
|------------------|--------------|-----------------------|--------------------|-------------------------------|
| TEE + WASM       | IronClaw     | Per-tool sandbox      | TEE vault          | Malicious tool, prompt inject |
| Grant/policy     | polkagent    | Per-extrinsic policy  | External signer    | Overprivileged agent          |
| Staking/slashing | Olas         | Economic penalty      | Operator-managed   | Byzantine agent instance      |
| MPC wallet       | AgentKit     | Coinbase infra        | Coinbase MPC       | Key exfiltration              |
| Type system      | Rig          | Compile-time          | Developer-managed  | Type errors only              |
| None             | ElizaOS, SAK | Same process          | Developer-managed  | Minimal                       |
| **Policy + DA**  | **tiagent**  | **Per-tool contract** | **Agent contract** | **Overprivileged + audit**    |

---

## 4. DA Layer Comparison for AI

No coding agent or on-chain framework has made data availability a core
architectural decision. tiagent is the first to treat a DA layer as the
persistence backbone for agent learning. This section compares the DA layers
that could serve this role.

### 4.1 DA Layer Feature Matrix

| DA Layer    | AI-specific focus | Ecosystem fund        | Mainnet status     | tiagent compatibility     |
|-------------|-------------------|-----------------------|--------------------|---------------------------|
| **Celestia**| None (yet)        | $0 AI-specific        | Production (2023+) | Native -- tiagent is built for this |
| **0G Labs** | Primary           | $88.88M + $20M accel. | Early/testnet      | Could adapt, not designed for       |
| **EigenDA** | None              | N/A                   | Production (ETH)   | Not architecturally relevant        |
| **Avail**   | None              | N/A                   | Production         | Could adapt                         |
| **Near DA** | Indirect          | NEAR AI fund          | Production         | IronClaw alignment, not Celestia    |

### 4.2 Celestia vs 0G: The Key Comparison

The most direct DA-layer competition for tiagent's narrative is between
Celestia and 0G Labs. Both are modular DA layers. The difference is strategic
maturity versus topical positioning.

| Dimension                | Celestia                         | 0G Labs                               |
|--------------------------|----------------------------------|---------------------------------------|
| **Mainnet age**          | 2+ years (launched Oct 2023)     | Months (Aristotle Mainnet Sep 2025)   |
| **Validator set**        | Large, decentralized             | Smaller, quorum-based                 |
| **Light node network**   | Production-grade, widely adopted | Early                                 |
| **AI-specific features** | None built-in                    | AI data types, verified training      |
| **Blob size limit**      | Standard (growing)               | 1GB+ claimed                          |
| **Ecosystem funding**    | General-purpose grants           | $88.88M AI-specific + $20M accel.     |
| **AI narrative**         | Not yet claimed                  | Core positioning                      |
| **Battle-tested infra**  | Yes                              | Not yet                               |
| **DAS (data sampling)**  | Production-grade                 | Early                                 |
| **Developer tooling**    | Mature (Rollkit, etc.)           | Developing                            |

**The strategic opportunity:** 0G has money and narrative but not maturity.
Celestia has maturity and infrastructure but has not yet claimed the AI
narrative. tiagent bridges this gap. It gives Celestia a concrete AI use case
-- self-improving coding agents whose learning state is DA-backed -- without
requiring Celestia to build AI-specific infrastructure. The agent framework
is the application layer; Celestia provides the DA layer exactly as designed.

### 4.3 Why Celestia Wins for Agent Learning

| Requirement for agent learning DA | Celestia | 0G Labs | Why it matters |
|-----------------------------------|----------|---------|----------------|
| Reliable blob posting             | Yes      | Unproven| Agents need guaranteed state persistence     |
| Light node verification           | Yes      | Early   | Agents verify each other's learning claims   |
| Namespace isolation               | Yes      | Yes     | Per-agent or per-team knowledge partitioning  |
| Low latency for small blobs       | Yes      | Optimized for large | Learning updates are small, frequent |
| Ecosystem trust                   | High     | Building | Developers trust the infra their agents use  |
| Cost predictability               | Yes      | TBD     | Agents need budgetable DA costs              |

Agent learning data is small (KB-scale playbooks, episode summaries, routing
tables) and frequent (posted after each task completion). Celestia's design
-- optimized for many small blobs with DAS verification -- is a better fit
than 0G's design, which optimizes for large AI artifacts (model weights,
training datasets).

---

## 5. What Makes tiagent Unique

No competitor -- coding agent or on-chain framework -- has the following
combination. Each capability exists in isolation in at most one competitor.
The combination exists nowhere.

### 5.1 Capabilities Nobody Else Has

| # | Capability                                | Nearest competitor         | Why they fall short                          |
|---|-------------------------------------------|----------------------------|----------------------------------------------|
| 1 | Self-improving coding agent               | None                       | No coding agent learns from past sessions    |
| 2 | Shared learning via DA (Waze for agents)  | None                       | No framework shares learned patterns via DA  |
| 3 | PRD-to-code autonomous workflow           | None                       | No agent takes a PRD and executes it as a plan |
| 4 | Quality gate pipeline                     | None                       | No agent runs compile/test/lint after every action |
| 5 | Celestia-native integration               | None                       | No agent framework is built for Celestia     |
| 6 | Model-agnostic cascade routing            | Aider, Continue (partial)  | They support multiple models but don't learn which works best |
| 7 | Open-source Rust implementation           | polkagent, IronClaw, Rig   | They are Rust but not coding agents          |

### 5.2 The Compound Advantage

These capabilities are not independent features. They compound:

```
Self-improvement + Shared learning = Network effects
    Every agent that uses tiagent makes every other agent better.
    This is the Waze model applied to coding.

Plan execution + Quality gates = Autonomous reliability
    Agents can execute multi-step plans without human babysitting
    because every step is validated before the next begins.

Cascade routing + Episode logging = Cost optimization
    The system learns which model handles which task type best
    and routes accordingly, reducing cost without reducing quality.

Celestia DA + Shared learning = Verifiable knowledge
    Learning claims are posted to a DA layer, so agents can verify
    that a playbook or routing table was genuinely learned, not
    fabricated.

All of the above = First self-improving coding agent with
    verifiable, shared learning on a production DA layer.
```

### 5.3 Competitive Moat Depth

| Moat type          | What it means for tiagent                                      | Time to replicate   |
|--------------------|----------------------------------------------------------------|---------------------|
| **Architectural**  | Agent loop built around learning, not bolted on                | 12-18 months        |
| **Data**           | Every deployed agent generates learning data that improves all | Grows with adoption |
| **Ecosystem**      | First Celestia-native agent framework                          | First-mover window  |
| **Network**        | Shared learning creates cross-instance network effects         | Cannot buy, must grow |

---

## 6. Competitive Positioning Summary

### 6.1 Against Coding Agents

| Dimension          | Incumbent position                          | tiagent position                              |
|--------------------|---------------------------------------------|------------------------------------------------|
| Code quality       | Claude Code leads (model quality)           | Model-agnostic; quality via routing + gates    |
| Session memory     | None. Every session starts from zero.       | Persistent knowledge store + episode logging   |
| Multi-step work    | Human-directed, one task at a time          | Autonomous plan execution with DAG scheduling  |
| Quality assurance  | Manual ("run the tests yourself")           | Automatic gates: compile, test, lint, diff     |
| Team knowledge     | Each developer teaches the agent separately | Shared learning pool via DA                    |
| Model flexibility  | Locked to one provider (usually)            | Any model, with learned routing per task type   |
| Extensibility      | MCP (Claude Code), plugins (some)           | MCP + on-chain tool discovery                  |

### 6.2 Against On-Chain Frameworks

| Dimension            | Incumbent position                         | tiagent position                              |
|----------------------|--------------------------------------------|------------------------------------------------|
| Primary use case     | Wallet ops, DeFi, token management         | Code generation, plan execution, development   |
| Self-improvement     | None. Static configuration.                | Episode-driven learning, playbook extraction   |
| DA integration       | None (use whatever chain provides)         | Celestia-native: blobs, namespaces, light nodes |
| Language             | TypeScript (ElizaOS, SAK), Python (Olas)   | Rust (performance, safety, WASM-ready)         |
| Security model       | Varies widely (none to TEE)                | Policy-based agent contracts + DA audit trail  |
| Coding capability    | None                                       | Full coding agent with file editing, shell, git |
| Community            | ElizaOS leads (23K+ stars)                 | New entrant -- must earn community             |

### 6.3 The Two-by-Two

```
                          On-chain integration
                    Low                     High
                 +-----------------------------+
          High   | Claude Code    |  tiagent   |
                 | Codex CLI      |            |
   Coding        | Cursor         |            |
   agent         | Aider          |            |
   capability    +--------------  +  ----------+
          Low    |                | ElizaOS    |
                 |  (empty)       | IronClaw   |
                 |                | Olas       |
                 |                | AgentKit   |
                 +-----------------------------+
```

tiagent occupies the upper-right quadrant alone.

---

## 7. Competitive Risks and Mitigations

### 7.1 Risk Matrix

| Risk                                         | Probability | Impact | Mitigation                                     |
|----------------------------------------------|:-----------:|:------:|------------------------------------------------|
| Claude Code adds learning/planning           | Medium      | High   | They won't share learning openly (closed model). tiagent's DA-backed shared learning is structurally different. |
| 0G funds a competing AI agent framework      | Medium      | Medium | Celestia's infra is more mature. tiagent's coding focus is orthogonal to 0G's compute narrative. |
| ElizaOS adds Celestia support                | Low         | Medium | ElizaOS is TypeScript -- performance and safety ceiling. Adding DA doesn't add coding or learning. |
| IronClaw expands beyond NEAR                 | Low-Medium  | Medium | IronClaw is security-focused, not coding-focused. Different value prop. |
| New Rust coding agent emerges                | Medium      | High   | First-mover on learning + DA. Network effects from shared learning create durable moat. |
| Olas adapts FSM model for LLM agents         | Low         | Low    | FSM rigidity is architecturally incompatible with LLM non-determinism. |
| Celestia builds its own agent framework      | Very Low    | High   | Foundations build infra, not apps. tiagent is the app layer. |
| ARC/Rig adds agent identity + chain layer    | Low-Medium  | Medium | Rig is an LLM library, not an agent framework. Different scope. |

### 7.2 Why Incumbents Cannot Easily Respond

**Claude Code / Codex CLI / Cursor:** Adding learning requires persistent
state, episode logging, and a knowledge store -- a fundamental rearchitecture
of the session-based model. Even if they add learning, they will not share it
openly across instances (it would leak proprietary training signal). And they
have zero incentive to integrate with Celestia or any DA layer.

**ElizaOS:** The largest community, but TypeScript imposes a performance
ceiling. Adding Celestia blob posting to ElizaOS is technically feasible but
does not address the core gaps: no coding capability, no self-improvement, no
quality gates. ElizaOS would need to become a different product.

**IronClaw:** Best security model in the survey, but designed for single-agent
NEAR operations, not multi-step coding workflows. The WASM sandbox model is
excellent for tool isolation but does not address plan execution or learning.

**0G Labs:** Has funding ($88.88M+) and the AI narrative, but is an
infrastructure layer, not an agent framework. 0G could fund someone to build
an agent framework on 0G DA, but that framework would still need to solve
the coding agent + learning + plan execution problems from scratch.

### 7.3 Asymmetric Advantages

tiagent has structural advantages that are difficult for competitors to
replicate:

1. **Learning compounds.** Every deployed tiagent instance generates data
   that improves every other instance. This advantage grows with adoption and
   cannot be purchased or shortcut.

2. **Celestia first-mover.** There is no Celestia-native agent framework.
   tiagent defines what "agent on Celestia" means. Latecomers compete against
   an established integration and community.

3. **Rust + DA is a narrow intersection.** The Venn diagram of "Rust agent
   framework" and "DA-aware" and "self-improving" contains exactly one entry.
   Competitors must invest in all three simultaneously to match.

4. **Coding agents and on-chain agents are converging.** The market is moving
   toward agents that can both write code and interact with chains. tiagent is
   already there. Competitors must bridge from one side or the other.

---

## 8. Standards Landscape

tiagent does not operate in a vacuum. Several emerging standards shape the
competitive landscape and inform tiagent's integration strategy.

| Standard / Protocol | What it is                                      | tiagent relevance                                |
|---------------------|-------------------------------------------------|--------------------------------------------------|
| **MCP**             | Tool integration protocol (Anthropic / Linux Foundation) | Core integration layer. Celestia ops exposed as MCP servers. |
| **A2A**             | Agent-to-agent communication (Google / Linux Foundation) | Inter-agent coordination for multi-agent plans.   |
| **ERC-8004**        | On-chain agent identity (Ethereum)              | Model for Celestia-native agent identity.         |
| **x402**            | HTTP payment protocol (Coinbase)                | Agent-to-agent payment for DA and compute.        |

tiagent's strategy: adopt MCP and A2A as integration layers, implement
ERC-8004-equivalent identity on Celestia namespaces, and support x402 for
metered service access. This positions tiagent as standards-compliant rather
than proprietary -- critical for ecosystem adoption.

---

## 9. Key Takeaways for Celestia Foundation

1. **The AI agent market has a structural gap.** Coding agents do not learn.
   On-chain frameworks do not code. Nobody uses DA for agent learning. tiagent
   fills all three gaps simultaneously.

2. **Celestia has no AI narrative today.** 0G Labs has claimed the "AI DA"
   positioning with $88.88M in funding. Celestia's infrastructure is more
   mature, but the narrative is unclaimed. tiagent provides a concrete AI use
   case that Celestia can point to without building AI-specific features.

3. **tiagent is first-mover on Celestia.** No other agent framework --
   coding or on-chain -- is built for Celestia. This is a window, not a
   permanent state. Early support (grants, integration assistance, co-marketing)
   compounds tiagent's head start before competitors arrive.

4. **The network effects are real.** Shared learning via DA creates a Waze-
   like dynamic where every agent improves every other agent. This is not
   possible on centralized coding agents (they won't share learning) and not
   possible without a DA layer (no verifiable, permissionless state). Celestia
   is the enabler.

5. **The competitive risks are manageable.** Claude Code could add learning
   but won't share it. 0G could fund a competitor but lacks mature infra.
   ElizaOS could add Celestia but lacks coding capability. The combination
   of coding + learning + DA + Celestia is structurally difficult to replicate.
