# Relay Protocol Freeze Checklist

**Owner doc for the relay / topic-bus protocol.** Verifies the frozen relay-bus
design (`tmp/relay-bus/`, the newest ~May-8 relay design) against the shipped
wire protocol, enumerates every gap, and gives an ordered roadmap to freeze.

Authoritative design: `tmp/relay-bus/{00-INDEX,01-relay-service-spec,02-validator-embedded-relay,03-coordination-use-cases,04-topic-grammar,05-decisions}.md`.
Depth doc (aspirational, ahead of code): `docs/v2-depth/12-connectivity/01-relay-wire-protocol.md`.
Shipped code: `apps/agent-relay/` (server) + `crates/roko-agent-server/src/features/{relay_client,relay_subscriber}.rs` (client) + `crates/roko-core/src/isfr_feed.rs` (bridge).

Status: **Built and wired, NOT frozen.** Four spec cleanups are unimplemented and one
(dot topics) is broadly regressed. `05-decisions.md` §5 explicitly says small breaking
changes are still allowed — do them, then freeze.

---

## Settled Decisions (from relay-bus/05, unchanged, all correct)

- MCP gateway is not a default Nunchi/Roko service; MCP config belongs with the agent runtime.
- Commonware chat (daeji PR #24) is dead; relay replaces it.
- Topics should use **dot**-separated grammar (relay-bus/04).
- Deployment: sidecar relay default; shared/validator relays optional/future.
- Relay protocol is **not v1 frozen** — breaking cleanups allowed now.
- Chain event read models live in the relay chain watcher, not an MCP gateway.
- Relay is sufficient for ordinary agent coordination; no separate chat layer.

---

## Wave-1 claims — VERIFIED

| Claim | Verdict | Evidence |
|---|---|---|
| Topic grammar decided on DOTS | TRUE | `relay-bus/04-topic-grammar.md:3` ("Use dots"), `05-decisions.md:45` |
| Code ships COLONS on the wire | **TRUE (and worse than reported)** | `isfr_feed.rs:125-127,149-151`; also `isfr_keeper.rs:475`, `chain_watcher.rs:40`, `chain_profile.rs:124`, `block_watcher.rs:329,396,458,483`, ~40 topics in `roko-serve/src/feed_agents/*` |
| `resume_after` reconnect-replay ABSENT | **TRUE** | `protocol.rs:107-110` `Subscribe { topic: String }` — no field; zero hits for `resume_after` in relay code |
| `timestamp_ms` on outbound `TopicMessage` ABSENT | **TRUE** | `protocol.rs:149-155` `TopicMessage` has `topic,msg_type,payload,publisher_id,seq` only; internal `TopicEnvelope` (`protocol.rs:161-168`) HAS `timestamp_ms:i64` but it is dropped at every fan-out (`lib.rs:283-289, 320-326`; `chain_watcher.rs:73-79`) |

Note: the referenced line window `protocol.rs:108-155` was accurate — `Subscribe` at 107-110, `TopicMessage` at 149-155.

---

## Frame-by-frame: spec vs code

### Inbound frames (`AgentInboundFrame`, `protocol.rs:88-129`)
| Frame | Spec (relay-bus/01 §Wire Format) | Code | Drift |
|---|---|---|---|
| `hello` | agent_id/name/capabilities | `AgentHello` + rest_endpoint/card/card_uri/metadata | Code superset — OK |
| `subscribe` | `{topic}` + **future `resume_after`** + **future `topics:[...]`** | `{topic}` only | **resume_after MISSING; batch subscribe MISSING** |
| `unsubscribe` | `{topic}` | `{topic}` | OK |
| `publish` | `{topic,msg_type,payload}` | same | OK |
| `ping` | `{}` | `Ping` | OK |
| `card`/`register_feed`/`unregister_feed`/`response`/`error` | (impl detail) | present | OK — undocumented in relay-bus/01 but in v2-depth doc |

### Outbound frames (`RelayOutboundFrame`, `protocol.rs:132-156`)
| Frame | Spec envelope fields | Code frame fields | Drift |
|---|---|---|---|
| `topic_message` | `seq, ts/timestamp_ms, topic, publisher_id, msg_type, payload` | `topic, msg_type, payload, publisher_id, seq` | **`timestamp_ms` DROPPED on the wire** (stored internally, only surfaced on the REST introspection path `lib.rs:478`) |
| `ack` | `{event}` | `{event}` | OK. Ack event STRINGS use colons: `subscribed:{topic}`, `published:{topic}:{seq}` (`lib.rs:296,330`) — cosmetic, but note if freezing ack grammar |
| `pong`/`message`/`error` | present | present | OK |

### Client mirror
`relay_client.rs` `on_topic_message(topic,msg_type,payload,publisher_id,seq)` (`:22-28`) and `relay_subscriber.rs::TopicMessage` struct (`:23-34`) both omit any timestamp — the client cannot receive `timestamp_ms` even if the server added it without a coordinated bump.

---

## Topic grammar drift (the big one)

Decision (relay-bus/04) = dots. Code = colons, everywhere that matters:

| Site | Topic strings | File:line |
|---|---|---|
| ISFR bridge input + subscribe list | `isfr:rates`, `isfr:epochs`, `chain:{id}` | `isfr_feed.rs:125-127,149-151` |
| ISFR keeper publish (wire) | `isfr:rates` | `isfr_keeper.rs:475` |
| Relay chain watcher publish (wire) | `chain:{chain_id}` | `chain_watcher.rs:40` |
| Chain profile helper | `chain:{chain_id}` | `chain_profile.rs:124` |
| Block watcher publish | `chain:block`, `chain:tx`, `chain:event` | `block_watcher.rs:329,396,458,483` |
| Feed-agent fleet (~40 feeds) | `feed:isfr:*`, `feed:chain:*`, `feed:defi:*`, `feed:analytics:*`, `feed:meta:*` | `roko-serve/src/feed_agents/*.rs` |
| roko-serve chain dispatch match arms | `chain:block/tx/event` | `roko-serve/src/lib.rs:2496,2513,2529` |

`map_topic()` (`isfr_feed.rs:123-130`) is the colon→dot translation layer relay-bus/04 wanted **removed**. It is still load-bearing: it converts inbound relay colon-topics to internal Pulse dot-topics. Removing it requires migrating every publisher above to dots in one coordinated change. Bus tests (`bus.rs:158-180`) and integration tests also assert colon topics.

**Migration surface is ~7 files / ~50 topic literals**, not the 1 file wave-1 implied.

---

## Doc-vs-code drift (aspirational depth doc)

`docs/v2-depth/12-connectivity/01-relay-wire-protocol.md` documents the *target*, not the code:
- §3.1/§4 show dot topics (`isfr.rates`) — code ships colons.
- §4 envelope JSON includes `timestamp_ms` and §4.1 lists it as a `TopicMessage` field — the outbound frame omits it.
- §5.4 correctly labels `resume_after` as "Future" / planned — consistent with code, inconsistent with relay-bus/05 which lists it as an allowed pre-freeze cleanup (i.e. it should be done, not deferred).

This doc must be reconciled to reality (or the code brought up to it) before freeze, per the Archive Rule below.

---

## Status matrix

| Item | Spec target | Shipped | Status |
|---|---|---|---|
| Hello / ack / ping / pong | ✓ | ✓ | **Wired** |
| Subscribe + ring replay | replay on subscribe | ✓ (`bus.rs:49-65`, `lib.rs:281-294`) | **Wired** |
| Global monotonic seq from 1 | ✓ | ✓ (`bus.rs:43,92-94`) | **Wired** |
| Publish fan-out (skip self) | ✓ | ✓ (`lib.rs:316-328`) | **Wired** |
| Request/response HTTP bridge | ✓ | ✓ (`RelayMessageRequest`, timeout clamp `protocol.rs:54-61`) | **Wired** |
| Feed registry / workspace directory / events WS | ✓ | ✓ | **Wired** |
| Chain watcher | block polling + future log decode | `new_block` poll only (`chain_watcher.rs`) | **Partial** |
| `timestamp_ms` on outbound frame | required for freeze | dropped on WS; present on REST | **Missing (P0)** |
| Dot topic grammar | required for freeze | colons everywhere | **Regressed (P0)** |
| `resume_after` reconnect | required for freeze | absent | **Missing (P1)** |
| Batch `topics:[...]` subscribe | pre-freeze cleanup | absent | **Missing (P1)** |
| Wildcard subscriptions (`chain.*`) | future | absent | **Missing (P2)** |
| Auth (agent passport) | future/shared relays | none | **Missing (P2)** |
| Topic GC / backpressure / metrics | low | none | **Missing (P3)** |

---

## Freeze checklist (each item = a merge-gate before v1 freeze)

- [ ] **Add `timestamp_ms` to `RelayOutboundFrame::TopicMessage`** (`protocol.rs:149-155`); populate from `envelope.timestamp_ms` at all three fan-out sites (`lib.rs:283-289,320-326`; `chain_watcher.rs:73-79`); extend client `on_topic_message` + `relay_subscriber::TopicMessage` to carry it; add a serialization test.
- [ ] **Migrate topics colons → dots** across the ~7 files / ~50 literals above; **delete `map_topic()`** translation; update `bus.rs` + integration + `chain_profile`/`block_watcher`/`feed_agents` tests. Grep-prove zero live colon-topic assumptions (`rg '"[a-z]+:[a-z]' crates apps --include='*.rs'`).
- [ ] **Decide ack-event grammar** (`subscribed:{topic}`, `published:{topic}:{seq}`): keep colon-delimited control strings or switch — document either way before freeze.
- [ ] **Implement `resume_after: seq`** on `Subscribe` (`protocol.rs:107-110`); in `bus.subscribe` replay only envelopes with `seq > resume_after`.
- [ ] **Implement batch `topics: Vec<String>`** subscribe (accept single or list; keep back-compat).
- [ ] **Reconcile `docs/v2-depth/12-connectivity/01-relay-wire-protocol.md`** to the frozen frame (dots + `timestamp_ms` + `resume_after` implemented, not "future").
- [ ] **Response-shape parity**: verify demo `demo/demo-app/src/lib/relay-api.ts` health/feeds/topic shapes match server (`lib.rs` handlers).
- [ ] (Freeze-adjacent, not blocking) chain watcher: decode contract logs via `eth_subscribe` instead of `new_block` polling.

---

## Ordered roadmap to freeze

1. **P0 — `timestamp_ms` on outbound frame** (small, self-contained, no publisher changes). Unblocks freeze of the envelope.
2. **P0 — dot-topic migration + delete `map_topic()`** (large, cross-crate; do as one atomic PR with tests). Single biggest freeze blocker by surface area.
3. **P1 — `resume_after`** (server + client + bus replay filter).
4. **P1 — batch `topics:[...]` subscribe.**
5. **Doc reconciliation** of `01-relay-wire-protocol.md` + demo `relay-api.ts` parity check.
6. **Freeze v1 wire format.** After this, `resume`/batch/timestamp/dots are contract.
7. **Post-freeze (additive, non-breaking):** wildcard subscriptions, auth, chain-log decode, topic GC, metrics.

---

## Archive Rule

Do NOT archive `tmp/relay-bus/` until each freeze-checklist item is either implemented
or copied into `docs/v2-depth/12-connectivity/` as explicit, dated future work. After
freeze: keep `05-decisions.md` as rationale, archive the rest as historical design input.
