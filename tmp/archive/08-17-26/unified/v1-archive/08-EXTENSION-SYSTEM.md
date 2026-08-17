# 08 — Extension System

> Extension = Block that intercepts another Block's pipeline. Every data flow through an Extension is tagged with its capability provenance. Extensions cannot launder capabilities.

**Subsumes**: Extension trait (arch-03), hook chain, extension loading, extension registry.

**Source**: Refactored from `tmp/architecture/03-extensions.md` using unified vocabulary. Updated with CaMeL IFC integration, Pulse-layer clarification, and 5-tier SPI packaging.

---

## 1. What Is an Extension?

An **Extension** is a Block that intercepts another Block's pipeline. It does not replace or wrap the target Block — it hooks into the runtime's execution path at well-defined points, observing and modifying data as it flows through.

Extensions are the specialization mechanism for Agents. Two Agents with the same 9-step pipeline Graph but different Extension chains behave differently. A coding Agent loads `git`, `compiler`, and `test-runner` Extensions. A research Agent loads `web-search`, `citation`, and `summarizer` Extensions. The pipeline is the same; the interceptors differ.

### Key distinction from other specializations

| Specialization | Relationship to target |
|---|---|
| **Lens** | Observes without modifying (read-only via Observe protocol) |
| **Connector** | Provides bidirectional I/O with external systems (Connect protocol) |
| **Extension** | Intercepts and modifies the target's pipeline (hook-based interception) |

A Lens cannot change what it sees. A Connector provides capability. An Extension modifies behavior.

### Signal vs Pulse in Extensions

Extensions operate on **two mediums** depending on their layer (see [doc-01](01-SIGNAL.md) for full definitions):

- **L1 Perception** and **L5 Social** layers receive **Pulses** (ephemeral). Incoming messages, observations, and outgoing communications arrive as Pulses on the Bus. Extensions in these layers filter, transform, or observe Pulses before they are graduated to Signals (if the graduation policy selects them) or consumed ephemerally.
- **L2 Memory**, **L3 Cognition**, **L4 Action**, and **L6 Meta** layers operate on **Signals** (durable). Knowledge retrieval, inference requests, tool calls, and reflection all work with persisted, scored, lineage-tracked Signals.
- **L0 Foundation** and **L7 Recovery** are lifecycle layers that operate on neither medium directly — they manage Agent state and error handling.

This distinction matters: an L1 Extension that drops a Pulse has no audit trail (the Pulse was ephemeral). An L4 Extension that blocks an action operates on a Signal that exists in the audit DAG.

---

## 2. CaMeL IFC Integration

Every data flow through an Extension is tagged with its **capability provenance** via CaMeL information flow control. Extensions cannot launder capabilities — they cannot elevate the privilege of data that passes through them.

See [doc-17 (Security Model)](17-SECURITY-MODEL.md) for the full CaMeL IFC specification.

### Capability tagging rules

1. **Input tags propagate.** When a Pulse or Signal enters an Extension hook, it carries capability tags from its origin. An L1 `filter_input()` hook receives a Pulse tagged with `{source: external, trust: untrusted}`. The Extension cannot strip these tags.

2. **Extensions cannot elevate.** An Extension's output inherits the **intersection** of its own capability grants and its input's capability tags. If an Extension has `{trust: local}` and receives data tagged `{trust: untrusted}`, the output is tagged `{trust: untrusted}`. The lower trust level wins.

3. **Decision enums carry tags.** When `FilterDecision::Transform` replaces a message, the replacement inherits the original's capability tags plus the Extension's own provenance tag. When `ToolDecision::Substitute` replaces a tool call, the substitute carries both the original request's tags and the Extension's tags.

4. **Audit trail.** Every capability tag transformation is logged as a Pulse on `extension:{name}:ifc` topic. The CaMeL monitor (a Verify-protocol Block, see doc-17) subscribes to these topics and flags violations.

### What "no laundering" means in practice

```
Untrusted input Pulse
    |
    v
Extension "web-search" (L1 Perception)
    |  Cannot strip {trust: untrusted} tag
    |  Can ADD its own tag: {via: web-search}
    v
Output Pulse: {trust: untrusted, via: web-search}
    |
    v
Extension "summarizer" (L3 Cognition)
    |  Cannot elevate to {trust: local}
    |  Output inherits: {trust: untrusted, via: web-search, via: summarizer}
    v
Signal in context: still marked untrusted
```

A downstream Verify-protocol Block can see that this Signal originated from untrusted external data, regardless of how many Extensions processed it. The provenance chain is intact.

---

## 3. Extension Manifest

Every Extension declares its identity, layer, dependencies, optionality, and **packaging tier** through a manifest.

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
    /// Packaging tier (see doc-14 for the full 5-tier SPI).
    pub tier: PackageTier,
}
```

### Packaging tiers

Extensions follow the **5-tier SPI** defined in [doc-14 (Config and Authoring)](14-CONFIG-AND-AUTHORING.md). The tier determines sandboxing, distribution, and trust level:

| Tier | Extension Form | Sandboxing | Distribution |
|---|---|---|---|
| **1. Prompts** | Markdown/TOML front-matter declaring hook behavior | None (no execution) | Marketplace |
| **2. Config** | TOML profile bundles that configure built-in Extensions | None | Marketplace |
| **3. Declarative tools** | TOML manifests for subprocess/HTTP/MCP hooks, sandboxed | OS-level process isolation | Verified publishers |
| **4. WASM** | Compiled WASM implementing Extension hooks | WASM sandbox (fuel-metered) | Marketplace (recommended) |
| **5. Native Rust** | `impl Extension for MyExt` compiled in-tree | Process-level | In-tree only |

Most third-party Extensions should target Tier 4 (WASM). Tiers 1-3 cover common cases without writing code. Tier 5 is reserved for built-in Extensions and trusted in-tree plugins. See doc-14 for authoring details at each tier.

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
tier = "wasm"
tags = ["reporting", "documentation"]
```

---

## 4. The 8 Layers

Extensions are organized into 8 layers. Each layer has a defined purpose and fires at a specific point in the Agent's 9-step pipeline.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionLayer {
    Foundation,   // L0 — Lifecycle setup and teardown
    Perception,   // L1 — Input filtering and observation (Pulse medium)
    Memory,       // L2 — Knowledge access interception (Signal medium)
    Cognition,    // L3 — LLM call modification (Signal medium)
    Action,       // L4 — Tool and action interception (Signal medium)
    Social,       // L5 — Communication interception (Pulse medium)
    Meta,         // L6 — Self-monitoring (Signal medium)
    Recovery,     // L7 — Error handling
}
```

### Layer-to-pipeline mapping

| Layer | # | Pipeline Steps | When it fires | Medium |
|---|---|---|---|---|
| Foundation | L0 | Init / Shutdown | Agent startup and teardown | N/A (lifecycle) |
| Perception | L1 | Step 1 (Observe) | After observations gathered, before analysis | **Pulse** |
| Memory | L2 | Step 2 (Retrieve) | During knowledge retrieval and storage | Signal |
| Cognition | L3 | Steps 4-5 (Gate, Simulate) | Before/after LLM inference, during gating | Signal |
| Action | L4 | Step 7 (Execute) | Before/after tool calls and actions | Signal |
| Social | L5 | Steps 1, 7 (Observe, Execute) | On message send/receive | **Pulse** |
| Meta | L6 | Step 9 (Reflect) | During reflection and cost accounting | Signal |
| Recovery | L7 | Any step (on error) | On errors and budget exhaustion | N/A (error path) |

The Perception and Social layers operate on the React layer's medium — **Pulses** (see [doc-02](02-BLOCK.md) section 3.6). This is consistent with the React protocol operating on Pulses rather than Signals. Observations and communications are ephemeral events; they become Signals only through explicit graduation.

---

## 5. The 22 Hooks

The Extension trait provides 22 hooks across the 8 layers. All hooks have default no-op implementations — an Extension only overrides the hooks it needs.

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this Extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // -- L0: Foundation -----------------------------------------------
    /// Called once at Agent startup. Initialize state, open connections.
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        Ok(())
    }
    /// Called once at Agent shutdown. Release resources.
    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        Ok(())
    }

    // -- L1: Perception (Pulse medium) --------------------------------
    /// Observe the current observation set (read-write). Receives Pulses.
    /// Can add or remove observations before they are analyzed.
    async fn on_observe(&self, obs: &mut Observations) -> Result<()> {
        Ok(())
    }
    /// Filter an incoming message (Pulse). Can pass, drop, or transform.
    async fn filter_input(&self, input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // -- L2: Memory (Signal medium) -----------------------------------
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

    // -- L3: Cognition (Signal medium) --------------------------------
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

    // -- L4: Action (Signal medium) -----------------------------------
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

    // -- L5: Social (Pulse medium) ------------------------------------
    /// Intercept outgoing messages (Pulses). Can modify content, recipients.
    async fn on_message_send(&self, msg: &mut AgentMessage) -> Result<()> {
        Ok(())
    }
    /// Observe incoming messages (Pulses). Read-only.
    async fn on_message_receive(&self, msg: &AgentMessage) -> Result<()> {
        Ok(())
    }

    // -- L6: Meta (Signal medium) -------------------------------------
    /// React to the Agent's cortical state during reflection.
    /// Can request goal changes, belief updates, attention shifts.
    async fn on_reflect(&self, state: &CorticalState) -> Result<Vec<Adjustment>> {
        Ok(vec![])
    }
    /// Observe cost/usage updates.
    async fn on_cost_update(&self, usage: &Usage) -> Result<()> {
        Ok(())
    }

    // -- L7: Recovery -------------------------------------------------
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

| Layer | Hooks | Count | Medium |
|---|---|---|---|
| L0 Foundation | `on_init`, `on_shutdown` | 2 | N/A |
| L1 Perception | `on_observe`, `filter_input` | 2 | Pulse |
| L2 Memory | `on_retrieve`, `on_store` | 2 | Signal |
| L3 Cognition | `pre_inference`, `post_inference`, `on_gate` | 3 | Signal |
| L4 Action | `pre_action`, `post_action`, `on_tool_call` | 3 | Signal |
| L5 Social | `on_message_send`, `on_message_receive` | 2 | Pulse |
| L6 Meta | `on_reflect`, `on_cost_update` | 2 | Signal |
| L7 Recovery | `on_error`, `on_budget_exceeded` | 2 | N/A |
| **Total** | | **22** | |

---

## 6. Decision Enums

Six hooks return decision values that control pipeline behavior. All other hooks return `Result<()>` (observation-only).

### FilterDecision (L1: Perception)

Returned by `filter_input()`. Controls whether an incoming Pulse reaches the Agent's pipeline.

```rust
pub enum FilterDecision {
    /// Pulse passes through unchanged.
    Pass,
    /// Pulse is silently discarded. Logged for debugging.
    Drop,
    /// Pulse is replaced with a transformed version.
    /// CaMeL IFC: replacement inherits original's capability tags.
    Transform(AgentMessage),
}
```

**Behavioral consequence**: `Drop` causes the Pulse to never reach the Agent's pipeline. The runtime logs `"Message dropped by extension {name}"` at DEBUG level. The sender receives no notification.

**CaMeL IFC**: `Transform` carries the original Pulse's capability tags forward. The Extension's own provenance tag is appended. No capability elevation is possible.

### ActionDecision (L4: Action)

Returned by `pre_action()`. Controls whether an action executes.

```rust
pub enum ActionDecision {
    /// Action executes normally.
    Proceed,
    /// Action is halted. Not an error — an intentional veto.
    Block { reason: String },
    /// Action is replaced with a modified version.
    /// CaMeL IFC: replacement inherits original's capability tags.
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
    /// CaMeL IFC: substitute carries both original and Extension tags.
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

## 7. AgentContext

Extensions receive `&AgentContext` for read access to Agent state. This is **read-only**. Extensions that need to modify Agent behavior do so through their return values (decision enums), not by mutating context.

```rust
pub struct AgentContext {
    /// Unique Agent identifier.
    pub agent_id: String,
    /// Domain profile (complete cognitive posture, see doc-14).
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
    /// CaMeL capability tags for this Agent's data flows.
    pub capability_tags: CapabilityTagSet,
}
```

The `on_init` and `on_shutdown` hooks receive `&mut AgentContext` — these are the only hooks that can modify the context, and only during lifecycle transitions.

---

## 8. Hook Execution Order

Per tick, Extensions fire in **layer order**: L0 (Foundation) through L7 (Recovery). Within a layer, Extensions fire in **dependency order** first (topological sort), then **config order** (the order they appear in the `extensions = [...]` array in `roko.toml`).

### Example execution trace

```
Tick #42 execution:

  L0 Foundation:   [git.on_init, compiler.on_init]           <- config order
  L1 Perception:   [git.on_observe, web-search.on_observe]   <- Pulse medium
  L2 Memory:       [neuro-store.on_retrieve]                  <- Signal medium
  L3 Cognition:    [safety.pre_inference, compiler.post_inference]
  L4 Action:       [git.pre_action, test-runner.post_action]
  L5 Social:       [slack.on_message_send]                    <- Pulse medium
  L6 Meta:         [cost-tracker.on_cost_update]
  L7 Recovery:     [circuit-breaker.on_error]
```

### Short-circuiting

Decision hooks can short-circuit the chain:

- `FilterDecision::Drop` — remaining perception Extensions still fire (they see the drop), but the Pulse is discarded after the chain completes.
- `ActionDecision::Block` — remaining action Extensions in the chain are **skipped**. The block is final.
- `ToolDecision::Block` — same as ActionDecision::Block. Chain is skipped.

Non-decision hooks (returning `Result<()>`) never short-circuit. All Extensions in the layer fire regardless.

### CaMeL IFC during execution

Throughout the hook chain, capability tags propagate according to the rules in section 2. The runtime maintains a `TagContext` per tick that accumulates provenance as data flows through Extensions. This context is available to the CaMeL monitor for post-hoc audit.

---

## 9. Fault Isolation

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
5. **CaMeL tag propagation survives failures.** If an Extension hook fails, the runtime uses the input's capability tags as the output's tags (pass-through). No capability elevation occurs from error handling.

---

## 10. Hook Timeout

All Extension hooks timeout after **5 seconds**. This is currently hardcoded.

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

A timeout is treated as a hook error and follows the fault isolation rules in section 9. The Extension's hook is cancelled via the CancellationToken, and the next Extension in the chain fires.

### Future enhancement

If timeout behavior becomes a problem, the first enhancement would be a per-Extension `timeout_ms` field in `manifest.toml`:

```toml
[extension]
name = "slow-analyzer"
timeout_ms = 15000  # 15 seconds (default: 5000)
```

This is not currently implemented. Keep it simple until proven needed.

---

## 11. Extension Loading and Discovery

Extensions are loaded from three sources, checked in order.

### Sources

| Source | Location | Format | Priority |
|---|---|---|---|
| **Built-in** | Compiled into the roko binary | Rust code (static dispatch) — Tier 5 | 1st (always available) |
| **Local** | `.roko/extensions/{name}/` | Any tier (TOML manifest + implementation) | 2nd |
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
    found --> Load per tier (TOML / WASM / .so/.dylib)
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
Verify CaMeL capability declarations match advertised
         |
         v
Load per tier
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

## 12. Dependency Resolution

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

## 13. Extension as Block

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
- `tier: PackageTier` — packaging tier (see [doc-14](14-CONFIG-AND-AUTHORING.md))
- `capability_tags: CapabilityTagSet` — CaMeL IFC declarations (see [doc-17](17-SECURITY-MODEL.md))
- The 22 hook methods — the actual interception points

---

## 14. Built-in Extensions

Roko ships with several built-in Extensions (Tier 5 — Native Rust) that are always available.

| Extension | Layer | Hooks Used | Purpose |
|---|---|---|---|
| `git` | L4 Action | `on_init`, `pre_action`, `post_action` | Git operations: commit, push, branch |
| `compiler` | L4 Action | `on_init`, `post_inference`, `post_action` | Compile checks after code changes |
| `test-runner` | L4 Action | `post_action` | Run tests after code changes |
| `safety` | L3 Cognition | `pre_inference`, `on_gate` | Safety checks on LLM requests |
| `cost-tracker` | L6 Meta | `on_cost_update`, `on_reflect` | Budget monitoring and alerts |
| `circuit-breaker` | L7 Recovery | `on_error`, `on_budget_exceeded` | Repeated failure detection |
| `neuro-store` | L2 Memory | `on_retrieve`, `on_store` | Knowledge store integration |
| `web-search` | L1 Perception | `on_observe` | Web search during observation (Pulse medium) |
| `camel-monitor` | L6 Meta | `on_reflect`, `on_cost_update` | CaMeL IFC violation detection |

### Domain profiles

Domain profiles provide default Extension sets. When an Agent declares `profile = "coding"`, it automatically loads the coding profile's Extensions unless overridden. See [doc-14](14-CONFIG-AND-AUTHORING.md) for the full domain profile specification — profiles are complete cognitive postures, not just extension lists.

```toml
# Built-in profile defaults (not user-configurable)
[profiles.coding]
extensions = ["git", "compiler", "test-runner", "safety", "cost-tracker", "circuit-breaker", "camel-monitor"]

[profiles.research]
extensions = ["web-search", "citation", "summarizer", "safety", "cost-tracker", "circuit-breaker", "camel-monitor"]

[profiles.trading]
extensions = ["chain-reader", "risk-manager", "safety", "cost-tracker", "circuit-breaker", "camel-monitor"]
```

Profile Extensions are loaded with `optional = false` by default — they are considered essential for the domain.

---

## 15. TOML Configuration

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

## 16. Relationship to Other Specializations

### Extension vs Lens

| Aspect | Extension | Lens |
|---|---|---|
| Protocol | Hook-based interception | Observe protocol |
| Modifies target? | Yes (via decision enums) | Never (read-only) |
| Can block actions? | Yes (`ActionDecision::Block`) | No |
| Can transform data? | Yes (`FilterDecision::Transform`) | No |
| CaMeL IFC | Tags propagate through hooks | Tags propagate (read-only, no elevation risk) |
| Failure impact | Logged, Agent continues | Logged, target unaffected |
| Use case | Behavior modification | Telemetry and monitoring |

### Extension vs Connector

| Aspect | Extension | Connector |
|---|---|---|
| Protocol | Hook-based interception | Connect protocol |
| Purpose | Modify Agent behavior | Provide external I/O |
| Agent relationship | Agent *loads* Extensions | Agent *uses* Connectors |
| Lifecycle | Tied to Agent lifecycle | Independent connection lifecycle |
| CaMeL IFC | Tags propagated and checked per hook | Tags set at connection boundary |
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

## 17. Extension Lifecycle

### Startup sequence

```
1. Load built-in Extensions (Tier 5)
2. Load local Extensions from .roko/extensions/ (Tiers 1-4)
3. Fetch registry Extensions (if referenced but not local)
4. Validate SHA-256 checksums (registry Extensions)
5. Validate CaMeL capability declarations against Space grants
6. Sort Extensions per layer:
   a. Topological sort by depends_on
   b. Stable sort by config order (within dependency groups)
7. For each Extension in layer order:
   a. Call on_init(&mut AgentContext)
   b. If on_init fails:
      - optional = true  --> log warning, remove from chain
      - optional = false  --> abort Agent startup
8. Agent pipeline begins
```

### Shutdown sequence

```
1. Agent pipeline stops (no new ticks)
2. For each Extension in REVERSE layer order (L7 -> L0):
   a. Call on_shutdown(&mut AgentContext)
   b. Log any errors (but continue shutdown)
3. Unload dynamic libraries / WASM modules
4. Release resources
```

Shutdown hooks fire in reverse order so that higher-layer Extensions (Recovery, Meta) clean up before lower-layer Extensions (Foundation) release resources they depend on.

---

## 18. Signal Flow Through Extensions

Extensions intercept data at specific pipeline points. Here is the flow for a typical Agent tick showing where Extension hooks fire and which medium each layer operates on:

```
                         Pulses In (Bus)
                             |
                             v
  L1: filter_input() --- [OBSERVE] --- L1: on_observe()     <- Pulse medium
                             |
                         graduation (if policy selects)
                             |
                             v
  L2: on_retrieve() ---- [RETRIEVE] --- L2: on_store()      <- Signal medium
                             |
                             v
                         [ANALYZE]
                             |
                             v
  L3: on_gate() --------- [GATE]                            <- Signal medium
                             |
                             v
  L3: pre_inference() -- [SIMULATE] -- L3: post_inference()
                             |
                             v
                         [VALIDATE]
                             |
                             v
  L4: pre_action() ----- [EXECUTE] --- L4: post_action()    <- Signal medium
  L4: on_tool_call()         |
                             |
  L5: on_message_send() ----|         <- Pulse medium
  L5: on_message_receive()   |
                             v
                          [VERIFY]
                             |
                             v
  L6: on_reflect() ----- [REFLECT] --- L6: on_cost_update()  <- Signal medium
                             |
                             v
                        Signals Out (Store)
                        Pulses Out (Bus)

  On error at any step:
  L7: on_error() or on_budget_exceeded()

  Throughout: CaMeL capability tags propagate per rules in section 2
```

---

## 19. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Extension trait compiles with all 22 hooks and default no-op implementations | `cargo check` on Extension trait definition |
| `FilterDecision::Drop` silently discards Pulse (logged at DEBUG) | Unit test: send Pulse through Extension with Drop, verify not delivered |
| `FilterDecision::Transform` replaces Pulse content, preserves CaMeL tags | Unit test: send Pulse, verify transformed version received with original tags |
| `ActionDecision::Block` halts action but Agent continues | Integration test: block an action, verify next tick executes normally |
| `ActionDecision::Modify` replaces action transparently | Unit test: modify action, verify modified version executed |
| `ToolDecision::Block` prevents tool call, Agent notified | Unit test: block tool call, verify Agent receives block reason |
| `ToolDecision::Substitute` transparently replaces tool call, tags propagated | Unit test: substitute tool, verify Agent sees substitute's result with merged tags |
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
| CaMeL tags propagate through Transform decisions | Unit test: Transform a Pulse, verify output carries input tags + Extension tag |
| CaMeL tags cannot be elevated through Extensions | Unit test: Extension attempts to strip `untrusted` tag, verify it persists |
| L1 and L5 hooks receive Pulses, not Signals | Integration test: verify medium type at hook invocation |
| 5-tier SPI loading: WASM Extension loads and runs sandboxed | Integration test: WASM Extension with fuel limit, verify sandboxing |

---

## 20. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-core` | Extension trait definition, ExtensionLayer enum, decision enums, CaMeL tag types |
| `roko-agent` | Extension loading, dependency resolution, hook dispatch, fault isolation, CaMeL tag propagation |
| `roko-std` | Built-in Extension implementations (git, compiler, test-runner, camel-monitor, etc.) |
| `roko-cli` | Extension configuration in roko.toml, profile defaults |
| `roko-serve` | Extension status endpoints (`GET /api/extensions`) |
| `roko-gate` | CaMeL IFC verification (see [doc-17](17-SECURITY-MODEL.md)) |
