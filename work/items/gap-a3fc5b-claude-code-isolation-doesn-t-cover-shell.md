+++
id = "gap-a3fc5b"
kind = "gap"
title = "Claude Code isolation doesn't cover shell snapshots, and a managed-mcp.json makes Claude refuse --strict-mcp-config"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude-cli"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-cc-isolate's report on gap-8be530, branch work/gap-8be530)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_command"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["gap-8be530"], blocks = [], related = ["gap-c4f364", "gap-b7a2d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_managed_mcp_config_is_reported_before_the_run' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_managed_mcp_config_is_reported_before_the_run"
+++

## Problem

gap-8be530's isolation (`--setting-sources`, `--strict-mcp-config`, `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`) leaves two gaps:

1. **Shell snapshots.** Claude Code's Bash tool sources a snapshot of the user's shell from `~/.claude/shell-snapshots/`: aliases, functions and exported variables. Setting sources don't cover it, so an agent's shell commands run with the user's shell setup. (This machine has 40 snapshot files.)
2. **Managed MCP.** When an administrator installs a managed `managed-mcp.json`, Claude Code refuses `--strict-mcp-config`, so every isolated run fails on that machine.

Both are listed under "Not covered" in gap-8be530's notes on `work/gap-8be530`, from a read of Claude Code 2.1.282's help and bundled source.

## Why it matters

Secrets and guard (epic spec-ba7bea), and reproducible benchmark runs: the ViabilityBench Claude Code arm (gap-c4f364) reuses this isolation. Exported variables in a shell snapshot can include tokens.

## Where

`crates/roko-agent/src/claude_cli_agent.rs::build_command`, where gap-8be530 adds the flags and env.

## Current state

No code in `crates/` mentions shell snapshots or a managed MCP config (checked at BASE).

## Plan

1. Probe Claude Code for a supported way to turn off or redirect shell snapshots (an env variable, or a setting that `--settings` can carry). If there is one, add it to the isolation. If not, record `shell_snapshot = "user"` in the run's isolation record and document it.
2. Before spawning, look for a managed MCP config in Claude Code's managed-settings location. Then either drop `--strict-mcp-config` and record that the run's MCP set is managed, or fail with a clear message. Choose one and document it.
3. Add `a_managed_mcp_config_is_reported_before_the_run`, with a temporary directory standing in for the managed location.

## Done when

- [ ] Shell snapshots are isolated, or recorded for each run.
- [ ] A machine with a managed MCP config gets a clear outcome instead of a failed spawn.
- [ ] The `[[verify]]` command passes.

## Notes

- Confirm both behaviours on the Claude Code version the benchmark pins before building on them.
