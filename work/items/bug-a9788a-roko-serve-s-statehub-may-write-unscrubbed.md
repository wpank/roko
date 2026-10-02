+++
id = "bug-a9788a"
kind = "bug"
title = "roko serve's StateHub may write unscrubbed agent output to .roko/events.jsonl"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-runtime", "roko-serve"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "fa10c24ef"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-8a1fb3"
anchors = ["crates/roko-runtime/src/state_hub.rs", "crates/roko-serve/src/lib.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_event_log_is_scrubbed' crates/roko-serve/src/ && cargo test -p roko-serve --lib serve_event_log_is_scrubbed"
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
- 2026-10-02 (wk-streams): confirmed, and implemented on work/gap-b35a57; cargo verification and the secret canaries
  deferred to the batch check.
  - Confirmed: serve's hub (`AppState::state_hub_for_workdir_with_config`) always opens `.roko/events.jsonl` and
    `.roko/projection-history.jsonl`, and `EventLogWriter::append` wrote `serde_json::to_string` as it was. Serve's
    runtime event log (`JsonlLogger::write_event`, `.roko/runtime-events.jsonl` and its per-run index, fed by
    `POST /api/events/ingest` and the service bundle) did the same.
  - Both writers now pass each line through `roko_core::obs::scrub_secrets_in_jsonl`, the scrubber the runner's own
    log writer (`roko_fs::log_rotation`) uses. Test: `serve_event_log_is_scrubbed` (`routes/run.rs`) plants a key in
    the process scrubber, then runs a serve-hosted one-shot run whose agent prints it, publishes a plan run's
    stream record and ingests a runtime event holding it. It checks that both logs hold it redacted and that no file
    under `.roko/` holds the key.
