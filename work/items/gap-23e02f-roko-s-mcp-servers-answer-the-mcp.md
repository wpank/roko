+++
id = "gap-23e02f"
kind = "gap"
title = "roko's MCP servers answer the MCP ping request with method-not-found"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-mcp-stdio"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-mcp-stdio/src/lib.rs::serve_stdio"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn serve_stdio_answers_ping_without_the_handler' crates/roko-mcp-stdio/src/lib.rs && cargo test -p roko-mcp-stdio serve_stdio_answers_ping_without_the_handler"
+++

## Problem

roko's MCP servers (roko-mcp-code, roko-mcp-github) answered the MCP `ping` request with `-32601 method not
found`. The MCP spec requires a receiver to answer `ping` promptly with an empty result; some editors send it as a
keep-alive and drop a server that fails it.

## Why it matters

Editor integrations (Cursor, Claude Desktop) keep MCP servers alive with `ping`.

## Where

`crates/roko-mcp-stdio/src/lib.rs::serve_stdio`, the JSON-RPC loop both servers run on.

## Current state

The 2026-09-21 stash `archive/stash-2026-09-21-main-2` added a `"ping"` arm to roko-mcp-code only (triage:
`salvage-07-mcp-code-ping.patch`).

## Plan

Answer `ping` in `serve_stdio` itself, before the server's handler, so every server built on it supports it.

## Done when

- [ ] `{"jsonrpc":"2.0","method":"ping","id":N}` gets `{"jsonrpc":"2.0","result":{},"id":N}` from both servers.
- [ ] `[[verify]]` passes.
