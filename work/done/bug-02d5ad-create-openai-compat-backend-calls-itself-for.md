+++
id = "bug-02d5ad"
kind = "bug"
title = "create_openai_compat_backend calls itself for Hermes and OpenClaw, and its limiter variant has no callers"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-53088d"
anchors = ["crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend_with_limiter"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-53088d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib openai_compat_backend_for_hermes"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:04Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:22Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`create_openai_compat_backend` (`tool_loop/backends/mod.rs`) calls itself for the Hermes and OpenClaw kinds, which recurses without end if `create_tool_loop_backend` routes either kind there. `create_openai_compat_backend_with_limiter` has no callers.

## Plan

Route Hermes and OpenClaw to their real backends, or return an error, and delete or wire the limiter variant. Add a test named `openai_compat_backend_for_hermes_*`.

## Done when

- `cargo test -p roko-agent --lib openai_compat_backend_for_hermes` passes.

## Notes

- Reported on 2026-10-01 by the worker on bug-53088d, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `create_openai_compat_backend` builds the chat completions backend for Hermes and OpenClaw in the OpenAiCompat
  arm; the arm that called the function again is gone. The second half of the premise was wrong:
  `create_openai_compat_backend_with_limiter` has two callers in roko-acp (`bridge_events/dispatch.rs`), so it
  stays. Test: `openai_compat_backend_for_hermes_and_openclaw_speaks_chat_completions`.
