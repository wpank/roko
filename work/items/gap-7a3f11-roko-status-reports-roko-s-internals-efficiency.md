+++
id = "gap-7a3f11"
kind = "gap"
title = "roko status reports roko's internals (efficiency events, gate thresholds, signal counts) instead of what is running, how the last runs ended, what today cost and what to do next"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/commands/util.rs", "crates/roko-cli/src/commands/status.rs"]
links = { depends_on = ["gap-cf3940"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn status_says_what_runs_and_how_the_last_runs_ended' crates/roko-cli/src && cargo test -p roko-cli --lib status_says_what_runs_and_how_the_last_runs_ended"
+++

## Problem

`roko status` reports roko's internals instead of the workspace's state. After three runs in round v8's capture
workspace it printed:
- "activity: 10 efficiency event(s) recorded (signal substrate empty)";
- "workspace: 0 agent pid(s), no run state found";
- "most recent episode: ep_… (passed=false)";
- 10 efficiency events by role, and 7 adaptive gate thresholds;
- a cost summary by model and plan, then "health: ready".

It never says whether anything is running, how the last runs ended, or what to do next. Capture:
`tmp/hermes-roko-v8/design/terminal/before/status.*`.

## Why it matters

Goal tooling. `roko status` is the first command a person tries in a workspace. Round v8 redesigns roko's terminal
output (`tmp/hermes-roko-v8/design/terminal/README.md`).

## Where

`crates/roko-cli/src/commands/util.rs` (`cmd_status`, re-exported by `commands/status.rs`). It reads the run
records (`.roko/runs/*/manifest.json`), the Graph checkpoints (`.roko/state/graph/*/checkpoint.json`) and the cost
ledger.

## Plan

1. The default text, in item 2's style:
   - the header, with today's spend;
   - Running: a live run, its plan and progress;
   - the last runs: each one's state, age, tasks done and spend, and whether its work is merged;
   - plans: a count, and any with problems;
   - spend today and this month, by model;
   - one Next line, a command the person can paste.
2. `--verbose` prints today's internals sections.
3. `--json` and `--quick` stay unchanged.
4. Update CLAUDE.md's CLI table and `docs/v3/28-CLI.md`.

## Done when

- `roko status` in a workspace with a passed, a failed and a one-task run shows those runs with a Next line.
- The exact-text tests pass, and `cli_fallback.rs` and `e2e.rs`'s status assertions are updated.
- The `[[verify]]` passes.
