# Refinements

> Index of all known design-proposal documents from `tmp/refinements/`. Each entry is a placeholder; the actual proposal content lives in the source file and has not yet been migrated to this tree.
>
> Migration of individual refinement content is a separate action from this Cluster I migration.

**Last reviewed**: 2026-04-19

---

## What this folder is

The `tmp/refinements/` directory contains a set of numbered design proposals (REF02–REF34) that collectively describe the architecture target state. This folder is the first-class home for those proposals once they are migrated.

Until migration, each entry below is a **placeholder** — it records the file exists, what it covers, and which roadmap milestones or refactor phases reference it, but it does not contain the proposal content.

---

## Index

### Kernel / Two-Medium Transport (REF02–REF09)

| ID | Source file | Topic | Status | Roadmap home |
|---|---|---|---|---|
| REF02 | `02-engram-vs-pulse.md` | `Engram` vs. `Pulse` distinction | Not yet migrated | Q1 — Phase B |
| REF03 | `03-bus-as-first-class.md` | `Bus` as first-class kernel primitive | Not yet migrated | Q1 — Phase B |
| REF04 | `04-operators-generalized.md` | `Datum<'_>` and generalized operators | Not yet migrated | Q1 — Phase B |
| REF05 | `05-loop-retold.md` | Seven-step universal cognitive loop | Not yet migrated | Q1 — Phase A + B |
| REF06 | `06-refactoring-plan.md` | Phased refactor plan (primary source for Phase A–D) | Not yet migrated | Q1 — Phases A–C |
| REF07 | `07-naming.md` | Canonical naming and rename pass | Not yet migrated | Q1 — Naming track |
| REF08 | `08-code-sketches.md` | Code sketches for new kernel types | Not yet migrated | Q1 — Phase B |
| REF09 | `09-phase-2-implications.md` | Phase-2 Bus/Substrate backends (ChainBus, MeshBus) | Not yet migrated | Q5–Q6 Phase D |

### Learning Substrate (REF10–REF16)

| ID | Source file | Topic | Status | Roadmap home |
|---|---|---|---|---|
| REF10 | `10-self-learning-cybernetic-loops.md` | Self-learning loops (prediction/outcome topics) | Not yet migrated | Q2 |
| REF11 | `11-hyperdimensional-substrate.md` | HDC fingerprint on every `Engram` | Not yet migrated | Q2 |
| REF12 | `12-knowledge-demurrage.md` | Demurrage / economically-shaped memory | Not yet migrated | Q2 |
| REF13 | `13-collective-intelligence-c-factor.md` | c-factor measurement and actuation | Not yet migrated | Q2 + Q4 |
| REF14 | `14-worldview-validation.md` | Heuristics, falsifiers, calibration | Not yet migrated | Q2 |
| _(REF15)_ | _(not referenced in sources)_ | Unknown | Not inventoried | — |
| REF16 | `16-research-to-runtime.md` | Research-to-runtime, replication ledger | Not yet migrated | Q2 + Q4 |

### Ecosystem and UX (REF17–REF30)

| ID | Source file | Topic | Status | Roadmap home |
|---|---|---|---|---|
| REF17 | `17-plugin-extension-architecture.md` | Plugin SPI (staged extension model, WASM) | Not yet migrated | Q3 |
| REF18 | `18-competitive-moat.md` | Competitive moat framing | Not yet migrated | — |
| _(REF19)_ | _(not referenced in sources)_ | Unknown | Not inventoried | — |
| REF20 | _(no filename in sources)_ | Modularity and composability / crate seams | Not yet migrated | Q1 — Modularity |
| _(REF21)_ | _(not referenced in sources)_ | Unknown | Not inventoried | — |
| REF22 | _(no filename in sources)_ | Developer UX (Rust SDK, `roko init`, CLI) | Not yet migrated | Q3 |
| REF23 | `23-user-ux-running-agents.md` | User UX — running agents | Not yet migrated | Q3 |
| REF24 | `24-deployment-ux.md` | Deployment UX + hardening | Not yet migrated | Q3 + Q4 |
| REF25 | `25-domain-specific-agents.md` | Domain profiles (`TypedContext`, starter heuristics) | Not yet migrated | Q4 |
| REF26 | `26-statehub-rearchitecture.md` | StateHub projection (kernel-tier shared data surface) | Not yet migrated | Q3 |
| REF27 | _(no filename in sources)_ | Realtime wire surface / protocol freeze | Not yet migrated | Q3 |
| REF28 | _(no filename in sources)_ | CLI parity | Not yet migrated | Q3 |
| REF29 | _(no filename in sources)_ | Web UI | Not yet migrated | Q3 |
| REF30 | `30-rich-ux-primitives.md` | Rich UX primitives | Not yet migrated | Q3 |

### Integrators and Hardening (REF31–REF34)

| ID | Source file | Topic | Status | Roadmap home |
|---|---|---|---|---|
| REF31 | `31-synergy-integration-map.md` | Synergy and integration framing | Not yet migrated | Q4 |
| REF32 | `32-safety-sandbox-provenance.md` | Safety spine (custody, taint, provenance) | Not yet migrated | Q4 |
| REF33 | `33-observability-telemetry.md` | Observability and telemetry | Not yet migrated | Q1 (baseline) + ongoing |
| REF34 | `34-glossary.md` | Glossary consolidation (canonical vocabulary) | Not yet migrated | Q1 — Naming track |

---

## Files referenced in source docs but not assigned REF numbers

| File | Topic | Referenced in |
|---|---|---|
| `35-consolidated-roadmap.md` | Consolidated roadmap (canonical proposal) | `33-refactor-plan-phases.md`, `35-consolidated-roadmap.md` |

---

## Migration notes

- REF01 is not referenced in any architecture source file. Its existence is inferred from the REF numbering series. No filename has been identified.
- REF15, REF19, REF21 are gaps in the numbering. Their files are either unnumbered, use different naming, or do not exist.
- REF20, REF22, REF27, REF28, REF29 are referenced by number in the roadmap but no `tmp/refinements/` filename has been identified for them. They may be named differently on disk.
- The `naming-history.md` stub assigned to this folder by Cluster D (see `strategy/refinements/naming-history.md`) covers the renaming history from earlier vocabulary to the current canonical names. It is produced by Cluster D, not this cluster.

---

## See also

- [`strategy/roadmap/README.md`](../roadmap/README.md) — how refinements map to quarterly milestones
- [`strategy/refactor-phases/README.md`](../refactor-phases/README.md) — how refinements map to phases A–D
- [`GLOSSARY.md`](../../GLOSSARY.md) — canonical vocabulary produced by REF34 / REF07
