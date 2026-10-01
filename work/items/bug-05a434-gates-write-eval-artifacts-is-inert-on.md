+++
id = "bug-05a434"
kind = "bug"
title = "gates.write_eval_artifacts is inert on the Graph path, and a test no longer proves the suppression it claims"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-017c2d"
anchors = ["crates/roko-core/src/config/gates.rs", "crates/roko-cli/src/graph_task_dispatch.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-017c2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib write_eval_artifacts"
+++

## Problem

After bug-017c2d, the Graph path has no property-body source, so `gates.write_eval_artifacts` does nothing. `skip_enrichment_plan_meta_is_read_and_suppresses_eval_artifacts` no longer proves any suppression. `EpisodeView::succeeded` has no reader (gap-d0f52f's notes).

## Plan

List write_eval_artifacts as inert, or give it a source again. Rewrite the test to check what still happens, and delete `EpisodeView::succeeded` or give it a reader.

## Done when

- `cargo test -p roko-cli --lib write_eval_artifacts` passes, and the setting is honest.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-017c2d, during the evening close-out round.
