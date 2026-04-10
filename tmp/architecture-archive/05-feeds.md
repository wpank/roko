# Feeds and data streams

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> See [Paid Feeds and MPP](06-paid-feeds.md) for payment protocols, pricing, and marketplace economics.

---

> **Universal primitive (PRD 23).** Feed is one of the 12 universal primitives in the canonical vocabulary. Its shape is `subscribe / unsubscribe / poll / status / configure`. A Feed uses a Connector as its source and publishes processed events to PulseBus. Feeds are the "always-on" complement to one-shot Connector queries. The architecture below already supports this universal definition -- blockchain feeds are the most data-intensive case, but the same pattern applies to CI status feeds, file change feeds, webhook streams, and any other continuous data source.

Any agent can produce and consume real-time data feeds. Blockchain agents subscribe to RPC WebSocket feeds; research agents ingest papers and web data; coding agents emit build status and metrics. The feed system is domain-agnostic -- the examples below use blockchain because that is the most data-intensive case, but the same extension pattern works for any data source.

### Blockchain: raw feed subscription

### Raw feed subscription

```rust
// Inside a ChainReaderExt extension
impl ChainReaderExt {
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Subscribe to Ethereum mainnet new blocks
        let eth_blocks = ctx.chain_subscribe(
            "ethereum",
            "newHeads",
            json!({}),
        ).await?;

        // Subscribe to Uniswap V3 swap events on Base
        let base_swaps = ctx.chain_subscribe(
            "base",
            "logs",
            json!({
                "address": "0x...",  // Uniswap V3 factory
                "topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"]
            }),
        ).await?;

        self.feeds.push(eth_blocks);
        self.feeds.push(base_swaps);
        Ok(())
    }
}
```

### Exposing feeds externally

Any agent -- regardless of domain -- can register feeds with the relay. The relay handles discovery, subscription routing, and payment gating. The agent just publishes data; the relay does the rest.

```rust
// Agent publishes a raw feed
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-mainnet-blocks",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Raw,
    schema: FeedSchema::EthBlock,
    rate_hz: 0.08,  // ~1 block per 12s
    access: FeedAccess::Public,
})?;

// Agent publishes a derived feed (computed from raw data)
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-gas-trend",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("gas_trend_v1"),
    rate_hz: 0.5,
    access: FeedAccess::Paid { price_per_hour: 100 },  // 100 units/hr
})?;
```

### Feed discovery

The relay maintains a feed registry. Dashboards and agents discover feeds dynamically.

```
GET /relay/feeds
→ [
    {
      "feed_id": "eth-mainnet-blocks",
      "agent_id": "chain-watcher-1",
      "kind": "raw",
      "schema": "eth_block",
      "rate_hz": 0.08,
      "access": "public",
      "subscribers": 3
    },
    {
      "feed_id": "eth-gas-trend",
      "agent_id": "chain-watcher-1",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": { "paid": { "price_per_hour": 100 } },
      "subscribers": 1
    }
  ]
```

### Feed registry pagination

`GET /relay/feeds` supports cursor-based pagination and filtering.

**Query parameters**:

```
Parameter    Type      Default   Description
─────────    ────      ───────   ───────────
limit        u32       50        Results per page. Max: 200.
cursor       string    (none)    Opaque cursor from previous response's next_cursor.
kind         string    (none)    Filter by feed kind: "raw" or "derived".
access       string    (none)    Filter by access type: "public" or "paid".
agent_id     string    (none)    Filter to feeds from a specific agent.
schema       string    (none)    Filter by schema name (exact match).
search       string    (none)    Full-text search across feed_id and description.
```

**Example**: Get the second page of paid derived feeds:

```
GET /relay/feeds?kind=derived&access=paid&limit=20&cursor=eyJsYXN0IjoiZXRoLWdhcy10cmVuZCJ9

→ {
    "feeds": [ ... ],
    "next_cursor": "eyJsYXN0IjoiYnRjLXZvbC1pbmRleCJ9",
    "total": 47
  }
```

**Cursor format**: Opaque base64-encoded JSON. Clients must not parse or construct cursors -- treat them as opaque strings. The relay uses the last `feed_id` as the cursor anchor (keyset pagination, not offset). This stays stable when feeds are added or removed between pages.

When `next_cursor` is `null`, there are no more results.

### Dashboard subscribes to agent chain feeds

```typescript
function useChainFeed(agentId: string, feedId: string) {
  const [data, setData] = useState<FeedData[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);
    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}:feed:${feedId}`]
      }));
    };
    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      if (event.type === "feed_data") {
        setData(prev => [...prev.slice(-999), event.payload]);
      }
    };
    return () => ws.close();
  }, [agentId, feedId]);

  return data;
}
```

### Agent-to-agent feed marketplace

Agents can subscribe to each other's feeds, creating a data marketplace:

```rust
// Agent B subscribes to Agent A's derived feed
let subscription = ctx.relay.subscribe_feed(SubscribeFeedRequest {
    feed_id: "eth-gas-trend",
    source_agent_id: "chain-watcher-1",
})?;

// For paid feeds, payment is handled automatically via the inference gateway's
// cost tracking. The subscribing agent's budget is debited per hour.
```

### Dynamic endpoint registration

Agents can register new endpoints at runtime. When an agent discovers a new data source or creates a derived feed, it announces it to the relay.

```rust
// Agent discovers a new DEX and creates a feed for it
ctx.relay.register_feed(FeedRegistration {
    feed_id: format!("dex-{}-swaps", dex_address),
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("dex_swap_v1"),
    rate_hz: 2.0,
    access: FeedAccess::Public,
})?;

// The dashboard discovers this feed dynamically
// because it subscribes to the "system" room and receives
// feed_registered events
```

---

## Recipes (universal primitive)

> Added 2026-04-24. Per dashboard PRD 23, Recipe is a first-class primitive in the 12-primitive vocabulary.

A Recipe is a composable data transformation pipeline. Its shape is `create / execute / chain / status / configure`. Recipes are the composition glue between Feeds, Scorers, and Signals -- they turn raw data streams into derived values.

### Why Recipe is distinct from Plan and Composer

A Plan is an agent task DAG (tasks with dependencies, assigned to agents). A Composer assembles prompts for LLM calls. A Recipe is a pure data pipeline: Feed -> transform -> transform -> output. No LLM inference, no agent dispatch. Recipes make indicator chains, P&L attribution, HDC encoding, and scoring pipelines into first-class composable objects that users can author, share, and backtest.

### Cross-domain examples

| Domain | Recipe | Pipeline |
|--------|--------|----------|
| DeFi | Indicator pipeline | price feed -> MACD -> RSI -> regime score -> trading signal |
| DeFi | P&L attribution | fill events -> FIFO matching -> realized P&L -> Sharpe ratio |
| DeFi | HDC encoding | market state -> role-filler binding -> hypervector -> similarity search |
| Governance | Voting power | delegation graph -> power aggregation -> threshold signal |
| Ops | Cost tracking | agent token usage -> pricing -> daily/weekly rollup -> budget signal |
| Code | Quality scoring | diff -> lint warnings -> test coverage -> quality score |

### Rust mapping

Recipes compose `Scorer` trait instances from `roko-core`. Existing implementations that become recipe templates:

| Crate | Struct | Recipe type |
|-------|--------|-------------|
| `roko-learn` | `TradingReflect` | P&L attribution pipeline |
| `roko-learn` | `FifoMatcher` | Position matching transform |
| `roko-learn` | `IndicatorTracker` | Indicator accuracy pipeline |
| `roko-primitives` | `MarketHdcEncoder` | HDC encoding pipeline |
| `roko-dreams` | `CounterfactualEngine` | Alternative outcome simulation |

### Dashboard authoring surface

The Recipe Editor is a 4-stage authoring surface (per PRD 23):

1. **Input selection** -- choose feed(s) or connector query as input (drag from feed/connector list)
2. **Pipeline builder** -- chain transform stages: map, filter, window, aggregate, score (visual DAG editor)
3. **Output configuration** -- emit as: Signal, Knowledge Entry, Feed, or raw value (type-checked output)
4. **Backtest and validate** -- run against historical data, compare output distribution (chart overlay)
