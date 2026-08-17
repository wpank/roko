# Batch ACP09 — File system bridge

## Goal

Implement the file system bridge that routes file reads/writes through the editor when available, with fallback to local filesystem.

## Target files

- `crates/roko-acp/src/bridge_fs.rs` — File system bridge implementation

## Implementation details

### AcpFileSystem struct

```rust
pub struct AcpFileSystem {
    /// Whether the editor supports fs operations
    editor_fs_available: bool,
    /// Transport for sending requests to the editor
    transport: Arc<Mutex<StdioTransport>>,
    /// Fallback working directory for local FS
    workdir: PathBuf,
}
```

### Methods

1. **`new(transport, client_caps, workdir)`** — Check if client declared `fs` capability

2. **`read_file(&self, path: &str) -> Result<String>`**
   - If `editor_fs_available`: send `fs/read_text_file` request to editor, await response
   - Otherwise: `tokio::fs::read_to_string(path).await`

3. **`write_file(&self, path: &str, content: &str) -> Result<()>`**
   - If `editor_fs_available`: send `fs/write_text_file` request to editor, await response
   - Otherwise: `tokio::fs::write(path, content).await`

### JSON-RPC messages

Read file request:
```json
{"jsonrpc": "2.0", "id": N, "method": "fs/read_text_file", "params": {"path": "/abs/path"}}
```

Expected response:
```json
{"jsonrpc": "2.0", "id": N, "result": {"text": "file contents..."}}
```

Write file request:
```json
{"jsonrpc": "2.0", "id": N, "method": "fs/write_text_file", "params": {"path": "/abs/path", "content": "new contents"}}
```

Expected response:
```json
{"jsonrpc": "2.0", "id": N, "result": {}}
```

### Error handling

- If the editor returns an error, fall back to local FS
- If the path is not absolute, resolve relative to `workdir`
- Log all operations via tracing (to file, never stdout)

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- AcpFileSystem handles read/write with editor fallback
- Correct JSON-RPC messages are sent for fs operations
- Local FS fallback works when editor doesn't support fs
- All paths are resolved correctly
