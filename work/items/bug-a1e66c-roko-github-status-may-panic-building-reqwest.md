+++
id = "bug-a1e66c"
kind = "bug"
title = "roko github status may panic building reqwest's blocking client on the async runtime"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-cd51b7"
anchors = ["crates/roko-cli/src/commands/github.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-cd51b7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --bin roko github_status_on_runtime"
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
