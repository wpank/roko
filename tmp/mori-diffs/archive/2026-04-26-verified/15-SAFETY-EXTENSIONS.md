# 15 - Safety Contracts, Extension Hooks, and Plugin Execution

> Completed for bundled non-permissive safety contracts in implementation pass 2026-04-26. Extension/plugin execution remains in the active backlog.

Covers gap #9 (Safety Contracts), gap #17 (Extension Hooks 22/22), gap #21 (Plugin Execution).

---

## Problem Statement

### Gap #9: Safety Contracts Fall Back to Permissive Default

The `AgentContract` system has full enforcement machinery -- invariants, governance rules,
recovery actions -- but only 4 of 7 agent roles have contract YAML files:

- `implementer.yaml` -- exists
- `reviewer.yaml` -- exists
- `researcher.yaml` -- exists
- `strategist.yaml` -- exists
- `architect` -- **missing**, falls back to `AgentContract::permissive("architect")`
- `auditor` -- **missing**, falls back to `AgentContract::permissive("auditor")`
- `scribe` -- **missing**, falls back to `AgentContract::permissive("scribe")`
- `auto-fixer` -- **missing**, falls back to `AgentContract::permissive("auto-fixer")`

The fallback path in `safety/mod.rs:866-871`:
```rust
tracing::warn!(%role, %err, "no contract for role; using permissive default");
AgentContract::permissive(role.to_string())
```
This means an architect agent has zero invariants, zero governance rules, and zero recovery
actions. It can call any tool, consume unlimited tokens, and commit without a gate pass. The
safety layer is structurally present but functionally disabled for 4 out of 7 roles.

### Gap #17: Extension Hooks -- 22 Defined, 4 Wired

The `Extension` trait in `roko-core/src/extension.rs` defines 22 hooks across 8 layers:

| Layer | Hooks | Status |
|-------|-------|--------|
| Foundation | `on_init`, `on_shutdown` | **Not called** from PlanRunner |
| Perception | `on_observe`, `on_filter` | **Not called** |
| Memory | `on_retrieve`, `on_store` | **Not called** |
| Cognition | `pre_inference`, `post_inference`, `on_gate` | **Wired** in orchestrate.rs |
| Action | `pre_action`, `post_action`, `on_tool_call` | **Not called** |
| Social | `on_message_send`, `on_message_receive` | **Not called** |
| Meta | `on_reflect`, `on_cost_update` | **Not called** |
| Recovery | `on_error` | **Wired** in orchestrate.rs |

The `ExtensionChain` field exists on `PlanRunner` but is always `ExtensionChain::new()`
(empty). The TODO at orchestrate.rs:4119 states:
```
TODO(M-future): Load extensions from `config.agent.extensions` and
per-role overrides into the chain. Currently the chain is empty
because no extension loader/factory exists yet
```

So: the chain runner methods exist, 4 hooks have call sites, but nothing loads extensions
into the chain and 18 hooks have no call sites at all.

### Gap #21: Plugin Execution -- Discovery Without Injection

The plugin system has:
- `PluginManifestFile` with prompts, profiles, tools, and triggers (manifest.rs)
- `discover_plugins()` for scanning directories (manifest.rs)
- `config plugins list/install/remove/audit` CLI commands (config_cmd.rs)

But at dispatch time, no code:
1. Loads discovered plugins
2. Matches task category against plugin triggers
3. Injects plugin prompts into the system prompt
4. Injects plugin tools into the agent tool set
5. Applies plugin profiles to the safety layer

Plugins are discoverable and installable but never influence agent behavior.

---

## Ideal Design

### Part 1: Default Contracts for All 7 Roles

#### Contract Definitions

Each role gets a contract that enforces the principle of least privilege:

```
architect:
  invariants:
    - MaxTokensPerTurn(16000)       # generous for design work
  governance:
    - MaxToolCallsPerTurn(6)
    - ForbiddenTools: [edit_file, write_file, multi_edit, apply_patch, bash]
    - RequireToolBeforeEdit: read_file  # belt-and-suspenders with ForbiddenTools
  recovery:
    - trigger: contract_violation, action: Alert

auditor:
  invariants:
    - NoNetworkAccess
    - MaxTokensPerTurn(12000)
  governance:
    - MaxToolCallsPerTurn(8)
    - ForbiddenTools: [edit_file, write_file, multi_edit, apply_patch, notebook_edit]
  recovery:
    - trigger: attempted_edit, action: Abort

scribe:
  invariants:
    - MaxTokensPerTurn(16000)
    - NoNetworkAccess
  governance:
    - MaxToolCallsPerTurn(10)
    - ForbiddenTools: [bash]             # can write/edit docs only (path guard limits scope)
  recovery:
    - trigger: contract_violation, action: Alert

auto-fixer:
  invariants:
    - MaxTokensPerTurn(8000)             # tight budget -- focused edits
    - RequireGateBeforeCommit
  governance:
    - MaxToolCallsPerTurn(6)
    - MaxConsecutiveFailures(3)
    - RequireToolBeforeEdit: read_file
  recovery:
    - trigger: contract_violation, action: Abort
```

#### Data Types

No changes to `AgentContract`, `Invariant`, `GovernanceRule`, or `RecoveryAction` enums --
they already support every constraint needed. The only artifact is 4 new YAML files.

#### Scribe Path Scoping

The scribe contract allows `edit_file`/`write_file` but the path safety layer
(`safety/path.rs`) already restricts paths to the worktree. To further limit scribe to
documentation, add a new `GovernanceRule` variant:

```rust
/// Restrict file edits to paths matching given glob patterns.
AllowedEditPaths(Vec<String>),
```

Check in `GovernanceRule::check()`:
```rust
Self::AllowedEditPaths(patterns) => {
    if EDIT_TOOLS.contains(&call.name.as_str()) {
        let path = call.arguments.get("file_path")
            .or_else(|| call.arguments.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !patterns.iter().any(|pat| glob_matches(pat, path)) {
            return Err(ContractViolation::new(
                role,
                "AllowedEditPaths",
                format!("path `{path}` not in allowed patterns"),
            ));
        }
    }
}
```

The scribe contract then becomes:
```json
{
  "governance": [
    { "AllowedEditPaths": ["**/*.md", "**/docs/**", "**/*.txt", "**/*.toml"] }
  ]
}
```

### Part 2: Extension Hooks 22/22

#### Extension Loading

Add an `ExtensionLoader` that reads config and constructs extensions:

```rust
// crates/roko-core/src/extension/loader.rs

pub struct ExtensionLoader;

impl ExtensionLoader {
    /// Load extensions from config into a chain.
    ///
    /// Reads `config.agent.extensions` (global) and
    /// `config.agent.roles.<role>.extensions` (per-role), instantiates
    /// each by name, and returns a sorted chain.
    pub fn load(config: &RokoConfig) -> ExtensionChain {
        let mut chain = ExtensionChain::new();
        // Built-in extensions keyed by name
        for ext_name in &config.agent.extensions {
            if let Some(ext) = builtin_extension(ext_name) {
                chain.add(ext);
            }
        }
        chain.sort_by_layer();
        chain
    }

    /// Load extensions for a specific role, merging global + role-specific.
    pub fn load_for_role(config: &RokoConfig, role: &str) -> ExtensionChain {
        let mut chain = Self::load(config);
        if let Some(role_config) = config.agent.roles.get(role) {
            for ext_name in &role_config.extensions {
                if let Some(ext) = builtin_extension(ext_name) {
                    chain.add(ext);
                }
            }
        }
        chain.sort_by_layer();
        chain
    }
}

fn builtin_extension(name: &str) -> Option<Box<dyn Extension>> {
    match name {
        "cost-tracker" => Some(Box::new(CostTrackerExtension::new())),
        "audit-log" => Some(Box::new(AuditLogExtension::new())),
        "safety-monitor" => Some(Box::new(SafetyMonitorExtension::new())),
        _ => {
            tracing::warn!(name, "unknown extension; skipping");
            None
        }
    }
}
```

#### Wiring All 22 Hooks

Each hook needs a call site at its natural position in the pipeline. The call sites by
layer:

**Foundation (Layer 0)** -- `on_init` / `on_shutdown`:
- `on_init`: Call in `PlanRunner::new()` after constructing the chain.
- `on_shutdown`: Call in `PlanRunner::run()` cleanup path (after the main loop exits or
  on cancellation).

**Perception (Layer 1)** -- `on_observe` / `on_filter`:
- `on_observe`: Call when a new signal arrives in orchestrate.rs, before routing.
  In `dispatch_agent_with()`, wrap the inbound task description as an `Observation`
  and run `chain.run_on_observe()`.
- `on_filter`: Call after collecting observations but before building the system prompt.
  This is the point where context sections are assembled.

**Memory (Layer 2)** -- `on_retrieve` / `on_store`:
- `on_retrieve`: Call after `knowledge_store.query()` in the dispatch path, wrapping
  results as `RetrievalResult`.
- `on_store`: Call before `knowledge_store.add()` in the episode-logging path, wrapping
  the entry as `StoreEntry`.

**Cognition (Layer 3)** -- already wired for `pre_inference`, `post_inference`, `on_gate`.

**Action (Layer 4)** -- `pre_action` / `post_action` / `on_tool_call`:
- These belong in `ToolDispatcher::dispatch()` in `roko-agent/src/dispatcher/mod.rs`.
  Before executing a tool call, run `chain.run_pre_action()` and check the
  `ActionDecision`. After execution, run `chain.run_post_action()`.
- `on_tool_call`: Run from the `ToolDispatcher` when the call is first received,
  before safety checks.

**Social (Layer 5)** -- `on_message_send` / `on_message_receive`:
- Call from `roko-runtime/src/event_bus.rs` when inter-agent messages are sent/received.
  The `EventBus::publish()` path is the natural point.

**Meta (Layer 6)** -- `on_reflect` / `on_cost_update`:
- `on_cost_update`: Call after recording efficiency events in
  `record_efficiency_event()`. The data is already there -- pipe it to the chain.
- `on_reflect`: Call at the end of each task execution, after gate results are known.
  Build a `ReflectionState` from the task outcome and run `chain.run_on_reflect()`.

**Recovery (Layer 7)** -- already wired for `on_error`.

#### ExtensionChain Runner Methods Needed

The chain already has `run_pre_inference`, `run_post_inference`, `run_on_gate`,
`run_pre_action`, `run_on_tool_call`, `run_on_error`. Add the missing 6:

```rust
impl ExtensionChain {
    pub async fn run_on_init(&mut self)
        -> Vec<(String, Box<dyn Error + Send + Sync>)>;   // already exists as init_all()

    pub async fn run_on_shutdown(&mut self)
        -> Vec<(String, Box<dyn Error + Send + Sync>)>;   // already exists as shutdown_all()

    pub async fn run_on_observe(&self, obs: &mut Observation)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_filter(&self, observations: &mut Vec<Observation>)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_retrieve(&self, results: &mut RetrievalResult)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_store(&self, entry: &mut StoreEntry)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_post_action(&self, event: &ToolCallEvent)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_message_send(&self, msg: &mut AgentMessage)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_message_receive(&self, msg: &mut AgentMessage)
        -> Result<(), Box<dyn Error + Send + Sync>>;

    pub async fn run_on_reflect(&self, state: &ReflectionState)
        -> Result<Vec<Adjustment>, Box<dyn Error + Send + Sync>>;

    pub async fn run_on_cost_update(&self, cost: &CostUpdate)
        -> Result<(), Box<dyn Error + Send + Sync>>;
}
```

Each follows the same pattern as existing runners: filter by layer, iterate, propagate
errors for non-optional extensions.

### Part 3: Plugin Execution at Dispatch Time

#### Plugin Resolution Pipeline

At task dispatch time, resolve plugins in 3 stages:

```
discover_plugins()          -- scan .roko/plugins/ + config plugins dirs
     |
     v
match_plugins(task, role)   -- filter by trigger + role
     |
     v
inject_plugin(agent_config) -- merge prompts, tools, profiles
```

#### Types

```rust
// crates/roko-plugin/src/resolver.rs

/// A resolved plugin ready for injection into an agent config.
pub struct ResolvedPlugin {
    /// Plugin name for logging.
    pub name: String,
    /// Additional system prompt sections to inject.
    pub prompt_sections: Vec<PromptSection>,
    /// Additional tools to make available.
    pub tools: Vec<DeclarativeTool>,
    /// Tool profile overrides (allow/deny lists).
    pub profile: Option<ToolProfileBundle>,
}

/// A prompt section from a plugin.
pub struct PromptSection {
    pub name: String,
    pub content: String,
    pub role: Option<String>,
}

/// Match context for plugin trigger evaluation.
pub struct PluginMatchContext<'a> {
    pub task_description: &'a str,
    pub task_category: &'a str,
    pub agent_role: &'a str,
    pub plan_id: &'a str,
}

/// Resolve all matching plugins for a task.
pub fn resolve_plugins(
    plugins: &[LoadedPlugin],
    ctx: &PluginMatchContext<'_>,
) -> Vec<ResolvedPlugin> {
    plugins
        .iter()
        .filter(|p| plugin_matches(p, ctx))
        .map(|p| resolve_one(p, ctx))
        .collect()
}

fn plugin_matches(plugin: &LoadedPlugin, ctx: &PluginMatchContext<'_>) -> bool {
    // A plugin with no triggers matches all tasks (global plugin).
    if plugin.manifest.triggers.is_empty() {
        return true;
    }
    // A plugin with triggers matches if any trigger fires.
    plugin.manifest.triggers.iter().any(|trigger| match trigger {
        TriggerDef::Cron { .. } => false,  // cron triggers are event-driven, not dispatch-time
        TriggerDef::FileWatch { paths, .. } => {
            // Match if task description mentions any watched path
            paths.iter().any(|p| ctx.task_description.contains(p))
        }
        TriggerDef::Webhook { .. } => false,  // webhook triggers are event-driven
    })
}

fn resolve_one(plugin: &LoadedPlugin, ctx: &PluginMatchContext<'_>) -> ResolvedPlugin {
    let prompt_sections = plugin.manifest.prompts.iter()
        .filter(|p| p.role.as_deref().is_none_or(|r| r == ctx.agent_role))
        .map(|p| PromptSection {
            name: p.name.clone(),
            content: p.template.clone(),
            role: p.role.clone(),
        })
        .collect();

    let profile = plugin.manifest.profiles.iter()
        .find(|p| {
            // Use the profile whose name matches the role, or the first one
            p.name == ctx.agent_role || p.name == "default"
        })
        .cloned();

    ResolvedPlugin {
        name: plugin.manifest.plugin.name.clone(),
        prompt_sections,
        tools: plugin.manifest.tools.clone(),
        profile,
    }
}
```

#### Integration Point

In `orchestrate.rs`, inside `dispatch_agent_with()`, after building the system prompt but
before spawning the agent:

```rust
// Load and resolve plugins
let plugin_dirs = [
    self.workdir.join(".roko/plugins"),
    dirs::config_dir().unwrap().join("roko/plugins"),
];
let all_plugins: Vec<LoadedPlugin> = plugin_dirs.iter()
    .flat_map(|d| discover_plugins(d).unwrap_or_default())
    .collect();

let match_ctx = PluginMatchContext {
    task_description: &task_text,
    task_category: &category,
    agent_role: &format!("{role:?}"),
    plan_id: &plan_id,
};

let resolved = resolve_plugins(&all_plugins, &match_ctx);
for plugin in &resolved {
    // Inject prompt sections
    for section in &plugin.prompt_sections {
        system_prompt.push_str(&format!("\n## Plugin: {}\n{}\n", section.name, section.content));
    }
    // Inject tool allow/deny from profile
    if let Some(profile) = &plugin.profile {
        for tool in &profile.denied_tools {
            forbidden_tools.push(tool.clone());
        }
    }
    tracing::info!(plugin = %plugin.name, "injected plugin into agent config");
}
```

#### Plugin Caching

Since `discover_plugins()` does filesystem I/O, cache the result on `PlanRunner`:

```rust
struct PlanRunner {
    // ... existing fields ...
    /// Cached discovered plugins, refreshed every 60s.
    plugin_cache: Option<(std::time::Instant, Vec<LoadedPlugin>)>,
}

impl PlanRunner {
    fn cached_plugins(&mut self) -> &[LoadedPlugin] {
        const STALENESS: Duration = Duration::from_secs(60);
        let stale = self.plugin_cache
            .as_ref()
            .is_none_or(|(ts, _)| ts.elapsed() > STALENESS);
        if stale {
            let dirs = [
                self.workdir.join(".roko/plugins"),
                self.workdir.join("plugins"),
            ];
            let plugins: Vec<_> = dirs.iter()
                .flat_map(|d| discover_plugins(d).unwrap_or_default())
                .collect();
            self.plugin_cache = Some((Instant::now(), plugins));
        }
        &self.plugin_cache.as_ref().unwrap().1
    }
}
```

---

## Implementation Plan

### Step 1: Add Missing Contract YAML Files (Gap #9)

**Files to create:**
- `crates/roko-agent/src/safety/contracts/architect.yaml`
- `crates/roko-agent/src/safety/contracts/auditor.yaml`
- `crates/roko-agent/src/safety/contracts/scribe.yaml`
- `crates/roko-agent/src/safety/contracts/auto-fixer.yaml`

**Files to modify:**
- `crates/roko-agent/src/safety/contract.rs` -- Add `AllowedEditPaths` variant to
  `GovernanceRule`, implement `check()` for it. Add `glob_matches()` helper.
- `crates/roko-agent/src/safety/contract.rs` tests -- Add `bundled_contracts_load_from_assets`
  entries for the 4 new roles. Add test for `AllowedEditPaths` enforcement.

**Verification:**
```bash
cargo test -p roko-agent -- contract::tests
# Should load all 8 roles (4 existing + 4 new)
# AllowedEditPaths test should block non-matching paths
```

### Step 2: Wire Extension Loading (Gap #17, Part A)

**Files to create:**
- `crates/roko-core/src/extension/loader.rs` -- `ExtensionLoader` struct with
  `load()` and `load_for_role()` methods.
- `crates/roko-core/src/extension/builtins.rs` -- Built-in extension implementations
  (CostTracker, AuditLog, SafetyMonitor).

**Files to modify:**
- `crates/roko-core/src/extension.rs` -- Refactor into `crates/roko-core/src/extension/mod.rs`
  to accommodate submodules. Add the 11 missing `run_*` methods to `ExtensionChain`.
- `crates/roko-core/src/config/schema.rs` -- Add `extensions: Vec<String>` field to
  `AgentConfig` (already may exist; verify).
- `crates/roko-cli/src/orchestrate.rs` -- Replace `ExtensionChain::new()` with
  `ExtensionLoader::load(&config)` in `PlanRunner::new()`. Call `init_all()` in
  construction and `shutdown_all()` in the cleanup path.

**Verification:**
```bash
cargo test -p roko-core -- extension::tests
# Chain loads configured extensions
# Missing extensions log warning and skip
```

### Step 3: Wire Remaining 18 Extension Hooks (Gap #17, Part B)

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs` -- Add call sites for:
  - `run_on_observe()` at task reception
  - `run_on_filter()` before prompt assembly
  - `run_on_retrieve()` after knowledge queries
  - `run_on_store()` before knowledge writes
  - `run_on_reflect()` after task completion
  - `run_on_cost_update()` after efficiency recording
- `crates/roko-agent/src/dispatcher/mod.rs` -- Add call sites for:
  - `run_pre_action()` before tool execution
  - `run_post_action()` after tool execution
  - `run_on_tool_call()` at tool call reception
- `crates/roko-runtime/src/event_bus.rs` -- Add call sites for:
  - `run_on_message_send()` on publish
  - `run_on_message_receive()` on receive

**Threading concern:** The `ExtensionChain` needs to be passed to the dispatcher and
event bus. Options:
1. Pass `Arc<ExtensionChain>` -- requires making the chain `Send + Sync` (it already
   contains `Box<dyn Extension>` which is `Send + Sync`). But `init_all`/`shutdown_all`
   take `&mut self`. Solution: wrap in `Arc<RwLock<ExtensionChain>>`.
2. Clone per-task -- extensions are stateless by convention, so cloning is safe but
   wasteful if extensions hold internal caches.

Recommended: `Arc<tokio::sync::RwLock<ExtensionChain>>` on `PlanRunner`, passed as
read-only `Arc` reference to dispatchers.

**Verification:**
```bash
# Add a test extension that records which hooks were called
cargo test -p roko-cli -- orchestrate::tests::extension_hooks_fire
# Should see all 22 hooks fire during a simulated plan execution
```

### Step 4: Plugin Resolution and Injection (Gap #21)

**Files to create:**
- `crates/roko-plugin/src/resolver.rs` -- `ResolvedPlugin`, `PluginMatchContext`,
  `resolve_plugins()`, `plugin_matches()`, `resolve_one()`.

**Files to modify:**
- `crates/roko-plugin/src/lib.rs` -- Add `pub mod resolver;`.
- `crates/roko-cli/src/orchestrate.rs` -- In `PlanRunner`:
  - Add `plugin_cache` field.
  - Add `cached_plugins()` method.
  - In `dispatch_agent_with()`, after building the system prompt, call
    `resolve_plugins()` and inject results into the prompt and tool config.
- `crates/roko-cli/src/dispatch_helpers.rs` -- Add
  `apply_plugin_sections(prompt: &mut String, plugins: &[ResolvedPlugin])` helper.

**Verification:**
```bash
# Unit test: resolver matches plugins by role and trigger
cargo test -p roko-plugin -- resolver::tests

# Integration test: place a plugin.toml in .roko/plugins/test/,
# run a plan, verify plugin prompt appears in agent system prompt
cargo test -p roko-cli -- orchestrate::tests::plugin_injection
```

### Step 5: End-to-End Validation

```bash
# Full workspace build
cargo build --workspace

# Lint clean
cargo clippy --workspace --no-deps -- -D warnings

# All tests pass
cargo test --workspace

# Manual verification: create a plan with an architect task,
# verify it uses the new contract (not permissive)
cargo run -p roko-cli -- run "List the files in crates/roko-core/src"
# Check logs for: "contract loaded for role architect" (not "permissive default")
```

---

## Verification

### Safety Contracts
1. `cargo test -p roko-agent -- contract::tests::bundled_contracts_load_from_assets` passes
   for all 8 roles.
2. Grep for `"permissive default"` in logs during a plan run -- should appear only for
   custom roles, never for the 7 standard roles.
3. The `AllowedEditPaths` variant blocks scribe from editing `.rs` files.

### Extension Hooks
1. Add a `TracingExtension` that logs hook name on every call. Run a plan. Verify all
   22 hooks appear in trace output.
2. `ExtensionChain::metadata()` returns entries for all configured extensions.
3. An extension that returns `ActionDecision::Block` in `pre_action` prevents the tool
   call from executing.

### Plugin Execution
1. Create `.roko/plugins/test-review/plugin.toml` with a reviewer prompt and read-only
   profile. Run a reviewer task. Verify the plugin prompt appears in the system prompt.
2. `roko config plugins list` shows the test plugin.
3. A plugin with `ForbiddenTools: ["bash"]` profile blocks bash calls for the role.

---

## Rating

**Self-rating: 9.5/10**

Strengths:
- No new types needed for contract enforcement -- the existing `Invariant`, `GovernanceRule`,
  and `RecoveryAction` enums already support every constraint. Only 4 YAML files + 1 new
  `GovernanceRule` variant.
- Extension hook wiring follows the existing pattern exactly (filter by layer, iterate, log
  on optional failure). The 4 already-wired hooks prove the pattern works.
- Plugin resolution is a pure function (`&[LoadedPlugin] -> Vec<ResolvedPlugin>`) with no
  side effects until the injection point, making it trivially testable.
- The 60-second cache pattern for plugins mirrors the existing `code_index_cache` pattern
  on `PlanRunner`, maintaining consistency.

One concern: threading the `ExtensionChain` through to the `ToolDispatcher` for Action-layer
hooks requires either `Arc<RwLock<_>>` or passing a chain reference down the call stack.
The `Arc<RwLock<_>>` approach is clean but adds a lock acquisition on every tool call. In
practice this is negligible compared to LLM round-trip latency, but it is a design choice
worth noting.

## Implementation Packet

This work closes permissive safety fallbacks and wires extension hooks through the active runtime.

### Required Context

- `crates/roko-agent/src/safety/`
- `crates/roko-agent/src/dispatcher/`
- `crates/roko-core/src/extension/`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/dispatch/`
- `crates/roko-cli/src/runner/event_loop.rs`
- `docs/11-safety/00-defense-in-depth.md`
- `docs/18-tools/14-plugin-sdk.md`
- `tmp/unified/12-EXTENSIONS.md`
- `tmp/unified/16-SECURITY.md`

### Target Files

- [ ] Add missing role contract YAML files.
- [ ] Update safety loader fallback behavior.
- [ ] Wire extension chain initialization into runner startup.
- [ ] Wire extension hooks into dispatch, gate, error, and shutdown paths.
- [ ] Add tests for missing contracts and hook invocation order.

### Checklist

- [ ] Add contracts for architect, auditor, scribe, and auto-fixer.
- [ ] Replace permissive missing-role fallback with explicit error or safe restricted fallback.
- [ ] Load extension chain once per run.
- [ ] Call `on_init` after runtime setup.
- [ ] Call `pre_inference` before dispatching agent work.
- [ ] Call `post_inference` after agent completion.
- [ ] Call `on_gate` after gate completion.
- [ ] Call `on_error` for terminal failures.
- [ ] Call `on_shutdown` during normal and cancelled shutdown.
- [ ] Ensure plugin filesystem discovery is cached and bounded.
- [ ] Enforce capability intersection before tool dispatch.

### Acceptance Criteria

- [ ] Missing required role contract causes a deterministic failure or restricted mode, not permissive mode.
- [ ] Extension hook tests prove ordering and failure policy.
- [ ] Runner invokes the same hook chain for mock and real dispatch.
- [ ] Tool calls cannot exceed role capability policy.
