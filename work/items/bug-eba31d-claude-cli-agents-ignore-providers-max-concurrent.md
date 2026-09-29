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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/provider/claude_cli.rs", "crates/roko-agent/src/codex_agent.rs::with_provider_semaphores"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'provider_semaphores.acquire\\|semaphores.acquire' crates/roko-agent/src/claude_cli_agent.rs && cargo test -p roko-agent --lib claude_cli_agent::tests::claude_cli_waits_for_provider_permit"
+++

Per-provider concurrency caps (`[providers.<id>] max_concurrent`) are enforced through `ProviderSemaphores`. `CodexAgent` accepts them (`with_provider_semaphores`); the Claude CLI agent does not. With parallel plans (`max_parallel_plans` above 1) or parallel waves, every `claude_cli` task spawns its process immediately, whatever the configured cap. That invites session-limit refusals and rate limiting on the most-used provider.

Fix: acquire the provider permit in the Claude CLI agent, or centrally in the factory, before spawning.
