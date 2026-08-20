# 02 — Reactive Agent Mode

**Priority**: P2 — Enables event-driven agents without idle compute
**Size**: L (3-5 days)
**Crates**: `crates/roko-core` (config), `crates/roko-agent` (lifecycle), `crates/roko-cli` (agent_serve, runner)
**Depends on**: None (E31 Trigger runtime is already wired)

---

## Background

Roko is a Rust toolkit for building agents that develop software autonomously. Agents are dispatched by the runner, execute tasks via an LLM, and persist results. Some agents are meant to run once and stop (Ephemeral), while others run continuously in a loop (Persistent).

A third class of work does not fit either mode: agents that need to be "alive" but doing nothing until an external event occurs. A PR review agent should wake when a GitHub webhook fires, review the diff, post a comment, then go back to sleep. A Monday-morning sweep should run once each Monday at 9am. A chain-event monitor should fire once per matching on-chain Transfer event. With Persistent mode, the agent loops continuously, burning compute polling for events that rarely arrive. With Ephemeral, the agent must be re-triggered externally each time and carries no state between runs.

Roko already has all the building blocks needed:

- `AgentMode::Reactive` is already defined in `crates/roko-core/src/config/agent.rs` (line 109). The variant exists in the enum but has no runtime behavior.
- The Trigger runtime (E31, 8/8 complete) can register webhook, cron, chain event, and message Bus triggers. It delivers `TriggerEvent` payloads to registered handlers.
- E23 agent cognitive autonomy provides `CorticalState` (energy fields, behavioral vitality) and lifecycle type-state machinery in `crates/roko-agent/src/lifecycle.rs`.
- `crates/roko-cli/src/agent_serve.rs` is where `roko agent serve` starts per-agent HTTP sidecars — this is where reactive agents would register their triggers at startup.

What is missing is the *runtime behavior*: the sleep/wake loop, trigger registration at serve startup, cortical state serialization between wake cycles, and status reporting.

## Current State

1. **`AgentMode::Reactive`** exists at `crates/roko-core/src/config/agent.rs` line 109 as an enum variant in `AgentMode`. It has no associated behavior anywhere in the codebase beyond being a config value that can be parsed from TOML.

2. **`AgentDefinition`** in `crates/roko-core/src/config/schema.rs` (lines 1754-1765) has fields `name`, `domain`, `prompt`, `model`, `chain_rpc`, `enabled`. It does not have a `mode` field or a `triggers` field. These must be added.

3. **`crates/roko-agent/src/lifecycle.rs`** exports `AgentCoreManifest`, `DeploymentMode`, `DomainPlugin` and related types. It does not contain `ReactiveAgentState` or `ReactiveAgentHandle` — these must be added.

4. **`crates/roko-cli/src/agent_serve.rs`** (lines 1-80+ visible) handles `roko agent serve` and `roko agent create`. There is no trigger registration logic for reactive agents at serve startup.

5. **The Trigger runtime** is accessible via `roko-runtime` or `roko-conductor`. The binding interface uses `TriggerBinding` and `TriggerEvent`. The exact module path needs to be confirmed by searching for `TriggerBinding` in `crates/roko-runtime/` or `crates/roko-conductor/`.

6. **`.roko/agents/{name}/cortical.json`** path convention is not yet established. The `.roko/agents/` directory layout is used for agent manifests (`.roko/agents/{name}/manifest.toml`).

## Implementation Plan

### Phase 1 (0.5d): Config types

**File**: `crates/roko-core/src/config/schema.rs` — modify `AgentDefinition` struct (line 1754)

Add `mode` and `triggers` fields, plus `reactive_idle_timeout_secs`:

```rust
/// Agent definition for multi-agent startup via `roko up`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub domain: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub chain_rpc: Option<String>,
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
    // NEW FIELDS:
    /// Lifecycle mode for this agent.
    #[serde(default)]
    pub mode: AgentMode,
    /// Trigger sources for reactive agents. Required when mode = "reactive".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<AgentTriggerSpec>,
    /// How long after a wake cycle before the agent auto-sleeps (seconds).
    #[serde(default = "default_reactive_idle_timeout")]
    pub reactive_idle_timeout_secs: u64,
}

fn default_reactive_idle_timeout() -> u64 { 300 }
```

Also add `AgentMode` re-export to the `use super::agent::*` glob at the top of `schema.rs` (already covered by the glob).

Add `AgentTriggerSpec` to `crates/roko-core/src/config/agent.rs`:

```rust
/// Trigger source for a reactive agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTriggerSpec {
    /// HTTP webhook at the given path suffix.
    Webhook {
        path: String,
    },
    /// Cron schedule (IANA cron expression, 5 or 6 fields).
    Schedule {
        cron: String,
        /// IANA timezone name (e.g. "America/New_York"). Defaults to UTC.
        #[serde(default)]
        timezone: Option<String>,
    },
    /// On-chain event (requires live chain connectivity — see GAPS.md).
    ChainEvent {
        chain_id: u64,
        contract: String,
        event_signature: String,
    },
    /// Bus message on a named topic.
    Message {
        topic: String,
    },
}
```

Add `AgentTriggerSpec` to the `pub use` exports in `crates/roko-core/src/config/mod.rs`.

**Config validation**: in `crates/roko-core/src/config/validation.rs` or wherever `AgentDefinition` is validated, add a check: `if mode == Reactive && triggers.is_empty() { return Err("reactive agent requires at least one trigger") }`.

### Phase 2 (0.5d): Handle types in `crates/roko-agent/src/lifecycle.rs`

Add these types at the top of the existing file (after the existing imports):

```rust
/// Runtime state of a reactive agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactiveAgentState {
    /// Waiting for a trigger to fire. No compute running.
    Sleeping,
    /// Executing a wake cycle (pipeline running).
    Awake,
    /// Finishing in-flight work before sleeping.
    Draining,
}

/// Runtime handle for a sleeping reactive agent registered with the serve runtime.
pub struct ReactiveAgentHandle {
    /// Name from config (matches `AgentDefinition.name`).
    pub agent_name: String,
    /// IDs returned by the Trigger runtime when bindings were registered.
    pub trigger_ids: Vec<String>,
    /// Current lifecycle state.
    pub state: ReactiveAgentState,
    /// Path to persisted cortical state.
    pub cortical_path: std::path::PathBuf,
    /// How long after a wake cycle the agent stays in Draining before sleeping.
    pub idle_timeout: std::time::Duration,
}
```

Also add `run_reactive()` — an async function that implements the wake cycle:

```rust
/// Execute one reactive wake cycle for the given agent.
///
/// Loads cortical state from `handle.cortical_path` if < 1 hour old,
/// runs the 9-step pipeline against the trigger payload, serializes
/// updated cortical state back to disk, then returns.
pub async fn run_reactive(
    handle: &mut ReactiveAgentHandle,
    trigger_payload: serde_json::Value,
    config: &roko_core::config::schema::RokoConfig,
) -> anyhow::Result<()> {
    handle.state = ReactiveAgentState::Awake;

    // Load cortical state if fresh.
    let cortical = load_cortical_if_fresh(&handle.cortical_path);

    // Run the pipeline with the trigger payload as the first observation.
    // TODO: integrate with the runner dispatch path. For now, log and return.
    tracing::info!(
        agent = %handle.agent_name,
        trigger = ?trigger_payload,
        has_cortical = cortical.is_some(),
        "reactive wake cycle start"
    );

    // Serialize cortical state after the cycle.
    // (Full implementation: pass cortical through pipeline, serialize result.)
    handle.state = ReactiveAgentState::Sleeping;
    Ok(())
}

fn load_cortical_if_fresh(path: &std::path::Path) -> Option<serde_json::Value> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let age = std::time::SystemTime::now()
        .duration_since(modified)
        .unwrap_or(std::time::Duration::MAX);
    if age > std::time::Duration::from_secs(3600) {
        return None; // stale — discard
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}
```

### Phase 3 (1d): Trigger registration in `crates/roko-cli/src/agent_serve.rs`

At serve startup (where `AgentDefinition` entries are read from config), add reactive agent registration:

```rust
// In the function that starts the agent serve runtime, after iterating config.agents:
for agent_def in &config.agents {
    if agent_def.mode == roko_core::config::AgentMode::Reactive && agent_def.enabled {
        register_reactive_agent(agent_def, &roko_dir, &trigger_runtime).await?;
    }
}
```

Implement `register_reactive_agent`:

```rust
async fn register_reactive_agent(
    def: &AgentDefinition,
    roko_dir: &Path,
    // trigger_runtime: &TriggerRuntime — inject from serve context
) -> anyhow::Result<ReactiveAgentHandle> {
    let cortical_path = roko_dir.join("agents").join(&def.name).join("cortical.json");
    std::fs::create_dir_all(cortical_path.parent().unwrap())?;

    let mut trigger_ids = Vec::new();
    for trigger_spec in &def.triggers {
        // Convert AgentTriggerSpec → TriggerBinding and register.
        // The exact API depends on the Trigger runtime's public interface.
        // Search for `TriggerBinding` in crates/roko-runtime/ or crates/roko-conductor/.
        let id = register_trigger(trigger_spec, &def.name).await?;
        trigger_ids.push(id);
    }

    tracing::info!(
        agent = %def.name,
        trigger_count = trigger_ids.len(),
        "reactive agent registered"
    );

    Ok(ReactiveAgentHandle {
        agent_name: def.name.clone(),
        trigger_ids,
        state: ReactiveAgentState::Sleeping,
        cortical_path,
        idle_timeout: Duration::from_secs(def.reactive_idle_timeout_secs),
    })
}
```

When a trigger fires (via the Trigger runtime's event delivery mechanism), call `run_reactive(handle, payload, config).await`.

On shutdown, iterate all `ReactiveAgentHandle` entries and deregister their `trigger_ids`.

### Phase 4 (1.5d): Wake/sleep loop

The wake/sleep loop runs inside the registered trigger callback. When the Trigger runtime delivers a `TriggerEvent`:

1. Find the `ReactiveAgentHandle` by matching the trigger ID.
2. If state is `Awake`, queue the event (do not drop it or start a second wake cycle).
3. If state is `Sleeping`, call `run_reactive()` with the trigger payload.
4. After `run_reactive()` returns, set state to `Sleeping`, flush cortical state.

### Phase 5 (0.5d): Status reporting in `crates/roko-cli/src/commands/agent.rs`

Locate the `roko agent status --name X` and `roko agent list` command handlers. Extend them to show `sleeping` for reactive agents in `Sleeping` state, and `awake` for those in `Awake` state. The serve runtime needs to expose the `ReactiveAgentHandle` state through a shared `Arc<RwLock<HashMap<String, ReactiveAgentHandle>>>`.

### Config example (for testing)

In the user's `roko.toml`:

```toml
[[agents]]
name = "pr-reviewer"
domain = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
  { type = "schedule", cron = "0 9 * * MON" },
]
reactive_idle_timeout_secs = 300
```

## Acceptance Criteria

1. `AgentDefinition` in TOML with `mode = "reactive"` and `triggers = [...]` deserializes without error. A `mode = "reactive"` entry with empty `triggers` produces a config validation error.

2. `roko serve` startup logs the count of registered triggers for each reactive agent (e.g., `reactive agent "pr-reviewer" registered, 2 triggers`).

3. `roko agent status --name pr-reviewer` shows `sleeping` when the agent is between wake cycles.

4. A webhook trigger (type = "webhook") wakes the agent within 100ms of the HTTP request arriving at the serve endpoint. Verifiable with an integration test that POSTs to the webhook path and asserts the wake cycle started.

5. A cron trigger (type = "schedule") fires on schedule. Verifiable with a short-interval cron expression (e.g., `*/1 * * * *` = every minute) in a test environment.

6. After a wake cycle, the agent returns to `sleeping` state and `cortical.json` is updated with the cycle timestamp.

7. Cortical state older than 1 hour is silently discarded at wake time; a fresh cycle starts.

8. A trigger firing while the agent is in `Awake` state queues the event (does not run a second parallel wake cycle).

9. `roko agent list` shows `sleeping` for idle reactive agents.

10. On `roko serve` shutdown, all trigger bindings are cleanly deregistered (no dangling webhook handlers).

11. Sleeping reactive agent consumes zero LLM tokens and zero polling CPU.

## Verification Checklist

- [ ] `cargo build --workspace` passes after adding `AgentTriggerSpec` and new `AgentDefinition` fields
- [ ] `cargo test -p roko-core config` passes — TOML round-trip for `AgentDefinition` with `mode = "reactive"` and trigger specs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean
- [ ] Add a TOML deserialization unit test in `schema.rs` tests section: parse a `[[agents]]` block with `mode = "reactive"` and two trigger specs, assert all fields deserialize correctly
- [ ] Add a validation unit test: `mode = "reactive"` with empty `triggers` returns an error
- [ ] Run `cargo run -p roko-cli -- agent list` with a reactive agent in `roko.toml` — should show the agent with `sleeping` status (may show `unknown` if serve is not running — that is acceptable)
- [ ] Check that `cortical.json` is written to `.roko/agents/{name}/cortical.json` after a wake cycle completes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/config/agent.rs` | Add `AgentTriggerSpec` enum |
| `crates/roko-core/src/config/schema.rs` | Add `mode`, `triggers`, `reactive_idle_timeout_secs` fields to `AgentDefinition` (line 1754) |
| `crates/roko-core/src/config/mod.rs` | Add `AgentTriggerSpec` to pub exports |
| `crates/roko-core/src/config/validation.rs` | Add reactive agent trigger validation |
| `crates/roko-agent/src/lifecycle.rs` | Add `ReactiveAgentState`, `ReactiveAgentHandle`, `run_reactive()`, `load_cortical_if_fresh()` |
| `crates/roko-cli/src/agent_serve.rs` | Add `register_reactive_agent()` called at serve startup; trigger event → wake dispatch |
| `crates/roko-cli/src/commands/agent.rs` | Extend `agent status` and `agent list` to show reactive state |
