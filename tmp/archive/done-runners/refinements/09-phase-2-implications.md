# Phase 2+ Implications

> **TL;DR**: The two-fabric model makes chain, dreams, coordination,
> and mesh land as swap-in Bus/Substrate backends, not as rewrites.
> Stigmergy becomes a literal sentence. The HTTP control plane and
> per-agent sidecar stop being special cases and become Bus
> consumers. Multi-agent collectives become pub/sub topologies.

> **For first-time readers**: Phase 2+ in Roko's roadmap refers to the
> chain layer (on-chain coordination), dreams (offline consolidation),
> mesh (inter-agent p2p), and a few cross-cuts (Daimon affect, heartbeat
> clock). Today they are partial or stubbed. This doc walks each Phase-2
> subsystem and shows how the two-fabric kernel from 01–08 makes each
> one *smaller* rather than adding to the architectural surface area.

## 1. Chain (Korai / Daeji — Phase 6)

`docs/00-architecture/08-chain-layer.md` describes Roko's chain
integration as shared on-chain state for agent coordination, with
three transport needs: **storing** signed Engrams (transactions,
attestations), **reading** shared knowledge (insights, bounties), and
**reacting** to on-chain events.

In the current architecture, these three needs are lumped together
into a single `ChainSubstrate` (see `crates/roko-core/src/traits.rs`
comments — "ChainSubstrate — on-chain state via RPC"). But reading
on-chain state is a query, and reacting to on-chain events is
fundamentally a subscription — those are different fabrics.

With two fabrics:

- **`ChainSubstrate`** stores and queries durable on-chain Engrams
  (transactions, attestations, insights, bounties, pheromones). It
  already makes sense as a Substrate.
- **`ChainBus`** (new) maps event-log topics to Bus topics. A smart
  contract emits a `Deposited(agent, amount)` log; the ChainBus
  turns it into a `chain.deposit.emitted` Pulse. Subscribers in
  `roko-learn`, `roko-conductor`, and dashboards see it the same
  way they see any other Pulse.

This is the clean mapping. Without it, every chain-event consumer
has to poll `ChainSubstrate` or set up its own RPC subscription — a
repetition of the polling-vs-streaming bug that's already P0 in
`tmp/ux-followup/12-tui-event-parity.md`, just at a different layer.

## 2. Dreams (offline consolidation — Phase 5C)

`docs/00-architecture/10-dreams.md` describes Dreams as a Delta-speed
(hours-scale) loop that consolidates recent Engrams into higher-tier
knowledge. It's scaffold-only today (per `docs/STATUS.md`).

In the one-noun model, Dreams has to walk the Substrate to find
candidate Engrams for consolidation. It's a polling loop.

In the two-fabric model, Dreams has two inputs:

1. **Substrate scan** — still the primary source, because
   consolidation is deliberate and wants completeness.
2. **Bus subscription** — to `substrate.engram.stored` (emitted by
   the Substrate when new durable Engrams land). This makes Dreams
   reactive: it can wake up when a threshold of new content is
   available rather than polling on a fixed schedule. That matters
   because Delta-speed doesn't mean fixed-cadence; it means
   "slower than Gamma/Theta" — and "slower" can be event-triggered.

Dreams also emits consolidated `Kind::Insight` and `Kind::Heuristic`
Engrams. In the two-fabric model it emits both the Engram (to
Substrate) *and* an `engram.promoted` Pulse (to Bus) so the Composer
at L2 can react and update its enrichment heuristics without
re-querying.

## 3. Coordination / Stigmergy — Phase 13

`docs/00-architecture/13-coordination.md` describes stigmergic
coordination — agents leaving pheromone traces that other agents
follow. Grassé's original stigmergy concept (1959) is *shared
environmental state as indirect communication*.

In the two-fabric model, stigmergy is a literal one-liner:

> Pheromones are Engrams persisted to a shared Substrate (chain or
> mesh) with Ebbinghaus decay. Agents deposit pheromones by
> `substrate.put`; they detect them by `substrate.query` and/or by
> subscribing to `mesh.pheromone.deposited` on the Bus.

No new mechanism needed. The one-noun model made this awkward
because "put a signal and also somehow tell nearby agents it's
there" required custom plumbing. The two-fabric model separates
*depositing* (Substrate) from *alerting* (Bus) — which is exactly
the ant-trail dynamic.

## 4. HTTP control plane — already Bus-shaped

`crates/roko-serve/` exposes ~85 routes plus SSE and WebSocket
streams. Today the WebSocket/SSE endpoints fan out internal
broadcast channels through ad-hoc conversions. In the two-fabric
model, the HTTP layer is trivial:

- REST GET routes → read from Substrate.
- REST POST routes → publish a Pulse or graduate an Engram.
- WebSocket/SSE streams → forward Bus subscriptions over HTTP.

`roko-serve` becomes mostly a thin Bus-and-Substrate projection to
HTTP. Auth, schema, and rate-limiting live in the serve layer; the
data model is the same as everywhere else in the system.

The agent sidecar in `roko-agent-server` works the same way: each
running agent has its own Bus (or a namespaced topic prefix on the
shared Bus) and exposes it via WebSocket. That's how the TUI's
F3 Agents tab renders live streams.

## 5. Mesh — inter-agent coordination — Phase 2+

`docs/00-architecture/18-tools.md` and `docs/14-identity-economy`
reference a future agent mesh for peer-to-peer relay and
permissioned subnets. The naming glossary at `01-naming-and-glossary.md`
calls it "Agent Mesh / Mesh" (formerly Styx).

With two fabrics, mesh is trivially:

- **MeshBus** — a Bus backend that fans out Pulses over NATS or a
  libp2p gossipsub topology. Agents subscribe to topics they care
  about.
- **MeshSubstrate** — a Substrate backend that replicates Engrams
  over the same transport. Could be CRDT-based; could use the
  chain as arbiter.

No part of the core architecture changes when the mesh crate lands.
It's a backend swap.

## 6. Multi-agent collectives — Phase 5+

`docs/00-architecture/14-c-factor-collective-intelligence.md`
describes collective intelligence metrics — how *groups* of agents
perform, not just individuals.

Collectives in the two-fabric model are pub/sub topologies:

- A **Swarm** is N agents subscribed to the same topic set; each
  publishes its own findings; the collective outcome is the union
  of all Pulses and Engrams.
- A **Pipeline** is a chain of topic subscriptions — agent A
  publishes to `work.stage1.done`, agent B subscribes, publishes
  `work.stage2.done`, etc.
- A **Committee** is a fan-in topology — N agents publish votes to
  `decision.vote`; a single aggregator publishes the result to
  `decision.result`.

All three are just topic wiring. No orchestrator code changes.
This is what the "generalized, modular, flexible, extensible"
language in the user's request actually cashes out to.

## 7. Heartbeat (Phase 16)

`docs/00-architecture/16-heartbeat.md` describes a "cognitive clock"
that runs at three speeds (Gamma/Theta/Delta). Today it doesn't
exist; in the two-fabric model it's a `HeartbeatPolicy` that
publishes `heartbeat.gamma.tick`, `heartbeat.theta.tick`, and
`heartbeat.delta.tick` Pulses on schedule. Every speed-adaptive
subsystem subscribes to the appropriate topic. The clock itself is
fifty lines.

## 8. Safety / Provenance (Phase 11)

Safety's audit model already assumes content-addressed Engrams for
the long-term forensic DAG. Two-fabric doesn't change that — it
adds live *detection* of violations. A `SafetyPolicy` subscribes to
`tool.call.started` Pulses, checks the intended op against role
permissions, and publishes `safety.approval.requested` or
`safety.violation.detected` Pulses as appropriate. The Engram DAG
preserves the whole trail.

## 9. Daimon (affect engine, Phase 9) — cross-cut

`docs/00-architecture/09-daimon.md` describes PAD-vector affect as a
cross-cut injected across layers. Daimon currently updates its PAD
vector by being called from `orchestrate.rs` after gate verdicts.
In the two-fabric model Daimon subscribes to `gate.verdict.emitted`
and `agent.turn.completed` Pulses directly, updating PAD without any
orchestrator wiring. Consumers of PAD (the CascadeRouter, the
Composer's affect-biased scoring) read it from a Daimon
trait-object injection as today. Decoupled but cross-cut —
exactly what the cross-cut concept is supposed to be.

## 10. Dreams / Neuro cross-pollination

A specific Phase-2+ win: when Dreams produces consolidated
insights, it publishes `neuro.insight.promoted` Pulses. Neuro's
tier-progression policy subscribes and moves Engrams between
tiers (Transient → Working → Semantic → Procedural). The
orchestrator's context-enrichment path subscribes and rebuilds its
enrichment cache. All of this is reactive in two-fabric; in
one-noun it would require polling and is why doc 24 marked all
those arrows as MISSING.

## 11. Summary

| Phase | Subsystem | One-noun pain | Two-fabric resolution |
|---|---|---|---|
| 6 | Chain | ChainSubstrate conflates storage and events | Split into ChainSubstrate + ChainBus |
| 5C | Dreams | Polling Substrate | Subscribe to `substrate.engram.stored` |
| 13 | Coordination / Stigmergy | Custom pheromone plumbing | Pheromone = Engram in shared Substrate + `mesh.pheromone.*` Pulse |
| 12 | HTTP serve | Ad-hoc stream conversion | Bus projection over HTTP |
| 2+ | Mesh | Requires new trait family | Just another Bus/Substrate backend |
| 5+ | Multi-agent collectives | Requires bespoke orchestration | Pub/sub topologies |
| 16 | Heartbeat | Separate clock mechanism | `HeartbeatPolicy` on the Bus |
| 11 | Safety | Audit-only, no live detection | Subscribe to tool-call Pulses |
| 9 | Daimon | Explicit orchestrator wiring | Subscribe to verdict + turn Pulses |

Every cell in the right column is simpler than its left-column
counterpart. The two-fabric refactor pays for itself at the L0
kernel level *and* at every Phase-2+ extension point.

## 12. Why the user's original framing was right

> "it is structure for the whole stack, down to the lowest level,
> and should be generalized enough that you can create agents in
> Rust, and run them however you want, compose them, extend them,
> etc — there's lots of data that flows through eventbusses, and
> things are generalized, modular, flexible, and extensible enough
> to be able to be performant, smart, and create intelligence."

This description doesn't fit the one-noun framing: "eventbusses"
plural, composed agents, extensibility at every layer. It fits the
two-fabric / two-medium framing exactly:

- **Two fabrics** give you the eventbusses *and* the persistent
  store as peer primitives.
- **Six operators** give you the algebra to build agents by
  composition rather than inheritance.
- **Five layers** give you the structural skeleton "down to the
  lowest level."
- **Three speeds** let performance characteristics differ without
  forking the architecture.
- **Three cross-cuts** inject intelligence (Neuro / Daimon /
  Dreams) across all of it without breaking the layer rule.

The phrase Roko should lead with isn't "one noun, six verbs." It's
closer to: **"Roko is a cognitive runtime — two mediums flowing
through two fabrics, six composable operators acting on them, five
layers of strictly-downward dependency, three adaptive speeds, three
injected cross-cuts."** That sentence carries the whole architecture
and tells the truth about the shape.

## 13. Worked scenario: a pheromone trail that follows a new bug

To make the Phase-2 payoff concrete, trace a pheromone through the
two-fabric model end-to-end. The scenario: one agent discovers a
subtle race condition; we want later agents working on the same file
to find it.

1. **Agent A** (coder, working on `src/net/client.rs`) hits a flaky
   test. The agent decides the failure is a race condition.
2. Agent A authors an Engram of kind `Pheromone` with body
   `"race condition suspected in retry loop around line 142"` and
   tags `{ file: "src/net/client.rs", function: "with_retry" }`.
   The Engram's fingerprint (HDC, see 11) encodes the tags.
3. Agent A calls `substrate.put(engram)`. Substrate stores it with
   `Decay::Ebbinghaus` (fades if never reinforced) and publishes
   `Pulse { topic: "mesh.pheromone.deposited", kind: Pheromone,
   body: ... , lineage_hint: Some(engram_hash) }` to the Bus.
4. Three hours later, **Agent B** starts a task on the same file.
   Agent B's Composer runs `substrate.query_similar(file_fingerprint,
   kind=Pheromone)` and finds Agent A's note. It gets injected into
   the prompt.
5. Agent B's first action reinforces the pheromone (demurrage §2
   `ReinforceKind::AgentQuoted`). The Engram's balance goes up; it
   persists past its natural decay.
6. Agent B proposes a fix. Gate pipeline verifies; `GateVerdict`
   lands; `gate.verdict.emitted` Pulse fires.
7. The `FixProvenancePolicy` observes the verdict, checks which
   Engrams were cited in the input context (Agent A's pheromone
   was), and publishes `pheromone.successful` with lineage pointing
   at Agent A's original. Agent A's pheromone is now a validated
   heuristic candidate (see 14).

Every step is a Substrate put or a Bus publish. There is no
"pheromone subsystem." The entire ant-colony-optimization dynamic
falls out of the kernel primitives. And because every step has
lineage, the audit trail for *why* the fix was proposed is
inspectable end-to-end — the forensic capability from
`docs/00-architecture/02-engram-data-type.md` gets the social
coordination story for free.

## 14. Timing of Phase-2 unlocks

The Phase-2 subsystems don't all land together. Rough sequencing,
with dependencies:

| Subsystem | Depends on | Relative priority | Why the order |
|---|---|---|---|
| Heartbeat clock | Phase C done | Immediate | Under 50 lines; unlocks three-speed consumers |
| Safety-live (subscribe to tool-call Pulses) | Phase C done | High | Closes P0 safety gap |
| Dreams (subscribe to `substrate.engram.stored`) | Phase C + demurrage (12) | Medium | Needs stable Engram lineage |
| Stigmergy (pheromone) | HDC-on-Engram (11) | Medium | Depends on `query_similar` |
| Mesh (NatsBus) | `roko-mesh` crate scaffold | Medium | Depends on new crate |
| ChainBus | `roko-chain` integration | Low | Requires on-chain attestation story |
| Collectives (pub/sub topologies) | Mesh done | Low | Patterns on top of mesh |
| Daimon (live PAD) | Phase C done | Low | Scoped by team capacity |

The refactor from 01–08 is a precondition for most of these, which
is why the foundation work is worth doing before Phase-2 builds on
top. `35-consolidated-roadmap.md` stitches this list into the full
multi-quarter plan.
