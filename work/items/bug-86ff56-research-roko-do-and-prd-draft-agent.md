+++
id = "bug-86ff56"
kind = "bug"
title = "Research, roko do and PRD-draft agent calls record no spend"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-ac5432"
anchors = ["crates/roko-cli/src/research.rs", "crates/roko-cli/src/commands/do_cmd.rs", "crates/roko-cli/src/prd.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-ac5432"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib research_calls_record_spend"
+++

## Problem

After bug-ac5432, capture episodes no longer write $0 cost rows. That exposes that research, `roko do` and PRD-draft agent calls record no spend at all.

## Plan

Record each call's real usage and cost in the cost log, the way plan generation does after bug-ac5432. Add a test named `research_calls_record_spend_*`.

## Done when

- `cargo test -p roko-cli --lib research_calls_record_spend` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-ac5432, during the evening close-out round.
