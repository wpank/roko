+++
id = "bug-52c48f"
kind = "bug"
title = "health_hold treats AuthFailure as a generic open circuit, and serve_runtime.rs records every failure as Unknown"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/provider-failover", "roko-cli/serve-runtime"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "730b43d91"
source = "wave-4 follow-up reports 2026-10-02 (PK02)"
discovered_from = "gap-e00238"
anchors = ["crates/roko-learn/src/provider_failover.rs::health_hold", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-serve/src/service_factory.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn health_hold_reports_auth_failure_as_definitive' crates/roko-learn/ && cargo test -p roko-learn health_hold_reports_auth_failure_as_definitive"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T02:27:42Z"
commit = "730b43d91"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T01:27:39Z"
forced = false
evidence = "Gate 6a (merged into main as 730b43d91, tree identical to work/backlog-batch-6a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 7,078 passed (roko-agent, roko-cli, roko-learn), roko-cli bin + golden-path canaries + operator_checkout_clean 442/442, hub_ipc 7/7, roko-learn legacy_rule_live + loop_audit_cs_reference; every [[verify]] passes."
+++

## Problem

Two places misclassify or under-classify provider failures so that an auth failure is not treated as the
"operator must act" case it actually is:

1. **`health_hold` treats `AuthFailure` as a generic open circuit.**
   `crates/roko-learn/src/provider_failover.rs::health_hold` (line ~297) matches the provider's most recent
   `failure_window` entry's `error_class`:
   ```rust
   let (class, reason, definitive) = match health.failure_window.back().map(|record| record.error_class) {
       Some(ErrorClass::Exhausted) => ("provider_exhausted", "out of usage", true),
       Some(ErrorClass::Billing) => ("billing", "billing failure", true),
       _ => ("circuit_open", "circuit open after repeated failures", false),
   };
   ```
   There is no `Some(ErrorClass::AuthFailure) => (...)` arm, so an auth failure falls into the catch-all
   `"circuit_open"` / `definitive: false` case — the same label and "might recover on its own" framing as a
   transient repeated-failure circuit, even though `ProviderHealth::record_failure`'s own comment two structs over
   says plainly: "an auth failure will not fix itself within a run" (`provider_health.rs:223`, backlog 1115). This
   function is what `roko-serve` and `roko-acp` consult (confirmed: `provider_failover::health_hold` is exported
   from `roko-learn` and used by both crates' dispatch paths) — so both report an auth failure to callers/operators
   as an ordinary open circuit, not as the definitive, operator-actionable failure it is.

2. **`serve_runtime.rs` hard-codes `Unknown`.**
   `crates/roko-cli/src/serve_runtime.rs:1414`: `health.record_failure(&provider, ErrorClass::Unknown);` — this
   generic path records every failure's class as `Unknown` regardless of what the error actually was, rather than
   using the shared classifier (`crates/roko-agent/src/model_call_service.rs::provider_error_kind`, or the
   `roko-agent/src/provider/error_classify.rs` entry points unified by backlog task 1113). Contrast
   `crates/roko-serve/src/service_factory.rs:1149`, which DOES classify correctly for one specific case:
   `health.record_failure("unavailable-provider", ErrorClass::AuthFailure)` — proving the correct classification is
   available and used elsewhere in roko-serve, just not from `serve_runtime.rs`'s generic failure-recording path.

## Why it matters

Goal: truth. `ErrorClass::AuthFailure` already gets special, intentional handling further down the stack —
`ProviderHealth::record_failure` trips the circuit immediately on the first occurrence for
`Billing | Exhausted | AuthFailure` (the `should_trip_billing` check), and gives it a 24-hour cooldown
(`cooldown_ms`, `provider_health.rs:411`) specifically because logging in again or fixing billing won't happen
automatically mid-run. But `health_hold`'s reporting (what callers and operators actually see) erases that
distinction for AuthFailure, and `serve_runtime.rs`'s recording erases it for everything. A roko-serve operator
sees "circuit open" or doesn't even get a real error class recorded, and has no reason to think "I need to log back
in" rather than "wait for it to recover" — which it will not, for 24 hours, regardless.

## Where

- `crates/roko-learn/src/provider_failover.rs::health_hold` (line ~297): missing `ErrorClass::AuthFailure` match
  arm.
- `crates/roko-cli/src/serve_runtime.rs:1414`: hard-coded `ErrorClass::Unknown`.
- Contrast/reference: `crates/roko-serve/src/service_factory.rs:1149` (correct classification, different call
  site); `crates/roko-agent/src/model_call_service.rs::provider_error_kind` / `roko-agent/src/provider/error_classify.rs`
  (the shared classifier serve_runtime.rs should call instead of hard-coding Unknown).

## Current state

Unfixed in both places. `health_hold`'s omission is a one-line gap (an unhandled, high-value enum variant falling
into the catch-all); `serve_runtime.rs`'s is a hard-coded constant that never varies regardless of the real error.

## Plan

1. Add `Some(ErrorClass::AuthFailure) => ("auth_failure", "needs the operator to log in again", true)` to
   `health_hold`'s match (mirroring the `Exhausted`/`Billing` arms' `definitive: true`).
2. Change `serve_runtime.rs:1414` to classify the failure's text the same way the CLI path does (via
   `provider_error_kind` or the shared `error_classify` entry points) before calling `record_failure`, instead of
   hard-coding `ErrorClass::Unknown`.
3. Tests: `health_hold` reports `class == "auth_failure"` and `definitive == true` for a provider whose last
   failure was classified `AuthFailure`; `serve_runtime`'s failure recording classifies an auth-failure-shaped
   error text as `ErrorClass::AuthFailure`, not `Unknown`.

## Done when

- `health_hold` on a provider last failed with `AuthFailure` returns a `Hold` distinct from a generic circuit-open
  (its own class/reason, `definitive: true`).
- `serve_runtime.rs` records a classified error, not an unconditional `Unknown`.
- The `[[verify]]` command passes.

## Notes

- Related: entry (d) of this same wave-4 report (filed separately) — once an auth failure IS correctly classified
  and surfaced as `definitive: true` / its own reason, there still needs to be a way to clear it after the operator
  fixes the underlying problem; that is the other item's scope, not this one's.

## Progress

- 2026-10-03: implemented on `work/gap-d90a93` at d639390ff; `health_hold` holds AuthFailure definitively; serve_runtime records `ErrorClass::from_failure_text`. Cargo verification deferred to the batch gate.
