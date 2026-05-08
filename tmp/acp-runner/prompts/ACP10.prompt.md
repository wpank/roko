# Batch ACP10 — Terminal bridge

## Goal

Implement the terminal bridge that routes shell commands through the editor when available, with fallback to local process execution.

## Target files

- `crates/roko-acp/src/bridge_terminal.rs` — Terminal bridge implementation

## Implementation details

### AcpTerminal struct

```rust
pub struct AcpTerminal {
    /// Whether the editor supports terminal operations
    editor_terminal_available: bool,
    /// Transport for sending requests to the editor
    transport: Arc<Mutex<StdioTransport>>,
    /// Active terminals (terminal_id → metadata)
    active_terminals: HashMap<String, TerminalMeta>,
    /// Fallback working directory
    workdir: PathBuf,
}

struct TerminalMeta {
    command: String,
    created_at: chrono::DateTime<chrono::Utc>,
}
```

### Methods

1. **`new(transport, client_caps, workdir)`** — Check if client declared `terminal` capability

2. **`create(&mut self, params: TerminalCreateParams) -> Result<String>`**
   - If editor available: send `terminal/create` request, get terminal_id
   - Otherwise: spawn process locally via `tokio::process::Command`, generate local terminal_id
   - Track in `active_terminals`
   - Return terminal_id

3. **`output(&self, terminal_id: &str) -> Result<TerminalOutputResult>`**
   - If editor available: send `terminal/output` request
   - Otherwise: read from local process stdout/stderr

4. **`wait_for_exit(&self, terminal_id: &str) -> Result<TerminalOutputResult>`**
   - If editor available: send `terminal/wait_for_exit` request
   - Otherwise: wait on local process

5. **`kill(&mut self, terminal_id: &str) -> Result<()>`**
   - If editor available: send `terminal/kill` request
   - Otherwise: kill local process

6. **`release(&mut self, terminal_id: &str) -> Result<()>`**
   - Send `terminal/release` to editor, remove from active map
   - For local processes: drop handle, remove from map

### JSON-RPC messages

Create terminal:
```json
{"jsonrpc": "2.0", "id": N, "method": "terminal/create", "params": {"command": "cargo", "args": ["test"], "cwd": "/project"}}
```

Get output:
```json
{"jsonrpc": "2.0", "id": N, "method": "terminal/output", "params": {"terminalId": "term_001"}}
```

Wait for exit:
```json
{"jsonrpc": "2.0", "id": N, "method": "terminal/wait_for_exit", "params": {"terminalId": "term_001"}}
```

Kill:
```json
{"jsonrpc": "2.0", "id": N, "method": "terminal/kill", "params": {"terminalId": "term_001"}}
```

Release:
```json
{"jsonrpc": "2.0", "id": N, "method": "terminal/release", "params": {"terminalId": "term_001"}}
```

### Local fallback

For local process execution:
```rust
let child = tokio::process::Command::new(&params.command)
    .args(&params.args)
    .current_dir(params.cwd.as_deref().unwrap_or(&self.workdir))
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
```

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- AcpTerminal handles create/output/wait/kill/release with editor fallback
- Active terminals are tracked
- Local process fallback works correctly
- Correct JSON-RPC messages for all terminal operations
