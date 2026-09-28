+++
id = "gap-594d3d"
kind = "gap"
title = "Per-Worktree MCP Config"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/orchestrator"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/187-per-worktree-mcp-config.md#187 — Per-Worktree MCP Config"
discovered_from = "audit:tmp/backlog/archive/187-per-worktree-mcp-config.md#187 — Per-Worktree MCP Config"
anchors = ["crates/roko-cli/src/orchestrator/worktree.rs", "crates/roko-cli/src/dispatch_v2.rs", "target/release/roko-mcp-code", "target/debug/roko-mcp-code", ".roko/mcp.json", "crates/roko-mcp-code/src/main.rs", "crates/roko-cli/src/runner/event_loop.rs", "WorktreeManager::create()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
correctness; agents in worktree X currently see symbols from the main checkout, not their branch. When the runner creates a worktree for a plan attempt (`WorktreeManager::create()` in `crates/roko-cli/src/orchestrator/worktree.rs`), agents dispatched into that worktree inherit the global MCP…

Imported without verification from:
- `tmp/backlog/archive/187-per-worktree-mcp-config.md#187 — Per-Worktree MCP Config`

Some cited files are gone: `crates/roko-cli/src/orchestrator/worktree.rs`, `crates/roko-cli/src/runner/event_loop.rs`, `target/debug/roko-mcp-code`, `target/release/roko-mcp-code`.

How to verify: Check: `WorktreeManager::create()` writes `.roko/mcp.json` inside the new worktree; The MCP config points `roko-mcp-code` at the worktree path via `--root` and `ROKO_WORKSPACE_ROOT`; Binary resolution tries `target/release`, then `target/debug`… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
