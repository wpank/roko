# Batch ACP05 — Session management

## Goal

Implement ACP session creation, listing, loading, and state management.

## Target files

- `crates/roko-acp/src/session.rs` — Session types and management
- `crates/roko-acp/src/config.rs` — AcpConfig struct

## Implementation details

### AcpSession struct

```rust
pub struct AcpSession {
    pub session_id: String,
    pub session_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub config_state: SessionConfigState,
    pub client_capabilities: ClientCapabilities,
    pub cancel_token: CancelToken,
    pub busy: Arc<AtomicBool>,
    pub mcp_servers: Vec<McpServerConfig>,
}
```

### SessionConfigState

```rust
#[derive(Debug, Clone)]
pub struct SessionConfigState {
    pub agent_mode: String,       // "code" | "plan" | "research" | "review" | "auto"
    pub model_tier: String,       // "auto" | "t0" | "t1" | "t2" | "t3"
    pub thinking: String,         // "auto" | "off" | "brief" | "verbose"
    pub gate_pipeline: bool,
    pub auto_correct: bool,
    pub knowledge_store: bool,
    pub daimon_enabled: bool,
}

impl Default for SessionConfigState {
    fn default() -> Self {
        Self {
            agent_mode: "code".into(),
            model_tier: "auto".into(),
            thinking: "auto".into(),
            gate_pipeline: true,
            auto_correct: true,
            knowledge_store: true,
            daimon_enabled: false,
        }
    }
}
```

### AcpConfig struct (in config.rs)

```rust
#[derive(Debug, Clone)]
pub struct AcpConfig {
    pub workdir: PathBuf,
    pub profile: String,
    pub config_path: Option<PathBuf>,
    pub log_file: PathBuf,
}

impl Default for AcpConfig {
    fn default() -> Self {
        Self {
            workdir: std::env::current_dir().unwrap_or_default(),
            profile: "default".into(),
            config_path: None,
            log_file: PathBuf::from(".roko/acp.log"),
        }
    }
}
```

### Session manager

```rust
pub struct SessionManager {
    sessions: HashMap<String, AcpSession>,
}

impl SessionManager {
    pub fn new() -> Self;
    pub fn create_session(&mut self, params: SessionNewParams) -> SessionNewResult;
    pub fn get_session(&self, id: &str) -> Option<&AcpSession>;
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut AcpSession>;
    pub fn list_sessions(&self) -> SessionListResult;
    pub fn load_session(&mut self, id: &str) -> Result<SessionNewResult>;
}
```

- Session IDs: `format!("sess_{}", uuid::Uuid::new_v4())`
- `create_session` returns `SessionNewResult` with empty config options (wired in ACP15) and mode info

### CancelToken

Create a simple cancel token (or use `tokio_util::sync::CancellationToken` if available, otherwise implement inline):

```rust
#[derive(Debug, Clone)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}
```

### Unit tests

- Test creating a session generates a valid session ID
- Test listing sessions returns correct count
- Test getting a non-existent session returns None
- Test default SessionConfigState values

## Verification

```bash
cargo test -p roko-acp --lib
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- SessionManager handles create/get/list/load
- SessionConfigState has correct defaults
- AcpConfig has all fields
- CancelToken supports cooperative cancellation
- Unit tests pass
