# tiagent: DA Storage Patterns for Vector Stores, HDC Fingerprints, and Shared State

> **This document describes optional network-mode patterns.** tiagent works as a fully
> functional, self-improving coding agent using only local storage. All vector stores, HDC
> fingerprints, routing weights, and learning artifacts work locally without Celestia. This
> document describes what you gain by enabling the optional Celestia integration: the ability
> to share learning across agent instances so every agent benefits from every other agent's
> experience. If you are using tiagent as a standalone coding agent, you can skip this
> document entirely.

This document explains how tiagent uses Celestia's data availability (DA) layer for advanced
agent state --- vector embeddings, Hyperdimensional Computing (HDC) fingerprints, model routing
weights, and shared learning artifacts. It covers what belongs on DA, what stays local, and the
concrete patterns that bridge the two.

If you have not read the preceding documents:

- **01-vision-and-overview.md** explains what tiagent is: a Rust toolkit for building
  self-improving AI agents on the Celestia blockchain ecosystem.
- **02-architecture.md** explains the core abstractions: one noun (Signal), six verb traits
  (Substrate, Scorer, Gate, Router, Composer, Policy), and a universal loop
  (query, score, route, compose, act, verify, write, react).
- **03-crate-structure.md** explains the workspace layout, including `tiagent-celestia`
  (DA integration) and `tiagent-store` (local storage).
- **04-celestia-integration.md** explains Celestia primitives (blobs, namespaces, light nodes),
  the tiered storage strategy, and the `CelestiaSubstrate` implementation.

This document assumes no prior knowledge of vector stores, HDC, or distributed state management.
Every concept is explained from first principles.

---

## Table of Contents

1. [The Storage Challenge](#1-the-storage-challenge)
2. [What Goes Where](#2-what-goes-where)
3. [HDC Fingerprints on DA](#3-hdc-fingerprints-on-da)
4. [Vector Store Commitment Pattern](#4-vector-store-commitment-pattern)
5. [Shared Learning via DA](#5-shared-learning-via-da)
6. [Trace Publishing for TraceCommons](#6-trace-publishing-for-tracecommons)
7. [Snapshot and Recovery](#7-snapshot-and-recovery)
8. [Cost Optimization](#8-cost-optimization)

---

## 1. The Storage Challenge

A single tiagent instance stores all of the following locally and uses it for
self-improvement without any external dependencies. Local storage handles everything for
a standalone developer. The challenge this document addresses is not "how to store agent
state" --- that is already solved by local files --- but rather **how to share learning
across agent instances**. When multiple agents are running (different developers, different
machines, different organizations), each one independently discovers what works. DA storage
solves the sharing problem: it gives agents a common substrate to publish and consume each
other's learning, so improvements compound across the network instead of staying siloed.

An agent that learns from experience needs to store several kinds of state:

- **Vector embeddings** --- numerical representations of text, code, or behavior that enable
  similarity search ("find episodes similar to this one"). A single embedding is a list of
  384 to 1536 floating-point numbers.
- **HDC fingerprints** --- compact binary vectors that capture behavioral signatures. Used for
  fast similarity comparisons, episode deduplication, and agent clustering.
- **Model routing weights** --- learned preferences for which LLM to use for which kind of task.
  These improve over time as the agent observes which models perform best.
- **Episode traces** --- structured logs of what the agent did, what tools it called, and what
  the outcomes were.
- **Gate results** --- records of whether the agent's output passed validation (compilation,
  tests, linting, diff review).

Each data type has different requirements:

| Requirement | Vector embeddings | HDC fingerprints | Routing weights | Episode traces | Gate results |
|-------------|-------------------|------------------|-----------------|----------------|--------------|
| **Persistence** | Durable (months) | Durable (months) | Durable (weeks) | Durable (weeks) | Durable (weeks) |
| **Queryability** | Similarity search (k-NN) | Hamming distance | Key-value lookup | Filter by time/task | Filter by gate/pass |
| **Shareability** | Optional | High value | High value | High value | High value |
| **Verifiability** | Low priority | Useful | Useful | Critical | Critical |
| **Size per item** | 1.5--6 KB | 128 bytes | ~2 KB snapshot | 20--80 KB | 1--5 KB |
| **Write frequency** | Per episode | Per episode | Every 50 tasks | Per task | Per task |

Celestia's DA layer provides shareability and verifiability, but it has constraints that make
it unsuitable for some of these workloads:

| DA constraint | Implication |
|---------------|-------------|
| **Append-only** | You cannot update a blob. Every write creates a new blob. State that changes frequently (like a vector index) cannot live on DA as a mutable structure. |
| **No random access** | You query by namespace + block height, not by content. There is no "SELECT * WHERE embedding NEAR [0.1, 0.3, ...]" on DA. |
| **7-day pruning** | Light nodes discard blobs older than 7 days. Data that must survive longer needs archival or cold backup. |
| **~12-second latency** | Writing requires waiting for block inclusion. Real-time state updates are impractical. |
| **Cost per byte** | At $0.07--$0.81 per MB, storing gigabytes of raw embeddings is not economical. |

The solution is a **dual-layer architecture**: local storage handles queryable, mutable,
latency-sensitive state; DA handles verifiable, shareable, append-only proofs and coordination.
The bridge between them is **commitments** --- compact proofs published to DA that attest to
the state of the local store.

### Standalone vs Network Mode

tiagent operates in one of two modes. Standalone is the default; Network mode is opt-in via
Celestia configuration.

| Aspect | Standalone mode (default) | Network mode (Celestia enabled) |
|--------|--------------------------|--------------------------------|
| **Storage** | All state local (`.tiagent/`) | Local + DA layer + optional archive |
| **Self-improvement** | Per-instance: the agent learns from its own experience only | Collective: every agent benefits from every other agent's experience |
| **Shared learning** | None --- routing weights, playbooks, and fingerprints stay on disk | Routing deltas, playbooks, HDC fingerprints, and efficiency summaries are published to DA and consumed by all agents |
| **Verifiability** | Local audit logs (episode JSONL, gate results) | Cryptographically verifiable traces with NMT inclusion proofs |
| **Discovery** | N/A --- single agent | Agents discover each other via the system namespace and find behaviorally similar peers via HDC fingerprint comparison |
| **Cost** | Disk space only | Disk + DA fees (~$0.37--$4.30/day for a typical workload) |
| **Setup** | `tiagent init` --- nothing else needed | Configure `[celestia]` in `tiagent.toml` and connect to a node |
| **Best for** | Individual developers, single-machine deployments, evaluation | Teams, multi-agent swarms, cross-organization learning, production deployments that benefit from network effects |

The rest of this document describes the storage patterns used in Network mode. If you are
running standalone, all of these data types live in local storage and the self-improvement
loop works identically --- just without the sharing.

---

## 2. What Goes Where

The following decision matrix determines where each data type lives. "Primary" means the
canonical, queryable copy. "Published" means a copy or commitment is written to DA for sharing
or verification. "Archived" means periodic snapshots are written to permanent storage (e.g.,
Arweave) for long-term durability.

**In standalone mode (no Celestia), everything lives in the Local store column.** The DA
and archive columns only apply when the Celestia integration is enabled. A standalone agent
stores, queries, and learns from all of these data types locally.

| Data type | Local store (primary) | Celestia DA (published) | Permanent archive | Standalone mode |
|-----------|----------------------|------------------------|-------------------|-----------------|
| **Vector embeddings** | HNSW index (queryable) | Commitment hash only | Periodic snapshot | Local only |
| **HDC fingerprints** | In-memory cache | Full fingerprints (small enough) | Via DA blobs | Local only |
| **Episode traces** | JSONL buffer | Full episode blobs | Via DA blobs | Local only |
| **Model routing weights** | Active state file | Delta updates | Via DA blobs | Local only |
| **Tool call traces** | JSONL buffer | Published within episodes | Via DA blobs | Local only |
| **Gate results** | Local cache | Full result blobs | Via DA blobs | Local only |
| **Playbook entries** | Local knowledge store | Full playbook blobs | Via DA blobs | Local only |
| **Efficiency metrics** | Aggregation buffer | Periodic summaries | Via DA blobs | Local only |

The logic behind each decision:

- **Vector embeddings** are too large and too query-dependent for DA. A 10,000-vector HNSW
  index with 768-dimensional float32 embeddings is ~30 MB. Publishing the full index to DA
  every time it changes would cost $2--$24 per update. Instead, we publish a Merkle root
  commitment (32 bytes) that lets anyone verify the index's integrity without downloading it.

- **HDC fingerprints** are the sweet spot for DA. A 1024-bit binary fingerprint is 128 bytes.
  Even batching 1,000 fingerprints into one blob costs under 130 KB --- well under $0.01 on DA.
  Other agents can read these fingerprints to discover behavioral similarity without exchanging
  full episode data.

- **Episode traces** and **gate results** are the core audit trail. They are append-only,
  moderate in size (20--80 KB per episode), and their value increases when shared. These are
  the primary DA workload.

- **Routing weights** change incrementally. Rather than publishing the full weight table on
  every update, we publish delta blobs that describe what changed. Other agents merge these
  deltas into their own local routing state.

```
Data flow overview:

    Local Agent Process                     Celestia DA Layer
    ───────────────────                     ─────────────────

    ┌──────────────────┐
    │ Vector Store     │───commitment──────► tiagent/learn namespace
    │ (HNSW index)     │   (32 bytes)        (Merkle root blob)
    └──────────────────┘

    ┌──────────────────┐
    │ HDC Fingerprints │───full publish────► tiagent/learn namespace
    │ (128 bytes each) │   (batch blob)      (fingerprint batch blob)
    └──────────────────┘

    ┌──────────────────┐
    │ Episode Buffer   │───full publish────► tiagent/agent/{id} namespace
    │ (JSONL)          │   (per episode)     (episode blob)
    └──────────────────┘

    ┌──────────────────┐
    │ Routing Weights  │───delta publish───► tiagent/learn namespace
    │ (JSON state)     │   (periodic)        (routing delta blob)
    └──────────────────┘

    ┌──────────────────┐
    │ Gate Results     │───full publish────► tiagent/agent/{id} namespace
    │ (cache)          │   (per gate run)    (gate result blob)
    └──────────────────┘
```

---

## 3. HDC Fingerprints on DA

### 3.1 What is HDC?

Hyperdimensional Computing (HDC) is a computational model that represents information as
high-dimensional vectors --- typically 1,024 to 10,000 dimensions. Unlike dense float vectors
used in neural network embeddings, HDC vectors are **binary** (each dimension is 0 or 1) or
**bipolar** (each dimension is -1 or +1).

HDC vectors have useful algebraic properties:

| Operation | What it does | Example use in tiagent |
|-----------|-------------|------------------------|
| **Bundling** (element-wise OR/majority) | Combines multiple vectors into one that is similar to all inputs | Summarize an episode's tool calls into one fingerprint |
| **Binding** (element-wise XOR) | Creates a vector that is dissimilar to both inputs but encodes their relationship | Associate a task type with a model name |
| **Permutation** (bit rotation) | Creates a positionally-aware encoding | Encode the sequence of tool calls (order matters) |
| **Hamming distance** | Counts differing bits between two vectors | Measure behavioral similarity between episodes |

The key advantage for agent systems: HDC fingerprints are **extremely compact** and
**comparison is extremely fast**. Computing Hamming distance between two 1024-bit vectors
is a single XOR + popcount operation, executable in nanoseconds on modern hardware.

### 3.2 What tiagent fingerprints

tiagent computes HDC fingerprints for three purposes:

1. **Episode fingerprinting** --- each completed episode gets a fingerprint that captures its
   behavioral signature: which tools were called, in what order, what kinds of outputs were
   produced. Two episodes that solved similar problems in similar ways will have similar
   fingerprints (low Hamming distance).

2. **Agent behavioral fingerprinting** --- an agent's overall behavioral signature is the
   bundled (majority-vote) fingerprint of its recent episodes. This captures "what kind of work
   does this agent typically do and how does it do it?"

3. **Task similarity** --- task descriptions are fingerprinted to enable fast "have I seen a
   task like this before?" lookups without running a full embedding model.

### 3.3 Fingerprint sizing

| Dimensionality | Size per fingerprint | Fingerprints per 1 MB blob | DA cost per 1,000 fingerprints |
|---------------|---------------------|---------------------------|-------------------------------|
| 1,024 bits | 128 bytes | ~8,000 | $0.009--$0.10 |
| 4,096 bits | 512 bytes | ~2,000 | $0.036--$0.41 |
| 10,000 bits | 1,250 bytes | ~800 | $0.088--$1.01 |

tiagent defaults to 1,024-bit fingerprints. This gives adequate discrimination for behavioral
clustering while keeping DA costs negligible. At 128 bytes per fingerprint, you can publish
thousands of fingerprints per day for pennies.

### 3.4 HDC blob schema

Fingerprints are batched into periodic blobs rather than published individually. A batch blob
contains multiple fingerprints from a single agent, covering a time window:

```rust
/// A batch of HDC fingerprints published to the DA layer.
/// Published to the `tiagent/learn` namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcFingerprintBatch {
    /// Protocol version for forward compatibility.
    pub version: u8,

    /// Agent that computed these fingerprints.
    pub agent_id: String,

    /// Time window covered by this batch.
    pub from_timestamp: u64,
    pub to_timestamp: u64,

    /// The fingerprints in this batch.
    pub fingerprints: Vec<HdcEntry>,
}

/// A single HDC fingerprint with its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcEntry {
    /// What this fingerprint represents.
    pub kind: HdcKind,

    /// The fingerprint itself: a bit-packed vector.
    /// For 1024-bit fingerprints, this is exactly 128 bytes.
    pub vector: Vec<u8>,

    /// Content hash of the source data (episode, task, etc.).
    /// Enables cross-referencing with full episode blobs.
    pub source_hash: Hash,

    /// Human-readable label for debugging (e.g., "deploy-rollup-task").
    pub label: Option<String>,
}

/// What kind of data was fingerprinted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HdcKind {
    /// Fingerprint of a single episode's behavioral pattern.
    Episode,
    /// Aggregate fingerprint of an agent's recent behavior.
    AgentBehavior,
    /// Fingerprint of a task description for similarity lookup.
    TaskSimilarity,
}
```

### 3.5 Publishing fingerprints

Fingerprints are batched by a timer or by count threshold --- whichever triggers first:

```rust
/// Configuration for HDC fingerprint publishing.
pub struct HdcPublishConfig {
    /// Publish a batch when this many new fingerprints accumulate.
    pub batch_size: usize,       // default: 50
    /// Publish a batch when this many seconds pass since last publish.
    pub flush_interval_secs: u64, // default: 300 (5 minutes)
}

impl HdcPublisher {
    /// Called after each episode completes. Buffers the fingerprint
    /// and publishes a batch when thresholds are reached.
    pub async fn record(&mut self, entry: HdcEntry) -> Result<()> {
        self.buffer.push(entry);

        let should_flush = self.buffer.len() >= self.config.batch_size
            || self.last_flush.elapsed() >= Duration::from_secs(self.config.flush_interval_secs);

        if should_flush {
            self.flush().await?;
        }
        Ok(())
    }

    /// Serialize the buffer into an HdcFingerprintBatch and submit to DA.
    async fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let batch = HdcFingerprintBatch {
            version: 1,
            agent_id: self.agent_id.clone(),
            from_timestamp: self.batch_start_time,
            to_timestamp: now_millis(),
            fingerprints: std::mem::take(&mut self.buffer),
        };

        let signal = Signal::new(SignalKind::HdcBatch, batch)?;
        self.substrate.write(&signal).await?;
        self.last_flush = Instant::now();
        self.batch_start_time = now_millis();
        Ok(())
    }
}
```

### 3.6 Cross-agent fingerprint sharing

When agent B wants to find agents with similar behavior to itself, it reads HDC batches from
the `tiagent/learn` namespace and computes Hamming distances:

```
Agent B reads tiagent/learn namespace:

    Block N:    [Agent A batch: 50 fingerprints]
    Block N+3:  [Agent C batch: 30 fingerprints]
    Block N+7:  [Agent A batch: 50 fingerprints]
    Block N+9:  [Agent D batch: 45 fingerprints]

Agent B computes its own aggregate fingerprint (bundled from recent episodes),
then computes Hamming distance to every other agent's aggregate fingerprint:

    Agent A:  distance = 142 / 1024 = 13.9%  ← similar behavior
    Agent C:  distance = 487 / 1024 = 47.6%  ← very different
    Agent D:  distance = 198 / 1024 = 19.3%  ← somewhat similar

Agent B now knows Agent A is the most behaviorally similar peer.
It can prioritize reading Agent A's episodes for relevant learning.
```

This enables **targeted learning**: instead of reading every episode from every agent
(expensive), an agent can first scan fingerprints (cheap) to identify which peers are most
likely to have relevant experience, then selectively read those peers' full episodes.

---

## 4. Vector Store Commitment Pattern

### 4.1 Why the full index stays local

A vector store (also called a vector database or embedding index) supports **approximate nearest
neighbor (ANN)** search: given a query vector, find the k most similar vectors in the index.
The standard data structure for this is **HNSW** (Hierarchical Navigable Small World graph),
which organizes vectors into a layered graph for fast traversal.

An HNSW index has properties that make it fundamentally incompatible with DA storage:

| Property | DA compatibility |
|----------|-----------------|
| **Mutable** --- insertions modify the graph structure | DA is append-only; no in-place updates |
| **Random access** --- queries traverse graph edges | DA supports sequential reads by namespace + height only |
| **Large** --- 10K vectors at 768 dims = ~30 MB raw, ~45 MB with graph edges | Expensive to republish on every change |
| **Latency-sensitive** --- queries must complete in <10 ms | DA reads take 100--500 ms minimum |

The HNSW index must live locally. But other agents should be able to **verify** that a given
agent's index has not been tampered with, and agents recovering from crashes should be able to
**reconstruct** their index from published data.

### 4.2 The commitment pattern

The commitment pattern bridges local storage and DA verification. The local index is the
authoritative, queryable store. Periodically, the agent computes a **Merkle root** of the
index contents and publishes it to DA as a compact commitment blob.

```
Commitment lifecycle:

    ┌─────────────────────────────────────────────────────────┐
    │ Local HNSW Index                                        │
    │                                                         │
    │   vec_001  [0.12, 0.87, 0.03, ...]  ──┐                │
    │   vec_002  [0.45, 0.22, 0.91, ...]    │                │
    │   vec_003  [0.78, 0.11, 0.54, ...]    ├── Merkle tree  │
    │   ...                                  │                │
    │   vec_N    [0.33, 0.66, 0.19, ...]  ──┘                │
    │                                     │                   │
    │                              ┌──────▼──────┐            │
    │                              │ Merkle root │            │
    │                              │ (32 bytes)  │            │
    │                              └──────┬──────┘            │
    └─────────────────────────────────────┼───────────────────┘
                                          │
                                          │  Published to DA
                                          ▼
                              ┌────────────────────┐
                              │ Commitment Blob    │
                              │                    │
                              │ merkle_root: 0xa3..│
                              │ vector_count: 9847 │
                              │ dimensions: 768    │
                              │ schema_v: 2        │
                              │ timestamp: ...     │
                              │ index_hash: 0xf1.. │
                              └────────────────────┘
                                tiagent/agent/{id}
                                namespace on DA
```

### 4.3 Commitment blob schema

```rust
/// Commitment to the current state of an agent's vector store.
/// Published to the agent's own namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreCommitment {
    /// Protocol version.
    pub version: u8,

    /// Merkle root computed over the sorted list of (hash, embedding) pairs.
    /// Any party with a copy of the index can recompute this root and verify
    /// it matches.
    pub merkle_root: Hash,

    /// Number of vectors currently in the index.
    pub vector_count: u64,

    /// Dimensionality of the embeddings (e.g., 384, 768, 1536).
    pub dimensions: u32,

    /// Schema version of the embedding model and quantization format.
    /// Ensures consumers know how to interpret the vectors.
    pub schema_version: u32,

    /// When this commitment was computed.
    pub committed_at: u64,

    /// SHA-256 of the serialized HNSW graph structure.
    /// Distinct from merkle_root: this captures the graph topology,
    /// not just the vector contents.
    pub index_structure_hash: Hash,

    /// Reference to the most recent full snapshot in permanent storage.
    /// Enables recovery: download snapshot, then replay episodes since.
    pub latest_snapshot_ref: Option<ArchiveRef>,
}

/// Reference to data in permanent storage (e.g., Arweave, IPFS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveRef {
    /// Storage system ("arweave", "ipfs", "s3").
    pub backend: String,
    /// Identifier within that system (transaction ID, CID, key).
    pub id: String,
    /// SHA-256 of the archived data for integrity verification.
    pub content_hash: Hash,
}
```

### 4.4 Verification flow

Another agent (or an auditor) can verify an agent's vector store by:

1. Reading the latest `VectorStoreCommitment` from the agent's DA namespace.
2. Requesting the full index data (via direct transfer or from the archive reference).
3. Recomputing the Merkle root from the received data.
4. Comparing the recomputed root to the published commitment.

If they match, the index contents are verified to be exactly what the agent claimed. If they
differ, the agent has either modified its index since the commitment or published a false
commitment.

### 4.5 Commitment frequency

How often should commitments be published? The tradeoff is between verification freshness and
DA cost:

| Frequency | Cost per month (mainnet) | Staleness window |
|-----------|------------------------|------------------|
| Every episode | ~$0.03--$0.30 per commit x ~3000 episodes | <1 minute | Expensive and unnecessary |
| Every 100 episodes | ~$0.03--$0.30 per commit x ~30 commits | ~hours | Good balance |
| Daily | ~$0.03--$0.30 per commit x ~30 commits | ~24 hours | Adequate for most uses |

The recommended default is **every 100 episodes or daily, whichever comes first**. This keeps
costs under $10/month while ensuring the commitment is never more than a day stale.

---

## 5. Shared Learning via DA

### 5.1 What is shared learning?

When an agent discovers that Claude Sonnet performs better than GPT-4o for Rust code generation
tasks, that knowledge is valuable to every other tiagent in the network. Shared learning is
the mechanism by which agents publish what they have learned and consume what others have
learned, using the DA layer as the communication medium.

Three kinds of learning artifacts are shared:

1. **Cascade router weight deltas** --- incremental updates to model selection preferences.
2. **Efficiency metric summaries** --- aggregated cost, latency, and token usage statistics.
3. **Playbook entries** --- reusable strategies extracted from high-scoring episodes.

### 5.2 Router weight delta pattern

The cascade router maintains a table of weights: for each task category, how well does each
model perform? Rather than publishing the full weight table (which would grow as more models
and categories are added), agents publish **deltas** --- what changed since the last publication.

```rust
/// Incremental update to cascade router weights.
/// Published to the `tiagent/learn` namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDelta {
    /// Protocol version.
    pub version: u8,

    /// Agent that observed these results.
    pub agent_id: String,

    /// Number of episodes that contributed to this delta.
    pub episode_count: u64,

    /// Time window of observations.
    pub from_timestamp: u64,
    pub to_timestamp: u64,

    /// The delta entries: changes to specific category/model pairs.
    /// Positive delta = model performed better than expected.
    /// Negative delta = model performed worse than expected.
    pub deltas: Vec<RoutingDeltaEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDeltaEntry {
    /// Task category (e.g., "code_generation", "test_writing", "documentation").
    pub category: String,
    /// Model identifier (e.g., "claude-sonnet-4", "gpt-4o").
    pub model: String,
    /// Change in performance score (range: -1.0 to +1.0).
    pub delta: f64,
    /// How many episodes contributed to this specific delta.
    pub sample_count: u64,
}
```

### 5.3 Consuming shared learning

When an agent reads routing deltas from the `tiagent/learn` namespace, it merges them into
its own local routing state using a weighted average:

```
Merge algorithm:

    For each (category, model) pair in the incoming delta:

        local_weight = current local weight for this pair (default: 0.5)
        remote_delta = incoming delta value
        remote_samples = incoming sample count
        local_samples = local sample count for this pair

        # Weight the remote observation by its sample size relative to local
        trust_factor = min(remote_samples / (local_samples + remote_samples), 0.3)

        # Apply the delta with bounded trust
        new_weight = local_weight + (remote_delta * trust_factor)

        # Clamp to valid range
        new_weight = clamp(new_weight, 0.0, 1.0)
```

The `trust_factor` is capped at 0.3 to prevent a single remote agent from dominating local
routing decisions. An agent always trusts its own experience more than remote observations.

### 5.4 Conflict resolution

Multiple agents may publish conflicting routing deltas (agent A says model X is great, agent B
says model X is terrible). The conflict resolution strategy is **experience-weighted merging**:

1. Each delta carries a `sample_count` field.
2. Deltas backed by more observations receive proportionally more influence.
3. The local agent's own observations always receive a floor weight of 70%.
4. If conflicting deltas cancel out, the local agent's prior state is preserved.

This is intentionally simple. The routing table is a soft preference, not a hard constraint ---
even a suboptimal routing decision only costs one extra LLM call, which is cheap relative to
the value of the learning signal.

### 5.5 Efficiency metric sharing

Efficiency metrics capture the cost-performance characteristics of different models and
strategies. They are published as periodic summaries:

```rust
/// Aggregated efficiency metrics published for cross-agent learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencySummary {
    pub version: u8,
    pub agent_id: String,
    pub period_start: u64,
    pub period_end: u64,

    /// Per-model statistics.
    pub model_stats: Vec<ModelEfficiency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEfficiency {
    pub model: String,
    pub task_count: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_cost_usd: f64,
    pub avg_duration_ms: f64,
    pub avg_completion_score: f64,
}
```

Other agents use these summaries to bootstrap cost estimates for models they have not yet
tried, and to calibrate their own expectations against the broader network.

---

## 6. Trace Publishing for TraceCommons

### 6.1 What is TraceCommons?

TraceCommons is a system for scoring trace quality and enabling trajectory retrieval-augmented
generation (RAG) across agents. The full design is in **07-tracecommons-integration.md**. This
section covers only the DA storage aspect: how traces are formatted and published for
TraceCommons compatibility.

### 6.2 Trace schema

Traces published to the `tiagent/trace` namespace follow a structured schema that enables
cross-agent trajectory search:

```rust
/// A TraceCommons-compatible trace published to DA.
/// Published to the `tiagent/trace` namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCommonsEntry {
    /// Protocol version.
    pub version: u8,

    /// Unique trace identifier.
    pub trace_id: String,

    /// Agent that produced this trace.
    pub agent_id: String,

    /// Task description that was being executed.
    pub task_description: String,

    /// Ordered segments of the execution trajectory.
    pub segments: Vec<TrajectorySegment>,

    /// Multi-dimensional quality score.
    pub quality_score: QualityScore,

    /// HDC fingerprint of this trace's behavioral pattern.
    /// Enables fast similarity filtering before full comparison.
    pub hdc_fingerprint: Vec<u8>,

    /// Quantized embedding of the task description.
    /// Enables semantic search across traces.
    /// Int8 quantization: 384 bytes for a 384-dim model.
    pub task_embedding_quantized: Option<Vec<i8>>,
}

/// One segment of an execution trajectory: a tool call
/// with its inputs, outputs, and observed reward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectorySegment {
    /// Which tool was called.
    pub tool_name: String,
    /// Abbreviated inputs (truncated to avoid large blobs).
    pub input_summary: String,
    /// Abbreviated outputs.
    pub output_summary: String,
    /// Time spent in this segment (milliseconds).
    pub duration_ms: u64,
    /// Reward signal: did this tool call contribute to task success?
    /// Range: -1.0 (harmful) to +1.0 (highly useful).
    pub reward: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub completion: f64,
    pub efficiency: f64,
    pub safety: f64,
    pub overall: f64,
}
```

### 6.3 Publication flow

```
Episode completes
    │
    ├──1──► Gate pipeline validates output
    │
    ├──2──► Score quality (completion, efficiency, safety)
    │
    ├──3──► Compute HDC fingerprint of behavioral pattern
    │
    ├──4──► Optionally compute quantized task embedding
    │
    ├──5──► Build TraceCommonsEntry from episode + scores + fingerprint
    │
    ├──6──► Serialize (MessagePack) and submit to tiagent/trace namespace
    │
    └──7──► Record DA reference locally for future retrieval
```

Not every episode is published to TraceCommons. A publication filter selects traces that
meet a minimum quality threshold:

```rust
/// Determine whether an episode's trace should be published to TraceCommons.
fn should_publish_trace(score: &QualityScore) -> bool {
    // Only publish traces that demonstrate competent execution.
    // Low-quality traces add noise without learning value.
    score.overall >= 0.6 && score.safety >= 0.8
}
```

---

## 7. Snapshot and Recovery

### 7.1 The durability problem

Celestia light nodes prune blobs after 7 days. Archival nodes keep data longer, but they are
not guaranteed to be available forever. An agent that crashes and restarts after a week may
find that some of its DA blobs are no longer accessible.

The snapshot-and-recovery pattern ensures that agent state can be reconstructed even when
original DA blobs have been pruned.

### 7.2 Snapshot contents

A full-state snapshot captures everything needed to reconstruct an agent's local state:

```rust
/// A full snapshot of an agent's local state, suitable for archival.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Snapshot protocol version.
    pub version: u8,

    /// Agent identity.
    pub agent_id: String,

    /// When this snapshot was taken.
    pub created_at: u64,

    /// The most recent DA block height included in this snapshot.
    /// Recovery replays from this height forward.
    pub last_da_height: u64,

    /// Serialized vector store (HNSW index).
    /// Compressed with zstd before archival.
    pub vector_store: Vec<u8>,

    /// All HDC fingerprints.
    pub hdc_fingerprints: Vec<HdcEntry>,

    /// Current cascade router weights (full table, not deltas).
    pub routing_weights: HashMap<String, HashMap<String, f64>>,

    /// Playbook entries.
    pub playbooks: Vec<PlaybookEntry>,

    /// Local hash index (content hash → DA reference).
    pub hash_index: HashMap<Hash, DaRef>,
}
```

### 7.3 Snapshot schedule

Full snapshots are written to permanent storage (Arweave, IPFS, or S3) on a configurable
schedule. After each snapshot, a commitment is published to DA:

```
Snapshot lifecycle:

    ┌──────────────┐     ┌──────────────────┐     ┌──────────────┐
    │ Local state  │────►│ Serialize + zstd │────►│ Upload to    │
    │ (index,      │     │ compress         │     │ Arweave/IPFS │
    │  weights,    │     │ (~15-50 MB)      │     │              │
    │  fingerprints│     └──────────────────┘     └──────┬───────┘
    │  playbooks)  │                                      │
    └──────────────┘                                      │
                                                          │ returns archive ref
                                                          ▼
                                               ┌──────────────────┐
                                               │ Publish snapshot  │
                                               │ commitment to DA  │
                                               │                   │
                                               │ {merkle_root,     │
                                               │  archive_ref,     │
                                               │  last_da_height,  │
                                               │  vector_count}    │
                                               └──────────────────┘
```

Default schedule: weekly, or after every 1,000 episodes, whichever comes first.

### 7.4 Recovery flow

When an agent needs to reconstruct its state (crash recovery, migration to new hardware,
or bootstrapping a new agent from an existing one):

```
Recovery steps:

    1. Read the agent's DA namespace for the latest SnapshotCommitment blob.
       ├── If found: proceed to step 2.
       └── If not found (all commitments pruned): start from scratch.

    2. Download the full snapshot from the archive reference.
       ├── Verify content_hash matches.
       └── Decompress and deserialize into AgentSnapshot.

    3. Restore local state from the snapshot:
       ├── Rebuild HNSW index from vector_store bytes.
       ├── Load routing weights.
       ├── Load HDC fingerprints.
       └── Load playbooks and hash index.

    4. Replay DA blobs from snapshot's last_da_height to current height:
       ├── Read agent namespace for episodes and gate results.
       ├── Read learn namespace for routing deltas from other agents.
       └── Apply each blob to local state (insert vectors, merge weights).

    5. Agent is now caught up and can resume normal operation.
```

The replay step (4) is bounded by the DA retention window: if the snapshot is less than 7 days
old, all blobs since the snapshot are still available on light nodes. If the snapshot is older
than 7 days, the agent must use archival nodes or accept a gap in its state.

---

## 8. Cost Optimization

### 8.1 The cost equation

Every DA blob submission has a cost determined by:

```
cost = blob_size_bytes * gas_per_byte * gas_price_utia * (TIA_price_usd / 1_000_000)
```

At current rates, this works out to approximately $0.07--$0.81 per megabyte. For a typical
tiagent deployment, the breakdown looks like:

| Data type | Size per item | Items per day | Daily MB | Daily cost range |
|-----------|--------------|---------------|----------|-----------------|
| Episodes | 50 KB avg | 100 | 5.0 MB | $0.35--$4.05 |
| Gate results | 3 KB avg | 100 | 0.3 MB | $0.02--$0.24 |
| HDC fingerprints | 13 KB/batch | 2 batches | 0.03 MB | <$0.01 |
| Routing deltas | 2 KB avg | 2 deltas | 0.004 MB | <$0.01 |
| Vector commitments | 0.5 KB | 1 commit | 0.0005 MB | <$0.01 |
| **Total** | | | **~5.3 MB** | **$0.37--$4.30/day** |

At $0.37--$4.30 per day, DA costs are comparable to a few LLM API calls --- negligible
relative to the agent's total operating cost (which is dominated by LLM inference).

### 8.2 Batching

The most effective cost reduction is batching: instead of submitting one blob per event,
accumulate events and submit them in a single blob. Batching reduces cost because the overhead
per blob submission (gas for the transaction itself, not just the data) is amortized across
multiple items.

```rust
/// Generic batching buffer for DA submissions.
pub struct DaBatchBuffer<T: Serialize> {
    items: Vec<T>,
    max_items: usize,      // default: 50
    max_age: Duration,     // default: 5 minutes
    created_at: Instant,
}

impl<T: Serialize> DaBatchBuffer<T> {
    /// Add an item to the buffer. Returns Some(batch) if the buffer
    /// should be flushed (either full or too old).
    pub fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.items.push(item);

        if self.items.len() >= self.max_items
            || self.created_at.elapsed() >= self.max_age
        {
            Some(std::mem::take(&mut self.items))
        } else {
            None
        }
    }
}
```

### 8.3 Compression

All blobs are compressed before submission. MessagePack serialization already produces compact
output, but zstd compression typically achieves an additional 40--60% reduction on agent data:

| Data type | Raw MessagePack | After zstd | Savings |
|-----------|----------------|------------|---------|
| Episode (50 KB raw) | 35 KB | 15 KB | 57% |
| HDC batch (13 KB raw) | 10 KB | 8 KB | 20% (binary data compresses poorly) |
| Routing delta (2 KB raw) | 1.5 KB | 0.8 KB | 47% |

The compression ratio for HDC fingerprints is low because they are high-entropy binary vectors
with little redundancy. For episodes and routing data, which contain repeated field names and
structured text, the savings are substantial.

### 8.4 Priority tiers

Not every signal warrants DA publication. tiagent classifies signals into priority tiers:

| Tier | What qualifies | DA treatment |
|------|---------------|-------------|
| **Critical** | Gate results, work proofs, coordination messages | Always publish immediately |
| **Standard** | Episodes, HDC fingerprints, routing deltas | Batch and publish periodically |
| **Optional** | Efficiency summaries, detailed tool call logs | Publish only if budget allows |
| **Local-only** | Raw prompts/responses, intermediate state, secrets | Never publish to DA |

The agent's DA budget (configurable in `tiagent.toml`) controls which tiers are active:

```toml
# tiagent.toml

[da.budget]
# Maximum daily DA spend in USD. Tiers above this budget are suppressed.
max_daily_usd = 5.00

# Priority tiers to publish (from highest to lowest priority).
# If budget is exhausted, lower tiers are suppressed first.
tiers = ["critical", "standard", "optional"]

# Override: always publish these signal kinds regardless of budget.
always_publish = ["gate_result", "work_proof"]
```

When the daily budget is approaching its limit, the agent automatically drops lower-priority
publications while maintaining the critical audit trail.

### 8.5 Deduplication

Before publishing, the agent checks whether a substantially similar blob was recently
published. HDC fingerprints enable fast deduplication:

```
Deduplication check:

    New episode fingerprint: 0b1010110...
    Recent published fingerprints (last 24h):

        Episode A:  distance = 12 / 1024 = 1.2%   ← near-duplicate, skip
        Episode B:  distance = 342 / 1024 = 33.4%  ← sufficiently different
        Episode C:  distance = 8 / 1024 = 0.8%     ← near-duplicate, skip

    Threshold: 5% (configurable)
    If distance < threshold, skip publication (the trace adds no new information).
```

Deduplication is especially valuable for agents that perform repetitive tasks (e.g., running
the same deployment workflow multiple times). The first trace is published; subsequent
near-identical traces are suppressed.
