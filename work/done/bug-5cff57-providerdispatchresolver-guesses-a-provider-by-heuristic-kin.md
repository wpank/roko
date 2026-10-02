+++
id = "bug-5cff57"
kind = "bug"
title = "ProviderDispatchResolver guesses a provider by heuristic kind for an unknown model key"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-fd44df"
anchors = ["crates/roko-cli/src/dispatch/"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-fd44df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib provider_dispatch_resolver_unknown_model"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:13Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:22Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-fd44df made `create_agent_for_model` error on an unknown model key with no command. roko-cli's `ProviderDispatchResolver::resolve` still picks a provider by a heuristic on the key, so an unknown key can dispatch to the wrong provider.

## Plan

Resolve through `try_resolve_model`, and fail with a clear error for unknown keys. Add a test named `provider_dispatch_resolver_unknown_model_*`.

## Done when

- `cargo test -p roko-cli --lib provider_dispatch_resolver_unknown_model` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-fd44df, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `ProviderDispatchResolver::resolve` (dispatch_v2.rs) returns an unsupported target with the new
  `UnsupportedProviderReason::UnknownModel` and `try_resolve_model`'s reason when no `[models.*]` entry,
  configured slug or builtin model resolves the key; builtin models resolve as before. Test:
  `provider_dispatch_resolver_unknown_model_is_unsupported`.
