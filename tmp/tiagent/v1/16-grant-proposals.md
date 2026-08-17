# tiagent: Grant Proposals

Four near-submission-ready grant proposals for tiagent development, plus a
grant strategy and stacking plan. Each proposal is self-contained and assumes
no prior reader context.

**Date**: August 2026

---

# Part 1: Grant Strategy and Competitive Positioning

## The Strategic Window

Celestia has a narrative gap. The numbers tell the story:

| Ecosystem | AI-specific funding | AI-specific grants | AI branding |
|---|---|---|---|
| **0G Labs** | $88.88M AI ecosystem fund + $20M Apollo AI Accelerator | Dedicated AI grants up to $1M+ | Explicit: "The AI Blockchain" |
| **Filecoin** | $3.68M ProPGF Batch 1 (includes AI/ML) | AI-aware grant categories | Moderate: "Compute + Storage for AI" |
| **Ethereum** | ESP rolling grants (AI-adjacent) | No dedicated AI track | None |
| **Celestia** | **$0** | **None** | **None** |

Yet Celestia has objectively better infrastructure for AI workloads:

| Capability | Celestia | 0G Labs |
|---|---|---|
| Block size | 128 MB (post-Matcha), scaling to 1 Tb/s (Fibre/V8) | 50 MB max (early testnet) |
| Network maturity | Production mainnet, 56+ rollups | Early-stage testnet |
| Economic activity | Real transaction volume, established ecosystem | Pre-launch |
| Light node verification | DAS with production lumina-node | Not yet available |
| Data partitioning | 29-byte namespace system, production-ready | Custom sharding (not yet stable) |
| Funding runway | $204M ($100M Series C, Sep 2024, led by Bain Capital Crypto) | $290M total but pre-revenue |

0G Labs has seized the "AI DA" narrative with $108M in AI-specific funding
despite having less mature infrastructure. Celestia has not made a single
AI-specific investment. This is a first-mover problem disguised as a
marketing problem: the ecosystem that builds the first credible AI agent
framework on a DA layer claims the narrative. tiagent is that framework.

## Why No One Has Built This Yet

There is no Celestia-native agent framework. The closest projects:

| Framework | Chain | What it does | Celestia support |
|---|---|---|---|
| Eliza (ai16z) | Multi-chain | Character-driven social agents | None |
| Rig | Multi-chain | Rust LLM framework with chain adapters | None |
| ARC | Solana | Solana-native DeFi agents | None |
| Solana Agent Kit | Solana | Wallet/DeFi tools for agents | None |
| Coinbase AgentKit | Base | Commerce/wallet agent tools | None |
| polkagent | Polkadot | 90-crate deep Polkadot integration | None |
| IronClaw | NEAR | WASM/TEE sandboxed agent runtime | None (runtime, not harness) |

Every major blockchain ecosystem except Celestia has at least one dedicated
agent framework. This gap exists because Celestia's modular DA-only
architecture is fundamentally different from execution-layer chains --- the
integration patterns are novel, and no one has figured out how to use a DA
layer as shared agent memory. tiagent fills this gap.

## The Novel Contribution: Shared Learning via DA

Using a data availability layer as a substrate for cross-agent shared
learning is genuinely new. No existing agent framework does this. The key
insight is that Celestia's append-only, namespace-partitioned, verifiable
blob storage is structurally ideal for agent coordination:

```
Traditional agent frameworks:

    Agent A          Agent B          Agent C
    ┌────────┐       ┌────────┐       ┌────────┐
    │ learns │       │ learns │       │ learns │
    │ locally │       │ locally │       │ locally │
    └────────┘       └────────┘       └────────┘
         ↕                ↕                ↕
    local state      local state      local state
    (isolated)       (isolated)       (isolated)

tiagent with Celestia DA:

    Agent A          Agent B          Agent C
    ┌────────┐       ┌────────┐       ┌────────┐
    │ learns │       │ learns │       │ learns │
    │ locally │       │ locally │       │ locally │
    └───┬────┘       └───┬────┘       └───┬────┘
        │                │                │
        ▼                ▼                ▼
    ┌──────────────────────────────────────────┐
    │         Celestia DA Layer                │
    │  ┌─────────────┐  ┌─────────────┐       │
    │  │ ns: traces   │  │ ns: routing │       │
    │  │ (episodes,   │  │ (weights,   │       │
    │  │  outcomes)   │  │  cascades)  │       │
    │  └─────────────┘  └─────────────┘       │
    │  ┌─────────────┐  ┌─────────────┐       │
    │  │ ns: vectors  │  │ ns: finger  │       │
    │  │ (embeddings, │  │ (HDC,       │       │
    │  │  traj-RAG)   │  │  behavioral)│       │
    │  └─────────────┘  └─────────────┘       │
    └──────────────────────────────────────────┘
         append-only, verifiable, namespace-partitioned
```

What each namespace stores:

- **Traces namespace**: Structured records of agent actions, tool calls,
  outcomes, and error patterns. Other agents can retrieve relevant
  trajectories via embedding similarity.
- **Routing namespace**: Cascade router weights and model performance
  statistics. Agents publish which model worked best for which task type;
  new agents bootstrap from the network's collective experience.
- **Vectors namespace**: Sentence embeddings of successful trajectories.
  Enables trajectory retrieval-augmented generation (trajectory RAG) ---
  agents can find "how did another agent solve a similar task?" and use that
  trajectory as in-context learning.
- **Fingerprint namespace**: HDC (Hyperdimensional Computing) behavioral
  fingerprints that compactly encode an agent's behavioral signature. Used
  for similarity matching, anomaly detection, and Sybil resistance.

The append-only nature of the DA layer means the learning corpus is
monotonically growing, verifiable, and tamper-evident. The namespace system
provides natural data partitioning that scales with the number of agents.
DAS light nodes let agents verify data availability cheaply without
downloading full blocks.

## AI Agents as DA Consumers

Eclipse already uploads more data to Celestia than all other rollups
combined. AI agent traces, coordination data, and learning state could
become an equally large DA consumer category.

Conservative data estimates per agent:

| Data type | Size per task | Tasks/day (active agent) | Daily DA usage |
|---|---|---|---|
| Episode trace | 5-50 KB | 10-100 | 50 KB - 5 MB |
| Embedding vectors | 2-10 KB | 10-100 | 20 KB - 1 MB |
| HDC fingerprint | 1-2 KB | 1 (per session) | 1-2 KB |
| Routing delta | 0.5-2 KB | 1 (per session) | 0.5-2 KB |

At 1,000 active agents publishing daily: 50 MB - 5 GB of DA consumption
per day. At 10,000 agents: 500 MB - 50 GB. This is meaningful blob revenue
for the Celestia network.

## Ecosystem Beneficiaries

tiagent benefits specific Celestia ecosystem projects:

| Project | What tiagent enables |
|---|---|
| **Eclipse** | Automated operations monitoring, anomaly detection, incident response agents |
| **Sovereign SDK** (now Celestia first-party, acquired July 2026) | Development agents that scaffold rollup code, run test suites, debug deployment issues |
| **Rollkit** | Agents that generate, test, and deploy Rollkit configurations |
| **Astria** | Monitoring agents for the shared sequencer, performance analysis, alert routing |
| **Flame** | DeFi strategy agents that manage positions, execute trades, monitor risk |
| **Caldera / Conduit** | RaaS deployment automation, rollup health monitoring agents |
| **Dymension** | RollApp creation and management agents |
| **OnchainDB** | Agent-queryable database with pay-per-query --- tiagent agents as consumers |
| **Neutron** | IBC-aware agents that manage 100+ cross-chain connections |
| **Osmosis** | Liquidity management, arbitrage detection, governance participation agents |

## Grant Program Landscape

| Program | Amount | Deadline | Fit | Priority |
|---|---|---|---|---|
| NLnet NGI Zero Restack | EUR 48K | Nov 3, 2026 | Strong (privacy + open-source) | **Submit first** (hard deadline) |
| Celestia Foundation strategic | $150-250K | Relationship-based | Best fit (DA integration) | **Submit second** |
| Interchain Foundation | $100K | Rolling | Strong (Cosmos/IBC tools) | Submit third |
| Modular Fellows | $9K stipend | Next cohort | Good entry + ecosystem access | Apply when cohort opens |
| Mammothon 2 | $25K finalist | Next event | Good demo | Hackathon submission |
| 0G Foundation | Up to $1M+ | Rolling | Cross-DA bridge | Explore |
| Filecoin ProPGF | $10-100K | Batch 2 | Archive integration | Later |

## The Narrative Pivot: General-Purpose Agent, Celestia-Powered

tiagent's grant pitch is NOT "build a tool for Celestia developers." It is
"build a better coding agent for EVERYONE, where Celestia DA is the secret
weapon that makes it collectively intelligent." Every developer who uses
tiagent becomes a Celestia DA consumer --- whether or not they know or care
about blockchain. This is a Trojan horse growth strategy for Celestia.

tiagent competes with Claude Code, Codex, and Cursor as a general-purpose
coding agent. It writes code, runs tests, debugs, refactors, deploys --- the
same tasks that millions of developers use commercial coding agents for today.
The Celestia integration is what makes tiagent NOVEL and grant-worthy: it is
the only coding agent where the learning layer is shared, verifiable, and not
captured by a single vendor. But the product itself serves ALL developers, not
just Celestia developers.

This actually STRENGTHENS the grant case: "Fund tiagent and you bring EVERY
developer into the Celestia ecosystem as a byproduct of using a better coding
agent." The DA consumption is invisible to the end user --- they just get a
coding agent that benefits from collective learning. But every trace published,
every routing weight shared, every trajectory stored is a Celestia blob
generating DA fees and growing the ecosystem.

## Grant Stacking Strategy

The proposals below are designed to stack without double-dipping. Each funds
different aspects of tiagent:

| Grant | Amount | What it funds | Duration |
|---|---|---|---|
| NLnet NGI Zero Restack | EUR 48K (~$52K) | Standalone coding agent + privacy-preserving collective learning | 12 months |
| Celestia Foundation | $200K | DA integration + ecosystem positioning + cross-agent learning via DA | 12 months |
| Interchain Foundation | $100K | Cosmos/IBC-aware tools + cross-chain agent coordination | 9 months |
| Modular Fellows | $9K | Personal development + MVP demo + ecosystem access | 3 months |
| Mammothon 2 | $25K (finalist) | Hackathon-scoped demo (general coding + DA artifacts) | 2-4 weeks |

Combined: $200K-$386K+ over 12-18 months. The strategy is layered:

- **NLnet** funds the standalone coding agent and privacy layer ---
  independent of any blockchain. This is the product foundation.
- **Celestia Foundation** funds the DA integration that makes the collective
  learning layer verifiable and decentralized. This is the novel
  infrastructure.
- **Interchain Foundation** funds the Cosmos/IBC-specific tools. This is the
  multi-chain expansion.
- **Modular Fellows** and **Mammothon** provide early validation, ecosystem
  access, and credibility.

No double-dipping: each grant funds a distinct layer of the stack.

## Timeline

| When | Action |
|---|---|
| **Aug 2026** | Finalize proposals. Begin warm introductions to Celestia Foundation. |
| **Sep 2026** | Submit Celestia Foundation proposal. Begin ICF conversation. |
| **Oct 2026** | Submit ICF proposal. Apply for next Modular Fellows cohort (if open). Finalize NLnet application. |
| **Nov 2026** | **Submit NLnet NGI Zero Restack (hard deadline: Nov 3).** Prepare Mammothon demo. |
| **Dec 2026** | Follow up on CF and ICF. Build MVP regardless of grant status. |
| **Q1 2027** | Mammothon 2 (if scheduled). First public demo. Broader grant applications. |

---

# Proposal 1: Celestia Foundation -- Strategic Ecosystem Grant

**Program**: Celestia Foundation strategic grants (relationship-based)
**Requested amount**: USD 200,000
**Duration**: 12 months, 6 milestones
**Contact**: Direct engagement with Celestia Foundation ecosystem team

## Proposal title

tiagent: Making Celestia the AI-Native Data Availability Layer

## Abstract

tiagent is a coding agent that competes with Claude Code and Cursor, but
uses Celestia DA as its shared learning layer. Every tiagent user becomes a
Celestia DA consumer.

AI coding agents are a $22.6--27 billion market by mid-2026. Millions of
developers use Claude Code, Codex, and Cursor daily. These tools are
powerful, but they share a structural limitation: learning is siloed.
Claude Code learns from your session, but that learning stays locked inside
Anthropic's infrastructure. Your hard-won debugging strategies, deployment
patterns, and architectural insights benefit no one else --- and you never
benefit from theirs.

tiagent breaks this pattern. It is a general-purpose coding agent ---
model-agnostic, open-source (MIT/Apache-2.0), written in Rust --- that
competes directly with commercial coding agents on capability. It writes
code, runs tests, debugs, refactors, and deploys, using Claude, GPT,
Gemini, Llama, or any OpenAI-compatible model. What makes it novel is the
learning layer: tiagent publishes learning artifacts (traces, routing
weights, embeddings, behavioral fingerprints) to Celestia's DA layer as
namespace-organized blobs. Other tiagent instances retrieve relevant
trajectories, bootstrap from collective routing experience, and verify the
provenance of learned strategies --- all through the same DA layer that
rollups use for block data.

The result: every developer who uses tiagent becomes a Celestia DA
consumer, whether or not they know or care about blockchain. A Python
developer debugging a Django app benefits from Celestia DA because the
collective learning pool that makes tiagent smarter is stored there. This
creates a new DA consumer category --- AI agent coordination and learning
data --- that could rival rollup block data in volume as adoption grows.

Meanwhile, 0G Labs has positioned itself as "the AI blockchain" with
$108M in AI-specific funding despite early-stage infrastructure. Celestia
has superior production capabilities (128 MB blocks, 56+ rollups, DAS
light nodes, 29-byte namespaces) but zero AI-specific investment. tiagent
is the concrete response: working code, not marketing budget.

This proposal funds 12 months of development across six milestones, taking
tiagent from design to production deployment. The target market is not the
56 Celestia rollup teams --- it is the millions of developers who currently
use commercial coding agents and would benefit from one that gets smarter
from collective experience.

## 1. Problem statement

### 1.1 The AI narrative gap

Celestia is the leading modular blockchain by every objective metric ---
mainnet maturity, rollup adoption, economic activity, technical throughput.
But in the rapidly growing AI agent infrastructure market, Celestia is
invisible.

0G Labs has spent $108 million on AI-specific programs:

- **$88.88M AI Ecosystem Fund** funding projects that build on 0G's DA +
  execution stack
- **$20M Apollo AI Accelerator** (with CoinFund and Hack VC) specifically
  targeting AI + blockchain startups
- **Dedicated AI Labs** building reference implementations for AI data
  availability
- **Explicit positioning**: "0G is purpose-built for AI" (from their
  marketing materials)

The result: when developers think "AI + data availability," they think 0G,
not Celestia. This is despite 0G's testnet having smaller block sizes, no
production rollup ecosystem, and no DAS light node infrastructure.

The gap is not technical --- Celestia's infrastructure is superior. The gap
is that no one has built the tooling that demonstrates why Celestia is
better for AI. tiagent closes this gap.

### 1.2 No Celestia-native agent framework

Every agent framework that exists today falls into one of three categories,
none of which serve Celestia developers:

**Non-blockchain frameworks** (LangChain, CrewAI, AutoGen, Eliza): Written
in Python or TypeScript. Treat blockchain as an afterthought --- a thin RPC
wrapper behind a tool definition. No concept of namespace-organized storage,
DA verification, or on-chain coordination. Cannot compile to lightweight
binaries for infrastructure embedding.

**Rust frameworks without chain affinity** (Rig): Provide LLM abstractions
and chain adapters but have no deep integration with any specific chain.
Rig wraps RPC calls the same way Python frameworks do, just in Rust. No
Celestia-specific tooling, namespace awareness, or DA patterns.

**Chain-native frameworks for other ecosystems** (polkagent for Polkadot,
ARC for Solana, Coinbase AgentKit for Base): Demonstrate that chain-native
agent tooling is valuable and viable. But Celestia's modular DA-only
architecture requires fundamentally different integration patterns than
execution-layer chains. Polkadot's shared-security parachain model, Solana's
account-based execution, and Base's EVM execution have nothing in common
with Celestia's blob/namespace/DAS model.

Celestia developers who want agents must write custom RPC integration, blob
serialization, namespace management, and light node interaction from
scratch. This is the kind of undifferentiated heavy lifting that a framework
exists to eliminate.

### 1.3 No shared learning infrastructure

Every agent deployment is an island. When an agent running on Eclipse's
infrastructure figures out the optimal blob size for their transaction
batches, or when an agent deploying Rollkit configurations learns which
template produces the most reliable results, that knowledge stays locked in
local state. There is no mechanism for sharing learned strategies across
agents, organizations, or projects.

Celestia's DA layer is uniquely suited to solve this. Its properties ---
append-only, namespace-partitioned, verifiable, permissionless read/write,
cheap blob storage --- map directly to the requirements of a shared learning
substrate. But no one has built the framework to use it this way.

## 2. Proposed solution: tiagent

### 2.1 What tiagent is

tiagent is a Rust toolkit (MIT/Apache-2.0, targeting 12--14 crates) for
building self-improving AI agents natively on Celestia. It provides:

- **A universal execution loop**: query, score, route, compose, act,
  verify, write, react. Every agent task follows this pattern.
- **Model-agnostic LLM dispatch**: Claude, GPT, Gemini, Llama, Mistral,
  Ollama, and any OpenAI-compatible API. The harness does not care which
  model runs --- it routes intelligently based on task complexity and cost.
- **Celestia DA substrate**: Blob submission and retrieval through
  organized namespaces. Agent traces, embeddings, and learning artifacts
  are first-class DA blobs, not database rows.
- **Cybernetic self-improvement**: Three nested feedback loops (execution,
  learning, cross-agent) that observe performance, identify gaps, generate
  improvement plans, and validate results.
- **Tool system with MCP integration**: Agents call tools through the Model
  Context Protocol (MCP, 97M monthly SDK downloads). tiagent ships built-in
  Celestia developer tools and acts as both MCP client and server.
- **Protocol interoperability**: MCP, A2A (Agent-to-Agent, 150+ orgs), AITP
  (AI Transfer Protocol), and x402 (paid API access) support.

### 2.2 Architecture overview

The core architecture follows a "1 noun + 6 verbs" model:

**The noun**: Signal. Every piece of data flowing through tiagent is a
Signal --- a content-addressed, typed, scored datum with metadata. Signals
are the atoms of agent state.

**The six verbs** (Rust traits):

| Trait | What it does |
|---|---|
| `Substrate` | Reads and writes signals. Implementations: `CelestiaSubstrate` (DA blobs), `LocalSubstrate` (JSONL files). |
| `Scorer` | Evaluates signal quality across dimensions (completion, efficiency, safety, cost). |
| `Gate` | Validates agent outputs against criteria (compilation, tests, lint, diff review). Multi-rung pipeline. |
| `Router` | Selects models, prompts, and strategies based on task features and historical performance. |
| `Composer` | Assembles system prompts from layered templates, context, and task specifications. |
| `Policy` | Enforces safety contracts, budget limits, and behavioral constraints. |

The universal loop:

```
      ┌─────────────────────────────────────────────────────┐
      │                                                     │
      ▼                                                     │
   Query ─► Score ─► Route ─► Compose ─► Act ─► Verify ─► Write ─► React
                                          │                        │
                                          ▼                        │
                                    LLM + Tools                    │
                                    (model-agnostic)               │
                                                                   │
                                          Celestia DA ◄────────────┘
                                          (namespace-organized blobs)
```

### 2.3 Celestia integration design

tiagent maps its data model onto Celestia's primitives:

**Namespace hierarchy** (using Celestia's 29-byte v0 namespace format):

```
tiagent/                        Root prefix for all tiagent data
  ├── system/                   Global configuration, agent registry
  ├── traces/                   Episode traces (agent actions + outcomes)
  │   ├── traces/<agent-id>     Per-agent trace namespace
  │   └── traces/shared         Cross-agent shared traces
  ├── routing/                  Cascade router weights and model stats
  ├── vectors/                  Sentence embeddings for trajectory RAG
  ├── fingerprints/             HDC behavioral fingerprints
  └── coordination/             Multi-agent coordination proofs
```

**Blob schema** (protobuf-encoded, self-describing):

```rust
/// Every tiagent blob starts with a header that identifies its type,
/// schema version, and provenance.
pub struct BlobHeader {
    pub schema_version: u16,     // Schema evolution
    pub blob_type: BlobType,     // Trace, Embedding, Routing, Fingerprint, etc.
    pub agent_id: [u8; 32],      // Pseudonymous agent identifier
    pub timestamp: u64,          // Unix timestamp (seconds)
    pub prev_hash: [u8; 32],     // Hash of this agent's previous blob (chain)
    pub payload_hash: [u8; 32],  // BLAKE3 hash of the payload
}

pub enum BlobType {
    EpisodeTrace,       // Structured record of agent execution
    EmbeddingVector,    // Sentence embedding for trajectory RAG
    RoutingDelta,       // Cascade router weight update
    HdcFingerprint,     // Behavioral signature
    CoordinationProof,  // Multi-agent coordination attestation
    GateAttestation,    // On-chain record of gate pass/fail
}
```

**Light node embedding** (via lumina-node):

```rust
/// tiagent embeds a Celestia light node directly in the agent process.
/// This enables DAS verification without running a separate node.
pub struct EmbeddedLightNode {
    node: lumina_node::Node,     // Production Rust light node
    network: Network,            // Mocha (testnet) or Mainnet
    store: SledStore,            // Local header + sample store
}

impl EmbeddedLightNode {
    /// Verify that a blob was included in a specific Celestia block.
    /// Uses DAS (Data Availability Sampling) --- downloads ~1-5% of
    /// block data, not the entire block.
    pub async fn verify_inclusion(
        &self,
        height: u64,
        namespace: Namespace,
        blob_hash: [u8; 32],
    ) -> Result<InclusionProof, VerifyError>;
}
```

### 2.4 Self-improvement loop

tiagent implements three nested feedback loops:

**Inner loop (per-task)**: Within a single task execution, the agent
observes tool call outcomes, adjusts strategy (e.g., retries with different
parameters, switches tools), and records the execution trace.

**Middle loop (across tasks)**: Across multiple task executions, the cascade
router observes which models, prompts, and strategies produced the best
results for which task types. It adjusts routing weights via exponential
moving average (EMA) updates. The gate pipeline tracks per-rung pass rates
and adjusts thresholds adaptively.

**Outer loop (across agents)**: Agents publish learning artifacts to
Celestia's DA layer. Other agents retrieve relevant trajectories via
embedding similarity (trajectory RAG), bootstrap from published routing
weights, and use HDC fingerprint matching to find behaviorally similar
agents whose strategies are likely transferable.

The outer loop is the novel contribution. No existing agent framework
implements cross-agent shared learning through a verifiable, append-only
data layer.

### 2.5 TraceCommons integration

tiagent produces and consumes TraceCommons-compatible traces. TraceCommons
is an open-source, privacy-preserving registry of AI agent session traces
(founded by Zaki Manian, co-creator of Cosmos SDK and IBC). The integration
provides:

- **Quality-gated trace publishing**: Traces pass through TraceCommons'
  two-gate pipeline (novelty + substance) before entering the shared corpus.
  Only high-value traces contribute to the commons.
- **Trajectory RAG**: Agents retrieve relevant trajectories from
  TraceCommons' scored corpus as in-context examples. This is retrieval-
  augmented generation using execution trajectories instead of documents.
- **Credit incentives**: Contributors earn TraceCommons credits for traces
  that pass quality gates. This incentivizes trace publication and
  creates a virtuous cycle of improvement.
- **Cross-ecosystem learning**: TraceCommons traces come from Claude Code,
  Codex, IronClaw, and other agents. tiagent agents benefit from learning
  across all these systems, not just Celestia-native agents.

## 3. Technical approach: milestones

### Milestone 1: Core harness (months 1-2)

**Budget**: USD 30,000

The foundational runtime that makes everything else possible. This milestone
delivers a working agent harness that can execute tasks, validate results,
and persist traces --- without Celestia integration.

**Deliverables:**

1. **Signal types and core traits.** The `tiagent-core` crate implementing
   the Signal data type, the six verb traits (`Substrate`, `Scorer`, `Gate`,
   `Router`, `Composer`, `Policy`), and the universal loop. All traits have
   default implementations and mock backends for testing.

2. **CLI binary.** The `tiagent-cli` crate providing `tiagent run "<prompt>"`,
   `tiagent status`, and `tiagent doctor` subcommands. Enough to run a
   single agent through a task and see the result.

3. **Two LLM backends.** Claude API and OpenAI-compatible backends in the
   `tiagent-agent` crate. Model-agnostic dispatch through the `Router`
   trait. At least one local model backend (Ollama) for development without
   API keys.

4. **Local substrate.** The `tiagent-store` crate implementing `Substrate`
   over local JSONL files. Agent traces, episodes, and learning state
   persist to a `.tiagent/` directory.

5. **Episode logger.** Structured recording of every agent turn (tool calls,
   model responses, timestamps, token counts, outcomes) in a replayable
   format compatible with TraceCommons envelope schema.

**Verification criteria:** `cargo run -p tiagent-cli -- run "list files in the
current directory"` executes successfully with both Claude and OpenAI
backends, producing a valid episode trace in `.tiagent/episodes.jsonl`.

### Milestone 2: Celestia DA substrate (months 3-4)

**Budget**: USD 40,000

The Celestia integration layer. This is where tiagent becomes Celestia-
native rather than chain-agnostic.

**Deliverables:**

1. **CelestiaSubstrate implementation.** The `tiagent-celestia` crate
   implementing the `Substrate` trait over Celestia's blob API. Submit and
   retrieve blobs through namespace-organized storage. Uses `celestia-rpc`
   and `celestia-types` crates (both at v1.0 as of 2026).

2. **Namespace management.** The namespace hierarchy described in Section
   2.3. Deterministic namespace derivation from agent IDs, data types, and
   coordination groups. Namespace registry published as a system blob.

3. **Light node embedding.** Integration of `lumina-node` (Celestia's
   production Rust light node) directly into the agent process. DAS
   verification of blob inclusion. Feature-gated (`light-node`) to avoid
   binary size overhead for users who connect to external nodes.

4. **Tiered storage.** Hot path (local cache for active sessions) + warm
   path (Celestia DA for shared state, 7-day availability window) + cold
   path (archival node or IPFS for long-term retention). Automatic
   promotion/demotion based on access patterns.

5. **Mocha testnet integration.** Full test suite running against Celestia's
   Mocha testnet. CI pipeline that submits blobs, verifies inclusion, and
   retrieves data through namespace queries.

**Verification criteria:** An agent running `tiagent run "submit a test blob
to Celestia Mocha testnet"` successfully submits a blob, retrieves it by
namespace, and verifies inclusion via DAS. End-to-end latency from
submission to verified retrieval is under 30 seconds.

### Milestone 3: Tool system and MCP integration (months 5-6)

**Budget**: USD 30,000

The tool system that lets agents interact with Celestia and the broader
ecosystem.

**Deliverables:**

1. **MCP client and server.** tiagent acts as both an MCP client (consuming
   tools from external MCP servers) and an MCP server (exposing its
   capabilities to other MCP-aware systems). Built on the MCP Rust SDK.

2. **Celestia developer tools.** A suite of built-in tools for Celestia
   development:

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

3. **Built-in general tools.** File operations, shell execution, HTTP
   requests, and JSON manipulation. Enough to be useful without external
   MCP servers.

4. **Tool safety layer.** Capability-based authorization for tools.
   Agents declare required capabilities; the policy layer approves or denies
   tool calls based on the agent's contract. Fail-closed: if a capability
   is not explicitly granted, the tool call is denied.

**Verification criteria:** `tiagent run "scaffold a new Rollkit rollup
called my-rollup and deploy it to Mocha testnet"` successfully generates
a configuration, submits it, and produces a valid deployment artifact.

### Milestone 4: Self-improvement loop (months 7-8)

**Budget**: USD 35,000

The cybernetic feedback system that makes agents get better with use.

**Deliverables:**

1. **Cascade router.** Model selection based on task complexity, historical
   performance, and cost constraints. Maintains per-model, per-task-type
   success rates with EMA updates. Persists routing state to both local
   storage and Celestia DA (routing namespace).

2. **Episode analysis.** Automated analysis of episode traces to extract
   performance metrics: task completion rate, token efficiency, tool call
   success rate, error frequency, and cost per task. Published as scoring
   signals.

3. **Adaptive gate thresholds.** The multi-rung gate pipeline (compilation,
   tests, lint, diff review) tracks per-rung pass rates and adjusts
   thresholds using EMA. When a model consistently fails a particular gate,
   the router learns to route away from it for that task type.

4. **Efficiency tracking.** Per-turn cost accounting (tokens in, tokens out,
   tool calls, wall-clock time). Published to `.tiagent/learn/efficiency.jsonl`
   and optionally to the Celestia efficiency namespace.

5. **Dynamic cheatsheet.** A persistent strategy memory (inspired by
   arXiv:2504.07952, ICLR 2026) that records successful strategies for
   task types and includes them in future system prompts. "Last time you
   deployed a Rollkit rollup, the following approach worked..."

**Verification criteria:** After 10 executions of similar tasks, the cascade
router demonstrably routes to the more cost-effective model for that task
type. Gate threshold adaptation reduces false failures by at least 15%
compared to static thresholds.

### Milestone 5: Cross-agent learning via DA (months 9-10)

**Budget**: USD 40,000

The outer loop. This is the novel contribution --- shared learning through
Celestia's DA layer.

**Deliverables:**

1. **Routing weight publication.** Agents publish cascade router weight
   updates (deltas, not full state) to the routing namespace. New agents
   can bootstrap from the network's collective routing experience instead
   of starting cold.

2. **Trajectory RAG.** Agents publish sentence embeddings of successful
   trajectories to the vectors namespace. When facing a new task, an agent
   queries the vectors namespace for similar trajectories and uses them as
   in-context examples. This is retrieval-augmented generation over
   execution trajectories, not documents.

3. **HDC behavioral fingerprinting.** Agents publish compact
   Hyperdimensional Computing fingerprints (10,000-dimensional binary
   vectors) to the fingerprints namespace. Fingerprints encode behavioral
   signatures --- what tools an agent uses, how it sequences operations,
   what error recovery patterns it employs. Agents use fingerprint
   similarity to find behaviorally similar agents whose strategies are
   likely transferable.

4. **Coordination proofs.** Multi-agent workflows publish coordination
   proofs to the coordination namespace --- attestations that agents A and B
   collaborated on task T with outcome O. Verifiable through NMT inclusion
   proofs.

5. **Anti-gaming measures.** Sybil resistance through stake-weighted blob
   submission. HDC fingerprint diversity analysis to detect agents publishing
   identical "learning" data from slightly different identities. Anomaly
   detection on routing weight distributions.

**Verification criteria:** Agent B, starting with no local learning state,
bootstraps from Agent A's published routing weights and achieves within 10%
of Agent A's task completion rate on a standard task suite within 5 task
executions (vs. 20+ executions from cold start).

### Milestone 6: Ecosystem integration and production (months 11-12)

**Budget**: USD 25,000

Integration with the Celestia ecosystem and production hardening.

**Deliverables:**

1. **TraceCommons integration.** Full bidirectional integration with
   TraceCommons. tiagent episodes are submitted as TraceCommons-compatible
   envelopes. TraceCommons trajectory RAG results are consumed as in-context
   examples. Quality gates ensure only high-value traces enter the commons.

2. **Rollup framework integration.** Reference agents for Sovereign SDK
   (now Celestia first-party) and Rollkit: agents that can scaffold, build,
   test, and deploy rollups with validated output.

3. **OnchainDB integration.** Agent-accessible querying of OnchainDB's
   pay-per-query database for Celestia chain state. Agents can query
   historical blob data, namespace statistics, and block metadata.

4. **Documentation and tutorials.** Comprehensive documentation including:
   getting started guide, Celestia integration tutorial, tool development
   guide, self-improvement loop explanation, and API reference. Published
   at tiagent.dev.

5. **Mainnet deployment.** Production deployment on Celestia mainnet with
   verified blob submission, namespace management, and DAS verification.
   Deployment guide for operators.

**Verification criteria:** A developer with no prior tiagent experience can
follow the getting started guide, install tiagent, run an agent against
Mocha testnet, and see their trace published to Celestia within 30 minutes.

## 4. Ecosystem impact

### 4.1 New DA consumer category

tiagent creates a new category of DA consumption: AI agent coordination and
learning data. As the agent ecosystem grows, this data could rival rollup
block data as a DA consumer. Conservative estimates suggest 1,000 active
agents would generate 50 MB--5 GB of daily DA usage, producing meaningful
blob fee revenue for the Celestia network.

The target market is not just the 56 rollup teams currently building on
Celestia --- it is the millions of developers who currently use Claude Code,
Codex, or Cursor. tiagent gives them a reason to interact with Celestia
without needing to understand blockchain. A frontend developer debugging
React components, a backend engineer optimizing database queries, a DevOps
engineer writing Terraform modules --- all of them become Celestia DA
consumers the moment they use tiagent, because the collective learning layer
runs on Celestia.

### 4.2 Competitive positioning against 0G Labs

tiagent is the most direct response to 0G Labs' AI narrative. By building a
production-quality agent framework on Celestia, the ecosystem demonstrates
that Celestia's existing infrastructure --- production mainnet, 128 MB
blocks, DAS, namespaces --- is better suited for AI workloads than 0G's
early-stage testnet. The framework exists; the narrative writes itself.

### 4.3 Developer onboarding

Celestia developer tools bundled as MCP-compatible tools mean that any
MCP-aware agent (Claude Code, Codex, and others) can interact with Celestia
through tiagent's tool server. This lowers the barrier for developers who
are already using AI agents to start building on Celestia.

### 4.4 Ecosystem project acceleration

The reference agents for Sovereign SDK, Rollkit, and Eclipse operations
provide immediate value to teams already building on Celestia. An agent that
can scaffold, test, and deploy a Sovereign SDK rollup saves developers days
of manual configuration.

### 4.5 Vision 2.0 alignment

Celestia's Vision 2.0 roadmap mentions AI agents as a potential application
category for the DA layer. tiagent is the concrete implementation of that
vision --- not a concept paper, but working code that developers can use
today.

### 4.6 Growth model

The growth flywheel works because tiagent is a better coding agent first
and a Celestia DA consumer second:

```
Developers adopt tiagent because it's a better coding agent
    (open-source, self-improving, model-agnostic, collective learning)
                              │
                              ▼
tiagent publishes learning artifacts to Celestia DA
    (traces, routing weights, embeddings, fingerprints)
                              │
                              ▼
DA usage grows → blob fee revenue → ecosystem grows
                              │
                              ▼
More learning data → tiagent gets smarter → more developers adopt
                              │
                              ▼
                        Flywheel spins
```

This is a fundamentally different growth model than "build Celestia tools
for Celestia developers." That approach caps the market at rollup teams.
The general-purpose coding agent approach caps the market at every
developer who writes code --- and the Celestia ecosystem grows as a
byproduct.

## 5. Why now

Three factors make this the right time:

1. **0G momentum is accelerating.** Every month without a Celestia-native
   agent framework strengthens 0G's claim to the "AI DA" narrative. The
   window for first-mover advantage is narrowing.

2. **Agent infrastructure is consolidating.** MCP has reached 97 million
   monthly SDK downloads. A2A has 150+ member organizations. ERC-8004
   (agent identity) is progressing. The standards are stabilizing, which
   means building on them now is lower-risk than 6 months ago.

3. **Celestia's technical readiness.** Post-Matcha 128 MB blocks, lumina-
   node 1.0, celestia-types/rpc v1.0 --- the Rust ecosystem for building
   on Celestia is finally production-ready. The infrastructure is ready;
   the agent tooling is the missing piece.

## 6. Team qualifications

*[Template -- to be completed by applicants]*

**Project Lead:** [Name]. [Experience with Rust systems programming,
blockchain infrastructure, AI agent systems.]

**Senior Engineer:** [Name]. [Experience with Celestia ecosystem, DA layer
integration, light node development.]

**Agent Systems Engineer:** [Name]. [Experience with LLM integration, MCP
protocol, tool system design.]

Evidence of capability: the team has designed the tiagent architecture
(15-document design suite totaling 50,000+ words), built related systems
(roko: 177K LOC Rust, 18 crates, fully self-hosting agent toolkit; polkagent:
90-crate Polkadot agent framework), and has production experience with
Celestia's blob submission and namespace APIs.

## 7. Budget breakdown

| Milestone | Duration | Amount | Key costs |
|---|---|---|---|
| M1: Core harness | Months 1-2 | USD 30,000 | 1.5 FTE engineering (Signal types, CLI, LLM backends, local substrate) |
| M2: Celestia DA substrate | Months 3-4 | USD 40,000 | 2 FTE engineering (CelestiaSubstrate, lumina integration, namespace mgmt, Mocha CI) |
| M3: Tool system + MCP | Months 5-6 | USD 30,000 | 1.5 FTE engineering (MCP client/server, Celestia tools, safety layer) |
| M4: Self-improvement loop | Months 7-8 | USD 35,000 | 1.5 FTE engineering (cascade router, episode analysis, adaptive gates, efficiency) |
| M5: Cross-agent learning | Months 9-10 | USD 40,000 | 2 FTE engineering (routing publication, trajectory RAG, HDC fingerprints, anti-gaming) |
| M6: Integration + production | Months 11-12 | USD 25,000 | 1 FTE engineering + infrastructure (TraceCommons, rollup agents, docs, mainnet) |
| **Total** | **12 months** | **USD 200,000** | |

Infrastructure costs (Celestia mainnet blob fees, CI/CD, testnet nodes,
LLM API costs for testing) are included in each milestone's allocation at
approximately 15% of the milestone budget.

## 8. Sustainability plan

### 8.1 During the grant (months 1-12)

Grant funding supports full-time development. All code is open source under
MIT/Apache-2.0 dual license. Community building starts at M3 (tool system
release) with developer documentation, example agents, and tutorial content.

### 8.2 Post-grant sustainability

Three revenue streams sustain tiagent after the grant period:

1. **DA usage fees.** tiagent generates blob submission revenue for the
   Celestia network. As agent adoption grows, the Celestia ecosystem has
   an economic incentive to continue supporting tiagent development through
   ecosystem grants or foundation investment.

2. **Hosted agent service.** A managed tiagent hosting service where
   developers deploy agents without managing infrastructure. The open-source
   codebase remains freely available for self-hosting. Revenue from hosting
   funds ongoing development.

3. **Enterprise support.** Commercial support contracts for organizations
   deploying tiagent in production. Includes priority bug fixes, custom
   tool development, and integration consulting.

4. **TraceCommons credit sharing.** tiagent agents that contribute
   high-quality traces to TraceCommons earn credits. A portion of credit
   revenue flows back to tiagent development.

### 8.3 Open-source community

tiagent is designed for community contribution. The trait system (six verb
traits with default implementations) provides clear extension points.
Custom LLM backends, custom tools, custom gate rungs, and custom routing
strategies can all be contributed as separate crates that implement the
core traits. A contributor guide, "good first issues" program, and monthly
community calls sustain engagement.

## 9. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Celestia DA costs increase significantly | Low | High | Tiered storage design (M2) ensures only high-value data goes on-chain. Local cache handles ephemeral state. Cost estimation tool (M3) helps agents make informed decisions. |
| 0G Labs ships a competitive framework first | Medium | Medium | tiagent's Celestia-native integration and production mainnet access are structural advantages. 0G's early-stage network limits what frameworks can actually do. |
| Insufficient developer adoption | Medium | High | Celestia developer tools as MCP servers (M3) provide value even without adopting the full framework. Developers can use tiagent tools from existing agents. |
| LLM API costs make self-improvement uneconomic | Low | Medium | Cascade router (M4) actively minimizes cost by routing to cheaper models when possible. Local model backends (Ollama) provide cost-free alternatives. |
| Cross-agent learning data is too noisy to be useful | Medium | Medium | Quality gates (M4) and TraceCommons integration (M6) filter for high-value learning data. HDC fingerprint matching (M5) ensures strategy transfer is between behaviorally similar agents. |

## 10. References

1. Celestia documentation: https://docs.celestia.org
2. Celestia Vision 2.0: https://blog.celestia.org/beyond-data-availability/
3. lumina-node (Rust light node): https://github.com/eigerco/lumina
4. celestia-types crate: https://crates.io/crates/celestia-types
5. celestia-rpc crate: https://crates.io/crates/celestia-rpc
6. Model Context Protocol specification: https://modelcontextprotocol.io
7. A2A Protocol: https://github.com/google/A2A
8. RHO: Harness Optimization (arXiv:2606.05922): SWE-Bench 59% to 78% through harness-level improvements
9. Dynamic Cheatsheet (arXiv:2504.07952, ICLR 2026): Persistent strategy memory across agent sessions
10. Sleep-Time Compute (Meta Research): ~5x inference cost reduction through offline pre-computation
11. ERC-8004: Agent identity standard for Ethereum
12. 0G Labs documentation: https://docs.0g.ai
13. TraceCommons: https://github.com/zmanian/trace-commons-server (MIT/Apache-2.0)
14. IronClaw: https://github.com/nickelpack/ironclaw

---

# Proposal 2: Interchain Foundation -- Cosmos Ecosystem Grant

**Program**: Interchain Foundation grants ($26.4M+ historically allocated)
**Requested amount**: USD 100,000
**Duration**: 9 months, 3 phases
**Applications**: Rolling
**URL**: https://interchain.io

## Proposal title

tiagent: AI Agent Infrastructure for the Interchain

## Abstract

The Cosmos ecosystem --- 80+ sovereign chains connected through IBC, secured
by the Cosmos SDK, and scaled through modular data availability on Celestia
--- has no AI agent tooling. Developers managing rollup deployments, IBC
channel configurations, validator monitoring, and cross-chain asset flows do
this work manually or through brittle bash scripts. There is no intelligent
automation layer that understands the Cosmos stack.

tiagent is a Rust agent harness (MIT/Apache-2.0, targeting 12--14 crates)
built natively for the Celestia/Cosmos ecosystem. It provides model-agnostic
AI agent infrastructure with deep integration into Celestia's DA layer (for
shared agent state), the Cosmos SDK (for chain interaction), and IBC (for
cross-chain operations). Agents built with tiagent can deploy rollups, manage
IBC channels, monitor validators, execute DeFi strategies, and automate any
Cosmos SDK operation --- and they get better at these tasks over time through
a cybernetic self-improvement loop.

This proposal funds three phases of development that deliver tiagent's core
harness, Cosmos-aware tooling, and cross-chain agent coordination via IBC.
The result is the first AI agent framework that natively understands the
interchain: IBC channels, Cosmos SDK modules, CometBFT consensus,
CosmWasm contracts, and Celestia DA --- all accessible as agent tools with
intelligent routing, quality gates, and shared learning.

tiagent serves any developer using Cosmos-based chains, not just Celestia.
The IBC-aware tools benefit the entire interchain ecosystem: a developer
managing IBC channels on Neutron, deploying CosmWasm contracts on Osmosis,
or monitoring validators across the Cosmos Hub gets the same self-improving
coding agent experience. The interchain is the natural multi-chain
environment for a coding agent that coordinates across sovereign chains.

The Interchain Foundation's mission is to foster the development of an open,
decentralized network of sovereign, interoperable blockchains. tiagent
advances this mission by bringing intelligent automation to every part of
the interchain stack, lowering the operational burden for chain operators,
and creating new pathways for developer adoption.

## 1. Problem statement

### 1.1 Manual operations across 80+ sovereign chains

The Cosmos ecosystem's strength --- sovereign chains connected through IBC
--- creates an operational challenge. Teams managing chains, relayers,
validators, and cross-chain assets face manual work that grows linearly
with the number of connected chains:

- **IBC channel management**: Neutron maintains 100+ IBC connections.
  Opening, monitoring, and troubleshooting channels requires deep protocol
  knowledge and manual intervention when packets time out or channels close.
- **Validator monitoring**: Each chain requires independent validator
  monitoring --- uptime tracking, missed blocks, jailing events, slashing
  risk, and governance participation.
- **Rollup deployment**: The modular Cosmos stack (Celestia DA + Rollkit or
  Sovereign SDK + shared sequencing via Astria) involves configuring multiple
  components across multiple layers. This is documented but not automated.
- **DeFi operations**: Protocols like Flame (on Astria), Osmosis (50+
  connected chains), and Noble (stablecoin/RWA infrastructure) require
  continuous position management, risk monitoring, and cross-chain
  rebalancing.
- **CosmWasm contract lifecycle**: Compilation, deployment, migration, and
  monitoring of CosmWasm contracts across multiple chains.

No intelligent automation layer exists that understands these operations as
a connected system rather than isolated scripts.

### 1.2 No agent tooling for the Cosmos SDK

The AI agent framework landscape is dominated by tools built for EVM chains
(Ethereum, Base, Arbitrum) or non-blockchain contexts (LangChain, CrewAI).
None of these understand:

- **IBC**: Inter-Blockchain Communication protocol semantics --- channels,
  ports, packets, timeouts, acknowledgments, relayer configuration.
- **Cosmos SDK modules**: Bank, staking, governance, distribution, slashing,
  authz, feegrant, and the module account system.
- **CometBFT consensus**: Validator sets, block production, evidence
  handling, Byzantine fault detection.
- **Celestia DA**: Blob submission, namespace management, DAS verification,
  data availability proofs.
- **CosmWasm**: Contract compilation, instantiation, execution, migration,
  and query semantics.

When a developer asks an AI agent to "set up an IBC channel between chain A
and chain B," the agent has no tools, no context, and no strategies for this
task. tiagent provides all three.

### 1.3 No cross-chain agent coordination

Multi-chain operations in Cosmos require coordination across chains. A DeFi
strategy that spans Osmosis, Neutron, and Noble involves transactions on
three separate chains, mediated by IBC transfers and relayer operations.
Today, this coordination is handled by monolithic applications or manual
orchestration. There is no agent-native coordination primitive that uses
IBC for cross-chain agent communication.

## 2. Technical approach

### 2.1 tiagent overview

tiagent is described fully in the Celestia Foundation proposal above. For
the ICF context, the key points are:

- **Rust toolkit**: 12--14 crates, MIT/Apache-2.0, model-agnostic (Claude,
  GPT, Gemini, Llama, Ollama, and any OpenAI-compatible API).
- **Core architecture**: 1 noun (Signal) + 6 verb traits (Substrate, Scorer,
  Gate, Router, Composer, Policy) + universal loop (query, score, route,
  compose, act, verify, write, react).
- **Celestia DA substrate**: Agent state, traces, and learning artifacts
  stored as namespace-organized blobs on Celestia's DA layer.
- **Self-improvement**: Three nested feedback loops (execution, learning,
  cross-agent) with cascade routing, adaptive gates, and trajectory RAG.
- **MCP integration**: Model Context Protocol client and server for tool
  interoperability.

This proposal focuses on the Cosmos-specific extensions that the ICF is
uniquely positioned to fund.

### Phase 1: Core harness + Celestia DA substrate (months 1-3, USD 35,000)

This phase delivers the foundational tiagent runtime with Celestia
integration (corresponding to M1-M2 of the Celestia Foundation proposal,
but scoped to what the ICF cares about: interchain relevance).

**Deliverables:**

1. **Core harness**: Signal types, verb traits, universal loop, CLI binary,
   two LLM backends (Claude API, OpenAI-compatible), local substrate,
   episode logger.

2. **Celestia DA substrate**: `CelestiaSubstrate` implementation, namespace
   management, light node embedding via lumina, Mocha testnet integration.

3. **Cosmos SDK RPC client**: A generic `CosmosClient` that speaks the
   Cosmos SDK REST/gRPC API. Query bank balances, submit transactions, query
   staking state, interact with governance. Built on `cosmrs` and
   `tendermint-rpc`.

4. **CometBFT integration**: Query block data, validator sets, consensus
   state, and evidence. Monitor chain health and detect anomalies.

**Verification criteria:** `tiagent run "check the validator set on Neutron
mainnet and report any validators with more than 5% missed blocks in the
last 1000 blocks"` executes successfully, producing a structured report.

### Phase 2: Cosmos-aware tools (months 4-6, USD 35,000)

The tool suite that makes tiagent useful for Cosmos developers.

**Deliverables:**

1. **IBC tools:**

   | Tool | What it does |
   |---|---|
   | `ibc_channel_open` | Open an IBC channel between two chains (init + try + ack + confirm handshake) |
   | `ibc_channel_status` | Check channel state, pending packets, timeout status |
   | `ibc_transfer` | Execute an ICS-20 token transfer with timeout handling |
   | `ibc_relayer_config` | Generate or update Hermes relayer configuration |
   | `ibc_packet_trace` | Trace a packet through its lifecycle (send, recv, ack, timeout) |
   | `ibc_channel_monitor` | Continuous monitoring of channel health with alerting |

2. **Cosmos SDK tools:**

   | Tool | What it does |
   |---|---|
   | `cosmos_tx_submit` | Build, sign, and submit a Cosmos SDK transaction |
   | `cosmos_query` | Query any Cosmos SDK module state (bank, staking, gov, etc.) |
   | `cosmos_gov_vote` | Submit governance votes across multiple chains |
   | `cosmos_staking_delegate` | Delegate, redelegate, or undelegate stake |
   | `cosmos_authz_grant` | Manage authorization grants between accounts |

3. **CosmWasm tools:**

   | Tool | What it does |
   |---|---|
   | `cosmwasm_compile` | Compile a CosmWasm contract with optimizer |
   | `cosmwasm_deploy` | Store code, instantiate contract, verify deployment |
   | `cosmwasm_execute` | Execute contract messages with gas estimation |
   | `cosmwasm_migrate` | Migrate contract to a new code version |
   | `cosmwasm_query` | Query contract state |

4. **Rollup deployment tools:**

   | Tool | What it does |
   |---|---|
   | `rollkit_deploy` | Generate configuration and deploy a Rollkit rollup using Celestia DA |
   | `sovereign_deploy` | Generate configuration and deploy a Sovereign SDK rollup |
   | `astria_sequencer_status` | Query Astria shared sequencer status |

**Verification criteria:** `tiagent run "deploy a CosmWasm counter contract
to Neutron testnet and execute 10 increment operations"` executes
successfully, producing a deployed contract address and 10 confirmed
transactions.

### Phase 3: Cross-chain agent coordination via IBC (months 7-9, USD 30,000)

The novel contribution for the interchain: agents that coordinate across
chains using IBC as the communication layer.

**Deliverables:**

1. **IBC agent coordination protocol.** A protocol for agents on different
   chains to coordinate through IBC packets. Agent A on Neutron can request
   Agent B on Osmosis to execute a specific operation, with the request,
   acknowledgment, and result all transported via IBC.

   The protocol uses a custom IBC port (`tiagent-coord`) and channel type
   for agent-to-agent messages. Messages are structured as:

   ```rust
   pub struct AgentCoordinationMessage {
       pub sender_agent_id: AgentId,
       pub request_type: CoordinationRequest,
       pub payload: Vec<u8>,          // Serialized task specification
       pub timeout_height: Height,
       pub timeout_timestamp: u64,
   }

   pub enum CoordinationRequest {
       TaskRequest,           // "Please execute this task"
       TaskResult,            // "Here is the result"
       StrategyShare,         // "This strategy worked for me"
       StateQuery,            // "What is the state of X?"
       CoordinationProof,     // "I attest to outcome Y"
   }
   ```

2. **Multi-chain DeFi agent.** A reference agent that executes a cross-chain
   DeFi strategy spanning Osmosis, Neutron, and Noble:
   - Monitor price feeds on Osmosis
   - Execute swaps through Osmosis DEX
   - Bridge assets via IBC to Neutron or Noble
   - Manage positions on Neutron DeFi protocols
   - Rebalance based on risk parameters

3. **Cross-chain validator monitoring.** A reference agent that monitors
   validator performance across multiple Cosmos chains simultaneously,
   aggregating metrics and generating alerts when validators miss blocks,
   get jailed, or face slashing risk.

4. **Coordination proofs on Celestia.** Multi-agent coordination outcomes
   are attested and published to Celestia's DA layer. These proofs enable
   third-party verification that agents collaborated correctly, creating
   an audit trail for multi-chain agent operations.

**Verification criteria:** Two agents on separate testnets coordinate
through IBC to execute a cross-chain token transfer. The coordination
flow (request, execution, result, attestation) completes end-to-end in
under 60 seconds, with the coordination proof published to Celestia Mocha.

## 3. Relevance to ICF mission

### 3.1 Supporting the modular thesis

Celestia's modular architecture --- DA separated from execution --- is a
core pillar of the Cosmos ecosystem's technical thesis. tiagent demonstrates
a novel application of this modularity: using the DA layer not just for
rollup block data, but for AI agent coordination and shared learning. This
expands the value proposition of modular architecture to a new domain.

### 3.2 Driving IBC adoption

tiagent's IBC tools and cross-chain agent coordination protocol create new
IBC usage. Every cross-chain agent operation produces IBC packets. As agent
automation grows, IBC traffic grows proportionally. The agent coordination
protocol itself is a new IBC application that demonstrates IBC's
extensibility beyond token transfers.

### 3.3 Cosmos SDK ecosystem growth

The Cosmos-aware tool suite lowers the barrier for developers to interact
with Cosmos SDK chains. Developers who are already using AI agents (Claude
Code, Codex) can access Cosmos SDK operations through tiagent's MCP tools
without learning the Cosmos SDK directly. This is a new onboarding pathway.

### 3.4 Developer experience

The operational burden of managing multiple Cosmos chains, IBC channels, and
cross-chain assets is a significant barrier to ecosystem growth. tiagent
automates the repetitive parts of this work --- monitoring, configuration,
deployment, troubleshooting --- so developers can focus on application logic.

## 4. Budget justification

### Phase 1: USD 35,000

| Item | Amount | Notes |
|---|---|---|
| Core harness engineering | USD 15,000 | Signal types, traits, CLI, LLM backends, episode logger |
| Celestia DA substrate | USD 12,000 | CelestiaSubstrate, namespace mgmt, lumina integration |
| Cosmos SDK client | USD 5,000 | cosmrs + tendermint-rpc based client, CometBFT queries |
| Infrastructure | USD 3,000 | Mocha testnet, CI/CD, LLM API costs for testing |

### Phase 2: USD 35,000

| Item | Amount | Notes |
|---|---|---|
| IBC tools | USD 14,000 | 6 IBC-specific tools with Hermes relayer integration |
| Cosmos SDK tools | USD 8,000 | 5 general Cosmos SDK tools |
| CosmWasm tools | USD 6,000 | 5 CosmWasm lifecycle tools |
| Rollup deployment tools | USD 4,000 | Rollkit, Sovereign SDK, Astria tools |
| Infrastructure | USD 3,000 | Multi-chain testnet access, relayer nodes, CI/CD |

### Phase 3: USD 30,000

| Item | Amount | Notes |
|---|---|---|
| IBC coordination protocol | USD 14,000 | Protocol design, IBC module implementation |
| Reference agents | USD 10,000 | Multi-chain DeFi agent, cross-chain validator monitor |
| Coordination proofs | USD 3,000 | Celestia DA attestation integration |
| Infrastructure | USD 3,000 | Multi-chain testnets, relayer operation, LLM API costs |

## 5. Ecosystem projects benefiting

| Project | How tiagent helps | Impact |
|---|---|---|
| **Neutron** | IBC channel management automation across 100+ connections. Automated monitoring, troubleshooting, and channel recovery. | Reduces operational burden for the most IBC-connected Cosmos chain. |
| **Osmosis** | DeFi strategy agents that interact with 50+ connected chains. Liquidity monitoring, arbitrage detection, governance automation. | New user onboarding pathway; reduced manual operations. |
| **Dymension** | RollApp creation and management agents. Automated deployment, monitoring, and upgrade coordination. | Lowers barrier to launching RollApps. |
| **Noble** | Stablecoin/RWA management agents. USDC bridging automation, RWA lifecycle management, compliance monitoring. | Operational efficiency for asset issuance infrastructure. |
| **Astria** | Shared sequencer monitoring and performance analysis agents. Alert routing and incident response automation. | Better observability for shared sequencing infrastructure. |
| **Flame** | DeFi agents on Astria's EVM rollup. Position management, yield optimization, risk monitoring. | Agent-driven DeFi on the newest Cosmos DeFi platform. |
| **Sovereign SDK** (Celestia first-party) | Development agents that scaffold, build, test, and deploy sovereign rollups. | Accelerates development of Celestia's first-party rollup framework. |

## 6. Team qualifications

*[Template -- to be completed by applicants]*

**Project Lead:** [Name]. [Experience with Cosmos ecosystem, Rust
development, agent infrastructure.]

**Senior Engineer:** [Name]. [Experience with IBC protocol, Cosmos SDK
module development, CosmWasm.]

**Agent Systems Engineer:** [Name]. [Experience with LLM integration,
MCP protocol, cross-chain operations.]

Evidence of capability: [Describe relevant prior work with Cosmos SDK,
IBC, Celestia, or agent framework development. Reference specific
repositories, deployments, or contributions.]

## 7. Sustainability plan

Post-grant sustainability follows the same model as the Celestia Foundation
proposal (Section 8). Additionally:

- **IBC tool adoption**: As Cosmos chains adopt tiagent for IBC management,
  operational efficiency gains create demand for commercial support.
- **Multi-chain agent hosting**: Managed hosting of multi-chain agents
  (monitoring, DeFi, operations) as a service.
- **Tool marketplace**: Community-contributed Cosmos-specific tools
  published through tiagent's MCP server registry, with optional paid
  tiers for commercial tools.

## 8. References

1. Cosmos SDK documentation: https://docs.cosmos.network
2. IBC specification: https://github.com/cosmos/ibc
3. IBC-Go implementation: https://github.com/cosmos/ibc-go
4. Hermes IBC relayer: https://github.com/informalsystems/hermes
5. CosmWasm documentation: https://docs.cosmwasm.com
6. Astria documentation: https://docs.astria.org
7. Celestia documentation: https://docs.celestia.org
8. cosmrs crate: https://crates.io/crates/cosmrs
9. tendermint-rpc crate: https://crates.io/crates/tendermint-rpc
10. Neutron documentation: https://docs.neutron.org
11. Osmosis documentation: https://docs.osmosis.zone
12. Noble documentation: https://docs.nobleassets.xyz

---

# Proposal 3: Modular Fellows / Mammothon 2 Submission

## Track A: Modular Fellows (Next Cohort)

**Program**: Celestia Modular Fellows
**Amount**: $9,000 stipend
**Duration**: 3 months
**Format**: Individual fellowship

### Application title

Building the First Self-Improving Agent Harness on Celestia

### Personal statement

*[Template -- to be completed by applicant]*

I am a Rust systems engineer building AI agent infrastructure. My previous
work includes [roko, a 177K LOC Rust toolkit for self-building agents /
polkagent, a 90-crate Polkadot agent framework / other relevant work].
I am applying to Modular Fellows because I believe Celestia's DA layer is
the ideal substrate for a new class of self-improving AI agents, and I want
to build the framework that proves it.

The agent framework landscape has a Celestia-shaped hole. Every major
blockchain ecosystem has at least one dedicated agent framework: Solana has
the Solana Agent Kit and ARC, Polkadot has polkagent, NEAR has IronClaw,
and Base has Coinbase AgentKit. Celestia --- the leading modular blockchain
with 56+ rollups, 128 MB blocks, and a $204M funding runway --- has none.

This gap is not just a missing library. It is a missing narrative. 0G Labs
has claimed the "AI DA" position with $108 million in AI-specific funding,
despite having less mature infrastructure than Celestia. The first team to
build credible AI agent tooling on Celestia rewrites that narrative.

During the fellowship, I will build tiagent: a minimal, model-agnostic Rust
toolkit for running AI agents that use Celestia's DA layer for shared state,
audit trails, and cross-agent learning.

### Fellowship deliverables (3 months)

**Month 1: Core harness**

- `tiagent-core` crate with Signal types, six verb traits, and the
  universal loop
- `tiagent-cli` binary with `run`, `status`, and `doctor` subcommands
- Two LLM backends (Claude API + OpenAI-compatible)
- Local file-based substrate for development
- Episode logging in TraceCommons-compatible format

Deliverable: `tiagent run "list files in this directory"` works end-to-end.

**Month 2: Celestia DA substrate**

- `tiagent-celestia` crate implementing the `Substrate` trait over Celestia
  blobs
- Namespace hierarchy for agent data (traces, routing, vectors, fingerprints)
- Blob submission and retrieval via `celestia-rpc`
- Optional lumina-node embedding for DAS verification
- Mocha testnet integration with CI pipeline

Deliverable: `tiagent run "submit my last agent trace to Celestia Mocha
testnet"` writes a blob and verifies inclusion.

**Month 3: Self-improvement + demo**

- Cascade router with EMA-updated routing weights
- Episode analysis (task completion rate, token efficiency, cost tracking)
- Routing weight publication to Celestia DA (shared learning)
- Celestia developer tools (blob submit, namespace query, cost estimation)
  exposed as MCP tools
- Public demo: `tiagent run "deploy a Rollkit rollup on Mocha testnet"`

Deliverable: A live demo showing an agent deploying a rollup, with the
execution trace published to Celestia and routing weights shared for other
agents to learn from.

### Why Modular Fellows

The fellowship provides three things that accelerate tiagent:

1. **Celestia team access.** Direct interaction with Celestia engineers on
   DA layer integration questions, namespace design review, and light node
   embedding best practices.

2. **Ecosystem connections.** Introductions to Sovereign SDK (now first-
   party), Rollkit, Astria, and other ecosystem teams who would be early
   adopters of Celestia-native agent tooling.

3. **Credibility signal.** A fellowship endorsement validates tiagent's
   approach and makes subsequent grant applications (Celestia Foundation,
   ICF) significantly stronger.

The $9,000 stipend does not fund the full project --- it funds the initial
3-month sprint that produces a demo-able MVP, which then supports larger
grant applications.

---

## Track B: Mammothon 2 Submission

**Program**: Celestia Mammothon (hackathon)
**Target**: Finalist prize ($25,000)
**Duration**: Hackathon period (typically 2-4 weeks)

### Project title

tiagent: Self-Improving Agents on Celestia DA

### One-line description

A Rust agent harness that uses Celestia's DA layer for shared state and
cross-agent learning --- demonstrated by an agent that deploys a rollup to
Mocha testnet.

### Problem

Celestia has no native agent tooling. Developers building agents that
interact with Celestia must write custom blob submission, namespace
management, and light node verification from scratch. Meanwhile, 0G Labs
has invested $108M in positioning itself as the AI DA layer, despite having
less mature infrastructure.

### Solution

tiagent --- a minimal Rust agent harness with native Celestia integration.
For the hackathon, the scope is:

1. **Core loop**: Signal types, two LLM backends, basic CLI
2. **Celestia substrate**: Blob submit/get to Mocha via `celestia-rpc`
3. **Demo agent**: "Deploy a Rollkit rollup on Mocha testnet"

### Demo script

The demo leads with a general coding task --- the kind any developer does
daily --- then reveals the DA-backed learning artifacts as a bonus.

```bash
# Install tiagent
cargo install tiagent-cli

# Configure (Celestia node, LLM backend)
tiagent init
tiagent config set celestia.node "https://rpc-mocha.pops.one"
tiagent config set agent.backend "claude"

# PART 1: General coding task (this is what developers care about)
tiagent run "implement a REST API with authentication \
  using Axum. Include JWT token generation, middleware \
  for protected routes, user registration, and login endpoints. \
  Write tests for all endpoints."

# View the result
tiagent status           # Shows task outcome, files created
tiagent replay latest    # Walk through the agent's decisions

# PART 2: The DA learning artifacts (this is what makes it novel)
# The agent trace is now a blob on Celestia Mocha:
tiagent celestia blobs --namespace tiagent/traces --latest

# Routing weights published --- other tiagent instances learn
# which model worked best for "REST API" tasks:
tiagent celestia blobs --namespace tiagent/routing --latest

# PART 3: Show a Celestia-specific task too
tiagent run "Deploy a new Rollkit rollup called 'mammothon-demo' \
  using Celestia Mocha as the DA layer. Generate the configuration, \
  submit the genesis blob, and verify the deployment."
```

The demo script tells a story: tiagent is a coding agent first (Part 1),
Celestia-powered second (Part 2), and Celestia-native when you need it
(Part 3).

### Technical architecture (for hackathon scope)

```
┌─────────────────────────────────────────────┐
│  tiagent-cli                                │
│  ┌──────────────┐  ┌──────────────────────┐ │
│  │ tiagent-core │  │ tiagent-agent        │ │
│  │ Signal types │  │ Claude + OpenAI      │ │
│  │ Verb traits  │  │ backends             │ │
│  │ Loop engine  │  │                      │ │
│  └──────┬───────┘  └──────────┬───────────┘ │
│         │                     │             │
│  ┌──────▼─────────────────────▼───────────┐ │
│  │ tiagent-celestia                       │ │
│  │ CelestiaSubstrate: Substrate trait     │ │
│  │ over celestia-rpc                      │ │
│  └────────────────┬───────────────────────┘ │
└───────────────────┼─────────────────────────┘
                    │
                    ▼
          Celestia Mocha Testnet
          (blob storage + DAS verification)
```

### What makes this novel

1. **First Celestia-native agent framework.** Not a wrapper around
   LangChain with a Celestia RPC call. A framework designed from the
   ground up around Celestia's namespace/blob/DAS model.

2. **DA as shared agent memory.** Agent traces go on-chain, so other
   agents can learn from them. This is a new DA consumer category.

3. **Self-improving.** The agent records its own performance and adjusts
   its routing for future tasks. If Claude fails but GPT succeeds at rollup
   configuration, the agent learns to use GPT for that task type.

### Judging criteria alignment

| Criterion | How tiagent addresses it |
|---|---|
| **Innovation** | First framework to use DA for cross-agent learning. Novel application of Celestia's modular architecture to AI agent infrastructure. |
| **Technical execution** | Rust, production-quality code, real Celestia integration (not mock), verifiable blob submission on Mocha. |
| **Ecosystem impact** | Creates new DA consumer category. Provides Celestia developer tools. Positions Celestia against 0G in AI narrative. |
| **Completeness** | Working demo: install, configure, run, see result on Celestia. Not a slide deck. |
| **Scalability** | Architecture designed for 12--14 crates, but hackathon demo is 3--4 crates. Foundation for full framework. |

---

# Proposal 4: NLnet NGI Zero Restack -- Open-Source AI Coding Agent

**Program**: NLnet NGI Zero Restack (funded by European Commission)
**Requested amount**: EUR 48,000
**Duration**: 12 months, 4 milestones
**Deadline**: November 3, 2026
**URL**: https://nlnet.nl/ngi0/restack/

## Proposal title

tiagent: Open-Source, Self-Improving Coding Agent with Collective Learning

## Problem

Commercial coding agents (Claude Code, Codex, Cursor) collect your coding
traces unilaterally. They learn from your work --- your debugging strategies,
your architectural patterns, your hard-won solutions --- and never share the
improvement back. The knowledge extraction is one-directional: you pay for
the tool, the tool learns from you, the vendor captures the value. You
never benefit from the collective experience of other developers using the
same tool.

This is a user data sovereignty problem. Developers have no control over how
their coding traces are used, no visibility into what the vendor learns from
them, and no way to benefit from collective learning on their own terms.
The current model mirrors the surveillance economics of social media: the
product is free (or cheap), and you are the data source.

## Solution

tiagent is an open-source (MIT/Apache-2.0), self-improving coding agent
where learning is bilateral. You contribute traces (optionally, with
privacy controls), and you benefit from the collective learning pool.
The key principles:

1. **Your traces stay yours.** tiagent runs locally. Your coding sessions,
   tool calls, and outcomes are stored in a local `.tiagent/` directory.
   Nothing leaves your machine without explicit opt-in.

2. **Sharing is opt-in and privacy-preserving.** When you choose to
   contribute traces to the collective learning pool, a local redaction
   pipeline strips sensitive content (secrets, proprietary code patterns,
   PII) before publication. You control what is shared.

3. **Learning is shared via open protocol.** Collective learning artifacts
   (routing weights, trajectory embeddings, behavioral fingerprints) are
   published to an open, verifiable data layer --- not a vendor-controlled
   database. Any tiagent instance can consume the learning pool. The
   protocol is open; no single entity controls access.

4. **The agent improves from collective experience.** tiagent uses three
   nested feedback loops: per-task self-correction, cross-session learning
   (cascade routing, adaptive gates, strategy memory), and cross-agent
   collective learning (trajectory RAG, routing bootstrap, behavioral
   fingerprint matching).

tiagent competes with Claude Code, Codex, and Cursor on capability --- it
writes code, runs tests, debugs, refactors, and deploys using any LLM
backend (Claude, GPT, Gemini, Llama, Ollama). What distinguishes it is the
learning model: open, bilateral, and user-sovereign, rather than closed,
extractive, and vendor-captured.

## Technical approach

### Architecture

tiagent is written in Rust (targeting 12--14 crates) with a "1 noun + 6
verbs" architecture:

- **Signal**: The universal data type. Every piece of data flowing through
  tiagent is a content-addressed, typed, scored datum.
- **Substrate**: Reads and writes signals (local files, or optionally a
  shared data layer for collective learning).
- **Scorer**: Evaluates signal quality across dimensions.
- **Gate**: Validates agent outputs (compilation, tests, lint, diff review).
- **Router**: Selects models and strategies based on task features and
  historical performance.
- **Composer**: Assembles system prompts from layered templates and context.
- **Policy**: Enforces safety contracts and behavioral constraints.

### Self-improvement loop

- **Inner loop (per-task)**: Agent observes tool call outcomes, adjusts
  strategy, records execution trace.
- **Middle loop (across tasks)**: Cascade router adjusts model selection via
  EMA updates. Gate pipeline tracks per-rung pass rates and adjusts
  thresholds adaptively. Strategy memory persists successful approaches.
- **Outer loop (across agents)**: Agents publish learning artifacts to a
  shared, verifiable data layer. Other agents retrieve relevant trajectories
  via embedding similarity, bootstrap from collective routing experience,
  and use behavioral fingerprint matching to find transferable strategies.

### Privacy-preserving trace contribution

The local redaction pipeline runs before any trace leaves the developer's
machine:

1. **Secret detection**: Scan for API keys, tokens, passwords, connection
   strings. Replace with type-tagged placeholders.
2. **Code abstraction**: Replace proprietary code with structural summaries
   (AST shape, dependency patterns, error types) that preserve learning
   value without exposing implementation details.
3. **PII scrubbing**: Remove file paths, usernames, organization names,
   and other identifying information.
4. **Differential privacy**: Add calibrated noise to numerical metrics
   (token counts, timing data) to prevent re-identification through
   statistical analysis.

Developers can review redacted traces before publication and set per-project
policies for what categories of data are shareable.

## Milestones

### M1: Core self-improving harness (months 1-3, EUR 12,000)

The foundational runtime that makes self-improvement work.

**Deliverables:**
- Core crate with Signal types, six verb traits, universal loop
- CLI binary (`tiagent run`, `tiagent status`, `tiagent doctor`)
- Model-agnostic LLM dispatch (Claude API, OpenAI-compatible, Ollama)
- Local file-based substrate for trace storage
- Episode logger (structured recording of every agent turn)
- Cascade router with EMA-updated routing weights
- Adaptive gate thresholds (compilation, tests, lint, diff review)
- Persistent strategy memory (dynamic cheatsheet)

**Verification:** After 10 executions of similar coding tasks, the cascade
router demonstrably routes to the more cost-effective model for that task
type.

### M2: Privacy-preserving trace contribution (months 4-6, EUR 12,000)

The privacy layer that makes trace sharing safe.

**Deliverables:**
- Local redaction pipeline (secret detection, code abstraction, PII
  scrubbing, differential privacy)
- Per-project sharing policies (what data categories are shareable)
- Pre-publication trace review interface
- TraceCommons-compatible envelope format for contributed traces
- Quality gates (novelty + substance) for filtering low-value traces

**Verification:** A developer can contribute traces from a private
commercial project, and the redacted output contains no secrets,
proprietary code, or identifying information, while retaining enough
structure for trajectory RAG to find relevant similar tasks.

### M3: Collective learning protocol (months 7-9, EUR 12,000)

The shared learning layer that makes tiagent collectively intelligent.

**Deliverables:**
- Routing weight publication and consumption (agents share which models
  work for which task types)
- Trajectory RAG (retrieve similar successful trajectories as in-context
  examples for new tasks)
- HDC behavioral fingerprinting (compact behavioral signatures for
  strategy transfer between similar agents)
- Pluggable backend for the shared data layer (DA-backed for verifiable
  storage, or direct peer-to-peer for simplicity)
- Anti-gaming measures (Sybil resistance, anomaly detection on routing
  distributions)

**Verification:** Agent B, starting with no local learning state,
bootstraps from Agent A's published routing weights and achieves within 10%
of Agent A's task completion rate within 5 task executions (vs. 20+ from
cold start).

### M4: Developer documentation and SDK (months 10-12, EUR 12,000)

Documentation, tooling, and SDK for community adoption.

**Deliverables:**
- Comprehensive documentation: getting started, architecture guide, privacy
  model explanation, self-improvement loop tutorial, API reference
- SDK for building custom tools, gates, and routing strategies
- Contributor guide and "good first issues" program
- Integration guides for existing developer workflows (VS Code, terminal,
  CI/CD pipelines)
- Performance benchmarks against Claude Code, Codex, and Cursor on standard
  coding tasks (SWE-Bench or equivalent)

**Verification:** A developer with no prior tiagent experience can follow
the getting started guide, install tiagent, run it on a coding task, and
see collective learning benefits within 30 minutes.

## Budget

| Milestone | Duration | Amount | Key costs |
|---|---|---|---|
| M1: Core self-improving harness | Months 1-3 | EUR 12,000 | Engineering: core crate, CLI, LLM dispatch, cascade router, adaptive gates |
| M2: Privacy-preserving trace contribution | Months 4-6 | EUR 12,000 | Engineering: redaction pipeline, sharing policies, quality gates |
| M3: Collective learning protocol | Months 7-9 | EUR 12,000 | Engineering: trajectory RAG, routing publication, HDC fingerprints, anti-gaming |
| M4: Developer documentation and SDK | Months 10-12 | EUR 12,000 | Engineering: docs, SDK, benchmarks, integration guides |
| **Total** | **12 months** | **EUR 48,000** | |

## Relevance to NGI Zero Restack

tiagent directly addresses NGI Zero Restack's focus areas:

- **User data sovereignty**: Developers control their coding traces. Sharing
  is opt-in with local redaction. No vendor captures learning unilaterally.
- **Open internet infrastructure**: The collective learning protocol is open
  and not controlled by a single entity. Any conforming agent can participate.
- **Alternative to vendor-controlled AI tools**: Commercial coding agents
  extract value from developer traces with no reciprocity. tiagent provides
  a bilateral alternative where developers both contribute and benefit.
- **Open-source commons**: MIT/Apache-2.0 dual license. The trait system
  provides clear extension points for community contribution.
- **European digital sovereignty**: tiagent can run entirely on local models
  (Ollama) with no data leaving the developer's machine, supporting
  data-sovereign AI development workflows.

## Team qualifications

*[Template -- to be completed by applicants]*

Evidence of capability: the team has designed the tiagent architecture
(15-document design suite totaling 50,000+ words), built related systems
(roko: 177K LOC Rust, 18 crates, fully self-hosting agent toolkit), and
has production experience with privacy-preserving data systems and
open-source Rust infrastructure.

## References

1. NGI Zero Restack program: https://nlnet.nl/ngi0/restack/
2. Model Context Protocol specification: https://modelcontextprotocol.io
3. RHO: Harness Optimization (arXiv:2606.05922): Harness-level improvements
   raise SWE-Bench from 59% to 78%
4. Dynamic Cheatsheet (arXiv:2504.07952, ICLR 2026): Persistent strategy
   memory across agent sessions
5. TraceCommons: https://github.com/zmanian/trace-commons-server
6. Differential privacy for machine learning: Dwork & Roth, "The Algorithmic
   Foundations of Differential Privacy"

---

# Part 2: Supporting Materials

## Appendix A: Competitive Matrix

| Feature | tiagent | Eliza (ai16z) | Rig | ARC (Solana) | polkagent (Polkadot) | Solana Agent Kit | Coinbase AgentKit |
|---|---|---|---|---|---|---|---|
| **Language** | Rust | TypeScript | Rust | Rust | Rust | TypeScript | TypeScript |
| **Chain-native** | Celestia | Multi-chain | Multi-chain | Solana | Polkadot | Solana | Base |
| **DA integration** | Native (blobs, namespaces, DAS) | None | None | None | None | None | None |
| **Self-improving** | Yes (3 feedback loops) | No | No | No | No | No | No |
| **Cross-agent learning** | Yes (via DA) | No | No | No | No | No | No |
| **MCP support** | Client + server | Plugin system | No | No | No | No | Plugin system |
| **Model-agnostic** | Yes (6+ backends) | Limited | Yes | Limited | Yes | Limited | Limited |
| **Quality gates** | Multi-rung pipeline | None | None | None | None | None | None |
| **Shared state** | On-chain (Celestia DA) | Off-chain DB | Off-chain | On-chain (Solana) | On-chain (Polkadot) | On-chain (Solana) | Off-chain |
| **Light node** | Embedded (lumina) | N/A | N/A | N/A | N/A | N/A | N/A |

## Appendix B: Celestia Ecosystem Data

### Network statistics (mid-2026)

| Metric | Value | Source |
|---|---|---|
| Rollups on mainnet | 56+ | Celestia blog |
| Maximum block size | 128 MB (post-Matcha) | Celestia docs |
| Target throughput (V8/Fibre) | 1 Tb/s | Celestia roadmap |
| Block time | ~12 seconds (3s target after V8) | Celestia docs |
| Blob cost | $0.07--$0.81/MB | On-chain data |
| Total funding | $204M ($100M Series C, Sep 2024) | Crunchbase |
| Namespace format | 29 bytes (1 byte version + 28 bytes ID) | celestia-types |
| Light node software | lumina-node (production Rust) | GitHub |
| DA sampling | Reed-Solomon 2D erasure coding | Celestia whitepaper |
| Finality | Single-slot (~12 seconds) | Celestia docs |

### Key ecosystem projects

| Project | Category | Status | Relationship to tiagent |
|---|---|---|---|
| Eclipse | L2 (SVM execution, Celestia DA) | Production mainnet | Largest DA consumer; ops automation target |
| Sovereign SDK | Rollup framework | Acquired by Celestia Labs (July 2026); now first-party | Development agent target |
| Rollkit | Rollup framework | Production | Deployment automation target |
| Astria | Shared sequencer | Production | Built Flame DeFi; monitoring target |
| Flame | DeFi protocol | Production (on Astria) | DeFi agent target |
| Caldera | RaaS provider | Production | Deployment automation target |
| Conduit | RaaS provider | Production | Deployment automation target |
| Dymension | RollApp platform | Production mainnet | RollApp management target |
| Manta Network | ZK L2 | Production mainnet | ZK proof agent potential |
| OnchainDB | AI-queryable database | Active | Data source for agents |
| Neutron | Cosmos Hub consumer chain | Production (100+ IBC) | IBC management target |
| Osmosis | DEX | Production (50+ chains) | DeFi agent target |
| Noble | Stablecoin/RWA infrastructure | Production | Asset management target |

### Celestia competitive position vs. 0G Labs

| Dimension | Celestia | 0G Labs |
|---|---|---|
| Mainnet status | Production (since Oct 2023) | Testnet |
| Rollup ecosystem | 56+ production rollups | 0 (pre-launch) |
| Block size | 128 MB (Matcha), 1 Tb/s target | 50 MB target |
| DAS implementation | Production (lumina-node v1.0) | Not available |
| Light nodes | Production Rust implementation | Not available |
| Ecosystem value | $204M funding + established TVL | $290M funding, pre-revenue |
| AI-specific funding | $0 | $108.88M ($88.88M + $20M) |
| AI-specific grants | None | Up to $1M+ per project |
| AI branding | None | "Purpose-built for AI" |

## Appendix C: AI Agent Market Context

### Market size

| Source | Estimate | Timeframe |
|---|---|---|
| Grand View Research | $22.6B | 2026 |
| Markets and Markets | $27B | 2026 |
| Gartner (agentic AI) | 33% of enterprise software by 2028 | Prediction |

### Protocol adoption

| Protocol | Metric | Value |
|---|---|---|
| MCP (Model Context Protocol) | Monthly SDK downloads | 97M+ |
| A2A (Agent-to-Agent) | Member organizations | 150+ |
| ERC-8004 (agent identity) | Status | EIP progressing |
| AITP (AI Transfer Protocol) | Status | Active development |
| x402 (paid API access) | Status | Active development |

### Key research

| Paper | What it shows | Relevance to tiagent |
|---|---|---|
| RHO (arXiv:2606.05922) | Harness optimization raises SWE-Bench from 59% to 78% | The harness matters more than the model. tiagent optimizes the harness. |
| Dynamic Cheatsheet (arXiv:2504.07952, ICLR 2026) | Persistent strategy memory improves agent performance across sessions | tiagent implements persistent cheatsheets via DA-backed state. |
| Sleep-Time Compute (Meta) | ~5x inference cost reduction through offline pre-computation | tiagent's middle loop performs offline consolidation between task executions. |

## Appendix D: Technical Specifications

### Crate structure (target)

```
tiagent/                           Workspace root
├── Cargo.toml                     Workspace manifest
├── crates/
│   ├── tiagent-core/              Signal types, 6 verb traits, universal loop
│   ├── tiagent-agent/             LLM backends, tool dispatch, MCP client
│   ├── tiagent-celestia/          CelestiaSubstrate, namespace mgmt, lumina
│   ├── tiagent-gate/              Multi-rung gate pipeline, adaptive thresholds
│   ├── tiagent-compose/           System prompt assembly, templates
│   ├── tiagent-learn/             Episodes, cascade router, efficiency tracking
│   ├── tiagent-store/             Local substrate (JSONL), caching layer
│   ├── tiagent-tools/             Built-in tools (file, shell, HTTP, Celestia)
│   ├── tiagent-mcp-server/        MCP server binary (exposes Celestia tools)
│   ├── tiagent-cli/               CLI binary (main entry point)
│   ├── tiagent-serve/             HTTP control plane (optional)
│   └── tiagent-primitives/        HDC vectors, embeddings, fingerprints
├── tools/
│   ├── tiagent-mcp-cosmos/        MCP server for Cosmos SDK tools
│   └── tiagent-mcp-ibc/           MCP server for IBC tools
└── docs/                          Documentation site source
```

### Key external dependencies

| Dependency | Version | What it provides |
|---|---|---|
| `celestia-rpc` | 1.0 | Celestia JSON-RPC client (blob submit, get, header) |
| `celestia-types` | 1.0 | Celestia data types (Namespace, Blob, Share, etc.) |
| `lumina-node` | 1.0 | Production Rust light node (DAS, header sync) |
| `cosmrs` | 0.20+ | Cosmos SDK types and transaction building |
| `tendermint-rpc` | 0.40+ | CometBFT RPC client |
| `tokio` | 1.x | Async runtime |
| `serde` | 1.x | Serialization/deserialization |
| `axum` | 0.7+ | HTTP server (optional, for serve crate) |
| `clap` | 4.x | CLI argument parsing |
| `ratatui` | 0.28+ | TUI (optional) |
| `fastembed` | latest | Sentence embeddings for trajectory RAG |
| `usearch` | latest | HNSW vector index for similarity search |

### Namespace encoding

```rust
use celestia_types::nmt::Namespace;

/// tiagent uses Celestia's v0 namespace format (29 bytes).
/// The 28-byte ID encodes a hierarchical path:
///
///   [tiagent-prefix (8 bytes)] [data-type (4 bytes)] [agent-id (16 bytes)]
///
/// This allows efficient namespace queries by prefix.
pub fn derive_namespace(data_type: DataType, agent_id: &AgentId) -> Namespace {
    let mut id = [0u8; 28];
    // Prefix: "tiagent\0" (8 bytes)
    id[..8].copy_from_slice(b"tiagent\0");
    // Data type: 4-byte discriminant
    id[8..12].copy_from_slice(&data_type.to_bytes());
    // Agent ID: first 16 bytes of agent's public key hash
    id[12..28].copy_from_slice(&agent_id.truncated_hash());
    Namespace::new_v0(&id).expect("valid namespace")
}

pub enum DataType {
    System,           // 0x00000000
    Trace,            // 0x00000001
    Routing,          // 0x00000002
    Vector,           // 0x00000003
    Fingerprint,      // 0x00000004
    Coordination,     // 0x00000005
    GateAttestation,  // 0x00000006
}
```

### Blob encoding

```rust
use prost::Message;

/// Every tiagent blob uses a self-describing envelope:
///
///   [magic (4 bytes)] [version (2 bytes)] [type (2 bytes)]
///   [header-len (4 bytes)] [header (variable)]
///   [payload-len (4 bytes)] [payload (variable)]
///
/// The magic bytes are "TIAG" (0x54494147).
/// Header is protobuf-encoded BlobHeader.
/// Payload is type-dependent (protobuf, CBOR, or raw bytes).
pub fn encode_blob(header: &BlobHeader, payload: &[u8]) -> Vec<u8> {
    let header_bytes = header.encode_to_vec();
    let mut blob = Vec::with_capacity(12 + header_bytes.len() + payload.len());
    blob.extend_from_slice(b"TIAG");                           // magic
    blob.extend_from_slice(&1u16.to_le_bytes());               // version
    blob.extend_from_slice(&header.blob_type.to_u16().to_le_bytes()); // type
    blob.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(&header_bytes);
    blob.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    blob.extend_from_slice(payload);
    blob
}
```

## Appendix E: Cross-Cutting Grant Themes

| Theme | NLnet | Celestia Foundation | ICF | Modular Fellows | Mammothon |
|---|---|---|---|---|---|
| General-purpose coding agent | **Primary** | Supporting | Supporting | Supporting | Supporting |
| User data sovereignty / privacy | **Primary** | -- | -- | -- | -- |
| Competitive positioning (vs. 0G) | -- | **Primary** | Supporting | Mention | Mention |
| Celestia DA as AI substrate | Supporting | **Primary** | Supporting | **Primary** | **Primary** |
| Self-improving agents | **Primary** | Supporting | Supporting | **Primary** | Mention |
| Cross-agent shared learning | **Primary** | **Primary** | Supporting | Supporting | Mention |
| Cosmos/IBC tools | -- | Supporting | **Primary** | -- | -- |
| Cross-chain coordination | -- | Supporting | **Primary** | -- | -- |
| Rollup developer tooling | -- | Supporting | Supporting | Supporting | **Primary** (demo) |
| New DA consumer category | Supporting | **Primary** | Mention | Supporting | Supporting |
| TraceCommons integration | Supporting | Supporting | -- | Mention | -- |
| Open-source commons | **Primary** | Supporting | Supporting | Supporting | -- |

## Appendix F: TraceCommons Integration Detail

tiagent's relationship with TraceCommons creates a flywheel effect:

```
                 tiagent agent executes task
                           │
                           ▼
                 Episode trace generated
                           │
                    ┌──────┴───────┐
                    │              │
                    ▼              ▼
            Local storage     TraceCommons
            (.tiagent/)       submission
                              │
                              ▼
                         Quality gates
                         (novelty + substance)
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
               Rejected            Accepted
               (low quality,       (enters corpus,
                duplicate)          earns credits)
                                       │
                              ┌────────┴────────┐
                              │                 │
                              ▼                 ▼
                     Celestia DA          TraceCommons
                     (namespace blobs)    corpus
                              │                 │
                              ▼                 ▼
                     Other tiagent       Trajectory RAG
                     agents learn        for all agents
                     from traces         (Claude Code,
                                         Codex, etc.)
```

**What TraceCommons provides to tiagent:**
- Quality-gated corpus of agent execution traces from multiple frameworks
- Trajectory RAG: "how did another agent solve a similar task?"
- Credit incentives for trace publication (positive-sum economics)
- Cross-framework learning (not just tiagent-to-tiagent, but tiagent
  learning from Claude Code traces and vice versa)

**What tiagent provides to TraceCommons:**
- A new trace source with rich Celestia-specific metadata
- DA-backed trace provenance (immutable inclusion proofs)
- A reference implementation of TraceCommons integration in a blockchain-
  native agent framework
- Increased trace volume and diversity for the commons

TraceCommons was founded by Zaki Manian (co-created Cosmos SDK, designed and
shipped IBC, built Sommelier DeFi protocol). The shared Cosmos lineage
between TraceCommons' founder and tiagent's ecosystem creates a natural
collaboration pathway.

## Appendix G: Frequently Asked Questions

### "Why not just use LangChain/CrewAI with Celestia RPC calls?"

Three reasons:

1. **Impedance mismatch.** Python/TypeScript frameworks cannot embed a
   Celestia light node, cannot compile to lightweight binaries, and cannot
   integrate with the Rust crate ecosystem (celestia-types, celestia-rpc,
   lumina-node) without FFI overhead.

2. **No DA-native patterns.** Wrapping `celestia_rpc::Client::blob_submit`
   in a LangChain tool gives you blob submission. It does not give you
   namespace-organized state management, tiered storage (local cache + DA +
   archival), DAS verification, or cross-agent learning through namespaced
   blobs. The patterns are different, not just the API calls.

3. **No self-improvement.** LangChain and CrewAI are open-loop: they execute
   the same way every time. tiagent's cybernetic feedback loops mean agents
   get better with use. The harness learns which models work for which
   Celestia tasks, which strategies succeed, and which tool sequences
   produce validated output.

### "Why not build a Celestia plugin for an existing Rust framework like Rig?"

Rig provides LLM abstractions with chain adapters. It is model-aware but
not chain-native. A Celestia "adapter" for Rig would be a thin RPC wrapper
--- the same pattern as Python frameworks, just in Rust. tiagent's value is
the deeper integration: DA as substrate (not just API target), namespace-
organized shared state, light node embedding, cross-agent learning through
blobs, and a cybernetic self-improvement loop that is structurally tied to
the DA layer.

The distinction is adapter vs. substrate. Rig + Celestia adapter = "agent
can call Celestia API." tiagent = "agent lives on Celestia."

### "How does this compare to 0G's AI agent offerings?"

0G has funded $108M in AI ecosystem grants but has not shipped a production
agent framework. Their focus is on DA + execution for AI model serving (0G
Serving, 0G Storage) --- infrastructure for running models, not for running
agents. tiagent is complementary in concept (both use DA for AI) but
competitive in narrative (which DA layer is "the AI chain?").

Celestia's advantages: production mainnet, 56+ rollups, DAS light nodes,
established ecosystem. 0G's advantages: explicit AI branding, massive
AI-specific funding, AI-optimized execution layer.

tiagent lets Celestia compete on technical merit rather than marketing
budget.

### "If tiagent is a general coding agent, why apply for blockchain grants?"

Because the DA-backed collective learning is what makes tiagent genuinely
novel. No other coding agent has this. Claude Code, Codex, and Cursor all
learn from user traces, but that learning is captured by the vendor and
never shared back. tiagent's collective learning layer --- built on
Celestia's append-only, namespace-partitioned, verifiable DA --- is the
structural innovation that separates it from every other coding agent.

And every tiagent user becomes a Celestia DA consumer, growing the ecosystem
organically. A Django developer debugging serializers, a React developer
building components, a Go developer writing microservices --- they all
generate Celestia blobs as a byproduct of using a better coding agent. This
is a growth model that no blockchain-specific tool can match: tiagent grows
the Celestia ecosystem by serving developers who have never heard of
Celestia and do not need to.

The blockchain grants fund the novel infrastructure (DA integration,
verifiable learning, namespace management). The general AI grants (NLnet)
fund the product layer (coding agent, privacy, UX). Together they build a
complete system.

### "What about data availability costs at scale?"

Celestia blob costs are $0.07--$0.81/MB. At current prices, publishing an
agent trace (5--50 KB) costs $0.0004--$0.04 per trace. Even at the high
end, this is negligible compared to LLM inference costs ($0.50--$5.00 per
agent task for a frontier model).

tiagent's tiered storage strategy ensures that only high-value data goes
on-chain. Ephemeral state stays in local cache. Session state goes to DA
(7-day availability window). Archival data goes to IPFS or similar long-
term storage. The cascade router's cost tracking (M4) explicitly accounts
for DA submission costs in its routing decisions.

### "What happens to blobs after the 7-day data availability window?"

Celestia light nodes prune data after 7 days. For tiagent:

- **Active learning data** (routing weights, recent trajectories): refreshed
  continuously, so the 7-day window is sufficient.
- **Archival traces**: published to both Celestia DA (for immediate
  availability and verifiable inclusion) and a persistent store (IPFS,
  Filecoin, or archival Celestia nodes) for long-term retention.
- **Inclusion proofs**: Compact NMT proofs are stored permanently (they are
  small --- hundreds of bytes). These prove a blob was included in a
  specific block, even after the blob data is pruned.

The tiered storage strategy (M2) handles this automatically. Developers do
not need to think about data lifecycle.

---

# Appendix H: Grant Application Checklist

For each grant program, verify the following before submission:

### Celestia Foundation Strategic Grant

- [ ] Warm introduction through ecosystem contacts
- [ ] Verify current grant process (may not have formal application)
- [ ] Align with Vision 2.0 language and priorities
- [ ] Reference specific Celestia blog posts and ecosystem announcements
- [ ] Include competitive 0G comparison with verified data
- [ ] Budget justified at market rates
- [ ] Open-source license confirmed (MIT/Apache-2.0)
- [ ] Mocha testnet demo ready for review meeting

### Interchain Foundation

- [ ] Review current ICF grant guidelines at interchain.io
- [ ] Verify IBC-specific deliverables align with ICF priorities
- [ ] Reference specific Cosmos ecosystem projects (Neutron, Osmosis, etc.)
- [ ] Budget aligned with ICF norms ($100K range)
- [ ] Sustainability plan addresses post-grant ICF expectations
- [ ] Open-source license confirmed
- [ ] Team section includes Cosmos ecosystem experience

### NLnet NGI Zero Restack

- [ ] Submit before November 3, 2026 hard deadline
- [ ] Frame as open-source alternative to vendor-controlled AI coding tools
- [ ] Emphasize user data sovereignty and privacy-preserving trace contribution
- [ ] Do NOT lead with blockchain/Celestia --- mention as optional backend for shared learning
- [ ] Include SWE-Bench or equivalent benchmarking plan
- [ ] Reference European digital sovereignty and data control themes
- [ ] Budget aligned with EUR 48K cap
- [ ] Open-source license confirmed (MIT/Apache-2.0)
- [ ] Verify alignment with current NGI Zero Restack call text

### Modular Fellows

- [ ] Check if next cohort is accepting applications
- [ ] Personal statement tailored to Celestia's modular thesis
- [ ] 3-month deliverables are concrete and demo-able
- [ ] Previous work in Rust/blockchain/agents documented
- [ ] Explain what fellowship access provides that money does not

### Mammothon 2

- [ ] Check if Mammothon 2 is announced and dates are set
- [ ] Scope is hackathon-realistic (3-4 crates, not 12)
- [ ] Demo script is runnable in under 5 minutes
- [ ] Judging criteria alignment is explicit
- [ ] Video demo prepared if required
