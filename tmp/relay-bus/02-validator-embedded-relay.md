# Validator-Embedded Relay

## Context

Daeji is designed as an agent coordination chain. The question is whether validators should also serve as relay operators, and if so, what that looks like architecturally.

This document specs two modes: a **minimal chain-event projector** embedded in the validator, and a **full relay** embedded as a supervised task (like the existing `daeji-chat` module).

## Precedents

| System | Pattern | What's embedded | What's separate |
|---|---|---|---|
| Ethereum + MEV-Boost | Sidecar | Nothing (consensus only) | MEV relay is separate binary |
| Cosmos + Slinky (dYdX) | Hybrid | Vote extension handler (consensus-critical) | Price fetcher/aggregator (sidecar via gRPC) |
| Solana | Strict separation | Consensus only | RPC is separate infrastructure |
| Jito-Solana | Modified validator | RelayerStage, BlockEngineStage | Relayer is separate server |
| Daeji + daeji-chat (current) | Embedded supervised task | Chat service (commonware-p2p) | Nothing |

The existing daeji-chat embedding proves the pattern works: a non-consensus service runs as a supervised tokio task inside kora with its own network port, isolated from consensus. A chat panic doesn't crash the validator.

## Why Embed at All?

Two concrete advantages over standalone:

1. **Zero-latency chain events.** A validator-embedded relay reads finalized blocks directly from `LedgerService::subscribe()` — the internal channel that fires immediately after a block is finalized. A standalone relay goes through the JSON-RPC layer, adding 50-200ms. For ISFR keepers competing on rate freshness, this matters.

2. **Single deployment.** Validators already run kora. Adding `--relay-port 9011` is one flag. No separate binary, no separate process supervision, no separate deployment config. For a small validator set (3-4 nodes), this eliminates operational overhead.

## Why NOT Embed?

1. **Resource contention.** At 400ms block time, the consensus path is tight. Pathological relay load (thousands of agents reconnecting simultaneously) could spike CPU/memory during consensus catch-up.

2. **Coupled lifecycle.** If the validator restarts for a chain upgrade, the relay goes down too. Agents lose their WebSocket connections and ring buffer state.

3. **Upgrade cadence.** Relay protocol changes (new frame types, topic grammar changes) shouldn't require a chain upgrade. With embedding, they do.

4. **Scale ceiling.** As the agent ecosystem grows, relay traffic will vastly exceed consensus traffic. Dedicated relay infrastructure is the long-term answer, just as dedicated RPC providers (Alchemy, Infura) emerged for Ethereum.

## Design: Two Modes

### Mode A: Minimal — Chain Event Projector Only

The validator embeds only the chain watcher component. It subscribes to `LedgerService::subscribe()` for finalized blocks, decodes ERC-8004/8183/ISFR contract events, and publishes them on an internal tokio broadcast channel.

The actual WebSocket relay runs as a **separate process** that reads from this channel (via a local Unix socket or shared memory pipe) instead of going through JSON-RPC.

```
┌────────────────────────────────┐    ┌─────────────────────────┐
│        kora validator          │    │    relay (separate)     │
│                                │    │                         │
│  Simplex BFT                   │    │  WebSocket handler      │
│  QMDB                          │    │  TopicBus               │
│  Transaction pool               │    │  Agent directory        │
│  RPC server                    │    │  Feed registry          │
│                                │    │                         │
│  ┌──────────────────────────┐  │    │  ┌───────────────────┐  │
│  │  Chain Event Projector   │──────▶│  │  Chain watcher     │  │
│  │  (LedgerService sub)    │  │    │  │  (reads from pipe) │  │
│  └──────────────────────────┘  │    │  └───────────────────┘  │
└────────────────────────────────┘    └─────────────────────────┘
```

**Embedded code: ~100 lines.** Subscribes to LedgerService, decodes events, publishes to a Unix socket.

**Advantage:** Validator has zero relay exposure. No WebSocket server, no agent connections, no ring buffer memory.

**Disadvantage:** Still need a separate relay process. The only gain is lower-latency chain events.

### Mode B: Full Relay as Supervised Task

The full relay (bus, protocol, state, chain watcher, WebSocket handler) runs inside kora as a supervised tokio task, exactly like `daeji-chat` today.

```
┌──────────────────────────────────────────────────────┐
│                    kora validator                      │
│                                                        │
│  Simplex BFT ─────────── consensus port (p2p)         │
│  QMDB                                                 │
│  Transaction pool                                      │
│  RPC server ──────────── rpc port (HTTP/WS)           │
│  Chat service ─────────── chat port (p2p)              │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Relay Service (supervised)                       │  │
│  │                                                    │  │
│  │  WebSocket handler ── relay port (WS, e.g. 9011)  │  │
│  │  HTTP API ─────────── relay port (HTTP)            │  │
│  │  TopicBus (in-memory ring buffers)                 │  │
│  │  Agent directory + feed registry                   │  │
│  │  Chain watcher (LedgerService → internal channel)  │  │
│  │                                                    │  │
│  │  Isolation:                                        │  │
│  │  - Own axum listener (separate port from RPC)      │  │
│  │  - No shared state with consensus                  │  │
│  │  - PanicTracker + exponential backoff restart      │  │
│  │  - DAEJI_RELAY_DISABLED env var kill switch        │  │
│  └──────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

**Embedded code: ~1,200 lines** (the full `daeji-relay` library crate).

**CLI:**

```bash
# Enable relay on validator
kora validator --relay-port 9011

# Disable at runtime
export DAEJI_RELAY_DISABLED=1
kill -USR1 $KORA_PID
```

**Isolation guarantees (same as daeji-chat):**
- Separate network port (axum, not commonware-p2p)
- No shared data structures with consensus
- PanicTracker: relay panic → log + restart with backoff, not validator crash
- Kill switch via env var + SIGUSR1
- Separate tokio task, not blocking the consensus event loop

## Implementation: Library + Binary

The relay should be a library crate that both kora and a standalone binary can use:

```
crates/network/daeji-relay/
├── src/
│   ├── lib.rs           # pub fn run_relay(config, chain_events?) → JoinHandle
│   ├── bus.rs           # TopicBus (ring buffer + pub/sub)
│   ├── protocol.rs      # Frame types
│   ├── state.rs         # Agent/feed/workspace directory
│   ├── chain.rs         # Chain event adapter (alloy OR LedgerService)
│   └── server.rs        # Axum routes + WebSocket handler
└── Cargo.toml
```

**Standalone binary:**

```rust
// bin/relay/src/main.rs (~30 lines)
fn main() {
    let config = RelayConfig::from_cli();
    let chain = AlloyChainWatcher::new(config.rpc_url, config.chain_id);
    daeji_relay::run_relay(config, Some(chain)).await;
}
```

**Kora embedding:**

```rust
// In LegacyNodeService or equivalent
fn with_relay(mut self, config: RelayConfig) -> Self {
    let chain = LedgerChainWatcher::new(self.ledger_service.subscribe());
    let handle = daeji_relay::run_relay(config, Some(chain));
    self.relay_handle = Some(supervised(handle, PanicTracker::new()));
    self
}
```

The chain watcher trait abstracts over the event source:

```rust
trait ChainEventSource: Send + Sync + 'static {
    fn subscribe(&self) -> Receiver<ChainEvent>;
}

// Standalone: AlloyChainWatcher (JSON-RPC subscription)
// Embedded: LedgerChainWatcher (direct LedgerService channel)
```

## Validator Incentives for Running Relay

| Incentive | Mechanism | When |
|---|---|---|
| Lower-latency chain events for own agents | Direct LedgerService access | Now |
| Simpler deployment | `--relay-port` flag | Now |
| Agent connection fees | Future: rate-limited paid access | Phase 2 |
| Block building advantage | Future: relay-mediated agent txns | Phase 3 |
| Reputation | Future: uptime attestations from connected agents | Phase 3 |

## Non-Validator Relay Operators

Anyone with a chain RPC endpoint can run a standalone relay. Non-validator relays are first-class:

- Same codebase, same binary
- Use `AlloyChainWatcher` instead of `LedgerChainWatcher`
- ~50-200ms higher latency on chain events (RPC hop)
- No consensus responsibilities
- Can specialize (geographic locality, specific topic subsets, premium SLAs)

As the agent ecosystem grows, dedicated relay operators will emerge — analogous to Alchemy/Infura for Ethereum RPC. Validators will focus on consensus and may stop running public relays.

## Recommendation

**Start with Mode B (full embedded) for the current validator set (3-4 nodes).** The operational simplicity of `kora --relay-port 9011` outweighs the coupling concerns at this scale. The isolation pattern is proven by daeji-chat.

**Build as a library crate** so the standalone binary is always available. When the validator set grows or relay traffic increases, operators can switch to standalone deployment without code changes.

**Do not make relay mandatory for consensus.** It's an optional service flag, like `--chat-config`. Validators that crash should not affect the relay — the multi-relay connection model (agents connect to 2-3 relays) provides redundancy.

## Comparison: Chat (PR #24) vs Relay

| Dimension | daeji-chat (PR #24) | Relay |
|---|---|---|
| Transport | commonware-p2p (validator mesh) | WebSocket (axum) |
| Language | Rust-only | Any (WebSocket + JSON) |
| NAT | Hostile (needs dialable addr) | Friendly (outbound connections) |
| Channels | 64 pre-allocated slots | Dynamic topics (unlimited) |
| Encryption | ChaCha20Poly1305 per room | None (application-level if needed) |
| Message types | Typed enum (Hello/Status/PartialResult/Vote/Final) | Opaque (relay routes by topic, ignores payload) |
| Coordination | Baked-in symphony pattern | Any pattern (agents decide) |
| Peers | File-backed registry, chain polling | WebSocket presence, relay directory |
| Embedding | Replaces commonware-p2p mesh with relay WebSocket | |

The relay subsumes all of chat's use cases. The typed message protocol (PartialResult/Vote/Final) is an application-level concern that agents implement over relay topics, not a transport concern.

Keep the AEAD crypto primitives (ChaCha20Poly1305, room key derivation from `keccak256("DAEJI_ROOM_V1" || jobId)`) if encrypted rooms are ever needed. Drop the transport (commonware-p2p mesh), the 64-slot pool, the typed message enum, and the kora embedding.
