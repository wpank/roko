+++
id = "bug-220385"
kind = "bug"
title = "Calls refused for provider exhaustion during failover leave no record"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "924008f6c"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-35379d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_exhaustion_refusal_during_failover_is_recorded' crates/roko-cli/src/graph_task_dispatch/ && cargo test -p roko-cli --lib an_exhaustion_refusal_during_failover_is_recorded"
+++

## Problem

During failover, Graph dispatch skips a candidate whose provider is exhausted (`PROVIDER_EXHAUSTED_CATEGORY`, `routing.exhaustion_cooldown_secs`; `graph_task_dispatch/failover.rs`:12, :120, :174 on bug-31438d's branch). A call refused this way leaves no episode, cost or efficiency row. The records show the model that finally served the call, but not the candidates tried and refused before it.

## Why it matters

One settled record per attempt (epic spec-b7303f): the failover chain is part of what happened in an attempt. Without it, analysis can't see how often exhaustion forces a substitution, and a pin check can't tell a refusal from a model that was never tried.

## Where

The exhaustion branch of the failover loop in `failover.rs`.

## Plan

1. Record each refused candidate on the attempt, with its model, provider, reason `provider_exhausted` and time. Either keep a `failover` list on the attempt's records, or write one row per refusal with zero usage.
2. Add `an_exhaustion_refusal_during_failover_is_recorded`.

## Done when

- [ ] An attempt's records list every candidate that failover refused, and why.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
- Implemented on `work/bug-b8af02` at `339429386` (a test that read the first cost row fixed at `924008f6c`); cargo verification deferred to the batch check. `an_exhaustion_refusal_during_failover_is_recorded` (targeted `cargo test` passed at the branch head). Changes:
  - Every candidate failover refused is a `roko_learn::telemetry::FailoverRefusal` (model, provider, class, reason, `called`, `at`, `until`) on the verdict's `executed.failover_refusals` and the episode's `extra.failover_refusals`. Classes: `provider_exhausted`, `billing`, `circuit_open`, `disabled`, `no_credentials`, `not_configured`, `not_dispatchable`.
  - A call the provider refused for its usage limit also gets its own cost and efficiency rows, with role `failover_refused`, the attempt's key, `attempt_id` `<key>/refused-N`, and the call's usage and duration. To key them, `run_bridge_with_failover` now takes the attempt key (a one-line change in `dispatch()`). The helper-call row writer became `write_side_call_rows` over a `SideCall`.
  - A candidate skipped before any call (`called: false`) gets no rows, only its refusal entry.
