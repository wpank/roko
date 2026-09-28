+++
id = "find-d1a17e"
kind = "finding"
title = "[provider F122] ClaudeCli dangerously_skip_permissions defaults to true"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/claude_cli_agent"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F122"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F122"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "ClaudeCli", "dangerously_skip_permissions", "true"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`ClaudeCliAgent::new()` sets `dangerously_skip_permissions: true` by default. All Claude CLI sessions run with permissions automatically accepted, bypassing Claude Code's interactive permission prompt.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F122`
- `tmp/archive/provider-audit/03-cli-subprocess.md`

How to verify: Confirm in crates/roko-agent/src/claude_cli_agent.rs whether still true: `ClaudeCli` `dangerously_skip_permissions` defaults to `true`
