+++
id = "gap-b7a2d5"
kind = "gap"
title = "roko chat and dispatch_v2 spawn claude without the Claude Code isolation flags"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/chat", "roko-cli/dispatch-v2"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "546d90ae1"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-cc-isolate's report on gap-8be530, branch work/gap-8be530)"
anchors = ["crates/roko-cli/src/chat_session.rs::build_streaming_command", "crates/roko-cli/src/dispatch_v2.rs::build_claude_invocation", "crates/roko-agent/src/claude_cli_agent.rs::build_command"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["gap-8be530"], blocks = [], related = ["bug-76dc76", "bug-6930e6", "gap-a3fc5b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q -- 'setting-sources' crates/roko-cli/src/chat_session.rs && grep -q -- 'setting-sources' crates/roko-cli/src/dispatch_v2.rs && grep -rqw 'fn claude_spawns_carry_the_isolation_flags' crates/roko-cli/src/ && cargo test -p roko-cli --lib claude_spawns_carry_the_isolation_flags"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "New ClaudeIsolation (roko-agent) builds the isolation args, env and tags for ClaudeCliAgent, roko chat's streaming command and dispatch_v2's build_claude_invocation (flags after provider args, request env wins); test claude_spawns_carry_the_isolation_flags (bab3d07a9; merged 546d90ae1). Batch 10 gate (dedicated target dir; batch tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-learn -p roko-dreams -p roko-agent -p roko-serve -p roko-acp --no-deps -D warnings clean; lib tests roko-cli 3100, roko-agent 2256, roko-learn 1182, roko-serve 956, roko-acp 197, roko-dreams pass, 0 failed."
+++

## Problem

gap-8be530 (on `work/gap-8be530`) isolates every Claude Code run that `ClaudeCliAgent` starts: `--setting-sources` (an empty list by default), `--strict-mcp-config` on every run, and `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` (`ISOLATION_ENV`). Two places in roko-cli build their own `claude` command and get none of it:

- `chat_session.rs::build_streaming_command` (:1209), the `roko chat` streaming turn;
- `dispatch_v2.rs::CliProviderConfig::build_claude_invocation` (:513), the provider-neutral CLI dispatch.

Both pass `--settings`, add `--strict-mcp-config` only when an MCP config is given, never pass `--setting-sources`, and don't set the auto-memory variable. These runs still load the user's hooks, plugins, permission rules, CLAUDE.md files and auto-memory.

## Why it matters

Secrets and guard (epic spec-ba7bea): the user's hooks and settings change what an agent does and can see, and runs differ between machines. The isolation is only as good as its least isolated spawn.

## Where

- The two builders above.
- `crates/roko-agent/src/claude_cli_agent.rs`: `ISOLATION_ENV` and the `--setting-sources` / `--strict-mcp-config` block in `build_command` (about :461 on the branch).

## Current state

Checked on `work/gap-8be530` (`67e67766b`, not merged at BASE): only `claude_cli_agent.rs` and its tests mention `--setting-sources` or `CLAUDE_CODE_DISABLE_AUTO_MEMORY`. gap-8be530's notes list these two spawns as not covered.

## Plan

1. After gap-8be530 merges, expose the isolation as one roko-agent helper that returns the extra args and env pairs (honouring `with_setting_sources`), so every spawn shares it.
2. Call it from `build_streaming_command` and `build_claude_invocation`, and always pass `--strict-mcp-config`.
3. Record the `setting_sources` tag for these runs, as `ClaudeCliAgent` does.
4. Add `claude_spawns_carry_the_isolation_flags`, checking both builders' argv and env.

## Done when

- [ ] Both spawns pass `--setting-sources` and `--strict-mcp-config`, and set `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on gap-8be530, which is not merged at BASE.
- `roko chat` changes the way `ClaudeCliAgent` runs did: the user's CLAUDE.md and allow rules stop applying. Say so in the chat docs.
- bug-76dc76 (`roko chat` ignores `env_passthrough`) touches the same builder.
- Implemented on `work/gap-b7a2d5` at `bab3d07a9`, extended at `5c3ee965c` (MCP config logged) and `13f6c9578`
  (managed MCP); cargo verification deferred to the batch check.
- The shared helper is `roko_agent::claude_cli_agent::ClaudeIsolation` (`args`, `env`, `tags`). `ClaudeCliAgent`,
  `build_streaming_command` and `build_claude_invocation` all build from it. The dispatcher appends the flags after
  `provider_args`, and a variable the request sets itself beats the isolation's.
- The literal flags live in roko-agent. The two roko-cli files name them in comments, which is what the verify's grep
  finds; `claude_spawns_carry_the_isolation_flags` (roko-cli, `chat_session.rs`) is the real check, comparing both
  spawns' flag block with `ClaudeIsolation::args`.
- Recording: `ClaudeCliAgent` outputs carry the tags; the chat and dispatch spawns log them (debug) with the MCP config
  they pass. A Graph attempt keeps no durable record of them, because the invocation is not persisted.
- Docs: the "Isolation" row of `docs/v3/05-AGENT.md` §3.2 covers `roko chat` too; there is no separate chat page.
- The new test assumes the host has no managed `managed-mcp.json`, like the existing MCP tests in
  `provider/claude_cli.rs` and roko-cli's `tests/smoke.rs`.
