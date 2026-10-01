+++
id = "bug-e03f92"
kind = "bug"
title = "A bare \"401\" substring classifies provider errors as auth failures in roko-agent"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-28c193"
anchors = ["crates/roko-agent/src/provider/mod.rs::map_provider_error", "crates/roko-agent/src/model_call_service.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28c193"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib http_401"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:25Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:22Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`map_provider_error` (`provider/mod.rs:1237`) and `model_call_service.rs:2167` treat any message containing "401" as an authentication error. Token counts, ids or durations that contain 401 are misclassified. bug-28c193 fixed the same false positive in roko-cli's error hint.

## Plan

Match 401 only as an HTTP status, reusing the shape of bug-28c193's `mentions_http_401`. Add tests named `http_401_*`.

## Done when

- `cargo test -p roko-agent --lib http_401` passes.

## Notes

- Reported on 2026-10-01 by the worker on bug-28c193, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `error_classify::mentions_http_401` (bug-28c193's shape, plus "code") matches a standalone 401 within three
  words of http, status, code, unauthorized, request or returned; `map_provider_error` and model_call_service's
  `provider_error_kind` use it. Tests: `http_401_matches_only_an_http_status`,
  `http_401_inside_a_count_is_not_an_api_key_error`, `http_401_is_an_auth_failure_only_as_a_status`.
