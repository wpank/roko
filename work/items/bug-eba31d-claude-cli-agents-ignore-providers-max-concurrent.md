+++
id = "bug-eba31d"
kind = "bug"
title = "Claude CLI agents ignore [providers.*] max_concurrent"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude_cli"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/provider/claude_cli.rs", "crates/roko-agent/src/codex_agent.rs::with_provider_semaphores"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'provider_semaphores.acquire\\|semaphores.acquire' crates/roko-agent/src/claude_cli_agent.rs && cargo test -p roko-agent --lib claude_cli_agent::tests::claude_cli_waits_for_provider_permit"
+++

Per-provider concurrency caps (`[providers.<id>] max_concurrent`) are enforced through `ProviderSemaphores`. `CodexAgent` accepts them (`with_provider_semaphores`); the Claude CLI agent does not. With parallel plans (`max_parallel_plans` above 1) or parallel waves, every `claude_cli` task spawns its process immediately, whatever the configured cap. That invites session-limit refusals and rate limiting on the most-used provider.

Fix: acquire the provider permit in the Claude CLI agent, or centrally in the factory, before spawning.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7cdce7; cargo verification deferred to the batch check.
  - `ClaudeCliAgent::with_provider_semaphores` takes the shared `ProviderSemaphores` and provider id, as
    `CodexAgent` does. `run_impl` waits for the provider's permit before it builds and spawns the command, and
    holds it until the run ends.
  - `ClaudeCliAdapter::create_agent` passes `options.provider_semaphores` with `model.provider`. On the Graph path
    those are the dispatcher's shared semaphores, so `[providers.claude_cli] max_concurrent` (default 10) now caps
    concurrent `claude` processes.
  - Test: `claude_cli_waits_for_provider_permit`. With the only permit held, a run stays waiting and spawns nothing;
    once the permit is freed it runs and succeeds.
  - Follow-up, not fixed here: the stall watchdog counts a Claude CLI call as silent from its start, after a 30 s
    first-output grace, and a queued call is no exception. With a low cap and long tasks, an attempt queued for
    more than `task_stall_secs` (300 s) is cancelled as stalled and burns a retry. It needs a "queued" signal the
    watchdog honours, or the permit taken before the watchdog's call start.
