# Mori → Roko: Context and Goals

> Written 2026-08-19 to capture Will's intent and serve as perpetual reference.

## The Problem

Roko was a ground-up rewrite of Mori/Bardo to fix architecture issues (streaming,
composability, modularity, extensibility). But the rewrite introduced its own problems:

1. **Tech debt from parallel agent development** — Claude agents built lots of code but
   didn't do the hard wiring work. Features exist but aren't connected end-to-end.
2. **TUI was thrown together** — Not hand-tested or thoughtfully designed. Mori's TUI was
   intuitive because Will hand-chose every decision and verified everything worked.
3. **Architecture may not be sound** — It's unclear if the new architecture actually
   solves the problems it was meant to fix, or if it introduced new ones.
4. **Things don't work** — Many roko subsystems are "built" but not verified working.

## What Was Good About Mori

- **Thoughtful TUI** — Every F1-F8 tab was hand-designed and verified working
- **Queue system** — Milestones, wave-based execution, clear progress tracking
- **Agent management** — ~30 specialized roles, clear role-to-model mapping
- **Git integration** — Branch/worktree view per plan, merge tracking
- **Configuration** — Backend defaults + per-role overrides, MCP config visible
- **Inspection** — AST index stats, tool/learning metrics, fixture tracking
- **UX flow** — The workflow from idea → plan → execute → verify was intuitive
- **Visual polish** — Progress bars, ETA, system metrics, color-coded status

## What Was Wrong With Mori

- Streaming/architecture issues that motivated the rewrite
- Composability problems — hard to extend or modify behavior
- Modularity gaps — tightly coupled components

## What Roko Should Be

The **better version of Mori** — keeping the UX/TUI quality, workflow intuitiveness,
and hand-verified reliability, while fixing the underlying architecture for:
- Better streaming
- More composable/modular design
- Extensible plugin/tool system
- The cybernetic features (affect engine, dreams, knowledge) that Mori didn't have

## Current State

The backlog at `tmp/archive/` and `tmp/backlog/` tracks known issues. The existing
`tmp/mori-diffs/` documents (00-33) already cover some architectural comparisons.
`.roko/GAPS.md` is the canonical gap tracker.

## Reference Locations

| What | Path |
|---|---|
| Mori source | `/Users/will/dev/uniswap/bardo/apps/mori/` |
| Mori TUI | `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/` |
| Bardo PRDs | `/Users/will/dev/uniswap/bardo/prd/` |
| Bardo crates | `/Users/will/dev/uniswap/bardo/crates/` |
| Bardo tmp docs | `/Users/will/dev/uniswap/bardo/tmp/` |
| Roko source | `/Users/will/dev/nunchi/roko/roko/` |
| Roko TUI | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/` |
| Roko mori-diffs | `/Users/will/dev/nunchi/roko/roko/tmp/mori-diffs/` |
| Roko backlog | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/` |
| Roko archive | `/Users/will/dev/nunchi/roko/roko/tmp/archive/` |
| Roko gaps | `/Users/will/dev/nunchi/roko/roko/.roko/GAPS.md` |

## End Goal

1. Make roko's TUI and UX match or exceed mori's quality
2. Ensure everything works end-to-end (no "built but not wired" features)
3. Add enough logs/diagnostics/endpoints that Claude can run and debug things
4. Create actionable backlog items and implementation plans from the analysis
5. Preserve roko's genuine architectural improvements while fixing what's broken
