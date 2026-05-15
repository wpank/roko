# Complete Relay Architecture

## Architecture diagram

```
┌──────────────────────────────────────────────────────────────┐
│                       daeji relay                            │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  WebSocket   │  │    HTTP      │  │   Chain Watcher  │   │
│  │  Handler     │  │    API       │  │                  │   │
│  │              │  │              │  │  ERC-8004 events │   │
│  │  /ws         │  │  /agents     │  │  ERC-8183 events │   │
│  │  hello       │  │  /feeds      │  │  InsightBoard    │   │
│  │  subscribe   │  │  /groups     │  │  Block events    │   │
│  │  publish     │  │  /cards/{id} │  │                  │   │
│  │  direct      │  │  /messages   │  │  Publishes as    │   │
│  │  resume      │  │  /health     │  │  Pulses on Bus   │   │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │                 │                    │              │
│  ┌──────┴─────────────────┴────────────────────┴──────────┐  │
│  │                    Internal Bus                         │  │
│  │                                                         │  │
│  │  Topics:                                                │  │
│  │    system              Agent lifecycle, provider status  │  │
│  │    agent:{id}          Per-agent lifecycle               │  │
│  │    agent:{id}:*        Heartbeat, output, feeds          │  │
│  │    feed:{id}:data      Feed data streams                 │  │
│  │    group:{id}          Group broadcast                   │  │
│  │    group:{id}:*        Coordination, knowledge, phero    │  │
│  │    chain:{chain_id}    Chain events                      │  │
│  │                                                         │  │
│  │  Ring buffer: 64K entries per connection                 │  │
│  │  Backpressure: coalesce / drop-oldest / lossless / sample│  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Agent       │  │  Feed        │  │  Group           │   │
│  │  Registry    │  │  Directory   │  │  Registry        │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │          Optional: Auth + Payment Layer               │    │
│  │  ERC-8004 verification · x402 gating · API keys       │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
     │              │              │              │
  roko agent    claude MCP     openclaw       python bot
  (Rust, WS)    (Python)       (any lang)     (any lang)
```

## File structure

```
daeji-relay/
├── src/
│   ├── main.rs            # 30 lines  — CLI args, start server
│   ├── server.rs          # 180 lines — axum routes + WebSocket handler
│   ├── protocol.rs        # 120 lines — Frame types, envelope format
│   ├── bus.rs             # 150 lines — Topic pub/sub, ring buffer, backpressure
│   ├── state.rs           # 100 lines — Agent registry, connection state
│   ├── feeds.rs           # 80 lines  — Feed directory, registration, discovery
│   ├── groups.rs          # 100 lines — Group lifecycle, chain-driven create/close
│   ├── chain.rs           # 200 lines — ERC-8004/8183 watcher, event → Pulse
│   ├── discovery.rs       # 80 lines  — Merged agent view, A2A card fetch
│   ├── auth.rs            # 60 lines  — Identity verification, payment gating
│   └── crypto.rs          # 50 lines  — Optional AEAD per-room (reuse from PR)
├── Cargo.toml             # axum, tokio, serde_json, alloy, dashmap
```

**Total: ~1,150 lines.** vs current PR's ~2,000+ lines with dramatically more capability.

## Capability comparison

| Capability | Current PR | Redesigned relay |
|-----------|-----------|-----------------|
| Language support | Rust only | Any (WebSocket + JSON) |
| NAT traversal | None | Built-in (outbound WS) |
| Data feeds | Not supported | First-class |
| Paid feeds | Not possible | x402 gating |
| Pheromone coordination | Not possible | Stigmergic mode |
| Pipeline coordination | Not possible | Pipeline mode |
| Chain event delivery | Not supported | Built-in chain watcher |
| Agent discovery | File polled 200ms | 4-source merged |
| Reconnection | None | Resume + ring buffer |
| Backpressure | None | Per-topic strategies |
| Coordination patterns | Symphony only | 4 modes |
| Message format | Typed enum | Opaque JSON |
| Topology | Full mesh O(n²) | Star (agents → relay) |

## Deployment options

### Standalone binary

```bash
daeji-relay --bind 0.0.0.0:9011 --chain-rpc ws://localhost:8545
```

### Embedded in kora

```bash
kora validator --relay-port 9011
```

### Multiple relays

For HA or geographic distribution. Chain is shared state. Agents reconnect to nearest.

## Migration from current PR

| Component | Action |
|-----------|--------|
| chain.rs (alloy subscription) | Adapt — change targets to ERC-8004/8183 |
| room.rs (AEAD) | Reuse — same ChaCha20Poly1305 |
| room.rs (room_id derivation) | Reuse — same keccak256 |
| card.rs (StatusCard) | Adapt — align with A2A format |
| messages.rs (typed variants) | Drop — opaque JSON payloads |
| lobby.rs | Drop — Bus topics replace |
| service.rs (commonware mesh) | Drop — WebSocket relay replaces |
| supervisor.rs | Drop — standard process supervision |
| registry.rs (file-based) | Drop — chain + relay state replaces |
| All commonware-p2p usage | Drop — not needed for agent coordination |

## What changes fundamentally

- **Transport**: commonware-p2p mesh → WebSocket relay (star topology)
- **Discovery**: file-based registry → chain + relay + A2A + deployment list
- **Messages**: typed enum → opaque JSON with standard envelope
- **Channels**: 64 pre-allocated → dynamic topics
- **Participation**: Rust-only → any language
- **Coordination**: symphony-only → stigmergic, pipeline, broadcast, leader-follower
- **Data**: messaging-only → messaging + feeds + chain events
- **Economics**: none → paid feeds + job escrow + service payments
- **Reconnection**: none → resume protocol with ring buffer

## V2 alignment

Every design decision traces back to v2 primitives:

- Topics = Bus topics
- Envelopes = Pulse envelopes
- Feeds = Cell + Connect + Trigger + Store, delivered via Bus
- Groups = Space + Membership + CoordinationMode, using Bus partitions
- Chain events = Store → Bus projection
- Pheromones = Signals in Store, notified via Pulses on Bus
- Discovery = Relay + A2A + ERC-8004 + deployment list

**The relay IS the v2 Bus. The chain IS the v2 Store. Daeji provides both.**
