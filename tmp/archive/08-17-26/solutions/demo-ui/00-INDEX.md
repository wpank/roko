# Demo UI Solutions Index (ARCHIVED)

> **This directory is archived.** The canonical spec is at **`tmp/solutions/demo-ui2/`**.
> All unique content from this series (Parts 0-13, AGENT-PROMPT) has been consolidated into the demo-ui2 10-document set.

Historical audit and solution catalog for the Roko demo app.
Goal: VC-grade presentation surface for $20M Series A.

## Documents

- [01-ARCHITECTURE-CRITIQUE.md](01-ARCHITECTURE-CRITIQUE.md) — Where bandaids exist, what proper design looks like (12 items)
- [02-RUNTIME-BUGS.md](02-RUNTIME-BUGS.md) — Things that crash, leak, or produce wrong results (17 items)
- [03-PRESENTER-CONTROL.md](03-PRESENTER-CONTROL.md) — Demo flow, pacing, step control issues (17 items)
- [04-VISUAL-AUDIT.md](04-VISUAL-AUDIT.md) — Per-page visual ratings, Playwright screenshots, improvements (14 pages)
- [05-WORKFLOW-E2E.md](05-WORKFLOW-E2E.md) — End-to-end workflow test results, friction log (9 workflows)
- [06-SOLUTIONS.md](06-SOLUTIONS.md) — Prioritized fix list with estimated effort (25 items)
- [07-DEEP-SOURCE-AUDIT.md](07-DEEP-SOURCE-AUDIT.md) — Line-by-line source audit: 54 new findings (bugs, UX, perf, a11y, dead code)
- [08-VISUAL-REASSESSMENT.md](08-VISUAL-REASSESSMENT.md) — Post-fix visual ratings with screenshots, per-page analysis
- [09-REDESIGN-PROPOSALS.md](09-REDESIGN-PROPOSALS.md) — What proper from-scratch design looks like per page
- [10-MASTER-CHECKLIST.md](10-MASTER-CHECKLIST.md) — **Master implementation checklist: 48 items, from-scratch solutions, agent-ready plans**
- [11-NEXT-GEN-SPEC.md](11-NEXT-GEN-SPEC.md) — **Complete from-scratch redesign spec: architecture, design system, page specs, file structure, implementation plan**
- [12-CURRENT-DEMO-APP-ISSUE-TRACKER.md](12-CURRENT-DEMO-APP-ISSUE-TRACKER.md) — **Current live ledger for `demo/demo-app`: bugs, broken wiring, anti-patterns, loose ends, and dead-code candidates**
- [13-GAME-UX-DESIGN-SYSTEM.md](13-GAME-UX-DESIGN-SYSTEM.md) — **Game-like UX design system: motion, animation, extensible Cell architecture, DataHub, scene-by-scene specs, roko endpoint mapping, multi-agent Spectre identity system, agent-attributed terminals/logs, implementation phases**

## Current Working Ledger

Use [12-CURRENT-DEMO-APP-ISSUE-TRACKER.md](12-CURRENT-DEMO-APP-ISSUE-TRACKER.md) as the current source of truth for unresolved issues. The earlier files are useful historical context, but several claims are stale against the current `demo/demo-app` source.

## Stats

- **39 runtime bugs** catalogued (B1–B39: 3 critical, 8 high, 6 medium + 22 new)
- **12 architecture critiques** with proper-design alternatives
- **17 presenter control issues** (4 blockers, 9 friction, 4 cosmetic)
- **20 views visually audited** with per-page ratings (twice: pre-fix and post-fix)
- **15 UX issues** catalogued (U1–U15)
- **10 visual issues** total (V1–V10)
- **5 accessibility issues** (A1–A5)
- **3 performance issues** (PERF1–PERF3)
- **6 dead code items** (DC1–DC6)
- **9 end-to-end workflows** tested with friction logs
- **8 redesign proposals** with mockups
- **25 prioritized solutions** (~17 hours estimated total effort)

## Key Numbers

| Metric | Value |
|--------|-------|
| Pages/views audited | 20 |
| Console errors found | 0 |
| Average visual rating (post-fix) | ~7.2/10 (estimated after batch 9) |
| Average potential rating | 9.2/10 |
| Total catalogued issues | 130+ |
| Crashing views | 0 (B39 fixed) |
| Scenarios with working step mode | 3/15 |
| Views below 5.0 rating | 0 (after batch 9 fixes) |
| Dead nav links | 0 |
| Dead code items | 6 |

## Fixes Already Applied (Batches 1–9)

| Batch | What | Status |
|-------|------|--------|
| 1. CSS variable aliases | 8 aliases in rosedust.css `:root` | ✅ Applied |
| 2. Duplicate route | Removed nested ShareView route + import | ✅ Applied |
| 3. KnowledgeEntries fallback | Switched to useApiWithFallback | ✅ Applied |
| 4. Server health false positive | Removed demo-mode fake connected | ✅ Applied |
| 5. ChainView timer leak | Added timeoutRef, clear on cleanup | ✅ Applied |
| 6. Builder model selection | Pass --model to CLI command | ✅ Applied |
| 7. Share loading state | Show "Loading receipt..." instead of null | ✅ Applied |
| 8. Dead ShareView | Deleted unreachable component | ✅ Applied |
| 9. Visual polish batch | See below | ✅ Applied |

### Batch 9 Fixes (latest)

| Fix | What | Bugs Fixed |
|-----|------|------------|
| Explorer redesign | Removed tabs, combined activity stream, auto-polling, crash-safe events | B39, U1, U3, U4, U5, V8 |
| Terminal fill space | Terminals flex to fill viewport, per-terminal close buttons, aria-labels | A2 |
| Builder preset wrap | `flex-wrap` on presets so they don't overflow | V6 partial |
| Knowledge Graph fix | Larger nodes (8+citations*1.5), working glow, energy-based animation stop, bigger labels | B18, B19, B20 |
| Fleet topology | 40% larger nodes, 35% spread radius, side-by-side layout, energy-based stop, bigger labels | B37 |
| Chain single-screen | Combined explanation+features, waterfall beside text, hash in mosaic | — |
| CostDashboard cleanup | Moved pulse-dot to rosedust.css, Status text in mono font | U9, V10 |
| GateWaterfall fix | roundRect polyfill for Safari, canvas clear on empty | B21, B22 |
| GateBar a11y | aria-label on status icons | A3 |
| TopNav a11y | aria-hidden on decorative mark | A4 |
| Dashboard nav a11y | aria-label on inner nav | A5 |
| useSSE leak fix | clearTimeout before reassigning reconnectTimer | B27 |
| useBench leak fix | Clear pollRef on double-start | B8 |

## Priority Fix Order (What's Next)

### Tier 0: Done ✅
1. ~~Explorer redesign~~ — combined activity stream (done)
2. ~~Knowledge Graph~~ — larger nodes, fix glow, real labels (done)
3. ~~Fleet~~ — scale topology, fit cards without scroll (done)
4. ~~Chain~~ — compress to one screen (done)
5. ~~Terminal~~ — fill available space (done)
6. ~~Builder~~ — presets wrap (done)

### Tier 1: Remaining high-impact
7. **Demo progressive disclosure** — start empty, reveal per phase (6.5 → 9.0)
8. **Rainbow color bar removal** — needs shell profile change or terminal clear-on-connect
9. **Typography pass** — minimum 12px body text everywhere (partially done)

### Tier 2: Data + UX
10. **Entries/Routing empty state** — show demo data when server returns empty arrays (U15)
11. **Bench links** — add View/Compare links from history table
12. **BenchCompare validation** — prevent same-run comparison (B25)

### Tier 3: Polish
13. Empty states for remaining panels
14. Fix B1 listener accumulation (partially addressed)
15. Fix B2 Promise.all null guards
16. Fix B9 resolveRoko TOCTOU race

## Screenshots

- Post-fix screenshots: `/tmp/roko-audit-1777443964541/`
- Previous screenshots: `/tmp/roko-demo-visual-audits/all-1777437536365/`

## Status

- Created: 2026-04-29
- Last audit: 2026-04-29 (Playwright, 20 views, 0 console errors, line-by-line source review)
- Last fix batch: 2026-04-29 (Batch 9: 13 fixes across 12 files)
- Total findings: 130+ across 9 documents
- Build: ✅ passes (tsc + vite build, 0 errors)
