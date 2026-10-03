+++
id = "bug-9ca6d7"
kind = "bug"
title = "chat_session, agent_exec and dispatch_via_model_call_service record provider health as Unknown, never the real failure class"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/learning-helpers", "roko-learn/model-call-feedback"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (health bundle, gap-d90a93)"
discovered_from = "gap-d90a93, related to bug-52c48f (same pattern, different call sites)"
anchors = ["crates/roko-cli/src/learning_helpers.rs::record_persisted_provider_health", "crates/roko-learn/src/model_call_feedback.rs::record_provider_health_at", "crates/roko-cli/src/dispatch_v2.rs::dispatch_via_model_call_service"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn classified_failure_through_agent_exec_is_not_recorded_as_unknown' crates/roko-cli/ && cargo test -p roko-cli classified_failure_through_agent_exec_is_not_recorded_as_unknown"
+++

## Problem

Three production call sites record a provider-call failure's health outcome through a path that always
classifies the failure as `ErrorClass::Unknown`, regardless of the real cause (auth failure, rate limit,
exhaustion, timeout, ...):

- `crates/roko-cli/src/agent_exec.rs:384`
- `crates/roko-cli/src/chat_session.rs:897` (via `record_chat_provider_health`, line 887)
- `crates/roko-cli/src/dispatch_v2.rs:196,204` (inside `dispatch_via_model_call_service`)

All three call `record_persisted_provider_health(workdir, provider, success: bool)`
(`crates/roko-cli/src/learning_helpers.rs:65-74`), whose signature takes only a `bool`. That function forwards to
`roko_learn::model_call_feedback::record_provider_health_for_workdir` (`model_call_feedback.rs:253-259`), which
forwards to `record_provider_health_at` (`model_call_feedback.rs:266-268`) — whose own doc comment says plainly:
"Persist one provider-health outcome under a `.roko/learn` directory, a failure as `ErrorClass::Unknown`." The
correctly-classified sibling, `record_provider_outcome_at(learn_dir, provider, success, error: ErrorClass)`, exists
right next to it (lines 276-296) and is what `health_hold`/`serve_runtime.rs` use after bug-52c48f's fix — but
none of these three call sites were updated to use it; they're all still on the bool-only wrapper.

## Why it matters

Goal: truth (honest provider health state). A quarantined/circuit-open provider whose health came through one of
these three paths carries no information about *why* it's down, so nothing downstream (operator messages, routing
decisions, `gap-d90a93`'s `reset-health` command's own reporting) can distinguish "needs a fresh login" from
"transient, will clear on its own" for calls made through chat, direct agent-exec, or the model-call-service
dispatch path. bug-52c48f fixed this for `health_hold`/`serve_runtime.rs`; these three are additional, uncovered
call sites doing the same thing by a different route.

## Where

- `crates/roko-cli/src/learning_helpers.rs::record_persisted_provider_health` (the bool-only wrapper all three call).
- `crates/roko-learn/src/model_call_feedback.rs::record_provider_health_for_workdir` and `::record_provider_health_at`
  (the two layers that hardcode `ErrorClass::Unknown`).
- The fix target, already correctly shaped: `crates/roko-learn/src/model_call_feedback.rs::record_provider_outcome_at`.
- Callers to update: `agent_exec.rs:384`, `chat_session.rs:887-897`, `dispatch_v2.rs:196,204`.

## Current state

Unfixed. `record_persisted_provider_health` has no `ErrorClass`-carrying variant for callers outside the Graph
dispatch path to use, so each of the three sites would need either a new overload taking a classified error, or to
call `record_provider_outcome_at` directly (which needs the workdir's `.roko/learn` path resolved the same way
`record_provider_health_for_workdir` does).

## Plan

1. Add a classified variant of `record_persisted_provider_health` in `learning_helpers.rs` (e.g.
   `record_persisted_provider_outcome(workdir, provider, success, error: ErrorClass)`) mirroring
   `record_provider_outcome_at`.
2. At each of the three call sites, classify the actual failure (the same classifier `bug-466060`/`gap-466060`
   unified, or whatever typed error each call site already has in scope) and pass it through, instead of calling
   the bool-only wrapper.
3. Add a regression test asserting a classified failure (e.g. an auth failure) through each path lands in
   `provider-health.json` with that class, not `Unknown`.

## Done when

- A classified failure through `agent_exec`, `chat_session`, or `dispatch_via_model_call_service` records its real
  `ErrorClass` in `.roko/learn/provider-health.json`, not `Unknown`.
- The `[[verify]]` command passes.

## Notes

- Distinct from bug-52c48f (done): that one covers `provider_failover::health_hold` and `serve_runtime.rs`
  recording `Unknown`; this covers three different call sites (chat, agent-exec, model-call-service dispatch) that
  reach the problem through the bool-only wrapper instead.
- Two other facets of the same "health bundle" report were checked and NOT included here: (1) serve's/ACP's
  "no usable model" error message — could not locate this exact phrase anywhere in `crates/roko-serve/` or
  `crates/roko-acp/` by several searches; needs the reporter's more specific pointer before it can be filed. (2) a
  mid-run `reset-health` only applying from the next run — already documented: `cmd_provider_reset_health`
  (`config_cmd.rs:1209-1218`, gap-d90a93) prints "a plan run already in progress keeps the provider health it
  loaded until it ends" on every use, so this is not a silent gap.

- 2026-10-03 (second pass, same wave-6 batch): the exact phrase isn't "no usable model", it's "no usable
  provider" — `crates/roko-acp/src/bridge_events/mod.rs:788`:
  `anyhow::anyhow!("no usable provider for the prompt: {why}")`, a bare `{why}` with no credentials/login
  guidance. Compare `crates/roko-cli/src/graph_task_dispatch/failover.rs::no_usable_provider` (~721-802), which
  calls `credentials_hint` (~804-816) per refusal for exactly this situation. I did not find an equivalent
  string in `crates/roko-serve/src/dispatch.rs` (only session-budget-exhaustion messages there) — roko-serve
  may delegate to the CLI path and inherit the good message, or may have its own gap under different wording;
  worth a closer look before writing the fix, but ACP's gap is confirmed and anchorable now.
