# Self-Hosting Roadmap (drafted 2026-05-18)

> **STALE**: All 5 gaps listed below were closed by June 2026. See CLAUDE.md
> "What to work on" for current status. Last updated: 2026-08-13

## What "self-hosting" means
Roko reads a PRD, generates an implementation plan, executes it via agents,
validates results through gates, learns from outcomes, and iterates.

## ~~Remaining gaps~~ (ALL RESOLVED)
1. ~~SystemPromptBuilder — built but not wired into orchestrate.rs~~ DONE: `RoleSystemPromptSpec` uses 9-layer builder
2. ~~EpisodeLogger — built but not recording agent turns~~ DONE: wired in runner/, logs to `.roko/episodes.jsonl`
3. ~~ProcessSupervisor — built but not tracking agent lifecycle~~ DONE: `PlanRunner` tracks via `roko-runtime`
4. ~~MCP passthrough — config exists but not passed to agents~~ DONE: `agent.mcp_config` in `roko.toml` + auto-discovery
5. ~~Learning feedback — efficiency events not emitted~~ DONE: efficiency events, cascade router, prompt experiments, adaptive gates

## Priority order
Wire existing code > build new code > optimize
