# Reactive Agent Mode

**Priority**: P2
**Size**: L (3-5 days)

---

## Problem

Agents in roko today have two modes: **Ephemeral** (run once and stop) and **Persistent**
(run continuously until manually stopped). Neither fits agents whose work is fundamentally
reactive — wait for something to happen, do focused work, go back to sleep.

A PR review agent, a Monday morning sweep, or a chain-event responder needs to be alive
and waiting but doing nothing 99% of the time. With `Persistent`, the agent runs a full
tick loop burning compute polling for events that may never arrive. With `Ephemeral`, the
agent must be externally re-triggered each time and carries no state between runs.

**Concrete examples currently under-served:**
- **PR reviewer**: sleep until a GitHub webhook fires, review the diff, post a comment, sleep.
- **Monday sweep**: cron `0 9 * * MON` wakes the agent to triage issues, then sleeps.
- **Chain monitor**: fire on every `Transfer` event matching a wallet, run a risk check.
- **Inbox responder**: wake on Bus message, compose a reply, sleep.

### What already exists

- `AgentMode::Reactive` is already defined in the config enum at
  `crates/roko-core/src/config/agent.rs`.
- The Trigger runtime (E31, 8/8 complete) has all seven trigger sources including
  Webhook, Cron, ChainEvent, and Bus. `TriggerBinding` registration and `TriggerEvent`
  delivery are its public interface.
- E23 (agent cognitive autonomy, 10/10) provides `CorticalState`, lifecycle type-state,
  and `AgentModeOwner` machinery.

**What is missing:** the runtime behavior. The sleep/wake loop, trigger registration,
cortical state persistence, and lifecycle wiring do not exist.

---

## Solution

A reactive agent:
1. **Registers** its triggers with the Trigger runtime at startup.
2. **Sleeps** — no tick loop, no compute, no LLM calls.
3. **Wakes** when a registered trigger fires, receiving the payload as its first observation.
4. **Runs** the full 9-step pipeline against the trigger payload.
5. **Sleeps again** after the pipeline completes or an idle timeout elapses.

Cortical state is serialized to `.roko/agents/{name}/cortical.json` after each wake
cycle so the agent accumulates knowledge across invocations without a persistent process.

### New types

**`AgentTriggerSpec`** — per-agent trigger definition in `roko.toml`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTriggerSpec {
    Webhook { path: String },
    Schedule { cron: String, timezone: Option<String> },
    ChainEvent { chain_id: u64, contract: String, event_signature: String },
    Message { topic: String },
}
```

**`ReactiveAgentHandle`** — runtime handle for a sleeping reactive agent:

```rust
pub struct ReactiveAgentHandle {
    pub agent_name: String,
    pub trigger_ids: Vec<TriggerId>,
    pub state: ReactiveAgentState,
    pub cortical_path: PathBuf,
    pub idle_timeout: Duration,
}

pub enum ReactiveAgentState { Sleeping, Awake, Draining }
```

### Config example

```toml
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
  { type = "schedule", cron = "0 9 * * MON" },
]
reactive_idle_timeout_secs = 300
```

### Wake cycle flow

```
Trigger fires → ReactiveAgentHandle.state → Awake
  → Load cortical.json if < 1 hour old (else start fresh)
  → Hydrate AgentRuntime with saved CorticalState
  → Deliver trigger payload as first Observation
  → Execute 9-step pipeline
  → Serialize CorticalState → cortical.json
  → state → Sleeping
```

---

## Where to implement

| Phase | Component | Path |
|---|---|---|
| 1 (0.5d) | Config types | `crates/roko-core/src/config/agent.rs` — add `AgentTriggerSpec`, `triggers` field, validation |
| 2 (0.5d) | Handle types | `crates/roko-agent/src/lifecycle.rs` — add `ReactiveAgentState`, `ReactiveAgentHandle` |
| 3 (1d) | Trigger registration | `crates/roko-cli/src/agent_serve.rs` — convert specs → bindings, register with E31 |
| 4 (1.5d) | Wake/sleep loop | `crates/roko-agent/src/lifecycle.rs` — `run_reactive()` async function |
| 5 (0.5d) | Runner integration | `crates/roko-cli/src/runner/event_loop.rs` — inject Message trigger for reactive agents |
| 6 (0.5d) | CLI status | `crates/roko-cli/src/commands/agent.rs` — show `sleeping` state |

---

## Acceptance criteria

### Functional
1. A reactive agent registers triggers at `roko serve` startup and logs the count.
2. `roko agent status --name X` shows `sleeping` when idle.
3. Webhook trigger wakes the agent within 100ms of the HTTP request.
4. Cron trigger fires on schedule (verifiable with second-granularity test expression).
5. After wake cycle, agent returns to `sleeping`.
6. Cortical state persists to `.roko/agents/{name}/cortical.json` and reloads if < 1h old.
7. `triggers = []` with `mode = "reactive"` rejected at config validation.
8. `roko agent list` shows `sleeping` for idle reactive agents.
9. On shutdown, all trigger bindings are unregistered.

### Non-functional
10. Sleeping agent consumes zero LLM tokens and zero background CPU.
11. Wake-to-first-LLM-call latency under 500ms for local webhook triggers.

### Negative cases
12. When `roko serve` is not running, `roko plan run` dispatches via Ephemeral fallback.
13. Stale cortical state (>1h old) is discarded silently.
14. A trigger firing while the agent is Awake is queued, not dropped or duplicated.

### Out of scope
- Persistent sub-mode with reactive wakeups (already works via Persistent + message queue).
- Trigger authoring UI (config-file-only).
- Horizontal reactive scaling (Phase 2+).
- Chain event delivery (depends on live chain connectivity, a product residual).
