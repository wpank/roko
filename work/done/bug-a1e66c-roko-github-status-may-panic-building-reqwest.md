+++
id = "bug-a1e66c"
kind = "bug"
title = "roko github status may panic building reqwest's blocking client on the async runtime"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-cd51b7"
anchors = ["crates/roko-cli/src/commands/github.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-cd51b7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --bin roko github_status_on_runtime"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:20Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:57Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`roko github status` builds reqwest's blocking client on the async runtime thread (`commands/github.rs:34`). reqwest 0.12 panics there in debug builds (`blocking/wait.rs` enter()). The tests always remove GITHUB_TOKEN, so the path is unexercised and this is unconfirmed.

## Plan

Use the async client or `spawn_blocking`. Add a test named `github_status_on_runtime_*` with a fake token and a local server.

## Done when

- `cargo test -p roko-cli --bin roko github_status_on_runtime` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-cd51b7, during the evening close-out round.
- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7; cargo verification deferred to the batch check.
- 2026-10-01 (wk-filer4): What changed: the new `remote_status` builds the client on the blocking worker that already made the GitHub calls; before, `GitHubClient::from_env()` ran on the async runtime thread (commands/github.rs:34 at ebdc0f5d5). The new test `github_status_on_runtime_builds_its_client_on_a_blocking_worker` runs on a tokio runtime with a fake token and a local HTTP server. The panic itself was never reproduced (no cargo run): it follows from reqwest 0.12.28 blocking/wait.rs `enter()`, which in debug builds creates and drops a runtime.
