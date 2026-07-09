# C — Gossip and P2P Network (Docs 07, 08, 09)

Parity of the three network-layer chapters in topic 08: the 4-tier gossip
architecture (GossipSub → MiroFish → FABRIC → Event Bus), the eight gossip
topics, and the three-layer peer scoring (protocol + application +
economic).

None of the libp2p, iroh, GossipSub, Kademlia, or Dandelion++ integrations
described in Docs 07-09 ship as Rust code today. `Grep 'libp2p|gossipsub|
GossipSub|iroh|rendezvous|DHT|Kademlia|FABRIC|Dandelion' crates/ apps/`
returns **zero matches in `.rs` files** (67 matches are docs-only). The
closest shipping surface is the `InsightBus` / `PheromoneBus` subscription
layer in `apps/mirage-rs/src/roko_bridge/subscription/` — a **local**
broadcast / mpsc multiplexer, not a libp2p mesh.

Generated 2026-04-16.

---

## C.01 — Four-tier gossip architecture (GossipSub → MiroFish → FABRIC → Event Bus) is absent (Doc 07 §"Four-Tier Architecture")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Gossip layering: GossipSub v1.1 (ms) → MiroFish simulation aggregator (sec) → FABRIC TEE aggregation (epoch) → Canonical Event Bus (block). Each tier has its own envelope format, retention, and validation.
**Reality**: `Grep 'libp2p|gossipsub|GossipSub|MiroFish|FABRIC' crates/ apps/ --include=*.rs` returns zero matches. No libp2p crate dependency; `grep -rn 'libp2p' crates/*/Cargo.toml apps/*/Cargo.toml` is empty. The only gossip-adjacent shipping surface is `apps/mirage-rs/src/roko_bridge/subscription/` (~1,200 LOC across `mod.rs`, `insight.rs`, `pheromone.rs`, `backpressure.rs`, `sink.rs`) implementing `InsightBus`, `PheromoneBus`, `MpscSink`, `BroadcastSink`, `VecSink`, `BackpressurePolicy`, `SubscriptionStats`. These are **in-process broadcast** primitives — every subscriber sits in the same address space — and carry no p2p or TEE semantics.
**Fix sketch**: Doc 07 should carry a `Design — Phase 2+` banner. Add a cross-link to the shipping `InsightBus` / `PheromoneBus` as local analogues whose topic model may inform future GossipSub schemas.

---

## C.02 — Gossip envelope format is design-only (Doc 07 §"Gossip Envelope")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Envelope struct carries sender passport id, topic id, payload, proof-of-authority signature, optional TEE quote, vector-clock metadata for causal ordering.
**Reality**: No envelope type exists. `Grep 'GossipEnvelope|MessageEnvelope|SubscriptionEnvelope|VectorClock' crates/ apps/ --include=*.rs` returns zero matches. The in-process `InsightBus` uses typed events (`InsightEvent`, `PheromoneEvent`) without envelopes — subscribers trust the bus.
**Fix sketch**: Stays Phase 2+. No code implications today.

---

## C.03 — Vector clocks, CRDTs, Dandelion++ privacy are pure design (Doc 07 §"Message Ordering", §"Privacy-Preserving Gossip")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Vector clocks for causal ordering (Lamport 1978), CRDTs for concurrent knowledge merges (GCounter, LWW-Register — Shapiro et al. 2011), Dandelion++ anonymous broadcast (Fanti et al. 2018), subscription privacy, cover traffic.
**Reality**: None of these concepts have any Rust implementation. The 2026-04-13 enhancement pass added these sections to Doc 07 but no scaffold was written.
**Fix sketch**: Apply a `Design — Phase 2+` banner to the §"Message Ordering" and §"Privacy-Preserving Gossip" subsections introduced by the enhancement pass.

---

## C.04 — Eight gossip topics are absent as code (Doc 08 §"Topic List")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Eight topics with specific schemas and TTL policies:
1. `knowledge` (insight publication)
2. `reputation` (feedback events)
3. `job` (job lifecycle)
4. `heartbeat` (liveness)
5. `anomaly` (abnormal behavior reports)
6. `simulation` (mirage-rs forecasts)
7. `governance` (proposals / votes)
8. `peer-discovery` (peer bootstrap)

Each topic has a schema, TTL, subscription rule (who must subscribe), and mesh parameters.
**Reality**: `Grep 'knowledge_topic|reputation_topic|job_topic|heartbeat_topic|anomaly_topic|governance_topic|peer_discovery_topic' crates/ apps/ --include=*.rs` returns zero matches. The `InsightBus` / `PheromoneBus` split in `apps/mirage-rs/src/roko_bridge/subscription/` is a **2-topic local** surface (insight + pheromone), not the 8-topic mesh. Topic schemas and TTL policies exist only in Doc 08.
**Fix sketch**: Doc 08 stays `Design — Phase 2+`. When a GossipSub integration lands, the 2-topic local split may expand naturally into the 8-topic mesh.

---

## C.05 — Heartbeat topic / liveness pings are missing on the chain side (Doc 08 §"heartbeat")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Heartbeat topic carries short liveness pings (passport id + block number + optional capability delta) at ~1 Hz. Consumers include the chain-agent watchdog and the reputation registry (for uptime reputation).
**Reality**: The shipping `contracts/src/AgentRegistry.sol:44-48` implements a `heartbeat()` Solidity function that updates `lastHeartbeat` block number — the closest shipping analogue. But this is a direct on-chain transaction, not a gossip topic, and the `LIVENESS_WINDOW = 200` blocks (`:19`) gate does not propagate to a reputation registry (because the registry doesn't exist; see B.11). Gossip heartbeat is not wired.

---

## C.06 — Three-layer peer scoring is absent (Doc 09 §"Three Layers")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Peer scoring = protocol layer (GossipSub mesh quality, Vyzovitis 2020) + application layer (domain behavior) + economic layer (stake-weighted). Combined score determines mesh membership and Sybil-resistance.
**Reality**: `Grep 'peer_score|PeerScore|ProtocolScore|ApplicationScore|EconomicScore' crates/ apps/ --include=*.rs` returns zero matches. No Sybil-resistant scoring anywhere.
**Fix sketch**: Phase 2+ banner.

---

## C.07 — Sybil resistance and mesh-membership rules are unused (Doc 09 §"Sybil Resistance")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Mesh membership requires reputation threshold + stake in at least one domain + passing protocol-layer score. EigenTrust for transitive trust.
**Reality**: None of these gates exist. Cross-link: EigenTrust is also referenced in Doc 14 reputation gaming resistance (see D.07). No EigenTrust crate, no `eigentrust.rs`.

---

## C.08 — `InsightBus` / `PheromoneBus` local broadcast surface (shipping; adjacent to Docs 07-08 but not the mesh)

**Status**: DONE (adjacent, not on the documented topology)
**Severity**: —
**Doc claim**: Docs 07-09 do not document the local broadcast surface as a separate concept; they describe the full 4-tier mesh.
**Reality**: `apps/mirage-rs/src/roko_bridge/subscription/` ships a real in-process pub/sub layer today. `mod.rs` re-exports `BackpressurePolicy`, `BroadcastSink`, `InsightBus`, `InsightEvent`, `InsightSubscription`, `MpscSink`, `PheromoneBus`, `PheromoneEvent`, `PheromoneSubscription`, `SinkError`, `SubscriptionId`, `SubscriptionSink`, `SubscriptionStats`, `VecSink`. Five implementation files: `insight.rs`, `pheromone.rs`, `backpressure.rs`, `sink.rs`, `mod.rs`. This surface is consumed today by `apps/mirage-rs/src/http_api/ws.rs` for WebSocket streaming (CLAUDE.md "roko-agent-server" sidecar `/stream` WS) and by `crates/roko-demo/*` scenarios. It is NOT the GossipSub mesh, NOT on libp2p, and NOT on an iroh transport — but it is a real, well-factored local topic surface that could seed the eventual p2p integration.
**Fix sketch**: Add a subsection to Doc 07 or 08 documenting the `InsightBus` / `PheromoneBus` local pub/sub as the shipping precursor. Clarify that it covers 2 topics (insight + pheromone) compared to the 8-topic gossip mesh.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 (C.08 InsightBus/PheromoneBus local pub/sub — adjacent to the documented mesh, not the mesh itself) |
| PARTIAL | 0 |
| NOT DONE | 7 (C.01 4-tier gossip, C.02 envelope format, C.03 vector clocks / CRDTs / Dandelion++, C.04 8 topics, C.05 heartbeat topic, C.06 3-layer peer scoring, C.07 Sybil resistance) |

Section C is the most uniformly **frontier** section of the chain
topic. The entire p2p / gossip / Sybil-resistance stack is specification.
The only thing that ships adjacent to it is the local in-process
`InsightBus` / `PheromoneBus` (C.08), which covers a subset of the "eight
gossip topics" but without any p2p transport.

The gossip chapters should be treated as **canonical design docs for
post-self-hosting work**, not as claims about shipping infrastructure.

## Agent Execution Notes

### C.01-C.07 — Frontier Banner Pass

Best use of this section in batch `08`:

1. add a `Design — Phase 2+` banner at the top of each of Docs 07, 08, 09
2. cite C.08 `InsightBus` / `PheromoneBus` as the nearest shipping ancestor

### C.08 — Local pub/sub documentation pass

Add a subsection to Doc 07 (or new Doc 25) describing the local
pub/sub surface so later agents know what shipping code is
insight-topic-like without mistaking it for the full GossipSub mesh.

Acceptance criteria for this section:

- Docs 07-09 are explicitly banner-tagged as frontier,
- the local pub/sub surface at `apps/mirage-rs/src/roko_bridge/subscription/` is documented as the shipping precursor,
- later agents can tell the difference between "InsightBus is a mesh" (false) and "InsightBus is a local topic broadcast" (true).
