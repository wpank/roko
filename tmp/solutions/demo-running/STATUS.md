# Demo Pipeline — Status

**Date**: 2026-05-05
**Branch**: `wp-arch2`
**State**: Backend batches committed. Architecture batch partially merged. UX/demo still needs
scenario overhaul and live panel verification.

---

## TL;DR

56 batches attempted. **Only 18 are fully done.** 37 partially done, 1 never started. Specifically:
- **Some formerly-dead items are now wired** (`roko do`, RuntimeEvent ingest, HttpEventSink,
  ACP event forwarding)
- **Several implementations remain partial** (Workspace/RokoLayout migration, `roko do`
  medium/complex pipelines, StateHub crate boundary, ingest E2E tests)
- **~26 items lack end-to-end verification** (code likely correct but never tested)
- **Demo app UX is unchanged** (still 14 old scenarios, no custom panels, no streaming)
- **The scenario redesign was never implemented** (design doc, never decomposed into work)
- **The CLI is still 35+ subcommands** nobody uses

---

## Folder Structure

```
demo-running/
├── STATUS.md              ← You are here
├── CURRENT-STATE.md       ← What's actually wired vs dead (source of truth)
├── DESIGN-DOCS-INDEX.md   ← Master index to ~30 design docs across tmp/
│
├── next-phase/            ← NOT YET IMPLEMENTED — all forward-looking work
│   ├── NEXT-PHASE.md          ← Implementation plan (waves A-D)
│   ├── CLI-REDESIGN.md        ← Synthesized CLI proposal (5 verbs, WorkflowEngine)
│   ├── WIRING-AUDIT.md        ← Dead code catalog + wiring instructions
│   ├── BATCH-GAPS.md          ← Per-batch gap analysis (18/37/1 breakdown)
│   ├── SCENARIO-REDESIGN.md   ← 5 demo scenarios with custom sidebars
│   ├── SCENARIO-DETAILS.md    ← Full specs per scenario
│   ├── SCENARIO-AUDIT.md      ← Diagnosis of current 14 scenarios
│   ├── 04-DEMO-UI-REDESIGN.md ← CommandList + ContextPanel pattern
│   ├── 06-STREAMING-DESIGN.md ← SSE streaming architecture
│   └── TERMINAL-SESSION-REDESIGN.md
│
└── archive/               ← Reference only
    ├── batches-executed/       ← 56 batch files (W0-A through W15-E)
    └── original-docs/          ← Planning docs from sessions 1-7
```

---

## What to Read

| Question | Document |
|----------|----------|
| What's actually working right now? | [CURRENT-STATE.md](CURRENT-STATE.md) |
| Where are all the design docs? | [DESIGN-DOCS-INDEX.md](DESIGN-DOCS-INDEX.md) |
| What's the next implementation plan? | [next-phase/NEXT-PHASE.md](next-phase/NEXT-PHASE.md) |
| What's the CLI overhaul proposal? | [next-phase/CLI-REDESIGN.md](next-phase/CLI-REDESIGN.md) |
| What code exists but is never called? | [next-phase/WIRING-AUDIT.md](next-phase/WIRING-AUDIT.md) |
| What batch work has gaps? | [next-phase/BATCH-GAPS.md](next-phase/BATCH-GAPS.md) |
| What should the 5 demo scenarios be? | [next-phase/SCENARIO-REDESIGN.md](next-phase/SCENARIO-REDESIGN.md) |

---

## Priority Order (Next Steps)

1. **Wire dead code** (~10-12h) — make existing implementations actually run
2. **Clean architecture leftovers** — StateHub crate boundary, duplicate EventBus, path
   migration boundary
3. **CLI simplification** — finish `roko do` beyond the current WorkflowEngine template selector
4. **Streaming infrastructure** — harden RuntimeEvent ingest/HttpEventSink/ACP E2E and inline output
5. **Demo scenario redesign** — 5 scenarios with custom sidebar panels
6. **Engine convergence** — one execution path, kill legacy orchestrate.rs
