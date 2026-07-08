# Roko Architectural Refinements

> **Created**: 2026-04-16
> **Status**: Proposal — docs-level thinking, with concrete code sketches. No production changes landed yet.
> **Scope**: Rework Roko's foundational story from "one noun, six verbs" to
> "two mediums, two fabrics, six operators, five layers, three speeds, three cross-cuts"
> — and then chase every consequence of that reframing through learning, moat,
> modularity, deployment, developer UX, user UX, realtime surfaces, and product polish.

## One paragraph for someone who has never seen Roko

Roko is a Rust toolkit for building agents that build themselves. Today it reads PRDs,
plans, executes tasks via sub-agents, validates with gates, and persists episodes — the
universal loop works end-to-end. Its current foundational story in
`crates/roko-core/src/lib.rs` is "one noun (Engram) and six verb traits" (Substrate,
Scorer, Gate, Router, Composer, Policy). That story is catchy and has carried the
project from ~0 to ~177K LOC — but the runtime grew a second medium (live events
on an in-memory bus) and a second fabric (the event bus itself) that the framing
never acknowledged, with visible consequences (a confirmed layer violation, trait
signatures that stretch to fit, TUI polling where the Bus should stream). This folder
is the honest reframing, plus the learning, scaling, moat, modularity, and UX work
the new frame unlocks.

## The 35 docs in one sentence each

See the **Files** tables below for full titles. In reading order — if you read nothing
else, skim these sentences:

1. **01** — three receipts from the code why "one noun" is reductive.
2. **02** — introduce Pulse (ephemeral) as Engram's sibling, with a graduation law.
3. **03** — promote Bus to a kernel trait next to Substrate.
4. **04** — generalize the six operators to work over either medium.
5. **05** — the universal loop becomes 7 steps; cross-cuts stop pretending to be step 9.
6. **06** — three-phase refactor plan (docs, kernel, subsystems).
7. **07** — name the new type `Pulse`, not `Signal` or `Event`.
8. **08** — actual Rust sketches for Bus, Pulse, Datum, a conductor port.
9. **09** — chain, dreams, mesh, and stigmergy all land cleanly on the new kernel.
10. **10** — every operator becomes a predictor; active inference becomes literal.
11. **11** — 10,240-bit HDC fingerprint on every Engram; similarity is O(1).
12. **12** — demurrage replaces decay; memory becomes an attention economy.
13. **13** — Woolley's c-factor as a continuously measured runtime signal.
14. **14** — heuristics with explicit falsifiers and calibration over lived experience.
15. **15** — seven compounding loops that make every week of usage cheaper than the last.
16. **16** — papers become Engrams; claims become testable hypotheses; replication ledger.
17. **17** — five-tier plugin SPI from pure-data prompts up to WASM sandboxes.
18. **18** — five structural moats, and what kills each one.
19. **19** — flat catalog of what's net-new vs prior art; the pitch deck.
20. **20** — cleaner dep graph; three new kernel crates (`roko-bus`, `roko-hdc`, `roko-spi`).
21. **21** — five candidate rewrites, sequenced into two months of focused work.
22. **22** — the four-layer Rust SDK: one-liner, builder, trait-impl, runtime-impl.
23. **23** — one verb-set across CLI, TUI, chat, web; interactive first-run; undo first-class.
24. **24** — five deployment shapes from laptop to edge, same binary, same config model.
25. **25** — six domain profiles (coding, research, chain, data, ops, writing) + TypedContext.
26. **26** — promote StateHub from TUI helper to kernel projection layer.
27. **27** — single realtime wire protocol over WebSocket / SSE / gRPC; first-party clients.
28. **28** — adopt Claude-Code / Aider muscle memory; diff-first; per-hunk control.
29. **29** — five-page first-party web UI on SvelteKit + StateHub.
30. **30** — ten rich UX primitives from reasoning streams to replay scrubbers.
31. **31** — **synergy map** — how Engram + Pulse + Bus + HDC + Demurrage + Heuristics + c-factor + Replication interlock into one coherent system.
32. **32** — **safety, sandboxing, provenance** — the defensive spine that runs orthogonally across every layer.
33. **33** — **observability & telemetry** — logs, metrics, traces, events, replay; what ships, what's pluggable.
34. **34** — **glossary** — every new term, every reclaimed term, in one place.
35. **35** — **consolidated roadmap** — the six-to-twelve-month sequencing across every previous doc, with dependencies.

## Why this folder exists

Roko's current foundational story ("one noun — Engram — plus six verb traits")
comes from `crates/roko-core/src/lib.rs` and
`docs/00-architecture/06-synapse-traits.md`. It is catchy, but it is
selling the system short in three ways:

1. There are already two distinct data shapes in the codebase. **Engram** is a
   durable, BLAKE3-addressed record with lineage, decay, and provenance.
   `roko-runtime::event_bus::Envelope<E>` is a typed, sequence-numbered,
   ring-buffered in-flight message. The docs pretend the second doesn't exist.
2. The six traits are advertised as operating on Engrams, but Policy already
   operates over streams, and several subsystems reach across layers to
   subscribe to live events because the bus isn't part of the architectural
   lexicon. `docs/00-architecture/23-architectural-analysis-improvements.md`
   flags one confirmed layer violation caused by exactly this gap.
3. The name "Signal" was recently freed when `Signal` → `Engram` landed in
   code (877:5 in crates/). The README, CLAUDE.md, docs/INDEX.md, and the
   naming-glossary still carry the old equivalence and the
   "code uses Signal / docs use Engram" disclaimer. All stale.

The proposal in this folder is to reorganize the foundational chapter around
**two mediums, two fabrics, and six operators**, with layers, tempos, and
cross-cuts stacked on top. This makes the event bus first-class, keeps every
name that is already load-bearing, and delivers the three things the user
asked for: agent runtime, harness, and scaffold *down to the lowest level*,
generalized enough to build and compose arbitrary agents in Rust.

## The narrative arc

The 35 docs cluster into five arcs that build on each other:

1. **Foundation reframing (01–09)** — diagnose "one noun, six verbs," introduce
   Pulse, promote Bus, generalize operators, retell the loop, plan the refactor,
   name the types, sketch the code, show the Phase-2 payoff.
2. **Learning and intelligence (10–16)** — once Bus exists, every operator is a
   predictor (10), HDC makes memory holographic (11), demurrage gives memory an
   economy (12), c-factor measures collective intelligence (13), heuristics
   encode lived-experience priors (14), compounding loops produce superlinear
   returns (15), and the research literature becomes a live input (16).
3. **Moat and modularity (17–21)** — a five-tier plugin SPI (17), five
   structural components of the competitive moat (18), a catalog of what's
   net-new (19), a target dependency graph with three new kernel crates (20),
   and five from-scratch rewrite candidates (21).
4. **Developer and operator UX (22–27)** — four-layer Rust SDK for agent
   authors (22), four-surface user UX with one verb set (23), five deployment
   shapes (24), six domain profiles (25), StateHub as kernel projection layer
   (26), unified realtime event surface (27).
5. **Product polish (28–30) and consolidation (31–35)** — CLI parity with
   familiar tools (28), five-page first-party web UI (29), ten rich UX
   primitives (30), and the four integrators: synergy map (31), safety spine
   (32), observability (33), glossary (34), roadmap (35).

The arc is not linear; later docs inform earlier ones. Demurrage (12) gives
the substrate (03) its attention rules; heuristics (14) give research (16) its
testable objects; StateHub (26) is the UX surface for the c-factor (13)
measurements; and so on. The synergy doc (31) makes the cross-weave explicit.

## Files

### Foundation: two mediums, operators, refactor path (01–09)

| # | File | What |
|---|---|---|
| 00 | [00-INDEX.md](00-INDEX.md) | This file |
| 01 | [01-critique-one-noun.md](01-critique-one-noun.md) | Why "one noun, six verbs" is reductive — receipts from code, docs, and event-bus usage |
| 02 | [02-engram-vs-pulse.md](02-engram-vs-pulse.md) | The two-medium split: durable Engram vs ephemeral Pulse, conversion law |
| 03 | [03-bus-as-first-class.md](03-bus-as-first-class.md) | Promoting `EventBus` to a kernel trait alongside `Substrate` |
| 04 | [04-operators-generalized.md](04-operators-generalized.md) | The six operators (Scorer, Gate, Router, Composer, Policy, plus Substrate/Bus) redrawn over either medium |
| 05 | [05-loop-retold.md](05-loop-retold.md) | The universal cognitive loop with three sense sources and a broadcast step |
| 06 | [06-refactoring-plan.md](06-refactoring-plan.md) | Concrete docs-level and code-level refactoring phases with effort estimates |
| 07 | [07-naming.md](07-naming.md) | Naming decisions: Pulse vs Event vs reclaiming Signal, trade-offs |
| 08 | [08-code-sketches.md](08-code-sketches.md) | Actual Rust: `Bus` trait, `Pulse` type, `graduate_to_engram`, conductor port |
| 09 | [09-phase-2-implications.md](09-phase-2-implications.md) | How the two-fabric model unlocks chain, dreams, coordination, stigmergy |

### Learning, intelligence, and moat (10–21)

| # | File | What |
|---|---|---|
| 10 | [10-self-learning-cybernetic-loops.md](10-self-learning-cybernetic-loops.md) | Every operator as a predictor; active inference as a literal implementation on Bus + Pulse |
| 11 | [11-hyperdimensional-substrate.md](11-hyperdimensional-substrate.md) | 10,240-bit HDC fingerprint on every Engram — similarity, consensus, analogy as O(1) vector ops |
| 12 | [12-knowledge-demurrage.md](12-knowledge-demurrage.md) | Economic memory: balance, holding cost, reinforcement-by-kind; self-trimming playbooks |
| 13 | [13-collective-intelligence-c-factor.md](13-collective-intelligence-c-factor.md) | Quantifying Woolley's c-factor from Bus statistics and optimizing it via Policy |
| 14 | [14-worldview-validation.md](14-worldview-validation.md) | Heuristics with falsifiers, worldviews as co-citation clusters, lived-experience calibration |
| 15 | [15-exponential-scaling.md](15-exponential-scaling.md) | Seven compounding loops, superlinear returns, "every week your Roko gets better on your codebase" |
| 16 | [16-research-to-runtime.md](16-research-to-runtime.md) | Papers as Engrams, Claims as testable hypotheses, Replication Ledger — living research |
| 17 | [17-plugin-extension-architecture.md](17-plugin-extension-architecture.md) | Five-tier SPI (prompts, profiles, manifests, native, WASM) with matched sandboxes |
| 18 | [18-competitive-moat.md](18-competitive-moat.md) | Five structural components: architectural coherence, heuristic commons, ecosystem, replication ledger, Rust |
| 19 | [19-net-new-innovations.md](19-net-new-innovations.md) | Flat catalog of primitives and patterns with no known prior art — the pitch deck |
| 20 | [20-modularity-composability.md](20-modularity-composability.md) | Proposed dep graph, three new kernel crates, rules that keep modularity honest |
| 21 | [21-from-scratch-redesigns.md](21-from-scratch-redesigns.md) | Five rewrite candidates with cost/unlock analysis and a 2-month sequencing |

### UX: developers, users, deployment, surfaces (22–30)

| # | File | What |
|---|---|---|
| 22 | [22-developer-ux-rust.md](22-developer-ux-rust.md) | Four-layer Rust SDK (one-liner / builder / trait / runtime), error vocabulary, `cargo roko`, macros, docs discipline |
| 23 | [23-user-ux-running-agents.md](23-user-ux-running-agents.md) | One verb-set, four renderings; first-run guided setup; TUI goes interactive; chat streams and inline artifacts |
| 24 | [24-deployment-ux.md](24-deployment-ux.md) | Five shapes (laptop / single-server / container / clustered / edge) from one binary; profiles, secrets, state portability, observability |
| 25 | [25-domain-specific-agents.md](25-domain-specific-agents.md) | Six domain profiles (coding, research, blockchain, data, ops, writing) + TypedContext + chain-of-custody |
| 26 | [26-statehub-rearchitecture.md](26-statehub-rearchitecture.md) | Promote StateHub from TUI helper to kernel subsystem: typed projections, filterable subs, query + stream |
| 27 | [27-realtime-event-surface.md](27-realtime-event-surface.md) | WebSocket / SSE / gRPC with a single wire protocol; channels, cursors, back-pressure, auth; first-party client libs |
| 28 | [28-cli-parity-familiar-workflows.md](28-cli-parity-familiar-workflows.md) | Claude Code / OpenClaw / Aider muscle memory adopted where non-conflicting; slash commands, diff-first, budgets |
| 29 | [29-web-ui-architecture.md](29-web-ui-architecture.md) | Five-page first-party web UI on SvelteKit + StateHub; component library, deep-linkable URLs, PWA |
| 30 | [30-rich-ux-primitives.md](30-rich-ux-primitives.md) | Ten UX primitives: reasoning streams, tool banners, heuristic footnotes, replay scrubber, uncertainty bars, ... |

### Integrators: synergy, safety, observability, glossary, roadmap (31–35)

| # | File | What |
|---|---|---|
| 31 | [31-synergy-integration-map.md](31-synergy-integration-map.md) | How every primitive in 02–30 reinforces the others; the moat is the *interaction*, not any single feature |
| 32 | [32-safety-sandbox-provenance.md](32-safety-sandbox-provenance.md) | Safety spine: role auth, sandboxes, pre/post checks, taint, attestation, chain-of-custody — across every layer |
| 33 | [33-observability-telemetry.md](33-observability-telemetry.md) | Logs, metrics, traces, events, replay, cost; what ships, what's pluggable, what's uniquely Roko |
| 34 | [34-glossary.md](34-glossary.md) | Every new term, every reclaimed term, every deprecated term, in one place |
| 35 | [35-consolidated-roadmap.md](35-consolidated-roadmap.md) | The cross-doc dependency graph and a six-to-twelve-month sequencing |

## Relationship to existing planning docs

This folder does not replace `tmp/ux-followup/` or `tmp/MASTER-PLAN.md`. It
sits above them: the items in `ux-followup/` are gap-catalogue entries against
the *current* architecture, and they should all still close. This folder
proposes a framing layer on top that makes the gap catalogue simpler to
reason about (e.g. item 24 "EngineEventBus proposal" becomes a one-liner:
"promote roko-runtime EventBus to a kernel trait").

If anything here conflicts with `docs/00-architecture/23-architectural-analysis-improvements.md`,
defer to 23 — it was the audit that caught the layer violation this folder
tries to dissolve. The `35-consolidated-roadmap.md` maps every refinement
item back to the ux-followup catalog and the master plan.

## How to use

Read `01` first for the critique, then `02` and `03` for the core proposal,
then `04`–`05` for how the loop and traits change. `06`–`08` are for someone
scoping the actual work. `09` is for the Phase 2+ team (chain, dreams,
coordination) to see why this refactor is worth doing before they build on top.

For the second arc (`10`–`21`): `10` and `11` stake out the learning and
HDC substrate story. `12` (demurrage) and `14` (heuristics) are the
memory-quality primitives; `13` (c-factor) is how we measure whether
collective intelligence is actually improving. `15` zooms out on why all
of this compounds. `16` wires academic research into the runtime. `17`
and `20` are the modularity / extension story; `18` and `19` are the
defensibility / differentiation story. `21` is the honest assessment of
which pieces benefit from a clean rewrite versus incremental refactor.

For the third arc (`22`–`30`): `22` is for Rust devs building custom
agents; `23`–`28` are for people running Roko day-to-day; `29`–`30` are
the web and UX polish layer; `26`–`27` are the architectural spine that
makes all the UIs possible.

The integrator arc (`31`–`35`) exists to stop the previous 30 docs from
reading as a pile of disconnected ideas. `31` is the connective tissue;
`32` is the defensive spine nobody thinks about until it fails;
`33` is the instrumentation that turns every claim in the earlier docs
into a measurable assertion; `34` is the canonical vocabulary; `35` is the
sequencing.

### Suggested reading orders

- **I want the big idea in ten minutes**: 00 (one-sentence summaries above) → 01 → 04 §10 → 15 → 19 → 31.
- **Product / strategy**: 18 → 19 → 15 → 13 → 17 → 29 → 34.
- **Architecture / engineering**: 02 → 03 → 20 → 11 → 12 → 14 → 21 → 26 → 27 → 31 → 32.
- **Research / academic**: 10 → 16 → 14 → 13 → 19 → 31.
- **Onboarding a new contributor**: 01 → 02 → 03 → 20 → 17 → 22 → 34.
- **UX design**: 23 → 28 → 30 → 29 → 26 → 27 → 33.
- **Operator / SRE**: 24 → 26 → 27 → 23 → 25 → 32 → 33.
- **Domain lead (coding / research / blockchain / ops)**: 25 → 17 → 22 → 28 → 32.
- **Security / compliance reviewer**: 32 → 24 §8 → 17 §5 → 18 → 14 → 16 §5.
- **Someone planning the actual work**: 35 → 06 → 21 → 20 → 17.

## Conventions inside the folder

- **Cross-links** between refinement docs use the bare filename without
  a leading path: `see 03-bus-as-first-class.md §5`, not
  `see tmp/refinements/03-bus-as-first-class.md §5`.
- **Links into the existing architecture docs** use the real relative
  path: `see docs/00-architecture/23-architectural-analysis-improvements.md`.
- **Code paths** are `crates/roko-<name>/src/<file>.rs` relative to
  the workspace root `/Users/will/dev/nunchi/roko/roko/`.
- **TL;DR** every doc opens with one. If you're skimming, the TL;DRs
  chained together are the executive summary of the folder.
- **"For first-time readers"** callouts appear at the top of docs that
  assume context from elsewhere. Three-to-four sentence orientations.
- **No emojis** in doc text. Status glyphs (`✓`, `✗`, `⚠`) are fine in
  examples of UI output.
- **Concrete over abstract**: every doc includes at least one worked
  example (code sketch, table, worked scenario) somewhere.
- **Name the prior art**: when a doc introduces a concept borrowed from
  research or another system, it cites. Citations become Paper Engrams
  once `16-research-to-runtime.md` lands.
