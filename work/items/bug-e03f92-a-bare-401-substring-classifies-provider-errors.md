+++
id = "bug-e03f92"
kind = "bug"
title = "A bare \"401\" substring classifies provider errors as auth failures in roko-agent"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-28c193"
anchors = ["crates/roko-agent/src/provider/mod.rs::map_provider_error", "crates/roko-agent/src/model_call_service.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28c193"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib http_401"
+++

## Problem

`map_provider_error` (`provider/mod.rs:1237`) and `model_call_service.rs:2167` treat any message containing "401" as an authentication error. Token counts, ids or durations that contain 401 are misclassified. bug-28c193 fixed the same false positive in roko-cli's error hint.

## Plan

Match 401 only as an HTTP status, reusing the shape of bug-28c193's `mentions_http_401`. Add tests named `http_401_*`.

## Done when

- `cargo test -p roko-agent --lib http_401` passes.

## Notes

- Reported on 2026-10-01 by the worker on bug-28c193, during the evening close-out round.
