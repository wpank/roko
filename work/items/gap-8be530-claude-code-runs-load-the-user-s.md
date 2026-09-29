+++
id = "gap-8be530"
kind = "gap"
title = "Claude Code runs load the user's own ~/.claude settings, hooks and plugins"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude-cli"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_command", "crates/roko-agent/src/claude_cli_agent.rs:370"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q -- \"setting-sources\" crates/roko-agent/src/claude_cli_agent.rs"
+++
roko passes `--settings` to the Claude CLI (`claude_cli_agent.rs:368`) but never `--setting-sources`, so the invoking user's global and project settings also apply: hooks, plugins, permissions and model defaults. The same plan can behave differently on different machines, and benchmark runs through the Claude CLI are not reproducible. The effect on runs has not been measured.

Fix: pass `--setting-sources` explicitly (or isolate the config directory), and record which settings a run used.
