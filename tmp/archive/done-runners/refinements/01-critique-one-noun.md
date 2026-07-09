# Critique: "One Noun, Six Verbs" Is Selling Roko Short

> **TL;DR**: The current foundational framing conflates two distinct data
> shapes, pretends the event bus isn't an architectural primitive, and
> stretches trait signatures to cover cases they don't naturally fit. The
> framing should evolve, not just have "Signal" renamed to "Engram."

> **For first-time readers**: Roko's current docs describe the system as
> "one noun (Engram — a durable, hashed, scored record) and six verb traits
> (Substrate, Scorer, Gate, Router, Composer, Policy)." This doc argues that
> framing no longer matches the code. The subsequent docs (02–09) propose the
> replacement — a two-medium, two-fabric, six-operator kernel. Read this one
> first for the diagnosis; read 02 and 03 for the cure.

## 1. Where the phrase comes from

The phrase appears verbatim in at least three places:

- `crates/roko-core/src/lib.rs` lines 5–15: "The entire Roko system is built
  from **one noun** ([`Engram`]) and **six verbs**".
- `docs/00-architecture/INDEX.md` abstract: "The Synapse Architecture is
  Roko's compositional foundation: one noun (Engram) and six verb traits".
- `docs/00-architecture/06-synapse-traits.md` §1.1: "The number six is not
  arbitrary. It emerged from analyzing the complete Roko design corpus…
  Every capability, without exception, reduces to one of these six operations."

`CLAUDE.md:59` also repeats it ("1 noun (Signal) + 6 verb traits…") with
the old name still attached — which is itself evidence that the framing is
stale.

## 2. Problem A — There are already two data shapes

### 2.1 Engram (durable)

`crates/roko-core/src/engram.rs` defines the content-addressed,
lineage-bearing record:

```
id: ContentHash           // BLAKE3(kind + body + author + tags)
kind: Kind
body: Body
tags: BTreeMap<String, String>
created_at_ms: i64
decay: Decay              // None | HalfLife | Ttl | Ebbinghaus
score: Score              // 7-axis appraisal
lineage: Vec<ContentHash> // parent Engrams (audit DAG)
provenance: Provenance
attestation: Option<Attestation>
```

This is an **artifact**. It has an identity, a DAG position, a decay curve,
a trust chain, and eventually a cryptographic attestation. It is designed to
survive, to be audited, and to be promoted on-chain.

### 2.2 Envelope / event (ephemeral)

`crates/roko-runtime/src/event_bus.rs:59` defines a completely separate
shape:

```
pub struct Envelope<E> {
    pub seq: u64,
    pub emitted_at_ms: i64,
    pub event: E,   // user-provided generic event type
}
```

This is an **in-flight message**. It has a sequence number and a timestamp
but no hash, no lineage, no decay, no provenance, no score. It is designed
to fan out to subscribers and live briefly in a ring buffer for replay.

The EventBus is parameterized over the user's event type `E`. Every caller
invents its own event enum — `OrchestrationEvent`, `AgentEvent`, `UiEvent`,
and so on. None of them are Engrams. None of them need to be.

### 2.3 The architecture cannot explain this cleanly today

Because the docs say "one noun, Engram," every subsystem that wants to
communicate either (a) forces its messages through the EventBus and calls
them "events" (undocumented in architecture docs, invisible to the
universal loop), or (b) materializes everything as Engrams and writes to
the Substrate, which is expensive and wrong for things like heartbeat
ticks or cancellation signals.

The docs compensate with phrases like "Policy observes a stream of
signals," but never acknowledge that those streams are Envelopes on a
Bus, not Engrams in a Substrate.

## 3. Problem B — The bus isn't part of the architectural lexicon

`docs/00-architecture/24-cross-section-integration-map.md` §6 openly
proposes an `EngineEventBus` to fix cross-section integration, with this
diagnosis:

> Currently subsystems communicate through direct compile-time dependencies,
> leading to the `roko-conductor → roko-learn` layer violation flagged in
> doc 23. An event bus inverts the dependency: subsystems publish and
> subscribe to typed topics, and the bus is the integration map.

That proposal is 100% correct, and the bus already exists in
`roko-runtime`. What's missing is architectural recognition: the Bus is
not called out in the five-layer taxonomy, is not listed among the six
traits, and has no `docs/00-architecture/XX-bus.md` sibling to the
Substrate deep-dive at `docs/00-architecture/07-substrate-trait.md`.

This is why `roko-conductor` reaches across layers into `roko-learn`. If
the Bus were a kernel primitive, the conductor would subscribe to a
`gate.verdict` topic and `roko-learn` would be just another subscriber
computing EMAs — and the dependency would flow the right way.

## 4. Problem C — The trait signatures stretch to fit

The trait definitions in `crates/roko-core/src/traits.rs` claim to operate
uniformly on `&Engram`:

- `Substrate::put(signal: Engram)` — fine, Engram is what gets stored.
- `Scorer::score(signal: &Engram, ctx: &Context) -> Score` — fine when
  scoring a stored Engram, awkward when scoring a live event (you have to
  materialize).
- `Gate::verify(signal: &Engram, ctx: &Context) -> Verdict` — fine, gates
  verify Engrams.
- `Router::select(candidates: &[Engram], ctx: &Context) -> Option<Selection>`
  — mostly fine, but model routing and tool selection don't really produce
  Engram candidates; they produce *choices*, which are then logged as
  Engrams after the fact.
- `Composer::compose(signals: &[Engram], budget, scorer, ctx) -> Result<Engram>`
  — fine, the Composer's output is an Engram (the prompt).
- `Policy::decide(stream: &[Engram], ctx: &Context) -> Vec<Engram>` — this
  one is the tell. "Stream of Engrams" is a workaround for "stream of
  Pulses." Conductor watchers, circuit breakers, and heartbeat policies
  all want to react to live events, not to historical Engrams. Today they
  either (a) convert Envelopes to synthetic Engrams, or (b) bypass the
  trait entirely and subscribe to the EventBus directly.

`docs/00-architecture/23-architectural-analysis-improvements.md` §2.2
explicitly acknowledges this:

> Telemetry emission (metrics, traces) — current implementation:
> `Policy::decide(&[], ctx)` returning metric Engrams. Fit quality:
> Adequate. Empty stream input is awkward but functional.

"Adequate" and "awkward but functional" are the usual symptoms of an
abstraction doing two jobs.

## 5. Problem D — The universal loop hides three different sense sources

`docs/00-architecture/09-universal-cognitive-loop.md` step 1 is "PERCEIVE
→ Substrate.query() → What is happening?" In practice the agent runtime
perceives three different ways:

1. **Substrate.query** — durable Engrams. Used for context retrieval,
   episode lookup, plan discovery.
2. **Bus.subscribe** — live Pulses. Used for process lifecycle, approval
   requests, cancellation, circuit-breaker trips, gate verdicts in
   flight, token streams.
3. **External I/O** — WebSocket chunks from the LLM, stdout from a tool
   subprocess, HTTP requests on `roko-serve`. These are the edge of the
   runtime; they produce Pulses (and eventually Engrams), but they aren't
   either of the above.

Flattening these three into "Substrate.query" either forces everything
through hash-and-store (expensive, wrong for heartbeats) or quietly
routes most real work outside the loop description (the status quo).

## 6. Problem E — The name "Signal" is sitting idle

The rename `Signal → Engram` in code (per
`docs/00-architecture/01-naming-and-glossary.md` and verified by
`grep -rn`: Engram 877, Signal 5) freed an excellent name. "Signal" in
engineering usage *means* an in-flight event — a notification, an
interrupt, a pub/sub message. It is the natural name for what
`EventBus<E>::Envelope<E>` carries.

Leaving the name idle while carrying a stale "Signal = Engram" disclaimer
in every doc is a missed opportunity.

## 7. What the critique does *not* say

A few things that are correct and should not change:

- The **six traits themselves** are the right decomposition. Doc 23's
  audit of all 131 trait implementations showed no 7th trait is needed.
  The traits' *signatures* should generalize; the trait *set* is fine.
- The **five-layer taxonomy** is right. Strictly-downward dependencies
  is the correct rule. The Bus belongs at L0, same as Substrate.
- The **three-speed model** (Gamma / Theta / Delta) is right and
  orthogonal to any of this.
- The **three cross-cuts** (Neuro / Daimon / Dreams) are right as
  trait-object injections across layers.
- The **content-addressed DAG** is right and is the core innovation. The
  critique is that not every message needs to be a hashed DAG node.

## 8. Summary

The "one noun, six verbs" framing was a useful mnemonic at the start of
the project. It remains useful for explaining the durable half of the
system. But the runtime grew a second half — the bus and its messages —
that the framing doesn't acknowledge, and the gap now shows up as layer
violations, awkward trait usage, leaky loop descriptions, and a stale
Signal-vs-Engram disclaimer in every foundational doc.

The rest of this folder proposes the minimal refactor that dissolves
these problems without throwing away anything that works today.

## 9. Smoke tests for the critique

If this critique is right, three predictions should hold when someone greps
the codebase today:

1. **Ad-hoc event enums exist in multiple crates.** Run
   `rg 'enum (Orchestration|Agent|Ui|Conductor)Event' crates/` — each hit
   is a subsystem that invented its own bus vocabulary because the kernel
   didn't have one.
2. **Polling loops appear where subscriptions should.** Look for `loop { sleep(..); query(..); }`
   patterns in `crates/roko-cli/src/tui/` — the confirmed P0 in
   `tmp/ux-followup/12-tui-event-parity.md` is the exemplar.
3. **`Policy::decide(&[], ctx)` appears with empty slices.** Run
   `rg 'decide\(\s*&\[\]' crates/` — every hit is a Policy that really
   wanted to subscribe to a stream but was forced to materialize nothing as
   a zero-length Engram slice.

If any of these three predictions fails, the critique should be tempered.
As of 2026-04-16, all three hold. See `08-code-sketches.md` for the
conductor-port worked example that makes the fix concrete.

## 10. What this critique is and isn't

This is **not** a critique of the authors who wrote "one noun, six verbs."
It was the right framing for the codebase at the time it was written. The
framing outgrew itself the moment the Bus landed in `roko-runtime` and the
TUI became a live consumer — roughly, the moment Roko stopped being a
batch "query → respond → store" loop and started being an agent runtime
with subprocesses, streams, and subscribers.

The refactor this folder proposes is the natural next chapter, not a repudiation.
Doc 06 (refactoring plan) stages it so that no subsystem breaks. Doc 21
discusses which pieces benefit from a clean rewrite versus an incremental
edit. If the rewrites in 21 are too aggressive, the incremental path in 06
still dissolves the three problems above. Either way, the critique stands.
