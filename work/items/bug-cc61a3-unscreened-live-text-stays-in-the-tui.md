+++
id = "bug-cc61a3"
kind = "bug"
title = "Unscreened live text stays in the TUI next to the screened transcript: settle_screened_transcript has no caller"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-633184"
anchors = ["crates/roko-cli/src/tui/state/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-633184"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib screened_transcript_replaces_unscreened_text"
+++

## Problem

`TuiState::settle_screened_transcript` has no production caller. Under Trusted live output, the unscreened text an agent streamed stays in the TUI beside the screened copy, though only the screened copy should remain once it settles.

## Plan

Call it when an attempt's screened result lands, so the unscreened text is replaced. Add a test named `screened_transcript_replaces_unscreened_text`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-633184, during the evening close-out round.
