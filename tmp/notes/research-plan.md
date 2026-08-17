# Research Sprint -- May 2026

> **STALE**: All research goals achieved and findings incorporated into implementation.
> Last updated: 2026-08-13

## Goals (ALL ACHIEVED)
- ~~Evaluate agent dispatch models (CLI vs API vs hybrid)~~ DONE: hybrid implemented
- ~~Benchmark prompt assembly overhead~~ DONE: see `prompt-assembly-profile.md`
- ~~Profile gate pipeline latency~~ DONE: see `gate-pipeline-notes.md`
- ~~Research MCP server patterns for code intelligence~~ DONE: `roko-mcp-code` built

## Findings
- CLI dispatch adds ~200ms overhead per invocation but simplifies auth
- API dispatch requires token management but enables streaming
- Hybrid approach: CLI for long tasks, API for short queries
- MCP stdio transport is most reliable for local tools
