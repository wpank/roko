# Paid feeds and agent services

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> See [Feeds and Data Streams](05-feeds.md) for the base feed system this builds on.

---

The previous section covers how agents subscribe to chain data and expose feeds via the relay. This section covers payment: how a feed producer sets a price, how subscribers pay, how sessions work, and how the dashboard surfaces it all.

### Payment protocols

Two protocols, both implemented in bardo (`crates/mpp/`).

**x402 (per-request, stateless)**

The simplest payment flow. No session, no state. Each request carries its own authorization.

```
Client                                  Server (relay / agent)
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │ ──────────────────────────────────────> │
  │                                         │
  │  HTTP 402                               │
  │  X-Payment-Required:                    │
  │    amount=50, recipient=0xABC...,       │
  │    nonce=1, expiry=1714000000           │
  │ <────────────────────────────────────── │
  │                                         │
  │  Client signs ERC-3009 authorization    │
  │  (gasless USDC approval, no on-chain tx)│
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │  X-Payment: <signed authorization>      │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Server verifies signature (ecrecover,  │
  │  no RPC needed), serves content         │
  │                                         │
  │  200 OK + feed data                     │
  │ <────────────────────────────────────── │
```

Settlement happens in batches: every 10 minutes or after 100+ accumulated authorizations, whichever comes first. The server submits a single on-chain transaction that settles all pending authorizations. This amortizes gas costs across many payments.

**MPP (session-based, streaming)**

For continuous feeds. One signature funds an entire session. No re-signing per message.

```
Client                                  Server (relay / agent)
  │                                         │
  │  POST /mpp/sessions                     │
  │  { amount: 500, authorization: <sig> }  │
  │ ──────────────────────────────────────> │
  │                                         │
  │  201 Created                            │
  │  { session_id: "abc-123",               │
  │    funded: 500, status: "active" }      │
  │ <────────────────────────────────────── │
  │                                         │
  │  WS subscribe with session_id           │
  │  { rooms: ["feed:eth-gas-trend"],       │
  │    payment: { session_id: "abc-123" } } │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Per-message draw from session          │
  │  (no client interaction needed)         │
  │                                         │
  │  feed_data: { ema_12: 42.5, ... }       │
  │  payment_draw: { amount: 1,             │
  │    balance_remaining: 499 }             │
  │ <────────────────────────────────────── │
```

Session lifecycle:

```
Active ──> Exhausted ──> Expired ──> Settled
  │            │                       │
  │  (top-up)  │                       │
  └────────────┘                       │
                                       └── Refund unspent balance
```

- **Active**: draws succeed, messages flow.
- **Exhausted**: balance hits zero. Server sends exhaustion notice and pauses delivery. Client can top-up to resume.
- **Expired**: TTL reached (default 24h). No more draws. Transitions to Settled.
- **Settled**: unspent balance refunded. Session closed. Settlement submitted on-chain.

**When to use which**

| Scenario | Protocol | Why |
|----------|----------|-----|
| Try a feed for 5 minutes | x402 | No session overhead, pay per message |
| Subscribe to a price feed for 24h | MPP | One signature, draws per tick |
| Query an agent's analysis on-demand | x402 | Stateless, pay per query |
| Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |

**Reputation-based pricing**

Higher ERC-8004 reputation tier = lower markup. Applied on top of the feed producer's base price.

| Tier | Markup |
|------|--------|
| None | +20% |
| Basic | +18% |
| Verified | +15% |
| Trusted | +12% |
| Sovereign | +8% |

A feed priced at $0.10/hr costs a `None`-tier subscriber $0.12/hr and a `Sovereign`-tier subscriber $0.108/hr. The spread goes to the relay as an infrastructure fee.

### Setting up a paid feed

A concrete walkthrough: building a "gas-oracle" agent that produces a paid ETH gas trend feed.

**Step 1: Declare the feed in the agent manifest**

```toml
# roko.toml -- agent manifest
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
base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)
# For pricier feeds:
# base_price_usdc_per_hour = 500000  # $0.50/hr

[agent.feeds.eth-gas-trend.sample]
# Sample payload shown to prospective subscribers before they pay
data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'
```

When the agent boots, `FeedPublisherExt` reads these declarations and registers them with the relay. No manual registration needed.

**Step 2: The FeedPublisherExt extension**

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
        for feed in &self.feeds {
            if let Some(data) = ctx.cortical.get_feed_data(&feed.id) {
                ctx.relay.publish_feed_data(&feed.id, data).await?;
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

**Step 3: Compute the feed data**

The feed's value comes from a `Cognition`-layer extension that runs during the agent's `on_observe` step -- before `FeedPublisherExt` publishes in `on_tick_end`.

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
        let gas = ctx.cortical.gas_gwei();
        self.window.push_back(gas);
        if self.window.len() > 100 { self.window.pop_front(); }

        // 12-block EMA
        let alpha = 2.0 / 13.0;
        self.ema = alpha * gas + (1.0 - alpha) * self.ema;

        // Percentiles from rolling window
        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p25 = sorted[sorted.len() / 4];
        let p75 = sorted[3 * sorted.len() / 4];
        let p95 = sorted[19 * sorted.len() / 20];

        // MEV spike: gas exceeds 2x the 95th percentile
        let mev_spike = gas > p95 * 2.0;

        ctx.cortical.set_feed_data("eth-gas-trend", json!({
            "ema_12": self.ema,
            "p25": p25,
            "p75": p75,
            "p95": p95,
            "mev_spike": mev_spike,
            "current": gas,
            "ts": now_ms(),
        }));

        Ok(())
    }
}
```

The pipeline order matters: `GasTrendExt` (Cognition layer) runs during `on_observe`, writes data to `cortical`. Then `FeedPublisherExt` (Social layer) runs during `on_tick_end`, reads the data and publishes it to the relay. Extension layers execute in order: Perception -> Cognition -> Social.

**Step 4: Subscribe from a dashboard**

```typescript
// 1. Discover available feeds
const feeds = await fetch(`${relayUrl}/relay/feeds`).then(r => r.json());
const gasFeed = feeds.find(f => f.feed_id === "eth-gas-trend");
// -> { feed_id, agent_id, kind: "derived", rate_hz: 0.5,
//      access: { paid: { price_per_hour: 50 } } }

// 2. Open an MPP session (one-time ERC-3009 signature)
const session = await openMppSession(relayUrl, {
  amount: 500,  // $0.0005 USDC -- enough for ~10 hours at $0.05/hr
  recipient: gasFeed.agent_wallet,
});
// -> { session_id: "abc-123", funded_amount: 500, status: "active" }

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
    // { ema_12: 42.5, p25: 35.0, p75: 55.0, p95: 120.0, mev_spike: false }
    updateGasChart(msg.payload);
  }
  if (msg.type === "payment_draw") {
    // { amount: 1, balance_remaining: 499, session_id: "abc-123" }
    updateBalance(msg.payload);
  }
};
```

**Step 5: Subscribe from another agent**

Agents consume feeds the same way dashboards do, but the subscription is managed by an extension and the session is opened programmatically.

```rust
pub struct GasConsumerExt {
    gas_subscription: Option<FeedSubscription>,
}

#[async_trait]
impl Extension for GasConsumerExt {
    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let session = ctx.mpp.open_session(
            "gas-oracle",  // agent producing the feed
            500,           // $0.0005 USDC
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
                    ctx.cortical.set_prediction_error(0.8);
                }
            }
        }
        Ok(())
    }
}
```

### Relay feed infrastructure

The relay manages the feed registry, payment gating, and message forwarding. All feed operations go through the relay -- producers publish to it, subscribers connect through it.

**Feed registration** (agent -> relay on boot):

```json
POST /relay/feeds/register
{
  "feed_id": "eth-gas-trend",
  "agent_id": "gas-oracle",
  "kind": "derived",
  "schema": "gas_trend_v1",
  "description": "12-block EMA gas price with percentile bands and MEV detection",
  "rate_hz": 0.5,
  "access": {
    "paid": {
      "base_price_usdc_per_hour": 50,
      "accepted_protocols": ["x402", "mpp"]
    }
  },
  "sample": {"ema_12": 42.5, "p25": 35.0}
}
```

**Feed discovery** (dashboard or agent -> relay):

```
GET /relay/feeds                              # all feeds
GET /relay/feeds?kind=derived&access=paid     # filter by kind and access
GET /relay/feeds?agent_id=gas-oracle          # feeds from a specific agent
GET /relay/feeds/{feed_id}                    # single feed metadata
GET /relay/feeds/{feed_id}/sample             # sample payload (free, no auth)
```

**Feed subscription with payment** (subscriber -> relay WebSocket):

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

The relay verifies the MPP session with the feed producer's agent, then forwards feed data to the subscriber. Each forwarded message triggers a draw.

**Payment flow through the relay**

```
Subscriber                    Relay                     Feed Producer
    │                           │                            │
    │  Open MPP session         │                            │
    │  (ERC-3009 auth)          │                            │
    │ ────────────────────────> │  Store session ref         │
    │                           │ ─────────────────────────> │
    │  Subscribe to feed room   │                            │
    │  with session_id          │                            │
    │ ────────────────────────> │                            │
    │                           │                            │
    │                           │  <── feed_data ──────────  │
    │                           │                            │
    │                           │  Draw from session:        │
    │                           │  cost = base_price         │
    │                           │        / rate_hz / 3600    │
    │                           │                            │
    │                           │  Draw succeeds?            │
    │  <── feed_data ────────── │  Yes: forward              │
    │  <── payment_draw ─────── │                            │
    │                           │                            │
    │                           │  Draw fails (exhausted)?   │
    │  <── exhaustion_notice ── │  Unsubscribe, notify       │
    │                           │                            │
    │  Top-up session           │                            │
    │ ────────────────────────> │  Resume draws              │
    │                           │                            │
    │  Disconnect / unsubscribe │                            │
    │ ────────────────────────> │  Session stays open        │
    │                           │  (reusable on reconnect)   │
```

### Feed types and composability

Feeds are domain-agnostic. Any agent can produce any feed -- blockchain data, ML model outputs, sentiment analysis, code quality metrics, research signals, market indicators, or arbitrary computed streams. The feed system is the same regardless of domain.

Feeds compose into value chains. Each layer adds computation and charges for it.

**Raw feeds** -- direct data ingestion:

- Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket)
- Research: `arxiv-new-papers`, `github-trending` (from web polling)
- Code: `repo-commit-stream`, `ci-build-results` (from webhooks)
- Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs)
- Any external data source an agent consumes can be re-published as a raw feed

**Derived feeds** -- computed from raw:

- Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability`
- Research: `paper-relevance-scores`, `topic-cluster-updates`
- Code: `code-quality-trend`, `dependency-risk-index`
- Market: `volatility-regime`, `cross-venue-spread`
- Any computation an agent performs on its inputs can be a derived feed

**Composite feeds** -- derived from multiple derived feeds:

- `cross-chain-arb-signal` (consumes gas trends + volume + funding rates)
- `research-portfolio-impact` (consumes paper scores + code quality + market sentiment)
- Cost stacks: producer pays for input feeds, charges for output feed

**Meta feeds** -- feeds about feeds:

- `feed-health` (monitors all feeds for staleness, drift, anomalies)
- `feed-accuracy` (tracks prediction accuracy of derived feeds over time)
- Produced by meta-agents

Composition example:

```
eth-mainnet-blocks (free, raw)
  └─> gas-oracle agent
       └─> eth-gas-trend ($0.05/hr, derived)
            └─> arb-bot agent
                 └─> cross-chain-gas-arb ($0.50/hr, composite)
                      └─> dashboard subscriber

arxiv-new-papers (free, raw)
  └─> research-scout agent
       └─> defi-paper-relevance ($0.02/hr, derived)
            └─> strategy-agent subscribes for research context
```

Each agent in the chain pays for its inputs and charges for its output.

### On-chain feed advertisement (ERC-8004)

Agents with wallets advertise their feeds in their ERC-8004 passport. This makes feeds discoverable on-chain even when the agent or relay is offline.

```solidity
// AgentRegistry.sol — feed advertisement extension
struct FeedAdvert {
    bytes32 feedId;        // keccak256 of feed name
    bytes32 schemaHash;    // keccak256 of schema definition
    uint16  rateMilliHz;   // rate in milli-Hz (500 = 0.5 Hz)
    uint96  pricePerHour;  // USDC base units per hour (0 = free)
    uint32  updatedAt;     // last update timestamp
}

function updateFeeds(FeedAdvert[] calldata adverts) external;
function getFeeds(address agent) external view returns (FeedAdvert[] memory);
```

When an agent boots with feeds configured, it:

1. Registers feeds with the relay (for live presence and subscription routing)
2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery)
3. On feed config changes (add/remove/reprice), updates both relay and chain

```rust
// In FeedPublisherExt::on_boot()
async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
    for feed in &self.feeds {
        // Register with relay (live routing)
        ctx.relay.register_feed(/* ... */).await?;

        // Advertise on-chain (persistent discovery)
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
// Dashboard merges relay (live) + chain (persistent) feed data
async function discoverFeeds(): Promise<Feed[]> {
  const [relayFeeds, chainFeeds] = await Promise.all([
    fetch(`${relayUrl}/relay/feeds`).then(r => r.json()),
    chainClient.getRegisteredFeeds(),  // reads all ERC-8004 feed adverts
  ]);

  // Merge: relay has live status, chain has persistent metadata
  return mergeFeeds(relayFeeds, chainFeeds);
  // Result: each feed has { ...chainAdvert, live: boolean, subscribers: number }
}
```

An agent's feeds appear in its passport even when the agent is offline. Users browsing the on-chain registry can see what feeds exist, their pricing, and their schemas -- then subscribe when the agent comes online.

### Dashboard chain feed subscriptions

When the dashboard connects to a blockchain agent, it automatically subscribes to that agent's chain feeds for UI rendering. This is not just data display -- the dashboard uses raw chain feeds to render live blockchain state.

```typescript
// When user opens Agent Detail for a chain agent:
function useAgentChainFeeds(agent: MergedAgent) {
  const feeds = agent.feeds?.filter(f =>
    f.schema.startsWith("eth_") ||
    f.schema.startsWith("evm_") ||
    f.schema === "block" ||
    f.schema === "transaction"
  );

  // Auto-subscribe to chain feeds for live rendering
  for (const feed of feeds ?? []) {
    useRealtimeFeed(feed.feedId, {
      // Chain feeds from a connected agent are rendered as live chain state
      onData: (data) => {
        updateBlockHeight(data.blockNumber);
        updateGasGauge(data.gasUsed);
        updateTransactionList(data.transactions);
      }
    });
  }
}
```

The dashboard renders different UI elements based on feed schema:

| Feed schema | Dashboard renders |
|-------------|-------------------|
| `eth_block` | Block height counter, gas gauge, tx list |
| `evm_logs` | Live event log with contract decode |
| `gas_trend_*` | Gas price sparkline with percentile bands |
| `funding_*` | Funding rate chart with divergence alerts |
| `position_*` | Position cards with P&L |
| `tick_activity_*` | Liquidity heatmap |
| Any custom | Raw JSON viewer with auto-detected chart type |

For non-blockchain feeds, the dashboard renders based on the data shape -- numeric values get sparklines, boolean values get status indicators, arrays get tables.

### Dashboard integration

Three new surfaces: a feed browser, a subscription manager, and a producer revenue panel.

**Feeds page** (in Fleet or System section):

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
|                                                               |
| +----------------------------------------------------------+ |
| | eth-mainnet-blocks                           * LIVE      | |
| | by chain-watcher-1 (Basic)                     FREE     | |
| | Raw Ethereum mainnet block headers                       | |
| | Schema: eth_block   Rate: 0.08 Hz   Subs: 12            | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

**My subscriptions** (in Treasury or Settings):

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

**Feed detail page** (click into a feed):

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

**Feed revenue** (in Treasury / Cost Analytics):

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

### Feed data-source mapping

Extending the page-to-data-source table from the [Dashboard architecture](15-dashboard.md) section:

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Fleet / Feeds | `system` | `feed_registered`, `feed_deregistered`, `feed_status` | `GET /relay/feeds` |
| Fleet / Feed detail | `feed:{id}` | `feed_data`, `feed_status`, `payment_draw` | `GET /relay/feeds/{id}` |
| Treasury / Subscriptions | `system` | `session_opened`, `session_exhausted`, `session_settled` | `GET /mpp/sessions` |
| Treasury / Feed Revenue | `system` | `feed_revenue_update`, `settlement_batch` | `GET /relay/feeds/revenue` |

### Practical example: funding rate divergence feed

A second example showing feed composition. This agent consumes two paid feeds and produces a third.

**Agent manifest**

```toml
[agent]
name = "funding-arb"
profile = "chain"
mode = "persistent"

# This agent CONSUMES two feeds...
[[agent.feed_subscriptions]]
feed_id = "binance-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000  # $0.001 USDC session deposit

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

**The extension**

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

        ctx.cortical.set_feed_data("funding-divergence", json!({
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
        }));

        // Extreme divergence triggers T2 reasoning via prediction error
        if zscore.abs() > 3.0 {
            ctx.cortical.set_prediction_error(0.9);
        }

        Ok(())
    }
}
```

**The value chain**

```
cex-connector produces binance-funding-rates ($0.05/hr)
cex-connector produces hyperliquid-funding-rates ($0.05/hr)
  └─> funding-arb consumes both, pays $0.10/hr
       └─> funding-arb produces funding-divergence ($0.20/hr)
            └─> trading-bot subscribes, pays $0.20/hr
            └─> dashboard subscribes, pays $0.20/hr
```

Economics for `funding-arb`: $0.20/hr revenue per subscriber minus $0.10/hr input cost. With 5 subscribers that's ($0.20 * 5) - $0.10 = $0.90/hr pure margin.

### Extensibility

Any extension can produce a feed. The pattern:

1. Declare the feed in the agent manifest (`[agent.feeds.*]`).
2. Compute data in an extension's `on_observe()` or `on_tick_end()` hook.
3. Store via `ctx.cortical.set_feed_data(feed_id, data)`.
4. `FeedPublisherExt` (auto-loaded when feeds are declared) publishes to the relay.

Users create custom feed schemas, set arbitrary pricing, compose feeds from other feeds. The relay handles discovery, payment gating, and delivery. The dashboard surfaces it all without any feed-specific code -- it reads the schema from the registry and renders accordingly.
