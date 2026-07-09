# Refinement Audit Runner — Batch AUD02

Run id: run-20260417-214125
Attempt: 1
Model: gpt-5.4
Reasoning: high

## Shared Context Pack

### 00-AUDIT-RULES

# Audit Application Rules

You are applying refinement-audit critiques to Roko's documentation and tooling.
The audit found that the refinements were "directionally correct but 5-10x overscoped."

## Core Principles

1. **The diagnosis is correct, the prescription was overscoped.** Ship what matters.
2. **Split "exists" from "planned."** Never describe unbuilt features in present tense.
3. **Narrow, don't delete.** Move overscoped content to "future work" sections.
4. **Fix factual errors.** Update LOC counts, route counts, crate counts, status labels.
5. **Reduce jargon inflation.** If a concept has 0 lines of code, it's a research hypothesis.

## Verdicts to Apply

- `keep` → Polish wording. Strengthen evidence. Keep it.
- `narrow` → Reduce scope. Add "aspirational" or "target-state" caveats.
- `defer` → Move to explicit future-work section with a clear label.
- `rewrite` → Reframe per the audit's specific guidance. Don't just edit — rethink.

## Factual Corrections (from codebase reality check)

- Total Rust LOC: 322,088 (not 177K)
- Workspace members: 36 (not 18)
- roko-serve routes: 200+ (not ~85)
- TUI: 58K LOC (wired, not "text-mode only")
- roko-learn: 42 modules, 35,847 LOC
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Pulse/Datum/Demurrage/Worldview/Custody: 0 lines of code each

## 5 Aspirational Concepts with 0 Code

These MUST be labeled as "target-state" or "planned" in docs, never described as existing:
1. Pulse (ephemeral event type)
2. Datum (medium polymorphism enum)
3. Demurrage (knowledge decay economic model)
4. Worldview (heuristic cluster)
5. Custody (chain-of-custody record)

### 01-PRIORITY-QUEUE

# Priority Queue

From the audit master summary — this is the recommended priority order.

## Ship Now (1-2 weeks total)

1. Add HDC fingerprint field to Engram — `roko-core/src/engram.rs` — 1 day
2. Unify event enums into `RokoEvent` — across 4 crates — 1 week
3. Add generic `Bus<E>` trait to roko-core — ~100 lines — 2-3 days
4. Clean up stale "Signal" references — traits.rs, README, kind.rs — 1 hour
5. Fix architecture INDEX status — `docs/00-architecture/INDEX.md` — 30 min

## Ship Soon (next month)

6. CLI parity / muscle memory (REF28)
7. StateHub hardening (REF26)
8. Heuristic calibration struct (REF14)
9. Safety: extend Attestation + expand taint (REF32)
10. Threat model doc (REF32 §13)

## Defer

- Pulse type, Datum enum, Operator generalization
- Demurrage, Plugin SPI tiers 4-5, 3 new kernel crates
- All 5 rewrite candidates, SvelteKit web UI, gRPC
- 12-month roadmap timeline

## Wrong (needs correction in docs)

- Synergy matrix (7/10 primitives don't exist)
- REF32 ignores existing safety system
- Glossary marks EventBus as "retired" (it's the only live transport)
- "Moat" framing (2/10 components exist fully)
- Doc INDEX says serve/TUI "not wired" (both definitively wired)

### 02-DOCS-TREE-MAP

# Docs Tree Map

The canonical documentation lives at `docs/`. Here is the full structure:

```
docs/
├── 00-architecture/        # 33+ files; kernel + trait system + analysis + design principles
├── 01-orchestration/       # Plan DAG, execution, plan runner
├── 02-agents/              # Agent dispatch, backends, sidecar
├── 03-composition/         # Prompts, context assembly, templates, budgets
├── 04-verification/        # Gates, validation, 7-rung pipeline
├── 05-learning/            # Self-learning loops, episodes, playbooks, experiments
├── 06-neuro/               # HDC, knowledge store, distillation, tier progression
├── 07-conductor/           # Event watchers, circuit breaker, diagnosis
├── 08-chain/               # On-chain primitives, ChainBus (Phase 2+)
├── 09-daimon/              # Behavior primitives (Phase 2+)
├── 10-dreams/              # Sleep-time compute, consolidation (Phase 2+)
├── 11-safety/              # Role auth, provenance, attestation, taint
├── 12-interfaces/          # CLI, HTTP API, TUI, Web UI, chat
├── 13-coordination/        # Stigmergy, coordination theory, c-factor
├── 14-identity-economy/    # Identity, economic models
├── 15-code-intelligence/   # Parser, indexing, HDC graphs
├── 16-heartbeat/           # Reactive/reflective loops, timing, CoALA mapping
├── 17-lifecycle/           # Agent lifecycle, shutdown
├── 18-tools/               # Tool system, plugin SPI
├── 19-deployment/          # Containers, orchestration, observability
├── 20-technical-analysis/  # Architecture audit, moat analysis, innovations
├── 21-references/          # Bibliography, research papers
├── INDEX.md                # Top-level index
├── STATUS.md               # Current wiring status
├── BENCHMARKS.md           # Performance data
└── CLI-REFERENCE.md        # Command documentation
```

## Key files you'll likely need to edit

- `docs/00-architecture/INDEX.md` — master architecture index (stale status claims)
- `docs/00-architecture/01-naming-and-glossary.md` — canonical glossary
- `docs/00-architecture/15-crate-map.md` — crate dependency graph
- `docs/00-architecture/31-implementation-readiness-audit.md` — readiness status
- `docs/INDEX.md` — top-level doc index
- `docs/STATUS.md` — current wiring status table

## What the refinements-runner already changed

The first pass (`tmp/refinements-runner/`) landed 35 batches (REF01-REF35) that introduced
new concepts (Pulse, Bus, Datum, demurrage, etc.) into the docs. Many of these concepts
have ZERO lines of code. The audit found that the docs now describe aspirational
architecture as if it exists. Your job is to fix that.

### 03-WORKSPACE-TOPOLOGY

# Workspace Topology

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/`.

## Crate map (36 workspace members)

| Crate | Path | LOC | Status |
|---|---|---|---|
| roko-core | `crates/roko-core/` | kernel | Stable — Engram + 6 traits + config + tools |
| roko-agent | `crates/roko-agent/` | large | 8 LLM backends, pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | medium | Per-agent HTTP sidecar, real LLM dispatch |
| roko-serve | `crates/roko-serve/` | 30K | HTTP control plane, 200+ routes, SSE, WebSocket |
| roko-orchestrator | `crates/roko-orchestrator/` | medium | Plan DAG, parallel executor, merge queue |
| roko-gate | `crates/roko-gate/` | medium | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | medium | Prompt assembly, 9 templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | medium | 10 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | 36K | 42 modules: episodes, playbooks, bandits, routing, experiments |
| roko-cli | `crates/roko-cli/` | 17K+ | CLI binary + ratatui TUI (58K LOC total) |
| roko-fs | `crates/roko-fs/` | small | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | medium | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | medium | ProcessSupervisor, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | small | HDC vectors (10,240-bit), tier routing |
| roko-neuro | `crates/roko-neuro/` | medium | Durable knowledge store, distillation, tiers |
| roko-mcp-code | `crates/roko-mcp-code/` | medium | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | medium | Parser + graph + HDC indexing |
| roko-lang-* | `crates/roko-lang-*/` | small | Language support (rust, typescript, go) |
| roko-dreams | `crates/roko-dreams/` | small | Offline consolidation (Phase 2+) |
| roko-daimon | `crates/roko-daimon/` | small | Behavior primitives (Phase 2+) |
| roko-chain | `crates/roko-chain/` | small | Chain witness primitives (Phase 2+) |

## Key numbers (from codebase audit)

- Total Rust LOC: 322,088
- Workspace members: 36
- Test functions: 3,761
- orchestrate.rs: 17,087 lines
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Signal→Engram rename: 99.6% complete

## Concepts with 0 lines of code

These exist ONLY in docs, not in any crate:
- Pulse, Datum, Demurrage, Worldview, Custody
- roko-bus, roko-hdc (as separate crate), roko-spi
- Bus trait (as a formalized kernel trait)

### 04-DELEGATION-GUIDANCE

# Delegation Guidance

You are explicitly authorized to use multiple subagents for this batch.
Use them where it helps, but keep the immediate blocking work local.

## Required delegation behavior

- Before editing, form a short plan and identify 2-4 concrete subtasks.
- Spawn explorers for targeted codebase/docs reads and workers for bounded edits.
- Give each worker a disjoint write scope — no two workers edit the same file.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally without failing.

## Reading files

Before editing any file, READ IT FIRST. You are working in a git worktree
that contains the full repository. Use your file-reading capabilities to
inspect the current state of any file before modifying it.

## Phase-specific guidance

### Phase 1 (AUD* batches) — Docs only
- Only edit files under `docs/`. Never touch `crates/`, `tmp/`, or `src/`.
- Read the target docs before editing to understand their current state.
- The refinements-runner already made changes — you are refining those changes.

### Phase 2 (PU* batches) — Parity content refresh
- Only edit files under `tmp/docs-parity/NN/`.
- Read the current `docs/` tree first to understand what the audit pass changed.
- Update context-pack/, BATCHES.md, 00-INDEX.md, and all batch detail .md files.
- Update the run-docs-parity.sh script if its batch descriptions or verify
  commands reference stale content.

### Phase 3 (PE* batches) — Code execution
- Edit files under `crates/` to implement what the parity docs describe.
- Read BATCHES.md and 00-INDEX.md from the parity section FIRST.
- Search before writing: `grep -rn 'Name' crates/ --include='*.rs' | grep -v target/`
- Wire existing code — do not reimplement what already exists.
- Run `cargo check` after changes to verify compilation.

## Audit Source Files

These are the critique/triage documents that drive your edits.
Read them carefully — they contain specific verdicts (keep/narrow/defer/rewrite)
and codebase reality checks.

--- BEGIN 01-foundation-audit.md ---

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

--- END 01-foundation-audit.md ---

--- BEGIN 02-foundation-learning.md ---

# Foundation And Learning

## Foundation: what to keep

### Keep the diagnosis, narrow the doctrine

The foundation set correctly identifies the right redesign pressure:
- durable records are not the whole runtime story;
- transport deserves explicit architectural status;
- several downstream concerns become cleaner once the runtime has a first-class
  bus or pulse concept.

That should survive the audit.

What should not survive unchanged is the jump from:
"transport is under-modeled"
to
"therefore every operator, trait, and noun should be redefined around a total
dual-medium worldview."

### Strongest foundational moves

- Treat storage vs transport as a real architectural axis.
- Keep `Pulse` as the likely transport noun if a new noun is needed.
- Use `Bus` as the runtime seam that carries transport explicitly.
- Use StateHub/projection logic as the practical bridge from live events to
  stable UI and operator surfaces.

### Foundational moves that need narrowing

- `Datum` should not become universal just because it is elegant. Use it only
  where medium polymorphism proves its worth.
- Do not rewrite every operator API around dual-medium input in the first pass.
- Treat the seven-step loop as a helpful reference architecture, not a law that
  every crate must immediately mirror.

## Foundation: biggest risks

### 1. Over-generalized operator algebra

The proposed operator generalization is attractive on paper but too broad as a
first migration target. Different operators want different abstractions.

Safer sequence:
- add a transport contract;
- identify which operators genuinely need dual-medium handling;
- only then widen traits or introduce local polymorphic wrappers.

### 2. Kernel rhetoric can outrun kernel need

The redesign does not need a full metaphysical restatement of the kernel before
it has a small number of new runtime contracts that are obviously useful.

Prefer:
- a transport contract;
- a small set of event topics or envelopes;
- projection contracts;
- explicit replay and subscription semantics.

Be careful with:
- total renaming passes;
- universal operator algebra;
- new foundational nouns that do not buy a concrete simplification.

### 3. Glossary can harden hypotheses too early

The glossary is useful, but it currently hardens many proposed nouns as if they
were settled redesign-level concepts. It should distinguish:
- current canonical terms;
- target-state terms likely to become canonical;
- exploratory or historical terms.

## Learning: what to keep

### Evented calibration is the best core idea

The strongest learning idea is not grand cybernetics. It is the practical move
toward shared calibration loops:
- expectation;
- outcome;
- discrepancy;
- adjustment.

This should remain central.

### Heuristics as a middle layer are worth building

A typed, inspectable heuristic layer between raw episodes and distilled
playbooks is one of the best ideas across the entire set.

That means:
- typed heuristic objects;
- visible provenance;
- challenge and contradiction records;
- calibration history;
- promotion/demotion rules tied to runtime evidence.

### HDC has real value in a narrower role

HDC is useful as:
- cheap similarity search;
- clustering aid;
- retrieval acceleration;
- lightweight representation for durable knowledge indexing.

It should not become:
- universal semantic truth geometry;
- reliable consensus detector;
- the hidden explanation for all future memory or reasoning behavior.

## Learning: what needs narrowing or deferral

### 1. Active inference claims are too large

Better framing:
- "calibration-driven control";
- "evented prediction/outcome scaffolding";
- "routing and prompt feedback loops first."

### 2. Worldview and falsifier rhetoric exceeds current mechanism

The conceptual story is interesting, but it is better as a later layer on top
of heuristics, contradictions, and typed claims. As a redesign target, it is
too abstract too early.

### 3. Demurrage is ahead of the memory model

Demurrage should be treated as:
- a hypothesis for future memory shaping,
not
- the governing explanation of memory or forgetting.

### 4. c-factor is not yet a stable core metric

Until it has one clear interpretation and one trusted measurement path, it
should be described as a coordination-health experiment, not a mature
collective-intelligence scalar.

### 5. Research-to-runtime is still a narrative layer

The instinct is good: make external knowledge auditable and contestable. The
problem is scope. Build this in ascending order:
- typed claims;
- provenance and source quality;
- contradiction and replication;
- only later, richer research-economy semantics.

## Recommended rewrite principles for this area

1. Keep the transport diagnosis and the need for cleaner runtime seams.
2. Rewrite foundation docs as a tighter target-state architecture, not a full
   kernel ideology.
3. Treat `Bus` as the main kernel addition and `Datum` as optional.
4. Reframe learning around calibration and typed heuristics, not sweeping
   cybernetic claims.
5. Reduce the number of places where HDC, demurrage, c-factor, and claims are
   described as foundational laws.
6. Move the more speculative parts into clearly marked research or future-work
   sections.

--- END 02-foundation-learning.md ---

## Master Summary (reference)

# Refinements Audit — Master Summary

> **Date**: 2026-04-17 | **Auditor**: Claude Opus 4.6 (7 parallel agents)
> **Scope**: All 35 refinement docs + runner infrastructure + landed doc updates + codebase reality check
> **Output**: 7 detailed audits in this directory (01-foundation through 07-doc-quality)

---

## Executive Verdict

**The diagnosis is correct. The prescription is 5-10x overscoped.**

The refinements correctly identify real problems in the codebase (event enum proliferation, a conductor/learn layer violation, stale "Signal" naming, Policy signature mismatch). But they propose a 6-12 month, 5-7 engineer refactoring program for a single-developer project, introducing ~15 types that don't exist yet (Pulse, Datum, Bus trait, TopicFilter, Demurrage, Custody, Worldview, Claim, Paper, TypedContext, etc.) to solve problems that could be fixed in ~1-2 weeks with targeted changes.

---

## The 5 Things to Ship Now

These emerged consistently across all 7 audit workstreams as high-value, low-risk:

| # | What | Where | Effort | Why |
|---|---|---|---|---|
| 1 | **Add HDC fingerprint field to Engram** | `roko-core/src/engram.rs` | 1 day | HdcVector exists (10,240-bit, tested). Episode fingerprinting already works. This is the single highest-value bridge between the learning and memory layers. |
| 2 | **Unify event enums into `RokoEvent`** | Across 4 crates | 1 week | Four incompatible event enums (2x `AgentEvent`, `RokoEvent`, `ServerEvent`) is the real problem. Unify them. |
| 3 | **Add generic `Bus<E>` trait to roko-core** | `roko-core/src/traits.rs` | 2-3 days | ~100 lines. Keep it generic (not Pulse-specific). Solves the layer violation. |
| 4 | **Clean up stale "Signal" references** | traits.rs, README, kind.rs, CLAUDE.md | 1 hour | 40+ stale occurrences across docs and code comments. |
| 5 | **Fix architecture INDEX status** | `docs/00-architecture/INDEX.md` | 30 min | Says "roko-serve: HTTP API not wired" and "TUI: Text-mode dashboard only" — both factually wrong per CLAUDE.md and code (30K LOC serve, 58K LOC TUI). |

---

## The 5 Things to Ship Soon (next month)

| # | What | Source | Effort |
|---|---|---|---|
| 6 | **CLI parity / muscle memory (REF28)** | UX audit | 1-2 weeks |
| 7 | **StateHub hardening (REF26)** | UX audit | 1 week |
| 8 | **Heuristic calibration struct** | Learning audit (REF14) | 3-5 days |
| 9 | **Safety: extend Attestation + expand taint** | Integrator audit (REF32) | 1 week |
| 10 | **Threat model doc** | Integrator audit (REF32 §13) | 2 days |

---

## The 10 Things to Defer

| What | Why defer |
|---|---|
| **Pulse type** (REF02) | Unified `RokoEvent` enum solves the same problem more simply |
| **Datum enum** (REF04) | Premature abstraction; doubles every trait's surface area |
| **Operator generalization** (REF04) | Only Policy actually needs a signature change |
| **Demurrage** (REF12) | Add `last_used + access_count` to Decay first; skip the full economic model |
| **Plugin SPI tiers 4-5** (REF17) | Zero plugin authors exist. WASM host is premature |
| **3 new kernel crates** (REF20) | roko-bus justified, roko-hdc unnecessary (345 LOC), roko-spi premature |
| **All 5 rewrite candidates** (REF21) | Existing code works. Build incrementally |
| **SvelteKit web UI** (REF29) | Zero frontend code exists. Build when someone asks |
| **gRPC wire protocol** (REF27) | No tonic dependency. WebSocket + SSE already work |
| **12-month roadmap timeline** (REF35) | Calibrated for 5-7 engineers, not 1 developer + AI |

---

## The 5 Things That Are Wrong

| What | Issue | Source |
|---|---|---|
| **Synergy matrix** (REF31) | 7 of 10 "load-bearing primitives" don't exist in code. Matrix is aspirational fiction. | Integrator audit |
| **REF32 ignores existing safety system** | The AgentContract/AgentWarrant/Capability system already exists and works. REF32 proposes replacing it without acknowledging it. | Integrator audit |
| **Glossary marks EventBus as "retired"** | `EventBus<E>` is the only live transport code. No Bus trait or Pulse exists. | Integrator audit |
| **"Moat" framing** (REF18) | Of 10 claimed moat components, 2 exist fully, 2 partially, 6 not at all. The moat is aspirational. | Moat audit |
| **Doc INDEX says serve/TUI "not wired"** | serve has 200+ routes (30K LOC), TUI has 58K LOC with WebSocket. Both are definitively wired. | Doc quality + reality check |

---

## Codebase Reality (Key Numbers)

From the reality-check audit:

| What | Reality |
|---|---|
| Total Rust LOC | 322,088 (not 177K as CLAUDE.md says) |
| Workspace members | 36 (not 18) |
| Test functions | 3,761 |
| orchestrate.rs | 17,087 lines (the integration hairball) |
| roko-serve routes | 200+ (not ~85) |
| TUI code | 58K LOC |
| roko-learn modules | 42 modules, 35,847 LOC |
| Signal→Engram rename | 99.6% complete (4 real stragglers) |
| Event bus event types | Exactly 2 (PlanRevision, PrdPublished) |
| Demurrage in code | 0 lines |
| Pulse in code | 0 lines |
| Worldview in code | 0 lines |

---

## Doc Quality Assessment

Overall: **3.8 / 5**

**Good**: No copy-paste artifacts. Glossary is excellent. Synergy map and safety spine read as unified docs. Cross-references resolve.

**Issues**:
1. "Signal" still used in ~40 places across 8+ pre-existing docs
2. Target crates (roko-bus, roko-hdc, roko-spi) described in present tense as if they exist
3. Architecture INDEX has stale status information contradicting CLAUDE.md

---

## Per-Arc Summary

### Foundation (01-09): PARTIALLY AGREE
The diagnosis is correct. The prescription (Pulse, Datum, generalized operators, 7-step TickConfig) is overcomplicated. Fix: unify events, add generic Bus trait, update docs. ~1 week instead of 6-7 weeks.

### Learning (10-16): SIMPLIFY
The docs undercount what already exists. roko-learn has 42 modules and 36K LOC. HDC fingerprint field on Engram is the highest-value change. Demurrage/worldviews/replication-ledger are premature.

### Moat (17-21): DEFER/SKEPTICAL
Zero plugin authors, zero external users. The moat is aspirational. Plugin tier 3 (tool manifests) is useful later. Everything else waits.

### UX (22-30): Pick 3 of 9
Ship REF28 (CLI parity), REF26 (StateHub), and the chat/init subset of REF23. Defer the four-layer SDK, six domain profiles, SvelteKit UI, gRPC, and rich UX primitives.

### Integrators (31-35): Integrate code, not plans
The synergy matrix, glossary, and roadmap are plans connecting to plans. Ship: threat model, glossary (split into "exists" vs "planned"), dependency ordering. Reject: quarterly timeline, synergy matrix of unbuilt features.

---

## Recommended Priority Queue

For a single developer + AI agents:

1. **Close the self-hosting loop** (CLAUDE.md items 10-11: auto plan generation + feedback loop)
2. Ship the 5 "now" items above
3. Ship the 5 "soon" items above
4. Address ux-followup P0 items (67 items in `tmp/ux-followup/`)
5. Decompose `orchestrate.rs` (17K lines is the real tech debt)
6. Everything else goes into "when the system needs it"

---

## Audit Files

| File | What |
|---|---|
| `01-foundation-audit.md` | REF01-09 vs codebase (28K chars) |
| `02-learning-audit.md` | REF10-16 vs codebase (30K chars) |
| `03-moat-audit.md` | REF17-21 vs codebase (25K chars) |
| `04-ux-audit.md` | REF22-30 vs codebase (25K chars) |
| `05-integrator-audit.md` | REF31-35 vs codebase (23K chars) |
| `06-codebase-reality-check.md` | 10 factual claims verified (27K chars) |
| `07-doc-quality-audit.md` | Landed doc updates quality (18K chars) |

## Refinement Matrix (per-REF verdicts)

# Refinement Matrix

Legend:
- `keep`
- `narrow`
- `defer`
- `rewrite`

| Ref | Title | Verdict | Audit note |
|---|---|---|---|
| REF01 | critique one noun | `keep` | The diagnosis is real: transport is under-modeled and the kernel story is too storage-centric. |
| REF02 | Engram vs Pulse | `keep` | `Pulse` is a good transport noun if used to clarify the redesign rather than force a total renaming campaign. |
| REF03 | Bus as first class | `keep` | This is the strongest foundational follow-up: unify and formalize transport. |
| REF04 | operators generalized | `narrow` | Good local idea, bad universal law. Medium polymorphism should be proven operator by operator. |
| REF05 | loop retold | `keep` | Useful as a reference architecture for the redesign, but should guide migration rather than dictate every interface immediately. |
| REF06 | refactoring plan | `keep` | A phased migration plan is appropriate; keep it honest and code-first. |
| REF07 | naming | `narrow` | Good cleanup instinct, but not every proposed term should become top-level canon immediately. |
| REF08 | code sketches | `narrow` | Helpful as exploratory sketches; should not be confused with settled API design. |
| REF09 | phase-2 implications | `narrow` | Good future map, but it should stay downstream of core runtime wins instead of shaping the first redesign pass. |
| REF10 | self-learning loops | `keep` | Strong direction if centered on calibration, contradiction, and adaptation rather than runtime-wide active-inference doctrine. |
| REF11 | HDC substrate | `narrow` | Keep HDC for retrieval/clustering; defer broader semantic-consensus rhetoric. |
| REF12 | knowledge demurrage | `defer` | Interesting hypothesis, but too early to present as the governing memory model. |
| REF13 | c-factor | `defer` | Worth exploring as coordination health, not yet worthy of strong canonical treatment. |
| REF14 | worldview validation | `narrow` | Keep typed heuristics and contradiction tracking; defer full worldview/dissonance stack. |
| REF15 | exponential scaling | `defer` | Too much product-theory confidence for the current maturity level. |
| REF16 | research-to-runtime | `narrow` | Claim registry and provenance-backed defaults are promising; the full paper economy is premature. |
| REF17 | plugin extension architecture | `keep` | Tiered extensibility is the right platform direction if it stays local-first and resists premature ecosystem ambition. |
| REF18 | competitive moat | `defer` | Too much architecture-theater and future-ecosystem assumption. |
| REF19 | net-new innovations | `rewrite` | The catalog format oversells speculative pieces; convert to research hypotheses or remove. |
| REF20 | modularity composability | `keep` | Crate-boundary cleanup and clearer seams are real needs. |
| REF21 | from-scratch redesigns | `narrow` | Useful as a pressure test and cleanup lens, but dangerous as the default implementation mindset. |
| REF22 | developer UX rust | `keep` | Strong redesign target if the SDK is kept crisp and optimized for time-to-first-agent rather than feature taxonomy. |
| REF23 | user UX running agents | `keep` | Strong target-state direction if parity follows a real shared session model instead of surface symmetry for its own sake. |
| REF24 | deployment UX | `keep` | Strong operator-centered direction; needs stricter sequencing and fewer assumptions bundled into the first wave. |
| REF25 | domain-specific agents | `keep` | Domain profiles are a strong packaging abstraction as long as bundles stay ahead of universal type formalism. |
| REF26 | StateHub rearchitecture | `keep` | One of the best proposals. Evolve the existing dashboard hub into real projections. |
| REF27 | realtime event surface | `keep` | Unification is the right target, but the contract should stay small: events, replay, filters, subscriptions. |
| REF28 | CLI parity familiar workflows | `keep` | Familiar-first is right if parity is earned from shared workflow semantics rather than copied command names. |
| REF29 | web UI architecture | `keep` | A web surface is a good redesign goal if it starts as an ops console and grows from projection contracts. |
| REF30 | rich UX primitives | `narrow` | Some primitives are valuable, but only when supported by real shared state and telemetry contracts. |
| REF31 | synergy integration map | `defer` | Fine as internal coherence tooling; too grand as canonical architecture backmatter. |
| REF32 | safety sandbox provenance | `keep` | Strong direction if safety remains a compact enforceable spine rather than an all-at-once governance superstructure. |
| REF33 | observability telemetry | `keep` | Strong direction if the signal set stays operator-useful and avoids speculative overmodeling. |
| REF34 | glossary | `rewrite` | Keep one glossary, but split current canon from target-state proposals. |
| REF35 | consolidated roadmap | `rewrite` | Keep sequencing discipline, but narrow the number of simultaneous deep bets and remove unearned quarter-level certainty. |

## Aggregated view

### Clear keeps

- REF01
- REF02
- REF03
- REF05
- REF06
- REF10
- REF17
- REF20
- REF22
- REF23
- REF24
- REF25
- REF26
- REF27
- REF28
- REF29
- REF32
- REF33

### Strong, but should be narrowed

- REF04
- REF07
- REF08
- REF09
- REF11
- REF14
- REF16
- REF21
- REF30

### Better deferred

- REF12
- REF13
- REF15
- REF18
- REF31

### Need substantive rewrite

- REF19
- REF34
- REF35

## Practical consequence

The refinement set should not be treated as a monolithic "land it all" bundle.
The right next pass is:

1. Preserve the `keep` items.
2. Rewrite the `narrow` items around smaller scope and less doctrinal force.
3. Move the `defer` items into explicit future-work or research-hypothesis sections.
4. Rebuild the `rewrite` items so they stop acting as authority multipliers for
   architecture that is still too speculative or too overloaded.

# Batch AUD02: Narrow foundation concepts (REF01-09) to target-state in architecture docs

**Audit refs**: 01-foundation-audit.md, 02-foundation-learning.md, 05-refinement-matrix.md
(REF01-09 rows). Applies the audit's "keep diagnosis, narrow prescription" verdict to
`docs/00-architecture/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/01-foundation-audit.md` (full file -- verdict per REF)
- `tmp/refinements-audit/02-foundation-learning.md` (foundation section: what to keep, what to narrow)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF01-09 rows)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 10 Things to Defer" section)
- `docs/00-architecture/02b-pulse-ephemeral-event.md`
- `docs/00-architecture/07b-bus-transport-fabric.md`
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md`
- `docs/00-architecture/09-universal-cognitive-loop.md`
- `docs/00-architecture/12-five-layer-taxonomy.md`
- `docs/00-architecture/15-crate-map.md`
- `docs/00-architecture/01-naming-and-glossary.md` (first 100 lines for orientation)

## Task

The refinements-runner wrote Pulse, Datum, Bus-as-trait, generalized operators,
and the seven-step loop into the architecture docs as if they were current
architecture. The audit found that Pulse has 0 lines of code, Datum has 0 lines,
Bus exists only as a concrete `EventBus<E>` struct (not a kernel trait), and
the operator generalization is premature. Mark these concepts as
**target-state** rather than **current architecture**.

## Current state (evidence)

The audit found these specific problems in the architecture docs:

1. **`02b-pulse-ephemeral-event.md`** describes Pulse as a current kernel type.
   Reality: no `Pulse` struct exists anywhere in the codebase. Zero lines.

2. **`07b-bus-transport-fabric.md`** describes `trait Bus` as a kernel trait.
   Reality: `EventBus<E>` exists as a concrete struct in `roko-runtime/src/event_bus.rs`
   with 2 event types (PlanRevision, PrdPublished). It is NOT a kernel trait.

3. **`08-scorer-gate-router-composer-policy.md`** describes `Datum` as the
   universal input type for operators. Reality: no `Datum` type exists. Operators
   take `&[Engram]` today.

4. **`12-five-layer-taxonomy.md`** line 221 says `roko-core, roko-bus, roko-hdc,
   and roko-spi are the only kernel-tier crates` (present tense). Reality:
   `roko-bus`, `roko-hdc`, and `roko-spi` do not exist as crates.

5. **`15-crate-map.md`** describes target crates (`roko-bus`, `roko-hdc`,
   `roko-spi`, `roko-defaults`, `roko-tools`, `roko-compose-core`,
   `roko-templates`). The crate-map doc itself is honest about the gap, but
   other docs reference it without qualification.

6. **`09-universal-cognitive-loop.md`** describes the seven-step loop with
   co-equal PERSIST/BROADCAST as current architecture. Reality: the loop exists
   as `loop_tick` in `roko-core` but BROADCAST (Bus-mediated) is not wired.

## Implementation

### 1. Add target-state markers to Pulse doc

In `docs/00-architecture/02b-pulse-ephemeral-event.md`:
- Add a prominent callout near the top (after the abstract) stating:
  `> **Implementation status**: Target-state design. No `Pulse` type exists in
  > the codebase yet. The current transport mechanism is `EventBus<RokoEvent>`
  > in `roko-runtime/src/event_bus.rs` with 2 event types.`
- Do NOT delete the Pulse design content. It is useful as a target spec.

### 2. Add target-state markers to Bus doc

In `docs/00-architecture/07b-bus-transport-fabric.md`:
- Add a prominent callout near the top:
  `> **Implementation status**: Target-state design. The current transport is
  > `EventBus<E>` (a concrete generic struct in roko-runtime, not a kernel
  > trait). It has 2 event types: PlanRevision and PrdPublished. The trait-based
  > Bus described here is the target architecture.`

### 3. Mark Datum as target-state in operator doc

In `docs/00-architecture/08-scorer-gate-router-composer-policy.md`:
- Where `Datum` is introduced as the operator input type, add a note:
  `> **Note**: `Datum` is a target-state abstraction. Current operators accept
  > `&[Engram]` directly. The medium-polymorphic `Datum` wrapper is planned
  > but not yet implemented.`

### 4. Fix five-layer taxonomy crate claims

In `docs/00-architecture/12-five-layer-taxonomy.md`:
- Change "roko-core, roko-bus, roko-hdc, and roko-spi **are** the only
  kernel-tier crates" to "roko-core is the current kernel-tier crate;
  roko-bus, roko-hdc, and roko-spi are **target** kernel crates proposed by
  REF20"
- Apply the same treatment to any other present-tense claims about crates that
  do not exist

### 5. Verify crate-map qualification

In `docs/00-architecture/15-crate-map.md`:
- Check that target crates are marked as "Target" or "Proposed" consistently
- If any target crates are described in present tense ("roko-bus provides..."),
  change to future tense or add a "(target)" qualifier

### 6. Mark BROADCAST step as target-state in loop doc

In `docs/00-architecture/09-universal-cognitive-loop.md`:
- Where the BROADCAST step is described as co-equal with PERSIST, add a note:
  `> **Implementation status**: PERSIST is wired (FileSubstrate). BROADCAST
  > (Bus-mediated event emission) exists only for PlanRevision and
  > PrdPublished events. Full Bus-mediated broadcast is target-state.`

## Write scope

- `docs/00-architecture/02b-pulse-ephemeral-event.md`
- `docs/00-architecture/07b-bus-transport-fabric.md`
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md`
- `docs/00-architecture/09-universal-cognitive-loop.md`
- `docs/00-architecture/12-five-layer-taxonomy.md`
- `docs/00-architecture/15-crate-map.md`

## Rules

1. **Mark, do not delete.** The target-state designs are valuable specs. Add
   implementation-status callouts; do not remove design content.
2. **Use consistent callout format.** Every target-state marker should be a
   blockquote starting with `> **Implementation status**:` followed by what
   exists today and what is target-state.
3. **Distinguish three levels**: "Shipping" (wired, tested, CLI-accessible),
   "Built" (code exists, not fully wired), "Target-state" (described in docs,
   no code).
4. **Do not touch the glossary.** Glossary fixes are AUD06's scope.
5. **Do not fix Signal->Engram references.** That is AUD07's scope.
6. **Do not change the architecture narrative.** The two-medium, two-fabric
   story is the intended target architecture. Just qualify what is current vs.
   what is planned.

## Done when

- Every architecture doc that describes Pulse, Datum, Bus-as-trait, or target
  crates has a visible implementation-status callout
- `12-five-layer-taxonomy.md` no longer claims target crates exist in present
  tense
- No architecture doc was deleted or had its design content removed
- The distinction between "current" and "target-state" is clear to a reader
  who opens any single doc in the set
- Final message lists every file edited and the specific callouts added
