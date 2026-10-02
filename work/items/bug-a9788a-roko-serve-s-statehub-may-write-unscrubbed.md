+++
id = "bug-a9788a"
kind = "bug"
title = "roko serve's StateHub may write unscrubbed agent output to .roko/events.jsonl"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-runtime", "roko-serve"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-8a1fb3"
anchors = ["crates/roko-runtime/src/state_hub.rs", "crates/roko-serve/src/lib.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib serve_event_log_is_scrubbed"
+++

## Problem

Under `roko serve`, the StateHub writes `.roko/events.jsonl` itself, through roko-runtime's `state_hub.rs` `EventLogWriter::append`, which serializes with plain serde_json and does not scrub. `WorkspaceEventLog` writes only the index when the hub persists. A serve-hosted run's agent stream records (unscreened text, tool results) therefore look like they reach `events.jsonl` unscrubbed. The CLI path scrubs (`roko_core::obs::scrub_secrets_in_jsonl`), and bug-230de6 fixed the same class of leak for the per-run index.

## Why it matters

A secret an agent prints during a serve-hosted run would be persisted in plain text, which is exactly what the C2 canary guards against on the CLI path.

## Plan

1. Confirm by reading the serve write path.
2. Scrub every line the hub writes, through the same scrubber the CLI path uses, or route the hub's persistence through `WorkspaceEventLog`.
3. Add a C2-style test, `serve_event_log_is_scrubbed`: a serve-hosted run whose agent prints a planted key, after which the key appears nowhere under `.roko/`.

## Done when

- The test passes, and the secret canaries still pass.

## Notes

- Reported on 2026-10-02 by wk-streams, working on gap-8a1fb3.
