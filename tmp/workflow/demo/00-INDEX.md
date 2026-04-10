# Demo Planning: Nunchi Series A

**Location**: `tmp/workflow/demo/`
**Date**: April 2026

**Important context**: These documents describe the Series A demo for Nunchi — a company building infrastructure for AI agent coordination. The product has two components: Roko (an open-source Rust runtime, ~177K LOC, 18 crates) and the Nunchi blockchain (a purpose-built sovereign EVM L1). The codebase lives at `/Users/will/dev/nunchi/roko/roko/`. If you have no prior context about this project, start with **CODEBASE-CONTEXT.md** — it explains every crate, every API endpoint, the build pipeline, and how everything connects.

---

## Documents

| File | What It Covers |
|------|---------------|
| **00-INDEX.md** | This file — orientation, summary, and reading order |
| **CODEBASE-CONTEXT.md** | **START HERE if you have no prior context.** Complete technical reference: glossary, all 33 crates, the `roko run` execution loop, all ~115 API routes, the React SPA (43 files), the build pipeline, the ACP editor protocol, configuration schema, CLI commands. Everything you need to understand the codebase. |
| **UI-AUDIT.md** | Comprehensive audit of the current React SPA — every page, component, API endpoint, demo script, and gap |
| **DEMO-STRATEGY.md** | Master strategy — what to demo, why, for whom, narrative arc, key numbers with sources, competitive positioning |
| **DEMO-VISUAL-SPEC.md** | Detailed visual design — design tokens, typography, terminal aesthetics, four dashboard views, shareable URL page, landing page, deck visual language |
| **DEMO-FLOW.md** | Beat-by-beat script — 3-minute general VC version, 5-minute a16z version, "hand them the laptop" moment, objection handling, timing rehearsal |
| **DEMO-COMPETITIVE.md** | Every relevant competitor — their demo, product, visual design, strengths, gaps, and what to say about each |
| **DEMO-BUILD.md** | Implementation requirements — what exists, what's missing, tiered build plan with effort estimates, dependency graph, acceptance criteria |
| **DEMO-IMPLEMENTATION.md** | Concrete React SPA implementation plan — ROSEDUST design system (from bardo/18-interfaces) adapted for web, component architecture, 6-phase build order, design tokens, page specs with ASCII mockups, Rust integration, verification checklist |

---

## The Demo in One Paragraph

Open a terminal. Run one command. The output shows a verified agent identity, three policy gates passing, a cost prediction, knowledge loaded from prior agents, and a cost delta (30% below prediction). Run a second agent — it starts with knowledge from the first, finishes cheaper. Kill the third agent mid-run. Resume from checkpoint. Zero work lost. Four primitives in three minutes: identity, cost prediction, shared knowledge, durability. The `--share` flag produces a URL with the full execution timeline, cost breakdown, and ZK proof.

---

## Reading Order

1. **CODEBASE-CONTEXT.md** — if you have no prior context, read this first. It defines every term, explains every component, and maps the entire codebase.
2. **DEMO-STRATEGY.md** — the "what and why" — the four primitives, narrative arc, market context, audience framing
3. **DEMO-FLOW.md** — the "how it happens" — beat-by-beat timing, exact commands, expected output, what to say
4. **DEMO-VISUAL-SPEC.md** — the "what it looks like" — design tokens, terminal aesthetics, dashboard views, visual benchmarks
5. **DEMO-COMPETITIVE.md** — the "how we compare" — every competitor's demo, product, gaps, and what to say about each
6. **DEMO-BUILD.md** — the "what to implement" — what exists, what's missing, tiered build plan with effort estimates
7. **UI-AUDIT.md** is reference — consult when you need to understand the current state of the React SPA

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Primary demo surface | CLI terminal, not web dashboard | Terminal is faster, more reliable, more impressive. Dashboard is secondary/technical diligence. |
| Terminal theme | Tokyo Night | Electric blue/purple palette signals sophistication. Strong community association with modern CLI tools. |
| Dashboard design | Evolved ROSEDUST → near-black + accent blue | Keep dark-first approach, replace dusty rose with infrastructure-appropriate blue |
| Font family | Geist Sans + Geist Mono | Vercel ecosystem, free, clean, professional |
| Charts | Canvas 2D, no library | Zero dependency, full control, matches existing approach |
| Knowledge visualization | Terrain (d3-contour + Canvas 2D), force-graph fallback | Compounding=elevation, demurrage=erosion. The visual differentiator — no competitor has this |
| Pulse Globe | three-globe + Three.js + UnrealBloomPass | 5K arcs at 60fps, 6 Lens colors, cold-open hero animation |
| Computation receipt | Hybrid dark canvas + cream receipt card (flip) | The artifact that leaves the room — downloadable PDF |
| Live benchmark widget | Bloomberg Two-Tape (400×300px corner overlay) | Roko vs LangGraph side-by-side, p<0.01 winner declaration |
| Primary benchmark | HAL (Princeton, ICLR 2026) | 9 benchmarks, Weave cost integration, reproducible methodology |
| Demo length | 3 minutes (general) / 5 minutes (a16z) | Research shows VC attention drops sharply after 2:14. Keep it tight. |
| "Stripe moment" | `nunchi run --share` → URL in 10 seconds | Produces an artifact that leaves the room with the investor |

---

## Key Intelligence (From Research)

| Finding | Impact | Where Integrated |
|---------|--------|-----------------|
| Aubakirova's "Big Ideas 2026" describes Nunchi's Pulse fabric verbatim | **Highest-leverage opening move**: show her quote on screen while Pulse fans out 5K sub-tasks | DEMO-FLOW.md §3 opening options |
| Aubakirova's "janky but native" phrase (Apr 22, 2026) | Perfect justification for 177K-line Rust runtime | DEMO-STRATEGY.md §4 a16z version |
| Casado's "non-consensus investing is overrated" | Do NOT lead with contrarian frame — consensus pitch for non-consensus product | DEMO-STRATEGY.md §9, DEMO-COMPETITIVE.md §1b |
| Casado has no public canon on sovereign EVM L1s | Frame chain as "vertical cloud for agent identity and settlement" | DEMO-STRATEGY.md §4 |
| The reverse demo ("give me a task you'd actually do") | Highest-risk, highest-payoff demo mechanic | DEMO-STRATEGY.md §5 |
| Physical Nunchi Cell PCB (~$30/unit) | Most durable physical artifact — sits on his desk for years | DEMO-STRATEGY.md §14 |
| Berkeley Mono > Geist (typography signaling) | "I read the manual; I have taste" vs "I deploy to Vercel" | DEMO-VISUAL-SPEC.md §2 |
| The dream cycle as deliberate withhold | Creates a reason for the second meeting | DEMO-FLOW.md §7 |
| Pre-meeting media arc (essay → Stratechery → Latent Space) | Pitch is the climax of a 6-week arc, not its opening | DEMO-STRATEGY.md §13 |
| 13 Casado portfolio companies to never criticize | Cursor, Convex, Netlify, Kong, Truffle, etc. | DEMO-COMPETITIVE.md §1b |
| Full 9-essay Aubakirova corpus verified | Author hub: a16z.com/author/malika-aubakirova/ — all on a16z.com, no external writing | DEMO-STRATEGY.md §4 |
| Name correction: Malika (byline) / Maika (conversational) | Use "Malika" in written follow-up | DEMO-STRATEGY.md §4 |
| Keycard sovereignization argument | Same axis (static→dynamic identity), perpendicular (centralized→sovereign). Defuse overlap with kill-chain framework | DEMO-STRATEGY.md §4, DEMO-COMPETITIVE.md §Keycard |
| Adjacent partners dossier (de la Garza, Lackey, Bornstein, Li) | Bridge tension: Joel="year of agents" vs Matt="agents don't work" → Nunchi closes the gap | DEMO-STRATEGY.md §4 |
| a16z vocabulary lexicon (10 terms) | Mirror her exact language: "machine identity," "agentic inference," "validated paths" | DEMO-STRATEGY.md §4 |
| Terrain viz > force-graph for knowledge | Compounding=elevation, demurrage=erosion. d3-contour, ~200 LOC delta, 60fps on M1 | DEMO-VISUAL-SPEC.md §View 3 |
| Pulse Globe for cold-open | three-globe, 5K arcs, 6 Lens colors, UnrealBloomPass glow | DEMO-VISUAL-SPEC.md §Pulse Globe |
| Hybrid computation receipt (dark+cream flip card) | The literal "thing that leaves the room" — downloadable PDF | DEMO-VISUAL-SPEC.md §6 |
| Crushed Bar cost comparison (3.3% vs 100%) | Tufte-correct at 30× ratio, on-viewport animation | DEMO-VISUAL-SPEC.md §6 |
| Bloomberg Two-Tape live benchmark widget | 400×300px corner overlay, p<0.01 winner declaration | DEMO-VISUAL-SPEC.md §7, DEMO-BUILD.md T2c.2 |
| HAL as primary benchmark, 5-task live subset | τ-bench + AppWorld + GAIA, ~$0.20 vs ~$6-8, ≤3 min wall clock | DEMO-BUILD.md T2c, DEMO-STRATEGY.md §3 |
| 30× cost number NOT directly verifiable | Reproduce locally or cite "derived from HAL methodology" — do not invent precision | DEMO-STRATEGY.md §2, DEMO-BUILD.md §5 |

---

## Source Material

These documents were synthesized from:

- **`tmp/unified/`** — 29 v3.0 specification documents (~1.5MB) covering the full Signal/Cell/Graph architecture
- **`tmp/unified-depth/`** — ~128 depth documents across 22 directories with detailed treatment of each subsystem
- **`tmp/research/`** — 15 research documents covering academic landscape, market strategy, a16z-specific pitch strategy, and pitch deck design
- **`tmp/learnings2/`** — 15 distilled briefing documents with quality ratings (Strategy: 9.5/10, Demo: 9.5/10, Competitive Intel: 9.2/10, Risks: 9.3/10)
- **`tmp/demo-resources/`** — 20 shell scripts across 8 directories that exercise the real `roko serve` API
- **Competitive research** — April 2026 analysis of Temporal, LangChain/LangSmith, CrewAI, Cursor, Devin, Linear, Vercel, Stripe
- **Visual design research** — April 2026 analysis of dark mode design systems, terminal aesthetics, dashboard patterns, landing page patterns
- **`tmp/ressearch2/research1.md`** — Deep research on demo mechanics, Casado/Aubakirova dossier, pitch performance art, physical artifacts, pre-meeting media arc, typography signaling, recovery protocols
- **`tmp/ressearch2/research3.md`** — Synthesis research: verified 9-essay Aubakirova corpus with per-essay tactical extraction, Keycard deep-dive, adjacent partner dossiers, execution trace visualization patterns (Chrome DevTools/Datadog/Jaeger/Honeycomb convergence), 4 computation receipt mockups, 3 cost comparison patterns, terrain/mycelial/star-map knowledge graph concepts, Pulse Globe Three.js spec, HAL/GAIA/τ-bench benchmark methodology, 4 live corner widget mockups, 5-task demo subset
