+++
id = "gap-751ac9"
kind = "gap"
title = "Graph attempts keep no durable record of their Claude Code isolation settings; the invocation is only debug-logged"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/claude_cli"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "e4771e454"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-cc-isolate's report on gap-b7a2d5, branch work/gap-b7a2d5 at 42859fc78)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-b7a2d5"], blocks = [], related = ["gap-b7a2d5", "bug-31438d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_records_carry_the_claude_isolation_settings' crates/roko-cli/src/ && cargo test -p roko-cli --lib attempt_records_carry_the_claude_isolation_settings"
+++

## Problem

On gap-b7a2d5's branch, every Roko spawn of `claude` builds its isolation from `ClaudeIsolation` and `ISOLATION_ENV` (`crates/roko-agent/src/claude_cli_agent.rs:121-175`): setting sources, strict MCP config, auto-memory off, and so on. The invocation is only debug-logged. No attempt, verdict or episode record says which isolation an attempt ran under.

## Why it matters

One settled record per attempt (epic spec-b7303f): isolation is part of the conditions of an attempt. Without it in the record, an audit can't show that a run was isolated, and a regression that drops a flag leaves no trace.

## Where

`ClaudeIsolation` in `claude_cli_agent.rs`, and the attempt record the Graph path writes (`graph_task_dispatch/attempt.rs`).

## Plan

1. Carry isolation tags (setting sources, MCP config mode, memory off, config dir) from the spawn to the attempt record, and on to the verdict.
2. Add `attempt_records_carry_the_claude_isolation_settings`.

## Done when

- [ ] Each Claude CLI attempt's record states its isolation settings.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-b7a2d5's branch.
- Implemented on `work/bug-739dcc` at `e4771e454`; cargo verification deferred to the batch check. `attempt_records_carry_the_claude_isolation_settings` (targeted `cargo test` passed). `ClaudeIsolation::tags()` adds `auto_memory` (off while `ISOLATION_ENV` switches it off) and `config_dir` (`user`), and `TAG_KEYS` lists all five. The verdict gains `isolation`, the map of those tags the attempt's output carried. Not done: episodes do not carry them, and `config_dir` says `user` even if a caller passes `CLAUDE_CONFIG_DIR` through the agent's env.
