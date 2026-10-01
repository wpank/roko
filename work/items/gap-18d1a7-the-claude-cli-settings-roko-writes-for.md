+++
id = "gap-18d1a7"
kind = "gap"
title = "The Claude CLI settings roko writes for agents have no Read deny rules for key files, which Claude Code would turn into Grep exclusions"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-6af02b, branch work/guard-l9 at d1cc396c0)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-6af02b", "bug-fa1537"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_settings_deny_reading_key_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib agent_settings_deny_reading_key_files"
+++

## Problem

The `--settings` roko passes to Claude Code agents installs the guard hook but no permission rules. Claude Code turns `Read` deny rules into `--iglob` exclusions for its Grep tool (wk-guard2, read in the CLI), so deny rules for key files would make Grep skip them before the guard is ever consulted.

## Why it matters

Secrets and git guard (epic spec-ba7bea): defence in depth. The guard refuses the call after the fact; deny rules keep key files out of searches in the first place, including tools the guard does not model.

## Plan

1. Add `permissions.deny` entries for `Read(...)` of the key files (`.roko/.env`, `~/.roko/.env` and the other files the guard treats as keys) to the settings roko writes.
2. Add `agent_settings_deny_reading_key_files`: the generated settings JSON carries the deny rules.

## Done when

- [ ] Agent settings deny reading every key file the guard knows.
- [ ] The `[[verify]]` command passes.
