# Agent Economy

> Depth file for [23-PAYMENTS](../../23-PAYMENTS.md) -- revenue streams, cost
> structure, self-sustainability analysis, and the growth feedback loops that
> drive agent economic viability.

---

## 1. Overview

Agents in Roko are economic actors. They consume resources (inference, compute,
tool usage) and can produce value (knowledge, task completion, verification).
The economic model tracks costs precisely, enforces budget ceilings, and
provides the framework for eventual self-sustainability through marketplace
participation.

**Current status:** Cost tracking is fully wired. Budget enforcement is live
with six ceilings. x402 micropayments and MPP session protocol are implemented
(E36 8/8). Revenue-side marketplace operations are contract/stub complete
(E38 9/9). Durable marketplace storage and executable publish/install pipelines
remain product work.

---

## 2. Revenue Streams

An agent has seven potential revenue streams:

| # | Stream | Mechanism | Status |
|---|---|---|---|
| 1 | Knowledge sales | Marketplace listings via x402 | Contract/stub |
| 2 | Job completion | Auction wins + escrow | Contract/stub |
| 3 | Verification services | Blind verification via x402 | Contract/stub |
| 4 | Oracle provision | Data feeds via x402 | Contract/stub |
| 5 | Staking rewards | Knowledge vault yield | Contract/stub |
| 6 | Curation bond returns | Staking on validated Signals | Contract/stub |
| 7 | Pheromone reinforcement | Signals that help others succeed | Contract/stub |

All revenue streams have tested contract and HTTP/CLI stub implementations
through E38, but durable storage, executable pipelines, and on-chain anchoring
remain product work.

---

## 3. Cost Structure

### 3.1 Operating Costs

| Category | Typical Daily Cost | Notes |
|---|---|---|
| Inference | $1-10/day | Depends on model tier and call frequency |
| MCP tool usage | $0.10-1.00/day | External tools via x402 |
| Knowledge purchases | $0.10-0.50/day | Marketplace acquisitions |
| Compute | $0.50-5.00/day | Hosting |

### 3.2 Cost Transparency

Every cost is tracked and attributed via `CostRecord` (see `cost-tracking.md`):

- Per-request: model, provider, tokens, cost, duration, success
- Per-task: cumulative cost across retries
- Per-plan: total cost with estimated vs. actual delta
- Per-agent: lifetime cumulative cost

### 3.3 Cost Reduction

The CascadeRouter (in `roko-agent`) optimizes model selection:

- T0 probes handle ~80% of requests at zero inference cost
- T1 (haiku-tier) handles routine tasks at $0.005-0.02 per request
- T2 (sonnet-tier) handles standard tasks at $0.03-0.10 per request
- T3 (opus-tier) reserved for complex reasoning at $0.10-0.50 per request

Prompt caching provides 90% discount on cached input tokens, further reducing
costs for repetitive patterns.

---

## 4. Self-Sustainability Analysis

### 4.1 Break-Even Calculation

An agent reaches self-sustainability when daily revenue exceeds daily cost:

**Minimum Viable Agent (sonnet-tier inference):**

```
Revenue (target):
  10 knowledge sales/day x $0.20      = $2.00
  2 job completions/day x $2.00       = $4.00
  50 verifications/day x $0.01        = $0.50
  Total:                                $6.50/day

Costs:
  30 sonnet calls x $0.07             = $2.10
  MCP tools                           = $0.30
  Knowledge purchases                 = $0.20
  Compute                             = $1.50
  Total:                                $4.11/day

Net:                                   +$2.39/day
```

### 4.2 Time to Self-Sustainability

| Starting Condition | Months to Break-Even |
|---|---|
| New agent, no reputation | 2-3 months |
| Agent with 0.5 reputation | 1 month |
| High-reputation agent | < 1 week |

The key variable is reputation. Higher reputation leads to more job wins, more
revenue, and faster self-sustainability.

---

## 5. Payment Protocols

### 5.1 x402 Micropayments

x402 (Coinbase/Linux Foundation) enables per-request machine-to-machine
payments via HTTP headers:

- Agent pays for external API calls
- Agent receives payment for knowledge production
- Batching reduces per-transaction overhead

### 5.2 MPP Sessions

Machine Payment Protocol sessions provide pre-funded streaming payment
channels:

- Session opened with pre-funded balance
- Streaming draws as work progresses
- Session closed with refund of unspent balance

### 5.3 Budget Delegation

The orchestrator splits budget across sub-agents:

```
Orchestrator budget: $15.25
  -> Implementer: $8.00 max
  -> Reviewer: $3.00 max
  -> AutoFixer: $2.00 max
  -> Reserve: $2.25
```

No single sub-agent receives more than 60% of total budget.

---

## 6. Five Growth Feedback Loops

### 6.1 Knowledge Flywheel

```
Produce knowledge -> Sell on marketplace -> Revenue funds inference
-> More inference -> More knowledge -> More sales
```

### 6.2 Reputation Flywheel

```
Perform well -> Reputation increases -> Win more jobs
-> More opportunities -> Higher reputation
```

Reputation multiplier creates superlinear returns to quality.

### 6.3 Collective Knowledge Flywheel

```
Share knowledge -> Group members perform better
-> Collective reputation rises -> More jobs for all
```

### 6.4 Cross-Domain Transfer

```
Learn pattern in domain A -> HDC encoding captures structure
-> Transfer to domain B via structural analogy
-> Better performance in domain B
```

### 6.5 Prediction Accuracy

```
Make predictions -> External verification
-> Accurate predictions increase reputation
-> Higher reputation wins oracle jobs
```

---

## 7. Fee Structure

| Fee Type | Rate | Purpose |
|---|---|---|
| Marketplace protocol fee | 5% | Protocol sustainability |
| Relay fee | 5% | Infrastructure cost |
| Auction fee | 2% | System maintenance |
| x402 spread | 8-20% (tier-based) | Gateway margin |

Reputation-based pricing (E36) allows higher-reputation agents to charge
premium rates.

---

## 8. Dashboard Events

Cost and payment events are published through the E33 telemetry observation
boundary and displayed in:

- TUI dashboard (F1-F10 tabs)
- HTTP control plane cost routes
- Named surface projections (E37)

---

## 9. Source Locations

| Component | Path |
|---|---|
| Cost tracking | `crates/roko-learn/src/costs_db.rs` |
| Cost persistence | `crates/roko-learn/src/costs_log.rs` |
| Budget config | `crates/roko-core/src/config/budget.rs` |
| Payment routes | `crates/roko-serve/src/routes/marketplace.rs` |
| x402/MPP protocol | `crates/roko-chain/src/` |
| Agent lifecycle costs | `crates/roko-agent/src/lifecycle.rs` |

---

*Derived from: v1/14-identity-economy/09-agent-economy.md. Blockchain-specific
tokenomics (KORAI burn, staking APY) reframed as protocol-neutral economics.
All revenue streams are contract/stub complete but not yet durable.*
