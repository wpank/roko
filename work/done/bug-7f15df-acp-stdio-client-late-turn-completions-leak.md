+++
id = "bug-7f15df"
kind = "bug"
title = "ACP stdio client: late turn completions leak into the next turn, and byte-slicing a log line panics on non-ASCII output"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/harness"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-97c2dc"
anchors = ["crates/roko-agent/src/harness/acp_client.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-97c2dc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib acp_client_"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:37:07Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

In `harness/acp_client.rs`:
- After a timed-out or cancelled turn, the late `stopReason` stays in the turn-done queue and stale notifications stay queued, so the next Hermes or OpenClaw turn reads them as its own: it ends at once or carries the old text.
- `&line[..line.len().min(200)]` (the stdout reader's trace and parse-error logs) and `&json[..json.len().min(500)]` (`send_request`) panic when the cut falls inside a multi-byte character, so one non-ASCII non-JSON stdout line kills the reader task.
- The `Drop` comment says dropping the JoinHandles aborts the reader tasks; tokio detaches them.

## Plan

Tag queued completions and notifications with their turn, and drop stale ones at the next turn's start. Cut log lines at a char boundary. Fix the Drop comment, or abort the tasks. Add tests named `acp_client_*` for both bugs.

## Done when

- `cargo test -p roko-agent --lib acp_client_` passes, and covers the stale turn and a multi-byte line.

## Notes

- Reported on 2026-10-01 by the worker on bug-97c2dc, during the evening close-out round.
- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- The reader tags each queued notification with the latest prompt id, and each completion carries its request id. `take_notification_rx` and `take_turn_done_rx` now lend `AcpNotificationRx` and `AcpTurnDoneRx`, whose `recv` skips notifications queued before the latest `session/prompt` or naming another session, and completions of other prompts; `send_prompt` marks the turn before it writes the request. Log lines are cut with `floor_char_boundary`, and `Drop` now aborts the reader tasks, as its comment claimed. Tests: `acp_client_turn_skips_what_an_earlier_prompt_left` and `acp_client_logs_a_multibyte_line_without_panicking`. The Hermes and OpenClaw consumers compile unchanged.
- Left as is: a late notification from an earlier prompt that names no session and arrives after the next prompt is sent still reaches the new turn. ACP's `session/update` carries `sessionId`, so compliant servers are covered.
