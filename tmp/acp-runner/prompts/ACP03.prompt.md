# Batch ACP03 — Stdio transport layer

## Goal

Implement the stdio transport for reading and writing JSON-RPC messages.

## Target files

- `crates/roko-acp/src/transport.rs` — Complete implementation

## Implementation details

### StdioTransport struct

```rust
pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
    next_id: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
}
```

### Methods

1. **`new()`** — Create transport from stdin/stdout

2. **`read_message(&mut self) -> Result<Option<JsonRpcMessage>>`** — Read one line from stdin, parse as JSON-RPC. Return `None` on EOF.

3. **`send_response(&mut self, id: JsonRpcId, result: serde_json::Value) -> Result<()>`** — Serialize response + newline + flush to stdout.

4. **`send_error(&mut self, id: JsonRpcId, code: i32, message: String) -> Result<()>`** — Send JSON-RPC error response.

5. **`send_notification(&mut self, method: &str, params: serde_json::Value) -> Result<()>`** — Send notification (no id) to client.

6. **`send_request(&mut self, method: &str, params: serde_json::Value) -> Result<JsonRpcResponse>`** — Send a request TO the client (for fs/terminal/permission/elicitation), register a pending oneshot, and return the response when the client replies.

7. **`handle_incoming_response(&mut self, response: JsonRpcResponse)`** — Route incoming responses to pending request oneshots.

### Design notes

- Use `tokio::io::AsyncBufReadExt::read_line` for reading
- Use `tokio::io::AsyncWriteExt::write_all` + `flush` for writing
- The `pending_requests` map enables bidirectional flow: when the agent sends a request to the editor (e.g., `fs/read_text_file`), it registers a oneshot and waits for the response
- Thread safety: `pending_requests` behind `Arc<Mutex<>>` so the handler can route responses

### Unit tests

Write tests using mock readers/writers:
- Test reading a valid JSON-RPC request
- Test reading EOF returns None
- Test writing a response produces correct JSON
- Test writing a notification produces correct JSON
- Test send_request + handle_incoming_response round-trip

For tests, create helper constructors that accept `impl AsyncRead` and `impl AsyncWrite` instead of actual stdin/stdout.

## Verification

```bash
cargo test -p roko-acp --lib
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- StdioTransport handles read/write/request/response
- Bidirectional request flow works (agent -> editor -> agent)
- Unit tests pass
- No clippy warnings
