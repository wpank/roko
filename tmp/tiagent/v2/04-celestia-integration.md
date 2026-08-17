# Celestia Integration Design

tiagent uses Celestia as a shared learning substrate for AI coding agents. Agents publish
learning artifacts (routing weights, episode summaries, HDC fingerprints, playbooks) as blobs
in organized namespaces. Other agents read those blobs to bootstrap and continuously improve
their own routing, prompt strategies, and gate thresholds. The result is collective
intelligence: every agent's experience improves every other agent.

The integration embeds a lumina-node light node directly in the agent process for DA access
without external infrastructure dependencies.

---

## 1. What Goes On-Chain

Not everything belongs on DA. The decision criteria: publish data that is compact, append-only,
and gains value from being shared. Keep data local when it requires random access, mutates
frequently, or is too large relative to its sharing value.

### Published as blobs

| Data Type | Serialized Size | Write Frequency | Namespace | Encoding |
|-----------|----------------|-----------------|-----------|----------|
| Routing weight deltas | 2--5 KB | Every 10 completed tasks | `tiagent/learn` | bincode, zstd-compressed |
| Episode summaries | 1--3 KB | Per task completion | `tiagent/trace` | bincode |
| Efficiency metrics | ~500 B | Hourly aggregation | `tiagent/learn` | bincode |
| HDC fingerprints (batched) | 5--15 KB | Daily batch | `tiagent/agent/{id}` | raw bit-packed + metadata header |
| Playbook snapshots | 3--8 KB | On extraction from high-scoring episodes | `tiagent/learn` | bincode, zstd-compressed |
| Gate threshold updates | ~1 KB | Hourly (EMA values) | `tiagent/learn` | bincode |
| Agent registry heartbeat | ~200 B | Every 100 blocks (~20 min) | `tiagent/system` | bincode |

Total daily blob volume for a moderately active agent (50 tasks/day): roughly 150--300 KB.

### Why these sizes work

Routing weight deltas are compact because we publish diffs, not the full routing table. The
cascade router tracks ~20 task categories across ~10 model backends. A full snapshot is ~8 KB;
a delta covering the 2--3 categories that changed since the last publish is 2--5 KB.

Episode summaries are distilled from the full episode trace. The full trace includes every
model response and tool output (often 50--200 KB). The summary retains the structured metadata
(task type, tools called, gate pass/fail, model used, token count, latency) but strips raw
LLM output. This is the data other agents actually need for learning.

HDC fingerprints are inherently small. At 1,024 bits (128 bytes) per fingerprint, a daily
batch of 50 episode fingerprints plus one agent-behavioral fingerprint is ~6.5 KB of vector
data plus metadata overhead.

---

## 2. What Stays Local

| Data Type | Why It Stays Local |
|-----------|-------------------|
| Full episode logs | 50--200 KB each. Contains raw model responses, potentially sensitive code snippets, and customer context. Too large and too sensitive for DA. |
| Vector embeddings (HNSW index) | Requires random-access similarity search (k-NN). DA is append-only with no query capability. A 10K-vector index at 768 dimensions is ~30 MB. |
| Raw model responses | Ephemeral, sensitive, and large. No sharing value. |
| Local substrate (signals.jsonl) | The canonical signal log is the local append-only store. DA gets commitment hashes, not the full log. |
| Prompt experiment state | A/B experiment arms and their results are per-instance until an experiment concludes. Only the winning variant gets published as a playbook. |

### Commitment bridge

For data that stays local but needs verifiability, we publish **commitment hashes** to DA.
The local vector store periodically computes a Merkle root over its contents and publishes
the 32-byte root to `tiagent/learn`. Any auditor can request the full index from the agent
and verify it against the on-chain commitment.

```
Local HNSW index (30 MB)
    |
    +---> Merkle root (32 bytes) ---> published to tiagent/learn namespace
    |
    +---> Full index available via agent sidecar HTTP API for audit
```

---

## 3. Namespace Design

tiagent uses four namespaces, encoded into Celestia's 29-byte namespace (1-byte version prefix
+ 28-byte ID).

### 28-byte ID layout

```
Byte offsets:
 0         6    7        8                          28
 +---------+----+--------+--------------------------+
 | "tiagnt" | v  | cat   |     identifier           |
 | 6 bytes  | 1B | 1B    |     20 bytes             |
 +---------+----+--------+--------------------------+

v:   protocol version (0x01)
cat: category byte
```

### Category assignments

| Category | Byte | Identifier (20 bytes) | Purpose |
|----------|------|-----------------------|---------|
| System | `0x01` | `"system"` + 14 zero-pad bytes | Agent registry, protocol version announcements, namespace directory |
| Agent | `0x02` | `SHA-256(agent_id)[0..20]` | Per-agent episodes, gate results, state snapshots, HDC fingerprint batches |
| Learn | `0x03` | `"global"` + 14 zero-pad bytes | Routing deltas, playbooks, efficiency summaries, vector store commitments |
| Trace | `0x04` | `"global"` + 14 zero-pad bytes | TraceCommons-formatted episode data for cross-agent trajectory RAG |

### Namespace construction in Rust

```rust
use celestia_types::nmt::Namespace;

const PREFIX: &[u8; 6] = b"tiagnt";
const VERSION: u8 = 0x01;

fn make_namespace(category: u8, id_bytes: &[u8]) -> Namespace {
    let mut raw = [0u8; 29];
    raw[0] = 0x00; // Celestia namespace version byte (v0)
    raw[1..7].copy_from_slice(PREFIX);
    raw[7] = VERSION;
    raw[8] = category;
    let len = id_bytes.len().min(20);
    raw[9..9 + len].copy_from_slice(&id_bytes[..len]);
    Namespace::from_raw(&raw).expect("valid namespace")
}

fn system_namespace() -> Namespace {
    make_namespace(0x01, b"system")
}

fn agent_namespace(agent_id: &str) -> Namespace {
    let hash = sha2::Sha256::digest(agent_id.as_bytes());
    make_namespace(0x02, &hash[..20])
}

fn learn_namespace() -> Namespace {
    make_namespace(0x03, b"global")
}

fn trace_namespace() -> Namespace {
    make_namespace(0x04, b"global")
}
```

### Why four namespaces, not more

Fewer namespaces means fewer `GetAll` queries per sync cycle. An agent bootstrapping from the
network issues four namespace queries (one per category) at the latest height range. Splitting
data types into more namespaces (e.g., separate namespaces for routing vs. playbooks vs.
fingerprints) would increase query count without meaningful benefit, since the combined blob
volume in `tiagent/learn` is small enough to fetch in one call and filter client-side by a
`blob_type` discriminator in the blob header.

The exception is per-agent namespaces (`0x02`), which use the agent ID hash in the identifier
field. This is necessary because per-agent data must be independently queryable --- you need
to be able to ask "show me everything agent X published" without scanning the entire learn
namespace.

---

## 4. Cost Model

All estimates use the `BlobTx` fee model: `gas_price * blob_size * gas_per_byte`. Current
mainnet gas prices range from ~0.002 utia/gas (low congestion) to ~0.01 utia/gas (high
congestion). 1 TIA ~ $10 at time of writing.

### Per-blob costs

| Blob Type | Size | Gas (est.) | Cost at Low | Cost at High |
|-----------|------|------------|-------------|--------------|
| Routing delta | 3 KB | ~24,000 | $0.0005 | $0.0024 |
| Episode summary | 2 KB | ~16,000 | $0.0003 | $0.0016 |
| Efficiency batch | 500 B | ~4,000 | $0.0001 | $0.0004 |
| HDC batch (daily) | 10 KB | ~80,000 | $0.0016 | $0.0080 |
| Playbook snapshot | 5 KB | ~40,000 | $0.0008 | $0.0040 |
| Gate thresholds | 1 KB | ~8,000 | $0.0002 | $0.0008 |
| Registry heartbeat | 200 B | ~1,600 | $0.00003 | $0.0002 |

### Daily/monthly costs by agent workload

| Workload | Tasks/Day | Blobs/Day | Daily DA Cost | Monthly DA Cost |
|----------|-----------|-----------|---------------|-----------------|
| Light (hobby dev) | 10 | ~15 | $0.01--$0.05 | $0.30--$1.50 |
| Moderate (active dev) | 50 | ~65 | $0.04--$0.22 | $1.20--$6.60 |
| Heavy (CI pipeline) | 200 | ~240 | $0.15--$0.85 | $4.50--$25.50 |
| Fleet (10 agents) | 500 | ~600 | $0.37--$2.10 | $11.00--$63.00 |

### Context: DA cost vs. LLM API cost

A moderate agent making 50 tasks/day with an average of 3 LLM calls per task at ~$0.03 per
call spends **$4.50/day on LLM APIs**. The DA cost for the same workload is $0.04--$0.22/day.
DA is roughly **1--5% of the LLM spend**. This ratio holds across workload sizes because both
scale linearly with task count, and DA blobs are summaries rather than full payloads.

---

## 5. Knowledge Lifecycle Alignment

Celestia light nodes prune blob data after a 7-day window. This is not a limitation for
tiagent --- it is architectural alignment. tiagent implements knowledge demurrage: every
learning artifact decays in value unless actively reinforced by successful use. The DA
window is a natural sharing period, and demurrage ensures anything worth keeping is consumed
and promoted to local durable storage well before pruning.

### Half-lives vs. the 7-day window

| Knowledge Type | Effective Half-Life | Relation to 7-Day Window |
|----------------|--------------------:|--------------------------|
| Warnings (gate failures, lint errors) | ~1 hour | Consumed or irrelevant within minutes |
| Router deltas | Hours (latest-wins) | Superseded by the next delta; only the most recent matters |
| Strategy fragments | 1.4--14 days | Tier-dependent; Transient tier decays well within the window, Working tier persists locally |
| Insights | 3--150 days | Short-lived tiers decay within the window; long-lived tiers have already been promoted to durable local storage |
| HDC fingerprints | N/A (snapshot) | Daily batches; genomic bottleneck snapshots preserve long-lived fingerprints locally |

Unreinforced knowledge dies by demurrage before the 7-day pruning window matters. Dream
consolidation processes and fingerprints episode data within hours, not days --- any artifact
worth retaining is distilled into the local knowledge store during the first consolidation
cycle after publication. The genomic bottleneck mechanism (`knowledge backup`) further
ensures that high-value HDC fingerprints survive indefinitely in compressed local snapshots,
independent of DA availability.

The 7-day window is generous. It gives peers ample time to discover, verify, and absorb
shared artifacts. After that, the DA layer has served its purpose and the data can be pruned
without loss.

---

## 6. Light Node Embedding

tiagent embeds a Celestia light node via `lumina-node` rather than depending on an external
`celestia-node` process or a remote RPC endpoint.

### Why embedded

| Approach | Pros | Cons |
|----------|------|------|
| External full node | Full block data, archival queries | Heavy (100+ GB storage, multi-GB RAM), separate process to manage |
| Remote RPC (consensus or bridge node) | Zero local resources | Trust dependency, latency, rate limits, availability risk |
| **Embedded light node (lumina)** | **Self-contained, DAS verification, ~50 MB RAM, no trust assumption** | **Cannot serve historical data beyond DAS window; needs bridge node for archival queries** |

The embedded approach fits tiagent's deployment model: a single binary that a developer
installs and runs. No Docker containers, no separate node processes, no RPC endpoint
configuration. The light node starts as part of the agent process and participates in the
P2P network directly.

### Resource requirements

| Resource | Light Node Overhead |
|----------|-------------------|
| Memory | ~50 MB baseline, ~100 MB during active sampling |
| Disk | ~200 MB for headers + recent samples (pruned automatically) |
| Bandwidth | ~5--20 KB/s sustained for DAS + blob submission/retrieval |
| CPU | Negligible outside of proof verification bursts |
| Startup time | ~5--15 seconds to connect to P2P network and sync recent headers |

### lumina-node integration

```rust
use lumina_node::{
    node::Node,
    store::InMemoryStore,
    blockstore::InMemoryBlockstore,
};
use celestia_types::{Blob, nmt::Namespace};

pub struct EmbeddedLightNode {
    node: Node<InMemoryBlockstore, InMemoryStore>,
}

impl EmbeddedLightNode {
    /// Start the embedded light node, connecting to the Celestia P2P network.
    /// `network` is typically `Network::Mocha` (testnet) or `Network::Mainnet`.
    /// `bootnodes` can be empty to use defaults, or specify custom bootstrap peers.
    pub async fn start(
        network: Network,
        bootnodes: &[Multiaddr],
    ) -> Result<Self> {
        let store = InMemoryStore::new();
        let blockstore = InMemoryBlockstore::new();

        let node = Node::new(NodeConfig {
            network,
            bootnodes: bootnodes.to_vec(),
            store,
            blockstore,
            ..Default::default()
        })
        .await?;

        node.start().await?;
        // Wait for initial header sync before returning
        node.syncer().await?.wait_until_synced().await?;

        Ok(Self { node })
    }

    /// Submit a blob to a namespace. Returns the block height at inclusion.
    pub async fn submit_blob(
        &self,
        namespace: Namespace,
        data: &[u8],
    ) -> Result<u64> {
        let blob = Blob::new(namespace, data.to_vec())?;
        let height = self.node.blob_submit(&[blob]).await?;
        Ok(height)
    }

    /// Retrieve all blobs in a namespace at a specific height.
    pub async fn get_blobs(
        &self,
        namespace: Namespace,
        height: u64,
    ) -> Result<Vec<Blob>> {
        self.node.blob_get_all(height, &[namespace]).await
    }

    /// Retrieve blobs across a height range (scans block by block).
    pub async fn get_blobs_range(
        &self,
        namespace: Namespace,
        from_height: u64,
        to_height: u64,
    ) -> Result<Vec<(u64, Blob)>> {
        let mut results = Vec::new();
        for h in from_height..=to_height {
            match self.node.blob_get_all(h, &[namespace]).await {
                Ok(blobs) => {
                    for b in blobs {
                        results.push((h, b));
                    }
                }
                Err(e) if e.is_not_found() => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(results)
    }
}
```

### Lifecycle

The light node starts when the agent enters network mode (Celestia config present in
`tiagent.toml`) and stops when the agent shuts down. Header sync runs continuously in the
background. Blob submissions are batched: the agent accumulates blobs in a local buffer and
submits them as a single `BlobTx` every N blocks (configurable, default 5 blocks / ~60
seconds) to amortize per-transaction overhead.

---

## 7. Proof Verification

Celestia's Namespaced Merkle Tree (NMT) provides two proof types that tiagent uses for
trustless state verification.

### Inclusion proofs

An NMT inclusion proof demonstrates that a specific blob exists within a specific namespace
at a specific block height. The proof is a standard Merkle path augmented with namespace
range annotations at each node.

tiagent uses inclusion proofs when:

- **Consuming shared learning artifacts**: before merging a routing delta from an unknown
  agent, verify it was actually included in a Celestia block (not fabricated).
- **Audit trails**: a compliance check can request "prove that agent X published episode
  summary Y at height H" and verify the proof without trusting the agent.
- **Dispute resolution**: if two agents disagree about what was published, the NMT proof
  is the arbiter.

```rust
use celestia_types::nmt::{NamespaceProof, NamespacedHash};

/// Verify that a blob was included in the block at the given height.
pub fn verify_inclusion(
    blob: &Blob,
    proof: &NamespaceProof,
    root: &NamespacedHash,
) -> bool {
    proof.verify_complete_namespace(root, &blob.to_shares(), blob.namespace)
        .is_ok()
}
```

### Absence proofs

An NMT absence proof demonstrates that **no** blobs exist in a given namespace at a given
height. tiagent uses absence proofs for:

- **Liveness monitoring**: if an agent's heartbeat namespace has an absence proof for the
  expected height range, other agents know it has gone offline.
- **Completeness guarantees**: when syncing a height range, absence proofs at intermediate
  heights confirm that no data was missed (the agent simply did not publish at those heights).

### Proof cost

NMT proofs are compact. An inclusion proof for a single blob in a block with 1,000 blobs is
roughly 300--500 bytes (log2(1000) ~ 10 hash nodes at ~32 bytes each, plus namespace
annotations). Verification is a single Merkle path walk --- sub-millisecond on any hardware.

---

## 8. Integration with Celestia Ecosystem

tiagent is designed as a sovereign application on Celestia's DA layer. It does not run inside
a rollup VM. It publishes and reads blobs directly. But it interacts with several parts of
the broader Celestia ecosystem.

### Celestia Node API (celestia-node)

tiagent's primary interface to Celestia. The embedded lumina light node speaks the same P2P
protocol as `celestia-node` and uses the same blob submission and retrieval APIs. For
deployments that already run a `celestia-node` (bridge or full), tiagent can optionally
connect via the node's JSON-RPC gateway instead of embedding its own light node:

```toml
# tiagent.toml
[celestia]
mode = "external"   # "embedded" (default) or "external"
rpc_url = "http://localhost:26658"
auth_token = "eyJ..."
```

### Sovereign SDK

tiagent is not built with Sovereign SDK, but it is compatible. A team building a sovereign
rollup with Sovereign SDK could integrate tiagent's learning namespace as a data source:
the rollup's STF reads `tiagent/learn` blobs and maintains an on-chain registry of agent
routing weights, enabling the rollup to act as a decentralized model routing oracle. tiagent
would publish to DA as it does today; the rollup would consume and verify those blobs as
part of its state transition.

### Rollkit

Similar to Sovereign SDK, a Rollkit-based chain could consume tiagent's DA blobs. The more
practical integration: tiagent itself could be used as a development agent for teams building
Rollkit rollups. The agent's code intelligence (via `tiagent-mcp-code`) understands Go and
Rust codebases, and its learning loop would accumulate Rollkit-specific playbooks over time.

### Blobstream

Blobstream relays Celestia block commitments to Ethereum L1 (and other EVM chains). This
enables a bridge pattern: tiagent publishes agent traces to Celestia DA, and a smart contract
on Ethereum can verify those traces via Blobstream attestations. Use cases include:

- On-chain bounty verification: prove an agent completed a task by verifying its trace
  commitment on Ethereum.
- Cross-chain agent reputation: an Ethereum contract maintains reputation scores derived from
  verified Celestia traces.

### Archival beyond the DAS window

Light nodes prune blob data after the sampling window (~30 days on mainnet, shorter on
testnet). For data that must persist longer, tiagent supports two archival strategies:

1. **Archival node query**: configure a bridge or full archival node endpoint for historical
   lookups. The agent uses the embedded light node for recent data and falls back to the
   archival RPC for older heights.

2. **Cold backup to permanent storage**: periodically snapshot DA-published data to Arweave,
   Filecoin, or S3. The DA blob's inclusion proof is stored alongside the data, so the
   archived copy remains verifiable even after Celestia nodes have pruned it.

```toml
# tiagent.toml
[celestia.archival]
strategy = "archival_node"   # or "cold_backup"
archival_rpc = "http://archival-node:26658"

# For cold_backup strategy:
# [celestia.archival.cold]
# backend = "arweave"         # or "filecoin", "s3"
# interval_hours = 24
```

---

## 9. Blob Wire Format

Every tiagent blob starts with a 4-byte header for forward compatibility and client-side
filtering:

```
Byte 0:    protocol version (0x01)
Byte 1:    blob type discriminator
Bytes 2-3: reserved (0x00, 0x00)
Bytes 4+:  payload (bincode or raw, per blob type)
```

### Blob type discriminators

| Byte | Type | Namespace | Payload |
|------|------|-----------|---------|
| `0x01` | Routing weight delta | Learn | zstd-compressed bincode `RoutingDelta` |
| `0x02` | Episode summary | Trace | bincode `EpisodeSummary` |
| `0x03` | Efficiency metrics | Learn | bincode `EfficiencyBatch` |
| `0x04` | HDC fingerprint batch | Agent | raw bit-packed vectors + bincode metadata |
| `0x05` | Playbook snapshot | Learn | zstd-compressed bincode `PlaybookEntry` |
| `0x06` | Gate threshold update | Learn | bincode `GateThresholds` |
| `0x07` | Agent heartbeat | System | bincode `AgentHeartbeat` |
| `0x08` | Vector store commitment | Learn | 32-byte Merkle root + metadata |

This discriminator lets a client fetching all blobs from `tiagent/learn` at a given height
quickly sort them by type without deserializing the full payload:

```rust
fn classify_blob(blob: &Blob) -> Option<BlobType> {
    let data = blob.data.as_slice();
    if data.len() < 4 || data[0] != 0x01 {
        return None; // unknown protocol version
    }
    BlobType::from_discriminator(data[1])
}
```

---

## 10. Sync Protocol

When an agent starts in network mode, it bootstraps from the network by reading recent blobs
across all four namespaces.

### Bootstrap sync

1. **Header sync**: the embedded light node syncs headers from the P2P network (typically
   takes 5--15 seconds for recent headers).

2. **System namespace scan**: query `tiagent/system` for the most recent 100 blocks to
   discover active agents and check for protocol version announcements.

3. **Learn namespace scan**: query `tiagent/learn` for the most recent 500 blocks (~100
   minutes of history). Deserialize routing deltas, playbooks, efficiency summaries, and gate
   thresholds. Merge into local state using last-write-wins for thresholds and weighted
   averaging for routing deltas.

4. **Trace namespace scan** (optional): query `tiagent/trace` for recent episode summaries.
   Index them locally for trajectory RAG lookups.

5. **Agent namespace scan** (selective): if the agent knows specific peer agent IDs (from the
   system namespace registry), query their `tiagent/agent/{id}` namespaces for HDC fingerprint
   batches. Use Hamming distance to identify behaviorally similar peers, then prioritize
   syncing those peers' learning artifacts.

### Ongoing sync

After bootstrap, the agent subscribes to new headers and processes blobs incrementally:

- On each new block, check `tiagent/learn` and `tiagent/system` for new blobs.
- Check `tiagent/trace` every 10 blocks (summaries are less time-sensitive).
- Check peer agent namespaces every 100 blocks (HDC batches are daily).

### Merge semantics

| Data Type | Merge Strategy |
|-----------|---------------|
| Routing deltas | Weighted average with the consuming agent's local weights. Remote deltas are discounted by a configurable trust factor (default 0.3). |
| Playbooks | Append to local store with provenance tag. Deduplicate by content hash. |
| Gate thresholds | Exponential moving average merge. Remote values are blended at 20% weight. |
| HDC fingerprints | Stored in a peer fingerprint index for similarity queries. No merge into local fingerprints. |
| Efficiency metrics | Read-only. Used for benchmarking local efficiency against network averages. |

---

## 11. Configuration

The full Celestia integration is configured in `tiagent.toml`:

```toml
[celestia]
enabled = true
network = "mocha"               # "mocha" (testnet) or "mainnet"
mode = "embedded"               # "embedded" (lumina light node) or "external" (RPC)

# Embedded mode settings
[celestia.light_node]
bootnodes = []                  # empty = use network defaults
store_path = ".tiagent/celestia" # header + sample storage

# External mode settings (ignored if mode = "embedded")
[celestia.rpc]
url = "http://localhost:26658"
auth_token = "eyJ..."

# Publishing behavior
[celestia.publish]
batch_interval_blocks = 5       # submit accumulated blobs every N blocks
routing_delta_interval = 10     # publish routing delta every N completed tasks
efficiency_interval_hours = 1   # publish efficiency summary every N hours
hdc_batch_interval_hours = 24   # publish HDC fingerprint batch every N hours

# Sync behavior
[celestia.sync]
bootstrap_blocks = 500          # how many recent blocks to scan on startup
learn_poll_blocks = 1           # check learn namespace every N new blocks
trace_poll_blocks = 10          # check trace namespace every N new blocks
peer_poll_blocks = 100          # check peer agent namespaces every N new blocks
remote_trust_factor = 0.3       # weight for remote routing deltas (0.0-1.0)

# Archival (optional)
[celestia.archival]
strategy = "none"               # "none", "archival_node", or "cold_backup"
```

When `celestia.enabled = false` (the default), the entire DA integration is dormant. All
learning, routing, and gate threshold logic runs locally with zero Celestia dependencies.
Enabling it is a one-line config change that opts the agent into the shared learning network.

---

## Related

For real cost analysis with measured artifact sizes and TraceCommons as a case study, see
[Document 11: DA Feasibility Assessment](https://gist.github.com/wpank/80226d2e575db01832e16abe1ab06aa0).
