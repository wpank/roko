# Refactoring Plan

> **TL;DR**: Three phases — Docs Alignment (1 week), Kernel Addition
> (2 weeks), Subsystem Migration (3–4 weeks). Each phase is
> independently mergeable and reversible. No existing functionality
> breaks until Phase 3, and even then only via compiler-assisted
> signature updates.

> **For first-time readers**: This is the "how, in what order, with what
> risk" doc. It assumes you've read 01 (critique), 02 (Pulse), 03 (Bus),
> 04 (operator generalization), and 05 (loop). If you're comparing this
> plan against the broader roadmap, see `35-consolidated-roadmap.md` for
> where Phase A/B/C/D of this plan sit among the other refinement
> sequencing (HDC, demurrage, plugin SPI, web UI, etc.).

## Phase A — Docs Alignment (1 week, doc-only, no code)

**Goal**: Update every foundational doc to the two-medium / two-fabric
framing. Remove stale "Signal = Engram" disclaimers. No runtime
changes.

### A.1 Kernel-crate docs (roko-core)

- Rewrite the doc comment at the top of `crates/roko-core/src/lib.rs`.
  New version: "Roko kernel — two mediums (Engram, Pulse), two
  fabrics (Substrate, Bus), six operators…". Preserve the "every
  capability is a trait implementation" line.
- Update module-level doc on `crates/roko-core/src/engram.rs`:
  Engram is the durable medium; Pulse is its ephemeral sibling;
  see `pulse.rs` (to be added in Phase B).

### A.2 Architecture chapter (docs/00-architecture/)

- **New** sub-doc: `02b-pulse-ephemeral-event.md` — the Pulse medium
  (copy substantial content from `02-engram-vs-pulse.md` in this
  folder).
- **New** sub-doc: `07b-bus-transport-fabric.md` — the Bus trait
  (copy from `03-bus-as-first-class.md`).
- **Rewrite**: `02-engram-data-type.md` — remove the
  "Signal vs Engram" disclaimer, clarify Engram's role as the
  durable medium. Cross-link to `02b`.
- **Rewrite**: `06-synapse-traits.md` — retitle "The Six Synapse
  Traits" → "Synapse Operators Over Two Fabrics". Update signatures
  per `04-operators-generalized.md`.
- **Rewrite**: `07-substrate-trait.md` — clarify it's one of two
  kernel fabrics (cross-link to `07b`).
- **Rewrite**: `09-universal-cognitive-loop.md` — seven-step loop
  per `05-loop-retold.md`.
- **Rewrite**: `12-five-layer-taxonomy.md` — add Bus to L0 alongside
  Substrate.
- **Update**: `INDEX.md` abstract — "one noun, six verbs" → "two
  mediums, two fabrics, six operators, five layers, three speeds,
  three cross-cuts".
- **Update**: `01-naming-and-glossary.md` — remove the
  "In Rust code it is `Signal`" language (stale), add `Pulse` +
  `Bus` + `Topic` entries.
- **Update**: `24-cross-section-integration-map.md` — the
  `EngineEventBus` proposal is no longer a proposal; it's the
  `Bus` trait. Reflag those items.
- **No change**: `23-architectural-analysis-improvements.md` —
  analysis doc, historical record. Add a footer: "Post-2026-04-16
  refinement resolved several items in §2.2 (Boundary Operations)
  and §3.2 (the roko-conductor → roko-learn violation)".

### A.3 Top-level docs

- **README.md**: Rewrite the "One noun, six verbs" paragraph (line
  118) to the two-mediums framing.
- **CLAUDE.md**: Replace line 59 ("1 noun (Signal) + 6 verb traits…")
  with the new phrase.
- **docs/INDEX.md**: Replace the "Signal / Engram Naming" section
  with a "Two Mediums" section.

### A.4 Status docs

- `docs/STATUS.md`: Add a row for `Bus` and `Pulse` under **Scaffold**
  (or wherever the Phase-B landing puts them).
- `tmp/ux-followup/11-execution-plan.md`: Insert the kernel-addition
  phase (Phase B of this plan) as a new batch prompt, T33.

### Effort — Phase A

~1 week for a single engineer. Pure documentation work, no runtime
risk. Can be done as a single PR or as seven small PRs (one per
doc).

## Phase B — Kernel Addition (2 weeks, additive code, non-breaking)

**Goal**: Add `Bus`, `Pulse`, `Topic`, `TopicFilter` to `roko-core`.
Ship `BroadcastBus` and `MemoryBus` in `roko-std`. No existing code
changes its behavior.

### B.1 roko-core additions

New modules in `crates/roko-core/src/`:

- `pulse.rs` — the `Pulse` struct, `PulseSource`, `TraceId`.
- `topic.rs` — `Topic` newtype, `TopicFilter` enum.
- Add `Bus` trait to `traits.rs` (alongside the existing six).
- Extend `Datum<'_>` enum to cover either medium.
- Add `BusReceiver`, `PolicyOutputs`.
- Add `GraduationPolicy` default impl (§3.1 of `02-engram-vs-pulse.md`).
- Add `Engram::to_pulse` and `Pulse::graduate` conversion methods.

Export everything from `lib.rs`.

### B.2 roko-std additions

- `crates/roko-std/src/bus/broadcast.rs` — wraps
  `tokio::sync::broadcast`, replay via ring buffer.
- `crates/roko-std/src/bus/memory.rs` — synchronous in-memory Bus
  for testing.

### B.3 roko-runtime simplification

- `crates/roko-runtime/src/event_bus.rs` — keep as-is but mark
  module deprecated with `#[deprecated = "use roko_core::Bus trait
  + roko_std::BroadcastBus"]` on the `EventBus` struct. Provide a
  `impl Bus for EventBus<Pulse>` shim.
- `crates/roko-runtime/src/lib.rs` — unchanged exports; add a
  re-export of `roko_core::{Bus, Pulse, Topic, TopicFilter}` for
  convenience.

### B.4 Test additions

- `crates/roko-core/tests/pulse_graduation.rs` — Pulse → Engram
  round-trip, conversion-law property tests.
- `crates/roko-std/tests/broadcast_bus.rs` — fan-out correctness,
  replay semantics, ring-wrap behavior.
- `crates/roko-core/tests/topic_filter.rs` — glob matching,
  boolean combinations.

### B.5 Test count

Target: +40–60 new tests across `roko-core` + `roko-std`. Total
workspace test count moves from ~4,508 to ~4,560.

### Effort — Phase B

~2 weeks for a single engineer. All additive; no existing behavior
changes. Can ship behind a feature flag (`--features bus-kernel`)
if extra caution is wanted, though I don't think it's necessary.

## Phase C — Subsystem Migration (3–4 weeks)

**Goal**: Port the subsystems that already use ad-hoc event enums to
the `Bus` + `Pulse` model. Fix the layer violation from doc 23.
Close the two P0 self-hosting blockers.

### C.1 Migration order (by dependency)

1. **roko-runtime** — replace `Envelope<E>` usages in callers with
   `Pulse`. Typed event enums become topic-strings + `Kind`. (Week 1.)
2. **roko-orchestrator** — `OrchestrationEvent` enum →
   `orchestration.*` topics. (Week 1.)
3. **roko-agent** — internal agent-to-agent events → `agent.*`
   topics. WebSocket sidecar in `roko-agent-server` → publishes
   Pulses instead of an ad-hoc JSON frame type. (Week 2.)
4. **roko-conductor** — remove the `roko-learn` dependency.
   Conductor subscribes to `gate.verdict.emitted` and
   `gate.failure.rate`. This closes doc-23 violation. (Week 2.)
5. **roko-learn** — `CircuitBreakerPolicy` publishes
   `gate.failure.rate` Pulses. `EfficiencyPolicy` publishes
   `efficiency.tick` Pulses. `EpisodePolicy` subscribes to
   `substrate.engram.stored`. (Week 3.)
6. **roko-cli** TUI — replace the polling code paths flagged in
   `tmp/ux-followup/12-tui-event-parity.md` with Bus subscriptions.
   Two P0 bugs close. (Week 3.)
7. **roko-serve** — WebSocket/SSE endpoints expose Bus topics to
   HTTP consumers. Replace internal broadcast channels with Bus
   subscriptions. (Week 4.)
8. **Self-hosting closure** — implement `PlanRevisionPolicy` and
   `PrdPublishPolicy`. Closes CLAUDE.md items 10 and 11. (Week 4.)

### C.2 Per-subsystem migration recipe

For each subsystem the pattern is the same:

1. Identify the ad-hoc event enum (`AgentEvent`, `OrchestrationEvent`,
   …) currently on a `tokio::sync::broadcast` channel.
2. For each variant, pick a topic string and a `Kind` + `Body` shape.
   Record the mapping in `roko-core::topics::<subsystem>` as `const`
   declarations.
3. Replace `tx.send(AgentEvent::Foo(...))` with
   `bus.publish(Pulse { topic: TOPIC_AGENT_FOO, kind: Kind::X,
   body: Body::Json(...), ... })`.
4. Replace `while let Ok(evt) = rx.recv().await { match evt { ... }}`
   with `while let Some(pulse) = receiver.next().await { match
   pulse.kind { ... }}`.
5. Delete the old enum. Run tests.
6. Check call sites across the workspace and fix.

### C.3 Breaking changes

Minimal external surface changes:
- Public types `OrchestrationEvent`, `AgentEvent`, etc. removed. These
  were not part of any stable API.
- The `EventBus<E>` shim stays for one release with deprecation warnings.

### Effort — Phase C

~3–4 weeks with 1–2 engineers. Some weeks parallelizable:
orchestrator (week 1) and agent-server (week 2) are independent.
Conductor (week 2) depends on learn publishing (week 3) in the other
direction than you'd think — actually conductor migrates before
learn, with a stub that publishes to itself, then learn lands and
becomes the real publisher.

## Phase D — Chain & Mesh Buses (Phase 2+, when chain lands)

**Goal**: Bus has multiple backend implementations, not just
broadcast.

- `ChainBus` in `roko-chain` — `chain.*` topics map to on-chain
  events; replay maps to block scanning.
- `NatsBus` in `roko-mesh` (new crate) — for multi-process
  deployments.
- `MultiBus` in `roko-core` — composes several Bus backends.

This is post-Phase-C and depends on `roko-chain` and `roko-mesh`
landing as first-class crates, which is Tier 6 in
`tmp/MASTER-PLAN.md`.

## Total effort

| Phase | Scope | Engineers | Duration |
|---|---|---|---|
| A | Docs alignment | 1 | 1 week |
| B | Kernel addition | 1 | 2 weeks |
| C | Subsystem migration | 1–2 | 3–4 weeks |
| D | Chain & mesh buses | 1–2 | Phase 2+ |
| **Total (A–C)** | | 1–2 | **~6–7 weeks** |

## Rollback plan

- Phase A — revert the documentation PRs. No runtime effect.
- Phase B — revert the kernel-addition PRs. The added types are not
  yet used anywhere critical.
- Phase C — each subsystem migration is an independent PR. Revert
  only the affected subsystem.

There is no "point of no return" inside Phases A–C. Phase D is
reversible only within the crate being extended.

## Risks

1. **Pulse ring-buffer sizing.** Too small and subscribers lose data;
   too large and memory grows. Default 4096 per bus with per-topic
   overrides; monitor ring-occupancy Pulse (`bus.ring.occupancy`).
2. **Graduation policy regressions.** A Pulse that should have
   graduated but didn't is a forensic gap. Mitigate with a
   `graduation.missed` metric Pulse emitted by the Substrate when it
   sees a `Pulse with lineage_hint` but has no matching Engram.
3. **Schema drift across Bus backends.** When chain/mesh buses land,
   a Pulse published on broadcast-bus and re-published on chain-bus
   must be the same Pulse. Canonical encoding spec (probably
   CBOR over the Pulse struct's serde impl) prevents this.
4. **Doc drift during migration.** Phase A can land before Phase B.
   If Phase B slips, the docs describe a Bus that doesn't exist.
   Mitigate by adding a "Planned" banner to the Bus docs until Phase
   B lands.

## Dependencies on existing work

None. This refactor does not block and is not blocked by the items in
`tmp/ux-followup/`. Running them in parallel is safe because:

- The gap-catalogue items work inside the current trait signatures.
- The refactor generalizes the signatures without removing them.
- Both converge on the same subsystem (e.g. TUI event parity) but
  from different angles — ux-followup fixes the polling-vs-streaming
  bug with whatever mechanism is at hand; the refactor lets the
  fix be "subscribe to a Bus topic" rather than ad-hoc.

If the refactor lands first, the ux-followup items get simpler. If
ux-followup lands first, the refactor's migration scope shrinks.
Either order works.

## Checkpoint criteria

For each phase, concrete "phase is done" definitions the team can
check rather than debate.

### Phase A done when

- Every doc under `docs/00-architecture/` that mentions "one noun" has
  been updated to "two mediums."
- `docs/00-architecture/02-engram-data-type.md` and
  `docs/00-architecture/07-substrate-trait.md` are renamed to
  `02a-*` and `07a-*` with redirects.
- `02b-pulse-ephemeral-event.md` and `07b-bus-transport-fabric.md`
  exist with a "Planned" banner.
- `CLAUDE.md`, `README.md`, and `docs/INDEX.md` no longer carry
  "Signal = Engram" disclaimers.
- A single PR against `docs/00-architecture/23-architectural-analysis-improvements.md`
  adds a footer noting which items the refactor dissolved.

### Phase B done when

- `cargo build --workspace` succeeds with `Pulse`, `Bus`, `Topic`,
  `TopicFilter`, `Datum`, `PolicyOutputs` exported from `roko-core`.
- `BroadcastBus` and `MemoryBus` in `roko-std` have >90% line coverage
  in their own test modules.
- `Pulse::graduate` round-trip property tests green on 10k random
  Pulses.
- Topic-filter glob matcher passes a hand-written spec of 30+ cases.
- `Envelope<E>` carries a `#[deprecated]` attribute but existing
  callers still compile.
- No subsystem migration has started yet. If anyone touched
  `roko-conductor` or `roko-learn` during Phase B, they rolled it
  back.

### Phase C done when

- No call site in the workspace sends to a subsystem-specific event
  enum. `rg 'enum (Orchestration|Agent|Conductor|Ui)Event' crates/`
  returns zero hits outside `#[deprecated]` shims.
- `roko-conductor`'s Cargo.toml has no `roko-learn` dependency.
- `PlanRevisionPolicy` and `PrdPublishPolicy` ship as separate
  modules with integration tests that fake `gate.verdict.emitted`
  streams.
- The TUI polling-vs-streaming bugs in
  `tmp/ux-followup/12-tui-event-parity.md` have been moved to
  DONE with linked PRs.
- `roko-serve` WebSocket/SSE endpoints forward Bus subscriptions
  via `27-realtime-event-surface.md`'s wire protocol.
- The `#[deprecated]` `Envelope<E>` has been removed.

### Phase D done when

- `ChainBus` in `roko-chain` has parity with `BroadcastBus` on the
  trait surface and has one integration test that emits a
  Pulse-from-chain-event.
- `NatsBus` in `roko-mesh` has parity and one integration test.
- `MultiBus` in `roko-core` has a test that composes `BroadcastBus`
  and an in-memory test bus.

## Metrics that should move

A refactor with no measurable effect is indistinguishable from
shuffling deck chairs. These are the numbers to track across the
phases:

| Metric | Baseline | Target after Phase C |
|---|---|---|
| Cross-crate type imports (grep `pub use .*::(Orchestration|Agent|Conductor)Event`) | N hits | 0 |
| Polling loops in TUI (`loop { sleep … query }`) | ≥2 confirmed | 0 |
| `roko-conductor` Cargo dependencies | includes `roko-learn` | excludes `roko-learn` |
| `Policy::decide(&[], ctx)` call sites | ≥3 | 0 |
| P0 bugs in `tmp/ux-followup/12-tui-event-parity.md` | 2 | 0 |
| Workspace test count | ~4,508 | ≥4,560 |
| TUI event latency p95 (poll period → delivery) | ~250 ms (poll) | <20 ms (sub) |

The last row is the user-visible win — the TUI stops feeling laggy
on token-heavy turns.

## What comes after Phase C

Every subsequent refinement in this folder presumes the kernel has
landed. Specifically:

- `10-self-learning-cybernetic-loops.md` §9 (the "Phase C.5" that
  adds `prediction.*` / `outcome.*` topics) runs right after C.
- `11-hyperdimensional-substrate.md` §11.1 (fingerprint on every
  Engram) can run in parallel with C if a second engineer is
  available.
- `12-knowledge-demurrage.md` §10 (migration path) runs after C.
- `17-plugin-extension-architecture.md` Stage A (tier-3 tool
  manifests) runs after C.
- `26-statehub-rearchitecture.md` §11 (promote StateHub to a kernel
  crate) runs after C — it depends on the new Bus trait surface.

`35-consolidated-roadmap.md` draws the full dependency graph across
all 30+ refinement docs and suggests a six-to-twelve-month landing
sequence.
