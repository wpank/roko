# Path 1: Agent Dispatch -- Provider-Agnostic Design

## Current State (What's Broken)

The runner v2 (`crates/roko-cli/src/runner/`) hardcodes Claude CLI as the only agent backend. Every problem below traces back to this single architectural gap.

### Problem 1: Claude CLI Hardcoding

`agent_stream.rs` spawns `claude` directly via `tokio::process::Command`:

```rust
// agent_stream.rs:197-207
let mut cmd = Command::new(&config.program);  // always config.claude_program
cmd.args(["--print", "--output-format", "stream-json"]);
cmd.args(["--model", &config.model]);
cmd.args(["--max-turns", &config.max_turns.to_string()]);
```

The `RunConfig` type itself embeds `claude_program: PathBuf` (types.rs:105). The stream parser (`parse_stream_line`) only understands Claude's `stream-json` protocol. There is no abstraction for other backends.

Meanwhile, `roko-agent` already supports 8 backends via `create_agent_for_model()` in `crates/roko-agent/src/provider/mod.rs`:

| Backend | Adapter | Provider Kind |
|---------|---------|---------------|
| Claude CLI | `ClaudeCliAdapter` | `ProviderKind::ClaudeCli` |
| Claude API (Messages) | `AnthropicApiAdapter` | `ProviderKind::AnthropicApi` |
| Codex | `OpenAiCompatAdapter` | `ProviderKind::OpenAiCompat` |
| Cursor | `CursorAcpAdapter` | `ProviderKind::CursorAcp` |
| OpenAI-compat (ZAI, etc.) | `OpenAiCompatAdapter` | `ProviderKind::OpenAiCompat` |
| Ollama | `OpenAiCompatAdapter` | `ProviderKind::OpenAiCompat` |
| Gemini | `GeminiAdapter` | `ProviderKind::GeminiApi` |
| Perplexity | `PerplexityAdapter` | `ProviderKind::PerplexityApi` |

None of these are reachable from the runner event loop.

### Problem 2: No Provider Abstraction in Runner

`AgentSpawnConfig` (agent_stream.rs:27-48) is Claude-specific:

```rust
pub struct AgentSpawnConfig {
    pub program: PathBuf,           // Claude CLI binary path
    pub dangerously_skip_permissions: bool,  // Claude-specific flag
    pub resume_session: Option<String>,      // Claude session resume
    // ...
}
```

The orchestrate.rs `dispatch_agent_with()` method (line 13911) already uses `create_agent_for_model()` + `SpawnAgentSpec` + role-scoped safety layers. The runner has none of this.

### Problem 3: Static model_hint

The runner resolves models via `task_def.model_hint` falling back to `config.model` (event_loop.rs:470-474):

```rust
let model = task_def
    .model_hint
    .as_deref()
    .unwrap_or(&ctx.config.model)
    .to_string();
```

No adaptive routing. The orchestrator (orchestrate.rs:14057) wires `CascadeRouter` for 3-stage adaptive selection (static -> confidence -> UCB1 bandit), but the runner ignores it entirely.

### Problem 4: Auto-Pass Verify Stub

The verify phase is a stub (event_loop.rs:545-548):

```rust
ExecutorAction::RunVerify { plan_id } => {
    info!(plan_id = %plan_id, "auto-passing verification (stub)");
    let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::VerifyPassed);
}
```

### Problem 5: No Warm Pool

Every agent spawn is cold. For gate-then-retry cycles, the reviewer agent could be pre-spawned during gate execution to reduce latency. The orchestrator has no warm pool either, but the runner's sequential nature makes this especially impactful.

### Problem 6: Stderr Discarded

Agent stderr is logged at `debug!` level and discarded (agent_stream.rs:277-285):

```rust
if let Some(stderr) = child.stderr.take() {
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                debug!(stderr = %line, "agent stderr");
            }
        }
    });
}
```

Crash diagnostics, provider errors, and DOA (dead-on-arrival) detection all depend on stderr content that is currently invisible.

### Problem 7: No Pre-Spawn Validation

The runner spawns the agent and only discovers failures after the process exits. No validation that:
- The binary exists and is executable
- The working directory is a valid git repo (when git operations are expected)
- The system prompt is non-empty
- The model key resolves to a configured provider

## Design Goals

1. **Provider-agnostic**: Use `create_agent_for_model()` for all backends, not just Claude CLI.
2. **Adaptive routing**: Wire `CascadeRouter` so model selection improves over time.
3. **Safety-scoped**: Use `SpawnAgentSpec` + `spawn_agent_with_layer()` for role-based safety.
4. **Observable**: Surface stderr, detect DOA, emit dispatch metrics.
5. **Resilient**: Pre-spawn validation, fallback chain, warm pool for reviewers.
6. **Backward-compatible**: Claude CLI stream-json remains the default; other backends are opt-in via `roko.toml`.

## Architecture

### New Module: `crates/roko-cli/src/dispatch/`

```
crates/roko-cli/src/dispatch/
  mod.rs            -- AgentDispatcher struct, public dispatch() method
  model_routing.rs  -- CascadeRouter integration, RoutingDecision
  outcome.rs        -- DispatchOutcome, DispatchError
  warm_pool.rs      -- WarmPool for pre-spawning reviewer agents
  validation.rs     -- pre-spawn checks (binary, git, prompt, model)
```

### New Types

```rust
// dispatch/mod.rs

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_agent::provider::{AgentOptions, ProviderSemaphores};
use roko_agent::{Agent, SafetyLayer};
use roko_core::AgentRole;
use roko_core::config::schema::RokoConfig;
use roko_learn::cascade_router::CascadeRouter;
use tokio::sync::mpsc;

use crate::task_parser::TaskDef;
use super::types::AgentEvent;

/// Provider-agnostic agent dispatcher for the runner v3 event loop.
///
/// Replaces the hardcoded `agent_stream::spawn_agent()` with a dispatcher
/// that routes through `create_agent_for_model()` and supports all 8 backends.
pub struct AgentDispatcher {
    /// Loaded roko.toml config (providers, models, agent settings).
    config: Arc<RokoConfig>,
    /// Shared semaphores for per-provider concurrency limits.
    semaphores: Arc<ProviderSemaphores>,
    /// Cascade router for adaptive model selection.
    cascade_router: Arc<CascadeRouter>,
    /// Working directory for agent subprocesses.
    workdir: PathBuf,
    /// Optional MCP config path passed to agents.
    mcp_config: Option<PathBuf>,
    /// Warm pool for pre-spawning reviewer agents.
    warm_pool: WarmPool,
}

/// Input to a dispatch call. Replaces `AgentSpawnConfig`.
pub struct DispatchRequest {
    /// Plan this agent is working on.
    pub plan_id: String,
    /// Task ID within the plan.
    pub task_id: String,
    /// Role of the agent (Implementer, Reviewer, etc.).
    pub role: AgentRole,
    /// The task definition from tasks.toml.
    pub task_def: TaskDef,
    /// User prompt (built by prompt_builder).
    pub prompt: String,
    /// System prompt (built by prompt_builder).
    pub system_prompt: String,
    /// Explicit model override (skips CascadeRouter).
    pub model_override: Option<String>,
    /// Explicit effort level override.
    pub effort: Option<String>,
    /// Previous gate failure output for retry context.
    pub gate_feedback: Option<GateFeedback>,
    /// Safety layer to scope the agent under.
    pub safety_layer: Option<SafetyLayer>,
}

/// Structured gate failure feedback (replaces raw text prepending).
pub struct GateFeedback {
    /// Which gate failed (compile, clippy, test, etc.).
    pub gate_name: String,
    /// Rung number that failed.
    pub rung: u32,
    /// Parsed error items from gate output.
    pub errors: Vec<GateError>,
    /// Raw gate output (fallback if parsing fails).
    pub raw_output: String,
}

/// A single parsed error from a gate failure.
pub struct GateError {
    /// File path where the error occurred.
    pub file: Option<String>,
    /// Line number.
    pub line: Option<u32>,
    /// Error code (e.g., E0308, clippy::needless_borrow).
    pub code: Option<String>,
    /// Error message.
    pub message: String,
}

/// Result of a successful dispatch.
pub struct DispatchOutcome {
    /// The agent handle (for kill/lifecycle).
    pub handle: AgentHandle,
    /// Model actually used (after routing).
    pub model: String,
    /// Provider kind used.
    pub provider: roko_agent::provider::ProviderKind,
    /// Routing decision metadata (for learning feedback).
    pub routing: RoutingDecision,
}

/// Metadata about how the model was selected.
pub struct RoutingDecision {
    /// Stage that made the decision (static, confidence, ucb1).
    pub stage: String,
    /// Why this model was chosen.
    pub reason: String,
    /// The model that was requested before routing.
    pub requested_model: String,
    /// The model that was actually selected.
    pub selected_model: String,
    /// Full fallback chain (in priority order).
    pub fallback_chain: Vec<String>,
    /// Optional CascadeRouteExplanation for dashboard display.
    pub explanation: Option<roko_learn::cascade_router::CascadeRouteExplanation>,
}
```

```rust
// dispatch/mod.rs - impl AgentDispatcher

impl AgentDispatcher {
    /// Create a new dispatcher from config.
    pub fn new(
        config: Arc<RokoConfig>,
        cascade_router: Arc<CascadeRouter>,
        workdir: PathBuf,
        mcp_config: Option<PathBuf>,
    ) -> Self {
        let providers = config.effective_providers();
        let semaphores = Arc::new(ProviderSemaphores::new(&providers));
        Self {
            config,
            semaphores,
            cascade_router,
            workdir,
            mcp_config,
            warm_pool: WarmPool::new(2), // max 2 pre-spawned agents
        }
    }

    /// Dispatch an agent for a task. Returns the handle and routing metadata.
    ///
    /// Steps:
    /// 1. Validate pre-conditions (binary exists, prompt non-empty, etc.)
    /// 2. Route model via CascadeRouter (or use override)
    /// 3. Build AgentOptions from DispatchRequest
    /// 4. Create agent via create_agent_for_model()
    /// 5. Spawn the agent and wire event streaming
    /// 6. Return DispatchOutcome with routing metadata
    pub async fn dispatch(
        &mut self,
        request: DispatchRequest,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<DispatchOutcome, DispatchError> {
        // 1. Pre-spawn validation
        self.validate(&request)?;

        // 2. Model routing
        let routing = self.route_model(&request)?;

        // 3. Build agent options
        let options = self.build_options(&request, &routing);

        // 4. Create agent via provider layer
        let agent = roko_agent::create_agent_for_model(
            &self.config,
            &routing.selected_model,
            options,
        ).map_err(|e| DispatchError::AgentCreation(e.to_string()))?;

        // 5. Spawn and wire streaming
        let handle = self.spawn_and_stream(agent, &request, event_tx).await?;

        Ok(DispatchOutcome {
            handle,
            model: routing.selected_model.clone(),
            provider: roko_core::agent::resolve_model(&self.config, &routing.selected_model)
                .provider_kind,
            routing,
        })
    }

    /// Route model selection through CascadeRouter.
    fn route_model(&self, request: &DispatchRequest) -> Result<RoutingDecision, DispatchError> {
        if let Some(ref model) = request.model_override {
            return Ok(RoutingDecision {
                stage: "override".to_string(),
                reason: "explicit model override".to_string(),
                requested_model: model.clone(),
                selected_model: model.clone(),
                fallback_chain: vec![],
                explanation: None,
            });
        }

        let task_reqs = task_def_to_requirements(&request.task_def, request.role);
        let cascade_model = self.cascade_router.route(&task_reqs);

        Ok(RoutingDecision {
            stage: cascade_model.stage_name().to_string(),
            reason: cascade_model.reason().to_string(),
            requested_model: request.task_def.model_hint
                .clone()
                .unwrap_or_default(),
            selected_model: cascade_model.primary.clone(),
            fallback_chain: cascade_model.fallback.clone(),
            explanation: Some(cascade_model.explain()),
        })
    }

    /// Pre-spawn validation checks.
    fn validate(&self, request: &DispatchRequest) -> Result<(), DispatchError> {
        if request.prompt.trim().is_empty() {
            return Err(DispatchError::EmptyPrompt);
        }
        if request.system_prompt.trim().is_empty() {
            return Err(DispatchError::EmptySystemPrompt);
        }
        if !self.workdir.exists() {
            return Err(DispatchError::WorkdirMissing(
                self.workdir.display().to_string(),
            ));
        }
        Ok(())
    }

    /// Report a dispatch outcome back to the cascade router for learning.
    pub fn report_outcome(
        &self,
        routing: &RoutingDecision,
        success: bool,
        tokens_used: u64,
        wall_ms: u64,
        cost_usd: f64,
    ) {
        self.cascade_router.record_observation(
            &routing.selected_model,
            success,
            tokens_used,
            wall_ms,
            cost_usd,
        );
    }

    /// Pre-spawn a reviewer agent for the next gate cycle.
    pub async fn pre_spawn_reviewer(&mut self) {
        self.warm_pool.pre_spawn_if_needed(
            &self.config,
            &self.semaphores,
            AgentRole::Reviewer,
            &self.workdir,
        ).await;
    }
}
```

```rust
// dispatch/outcome.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("empty prompt - cannot dispatch agent with no task")]
    EmptyPrompt,

    #[error("empty system prompt - prompt builder failed")]
    EmptySystemPrompt,

    #[error("working directory does not exist: {0}")]
    WorkdirMissing(String),

    #[error("model key does not resolve to a configured provider: {0}")]
    ModelNotConfigured(String),

    #[error("agent creation failed: {0}")]
    AgentCreation(String),

    #[error("agent process died on arrival (exit code {exit_code:?}, stderr: {stderr})")]
    DeadOnArrival {
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("provider semaphore timeout after {0:?}")]
    SemaphoreTimeout(std::time::Duration),

    #[error("budget exhausted: ${spent:.2} >= ${limit:.2}")]
    BudgetExhausted { spent: f64, limit: f64 },
}
```

```rust
// dispatch/warm_pool.rs

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use roko_agent::provider::ProviderSemaphores;
use roko_agent::Agent;
use roko_core::AgentRole;
use roko_core::config::schema::RokoConfig;

/// Pre-spawned agents ready for immediate use.
///
/// The warm pool keeps at most `capacity` agents alive. When a gate
/// run starts, the dispatcher calls `pre_spawn_reviewer()` so that
/// the reviewer is ready by the time the gate completes.
pub struct WarmPool {
    capacity: usize,
    agents: VecDeque<WarmAgent>,
}

struct WarmAgent {
    role: AgentRole,
    agent: Box<dyn Agent>,
    model: String,
}

impl WarmPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            agents: VecDeque::with_capacity(capacity),
        }
    }

    /// Take a pre-spawned agent for the given role, if available.
    pub fn take(&mut self, role: AgentRole) -> Option<(Box<dyn Agent>, String)> {
        let idx = self.agents.iter().position(|a| a.role == role)?;
        let warm = self.agents.remove(idx)?;
        Some((warm.agent, warm.model))
    }

    /// Pre-spawn an agent if the pool has capacity and no matching agent exists.
    pub async fn pre_spawn_if_needed(
        &mut self,
        config: &RokoConfig,
        semaphores: &Arc<ProviderSemaphores>,
        role: AgentRole,
        workdir: &PathBuf,
    ) {
        if self.agents.len() >= self.capacity {
            return;
        }
        if self.agents.iter().any(|a| a.role == role) {
            return;
        }

        // Use the default model for this role.
        let model = config
            .agent
            .role_model(role.label())
            .unwrap_or_else(|| "claude-sonnet-4-6".to_string());

        let options = roko_agent::provider::AgentOptions {
            working_dir: Some(workdir.clone()),
            provider_semaphores: Some(Arc::clone(semaphores)),
            name: format!("warm-{}", role.label()),
            ..Default::default()
        };

        match roko_agent::create_agent_for_model(config, &model, options) {
            Ok(agent) => {
                self.agents.push_back(WarmAgent {
                    role,
                    agent,
                    model,
                });
                tracing::debug!(role = %role.label(), "pre-spawned warm agent");
            }
            Err(e) => {
                tracing::warn!(role = %role.label(), err = %e, "failed to pre-spawn warm agent");
            }
        }
    }

    /// Drain all warm agents (called on shutdown).
    pub fn drain(&mut self) -> Vec<Box<dyn Agent>> {
        self.agents.drain(..).map(|a| a.agent).collect()
    }
}
```

```rust
// dispatch/model_routing.rs

use roko_core::agent::{AgentRole, TaskRequirements, ModelTier};
use roko_core::task::TaskCategory;
use roko_learn::cascade_router::{CascadeModel, CascadeRouter};

use crate::task_parser::TaskDef;

/// Convert a TaskDef into TaskRequirements for CascadeRouter.
pub fn task_def_to_requirements(task_def: &TaskDef, role: AgentRole) -> TaskRequirements {
    let category = match task_def.tier.as_str() {
        "fast" | "mechanical" => TaskCategory::Mechanical,
        "standard" => TaskCategory::Standard,
        "complex" | "premium" => TaskCategory::Complex,
        "architectural" => TaskCategory::Architectural,
        _ => TaskCategory::Standard,
    };

    let tier = match task_def.tier.as_str() {
        "fast" | "mechanical" => ModelTier::Fast,
        "standard" => ModelTier::Standard,
        "complex" | "premium" => ModelTier::Premium,
        "architectural" => ModelTier::Frontier,
        _ => ModelTier::Standard,
    };

    TaskRequirements {
        role,
        category,
        tier,
        needs_tools: true,  // runner tasks always have tool access
        needs_vision: false,
        needs_web_search: false,
        context_tokens_estimate: estimate_context_tokens(task_def),
        max_cost_usd: None,  // enforced at dispatch level
    }
}

/// Rough estimate of context tokens a task will use.
fn estimate_context_tokens(task_def: &TaskDef) -> usize {
    let base = 2000; // system prompt + task prompt overhead
    let file_tokens = task_def.files.len() * 500; // ~500 tokens per file reference
    let context_tokens = task_def.context.as_ref().map_or(0, |ctx| {
        ctx.read_files.len() * 1000 // ~1000 tokens per read file
    });
    base + file_tokens + context_tokens
}
```

```rust
// dispatch/validation.rs

use std::path::Path;

use super::outcome::DispatchError;

/// Validate that a command binary is available on PATH or as absolute path.
pub fn validate_binary(program: &str) -> Result<(), DispatchError> {
    if Path::new(program).is_absolute() {
        if !Path::new(program).exists() {
            return Err(DispatchError::AgentCreation(
                format!("binary not found: {program}"),
            ));
        }
    } else {
        // Check PATH
        if which::which(program).is_err() {
            return Err(DispatchError::AgentCreation(
                format!("binary not found on PATH: {program}"),
            ));
        }
    }
    Ok(())
}

/// Validate that the working directory is a git repo (when git ops are expected).
pub fn validate_git_repo(workdir: &Path) -> Result<(), DispatchError> {
    if !workdir.join(".git").exists() {
        return Err(DispatchError::WorkdirMissing(
            format!("{} is not a git repository", workdir.display()),
        ));
    }
    Ok(())
}
```

### Integration Points

The event loop calls `AgentDispatcher` instead of `agent_stream::spawn_agent()`:

```rust
// runner/event_loop.rs - inside dispatch_action, SpawnAgent branch

ExecutorAction::SpawnAgent { plan_id, task, .. } => {
    // ... (task resolution unchanged) ...

    // NEW: Build prompt via prompt_builder (see 05-PROMPT-ASSEMBLY.md)
    let (prompt, system_prompt) = ctx.prompt_builder.build_for_task(
        &task_def, plan_id, &ctx.state.gate_output,
    );

    // NEW: Dispatch via AgentDispatcher instead of agent_stream::spawn_agent()
    let request = DispatchRequest {
        plan_id: plan_id.clone(),
        task_id: task_id.clone(),
        role: task_def.role_enum(),
        task_def: task_def.clone(),
        prompt,
        system_prompt,
        model_override: None,
        effort: None,
        gate_feedback: parse_gate_feedback(&ctx.state.gate_output),
        safety_layer: None,
    };

    match ctx.dispatcher.dispatch(request, ctx.agent_tx.clone()).await {
        Ok(outcome) => {
            ctx.state.agent_active = true;
            ctx.state.agent_pid = Some(outcome.handle.pid);
            ctx.state.agent_model = outcome.model.clone();
            ctx.tui.agent_spawned(&agent_id, role, &outcome.model);
            *ctx.agent_handle = Some(outcome.handle);

            // Stash routing decision for post-gate learning feedback
            ctx.state.last_routing = Some(outcome.routing);
        }
        Err(e) => {
            error!(err = %e, "dispatch failed");
            ctx.tui.error(&format!("dispatch failed: {e}"));
            let _ = ctx.executor.apply_event(
                plan_id,
                &ExecutorEvent::Fatal(format!("dispatch failed: {e}")),
            );
        }
    }
}
```

The verify phase becomes real:

```rust
// runner/event_loop.rs - RunVerify branch

ExecutorAction::RunVerify { plan_id } => {
    // NEW: Use warm pool reviewer instead of auto-pass stub
    if let Some((reviewer, model)) = ctx.dispatcher.warm_pool.take(AgentRole::Reviewer) {
        info!(plan_id = %plan_id, model = %model, "dispatching warm reviewer");
        // ... wire reviewer agent to verify channel ...
    } else {
        // Fallback: dispatch a fresh reviewer
        let request = DispatchRequest {
            role: AgentRole::Reviewer,
            // ...
        };
        ctx.dispatcher.dispatch(request, ctx.verify_tx.clone()).await?;
    }
}
```

Post-gate learning feedback:

```rust
// runner/event_loop.rs - after gate completion

if let Some(routing) = ctx.state.last_routing.take() {
    ctx.dispatcher.report_outcome(
        &routing,
        completion.passed,
        state.tokens_in + state.tokens_out,
        state.task_elapsed_ms(),
        state.cost_usd,
    );
}
```

## Detailed Specification

### Agent Streaming Abstraction

The current `parse_stream_line()` only handles Claude's `stream-json` format. For provider-agnostic dispatch, we need an `AgentEventStream` trait:

```rust
/// Adapter between different agent output formats and our AgentEvent protocol.
pub trait AgentEventStream: Send {
    /// Read the next event from the agent. Returns None when the stream ends.
    async fn next_event(&mut self) -> Option<AgentEvent>;
}
```

For Claude CLI, the existing `parse_stream_line()` becomes the implementation. For API-based backends (Anthropic API, OpenAI-compat), the `Agent::run()` return value is mapped to `AgentEvent`s. This is an incremental change -- the Claude CLI path remains the default and other backends are added as `AgentEventStream` implementations.

### Stderr Surfacing and DOA Detection

Replace the debug-only stderr reader with structured capture:

```rust
/// Captured stderr from an agent process.
pub struct StderrCapture {
    /// All stderr lines (capped at 1000 lines).
    pub lines: Vec<String>,
    /// Whether the agent exited within 2 seconds of spawn (DOA).
    pub is_doa: bool,
}
```

DOA detection: if the agent process exits within 2 seconds of spawn and exit code is non-zero, it's dead-on-arrival. The dispatcher should:
1. Capture stderr
2. Emit `AgentEvent::Error` with the stderr content
3. Return `DispatchError::DeadOnArrival` if DOA

### Fallback Chain

When the primary model fails (DOA, rate limit, auth failure), the dispatcher tries the fallback chain from `CascadeModel`:

```rust
async fn dispatch_with_fallback(
    &mut self,
    request: DispatchRequest,
    event_tx: mpsc::Sender<AgentEvent>,
) -> Result<DispatchOutcome, DispatchError> {
    let routing = self.route_model(&request)?;
    let mut models_to_try = vec![routing.selected_model.clone()];
    models_to_try.extend(routing.fallback_chain.iter().cloned());

    for model in &models_to_try {
        match self.try_dispatch(&request, model, &event_tx).await {
            Ok(outcome) => return Ok(outcome),
            Err(DispatchError::DeadOnArrival { .. }) |
            Err(DispatchError::AgentCreation(_)) => {
                tracing::warn!(model = %model, "dispatch failed, trying fallback");
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(DispatchError::AgentCreation(
        "all models in fallback chain failed".to_string(),
    ))
}
```

### RunConfig Changes

`RunConfig` drops Claude-specific fields and gains provider-agnostic ones:

```rust
pub struct RunConfig {
    pub workdir: PathBuf,
    pub plan_dir: PathBuf,
    pub model: String,              // default model key (not slug)
    pub fallback_model: Option<String>,  // NEW: explicit fallback
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub approval: bool,
    pub mcp_config: Option<PathBuf>,
    pub resume_session: Option<String>,
    pub max_gate_rung: u32,
    // REMOVED: claude_program, dangerously_skip_permissions
    // NEW:
    pub effort: Option<String>,     // --effort level
    pub allowed_tools: Option<Vec<String>>,  // tool allowlist
    pub max_plan_usd: f64,
    pub max_turn_usd: f64,
    pub clippy_enabled: bool,
    pub skip_tests: bool,
}
```

### RunContext Changes

`RunContext` gains a dispatcher field:

```rust
struct RunContext<'a> {
    executor: &'a mut ParallelExecutor,
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    skip_enrichment: &'a HashMap<String, bool>,
    config: &'a RunConfig,
    tui: &'a TuiBridge,
    state: &'a mut RunState,
    agent_handle: &'a mut Option<AgentHandle>,
    dispatcher: &'a mut AgentDispatcher,      // NEW
    prompt_builder: &'a PromptAssembler,      // NEW (see 05-PROMPT-ASSEMBLY.md)
    agent_tx: &'a mpsc::Sender<AgentEvent>,
    gate_tx: &'a mpsc::Sender<GateCompletion>,
    paths: &'a PersistPaths,
}
```

## Error Handling

| Error | Behavior |
|-------|----------|
| `EmptyPrompt` | Fatal -- prompt builder bug, abort task |
| `EmptySystemPrompt` | Fatal -- prompt builder bug, abort task |
| `WorkdirMissing` | Fatal -- configuration error, abort plan |
| `ModelNotConfigured` | Try fallback chain, then fatal |
| `AgentCreation` | Try fallback chain, then fatal |
| `DeadOnArrival` | Try fallback chain, log stderr, then fatal |
| `SemaphoreTimeout` | Try fallback chain (different provider), then fatal |
| `BudgetExhausted` | Fatal -- no retry, plan budget is hard limit |

All errors are reported via `TuiBridge::error()` and persisted in the executor snapshot. `DispatchError` implements `std::error::Error` and is logged with full context.

## Testing Strategy

### Unit Tests

1. **`model_routing.rs`**: Test `task_def_to_requirements()` for all tier values.
2. **`validation.rs`**: Test pre-spawn validation with missing binary, non-git dir, empty prompt.
3. **`warm_pool.rs`**: Test capacity limits, take-by-role, drain.
4. **`outcome.rs`**: Test `DispatchError` Display formatting.

### Integration Tests

1. **Mock dispatcher**: Use `ROKO_DISPATCHER=mock-*` env var to test dispatch flow with scripted responses. The existing `MockAgent` in roko-agent supports this.
2. **Fallback chain**: Configure a primary model that fails (mock DOA) and verify fallback model is used.
3. **Routing feedback**: Dispatch, complete gate, verify `report_outcome()` updates cascade router state.

### Existing Test Compatibility

The current `tests/smoke.rs` and `tests/cli_fallback.rs` tests use `ROKO_DISPATCHER=mock-*`. The new dispatcher must honor this env var and route through `MockAgent` when set, preserving all existing test infrastructure.

## Open Questions

1. **Agent::run() vs. process spawn**: For API-based backends, `Agent::run()` returns a single `AgentResult`. The runner expects a stream of `AgentEvent`s. Should API backends emit synthetic events (SystemInit -> MessageDelta -> TurnCompleted), or should the event loop handle both streaming and one-shot patterns?

2. **Warm pool lifecycle**: Should warm agents have a TTL? Pre-spawned Claude CLI agents consume a process slot. If the gate takes 30+ seconds, the warm agent is idle for that duration.

3. **CascadeRouter persistence**: The runner currently doesn't persist cascade router state. Should `report_outcome()` trigger an immediate flush, or batch writes on the flush interval (every 2 seconds)?

## Implementation Packet

This work turns the runner from a Claude CLI process launcher into a provider-neutral agent dispatcher.

### Required Context

- `crates/roko-cli/src/runner/agent_stream.rs`
- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-agent/src/provider/mod.rs`
- `crates/roko-agent/src/streaming.rs`
- `crates/roko-agent/src/session.rs`
- `crates/roko-agent/src/tool_loop/mod.rs`
- `docs/02-agents/00-agent-trait.md`
- `docs/02-agents/02-provider-adapters.md`
- `tmp/unified/05-AGENT.md`

### Target Files

- [ ] Create `crates/roko-cli/src/dispatch/mod.rs`.
- [ ] Create `crates/roko-cli/src/dispatch/model_routing.rs`.
- [ ] Create `crates/roko-cli/src/dispatch/outcome.rs`.
- [ ] Create `crates/roko-cli/src/dispatch/preflight.rs`.
- [ ] Create `crates/roko-cli/src/dispatch/session.rs`.
- [ ] Create or extend `crates/roko-agent/src/runtime_events.rs`.
- [ ] Update `crates/roko-agent/src/lib.rs` exports.
- [ ] Update `crates/roko-cli/src/runner/event_loop.rs` to call the dispatcher.
- [ ] Update `crates/roko-cli/src/runner/agent_events.rs` to consume normalized events.

### Checklist

- [ ] Define `AgentRuntimeEvent` in `roko-agent` with started, output delta, tool call, usage, completed, failed, and exited variants.
- [ ] Implement a Claude CLI adapter that converts existing stream JSON into `AgentRuntimeEvent`.
- [ ] Implement a synthetic event adapter for one-shot API providers that only return `AgentResult`.
- [ ] Define `DispatchRequest` with `plan_id`, `task_id`, `role`, `task`, `model_hint`, retry context, and approval mode.
- [ ] Define `DispatchResult` with provider, requested model, actual model, run id, session id, and pid when available.
- [ ] Move model selection out of `runner/event_loop.rs` and into `dispatch/model_routing.rs`.
- [ ] Preserve `ROKO_DISPATCHER=mock-*` behavior for existing tests.
- [ ] Add preflight checks for binary availability, workdir existence, missing model profile, and blocked role/tool policy.
- [ ] Add warm pool TTL and eviction policy; default TTL should be conservative, for example 120 seconds.
- [ ] Ensure every dispatch emits `AgentRuntimeEvent::Started` before any output event.
- [ ] Ensure every terminal path emits either `Completed`, `Failed`, or `Exited`.

### Verification

- [ ] `cargo check -p roko-agent -p roko-cli`.
- [ ] Unit test: Claude stream JSON fixture maps to normalized events in order.
- [ ] Unit test: one-shot provider result maps to started, output, usage, completed.
- [ ] Unit test: `ROKO_DISPATCHER=mock-success` still works.
- [ ] Integration test: runner receives normalized events without importing Claude event types.
- [ ] Search gate: `rg "ClaudeStreamEvent" crates/roko-cli/src/runner` returns no result.

## Worker 9 Evidence Checklist (2026-04-26)

Source-backed progress that exists now:

- [x] `crates/roko-cli/src/dispatch_v2.rs` defines `CliProtocol`, `CliProviderConfig`, `CliDispatchRequest`, `CliInvocation`, `ProviderRuntime`, `ProviderDispatchResolver`, and `AgentDispatcherV2`.
- [x] `CliProviderConfig::from_legacy_runner_program` lets the live runner resolve the configured `claude` or `codex` CLI shape through `dispatch_v2`.
- [x] `CliProviderConfig::from_provider_config` and `AgentDispatcherV2::create_agent` can resolve non-CLI providers through `roko_agent::create_agent_for_model`.
- [x] `crates/roko-cli/src/runner/agent_stream.rs` surfaces stderr as `AgentEvent::Error`; `runner/event_loop.rs` persists that as `agent.error` through `append_agent_event`.
- [x] `crates/roko-cli/src/runner/event_loop.rs` prevents repeated spawn effects while `state.agent_active` or `agent_handle` is set.

Concrete remaining work for this doc:

- [ ] Create the intended `crates/roko-cli/src/dispatch/` module family or reconcile the plan with the existing `dispatch_v2.rs` file.
- [ ] Add `crates/roko-agent/src/runtime_events.rs` with provider-neutral `AgentRuntimeEvent` variants.
- [ ] Move `ClaudeStreamEvent`, `ClaudeContentBlock`, and CLI stream parsing out of `crates/roko-cli/src/runner/agent_stream.rs`.
- [ ] Replace `agent_stream::spawn_agent` in `runner/event_loop.rs` with a dispatcher facade that can consume both CLI streams and one-shot `AgentResult` providers.
- [ ] Wire `AgentDispatcherV2::run_agent_result_bridge`; it is currently explicitly not used by runner v2 because started events require an OS pid.
- [ ] Move model choice from `task_def.model_hint.or(config.model)` in `runner/event_loop.rs` to a routing module that can consult `roko-learn::CascadeRouter`.
- [ ] Preserve or reintroduce a no-mock-compatible test seam without relying on production mocks.

## 2026-04-27 Deepening Pass - Source-Corrected Dispatch Authority

Self-grade for this pass:

- Initial rating: 9.91 / 10.
- Reasoning: this section corrects stale claims about missing dispatch/runtime-event modules and narrows the remaining work to authority, provider proof, model routing semantics, and transition-surface retirement. The score is not higher because no provider matrix run was executed in this pass.

This section supersedes the "Concrete remaining work" list above where source has moved forward.

### Current Source Truth

- [x] `crates/roko-agent/src/runtime_events.rs` exists and exports `AgentRuntimeEvent` and `AgentEventStream`.
- [x] `crates/roko-agent/src/provider/claude_cli/stream.rs` owns Claude stream parsing and returns provider-neutral `AgentRuntimeEvent` values.
- [x] `crates/roko-cli/src/runner/types.rs` aliases `AgentEvent` to `roko_agent::AgentRuntimeEvent`.
- [x] `crates/roko-cli/src/dispatch/` exists with `mod.rs`, `model_routing.rs`, `outcome.rs`, `prompt_builder.rs`, and `warm_pool.rs`.
- [x] `Dispatcher::plan` performs route, prompt assembly, and dispatch-plan materialization.
- [x] `Dispatcher::spawn_streaming_cli_agent` is the active runner-facing facade for CLI streaming agents.
- [x] `resolve_agent_runtime` chooses between CLI subprocess runtime and `AgentDispatcherV2::run_agent_result_bridge`.
- [x] `spawn_agent_result_bridge` forwards API/provider-backed `AgentResult` output as normalized runtime events.
- [x] `runner/event_loop.rs` calls `Dispatcher::new`, `PromptAssembler::new`, `resolve_agent_runtime`, `spawn_streaming_cli_agent`, and `spawn_agent_result_bridge`.
- [x] `dispatch_v2.rs` still provides provider resolution, CLI invocation metadata, and provider-backed `create_agent_for_model` bridging.

### Current Dispatch Gaps

- [ ] `runner/event_loop.rs` currently sets `DispatchContext::model_hint` to `Some(ctx.config.model.clone())`; this makes the configured default look like a task hint and can bypass cascade routing.
- [ ] `dispatch/model_routing.rs` marks the cascade branch as `ModelChoiceSource::Router` but currently returns `default_slug` because full routing feature vectors are not built end to end.
- [ ] `force_backend` is modeled but active plan-run wiring sets it to `None`; operator/backend override policy needs a real source and proof.
- [ ] `WarmPool` exists as a typed container but active runner constructs `WarmPool::new(0)`, so no real pre-spawn or reuse behavior is active.
- [ ] `dispatch/preflight.rs` and `dispatch/session.rs` do not exist as separate modules; preflight/session behavior is split across `dispatch_v2`, provider config, and runner process handling.
- [ ] `dispatch_direct.rs` bypasses the plan-run dispatch facade for chat/unified paths and needs classification or convergence.
- [ ] `dispatch_v2.rs` remains a transition surface beside `dispatch/`; callers and ownership need cleanup before archive.
- [ ] `AgentOutcome` fidelity is not fully propagated into learning/projection events; feedback currently receives synthetic blanks for some runner events.
- [ ] Provider matrix proof is missing for Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI through the same dispatch path.
- [ ] Auth failures, missing credentials, unsupported providers, rate limits, and spawn failures need normalized status and queryable evidence.

### Target Dispatch Contract

All runtime entry points should pass through one dispatch command service:

```rust
pub struct DispatchCommand {
    pub run_id: String,
    pub plan_id: Option<String>,
    pub task_id: Option<String>,
    pub role: String,
    pub workdir: PathBuf,
    pub prompt: DispatchPrompt,
    pub routing_policy: RoutingPolicyInput,
    pub provider_policy: ProviderPolicyInput,
    pub safety_policy: SafetyPolicyInput,
    pub observability: DispatchObservabilityInput,
}

pub struct DispatchRecord {
    pub dispatch_id: String,
    pub model_choice: ModelChoice,
    pub provider_id: String,
    pub runtime_kind: String,
    pub session_id: Option<String>,
    pub pid: Option<u32>,
    pub prompt_diagnostics_id: Option<String>,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub terminal_status: Option<String>,
}
```

Rules:

- [ ] Configured default model is fallback policy only; it must not be passed as `task_model_hint`.
- [ ] Task `model_hint` is author intent and produces `ModelChoiceSource::TaskHint`.
- [ ] Operator `force_backend` produces `ModelChoiceSource::Override`.
- [ ] Cascade router choices produce `ModelChoiceSource::Router` and include feature vector/provenance.
- [ ] Every dispatch emits a durable `DispatchRecord` before provider execution starts.
- [ ] Every terminal provider path emits completed, failed, rate_limited, auth_failed, missing_credentials, unsupported, or cancelled.
- [ ] CLI, API, chat, one-shot, and plan-run paths either use this service or are explicitly documented as out of scope.

### Implementation Batches

#### DSP-01: Fix Model Choice Semantics

- [ ] Change `runner/event_loop.rs` so `DispatchContext::model_hint` receives only task/user overrides, not `ctx.config.model`.
- [ ] Pass default model into `ModelRouter::with_default_slug` or an explicit fallback policy object.
- [ ] Build a full routing context from role, domain, tier, complexity, budget, attempt, provider health, and prior observations.
- [ ] Call `CascadeRouter` with that context instead of returning `default_slug` from the cascade branch.
- [ ] Emit `ModelChoiceSource` and fallback reason in prompt diagnostics, runtime events, feedback, and projection.

#### DSP-02: Consolidate Runtime Surfaces

- [ ] Classify `dispatch_v2.rs` as provider resolver/compatibility layer or fold it under `dispatch/provider_runtime.rs`.
- [ ] Classify `dispatch_direct.rs` callers as interactive-only or migrate them to the shared dispatch command service.
- [ ] Ensure `runner/event_loop.rs` imports no provider-specific protocol types.
- [ ] Keep CLI process ownership in runner only for cancellation/orphan cleanup; provider invocation construction belongs below dispatch/provider code.
- [ ] Remove or rename any old dispatch helpers after grep gates prove no production callers remain.

#### DSP-03: Provider Matrix Proof

- [ ] Add a provider matrix script that uses the same dispatch command path for all configured providers.
- [ ] Cover Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI.
- [ ] For each provider emit one status: `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, or `unsupported`.
- [ ] For `proved`, record model, provider id, runtime kind, started event, output event, usage event when available, terminal event, duration, and cost when available.
- [ ] Store results in `tmp/mori-diffs/generated/provider-dispatch-matrix.json`.

#### DSP-04: Preflight And Policy

- [ ] Validate workdir exists before dispatch.
- [ ] Validate CLI binary exists for CLI runtimes.
- [ ] Validate provider credentials or report `missing_credentials` without spawning.
- [ ] Validate model key resolves to exactly one provider or produces an explicit unsupported/ambiguous error.
- [ ] Validate role/tool safety policy before provider launch.
- [ ] Emit preflight diagnostics through projection and HTTP/query surfaces.

#### DSP-05: Warm Pool Decision

- [ ] Decide whether warm pool is a real production feature or an inactive future optimization.
- [ ] If real, set nonzero capacity through config, add TTL, health check, eviction, and cancellation.
- [ ] If inactive, mark it disabled by policy and remove claims that reviewer pre-spawn is implemented.
- [ ] Prove warm pool hit/miss/eviction stats through an inspect endpoint or CLI query.

#### DSP-06: Outcome Fidelity

- [ ] Ensure `AgentOutcome` carries provider, model, tokens, cost, duration, exit code, session id, dispatch id, and output summary.
- [ ] Attach `AgentOutcome` to task-completed runner events.
- [ ] Feed exact outcome into learning, projection, persistence, and HTTP queries.
- [ ] Do not synthesize blank provider/model values for completed dispatches.

### Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/agent-dispatch-proof.json`:

```json
{
  "schema": "mori-diffs.agent-dispatch-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "routing_cases": {
    "force_backend_override": false,
    "task_model_hint": false,
    "cascade_router": false,
    "default_fallback": false,
    "config_default_not_hint": false
  },
  "provider_matrix": [],
  "preflight_cases": {
    "missing_binary": false,
    "missing_credentials": false,
    "unsupported_model": false,
    "auth_failed": false,
    "rate_limited": false
  },
  "event_contract": {
    "started_before_output": false,
    "single_terminal_event": false,
    "runner_has_no_claude_protocol_types": false
  },
  "remaining_gaps": []
}
```

### Grep Gates

- [ ] `rg -n "ClaudeStreamEvent|ClaudeAssistantEvent|ClaudeToolEvent|ClaudeContentBlock" crates/roko-cli/src/runner` returns no hits.
- [ ] `rg -n "model_hint: Some\\(ctx.config.model.clone\\(\\)\\)" crates/roko-cli/src/runner/event_loop.rs` returns no hits.
- [ ] `rg -n "dispatch_direct|dispatch_v2" crates/roko-cli/src` has every hit classified in `tmp/mori-diffs/generated/migration-legacy-surface-report.json`.
- [ ] `rg -n "provider=\\\"claude\\\"|backend=\\\"claude\\\"" crates/roko-cli/src/runner crates/roko-cli/src/runtime_feedback` returns no hardcoded production attribution.

### No-Context Handoff Checklist

- [ ] Open `crates/roko-cli/src/runner/event_loop.rs` and fix `DispatchContext::model_hint`.
- [ ] Open `crates/roko-cli/src/dispatch/model_routing.rs` and replace cascade placeholder logic with real `CascadeRouter` input.
- [ ] Open `crates/roko-cli/src/dispatch/mod.rs` and decide where `dispatch_v2` should live.
- [ ] Open `crates/roko-cli/src/dispatch_direct.rs` and classify/migrate every caller.
- [ ] Add provider matrix proof and write `agent-dispatch-proof.json`.
- [ ] Update [12-AFFECT-ROUTING.md](12-AFFECT-ROUTING.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md), and [README.md](README.md).

### Archive Gate

- [ ] Config default no longer bypasses routing.
- [ ] Cascade branch uses real router features.
- [ ] Provider matrix proof exists.
- [ ] Transition surfaces are classified or retired.
- [ ] Outcome fidelity reaches feedback/projection/persistence.
- [ ] `agent-dispatch-proof.json` exists and is linked from README.
