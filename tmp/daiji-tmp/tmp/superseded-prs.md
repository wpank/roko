# Daeji PR #24 — Superseded PRs Analysis

PR #24 (`jl/chat-consolidated`) consolidates 5 prior PRs into a single unit.

## What Each PR Contributed

| PR | Branch | What it added |
|----|--------|--------------|
| **#11** | `chat-chain-events` | Chain event watcher: `AgentRegistered` + `JobAwarded` events via alloy WS provider. Status card verification via keccak256(body)==passportHash. HTTP fetch of off-chain agent card. |
| **#13** | `lobby-slot-pool-primitives` | Lobby message format (JobAnnounce/RoomJoined/JobConcluded/MiningClaim). Room ID derivation matching `MultiAgentMarket.computeRoomId`. POOL_SIZE=64 constant. |
| **#14** | `lobby-slot-pool-runtime` | Pre-registration of lobby + 64 slot channels before `network.start()`. Slot loop with AEAD try-decrypt routing. |
| **#17** | `supervisor-primitives` | Exponential backoff (1s→60s cap). PanicTracker sliding window (3 failures in 60s → give up). `DAEJI_CHAT_DISABLED` env var. |
| **#19** | `supervised-chat-hookup` | CLI flags (`--chat-config`, `--disable-chat`). kora-service integration with supervised spawn. |

## What Was Preserved in #24

All code from the superseded PRs is present in the consolidated crate at `crates/network/daeji-chat/src/`:

- `chain.rs` (350 lines) — from #11, chain event watcher
- `lobby.rs` (220 lines) — from #13, lobby message format
- `room.rs` (220 lines) — from #13, room ID derivation + AEAD encryption
- `service.rs` (700 lines) — from #14, pre-registration + slot loops + orchestration
- `supervisor.rs` (290 lines) — from #17, backoff + panic tracking
- `card.rs` (220 lines) — from #11, status card schema + verification
- `registry.rs` (290 lines) — from #14, file-backed peer set
- `messages.rs` (70 lines) — from #13, in-room message types

**53 unit tests all pass.** 3-agent e2e AEAD-encrypted communication verified.

## What Was Lost (or Deferred)

### Design Rationale

The individual PRs contained design discussion in their descriptions that's no longer visible:

1. **Birthday paradox analysis** (#13) — why POOL_SIZE=64, collision probability math (~50% at 8 concurrent jobs per agent), AEAD disambiguates collisions
2. **Separate mesh reasoning** (#14) — why chat runs on its own commonware-p2p network (bug isolation from consensus)
3. **Panic policy** (#17) — explicit decision to NOT catch panics in Rust (defer to systemd/cgroup), only catch Result::Err

### Contract References

PR descriptions referenced specific contracts-core PRs:
- **#111** (MultiAgentMarket, MERGED) — the N-winner market that emits `JobAwarded` with `roomId`
- **#114** (referenced but unclear status) — related contract work

These references are embedded in the chain watcher code but the explicit dependency tracking is gone.

### Future Work Items (from PR descriptions)

1. **X25519 ECDH key wrapping** — v1 uses plaintext room keys; ECDH wrap was planned for "PR-Daeji-F"
2. **Auto-join from JobAwarded** — chain watcher detects `JobAwarded` but auto-join is deferred ("requires architectural reshape per canonical plan Q-Open-6")
3. **JobAwarded state machine** — full state tracking of job lifecycle in chat was deferred
4. **Shared mesh** — v1 runs chat on separate commonware-p2p network; future PR would share mesh with consensus

### Symphony Pattern Details

The symphony coordination pattern is preserved in the code but the design rationale was spread across multiple PR descriptions:

**Coordination flow:**
1. On-chain: `MultiAgentMarket.award(jobId, winners)` → emits `JobAwarded(jobId, winners[], roomId)`
2. Chain watcher: detects event, verifies room ID parity
3. Lobby: coordinator broadcasts `JobAnnounce` with room key wraps per recipient
4. Room: winners join, exchange Hello→Status→PartialResult→Vote→Final
5. Settlement: final result submitted via `submitMulti(jobId, result, signatures)`

**Four job coordination modes were discussed** (from canonical plan §11):
- Symphony/reputation-gated/swarm-open → coordinator broadcasts JobAnnounce
- Mining-bounty → claim-first via MiningClaim (no central coordinator)
- Public-broadcast (ISFR/autoresearch) → well-known signal channels (no lobby)
- MEV race → no lobby (latency budget too tight)

Only the first mode (symphony) is implemented. The other three modes were planned but not built.

## Architectural Decisions Worth Preserving

| Decision | Rationale | Still valid? |
|----------|-----------|-------------|
| Library-only, no binaries | Chat runs on kora's authenticated mesh | **No** — relay should be standalone |
| Separate chat network port | Bug isolation from consensus | **Partially** — relay is inherently separate |
| Pre-registered channels at startup | commonware-p2p requirement | **No** — relay has dynamic topics |
| POOL_SIZE = 64 | Birthday paradox math | **No** — relay has unlimited topics |
| Try-decrypt loop per slot | No routing table, AEAD routes | **No** — relay has topic-based routing |
| Supervisor backoff 1s→60s | Prevent rapid spawn loops | **Yes** — good retry pattern for any service |
| Keccak256 room ID derivation | Cross-language parity with Solidity | **Yes** — same derivation useful for chain-driven groups |
| AEAD with room_id as AAD | Prevents cross-room replay | **Yes** — same encryption pattern for confidential rooms |

## SupervisedContext Trait

The trait bounds pattern from `service.rs` is worth noting:

```rust
pub trait SupervisedContext:
    Spawner + Metrics + Clock + BufferPooler + Network + Resolver + CryptoRngCore
```

This tightly couples chat to commonware's runtime abstraction. In a relay design, this coupling disappears — the relay is a standalone axum server, agents connect via WebSocket.

## Key Takeaway

PR #24 is well-engineered code that solves the wrong problem. The design rationale, birthday paradox analysis, and coordination mode taxonomy from the superseded PRs are valuable context even though the implementation approach (commonware-p2p mesh) should be replaced with a relay. The AEAD encryption, room key derivation, and chain event patterns should carry forward.
