# Depth: Stigmergy Scaling

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 7

---

## Overview

A critical question for any multi-agent coordination mechanism: **how does
coordination cost scale with agent count?** This document analyzes the scaling
properties of Roko's stigmergic coordination system and shows that stigmergy
provides fundamentally better scaling than direct communication alternatives.

---

## Coordination Cost Models

### Direct Communication: O(N^2)

For N agents with point-to-point channels:

- **Channels**: N x (N-1) / 2 = O(N^2)
- **Message volume**: O(N^2) messages per round
- **Leader election**: O(N^2) or O(N log N) messages

### Stigmergy: O(N x M)

- **N** = number of agents
- **M** = distinct pheromone kinds x scope levels

Since M is bounded and small (7 built-in kinds x 3 scopes = 21 channels, plus
custom kinds), coordination cost grows **linearly** with agent count.

Each agent performs per cycle:

1. **Deposit**: Write pheromone Signals to Substrate -> O(1) per agent
2. **Sense**: Query Substrate for relevant pheromones -> O(M) per agent

Total per cycle: O(N x M) = O(N) for fixed M.

### Comparison

| Agent Count (N) | Direct O(N^2) | Stigmergy O(N x M), M=20 | Ratio |
|----------------|--------------|---------------------------|-------|
| 5 | 10 | 100 | 0.1x (direct wins for tiny N) |
| 20 | 190 | 400 | 0.5x |
| 50 | 1,225 | 1,000 | 1.2x (crossover point) |
| 100 | 4,950 | 2,000 | 2.5x |
| 500 | 124,750 | 10,000 | 12.5x |
| 1,000 | 499,500 | 20,000 | 25x |
| 10,000 | 49,995,000 | 200,000 | 250x |

Crossover point at approximately N = 40 for M = 20.

---

## Pheromone Field Scaling

### Storage Scaling

```
Storage = N_agents x R_deposit x tau_mean / R_gc
```

For a typical 10-agent Collective:

- Each agent deposits ~15 pheromones/day
- Mean half-life: ~8 hours
- After 3 half-lives (~24h), intensity < 12.5% (near GC)
- Steady-state: ~10 x 15 x 1.0 = **~150 active pheromones**

At 1,000 agents: ~15,000 active pheromones -- manageable with a simple
in-memory store or JSONL file.

### Query Scaling

| Strategy | Complexity | When to use |
|----------|-----------|-------------|
| Naive scan | O(P) | P < 100,000 (~1ms for 15,000) |
| Indexed scan | O(log P) | P > 100,000 |
| Approximate (Bloom filters) | O(1) | Cross-Collective queries |

### Decay as Natural GC

Exponential decay provides automatic garbage collection:

1. **No unbounded growth**: Field bounded by `N x R_deposit x tau_mean`
2. **Self-healing**: Noise decays; only confirmed signals persist
3. **No compaction**: Field self-compacts through decay

---

## Channel Capacity of the Pheromone Field

The effective channel capacity -- maximum rate at which coordination
information can be reliably transmitted -- is bounded by field saturation,
decay rate, and confirmation overhead:

```
C_stigmergy = M x R_gc x log_2(1 + I_signal / I_noise)
```

Where:
- `M` = number of distinguishable pheromone kinds (channel multiplexing)
- `R_gc` = effective garbage collection rate (channel clearing)
- `I_signal` = intensity of target pheromone
- `I_noise` = aggregate intensity of non-target pheromones

```rust
/// Estimate effective information throughput of a pheromone field.
///
/// Models the field as a multi-channel communication medium where each
/// PheromoneKind is an independent sub-channel.
///
/// Returns bits per tick of coordination information capacity.
pub fn stigmergic_channel_capacity(
    kind_count: usize,
    avg_signal_intensity: f64,
    avg_noise_intensity: f64,
    gc_rate_per_tick: f64,
) -> f64 {
    if avg_noise_intensity <= 0.0 || gc_rate_per_tick <= 0.0 {
        return 0.0;
    }
    let snr = avg_signal_intensity / avg_noise_intensity;
    kind_count as f64 * gc_rate_per_tick * (1.0 + snr).log2()
}
```

### Entropy Rate

The entropy rate measures information content per unit time:

- **Too low**: Static field -- no coordination being generated
- **Too high**: Chaotic field -- signals change too fast
- **Optimal**: Field evolves at rate matched to agent sensing/response

```rust
pub struct FieldEntropyEstimator {
    /// Number of intensity buckets. Default: 10. Range: [4, 100].
    pub intensity_buckets: usize,
    /// Sliding window size. Default: 100 ticks. Range: [20, 1000].
    pub window_size: usize,
}
```

The entropy rate connects to the edge-of-chaos operating point [Langton, C.G.
"Computation at the Edge of Chaos." *Physica D*, 42(1-3):12-37, 1990].

### Transfer Entropy

Quantifies directed information flow between agents through the stigmergic
medium [Schreiber, T. "Measuring Information Transfer." *PRL*, 85(2):461-464,
2000]:

```
T_{A->B} = sum p(b_{t+1}, b_t, a_t) x log_2(p(b_{t+1} | b_t, a_t) / p(b_{t+1} | b_t))
```

- High T_{A->B}: Agent A causally influences Agent B -- stigmergic loop works
- Low transfer entropy in both directions: agents operating independently

---

## Transport Scaling

### WebSocket Relay

| Factor | Scaling |
|--------|---------|
| Connections | O(N) -- one per agent |
| Fan-out | O(N) per message |
| Bandwidth | O(N x R_deposit x S_msg) |
| Latency | O(1) -- constant |

For 50-agent Collective at 15 deposits/day/agent:
- Total: 750 messages/day, fan-out 36,750 deliveries = ~0.4/second
- Bandwidth: ~36 MB/day (trivial)

### Iroh Gossip

| Factor | Scaling |
|--------|---------|
| Connections per node | O(log N) -- HyParView |
| Message delivery | O(N x log N) total |
| Bandwidth per node | O(R_deposit x log N x S_msg) -- bounded |
| Latency | O(log N) hops |

### Morphogenetic Scaling

Role vector propagation: negligible.

```
Bandwidth = N x (64 bytes / 50 ticks) x tick_rate
```

For 20 agents at 4 ticks/minute: ~6 KB/hour.

Inhibition computation: O(N x 8) per agent per update cycle.

| Collective Size | Computation | Time (est.) |
|----------------|------------|-------------|
| 5 | 40 multiply-adds | ~1 us |
| 50 | 400 multiply-adds | ~10 us |
| 500 | 4,000 multiply-adds | ~100 us |
| 5,000 | 40,000 multiply-adds | ~1 ms |

---

## Scaling Limits

| Factor | Practical Limit | Bottleneck |
|--------|----------------|-----------|
| Agents per Collective | ~10,000 | Gossip fan-out latency |
| Pheromone field size | ~1M active | In-memory storage |
| Kind diversity | ~100 custom | Configuration complexity |
| Morphogenetic convergence | ~50 agents | Convergence time > 12h |
| WebSocket connections | ~50,000 | Single relay capacity |

### Mitigation

1. **Partition**: Use permissioned subnets for smaller coordination groups.
2. **Hierarchical aggregation**: Sub-Collective fields summarized for parent.
3. **Gossip over relay**: Switch to Iroh for Collectives > 50 agents.
4. **Morphogenetic partitioning**: Partition by domain for > 50 agents.

---

## Reed's Law Implications

Reed's Law: network value grows as O(2^N) for group-forming networks [Reed,
D.P. "The Law of the Pack." *Harvard Business Review*, 2001].

For Roko Collectives:
- Each agent subset can form a productive sub-group
- Number of subsets grows exponentially with N
- Coordination cost grows only linearly (O(N x M))
- **Value-to-cost ratio grows exponentially**

---

## Design Implications

| Principle | Rationale | Implementation |
|-----------|-----------|----------------|
| Kind multiplexing | Each kind is independent sub-channel | 7 universal + Custom kinds |
| Decay = bandwidth | Faster decay clears channel for new signals | Kind-specific half-lives |
| Confirmation = coding gain | Confirmation acts like error-correcting codes | Reputation-weighted confirmation |
| Scope = frequency reuse | Different scopes reuse kind namespace | Three-level hierarchy |
| Sensing threshold = noise floor | 0.01 threshold filters noise | Configurable per role |
| Field entropy monitoring | Tracks frozen/healthy/chaotic state | FieldEntropyEstimator |

---

## Summary

| Operation | Scaling | Notes |
|-----------|---------|-------|
| Pheromone deposit | O(1) per agent | Constant cost |
| Pheromone sensing | O(M) per agent | M bounded |
| Transport (relay) | O(N) per message | Linear fan-out |
| Transport (gossip) | O(N log N) total | Bounded per-node |
| Morphogenetic update | O(N x 8) per agent | 8 strategy dims |
| Knowledge sync | O(N) total entries | Linear |
| Version vector dedup | O(1) per message | Hash map lookup |

---

## References

- [Langton 1990] Computation at the Edge of Chaos, *Physica D*
- [Parunak 1997] Engineering from natural MAS, *Ann. Oper. Res.*
- [Reed 2001] The Law of the Pack, *Harvard Business Review*
- [Schreiber 2000] Measuring Information Transfer, *PRL*
- [Shannon 1948] Mathematical Theory of Communication, *Bell System Tech. J.*
- [Turing 1952] Chemical Basis of Morphogenesis, *Phil. Trans. Royal Society B*
