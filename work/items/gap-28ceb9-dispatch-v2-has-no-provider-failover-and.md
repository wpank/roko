+++
id = "gap-28ceb9"
kind = "gap"
title = "dispatch_v2 has no provider failover and no usage-exhausted error class"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/dispatch_v2"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c58c7c2ba"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::classify_provider_error", "crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover", "crates/roko-learn/src/provider_failover.rs::Failover", "crates/roko-cli/src/serve_runtime.rs::dispatch_bench_prompt", "crates/roko-acp/src/bridge_events/failover.rs::forward_attempt_events"]
links = { depends_on = [], blocks = [], related = ["bug-35379d", "find-229e9c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'detect_provider_exhaustion' crates/roko-cli/src/dispatch_v2.rs && cargo test -p roko-cli --lib classify_provider_error_detects_usage_exhaustion && cargo test -p roko-learn --lib provider_failover && cargo test -p roko-cli --lib tests_provider_failover && cargo test -p roko-acp --lib bridge_events::failover && cargo test -p roko-acp --test provider_failover"
+++

Graph plan runs switch to a fallback model when the planned provider is blocked or exhausted (`run_bridge_with_failover`). `dispatch_v2` (`AgentDispatcherV2`) has no failover, and its `classify_provider_error` knows only `insufficient_credits`, `rate_limit`, `timeout`, `server_error` and `unknown`, with no class for session or usage-limit exhaustion. Callers that dispatch through it, which the e5 audit identified as the serve chat and ACP paths, fail on an exhausted provider instead of moving to a configured fallback.

Fix: share the Graph path's exhaustion detection and failover policy, recording the substitution (see bug-35379d).

## Notes

- 2026-10-01 (wk-tiers): partial, on work/bug-7cdce7; cargo verification deferred to the batch check.
  - Done (the error class): `dispatch_v2::classify_provider_error` checks `detect_provider_exhaustion` first and
    returns `provider_exhausted`, which the registry records as `ErrorClass::Exhausted`. So a session or
    usage-limit refusal trips the breaker at once. Before, it counted as `unknown`, or as billing when it mentioned
    a quota. On the Graph path, `run_bridge_with_failover` still refines the quarantine to the reported reset.
    Test: `classify_provider_error_detects_usage_exhaustion`.
  - Left (the failover): serve chat and ACP still dispatch through `AgentDispatcherV2` with no fallback. Sharing
    the Graph policy means lifting `failover_candidates`/`failover_model` out of `GraphTaskDispatcher` into
    dispatch_v2 (or a shared module), then calling it from those entry points and recording the substitution
    (bug-35379d). Those entry points are in serve/ACP, outside this packet.
- 2026-10-02 (wk-tiers): the remainder, failover on serve and ACP, on work/bug-7cdce7; cargo verification deferred to
  the batch check.
  - The premise was partly off. Serve's one-shot dispatch (`CliRuntime::run_once`, used by plan chat, PRDs, research,
    templates, jobs, shared runs and bench) and ACP prompts dispatch through `ModelCallService`, not
    `AgentDispatcherV2`. `dispatch_v2::dispatch_via_model_call_service` has no caller.
  - `ModelCallService` treats an exhaustion as terminal, so it never reached a fallback.
  - Shared policy: `roko_learn::provider_failover`.
    - `failover_candidates` gives the Graph path's order: the refused slug on its family's other providers, then
      `[routing] fallback_models`, `agent.fallback_model` and `agent.default_model`. The Graph path's
      `failover_candidates` now delegates to it, along with the candidate type and the helpers it shares.
    - `Failover` adds the rest:
      - it passes over a disabled, keyless or quarantined planned provider before any call;
      - it quarantines a usage exhaustion until its reset (`record_exhaustion`);
      - it moves to the next usable candidate, logging the substitution at WARN, and keeps the refusals;
      - a pinned call never moves.
  - Serve (`serve_runtime::dispatch_bench_prompt`):
    - Mechanics: a failover loop, one persisted health registry per prompt, and `[routing]` read from the workspace
      config. A synthesized `slug@provider` candidate is called by its key.
    - Pin: `run_once_with_config`'s model override.
    - Tests: `tests_provider_failover::a_one_shot_prompt_fails_over_from_an_exhausted_provider` and
      `a_pinned_one_shot_prompt_does_not_fail_over`, with fake Claude CLIs.
  - ACP (`bridge_events/mod.rs`, single-agent prompts):
    - Same-turn failover: `failover::forward_attempt_events` holds back an exhaustion failure that ends an attempt
      before it showed any answer, and the prompt reruns on the next model.
    - Pin: an explicit model selection or an experiment's model.
    - Attribution: the episode, efficiency event and router observation name the model that ran.
    - Anthropic providers that only an environment key synthesizes are disabled for failover, since ACP's Anthropic
      path needs a configured provider.
    - Tests: `bridge_events::failover` unit tests, plus `tests/provider_failover.rs`. In the integration test the
      first prompt fails over in the same turn, the second skips the quarantined provider, and an explicit
      selection does not move.
  - Not covered: the workflow-pipeline and slash-command ACP paths keep their own model handling.
