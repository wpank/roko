# Depth: Pheromone Scope

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 3

---

## Overview

Every digital pheromone in Roko has a `PheromoneScope` that determines how far
the signal propagates and who can sense it. The three-level hierarchy is
inspired by the Constructal Law's prediction that optimal flow systems evolve
dendritic (tree-like) structures [Bejan, A. "Constructal-Theory Network."
*Int. J. Heat and Mass Transfer*, 40(4):799-816, 1997].

| Scope | Environment | Audience | Trust | Persistence | Cost |
|-------|-------------|----------|-------|-------------|------|
| `Local(SubstrateId)` | Agent's NeuroStore | Self only | 1.0 | Until GC | Free |
| `Mesh(CollectiveId)` | Collective's Agent Mesh | Collective members | 0.8-0.9 | Hours-days | ~$0.01/day |
| `Global` | Chain (public) | All agents | 0.5-0.7 | Permanent | Chain tx fee |

---

## The PheromoneScope Enum

```rust
/// Propagation scope of a digital pheromone.
///
/// Scopes form a strict hierarchy: Local < Mesh < Global.
/// Information flows upward through promotion gates and downward
/// through queries.
///
/// Three scopes, not two or five, because:
///   Two lacks middle ground for private group coordination
///   Five+ adds complexity without clear benefit
///   Three maps to biological precedent: individual, colony, species
pub enum PheromoneScope {
    /// Visible only within the specified Substrate.
    /// Use for: internal state, work-in-progress, hypotheses.
    Local(SubstrateId),

    /// Propagates to all agents in the specified Collective.
    /// Use for: coordination signals, shared alerts, morphogenetic signals.
    Mesh(CollectiveId),

    /// Published to chain for global visibility.
    /// Use for: ecosystem-wide alerts, validated research, public signals.
    Global,
}
```

---

## Local Scope: The Agent's Private Field

### Characteristics

| Property | Value |
|----------|-------|
| Audience | Self only |
| Transport | None (stored in NeuroStore) |
| Trust multiplier | 1.0 (self-generated, maximum trust) |
| Persistence | Until garbage collection |
| Decay | Standard exponential per `PheromoneKind` |
| Cost | Free (local I/O only) |

### Use Cases

| Use Case | PheromoneKind | Description |
|----------|--------------|-------------|
| Working hypothesis | Pattern | "I think this module has a dependency cycle" |
| Internal bookmark | Opportunity | "This function could be optimized" |
| Personal threat note | Threat | "This test is flaky -- saw it fail once" |
| Draft insight | Wisdom | "NaN handling is a recurring issue" (not yet confirmed) |

### Promotion to Mesh Scope

Promotion requires active decision by the agent:

1. **Confidence threshold**: The pheromone has been locally confirmed (agent
   observed the pattern multiple times).
2. **Relevance**: The signal is relevant to other Collective members.
3. **Novelty**: No existing Mesh-scope pheromone covers the same observation.

Inherited knowledge (received from Mesh scope) starts at reduced confidence
(x0.80), ensuring each agent independently validates shared signals before
relying on them -- the Weismann barrier principle [Heard, E. & Martienssen,
R.A. "Transgenerational Epigenetic Inheritance." *Cell*, 157(1), 2014].

---

## Mesh Scope: The Collective's Shared Field

### Characteristics

| Property | Value |
|----------|-------|
| Audience | All agents in the Collective |
| Transport | Agent Mesh: WebSocket relay and/or Iroh P2P |
| Trust multiplier | 0.80-0.90 |
| Persistence | Until decay + configurable store-and-forward TTL (7d default) |
| Cost | ~$0.01/day for typical sync volume (5-25 entries/agent/day) |

### Transport Layer

| Transport | Mechanism | Latency | Offline Handling |
|-----------|-----------|---------|-----------------|
| WebSocket | Relay through Agent Mesh server | ~50ms | Store-and-forward (7-day TTL) |
| Iroh | Direct P2P via QUIC + NAT traversal | ~10ms (LAN), ~100ms (WAN) | Relay fallback |

Deduplication via version vectors (`{agent_id -> last_seen_seq}`) ensures
pheromones received via both transports are processed only once.

### Sync Triggers

1. **Event-driven (immediate)**: High-priority signals (`Threat`, high-intensity
   `Anomaly`) push immediately. Latency: milliseconds.
2. **Curator-aligned (batch, every 50 ticks)**: Lower-priority signals
   (`Pattern`, `Opportunity`, `Wisdom`) are batched after Curator validation.
3. **On-demand**: Full sync at any time (e.g., after boot or recovery).

### Confidence Discounting

Received pheromone intensity is discounted by trust multiplier:

```
received_intensity = original_intensity x trust_multiplier
```

| Relationship | Trust Multiplier |
|-------------|-----------------|
| Self (own agent) | 1.00 |
| Collective member (sibling) | 0.80 |
| Cross-collective (marketplace) | 0.60 |
| Anonymous (public) | 0.50 |

### Morphogenetic Signals at Mesh Scope

Mesh scope carries morphogenetic specialization signals:

- **Role vectors**: 8-dimensional strategy vectors broadcast every 50 ticks
- **Inhibition signals**: Computed locally from aggregated role vectors
- **Role conflict alerts**: Pushed immediately when two agents' vectors have
  cosine similarity > 0.9 for 100+ consecutive ticks

---

## Global Scope: The Ecosystem's Public Field

### Characteristics

| Property | Value |
|----------|-------|
| Audience | All agents on the network |
| Transport | On-chain transaction |
| Trust multiplier | 0.50-0.70 |
| Persistence | Permanent (on-chain storage) |
| Decay | Relevance-based, not time-based |
| Cost | Chain transaction fee |

### Governance Rules

1. **Staking requirement**: Only Tier 2+ agents can deposit Global pheromones.
2. **Reputation gate**: Minimum reputation score 0.7+ in relevant domain.
3. **Quality validation**: On-chain validators check format and content.
4. **Spam prevention**: Rate limits per agent per epoch.

### When to Use Each

| Criterion | Use Mesh | Use Global |
|-----------|----------|-----------|
| Audience | My Collective | Everyone |
| Sensitivity | Private/competitive | Public benefit |
| Persistence | Hours-days | Permanent |
| Cost tolerance | Low | Higher (chain fee) |
| Validation level | Collective confirmation | Ecosystem-wide |

---

## Scope Hierarchy and Information Flow

### Upward Flow (Promotion)

```
Local -> Mesh: Agent promotes after gaining confidence
   Gate: confidence >= 0.6, self-confirmed >= 2 times
   Effect: signal visible to Collective members

Mesh -> Global: Collective promotes after consensus
   Gate: confirmations >= 4, collective agreement
   Effect: signal visible to all agents on chain
```

### Downward Flow (Query)

```
Global -> Mesh: Agent queries chain for relevant public signals
   Filter: domain, kind, minimum intensity

Mesh -> Local: Agent receives Collective signals via sync
   Filter: relevance to current task, intensity threshold
```

### Cross-Scope Composition

When the Composer assembles context, it queries all three scopes and merges:

```rust
fn assemble_pheromone_context(
    local_substrate: &dyn Substrate,
    mesh_substrate: &dyn Substrate,
    global_substrate: &dyn Substrate,
    filter: &PheromoneFilter,
    budget: usize,
) -> Vec<ScoredPheromone> {
    let mut all = Vec::new();

    all.extend(local_substrate.query(filter)?);
    all.extend(mesh_substrate.query(filter)?
        .into_iter()
        .map(|p| p.with_trust_discount(0.80)));
    all.extend(global_substrate.query(filter)?
        .into_iter()
        .map(|p| p.with_trust_discount(0.60)));

    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    all.truncate(budget);
    all
}
```

---

## Configuration

```toml
[pheromone.scope]
default_scope = "local"  # "local", "mesh", or "global"

[pheromone.scope.local]
gc_threshold = 0.001
max_pheromones = 10000

[pheromone.scope.mesh]
enabled = true
collective_id = "my-collective"
sync_interval_ticks = 50
immediate_threshold = 0.7
trust_multiplier = 0.80

[pheromone.scope.global]
enabled = false
min_reputation = 0.7
rate_limit_per_epoch = 10
```

---

## Summary

The three-scope system provides a complete coordination hierarchy:

1. **Local**: Private working memory -- fast, free, no propagation
2. **Mesh**: Collective coordination -- moderate cost, high trust, real-time
3. **Global**: Ecosystem intelligence -- permanent, public, on-chain

Information flows upward through promotion gates (increasing audience and
persistence) and downward through queries (increasing specificity). The trust
multiplier ensures inherited signals are discounted until independently
validated.

---

## References

- [Bejan 1997] Constructal Law, *Int. J. Heat and Mass Transfer*
- [Fidge 1988] Timestamps in Message-Passing Systems, *ACSC*
- [Heard & Martienssen 2014] Transgenerational Epigenetic Inheritance, *Cell*
- [Lamport 1978] Time, Clocks, and Events, *CACM*
- [Roediger & Karpicke 2006] Test-Enhanced Learning, *Psychological Science*
