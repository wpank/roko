# Demo UI v2: Comprehensive Redesign

**The canonical specification** for rebuilding the Roko demo surface. This is the single source of truth.

Synthesizes and consolidates:
- `tmp/solutions/demo-ui/` — 380KB of historical audit docs (01-13), now archived
- `bardo/prd/18-interfaces/` — 28 PRD files from the predecessor bardo/mori interface vision
- `demo/demo-app/src/` — current codebase: ~16.6K LOC, 100+ files

## Documents

| # | File | What |
|---|------|------|
| 01 | [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | Complete inventory of what exists: subsystems, components, hooks, data flows, API surface, what works, what's broken |
| 02 | [02-ARCHITECTURE.md](02-ARCHITECTURE.md) | Target architecture: DataHub, transport layer, Cell system, motion system, **layout architecture** (sticky TopNav + scrollable container + component library structure), **terminal/demoscene aesthetic** as architectural principle, **pipeline state machine**, **Activity Strip**, loading/skeleton/error states, state persistence |
| 03 | [03-REALTIME-DATA.md](03-REALTIME-DATA.md) | Generalized real-time data architecture: SSE/WS/REST unified adapter layer, event taxonomy, subscription model |
| 04 | [04-DESIGN-SYSTEM.md](04-DESIGN-SYSTEM.md) | ROSEDUST v2 design tokens, component specs, animation system, atmospheric layers |
| 05 | [05-PAGES.md](05-PAGES.md) | Page-by-page specs: Orchestrate, Observe, Evaluate, Build, Knowledge |
| 06 | [06-AGENT-MODEL.md](06-AGENT-MODEL.md) | Portable agent concept: lifecycle modes, **Spectre avatar system** (procedural dot-cloud identity), **agent-attributed terminals**, **agent-attributed logs**, **topology graph**, **knowledge transfer viz**, **multi-agent playback** |
| 07 | [07-IMPLEMENTATION.md](07-IMPLEMENTATION.md) | Phased implementation plan: 75 tasks across 6 phases, with file paths, line numbers, acceptance criteria |
| 08 | [08-AUDIT-FINDINGS.md](08-AUDIT-FINDINGS.md) | Codebase audit: dead code inventory, memory leaks, bugs, duplicated utilities, CSS conflicts, a11y gaps |
| 09 | [09-DESIGN-PRIMITIVES.md](09-DESIGN-PRIMITIVES.md) | 35+ composable primitives organized by category (design/, layout/, cells/, motion/, ascii/): containers, feedback, data display, layout, interactive, theming, ASCII/terminal aesthetic, **density guidelines**, per-page migration, complete primitives catalog |
| 10 | [10-UX-PHILOSOPHY.md](10-UX-PHILOSOPHY.md) | **Core UX principles** (12), **17 anti-patterns** (hard prohibitions), **"What NOT to Build"** (20 items), **migration guide**, **working methodology**, **"World Class" component checklist**, visual quality targets, inspiration references |
| 10b | [10-EXPRESSIVE-PRIMITIVES.md](10-EXPRESSIVE-PRIMITIVES.md) | **35 advanced primitives**: resizable panes (inside scrollable container, not viewport-locked), loading transitions, stepped progress variants (5), WebGL backgrounds (6), agent-namespaced components (dense cards in page flow, not full-screen panels), event feeds (compact scrollable sections with max-height + fade), floating chat, enhanced modals/overlays, layout additions (4) -- extends 09's catalog with wave 2-4 components |
| 11 | [11-CURRENT-DELTA-CHECKLIST.md](11-CURRENT-DELTA-CHECKLIST.md) | **Live delta checklist**: 85 items across 7 categories (architecture, design system, agent model, UX, a11y, performance, cells) — what the current codebase needs to match the spec, with priority tiers |

## Core Design Principles

The terminal/demoscene aesthetic is a core architectural constraint, not a stylistic preference. All documents in this set are governed by:

1. **Scrollable density-first layout.** Pages scroll naturally; no viewport-locking. TopNav is sticky; everything else flows in a scrollable container. See `04-DESIGN-SYSTEM.md` section 8.
2. **Terminal/demoscene visual language.** Mono uppercase chrome, ASCII vocabulary (box-drawing, braille, block elements, status glyphs), phosphor decay for value changes. See `04-DESIGN-SYSTEM.md` section 9.
3. **Space efficiency.** Tight padding (10-12px cards, 8px gaps), no wasted space, content-determined heights, compact empty states. See `04-DESIGN-SYSTEM.md` section 10, `09-DESIGN-PRIMITIVES.md` Density Guidelines.
4. **DataHub + Cell + Motion.** Three orthogonal systems that compose every feature. See `02-ARCHITECTURE.md` sections 1-5.

## Key Numbers

| Metric | Current | Target |
|--------|---------|--------|
| Source files | 100+ | ~55 |
| LOC | 16,600 | ~8,000 |
| Dead code to delete | 813 lines | 0 |
| Duplicated utilities | 6+ (hexToRgba, shortModel, fmtUptime, canvas DPR, palettes, relativeTime) | 0 |
| Hardcoded colors in TSX | ~130 | 0 |
| Inline styles | ~330 | ~130 (dynamic only) |
| Memory leaks | 2 confirmed | 0 |
| Implementation tasks | — | 75 across 6 phases |
| Pages/views | 14 + 7 dashboard sub-views | 5 scenes + sub-views |
| Hooks | 19 | ~8 thin selectors |
| Context providers | 3 (+ scattered useState) | 1 DataHub (Zustand) |
| SSE connections | 3 independent (orphaned, workflow, bench) | 1 unified transport |
| Data fetching patterns | 5 different | 1 (DataHub actions) |
| Error handling patterns | 5+ different | 1 (DataSurface wrapper) |
| Animation approaches | Ad-hoc CSS + inline + rAF | Motion library + CSS tokens |

## What's Covered

This 11-document set covers the full stack:

| Layer | Documents |
|-------|-----------|
| **What exists (current state)** | 01, 08 |
| **Architecture & data flow** | 02, 03 |
| **Visual design & tokens** | 04 (sections 8-10: layout model, terminal aesthetic, density), 09, 10b |
| **Page specs** | 05 |
| **Agent identity & multi-agent** | 06 |
| **Implementation plan** | 07 |
| **UX philosophy & methodology** | 10 |

## Predecessor Context

### From bardo/prd/18-interfaces/
The bardo TUI specification defined concepts that carry forward:
- **32 interpolating variables** driving all visual state (emotion, health, lifecycle)
- **Three simultaneous timescales** (fast emotion, medium health, glacial lifecycle)
- **Transducer widgets** — each component is a pure visual function of state channels
- **PAD modulation** — pleasure/arousal/dominance vectors multiply into visual parameters
- **Lifecycle-driven degradation** — UI complexity reduces as entity health declines
- **Perpetual motion** — no static pixels; heartbeat pulses, noise, decay animations
- **ROSEDUST design system** — monochromatic rose-on-violet-black, bone accent, 32-token palette
- **Spectre creature system** — procedural dot-cloud identity from agent fingerprint (8 archetypes, spring physics, PAD-driven animation)

### Consolidated from demo-ui/ (archived)
The original audit series (01-13, 16 files, 380KB) has been consolidated into this set:
- Docs 01-08 (audit, bugs, visual ratings, workflow tests) → `01-CURRENT-STATE.md` + `08-AUDIT-FINDINGS.md`
- Doc 09 (redesign proposals) → `05-PAGES.md` + `09-DESIGN-PRIMITIVES.md`
- Doc 10 (master checklist) → `07-IMPLEMENTATION.md`
- Doc 11 (next-gen spec) → `02-ARCHITECTURE.md` + `04-DESIGN-SYSTEM.md` + `05-PAGES.md` + `10-UX-PHILOSOPHY.md`
- Doc 12 (issue tracker) → `08-AUDIT-FINDINGS.md`
- Doc 13 (game UX design system) → `02-ARCHITECTURE.md` (state machine) + `06-AGENT-MODEL.md` (Spectre, multi-agent) + `04-DESIGN-SYSTEM.md` (motion, tokens)
- AGENT-PROMPT.md → `10-UX-PHILOSOPHY.md` (methodology, checklist)

## Historical Reference

The original audit series is archived at `tmp/solutions/demo-ui/`. It contains useful historical context (per-page visual ratings, playwright screenshots, line-by-line source audit) but should not be used as the spec — use this directory instead.
