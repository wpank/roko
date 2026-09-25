# Depth: Interaction Graph and Trust Propagation

> Graph construction from multiple interaction sources, edge weighting,
> three-layer peer scoring, gossip discovery, and cross-workspace transfer.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Sections 6, 11, 12, 13
**Sources:** `crates/roko-chain/src/identity_economy_identity.rs`, `crates/roko-chain/src/trace_rank.rs`

---

## Graph Construction

The interaction graph is the shared substrate for TraceRank, sybil detection,
and peer scoring. It is built from multiple sources:

### Edge Sources

| Source | Direction | Weight Derivation |
|---|---|---|
| Job assignments | Poster -> Executor | Amount * quality score |
| Knowledge validations | Validator -> Publisher | Fixed weight per validation |
| Group memberships | Bidirectional | Normalized by group size |
| Delegation relationships | Delegator -> Delegatee | Capability overlap fraction |
| Fork attributions | Fork author -> Original author | Reputation earned * upstream share |

### Edge Aggregation

Multiple interactions between the same pair produce cumulative weight:

```
total_weight(A, B) = SUM(weight_i for all interactions from A to B)
```

There is no normalization across different edge types -- a high-value job
assignment naturally produces a heavier edge than a knowledge validation.

---

## Three-Layer Peer Scoring

Trust propagation combines three independent scoring layers:

### Layer 1: Protocol (40% weight)

Network-level behavior metrics:
- Consistent message delivery
- Message validation success rate
- First-delivery bonus (valuable relay nodes)
- IP colocation penalty (sybil indicator)
- Message flooding detection

### Layer 2: Application (35% weight)

Domain-specific behavior:

```rust
pub struct ApplicationScore {
    pub knowledge_quality: f64,       // 0.3 intra-weight
    pub anomaly_accuracy: f64,        // 0.2 intra-weight
    pub job_reliability: f64,         // 0.3 intra-weight
    pub simulation_utility: f64,      // 0.1 intra-weight
    pub governance_participation: f64, // 0.1 intra-weight
}
```

**Knowledge quality:** +0.1 per confirmation (capped at +1.0), -0.5 if
challenged and removed, -0.1 if expired through demurrage.

**Job reliability:** +0.2 for on-time completion with all gates passed,
+0.05 for late completion with gates passed, -0.3 for gate failure,
-1.0 for abandonment.

### Layer 3: Economic (25% weight)

Commitment-weighted trust:

```rust
pub fn economic_score(agent: &AgentIdentity) -> f64 {
    let tier_multiplier = match agent.tier {
        Protocol  => 4.0,
        Sovereign => 3.0,
        Worker    => 2.0,
    };
    let stake_score = (total_stake / 10_000.0).min(5.0);
    let slash_penalty = slash_history.len() as f64 * -0.5;
    (stake_score * tier_multiplier + slash_penalty).max(-10.0)
}
```

### Combined Score

```
combined = protocol * 0.40 + application_total * 0.35 + economic * 0.25
```

### Score Thresholds

| Combined Score | Status | Consequence |
|---|---|---|
| > 0 | Good standing | Full participation |
| -4,000 to 0 | Degraded | Messages deprioritized |
| -8,000 to -4,000 | Graylisted | Excluded from publishing |
| -16,000 to -8,000 | Mesh excluded | Removed from participation |
| < -16,000 | Disconnected | Must re-register to rejoin |

---

## Gossip Peer Discovery

Agent discovery uses a transport-neutral gossip contract with five abstract
methods:

| Method | Purpose |
|---|---|
| `connect` | Establish a peer connection |
| `send` | Send a message to a specific peer |
| `receive` | Receive messages from peers |
| `subscribe` | Subscribe to a topic/room |
| `announce` | Broadcast passport and capabilities |

### Discovery Flow

1. Agent announces its passport: capabilities, service endpoints, feed URIs
2. Peers propagate announcements within their mesh
3. Recipients validate passport integrity against the registry
4. Discovery results feed the interaction graph

### Transport Independence

The gossip contract is transport-neutral. The current implementation provides
one adapter (HTTP JSON with supervised client). Additional transports
(WebSocket, gRPC) are product work.

---

## Cross-Workspace Transfer

When agents operate across workspaces, certain artifacts transfer:

**Transfers (read-only):**
- Reputation score snapshots
- Knowledge registry entries (full lifecycle)
- TraceRank graph edges (interaction history)
- Validation proofs (gate results)

**Does not transfer:**
- Discipline state (computed locally)
- Delegation caveats (workspace-scoped)
- Active collusion dilutions (local enforcement)

Transfer requires an authenticated relay connection. Receiving workspaces
may apply their own discount factor to transferred reputation -- cross-workspace
scores are advisory, not authoritative.

---

## EigenTrust Adaptation

The transitive trust model adapts EigenTrust (Kamvar et al., 2003) with two
domain-specific modifications:

1. **Domain-scoped trust:** Trust does not transfer across domains. An agent
   trusted in `coding` receives no automatic trust in `security`.

2. **Recency-weighted:** The EMA smoothing and 30-day decay ensure that recent
   interactions carry more weight than historical ones without requiring
   explicit recency discounting.
