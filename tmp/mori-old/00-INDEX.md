# Mori → Roko Analysis Index

> Generated 2026-08-19 from 17 parallel deep-dive agents.

## Context & Reference
| File | Description |
|---|---|
| [CONTEXT.md](CONTEXT.md) | Will's goals, intent, and reference locations |
| [MORI-TUI-SCREENSHOTS.md](MORI-TUI-SCREENSHOTS.md) | Detailed descriptions of all 10 mori TUI screenshots |

## Comparison Documents
| # | File | Domain |
|---|---|---|
| 01 | [01-MORI-TUI-ARCHITECTURE.md](01-MORI-TUI-ARCHITECTURE.md) | Mori TUI: 47 files, ROSEDUST palette, VFX system, adaptive framerate |
| 02 | [02-ROKO-TUI-ARCHITECTURE.md](02-ROKO-TUI-ARCHITECTURE.md) | Roko TUI: 78 files, 44K LOC, app.rs god object, two parallel data models |
| 03 | [03-EXECUTION-MODEL-COMPARISON.md](03-EXECUTION-MODEL-COMPARISON.md) | Queue/wave/milestone vs runner-v2; 12 recommendations |
| 04 | [04-AGENT-SYSTEM-COMPARISON.md](04-AGENT-SYSTEM-COMPARISON.md) | Role dispatch, 28 roles shared, preset layer design |
| 05 | [05-MORI-WORKFLOW-UX.md](05-MORI-WORKFLOW-UX.md) | Mori's 9 UX patterns, mori.sh bootstrap, 6-layer config |
| 06 | [06-ROKO-E2E-WIRING-AUDIT.md](06-ROKO-E2E-WIRING-AUDIT.md) | Core CLI commands are genuinely wired end-to-end |
| 07 | [07-GIT-WORKTREE-COMPARISON.md](07-GIT-WORKTREE-COMPARISON.md) | Roko's immutable-tip model is more robust; mori's TUI richer |
| 08 | [08-MCP-TOOL-COMPARISON.md](08-MCP-TOOL-COMPARISON.md) | Mori auto-generates per-worktree MCP configs; roko has richer safety |
| 09 | [09-LEARNING-METRICS-COMPARISON.md](09-LEARNING-METRICS-COMPARISON.md) | Roko 21x larger learning system; mori had production proof + single-pane visibility |
| 10 | [10-CONFIG-SYSTEM-COMPARISON.md](10-CONFIG-SYSTEM-COMPARISON.md) | Mori's flat 59-field config vs roko's nested 30+ sections |
| 11 | [11-CYBERNETIC-FEATURES-AUDIT.md](11-CYBERNETIC-FEATURES-AUDIT.md) | 4 WORKING, 8 PARTIAL; runner only absorbs dispatch-modulating features |
| 12 | [12-EXISTING-ANALYSIS-META.md](12-EXISTING-ANALYSIS-META.md) | Synthesizes 42 mori-diffs + GAPS.md + backlog; 5 recurring patterns |
| 13 | [13-CRATE-ARCHITECTURE-COMPARISON.md](13-CRATE-ARCHITECTURE-COMPARISON.md) | Roko is 2.65x bardo (893K vs 337K LOC); merged two parallel stacks |
| 14 | [14-STATE-PERSISTENCE-COMPARISON.md](14-STATE-PERSISTENCE-COMPARISON.md) | Checksummed snapshots, fingerprinted resume, no SQLite dependency |
| 15 | [15-PROMPT-CONTEXT-COMPARISON.md](15-PROMPT-CONTEXT-COMPARISON.md) | 9-layer builder with bidder auctions vs mori's monolithic but practical assembly |
| 16 | [16-ROKO-HTTP-ROUTES-AUDIT.md](16-ROKO-HTTP-ROUTES-AUDIT.md) | ~365 real routes, ~97% serve real data, SSE/WS fully functional |
| 17 | [17-ERROR-DIAGNOSTICS-COMPARISON.md](17-ERROR-DIAGNOSTICS-COMPARISON.md) | Mori's 5 recovery keybindings + LLM reflections vs roko's read-only dashboard |

## Synthesis
| File | Description |
|---|---|
| [SYNTHESIS.md](SYNTHESIS.md) | Master findings, priorities, and recommended action plan |
