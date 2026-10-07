+++
id = "bug-cc61a3"
kind = "bug"
title = "Unscreened live text stays in the TUI next to the screened transcript: settle_screened_transcript has no caller"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-633184"
anchors = ["crates/roko-cli/src/tui/state/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-633184"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn screened_transcript_replaces_unscreened_text' crates/roko-cli/src/ && cargo test -p roko-cli --lib screened_transcript_replaces_unscreened_text"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:02Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:45Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`TuiState::settle_screened_transcript` has no production caller. Under Trusted live output, the unscreened text an agent streamed stays in the TUI beside the screened copy, though only the screened copy should remain once it settles.

## Plan

Call it when an attempt's screened result lands, so the unscreened text is replaced. Add a test named `screened_transcript_replaces_unscreened_text`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-633184, during the evening close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57; cargo verification deferred to the batch check.
  - `AgentOutputHistory::ingest_line` settles an agent's unscreened text and reasoning when the first record of its
    screened transcript arrives (a stream record that is not live), keeping live tool steps, so live events,
    snapshot backfill and task-output rings all settle alike. `settle_screened_transcript` uses the same helper.
  - The live forwarder runs on its own task, so its last unscreened deltas can trail the screened copy; those are
    dropped until the agent's next attempt, which `drain_state_events` marks on `AgentSpawned`
    (`AgentOutputHistory::begin_attempt`). `clear_agent` now also clears the unscreened tracking.
  - Test `screened_transcript_replaces_unscreened_text` (`tui/app/tests.rs`) drives an `App`: drafts show, the
    screened copy replaces them and drops a late draft, and a new attempt's drafts show again. gap-836ae9's parity
    test now expects the draft settled away on both the live and the replayed path.
  - Unscreened tool starts and results (with raw arguments and output) are still kept beside the screened copies,
    as `classify_output_line` keeps every tool record.
