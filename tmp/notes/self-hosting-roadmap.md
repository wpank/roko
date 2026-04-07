# Self-Hosting Roadmap (drafted 2026-05-18)

## What "self-hosting" means
Roko reads a PRD, generates an implementation plan, executes it via agents,
validates results through gates, learns from outcomes, and iterates.

## Remaining gaps
1. SystemPromptBuilder — built but not wired into orchestrate.rs
2. EpisodeLogger — built but not recording agent turns
3. ProcessSupervisor — built but not tracking agent lifecycle
4. MCP passthrough — config exists but not passed to agents
5. Learning feedback — efficiency events not emitted

## Priority order
Wire existing code > build new code > optimize
