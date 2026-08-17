# tmp/archive/ -- What is this?

> **Status**: ARCHIVED -- historical development artifacts
> **Last updated**: 2026-08-13

This directory is the graveyard for completed, superseded, or stale development artifacts.
Contents were moved here from other `tmp/` subdirectories after their work was finished.
Nothing here is actively used by the runtime or referenced by current documentation.

## Contents

| Directory | What | Approx size |
|---|---|---|
| `04-21-26/` | Single-day work artifacts from April 21, 2026 | Small |
| `demo/` | Demo task definitions | Small |
| `demo-parity/` | Demo feature parity checklist | ~15 files |
| `depth-v1/` | First-pass deep documentation | ~24 files |
| `depth-v1-copy/` | Duplicate of depth-v1 (backup) | ~24 files |
| `done-runners/` | Completed runner scripts and their logs (docs-parity, refinements, TUI, UX, PRD-enhance) | Large (~600+ files, mostly logs) |
| `learnings-v1/` | Early learning subsystem notes | ~12 files |
| `misc/` | Mixed artifacts: architecture plans, implementation plans, PRD analysis, stale root docs | ~150+ files |
| `new-docs-section-00-v1/` | First attempt at structured documentation tree (reference, research, strategy, testing) | ~50+ files |
| `new-docs-v1/` | Second attempt at structured documentation tree | ~50+ files |
| `roko-trustworthy/` | Trustworthiness/safety design docs | ~10 files |
| `stale-root/` | Old root-level documents moved out of the way | ~30 files (includes MASTER-PLAN, MORI-PARITY-GAP-ANALYSIS, architecture redesigns) |
| `visual-gate-v1/` | Visual gate pipeline design | ~10 files |
| `workflow-v1/` | Workflow execution design | ~15 files |

## Naming note

Many files in this archive (especially in `done-runners/`, `new-docs-v1/`, `misc/`)
use "Engram" as the data unit name. This was the **old spec-level name** for what is now
called "Signal" in all current documentation. The Rust struct is still named `Engram`
(in `roko-core::engram`) with `type Signal = Engram;` as a bridge. Spec-level
references to "Engram" in these archived files should be understood as equivalent to
"Signal."

## Do NOT modify these files

These are historical artifacts. Over 1,000 files reference the old naming. Bulk-renaming
would destroy git blame history for no practical benefit. New documentation should use
"Signal" exclusively.
