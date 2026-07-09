# E13 — v2 Spec-Debt (long-horizon)

> Executable backlog epic · verified against HEAD `5852c93c05` · sources: `102-SPEC-DEBT-LEDGER`, `15-V2-COVERAGE`, `85-V2-COVERAGE-KERNEL`, `86-V2-COVERAGE-PLATFORM`, `87-V2-COVERAGE-ECOSYSTEM`, `18-V2-DEPTH-COVERAGE`
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef` · exemplars: `plans/P11-runner-v2-default/tasks.toml`
> **Milestone: M3+ (post-correctness).** This epic MUST NOT block M0–M2. See "Why this must not gate M0–M2" below.
> **Ties to: E09** (Observability). The one load-bearing item here — `Lens` — is the same telemetry-unifier flagged in E09-T09; E13-T01/T02 are the concept-level continuation of that design task.

## Why this epic

The v2 / v2-depth spec corpora name **~129 distinct architectural concepts**. Against code at HEAD:
**~24 Built · ~52 Partial · ~5 Renamed-only · ~55 Zero-code** (102-SPEC-DEBT-LEDGER tally). The Zero-code
band is not scattered — it clusters into whole *aspirational bands* (10-GROUPS, 23-ARENAS, 24-DEFI trading
stack, MPP, package marketplace) that the newest docs describe as if they ship, plus a small number of
genuinely load-bearing kernel abstractions that many other specs lean on.

This epic exists to do one thing the census docs could not: **triage**. It separates the handful of zero-code
concepts that a *coherent v2* actually needs from the large majority that are ecosystem/aspirational and
should be explicitly deferred (or dropped) rather than left to inflate the coverage gap and make every
depth doc read as vaporware. Granular tasks are authored **only** for the load-bearing survivors.

## The one concept that matters

**`Lens`** — the organizing abstraction for telemetry (09/15), the conductor reframe (03-GRAPH), c-factor
learning (10-LEARNING), surface composition (16/20), and marketplace projections (21). It greps to **exactly
0** (`rg 'trait Lens|struct .*Lens|LensScope|CollectorLens|TransformLens|ExportLens' crates/ apps/` →
**0 hits**, re-run this pass). Its nearest reality is a hand-rolled `MetricRegistry`
(`crates/roko-core/src/obs/metrics.rs:263`, 889 LOC, ~12 referencing files). Until a `Lens` trait exists,
**five spec families read as vaporware** and E09's telemetry pipeline has no composable shape to grow into.
Everything else zero-code in v2 is either narrower or genuinely deferrable.

## Concept ledger (zero-code v2 concepts → triage)

Load-bearing (LB) = a v2 execution/telemetry contract depends on it; multiple downstream specs are blocked
until it exists. Aspirational (ASP) = self-contained ecosystem/futures band; **no** other v2 concept depends
on it. All grep counts re-run at HEAD `5852c93c05`.

| Concept | Grep (crates/ + apps/) | LB? | Dependent specs | Recommendation |
|---|---|---|---|---|
| **`Lens` / LensScope / Collector·Transform·ExportLens** | **0 hits** | **LB (top)** | 09-TELEMETRY, 15-TELEMETRY, 03-GRAPH (conductor-as-Lens), 10-LEARNING (c-factor-as-Lens), 16/20-SURFACES, 21-MARKETPLACE | **BUILD** — `trait Lens` + adapt `MetricRegistry` as first lens. E13-T01/T02. Ties to E09-T09. |
| **`Block` type (Cell rename)** | **0 hits** (`(struct\|enum\|trait) Block` → 0) | **LB (naming)** | v2-depth 02-block corpus; two live `Cell` traits (`roko-core/cell.rs:91`, `roko-graph/cell.rs:74`) | **DECIDE (doc)** — resolve Cell↔Block↔block three-way drift; pick canonical, no rename until decided. E13-T03. |
| **4-role Verdict** (reward/CriterionResult/EvidenceKind) | `CriterionResult`/`EvidenceKind` → **0**; reward on wrong type (`Outcome` `verdict.rs:200`) | LB | 02-CELL, routing-reward, hindsight relabel, pre-action veto, reputation | **ADAPT** — migrate `reward` onto `Verdict`; add criteria/evidence. **Owned by E03/E05, not here** (correctness work). Cross-ref only. |
| **`SignalId`/`SignalRef` + `to_pulse` + Pulse lineage** | **0 hits** | LB | 01-SIGNAL, predict-publish-correct, every Lens that reads Pulses | **ADAPT** — prerequisite for spec-shaped Pulses. **Owned by E03** (type consolidation). Cross-ref only. |
| **`Engine` API + `NodeKind` + conditional edges** | `Engine`/`NodeKind` → **0** | LB | 03/04 graph runtime | **Owned by E01** (execution engine). Cross-ref only. |
| Signal→Engram (noun inversion) | `Engram` is struct; `Signal` alias `signal.rs:6` | LB (naming) | whole kernel vocabulary | **Owned by E03.** Renamed-only, not zero-code. Cross-ref only. |
| **10-GROUPS** (Group/GroupIdentity/GroupContextBidder/CoordinationMode/RelayRoom) | **0 hits** | ASP | 10-GROUPS only (933-line spec, self-contained) | **DEFER (Phase 2+)** — tag spec "unfunded/research". No task here. |
| **23-ARENAS** (Arena/Eval/Bounty/ArenaRegistry + 8 arenas + 37 routes) | **1 hit** — a *comment* `bench.rs:82` | ASP | 19-arenas / 23-ARENAS only | **DEFER (Phase 2+)** — nearest reality is bench harness. Tag spec deferred. No task. |
| **24-DEFI trading stack** (ClearingHouse/yield-perps/VenueAdapter/DeFiRiskEngine) | ClearingHouse → **1** *comment* `mirage-rs/rpc.rs:844`; rest **0** | ASP | 24-DEFI only | **DEFER (Phase 2+)** — pure vapor. Tag "research/futures". No task. |
| **MPP** (MppSession/Micropayment streaming) | **0 hits** (one doc-comment field, not a type) | ASP | 18-PAYMENTS only | **DEFER (Phase 2+).** No task. |
| **CamelTag IFC** (propagation/no-laundering) | **0 hits** | ASP-ish | 12/16-SECURITY; `Taint` enum exists but no lattice | **DROP-or-DEFER** — taint-lattice work belongs to E04 (security perimeter); CamelTag naming defers. No task here. |
| **CrossCutFunctor** (endofunctor formalism) | **0 hits** | ASP | 26-CROSS-CUTS only; behavior already inlined in runner | **DROP** — formalism-only; behavior ships as direct calls. Amend spec, no code. No task. |
| Package marketplace / `roko market` (14 subcmds) | **0 hits** | ASP | 21-MARKETPLACE | **DEFER (Phase 2+).** No task. |
| `roko inbox`/`roko autonomy`; 5 surface contracts | **0 hits** | ASP | 20-SURFACES | **DEFER.** No task. |
| WASM/edge; Merkle-CRDT `.roko-brain`; device flow; event indexer; ZK-HDC | **0 hits each** | ASP | 20/22/25 | **DEFER (Phase 2+).** No task. |
| `AutonomyLevel`/`RecursiveSafetyMonitor`/`StructuralChange` (L4) | **0 hits** | LB (last) | 07-LEARNING L4 | **DEFER** — deliberately last; needs Lens + 4-role Verdict + approval CLI first. No task until those land. |

## TRIAGE

### Needed for a coherent v2 (survivors — get tasks or are owned by correctness epics)

- **`Lens` (build here).** The single widest zero-code gap; blocks 5 spec families and gives E09 a shape to
  grow into. Minimum viable: a `trait Lens` + `LensScope` + one adapter that wraps the existing
  `MetricRegistry`/`CFactorSummary`/efficiency stream as the first concrete lenses feeding StateHub.
  **Owned by E13** → E13-T01 (trait + scope), E13-T02 (MetricRegistry adapter).
- **Cell↔Block naming (decide here).** Three-way drift (code=`Cell`×2 traits, v2=`Cell`, v2-depth=`block`) with
  **zero code motion** and two incompatible `Cell` traits. This is a *decision*, not a build — publishing a
  canonical-naming decision doc unblocks every depth doc that references either term and prevents a third
  duplicate. **Owned by E13** → E13-T03 (decision doc, no rename).
- **4-role Verdict, SignalId/Pulse-lineage, Engine/NodeKind, Signal→Engram.** All load-bearing, but all are
  **correctness work owned by E01/E03/E05** — they are M0–M2 items, not long-horizon. E13 only cross-references
  them so the spec-debt picture is complete; it does **not** duplicate or re-own them.

### Aspirational / ecosystem (defer to Phase 2+ — no tasks, spec-tag only)

10-GROUPS · 23-ARENAS · 24-DEFI trading stack · MPP · CamelTag · CrossCutFunctor formalism · package
marketplace / `roko market` · `roko inbox`/`autonomy` + surface contracts · WASM/edge · Merkle-CRDT brain
export · device flow · event indexer · ZK-HDC · L4 structural adaptation.

**These are self-contained: no other v2 concept depends on any of them.** The correct action is a
documentation move — retag each section header "unfunded / research (Phase 2+)" in the spec so they stop
inflating the coverage gap and stop reading as if they ship. That doc-hygiene pass is tracked in
102-SPEC-DEBT-LEDGER's checklist, **not** as engineering tasks here.

### Why this epic must NOT gate M0–M2

1. **Nothing in M0–M2 imports a Lens, a Block, a Group, or an Arena.** The correctness path (make the default
   plan run do real work, consolidate duplicate types, close the security perimeter, make gates adaptive) is
   entirely expressible in the *existing* vocabulary. A `Lens` improves how telemetry composes; it does not
   change whether a task executes.
2. **The load-bearing survivors that ARE urgent are already owned elsewhere** (Verdict→E05, Signal/Pulse→E03,
   Engine→E01). Pulling them into E13 would double-own correctness work and invert priority.
3. **Building Lens/Block on top of an unstable base is waste.** `Lens` should wrap the *final* MetricRegistry
   shape (post-E09) and the Cell-naming decision should follow the *engine* decision (RunnerV2-vs-Graph, E01),
   not precede it. Sequencing E13 after M2 means it adapts settled abstractions instead of chasing moving ones.
4. **The rest is aspirational by construction** — deferring 10-GROUPS/ARENAS/DEFI costs nothing because no
   shipping path touches them. Spending M0–M2 capacity on them would trade correctness for coverage-theatre.

**Milestone placement: M3+.** Do not schedule any E13-Txx until E01 (engine default), E03 (type consolidation),
and E09 (observability plumbing) have landed.

## Task breakdown (E13-Txx) — load-bearing only

| Task | Tier | Summary | Depends |
|---|---|---|---|
| **E13-T01** | design | Define `trait Lens` + `LensScope` in roko-core (scope/collect/transform/export shape) reconciling `docs/v2-depth/09-telemetry` + `15-TELEMETRY`. Deliverable: trait + doc, no consumers yet. | E09-T09 |
| **E13-T02** | medium | Adapt the hand-rolled `MetricRegistry` (`obs/metrics.rs:263`) as the first concrete `CollectorLens`, feeding StateHub — the proof that the trait wraps real telemetry. | E13-T01, E09-T01 |
| **E13-T03** | design | Resolve the Cell↔Block↔`block` naming drift + the two incompatible `Cell` traits (`roko-core/cell.rs:91`, `roko-graph/cell.rs:74`). Deliverable: a canonical-naming decision doc under `tmp/status-quo/references/`. **No rename in this task.** | E01 (engine decision) |

> No further E13 tasks are authored: every other zero-code v2 concept is either owned by a correctness epic
> (E01/E03/E04/E05) or triaged **Aspirational → defer to Phase 2+** above. L4 structural adaptation stays
> unscheduled until E13-T01/T02 + a 4-role Verdict exist.

## First 2 tasks (executable TOML)

```toml
[meta]
plan = "E13-SPEC-DEBT-V2"
total = 3
done = 0
status = "ready"
milestone = "M3+"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────────────
# E13-T01: Define the `Lens` protocol (trait + scope) — the telemetry unifier
# ─────────────────────────────────────────────────────────────────────────────
#
# `rg 'trait Lens|struct .*Lens|LensScope|CollectorLens|TransformLens|ExportLens'
#   crates/ apps/` → 0 hits at HEAD 5852c93c05. This is the single widest
# zero-code v2 gap: 09/15-TELEMETRY, 03-GRAPH (conductor-as-Lens), 10-LEARNING
# (c-factor-as-Lens), 16/20-SURFACES and 21-MARKETPLACE all describe an "X-as-Lens"
# pipeline that has no trait to hang on. This task defines ONLY the trait + scope
# type (the shape 5 specs agree on) — no concrete lenses, no consumers. That is
# E13-T02. Deliverable is a compiling trait + module doc that the depth docs can
# finally point at.
#
# Nearest reality to reconcile against: the hand-rolled MetricRegistry
# (crates/roko-core/src/obs/metrics.rs:263, 889 LOC). The Lens trait must be able
# to *wrap* it in E13-T02 without changing MetricRegistry's public API.
#
[[task]]
id = "E13-T01"
title = "Define trait Lens + LensScope in roko-core (no consumers)"
status = "ready"
tier = "design"
model_hint = "claude-opus-4-1"
max_loc = 90
files = ["crates/roko-core/src/obs/lens.rs", "crates/roko-core/src/obs/mod.rs"]
role = "architect"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-core/src/obs/metrics.rs", lines = "255-320", why = "MetricRegistry (:263) — the hand-rolled reality the Lens trait must be able to wrap in E13-T02 without breaking its API" },
    { path = "crates/roko-core/src/obs/mod.rs", lines = "1-40", why = "obs module root — register the new lens submodule here" },
    { path = "tmp/status-quo/backlog/epics/E09-OBSERVABILITY.md", lines = "1-40", why = "E09-T09 is the plumbing-level continuation; keep the Collector/Transform/Export vocabulary consistent" },
]
symbols = ["MetricRegistry", "LensScope", "Lens"]
anti_patterns = [
    "Do NOT add any concrete lens impls or wire MetricRegistry — that is strictly E13-T02.",
    "Do NOT change MetricRegistry's public surface; the trait must adapt to it, not vice versa.",
    "Do NOT rename Signal/Engram or Cell/Block here — orthogonal (E03 / E13-T03).",
    "Keep it a trait + scope type + doc; resist building a whole pipeline engine.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'trait Lens' crates/roko-core/src/obs/lens.rs && grep -q 'LensScope' crates/roko-core/src/obs/lens.rs"
fail_msg = "obs/lens.rs must define both `trait Lens` and a `LensScope` type"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core 2>&1"
fail_msg = "roko-core must compile with the new Lens trait module"

acceptance = "`rg 'trait Lens' crates/` is no longer 0 hits; roko-core compiles; MetricRegistry API is unchanged; no concrete lens impls added yet."

# ─────────────────────────────────────────────────────────────────────────────
# E13-T02: Adapt MetricRegistry as the first CollectorLens (proof of shape)
# ─────────────────────────────────────────────────────────────────────────────
#
# With `trait Lens` defined (E13-T01), prove it wraps real telemetry by making
# the existing MetricRegistry (obs/metrics.rs:263) the first concrete lens: a
# CollectorLens that projects registry snapshots into the LensScope shape and
# feeds StateHub. This is the "adapt, don't rebuild" pattern — MetricRegistry
# keeps recording exactly as it does today; the lens is a read-side adapter over
# it. Requires E09-T01 (the registry must actually be threaded into RunConfig and
# recording) for the adapter to have live data.
#
[[task]]
id = "E13-T02"
title = "Wrap MetricRegistry as the first CollectorLens feeding StateHub"
status = "ready"
tier = "medium"
model_hint = "claude-opus-4-1"
max_loc = 140
files = ["crates/roko-core/src/obs/lens.rs", "crates/roko-core/src/obs/metrics.rs"]
role = "implementer"
depends_on = ["E13-T01"]

[task.context]
read_files = [
    { path = "crates/roko-core/src/obs/lens.rs", lines = "1-90", why = "The trait + LensScope defined in E13-T01 — implement CollectorLens against it" },
    { path = "crates/roko-core/src/obs/metrics.rs", lines = "255-340", why = "MetricRegistry snapshot/render surface to project from — read-only adapter, do not mutate its API" },
    { path = "crates/roko-runtime/src/state_hub.rs", lines = "50-130", why = "StateHub publish surface — the lens sink target (matches E09 obs-signal map)" },
]
symbols = ["MetricRegistry", "Lens", "LensScope", "CollectorLens", "StateHub"]
anti_patterns = [
    "Do NOT re-implement metric recording — the lens is a read-side adapter over the existing MetricRegistry.",
    "Do NOT change MetricRegistry's recording API or field layout.",
    "Do NOT build TransformLens/ExportLens here — one concrete CollectorLens is the deliverable.",
    "Do NOT couple to serve-only state; the adapter lives in roko-core so both CLI and serve can use it.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'CollectorLens' crates/roko-core/src/obs/lens.rs && grep -q 'impl .*Lens.* for' crates/roko-core/src/obs/lens.rs"
fail_msg = "obs/lens.rs must define a CollectorLens that impls the Lens trait over MetricRegistry"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core 2>&1"
fail_msg = "roko-core must compile with the MetricRegistry-backed CollectorLens"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-core lens 2>&1"
fail_msg = "the CollectorLens must have a passing unit test proving it projects MetricRegistry state into LensScope"

acceptance = "A CollectorLens wraps MetricRegistry and projects its snapshot into LensScope; a unit test proves the projection; MetricRegistry's own API is untouched; the trait is demonstrably not vaporware."
```

## Verification commands (re-run to refresh this epic)

```bash
cd /Users/will/dev/nunchi/roko/roko
# Load-bearing zero-code sentinel (expect 0 until E13-T01):
rg -c 'trait Lens|struct .*Lens|LensScope|CollectorLens|TransformLens|ExportLens' crates/ apps/   # → 0
# Naming-drift sentinels:
rg '(struct|enum|trait) Block\b' crates/ apps/                                                    # → 0 (no Block type)
rg -n 'pub trait Cell' crates/roko-core/src/cell.rs crates/roko-graph/src/cell.rs                 # → 2 incompatible traits
# Aspirational sentinels (expect 0 / comment-only — confirms defer is safe):
rg 'struct Group\b|GroupContextBidder|RelayRoom' crates/ apps/                                    # → 0
rg -n '\bArena\b' crates/ apps/                                                                    # → 1, a comment (bench.rs:82)
rg -n 'ClearingHouse' crates/ apps/                                                                # → 1, a comment (mirage-rs/rpc.rs:844)
rg 'MppSession|Micropayment' crates/ apps/                                                         # → 0
rg -il 'CamelTag' crates/ apps/                                                                    # → 0
rg 'CrossCutFunctor' crates/ apps/                                                                 # → 0
# Nearest-reality anchor:
rg -n 'struct MetricRegistry' crates/roko-core/src/obs/metrics.rs                                  # → :263
```
