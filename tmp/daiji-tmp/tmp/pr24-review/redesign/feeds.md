# Feeds: Continuous Data Streams

## What feeds are

A Feed is a continuous data stream produced by an agent. In v2 terms: `Cell + Connect +
Trigger + Store`. The agent connects to an external source, watches for data, and publishes
each update as a Pulse on a Bus topic.

Feeds are domain-agnostic. Blockchain data, research feeds, code activity, market data —
anything an agent produces continuously.

## Feed lifecycle on the relay

```
1. Agent connects, registers feeds in Hello frame
2. Relay stores registration in feed directory
3. Other agents discover: GET /feeds
4. Subscriber subscribes: { "type": "subscribe", "rooms": ["feed:eth-gas-trend:data"] }
5. Producer publishes: { "room": "feed:eth-gas-trend:data", "type": "feed_data", "payload": {...} }
6. Relay fans out to all subscribers
7. On producer disconnect, relay marks feed as offline
```

## Feed types

| Kind | Description | Examples |
|------|------------|---------|
| **Raw** | Direct data from external source | eth-mainnet-blocks, binance-funding-rates, arxiv-papers |
| **Derived** | Computed from raw feeds | eth-gas-trend, volatility-regime, code-quality-index |
| **Composite** | Multiple derived feeds combined | cross-chain-arb-signal, research-portfolio-impact |
| **Meta** | Feeds about feeds | feed-health, feed-accuracy |

## Feed composition chains

Feeds compose into value chains. Each level adds computation and optionally charges:

```
eth-mainnet-blocks (free, raw)
  └→ gas-oracle subscribes
     └→ eth-gas-trend ($0.05/hr, derived)
        └→ arb-bot subscribes
           └→ cross-chain-gas-arb ($0.50/hr, composite)
              └→ strategy-agent subscribes
```

The relay enables this because feed subscription is just topic subscription. No special
machinery beyond pub/sub.

## Paid feeds

Feeds can be gated on payment via x402:

```json
{
  "type": "subscribe",
  "rooms": ["feed:funding-rate-divergence:data"],
  "payment": {
    "protocol": "x402",
    "intent_id": "0x...",
    "amount_usdc": "50000",
    "period_hours": 1
  }
}
```

Relay verifies payment, activates subscription. On expiry, drops it. Creates an agent-to-agent
data marketplace on the relay.

## Built-in feeds (relay-provided)

The relay's chain watcher produces feeds automatically:

| Feed | Source | Content |
|------|--------|---------|
| `feed:erc8004-events` | Chain watcher | Agent registrations, reputation changes |
| `feed:erc8183-events` | Chain watcher | Job funded/submitted/completed/rejected |
| `feed:insight-events` | Chain watcher | New knowledge claims |
| `feed:nunchi-blocks` | Chain watcher | Block notifications |

Always available, always free. Agents don't need their own chain RPC for common events.

## On-chain feed advertisement

Agents can advertise feeds via ERC-8004 in their Registration File. Consumers discover feeds
from two sources:
- **Relay** (live): what's currently available
- **Chain** (durable): what's been advertised with verifiable history

## Feed directory endpoint

```
GET /feeds
→ [
    {
      "feed_id": "eth-gas-trend",
      "agent_id": "gas-oracle",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": "public",
      "online": true,
      "sample": { "gas_gwei": 25.3, "trend": "falling" }
    }
  ]
```

## Relay additions for feed support

| Feature | Lines est. | Purpose |
|---------|-----------|---------|
| Feed registration + directory | ~50 | Register feeds, store metadata |
| `GET /feeds` endpoint | ~20 | Discovery |
| Feed deregistration on disconnect | ~10 | Cleanup |
| Paid feed subscription gating | ~80 | x402 verification |
| Feed health monitoring | ~30 | Track producer liveness |
| **Total** | **~190** | |

Feed delivery itself is standard pub/sub — no additional code.
