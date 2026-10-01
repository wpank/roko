+++
id = "bug-7feee7"
kind = "bug"
title = "POST /plans with a long multi-byte prompt panics: derive_unique_slug cuts the first line at byte 80"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-1cb461"
anchors = ["crates/roko-serve/src/routes/plans.rs::derive_unique_slug"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1cb461"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib derive_unique_slug"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:21Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:46:55Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`derive_unique_slug` (`crates/roko-serve/src/routes/plans.rs`) takes a free-text prompt's first line as the plan title and cuts it with `&first_line[..80]` when the line is longer than 80 bytes. Byte 80 can fall inside a multi-byte character (any accented letter, CJK text or emoji), and slicing a `str` off a char boundary panics. The handler then fails with a 500, or the task dies.

## Why it matters

Authoring a plan from the portal or the API with a non-ASCII prompt crashes the request. It is the same bug class as bug-1cb461 (the aggregator's byte slicing), found while fixing that one.

## Where

- `crates/roko-serve/src/routes/plans.rs::derive_unique_slug`, the `if first_line.len() > 80 { &first_line[..80] }` branch.

## Current state

Confirmed by reading the code at `ebdc0f5d5`: the slice uses a byte index without a char-boundary check.

## Plan

1. Cut the title at a char boundary: take the first 80 chars (`char_indices().nth(80)`) or back off to `floor_char_boundary`, matching the helper bug-1cb461 used, if there is one.
2. Add a unit test named `derive_unique_slug_*` with a first line of more than 80 bytes whose byte 80 falls inside a multi-byte character. It checks that the call returns a non-empty slug without panicking.

## Done when

- `cargo test -p roko-serve --lib derive_unique_slug` passes, and includes the multi-byte case.

## Notes

- Reported by wk-serve2 on 2026-10-01 while working on bug-1cb461. The file is wk-runstate's area in that round.

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `derive_unique_slug` now takes at most 80 characters of the first line (`char_indices().nth(80)`) instead of 80 bytes, so the cut always lands on a character boundary, as the draft writer's "≤80 chars" title rule says. No shared char-boundary helper exists in roko-serve to reuse. Test: `derive_unique_slug_cuts_a_multi_byte_first_line_at_a_char_boundary` (routes/plans.rs). Its first line is longer than 80 bytes, byte 80 falls inside an `é`, and the slug comes back as `a` without a panic.
