+++
id = "bug-aad63e"
kind = "bug"
title = "roko acp distils every episode with unrecorded spend, and roko serve's dispatch path is unchecked"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0f8948"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs", "crates/roko-serve/src/dispatch.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0f8948"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib acp_distillation_records_spend"
+++

## Problem

bug-0f8948 stopped the CLI's per-episode distillation call from going unaccounted. `roko-acp/src/bridge_events/cost.rs` (around line 237) still distils every ACP episode with spend recorded nowhere, and `roko-serve/src/dispatch.rs` (around line 2633) has not been checked.

## Plan

Apply bug-0f8948's rule in both places: record the distillation call's spend, or skip it as the CLI now does. Add a test named `acp_distillation_records_spend_*`.

## Done when

- The test passes, and serve's path is checked and noted.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-0f8948, during the evening close-out round.
