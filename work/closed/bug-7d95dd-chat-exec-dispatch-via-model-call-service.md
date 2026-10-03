+++
id = "bug-7d95dd"
kind = "bug"
title = "Chat/exec/dispatch-via-model-call-service provider-health writes lose the real error class, and ACP's exhaustion message has no login hint"
status = "superseded"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/learning-helpers", "roko-learn/model-call-feedback", "roko-acp"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "f74890b4b"
source = "wave-6 follow-up reports 2026-10-03 (gap-d90a93)"
discovered_from = "gap-d90a93"
anchors = ["crates/roko-cli/src/learning_helpers.rs::record_persisted_provider_health", "crates/roko-learn/src/model_call_feedback.rs::record_provider_health_at", "crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-cli/src/commands/config_cmd.rs::cmd_provider_reset_health"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-9ca6d7" }

[[verify]]
command = "grep -rqw 'fn chat_provider_failure_persists_its_real_error_class' crates/roko-cli/ && cargo test -p roko-cli chat_provider_failure_persists_its_real_error_class"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:27:59Z"
forced = false
evidence = "Duplicate of bug-9ca6d7 (the older item; keep that ID). Both independently found the same root cause (record_persisted_provider_health's bool-only wrapper hardcoding ErrorClass::Unknown) for the same three call sites, filed minutes apart by concurrent agents working the same wave-6 batch. bug-9ca6d7 is more complete (cross-references bug-52c48f, documents why it excluded the other two health-bundle facets). My additional finding not yet in bug-9ca6d7 (ACP's exact 'no usable provider' message location) is being added there as a Notes line instead."
+++

## Problem

Three residuals from `gap-d90a93` (done — it added `roko config providers reset-health`), confirmed in code
at HEAD:

1. **Three call sites still record every failure as `ErrorClass::Unknown`, even when a real cause is known.**
   `crates/roko-cli/src/chat_session.rs` (`record_chat_provider_health`, lines 887-897) and
   `crates/roko-cli/src/agent_exec.rs` (line 384) both call `learning_helpers::record_persisted_provider_health`
   (`crates/roko-cli/src/learning_helpers.rs:65-73`), a thin `bool`-only wrapper with no error-class parameter
   at all. It calls `roko_learn::model_call_feedback::record_provider_health_for_workdir`, which calls
   `record_provider_health_at` (`model_call_feedback.rs:268-269`) — whose own doc comment says plainly "a
   failure as `ErrorClass::Unknown`". `dispatch_via_model_call_service`
   (`crates/roko-cli/src/dispatch_v2.rs:93`) imports the same `record_persisted_provider_health` helper
   (line 96) for the same reason. Contrast the Graph-dispatch path, `record_agent_dispatch_feedback`
   (`dispatch_v2.rs:2032-2047`), which builds a real `ModelCallFeedback.error_class` from
   `ProviderHealthOutcome::of(result)` and reaches the full `record_provider_outcome_at`/`ErrorClass::from_kind`
   path (`model_call_feedback.rs:226-240`) — so the capability to classify exists in this codebase, just isn't
   wired to these three callers, all of which have the provider's result (a `chat`/interactive turn or an
   exec'd agent run) in hand and could classify it the same way.
2. **ACP's "no usable provider" message has no login/credentials hint.** `crates/roko-acp/src/bridge_events/mod.rs:788`:
   `anyhow::anyhow!("no usable provider for the prompt: {why}")` — a bare `{why}`. Compare Graph dispatch's
   own exhaustion message, `failover.rs::no_usable_provider` (~line 721-802), which calls `credentials_hint`
   (~line 804-816) for each refusal: a provider-specific fix (e.g. "`X` rejected its credentials: <fix>, then
   run `roko config providers reset-health X` to use it before its skip ends"). ACP's path never calls
   anything like it. I did **not** find an equivalent "no usable provider" string in `roko-serve/src/dispatch.rs`
   at all (it has session-budget-exhaustion messages, not provider-exhaustion ones) — roko-serve's dispatch
   path may delegate to the same CLI machinery and inherit the good message, or may have its own gap under a
   different message string; this needs to be confirmed at implementation time rather than assumed from this
   search.
3. **A mid-run `reset-health` only takes effect on the next run.** `roko config providers reset-health`
   (`commands/config_cmd.rs:1206-1209`, `cmd_provider_reset_health`) edits the persisted
   `provider-health.json` on disk directly. A plan run already in progress has very likely already loaded that
   registry into an in-process structure (the failover/dispatch path's own health tracker) and does not re-read
   the file mid-run, so clearing the quarantine while a run is active has no effect on that run's own circuit
   state — only a fresh process picks up the cleared file. Unconfirmed whether any in-process reload path
   exists; if not, this is a real limitation worth documenting (and possibly fixing) rather than a bug per se.

## Why it matters

Goal `truth`: items 1 and 2 both degrade an operator's ability to diagnose *why* a provider went unhealthy —
item 1 erases the distinction between an auth failure, a rate limit, and a genuine outage in the persisted
health record for three real call paths (interactive chat, direct agent exec, and the model-call-service CLI
path); item 2 means an ACP client (or, possibly, a served session) hits a dead end with no actionable next
step exactly when `gap-d90a93`'s own fix (`reset-health`) would help most. Item 3 is a smaller, documentation
or follow-up-fix matter: an operator who runs `reset-health` mid-run reasonably expects it to un-stick the
run they're watching, and it won't.

## Where

- `crates/roko-cli/src/learning_helpers.rs::record_persisted_provider_health` (bool-only signature).
- `crates/roko-cli/src/chat_session.rs::record_chat_provider_health`; `crates/roko-cli/src/agent_exec.rs:384`;
  `crates/roko-cli/src/dispatch_v2.rs::dispatch_via_model_call_service`.
- `crates/roko-learn/src/model_call_feedback.rs` (`record_provider_health_at` vs. the full
  `record_provider_outcome_at`/`ErrorClass::from_kind` path `record_provider_health` already uses).
- `crates/roko-acp/src/bridge_events/mod.rs:788` vs. `crates/roko-cli/src/graph_task_dispatch/failover.rs::no_usable_provider`/`credentials_hint`.
- `crates/roko-cli/src/commands/config_cmd.rs::cmd_provider_reset_health`.

## Current state

Unfixed on all three. `gap-d90a93`'s own fix (the `reset-health` command) exists and is tested
(`reset_health_clears_a_persisted_quarantine`), but none of these three residuals were in its scope.

## Plan

1. Give `record_persisted_provider_health` (or its three callers) a real error classification to pass through
   to `record_provider_outcome_at`, using the same classifier `record_agent_dispatch_feedback` already uses
   (`roko_agent::provider::error_classify`/`ProviderHealthOutcome::of`-equivalent logic for a chat/exec result).
2. Give ACP's "no usable provider" error the same `credentials_hint`-style guidance Graph dispatch's
   `no_usable_provider` already builds (factor `credentials_hint` into something ACP can call, or duplicate its
   shape); check roko-serve's own dispatch-exhaustion path for the same gap before assuming it's clean.
3. For the mid-run `reset-health` limitation: either make the in-process health tracker re-read
   `provider-health.json` (or accept a live signal) before refusing a dispatch, or document the limitation
   explicitly in `cmd_provider_reset_health`'s own help text / doc comment ("takes effect on the next run").

## Done when

- A chat, agent-exec, or `dispatch_via_model_call_service` failure with a known cause (auth, rate limit, etc.)
  persists that real `ErrorClass`, not `Unknown`.
- ACP's "no usable provider" error names the same kind of fix Graph dispatch's does.
- The mid-run `reset-health` limitation is either fixed or explicitly documented.
- The `[[verify]]` command passes.

## Notes

- Three related but independently fixable facets of the same provider-health bundle; split into separate
  items later if any one turns out to need materially different reviewers/timing.
- roko-serve's own exhaustion-message path was not found under the string "no usable" — confirm it either
  shares ACP's gap or Graph dispatch's good message before writing the fix for part 2.
