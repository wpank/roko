+++
id = "bug-a9a251"
kind = "bug"
title = "roko chat's resolve_mcp_config still falls back to ~/.claude/mcp-config.json"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/chat"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-cc-isolate's report on gap-b7a2d5, branch work/gap-b7a2d5 at 42859fc78)"
anchors = ["crates/roko-cli/src/chat_session.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["gap-b7a2d5"], blocks = [], related = ["gap-b7a2d5", "gap-8be530"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '.claude/mcp-config.json' crates/roko-cli/src/chat_session.rs"
+++

## Problem

gap-b7a2d5 (branch `work/gap-b7a2d5`, not merged at d2cc43346) gives `roko chat` and `dispatch_v2` the Claude Code isolation flags. `roko chat`'s own MCP lookup, `resolve_mcp_config` (`crates/roko-cli/src/chat_session.rs:1903` on the branch), still falls back to the user's global `~/.claude/mcp-config.json` (:1900, :1927). So a chat session can load the user's personal MCP servers, which the isolation exists to keep out.

## Why it matters

Secrets and guard (epic spec-ba7bea): the user's MCP servers can hold credentials and reach outside services. Isolated runs must not pick them up by fallback.

## Where

`resolve_mcp_config` in `chat_session.rs`.

## Plan

1. Drop the `~/.claude` fallback. Use the project's MCP config or roko's own, and nothing from the user's Claude home.
2. Add a test that a chat session with a `~/.claude/mcp-config.json` present doesn't load it.

## Done when

- [ ] `roko chat` never reads `~/.claude/mcp-config.json`.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-b7a2d5's branch.
