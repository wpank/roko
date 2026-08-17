# tmp/unified/ -- What is this?

> **Status**: ARCHIVED -- superseded specification drafts
> **Last updated**: 2026-08-13

This directory contains two archived versions of the "Unified Specification," which was
the master design document for the roko agent economy protocol. These are large, detailed
spec documents (22+ chapters each, ~1MB total) that defined Signal/Cell/Graph fundamentals,
9 protocols, 10 specializations, and the full system architecture.

## Contents

| Directory | What | Size |
|---|---|---|
| `v1-archive/` | v2.0 draft (April 2026) -- uses "Block" as computation primitive | 30 files, ~1MB |
| `v2-archive/` | v2.0 final (April 2026) -- uses "Cell" as computation primitive | 28 files, ~850KB |

## Relationship to current codebase

The spec defined aspirational architecture. The **actually implemented** subset is
documented in `CLAUDE.md` at the workspace root. Key differences:

- The spec defines Blocks/Cells, Graphs, Flows, Racks, Triggers, Loops, etc. as
  composable primitives. The codebase implements `Signal` + 6 verb traits
  (Substrate, Scorer, Gate, Router, Composer, Policy) as the practical kernel.
- The spec's "predict-publish-correct" learning pattern is partially realized through
  the cascade router, adaptive gate thresholds, and efficiency tracking.
- VCG auction composition is built but greedy path dominates at runtime.

## Naming note

These docs discuss the Signal naming transition (renamed from Engram in 2026-08-12). Both
versions correctly note that the Rust struct is now `Signal` with `pub type Engram = Signal;`
as a backward-compat alias. The spec-level name "Signal" is used throughout CLAUDE.md and
all newer documentation.

## Do NOT modify these files

These are historical specification snapshots. They should not be updated to reflect current
code state -- that is what CLAUDE.md and `.roko/GAPS.md` are for.
