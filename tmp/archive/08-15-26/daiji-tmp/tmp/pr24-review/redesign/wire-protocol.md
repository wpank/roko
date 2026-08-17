# Wire Protocol

## Connection

Single WebSocket endpoint: `wss://relay.daeji.local:9011/ws`

All agent types use the same endpoint. Room subscriptions determine what you receive.

## Frames

### Hello (first frame, agent → relay)

```json
{
  "type": "hello",
  "agent_id": "roko-alpha-1",
  "name": "Alpha Research Agent",
  "capabilities": ["reasoning", "coding"],
  "chain_identity": {
    "address": "0xabc...",
    "agent_id_8004": 42
  },
  "card_uri": "https://agent.example.com/.well-known/agent.json",
  "feeds": [
    {
      "feed_id": "eth-gas-trend",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": "public"
    }
  ],
  "subscribe": ["system", "chain:nunchi"]
}
```

Combines registration, feed registration, and initial subscriptions in one frame.

### Ack (relay → agent)

```json
{
  "type": "ack",
  "seq": 0,
  "ts": 1713974400000,
  "your_id": "roko-alpha-1",
  "rooms": ["system", "chain:nunchi"]
}
```

### Subscribe / Unsubscribe

```json
{ "type": "subscribe", "rooms": ["group:job-42", "feed:eth-gas-trend:data"] }
{ "type": "unsubscribe", "rooms": ["group:job-42"] }
```

### Publish (agent → relay → subscribers)

```json
{
  "type": "publish",
  "room": "group:job-42",
  "event_type": "group.message",
  "payload": {
    "kind": "partial_result",
    "content_hash": "0x...",
    "confidence": 0.82
  }
}
```

Relay wraps in standard envelope with `seq`, `ts`, `from`, delivers to subscribers.

### Direct message (point-to-point, request/response)

```json
{
  "type": "direct",
  "to": "roko-beta-2",
  "message_id": "msg-abc123",
  "payload": { "type": "ping" },
  "timeout_ms": 5000
}
```

Response:

```json
{
  "type": "response",
  "message_id": "msg-abc123",
  "payload": { "type": "pong", "uptime_ms": 86400000 }
}
```

### Resume (reconnection)

```json
{ "type": "resume", "last_seq": 4821 }
```

If gap ≤ ring buffer: relay replays missed messages.
If gap > ring buffer: relay sends snapshot.

### Snapshot (relay → agent, on large reconnection gap)

```json
{
  "type": "snapshot",
  "seq": 71042,
  "state": {
    "agents": [{ "id": "coder-1", "online": true }],
    "groups": [{ "id": "job-42", "members": ["roko-alpha-1", "roko-beta-2"] }],
    "feeds": [{ "feed_id": "eth-gas-trend", "agent_id": "gas-oracle" }],
    "your_rooms": ["system", "chain:nunchi", "group:job-42"]
  }
}
```

### Feed register / deregister

```json
{ "type": "feed_register", "feed_id": "eth-gas-trend", "kind": "derived", "schema": "gas_trend_v1", "rate_hz": 0.5, "access": "public" }
{ "type": "feed_deregister", "feed_id": "eth-gas-trend" }
```

### Keepalive

```json
{ "type": "ping" }
{ "type": "pong" }
```

## HTTP endpoints

For non-WebSocket callers:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Relay health |
| `/agents` | GET | Connected agents with capabilities |
| `/cards/{agent_id}` | GET | Agent card (A2A format) |
| `/feeds` | GET | Registered feeds with schema + pricing |
| `/groups` | GET | Active groups |
| `/groups/{id}` | GET | Group details + members |
| `/messages` | POST | Direct message (request/response with timeout) |
| `/ws` | GET | WebSocket connection |

## Backpressure strategies

| Room pattern | Strategy | Behavior |
|-------------|----------|----------|
| `agent:*:heartbeat` | Coalesce | Send latest per agent every 500ms |
| `agent:*:output` | Drop-oldest | Ring buffer (1024). Slow consumers miss old. |
| `feed:*:data` | Sample | Every Nth update where N = ceil(source_rate / 2Hz) |
| `group:*` | Lossless | Queue with TCP backpressure on overflow |
| `chain:*` | Lossless | Queue with TCP backpressure on overflow |

## Auth model

| Action | PoC | Production |
|--------|-----|------------|
| Connect | Open | API key or ERC-8004 identity |
| Subscribe to public rooms | Open | Open |
| Subscribe to group rooms | Open | Membership verification |
| Subscribe to paid feeds | N/A | x402 payment verification |
| Publish to group rooms | Open | Membership verification |
| Direct message | Open | Authenticated connection |

## Frame summary

| Frame | Direction | Purpose |
|-------|-----------|---------|
| `hello` | Agent → Relay | Register, announce feeds, initial subscriptions |
| `ack` | Relay → Agent | Confirm hello |
| `subscribe` | Agent → Relay | Subscribe to topics |
| `unsubscribe` | Agent → Relay | Unsubscribe from topics |
| `publish` | Agent → Relay | Send to topic subscribers |
| `direct` | Agent → Relay | Point-to-point request |
| `response` | Agent → Relay | Point-to-point response |
| `resume` | Agent → Relay | Reconnection replay request |
| `snapshot` | Relay → Agent | Full state on large gap |
| `feed_register` | Agent → Relay | Register a feed |
| `feed_deregister` | Agent → Relay | Remove a feed |
| `error` | Relay → Agent | Error notification |
| `ping`/`pong` | Both | Keepalive |
