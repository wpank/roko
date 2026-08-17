# tiagent: Vision and Overview

## 1. What is tiagent?

tiagent is a **general-purpose, self-improving coding agent harness** --- a direct
alternative to Claude Code, Codex, Cursor, and Windsurf. It is written in Rust, open
source, and works with **any LLM backend** (Claude, GPT, Gemini, Llama, Mistral, Ollama,
or any OpenAI-compatible API).

What makes tiagent different from other coding agents:

- **Self-improving**: Gets better the more you use it. Cascade routing learns which models
  handle which tasks best. Adaptive gates tune quality thresholds. Playbooks extract
  reusable strategies from successful runs. Efficiency tracking identifies bottlenecks.
  This is not prompt engineering --- it is a structural feedback loop built into the runtime.
- **Collectively improving**: Through optional Celestia DA integration, tiagent instances
  can share learning artifacts --- routing weights, efficiency patterns, behavioral
  fingerprints, successful strategies --- so the entire network of agents gets smarter.
- **Agent orchestration**: Run complex multi-step tasks with parallel agent dispatch, plan
  DAGs, and automated quality gates (compile, test, lint, diff review).
- **PRD-driven development**: Write specs, generate implementation plans, and have agents
  execute them autonomously with validation at every step.
- **Long-running agents**: Not just single prompts --- agents can run for hours on complex
  tasks with snapshot-resume, state persistence, and interruption recovery.
- **Model/provider agnostic**: Use any LLM. Switch providers without changing your workflow.
  Route different task types to different models automatically.
- **Open source**: Not locked into any vendor's ecosystem. Composable via MCP, A2A, AITP,
  and x402 protocols.

tiagent is **not** a model, a prompt library, or a chat wrapper. It is the **runtime
infrastructure** that sits between an LLM and the outside world --- managing tool execution,
state persistence, learning, coordination, and verification. Think of it as an operating
system for coding agents.

### The Celestia advantage (optional, not required)

tiagent works as a **fully standalone local coding agent** with no blockchain dependencies.
But its unique differentiator is optional integration with **Celestia's data availability
(DA) layer**, which enables:

- **Shared learning across agents**: When one agent figures out an efficient strategy, all
  agents on the network can benefit. Traces, embeddings, and routing decisions are published
  as blobs in organized namespaces.
- **Verifiable agent state**: An append-only, immutable audit trail of every agent decision
  and action, verifiable by any third party.
- **Cross-agent coordination**: Multi-agent workflows where agents discover, verify, and
  learn from each other through on-chain shared state.

Think of Celestia as the network effect layer: tiagent without Celestia is a great coding
agent; tiagent with Celestia is a great coding agent that gets better from *everyone's*
experience.

### What "cybernetic" means here

The word "cybernetic" is used deliberately. tiagent implements a closed-loop control system
where agents observe the outcomes of their own actions, compare those outcomes to desired
goals, and adjust their behavior accordingly. This is not just "fine-tuning" or "prompt
engineering" --- it is a structural feedback loop built into the runtime:

1. **Observe**: Record every agent action, tool call, and outcome as a structured trace.
2. **Score**: Evaluate trace quality using multiple dimensions (task completion, efficiency,
   safety, cost).
3. **Route**: Use scoring history to select better models, prompts, and strategies for
   future tasks.
4. **Improve**: Generate concrete improvement plans (new tools, better prompts, adjusted
   routing) based on observed gaps.
5. **Verify**: Run the improved agent through the same (or similar) tasks and measure
   whether performance actually improved.
6. **Share**: Publish traces and learning artifacts to Celestia's DA layer so other agents
   can benefit from the improvement.

This loop runs continuously. Every agent execution is both a productive task and a learning
opportunity.

### Use case: Everyday software development

tiagent is designed to be used exactly like Claude Code, Codex, or Cursor --- but with
self-improvement and optional shared learning built in. Here are the primary workflows:

**Single task execution** --- give tiagent a prompt, get working code:

```bash
tiagent run "implement the login page with email/password auth"
tiagent run "add rate limiting to the /api/users endpoint"
tiagent run "refactor the payment module to use the strategy pattern"
```

**Multi-task plan execution** --- define a plan with multiple tasks, run them with
parallel dispatch and automated quality gates:

```bash
tiagent plan run plans/feature-auth/
```

Each task in the plan is executed by an agent, validated by gates (compilation, tests,
linting, diff review), and persisted. Failed tasks trigger replanning automatically.

**Full PRD-driven workflow** --- write a spec, generate a plan, execute it:

```bash
# 1. Draft a PRD from an idea
tiagent prd draft "user-authentication"

# 2. Generate an implementation plan with concrete tasks
tiagent prd plan "user-authentication"

# 3. Execute the plan autonomously
tiagent plan run plans/user-authentication/
```

The agent reads the PRD, decomposes it into tasks with dependencies, executes them in
topological order (parallelizing where possible), validates each step, and persists state
so you can resume after interruption.

**Long-running agents** --- for complex refactors or multi-hour tasks:

```bash
# Start a long-running agent on a complex task
tiagent plan run plans/major-refactor/ --resume .tiagent/state/executor.json

# Monitor progress
tiagent dashboard
tiagent status
```

Agents snapshot their state periodically. If interrupted (laptop sleeps, network drops,
you kill the process), resume exactly where you left off.

**Agent orchestration** --- multiple agents working in parallel:

```bash
# Plans with independent tasks are automatically parallelized
tiagent plan run plans/full-stack-feature/

# The executor dispatches independent tasks to separate agents,
# respects dependency ordering, and merges results through gates
```

---

## 2. Why does tiagent exist?

### The problem with current coding agents

Today's coding agents --- Claude Code, Codex, Cursor, Windsurf --- share a fundamental
limitation: **they do not learn**. Run 1 and run 1,000 behave identically. Every session
starts from zero. There is no memory of what worked, what failed, which models handle which
tasks best, or what strategies to reuse.

Beyond that:

- **Vendor lock-in**: Each tool is locked to a single model provider. Switch providers and
  you lose your entire workflow.
- **Single-task scope**: Most tools handle one prompt at a time. Complex multi-step
  development workflows (feature implementation, large refactors, PRD-to-code pipelines)
  require manual orchestration.
- **No quality gates**: Agent output is unvalidated. You get code, but nobody checked if it
  compiles, passes tests, or introduces regressions.
- **No shared learning**: When one developer's agent figures out an efficient approach to a
  common task, that knowledge is lost. There is no mechanism for agents to learn from each
  other.
- **Closed source**: Most commercial coding agents are proprietary, closed-source, and
  subscription-locked. You cannot extend, audit, or self-host them.

### The opportunity

tiagent addresses all of these gaps as a standalone coding agent. But its architecture also
enables something no other coding agent can offer: **collective improvement via Celestia**.

Celestia's architecture is uniquely well-suited for shared agent learning:

- **Modular DA layer**: Celestia separates data availability from execution and consensus.
  Agents can publish learning artifacts cheaply without needing a full execution environment.
- **Large blocks**: After the Matcha upgrade, Celestia supports 128MB blocks --- more than
  enough for agent traces, vector embeddings, and coordination artifacts.
- **Low cost**: Blob submission costs approximately $0.07--$0.81 per megabyte, making it
  economically viable to store agent learning artifacts on-chain.
- **Namespace system**: Celestia's namespace-based blob organization maps naturally to
  agent data partitioning --- one namespace per agent, per data type, per learning domain.
- **Light node infrastructure**: Celestia's light nodes can verify data availability
  without downloading full blocks, enabling lightweight agent deployments that can still
  verify shared state.
- **Growing ecosystem**: Celestia has an active developer community and a clear need for
  native agent tooling.

tiagent exists at this intersection: a **production-grade coding agent** that works
standalone, with optional Celestia integration for shared learning that no other tool can
match.

---

## 3. Prior art and influences

tiagent does not start from zero. It draws on lessons learned from two substantial prior
projects, as well as the broader landscape of on-chain agent frameworks.

### 3.1 roko

**roko** is a Rust toolkit for building agents that build themselves. It is a large system
(approximately 177,000 lines of code across 18 crates) that implements a full
plan-execute-gate-persist loop:

- An agent reads a Product Requirements Document (PRD).
- It generates an implementation plan with concrete tasks.
- It executes those tasks by dispatching work to LLM backends (Claude, GPT, Codex, Ollama,
  Gemini, and others).
- After each task, a gate pipeline validates the results (compilation, tests, linting, diff
  review).
- State is persisted so runs can be resumed after interruption.
- A learning system records agent performance and adjusts routing, prompts, and thresholds
  over time.

roko has reached the point where it can fully self-host: it reads its own PRDs, generates
plans, executes them, validates results, learns from failures, and iterates. It includes an
interactive TUI, an HTTP control plane with approximately 85 REST routes, per-agent HTTP
sidecars, MCP integration, and a knowledge store with HDC (Hyperdimensional Computing)
fingerprinting.

**What tiagent takes from roko:**

| Concept | What it is | How tiagent adapts it |
|---------|-----------|----------------------|
| Universal loop | query, score, route, compose, act, verify, write, react | Same pattern, adapted for DA-backed state |
| Signal model | Content-addressed, typed, scored data atoms | Signals become DA blobs with namespace metadata |
| 6 verb traits | Substrate, Scorer, Gate, Router, Composer, Policy | Same trait system, fewer concrete implementations |
| Gate pipeline | Multi-rung validation (compile, test, lint, diff) | Gates with on-chain attestation |
| Episode logging | Structured trace of agent turns and tool calls | Episodes published to DA layer |
| Cascade router | Model selection based on task complexity and history | Same routing, with DA-backed learning state |
| HDC fingerprints | Compact behavioral signatures for similarity matching | Fingerprints stored on DA for cross-agent matching |
| Self-improvement loop | Observe, plan, execute, validate, iterate | Same loop, with shared improvement via DA |

**What tiagent does differently from roko:**

- **Smaller crate count**: roko's 18 crates grew organically. tiagent targets 8--12 crates
  with clearer boundaries.
- **DA-optional shared learning**: roko uses local JSONL files and a `.roko/` directory.
  tiagent defaults to local storage but can optionally publish learning artifacts to
  Celestia's DA layer for cross-agent shared learning.
- **Shared learning**: roko's learning is local to a single agent instance. tiagent
  publishes learning artifacts to DA so all agents on the network can benefit.
- **No TUI/HTTP control plane in core**: roko bundles a ratatui TUI and 85-route HTTP
  server. tiagent keeps the core harness minimal and provides these as optional crates.

### 3.2 polkagent

**polkagent** is a 90-crate Rust workspace for building agents on the Polkadot ecosystem.
It demonstrates that "natively on-chain" agent frameworks are viable and valuable:

- Agents can submit extrinsics, query chain state, participate in governance, and manage
  cross-chain transfers natively.
- The framework provides deep integration with Polkadot's substrate runtime, parachain
  messaging (XCM), and staking/governance pallets.
- It includes specialized tooling for Polkadot developers (pallet scaffolding, runtime
  debugging, weight analysis).

**What tiagent takes from polkagent:**

- The conviction that "natively on-chain" agent frameworks are worth building.
- The pattern of providing deep, chain-specific developer tools (not just generic RPC
  wrappers).
- The approach of treating chain state as a first-class data source for agent context.

**What tiagent does differently from polkagent:**

- **Celestia instead of Polkadot**: Different chain, different architecture, different
  integration surface. Celestia's modular DA design requires different patterns than
  Polkadot's shared-security parachain model.
- **Fewer crates**: polkagent's 90-crate workspace is unwieldy. tiagent targets 8--12
  crates.
- **Self-improvement focus**: polkagent is a capable agent framework but does not
  emphasize self-improvement or shared learning. tiagent makes the cybernetic loop a
  first-class feature.
- **DA layer for shared state**: polkagent stores agent state off-chain or in parachain
  storage. tiagent uses Celestia's DA layer as shared, verifiable agent memory.

### 3.3 Broader landscape

tiagent competes in two categories: **general-purpose coding agents** and **on-chain agent
frameworks**.

**Coding agents** (primary competition):

| Tool | Type | Approach |
|------|------|----------|
| **Claude Code** | CLI agent | Anthropic-only, single-task, no learning |
| **Codex** | CLI agent | OpenAI-only, single-task, no learning |
| **Cursor** | IDE agent | Multi-model, IDE-coupled, no plan execution, closed source |
| **Windsurf** | IDE agent | Multi-model, IDE-coupled, no plan execution, closed source |
| **tiagent** | CLI agent | Any model, self-improving, plan DAGs + gates, open source |

tiagent's differentiators vs. coding agents: self-improvement (cascade routing, adaptive
gates, playbooks), multi-step plan execution with quality gates, PRD-to-code workflows,
model agnosticism, and optional shared learning via DA.

**On-chain agent frameworks**:

| Framework | Chain | Language | Approach |
|-----------|-------|----------|----------|
| **Eliza (ai16z)** | Multi-chain | TypeScript | Character-driven agents, social media focus, plugin system |
| **Rig** | Multi-chain | Rust | Rust LLM framework with chain adapters, not chain-native |
| **ARC** | Solana | Rust | Solana-native, DeFi-focused agent framework |
| **Zerebro** | Multi-chain | TypeScript | Creative AI agents, NFT/content generation |
| **GAME (Virtuals)** | Base | TypeScript | Agent commerce and monetization platform |
| **polkagent** | Polkadot | Rust | Deep Polkadot integration, 90 crates |
| **tiagent** | Celestia (optional) | Rust | General-purpose coding agent + cybernetic self-improvement + DA-backed shared learning |

tiagent's differentiators vs. on-chain frameworks:

1. **Works standalone**: tiagent is a full coding agent without any chain dependency. Other
   on-chain frameworks are useless without their target chain.
2. **Self-improving**: Most frameworks are static --- they run the same way every time.
   tiagent gets better with use.
3. **Shared learning via DA**: No other framework uses a DA layer for cross-agent learning.
4. **Rust + minimal**: Unlike TypeScript frameworks, tiagent provides memory safety and
   performance. Unlike polkagent, it stays small.

A detailed survey of on-chain frameworks is available in **14-on-chain-agent-survey.md**.

---

## 4. Core goals

tiagent has six core goals, listed in priority order. Every design decision should be
evaluated against these goals. Goals 1--4 define the standalone coding agent. Goals 5--6
add the Celestia integration layer.

### Goal 1: Model and provider agnosticism

tiagent must work with any LLM backend. The harness must not be coupled to any specific
model or provider. This is the foundation that makes tiagent a viable alternative to
vendor-locked tools like Claude Code and Codex.

**Supported backend types (at minimum):**

| Backend type | Examples |
|-------------|----------|
| API-based commercial | Claude (Anthropic), GPT (OpenAI), Gemini (Google) |
| API-based open | Together, Fireworks, Groq, Replicate |
| Local inference | Ollama, llama.cpp, vLLM |
| CLI-based | Claude CLI, Codex CLI |
| Custom/self-hosted | Any OpenAI-compatible API endpoint |

The backend abstraction must be a simple trait that new providers can implement in under
100 lines of code.

### Goal 2: Self-improving paradigm

Agents must get better the more they are used. Every execution is both a productive task
and a learning opportunity.

- **Local learning**: Each agent maintains a local learning state (cascade router weights,
  gate thresholds, prompt experiment results) that improves with every execution.
- **Playbook extraction**: Successful strategies (tool call sequences, prompt patterns,
  task decompositions) are extracted and reused on similar future tasks.
- **Efficiency tracking**: Per-turn cost, latency, and token usage are tracked and used to
  optimize future dispatch decisions.
- **HDC fingerprinting**: Agent behaviors and task signatures are encoded as
  Hyperdimensional Computing vectors, enabling fast similarity matching.

### Goal 3: Production-grade agent orchestration

tiagent must support complex, multi-step development workflows --- not just single prompts.

- **Plan DAG execution**: Tasks with dependencies are executed in topological order, with
  independent tasks parallelized automatically.
- **Quality gates**: Every task output is validated by a multi-rung gate pipeline
  (compilation, tests, linting, diff review, custom checks).
- **PRD-to-code pipeline**: Specs can be read, decomposed into plans, and executed
  autonomously.
- **Snapshot-resume**: Long-running agents persist state and can resume after interruption.
- **Gate failure replanning**: When a gate fails, the agent generates a revised plan
  automatically.

### Goal 4: Production-grade tool calling

Tool calling is the primary mechanism by which agents affect the world. It must be
reliable, safe, and extensible. This goal applies to all tiagent usage, not just Celestia.

- **Structured tool definitions**: Every tool has a typed schema (JSON Schema), a
  description, and documented side effects.
- **Safety controls**: Tools are classified by risk level (read-only, write, destructive).
  Agents have role-based access to tools. Destructive tools require explicit authorization.
- **MCP integration**: Any MCP-compatible tool server can be connected. tiagent ships with
  built-in MCP servers for common development tasks, and optionally for Celestia-specific
  tools.
- **Composability**: tiagent agents are composable with each other and with agents built on
  other frameworks via MCP (client + server), A2A, AITP, and x402 protocols.

### Goal 5: Native Celestia integration (optional layer)

When enabled, tiagent integrates deeply with Celestia's infrastructure for shared learning,
verifiable audit trails, and cross-agent coordination.

| Capability | Description |
|-----------|-------------|
| Blob submission | Submit arbitrary data as Celestia blobs with configurable namespace, gas, and fee parameters. |
| Namespace management | Create, organize, and query namespaces for different data types (traces, embeddings, state, proofs). |
| Light node embedding | Optionally embed a Celestia light node in the agent process for direct DA verification without relying on external full nodes. |
| Block/blob queries | Query historical blobs by namespace, height range, and content filters. |
| Fee estimation | Estimate blob submission costs before committing, enabling cost-aware agent decisions. |
| Event subscription | Subscribe to new blobs in specific namespaces for real-time coordination. |

### Goal 6: Shared learning via DA (optional layer)

When Celestia integration is enabled, agents can share learning artifacts across the
network:

- **Shared learning**: Agents periodically publish learning artifacts (scored traces, model
  routing decisions, successful strategies) to Celestia's DA layer.
- **Trajectory RAG**: Agents can retrieve relevant past trajectories (their own or others')
  from the DA layer and use them as context for current tasks.
- **Cross-agent fingerprinting**: HDC fingerprints stored on DA enable similarity matching
  across the network, so agents can find and learn from the most relevant prior work.

### Goal 7: TraceCommons and IronClaw integration

tiagent integrates with two companion systems (both optional):

- **TraceCommons**: A system for scoring trace quality and enabling trajectory
  retrieval-augmented generation (RAG). When an agent completes a task, the trace is scored
  by TraceCommons and stored in a way that enables future agents to retrieve relevant
  trajectories as context. This is the mechanism by which shared learning works at scale.
  Details in **07-tracecommons-integration.md**.

- **IronClaw**: A WASM/TEE runtime for sandboxed, verifiable agent execution. When agents
  need to run untrusted code, execute in a verified environment, or provide cryptographic
  attestation of their behavior, they can run inside IronClaw. This is optional --- most
  agent workloads do not require it --- but it is available for high-security use cases.
  Details in **08-ironclaw-integration.md**.

---

## 5. Design philosophy

tiagent follows five guiding principles. These are not aspirational --- they are hard
constraints that govern every design and implementation decision.

### 5.1 "Wire, don't build"

The most common failure mode in agent framework development is reimplementing functionality
that already exists. This was a hard-learned lesson from roko, where parallel development
tracks frequently produced duplicate implementations of the same feature.

**The rule**: Before building anything new, check whether existing code (in tiagent, in
Celestia client libraries, in MCP SDKs, in TraceCommons) just needs to be connected.
If your change is not observable through the CLI or through a running agent, it is probably
wrong.

**Concretely:**

- Use `celestia-node`'s existing RPC client rather than reimplementing blob submission.
- Use the `mcp-sdk` crate rather than reimplementing MCP protocol handling.
- Use `serde` and `schemars` for tool schemas rather than inventing a custom schema system.
- When a feature exists but is not wired to the runtime, wire it --- do not build a second
  copy.

### 5.2 Minimal viable crate count

polkagent's 90-crate workspace and roko's 18-crate workspace demonstrate that large crate
counts create maintenance burden, slow compilation, and make the dependency graph hard to
reason about.

**The target**: 8--12 crates, each with a clear, non-overlapping responsibility.

**The test**: If you cannot explain a crate's purpose in one sentence, it should be merged
into another crate. If a crate has fewer than 500 lines of code, it should probably be a
module within another crate.

The proposed crate structure is detailed in **03-crate-structure.md**.

### 5.3 Works without Celestia, better with Celestia

tiagent must be a **fully functional coding agent with zero blockchain dependencies**. A
developer should be able to install tiagent, point it at an LLM, and start using it
immediately --- no Celestia node, no wallet, no tokens, no configuration beyond an API key.

Celestia integration is an **optional layer** that unlocks shared learning, verifiable
audit trails, and cross-agent coordination. It makes tiagent better, but it is not a
prerequisite for tiagent being useful.

**Concretely:**

- The `Substrate` trait abstracts over storage backends. `LocalSubstrate` (writes to local
  files) is the default. `CelestiaSubstrate` is opt-in. The trait could also be implemented
  for Avail, EigenDA, or other DA layers.
- Agent logic never calls Celestia RPC directly. It always goes through the `Substrate`
  trait.
- All self-improvement features (cascade routing, adaptive gates, playbook extraction,
  efficiency tracking) work locally without any DA integration.
- When Celestia is enabled, learning artifacts are *also* published to the DA layer,
  enabling shared improvement across agents on the network.
- Configuration specifies which DA backend to use. The default is `local`, not `celestia`.

### 5.4 Self-hosting target

The ultimate test of an agent framework is whether it can develop itself. tiagent should
reach the point where:

1. A developer writes a PRD describing a new tiagent feature.
2. A tiagent agent reads the PRD and generates an implementation plan.
3. The agent executes the plan, writing Rust code, running tests, and validating results.
4. The agent publishes the trace to Celestia's DA layer.
5. Future agents can retrieve that trace and learn from it.

This is not a day-one goal. But every design decision should be evaluated against the
question: "Does this move us closer to or further from self-hosting?"

---

## 6. Target users

tiagent is designed for five user groups, listed in priority order. The primary audience is
general software developers --- everyone else benefits from the same core harness with
additional capabilities layered on.

### 6.1 Software developers (primary audience)

**Who**: Any developer who wants a self-improving coding assistant that is not locked to a
single vendor.

**What they need**:
- A coding agent that works with their preferred LLM provider (or local models)
- Single-task execution: "implement this feature", "fix this bug", "refactor this module"
- Multi-step plan execution with automated quality validation
- An agent that gets better the more they use it --- learning which models work best for
  which tasks, extracting reusable strategies, adapting quality thresholds
- Open source and extensible, not subscription-locked

**How tiagent serves them**:
- Model-agnostic dispatch: Claude, GPT, Gemini, Ollama, llama.cpp, any OpenAI-compatible API
- `tiagent run "<prompt>"` for single tasks with the full universal loop
- `tiagent plan run <dir>` for multi-task plans with parallel dispatch and gate validation
- Cascade router learns optimal model selection over time
- Playbook extraction captures successful strategies for reuse
- Adaptive gate thresholds tune strictness based on task type and history
- Efficiency tracking identifies cost and latency bottlenecks
- Snapshot-resume for long-running agents

### 6.2 Teams wanting PRD-driven autonomous development

**Who**: Engineering teams that want to go from specs to working code with minimal manual
intervention.

**What they need**:
- A workflow that takes a product requirement and produces validated, tested code
- Automated task decomposition with dependency management
- Quality gates at every step (not just "generate code and hope it works")
- Resumable execution for multi-hour workflows

**How tiagent serves them**:
- `tiagent prd draft` / `tiagent prd plan` / `tiagent plan run` --- full PRD-to-code pipeline
- Plan DAG executor with topological ordering and parallel dispatch
- Multi-rung gate pipeline: compilation, tests, linting, diff review, custom checks
- Gate failure triggers automatic replanning
- State persistence with `--resume` for interruption recovery

### 6.3 Celestia developers

**Who**: Developers building applications, rollups, and infrastructure on Celestia.

**What they need**:
- Agent-assisted development: "Write a Rollkit config that submits to Mocha testnet"
- Chain interaction tools: blob submission, namespace queries, fee estimation
- Development workflow automation: testing, deployment, monitoring
- Context-aware agents that understand Celestia's architecture and APIs

**How tiagent serves them**:
- Everything in 6.1 and 6.2 (tiagent is a full coding agent first)
- Built-in Celestia MCP server with 20+ chain-specific tools
- System prompts pre-loaded with Celestia documentation and best practices
- Tool definitions for common Celestia development tasks
- Integration with Celestia's testnet and mainnet infrastructure

### 6.4 Agent operators

**Who**: Teams and individuals running agents in production who need verifiability,
coordination, and audit trails.

**What they need**:
- On-chain audit trails for agent decisions and actions
- Multi-agent coordination through shared state
- Verifiable execution for compliance and trust
- Cost-efficient on-chain storage for agent data

**How tiagent serves them**:
- Every agent action is recorded as a structured trace
- Traces are published to Celestia's DA layer in organized namespaces
- Agents can discover and coordinate with each other through namespace subscriptions
- Light node embedding enables lightweight verification without full node infrastructure
- Fee estimation and cost tracking for on-chain storage budgeting

### 6.5 Researchers

**Who**: Researchers exploring shared agent learning, verifiable AI, and decentralized
coordination.

**What they need**:
- A platform for experimenting with cross-agent learning through shared DA
- Mechanisms for trajectory retrieval and behavioral comparison
- HDC fingerprinting for agent behavior analysis
- Verifiable execution environments (WASM/TEE) for reproducible experiments

**How tiagent serves them**:
- TraceCommons integration for trace quality scoring and trajectory RAG
- HDC fingerprint computation and similarity search across DA-stored traces
- IronClaw integration for WASM-sandboxed, attestable experiments
- Open trace format that enables third-party analysis tools

---

## 7. Architecture overview

This section provides a high-level overview of tiagent's architecture. The full
architectural design is in **02-architecture.md**.

### 7.1 The universal loop

tiagent's core execution model is a **universal loop** with eight stages:

```
query -> score -> route -> compose -> act -> verify -> write -> react
```

| Stage | What happens | Key trait |
|-------|-------------|-----------|
| **Query** | Receive a task or input signal. | (entry point) |
| **Score** | Evaluate the signal's priority, complexity, and relevance. | `Scorer` |
| **Route** | Select the appropriate model, prompt strategy, and tool set. | `Router` |
| **Compose** | Assemble the system prompt, context, and tool definitions. | `Composer` |
| **Act** | Dispatch the composed prompt to an LLM backend and execute tool calls. | (dispatch) |
| **Verify** | Run gate checks on the action's results (compile, test, lint, diff). | `Gate` |
| **Write** | Persist the results to the storage substrate (DA layer, local state). | `Substrate` |
| **React** | Emit events, update learning state, trigger downstream tasks. | `Policy` |

Every agent execution follows this loop. The loop can be nested (a "react" stage can
trigger a new "query") and parallelized (multiple signals can flow through the loop
concurrently).

### 7.2 The signal model

The fundamental data type in tiagent is the **Signal**: a content-addressed, typed, scored
datum that flows through the universal loop.

```rust
pub struct Signal {
    pub hash: Hash,           // Content-addressed identity (SHA-256)
    pub kind: SignalKind,     // What type of data this is
    pub payload: Payload,     // The actual data (JSON, bytes, or structured)
    pub score: f64,           // Relevance/priority score (0.0 to 1.0)
    pub metadata: Metadata,   // Timestamps, source, provenance
    pub da_ref: Option<DaRef>, // Reference to DA layer blob (if published)
}
```

Signals are the atoms of tiagent's data model. Everything --- tasks, tool results, traces,
learning artifacts, coordination messages --- is a Signal.

### 7.3 The six verb traits

tiagent's behavior is defined by six traits that correspond to the verbs in the universal
loop:

| Trait | Verb | Responsibility |
|-------|------|---------------|
| `Substrate` | write/read | Persist and retrieve signals (DA layer, local FS, hybrid) |
| `Scorer` | score | Evaluate signal priority, complexity, and relevance |
| `Gate` | verify | Validate action results against quality criteria |
| `Router` | route | Select model, prompt strategy, and tool set |
| `Composer` | compose | Assemble system prompts, context, and tool definitions |
| `Policy` | react | Define rules for event emission, state updates, and cascading |

Each trait has a well-defined interface. Concrete implementations are pluggable. For
example, the `Substrate` trait has a `CelestiaSubstrate` implementation (writes to DA
layer), a `LocalSubstrate` implementation (writes to local files), and a
`HybridSubstrate` implementation (writes to both with configurable sync policy).

### 7.4 Crate structure (summary)

The full crate structure is defined in **03-crate-structure.md**. At a high level:

| Crate | Responsibility |
|-------|---------------|
| `tiagent-core` | Signal type, 6 verb traits, errors, config |
| `tiagent-agent` | LLM backend dispatch, tool loop, safety |
| `tiagent-celestia` | Celestia DA integration (blobs, namespaces, light node) |
| `tiagent-gate` | Gate pipeline (compile, test, lint, custom checks) |
| `tiagent-compose` | Prompt assembly, templates, context management |
| `tiagent-learn` | Episodes, routing, experiments, shared learning |
| `tiagent-tools` | Built-in tools, MCP client/server, Celestia dev tools |
| `tiagent-cli` | CLI binary, subcommands, configuration |

Additional optional crates may include `tiagent-serve` (HTTP API), `tiagent-tui`
(terminal UI), and `tiagent-ironclaw` (WASM/TEE integration).

---

## 8. Celestia integration overview

This section summarizes how tiagent integrates with Celestia. The full design is in
**04-celestia-integration.md** and **05-da-storage-patterns.md**.

### 8.1 Why DA for agent state?

Traditional agent frameworks store state in local files, databases, or cloud storage.
tiagent uses Celestia's DA layer because:

1. **Shared and verifiable**: Any agent can read any other agent's published state. Data
   availability sampling ensures the data is actually available.
2. **Append-only**: The DA layer is immutable. Once a trace is published, it cannot be
   altered. This provides a trustworthy audit trail.
3. **Namespace-organized**: Celestia's namespace system maps naturally to agent data
   partitioning (one namespace per agent, per data type, per coordination group).
4. **Economically viable**: At $0.07--$0.81 per MB, storing agent traces (typically
   10--100KB per task) costs fractions of a cent.
5. **Decentralized**: No single point of failure. Agent state survives the shutdown of any
   individual node, server, or cloud provider.

### 8.2 What goes on-chain?

Not everything goes on Celestia. tiagent uses a tiered storage model:

| Data type | Storage | Rationale |
|-----------|---------|-----------|
| Execution traces | DA layer | Shared learning, audit trail, coordination |
| HDC fingerprints | DA layer | Cross-agent similarity matching |
| Learning summaries | DA layer | Shared model routing and prompt strategies |
| Coordination proofs | DA layer | Multi-agent workflow verification |
| Raw LLM responses | Local only | Too large, not useful for sharing |
| Intermediate state | Local only | Ephemeral, not worth the cost |
| Secrets and credentials | Local only (encrypted) | Must never go on-chain |
| Full source code artifacts | Local only | Too large for DA |

### 8.3 Namespace schema

tiagent uses a structured namespace schema to organize on-chain data:

```
tiagent/<version>/<data-type>/<agent-id-or-group>
```

Examples:

| Namespace | Contents |
|-----------|----------|
| `tiagent/v1/traces/agent-abc123` | Execution traces for a specific agent |
| `tiagent/v1/fingerprints/global` | HDC fingerprints from all agents |
| `tiagent/v1/learning/router-weights` | Shared cascade router weights |
| `tiagent/v1/coord/group-xyz` | Coordination messages for a multi-agent group |

---

## 9. Self-improvement overview

This section summarizes tiagent's self-improvement mechanisms. The full design is in
**11-self-improving-loop.md**.

### 9.1 The feedback loop

tiagent implements a continuous feedback loop at three levels:

**Level 1: Per-execution learning (automatic)**
- After every task, the agent records an episode (structured trace of all turns, tool
  calls, and outcomes).
- The episode is scored on multiple dimensions (task completion, efficiency, cost, safety).
- Model routing weights are updated based on which models performed well on which task
  types.
- Gate thresholds are adjusted using exponential moving averages of pass/fail rates.

**Level 2: Cross-execution learning (periodic)**
- Periodically, the agent analyzes its accumulated episodes to identify patterns.
- Successful strategies are extracted and stored as "playbooks" --- reusable templates for
  common task types.
- Failed strategies are flagged to avoid in future routing decisions.
- Learning artifacts are published to the DA layer for other agents to consume.

**Level 3: Cross-agent learning (via DA layer)**
- Agents can query the DA layer for traces from other agents that handled similar tasks.
- TraceCommons scores these traces for quality, enabling agents to preferentially learn
  from high-quality trajectories.
- HDC fingerprint similarity enables fast retrieval of relevant traces without full-text
  search.

### 9.2 What "self-improving" means concretely

To be concrete, here are examples of self-improvement tiagent performs:

| Improvement | Mechanism | Persistence |
|-------------|-----------|-------------|
| Route complex tasks to stronger models | Cascade router weight updates | Local + DA |
| Avoid prompts that cause hallucination | Prompt experiment A/B testing | Local + DA |
| Adjust gate strictness based on task type | EMA-based threshold adaptation | Local + DA |
| Reuse successful tool call sequences | Playbook extraction from episodes | Local + DA |
| Learn from other agents' blob submission patterns | Trajectory RAG from DA layer | DA-sourced |
| Pre-populate context with relevant prior work | HDC fingerprint similarity search | DA-sourced |

---

## 10. Interoperability overview

This section summarizes tiagent's interoperability story. The full design is in
**09-interop-protocols.md**.

### 10.1 Protocol support matrix

| Protocol | Direction | Status target | Use case |
|----------|-----------|---------------|----------|
| **MCP** | Client | MVP | Connect to any MCP tool server (GitHub, databases, file systems) |
| **MCP** | Server | MVP | Expose tiagent's Celestia tools to Claude, Cursor, and other MCP clients |
| **A2A** | Client | Post-MVP | Discover and invoke agents on other platforms |
| **A2A** | Server | Post-MVP | Expose tiagent agents as A2A-discoverable services |
| **AITP** | Client | Post-MVP | Structured communication with Near ecosystem agents |
| **AITP** | Server | Post-MVP | Accept structured messages from Near ecosystem agents |
| **x402** | Client | Post-MVP | Pay for external agent services with HTTP 402 flows |
| **x402** | Server | Post-MVP | Sell tiagent agent services for payment |

### 10.2 MCP as the primary integration surface

MCP (Model Context Protocol) is tiagent's primary interoperability mechanism for the MVP.
Every tiagent tool is exposed as an MCP tool, and every external MCP server can be connected
as a tool source. This means:

- A tiagent agent can use any tool available in the MCP ecosystem (thousands of tools across
  hundreds of servers).
- Any MCP client (Claude Desktop, Cursor, VS Code with Continue, etc.) can access tiagent's
  Celestia tools.
- tiagent's Celestia MCP server can be used independently of the agent harness --- just
  connect it to your preferred LLM client.

---

## 11. Comparison summary

### vs. coding agents (primary competition)

| Dimension | tiagent | Claude Code | Codex | Cursor | Windsurf |
|-----------|---------|------------|-------|--------|----------|
| **Model agnostic** | Yes (any LLM) | No (Claude only) | No (OpenAI only) | Partial | Partial |
| **Self-improving** | Yes | No | No | No | No |
| **Shared learning** | Yes (via DA) | No | No | No | No |
| **Multi-step plans** | Yes (DAG executor) | No | No | No | No |
| **PRD-to-code** | Yes | No | No | No | No |
| **Quality gates** | Yes (multi-rung) | No | No | No | No |
| **Snapshot-resume** | Yes | No | Partial | No | No |
| **Open source** | Yes | No | Partial | No | No |
| **IDE-independent** | Yes (CLI-first) | Yes (CLI) | Yes (CLI) | No (IDE) | No (IDE) |
| **MCP support** | Client + Server | Client | No | Client | Client |
| **Vendor lock-in** | None | Anthropic | OpenAI | Subscription | Subscription |

### vs. on-chain agent frameworks

| Dimension | tiagent | roko | polkagent | Eliza | Rig |
|-----------|---------|------|-----------|-------|-----|
| **Language** | Rust | Rust | Rust | TypeScript | Rust |
| **Target chain** | Celestia (optional) | None (local) | Polkadot | Multi-chain | Multi-chain |
| **Standalone mode** | Yes (full agent) | Yes | No | Yes | Yes |
| **Self-improving** | Yes (DA-shared) | Yes (local) | No | No | No |
| **Shared learning** | Yes (via DA) | No | No | No | No |
| **Model agnostic** | Yes | Yes | Yes | Partial | Yes |
| **MCP support** | Client + Server | Client + Server | No | No | No |
| **Tool system** | Typed + safe | Typed + safe | Chain-specific | Plugin-based | Typed |
| **Self-hosting goal** | Yes | Achieved | No | No | No |

---

## 12. Why not just use Claude Code?

Claude Code is excellent. So are Codex and Cursor. If you just need a coding agent that
works right now, use them. tiagent is for developers who want more:

| Limitation of current tools | How tiagent addresses it |
|---------------------------|------------------------|
| **No learning**: Run 1 and run 1,000 behave identically. | tiagent's cascade router, adaptive gates, playbook extraction, and efficiency tracking mean the agent genuinely improves with use. |
| **Single-model lock-in**: Claude Code only works with Claude, Codex only with OpenAI. | tiagent dispatches to any LLM backend. Use Claude for complex reasoning, GPT for code generation, Ollama for quick local tasks --- or let the cascade router choose automatically. |
| **Single-task scope**: One prompt, one response. Complex workflows require manual orchestration. | tiagent executes plan DAGs with parallel agent dispatch, dependency ordering, and automated quality gates. |
| **No quality validation**: The agent generates code, but nobody checks if it compiles, passes tests, or introduces regressions. | tiagent's gate pipeline validates every task output: compilation, tests, linting, diff review, custom checks. Failures trigger automatic replanning. |
| **No PRD workflow**: You can prompt for individual tasks, but there is no path from "spec" to "validated implementation." | tiagent's PRD pipeline: `prd draft` -> `prd plan` -> `plan run`. Write a spec, generate tasks, execute autonomously with validation. |
| **No shared learning**: Your agent's experience benefits only you, and only within a single session. | tiagent can optionally publish learning artifacts to Celestia's DA layer, enabling collective improvement across all tiagent instances. |
| **Closed source**: You cannot extend, audit, or self-host. | tiagent is open source, written in Rust, and fully extensible through traits and MCP. |

If you do not need self-improvement, plan execution, or shared learning, Claude Code is
simpler. tiagent is for developers who want their tools to compound over time.

---

## 13. What comes next

This document provides the vision and overview. The rest of the document suite fills in
the details:

1. **02-architecture.md**: Full architectural design --- traits, types, runtime structure,
   and data flow.
2. **03-crate-structure.md**: Workspace layout, crate boundaries, and dependency graph.
3. **04-celestia-integration.md**: Detailed Celestia integration design.
4. **05-da-storage-patterns.md**: Patterns for storing agent data on Celestia.
5. **06-tool-system.md**: Tool calling, MCP, and Celestia developer tools.
6. **07-tracecommons-integration.md**: Trace quality and trajectory RAG.
7. **08-ironclaw-integration.md**: WASM/TEE runtime integration.
8. **09-interop-protocols.md**: Protocol interoperability details.
9. **10-design-patterns.md**: Patterns catalog.
10. **11-self-improving-loop.md**: Cybernetic self-improvement loop.
11. **12-prd-core-harness.md**: Core harness MVP requirements.
12. **13-prd-celestia-native.md**: Celestia-native feature requirements.
13. **14-on-chain-agent-survey.md**: On-chain agent framework survey.
14. **15-deep-research-queries.md**: Open research questions.

The recommended next read depends on your interest:

- **General developers**: **02-architecture.md** (core abstractions), then
  **11-self-improving-loop.md** (how the agent gets better with use).
- **Technical readers**: **02-architecture.md** for the full architectural design.
- **Product readers**: **12-prd-core-harness.md** for the standalone agent harness MVP.
- **Celestia developers**: **04-celestia-integration.md** for the DA integration layer.
