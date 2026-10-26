+++
id = "bug-80f0d0"
kind = "bug"
title = "A request a provider refuses counts against the provider's circuit: three HTTP 400s on gpt-5.4-mini in phase 4's run 16fc88ae opened the openai circuit, blocking gpt-5.4 on strong and every OpenAI model on every rung for a fault in one model's request"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-learn/provider_health"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7 phase 4 run 16fc88ae, the lead's ask after bug-35b190"
discovered_from = "agent:team-lead (round v7 phase 4)"
anchors = ["crates/roko-learn/src/provider_health.rs::ErrorClass", "crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry"]
links = { depends_on = [], blocks = [], related = ["bug-35b190", "bug-9b65dc", "bug-29014c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn refused_requests_leave_the_providers_circuit_closed' crates/roko-cli/src && cargo test -p roko-cli --lib -- refused_requests_leave_the_providers_circuit_closed"

[[verify]]
command = "grep -rqw 'fn a_refusal_never_counts_toward_the_providers_circuit' crates/roko-learn/src && cargo test -p roko-learn --lib -- a_refusal_never_counts_toward_the_providers_circuit"

[[verify]]
command = "grep -rqw 'fn a_refused_request_twice_on_a_rung_climbs' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_refused_request_twice_on_a_rung_climbs"
+++

## Problem

A request a provider refuses counts against the provider's circuit, which blocks every model of that provider. In
phase 4's run 16fc88ae, OpenAI answered HTTP 400 to gpt-5.4-mini's requests. It refused `reasoning_effort` beside
function tools, a parameter the gpt-5.4 family rejects there. After three of them the `openai` circuit opened. That
blocked gpt-5.4 on strong, and every other OpenAI model on every rung, for the circuit's cooldown, for a fault in one
model's request.

Since bug-35b190 such a refusal is typed and classed `request_refused` (`model_not_found` for a 404). Provider health
still files those classes as `ErrorClass::Unknown`, and three in a row trip the circuit.

## Where

- `crates/roko-learn/src/provider_health.rs`: `ErrorClass::from_kind` and `ProviderHealthRegistry::record_failure`,
  which every path that records provider health goes through:
  - Graph dispatch's per-try outcome (`dispatch_v2::record_provider_outcome`);
  - the tool-loop backends' outcome recorder;
  - the model-call feedback recorder;
  - `roko serve`'s runtime.

## Plan

- A refused request (`request_refused`, `model_not_found`) is a fault in the request or its model, not the provider:
  provider health files it as its own class and never counts it toward the provider's circuit.
- A 403 is "you may not do this": no access to this model for the project, a region block or an org policy, most
  often scoped to the model or the request. It is a model-scoped refusal (`request_refused`), not the auth failure
  bug-35b190 made it, unless its words say the credential itself is invalid or revoked ("invalid api key",
  "incorrect api key", "revoked", "unauthenticated"): then it is `auth_failure`, with the provider's words. A 401
  stays an auth failure (the lead's call).
- The model side already exists. The ladder counts `provider_error:request_refused` as an infra failure on its rung,
  so the same refusal twice in a row climbs the task past the model (bug-9b65dc).

## Done when

Three refusals of one model's request, a model-access 403 among them, leave the provider's circuit closed, and the
next try, on another model of the same provider, is dispatched; the same refusal twice on a rung climbs the task; a
403 that says the key is invalid trips `auth_failure`; a 401 is unchanged. Checked by the tests in
`[[verify]]`.
