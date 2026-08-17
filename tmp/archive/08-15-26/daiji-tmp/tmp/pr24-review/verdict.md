# Verdict and Recommendations

## Assessment

PR #24 delivers working, well-tested chat code. 53/53 tests pass. 3-agent AEAD-encrypted
e2e works. The code quality is high.

**But the architecture is wrong.** It uses commonware-p2p (a validator consensus transport)
for agent coordination (a dynamic pub/sub problem). The result is Rust-only, NAT-hostile,
pre-allocated channels, no API surface, embedded in kora, full mesh O(n²). None of these
properties are desirable for agent coordination.

## Recommendation: Don't merge as-is. Redesign as relay.

The relay-first architecture (~1,150 lines) provides dramatically more capability than the
current 2,000+ line implementation:

- **Language-agnostic** — any agent that speaks WebSocket + JSON
- **NAT-friendly** — outbound connections, no dialable_addr needed
- **Dynamic topics** — no 64-slot ceiling
- **Data feeds** — first-class continuous data streams
- **Four coordination modes** — not locked to symphony pattern
- **Chain-driven lifecycle** — auto-create/close groups from ERC-8183 events
- **Reconnection** — resume protocol with ring buffer replay
- **Payment-gated feeds** — x402 integration
- **V2-aligned** — implements the Bus fabric from roko's v2 spec

## What to keep from the PR

| Component | Keep | Notes |
|-----------|------|-------|
| ChaCha20Poly1305 AEAD | Yes | Same encryption for confidential rooms |
| Room key derivation | Yes | `keccak256("DAEJI_ROOM_V1" \|\| jobId)` |
| Chain event watching (alloy) | Yes | Adapt targets to ERC-8004/8183 |
| Agent card concept | Adapt | Align with A2A agent-card format |

## What to drop

| Component | Why |
|-----------|-----|
| commonware-p2p mesh | Wrong tool for agent coordination |
| 64-slot pool | Pre-allocation is wasteful and limiting |
| Typed message enum | Bakes one coordination pattern into protocol |
| File-based registry | Chain + relay state is cleaner |
| kora embedding | Agents should connect to infrastructure, not embed it |
| Lobby/room dual channel | Single connection with topic subscriptions |

## Priority implementation order

### Phase 1: Base relay
Stand up an axum WebSocket server with topic pub/sub, ring buffer, standard envelope.
Port the chain watcher for ERC-8004/8183 events. Basic agent registry.
~500 lines. Replaces the entire current PR.

### Phase 2: Feeds + groups
Add feed registration/directory and group management. Auto-create groups from JobFunded.
~400 lines.

### Phase 3: Auth + payments
ERC-8004 identity verification. x402 payment gating for paid feeds. Group membership
verification.
~250 lines.

### Phase 4: Polish
Backpressure strategies. A2A card integration. Workspace discovery. Optional AEAD
per-room. Metrics and observability.

## commonware-p2p is still valuable

For **validator-to-validator consensus transport** (5 channels for votes/certs/blocks/
resolver/backfill). That's what kora already uses it for. It excels there.

Not for agent coordination. Different problem, different tool.
