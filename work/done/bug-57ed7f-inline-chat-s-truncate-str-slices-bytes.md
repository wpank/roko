+++
id = "bug-57ed7f"
kind = "bug"
title = "Inline chat's truncate_str slices bytes and its render subtracts from a narrow width, both panicking"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/chat_inline"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d3c72e"
anchors = ["crates/roko-cli/src/chat_inline/session.rs", "crates/roko-cli/src/chat_inline/render.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-d3c72e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib truncate_str_cuts_at_a_char_boundary"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:34Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`chat_inline/session.rs::truncate_str` does `&s[..max - 3]`, which panics when the cut falls inside a multi-byte character. It truncates error messages in `chat_inline/render.rs:50`, which also computes `area.width as usize - 6` and underflows when the area is narrower than 6.

## Plan

Cut at a char boundary, and saturate the width arithmetic. Add tests named `truncate_str_cuts_at_a_char_boundary` and `render_handles_a_narrow_area`.

## Done when

- Both tests pass.

## Notes

- Reported on 2026-10-02 by wk-filer4, working on bug-d3c72e, during the overnight close-out round.
- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7; cargo verification deferred to the batch check.
- 2026-10-01 (wk-filer4): What changed: `truncate_str` now counts and cuts characters instead of bytes. It also no longer shortens a string that fits by characters but not by bytes: the old code returned "é" for ("éé", 2). render.rs passes `(area.width as usize).saturating_sub(6)`. The tests are `truncate_str_cuts_at_a_char_boundary` and `render_handles_a_narrow_area`; the second draws the error phase on TestBackend widths 1, 4, 5, 6, 7 and 40. The existing `truncate_short` and `truncate_long` expectations still hold.
