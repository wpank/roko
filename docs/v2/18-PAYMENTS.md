# 18 -- Payments

> Two payment protocols (x402 per-request, MPP session-based), reputation-based pricing with 5 tiers, feed marketplace economics, and relay payment flow. The current HTTP payment boundary is implemented in `roko-serve`; protocol-native Verify Cells remain the target architecture.

> **Implementation status (E36, verified 2026-08-15):** payment domain types, feed pricing metadata, x402 challenge/authorization gating for paid feed reads, settlement batching, MPP accounting, reputation tier resolution, payment cost persistence, and dashboard event contracts are implemented. Cryptographic signature verification, on-chain batch submission, MPP HTTP/WS delivery, and concrete `VerifyX402Cell` / `VerifyMppCell` implementations remain planned.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal), [02-CELL](02-CELL.md) (Verify protocol), [11-CONNECTIVITY](11-CONNECTIVITY.md) (Relay, feeds), [17-AUTH](17-AUTH.md) (agent bearer tokens)

---

## 1. Overview

Roko models two payment protocols for paid feeds and agent services. x402 has a live HTTP guard on paid `GET /api/feeds/{id}` reads. MPP currently has an in-memory accounting manager but is not connected to an HTTP/WS subscription route. Both are intended to become Verify Cells in the feed subscription pipeline.

| Protocol | Model | Signing | Settlement | Use case |
|---|---|---|---|---|
| **x402** | Per-request, stateless | ERC-3009 per request | Batch (10min or 100+ auths) | On-demand queries, trying a feed |
| **MPP** | Session-based, streaming | One ERC-3009 per session | On session close/expire | Continuous feeds, multi-agent pipelines |

### 1.1 Shipped implementation

| Area | Current implementation | Boundary |
|---|---|---|
| Domain model | `PricingTier`, `PaymentProtocol`, `SessionPricing`, and `FeedPricingConfig` in `roko-core::feed` | Pure serializable data; no chain dependency |
| Feed metadata | Optional, backward-compatible `FeedInfo.pricing` | Older JSON without `pricing` still deserializes |
| x402 HTTP gate | `roko-serve::routes::middleware::require_payment` | Missing, malformed, or underfunded authorization returns HTTP 402 with a `PaymentRequest` body and `X-Payment-Request` header |
| x402 accounting | `X402Manager`, `SettlementBatch` | Verifies amount, recipient, validity window, and nonce reuse before queueing; drains at 100 authorizations or the configured timeout |
| MPP accounting | `MppSessionManager` | Opens sessions, meters capped usage, detects expiry, and computes final settlement; no transport or chain submission yet |
| Reputation pricing | `resolve_pricing_tier` and `PricingTierResult` | Mean of all seven effective reputation domains plus discipline gating |
| Cost persistence | `PaymentCostRecord` and `PaymentSummary` in `CostsDb` | Payment records use a separate lock/vector; mixed LLM/payment JSONL is type-tagged with legacy LLM import support |
| Dashboard state | `PaymentReceived`, `SettlementCompleted`, and payment counters | Event serialization and snapshot accumulation are implemented; dedicated payment UI remains planned |

### 1.2 Target: payment as Verify Cells

The following is target architecture, not shipped Rust. Concrete structs conforming to `Cell + VerifyProtocol` would move the current HTTP/accounting boundaries into the feed subscription graph so every delivery passes through the appropriate payment Verify Cell.

```rust
/// x402 per-request payment verification.
/// Sits in the feed subscription pipeline. Rejects requests
/// that lack a valid ERC-3009 authorization header.
pub struct VerifyX402Cell {
    /// Pending authorizations waiting for batch settlement.
    settlement: X402Settlement,
}

impl Cell for VerifyX402Cell {
    fn name(&self) -> &str { "verify-x402" }
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Verify] }
}

#[async_trait]
impl VerifyProtocol for VerifyX402Cell {
    async fn verify_pre(&self, signal: &Signal, ctx: &CellContext) -> Result<Verdict> {
        // Extract X-Payment header from the request Signal
        let auth = match signal.metadata.get("x_payment") {
            Some(header) => parse_erc3009_authorization(header)?,
            None => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentMissing,
                reason: "No X-Payment header. Send GET first to receive 402 with payment terms.".into(),
            }),
        };

        // Verify ERC-3009 signature locally via ecrecover (no RPC needed)
        if !verify_erc3009_signature(&auth)? {
            return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentInvalid,
                reason: "ERC-3009 signature verification failed.".into(),
            });
        }

        // Check amount >= required price (with reputation tier discount)
        let pricing = ctx.store().get::<ReputationPricing>(&auth.sender).await?;
        if auth.amount < pricing.effective_price() {
            return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentInsufficient,
                reason: format!("Amount {} < required {}", auth.amount, pricing.effective_price()),
            });
        }

        // Check nonce and expiry
        if auth.expiry < now_unix() {
            return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentExpired,
                reason: "Authorization expired.".into(),
            });
        }

        Ok(Verdict {
            passed: true,
            reward: 1.0,
            evidence: Evidence::PaymentVerified { protocol: "x402", amount: auth.amount },
            reason: "x402 payment verified.".into(),
        })
    }

    async fn verify_post(&self, _signal: &Signal, _output: &Signal, _ctx: &CellContext) -> Result<Verdict> {
        // Post-verification: collect the authorization for batch settlement
        // (handled in execute() after verify_pre passes)
        Ok(Verdict::pass())
    }
}

impl VerifyX402Cell {
    async fn execute(&mut self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        let auth = parse_erc3009_authorization(input[0].metadata.get("x_payment").unwrap())?;

        // Collect for batch settlement
        self.settlement.pending.push(auth);

        // Check if batch settlement should fire
        if self.settlement.should_settle() {
            if let Some(chain) = ctx.chain_client() {
                self.settlement.settle(chain).await?;
            }
        }

        // Pass through: return the feed data Signal
        Ok(input)
    }
}

/// MPP session-based payment verification.
/// Sits in the feed subscription pipeline. Verifies that the subscriber
/// has an active MPP session with sufficient balance for the draw.
pub struct VerifyMppCell {
    /// Active sessions indexed by session_id.
    sessions: HashMap<SessionId, MppSession>,
}

impl Cell for VerifyMppCell {
    fn name(&self) -> &str { "verify-mpp" }
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Verify] }
}

#[async_trait]
impl VerifyProtocol for VerifyMppCell {
    async fn verify_pre(&self, signal: &Signal, ctx: &CellContext) -> Result<Verdict> {
        // Extract session_id from the subscription request
        let session_id = match signal.metadata.get("mpp_session_id") {
            Some(id) => SessionId::from(id),
            None => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentMissing,
                reason: "No MPP session_id. Create a session via POST /mpp/sessions first.".into(),
            }),
        };

        // Look up the session
        let session = match self.sessions.get(&session_id) {
            Some(s) => s,
            None => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::SessionNotFound,
                reason: format!("MPP session {} not found.", session_id),
            }),
        };

        // Check session status
        match &session.status {
            SessionStatus::Active => {}
            SessionStatus::Exhausted => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::SessionExhausted,
                reason: "Session balance exhausted. Top-up to resume.".into(),
            }),
            SessionStatus::Expired => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::SessionExpired,
                reason: "Session expired.".into(),
            }),
            SessionStatus::Settled { .. } => return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::SessionSettled,
                reason: "Session already settled.".into(),
            }),
        }

        // Check sufficient balance for one draw
        let draw_cost = per_message_cost(
            ctx.store().get::<FeedConfig>(&signal.feed_id()).await?.base_price_per_hour,
            ctx.store().get::<FeedConfig>(&signal.feed_id()).await?.rate_hz,
        );

        if session.balance_remaining < draw_cost {
            return Ok(Verdict {
                passed: false,
                reward: 0.0,
                evidence: Evidence::PaymentInsufficient,
                reason: format!("Balance {} < draw cost {}", session.balance_remaining, draw_cost),
            });
        }

        Ok(Verdict {
            passed: true,
            reward: 1.0,
            evidence: Evidence::PaymentVerified { protocol: "mpp", amount: draw_cost },
            reason: "MPP session payment verified.".into(),
        })
    }

    async fn verify_post(&self, _signal: &Signal, _output: &Signal, _ctx: &CellContext) -> Result<Verdict> {
        Ok(Verdict::pass())
    }
}

impl VerifyMppCell {
    async fn execute(&mut self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        let session_id = SessionId::from(input[0].metadata.get("mpp_session_id").unwrap());
        let session = self.sessions.get_mut(&session_id).unwrap();

        let draw_cost = per_message_cost(
            ctx.store().get::<FeedConfig>(&input[0].feed_id()).await?.base_price_per_hour,
            ctx.store().get::<FeedConfig>(&input[0].feed_id()).await?.rate_hz,
        );

        // Deduct from session balance
        session.balance_remaining = session.balance_remaining.saturating_sub(draw_cost);
        session.draws.push(MppDraw {
            amount: draw_cost,
            feed_id: input[0].feed_id().to_string(),
            message_id: input[0].hash().to_string(),
            timestamp: Utc::now(),
        });

        // Transition to Exhausted if balance is zero
        if session.balance_remaining == 0 {
            session.status = SessionStatus::Exhausted;
            // Publish exhaustion notice on Bus
            ctx.bus().publish(Pulse {
                topic: format!("mpp:session:{}:exhausted", session_id),
                payload: json!({ "session_id": session_id, "total_draws": session.draws.len() }),
                seq: ctx.next_seq(),
            }).await?;
        }

        Ok(input)
    }
}
```

---

## 2. x402: Per-Request Stateless Payment

The simplest payment flow. No session, no state. Each request carries its own authorization.

### 2.1 Protocol Flow

The shipped HTTP flow protects paid feed descriptor reads:

```
Client                              roko-serve
  |  GET /api/feeds/{id}                |
  | -----------------------------------> |
  |  402 Payment Required               |
  |  X-Payment-Request: <JSON>           |
  |  body: PaymentRequest JSON           |
  | <----------------------------------- |
  |                                     |
  |  GET /api/feeds/{id}                |
  |  X-Payment-Authorization: <JSON>     |
  | -----------------------------------> |
  |                                     |
  |  200 OK + FeedInfo                  |
  | <----------------------------------- |
```

The retry sends the serialized authorization in `X-Payment-Authorization`. The request amount is the configured per-request cost multiplied by the feed tier multiplier and rounded up to KORAI base units. Public and private feeds bypass this payment check. Paid feeds at an effective zero price also pass without a payment header.

### 2.2 ERC-3009 Signatures

x402 models ERC-3009 `transferWithAuthorization` through `PaymentAuthorization`. The live HTTP guard deserializes the exact JSON structure and requires `authorization.value >= required_amount`; it deliberately does **not** verify the signature. `X402Manager::verify_authorization` additionally checks recipient, validity window, and nonce reuse for settlement queueing, but cryptographic `ecrecover` verification is still unimplemented.

### 2.3 Batch Settlement

`SettlementBatch` stores verified `(PaymentAuthorization, PaymentRequest)` pairs. `should_settle` fires at 100 accumulated authorizations or once a non-empty batch reaches its configurable timeout (600 seconds by default), and `drain` atomically takes the pending pairs. `X402Manager::queue_for_settlement` verifies and replay-protects an authorization before adding it.

The in-memory queue and trigger semantics are shipped. Connecting the paid-feed HTTP guard to this queue and submitting the drained batch on-chain remain integration work.

```rust
pub struct SettlementBatch {
    pub authorizations: Vec<(PaymentAuthorization, PaymentRequest)>,
    pub created_at: u64,
    pub max_batch_size: usize,
    pub batch_timeout_secs: u64,
}

let mut batch = SettlementBatch::new(100, 600);
batch.add_authorization(authorization, request);
if batch.should_settle(now) {
    let pending = batch.drain();
    // Planned: submit `pending` through the chain backend.
}
```

---

## 3. MPP: Session-Based Streaming Payment

MPP is the session-oriented accounting model for continuous feeds. One authorization caps an entire session, so metering does not require a new authorization per usage increment.

### 3.1 Current accounting flow

```
open_session(MppSession { authorization, rate_per_unit, max_cost, ... })
    -> meter_usage(session_id, units)
    -> expire_stale(now), when appropriate
    -> settle(session_id)
    -> MppSettlement { usage_units, amount, subscriber, provider }
```

`meter_usage` uses checked arithmetic and rejects an increment whose `usage_units * rate_per_unit` would exceed `max_cost`. `settle` computes that exact product and transitions an active or expired session to `Closed`.

There is currently no `POST /mpp/sessions` route, WebSocket payment binding, top-up endpoint, automatic per-message draw, exhaustion Pulse, refund transaction, or on-chain settlement submission. Those flows below are product targets rather than current API contracts.

### 3.2 Session Lifecycle

```
Active -------------------------------> Closed
  |                                       ^
  +------------> Expired -----------------+
  |
  +------------> Disputed
```

- **Active**: usage may be metered up to `max_cost`.
- **Expired**: `expire_stale` marks an active session once the supplied time is past its deadline; expired sessions may still be settled.
- **Closed**: final amount has been computed by `settle`; further metering and repeated settlement are rejected.
- **Disputed**: accounting is paused and the session is not settleable by the current manager.

```rust
pub struct MppSession {
    pub session_id: [u8; 32],
    pub subscriber: Address,
    pub provider: Address,
    pub authorization: PaymentAuthorization,
    pub usage_units: u64,
    pub rate_per_unit: u256,
    pub max_cost: u256,
    pub started_at: u64,
    pub deadline: u64,
    pub state: MppSessionState,
}

pub enum MppSessionState {
    Active,
    Closed,
    Expired,
    Disputed,
}
```

### 3.3 Planned transport integration

The intended transport flow is to open a session, attach its id to a streaming subscription, meter delivered usage, and settle on close or expiry. Top-up/resume behavior requires a future extension to the current state model and is not implemented.

### 3.4 One-authorization accounting

The accounting layer stores one `PaymentAuthorization` on the session and accumulates usage independently. Cryptographic authorization verification and transport-driven metering must be added before treating this as a production payment path.

---

## 4. When to Use Which

| Scenario | Protocol | Why |
|---|---|---|
| Try a feed for 5 minutes | x402 | No session overhead, pay per message |
| Subscribe to a price feed for 24h | MPP | One signature, draws per tick |
| Query an agent's analysis on-demand | x402 | Stateless, pay per query |
| Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |
| Webhook-triggered one-shot request | x402 | No state to manage |
| Dashboard monitoring live feed | MPP | Continuous stream, balance visible in UI |
| Agent consuming another agent's feed | MPP | Autonomous operation without re-signing |
| Trying feed before committing | x402 | Pay-per-request, low commitment |

---

## 5. Reputation-Based Pricing

Pricing tiers are represented by `roko_core::feed::PricingTier`. `resolve_pricing_tier` in `roko-chain` calculates the mean of the seven effective reputation-domain scores and applies discipline gating before score thresholds. It returns a string-based `PricingTierResult` so `roko-chain` does not depend on `roko-core`; callers map that name to the core enum.

### 5.1 Five Tiers

| Tier | Resolution | Price multiplier |
|---|---|---:|
| **Free** | Discipline is not `GoodStanding`; selling is disabled | 0.0 |
| **Starter** | Good standing and aggregate score `< 0.4` | 0.5 |
| **Standard** | Good standing and `0.4 <= score < 0.6` | 1.0 |
| **Professional** | Good standing and `0.6 <= score < 0.8` | 1.5 |
| **Enterprise** | Good standing and score `>= 0.8` | 2.0 |

### 5.2 Pricing Example

A feed with a base per-request cost of 10 KORAI resolves as follows:

| Tier | Effective per-request cost |
|---|---:|
| Free | 0 KORAI |
| Starter | 5 KORAI |
| Standard | 10 KORAI |
| Professional | 15 KORAI |
| Enterprise | 20 KORAI |

The HTTP payment guard uses the tier already stored in `FeedPricingConfig`; automatic registry-to-feed tier refresh is not wired yet.

```rust
impl PricingTier {
    pub const fn price_multiplier(self) -> f64 {
        match self {
            Self::Free => 0.0,
            Self::Starter => 0.5,
            Self::Standard => 1.0,
            Self::Professional => 1.5,
            Self::Enterprise => 2.0,
        }
    }
}
```

---

## 6. Relay Payment Flow

The target relay owns feed registration, payment gating, and message forwarding. Today `roko-serve` owns the in-memory `FeedRegistry` and applies the x402 structural guard to paid feed descriptor reads. Payment-gated message forwarding, MPP draws, and automated settlement are not wired.

### 6.1 Target Payment Flow Diagram

```
Subscriber                    Relay                     Feed Producer
    |                           |                            |
    |  Open MPP session         |                            |
    |  (ERC-3009 auth)          |                            |
    | ------------------------> |  Store session ref         |
    |                           | -------------------------> |
    |  Subscribe to feed room   |                            |
    |  with session_id          |                            |
    | ------------------------> |                            |
    |                           |                            |
    |                           |  <-- feed_data ----------  |
    |                           |                            |
    |                           |  Draw from session:        |
    |                           |  cost = base_price         |
    |                           |        / rate_hz / 3600    |
    |                           |                            |
    |                           |  Draw succeeds?            |
    |  <-- feed_data ---------- |  Yes: forward              |
    |  <-- payment_draw ------- |                            |
    |                           |                            |
    |                           |  Draw fails (exhausted)?   |
    |  <-- exhaustion_notice -- |  Unsubscribe, notify       |
    |                           |                            |
    |  Top-up session           |                            |
    | ------------------------> |  Resume draws              |
    |                           |                            |
    |  Disconnect / unsubscribe |                            |
    | ------------------------> |  Session stays open        |
    |                           |  (reusable on reconnect)   |
```

### 6.2 Per-Message Draw Calculation

This calculation is a target for future transport-driven MPP metering; the current `MppSessionManager` accepts explicit usage units and multiplies them by `rate_per_unit`.

```rust
pub fn per_message_cost(base_price_per_hour: u64, rate_hz: f64) -> u64 {
    // base_price_per_hour is in KORAI base units
    // rate_hz is messages per second
    // cost per message = price_per_hour / (rate_hz * 3600)
    let messages_per_hour = (rate_hz * 3600.0) as u64;
    if messages_per_hour == 0 { return base_price_per_hour; }
    base_price_per_hour / messages_per_hour
}
```

---

## 7. Payment Disputes

This section is planned behavior. E36 defines the `Disputed` MPP state but does not implement dispute creation, arbitration, credits, reputation effects, or feed suspension.

### 7.1 Dispute Triggers

| Trigger | Detection | Example |
|---|---|---|
| **Wrong data** | `feed-accuracy` meta feed shows accuracy drop below threshold | A price feed reports $0.01 ETH instead of $3,000 |
| **Stale data** | `feed-health` meta feed detects staleness (no update for 3x expected interval) | Feed advertised at 0.5 Hz hasn't updated in 10 seconds |
| **Service interruption** | Subscriber received no data during an active MPP session | Relay outage, producer crash, network partition |
| **Overcharge** | Draw amount exceeds the per-message cost at the subscriber's reputation tier | Bug in pricing logic |

### 7.2 Dispute Resolution Flow

```
Subscriber                    Relay                     Feed Producer
    |                           |                            |
    |  POST /disputes           |                            |
    |  { session_id, reason,    |                            |
    |    evidence: [...] }      |                            |
    | ------------------------> |                            |
    |                           |  Create dispute record     |
    |                           |  Freeze draws on session   |
    |                           |                            |
    |                           |  Notify producer           |
    |                           | -------------------------> |
    |                           |                            |
    |                           |  72-hour response window   |
    |                           |                            |
    |                           |  Producer responds:        |
    |                           |  accept / contest          |
    |                           | <------------------------- |
    |                           |                            |
    |  Resolution applied       |                            |
    | <------------------------ |                            |
```

### 7.3 Resolution Outcomes

**Automatic resolution** (no human needed):

| Condition | Action |
|---|---|
| `feed-accuracy` score < 0.5 for the disputed period | Credit subscriber from next MPP session. Producer's TraceRank -0.02. |
| `feed-health` confirms staleness during disputed period | Pro-rata refund for stale duration. No reputation hit if < 5 minutes. |
| Producer does not respond within 72 hours | Dispute auto-resolved in subscriber's favor. Full session credit. |

**Contested resolution** (relay arbitrates):

When the producer contests and evidence is ambiguous, the relay acts as arbiter:

1. **Evidence review**: The relay examines Bus event logs for the disputed period. All feed data, draws, and health events are logged on Bus and recoverable from the ring buffer or graduated Store Signals.
2. **Ruling**: The relay issues a `DisputeVerdict`:
   - `Credit` -- subscriber receives credit applied to their next MPP session (or refund if no future session).
   - `Dismissed` -- dispute rejected; no action taken.
   - `Split` -- partial credit (e.g., stale for 50% of disputed period).
3. **Appeal**: Either party can appeal to on-chain arbitration via ERC-8004 dispute mechanism (Phase 2+). The on-chain record is immutable.

```rust
pub struct Dispute {
    pub id: DisputeId,
    pub session_id: SessionId,
    pub subscriber: AgentId,
    pub producer: AgentId,
    pub reason: DisputeReason,
    pub evidence: Vec<SignalRef>,     // references to Bus events or Store Signals
    pub status: DisputeStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

pub enum DisputeReason {
    WrongData { expected: String, actual: String },
    StaleData { last_update: DateTime<Utc>, expected_interval: Duration },
    ServiceInterruption { start: DateTime<Utc>, end: DateTime<Utc> },
    Overcharge { charged: u64, expected: u64 },
}

pub enum DisputeStatus {
    Open,
    ProducerNotified { notified_at: DateTime<Utc> },
    Contested { producer_evidence: Vec<SignalRef> },
    Resolved { verdict: DisputeVerdict },
}

pub enum DisputeVerdict {
    Credit { amount: u64, applied_to: Option<SessionId> },
    Dismissed { reason: String },
    Split { credit_amount: u64, credit_ratio: f64 },
}
```

### 7.4 Credit Application

Credits from resolved disputes are applied to the subscriber's next MPP session with the same producer. If the subscriber opens a new session, the credit is automatically deducted from the required funding amount. Credits expire after 30 days if unused.

```rust
pub struct DisputeCredit {
    pub subscriber: AgentId,
    pub producer: AgentId,
    pub amount: u64,
    pub dispute_id: DisputeId,
    pub expires_at: DateTime<Utc>,  // created_at + 30 days
    pub applied: bool,
}
```

### 7.5 Reputation Impact

Dispute outcomes affect TraceRank reputation scores:

| Outcome | Producer Impact | Subscriber Impact |
|---|---|---|
| Credit (subscriber wins) | TraceRank -0.02 | No change |
| Dismissed (producer wins) | No change | TraceRank -0.01 (discourage frivolous disputes) |
| Split | TraceRank -0.01 | No change |
| 3+ disputes in 7 days | Relay warning; feed flagged in discovery | -- |
| 10+ upheld disputes in 30 days | Feed suspended from relay | -- |

---

## 8. Feed Registration and Discovery

### 7.1 Feed Registration

```json
POST /api/feeds
{
  "name": "eth-gas-trend",
  "agent_id": "gas-oracle",
  "kind": "derived",
  "access": "paid",
  "description": "12-block EMA gas price with percentile bands and MEV detection",
  "schema": null,
  "pricing": {
    "tier": "standard",
    "per_request_cost": 50.0,
    "session_pricing": null,
    "protocol": "x402"
  }
}
```

The registry assigns the feed id. `pricing` is optional for serialized compatibility, though paid feeds should advertise it.

### 7.2 Feed Discovery (Dashboard or Agent -> Relay)

```
GET /api/feeds                       # all feeds
GET /api/feeds?kind=derived          # filter by kind
GET /api/feeds?agent_id=gas-oracle   # filter by producer
GET /api/feeds/{feed_id}             # single descriptor; paid feeds run the x402 guard
```

### 7.3 Feed Subscription with Payment

The WebSocket MPP subscription payload below is target design. It is not a currently registered route or message contract.

```json
{
  "type": "subscribe",
  "rooms": ["feed:eth-gas-trend"],
  "payment": {
    "intent": "session",
    "session_id": "abc-123"
  }
}
```

Planned behavior is for the relay to verify the MPP session and meter forwarded feed data.

---

## 9. Feed Marketplace Economics

This section describes the intended marketplace model; E36 does not implement revenue splitting, paid composition, or marketplace settlement.

### 8.1 Feed Types and Composability

Feeds compose into value chains. Each layer adds computation and charges for it.

**Raw feeds** -- direct data ingestion:
- Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket)
- Research: `arxiv-new-papers`, `github-trending` (from web polling)
- Code: `repo-commit-stream`, `ci-build-results` (from webhooks)
- Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs)

**Derived feeds** -- computed from raw:
- Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability`
- Research: `paper-relevance-scores`, `topic-cluster-updates`
- Code: `code-quality-trend`, `dependency-risk-index`
- Market: `volatility-regime`, `cross-venue-spread`

**Composite feeds** -- derived from multiple derived feeds:
- `cross-chain-arb-signal` (consumes gas trends + volume + funding rates)
- `research-portfolio-impact` (consumes paper scores + code quality + market sentiment)
- Cost stacks: producer pays for input feeds, charges for output feed

**Meta feeds** -- feeds about feeds:
- `feed-health` (monitors all feeds for staleness, drift, anomalies)
- `feed-accuracy` (tracks prediction accuracy of derived feeds over time)

### 8.2 Composition Example: Value Chain

```
eth-mainnet-blocks (free, raw)
  +-> gas-oracle agent
       +-> eth-gas-trend ($0.05/hr, derived)
            +-> arb-bot agent
                 +-> cross-chain-gas-arb ($0.50/hr, composite)
                      +-> dashboard subscriber

arxiv-new-papers (free, raw)
  +-> research-scout agent
       +-> defi-paper-relevance ($0.02/hr, derived)
            +-> strategy-agent subscribes for research context
```

Each agent in the chain pays for its inputs and charges for its output.

### 8.3 Practical Example: Funding Rate Divergence

An agent that consumes two paid feeds and produces a third:

```toml
[agent]
name = "funding-arb"
profile = "chain"
mode = "persistent"

# This agent CONSUMES two feeds...
[[agent.feed_subscriptions]]
feed_id = "binance-funding-rates"
agent_id = "cex-connector"
budget_korai = 1000  # KORAI session deposit

[[agent.feed_subscriptions]]
feed_id = "hyperliquid-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000

# ...and PRODUCES one feed
[agent.feeds.funding-divergence]
kind = "derived"
description = "Cross-venue funding rate divergence with z-score normalization"
schema = "funding_divergence_v1"
rate_hz = 0.1  # Every 10 seconds
access = "paid"
base_price_usdc_per_hour = 200000  # $0.20/hr
```

The extension that computes the feed data. Feed data is published via Bus (not `ctx.cortical`) -- this eliminates hidden channels and aligns with the universal transport rule (see [00-INDEX](00-INDEX.md): "Everything through Bus or Store").

```rust
pub struct FundingDivergenceExt {
    binance_sub: FeedSubscription,
    hyperliquid_sub: FeedSubscription,
    history: VecDeque<f64>,
}

#[async_trait]
impl Extension for FundingDivergenceExt {
    fn name(&self) -> &str { "funding-divergence" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let binance = self.binance_sub.latest_or_default();
        let hyper = self.hyperliquid_sub.latest_or_default();

        let divergence = binance["rate"].as_f64().unwrap_or(0.0)
            - hyper["rate"].as_f64().unwrap_or(0.0);

        self.history.push_back(divergence);
        if self.history.len() > 1000 { self.history.pop_front(); }

        let mean = self.history.iter().sum::<f64>() / self.history.len() as f64;
        let variance = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        let zscore = if variance > 0.0 {
            (divergence - mean) / variance.sqrt()
        } else {
            0.0
        };

        // Publish feed data via Bus (not cortical -- no hidden channels)
        ctx.bus().publish(Pulse {
            topic: "feed:funding-divergence:data".into(),
            payload: json!({
                "divergence_bps": divergence * 10000.0,
                "zscore": zscore,
                "binance_rate": binance["rate"],
                "hyperliquid_rate": hyper["rate"],
                "signal": if zscore.abs() > 2.0 { "strong" }
                          else if zscore.abs() > 1.0 { "moderate" }
                          else { "none" },
                "direction": if divergence > 0.0 { "long_hyper" }
                             else { "long_binance" },
                "ts": now_ms(),
            }),
            seq: ctx.next_seq(),
        }).await?;

        // Extreme divergence triggers T2 reasoning via prediction error Bus event
        if zscore.abs() > 3.0 {
            ctx.bus().publish(Pulse {
                topic: format!("agent:{}:prediction_error", ctx.agent_id()),
                payload: json!({ "prediction_error": 0.9, "source": "funding-divergence" }),
                seq: ctx.next_seq(),
            }).await?;
        }

        Ok(())
    }
}
```

> **Design note**: Previous versions used `ctx.cortical.set_feed_data()` which is a hidden side-channel. Feed data MUST flow through Bus (ephemeral) or Store (durable). The `FeedPublisherExt` (Social layer) subscribes to `feed:{id}:data` Bus topics and forwards to the relay. This maintains the invariant: two fabrics, no exceptions.

**Economics for `funding-arb`**: $0.20/hr revenue per subscriber minus $0.10/hr input cost. With 5 subscribers: ($0.20 * 5) - $0.10 = $0.90/hr pure margin.

---

## 10. Setting Up a Paid Feed

The extension and manifest APIs below are target developer ergonomics. The shipped registration path is the `FeedInfo` / `FeedPricingConfig` model and `POST /api/feeds` described in Section 8.

### 9.1 Declare the Feed in Agent Manifest

```toml
[agent]
name = "gas-oracle"
profile = "chain"
mode = "persistent"

[agent.feeds]
[agent.feeds.eth-gas-trend]
kind = "derived"
description = "12-block EMA gas price with percentile bands and MEV spike detection"
schema = "gas_trend_v1"
rate_hz = 0.5
access = "paid"
base_price_usdc_per_hour = 50
```

When the agent boots, `FeedPublisherExt` reads these declarations and registers them with the relay.

### 9.2 The FeedPublisherExt Extension

Auto-loaded when `[agent.feeds.*]` entries exist. Handles the full lifecycle: register on boot, publish on each tick, deregister on shutdown.

```rust
pub struct FeedPublisherExt {
    feeds: Vec<FeedConfig>,
    relay: RelayHandle,
}

#[async_trait]
impl Extension for FeedPublisherExt {
    fn name(&self) -> &str { "feed-publisher" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.register_feed(FeedRegistration {
                feed_id: feed.id.clone(),
                agent_id: ctx.agent_id.clone(),
                kind: feed.kind,
                schema: feed.schema.clone(),
                rate_hz: feed.rate_hz,
                access: feed.access.clone(),
                sample: feed.sample.clone(),
            }).await?;
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Subscribe to each feed's Bus topic and forward to relay
        for feed in &self.feeds {
            if let Some(pulse) = ctx.bus().latest(&format!("feed:{}:data", feed.id)).await {
                ctx.relay.publish_feed_data(&feed.id, pulse.payload).await?;
            }
        }
        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.deregister_feed(&feed.id).await?;
        }
        Ok(())
    }
}
```

### 9.3 Compute Feed Data (Cognition Layer)

Feed data is published via Bus, not `ctx.cortical` -- see design note in SS8.3.

```rust
pub struct GasTrendExt {
    ema: f64,
    window: VecDeque<f64>,
}

#[async_trait]
impl Extension for GasTrendExt {
    fn name(&self) -> &str { "gas-trend" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Read gas price from the raw feed Bus topic
        let gas = ctx.bus().latest("feed:eth-mainnet-blocks:data").await
            .and_then(|p| p.payload["gas_gwei"].as_f64())
            .unwrap_or(0.0);
        self.window.push_back(gas);
        if self.window.len() > 100 { self.window.pop_front(); }

        let alpha = 2.0 / 13.0;
        self.ema = alpha * gas + (1.0 - alpha) * self.ema;

        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p25 = sorted[sorted.len() / 4];
        let p75 = sorted[3 * sorted.len() / 4];
        let p95 = sorted[19 * sorted.len() / 20];
        let mev_spike = gas > p95 * 2.0;

        // Publish derived feed data via Bus
        ctx.bus().publish(Pulse {
            topic: "feed:eth-gas-trend:data".into(),
            payload: json!({
                "ema_12": self.ema,
                "p25": p25,
                "p75": p75,
                "p95": p95,
                "mev_spike": mev_spike,
                "current": gas,
                "ts": now_ms(),
            }),
            seq: ctx.next_seq(),
        }).await?;

        Ok(())
    }
}
```

Pipeline order: `GasTrendExt` (Cognition layer) runs during `on_observe`, publishes data to the `feed:eth-gas-trend:data` Bus topic. Then `FeedPublisherExt` (Social layer) runs during `on_tick_end`, reads the latest Pulse from that Bus topic and forwards to the relay. Extension layers execute in order: Perception -> Cognition -> Social. All data flows through Bus -- no hidden channels.

---

## 11. Dashboard Subscription (TypeScript)

This is a target client flow. The MPP session endpoints used by the example are not implemented.

```typescript
// 1. Discover available feeds
const feeds = await fetch(`${relayUrl}/relay/feeds`).then(r => r.json());
const gasFeed = feeds.find(f => f.feed_id === "eth-gas-trend");

// 2. Open an MPP session (one-time ERC-3009 signature)
const session = await openMppSession(relayUrl, {
  amount: 500,  // KORAI base units
  recipient: gasFeed.agent_wallet,
});

// 3. Subscribe to the feed via WebSocket with session auth
const ws = new WebSocket(`${relayUrl}/relay/ws`);
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: "subscribe",
    rooms: [`feed:${gasFeed.feed_id}`],
    payment: {
      intent: "session",
      session_id: session.session_id,
    }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === "feed_data") {
    updateGasChart(msg.payload);
  }
  if (msg.type === "payment_draw") {
    updateBalance(msg.payload);
  }
};
```

---

## 12. Agent-to-Agent Feed Subscription (Rust)

This is a target client flow; there is no shipped MPP transport client or paid feed-data subscription route yet.

```rust
pub struct GasConsumerExt {
    gas_subscription: Option<FeedSubscription>,
}

#[async_trait]
impl Extension for GasConsumerExt {
    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let session = ctx.mpp.open_session(
            "gas-oracle",  // agent producing the feed
            500,           // KORAI base units
        ).await?;

        self.gas_subscription = Some(
            ctx.relay.subscribe_feed("eth-gas-trend", session.session_id).await?
        );
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        if let Some(sub) = &self.gas_subscription {
            if let Some(data) = sub.latest() {
                let mev_spike = data["mev_spike"].as_bool().unwrap_or(false);
                if mev_spike {
                    // Publish prediction error via Bus (not cortical)
                    ctx.bus().publish(Pulse {
                        topic: format!("agent:{}:prediction_error", ctx.agent_id()),
                        payload: json!({ "prediction_error": 0.8, "source": "eth-gas-trend" }),
                        seq: ctx.next_seq(),
                    }).await?;
                }
            }
        }
        Ok(())
    }
}
```

---

## 13. On-Chain Feed Advertisement (ERC-8004)

On-chain feed advertisement and merged chain/relay discovery remain planned and were not part of E36.

Agents with wallets advertise their feeds in their ERC-8004 passport. This makes feeds discoverable on-chain even when the agent or relay is offline.

```solidity
// AgentRegistry.sol -- feed advertisement extension
struct FeedAdvert {
    bytes32 feedId;        // keccak256 of feed name
    bytes32 schemaHash;    // keccak256 of schema definition
    uint16  rateMilliHz;   // rate in milli-Hz (500 = 0.5 Hz)
    uint96  pricePerHour;  // KORAI base units per hour (0 = free)
    uint32  updatedAt;     // last update timestamp
}

function updateFeeds(FeedAdvert[] calldata adverts) external;
function getFeeds(address agent) external view returns (FeedAdvert[] memory);
```

When an agent boots with feeds configured, it:

1. Registers feeds with the relay (for live presence and subscription routing).
2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery).
3. On feed config changes (add/remove/reprice), updates both relay and chain.

```rust
// In FeedPublisherExt::on_boot()
async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
    for feed in &self.feeds {
        ctx.relay.register_feed(/* ... */).await?;

        if let Some(chain) = &ctx.chain_client {
            chain.update_feed_advert(&ctx.agent_wallet, FeedAdvert {
                feed_id: keccak256(feed.id.as_bytes()),
                schema_hash: keccak256(feed.schema.as_bytes()),
                rate_milli_hz: (feed.rate_hz * 1000.0) as u16,
                price_per_hour: feed.price_usdc_per_hour,
            }).await?;
        }
    }
    Ok(())
}
```

Feed discovery uses both sources:

```typescript
async function discoverFeeds(): Promise<Feed[]> {
  const [relayFeeds, chainFeeds] = await Promise.all([
    fetch(`${relayUrl}/relay/feeds`).then(r => r.json()),
    chainClient.getRegisteredFeeds(),
  ]);

  return mergeFeeds(relayFeeds, chainFeeds);
  // Result: each feed has { ...chainAdvert, live: boolean, subscribers: number }
}
```

An agent's feeds appear in its passport even when the agent is offline.

---

## 14. Dashboard Integration

E36 ships the shared dashboard state contract, not the subscription/revenue screens mocked below. `DashboardEvent::PaymentReceived` updates payment count, cumulative KORAI, and per-protocol counts; `DashboardEvent::SettlementCompleted` updates settlement count.

### 13.1 Feeds Page

```
+--------------------------------------------------------------+
| Available Feeds                               [+ Publish Feed]|
|                                                               |
| Filter: [All v] [Paid v] [Chain: All v] [Search...]          |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend                                 * LIVE      | |
| | by gas-oracle (Trusted)                      $0.05/hr    | |
| | 12-block EMA gas with percentile bands + MEV detect      | |
| | Schema: gas_trend_v1   Rate: 0.5 Hz   Subs: 7           | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | uniswap-v3-tick-activity                     * LIVE      | |
| | by pool-watcher (Verified)                  $0.20/hr    | |
| | Real-time tick-level activity for top 50 pools           | |
| | Schema: tick_activity_v2   Rate: 2 Hz   Subs: 3         | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

### 13.2 My Subscriptions

```
+--------------------------------------------------------------+
| My Feed Subscriptions                                         |
|                                                               |
| Active spend: $0.25/hr across 3 feeds                        |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend          * Active    Session: $4.82 left    | |
| | gas-oracle             $0.05/hr   Since: 2h ago           | |
| | [Pause] [Top-up $5] [Unsubscribe]                        | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | cross-chain-gas-arb    * Active    Session: $1.20 left    | |
| | arb-bot                $0.50/hr   Since: 45m ago          | |
| | [Pause] [Top-up $10] [Unsubscribe]                       | |
| +----------------------------------------------------------+ |
|                                                               |
| Total spent this month: $12.40                                |
| Total earned from my feeds: $8.70                             |
+--------------------------------------------------------------+
```

### 13.3 Feed Detail Page

```
+--------------------------------------------------------------+
| eth-gas-trend                                     * LIVE      |
| by gas-oracle (Trusted, 342 episodes)            $0.05/hr    |
|                                                               |
| +--------------- Live Preview -------------------------+     |
| | EMA: 42.5 gwei   P25: 35.0   P75: 55.0   P95: 120  |     |
| | MEV: none                                            |     |
| |                                                      |     |
| | [sparkline chart of last 100 data points]            |     |
| +------------------------------------------------------+     |
|                                                               |
| Schema: gas_trend_v1                                          |
| Fields: ema_12 (f64), p25 (f64), p75 (f64), p95 (f64),      |
|         mev_spike (bool), current (f64), ts (u64)            |
|                                                               |
| Uptime: 99.7% (30d)   Avg latency: 120ms                     |
| Subscribers: 7   Revenue: $84.20 (30d)                        |
|                                                               |
| Dependencies: eth-mainnet-blocks (free)                       |
|                                                               |
| Payment: x402 or MPP session                                  |
| [Subscribe with MPP ($5 deposit)]  [Try with x402 ($0.01)]   |
+--------------------------------------------------------------+
```

### 13.4 Feed Revenue

```
+--------------------------------------------------------------+
| Feed Revenue                                                  |
|                                                               |
| Total earned (30d): $84.20    Active subscribers: 7           |
|                                                               |
| Feed               Subs  Revenue/30d  Status                  |
| eth-gas-trend       7     $84.20      * producing             |
|                                                               |
| [chart: revenue over time, subscriber count over time]        |
|                                                               |
| Settlement: 12 batches settled on-chain                       |
| Pending: $2.30 (next batch in ~8 min)                         |
+--------------------------------------------------------------+
```

### 13.5 Dashboard Data Sources

| Shipped event | Payload | Snapshot effect |
|---|---|---|
| `payment_received` | `feed_id`, `protocol`, `amount_korai`, `payer`, `payee` | Increment `payment_count`, add to `total_payment_korai`, increment `payments_by_protocol[protocol]` |
| `settlement_completed` | `protocol`, `batch_size`, `total_korai` | Increment `settlement_count` |

REST/WS views dedicated to MPP session balances, pending batches, and feed revenue remain planned.

---

## 15. Verification Status

### 15.1 E36 shipped acceptance

| Task | Verified result |
|---|---|
| E36-T01 | Core pricing tiers and per-request/session pricing types serialize and deserialize |
| E36-T02 | Settlement batch triggers at 100 authorizations or timeout, rejects overflow, drains, and replay-protects manager queueing |
| E36-T03 | MPP usage metering, max-cost rejection, expiry, and exact settlement accounting |
| E36-T04 | Seven-domain aggregate tier resolution with exact boundaries and discipline gate |
| E36-T05 | `FeedInfo.pricing` round-trip plus legacy JSON compatibility |
| E36-T06 | Separate payment cost storage, queries, summaries, mixed tagged JSONL round-trip, and legacy import |
| E36-T07 | Paid feed reads: missing/malformed/underfunded authorization returns 402; sufficient authorization passes; public/private feeds bypass |
| E36-T08 | Payment and settlement events serialize and update dashboard counters |

Focused verification passed in `roko-core`, `roko-chain`, `roko-learn`, and `roko-serve`; the affected crates compile.

### 15.2 Long-term acceptance criteria (roadmap)

The criteria below describe the full target system. They are not all shipped by E36; in particular, signature recovery, on-chain submission, MPP transport/top-up/refunds, Verify Cells, disputes, and dedicated dashboard screens remain open.

| # | Criterion | Verification |
|---|---|---|
| P-1 | x402: 402 response includes amount, recipient, nonce, expiry | Unit test |
| P-2 | x402: ERC-3009 signature verified via ecrecover (no RPC) | Unit test |
| P-3 | x402: batch settlement fires at 100 accumulated authorizations | Integration test |
| P-4 | x402: batch settlement fires at 10-minute interval | Integration test with mocked clock |
| P-5 | MPP: session created with funded balance | Integration test |
| P-6 | MPP: per-message draw decrements balance | Unit test |
| P-7 | MPP: exhausted session pauses delivery and sends notice | Integration test |
| P-8 | MPP: top-up resumes delivery | Integration test |
| P-9 | MPP: expired session transitions to settled with refund | Integration test |
| P-10 | MPP: session stays open on disconnect (reusable) | Integration test |
| P-11 | Reputation tier correctly applies markup to base price | Unit test for all 5 tiers |
| P-12 | Producer receives base price regardless of subscriber tier | Unit test |
| P-13 | Relay receives markup as infrastructure fee | Unit test |
| P-14 | Feed registration stores metadata and makes feed discoverable | Integration test |
| P-15 | Feed sample accessible without auth or payment | Integration test |
| P-16 | Feed subscription with MPP session receives data | Integration test |
| P-17 | Feed subscription with x402 returns single data point | Integration test |
| P-18 | Composite feed: agent pays for input feeds and charges for output | Integration test |
| P-19 | ERC-8004 feed advert updated on agent boot | Integration test with mock chain |
| P-20 | Feed discovery merges relay (live) and chain (persistent) sources | Integration test |
| P-21 | Dashboard subscription manager shows session balance | E2E test |
| P-22 | Dashboard feed revenue shows settlement status | E2E test |
| P-23 | VerifyX402Cell implements Cell + VerifyProtocol | Unit test |
| P-24 | VerifyX402Cell rejects requests without X-Payment header with 402 terms | Unit test |
| P-25 | VerifyX402Cell verifies ERC-3009 signature via ecrecover | Unit test |
| P-26 | VerifyX402Cell collects authorization for batch settlement | Integration test |
| P-27 | VerifyMppCell implements Cell + VerifyProtocol | Unit test |
| P-28 | VerifyMppCell rejects requests with exhausted session | Unit test |
| P-29 | VerifyMppCell deducts draw from session balance and transitions to Exhausted at zero | Unit test |
| P-30 | VerifyMppCell publishes exhaustion Pulse on Bus | Integration test |
| P-31 | Dispute creation freezes draws on the disputed session | Integration test |
| P-32 | Dispute auto-resolves in subscriber favor if producer does not respond in 72h | Integration test with mocked clock |
| P-33 | Dispute credit applied to subscriber's next MPP session with same producer | Integration test |
| P-34 | Dispute credits expire after 30 days if unused | Unit test |
| P-35 | 10+ upheld disputes in 30 days suspends producer feed from relay | Integration test |
| P-36 | Feed data published via Bus (not cortical) -- no hidden channels | Unit test: verify Bus Pulse on `feed:{id}:data` topic |
| P-37 | FeedPublisherExt reads from Bus topic (not cortical) for relay forwarding | Unit test |

---

## 16. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal, Pulse | [doc-01](01-SIGNAL.md) | SS1-3 |
| Verify protocol | [doc-02](02-CELL.md) | SS3.3 |
| Bus (ephemeral transport) | [doc-01](01-SIGNAL.md) | SS2 |
| Relay, feed infrastructure | [doc-11](11-CONNECTIVITY.md) | SS3 |
| Agent bearer tokens | [doc-17](17-AUTH.md) | SS4 |
| ERC-8004 passport | [doc-22](22-REGISTRIES.md) | SS2 |
| Reputation tiers | [doc-22](22-REGISTRIES.md) | SS3 |
| Extension layers (Perception/Cognition/Social) | [doc-12](12-EXTENSIONS.md) | SS3 |
| Feed Cell (Connect+Trigger+Store) | [doc-09](09-FEEDS.md) | SS1-4 |
| Anti-pattern: hidden channels | [doc-00](00-INDEX.md) | Anti-Principles |
