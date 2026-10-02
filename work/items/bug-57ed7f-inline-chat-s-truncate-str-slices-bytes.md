+++
id = "bug-57ed7f"
kind = "bug"
title = "Inline chat's truncate_str slices bytes and its render subtracts from a narrow width, both panicking"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/chat_inline"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d3c72e"
anchors = ["crates/roko-cli/src/chat_inline/session.rs", "crates/roko-cli/src/chat_inline/render.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-d3c72e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib truncate_str_cuts_at_a_char_boundary"
+++

## Problem

`chat_inline/session.rs::truncate_str` does `&s[..max - 3]`, which panics when the cut falls inside a multi-byte character. It truncates error messages in `chat_inline/render.rs:50`, which also computes `area.width as usize - 6` and underflows when the area is narrower than 6.

## Plan

Cut at a char boundary, and saturate the width arithmetic. Add tests named `truncate_str_cuts_at_a_char_boundary` and `render_handles_a_narrow_area`.

## Done when

- Both tests pass.

## Notes

- Reported on 2026-10-02 by wk-filer4, working on bug-d3c72e, during the overnight close-out round.
