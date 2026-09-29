# Knowledge Transfer via Mesh

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/knowledge-transfer-via-mesh.md`
> Canonical source: v1 `docs/v1/17-lifecycle/09-knowledge-transfer-via-mesh.md`
> Status: **Current** (mesh knowledge sync live via `roko knowledge sync`;
> E28 agent groups 8/8; E29 connectivity and relay scoped runtime complete)

---

## 1. Purpose

While backup/restore handles knowledge transfer between a deleted agent and
its successor, the coordination mesh enables **live knowledge sharing between
running agents**. Agents in the same group can exchange Signals in real time
through the relay infrastructure.

Three knowledge sharing modes:

1. **Group sync**: Bidirectional sync between agents in the same group.
2. **P2P Signal sharing**: Direct agent-to-agent knowledge transfer.
3. **Public knowledge feeds**: Subscribe to curated Signal streams from other
   groups.

---

## 2. Group Knowledge Sharing

A group is a set of agents organized under a common owner or purpose. The
legacy system called this a "Clade" -- roko uses "group" with explicit
membership and permissions (E28).

### Configuration

```toml
[mesh]
enabled = true
relay_url = "wss://mesh.roko.dev/v1/ws"
collective_id = "my-team"

[mesh.sharing]
share_types = ["insight", "warning", "causal_link"]
min_share_confidence = 0.5
share_on_gate_pass = true
received_confidence_discount = 0.7
max_received_per_hour = 100
sync_interval_secs = 300
```

### Delta Sync Protocol

The sharing protocol uses version-vector-based delta sync (Lamport 1978,
Fidge 1988). Each agent maintains a version vector tracking the highest
sequence number received from each peer:

```rust
pub type VersionVector = HashMap<String, u64>;

pub struct SyncDelta {
    pub source_agent: String,
    pub signals: Vec<SharedSignal>,
    pub version_vector: VersionVector,
    pub timestamp: u64,
}

pub struct SharedSignal {
    pub signal: BackupSignal,
    pub seq: u64,
    pub shared_by: String,
    pub shared_at: u64,
    pub attestation: Option<Attestation>,
}
```

### Bloom Filter Discovery

Before requesting full Signal content, agents exchange Bloom filters to
discover which knowledge exists across the group. This prevents redundant
transfers:

1. Agent A sends its Bloom filter (covering all Signal hashes) to the relay.
2. Agent B receives A's filter and checks which of its Signals are novel.
3. B sends only the novel Signals to A.

---

## 3. Affect-Driven Sharing Thresholds

The affect engine's PAD (Pleasure-Arousal-Dominance) state modulates sharing
behavior. High arousal (from resource pressure, deadline proximity, or task
difficulty) increases sharing frequency and lowers the confidence threshold:

```rust
pub fn sharing_threshold(base_threshold: f64, pad: &PADVector) -> f64 {
    let arousal_modifier = -pad.arousal * 0.15;   // +/-0.15
    let dominance_modifier = pad.dominance * 0.10;  // +/-0.10
    (base_threshold + arousal_modifier + dominance_modifier).clamp(0.1, 0.9)
}
```

### Behavioral State Sharing Patterns

| State | Sharing Behavior | Rationale |
|-------|-----------------|-----------|
| Engaged | Standard: share at base threshold | Normal operation |
| Struggling | Increased: lower threshold by 15% | Need help, share partial findings |
| Coasting | Reduced: raise threshold by 10% | Low urgency, share only high-quality |
| Exploring | Increased: lower threshold by 20% | Discovering new knowledge |
| Focused | Reduced: raise threshold by 15% | Deep work, avoid distraction |
| Resting | Minimal: share only Warnings | In consolidation, not producing |

---

## 4. Receiving Knowledge from Mesh

Incoming Signals go through the same quarantine-validate-adopt pipeline used
for backup restore (see `selective-restore.md`), with additional mesh-specific
checks:

1. **Rate limiting**: Enforce `max_received_per_hour`.
2. **Attestation verification**: Verify Ed25519 signature if present.
3. **Reputation check**: Filter by sender reputation if enabled.
4. **Confidence discount**: Multiply incoming confidence by
   `received_confidence_discount` (default: 0.7).
5. **Quarantine and validate**: Same pipeline as restore.
6. **Add provenance**: Tag with source agent, group, and timestamp.
7. **Adopt**: Insert into knowledge store.

### Confidence Discount

Received Signals always have their confidence multiplied by the discount
factor (default: 0.7). This is the mesh equivalent of generational confidence
decay -- knowledge from another agent is treated with appropriate skepticism
until independently validated.

---

## 5. P2P Knowledge Transfer

Beyond group sync, agents can share knowledge directly via peer-to-peer mesh
connections:

```rust
pub struct KnowledgeRequest {
    pub target_agent: String,
    pub query: KnowledgeQuery,
    pub max_results: u32,
    pub offer: Option<Vec<String>>,  // Signal hashes to offer in return
}

pub enum KnowledgeQuery {
    Semantic(String),                              // Keyword/topic search
    HdcSimilarity { vector: Vec<u8>, threshold: f64 }, // HDC similarity
    ByType(SignalKind),                            // Knowledge type filter
    ByTag(String),                                 // Tag-based search
}
```

Use cases for P2P transfer:
- Targeted knowledge acquisition (A asks B for specific expertise).
- Cross-domain insight resonance (structural analogies via HDC similarity).
- Cooperative task completion (sharing task-relevant knowledge during
  collaboration).

---

## 6. Stigmergy: Indirect Coordination

The mesh supports stigmergic coordination -- agents indirectly coordinate by
modifying their shared knowledge environment, rather than through direct
message passing. This is grounded in Grasse's observation of termite
coordination (Grasse 1959): individual termites deposit pheromones that
modify the environment, and subsequent termites respond to the modified
environment.

In roko, the "pheromones" are typed Signals with specific decay profiles:

| Signal subtype | Decay profile | Purpose |
|---------------|--------------|---------|
| Threat | Fast | Immediate danger warnings |
| Opportunity | Moderate | Discovered opportunities |
| Wisdom | Slow | Validated long-term knowledge |
| Anomaly | Variable | Unusual patterns requiring investigation |

Agents reading the shared knowledge space respond to accumulated patterns --
a concentration of Threat Signals in a domain triggers increased caution
across the group without explicit commands.

---

## 7. C-Factor: Collective Intelligence

The C-Factor measures whether a group performs better than the sum of its
parts:

```
C-Factor = Group Performance / Sum(Individual Performances)
```

C-Factor > 1.0 indicates superlinear collective intelligence (Woolley et al.
2010). This metric is tracked across mesh-connected groups and drives
calibration of knowledge sharing parameters.

---

## 8. Permissioned Subnets

Operators can create permissioned subnets -- private groups with restricted
membership:

```toml
[mesh]
collective_id = "company-private"
collective_type = "permissioned"
allowed_operators = ["0x...", "0x..."]
private = true
```

Knowledge shared within a private subnet is never propagated to the public
mesh. The relay enforces access control based on agent attestation and
operator signatures.

---

## 9. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `MeshConfig` | `crates/roko-agent/src/lifecycle.rs` | Sharing configuration |
| `MeshSharingConfig` | `crates/roko-agent/src/lifecycle.rs` | Sharing policy |
| Agent groups (E28) | `crates/roko-core/src/` | Group membership, permissions |
| Relay (E29) | `crates/roko-core/src/` | Bounded delivery, cursor restore |
| `roko knowledge sync` | `crates/roko-cli/src/main.rs` | CLI sync command |

---

## Cross-References

- [selective-restore.md](selective-restore.md) -- Offline knowledge transfer via backup
- [ebbinghaus-for-knowledge.md](ebbinghaus-for-knowledge.md) -- Decay on shared knowledge
- [academic-foundations.md](academic-foundations.md) -- Stigmergy, collective intelligence
