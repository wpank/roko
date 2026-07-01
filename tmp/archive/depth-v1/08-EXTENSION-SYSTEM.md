# 08 — Extension System

> Extension = Block that intercepts another Block's pipeline.

**Subsumes**: Extension trait (arch-03), hook chain, extension loading, extension registry.

**Source**: Refactored from `tmp/architecture/03-extensions.md` using unified vocabulary.

---

## 1. What Is an Extension?

An **Extension** is a Block that intercepts another Block's pipeline. It does not replace or wrap the target Block — it hooks into the runtime's execution path at well-defined points, observing and modifying Signals as they flow through.

Extensions are the specialization mechanism for Agents. Two Agents with the same 9-step pipeline Graph but different Extension chains behave differently. A coding Agent loads `git`, `compiler`, and `test-runner` Extensions. A research Agent loads `web-search`, `citation`, and `summarizer` Extensions. The pipeline is the same; the interceptors differ.

### Key distinction from other specializations

| Specialization | Relationship to target |
|---|---|
| **Lens** | Observes without modifying (read-only via Observe protocol) |
| **Connector** | Provides bidirectional I/O with external systems (Connect protocol) |
| **Extension** | Intercepts and modifies the target's pipeline (hook-based interception) |

A Lens cannot change what it sees. A Connector provides capability. An Extension modifies behavior.

---

## 2. Extension Manifest

Every Extension declares its identity, layer, dependencies, and optionality through a manifest.

```rust
pub struct ExtensionManifest {
    /// Stable identifier. kebab-case.
    pub name: String,
    /// Semver of this Extension.
    pub version: Version,
    /// Human-readable description.
    pub description: String,
    /// Which layer this Extension operates in.
    pub layer: ExtensionLayer,
    /// Other Extensions this one requires (within the same layer).
    pub depends_on: Vec<String>,
    /// If true, agent continues when this Extension fails to load.
    pub optional: bool,
    /// Tags for filtering and discovery.
    pub tags: Vec<String>,
}
```

### TOML authoring

```toml
# .roko/extensions/report-writer/manifest.toml
[extension]
name = "report-writer"
version = "1.0.0"
description = "Generates structured reports from agent output"
layer = "action"
depends_on = ["citation", "summarizer"]
optional = true
tags = ["reporting", "documentation"]
```

---

## 3. The 8 Layers

Extensions are organized into 8 layers. Each layer has a defined purpose and fires at a specific point in the Agent's 9-step pipeline.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionLayer {
    Foundation,   // L0 — Lifecycle setup and teardown
    Perception,   // L1 — Input filtering and observation
    Memory,       // L2 — Knowledge access interception
    Cognition,    // L3 — LLM call modification
    Action,       // L4 — Tool and action interception
    Social,       // L5 — Communication interception
    Meta,         // L6 — Self-monitoring
    Recovery,     // L7 — Error handling
}
```

### Layer-to-pipeline mapping

| Layer | # | Pipeline Steps | When it fires |
|---|---|---|---|
| Foundation | L0 | Init / Shutdown | Agent startup and teardown |
| Perception | L1 | Step 1 (Observe) | After observations gathered, before analysis |
| Memory | L2 | Step 2 (Retrieve) | During knowledge retrieval and storage |
| Cognition | L3 | Steps 4-5 (Gate, Simulate) | Before/after LLM inference, during gating |
| Action | L4 | Step 7 (Execute) | Before/after tool calls and actions |
| Social | L5 | Steps 1, 7 (Observe, Execute) | On message send/receive |
| Meta | L6 | Step 9 (Reflect) | During reflection and cost accounting |
| Recovery | L7 | Any step (on error) | On errors and budget exhaustion |

---

## 4. The 22 Hooks

The Extension trait provides 22 hooks across the 8 layers. All hooks have default no-op implementations — an Extension only overrides the hooks it needs.

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this Extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // ── L0: Foundation ─────────────────────────────────────────
    /// Called once at Agent startup. Initialize state, open connections.
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        Ok(())
    }
    /// Called once at Agent shutdown. Release resources.
    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        Ok(())
    }

    // ── L1: Perception ─────────────────────────────────────────
    /// Observe the current observation set (read-write). Can add or remove observations.
    async fn on_observe(&self, obs: &mut Observations) -> Result<()> {
        Ok(())
    }
    /// Filter an incoming message. Can pass, drop, or transform it.
    async fn filter_input(&self, input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // ── L2: Memory ─────────────────────────────────────────────
    /// Intercept knowledge retrieval. Can re-rank, filter, or augment results.
    async fn on_retrieve(
        &self,
        query: &str,
        results: &mut Vec<Signal>,
    ) -> Result<()> {
        Ok(())
    }
    /// Intercept knowledge storage. Can validate or annotate before persistence.
    async fn on_store(&self, signal: &Signal) -> Result<()> {
        Ok(())
    }

    // ── L3: Cognition ──────────────────────────────────────────
    /// Modify the inference request before it reaches the LLM.
    async fn pre_inference(&self, req: &mut InferenceRequest) -> Result<()> {
        Ok(())
    }
    /// Modify the inference response after LLM returns.
    async fn post_inference(&self, resp: &mut InferenceResponse) -> Result<()> {
        Ok(())
    }
    /// Intercept gate decisions. Can modify pass/fail/confidence.
    async fn on_gate(&self, decision: &mut GateDecision) -> Result<()> {
        Ok(())
    }

    // ── L4: Action ─────────────────────────────────────────────
    /// Intercept an action before execution. Can proceed, block, or modify.
    async fn pre_action(&self, action: &mut Action) -> Result<ActionDecision> {
        Ok(ActionDecision::Proceed)
    }
    /// Observe an action after execution. Read-only access to result.
    async fn post_action(&self, action: &Action, result: &ActionResult) -> Result<()> {
        Ok(())
    }
    /// Intercept a tool call. Can allow, block, or substitute.
    async fn on_tool_call(&self, call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    // ── L5: Social ─────────────────────────────────────────────
    /// Intercept outgoing messages. Can modify content, recipients.
    async fn on_message_send(&self, msg: &mut AgentMessage) -> Result<()> {
        Ok(())
    }
    /// Observe incoming messages. Read-only.
    async fn on_message_receive(&self, msg: &AgentMessage) -> Result<()> {
        Ok(())
    }

    // ── L6: Meta ───────────────────────────────────────────────
    /// React to the Agent's cortical state during reflection.
    /// Can request goal changes, belief updates, attention shifts.
    async fn on_reflect(&self, state: &CorticalState) -> Result<Vec<Adjustment>> {
        Ok(vec![])
    }
    /// Observe cost/usage updates.
    async fn on_cost_update(&self, usage: &Usage) -> Result<()> {
        Ok(())
    }

    // ── L7: Recovery ───────────────────────────────────────────
    /// Handle an error. Can propagate, retry, ignore, or escalate.
    async fn on_error(&self, error: &AgentError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Propagate)
    }
    /// Handle budget exhaustion. Can sleepwalk, stop, or request more.
    async fn on_budget_exceeded(&self, usage: &Usage) -> Result<BudgetAction> {
        Ok(BudgetAction::Sleepwalk)
    }
}
```

### Hook count by layer

| Layer | Hooks | Count |
|---|---|---|
| L0 Foundation | `on_init`, `on_shutdown` | 2 |
| L1 Perception | `on_observe`, `filter_input` | 2 |
| L2 Memory | `on_retrieve`, `on_store` | 2 |
| L3 Cognition | `pre_inference`, `post_inference`, `on_gate` | 3 |
| L4 Action | `pre_action`, `post_action`, `on_tool_call` | 3 |
| L5 Social | `on_message_send`, `on_message_receive` | 2 |
| L6 Meta | `on_reflect`, `on_cost_update` | 2 |
| L7 Recovery | `on_error`, `on_budget_exceeded` | 2 |
| **Total** | | **22** |

---

## 5. Decision Enums

Six hooks return decision values that control pipeline behavior. All other hooks return `Result<()>` (observation-only).

### FilterDecision (L1: Perception)

Returned by `filter_input()`. Controls whether an incoming message reaches the Agent's pipeline.

```rust
pub enum FilterDecision {
    /// Message passes through unchanged.
    Pass,
    /// Message is silently discarded. Logged for debugging.
    Drop,
    /// Message is replaced with a transformed version.
    Transform(AgentMessage),
}
```

**Behavioral consequence**: `Drop` causes the message to never reach the Agent's pipeline. The runtime logs `"Message dropped by extension {name}"` at DEBUG level. The sender receives no notification.

### ActionDecision (L4: Action)

Returned by `pre_action()`. Controls whether an action executes.

```rust
pub enum ActionDecision {
    /// Action executes normally.
    Proceed,
    /// Action is halted. Not an error — an intentional veto.
    Block { reason: String },
    /// Action is replaced with a modified version.
    Modify(Action),
}
```

**Behavioral consequence**: `Block` halts the action but the Agent continues. The Agent receives `"Action blocked by {extension_name}: {reason}"` in its next turn, allowing it to choose an alternative. This is the primary mechanism for safety Extensions to prevent dangerous operations.

### ToolDecision (L4: Action)

Returned by `on_tool_call()`. Controls whether a specific tool call executes.

```rust
pub enum ToolDecision {
    /// Tool call proceeds as requested.
    Allow,
    /// Tool call is blocked. Logged, Agent is notified.
    Block { reason: String },
    /// Tool call is transparently replaced with a different call.
    Substitute(ToolCall),
}
```

**Behavioral consequence**: `Substitute` replaces the original tool call transparently. The Agent sees the substitute's result as if it had made the substitute call originally. This enables tool wrapping (e.g., replacing `rm -rf` with a safer alternative).

### RecoveryAction (L7: Recovery)

Returned by `on_error()`. Controls error handling behavior.

```rust
pub enum RecoveryAction {
    /// Error propagates up the call stack (default).
    Propagate,
    /// Retry the failed operation.
    Retry,
    /// Suppress the error. Agent continues as if nothing happened.
    Ignore,
    /// Escalate with a custom message (e.g., notify human).
    Escalate(String),
}
```

### BudgetAction (L7: Recovery)

Returned by `on_budget_exceeded()`. Controls behavior when the Agent's budget runs out.

```rust
pub enum BudgetAction {
    /// Enter sleepwalk mode: observe and reflect only, no LLM calls.
    Sleepwalk,
    /// Shut down the Agent gracefully.
    Stop,
    /// Request additional budget (amount in microdollars).
    RequestMore(u64),
}
```

### Adjustment (L6: Meta)

Returned by `on_reflect()`. Requests modifications to the Agent's cortical state.

```rust
pub enum Adjustment {
    /// Replace or add a goal in the Agent's goal set.
    SetGoal(Goal),
    /// Update a belief key-value pair.
    UpdateBelief(String, f64),
    /// Shift the Agent's attention focus.
    ShiftAttention(String),
}
```

---

## 6. AgentContext

Extensions receive `&AgentContext` for read access to Agent state. This is **read-only**. Extensions that need to modify Agent behavior do so through their return values (decision enums), not by mutating context.

```rust
pub struct AgentContext {
    /// Unique Agent identifier.
    pub agent_id: String,
    /// Domain profile (e.g., "coding", "research", "trading").
    pub profile: DomainProfile,
    /// Current Agent mode (Ephemeral, Persistent, Reactive).
    pub mode: AgentMode,
    /// Current adaptive clock regime (Calm, Normal, Volatile, Crisis).
    pub regime: Regime,
    /// Remaining budget in microdollars.
    pub budget_remaining: u64,
    /// Total episodes logged by this Agent.
    pub episode_count: u64,
    /// Full Agent configuration (read-only).
    pub config: Arc<AgentConfig>,
}
```

The `on_init` and `on_shutdown` hooks receive `&mut AgentContext` — these are the only hooks that can modify the context, and only during lifecycle transitions.

---

## 7. Hook Execution Order

Per tick, Extensions fire in **layer order**: L0 (Foundation) through L7 (Recovery). Within a layer, Extensions fire in **dependency order** first (topological sort), then **config order** (the order they appear in the `extensions = [...]` array in `roko.toml`).

### Example execution trace

```
Tick #42 execution:

  L0 Foundation:   [git.on_init, compiler.on_init]           <- config order
  L1 Perception:   [git.on_observe, web-search.on_observe]
  L2 Memory:       [neuro-store.on_retrieve]
  L3 Cognition:    [safety.pre_inference, compiler.post_inference]
  L4 Action:       [git.pre_action, test-runner.post_action]
  L5 Social:       [slack.on_message_send]
  L6 Meta:         [cost-tracker.on_cost_update]
  L7 Recovery:     [circuit-breaker.on_error]
```

### Short-circuiting

Decision hooks can short-circuit the chain:

- `FilterDecision::Drop` — remaining perception Extensions still fire (they see the drop), but the message is discarded after the chain completes.
- `ActionDecision::Block` — remaining action Extensions in the chain are **skipped**. The block is final.
- `ToolDecision::Block` — same as ActionDecision::Block. Chain is skipped.

Non-decision hooks (returning `Result<()>`) never short-circuit. All Extensions in the layer fire regardless.

---

## 8. Fault Isolation

If one Extension's hook returns `Err`, the runtime logs the error and continues to the next Extension. A buggy optional Extension cannot take down the Agent.

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

### Rules

1. **Hook errors are not Agent errors.** A single Extension failure does not trigger the Recovery layer. The error is logged and the pipeline continues.
2. **Decision defaults apply on error.** If `pre_action` errors, the default `ActionDecision::Proceed` is used. If `on_error` errors, the default `RecoveryAction::Propagate` is used.
3. **Repeated failures trigger circuit breaking.** If an Extension's hooks fail 5 times consecutively, the runtime disables it for the remainder of the session and logs a warning:

```
[WARN] Extension "flaky-ext" disabled after 5 consecutive hook failures.
       Agent continues with reduced extension chain.
```

4. **Required vs optional distinction is for loading only.** Once loaded, all Extensions follow the same fault isolation rules. The `optional` flag controls startup behavior, not runtime behavior.

---

## 9. Hook Timeout

All Extension hooks timeout after **5 seconds**. This is currently hardcoded.

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

A timeout is treated as a hook error and follows the fault isolation rules in section 8. The Extension's hook is cancelled via the CancellationToken, and the next Extension in the chain fires.

### Future enhancement

If timeout behavior becomes a problem, the first enhancement would be a per-Extension `timeout_ms` field in `manifest.toml`:

```toml
[extension]
name = "slow-analyzer"
timeout_ms = 15000  # 15 seconds (default: 5000)
```

This is not currently implemented. Keep it simple until proven needed.

---

## 10. Extension Loading and Discovery

Extensions are loaded from three sources, checked in order.

### Sources

| Source | Location | Format | Priority |
|---|---|---|---|
| **Built-in** | Compiled into the roko binary | Rust code (static dispatch) | 1st (always available) |
| **Local** | `.roko/extensions/{name}/` | Compiled `.so` (Linux), `.dylib` (macOS) | 2nd |
| **Registry** | Fetched from relay extension registry | Downloaded to `.roko/extensions/`, then loaded as local | 3rd (on first use) |

### Load order

1. Built-in Extensions load first. These are always available and cannot fail to load.
2. Local Extensions from disk. The runtime scans `.roko/extensions/` for manifest files.
3. Registry Extensions are fetched only if referenced in config but not found locally.

### Registry fetch flow

```
Config references "vuln-scanner"
         |
         v
Check .roko/extensions/vuln-scanner/
         |
    found --> Load .so/.dylib
         |
    not found
         |
         v
GET {relay_url}/registry/extensions/vuln-scanner
         |
         v
Download to .roko/extensions/vuln-scanner/
         |
         v
Verify SHA-256 checksum from registry manifest
         |
         v
Load .so/.dylib
```

### Error handling

```toml
# roko.toml
[[agents]]
name = "coder-1"
extensions = [
  { name = "git",           optional = false },  # abort on load failure (default)
  { name = "custom-linter", optional = true },    # skip with warning on load failure
]
```

| `optional` | Load failure behavior |
|---|---|
| `false` (default) | Agent startup aborts with an error. The Agent cannot function without this Extension. |
| `true` | Log a warning and continue startup without it. The Agent operates with a reduced Extension chain. |

---

## 11. Dependency Resolution

Extensions can declare dependencies on other Extensions within the same layer.

```toml
# .roko/extensions/report-writer/manifest.toml
[extension]
name = "report-writer"
layer = "action"
depends_on = ["citation", "summarizer"]
```

### Resolution rules

1. **Within-layer topological sort.** On load, the runtime performs a topological sort of Extensions within each layer. If `report-writer` depends on `citation`, then `citation` hooks always fire before `report-writer` hooks.

2. **Cyclic dependency is a startup error.**

```
Error: Cyclic extension dependency detected: report-writer -> citation -> report-writer
       Remove the cycle or merge the extensions.
```

3. **Cross-layer dependencies are not supported.** Extensions in different layers already have a fixed execution order (L0 before L1 before L2, etc.). A Memory-layer Extension that needs Foundation-layer setup gets it automatically through layer ordering.

4. **Missing dependency behavior depends on optionality.** If `report-writer` depends on `citation` and `citation` fails to load:
   - If `report-writer` is `optional = true`: skip it with a warning.
   - If `report-writer` is `optional = false`: abort Agent startup.

---

## 12. Extension as Block

In the unified vocabulary, an Extension is a Block specialization. It conforms to the Block trait with additional interception metadata.

```rust
impl Block for MyExtension {
    fn name(&self) -> &str { "my-extension" }
    fn version(&self) -> &Version { &self.version }
    fn description(&self) -> &str { "Intercepts action pipeline for safety" }
    fn input_schema(&self) -> &TypeSchema { &TypeSchema::Signal { kind: None } }
    fn output_schema(&self) -> &TypeSchema { &TypeSchema::Signal { kind: None } }
    fn capabilities(&self) -> &[Capability] { &self.required_capabilities }
    fn protocols(&self) -> &[Protocol] { &[] }  // Extensions use hooks, not protocols

    async fn run(&self, input: BlockInput, ctx: &BlockContext) -> Result<BlockOutput, BlockError> {
        // Extensions are not invoked via run().
        // Their hooks are called by the runtime at the appropriate pipeline points.
        Err(BlockError::LogicError {
            reason: "Extensions are invoked via hooks, not run()".into()
        })
    }
}
```

The `run()` method is not the primary execution path for Extensions. The runtime calls hooks directly. However, conforming to Block allows Extensions to participate in the type system, capability model, and discovery mechanisms.

### Extension-specific metadata

Beyond the Block trait, Extensions carry:

- `layer: ExtensionLayer` — which pipeline layer they intercept
- `depends_on: Vec<String>` — dependency declarations
- `optional: bool` — loading behavior
- The 22 hook methods — the actual interception points

---

## 13. Built-in Extensions

Roko ships with several built-in Extensions that are always available.

| Extension | Layer | Hooks Used | Purpose |
|---|---|---|---|
| `git` | L4 Action | `on_init`, `pre_action`, `post_action` | Git operations: commit, push, branch |
| `compiler` | L4 Action | `on_init`, `post_inference`, `post_action` | Compile checks after code changes |
| `test-runner` | L4 Action | `post_action` | Run tests after code changes |
| `safety` | L3 Cognition | `pre_inference`, `on_gate` | Safety checks on LLM requests |
| `cost-tracker` | L6 Meta | `on_cost_update`, `on_reflect` | Budget monitoring and alerts |
| `circuit-breaker` | L7 Recovery | `on_error`, `on_budget_exceeded` | Repeated failure detection |
| `neuro-store` | L2 Memory | `on_retrieve`, `on_store` | Knowledge store integration |
| `web-search` | L1 Perception | `on_observe` | Web search during observation |

### Domain profiles

Domain profiles provide default Extension sets. When an Agent declares `profile = "coding"`, it automatically loads the coding profile's Extensions unless overridden.

```toml
# Built-in profile defaults (not user-configurable)
[profiles.coding]
extensions = ["git", "compiler", "test-runner", "safety", "cost-tracker", "circuit-breaker"]

[profiles.research]
extensions = ["web-search", "citation", "summarizer", "safety", "cost-tracker", "circuit-breaker"]

[profiles.trading]
extensions = ["chain-reader", "risk-manager", "safety", "cost-tracker", "circuit-breaker"]
```

Profile Extensions are loaded with `optional = false` by default — they are considered essential for the domain.

---

## 14. TOML Configuration

### Agent-level Extension config

```toml
# roko.toml
[[agents]]
name = "coder-1"
profile = "coding"
extensions = [
  # Override: add custom linter (optional)
  { name = "custom-linter", optional = true },
  # Override: add report writer with config
  { name = "report-writer", optional = true, config = { format = "markdown", max_length = 5000 } },
]

# Profile extensions are loaded first, then agent-level extensions.
# To disable a profile extension, use:
# disable_extensions = ["test-runner"]
```

### Extension-level config

Extensions receive their config section from `roko.toml`:

```toml
[extensions.custom-linter]
rules_path = ".roko/lint-rules.yaml"
severity = "warning"
timeout_ms = 10000
```

The runtime passes this config to the Extension's `on_init` hook via `AgentContext.config`.

---

## 15. Relationship to Other Specializations

### Extension vs Lens

| Aspect | Extension | Lens |
|---|---|---|
| Protocol | Hook-based interception | Observe protocol |
| Modifies target? | Yes (via decision enums) | Never (read-only) |
| Can block actions? | Yes (`ActionDecision::Block`) | No |
| Can transform data? | Yes (`FilterDecision::Transform`) | No |
| Failure impact | Logged, Agent continues | Logged, target unaffected |
| Use case | Behavior modification | Telemetry and monitoring |

### Extension vs Connector

| Aspect | Extension | Connector |
|---|---|---|
| Protocol | Hook-based interception | Connect protocol |
| Purpose | Modify Agent behavior | Provide external I/O |
| Agent relationship | Agent *loads* Extensions | Agent *uses* Connectors |
| Lifecycle | Tied to Agent lifecycle | Independent connection lifecycle |
| Composition | Extensions can *wrap* Connectors | Connectors cannot intercept Extensions |

An Extension can register Connectors in its `on_init()` hook, bridging the two specializations.

### Extension + Connector composition

```rust
// A rate-limiting Extension wraps a Connector
struct RateLimitExt {
    connector: Box<dyn Connect>,
    limiter: RateLimiter,
}

impl Extension for RateLimitExt {
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Action }

    async fn on_tool_call(&self, call: &mut ToolCall) -> Result<ToolDecision> {
        if call.targets_connector(&self.connector) {
            if self.limiter.check().is_err() {
                return Ok(ToolDecision::Block {
                    reason: "Rate limit exceeded".into()
                });
            }
        }
        Ok(ToolDecision::Allow)
    }
}
```

---

## 16. Extension Lifecycle

### Startup sequence

```
1. Load built-in Extensions
2. Load local Extensions from .roko/extensions/
3. Fetch registry Extensions (if referenced but not local)
4. Validate SHA-256 checksums (registry Extensions)
5. Sort Extensions per layer:
   a. Topological sort by depends_on
   b. Stable sort by config order (within dependency groups)
6. For each Extension in layer order:
   a. Call on_init(&mut AgentContext)
   b. If on_init fails:
      - optional = true  --> log warning, remove from chain
      - optional = false  --> abort Agent startup
7. Agent pipeline begins
```

### Shutdown sequence

```
1. Agent pipeline stops (no new ticks)
2. For each Extension in REVERSE layer order (L7 -> L0):
   a. Call on_shutdown(&mut AgentContext)
   b. Log any errors (but continue shutdown)
3. Unload dynamic libraries
4. Release resources
```

Shutdown hooks fire in reverse order so that higher-layer Extensions (Recovery, Meta) clean up before lower-layer Extensions (Foundation) release resources they depend on.

---

## 17. Signal Flow Through Extensions

Extensions intercept Signals at specific pipeline points. Here is the flow for a typical Agent tick showing where Extension hooks fire:

```
                         Signals In
                             |
                             v
  L1: filter_input() --- [OBSERVE] --- L1: on_observe()
                             |
                             v
  L2: on_retrieve() ---- [RETRIEVE] --- L2: on_store()
                             |
                             v
                         [ANALYZE]
                             |
                             v
  L3: on_gate() --------- [GATE]
                             |
                             v
  L3: pre_inference() -- [SIMULATE] -- L3: post_inference()
                             |
                             v
                         [VALIDATE]
                             |
                             v
  L4: pre_action() ----- [EXECUTE] --- L4: post_action()
  L4: on_tool_call()         |         L5: on_message_send()
  L5: on_message_receive()   |
                             v
                          [VERIFY]
                             |
                             v
  L6: on_reflect() ----- [REFLECT] --- L6: on_cost_update()
                             |
                             v
                        Signals Out

  On error at any step:
  L7: on_error() or on_budget_exceeded()
```

---

## 18. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Extension trait compiles with all 22 hooks and default no-op implementations | `cargo check` on Extension trait definition |
| `FilterDecision::Drop` silently discards message (logged at DEBUG) | Unit test: send message through Extension with Drop, verify not delivered |
| `FilterDecision::Transform` replaces message content | Unit test: send message, verify transformed version received |
| `ActionDecision::Block` halts action but Agent continues | Integration test: block an action, verify next tick executes normally |
| `ActionDecision::Modify` replaces action transparently | Unit test: modify action, verify modified version executed |
| `ToolDecision::Block` prevents tool call, Agent notified | Unit test: block tool call, verify Agent receives block reason |
| `ToolDecision::Substitute` transparently replaces tool call | Unit test: substitute tool, verify Agent sees substitute's result |
| `RecoveryAction::Retry` retries the failed operation | Integration test: fail then retry, verify second attempt |
| `BudgetAction::Sleepwalk` restricts Agent to observe+reflect | Integration test: exceed budget, verify no LLM calls in sleepwalk |
| Hook timeout at 5 seconds triggers warning, next Extension continues | Integration test: hook that sleeps 10s, verify timeout logged |
| Missing optional Extension logs warning, Agent starts normally | Start Agent with missing optional Extension, verify startup succeeds |
| Missing required Extension aborts Agent startup with clear error | Start Agent with missing required Extension, verify startup fails |
| Cyclic dependency detected at startup with cycle description | Config two Extensions with circular depends_on, verify error message |
| Extensions sorted: by layer, then by dependency (topological), then by config order | Unit test: 6 Extensions across 3 layers with deps, verify execution order |
| Built-in Extensions always load first | Verify built-in available before local scan |
| Registry fetch verifies SHA-256 checksum | Tamper with downloaded Extension, verify checksum failure |
| Shutdown hooks fire in reverse layer order | Integration test: log shutdown order, verify L7 before L0 |
| Extension hook errors do not crash the Agent | Integration test: hook that panics, verify Agent continues |
| 5 consecutive hook failures disable the Extension | Integration test: fail 5 times, verify Extension disabled |
| Decision defaults apply when hook errors | Integration test: pre_action errors, verify Proceed used |

---

## 19. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-core` | Extension trait definition, ExtensionLayer enum, decision enums |
| `roko-agent` | Extension loading, dependency resolution, hook dispatch, fault isolation |
| `roko-std` | Built-in Extension implementations (git, compiler, test-runner, etc.) |
| `roko-cli` | Extension configuration in roko.toml, profile defaults |
| `roko-serve` | Extension status endpoints (`GET /api/extensions`) |
