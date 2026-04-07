# Research Sprint — May 2026

## Goals
- Evaluate agent dispatch models (CLI vs API vs hybrid)
- Benchmark prompt assembly overhead
- Profile gate pipeline latency
- Research MCP server patterns for code intelligence

## Findings
- CLI dispatch adds ~200ms overhead per invocation but simplifies auth
- API dispatch requires token management but enables streaming
- Hybrid approach: CLI for long tasks, API for short queries
- MCP stdio transport is most reliable for local tools
