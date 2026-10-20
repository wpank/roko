+++
id = "gap-7e375c"
kind = "gap"
title = "The repo records JaeLeex's written agreement to license their apps/mirage-rs code under MIT OR Apache-2.0"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
hold = "Will sends the request drafted in tmp/status-quo/2026-10-15/licence-request-mirage-rs.md and forwards the reply"
subsystem = ["apps/mirage-rs"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-10"
anchors = ["apps/mirage-rs/src/precompiles/hdc.rs", "LICENSE-MIT"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["q-bc6cc5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f CONTRIBUTORS.md && grep -q 'JaeLeex' CONTRIBUTORS.md && grep -q 'MIT OR Apache-2.0' CONTRIBUTORS.md"
+++

## Problem

About 1,370 surviving lines at HEAD were written by the git author JaeLeex (15 commits, 2026-04-16 to 04-23). Of
those, 1,363 are in `apps/mirage-rs`; the rest are a few lines in `crates/roko-demo` and `crates/roko-serve`. Nothing
records that this code may be released under the repo's MIT OR Apache-2.0 licence (q-bc6cc5).

## Why it matters

Goal `release`. A clean public licence needs it. Will chose option A on 2026-10-15: the contributor agrees in writing
(`tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`, decision 10).

## Where

- `apps/mirage-rs/src/precompiles/hdc.rs`: 839 of 842 lines are theirs.
- Parts of `rpc.rs`, `provider.rs`, `fork.rs`, `persist.rs`, `lib.rs` and `precompiles/mod.rs`.
- `LICENSE-MIT`.

## Current state

The request is drafted in `tmp/status-quo/2026-10-15/licence-request-mirage-rs.md`. Will sends it.

## Plan

1. When the reply arrives, add `CONTRIBUTORS.md`: one row naming JaeLeex, the date of the agreement, and the sentence
   they agreed to ("may be distributed under the MIT License or the Apache License 2.0", i.e. MIT OR Apache-2.0).
2. If they decline or don't answer, fall back to option C (carve `apps/mirage-rs` out of the release and the
   workspace, with a NOTICE) or option D (rewrite the HDC precompile). Will picks which.

## Done when

`CONTRIBUTORS.md` records the agreement (the `[[verify]]` command).

## Notes

Held until Will has the reply. Never contact the contributor from an agent.
