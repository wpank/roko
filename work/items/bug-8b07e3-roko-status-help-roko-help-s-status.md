+++
id = "bug-8b07e3"
kind = "bug"
title = "roko status --help, roko --help's status line and roko plan status --help still describe the old views"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "befee2f9b"
source = "docs sweep on branch work/readme-v8, checked with roko at befee2f9b (2026-11-19)"
anchors = ["crates/roko-cli/src/main.rs:652", "crates/roko-cli/src/commands/plan.rs:501"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Print signal counts, most recent episode, and gate pass/fail' crates/roko-cli/src/main.rs && ! grep -q 'lightweight runner status from' crates/roko-cli/src/commands/plan.rs && cargo run -q -p roko-cli -- status --help 2>/dev/null | sed -n 1p | grep -qi 'run' && cargo run -q -p roko-cli -- plan status --help 2>/dev/null | sed -n 1p | grep -qi 'last run'"
+++

## Problem

`roko status --help`, and the `status` line of `roko --help`, read "Print signal counts, most recent episode, and gate
pass/fail". `roko status` no longer prints that. It prints what is running, how the last runs ended and what they
spent ("13 plans · $0.00 today · $12.66 this month", then a line per recent plan), and signal counts only under
`--details`. The command's own examples, in its `after_help`, already describe the new view, so the help contradicts
itself.

`roko plan status --help` reads "Show the lightweight runner status from `.roko/state/status.json`. Reads the < 500
byte status file written by the runner on every tick…", with the examples "Show global runner status" and "Show task
status for a specific plan". The command prints a plan's last run in the run view's style instead:

- the four readings (tasks, time, spend against the cap, checks);
- each task as the run left it;
- the plan check;
- where the work went;
- the next command.

## Why it matters

Help is where a new user reads what a command does. README.md and docs/v2/CLI-REFERENCE.md now describe both views as
they print (branch work/readme-v8), so the binary's help is the one place that still describes the old ones.

## Where

- `crates/roko-cli/src/main.rs:652`: the doc comment on `Commands::Status`. It is clap's about text for `roko status`
  and its line in `roko --help`.
- `crates/roko-cli/src/commands/plan.rs:501`: the doc comment and `after_help` examples on `PlanCmd::Status`.

## Current state

Seen with the binary at befee2f9b on 2026-11-19:

- `roko status --help` starts "Print signal counts, most recent episode, and gate pass/fail";
- `roko plan status --help` starts "Show the lightweight runner status from `.roko/state/status.json`.";
- both views print the new format.

## Plan

1. Rewrite `Commands::Status`'s doc comment as one line on what it prints, with `--details` for roko's internals.
2. Rewrite `PlanCmd::Status`'s doc comment and examples: a plan's last run in the run view's style, from any terminal,
   while it runs or after. With no plan directory, say what it prints, after checking the code path.
3. Check that `roko --help`'s listing reads well.

## Done when

The `[[verify]]` command passes: neither stale sentence is left in the source, and the first help line of each command
describes the view it prints.

## Notes

Text only, with no change in behaviour.
