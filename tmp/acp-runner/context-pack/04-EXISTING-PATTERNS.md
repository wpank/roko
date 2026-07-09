# Existing Roko Patterns (for bridge implementations)

## Pattern 1: Substrate trait (for FS bridge)

The `Substrate` trait in `roko-core` handles persistent storage:

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    async fn write(&self, engram: &Engram) -> Result<()>;
    async fn read(&self, hash: &str) -> Result<Option<Engram>>;
    async fn query(&self, filter: &Filter) -> Result<Vec<Engram>>;
    async fn delete(&self, hash: &str) -> Result<()>;
}
```

The ACP FS bridge (`bridge_fs.rs`) should implement a similar interface but route reads/writes through the editor's `fs/read_text_file` and `fs/write_text_file` JSON-RPC methods. When the editor doesn't declare `fs` capability, fall back to direct filesystem I/O.

## Pattern 2: ProcessSupervisor (for terminal bridge)

`roko-runtime` provides `ProcessSupervisor` for managing child processes:

```rust
pub struct ProcessSupervisor {
    processes: HashMap<String, ProcessHandle>,
}

impl ProcessSupervisor {
    pub async fn spawn(&mut self, cmd: &str, args: &[String]) -> Result<String>;
    pub async fn output(&self, id: &str) -> Result<ProcessOutput>;
    pub async fn kill(&mut self, id: &str) -> Result<()>;
    pub async fn wait(&self, id: &str) -> Result<ExitStatus>;
}
```

The ACP terminal bridge (`bridge_terminal.rs`) should route commands through the editor's `terminal/*` JSON-RPC methods. When the editor doesn't declare `terminal` capability, fall back to `ProcessSupervisor`.

## Pattern 3: StateHub / TuiBridge (for event streaming)

The TUI uses a push-based event model:

```rust
pub enum DashboardEvent {
    AgentOutput(String),
    GateStarted { name: String },
    GateCompleted { name: String, passed: bool },
    PhaseTransition(PlanPhase),
    // ...
}
```

Events flow via `tokio::sync::watch` channels. The ACP bridge should use a similar channel-based approach: the cognitive loop sends `CognitiveEvent`s, and `bridge_events.rs` maps them to ACP `session/update` notifications.

## Pattern 4: CostLens (for usage bridge)

`roko-learn` provides `CostLens` for tracking costs:

```rust
pub struct CostLens {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub total_cost_usd: f64,
}
```

The ACP usage bridge (`bridge_usage.rs`) should accumulate from `CostLens` and push `usage_update` notifications.

## Pattern 5: CancelToken (for session cancellation)

`roko-runtime` provides cooperative cancellation:

```rust
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self;
    pub fn cancel(&self);
    pub fn is_cancelled(&self) -> bool;
    pub async fn cancelled(&self); // Future that resolves when cancelled
}
```

Each ACP session should have its own `CancelToken`. When the client sends `session/cancel`, call `token.cancel()` to stop the cognitive loop.

## Pattern 6: GateResult (for gate bridge)

Gate results carry structured data:

```rust
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub duration: Duration,
    pub details: GateDetails,
}

pub enum GateDetails {
    Compile { warnings: u32, errors: u32 },
    Test { passed: u32, failed: u32, total: u32 },
    Clippy { warnings: u32 },
    // ...
}
```

The gate bridge (`bridge_gates.rs`) maps these to ACP `tool_call` and `tool_call_update` notifications with markdown content summaries.
