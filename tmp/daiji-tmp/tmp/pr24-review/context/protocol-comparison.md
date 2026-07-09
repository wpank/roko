# Protocol Comparison: daeji-chat vs Alternatives

## Feature matrix

| Feature | daeji-chat (PR #24) | Matrix | Waku | GossipSub | OpenClaw A2A |
|---------|-------------------|--------|------|-----------|-------------|
| **Transport** | commonware-p2p mesh | HTTP + WebSocket | libp2p | libp2p | HTTP |
| **Topology** | Full mesh (O(n²)) | Client-server + federation | Gossip mesh | Gossip mesh | Point-to-point |
| **Discovery** | File-based registry | Homeserver directory | DHT + ENR | DHT + bootstrap | .well-known/agent-card |
| **Encryption** | ChaCha20Poly1305 (AEAD) | Olm/MegOlm (double ratchet) | Noise (libp2p) | None (plaintext) | TLS |
| **Persistence** | None | Full history (homeserver) | Store protocol | None | None |
| **Language** | Rust only | Any (HTTP/WS API) | Rust, JS, Go, C | Rust, Go, JS | Any (HTTP) |
| **NAT** | None (needs dialable_addr) | Homeserver handles | Relay + hole-punch | Relay | Standard HTTP |
| **Auth** | ed25519 transport key | Username/password or SSO | Noise handshake | Signed peer records | API keys / OAuth |
| **Channels** | 64 pre-allocated | Unlimited rooms | Content topics | Unlimited topics | N/A |
| **Message format** | Typed enum (Hello/Status/Vote/Final) | JSON events | Protobuf | Arbitrary bytes | JSON-RPC |
| **Group messaging** | Rooms (broadcast) | Rooms | Content topics | Topics | N/A (point-to-point) |
| **Ordering** | Unordered | Server-ordered | Unordered | Unordered | Request/response |
| **Backpressure** | None | Server-side | Rate limiting | Flood/score | N/A |
| **Federation** | None | Server-to-server | Peer-to-peer | Peer-to-peer | None |

## What to adopt from each

**From Matrix:** Homeserver-style relay that handles NAT, persistence (bounded ring buffer),
room management, and multi-language access via HTTP/WS API.

**From Waku:** Content-topic addressing (messages routed by topic, not channel ID). Store
protocol for bounded history replay.

**From GossipSub:** Topic-based pub/sub model. Backpressure via scoring. The topology (gossip
mesh) is not needed — the relay is the routing point.

**From A2A:** Agent Card discovery (`.well-known/agent-card.json`). HTTP API for non-WebSocket
callers. Point-to-point request/response with timeout.

## Key insight

daeji-chat's commonware-p2p mesh is the wrong tool for agent coordination. The relay pattern
(Matrix homeserver / roko relay) with topic-based pub/sub (Waku/GossipSub) and agent card
discovery (A2A) is the right combination.

## commonware-p2p constraints

commonware-p2p is designed for validator consensus — a small, fixed set of known peers needing
authenticated, high-throughput, low-latency channels.

Key constraints:
- **Channels must be registered before `network.start()`.** Cannot add channels dynamically.
- **Full mesh.** O(n²) connections. Doesn't scale beyond ~100 peers.
- **Rust-only.** `Sender`/`Receiver` types require commonware runtime.
- **No NAT traversal.** Peers must have dialable addresses.
- **Pre-configured peer set.** `oracle.track()` with known public keys.

Right for validator-to-validator consensus. Wrong for dynamic agent coordination.
