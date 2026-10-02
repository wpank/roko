+++
id = "bug-9affca"
kind = "bug"
title = "Under trusted live output, unscreened tool starts and results stay beside their screened copies"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-cc61a3"
anchors = ["crates/roko-cli/src/tui/state/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-cc61a3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib screened_tool_steps_replace_unscreened"
+++

## Problem

bug-cc61a3 settles unscreened assistant text when the screened transcript arrives. Unscreened tool starts and results (raw arguments and output) still stay in the TUI beside their screened copies.

## Plan

Settle tool steps the same way. Add a test named `screened_tool_steps_replace_unscreened`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-streams, working on bug-cc61a3, during the overnight close-out round.
