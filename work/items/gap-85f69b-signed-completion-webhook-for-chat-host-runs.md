+++
id = "gap-85f69b"
kind = "gap"
title = "Signed completion webhook for chat-host runs (9118), after Will confirms decision 9106's two facts"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
hold = "decision 9106: Will confirms that OpenClaw is an MCP client and that Hermes can post to live sessions"
subsystem = ["roko-serve/mcp"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 9118 (blocked in wave 5, PK74)"
anchors = ["crates/roko-serve/src/routes/mcp.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_completion_is_posted_to_the_host_once' crates/roko-serve/ && cargo test -p roko-serve run_completion_is_posted_to_the_host_once"
+++

## Problem

PK74's task 9118 (a signed completion webhook: when a chat host's run finishes, roko posts the run summary to the
host) was not built. Decision 9106 says MCP first, and to confirm before 9118 that OpenClaw is an MCP client and that
Hermes can post to live sessions. Neither fact is recorded.

## Why it matters

Without it a chat host learns a run finished only by polling `run_status` (9114's 30 s wait).

## Where

`crates/roko-serve/src/routes/mcp.rs` (the /mcp run tools from 9115), the backlog task file
`tmp/backlog/2026-10-02-complete-and-wire/9118-*.md`.

## Current state

/mcp has run_prompt, plan_generate, plan_run, run_cancel and run_status (wave 5, 2347ad858). No completion push.

## Plan

1. Will confirms the two facts of decision 9106.
2. Then build 9118 as its task file says, with the verify below.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Filed at gate 5b, 2026-10-03, from PK74 (gap-ce1d11).
