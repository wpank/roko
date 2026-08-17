# PAH_03: Extract shared ChatSession for chat mode unification

## Task
Extract shared dispatch, streaming, and cost tracking logic from the 5 CLI modes into a reusable `ChatSession` trait or struct.

## Runner Context
Runner PAH (UX & Data Model), batch 3 of 3. Depends on PP_03.

## Problem
UX-2 anti-pattern: "5 modes, zero shared rendering." After PP_03 deduplicates the two chat event loops, there's still no shared abstraction for:
- Dispatch (model selection, prompt assembly)
- Streaming (token display, progress)
- Cost tracking (per-turn, per-session)
- Session persistence (chat_session.rs)
- Tool output rendering

Each of the 5 modes (unified REPL, one-shot, agent chat, universal loop, dashboard) implements these independently.

## Exact Changes

### Step 1: Define ChatSessionCore trait

```rust
// In crates/roko-cli/src/chat_session_core.rs (new file):

/// Shared core logic for all CLI chat modes.
/// Handles dispatch, cost tracking, and session state.
pub struct ChatSessionCore {
    pub session_id: Option<String>,
    pub model: String,
    pub cost_meter: CostMeter,
    pub message_history: Vec<Message>,
    pub model_call_service: Option<Arc<ModelCallService>>,
}

impl ChatSessionCore {
    pub fn new(config: &ChatSessionConfig) -> Self { ... }

    /// Dispatch a prompt and return the response.
    /// All modes use this for the actual LLM call.
    pub async fn dispatch(&mut self, prompt: &str) -> Result<DispatchResponse> {
        // 1. Build request with session state
        // 2. Dispatch via ModelCallService or CLI fallback
        // 3. Update cost meter
        // 4. Append to message history
        // 5. Return response
    }

    /// Record cost from a completed turn
    pub fn record_cost(&mut self, cost: f64, tokens_in: u64, tokens_out: u64) { ... }

    /// Get session summary for display
    pub fn summary(&self) -> SessionSummary { ... }
}

pub struct ChatSessionConfig {
    pub model: String,
    pub system_message: Option<String>,
    pub mcp_config: Option<PathBuf>,
    pub effort: Option<String>,
    pub session_persist_path: Option<PathBuf>,
}
```

### Step 2: Use ChatSessionCore in unified REPL (mode 1)

```rust
// In chat_inline.rs:
let mut core = ChatSessionCore::new(&config);
// Replace inline dispatch logic with core.dispatch()
```

### Step 3: Use ChatSessionCore in one-shot mode (mode 2)

```rust
// In the one-shot entry point:
let mut core = ChatSessionCore::new(&config);
let response = core.dispatch(&prompt).await?;
println!("{}", response.content);
```

### Step 4: Use ChatSessionCore in roko run (mode 4)

```rust
// In run.rs:
let mut core = ChatSessionCore::new(&config);
// Replace dispatch logic with core.dispatch()
```

## Write Scope
- `crates/roko-cli/src/chat_session_core.rs` (new file)
- `crates/roko-cli/src/chat_inline.rs` (use ChatSessionCore)
- `crates/roko-cli/src/run.rs` (use ChatSessionCore where applicable)

## Read-Only Context
- `crates/roko-cli/src/chat.rs` (agent chat mode — mode 3)
- `crates/roko-cli/src/tui/` (dashboard mode — mode 5, different rendering)


## Verify
```bash
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-cli 2>&1 | tail -20
```
## Acceptance Criteria
- `ChatSessionCore` handles dispatch, cost tracking, message history
- At least 2 modes use the shared core
- Cost tracking consistent across modes
- Session ID properly managed (not always None)
- Existing mode behavior unchanged

## Do NOT
- Unify the rendering layer (each mode has different UX requirements)
- Force dashboard mode to use ChatSessionCore (it's fullscreen TUI, not REPL)
- Change the CLI interface or flag parsing
