+++
id = "bug-7feee7"
kind = "bug"
title = "POST /plans with a long multi-byte prompt panics: derive_unique_slug cuts the first line at byte 80"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-1cb461"
anchors = ["crates/roko-serve/src/routes/plans.rs::derive_unique_slug"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1cb461"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib derive_unique_slug"
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
