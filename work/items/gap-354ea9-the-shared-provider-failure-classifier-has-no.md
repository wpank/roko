+++
id = "gap-354ea9"
kind = "gap"
title = "The shared provider-failure classifier has no class for a connection that never opened (refused, DNS failure), so it reads as unknown"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-466060 (worker, 2026-10-27)"
discovered_from = "gap-466060: provider_error_kind's copy called these a timeout (P0-16); the shared classifier it now delegates to calls them unknown"
anchors = ["crates/roko-agent/src/provider/error_classify.rs::classify_failure_text", "crates/roko-acp/src/bridge_events/cost.rs::acp_learning_success"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-466060"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_connection_that_never_opened_has_a_class' crates/roko-agent/ && cargo test -p roko-agent --lib a_connection_that_never_opened_has_a_class"
+++

## Problem

`classify_failure_text` (`crates/roko-agent/src/provider/error_classify.rs`) has no class for a connection that never
opened. All of these classify as `unknown`:

- `tcp connect error: Connection refused (os error 61)`, as reqwest reports a refused connection;
- `dns error: failed to lookup address information`;
- `network is unreachable (os error 51)`;
- `connect ECONNREFUSED 127.0.0.1:443`;
- `getaddrinfo ENOTFOUND api.example.test`.

A connection that broke while a response streamed is a `server_error` (bug-9b65dc). One that never opened has no class.

## Why it matters

The class decides what provider health and learning do with the failure:

- ACP learning (`crates/roko-acp/src/bridge_events/cost.rs::acp_learning_success`) counts an `unknown` failure
  against the model. A provider it could not reach becomes a failed task for the model.
- The ladder records the try's infra class as `agent:unknown`
  (`crates/roko-cli/src/graph_task_dispatch/ladder.rs::infra_failure_class`).
- Provider health gives the failure the 5 s cooldown of an unknown failure.

Until gap-466060, the provider-call boundary (`model_call_service::provider_error_kind`) kept its own copy of the
classifier. That copy called these failures a `timeout` (its "P0-16" branch, so the health registry gave them a
timeout's cooldown). The boundary delegates to the shared classifier now, and
`model_call_service::tests::provider_error_kind_delegates_to_the_shared_classifier` pins `unknown` for these texts.

## Where

- `crates/roko-agent/src/provider/error_classify.rs::classify_failure_text`
- `crates/roko-acp/src/bridge_events/cost.rs::acp_learning_success`, which reads the class
- `crates/roko-learn/src/provider_health.rs::ErrorClass::from_kind`, which maps the class to a cooldown

## Plan

1. Decide the class. Either a `timeout`, as the old copy had it, so that `ErrorClass::from_kind` needs no change, or
   a new class such as `unreachable`, which `from_kind` and the cooldown table then learn.
2. Name it in `classify_failure_text` after `server_error`, from the words above.
3. Update the pinned cases in `provider_error_kind_delegates_to_the_shared_classifier`, and test the texts above in
   error_classify's tests (`a_connection_that_never_opened_has_a_class`).

## Done when

None of the texts above classifies as `unknown`, ACP learning no longer counts them against the model, and the
`[[verify]]` passes.
