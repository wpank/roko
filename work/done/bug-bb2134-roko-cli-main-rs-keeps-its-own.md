+++
id = "bug-bb2134"
kind = "bug"
title = "roko-cli main.rs keeps its own mentions_http_401, a copy of roko-agent's"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e03f92"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-e03f92"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test $(grep -rn 'fn mentions_http_401' crates/ --include=*.rs | wc -l) -le 1"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:22Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:34Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

bug-28c193 added `mentions_http_401` to roko-cli `main.rs`, and bug-e03f92 added the same check in roko-agent. There are now two copies.

## Plan

Keep one: export roko-agent's, and use it from main.rs.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-e03f92, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  main.rs's `error_hint` calls `roko_agent::provider::error_classify::mentions_http_401` (already `pub`), and
  main.rs's copy and its `is_http_status_word` are gone. roko-agent's version also counts "code" as a status word
  ("error code 401"); bug-28c193's `error_hint` tests still hold. The verify passes (run under bash).
