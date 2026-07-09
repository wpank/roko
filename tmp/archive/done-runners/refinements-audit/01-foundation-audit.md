# Foundation Arc Audit (Refinements 01-09)

Auditor: Claude Opus 4.6, 2026-04-17
Method: Read all 9 refinement docs, then cross-referenced against actual
codebase (`roko-core`, `roko-runtime`, `roko-conductor`, `roko-learn`,
`roko-cli/src/orchestrate.rs`, `roko-serve`, `roko-agent`).

---

## 01 — Critique: "One Noun, Six Verbs" Is Selling Roko Short

**Verdict: PARTIALLY AGREE**

### Diagnosis accuracy

The critique makes three testable predictions (section 9). Here is how they
hold against the actual codebase:

**Prediction 1 — Ad-hoc event enums exist in multiple crates.**
CONFIRMED. Grepping the codebase finds:
- `roko-learn/src/events.rs:15` — `pub enum AgentEvent` (17 variants)
- `roko-agent/src/task_runner.rs:73` — `pub enum AgentEvent` (duplicate,
  different variant set)
- `roko-runtime/src/event_bus.rs:103` — `pub enum RokoEvent`
  (PlanRevision, PrdPublished)
- `roko-serve/src/events.rs:84` — `pub enum ServerEvent` (PlanStarted,
  PlanCompleted, AgentSpawned, AgentOutput, GateResult, Execution,
  PhaseTransition, Episode)

Four different event enums across four crates. The `AgentEvent` name is
defined twice — in `roko-learn` and `roko-agent` — with overlapping but
non-identical variant sets. This is exactly the problem described.

**Prediction 2 — Polling loops appear where subscriptions should.**
PARTIALLY CONFIRMED. The TUI uses `crossterm::event::poll(Duration::from_millis(250))`
in `tui/app.rs:396` and a tick-rate based polling loop in `tui/event.rs`.
However, this is standard TUI terminal event polling, not polling the
Substrate for data. The `git_watch.rs` module has an explicit polling
fallback, but it's for filesystem events (inotify fallback), not for
Substrate queries. The critique is less clean than claimed here — the TUI's
"polling" is mostly about terminal I/O, not about missing Bus integration.

**Prediction 3 — `Policy::decide(&[], ctx)` appears with empty slices.**
CONFIRMED. Found in:
- `roko-conductor` watchers: 10 call sites across all 10 watchers (in tests)
- `roko-core/src/cfactor.rs:167,196` — CFactorPolicy called with empty
  stream in tests
- `roko-cli/src/orchestrate.rs:347` — production code, CFactorPolicy
  invoked with `decide(&[], &Context::now())`

The production call site in `orchestrate.rs` is the strongest evidence.
The test call sites in conductor are less damning — tests often pass empty
inputs as a baseline assertion.

### Where the critique overstates

1. **"The architecture cannot explain this cleanly today"** — this is
   partially true but overstated. The event bus was intentionally built as
   an infrastructure primitive in `roko-runtime`, not as a kernel concept.
   It works. The question is whether formalizing it as a kernel primitive
   is worth the churn versus just documenting it better.

2. **The `Envelope<E>` criticism** — the critique treats the generic
   `Envelope<E>` as evidence of architectural incoherence. But generic
   typing is normal Rust practice. The real problem isn't `Envelope<E>` —
   it's the four incompatible event enums that get stuffed into it. You
   could fix that by unifying the enums without introducing Pulse at all.

3. **Problem E (idle Signal name)** — the audit of the codebase shows the
   rename Signal -> Engram is indeed done (only `CatalystSignalSource`
   remains in `roko-core/src/lib.rs` exports, plus doc comments in
   `Substrate` trait still say "signal" 6+ times). But reclaiming "Signal"
   for a new purpose is a documentation maintenance nightmare, and the
   critique itself wisely advises against it in 07.

### What the critique gets right

The `roko-conductor -> roko-learn` dependency is real and confirmed in
`crates/roko-conductor/Cargo.toml:15`. Specifically,
`roko-conductor/src/watchers/context_window_pressure.rs` imports
`roko_learn::efficiency::AgentEfficiencyEvent`. This is a genuine layer
violation.

The proliferation of event types is also real. The orchestrator imports
THREE separate event bus systems:
```
use roko_agent::task_runner::EventBus as RunnerEventBus
use roko_learn::events::{AgentEvent, EventBus as LearningEventBus}
use roko_runtime::event_bus::{EventBus as RuntimeEventBus, ...}
```
This is messy.

### Better alternative

Before introducing Pulse as a formal kernel type, consider a simpler fix:
unify the event enums into a single `RokoEvent` enum (already started in
`roko-runtime/src/event_bus.rs`) and use the existing `EventBus<RokoEvent>`
everywhere. The `global_event_bus()` function already exists. The problem
is not the absence of Pulse — it's the absence of discipline.

---

## 02 — Two Mediums: Engram (Durable) and Pulse (Ephemeral)

**Verdict: OVERCOMPLICATED**

### Diagnosis accuracy

The split between durable and ephemeral data is real. Token stream chunks
should not be Engrams. Heartbeat ticks should not be Engrams. The
diagnosis is correct.

### What is wrong with the proposed solution

1. **Pulse reuses `Kind` and `Body` from Engram.** This sounds elegant but
   creates a semantic mismatch. `Kind::GateVerdict` on an Engram means
   "this is a verified, hashed, lineage-bearing gate result." The same
   `Kind::GateVerdict` on a Pulse means "this is an ephemeral notification
   that a gate ran." These are different things wearing the same name. When
   code dispatches on `Kind`, it now has to ask "but is this a real one or
   a preview?" This is the dual of the problem the critique identifies in
   01 section 4.

2. **`Pulse::graduate()` is a factory method pretending to be a conversion.**
   The method takes `provenance`, `decay`, `score`, and `tags` — four
   fields the Pulse does not have. This is not "graduating" an existing
   datum; it's constructing a new Engram that happens to share `kind` and
   `body` with a Pulse. You could get the same result with
   `Engram::builder(pulse.kind).body(pulse.body).build()`, which is
   shorter and makes the construction explicit.

3. **The graduation policy table (section 3.1) is configuration masquerading
   as architecture.** Whether `agent.msg.chunk` gets persisted is a runtime
   policy decision. Baking it into the type system adds complexity without
   adding correctness.

4. **The `lineage_hint: Option<ContentHash>` field** is a half-measure. An
   Engram has `lineage: Vec<ContentHash>` (multiple parents); a Pulse gets
   a single optional hint. If lineage matters, why limit it to one? If it
   does not, why carry it at all? The answer ("some Pulses contextualize an
   Engram") is better handled by including the Engram hash in the Pulse's
   `body` JSON, not by adding a structural field.

### What is actually needed

The codebase needs a unified event type. Not a second first-class kernel
noun. Consider:

```rust
pub enum RokoEvent {
    PlanRevision { ... },
    PrdPublished { ... },
    AgentTurnStarted { ... },
    AgentTurnCompleted { ... },
    GateVerdictEmitted { ... },
    TokensUsed { ... },
    // etc.
}
```

This is what `roko-runtime/src/event_bus.rs` already started building.
The `RokoEvent` enum currently has 2 variants. Expanding it to 20-30
covers every use case in the current codebase without introducing a new
kernel primitive, a graduation law, or a `Datum` enum.

---

## 03 — Bus as a First-Class Kernel Primitive

**Verdict: PARTIALLY AGREE — but the scope is wrong**

### Diagnosis accuracy

The Bus exists and works (`roko-runtime/src/event_bus.rs`, 268 lines,
well-tested). It is not in the kernel (`roko-core`). This is a fact.

The `roko-conductor -> roko-learn` layer violation is real. Confirmed:
`crates/roko-conductor/Cargo.toml:15` has `roko-learn = { path = "../roko-learn" }`.

### What the proposal gets right

- Moving the Bus trait to `roko-core` so it's at the same level as
  `Substrate` is reasonable.
- The TopicFilter concept with glob matching is a good idea.
- Fixing the conductor/learn dependency via pub/sub is the right approach.

### What the proposal gets wrong

1. **The proposed Bus trait has too many methods.** `publish`, `subscribe`,
   `replay_since` — good. `current_seq`, `total_published`, `ring_len`,
   `ring_capacity` — these are implementation details of a ring-buffer
   backend, not trait surface. A Bus backed by NATS or Kafka would not have
   a "ring capacity." The trait is leaking its first implementation.

2. **BusReceiver wrapping mpsc::Receiver<Pulse>** — this hardcodes Pulse
   as the payload type. If we take the simpler path (unified RokoEvent
   enum instead of Pulse), the Bus trait should be generic over the event
   type: `Bus<E>`. This is exactly what the current `EventBus<E>` already
   does. The refinement is proposing to remove generality that the existing
   code correctly has.

3. **Moving the event bus impl from `roko-runtime` to `roko-std`** is
   churn. The current location is fine. `roko-runtime` is already L0.

### Better alternative

- Add a `Bus` trait to `roko-core::traits` with three methods: `publish`,
  `subscribe`, `replay_since`. Keep it generic: `Bus<E: Clone + Send + Sync>`.
- Add `TopicFilter` support as an optional extension.
- Fix the conductor/learn dependency by having both subscribe to a shared
  bus instance.
- Do NOT introduce Pulse as the hardcoded payload type.
- Do NOT move the implementation out of `roko-runtime`.

---

## 04 — The Six Operators, Generalized Over Two Mediums

**Verdict: OVERCOMPLICATED**

### What the proposal does

Changes every trait signature to accept `Datum<'a>` (either Engram or
Pulse) instead of just `&Engram`. Adds `score_engram` / `score_pulse`
pairs to every trait. Changes Policy from `&[Engram]` to `&[Pulse]`.
Introduces `PolicyOutputs { pulses: Vec<Pulse>, engrams: Vec<Engram> }`.

### Problems

1. **The Datum enum is premature abstraction.** Today there are ~15 Scorer
   implementations, ~11 Gate implementations, and ~10 Policy implementations
   across the codebase. None of them need to score Pulses. None of them
   verify Pulses. Generalizing every signature to accept both types to
   enable speculative future use cases ("what if we wanted to score a
   health Pulse?") violates YAGNI.

2. **`score_engram` / `score_pulse` pairs double the trait surface.**
   Every trait implementor now has two methods to consider, plus the
   dispatcher. For a codebase with ~131 trait implementations (per doc 23),
   this is a significant maintenance burden for hypothetical future
   flexibility.

3. **The Policy signature change is breaking for every existing
   implementation.** `decide(&[Engram], ctx) -> Vec<Engram>` changes to
   `decide(&[Pulse], ctx) -> PolicyOutputs`. The doc acknowledges this is
   "the most consequential signature change" and proposes a shim. A shim
   that persists through a migration window is architecture debt being
   introduced intentionally.

4. **The "generalized Router" with `select_pulse` returning
   `Option<Selection>` does not make sense.** A Selection in the current
   code carries a `ContentHash` of the chosen Engram. Pulses do not have
   content hashes. The Selection type would need to change too, which
   ripples further.

### What would actually help

If Pulse exists (which I argue against, per 02), the traits that need it
should get it:
- **Policy**: yes, change to accept events/pulses. This is the one trait
  that genuinely wants to react to live streams.
- **Gate, Scorer, Router, Composer**: leave them on `&Engram`. They
  operate on durable data. If you need to verify a stream, graduate the
  stream into a synthetic Engram first (which the existing code already
  does implicitly).

Do not generalize all six traits to save one or two future call sites.

---

## 05 — The Universal Loop, Retold

**Verdict: PARTIALLY AGREE — diagnostic is good, prescription is too much**

### What is right

1. **Step 1 (PERCEIVE) is indeed three things.** The current loop_tick in
   `roko-core/src/loop_tick.rs` only does `substrate.query()`. The actual
   orchestrator in `roko-cli/src/orchestrate.rs` also subscribes to event
   buses and reads external I/O. The doc correctly identifies that the
   formal loop_tick does not match the real runtime.

2. **Steps 2 and 3 (EVALUATE and ATTEND) do collapse.** The Router uses
   the Scorer internally. Separating them in the doc was always misleading.

3. **Step 9 (META-COGNIZE) is indeed a cross-cut, not a loop step.**
   Correct diagnosis. Daimon is injected, not sequenced.

### What is wrong

1. **The proposed 7-step TickConfig struct** adds `bus: Option<&dyn Bus>`
   to every loop_tick call. This is a breaking change to a function that
   currently has 9 parameters (already a code smell). The `Option` wrapping
   makes it optional, but then callers that do not use the Bus get a dead
   parameter.

2. **The revised loop_tick code snippet** (section 8) is approximately
   correct Rust, but it intermixes concerns that the current clean
   separation handles better. The current loop_tick is pure: query, route,
   compose, verify, persist. The revised version adds bus publishing,
   policy reactions, pulse draining — all inside one function. This makes
   the function harder to test and harder to reason about.

3. **"The self-hosting closure becomes a one-liner"** (section 7) is
   aspirational, not accurate. `PlanRevisionPolicy` and `PrdPublishPolicy`
   are already partially implemented in the existing `RokoEvent` enum
   (`PlanRevision` and `PrdPublished` variants) and in `orchestrate.rs`
   (`handle_runtime_event`). The two-fabric model does not make these
   "one-liners" — it changes where the subscription lives, but the logic
   is the same.

### Better alternative

- Update the documentation to describe the real loop honestly: "SENSE
  (3 sources) -> ASSESS (score+route) -> COMPOSE -> ACT -> VERIFY ->
  PERSIST -> REACT." Seven steps, yes. But do this as a doc update, not
  as a refactor of `loop_tick.rs`.
- Keep `loop_tick` as the pure Substrate-only function it is today.
- Build the Bus-aware orchestration loop in `orchestrate.rs`, where it
  already lives.

---

## 06 — Refactoring Plan

**Verdict: PARTIALLY AGREE — phasing is correct, scope is too large**

### What is right

1. **Three phases (docs, kernel, subsystem) is the correct sequencing.**
   Docs-first prevents the "code landed but nobody knows why" problem.
   Kernel-addition-before-migration prevents "subsystems migrated to
   something that does not exist."

2. **Phase A (docs alignment) is pure upside.** One week of doc work,
   zero runtime risk. Should happen regardless of whether Pulse lands.

3. **Phase B checkpoint criteria are concrete and measurable.** "Cargo
   build succeeds," "property tests green on 10k random Pulses" — these
   are good definitions of done.

4. **Rollback plan is honest.** Each phase is independently revertible.

### What is wrong

1. **6-7 weeks of engineering time** for a refactor whose primary benefit
   is "the TUI stops polling" and "the conductor/learn dependency is
   cleaner." The TUI polling issue could be fixed in a day by subscribing
   to the existing `EventBus<RokoEvent>`. The conductor/learn dependency
   could be fixed in a few hours by extracting `AgentEfficiencyEvent` to
   `roko-core`.

2. **Phase C migrates subsystems that work.** The ad-hoc event enums are
   ugly but functional. `orchestrate.rs` is 4000+ lines and works. The
   migration risk for "replace every event emit site" across the entire
   codebase is non-trivial, and the benefit is mostly aesthetic.

3. **"No data shape changes"** is technically true (Pulse reuses Kind and
   Body) but misleading. The trait signature changes in Phase C are
   breaking. Every Policy implementation must be rewritten. Every
   subscriber must change from `match event { AgentEvent::Foo => ... }`
   to `match pulse.kind { Kind::Foo => ... }`.

### Better alternative

- **Do Phase A.** Update docs. This is free.
- **Do a minimal Phase B.** Add a `Bus` trait to `roko-core` (generic,
  not Pulse-specific). 2-3 days, not 2 weeks.
- **Skip Phase C as written.** Instead, fix the two concrete problems:
  1. Extract `AgentEfficiencyEvent` to `roko-core` to break the
     conductor/learn dependency. (1 hour.)
  2. Unify the event enums into `RokoEvent`. (1 week.)
- **Defer Phase D** until chain/mesh actually exist.

---

## 07 — Naming Decisions

**Verdict: AGREE (largely)**

### What is right

1. **`Engram` stays.** Correct. 877 occurrences. The rename is done and
   should not be revisited.

2. **Do not reclaim `Signal`.** Correct. The rename history would make
   grep ambiguous. The codebase still has residual "signal" references in
   doc comments (e.g., Substrate trait says "Store a signal" on line 35 of
   traits.rs, and `roko-core/README.md:3` still says "One noun (`Signal`)").
   These should be cleaned up but do not justify reusing the name.

3. **`Event` is too generic.** Correct. Collides with half the Rust
   ecosystem.

4. **`Bus` for the transport trait.** Correct. Short, clear, standard.

5. **`Topic` for routing handle.** Correct.

### What deserves pushback

1. **`Pulse` as a name** — it is fine if we adopt the Pulse concept. But
   if we take the simpler path (unified `RokoEvent` enum), we do not need
   a new name at all. The naming section is correct *given the assumption
   that a new type is needed*, but that assumption is the thing under
   dispute.

2. **The `Datum` enum** — naming an either-medium type `Datum` is
   confusing. In Latin, "datum" means "a given thing" (i.e., a fact). An
   enum that might be a durable record or an ephemeral blip is not "a
   given" — it is "one of two possible things." If this enum must exist,
   `DataRef` or `Input` would be clearer.

3. **The topic namespace registry** (section 7-8) is over-specified at
   this stage. Reserving `orchestration.*`, `agent.*`, `gate.*`, etc. as
   `const` declarations in `roko-core` before there is a single Bus
   subscriber is premature. Define topics when consumers exist.

---

## 08 — Code Sketches

**Verdict: PARTIALLY AGREE — good reference, some issues**

### What is right

1. **The `Pulse` struct** is well-designed as a sketch. Clean fields,
   sensible derives, good doc comments.

2. **The `TopicFilter` enum with glob matching** is well-implemented.
   The recursive `glob_segments` function is correct and the test suite
   covers the important cases.

3. **The conductor port (section 5)** is the strongest part of the entire
   refinement set. The before/after comparison is concrete, the layer
   violation dissolves, and the code is plausible Rust. This is the one
   example where the two-fabric model clearly produces a better outcome
   than the status quo.

4. **The `PlanRevisionPolicy` sketch (section 6)** is reasonable and
   close to what `orchestrate.rs` already does with
   `build_gate_failure_plan_revision()` and `handle_runtime_event()`.

### What is wrong

1. **`BroadcastBus::publish` takes `&self` but mutates `pulse.seq`.**
   The sketch passes `mut pulse` by move. This is fine if the Bus
   consumes the Pulse, but the caller might want to keep a reference.
   The real `EventBus::emit` in `roko-runtime` takes ownership too, so
   this is consistent with existing practice — but the clone in the ring
   push is a cost the sketch does not discuss.

2. **`BusReceiver::new` is called in the `subscribe` method** but is not
   defined in the code sketch for `BusReceiver` (only `recv` and
   `last_seq` are shown). Minor omission.

3. **The conductor port drops all the internal state management.** The
   real `CircuitBreaker` in `roko-conductor/src/conductor.rs` has EMA
   calculations, threshold adaptation, and watcher composition. The
   sketch replaces all of this with "subscribe to `gate.failure.rate` and
   compare a float." The refactor is simpler, yes, but because it moved
   the complexity out of scope (to `roko-learn`'s `FailureRatePolicy`),
   not because it eliminated it.

4. **The `FailureRatePolicy` sketch uses `parking_lot::Mutex`** for a
   `Vec<(i64, bool)>` that gets scanned on every event. This is O(n) per
   event where n is the window size. The existing EMA in `roko-learn` is
   O(1). The sketch trades algorithmic efficiency for architectural
   cleanliness.

---

## 09 — Phase 2+ Implications

**Verdict: AGREE in spirit, but speculative**

### What is right

1. **ChainBus as a separate backend from ChainSubstrate** makes sense.
   Storage and event notification are different concerns.

2. **Dreams subscribing to `substrate.engram.stored`** is a clean pattern.
   Reactive consolidation instead of polling.

3. **Stigmergy as Engram+Pulse** is elegant on paper.

4. **HTTP control plane as Bus projection** matches what `roko-serve`
   already does with `ServerEvent`. The SSE/WebSocket endpoints are
   already forwarding events.

5. **Heartbeat as a Policy publishing Pulses** is clean and small.

### What is wrong

1. **Everything in this doc is Phase 2+.** The codebase is at Phase 1.
   Designing kernel primitives around the needs of systems that do not
   exist yet (ChainBus, MeshBus, NATS, collective intelligence topologies)
   is speculative architecture. The refinements should optimize for Phase 1
   needs (fix conductor/learn, unify events, clean up docs) and leave
   Phase 2+ extensibility as a "nice to have" rather than a design driver.

2. **The pheromone trail worked example (section 13)** requires HDC
   fingerprinting, Ebbinghaus decay curves, demurrage, and multiple
   interacting agents — none of which exist yet. The example is
   aspirational, not demonstrative.

3. **"Every cell in the right column is simpler than its left-column
   counterpart"** (section 11) — this is true for the descriptions, but
   the Phase 2+ subsystems do not exist in either model. Claiming one
   model makes them "simpler" when neither model has implemented them is
   unfalsifiable.

### What is actually useful from this doc

The one concrete takeaway: if we add a `Bus` trait to `roko-core`, we
should keep it generic enough that future backends (NATS, chain event
logs) can implement it. This argues *against* hardcoding Pulse as the
payload type and *for* keeping the Bus generic (as the existing
`EventBus<E>` already does).

---

## Cross-Cutting Concerns (All 9 Docs)

### 1. The diagnosis is stronger than the prescription

The critique (01) correctly identifies real problems:
- Four incompatible event enums
- `roko-conductor -> roko-learn` layer violation
- `Policy::decide(&[], ctx)` anti-pattern in production code
- Stale "Signal = Engram" references in docs

The prescription (02-08) proposes a 6-7 week refactor introducing two new
kernel types (Pulse, Datum), one new kernel trait (Bus), signature changes
to all six existing traits, and subsystem-wide migration. This is a large
investment whose primary concrete deliverables (fix event proliferation,
fix conductor/learn, fix docs) could be achieved in approximately 1 week
with targeted fixes.

### 2. The Pulse type solves a real problem the wrong way

The real problem: ephemeral messages (token chunks, heartbeats, UI
refreshes) should not be Engrams. The right fix is: do not make them
Engrams. The current code already does not make them Engrams — it sends
them on `EventBus<AgentEvent>` and `EventBus<RokoEvent>`. The problem is
not the absence of a Pulse type; it is the proliferation of incompatible
event enums. Unify the enums. Document the pattern. Move on.

### 3. The Bus trait addition is the most defensible part

Adding a `Bus` trait to `roko-core` is the right move. But keep it:
- Generic (`Bus<E>`, not `Bus` hardcoded to `Pulse`)
- Minimal (publish, subscribe, replay_since — not ring_len, ring_capacity)
- At L0, alongside Substrate

This is approximately a 100-line addition to `roko-core/src/traits.rs`
plus a one-paragraph doc update. It does not require Pulse, Datum,
PolicyOutputs, or any trait signature changes.

### 4. The 7-step loop revision is good documentation

Regardless of whether Pulse lands, the 9-step loop should be revised to
7 steps in the docs. The collapsing of EVALUATE+ATTEND and the removal
of META-COGNIZE as a step (it's a cross-cut) are correct observations.
This is a doc change, not a code change.

### 5. Naming cleanup is needed regardless

The codebase still has "signal" references in:
- `crates/roko-core/src/traits.rs` — Substrate trait doc comments say
  "Store a signal" (line 35), "Retrieve a signal" (line 38), etc.
- `crates/roko-core/README.md:3` — "One noun (`Signal`)"
- `crates/roko-core/src/kind.rs:1` — "Engram kinds -- what a signal
  represents"
- `CLAUDE.md:59` — "1 noun (Signal)"

These should be cleaned up. This is 30 minutes of find-and-replace, not
a 6-week refactor.

### 6. Complexity budget

The codebase is ~177K LOC across 18 crates. The refinements propose
adding approximately 1500 lines of new kernel types (Pulse, Topic,
TopicFilter, Datum, BusReceiver, PolicyOutputs, GraduationPolicy,
BroadcastBus, MemoryBus), changing every trait signature, and migrating
every event publisher and subscriber. Against a working system with
known, targeted problems, this is an expensive trade.

The right question is: **what is the minimum change that fixes the
concrete problems?**

1. Unify event enums into `RokoEvent`. (~200 lines changed)
2. Extract `AgentEfficiencyEvent` to break conductor/learn. (~20 lines)
3. Add a generic `Bus<E>` trait to `roko-core`. (~100 lines)
4. Clean up stale "Signal" references. (~50 lines)
5. Revise loop documentation to 7 steps. (~doc change only)

Total: approximately 370 lines of code changes + documentation updates.
Timeline: 1 week. Achieves the same concrete outcomes the refinements
propose in 6-7 weeks.

### 7. What the refinements get exactly right

Despite the overcomplicated prescription, the refinements demonstrate
genuine architectural insight:

- The event bus is an L0 primitive that deserves trait-level recognition.
- The conductor/learn dependency should be inverted via pub/sub.
- Policy's current signature (`&[Engram]`) does not match how policies
  actually consume data in the runtime.
- The 9-step loop should be 7 steps.
- "One noun, six verbs" is no longer an accurate summary of the system.

These observations should inform the simpler fix described above.

### 8. Risk: documentation drift

Phase A (docs alignment) is proposed as a precursor to Phase B (code).
If Phase A lands and Phase B does not, the docs describe a system that
does not exist. The refinements acknowledge this risk (section 4, risk 4)
and propose a "Planned" banner. This is adequate mitigation but worth
flagging: do not update docs to describe Pulse until Pulse exists in code.

---

## Summary Table

| Doc | Verdict | Key finding |
|-----|---------|-------------|
| 01 — Critique | PARTIALLY AGREE | Diagnosis is correct; predictions 1 and 3 confirmed, prediction 2 weaker than claimed |
| 02 — Engram vs Pulse | OVERCOMPLICATED | Pulse solves a real problem the wrong way; unified RokoEvent enum is simpler |
| 03 — Bus as First-Class | PARTIALLY AGREE | Bus trait in roko-core is right; but keep generic, keep minimal |
| 04 — Operators Generalized | OVERCOMPLICATED | Datum enum and dual signatures are premature; only Policy actually needs change |
| 05 — Loop Retold | PARTIALLY AGREE | 7-step revision is correct; TickConfig struct is overengineered |
| 06 — Refactoring Plan | PARTIALLY AGREE | Phasing is right; scope is 5x too large for the problems being solved |
| 07 — Naming | AGREE | Naming decisions are solid; Datum is the one weak name |
| 08 — Code Sketches | PARTIALLY AGREE | Conductor port is excellent; BroadcastBus has minor issues |
| 09 — Phase 2+ | AGREE (speculative) | Directionally correct but unfalsifiable; do not design kernel for Phase 2 |
