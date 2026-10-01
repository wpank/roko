+++
id = "bug-7cdce7"
kind = "bug"
title = "Attempt timeouts and turn-cap stops count as provider failures and can open the circuit breaker"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch_v2", "roko-learn/provider_health"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::run_agent_result_bridge_with_tools_and_cli_mcp", "crates/roko-cli/src/dispatch_v2.rs::run_agent_result_bridge", "crates/roko-cli/src/dispatch_v2.rs::classify_provider_error", "crates/roko-learn/src/provider_health.rs::record_failure", "crates/roko-cli/src/graph_task_dispatch/failover.rs::blocked_provider"]
links = { depends_on = [], blocks = [], related = ["gap-28ceb9", "find-cb5eeb", "find-43768e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_timeouts_do_not_open_the_provider_circuit' crates/roko-cli/src && cargo test -p roko-cli --lib attempt_timeouts_do_not_open_the_provider_circuit"
+++

## Problem

Graph dispatch runs every attempt through `AgentDispatcherV2::run_agent_result_bridge_with_tools_and_cli_mcp`. That
bridge records every unsuccessful result as a provider failure in the shared circuit breaker, classified by
`classify_provider_error`.

An attempt killed at its own wall-clock budget returns `"timed out after N ms"`. It is classified `"timeout"` and
recorded as `ErrorClass::Timeout`. That is a task outcome (the task took longer than its budget), not a sign that the
provider is unhealthy. The breaker opens after 3 consecutive failures, or when fewer than 30% of the last 10 requests
succeeded. The Timeout cooldown is 10 s, and each further failure while open extends it.

So a slow task can open the circuit for a healthy provider, and since `e0673e3e0` that is more likely: a timed-out task
is retried with 1.5x the timeout, so one hard task can produce three consecutive timeouts. Then:

- `GraphTaskDispatcher::blocked_provider` refuses the provider for the next dispatches ("circuit open after repeated
  failures");
- those dispatches fail over to fallback models, or run the planned model anyway when there is no alternative;
- the state persists in `.roko/learn/provider-health.json`.

Turn-cap stops have the same problem on this path. The sibling bridge `run_agent_result_bridge` already records a
turn-cap stop as a provider success ("a task outcome, not a provider fault"), but the bridge Graph uses does not.

## Why it matters

Goal `core`. Spurious failover sends tasks to models the plan did not choose, and it corrupts the provider-health signal
that routing reads. That happens exactly on long tasks, where the planned model mattered most.

## Where

- `crates/roko-cli/src/dispatch_v2.rs`:
  - `run_agent_result_bridge_with_tools_and_cli_mcp` records the outcome at :1808-1826, with no turn-cap or
    attempt-timeout exemption;
  - `run_agent_result_bridge` (:1567) is the variant with the turn-cap exemption (:1580-1601);
  - `classify_provider_error` (:1923): "timeout" / "timed out" become `timeout`.
- `crates/roko-learn/src/provider_health.rs`:
  - `ProviderHealth::record_failure` (:183, trips at `consecutive_failures >= 3`, :208, or by the rolling rate);
  - `cooldown_ms` (Timeout 10 s, :371);
  - the `"timeout"` to `ErrorClass::Timeout` mapping (:1196).
- `crates/roko-cli/src/graph_task_dispatch.rs::blocked_provider` (:5131) reads `health_registry.is_available`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (:875-880) loads the persisted registry for plan runs.
- `roko_agent::provider::error_classify::detect_attempt_timeout` (`ATTEMPT_TIMEOUT_MARKER`) and `detect_turn_cap`
  already recognise both outcomes.

## Current state

Checked statically at `33e107da1`:

- The only exemption from provider failure is the turn cap, and only in `run_agent_result_bridge`.
- Genuine provider timeouts (an HTTP request or connect timeout) should keep counting; those messages do not carry the
  attempt-timeout marker.

## Plan

1. Factor the outcome recording of both bridges into one helper. It records:
   - success, or a turn-cap stop: provider success;
   - an attempt timeout (`detect_attempt_timeout`): no provider outcome at all, or a neutral one that neither trips nor
     resets the breaker;
   - anything else: failure, classified as today.
2. Keep exhaustion and billing handling as they are (`gap-28ceb9` covers the missing exhaustion class).
3. Test `attempt_timeouts_do_not_open_the_provider_circuit` (dispatch_v2 tests, lib): a fake CLI that sleeps past a
   1 s timeout, dispatched 3 times through the bridge with a registry. Assert that the provider is still available and
   `consecutive_failures` is 0. Add a turn-cap case too.

## Done when

- Three consecutive attempt timeouts, or turn-cap stops, leave the provider's circuit closed.
- A real provider failure still trips it.
- The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7cdce7; cargo verification deferred to the batch check.
  - Both bridges record the provider outcome through `AgentDispatcherV2::record_provider_outcome`:
    - a success or a turn-cap stop is a provider success;
    - an attempt timeout (`detect_attempt_timeout`) records nothing;
    - anything else is a classified failure, as before.
  - Test `attempt_timeouts_do_not_open_the_provider_circuit` runs three cases through the Graph bridge: three
    attempt timeouts, three turn-cap stops and three 503 failures. Only the 503 case opens the circuit.
  - Caveat: `detect_attempt_timeout` matches any "timed out after". So an adapter whose own request timeout uses
    that wording (Hermes: "request timed out after 90s") no longer counts against the provider either. This matches
    how `turn_policy` already classifies it. Tightening the marker to "timed out after <N> ms" would separate the two.
- 2026-10-01 (wk-tiers): the caveat above is resolved on work/bug-7cdce7. `detect_attempt_timeout` now matches only
  the adapters' own `"timed out after <N> ms"`, so a provider's request timeout (Hermes' "request timed out after
  90s") counts against the provider again. Test: `a_provider_request_timeout_is_not_an_attempt_timeout`.
