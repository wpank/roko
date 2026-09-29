+++
id = "bug-6930e6"
kind = "bug"
title = "find_mcp_config hands Claude runs an ancestor directory's or $HOME's .mcp.json"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/mcp", "roko-agent/claude-cli"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-cc-isolate's report on gap-8be530, branch work/gap-8be530)"
anchors = ["crates/roko-agent/src/mcp/config.rs::find_mcp_config", "crates/roko-agent/src/claude_cli_agent.rs::discovered_mcp_config"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-8be530", "gap-b7a2d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn discovered_mcp_config_ignores_ancestor_and_home_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib discovered_mcp_config_ignores_ancestor_and_home_files"
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
