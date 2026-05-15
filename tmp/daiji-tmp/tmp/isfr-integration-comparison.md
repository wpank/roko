# ISFR Integration: PR #24 vs Relay

Two hypotheticals compared: integrating ISFR oracle keepers using (A) the current PR #24 chat layer, and (B) a relay with feeds.

## What ISFR Actually Needs

The ISFR oracle has two submission paths:

1. **Fast path** — single permissioned keeper submits rate directly to ISFROracle contract. Zero communication with other keepers. The existing Python keeper (`offchainservices-agent/cli/jobs/keepers/funding.py`) does this today as a standalone process.

2. **Block-range path** — multiple keepers coordinate on a range window (start block, end block), each votes with their computed rate, quorum triggers close, trust-weighted median aggregation. This needs inter-keeper communication.

Communication requirements for block-range coordination:

| Need | Why |
|------|-----|
| **Presence** | Who is online and ready to vote on a range |
| **Range proposals** | A keeper proposes (start, end) window |
| **Rate votes** | Each keeper publishes their computed rates for the range |
| **Quorum notification** | When enough votes arrive, someone submits on-chain |
| **Close confirmation** | Chain event confirms range closed, rewards distributed |

This is a **broadcast pattern** — keepers publish to whoever's listening. Not a job. Not a symphony. Not a coordinator-driven workflow.

## Approach A: ISFR with Current PR #24

### The PR Knows It Doesn't Fit

The lobby.rs source code (line 15-16) explicitly says:

> **Public-broadcast (ISFR / autoresearch) → no lobby; agents post estimates on well-known signal channels directly.**

The PR's own design acknowledges ISFR bypasses its coordination model. But if you tried anyway:

### Step 1: Language Barrier

The existing ISFR keeper is Python. commonware-p2p is Rust-only. Two options:
- Rewrite the keeper in Rust (significant effort, maintains two implementations)
- Somehow bridge Python to the commonware mesh (no existing bridge, substantial work)

### Step 2: Connection Setup

The keeper must join the commonware-p2p authenticated mesh:
- Embed commonware-p2p dependency
- Pre-register channels before `network.start()`
- Join the peer set via the file-based registry (polled every 200ms)
- Run inside kora or a separate commonware-p2p node

### Step 3: Channel Mapping

ISFR is continuous — no job start/end. The chat layer is job-centric. Options:

- **Hack a permanent "ISFR job"** — fabricate a fake job ID to get a room. But rooms are created from `JobAwarded` chain events, so you'd need to emit a fake event or bypass the chain watcher. Room keys are derived from job IDs, so the fake job ID would need to be well-known.

- **Use the lobby directly** — but `LobbyMessage` is typed: `JobAnnounce`, `RoomJoined`, `JobConcluded`, `MiningClaim`. None of these are "here's my rate observation." You'd need to add ISFR-specific variants, coupling the protocol to one use case.

- **Reserve a hardcoded slot** — grab slot 0 as "ISFR signal channel" with a hardcoded room ID. Wastes 1/64 of the slot pool permanently.

### Step 4: Message Format

`RoomMessage` is `Hello`/`Status`/`PartialResult`/`Vote`/`Final`. An ISFR rate submission maps to none of these cleanly:

```rust
// Abusing PartialResult to carry rate data:
let msg = RoomMessage::PartialResult {
    from_pubkey_hex: my_pubkey.clone(),
    partial_id: epoch,                          // epoch number shoved into partial_id
    content_hash_hex: keccak256(&rate_bytes),
    content_ref: serde_json::to_string(&ISFRRates {  // rates jammed into URL field
        composite_bps: 690,
        lending_bps: 620,
        structured_bps: 710,
        funding_bps: 45,
        staking_bps: 32,
    })?,
};
```

Rate data crammed into `content_ref` (a field designed for URLs). The five class rates have no dedicated fields. Confidence score has nowhere to go. Range coordination (propose/vote) can't be expressed at all.

### Step 5: Encryption Overhead

AEAD encryption with room keys. ISFR rates are public data — encryption adds overhead for no security benefit. Every message gets ChaCha20Poly1305 encrypt/decrypt with the room ID as AAD.

### Step 6: Slot Pool Waste

64 pre-allocated channels with birthday-paradox collision handling. ISFR uses exactly one channel. 63 slots are wasted. The AEAD try-decrypt routing loop runs across potentially multiple jobs sharing a slot — unnecessary complexity for a single-purpose channel.

### What It Actually Looks Like

```rust
// In service.rs setup, before network.start():
let isfr_room_id = keccak256(b"DAEJI_ISFR_SIGNAL_V1");
let isfr_channel_id = u64::from_be_bytes(isfr_room_id[0..8].try_into().unwrap());
network.register(isfr_channel_id);

// In the keeper loop:
loop {
    let rates = read_source_protocols().await;

    // Abuse PartialResult to carry rate data
    let msg = RoomMessage::PartialResult {
        from_pubkey_hex: my_pubkey.clone(),
        partial_id: epoch,
        content_hash_hex: hex::encode(keccak256(&serde_json::to_vec(&rates)?)),
        content_ref: serde_json::to_string(&rates)?,
    };

    // Encrypt public data unnecessarily
    let plaintext = serde_json::to_vec(&msg)?;
    let encrypted = room::encrypt(&isfr_room_key, &isfr_room_id, &plaintext)?;

    // Send on the hardcoded channel
    sender.send(isfr_channel_id, encrypted).await?;

    sleep(Duration::from_secs(10)).await;
}
```

**Problems:**
- Rust-only (Python keeper can't participate)
- Rate data shoved into wrong fields
- Public data encrypted for no reason
- 63/64 slot pool channels wasted
- No range coordination expressible in the message types
- Must join commonware-p2p mesh (NAT-hostile, pre-configured peer set)
- Embedded in kora or requires separate commonware node

## Approach B: ISFR with Relay + Feeds

### Python Keeper (Minimal Changes)

The existing Python keeper adds ~20 lines for relay connection:

```python
import websockets, json, asyncio, time

async def run_keeper():
    async with websockets.connect("ws://relay:9011/ws") as ws:
        # Register
        await ws.send(json.dumps({
            "type": "hello",
            "agent_id": "isfr-keeper-1"
        }))

        # Subscribe to coordination + chain events
        await ws.send(json.dumps({
            "type": "subscribe",
            "topics": ["feed:isfr:rates", "feed:isfr:ranges", "chain:nunchi"]
        }))

        while True:
            # Existing logic: read source protocols
            rates = read_source_protocols()

            # Publish observation — JSON with proper fields
            await ws.send(json.dumps({
                "type": "publish",
                "topic": "feed:isfr:rates",
                "payload": {
                    "composite_bps": rates.composite,
                    "lending_bps": rates.lending,
                    "structured_bps": rates.structured,
                    "funding_bps": rates.funding,
                    "staking_bps": rates.staking,
                    "confidence_bps": rates.confidence,
                    "timestamp": int(time.time())
                }
            }))

            # Handle incoming messages (range proposals, chain events)
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=0.1)
                envelope = json.loads(msg)
                if envelope.get("topic") == "feed:isfr:ranges":
                    handle_range_coordination(envelope["payload"])
                elif envelope.get("topic") == "chain:nunchi":
                    handle_chain_event(envelope["payload"])
            except asyncio.TimeoutError:
                pass

            await asyncio.sleep(10)
```

### Rust Keeper (Roko Agent)

```rust
// In a roko agent's task handler:
let relay = ctx.relay_client();
relay.subscribe(&["feed:isfr:rates", "feed:isfr:ranges", "chain:nunchi"]).await?;

loop {
    let rates = read_sources(&ctx).await?;

    relay.publish("feed:isfr:rates", json!({
        "composite_bps": rates.composite,
        "lending_bps": rates.lending,
        "structured_bps": rates.structured,
        "funding_bps": rates.funding,
        "staking_bps": rates.staking,
        "confidence_bps": rates.confidence,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    })).await?;

    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

Both keepers — Python and Rust — see each other's publications on the same topic. No language barrier.

### Block-Range Coordination

```
Keeper-1 proposes a range:
  topic: "feed:isfr:ranges"
  payload: {
    "type": "range_propose",
    "start": 1000,
    "end": 1009,
    "proposed_by": "isfr-keeper-1",
    "proposed_at": 1713960000
  }

Keeper-2 votes:
  topic: "feed:isfr:ranges"
  payload: {
    "type": "range_vote",
    "start": 1000,
    "end": 1009,
    "composite_bps": 685,
    "components": [620, 710, 45, 32],
    "confidence_bps": 8500,
    "voter": "isfr-keeper-2"
  }

Keeper-3 votes:
  topic: "feed:isfr:ranges"
  payload: {
    "type": "range_vote",
    "start": 1000,
    "end": 1009,
    "composite_bps": 692,
    "components": [625, 708, 48, 30],
    "confidence_bps": 9000,
    "voter": "isfr-keeper-3"
  }

Any keeper detects quorum, submits on-chain:
  → ISFROracle.submitRateForRange(1000, 1009, median_rate, components, confidence)

Chain watcher detects RangeClosed event, publishes:
  topic: "chain:nunchi"
  payload: {
    "type": "isfr.range_closed",
    "range_start": 1000,
    "range_end": 1009,
    "composite_bps": 690,
    "voter_count": 3,
    "block_number": 19234567
  }
```

The relay doesn't know this is ISFR. It's just JSON on topics. The coordination protocol lives entirely in the keepers. No new frame types, no protocol coupling, no special message variants.

### What Each Layer Provides

```
Keepers (Python, Rust, whatever)
  │ Application logic: read sources, compute rates, coordinate ranges
  │
  ▼
Relay (daeji-relay)
  │ Transport: topic pub/sub, ring buffer, agent directory
  │ Chain: ERC-8004/8183 events → bus topics
  │
  ▼
Chain (daeji)
  │ Settlement: ISFROracle.submitRateForRange(), ISFRBountyPool
  │
  ▼
Consumers (dashboards, other agents, yield perpetual contracts)
  │ Subscribe to feed:isfr:rates or chain:nunchi for finalized values
```

Clean separation. Each layer does one thing.

## Side-by-Side Comparison

| Aspect | PR #24 (chat) | Relay + feeds |
|--------|---------------|---------------|
| **Keeper language** | Rust only | Any (Python, Rust, JS, Go...) |
| **Connection** | Join commonware-p2p mesh | WebSocket to relay |
| **NAT** | Hostile (needs dialable address) | Friendly (outbound WS) |
| **Rate message** | Abuse PartialResult fields | Proper JSON with correct fields |
| **Range coordination** | Not expressible in typed enum | Application-level JSON on topics |
| **Encryption** | AEAD on public data (wasteful) | None needed (rates are public) |
| **Channel allocation** | 1 of 64 hardcoded slots | Dynamic topic (unlimited) |
| **Chain events** | Each keeper runs own watcher | Relay delivers automatically |
| **Reconnection** | None (restart = lost state) | Resume from sequence number |
| **Mixed keepers** | Can't mix Python + Rust | Both see same topics |
| **Code changes** | Rewrite keeper in Rust + new message types | ~20 lines added to existing keeper |
| **Protocol coupling** | ISFR variants baked into message enum | Zero — relay is protocol-agnostic |

## Where ISFR Integration Lives in Roko

The most elegant integration point:

```
roko/
├── crates/
│   └── roko-agent-server/
│       └── src/features/
│           └── relay_client.rs      # Add subscribe/publish frames
│                                     # Currently: hello, card, response, ping
│                                     # Add: subscribe(topics), publish(topic, payload)
│                                     # ~50 lines changed
│
├── apps/
│   └── agent-relay/                 # Add bus.rs to existing relay
│       └── src/
│           ├── lib.rs               # Wire subscribe/publish into WebSocket handler
│           ├── protocol.rs          # Add Subscribe/Publish/Envelope frame types
│           └── bus.rs               # NEW: ~150 lines (topic pub/sub + ring buffer)
```

The relay changes are **additive** to existing roko code. The request/response pattern stays untouched. You're adding three frame types and one module.

For daeji-specific chain awareness, add `chain.rs` (~100 lines) — this is what makes daeji-relay distinct from roko's relay.

## Total Delta

| Where | What | Lines |
|-------|------|-------|
| Roko relay (or daeji-relay) | bus.rs — topic pub/sub + ring buffer | ~150 |
| Roko relay (or daeji-relay) | Subscribe/Publish/Envelope frames in protocol.rs | ~40 |
| Roko relay (or daeji-relay) | Wire frames into WebSocket handler | ~60 |
| Daeji-relay only | chain.rs — ERC-8004/8183 watcher → bus | ~100 |
| Roko relay_client.rs | Add subscribe/publish to existing client | ~50 |
| Python ISFR keeper | WebSocket connection + publish loop | ~30 |
| **Total** | | **~430** |

~430 lines across both codebases gets ISFR keepers coordinating through feeds, in any language, with chain events delivered automatically.

Compare to PR #24: ~2,000 lines that can't serve ISFR without language rewrite and protocol coupling.
