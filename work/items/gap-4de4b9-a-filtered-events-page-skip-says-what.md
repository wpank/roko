+++
id = "gap-4de4b9"
kind = "gap"
title = "A filtered events page (?skip=) says what it skipped but not how many rows, so a reader cannot show the index's true size"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-plugin-wow#skipped_rows"
discovered_from = "agent:plugin-wow (round v7, 2026-10-20)"
anchors = ["crates/roko-serve/src/routes/runs.rs::IndexedPage", "crates/roko-serve/src/routes/runs.rs::get_run_events"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-cb9044"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_events_page_can_skip_live_words' crates/roko-serve/ && grep -q 'skipped_rows' crates/roko-serve/src/routes/runs.rs && cargo test -p roko-serve --lib -- a_run_events_page_can_skip_live_words"
+++

## Problem

A page of `GET /api/runs/{id}/events?skip=…` (gap-cb9044) says what it skipped (`"skipped": ["live_text", …]`) but not
how many rows. A reader that counts what it read (the Hermes plugin's "N events read") then shows only the rows it
was sent: on the pilot's P0 index, 891 of 34,919.

## Why it matters

The page's count should stay the index's true size, so the person can see how much the run wrote, even when the
reader asked to skip its words. plugin-wow asked for it (round v7, 2026-10-20). It is not needed for correctness.

## Where

- `crates/roko-serve/src/routes/runs.rs`: `IndexedPage`, `read_index_page_scan` (its `skip` passes over a line before
  parsing it), and `get_run_events`.

## Plan

1. Count the lines the scan's `skip` passes over in `IndexedPage`.
2. A page that skips gets `skipped_rows`. A reader's total is `events.len() + skipped_rows`, summed over its pages.

## Done when

The sum of a run's filtered pages' `events` and `skipped_rows` is the number of records in its index. Checked by the
test in `[[verify]]`.

## Notes

- Follow-up to gap-cb9044, at plugin-wow's request. Owner: roko-speed.
