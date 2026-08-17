# tmp/notes/ -- What is this?

> **Status**: STALE -- historical design notes from May 2026
> **Last updated**: 2026-08-13

This directory contains short design notes and sketches from the May 2026 development
sprint. All items described here have since been **implemented and wired** into the
runtime. These notes are preserved for historical context only.

## Contents

| File | Topic | Current status |
|---|---|---|
| `cascade-router-design.md` | Model routing tiers + learning | DONE -- `CascadeRouter` in `roko-learn`, persists to `.roko/learn/cascade-router.json` |
| `dispatch-benchmarks.md` | CLI vs API dispatch latency | DONE -- hybrid approach implemented (CLI for tasks, API for chat) |
| `episode-logger-design.md` | Agent turn recording | DONE -- `EpisodeLogger` wired in runner/, logs to `.roko/episodes.jsonl` |
| `gate-pipeline-notes.md` | 7-rung gate pipeline analysis | DONE -- adaptive thresholds (EMA) wired in `runner/gate_dispatch.rs` |
| `mcp-patterns.md` | MCP transport patterns | DONE -- stdio for local, `agent.mcp_config` passthrough in `roko.toml` |
| `project-bootstrap.md` | Initial workspace setup decisions | Historical -- describes April 2026 bootstrap |
| `prompt-assembly-profile.md` | 9-layer SystemPromptBuilder profiling | DONE -- `RoleSystemPromptSpec` wired with all 9 layers |
| `research-plan.md` | May 2026 research sprint goals | DONE -- all goals achieved |
| `self-hosting-roadmap.md` | Self-hosting gap analysis | DONE -- all 5 listed gaps have been closed (see CLAUDE.md "What to work on") |
| `tui-design-sketch.md` | TUI tab layout (F1-F7) | DONE -- ratatui TUI wired, `roko dashboard` |
| `week-review-may-22.md` | Week of May 18-22 review | Historical -- all "next week" items completed |

## Naming note

These notes use "Signal" throughout, consistent with the current spec-level naming.
The Rust struct is `Engram` with `type Signal = Engram;` as a bridge (see `roko-core::engram`).
