#  Current Design 

## The fundamental mismatch

PR #24 uses commonware-p2p (a validator consensus transport) for agent coordination (a
dynamic pub/sub problem). These are different problem classes.

**Validator consensus:** Small, fixed set of known peers. Authenticated, high-throughput,
low-latency. Pre-configured at startup. All peers known in advance. O(n²) is acceptable
because n ≤ 100.

**Agent coordination:** Dynamic, potentially large set of agents. Ad-hoc groups. Different
languages. Many behind NAT. Come and go. Need discovery. O(n²) is not acceptable.

## Specific problems

### 1. Full mesh doesn't scale
O(n²) connections. 10 agents = 90 connections. 100 agents = 9,900. 1,000 = 999,000.
Validators are 5-50 nodes. Agents could be thousands.

### 2. Pre-allocated channels are wasteful
64 channels registered at startup (`POOL_SIZE = 64`). Can't add more. Can't remove unused
ones. Multiple jobs collide on the same slot and rely on AEAD try-decrypt to sort them out.

### 3. Embedded in kora is backwards
Chat requires `SupervisedContext` — 7+ trait bounds from commonware runtime. Only Rust
callers. No API for external processes. Agents should connect to infrastructure, not embed it.

### 4. No API surface
The demo driver hardcodes Hello → Status → Final. No event channel, no send handle, no
callback. An agent literally cannot use this programmatically.

### 5. No NAT traversal
commonware-p2p requires `dialable_addr`. Agents behind NAT, on home servers, or on corporate
networks can't participate. The relay pattern (outbound WebSocket) solves this inherently.

### 6. Rust-only
commonware-p2p's `Sender`/`Receiver` types are Rust-specific. A Python bot, Claude via MCP,
or an OpenClaw agent cannot participate.

### 7. File-based discovery is fragile
Registry file polled every 200ms. Multiple writers risk corruption. No atomic peer set updates.
Chain-driven discovery through a relay is cleaner.

### 8. Typed messages bake in one pattern
`RoomMessage` enum (Hello, Status, PartialResult, Vote, Final) bakes the "symphony" coordination
pattern into the protocol. Other patterns (keeper tasks, mining races, ad-hoc chat, data feeds)
don't fit these types. Opaque JSON payloads with a minimal envelope are more flexible.

## What jobs actually look like

Most useful agent tasks fall into three categories:

### Category 1: Notifications (relay tells agents about jobs)
Chain event → relay → agent. One-directional. Agent decides whether to bid/claim.
No coordination between agents. No room needed.

### Category 2: Request/response (agent asks agent)
Point-to-point. Timeout-bounded. Roko's relay already handles this perfectly.

### Category 3: Rooms (agents coordinate on a shared task)
N agents exchange messages in a group. The room protocol should be minimal and flexible —
opaque JSON payloads, not rigid Hello → Status → Vote → Final state machine.

### What probably works as jobs
- **Multi-step reasoning with verification** — clear deliverable, escrow incentivizes, voting catches inconsistencies
- **Keeper tasks** (oracle updates, liquidations) — atomic, objectively verifiable
- **Competitive mining** — winner-takes-all, verifiable, no coordination needed

### What probably doesn't work
- **"Symphony" multi-agent consensus** — requires synchronized phases, deterministic protocol states, real-time cross-evaluation. LLM agents don't decompose this way.
- **"Leaderless voting"** — run 3 agents, pick majority. Doesn't need a chat protocol.

## The alternative: relay-first

Roko's existing agent-relay (~350 lines of axum WebSocket) is more than sufficient for agent
discovery and messaging. Extended with rooms and chain awareness, it replaces the entire
commonware-p2p chat layer while being simpler, more capable, and language-agnostic.

commonware-p2p should be used where it excels: validator-to-validator consensus transport.
Not for agent coordination.
