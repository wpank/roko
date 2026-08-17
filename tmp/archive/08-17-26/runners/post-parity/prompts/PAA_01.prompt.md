# PAA_01: Wire WebSocket ws/terminal/{id} PTY endpoint for Builder terminal

## Task
Implement the PTY WebSocket endpoint that the demo app's Builder page requires for terminal interaction.

## Runner Context
Runner PAA, batch 1 of 3. No dependencies.

## Problem
`demo/demo-app/src/hooks/useTerminal.ts:186`:
```ts
const ws = new WebSocket(`${WS_BASE}/ws/terminal/${id}`);
```

Protocol: server sends `ArrayBuffer` (PTY output), client sends raw text (input) or `JSON { type: 'resize', cols, rows }`. Without this endpoint, the Builder page's terminal is non-functional.

## Exact Changes

### Step 1: Check if terminal.rs route file exists

Search `crates/roko-serve/src/routes/` for `terminal.rs`. If it exists, check what's implemented. If not, create it.

### Step 2: Implement PTY WebSocket handler

```rust
async fn terminal_ws(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_session(socket, id, state))
}

async fn handle_terminal_session(socket: WebSocket, id: String, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Spawn PTY with default shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let pty = portable_pty::native_pty_system();
    let pair = pty.openpty(PtySize { rows: 24, cols: 80, .. }).unwrap();
    let mut child = pair.slave.spawn_command(CommandBuilder::new(&shell))?;
    let mut reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    // PTY output → WebSocket
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => { sender.send(Message::Binary(buf[..n].to_vec())).await.ok(); }
                _ => break,
            }
        }
    });

    // WebSocket input → PTY
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(resize) = serde_json::from_str::<ResizeMsg>(&text) {
                    pair.master.resize(PtySize { rows: resize.rows, cols: resize.cols, .. }).ok();
                } else {
                    writer.write_all(text.as_bytes()).ok();
                }
            }
            Message::Binary(data) => { writer.write_all(&data).ok(); }
            _ => {}
        }
    }
}
```

### Step 3: Register route

```rust
.route("/ws/terminal/{id}", get(terminal_ws))
```

### Step 4: Add PTY dependency if needed

Check if `portable-pty` or similar is already in `Cargo.toml`. If not, add it.

## Write Scope
- `crates/roko-serve/src/routes/terminal.rs`

## Read-Only Context
- `demo/demo-app/src/hooks/useTerminal.ts` (protocol expectations)


## Verify
```bash
cargo build -p roko-serve 2>&1 | head -30
cargo test -p roko-serve 2>&1 | tail -20
```
## Acceptance Criteria
- `ws://localhost:6677/ws/terminal/{id}` connects and spawns a PTY
- Terminal output streams as binary WebSocket frames
- Text input forwarded to PTY stdin
- `{ type: 'resize', cols, rows }` JSON resizes the PTY

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
