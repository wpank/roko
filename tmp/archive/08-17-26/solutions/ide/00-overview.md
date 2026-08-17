# IDE Integration — Issues & Solutions Overview

## Context

The Nunchi Demo IDE (Tauri 2 + React 19) consumes `roko acp` as a stdio subprocess.
The IDE spawns `roko acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml` and
communicates via JSON-RPC 2.0 over stdin/stdout.

This folder documents **bugs and design issues in the roko repo** discovered while
running the IDE integration, along with proposed fixes. The IDE itself is NOT being
modified — all changes target `roko` crates.

## Test Environment

- roko 0.1.0 (rustc 1.95.0, aarch64-apple-darwin, git 042c26eae)
- Config: `~/.nunchi/roko/roko.toml` (config_version = 2)
- Providers tested: openai (gpt-4o), openai (gpt-4o-mini), claude_cli (sonnet), zai, moonshot
- MCP server: nunchi-mcp (stdio transport, bridges to localhost HTTP)
- All tests run via FIFO-based harness against `roko acp`

## Summary of Issues

| # | Issue | Severity | File |
|---|-------|----------|------|
| 1 | MCP spawn failures are silent — no error to client | High | `01-mcp-error-propagation.md` |
| 2 | `session/new` ignores `model` param (not in struct) | High | `02-model-param-in-session-new.md` |
| 3 | Model/provider defaults use HashMap ordering (non-deterministic) | Medium | `03-deterministic-defaults.md` |
| 4 | MCP discovery timeout not configurable, no eager option | Medium | `04-mcp-configuration.md` |
| 5 | `bare_mode` still exposes 50+ workspace commands | Low | `10-bare-mode-commands.md` |
| 6 | `max_output` default (900) too low for IDE agents | Medium | `11-max-output-default.md` |
| 7 | Provider readiness is informational only, no structured check | Low | `09-provider-readiness.md` |

## Document Index

| File | Contents |
|------|----------|
| `00-overview.md` | This file |
| `01-mcp-error-propagation.md` | MCP failures are silent — proposed structured error reporting |
| `02-model-param-in-session-new.md` | session/new drops model param — proposed fix |
| `03-deterministic-defaults.md` | HashMap ordering → IndexMap for deterministic defaults |
| `04-mcp-configuration.md` | MCP timeout, eager discovery, status query |
| `05-ide-consumer-guide.md` | Full ACP protocol reference for IDE developers |
| `06-streaming-protocol.md` | Streaming message format, parsing strategy |
| `07-config-for-ide.md` | Config layering design (profiles, merge-global) |
| `08-test-harness.md` | Reproducible FIFO-based test scripts |
| `09-provider-readiness.md` | Structured provider health reporting |
| `10-bare-mode-commands.md` | Command filtering for bare_mode |
| `11-max-output-default.md` | Default token limit too low |
| `12-test-results.md` | Full test log with all results |

## What Works Well (confirmed by testing)

- Basic ACP flow: session/new -> session/prompt -> streaming chunks -> final result
- Multiple concurrent sessions on same ACP process
- Wrong session ID returns proper JSON-RPC error (-32000)
- Wrong method name returns proper error (-32601)
- MCP tool calls work correctly when binary path is valid
- Bridge tool calls (tiles.list, tiles.create) work end-to-end
- claude_cli provider works (even nested inside another Claude session)
- Clean disconnect: process exits on stdin close, no resource leak
- session/update notifications deliver config options, commands, usage updates
- nunchi-mcp responds correctly to MCP initialize protocol

## Recommended Fix Priority

1. **Issue #02** (model param) — Most impactful for IDE. Simple fix, no breaking changes.
2. **Issue #01** (MCP errors) — Critical for UX. Requires new notification type.
3. **Issue #11** (max_output) — Easy fix, high impact. Just change the default.
4. **Issue #03** (HashMap ordering) — One-line dep add + type change.
5. **Issue #04** (MCP config) — Nice-to-have, can wait.
6. **Issue #10** (bare_mode) — Token waste, low urgency.
7. **Issue #09** (provider check) — Future feature.
