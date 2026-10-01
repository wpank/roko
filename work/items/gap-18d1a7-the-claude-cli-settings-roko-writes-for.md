+++
id = "gap-18d1a7"
kind = "gap"
title = "The Claude CLI settings roko writes for agents have no Read deny rules for key files, which Claude Code would turn into Grep exclusions"
status = "superseded"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1cc42be6d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-6af02b, branch work/guard-l9 at d1cc396c0)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-6af02b", "bug-fa1537"], supersedes = [], duplicate_of = "bug-a66941" }

[[verify]]
command = "grep -rqw 'fn agent_settings_deny_reading_key_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib agent_settings_deny_reading_key_files"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:45:14Z"
by = "coordinator (session 7622b882)"
size = "S"
claimed_at = "2026-10-01T08:40:03Z"
forced = false
evidence = "Premise false (wk-guard2, 2026-10-01): roko's --settings has denied Read/Edit of every key file since 9c0d7196b (bug-a66941): key_file_deny_rules in claude_cli_agent.rs emits Read(//**/.roko/<name>) for .env, secrets.toml, credentials.json and config.toml, pinned by settings_json_denies_reading_key_files. Claude Code 2.1.282 re-roots those rules at the searched dir and passes them to its Grep and Glob as --iglob exclusions."
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
