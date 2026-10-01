+++
id = "bug-6930e6"
kind = "bug"
title = "find_mcp_config hands Claude runs an ancestor directory's or $HOME's .mcp.json"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/mcp", "roko-agent/claude-cli"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "546d90ae1"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-cc-isolate's report on gap-8be530, branch work/gap-8be530)"
anchors = ["crates/roko-agent/src/mcp/config.rs::find_mcp_config", "crates/roko-agent/src/claude_cli_agent.rs::discovered_mcp_config"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-8be530", "gap-b7a2d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn discovered_mcp_config_ignores_ancestor_and_home_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib discovered_mcp_config_ignores_ancestor_and_home_files"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "workspace_mcp_config reads only <workdir>/.mcp.json for ClaudeCliAgent, serve template dispatch and validation, and ACP sessions; find_mcp_config's ancestor/HOME walk stays only for config mcp and doctor (5c3ee965c; merged 546d90ae1). Batch 10 gate (dedicated target dir; batch tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-learn -p roko-dreams -p roko-agent -p roko-serve -p roko-acp --no-deps -D warnings clean; lib tests roko-cli 3100, roko-agent 2256, roko-learn 1182, roko-serve 956, roko-acp 197, roko-dreams pass, 0 failed."
+++

## Problem

`find_mcp_config(start_dir)` walks from `start_dir` up to the filesystem root looking for `.mcp.json`, then falls back to `$HOME/.mcp.json`. `ClaudeCliAgent::discovered_mcp_config` calls it whenever no MCP config is set, and passes the result as `--mcp-config`. A task in a worktree under the user's home therefore gets the MCP servers of any `.mcp.json` in an ancestor directory or in `$HOME`, with their commands and env. `--strict-mcp-config` (gap-8be530) doesn't help: it stops Claude Code from finding servers itself, but here Roko hands them over.

## Why it matters

Secrets and guard (epic spec-ba7bea). MCP server entries often carry tokens in `env`, and the servers can act outside the workspace. A plan or benchmark run should see only the MCP servers its workspace declares.

## Where

- `crates/roko-agent/src/mcp/config.rs::find_mcp_config` (:84): the upward walk and the `$HOME` fallback.
- `crates/roko-agent/src/claude_cli_agent.rs::discovered_mcp_config` (:367).
- Other callers of the same walk: roko-serve `templates.rs:486`, `routes/templates.rs:229` and `dispatch.rs:2376`/`:2389`; roko-acp `session.rs:1322`; roko-cli `commands/mcp.rs:134` and `doctor.rs:1962`.

## Current state

Unchanged at BASE and on `work/gap-8be530`, whose notes list it as not covered. The walk is documented in `find_mcp_config`'s doc comment, so it is intended for discovery; handing its result to agent runs is the problem.

## Plan

1. Separate discovery from agent use. Agent spawns read only the workspace's own `.mcp.json` (its root, not ancestors) or an explicit `[agent] mcp_config`. The ancestor walk and the `$HOME` fallback stay for `roko config mcp list` and `roko doctor`, which report where they found the file.
2. Apply the same rule to the serve and ACP dispatch callers.
3. Log which MCP config a run used, next to its `setting_sources` tag.
4. Add `discovered_mcp_config_ignores_ancestor_and_home_files`: with a `.mcp.json` in a parent directory and in a fake `$HOME`, an agent in the child directory gets no `--mcp-config`.

## Done when

- [ ] An agent run gets MCP servers only from its workspace's own `.mcp.json` or from explicit config.
- [ ] The `[[verify]]` command passes.

## Notes

- If some users rely on `$HOME/.mcp.json` reaching agents, make that an explicit opt-in (`[agent] mcp_config = "~/.mcp.json"`).
- Implemented on `work/gap-b7a2d5` at `5c3ee965c`; cargo verification deferred to the batch check.
- `roko_agent::mcp::workspace_mcp_config` reads only `<workdir>/.mcp.json`. `ClaudeCliAgent::discovered_mcp_config`,
  roko-serve's `resolve_template_mcp_config` (`dispatch.rs`), `load_configured_mcp_servers` (`templates.rs`) and
  `configured_mcp_servers` (`routes/templates.rs`), and roko-acp's `resolve_mcp_config_path` use it.
  `find_mcp_config` keeps the walk and the `$HOME` fallback for `roko config mcp` and `roko doctor`.
- Graph plan tasks were not affected: they pass only `[agent] mcp_config` (`graph_task_dispatch.rs`).
- The test cannot point `$HOME` at a temporary directory (`set_var` is unsafe in edition 2024 and the workspace denies
  unsafe code), so it models home as a directory above the workdir; the new lookup never reads `$HOME`.
- Each Claude spawn logs its isolation tags with the MCP config it passes (debug).
- Not changed: `roko chat`'s own `resolve_mcp_config` still falls back to `.roko/mcp.json` and then
  `~/.claude/mcp-config.json`, a user-level file. `docs/v3/05-AGENT.md` §7 says so; whether chat should keep it is a
  separate decision.
