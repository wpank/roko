+++
id = "bug-8589fc"
kind = "bug"
title = "roko dream run fails with run's missing-prompt error instead of the unknown-command message"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-childenv's report, checked on work/bug-17f0e4 at 6b332b25f)"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-17f0e4"], blocks = [], related = ["bug-17f0e4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dream_run_reports_an_unknown_command' crates/roko-cli/src/ && cargo test -p roko-cli dream_run_reports_an_unknown_command"
+++

## Problem

bug-17f0e4's branch makes unknown subcommands fail instead of running them as agent prompts (`#[command(external_subcommand)]`, `main.rs:2349` on the branch). But `roko dream run` still fails with `run`'s missing-prompt error. Its test only asserts that parsing fails (`main.rs:4963`), not which message the user sees. `dream` isn't a command (the command is `roko knowledge dream run`), so the user should be told that, and pointed to the right one.

## Why it matters

A misleading error for a plausible typo. p3.

## Where

The external-subcommand handling in `main.rs`.

## Plan

1. Report an unknown first word as an unknown command, whatever follows it, with a "did you mean `roko knowledge dream`?" suggestion.
2. Add `dream_run_reports_an_unknown_command`, which checks the message.

## Done when

- [ ] `roko dream run` says the command is unknown and suggests the right one.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-17f0e4's branch.
