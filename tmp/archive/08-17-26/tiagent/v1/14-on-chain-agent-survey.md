# 14 -- Survey of Agent Frameworks and Coding Harnesses

> tiagent is a general-purpose self-improving coding agent that also
> operates natively on Celestia. This document surveys the landscape of
> coding agents, on-chain agent frameworks, and AI development harnesses
> as of mid-2026, extracts recurring architectural patterns, catalogs
> emerging interoperability standards, and identifies specific lessons
> that should inform tiagent's design.

---

## 1. Introduction

tiagent competes on two fronts: as a general-purpose coding agent (against
Claude Code, Codex, Cursor, and other development harnesses) and as an
on-chain agent framework (against polkagent, IronClaw, ElizaOS, and
others). This survey covers both landscapes.

Every major blockchain ecosystem now hosts at least one agent framework.
The ambition is the same everywhere: give an LLM a wallet, let it read
chain state, let it sign transactions, and wrap the whole thing in
enough safety machinery that the agent cannot drain the treasury on its
first turn. Meanwhile, the general-purpose coding agent space has
converged on a remarkably similar set of capabilities -- and a
remarkably similar set of blind spots. The implementations differ
dramatically in language, architecture, security model, and philosophy,
but none of them learn.

This survey covers the dominant coding agents, eight on-chain frameworks
spanning six ecosystems, five cross-cutting protocols, and the gaps that
remain. The goal is not to rank them but to extract what tiagent should
adopt, what it should avoid, and where it can differentiate.

### What we are looking for

| Dimension              | Why it matters for tiagent                                    |
|------------------------|---------------------------------------------------------------|
| Language / runtime     | tiagent is Rust; interop cost with TS/Python frameworks is high |
| Architecture style     | Monolithic vs. modular vs. hexagonal affects extensibility      |
| Tool system            | Native tools, MCP, or plugin? Determines integration surface    |
| Security model         | TEE, sandboxing, policy-based? Determines trust boundary        |
| Self-improvement       | Most frameworks are static; tiagent explicitly learns            |
| Plan execution         | Multi-step autonomous workflows with quality gates               |
| Model routing          | Model-agnostic with learned cascade routing                     |
| Shared learning        | Cross-instance knowledge transfer and playbook extraction        |
| DA / settlement layer  | Must align with Celestia's modular DA model                     |
| Agent identity         | On-chain identity is becoming table stakes                      |
| Inter-agent comms      | Multi-agent coordination is the next frontier                   |

---

## 2. General-Purpose Coding Agent Landscape

Before examining on-chain frameworks, we survey the coding agents that
tiagent competes with directly. These are the tools developers use
today for AI-assisted development. Every one of them is limited in ways
that define tiagent's opportunity.

### 2.0a Claude Code (Anthropic)

**What it is.** The dominant CLI-based coding agent. Runs in the terminal,
reads the codebase, makes edits, runs commands. Ships with Anthropic's
Claude models and has the deepest tool-use integration of any coding
agent.

**Strengths.**
- Best-in-class code understanding and generation quality (as of mid-2026).
- Deep tool integration: file editing, shell commands, search, web fetch.
- Agentic loop handles multi-step tasks without human intervention.
- MCP support for extending with custom tool servers.
- Large context window (200K tokens) for whole-codebase reasoning.

**Weaknesses.**
- Locked to Claude models. Cannot use GPT, Gemini, or open-source models.
- No learning. Every session starts from zero. Cannot recall what worked
  in past sessions, what patterns failed, or what the codebase's conventions
  are beyond what fits in CLAUDE.md.
- No plan execution. Cannot take a PRD or task list and execute it
  autonomously across multiple tasks with quality gates between steps.
- No shared learning. Ten developers on the same team each teach it the
  same codebase conventions independently.
- No quality gates. No compilation check, no test run, no lint pass
  between agent actions unless the user manually asks.
- Subscription model with usage caps.

### 2.0b Codex CLI (OpenAI)

**What it is.** OpenAI's answer to Claude Code. A CLI coding agent that
runs in a sandboxed environment, executing code generation and editing
tasks.

**Strengths.**
- Sandboxed execution environment for safety.
- Integration with OpenAI's model lineup (o3, o4-mini, GPT-4.1).
- Open-source CLI client.

**Weaknesses.**
- Locked to OpenAI models. No Claude, no Gemini, no local models.
- Single-task oriented. No multi-step plan execution.
- No learning or self-improvement.
- No quality gates or validation pipeline.
- Newer and less mature than Claude Code.

### 2.0c Cursor

**What it is.** An IDE (fork of VS Code) with deep AI integration. The
most popular AI-assisted coding environment by user count.

**Strengths.**
- Seamless IDE integration: autocomplete, inline editing, chat, and
  agent mode all in one environment.
- Multi-model support (Claude, GPT, Gemini) via configuration.
- Large user community and active development.
- Agent mode can make multi-file edits autonomously.

**Weaknesses.**
- Closed-source. No ability to extend the agent loop, customize
  validation, or integrate custom learning.
- IDE-locked. Cannot run headlessly, in CI, or as part of a pipeline.
- No plan execution, quality gates, or autonomous multi-task workflows.
- No learning across sessions. Each session starts fresh.
- Subscription model with opaque pricing per model tier.
- No shared learning across team members.

### 2.0d Windsurf

**What it is.** Similar to Cursor -- an AI-powered IDE with integrated
agent capabilities. Competes directly with Cursor.

**Strengths.**
- IDE-integrated agent with "Cascade" multi-step reasoning.
- Multi-model support.
- Active development and growing user base.

**Weaknesses.**
- Same fundamental limitations as Cursor: closed-source, IDE-locked,
  no learning, no plan execution, no quality gates.
- Smaller community than Cursor.

### 2.0e Aider

**What it is.** An open-source CLI coding agent. The most capable open-source
alternative to Claude Code and Codex.

**Strengths.**
- Open-source (Apache-2.0).
- Multi-model: works with Claude, GPT, Gemini, Ollama, and many others.
- Git-aware: creates commits automatically with meaningful messages.
- Repository map for codebase understanding.
- Active community with frequent releases.

**Weaknesses.**
- Python-based. Slower startup and higher resource usage than a compiled tool.
- No self-improvement. Does not learn from past sessions or outcomes.
- No plan execution. Single-task focused.
- No quality gates or validation pipeline.
- No shared learning across instances.

### 2.0f Continue

**What it is.** An open-source IDE extension (VS Code and JetBrains) for
AI-assisted development. Emphasizes extensibility and model flexibility.

**Strengths.**
- Open-source.
- Multi-IDE support (VS Code, JetBrains).
- Multi-model with easy provider configuration.
- Extensible via custom context providers and slash commands.

**Weaknesses.**
- IDE extension, not a standalone agent. Limited headless/CLI capability.
- No autonomous plan execution.
- No learning or self-improvement.
- No quality gates.

### What NONE of them have

The general-purpose coding agent landscape has a striking uniformity of
omissions. No coding agent available today provides:

| Capability                         | Status across all coding agents |
|------------------------------------|-------------------------------|
| **Self-improvement loops**         | None. Every session starts from zero context about what worked before. |
| **Autonomous plan execution**      | None. All are single-task or require human intervention between steps. |
| **Quality gates**                  | None. No agent automatically runs compile/test/lint gates between actions. |
| **Shared learning**               | None. Knowledge does not transfer between instances or team members. |
| **PRD-to-execution pipeline**      | None. No agent can take a product requirements document and execute it as a multi-step plan. |
| **Model-agnostic learned routing** | None. Some support multiple models, but none learn which model works best for which task type. |
| **Playbook extraction**            | None. No agent distills successful patterns into reusable playbooks. |
| **Persistent knowledge store**     | None. Beyond conversation history, no agent maintains durable knowledge about a codebase or domain. |

These are not niche features. They are the difference between a tool that
assists and a harness that improves. tiagent occupies this gap.

---

## 3. On-Chain Framework-by-Framework Analysis

### 3a. polkagent (Polkadot)

**What it is.** A 90-crate Rust workspace for building autonomous agents
on Polkadot and its parachains. Built by the Polkadot agent working
group (a community initiative, not Parity directly).

**Architecture.** Hexagonal (ports and adapters). The domain core defines
an effect pipeline -- Intent, Claim, Attempt, Outcome -- that is
independent of any specific chain runtime. Adapters plug in chain
clients, LLM providers, and storage backends without touching the
domain.

**Key features.**

- Grant/policy system for fine-grained authorization: each agent holds a
  policy document that declares exactly which extrinsics it may submit,
  which pallets it may call, and budget ceilings per epoch.
- Effect pipeline forces every action through a four-stage lifecycle with
  explicit rollback at each stage.
- Native Substrate integration via subxt; can target any parachain
  without custom glue.
- Multi-model support with provider traits (OpenAI, Anthropic, Ollama).

**Strengths.**

- Architecturally the most sophisticated framework surveyed. The
  hexagonal boundary means you can swap the chain layer from Polkadot to
  Cosmos (or Celestia) without rewriting business logic.
- The effect pipeline is a genuine contribution: it makes agent actions
  auditable and reversible by design.
- Rust-native, so FFI cost to tiagent is zero.

**Weaknesses.**

- 90 crates is a lot of surface area. Documentation is sparse outside
  the README and a few blog posts.
- Polkadot-specific idioms (extrinsics, pallets, XCM) leak through the
  hexagonal boundary in practice, limiting portability.
- No self-improvement loop. Agents are configured statically.

**Relevance to tiagent.** High. The effect pipeline (Intent -> Claim ->
Attempt -> Outcome) is directly applicable. The grant/policy system is
the strongest authorization model in the survey and should inform
tiagent's own permission model. The hexagonal architecture validates
tiagent's decision to separate domain logic from chain-specific adapters.

---

### 3b. IronClaw (NEAR)

**What it is.** A security-first, open-source AI agent runtime built in
Rust by NEAR AI. Launched publicly at NEARCON 2026, it treats isolation
and credential safety as first-class concerns rather than afterthoughts.

**Architecture.** Agent OS model. The runtime manages a set of "claws"
(tools), each running inside its own WASM sandbox. A central credential
vault, optionally backed by a Trusted Execution Environment, injects
secrets at the network boundary and never exposes them to the LLM.

**Key features.**

- WASM sandbox per tool: capability-based permissions, approved
  endpoints, strict resource limits. No tool can access the filesystem,
  network, or other tools outside its declared capabilities.
- TEE-backed credential vault: secrets are encrypted at rest and
  decrypted only inside the TEE enclave. Even the cloud operator cannot
  read agent memory.
- Prompt-injection detection as a runtime layer.
- Network whitelists per tool: an agent can call api.coingecko.com but
  not arbitrary URLs.
- Zero telemetry, local-first storage, data sovereignty by default.
- Integration with standard enterprise tools (Slack, Notion, email,
  REST APIs) via isolated credential sandboxes.

**Strengths.**

- Best-in-class security model. WASM isolation per tool is stronger than
  process-level sandboxing and cheaper than a full VM per tool.
- Rust + WASM means memory safety is enforced at two levels: compile
  time (Rust) and runtime (WASM sandbox).
- TEE support for high-value operations (key management, signing).
- The "Agent OS" framing is appropriate: it manages lifecycle, resources,
  and permissions the way an OS manages processes.

**Weaknesses.**

- NEAR-centric. The WASM sandbox and credential vault are designed around
  NEAR's key model and account system.
- No built-in multi-agent coordination. IronClaw manages a single agent;
  orchestrating multiple agents requires external tooling.
- Young project. The public release is from 2026; ecosystem and
  documentation are still developing.

**Relevance to tiagent.** Very high. IronClaw's WASM-sandbox-per-tool
model is the strongest security architecture in the survey. tiagent
should adopt a similar isolation model for tool execution. The TEE
integration pattern (encrypt at rest, decrypt at boundary, never expose
to LLM) should be the baseline for tiagent's key management. The prompt-
injection detection layer is a practical feature worth replicating.

---

### 3c. ElizaOS (Multi-chain)

**What it is.** Originally ai16z's Eliza, now ElizaOS: a TypeScript-based,
MIT-licensed agent framework with broad multi-chain support. With over
23,000 GitHub stars, it is the most popular agent framework by developer
adoption.

**Architecture.** Monorepo-based TypeScript framework built around a
unified plugin model. Each message flows through a three-stage pipeline:

1. **Providers** inject contextual data (wallet balances, price feeds,
   conversation history) into the LLM prompt.
2. The LLM selects from **Actions** (executable capabilities such as
   token swaps, NFT mints).
3. **Evaluators** run post-hoc to update persistent memory and validate
   outcomes.

A unified message bus supports multiple transports: Discord, Telegram,
X (formerly Twitter), HTTP, and on-chain channels.

**Key features.**

- Action chaining: agents can execute sequences of dependent tasks.
- Persistent memory across interactions via evaluator-updated state.
- Native Solana integration for token management.
- Cross-chain support via Chainlink CCIP (EVM chains).
- Plugin marketplace with community-contributed integrations.
- Large and active developer community.

**Strengths.**

- Largest community and plugin ecosystem in the survey.
- The Provider -> Action -> Evaluator pipeline is clean and easy to
  reason about.
- Multi-chain by design; not locked to a single ecosystem.
- Action chaining enables genuinely complex agent workflows.

**Weaknesses.**

- TypeScript. Performance ceiling is lower than Rust, and type safety is
  weaker despite TS's type system.
- Developer experience is mixed: the framework is powerful but suffers
  from dropped features, breaking changes between versions, and weak
  migration paths (per independent 2026 assessment).
- Security model is minimal. No WASM sandboxing, no TEE integration, no
  formal policy system. Tools run in the same Node.js process as the
  agent.
- The rename from AI16Z to ElizaOS involved architectural churn that
  fragmented the community.

**Relevance to tiagent.** Medium. The Provider/Action/Evaluator pipeline
is a useful conceptual reference, but the TypeScript implementation is
not directly reusable. The plugin marketplace model demonstrates what a
mature ecosystem looks like; tiagent should plan for a similar extension
mechanism. The security gaps are a cautionary example of what happens
when isolation is an afterthought.

---

### 3d. Olas / Autonolas (Ethereum)

**What it is.** A protocol and framework for decentralized autonomous
agent services, built on Ethereum. Agents run off-chain as a
multi-agent-system (MAS) and coordinate via on-chain registries.
Originally Autonolas, rebranded to Olas.

**Architecture.** FSM (Finite-State Machine) based. Each agent service
implements its business logic as an FSM App. The internal state of the
FSM is replicated and synchronized across all agent instances forming
the service. A "period" is a cycle through the FSM's main states (e.g.,
"collect observations" -> "compute value" -> "publish on-chain").

**Key features.**

- FSM-based agent coordination: deterministic state transitions,
  replayable behavior.
- Multi-agent consensus: multiple agent instances run the same FSM and
  reach agreement on state transitions.
- On-chain component registry: agents, services, and skills are
  registered as NFTs on Ethereum.
- Ethereum staking for economic security: operators stake OLAS tokens.
- Marketplace with 10M+ agent-to-agent transactions as of 2026.
- Open Autonomy framework for building new agent services.

**Strengths.**

- Most mature economic model. Staking, slashing, and the component
  registry create real incentives for quality.
- FSM-based coordination is deterministic and auditable -- you can
  replay any agent's history.
- Multi-agent consensus is unique in the survey: multiple instances
  agree on actions before executing them.
- Battle-tested: the marketplace has processed millions of transactions
  since 2023.

**Weaknesses.**

- FSM rigidity: every possible state transition must be defined upfront.
  This limits the kind of adaptive behavior that LLM-driven agents can
  exhibit.
- Python-heavy stack. The Open Autonomy framework is Python-based.
- Ethereum gas costs for on-chain registration and staking are
  non-trivial.
- The learning curve is steep: building a new agent service requires
  understanding FSMs, the ABCI interface, and the component registry.

**Relevance to tiagent.** Medium-high. The FSM coordination model is too
rigid for LLM agents but the underlying idea -- deterministic state
machines for consensus -- is applicable to tiagent's gate pipeline. The
on-chain component registry (agents as NFTs) is a pattern worth
studying. The staking/slashing model for economic security is directly
relevant to tiagent's Celestia-native operation.

---

### 3e. Solana Agent Kit / SendAI (Solana)

**What it is.** An SDK by SendAI for building agents that interact with the
Solana ecosystem. Provides 60+ pre-built actions covering token
operations, NFT minting, DeFi interactions, and more.

**Architecture.** Modular plugin architecture. Install only what you need:
the token plugin handles SPL token transfers and swaps, the NFT plugin
manages Metaplex operations, and the DeFi plugin integrates with
Jupiter, Raydium, and other Solana protocols. Framework-agnostic:
integrates with LangChain, Vercel AI SDK, and others.

**Key features.**

- 60+ pre-built actions across tokens, NFTs, DeFi, and staking.
- Interactive chat mode for guided operations and autonomous mode for
  independent agent actions.
- Built-in error handling and recovery.
- Natural language interface for Solana blockchain operations.
- Plugin architecture allows selective installation.

**Strengths.**

- Broadest action coverage of any single-chain framework.
- Plugin modularity means agents carry only the weight they need.
- Solana's fast finality (400ms slots) enables real-time agent loops.
- Strong integrations with popular AI frameworks (LangChain, Vercel AI).

**Weaknesses.**

- TypeScript/JavaScript only. No Rust SDK despite Solana programs being
  written in Rust.
- Single-chain: tightly coupled to Solana's account model and program
  architecture.
- Security model is basic: relies on wallet permissions rather than
  granular tool-level isolation.
- No self-improvement or learning capabilities.

**Relevance to tiagent.** Low-medium. The plugin architecture and action
catalog demonstrate what "batteries included" looks like for a chain-
specific SDK. The 60+ actions are a useful reference for what agent
capabilities a chain ecosystem needs. However, the TypeScript-only
implementation and Solana specificity limit direct applicability.

---

### 3f. Coinbase AgentKit (Base / EVM)

**What it is.** An open-source framework by Coinbase for enabling AI agents
to take on-chain actions. Framework-agnostic and wallet-agnostic by
design. The "skills layer" that sits beneath Coinbase's Agentic Wallets
infrastructure.

**Architecture.** Skills-based. AgentKit defines "skills" -- composable
units of on-chain capability (e.g., "transfer ERC-20", "swap on
Uniswap", "deploy contract"). Skills are framework-agnostic: they work
with any LLM framework. The wallet layer abstracts key management via
Coinbase's MPC infrastructure.

**Key features.**

- Agentic Wallets (launched February 2026): wallet infrastructure
  purpose-built for autonomous agents.
- MCP-native: four major MCP-compatible products shipped between May
  2025 and June 2026 (Payments MCP, Base MCP, Coinbase for Agents,
  Coinbase Advisor).
- Native x402 protocol support: agents auto-pay HTTP 402 challenges for
  metered API access.
- Multi-chain coverage: Base (primary), plus Ethereum, Polygon,
  Arbitrum, World, and Solana via x402.
- Python and TypeScript SDKs.

**Strengths.**

- Best wallet abstraction in the survey. Agents never touch raw private
  keys; Coinbase's MPC infrastructure handles signing.
- x402 integration is a genuine differentiator: agents can autonomously
  pay for services without pre-configured billing.
- MCP-native design means tools are discoverable and composable by any
  MCP-aware LLM.
- Backed by Coinbase's infrastructure: production-grade reliability.

**Weaknesses.**

- Coinbase dependency. The MPC wallet infrastructure is a Coinbase
  service, not a self-hosted component. This creates a trust dependency
  that conflicts with decentralization goals.
- No Rust SDK.
- Skills are shallow: "call this contract method" rather than complex
  multi-step workflows.
- No agent coordination, learning, or self-improvement.

**Relevance to tiagent.** Medium. The x402 payment integration is directly
applicable: tiagent agents operating on Celestia will need to pay for
DA submissions and cross-chain calls. The MCP-native design validates
tiagent's own MCP integration plans. The wallet abstraction pattern
(agents never touch keys) should be a hard requirement for tiagent.

---

### 3g. ARC / Rig (Chain-Agnostic)

**What it is.** Rig (0xPlaygrounds/rig) is the most widely adopted Rust
LLM library for building agent applications. ARC (AI Rig Complex) is
the token and ecosystem layer built around the Rig framework. The
framework provides type-safe, async Rust primitives for LLM
orchestration.

**Architecture.** Trait-based composition. The core defines a small set of
composable traits: CompletionModel (any LLM provider), Agent (model +
preamble + tools), and Pipeline (agent + chain of transformations). No
opinionated runtime; developers compose these traits into their own
architectures.

**Key features.**

- Type-safe Rust primitives for LLM orchestration.
- Provider traits for OpenAI, Anthropic, Cohere, and others.
- Agent abstraction: LLM model + system prompt + context documents +
  tools.
- Pipeline composition for multi-step agent workflows.
- Approximately 6,700 GitHub stars as of early 2026.
- Enterprise deployments at Cloudflare, Neon, Nethermind, and others.

**Strengths.**

- Rust-native. Zero FFI cost for tiagent integration.
- Minimal and composable: no framework lock-in, no mandatory runtime.
- Type-safe tool definitions at compile time.
- Production-proven in enterprise settings.
- The hybrid architecture pattern (Python for training, Rust for agent
  runtime hot path) is validated by real deployments.

**Weaknesses.**

- No chain integration. Rig is an LLM orchestration library, not a
  blockchain agent framework. Wallet management, transaction signing,
  and chain state reading are out of scope.
- No security model beyond Rust's type system. No sandboxing, no TEE,
  no policy engine.
- No agent identity, coordination, or economic model.
- The ARC token ecosystem adds complexity without clear technical value.

**Relevance to tiagent.** High for the LLM layer. Rig's trait-based
composition is the right model for tiagent's LLM integration layer. The
Agent and Pipeline abstractions are directly applicable. However, Rig
provides nothing for the chain-specific, security, or self-improvement
layers that tiagent needs. tiagent could use Rig as a dependency for
LLM orchestration while building everything else on top.

---

### 3h. 0G Labs (AI-Native DA)

**What it is.** A decentralized AI operating system (DeAIOS) with modular
infrastructure including a dedicated data availability layer, compute
network, storage, service marketplace, and alignment nodes. Not an
agent framework per se, but an infrastructure layer that agent
frameworks can build on.

**Architecture.** Modular DA + compute. The DA layer uses quorum-based
consensus with VRF for randomization. The compute network provides
decentralized inference and training. The Aristotle Mainnet launched
September 2025.

**Key features.**

- DA layer claimed to be 50,000x faster and 100x cheaper than Ethereum
  DA.
- Scalable and programmable: supports AI-specific data types (model
  weights, training data, inference logs).
- Trained the world's largest decentralized AI model at 107B parameters
  (2025).
- Verification framework for decentralized AI training.
- Partnerships with Chainlink, Google Cloud, Alibaba Cloud.
- Consumer AI development platform launched April 2026.

**Strengths.**

- Purpose-built for AI workloads. Unlike Celestia (general-purpose DA)
  or EigenDA (restaking-based DA), 0G optimizes specifically for the
  data patterns of AI agents: large blobs, sequential writes, model
  checkpoints.
- High throughput: the 1GB+ blob support exceeds what most DA layers
  offer.
- Verification framework addresses the "provable inference" problem that
  other frameworks ignore.

**Weaknesses.**

- Not an agent framework. 0G provides infrastructure (DA, compute) but
  no agent runtime, tool system, or coordination layer.
- Newer and less battle-tested than Celestia or EigenDA.
- The "50,000x faster" claim is hard to verify independently.
- Centralization concerns: the quorum-based consensus relies on a
  relatively small validator set.

**Relevance to tiagent.** Medium. 0G demonstrates that DA layers can be
AI-aware: supporting model weights, training data, and inference logs
as first-class data types. tiagent should consider whether Celestia
namespaces can be used similarly -- storing not just agent state but
also model artifacts and learning checkpoints. The verification
framework for decentralized training is relevant to tiagent's self-
improvement loop, where agents need to prove that their improvements
are genuine.

---

## 4. Cross-Framework Pattern Analysis

### 4.1 Common Patterns

| Pattern                    | polkagent | IronClaw | ElizaOS | Olas | Solana AK | AgentKit | Rig  | 0G   |
|----------------------------|-----------|----------|---------|------|-----------|----------|------|------|
| Rust-native                | Yes       | Yes      | No      | No   | No        | No       | Yes  | N/A  |
| Multi-model LLM support    | Yes       | Yes      | Yes     | Yes  | Yes       | Yes      | Yes  | N/A  |
| MCP integration            | Partial   | No       | Plugin  | No   | No        | Yes      | No   | N/A  |
| TEE support                | No        | Yes      | No      | No   | No        | No       | No   | Yes  |
| WASM sandboxing            | No        | Yes      | No      | No   | No        | No       | No   | No   |
| Policy / grant system      | Yes       | Yes      | No      | Yes  | No        | No       | No   | N/A  |
| Agent identity (on-chain)  | No        | NEAR ID  | No      | NFTs | No        | Wallet   | No   | No   |
| Multi-agent coordination   | No        | No       | No      | Yes  | No        | No       | No   | N/A  |
| Self-improvement / learning| No        | No       | No      | No   | No        | No       | No   | N/A  |
| Persistent memory          | Partial   | Yes      | Yes     | Yes  | No        | No       | No   | N/A  |
| x402 payments              | No        | No       | No      | No   | No        | Yes      | No   | No   |
| Action chaining            | Yes       | No       | Yes     | Yes  | No        | No       | Yes  | N/A  |

### 4.2 Architecture Comparison

| Style       | Framework(s)        | Pros                                      | Cons                                      |
|-------------|---------------------|--------------------------------------------|-------------------------------------------|
| Hexagonal   | polkagent           | Swappable adapters, testable core          | More crates, indirection overhead          |
| Agent OS    | IronClaw            | Strong isolation, OS-like resource mgmt    | Complex runtime, single-agent focus        |
| Plugin      | ElizaOS, Solana AK  | Easy extension, community contributions    | Security boundary is weak                  |
| FSM         | Olas                | Deterministic, auditable, consensus-ready  | Rigid, poor fit for LLM non-determinism    |
| Skills      | Coinbase AgentKit   | Simple, composable, framework-agnostic     | Shallow capabilities, no orchestration     |
| Trait-based | Rig                 | Minimal, composable, type-safe             | No runtime, no chain integration           |

### 4.3 Tool System Comparison

| Approach    | Framework(s)        | Discovery           | Isolation           | Composability       |
|-------------|---------------------|---------------------|---------------------|---------------------|
| Native      | polkagent, Olas     | Static registration | Process-level       | Via effect pipeline |
| MCP         | AgentKit            | MCP server registry | None (trusted)      | MCP tool chaining   |
| Plugin      | ElizaOS, Solana AK  | Plugin manifest     | None (same process) | Action chaining     |
| WASM        | IronClaw            | Capability manifest | WASM sandbox        | Limited             |
| Trait       | Rig                 | Compile-time        | Rust type system    | Pipeline composition|

### 4.4 Security Model Comparison

| Model           | Framework(s)   | Trust boundary          | Key management        | Threat model             |
|-----------------|----------------|-------------------------|-----------------------|--------------------------|
| TEE + WASM      | IronClaw       | Per-tool sandbox        | TEE vault             | Malicious tool, prompt injection |
| Grant/policy    | polkagent      | Per-extrinsic policy    | External signer       | Overprivileged agent     |
| Staking/slashing| Olas           | Economic penalty        | Operator-managed      | Byzantine agent instance |
| MPC wallet      | AgentKit       | Coinbase infra boundary | Coinbase MPC          | Key exfiltration         |
| Type system     | Rig            | Compile-time            | Developer-managed     | Type errors only         |
| None            | ElizaOS, SAK   | Same process            | Developer-managed     | Minimal                  |

---

## 5. Standards and Protocols

### 5.1 ERC-8004: Agent Identity

**What.** An Ethereum Improvement Proposal (created August 2025) that
establishes three lightweight on-chain registries: Identity, Reputation,
and Validation. Co-authored by representatives from MetaMask, the
Ethereum Foundation, Google, and Coinbase.

**How it works.** Agents register an on-chain identity tied to an Ethereum
address. They publish capabilities (what they can do), accumulate
reputation signals (how well they have done it), and optionally request
third-party validation (attestation from trusted parties). Builds on
EIP-155, EIP-712, and ERC-721.

**Status.** Reference implementations deployed on Ethereum mainnet in late
January 2026. Thousands of agents registered by mid-February 2026. At
draft EIP stage; production adoption depends on wallet and framework
integration still rolling out.

**Relevance to tiagent.** High. tiagent needs agent identity on Celestia.
ERC-8004's three-registry model (Identity, Reputation, Validation) is
portable to any chain. tiagent should implement an equivalent on
Celestia, potentially using namespaces for the identity registry and
blob submissions for reputation updates.

---

### 5.2 MCP: Model Context Protocol

**What.** An open standard by Anthropic (November 2024) that gives AI
models a universal way to connect to external tools, data sources, and
services. Often described as "USB-C for AI."

**How it works.** Three capability types: Tools (executable functions),
Resources (readable data entities), and Prompts (interaction templates).
MCP servers expose capabilities; MCP clients (LLMs) discover and invoke
them. Transport is stdio or HTTP+SSE.

**Status.** De facto standard by mid-2026. Approximately 97 million
monthly SDK downloads. Native support in Claude, ChatGPT, Gemini,
Copilot, and Cursor. Governance transferred to the Linux Foundation
(December 2025). Over 10,000 servers indexed across public registries.

**Relevance to tiagent.** Critical. MCP is the tool integration layer
tiagent should use. Rather than building a bespoke tool system, tiagent
should expose Celestia operations (blob submission, namespace queries,
light node interactions) as MCP servers and consume them via the
standard MCP client protocol. This makes tiagent's tools accessible to
any MCP-aware LLM, not just tiagent's own agents.

---

### 5.3 A2A: Agent-to-Agent Protocol

**What.** A protocol by Google (April 2025, Apache-2.0) for direct
communication between autonomous agents across organizations and
systems. Now governed by the Linux Foundation.

**How it works.** Three primitives: Agent Cards (capability advertisements),
Tasks (structured work units), and a transport layer (HTTP, SSE,
JSON-RPC 2.0). Three-layer architecture: communication (connectivity),
syntactic (structure), semantic (shared understanding).

**Status.** v1.0.0 released January 2026. Over 150 organizations
supporting the protocol, including Google, Microsoft, AWS, Salesforce,
SAP, ServiceNow, Workday, and IBM.

**Relevance to tiagent.** High. A2A is the inter-agent coordination layer
that complements MCP's tool integration. If tiagent agents need to
delegate tasks to agents in other ecosystems (e.g., an EVM agent for
cross-chain operations), A2A provides the standard protocol. tiagent
should implement A2A Agent Cards for its agents and support A2A Tasks
for incoming delegation requests.

---

### 5.4 AITP: Agent Interaction and Transaction Protocol

**What.** A standard by NEAR AI for agent-to-agent and user-to-agent
communication across trust boundaries. Pairs a chat-thread-centric core
with an extensible capabilities system.

**How it works.** Thread-based messaging where each thread can carry
structured capabilities beyond plain text: structured UI, forms,
payments, and human-in-the-loop attestations. Designed for a world
where your personal AI assistant communicates directly with merchant
agents, service agents, and infrastructure agents.

**Status.** RFC stage. Implemented in IronClaw and NEAR AI's agent
infrastructure. Not yet widely adopted outside the NEAR ecosystem.

**Relevance to tiagent.** Medium. AITP's thread-based messaging with typed
capabilities is a cleaner model than raw JSON-RPC for agent-to-agent
conversations. However, A2A has broader adoption and ecosystem support.
tiagent should prioritize A2A but track AITP for features that A2A
lacks (particularly structured UI capabilities and payment flows).

---

### 5.5 x402: HTTP-Native Payments

**What.** An open payment standard that uses the HTTP 402 "Payment
Required" status code to embed stablecoin micropayments directly into
web requests. The most-used agentic payment protocol in 2026.

**How it works.** When an agent makes an HTTP request to a metered service,
the server responds with 402 and a payment challenge. The agent's wallet
automatically signs a payment and retries the request. V2 (January 2026)
adds session support: authenticate once, then make subsequent requests
without repeating the handshake.

**Status.** Production. 69,000 active agents, 165 million transactions,
approximately $50 million in cumulative volume as of late April 2026.
Primarily on Base and Solana (low fees, fast finality). Most
transactions settle in USDC.

**Relevance to tiagent.** High. tiagent agents will need to pay for
services: DA submissions to Celestia, API calls to LLM providers,
cross-chain bridge fees. x402 is the right protocol for autonomous
micropayments. tiagent should integrate x402 support so agents can pay
for services without pre-configured billing or human approval for each
transaction. The session support in V2 reduces overhead for repeated
interactions with the same service.

---

## 6. Gaps in the Landscape

### 6.1 No Coding Agent Learns from Usage

This is the most significant gap across the entire landscape -- both
coding agents and on-chain frameworks. Claude Code, Codex, Cursor,
Aider, and every on-chain framework treat each session as stateless.
An agent that debugged a tricky Rust lifetime issue yesterday has no
memory of the solution today. A team of ten developers each teaches
the same codebase conventions to their coding agent independently.

The closest approximation is Cursor's and Claude Code's support for
project-level instruction files (`.cursorrules`, `CLAUDE.md`), but
these are manually maintained by humans. No agent writes its own
instructions based on what worked.

tiagent's self-improvement loop -- where outcomes feed back into
prompt templates, model routing weights, gate thresholds, and
playbook extraction -- is architecturally unique across both
landscapes.

### 6.2 No Coding Agent Supports Autonomous Plan Execution with Gates

Every coding agent is single-task: you give it a prompt, it produces
output, you evaluate. None can take a multi-task plan (e.g., "implement
this PRD as 12 tasks with dependencies") and execute it autonomously,
running compilation gates, test gates, and lint gates between each task,
replanning on failure, and persisting state for resume.

This is what separates a coding assistant from a development harness.
Claude Code can write good code for a single prompt, but it cannot
execute a sprint.

### 6.3 No Coding Agent is Truly Model-Agnostic with Learned Routing

Cursor and Aider support multiple models via configuration, but the
user manually selects which model to use. No agent learns from task
outcomes to route automatically -- sending simple refactoring tasks to
cheaper, faster models and reserving expensive models for complex
architectural work. tiagent's cascade router with Thompson Sampling
addresses this directly.

### 6.4 No Coding Agent Shares Learning Across Instances

When a coding agent discovers that a particular approach works for a
codebase, that knowledge dies with the session. There is no mechanism
for one agent instance to publish what it learned so that other
instances (or other developers on the same team) benefit. tiagent's
trajectory publishing to Celestia DA and playbook extraction create
this shared learning layer.

### 6.5 No Celestia-Native Agent Framework

This is the most significant gap. Every major ecosystem -- Ethereum
(Olas, AgentKit), Solana (Solana Agent Kit), NEAR (IronClaw), Polkadot
(polkagent) -- has at least one dedicated agent framework. Celestia, as
a modular DA layer used by dozens of rollups and sovereign chains, has
none. tiagent fills this gap.

The closest infrastructure is Celestia itself (DA), Astria (shared
sequencer, which halted operations in December 2025), and various
rollup frameworks that post to Celestia. But none of these provide an
agent runtime, tool system, or coordination layer.

### 6.6 No Shared Learning Across On-Chain Agents

Every framework surveyed treats agents as statically configured. An
agent's capabilities, strategies, and knowledge are defined at
deployment time and do not change during operation. The exceptions are
narrow:

- ElizaOS has persistent memory, but this is conversation history, not
  learned strategy.
- Olas agents cycle through FSM states, but the FSM itself does not
  adapt.

No framework implements what tiagent calls the "self-improving loop":
observing outcomes, distilling patterns, updating prompts and tool
configurations, and verifying that changes improve performance.

### 6.7 Limited Composability Across Frameworks

Despite A2A and MCP, agent frameworks remain silos. An ElizaOS agent
cannot delegate to an Olas agent service. A polkagent effect pipeline
cannot include an IronClaw tool as a step. Interoperability exists at
the protocol level (A2A, MCP) but not at the runtime level.

### 6.8 Rust Underrepresentation

Three of the eight frameworks surveyed are Rust-native (polkagent,
IronClaw, Rig). The rest are TypeScript or Python. For a domain where
safety, performance, and determinism matter, Rust is underrepresented.
tiagent's choice of Rust is a differentiator.

### 6.9 Weak Authorization Models

Most frameworks have minimal or no authorization beyond wallet
permissions. Only polkagent (grant/policy system) and IronClaw (WASM
sandbox + capability manifest) have granular authorization. For agents
that handle real value, this is inadequate. tiagent needs authorization
that is at least as granular as polkagent's.

### 6.10 No DA-Aware Agent State

No framework uses a data availability layer for agent state persistence.
Agent state is stored locally (filesystem, SQLite) or on-chain (costly,
limited). Using Celestia DA for agent state -- learning checkpoints,
episode logs, tool configurations -- is a novel architectural choice
that tiagent can pioneer.

---

## 7. Lessons for tiagent

### 7.1 Adopt

| Source            | Pattern                                      | How to apply in tiagent                            |
|-------------------|----------------------------------------------|----------------------------------------------------|
| polkagent         | Effect pipeline (Intent->Claim->Attempt->Outcome) | Map tiagent's action lifecycle to this four-stage model. Every agent action becomes auditable and reversible. |
| polkagent         | Grant/policy authorization                   | Implement per-agent policy documents: which namespaces, which blob sizes, which cross-chain calls, what budget ceilings. |
| IronClaw          | WASM sandbox per tool                        | Run each tiagent tool in a WASM sandbox with capability-based permissions. Prevents tool escape and lateral movement. |
| IronClaw          | TEE credential vault                         | Keys never leave the TEE. LLM sees tool outputs, never raw keys. Critical for signing Celestia transactions. |
| IronClaw          | Prompt-injection detection                   | Add as a runtime layer before tool dispatch. |
| Coinbase AgentKit | x402 payment integration                     | Integrate x402 so agents can pay for DA, APIs, and cross-chain services autonomously. |
| Coinbase AgentKit | MCP-native tool exposure                     | Expose Celestia operations as MCP servers. |
| Olas              | On-chain component registry                  | Register tiagent agents and their capabilities in Celestia namespaces. |
| Olas              | Staking for economic security                | Require operators to stake TIA for running tiagent agents. |
| ERC-8004          | Identity + Reputation + Validation registries| Implement on Celestia using namespaces and blob submissions. |
| Rig               | Trait-based LLM composition                  | Use or mirror Rig's CompletionModel/Agent/Pipeline traits for the LLM layer. |
| A2A               | Agent Cards + Tasks                          | Publish tiagent Agent Cards for cross-ecosystem delegation. |

### 7.2 Avoid

| Source            | Anti-pattern                                 | Why to avoid                                      |
|-------------------|----------------------------------------------|---------------------------------------------------|
| Olas              | Rigid FSM coordination                       | LLM agents are non-deterministic. FSMs force you to enumerate all states upfront, which conflicts with adaptive behavior. |
| ElizaOS           | Same-process tool execution                  | No isolation means a compromised tool can access all agent state, keys, and memory. |
| ElizaOS           | Breaking changes between versions            | tiagent should commit to stable trait interfaces early. |
| Coinbase AgentKit | Centralized key management dependency        | MPC wallets through a single provider create a trust bottleneck. tiagent should support multiple signing backends. |
| Solana Agent Kit  | Single-chain coupling                        | Tight coupling to one chain's account model makes cross-chain operation painful. tiagent should abstract chain interactions behind traits. |
| 0G Labs           | Unverifiable performance claims              | Publish benchmarks with reproducible methodology. |

### 7.3 Lessons from the General Coding Agent Landscape

The coding agent space reveals what tiagent's approach makes possible
that no existing tool provides:

| Lesson                              | What it means for tiagent                                     |
|-------------------------------------|---------------------------------------------------------------|
| **Sessions are stateless by default** | tiagent's episode logger, knowledge store, and playbook extraction turn sessions into cumulative learning. Every task execution makes the next one better. This is the single largest differentiator. |
| **Single-task orientation is universal** | tiagent's plan executor with DAG scheduling, quality gates, and automatic replanning on failure enables autonomous multi-task execution. This is the difference between "write me a function" and "execute this PRD." |
| **Model lock-in is the norm** | tiagent's cascade router with learned model selection (Thompson Sampling, contextual bandits) means the right model is chosen per-task, not per-subscription. Cost drops 60-80% without quality loss. |
| **No agent shares what it learns** | tiagent's trajectory publishing to Celestia DA creates a shared learning commons. One team's hard-won debugging patterns become available to every agent on the network. |
| **Quality gates are human-dependent** | tiagent's 7-rung gate pipeline (syntax, compile, lint, unit test, integration test, diff review, e2e) runs automatically between agent actions. Bad code is caught before it compounds. |
| **CLI and IDE are separate worlds** | tiagent is CLI-first and headless-capable, meaning it runs in CI, in cron jobs, as a daemon, and as a sidecar. Not locked to any editor. |
| **Instruction files are manual** | tiagent's Dynamic Cheatsheets and playbook extraction automate what CLAUDE.md and .cursorrules require humans to maintain. |

### 7.4 tiagent's Unique Positioning

No existing tool -- coding agent or on-chain framework -- occupies
tiagent's niche. The positioning is defined by the intersection of
seven properties:

1. **General-purpose coding agent.** Competes directly with Claude Code,
   Codex, and Cursor for everyday development tasks. Not a niche
   blockchain tool.

2. **Self-improving.** The only coding agent or framework with a built-in
   learning loop: observe outcomes, distill patterns, update strategies,
   verify improvements. Every task makes the next one better.

3. **Autonomous plan execution.** The only agent that takes a PRD or task
   list and executes it as a multi-step plan with DAG scheduling, quality
   gates between tasks, and automatic replanning on failure.

4. **Model-agnostic with learned routing.** Works with any LLM provider.
   The cascade router learns which model works best for which task type,
   optimizing cost-quality tradeoffs automatically.

5. **Celestia-native.** The only agent framework that treats Celestia DA
   as the primary persistence and coordination layer, enabling shared
   learning across instances.

6. **Rust-native with WASM isolation.** Combines Rust's compile-time
   safety with per-tool WASM sandboxing (adopting IronClaw's model).

7. **Standards-first.** MCP for tools, A2A for inter-agent coordination,
   ERC-8004-equivalent for identity, x402 for payments. No bespoke
   protocols where standards exist.

The closest competitors by axis:

| Axis                   | Closest competitor       | tiagent's advantage                                  |
|------------------------|--------------------------|------------------------------------------------------|
| Coding agent quality   | Claude Code              | Self-improvement, plan execution, model-agnostic     |
| Model flexibility      | Aider                    | Learned routing, quality gates, plan execution        |
| IDE integration        | Cursor                   | Headless/CI-capable, open architecture, self-improving|
| Rust + agents          | polkagent, Rig           | Self-improvement loop, Celestia DA native             |
| Security model         | IronClaw                 | Celestia DA for verifiable state                      |
| Multi-agent coord      | Olas                     | LLM-native (not FSM-locked)                          |
| Chain integration      | Solana AK, AgentKit      | Modular DA, not single-chain                          |
| Ecosystem size         | ElizaOS                  | Rust performance, security guarantees                 |

---

## Appendix A: Framework and Agent Summary Table

**General-Purpose Coding Agents**

| Agent            | Type        | Language   | Multi-model   | Plan execution   | Self-improve | Quality gates      |
|------------------|-------------|------------|---------------|------------------|--------------|--------------------|
| Claude Code      | CLI agent   | N/A        | No (Claude)   | No               | No           | No                 |
| Codex CLI        | CLI agent   | N/A        | No (OpenAI)   | No               | No           | No                 |
| Cursor           | IDE agent   | N/A        | Yes           | No               | No           | No                 |
| Windsurf         | IDE agent   | N/A        | Yes           | No               | No           | No                 |
| Aider            | CLI agent   | Python     | Yes           | No               | No           | No                 |
| Continue         | IDE ext.    | TypeScript | Yes           | No               | No           | No                 |

**On-Chain Agent Frameworks**

| Framework        | Ecosystem   | Language   | Architecture  | Security         | Self-improve | Stars/Adoption     |
|------------------|-------------|------------|---------------|------------------|--------------|--------------------|
| polkagent        | Polkadot    | Rust       | Hexagonal     | Grant/policy     | No           | Community project  |
| IronClaw         | NEAR        | Rust       | Agent OS      | TEE + WASM       | No           | NEARCON 2026 launch|
| ElizaOS          | Multi-chain | TypeScript | Plugin/monorepo| Minimal         | No           | 23K+ GitHub stars  |
| Olas             | Ethereum    | Python     | FSM           | Staking/slashing | No           | 10M+ transactions  |
| Solana Agent Kit | Solana      | TypeScript | Plugin        | Wallet-level     | No           | SendAI ecosystem   |
| Coinbase AgentKit| Base/EVM    | TS/Python  | Skills        | MPC wallet       | No           | Coinbase-backed    |
| Rig/ARC          | Agnostic    | Rust       | Trait-based   | Type system      | No           | ~6,700 GitHub stars|
| 0G Labs          | Own chain   | N/A        | Modular DA    | Quorum + VRF     | N/A          | Mainnet Sept 2025  |

**tiagent**

| Framework        | Ecosystem   | Language   | Architecture  | Security         | Self-improve | Plan execution     |
|------------------|-------------|------------|---------------|------------------|--------------|--------------------|
| **tiagent**      | **Celestia**| **Rust**   | **Modular**   | **WASM + policy**| **Yes**      | **Yes (DAG + gates)**|

## Appendix B: Protocol Adoption Matrix

| Protocol | polkagent | IronClaw | ElizaOS | Olas | Solana AK | AgentKit | Rig | tiagent (planned) |
|----------|-----------|----------|---------|------|-----------|----------|-----|-------------------|
| MCP      | Partial   | No       | Plugin  | No   | No        | Yes      | No  | Yes               |
| A2A      | No        | No       | No      | No   | No        | No       | No  | Yes               |
| AITP     | No        | Yes      | No      | No   | No        | No       | No  | Track             |
| ERC-8004 | No        | No       | No      | No   | No        | Partial  | No  | Equivalent        |
| x402     | No        | No       | No      | No   | No        | Yes      | No  | Yes               |
