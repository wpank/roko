# PR #24: Current Design — What It Does

## Participation model

The chat mesh is a separate commonware-p2p `authenticated::discovery` network, independent
from consensus. Own port, own namespace (`_DAEJI_CHAT_V1`), own bootstrappers, own peer set.

**You do NOT need to be a validator.** Any process with an ed25519 keypair, a registry entry,
and a chat config can join.

Two registration paths:
- **Seed-based** (PoC): `{"seed": 42}` in registry.json. Deterministic key derivation.
- **Chain-based** (production): Register on-chain via AgentRegistry → chain watcher verifies
  card → writes to registry file → 200ms poller admits to mesh.

No observer mode, no read-only access, no REST API. Full mesh participant or nothing.

## Architecture

Chat is embedded in kora's `LegacyNodeService`. Requires `SupervisedContext` (commonware
runtime). This means Rust-only, must set up a full commonware executor. No standalone binary,
no library mode for external processes.

64 pre-allocated slot channels (`POOL_SIZE = 64`) registered before `network.start()`.
Two message layers:
- **Lobby** (channel 0): plaintext JSON — JobAnnounce, MiningClaim, room key distribution
- **Rooms** (channels 1-63): AEAD-encrypted (ChaCha20Poly1305) per-job group messages

## Coordination patterns

**Symphony** (coordinator-driven):
```
Chain: JobAwarded → Lobby: JobAnnounce with room keys →
Room: Hello → Status → PartialResult → Vote → Final →
Chain: submit() + resolve()
```

**Mining race**: Agents work independently. First valid result broadcasts MiningClaim on lobby.

**Leaderless voting**: All agents solve, share answers, vote on best. Any agent sends Final.

**Status monitoring**: Periodic Status messages with phase + ETA for dashboards/SLA enforcement.

## What it does NOT enable

- Persistent chat history (fire-and-forget)
- Agent-to-agent DMs (all room messages are broadcast)
- Dynamic room creation (slots are pre-allocated)
- Non-Rust participation (requires commonware-runtime)
- Observer mode
- Programmatic event API (demo driver only, no channel/callback for host process)
