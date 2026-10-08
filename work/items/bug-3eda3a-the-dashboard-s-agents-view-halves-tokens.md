+++
id = "bug-3eda3a"
kind = "bug"
title = "The dashboard's Agents view halves tokens_used instead of showing the real input/output split"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-1"
anchors = ["crates/roko-cli/src/tui/dashboard_types.rs::build_agent_activity_snapshot", "crates/roko-cli/src/tui/views/agents_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_rows_keep_the_real_token_split' crates/roko-cli/src/tui/ && cargo test -p roko-cli --lib agent_rows_keep_the_real_token_split"
+++

## Problem

The dashboard's Agents view showed each agent's input and output tokens as `tokens_used / 2` and the remainder
(`crates/roko-cli/src/tui/views/agents_view.rs`), although every efficiency event carries the real split. Input and
output tokens are priced differently, so the view's per-agent cost estimate was skewed too.

## Why it matters

The Agents view is where an operator checks what each agent spends; halving hides input-heavy agents.

## Where

`crates/roko-cli/src/tui/dashboard_types.rs::build_agent_activity_snapshot` (aggregates efficiency events into
`AgentActivityRow`) and `crates/roko-cli/src/tui/views/agents_view.rs::render_agent_roster`.

## Current state

The fix sat uncommitted in `archive/stash-2026-09-21-main-1` (triage: `salvage-05-tui-agent-token-split.patch`).

## Plan

Accumulate `input_tokens` and `output_tokens` on the activity row and show them; halve only for a row with no split.

## Done when

- [ ] The activity row carries the summed input and output tokens of the agent's events.
- [ ] `[[verify]]` passes.
