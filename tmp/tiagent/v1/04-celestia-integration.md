# tiagent: Celestia DA Layer Integration Design

> **This document describes an optional integration.** tiagent works as a fully functional,
> self-improving coding agent without Celestia. The Celestia DA layer is an optional substrate
> that enables shared cross-agent learning, verifiable traces, and network-wide improvement.
> If you are using tiagent as a standalone coding agent, you can skip this document entirely.

This document explains how tiagent integrates with Celestia's data availability (DA) layer.
It covers what Celestia is, why its architecture suits agent workloads, how tiagent maps its
data model onto Celestia's primitives, and the concrete Rust design for reading and writing
agent state through the DA layer.

If you have not read the preceding documents:

- **01-vision-and-overview.md** explains what tiagent is: a Rust toolkit for building
  self-improving AI agents on the Celestia blockchain ecosystem.
- **02-architecture.md** explains the core abstractions: one noun (Signal), six verb traits
  (Substrate, Scorer, Gate, Router, Composer, Policy), and a universal loop
  (query, score, route, compose, act, verify, write, react).
- **03-crate-structure.md** explains the workspace layout, including `tiagent-celestia`
  (the crate that houses everything in this document) and `tiagent-store` (local storage).

This document assumes no prior knowledge of Celestia. Every concept is explained from first
principles.

---

## Table of Contents

1. [Celestia Primer](#1-celestia-primer)
2. [Why Celestia for Agent State](#2-why-celestia-for-agent-state)
3. [Namespace Design](#3-namespace-design)
4. [Blob Schema](#4-blob-schema)
5. [CelestiaSubstrate Implementation](#5-celestiasubstrate-implementation)
6. [Light Node Embedding](#6-light-node-embedding)
7. [Tiered Storage Strategy](#7-tiered-storage-strategy)
8. [Proof Verification](#8-proof-verification)
9. [Cost Model](#9-cost-model)
10. [Mocha Testnet Setup](#10-mocha-testnet-setup)

---

## 1. Celestia Primer

### What is Celestia?

Celestia is a blockchain, but it is not the kind of blockchain most people are familiar with.
Ethereum, Solana, and most other chains are **monolithic**: they handle data availability,
consensus, and execution all in one system. Celestia is **modular** --- it does exactly one
thing and does it well: it orders data and guarantees that the data is available for anyone
to download.

Celestia does **not** execute transactions. It does not run smart contracts. It does not
maintain account balances. It has no virtual machine. Its only job is to accept blobs of
arbitrary data, order them into blocks, and make those blocks available to the network. Other
systems (rollups, sovereign chains, or --- in tiagent's case --- agent runtimes) submit their
data to Celestia and build their own execution logic on top.

This separation of concerns is called **modular blockchain architecture**:

```
Monolithic chain (e.g., Ethereum):

    ┌───────────────────────────────────────┐
    │  Execution + Consensus + DA           │
    │  (all responsibilities in one system) │
    └───────────────────────────────────────┘

Modular architecture (Celestia's approach):

    ┌───────────────┐
    │  Execution    │  ← Rollups, sovereign chains, agent runtimes
    │  (your logic) │     build this layer themselves
    └───────┬───────┘
            │ submits data to
    ┌───────▼───────┐
    │  Consensus    │  ← Celestia orders the data into blocks
    │  + DA layer   │     and guarantees it is available
    └───────────────┘
```

### How data gets into Celestia: blobs and namespaces

Data is submitted to Celestia as **blobs** (Binary Large Objects). A blob is just a byte
array --- it can contain anything. JSON, protobuf, raw bytes, serialized Rust structs,
images, whatever the submitter wants to store. Celestia does not inspect or interpret blob
contents. It just stores them and makes them available.

Every blob belongs to a **namespace**. A namespace is a 29-byte identifier that partitions
block data so that clients can download only the blobs they care about, rather than the
entire block. The namespace format (version 0) is:

```
┌──────────┬────────────────────────┐
│ 1 byte   │ 28 bytes               │
│ version  │ namespace ID           │
│ (0x00)   │ (user-defined)         │
└──────────┴────────────────────────┘
```

For example, tiagent might use a namespace like:

```
version: 0x00
ID:      "tiagent/system" (padded to 28 bytes)
```

When a client wants to read tiagent's system data from a block, it asks Celestia for "all
blobs in the namespace `tiagent/system` at height N." The node returns only those blobs,
not the full block. This namespace-based filtering is fundamental to Celestia's scalability.

### Data Availability Sampling (DAS)

A defining feature of Celestia is that light nodes --- small, cheap nodes that do not store
the full blockchain --- can verify that data is available without downloading entire blocks.

In a traditional blockchain, verifying data availability means downloading every block and
checking that the data is there. This is expensive. As block sizes grow, the hardware
requirements for full nodes grow with them, which limits how large blocks can be.

Celestia solves this with **Data Availability Sampling (DAS)**:

1. Each block's data is arranged into a 2D matrix.
2. The matrix is extended using Reed-Solomon erasure coding, which adds redundancy. Even if
   up to 50% of the extended data is missing, the original data can be reconstructed.
3. Light nodes randomly sample small portions of the extended matrix and check that the
   samples are valid.
4. If enough light nodes independently sample different portions and all samples check out,
   the network has high statistical confidence that the full data is available.

The important consequence for tiagent: **you do not need a full node to verify that your
agent's blobs were actually included in a Celestia block.** A light node running inside the
agent process can perform DAS to confirm data availability, using only a small fraction of
the block's bandwidth.

```
Traditional verification:

    Light node must download entire block
    ┌──────────────────────────────────┐
    │  ████████████████████████████████│  100% of block data
    └──────────────────────────────────┘

Celestia DAS:

    Light node samples random cells from the extended matrix
    ┌──────────────────────────────────┐
    │  ░░█░░░░░█░░░░░░█░░░░█░░░░░░░░░│  ~1-5% of block data
    └──────────────────────────────────┘
    If all sampled cells are valid → high confidence data is available
```

### Namespaced Merkle Trees (NMT)

Celestia uses a specialized data structure called a **Namespaced Merkle Tree (NMT)** to
organize blobs within a block. An NMT is a standard Merkle tree with one key addition: every
node in the tree is annotated with the minimum and maximum namespace of the data in its
subtree.

This annotation enables **namespace proofs**: a proof that all blobs in a given namespace
have been included (inclusion proof) or that no blobs exist in a given namespace (absence
proof). These proofs are compact and can be verified by a light node without downloading the
full block.

For tiagent, NMT proofs provide:

- **Inclusion verification**: Confirm that a specific agent trace was included in a specific
  block. Useful for audit trails and dispute resolution.
- **Completeness verification**: Confirm that you have retrieved ALL blobs in a namespace at
  a given height, not just some of them. Useful for coordination workflows where missing a
  message could cause incorrect behavior.

### Key numbers

| Property | Value |
|----------|-------|
| Block time | ~12 seconds |
| Maximum block size | 128 MB (post-Matcha upgrade) |
| Blob cost | ~$0.07/MB (testnet) to ~$0.81/MB (mainnet under load) |
| Light node data pruning | 7 days (archival nodes keep everything) |
| Namespace size | 29 bytes (1 byte version + 28 bytes ID) |
| Finality | Single-slot (~12 seconds) |

---

## 2. Why Celestia for Agent State

Agent frameworks typically store state in local files, databases, or cloud storage. tiagent
can optionally use Celestia's DA layer as a shared state substrate. This section explains
what you already get without it, and what Celestia adds on top.

### 2.0 What you already get without Celestia

tiagent is a fully functional, self-improving coding agent using only local storage. Without
any Celestia integration, you get:

- **Cascade model routing** --- the agent learns which LLM performs best for each task
  category and routes accordingly. Weights persist locally in `.tiagent/learn/cascade-router.json`.
- **Adaptive gate thresholds** --- gate pass/fail thresholds adjust automatically based on
  observed results via exponential moving averages.
- **Playbook extraction** --- high-scoring episodes are distilled into reusable strategies
  stored in the local knowledge store.
- **Efficiency tracking** --- per-turn cost, latency, and token usage are logged to
  `.tiagent/learn/efficiency.jsonl` for local analysis and self-tuning.
- **Episode logging** --- every agent execution is recorded as a structured trace in
  `.tiagent/episodes.jsonl`, enabling replay and learning.
- **Plan execution** --- the full plan-execute-gate-persist loop works entirely locally.
  DAG-based task orchestration, parallel execution, and session resume all use local state.
- **Tool calling** --- all 19+ built-in tools and MCP integrations work without DA.
- **PRD workflows** --- idea capture, draft lifecycle, plan generation, and execution are
  entirely local operations.
- **Prompt experiments** --- A/B testing of prompt variations runs locally with results
  stored in `.tiagent/learn/experiments.json`.

All of these capabilities are per-instance: one agent improving itself based on its own
experience. **Celestia adds the ability to share that improvement across agent instances.**

### 2.1 Shared visibility without a central server

When agent state lives on a local filesystem, other agents cannot see it. When it lives in a
cloud database, access depends on whoever controls that database. Celestia's DA layer is
permissionless: any agent can submit blobs, and any agent can read blobs from any namespace.

This means agent A can publish a trace of how it solved a blob submission problem, and agent
B --- run by a completely different operator, on a different machine, in a different country
--- can find that trace by querying the namespace and learn from it. No API keys, no access
control negotiations, no shared database credentials.

### 2.2 Append-only audit trail

Once a blob is included in a Celestia block, it cannot be altered or deleted (at least within
the data availability window). This makes the DA layer a natural audit log for agent actions.
Regulators, auditors, or other agents can verify exactly what an agent did by reading its
trace namespace.

### 2.3 Namespace-based data partitioning

Celestia's namespace system maps naturally to how agent data should be organized:

- Each agent gets its own namespace for traces and state.
- Shared learning data lives in a global namespace.
- Multi-agent coordination groups get their own coordination namespaces.
- Data types (traces, fingerprints, learning updates) are separated into distinct namespaces.

This is the same partitioning you would build with database tables or directory structures,
but it is built into the protocol. Clients can subscribe to specific namespaces and receive
only the data they care about.

### 2.4 Economically viable at agent scale

Agent traces are typically 10--100 KB per task execution. At Celestia's current pricing
($0.07--$0.81 per MB), storing a 50 KB trace costs between $0.0035 and $0.04. An agent that
executes 100 tasks per day would spend $0.35--$4.00 per day on DA storage. This is comparable
to cloud storage costs and well within the budget of any serious agent deployment.

### 2.5 Light node infrastructure

Celestia's light node infrastructure means agents do not need to run (or trust) full nodes.
An embedded light node can verify data availability through DAS, submit blobs, and read
namespace data --- all from within the agent process, with modest resource requirements.

### 2.6 What DA does NOT provide

It is equally important to understand what Celestia's DA layer does not provide:

| Not provided | Implication for tiagent |
|--------------|------------------------|
| Execution / smart contracts | tiagent handles all logic locally; Celestia is pure storage + ordering |
| Access control | Any namespace is readable by anyone; sensitive data must be encrypted before submission |
| Querying by content | You can query by namespace + height, not by content; tiagent maintains local indexes |
| Permanent storage | Light nodes prune after 7 days; archival nodes keep data longer, but cold backup is recommended |
| Sub-second latency | Block time is ~12 seconds; tiagent uses local cache for fast reads |

---

## 3. Namespace Design

tiagent organizes its data into four categories of namespaces. Each category uses a
structured naming scheme that fits within Celestia's 28-byte namespace ID.

### 3.1 Namespace encoding

Celestia namespace IDs are 28 bytes. tiagent uses the following encoding to pack structured
names into this space:

```
┌────────┬────────┬──────────┬───────────────────────┐
│ 6 bytes│ 1 byte │ 1 byte   │ 20 bytes              │
│ prefix │ version│ category │ identifier             │
│"tiagnt"│ 0x01   │ (enum)   │ (varies by category)   │
└────────┴────────┴──────────┴───────────────────────┘
         28 bytes total
```

| Field | Size | Description |
|-------|------|-------------|
| prefix | 6 bytes | Fixed string `tiagnt` (abbreviated to fit). Identifies blobs as tiagent data. |
| version | 1 byte | Protocol version. Currently `0x01`. Allows future schema changes. |
| category | 1 byte | Which category this namespace belongs to (system, agent, learn, trace). |
| identifier | 20 bytes | Category-specific identifier. For agent namespaces, this is a truncated hash of the agent ID. |

Category byte values:

| Category | Byte | Description |
|----------|------|-------------|
| System | `0x01` | Protocol-level metadata and agent registry |
| Agent | `0x02` | Per-agent signals and episodes |
| Learn | `0x03` | Shared learning artifacts |
| Trace | `0x04` | TraceCommons-compatible trace data |

### 3.2 System namespace: `tiagent/system`

The system namespace holds protocol-level metadata that is not specific to any single agent.

**Contents:**

| Data type | Description | Write frequency |
|-----------|-------------|-----------------|
| Agent registry entries | Agent ID, capabilities, public key, last-seen height | On agent startup and periodic heartbeat |
| Protocol version announcements | Schema version, migration instructions | On protocol upgrades |
| Namespace directory | Known namespaces and their purposes | Periodically aggregated |

**Namespace ID construction:**

```
prefix:   "tiagnt"  (6 bytes)
version:  0x01      (1 byte)
category: 0x01      (1 byte, System)
id:       "system" + 14 zero-bytes  (20 bytes, padded)
```

Agents read the system namespace on startup to discover other agents and check for protocol
updates. Writes to this namespace are infrequent (agent registration, periodic heartbeats).

### 3.3 Per-agent namespace: `tiagent/agent/{id}`

Each agent gets its own namespace for storing execution data. This isolates each agent's
data, making it easy to audit a specific agent's behavior or subscribe to its output.

**Contents:**

| Data type | Description | Write frequency |
|-----------|-------------|-----------------|
| Episode signals | Structured traces of agent executions (turns, tool calls, outcomes) | After each task completion |
| Gate result signals | Validation outcomes (compile, test, lint results) | After each gate check |
| State snapshots | Serialized agent state for resume-after-interruption | Periodically during long runs |

**Namespace ID construction:**

The 20-byte identifier is the first 20 bytes of the SHA-256 hash of the agent's full ID
string. This deterministic mapping means any party can compute an agent's namespace from its
ID without a lookup table.

```rust
fn agent_namespace(agent_id: &str) -> [u8; 28] {
    let mut ns = [0u8; 28];
    ns[0..6].copy_from_slice(b"tiagnt");
    ns[6] = 0x01; // version
    ns[7] = 0x02; // category: Agent
    let hash = sha256(agent_id.as_bytes());
    ns[8..28].copy_from_slice(&hash[..20]);
    ns
}
```

### 3.4 Shared learning namespace: `tiagent/learn`

The learning namespace holds artifacts that any agent can read to bootstrap or improve its
own behavior. This is the mechanism by which shared learning works.

**Contents:**

| Data type | Description | Write frequency |
|-----------|-------------|-----------------|
| Cascade router weight snapshots | Model selection weights learned from execution history | Periodically (e.g., every 50 tasks) |
| Playbook entries | Reusable strategies extracted from high-scoring episodes | When a new playbook is identified |
| Efficiency summaries | Aggregated cost/latency/token statistics | Periodically |
| HDC fingerprints | Hyperdimensional Computing vectors representing behavioral signatures | After each episode |

**Namespace ID construction:**

```
prefix:   "tiagnt"  (6 bytes)
version:  0x01      (1 byte)
category: 0x03      (1 byte, Learn)
id:       "global" + 14 zero-bytes  (20 bytes, padded)
```

A new agent joining the network can read the learning namespace to bootstrap its routing
weights and playbook library, rather than starting from scratch.

### 3.5 Trace namespace: `tiagent/trace`

The trace namespace stores data formatted for compatibility with TraceCommons, a system for
scoring trace quality and enabling trajectory retrieval-augmented generation (RAG) across
agents. See **07-tracecommons-integration.md** for the full TraceCommons design.

**Contents:**

| Data type | Description | Write frequency |
|-----------|-------------|-----------------|
| Scored traces | Execution traces annotated with quality scores (task completion, efficiency, safety) | After TraceCommons scoring completes |
| Trajectory embeddings | Vector embeddings of traces for similarity search | Alongside scored traces |
| Quality attestations | Third-party quality ratings of traces | When attestation is received |

**Namespace ID construction:**

```
prefix:   "tiagnt"  (6 bytes)
version:  0x01      (1 byte)
category: 0x04      (1 byte, Trace)
id:       "commons" + 13 zero-bytes  (20 bytes, padded)
```

### 3.6 Namespace summary

```
tiagent namespaces in a Celestia block:

    ┌─────────────────────────────────────────────────┐
    │                 Celestia Block N                  │
    │                                                   │
    │  ┌─────────────────┐  ┌─────────────────┐        │
    │  │ tiagent/system  │  │ tiagent/learn   │        │
    │  │                 │  │                 │        │
    │  │ - Registry      │  │ - Router wts    │        │
    │  │ - Proto version │  │ - Playbooks     │        │
    │  └─────────────────┘  │ - HDC prints    │        │
    │                       └─────────────────┘        │
    │  ┌─────────────────┐  ┌─────────────────┐        │
    │  │ tiagent/agent/  │  │ tiagent/agent/  │        │
    │  │   abc123        │  │   def456        │        │
    │  │                 │  │                 │        │
    │  │ - Episodes      │  │ - Episodes      │        │
    │  │ - Gate results  │  │ - Gate results  │        │
    │  └─────────────────┘  └─────────────────┘        │
    │                                                   │
    │  ┌─────────────────┐                              │
    │  │ tiagent/trace   │                              │
    │  │                 │                              │
    │  │ - Scored traces │                              │
    │  │ - Embeddings    │                              │
    │  └─────────────────┘                              │
    └─────────────────────────────────────────────────┘
```

---

## 4. Blob Schema

This section defines what goes into the blobs that tiagent submits to Celestia. Every blob
contains a serialized tiagent Signal (the universal data type described in
**02-architecture.md**), wrapped in an envelope that adds DA-specific metadata.

### 4.1 Blob envelope format

Every tiagent blob is a serialized `BlobEnvelope`:

```rust
/// Wraps a Signal for DA layer submission. Adds metadata needed
/// for retrieval, verification, and cross-agent discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobEnvelope {
    /// Protocol version. Used for forward/backward compatibility.
    /// Reader code checks this before attempting deserialization.
    pub version: u8,

    /// The Signal being stored. This is the actual agent data.
    pub signal: Signal,

    /// Content hash of the serialized signal (before envelope wrapping).
    /// Used for integrity verification: recompute and compare on read.
    pub content_hash: Hash,

    /// Agent ID of the submitter. Enables filtering by agent even
    /// within shared namespaces (like the learn namespace).
    pub agent_id: String,

    /// Wall-clock timestamp of submission (UTC, millisecond precision).
    /// Used for ordering when multiple blobs appear at the same height.
    pub submitted_at: u64,

    /// Optional HDC fingerprint of the signal's semantic content.
    /// Enables similarity search without deserializing the full payload.
    pub hdc_fingerprint: Option<Vec<u8>>,

    /// Optional tags for filtering. Example: ["gate:compile", "task:deploy"].
    pub tags: Vec<String>,
}
```

Serialization format: MessagePack (compact binary, schema-compatible with JSON, lower
overhead than JSON for repeated field names). The choice of MessagePack over JSON saves
roughly 30-50% on blob size, which directly reduces DA costs.

### 4.2 Signal types stored on DA

Not every Signal type is published to the DA layer. tiagent selects which Signal types
warrant shared, verifiable storage.

| Signal kind | Published to DA? | Namespace | Rationale |
|-------------|-----------------|-----------|-----------|
| `Episode` | Yes | `agent/{id}` | Core learning data. Other agents learn from episodes. |
| `GateResult` | Yes | `agent/{id}` | Proves that outputs were validated. Audit trail. |
| `RoutingUpdate` | Yes | `learn` | Shared routing intelligence. Network-wide benefit. |
| `Playbook` | Yes | `learn` | Reusable strategies. Cross-agent transfer. |
| `WorkProof` | Yes | `agent/{id}` | Verifiable proof of completed work. |
| `Coordination` | Yes | `agent/{id}` or group | Multi-agent workflow messages. |
| `Prompt` | No | (local only) | Too large, contains task-specific context with no reuse value. |
| `Response` | No | (local only) | Raw LLM output is large and not useful for sharing. |
| `ToolCall` | No | (local only) | Captured within episodes; individual calls are too granular. |
| `ToolResult` | No | (local only) | Same as ToolCall --- captured within episodes. |

### 4.3 Episode blob structure

The most common blob type is an Episode --- a structured trace of one agent execution. Here
is the payload structure:

```rust
/// A complete structured trace of one agent execution.
/// This is what gets serialized into the Signal's payload field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodePayload {
    /// Unique identifier for this episode.
    pub episode_id: String,

    /// The task that was being executed.
    pub task_description: String,

    /// Ordered list of agent turns (prompt/response pairs + tool calls).
    pub turns: Vec<Turn>,

    /// Which model was used (and which backend dispatched it).
    pub model: String,
    pub backend: String,

    /// Multi-dimensional score assigned by the gate pipeline.
    pub score: EpisodeScore,

    /// Total wall-clock duration in milliseconds.
    pub duration_ms: u64,

    /// Token usage (input + output).
    pub input_tokens: u64,
    pub output_tokens: u64,

    /// Estimated cost in USD.
    pub cost_usd: f64,

    /// HDC fingerprint of this episode's behavioral signature.
    /// Used for similarity search: "find episodes similar to this one."
    pub hdc_fingerprint: Vec<u8>,

    /// Gate results that validated this episode's output.
    pub gate_results: Vec<GateResultSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeScore {
    /// Did the task complete successfully? (0.0 to 1.0)
    pub completion: f64,
    /// How efficiently were resources used? (0.0 to 1.0)
    pub efficiency: f64,
    /// Were safety constraints respected? (0.0 to 1.0)
    pub safety: f64,
    /// Overall composite score.
    pub overall: f64,
}
```

A typical Episode blob is 20--80 KB after MessagePack serialization.

### 4.4 Learning update blob structure

Learning updates capture changes to the cascade router's model selection weights:

```rust
/// Snapshot of cascade router weights for cross-agent sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingUpdatePayload {
    /// Map of task_category -> model -> performance_score.
    /// Example: { "code_generation": { "claude-sonnet-4": 0.87, "gpt-4o": 0.72 } }
    pub weights: HashMap<String, HashMap<String, f64>>,

    /// How many episodes contributed to these weights.
    pub episode_count: u64,

    /// Timestamp range of the episodes that contributed.
    pub from_timestamp: u64,
    pub to_timestamp: u64,
}
```

### 4.5 Gate result blob structure

Gate results prove that an agent's output was validated:

```rust
/// Summary of a gate pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResultPayload {
    /// Which gate ran (e.g., "compile", "test", "lint", "diff").
    pub gate_name: String,

    /// Did the gate pass?
    pub passed: bool,

    /// Human-readable summary of the result.
    pub summary: String,

    /// Duration of the gate check in milliseconds.
    pub duration_ms: u64,

    /// Hash of the Episode this gate checked.
    pub episode_hash: Hash,
}
```

---

## 5. CelestiaSubstrate Implementation

The `CelestiaSubstrate` is the concrete implementation of tiagent's `Substrate` trait that
reads and writes Signals through Celestia's DA layer. It lives in the `tiagent-celestia`
crate.

### 5.1 The Substrate trait (recap)

The `Substrate` trait is defined in `tiagent-core`. It abstracts over storage backends:

```rust
/// Persist and retrieve Signals. Implementations include local
/// filesystem, Celestia DA, SQLite, and hybrid (local + DA).
#[async_trait]
pub trait Substrate: Send + Sync {
    /// Write a Signal to storage. Returns a reference that can be
    /// used to retrieve it later.
    async fn write(&self, signal: &Signal) -> Result<StorageRef>;

    /// Read a Signal by its content hash.
    async fn read(&self, hash: &Hash) -> Result<Option<Signal>>;

    /// Query Signals matching a filter (by kind, time range, metadata).
    async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>>;

    /// Check whether a Signal exists without reading its full content.
    async fn exists(&self, hash: &Hash) -> Result<bool>;
}
```

### 5.2 CelestiaSubstrate struct

```rust
use celestia_rpc::Client as CelestiaRpcClient;
use celestia_types::{Blob, nmt::Namespace as CelestiaNamespace};
use lru::LruCache;

/// Substrate implementation backed by Celestia's DA layer.
///
/// Writes: serializes Signals into BlobEnvelopes, submits to the
/// appropriate namespace via Celestia's node API.
///
/// Reads: queries the DA layer by namespace + height, deserializes
/// BlobEnvelopes back into Signals. Maintains an LRU cache to avoid
/// redundant network calls.
pub struct CelestiaSubstrate {
    /// RPC client for communicating with a Celestia node.
    /// This can be a connection to an external node or to an
    /// embedded light node (see Section 6).
    rpc: CelestiaRpcClient,

    /// Namespace manager that handles namespace encoding,
    /// lookup, and caching.
    namespaces: NamespaceManager,

    /// This agent's ID, used to construct per-agent namespaces
    /// and to tag blobs with their submitter.
    agent_id: String,

    /// Gas price for blob submissions (in utia, Celestia's
    /// smallest denomination). Higher gas price = faster inclusion.
    gas_price: f64,

    /// LRU cache of recently accessed Signals, keyed by content hash.
    /// Avoids redundant RPC calls for frequently accessed data.
    cache: LruCache<Hash, Signal>,

    /// Index mapping content hashes to DA references (height + commitment).
    /// Enables read-by-hash without scanning all namespaces.
    hash_index: HashMap<Hash, DaRef>,
}
```

### 5.3 Write path

When the universal loop reaches the **write** stage, it calls `substrate.write(signal)`.
For the `CelestiaSubstrate`, this triggers the following sequence:

```
Signal
  │
  ├─1─► Determine target namespace (from signal.namespace or default)
  │
  ├─2─► Serialize Signal into BlobEnvelope (MessagePack)
  │
  ├─3─► Construct celestia_types::Blob with namespace + data
  │
  ├─4─► Submit blob via rpc.blob_submit([blob], gas_price)
  │
  ├─5─► Receive submission result: height + commitment
  │
  ├─6─► Store DaRef in local hash_index (hash → height + commitment)
  │
  ├─7─► Insert Signal into LRU cache
  │
  └─8─► Return StorageRef { da_ref, content_hash }
```

In Rust pseudocode:

```rust
#[async_trait]
impl Substrate for CelestiaSubstrate {
    async fn write(&self, signal: &Signal) -> Result<StorageRef> {
        // 1. Determine target namespace
        let ns = match &signal.namespace {
            Some(ns) => self.namespaces.to_celestia_namespace(ns)?,
            None => self.namespaces.default_agent_namespace(&self.agent_id)?,
        };

        // 2. Build the blob envelope
        let envelope = BlobEnvelope {
            version: 1,
            signal: signal.clone(),
            content_hash: signal.id.clone(),
            agent_id: self.agent_id.clone(),
            submitted_at: now_millis(),
            hdc_fingerprint: signal.metadata.get("hdc_fingerprint")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            tags: extract_tags(signal),
        };

        // 3. Serialize to bytes
        let data = rmp_serde::to_vec(&envelope)?;

        // 4. Construct Celestia blob
        let blob = Blob::new(ns, data)?;

        // 5. Submit to DA layer
        let height = self.rpc.blob_submit(&[blob], self.gas_price).await?;

        // 6. Record the DA reference
        let da_ref = DaRef { height, namespace: ns, commitment: blob.commitment };
        self.hash_index.insert(signal.id.clone(), da_ref.clone());

        // 7. Cache locally
        self.cache.put(signal.id.clone(), signal.clone());

        Ok(StorageRef::Da(da_ref))
    }
}
```

### 5.4 Read path

Reading a Signal by hash first checks the local cache, then falls back to DA:

```
Hash
  │
  ├─1─► Check LRU cache → if hit, return Signal
  │
  ├─2─► Look up DaRef in hash_index → if missing, return None
  │
  ├─3─► Fetch blob via rpc.blob_get(height, namespace, commitment)
  │
  ├─4─► Deserialize BlobEnvelope (MessagePack)
  │
  ├─5─► Verify content_hash matches (integrity check)
  │
  ├─6─► Insert into LRU cache
  │
  └─7─► Return Signal
```

### 5.5 Query path

Querying by filter scans a namespace over a height range:

```rust
async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>> {
    let ns = filter.namespace
        .as_ref()
        .map(|n| self.namespaces.to_celestia_namespace(n))
        .transpose()?
        .unwrap_or_else(|| self.namespaces.default_agent_namespace(&self.agent_id).unwrap());

    let mut results = Vec::new();

    // Scan the height range
    for height in filter.from_height..=filter.to_height {
        let blobs = self.rpc.blob_get_all(height, ns).await?;

        for blob in blobs {
            let envelope: BlobEnvelope = rmp_serde::from_slice(&blob.data)?;

            // Apply filter predicates
            if filter.matches(&envelope.signal) {
                results.push(envelope.signal);
            }
        }
    }

    results
}
```

### 5.6 Namespace manager

The `NamespaceManager` handles encoding tiagent's logical namespace names into Celestia's
28-byte namespace ID format:

```rust
pub struct NamespaceManager {
    /// Cache of encoded namespaces to avoid recomputing.
    cache: HashMap<String, CelestiaNamespace>,
}

impl NamespaceManager {
    /// Encode a tiagent namespace into a Celestia namespace ID.
    pub fn to_celestia_namespace(&self, ns: &Namespace) -> Result<CelestiaNamespace> {
        let mut id = [0u8; 28];
        id[0..6].copy_from_slice(b"tiagnt");
        id[6] = ns.version;
        id[7] = ns.category as u8;

        // Remaining 20 bytes: hash of the namespace identifier
        let hash = sha256(ns.identifier.as_bytes());
        id[8..28].copy_from_slice(&hash[..20]);

        CelestiaNamespace::new_v0(&id)
    }

    /// Compute the default namespace for an agent's own data.
    pub fn default_agent_namespace(&self, agent_id: &str) -> Result<CelestiaNamespace> {
        self.to_celestia_namespace(&Namespace {
            version: 1,
            category: NamespaceCategory::Agent,
            identifier: agent_id.to_string(),
        })
    }
}
```

---

## 6. Light Node Embedding

tiagent can optionally embed a Celestia light node directly in the agent process. This
eliminates the dependency on an external Celestia node for DA access.

### 6.1 What is a light node?

A Celestia light node is a minimal node that participates in the network without storing the
full blockchain. It performs Data Availability Sampling (DAS) to verify that block data is
available, syncs block headers, and can submit and retrieve blobs. Light nodes are much
cheaper to run than full nodes:

| Resource | Full node | Light node |
|----------|-----------|------------|
| Storage | Hundreds of GB (full chain) | ~1 GB (headers + recent samples) |
| CPU | Moderate (block validation) | Low (header sync + sampling) |
| Bandwidth | High (full blocks) | Low (sampled cells + headers) |
| Startup time | Hours (chain sync) | Minutes (header sync) |

### 6.2 lumina-node: Rust light node implementation

`lumina-node` is the production-grade Rust implementation of a Celestia light node. It is
maintained by the Celestia team and provides:

- Header syncing with the Celestia network
- Data Availability Sampling (DAS) for block verification
- Blob submission and retrieval via the node's local API
- Namespace-scoped blob queries
- Both in-process (library) and standalone (binary) modes

tiagent uses lumina-node in **library mode**: the light node runs as an async task within
the agent's tokio runtime, sharing the same process. No separate process, no IPC, no
external node to manage.

### 6.3 Embedded vs. external node

tiagent supports two modes of Celestia connectivity:

```
Mode 1: External node (default)

    ┌─────────────┐         ┌──────────────────┐
    │  tiagent    │  RPC    │  External         │
    │  agent      │────────►│  Celestia node    │
    │  process    │  :26658 │  (light or full)  │
    └─────────────┘         └──────────────────┘

Mode 2: Embedded light node (feature-gated)

    ┌──────────────────────────────────────┐
    │  tiagent agent process               │
    │                                      │
    │  ┌────────────┐  ┌────────────────┐  │
    │  │ Agent      │  │ lumina-node    │  │
    │  │ runtime    │──│ (embedded)     │  │
    │  │            │  │                │  │
    │  │ Universal  │  │ Header sync,   │  │
    │  │ loop, tools│  │ DAS, blob      │  │
    │  │            │  │ submit/read    │  │
    │  └────────────┘  └────────────────┘  │
    └──────────────────────────────────────┘
```

**When to use external node:**
- Development and testing (connect to Mocha testnet node)
- Lightweight deployments where binary size matters
- When a shared Celestia node is already available in the infrastructure

**When to use embedded light node:**
- Production deployments that need DA verification without trusting external nodes
- Fully self-contained agent deployments
- Scenarios requiring offline resilience (the light node maintains local state)

### 6.4 Configuration

```toml
# tiagent.toml

[celestia]
# Mode: "rpc" (external node) or "light" (embedded lumina-node)
mode = "rpc"

# External node connection (used when mode = "rpc")
rpc_url = "http://localhost:26658"
auth_token = "eyJ..."

# Light node configuration (used when mode = "light")
[celestia.light_node]
# Celestia network to connect to
network = "mocha"         # "mocha", "arabica", or "mainnet"
# Local directory for light node state (headers, samples)
store_path = ".tiagent/celestia/"
# Bootstrap peers for initial sync (optional, defaults to network's bootstrap list)
bootstrap_peers = []

[celestia.submission]
# Gas price in utia (Celestia's smallest denomination)
gas_price = 0.002
# Maximum fee willing to pay per blob submission (in TIA)
max_fee = 0.1
```

### 6.5 Feature flag

The embedded light node is behind the `light-node` Cargo feature flag because `lumina-node`
is a substantial dependency that increases binary size by approximately 15--20 MB. Developers
who do not need embedded DA verification can omit it:

```toml
# Cargo.toml for tiagent-celestia

[dependencies]
celestia-types = "1.0"
celestia-rpc = "1.0"

[dependencies.lumina-node]
version = "0.6"
optional = true

[features]
default = ["celestia-rpc"]
light-node = ["lumina-node"]
```

---

## 7. Tiered Storage Strategy

Not all data should live on the DA layer. tiagent uses a three-tier storage model that
balances access speed, cost, verification guarantees, and retention duration.

**The Hot tier (local filesystem) is the default and works standalone.** A tiagent instance
with no Celestia configuration uses only the Hot tier and is fully functional --- all
self-improvement, plan execution, and learning features work locally. The Warm (Celestia)
and Cold (archival) tiers are optional add-ons that enable cross-agent sharing and
long-term verifiable storage. Enable them when you want your agent's learning to benefit
other agents in the network.

### 7.1 The three tiers

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   HOT TIER: Local Filesystem  ★ DEFAULT — works standalone      │
│   ─────────────────────────                                     │
│   Format:   JSONL files in .tiagent/ (or SQLite)                │
│   Latency:  < 1 ms                                              │
│   Cost:     Disk space only                                     │
│   Durability: Single machine (lost if disk fails)               │
│   Retention: Configurable (default: 30 days, then GC)           │
│   Visibility: Local only (this agent)                           │
│                                                                 │
│   What lives here:                                              │
│   - ALL signals during execution (everything starts local)      │
│   - ALL self-improvement state (routing, gates, playbooks,      │
│     episodes, efficiency) — the full learning loop              │
│   - Raw LLM prompts and responses                               │
│   - Intermediate tool call results                              │
│   - Secrets and credentials (encrypted at rest)                 │
│   - Local indexes (hash → DA reference, namespace lookups)      │
│   - LRU cache of recently fetched DA blobs                      │
│                                                                 │
│   This tier alone gives you a fully self-improving agent.       │
│   Everything below is optional and adds cross-agent sharing.    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   WARM TIER: Celestia DA Layer  (optional)                      │
│   ────────────────────────────                                  │
│   Format:   MessagePack blobs in organized namespaces           │
│   Latency:  1-10 seconds (write: wait for block inclusion)      │
│             100-500 ms (read: RPC or light node query)          │
│   Cost:     ~$0.07-$0.81 per MB                                 │
│   Durability: Replicated across Celestia's validator set        │
│   Retention: 7 days (light nodes), indefinite (archival nodes)  │
│   Visibility: Global (any agent can read any namespace)         │
│                                                                 │
│   What lives here:                                              │
│   - Completed episodes (after gate validation)                  │
│   - Gate results (validation proofs)                            │
│   - Learning artifacts (routing weights, playbooks)             │
│   - HDC fingerprints (behavioral signatures)                    │
│   - Work proofs (verifiable task completion)                    │
│   - Coordination messages (multi-agent workflows)               │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   COLD TIER: Permanent Archival  (optional, future)             │
│   ──────────────────────────────────────                        │
│   Format:   Platform-specific (Arweave, Filecoin, IPFS)         │
│   Latency:  Seconds to minutes (write), seconds (read via CID) │
│   Cost:     Varies ($1-5/GB for permanent storage)              │
│   Durability: Permanent (by design)                             │
│   Retention: Indefinite                                         │
│   Visibility: Global (content-addressed retrieval)              │
│                                                                 │
│   What lives here:                                              │
│   - High-value episodes approaching the 7-day pruning window    │
│   - Historical learning state snapshots                         │
│   - Compliance-required audit trails                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 Promotion rules (hot to warm)

Signals are promoted from local storage to the DA layer based on configurable rules:

| Signal kind | Promotion trigger | Condition |
|-------------|-------------------|-----------|
| Episode | Automatic, post-gate | Only if gate pipeline passes (no point publishing failed traces) |
| GateResult | Automatic, with episode | Published alongside the episode it validated |
| RoutingUpdate | Periodic | Every N episodes (configurable, default: 50) |
| Playbook | On extraction | When a new playbook is identified from episode analysis |
| Coordination | Immediate | Published as soon as generated (latency-sensitive) |
| HDC fingerprint | With episode | Published alongside the episode, in the learn namespace |

### 7.3 Demotion rules (warm to cold)

Celestia light nodes prune data after 7 days. Data that must be retained beyond this window
needs to be archived to cold storage before pruning occurs. The archival process:

```
Day 1-5:  Signal lives on DA layer. Readable by all agents.
Day 5:    Archival daemon checks signal's retention policy.
          If policy = "permanent", copies blob to cold storage.
Day 7:    Light nodes prune the blob. Archival nodes may still have it.
Day 7+:   Signal readable from cold storage (if archived) or archival
          nodes (if available). Not guaranteed from light nodes.
```

### 7.4 The HybridSubstrate

The `HybridSubstrate` in `tiagent-store` combines local and DA storage:

```rust
/// Combines local filesystem storage with Celestia DA storage.
/// Writes go to local first (always), then to DA (if promotion
/// rules say so). Reads check local cache first, then DA.
pub struct HybridSubstrate {
    local: FileSubstrate,
    da: CelestiaSubstrate,
    promotion_rules: PromotionConfig,
}

#[async_trait]
impl Substrate for HybridSubstrate {
    async fn write(&self, signal: &Signal) -> Result<StorageRef> {
        // Always write locally first (fast, guaranteed)
        let local_ref = self.local.write(signal).await?;

        // Check if this signal should be promoted to DA
        if self.promotion_rules.should_promote(signal) {
            let da_ref = self.da.write(signal).await?;
            return Ok(StorageRef::Hybrid { local: local_ref, da: Some(da_ref) });
        }

        Ok(StorageRef::Hybrid { local: local_ref, da: None })
    }

    async fn read(&self, hash: &Hash) -> Result<Option<Signal>> {
        // Try local first (fast)
        if let Some(signal) = self.local.read(hash).await? {
            return Ok(Some(signal));
        }

        // Fall back to DA (slower, but shared)
        self.da.read(hash).await
    }
}
```

---

## 8. Proof Verification

Celestia provides cryptographic proofs that a blob was included in a specific block. tiagent
uses these proofs for audit trails, dispute resolution, and trust verification.

### 8.1 NMT inclusion proofs

When you retrieve a blob from Celestia, you can also request a **Namespaced Merkle Tree
(NMT) inclusion proof**. This proof is a set of sibling hashes in the Merkle tree that, when
combined with the blob's hash, reproduce the block's data root. Anyone with the block header
(which contains the data root) can verify the proof.

```
Verification flow:

    ┌─────────────────────────────────────────────────┐
    │  Block Header (from header sync)                 │
    │  ┌───────────────────────────────────────┐       │
    │  │ data_root: 0x7a3f...                  │       │
    │  └───────────────────────────────────────┘       │
    └─────────────────────────────────────────────────┘
                       │
                       │ compare
                       ▼
    ┌─────────────────────────────────────────────────┐
    │  NMT Proof                                       │
    │                                                  │
    │  blob_hash ──┐                                   │
    │              ├── intermediate_hash ──┐            │
    │  sibling_1 ──┘                      │            │
    │                                     ├── root     │
    │  sibling_2 ─────────────────────────┘            │
    │                                                  │
    │  If computed root == data_root → blob is in block│
    └─────────────────────────────────────────────────┘
```

### 8.2 Verification in Rust

```rust
use nmt_rs::{NamespacedMerkleTree, NamespaceProof};
use celestia_types::nmt::Namespace;

/// Verify that a blob was included in a Celestia block.
///
/// Arguments:
///   data_root   - from the block header (obtained via header sync)
///   proof       - NMT inclusion proof (obtained via blob.GetProof RPC)
///   blob_data   - the blob's raw bytes
///   namespace   - the blob's namespace
///
/// Returns true if the proof is valid (blob was in the block).
pub fn verify_blob_inclusion(
    data_root: &[u8; 32],
    proof: &NamespaceProof,
    blob_data: &[u8],
    namespace: &Namespace,
) -> bool {
    proof.verify_inclusion(data_root, blob_data, namespace)
}
```

### 8.3 When tiagent uses proofs

| Scenario | What is proved | Why |
|----------|---------------|-----|
| Audit request | Agent's episode blob was included at height N | Regulatory compliance, dispute resolution |
| Cross-agent learning | Retrieved learning artifact is authentic | Prevent poisoned training data |
| Coordination | Coordination message was published before a deadline | Time-ordering guarantees in multi-agent workflows |
| Work proof | Task completion was recorded on-chain | Verifiable work for marketplace/payment systems |

### 8.4 Proof storage and caching

Proofs are not stored on the DA layer (that would be circular). They are stored locally and
can be regenerated on demand by re-requesting the proof from a Celestia node:

```rust
/// Cached proof for a previously verified blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProof {
    /// DA reference to the blob this proof covers.
    pub da_ref: DaRef,
    /// The NMT inclusion proof bytes.
    pub proof_bytes: Vec<u8>,
    /// Block header data root at the time of verification.
    pub data_root: [u8; 32],
    /// When this proof was last verified.
    pub verified_at: u64,
}
```

---

## 9. Cost Model

This section provides concrete cost estimates for different agent workloads at current
Celestia pricing.

### 9.1 Per-blob cost formula

```
blob_cost = blob_size_bytes * gas_per_byte * gas_price

Where:
  gas_per_byte ≈ 8 gas/byte (Celestia's gas schedule)
  gas_price    ≈ 0.002 utia/gas (current Mocha testnet, variable on mainnet)
  1 TIA        = 1,000,000 utia
```

At current mainnet rates, blob costs roughly translate to:

| Blob size | Approximate cost (mainnet) | Approximate cost (Mocha testnet) |
|-----------|---------------------------|----------------------------------|
| 10 KB | $0.001 - $0.008 | $0.0007 |
| 50 KB | $0.004 - $0.04 | $0.004 |
| 100 KB | $0.007 - $0.08 | $0.007 |
| 500 KB | $0.035 - $0.40 | $0.035 |
| 1 MB | $0.07 - $0.81 | $0.07 |

### 9.2 Cost by signal type

| Signal type | Typical size | Cost per signal (mainnet) |
|-------------|-------------|--------------------------|
| Episode (small task, 3-5 turns) | 15-30 KB | $0.001 - $0.02 |
| Episode (large task, 10-20 turns) | 50-100 KB | $0.004 - $0.08 |
| Gate result | 1-3 KB | < $0.001 |
| Routing update | 5-15 KB | $0.001 - $0.01 |
| Playbook | 3-10 KB | < $0.01 |
| HDC fingerprint | 1-4 KB | < $0.001 |
| Coordination message | 2-8 KB | < $0.01 |

### 9.3 Workload cost projections

| Workload profile | Tasks/day | Blobs/day | Estimated daily DA cost |
|------------------|-----------|-----------|------------------------|
| Light (personal dev agent) | 10 | ~15 | $0.01 - $0.15 |
| Medium (team CI/CD agent) | 50 | ~75 | $0.08 - $0.75 |
| Heavy (production orchestrator) | 200 | ~300 | $0.30 - $3.00 |
| Extreme (multi-agent swarm, 10 agents) | 2,000 | ~3,000 | $3.00 - $30.00 |

These costs are comparable to cloud logging services (Datadog, CloudWatch) for similar data
volumes, with the added benefit of decentralized, verifiable, permissionless access.

### 9.4 Cost optimization strategies

| Strategy | Mechanism | Savings |
|----------|-----------|---------|
| Selective promotion | Only publish episodes that pass gates (skip failed runs) | 30-50% fewer blobs |
| Batch submission | Aggregate multiple small signals into one blob | Reduced per-blob overhead |
| Compression | gzip blob data before submission | 40-60% size reduction |
| Score threshold | Only publish episodes above a quality score threshold | Variable, depends on threshold |
| Learning dedup | Publish routing updates only when weights change significantly | 80-90% fewer routing blobs |

### 9.5 Fee estimation API

Before submitting a blob, tiagent estimates the cost to avoid surprises:

```rust
pub struct FeeEstimator {
    rpc: CelestiaRpcClient,
}

impl FeeEstimator {
    /// Estimate the cost of submitting a blob of the given size.
    /// Returns the estimated fee in TIA and the current gas price.
    pub async fn estimate(&self, blob_size_bytes: usize) -> Result<FeeEstimate> {
        let gas_per_byte = 8u64;
        let total_gas = blob_size_bytes as u64 * gas_per_byte;
        let min_gas_price = self.rpc.state_min_gas_price().await?;

        Ok(FeeEstimate {
            blob_size_bytes,
            total_gas,
            gas_price: min_gas_price,
            fee_utia: total_gas as f64 * min_gas_price,
            fee_tia: (total_gas as f64 * min_gas_price) / 1_000_000.0,
        })
    }
}

#[derive(Debug)]
pub struct FeeEstimate {
    pub blob_size_bytes: usize,
    pub total_gas: u64,
    pub gas_price: f64,
    pub fee_utia: f64,
    pub fee_tia: f64,
}
```

---

## 10. Mocha Testnet Setup

Mocha is Celestia's long-running testnet. It mirrors mainnet's behavior and is the
recommended environment for developing and testing tiagent's DA integration.

### 10.1 Option A: Connect to a public Mocha node

The simplest setup. No local node required.

**Step 1: Get a Mocha testnet auth token.**

Celestia nodes require authentication. For development, you can run a local light node
(see Option B) or use a public RPC provider.

**Step 2: Get testnet TIA.**

Mocha testnet TIA is free. Request tokens from the Celestia faucet:
`https://faucet.celestia-mocha.com/`

**Step 3: Configure tiagent.**

```toml
# tiagent.toml

[celestia]
mode = "rpc"
rpc_url = "http://consensus-mocha.celestia.org:26658"
auth_token = "<your-auth-token>"

[celestia.submission]
gas_price = 0.002
max_fee = 1.0  # generous on testnet
```

### 10.2 Option B: Run a local Mocha light node

Running your own light node gives you direct DA verification and does not depend on a third
party.

**Step 1: Install celestia-node.**

```bash
# Clone and build celestia-node (requires Go 1.21+)
git clone https://github.com/celestiaorg/celestia-node.git
cd celestia-node
make build
make install
```

**Step 2: Initialize the light node for Mocha.**

```bash
celestia light init --p2p.network mocha
```

This creates a node store at `~/.celestia-light-mocha-4/` with a generated key pair.

**Step 3: Fund the light node's account.**

```bash
# Show the node's account address
celestia light auth admin --p2p.network mocha

# Use the Mocha faucet to send testnet TIA to this address
```

**Step 4: Start the light node.**

```bash
celestia light start \
    --core.ip consensus-mocha.celestia.org \
    --p2p.network mocha
```

The node will sync headers and begin Data Availability Sampling. Sync typically completes
in 2-5 minutes.

**Step 5: Get the auth token.**

```bash
# The auth token is printed during `celestia light start`
# or can be retrieved with:
celestia light auth admin --p2p.network mocha
```

**Step 6: Configure tiagent to use the local node.**

```toml
# tiagent.toml

[celestia]
mode = "rpc"
rpc_url = "http://localhost:26658"
auth_token = "<auth-token-from-step-5>"

[celestia.submission]
gas_price = 0.002
max_fee = 1.0
```

### 10.3 Option C: Embedded light node (no external process)

When the `light-node` feature is enabled, tiagent embeds a Celestia light node directly in
the agent process using `lumina-node`. No separate `celestia-node` binary is needed.

**Step 1: Build tiagent with the light-node feature.**

```bash
cargo build -p tiagent-cli --features light-node
```

**Step 2: Configure the embedded node.**

```toml
# tiagent.toml

[celestia]
mode = "light"

[celestia.light_node]
network = "mocha"
store_path = ".tiagent/celestia/"
# Bootstrap peers are optional; defaults to Mocha's bootstrap list

[celestia.submission]
gas_price = 0.002
max_fee = 1.0
```

**Step 3: Run tiagent normally.** The light node starts automatically and syncs in the
background.

```bash
tiagent run "submit a test blob to my namespace"
```

### 10.4 Verifying the integration

Once connected to Mocha (via any option above), verify the integration:

```bash
# Check Celestia node connectivity
tiagent doctor

# Submit a test blob
tiagent run "submit a hello-world blob to my agent namespace"

# Verify the blob was stored
tiagent run "retrieve the most recent blob from my agent namespace"
```

### 10.5 Node API reference

The Celestia node exposes a JSON-RPC 2.0 API on port 26658. tiagent uses the following
endpoints through the `celestia-rpc` crate:

| Endpoint | Description | tiagent usage |
|----------|-------------|---------------|
| `blob.Submit(blobs, gas_price)` | Submit one or more blobs to the DA layer | Write path (Section 5.3) |
| `blob.Get(height, namespace, commitment)` | Retrieve a specific blob | Read path (Section 5.4) |
| `blob.GetAll(height, namespace)` | Retrieve all blobs in a namespace at a height | Query path (Section 5.5) |
| `blob.GetProof(height, namespace, commitment)` | Get an NMT inclusion proof for a blob | Proof verification (Section 8) |
| `header.GetByHeight(height)` | Get a block header (contains data root) | Proof verification (Section 8) |
| `state.Balance()` | Get the node's account balance | Fee estimation, cost tracking |

All RPC calls are authenticated via the `auth_token` provided in configuration.

### 10.6 Network comparison

| Property | Mocha (testnet) | Arabica (devnet) | Mainnet Beta |
|----------|----------------|-------------------|--------------|
| Purpose | Stable testing | Rapid iteration | Production |
| Block time | ~12s | ~12s | ~12s |
| Max block size | 128 MB | 128 MB | 128 MB |
| Token | Test TIA (free) | Test TIA (free) | Real TIA |
| Stability | High | Lower (resets possible) | Highest |
| Recommended for | Development, integration testing | Experimental features | Production agents |

---

## Appendix A: Celestia RPC Authentication

Celestia nodes use JWT-based authentication. The auth token is a JWT signed with the node's
key and includes a permission level:

| Level | Permissions |
|-------|------------|
| `public` | Read-only: headers, blob retrieval |
| `read` | Read + namespace queries |
| `write` | Read + write: blob submission |
| `admin` | Full access: all operations + node management |

tiagent requires at minimum `write` permission for blob submission and `read` permission for
retrieval. For development, `admin` is simplest.

```bash
# Generate a token with write permissions
celestia light auth write --p2p.network mocha
```

---

## Appendix B: Blob Size Limits and Batching

Celestia imposes a maximum blob size based on the current block size parameters. After the
Matcha upgrade, the maximum single blob size is limited by the square size of the block's
data square.

For practical purposes:

- Individual blobs up to ~2 MB are reliably accepted.
- Blobs larger than 2 MB should be split into multiple blobs with a sequence number in their
  envelope metadata.
- tiagent's typical blobs (episodes, learning artifacts) are 10-100 KB --- well within limits.

When multiple small signals need to be published in the same block, tiagent can batch them
into a single `blob.Submit` call. The RPC accepts an array of blobs, and all blobs in one
call are included in the same block (if there is space):

```rust
// Batch submission: multiple signals in one RPC call
let blobs: Vec<Blob> = signals.iter()
    .map(|s| build_blob(s, &namespace))
    .collect::<Result<Vec<_>>>()?;

// All blobs submitted atomically in one call
let height = rpc.blob_submit(&blobs, gas_price).await?;
```

This reduces the per-blob overhead and ensures related signals (e.g., an episode and its
gate results) land in the same block.
