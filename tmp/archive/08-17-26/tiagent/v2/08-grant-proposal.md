# Grant Proposal: Celestia Foundation Strategic Ecosystem Grant

---

**Program:** Celestia Foundation Strategic Ecosystem Grant
**Requested Amount:** USD $200,000
**Duration:** 12 months, 6 milestones
**Applicant:** [Template -- to be filled by applicant]
**License:** MIT / Apache-2.0 (dual-licensed)
**Contact:** [Template -- direct engagement with Celestia Foundation ecosystem team]

---

## Abstract

tiagent is an open-source coding agent -- a direct alternative to Claude Code,
Codex, and Cursor -- that uses Celestia's data availability layer as a shared
learning backbone.

AI coding agents are a $22.6-27 billion market as of mid-2026. Millions of
developers use Claude Code, Codex, and Cursor daily. These tools are powerful,
but they share a structural limitation: learning is siloed. Claude Code learns
from your session, but that learning stays locked inside Anthropic's
infrastructure. Your debugging strategies, deployment patterns, and
architectural insights benefit no one else, and you never benefit from theirs.

tiagent breaks this pattern. It is a general-purpose coding agent --
model-agnostic, open-source, written in Rust -- that competes directly with
commercial coding agents on capability. It writes code, runs tests, debugs,
refactors, and deploys, using Claude, GPT, Gemini, Llama, or any
OpenAI-compatible model. What makes it novel is the learning layer: tiagent
publishes learning artifacts (traces, routing weights, embeddings, behavioral
fingerprints) to Celestia's DA layer as namespace-organized blobs. Other
tiagent instances retrieve relevant trajectories, bootstrap from collective
routing experience, and verify the provenance of learned strategies -- all
through the same DA infrastructure that rollups use for block data.

The result: every developer who uses tiagent becomes a Celestia DA consumer,
whether or not they know or care about blockchain. A Python developer debugging
a Django app benefits from Celestia DA because the collective learning pool
that makes tiagent smarter is stored there. This creates a new DA consumer
category -- AI agent coordination and learning data -- that could rival rollup
block data in volume as adoption grows.

Meanwhile, 0G Labs has positioned itself as "the AI blockchain" with $108M in
AI-specific funding despite early-stage testnet infrastructure. Celestia has
superior production capabilities (128 MB blocks, 56+ rollups, DAS light nodes,
29-byte namespaces) but zero AI-specific investment. tiagent is the concrete
response: working code, not marketing budget.

This proposal funds 12 months of development across six milestones, taking
tiagent from design to production deployment. The target market is not the 56
Celestia rollup teams -- it is the millions of developers who currently use
commercial coding agents and would benefit from one that gets smarter from
collective experience.

---

## 1. Problem Statement

### 1.1 Celestia has no AI narrative

Celestia is the leading modular blockchain by every objective metric --
mainnet maturity, rollup adoption, economic activity, technical throughput.
But in the rapidly growing AI agent infrastructure market, Celestia is
invisible.

The numbers are stark:

| Ecosystem | AI-specific funding | AI grants | AI branding |
|---|---|---|---|
| **0G Labs** | $88.88M ecosystem fund + $20M Apollo Accelerator | Dedicated AI grants up to $1M+ | Explicit: "The AI Blockchain" |
| **Filecoin** | $3.68M ProPGF Batch 1 (includes AI/ML) | AI-aware grant categories | Moderate: "Compute + Storage for AI" |
| **Ethereum** | ESP rolling grants (AI-adjacent) | No dedicated AI track | None |
| **Celestia** | **$0** | **None** | **None** |

0G Labs has spent $108 million on AI-specific programs:

- **$88.88M AI Ecosystem Fund** funding projects on 0G's DA + execution stack
- **$20M Apollo AI Accelerator** (with CoinFund and Hack VC) targeting AI +
  blockchain startups
- **Dedicated AI Labs** building reference implementations for AI data
  availability
- **Explicit positioning**: "0G is purpose-built for AI"

The result: when developers think "AI + data availability," they think 0G, not
Celestia. This is despite 0G having smaller block sizes, no production rollup
ecosystem, and no DAS light node infrastructure.

| Capability | Celestia | 0G Labs |
|---|---|---|
| Mainnet status | Production (since Oct 2023) | Testnet |
| Rollup ecosystem | 56+ production rollups | 0 (pre-launch) |
| Block size | 128 MB (post-Matcha), 1 Tb/s target | 50 MB target |
| DAS implementation | Production (lumina-node v1.0) | Not available |
| Light nodes | Production Rust implementation | Not available |
| Total funding | $204M ($100M Series C, Sep 2024) | $290M, pre-revenue |
| AI-specific funding | $0 | $108.88M |

The gap is not technical -- Celestia's infrastructure is objectively superior.
The gap is that no one has built the tooling that demonstrates why Celestia is
better for AI. tiagent closes this gap.

### 1.2 No coding agent learns from usage

Every coding agent on the market today -- Claude Code, Codex, Cursor, Windsurf
-- is fundamentally static from the user's perspective. They use whatever model
the vendor provides. They do not learn from your successes. They do not adjust
their routing based on which model performed best for your task types. They do
not share strategies across users.

The learning problem has three dimensions:

- **No local self-improvement.** When a model fails at a task and succeeds on
  retry with a different approach, no existing agent records that outcome and
  adjusts future behavior.
- **No cross-session learning.** When you solve a complex deployment problem
  in one session, that strategy is lost when the session ends. The next time
  you face a similar problem, the agent starts from zero.
- **No collective learning.** When millions of developers use the same agent,
  each one discovers the same patterns independently. There is no mechanism to
  share learned strategies across users without a centralized, vendor-controlled
  infrastructure.

### 1.3 No Celestia-native agent tooling exists

Every major blockchain ecosystem except Celestia has at least one dedicated
agent framework:

| Framework | Chain | What it does | Celestia support |
|---|---|---|---|
| Eliza (ai16z) | Multi-chain | Character-driven social agents | None |
| Rig | Multi-chain | Rust LLM framework with chain adapters | None |
| ARC | Solana | Solana-native DeFi agents | None |
| Solana Agent Kit | Solana | Wallet/DeFi tools for agents | None |
| Coinbase AgentKit | Base | Commerce/wallet agent tools | None |
| polkagent | Polkadot | 90-crate deep Polkadot integration | None |
| IronClaw | NEAR | WASM/TEE sandboxed agent runtime | None |

Celestia's modular DA-only architecture requires fundamentally different
integration patterns than execution-layer chains. Polkadot's shared-security
parachain model, Solana's account-based execution, and Base's EVM have nothing
in common with Celestia's blob/namespace/DAS model. No one has figured out how
to use a DA layer as shared agent memory. tiagent fills this gap.

---

## 2. Proposed Solution

### 2.1 What tiagent is

tiagent is a general-purpose coding agent with Celestia-powered collective
learning. It is a Rust toolkit (targeting 12-14 crates, MIT/Apache-2.0) that
provides:

- **A universal execution loop**: query, score, route, compose, act, verify,
  write, react. Every agent task follows this pattern.
- **Model-agnostic LLM dispatch**: Claude, GPT, Gemini, Llama, Mistral,
  Ollama, and any OpenAI-compatible API. The harness routes intelligently
  based on task complexity, historical performance, and cost constraints.
- **Celestia DA substrate**: Agent traces, embeddings, routing weights, and
  behavioral fingerprints stored as namespace-organized blobs on Celestia.
- **Cybernetic self-improvement**: Three nested feedback loops (per-task,
  cross-session, cross-agent) that observe performance, adjust strategy, and
  share learning.
- **Tool system with MCP integration**: Built-in tools plus Model Context
  Protocol (MCP, 97M+ monthly SDK downloads) client and server support.

### 2.2 The standalone-to-collective gradient

tiagent is designed to work at every point on a spectrum:

**Fully standalone (no Celestia).** Install tiagent, configure an LLM
backend, run coding tasks. All learning persists locally in `.tiagent/`. The
agent improves from its own experience through cascade routing and adaptive
gate thresholds. This alone competes with Claude Code and Codex.

**Better with Celestia.** Enable Celestia integration and the agent publishes
learning artifacts to the DA layer. It also consumes learning from other
agents. Routing weights from the network help new agents bootstrap. Trajectory
RAG provides in-context examples from similar successful tasks. The agent gets
smarter from collective experience.

**Every user becomes a DA consumer.** This is the Trojan horse. Developers
adopt tiagent because it is a better coding agent. The Celestia DA consumption
is invisible to them -- they just see an agent that gets smarter over time.
But every trace published, every routing weight shared, every trajectory stored
is a Celestia blob generating DA fees and growing the ecosystem.

### 2.3 How Celestia fits

Using a data availability layer as a substrate for cross-agent shared learning
is genuinely new. No existing agent framework does this. Celestia's properties
map directly to the requirements:

| Requirement | Celestia property |
|---|---|
| Append-only learning corpus | Immutable blob storage |
| Data partitioning by type | 29-byte namespace system |
| Verifiable provenance | NMT inclusion proofs |
| Cheap verification | DAS light nodes (~1-5% of block data) |
| Permissionless participation | Open blob submission |
| No single point of control | Decentralized validator set |

The namespace hierarchy organizes agent data:

```
tiagent/
  +-- system/                   Global config, agent registry
  +-- traces/                   Episode traces (actions + outcomes)
  |   +-- traces/<agent-id>     Per-agent traces
  |   +-- traces/shared         Cross-agent shared traces
  +-- routing/                  Cascade router weights, model stats
  +-- vectors/                  Sentence embeddings for trajectory RAG
  +-- fingerprints/             HDC behavioral fingerprints
  +-- coordination/             Multi-agent coordination proofs
```

---

## 3. Technical Approach

### Architecture overview

The core architecture follows a "1 noun + 6 verbs" model:

**The noun**: Signal. Every piece of data flowing through tiagent is a Signal
-- a content-addressed, typed, scored datum with metadata.

**The six verbs** (Rust traits):

| Trait | What it does |
|---|---|
| `Substrate` | Reads and writes signals. Implementations: `CelestiaSubstrate` (DA blobs), `LocalSubstrate` (JSONL files). |
| `Scorer` | Evaluates signal quality across dimensions (completion, efficiency, safety, cost). |
| `Gate` | Validates agent outputs against criteria (compilation, tests, lint, diff review). Multi-rung pipeline. |
| `Router` | Selects models, prompts, and strategies based on task features and historical performance. |
| `Composer` | Assembles system prompts from layered templates, context, and task specifications. |
| `Policy` | Enforces safety contracts, budget limits, and behavioral constraints. |

**The universal loop:**

```
     +-----------------------------------------------------+
     |                                                     |
     v                                                     |
  Query -> Score -> Route -> Compose -> Act -> Verify -> Write -> React
                                         |                        |
                                         v                        |
                                   LLM + Tools                    |
                                   (model-agnostic)               |
                                                                  |
                                         Celestia DA <------------+
                                         (namespace-organized blobs)
```

### Self-improvement: three nested loops

**Inner loop (per-task):** Within a single task, the agent observes tool call
outcomes, adjusts strategy (retries with different parameters, switches tools),
and records the execution trace.

**Middle loop (across tasks):** Across multiple executions, the cascade router
observes which models, prompts, and strategies produced the best results for
which task types. It adjusts routing weights via exponential moving average
(EMA) updates. The gate pipeline tracks per-rung pass rates and adjusts
thresholds adaptively.

**Outer loop (across agents):** Agents publish learning artifacts to Celestia's
DA layer. Other agents retrieve relevant trajectories via embedding similarity
(trajectory RAG), bootstrap from published routing weights, and use HDC
fingerprint matching to find behaviorally similar agents whose strategies are
likely transferable.

The outer loop is the novel contribution. No existing agent framework
implements cross-agent shared learning through a verifiable, append-only data
layer.

**Knowledge demurrage:** tiagent applies a Gesellian tax on stored knowledge --
entries that are not actively validated by successful task outcomes decay over
time. This creates natural alignment with Celestia's 7-day pruning window:
unreinforced knowledge dies before pruning matters, so agents must continuously
publish fresh artifacts to maintain collective intelligence. The result is a
self-selecting network where only validated knowledge survives, driving
sustained DA consumption as a byproduct of agent quality maintenance rather
than artificial incentives.

---

### Milestone 1: Core Harness (Months 1-2)

**Budget:** USD $30,000

The foundational runtime. This milestone delivers a working agent harness that
can execute tasks, validate results, and persist traces -- without Celestia
integration.

**Deliverables:**

1. **Signal types and core traits.** The `tiagent-core` crate implementing
   the Signal data type, the six verb traits, and the universal loop. All
   traits have default implementations and mock backends for testing.

2. **CLI binary.** The `tiagent-cli` crate providing `tiagent run "<prompt>"`,
   `tiagent status`, and `tiagent doctor` subcommands. Enough to run a single
   agent through a task and see the result.

3. **Two LLM backends.** Claude API and OpenAI-compatible backends in the
   `tiagent-agent` crate. Model-agnostic dispatch through the `Router` trait.
   At least one local model backend (Ollama) for development without API keys.

4. **Local substrate.** The `tiagent-store` crate implementing `Substrate`
   over local JSONL files. Agent traces, episodes, and learning state persist
   to a `.tiagent/` directory.

5. **Episode logger.** Structured recording of every agent turn (tool calls,
   model responses, timestamps, token counts, outcomes) in a replayable
   format compatible with TraceCommons envelope schema.

**Verification criteria:**

```bash
cargo run -p tiagent-cli -- run "list files in the current directory"
```

Executes successfully with both Claude and OpenAI backends, producing a valid
episode trace in `.tiagent/episodes.jsonl`. The trace is parseable, contains
timestamped tool calls and model responses, and conforms to the TraceCommons
envelope schema.

**Engineering hours:** ~480 hours (1.5 FTE x 2 months)
**Infrastructure:** LLM API costs for testing (~$500)

---

### Milestone 2: Celestia DA Substrate (Months 3-4)

**Budget:** USD $40,000

The Celestia integration layer. This is where tiagent becomes Celestia-native.

**Deliverables:**

1. **CelestiaSubstrate implementation.** The `tiagent-celestia` crate
   implementing the `Substrate` trait over Celestia's blob API. Submit and
   retrieve blobs through namespace-organized storage. Uses `celestia-rpc`
   and `celestia-types` crates (both at v1.0 as of 2026).

2. **Namespace management.** The 4-namespace hierarchy: traces, routing,
   vectors, fingerprints. Deterministic namespace derivation from agent IDs,
   data types, and coordination groups. Namespace registry published as a
   system blob.

   ```rust
   // 29-byte v0 namespace format:
   // [tiagent-prefix (8 bytes)] [data-type (4 bytes)] [agent-id (16 bytes)]
   pub fn derive_namespace(data_type: DataType, agent_id: &AgentId) -> Namespace {
       let mut id = [0u8; 28];
       id[..8].copy_from_slice(b"tiagent\0");
       id[8..12].copy_from_slice(&data_type.to_bytes());
       id[12..28].copy_from_slice(&agent_id.truncated_hash());
       Namespace::new_v0(&id).expect("valid namespace")
   }
   ```

3. **Light node embedding.** Integration of `lumina-node` (Celestia's
   production Rust light node) directly into the agent process. DAS
   verification of blob inclusion without running a separate node.
   Feature-gated (`light-node`) to avoid binary size overhead for users who
   connect to external nodes.

4. **Tiered storage.** Hot path (local cache for active sessions) + warm path
   (Celestia DA for shared state, 7-day availability window) + cold path
   (archival node or IPFS for long-term retention). Automatic
   promotion/demotion based on access patterns.

5. **Mocha testnet integration.** Full test suite running against Celestia's
   Mocha testnet. CI pipeline that submits blobs, verifies inclusion, and
   retrieves data through namespace queries.

**Verification criteria:**

```bash
tiagent run "submit a test blob to Celestia Mocha testnet"
```

Successfully submits a blob, retrieves it by namespace, and verifies inclusion
via DAS. End-to-end latency from submission to verified retrieval is under 30
seconds. Blobs are visible on a Mocha block explorer.

**Engineering hours:** ~640 hours (2 FTE x 2 months)
**Infrastructure:** Mocha testnet node, CI/CD, blob submission fees (~$1,000)

---

### Milestone 3: Tool System + MCP Integration (Months 5-6)

**Budget:** USD $30,000

The tool system that lets agents interact with Celestia and the broader
development ecosystem.

**Deliverables:**

1. **MCP client and server.** tiagent acts as both an MCP client (consuming
   tools from external MCP servers like Claude Code's filesystem server or
   GitHub's MCP server) and an MCP server (exposing its capabilities to other
   MCP-aware systems). Built on the MCP Rust SDK.

2. **Celestia developer tools.** A suite of built-in tools for Celestia
   development, exposed as both native tools and MCP tools:

   | Tool | What it does |
   |---|---|
   | `celestia_submit_blob` | Submit a blob to a namespace |
   | `celestia_get_blobs` | Retrieve blobs from a namespace at a height |
   | `celestia_namespace_data` | List all blobs in a namespace across a height range |
   | `celestia_verify_inclusion` | Verify blob inclusion via DAS |
   | `celestia_estimate_cost` | Estimate blob submission cost |
   | `celestia_balance` | Check account balance |
   | `rollkit_scaffold` | Generate a Rollkit rollup configuration |
   | `sovereign_scaffold` | Generate a Sovereign SDK rollup scaffold |
   | `mocha_faucet` | Request testnet tokens |

3. **Built-in general tools.** File operations (read, write, search, glob),
   shell execution (with sandboxing), HTTP requests, git operations, and JSON
   manipulation. These tools make tiagent useful as a standalone coding agent
   without external MCP servers.

4. **Celestia-native MCP server.** A standalone MCP server binary
   (`tiagent-mcp-server`) that exposes Celestia tools to any MCP client. This
   means Claude Code, Codex, and other MCP-aware agents can interact with
   Celestia through tiagent's tool server -- creating an onboarding pathway
   even for developers who do not adopt tiagent as their primary agent.

5. **Tool safety layer.** Capability-based authorization for tools. Agents
   declare required capabilities; the policy layer approves or denies tool
   calls based on the agent's contract. Fail-closed: if a capability is not
   explicitly granted, the tool call is denied.

**Verification criteria:**

```bash
# Native tool execution
tiagent run "scaffold a new Rollkit rollup called my-rollup"

# MCP server serving tools to an external client
tiagent mcp-server --port 8080
# (External MCP client connects and discovers Celestia tools)
```

The Rollkit scaffold command generates valid configuration files. The MCP
server responds to tool discovery and execution requests from external clients.

**Engineering hours:** ~480 hours (1.5 FTE x 2 months)
**Infrastructure:** Mocha testnet, MCP testing infrastructure (~$500)

---

### Milestone 4: Self-Improvement Loop (Months 7-8)

**Budget:** USD $35,000

The cybernetic feedback system that makes agents get better with use.

**Deliverables:**

1. **Cascade router.** Model selection based on task complexity, historical
   performance, and cost constraints. Maintains per-model, per-task-type
   success rates with exponential moving average (EMA) updates. Persists
   routing state to both local storage and Celestia DA (routing namespace).

   Example: after 20 tasks, the router learns that Claude performs 23% better
   on refactoring tasks while GPT-4o is 40% cheaper for documentation tasks.
   It routes accordingly.

2. **Gate pipeline.** Multi-rung validation pipeline:

   | Rung | What it checks | Pass criterion |
   |---|---|---|
   | 1. Compilation | `cargo build` or equivalent | Exit code 0 |
   | 2. Tests | `cargo test` or equivalent | All tests pass |
   | 3. Lint | `cargo clippy` or equivalent | No warnings |
   | 4. Diff review | Structural diff analysis | Changes are coherent |
   | 5. Security | Secret scanning, dependency audit | No vulnerabilities introduced |

3. **Adaptive gate thresholds.** Per-rung pass rates tracked with EMA. When
   a model consistently fails a particular gate, the router learns to route
   away from it for that task type. Thresholds adjust based on observed
   performance rather than static configuration.

4. **Episode logging and analysis.** Automated analysis of episode traces to
   extract performance metrics: task completion rate, token efficiency, tool
   call success rate, error frequency, cost per task. Published as scoring
   signals that feed back into the router.

5. **Playbook extraction.** A persistent strategy memory (inspired by Dynamic
   Cheatsheet, arXiv:2504.07952, ICLR 2026) that records successful
   strategies for task types and includes them in future system prompts. "Last
   time you deployed a Rollkit rollup, the following approach worked..."

**Verification criteria:**

After 50+ executions of coding tasks across multiple task types:

- The cascade router demonstrably routes to the more cost-effective model for
  each task type (measured by comparing routing decisions at run 1 vs run 50).
- Gate threshold adaptation reduces false failures by at least 15% compared
  to static thresholds (measured by tracking pass rate variance).
- Playbook entries are extracted and included in system prompts for relevant
  future tasks.
- All metrics are persisted to `.tiagent/learn/` and optionally to Celestia DA.

**Engineering hours:** ~480 hours (1.5 FTE x 2 months)
**Infrastructure:** LLM API costs for 50+ task executions (~$1,500)

---

### Milestone 5: Cross-Agent Learning via DA (Months 9-10)

**Budget:** USD $40,000

The outer loop. This is the novel contribution -- shared learning through
Celestia's DA layer.

**Deliverables:**

1. **Routing weight delta publishing.** Agents publish cascade router weight
   updates (deltas, not full state) to the routing namespace on Celestia.
   New agents can bootstrap from the network's collective routing experience
   instead of starting cold. Weight deltas are compact (0.5-2 KB per session)
   and use protobuf encoding with a self-describing blob envelope.

2. **Shared playbook discovery.** Agents publish anonymized, redacted playbook
   entries to the traces namespace. Other agents query by task-type similarity
   to find relevant strategies. A local redaction pipeline strips secrets,
   proprietary code, and PII before publication.

3. **HDC fingerprint batching.** Agents publish compact Hyperdimensional
   Computing fingerprints (10,000-dimensional binary vectors, ~1.2 KB each) to
   the fingerprints namespace. Fingerprints encode behavioral signatures --
   what tools an agent uses, how it sequences operations, what error recovery
   patterns it employs. Agents use fingerprint similarity (Hamming distance) to
   find behaviorally similar agents whose strategies are likely transferable.

4. **Merge protocol.** A protocol for consuming shared learning from the DA
   layer:
   - Retrieve routing weight deltas from the routing namespace
   - Filter by recency, source reputation, and behavioral similarity
   - Apply weighted merge into local routing state (with configurable trust
     decay for older data)
   - Verify blob inclusion via DAS before consuming

5. **Anti-gaming measures.** Sybil resistance through stake-weighted blob
   submission priority. HDC fingerprint diversity analysis to detect agents
   publishing identical "learning" from slightly different identities. Anomaly
   detection on routing weight distributions to flag statistical outliers.

**Verification criteria:**

Agent B, starting with no local learning state, bootstraps from Agent A's
published routing weights and achieves within 10% of Agent A's task completion
rate on a standard task suite within 5 task executions (vs. 20+ executions
from cold start). The bootstrap process:

1. Agent A runs 50 tasks and publishes routing weights to Celestia Mocha
2. Agent B starts fresh, queries the routing namespace, verifies blob
   inclusion via DAS
3. Agent B merges Agent A's weights into its local router
4. Agent B runs the same task suite and achieves comparable performance
   within 5 tasks

**Engineering hours:** ~640 hours (2 FTE x 2 months)
**Infrastructure:** Celestia Mocha testnet, multi-agent test harness (~$1,500)

---

### Milestone 6: Production + Ecosystem Integration (Months 11-12)

**Budget:** USD $25,000

Integration with the Celestia ecosystem, production hardening, and
documentation.

**Deliverables:**

1. **Plan DAG execution.** Multi-task plan execution with parallel dispatch.
   Tasks define dependencies as a directed acyclic graph; independent tasks
   run concurrently. Snapshot-resume for crash recovery -- interrupted plans
   resume from the last completed task, not from scratch.

2. **PRD workflow.** End-to-end product requirement document workflow:
   draft a PRD, generate an implementation plan with tasks, execute the plan,
   validate results through gates. This is the self-hosting workflow --
   tiagent can develop itself.

3. **TraceCommons integration.** Bidirectional integration with TraceCommons
   (MIT/Apache-2.0, founded by Zaki Manian, co-creator of Cosmos SDK and
   IBC). tiagent episodes are submitted as TraceCommons-compatible envelopes.
   Quality gates (novelty + substance) ensure only high-value traces enter the
   commons. TraceCommons trajectory RAG results are consumed as in-context
   examples.

4. **Snapshot-resume for crash recovery.** Persistent executor state enables
   interrupted sessions to resume cleanly. The executor serializes its DAG
   state, completed task list, and pending work queue to `.tiagent/state/`.
   On restart with `--resume`, execution continues from where it left off.

5. **Documentation and deployment.** Comprehensive documentation including:
   - Getting started guide (install, configure, first task in 10 minutes)
   - Celestia integration tutorial (namespace setup, blob submission, DAS)
   - Tool development guide (building custom tools and MCP servers)
   - Self-improvement loop explanation (how learning works, how to tune)
   - API reference for all public traits and types
   - Deployment guide for mainnet operation

6. **Mainnet deployment.** Production deployment on Celestia mainnet with
   verified blob submission, namespace management, and DAS verification.

**Verification criteria:**

A developer with no prior tiagent experience can follow the getting started
guide, install tiagent, run an agent against Mocha testnet, and see their
trace published to Celestia within 30 minutes. The end-to-end PRD-to-code
workflow executes successfully:

```bash
tiagent prd draft "Build a REST API with authentication"
tiagent prd plan rest-api-auth
tiagent plan run plans/rest-api-auth/
# Result: working code, passing tests, traces on Celestia
```

**Engineering hours:** ~320 hours (1 FTE x 2 months)
**Infrastructure:** Celestia mainnet blob fees, documentation hosting (~$1,000)

---

## 4. Ecosystem Impact

### 4.1 New DA consumer category

tiagent creates a fundamentally new category of Celestia DA consumption: AI
agent coordination and learning data.

Conservative data estimates per active agent:

| Data type | Size per event | Events/day | Daily DA usage |
|---|---|---|---|
| Episode trace | 5-50 KB | 10-100 tasks | 50 KB - 5 MB |
| Embedding vectors | 2-10 KB | 10-100 | 20 KB - 1 MB |
| HDC fingerprint | 1-2 KB | 1 per session | 1-2 KB |
| Routing delta | 0.5-2 KB | 1 per session | 0.5-2 KB |

Scaling projections:

| Active agents | Daily DA usage | Annual blob fees (at $0.40/MB) |
|---|---|---|
| 100 | 5 MB - 500 MB | $730 - $73,000 |
| 1,000 | 50 MB - 5 GB | $7,300 - $730,000 |
| 10,000 | 500 MB - 50 GB | $73,000 - $7,300,000 |
| 100,000 | 5 GB - 500 GB | $730,000 - $73,000,000 |

For context, Eclipse already uploads more data to Celestia than all other
rollups combined. AI agent traces could become an equally large DA consumer
category. At 10,000 active agents, tiagent would generate meaningful blob fee
revenue for the Celestia network.

The target market is not the 56 rollup teams currently building on Celestia --
it is the millions of developers who currently use Claude Code, Codex, or
Cursor. A frontend developer debugging React components, a backend engineer
optimizing database queries, a DevOps engineer writing Terraform modules --
all of them become Celestia DA consumers the moment they use tiagent.

### 4.2 Celestia ecosystem project benefits

tiagent provides direct value to specific Celestia ecosystem projects:

| Project | What tiagent enables |
|---|---|
| **Sovereign SDK** (now Celestia first-party, acquired July 2026) | Development agents that scaffold rollup code, run test suites, debug deployment issues. Reference agents ship in M6. |
| **Rollkit** | Agents that generate, test, and deploy Rollkit configurations. Rollkit scaffold tool ships in M3. |
| **Eclipse** | Automated operations monitoring, anomaly detection, incident response agents. Eclipse is the largest DA consumer; operations agents drive additional DA usage. |
| **Astria** | Monitoring agents for the shared sequencer, performance analysis, alert routing. |
| **Flame** | DeFi strategy agents that manage positions, execute trades, monitor risk. |
| **Caldera / Conduit** | RaaS deployment automation, rollup health monitoring agents. |
| **Dymension** | RollApp creation and management agents. |
| **OnchainDB** | Agent-queryable database with pay-per-query -- tiagent agents as consumers. |
| **Neutron** | IBC-aware agents managing 100+ cross-chain connections. |
| **Osmosis** | Liquidity management, arbitrage detection, governance participation agents. |

### 4.3 Celestia developer tools as MCP servers

The Celestia tools built in M3 are exposed as MCP servers. This means any
MCP-aware agent -- including Claude Code (which uses MCP natively) -- can
interact with Celestia through tiagent's tool server. A developer using Claude
Code can install the tiagent MCP server and immediately have `celestia_submit_blob`,
`celestia_verify_inclusion`, `rollkit_scaffold`, and other Celestia tools
available in their existing workflow. This creates an onboarding pathway for
developers who are already using AI agents but have never interacted with
Celestia.

### 4.4 Competitive response to 0G Labs

tiagent is the most concrete possible response to 0G Labs' AI narrative. By
building a production-quality agent framework on Celestia, the ecosystem
demonstrates through working code -- not marketing materials -- that
Celestia's existing infrastructure is better suited for AI workloads than
0G's early-stage testnet. The framework exists; the narrative writes itself.

### 4.5 Vision 2.0 alignment

Celestia's Vision 2.0 roadmap mentions AI agents as a potential application
category for the DA layer. tiagent is the concrete implementation of that
vision -- not a concept paper, but working code that developers can use today.

---

## 5. Growth Model

### 5.1 The flywheel

tiagent's growth model works because it is a better coding agent first and a
Celestia DA consumer second:

```
Developers adopt tiagent because it is a better coding agent
    (open-source, self-improving, model-agnostic, collective learning)
                              |
                              v
tiagent publishes learning artifacts to Celestia DA
    (traces, routing weights, embeddings, fingerprints)
                              |
                              v
DA usage grows -> blob fee revenue -> ecosystem value grows
                              |
                              v
More learning data -> tiagent gets smarter -> more developers adopt
                              |
                              v
                        Flywheel spins
```

This is fundamentally different from "build Celestia tools for Celestia
developers." That approach caps the market at rollup teams. The
general-purpose coding agent approach caps the market at every developer who
writes code -- and the Celestia ecosystem grows as a byproduct.

### 5.2 Revenue projections at scale

| Scale | Monthly DA fees | Annual DA fees | Developer reach |
|---|---|---|---|
| **Year 1** (100 agents) | $60 - $6,000 | $730 - $73,000 | Early adopters, Celestia ecosystem |
| **Year 2** (1,000 agents) | $600 - $60,000 | $7,300 - $730,000 | Rust developers, open-source community |
| **Year 3** (10,000 agents) | $6,000 - $600,000 | $73,000 - $7,300,000 | Broader developer market |

These projections are conservative. They assume a steady-state agent population
with moderate daily usage. If agent adoption follows the trajectory of existing
coding agents (Claude Code went from launch to millions of users in under a
year), the upper bounds could be significantly higher.

### 5.3 Network effects

The collective learning pool exhibits strong network effects:

- **Data network effect:** More agents publishing traces means more learning
  data, which means better trajectory RAG, which means better agent
  performance, which attracts more agents.
- **Model routing network effect:** More routing weight data means faster
  bootstrap for new agents, which lowers the barrier to adoption.
- **Tool network effect:** More Celestia-native MCP tools means more reasons
  for developers to interact with Celestia through tiagent.

These network effects create a defensible moat once the flywheel starts
spinning. The first DA-native agent framework to reach critical mass in
learning data becomes difficult to displace.

---

## 6. Team

*[Template -- to be completed by applicant]*

**Project Lead:** [Name]
- [X] years of Rust systems programming experience
- [X] years of blockchain infrastructure development
- Previous work: [relevant projects, repositories, or deployments]

**Senior Engineer:** [Name]
- Experience with Celestia ecosystem, DA layer integration, light node
  development
- Previous work: [relevant Celestia or Cosmos ecosystem contributions]

**Agent Systems Engineer:** [Name]
- Experience with LLM integration, MCP protocol, tool system design
- Previous work: [relevant AI/ML infrastructure projects]

**Evidence of capability:**

The team has designed the tiagent architecture (15-document design suite
totaling 50,000+ words), built related systems (roko: 800K+ LOC Rust, 35
workspace members, fully self-hosting agent toolkit), and has production
experience with Celestia's blob submission and namespace APIs.

The team's prior work demonstrates:
- Production Rust systems at scale (800K+ LOC across 35 crates)
- End-to-end agent development lifecycle (PRD to plan to execution to gates)
- Multi-model LLM dispatch (11 backend providers implemented)
- Self-improvement loops (cascade router, adaptive gates, episode logging)
- Protocol integration (MCP client/server, A2A, x402)

---

## 7. Budget

### 7.1 Per-milestone breakdown

| Milestone | Duration | Amount | Engineering | Infrastructure |
|---|---|---|---|---|
| M1: Core harness | Months 1-2 | $30,000 | $29,500 (480 hrs) | $500 (LLM APIs) |
| M2: Celestia DA substrate | Months 3-4 | $40,000 | $39,000 (640 hrs) | $1,000 (Mocha testnet, CI/CD) |
| M3: Tool system + MCP | Months 5-6 | $30,000 | $29,500 (480 hrs) | $500 (testnet, MCP testing) |
| M4: Self-improvement loop | Months 7-8 | $35,000 | $33,500 (480 hrs) | $1,500 (LLM APIs for 50+ runs) |
| M5: Cross-agent learning | Months 9-10 | $40,000 | $38,500 (640 hrs) | $1,500 (multi-agent testing) |
| M6: Production + ecosystem | Months 11-12 | $25,000 | $24,000 (320 hrs) | $1,000 (mainnet fees, docs hosting) |
| **Total** | **12 months** | **$200,000** | **$194,000 (3,040 hrs)** | **$6,000** |

### 7.2 Cost justification

- **Engineering rate:** ~$64/hour blended rate across the team. This is below
  market rate for senior Rust systems engineers, reflecting the team's
  commitment to the project and willingness to accept below-market
  compensation during the grant period.
- **Infrastructure at ~3% of total:** Infrastructure costs are deliberately
  low because tiagent's architecture uses tiered storage (local cache for hot
  data, DA for shared data) and the team operates existing testnet
  infrastructure.
- **No hardware costs:** Development uses existing team hardware. CI/CD uses
  GitHub Actions (free for open-source projects).

### 7.3 Payment schedule

Milestone-based payments upon delivery and verification of each milestone's
acceptance criteria. Suggested schedule:

| Payment | Trigger | Amount |
|---|---|---|
| Payment 1 | M1 verification passed | $30,000 |
| Payment 2 | M2 verification passed | $40,000 |
| Payment 3 | M3 verification passed | $30,000 |
| Payment 4 | M4 verification passed | $35,000 |
| Payment 5 | M5 verification passed | $40,000 |
| Payment 6 | M6 verification passed | $25,000 |

---

## 8. Sustainability

### 8.1 During the grant (months 1-12)

Grant funding supports full-time development. All code is open source under
MIT/Apache-2.0 dual license from day one. Every commit is public. Community
building starts at M3 (tool system release) with developer documentation,
example agents, and tutorial content.

### 8.2 Post-grant revenue streams

Four mechanisms sustain tiagent after the grant period:

1. **DA utility generates ongoing value.** tiagent generates blob submission
   revenue for the Celestia network. As agent adoption grows, the Celestia
   ecosystem has an economic incentive to continue supporting tiagent
   development through ecosystem grants or foundation investment. The more
   developers use tiagent, the more DA fees flow to validators.

2. **Hosted agent service.** A managed tiagent hosting service where
   developers deploy agents without managing infrastructure. The open-source
   codebase remains freely available for self-hosting. Revenue from hosting
   fees funds ongoing development. Target: $10-50/month per hosted agent.

3. **Enterprise support.** Commercial support contracts for organizations
   deploying tiagent in production. Includes priority bug fixes, custom tool
   development, integration consulting, and SLA guarantees.

4. **TraceCommons credit sharing.** tiagent agents that contribute high-quality
   traces to TraceCommons earn credits. A portion of credit revenue flows back
   to tiagent development, creating a virtuous cycle: better traces earn more
   credits, which fund better agent development, which produces better traces.

### 8.3 Open-source community sustainability

tiagent is designed for community contribution. The trait system (six verb
traits with default implementations) provides clear extension points:

- **Custom LLM backends:** Implement the `Router` trait with a new provider
- **Custom tools:** Implement the tool interface or build an MCP server
- **Custom gate rungs:** Add new validation criteria to the gate pipeline
- **Custom routing strategies:** Replace or extend the cascade router
- **Custom substrates:** Implement `Substrate` for new storage backends

A contributor guide, "good first issues" program, and monthly community calls
sustain engagement beyond the grant period.

---

## 9. Why Now

Five factors make this the right moment for Celestia to invest in AI agent
infrastructure:

### 9.1 0G momentum is accelerating

Every month without a Celestia-native agent framework strengthens 0G's claim
to the "AI DA" narrative. 0G has already deployed $108M in AI-specific
programs. Their developer mindshare is growing. The window for Celestia to
claim first-mover advantage as the production-ready AI DA layer is narrowing.
tiagent is the concrete response -- working code on a production network,
not vaporware on a testnet.

### 9.2 The AI agent market is accelerating

The AI agent market is valued at $22.6-27 billion as of mid-2026 (Grand View
Research, Markets and Markets). Gartner predicts 33% of enterprise software
will use agentic AI by 2028. Claude Code went from launch to millions of
users in under a year. The market is growing faster than any single company
can capture, and the infrastructure layer -- where tiagent and Celestia
operate -- is where durable value accrues.

### 9.3 Celestia Vision 2.0 mentions AI agents

Celestia's own roadmap identifies AI agents as a potential DA consumer
category. tiagent is the implementation of that vision. Funding tiagent is
funding the roadmap.

### 9.4 MCP ecosystem has reached critical mass

The Model Context Protocol has reached 97 million+ monthly SDK downloads.
MCP is becoming the standard interface for agent-tool interaction. Building
Celestia tools as MCP servers means instant compatibility with Claude Code,
Codex, and every other MCP-aware agent. The protocol ecosystem is mature
enough that building on it is low-risk.

### 9.5 Celestia's technical stack is production-ready

Post-Matcha 128 MB blocks, lumina-node 1.0, celestia-types/rpc v1.0 -- the
Rust ecosystem for building on Celestia is finally production-ready. The
infrastructure is ready; the agent tooling is the missing piece. Building
tiagent six months ago would have meant working around immature Rust crates.
Building it now means building on stable, production APIs.

---

## 10. References

### Celestia infrastructure

1. Celestia documentation: https://docs.celestia.org
2. Celestia Vision 2.0 ("Beyond Data Availability"): https://blog.celestia.org/beyond-data-availability/
3. lumina-node (Rust light node): https://github.com/eigerco/lumina
4. celestia-types crate: https://crates.io/crates/celestia-types
5. celestia-rpc crate: https://crates.io/crates/celestia-rpc

### Agent protocols and frameworks

6. Model Context Protocol specification: https://modelcontextprotocol.io
7. A2A Protocol (Agent-to-Agent): https://github.com/google/A2A
8. TraceCommons: https://github.com/zmanian/trace-commons-server
9. IronClaw (NEAR agent runtime): https://github.com/nickelpack/ironclaw

### Research

10. RHO: Harness Optimization (arXiv:2606.05922) -- Harness-level improvements raise SWE-Bench from 59% to 78%, demonstrating that the harness matters more than the model.
11. Dynamic Cheatsheet (arXiv:2504.07952, ICLR 2026) -- Persistent strategy memory across agent sessions improves task completion rates.
12. Sleep-Time Compute (Meta Research) -- ~5x inference cost reduction through offline pre-computation, validating tiagent's middle-loop approach.

### Market data

13. Grand View Research -- AI agent market at $22.6B (2026)
14. Markets and Markets -- AI agent market at $27B (2026)
15. Gartner -- 33% of enterprise software will use agentic AI by 2028

### Competitor analysis

16. 0G Labs documentation: https://docs.0g.ai
17. 0G AI Ecosystem Fund: $88.88M allocation for AI-specific projects
18. 0G Apollo AI Accelerator: $20M (with CoinFund and Hack VC)

### Standards

19. ERC-8004: Agent identity standard for Ethereum (EIP in progress)
20. AITP: AI Transfer Protocol (active development)
21. x402: Paid API access protocol (active development)
