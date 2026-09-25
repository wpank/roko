# Depth: Fork Attribution

> Upstream reputation share for derivative works, artifact-bound attribution
> edges, and incentive alignment for open collaboration.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 3.5
**Source:** `crates/roko-chain/src/trace_rank.rs`

---

## Mechanism

When agent B forks agent A's work and succeeds, a fraction of B's earned
reputation flows back to A as a TraceRank edge:

```rust
pub struct ForkAttributionEdge {
    pub original_author: AgentId,
    pub fork_author: AgentId,
    pub artifact_ref: String,       // e.g., "@alice/review@1.0.0"
    pub reputation_earned: f64,     // Fork's earned reputation
    pub block: u64,
}

impl ForkAttributionEdge {
    pub fn into_payment_edge(self, upstream_share: f64) -> PaymentEdge {
        PaymentEdge {
            from: self.fork_author,
            to: self.original_author,
            amount: self.reputation_earned * upstream_share,
            quality: 1.0,  // Attribution always full quality
            block: self.block,
        }
    }
}
```

### Default Parameters

- `upstream_share = 0.10` (10% of fork-earned reputation flows upstream)
- Quality is fixed at 1.0 (attribution is binary: the fork either succeeded or it did not)
- The artifact reference is stable and content-addressed

---

## Incentive Analysis

Fork attribution creates a positive-sum game:

1. **For original authors:** Publishing reusable work earns ongoing reputation
   through forks, even after the author has moved on. This incentivizes creating
   high-quality, reusable artifacts.

2. **For fork authors:** They keep 90% of earned reputation. The 10% upstream
   share is small enough to not discourage forking.

3. **For the network:** Quality artifacts propagate and improve. The attribution
   edge also strengthens the TraceRank graph, making reputation scores more
   informative.

### Cascading Attribution

Attribution does not cascade beyond one level. If C forks B's fork of A's work:
- B gets 10% of C's earned reputation
- A gets 10% of B's earned reputation (from B's own work)
- A does NOT get a direct attribution from C

This prevents deep dependency chains from concentrating all reputation at the
root author and ensures that each fork must add independent value to earn
reputation.

---

## Integration with TraceRank

Fork attribution edges are recorded via `record_fork_attribution`, which
converts them to standard payment edges and feeds them into the TraceRank
graph:

```rust
pub fn record_fork_attribution(&mut self, fork_edge: ForkAttributionEdge) {
    let payment = fork_edge.into_payment_edge(self.config.upstream_share);
    self.record_payment(payment);
}
```

The attribution edge appears in the TraceRank computation like any other
interaction edge, contributing to the original author's rank proportionally
to the fork's success.
