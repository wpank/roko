# Warm Agent Spawning

**Origin**: `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 6 —
"Warm agent spawning" (lines 191–215)
**Status**: Backlog
**Priority**: P3 — performance optimization; not blocking correctness
**Size**: M (2–3 days)

---

## Problem statement

Every phase transition in the runner involves a cold agent spawn. Between the
implementer completing and the reviewer starting, the runner serialises the gate
result, selects a provider, forks a subprocess (or opens a connection), performs
an MCP handshake, and waits for the first token. This takes 5–15 seconds on
typical provider and hardware combinations. For a plan with 10 tasks, each
passing through implement → gate → review → merge, that idle time accumulates
to 1.5–5 minutes of pure waiting.

The solution specified in the original gap is a `WarmPool`: a `HashMap<AgentRole,
WarmAgent>` of pre-spawned agents that initialise during gate execution so the
next phase's agent is ready when the gate completes.

The workspace has substantial scaffolding for this:

- `crates/roko-cli/src/dispatch/warm_pool.rs` — `WarmPool`, `WarmAgent`,
  `WarmPoolStats` with full LRU/TTL/capacity container semantics, exhaustively
  tested.
- `crates/roko-cli/src/runner/event_loop.rs` line 10800 — `WarmPool: pre-spawn`
  block that inserts a `WarmAgent` placeholder into the pool when a gate starts.
- `crates/roko-cli/src/runner/event_loop.rs` line 4116 — promote path: calls
  `warm_pool().take(next_role)` on gate pass.
- `crates/roko-cli/src/runner/event_loop.rs` line 4395 — evict path: calls
  `evict_expired()` on gate failure.

What is **not** implemented is the actual subprocess spawn. The pre-spawn block
(line 10800) inserts a placeholder `WarmAgent` struct into the pool container but
comments explicitly state: _"real provider spawn happens in the dispatcher when
`take()` is called and the slot is promoted."_ `take()` returns the placeholder
struct; the dispatcher then cold-spawns from it. The 5–15 s saving does not
materialise because the spawn has merely been deferred, not front-loaded.

The promotion path (line 4116) receives the `WarmAgent` id but does not use it
to locate a live process handle — there is no live process to locate. Real
warm-agent spawning requires the dispatcher to actually fork and initialise the
provider subprocess during gate execution, store a live handle indexed by the
`WarmAgent.id`, and look it up on promote.

---

## Proposed solution

Add a real background-spawn path to the dispatcher. The key invariant: the warm
slot must hold a live, initialised provider connection — not a placeholder struct.

**Phase 1 — Live handle storage.** Extend `WarmPool` (or add a parallel
`WarmHandleRegistry` in `crates/roko-cli/src/dispatch/`) to store a
`Box<dyn LiveAgentHandle>` indexed by the `WarmAgent.id`. `LiveAgentHandle` is a
trait:

```rust
pub trait LiveAgentHandle: Send + Sync {
    /// Send the task prompt and start streaming the response.
    fn start_task(&mut self, prompt: &str) -> Result<AgentStream, AgentError>;
    /// Gracefully shut down the underlying process or connection.
    fn shutdown(&mut self);
}
```

Concrete implementations: one per provider family (CLI subprocess wrapping
`tokio::process::Child`, HTTP persistent connection pooling via `reqwest`).

**Phase 2 — Background spawn during gate.** Replace the placeholder-insertion
block at event_loop line 10800 with an actual `tokio::spawn` that:
1. Selects a provider via the cascade router (same logic as cold dispatch, but
   using the next-phase role).
2. Forks the subprocess or opens the HTTP connection.
3. Completes the MCP/protocol handshake.
4. Stores the handle in the `WarmHandleRegistry` under the `WarmAgent.id`.

The `tokio::spawn` runs concurrently with the gate pipeline. Gate execution is
not blocked; if the spawn fails, the promotion path falls through to a cold spawn
gracefully.

**Phase 3 — Promote path.** Update the promotion block at event_loop line 4116
to look up the handle in `WarmHandleRegistry` by `WarmAgent.id`. If found, use
it directly. If not (spawn still in flight or failed), fall back to cold spawn.
Remove the handle from the registry after promotion to prevent double-use.

**Phase 4 — Evict path.** Update event_loop line 4395 and the TTL eviction
timer to call `handle.shutdown()` on evicted handles before dropping them,
preventing process leaks.

Provider scope for initial implementation: CLI subprocess providers (Claude CLI,
Cursor CLI, Gemini CLI) — these have the highest cold-spawn cost and clearest
subprocess lifetime. HTTP providers (Anthropic API, OpenAI-compat) can follow as
a separate PR since they use connection pooling and benefit less.

---

## Implementation location

| File | Change |
|---|---|
| `crates/roko-cli/src/dispatch/warm_pool.rs` | Add `WarmHandleRegistry` (or extend `WarmPool`) to store live `Box<dyn LiveAgentHandle>` by id |
| `crates/roko-cli/src/dispatch/mod.rs` | Export `LiveAgentHandle` trait and `WarmHandleRegistry` |
| `crates/roko-cli/src/runner/event_loop.rs` line 10800 | Replace placeholder insert with `tokio::spawn` that performs real provider init and stores handle |
| `crates/roko-cli/src/runner/event_loop.rs` line 4116 | Promote: look up handle by id; use live handle or fall back to cold spawn |
| `crates/roko-cli/src/runner/event_loop.rs` line 4395 | Evict: call `handle.shutdown()` before dropping |
| `crates/roko-agent/src/provider/claude_cli.rs` | Implement `LiveAgentHandle` for the Claude CLI subprocess |

---

## Acceptance criteria

1. When a task gate starts, a background `tokio::spawn` initiates a real provider
   subprocess for the next-phase role and stores the live handle; this is
   observable via a `debug!("warm_pool: live handle stored ...")` log line.
2. On gate pass, the promotion path retrieves the live handle (not a placeholder)
   and passes it to the dispatcher; the implementer-to-reviewer transition
   completes in under 500 ms as measured by a timing assertion in an integration
   test with a mock provider.
3. On gate failure, `handle.shutdown()` is called on every evicted handle;
   `ps aux` shows no leaked subprocesses after a plan run that exercises the
   failure path.
4. If the background spawn fails or is still in flight when promotion is
   attempted, the runner falls through to a standard cold spawn without error;
   no panic or visible UX regression.
5. `WarmPool` container tests in `warm_pool.rs` continue to pass without
   modification; the LRU/TTL/capacity semantics are unchanged.
6. `WarmPool::stats()` accurately reflects whether pooled slots hold live handles
   vs. placeholder structs (add a `live_handles` field to `WarmPoolStats`).

---

## References

- `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 6 (lines 191–215) —
  original specification: `WarmPool`, `pre_spawn_warm`, `promote_warm`,
  `evict_warm`, timing target (5–15 s saving), integration in event loop
- `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 6 source reference —
  `bardo/apps/mori/src/agent/mod.rs` `MultiAgentPool`, `pre_spawn_warm()`,
  `promote_warm()`, `evict_warm()` (reference implementation)
- `crates/roko-cli/src/dispatch/warm_pool.rs` — existing `WarmPool` /
  `WarmAgent` / `WarmPoolStats` container (fully implemented, all tests pass)
- `crates/roko-cli/src/runner/event_loop.rs` line 10800 — pre-spawn placeholder
  block (the stub to be replaced with a real spawn)
- `crates/roko-cli/src/runner/event_loop.rs` line 4116 — promotion path (to be
  extended to use live handles)
- `crates/roko-cli/src/runner/event_loop.rs` line 4395 — eviction path (to be
  extended to call `shutdown()`)
- `crates/roko-cli/src/dispatch/factory.rs` line 127 — `WarmPool::new(2)` in
  `SharedAgentFactory`; the `max_per_role` cap is already set
- `crates/roko-agent/src/provider/claude_cli.rs` — CLI subprocess provider;
  reference for what a real spawn entails (MCP handshake, stdio wiring)
