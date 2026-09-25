# MEV Protection

> **v3 depth file** -- `/docs/v3/depth/12-safety/mev-protection.md`
> Canonical source: v1 `docs/v1/11-safety/10-mev-protection.md`
> Status: **Design only**. Chain domain is deprecated for production. The general
> pattern of pre-flight simulation, private submission, and slippage bounds applies
> to any domain where adversarial ordering can extract value from the agent's actions.

---

## 1. The General Pattern

Maximal Extractable Value (MEV) is profit that adversaries extract by observing,
reordering, or front-running an agent's actions. While the term originates in
blockchain (Daian et al., "Flash Boys 2.0," IEEE S&P 2020), the pattern generalizes:

| Domain | MEV analogue | Adversary |
|---|---|---|
| Chain | Sandwich attacks, front-running | Block proposers, searchers |
| Marketplace | Price manipulation before bidding | Competing bidders with visibility |
| API calls | Rate-limit exhaustion, priority queue abuse | Concurrent agents |
| Publishing | Pre-empting announcements | Competitors monitoring drafts |

The defenses are structurally similar across domains:

1. **Pre-flight simulation**: estimate the cost of adversarial interference before
   committing to the action.
2. **Private submission**: avoid broadcasting intent before execution.
3. **Slippage bounds**: set maximum acceptable deviation so the action reverts if
   adversarial interference is detected.
4. **Timing strategies**: batch actions, add random delays, or execute during
   low-competition windows.

---

## 2. Chain Domain: Attack Taxonomy

### 2.1 Sandwich attacks

The most common MEV attack. The attacker observes a pending swap, places a buy order
before it (front-run) and a sell order after it (back-run), profiting from the price
impact.

### 2.2 Front-running

Pure front-running without a corresponding back-run. The attacker copies a profitable
transaction and submits it with higher priority.

### 2.3 JIT (Just-In-Time) liquidity

The attacker provides concentrated liquidity in the exact range a swap will traverse,
earns fees, and removes liquidity in the same block.

### 2.4 Cyclic arbitrage

Multi-hop trades through multiple pools that start and end with the same token,
profiting from price discrepancies.

---

## 3. Detection Algorithms

### 3.1 Sandwich detection

Pattern: three transactions on the same pool where `tx_a` (attacker buy) precedes
`tx_b` (victim) precedes `tx_c` (attacker sell), all with the same pool address.

```rust
pub struct SandwichBundle {
    pub attacker: Address,
    pub frontrun_tx: TxHash,
    pub victim_tx: TxHash,
    pub backrun_tx: TxHash,
    pub pool: Address,
    pub estimated_profit: U256,
    pub victim_impact_bps: u32,
}
```

### 3.2 JIT liquidity detection

Within the same block and pool: address X adds liquidity, a large swap executes
through that range, address X removes liquidity.

### 3.3 Back-run detection

A swap by a known bot in the opposite direction, within 2 transaction indices of a
large target swap on the same pool.

---

## 4. Protection Strategies

### Pre-flight simulation

Before submitting any action, simulate against current state. The simulation detects
expected impact, cost relative to expected return, and whether the action creates an
exploitation opportunity.

### Private submission

Use private channels (Flashbots Protect for Ethereum, or equivalent private queues for
non-chain domains) to avoid broadcasting intent.

### Slippage bounds

Set maximum acceptable deviation. For chain swaps: `amountOutMinimum`. For general
actions: define rollback conditions before execution.

### Timing strategies

- **Batch**: group related actions to reduce per-action exposure.
- **Random delay**: break detectable patterns.
- **Off-peak**: execute non-urgent actions during low-competition periods.

---

## 5. Integration with the Safety Pipeline

MEV detection integrates as a pre-execution Gate. Before any action with adversarial
ordering risk, the Gate runs simulation and checks whether the expected impact exceeds
the configured threshold.

Gate verdicts are persisted as part of the Decision vertex in the Witness DAG (see
`witness-dag.md`), enabling forensic replay of why an action was submitted or rejected.

---

## 6. MEV as Intelligence Signal

MEV patterns reveal domain microstructure:

- High adversarial activity on a resource indicates it is heavily monitored.
- Frequent exploitation between venues indicates close correlation.
- Gas/priority price spikes correlate with competitive opportunities.

The agent can learn from MEV patterns over time, adapting its strategy to avoid
heavily monitored resources and exploit informational advantages.

---

## Academic References

| Paper | Contribution |
|---|---|
| Daian et al. (IEEE S&P 2020, arXiv:1904.05234) | Flash Boys 2.0: foundational MEV framework |
| Qin et al. (IEEE S&P 2022) | Quantifying Blockchain Extractable Value |
| Zust et al. (ETH Zurich, 2021) | 525,004 sandwich attacks, 57,493 ETH extracted |
| Milionis et al. (2022) | LVR: Loss-Versus-Rebalancing for LP risk |
| Flashbots (2021) | MEV-Protect: private transaction submission |

---

## Implementation References

| Component | Location |
|---|---|
| Chain primitives | `crates/roko-chain/` |
| Gate pipeline | `crates/roko-gate/` |
| Adaptive risk Layer 5 | `crates/roko-agent/src/safety/risk.rs` |
