# M154 — Wire Daemon IPC Socket Protocol

## Objective
Wire the daemon IPC protocol over Unix domain sockets. The daemon.rs already has `UnixListener` imports and a `DaemonCmd` enum, but the full JSON-RPC protocol with methods `list-subscriptions`, `add-subscription`, `remove-subscription`, `pause`, `resume`, `status` needs to be implemented. Wire the socket listener at the platform-appropriate path and connect to the `roko daemon` subcommands so CLI commands can communicate with a running daemon.

## Scope
- Crates: `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs` (extend IPC protocol)
- Depth doc: `tmp/unified-depth/14-deployment/` (daemon architecture)

## Steps
1. Read the existing daemon module and DaemonCmd:
   ```bash
   grep -n 'DaemonCmd\|pub enum\|pub struct\|pub async fn' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs | head -30
   ```

2. Check what socket path is already defined:
   ```bash
   grep -n 'sock\|socket\|unix\|ipc\|SOCKET_PATH\|daemon.*path' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs | head -15
   ```

3. Define the JSON-RPC protocol:
   ```rust
   /// JSON-RPC 2.0 request for daemon IPC.
   #[derive(Debug, Serialize, Deserialize)]
   pub struct DaemonRequest {
       pub jsonrpc: String, // always "2.0"
       pub id: u64,
       pub method: String,
       pub params: Option<serde_json::Value>,
   }

   /// JSON-RPC 2.0 response from daemon.
   #[derive(Debug, Serialize, Deserialize)]
   pub struct DaemonResponse {
       pub jsonrpc: String,
       pub id: u64,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub result: Option<serde_json::Value>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub error: Option<DaemonError>,
   }
   ```

4. Implement socket path resolution:
   ```rust
   /// Platform-appropriate socket path for daemon IPC.
   pub fn daemon_socket_path() -> PathBuf {
       #[cfg(target_os = "macos")]
       { PathBuf::from("/tmp/roko-daemon.sock") }
       #[cfg(target_os = "linux")]
       {
           std::env::var("XDG_RUNTIME_DIR")
               .map(|d| PathBuf::from(d).join("roko-daemon.sock"))
               .unwrap_or_else(|_| PathBuf::from("/tmp/roko-daemon.sock"))
       }
       #[cfg(not(any(target_os = "macos", target_os = "linux")))]
       { PathBuf::from("/tmp/roko-daemon.sock") }
   }
   ```

5. Implement the IPC server loop:
   ```rust
   /// Run the daemon IPC server on a Unix domain socket.
   pub async fn run_ipc_server(cancel: CancellationToken) -> Result<()> {
       let path = daemon_socket_path();
       // Remove stale socket
       let _ = std::fs::remove_file(&path);
       let listener = UnixListener::bind(&path)?;

       loop {
           tokio::select! {
               _ = cancel.cancelled() => break,
               result = listener.accept() => {
                   let (stream, _) = result?;
                   tokio::spawn(handle_ipc_connection(stream));
               }
           }
       }
       let _ = std::fs::remove_file(&path);
       Ok(())
   }
   ```

6. Implement method handlers:
   - `status` → return daemon uptime, active subscriptions count, last event time
   - `list-subscriptions` → return all configured subscriptions with enabled/disabled state
   - `add-subscription` → add a subscription entry (persists to config)
   - `remove-subscription` → remove by id
   - `pause` → disable event processing (subscriptions stay but don't fire)
   - `resume` → re-enable event processing

7. Wire CLI commands to connect as IPC client:
   ```rust
   /// Send a command to the running daemon via IPC socket.
   pub async fn send_daemon_cmd(method: &str, params: Option<serde_json::Value>) -> Result<DaemonResponse> {
       let path = daemon_socket_path();
       let stream = UnixStream::connect(&path).await
           .context("Daemon not running — start with `roko daemon start`")?;
       // Send request, read response
   }
   ```

8. Write tests:
   - Socket creation and cleanup on shutdown
   - `status` method returns valid response
   - Unknown method returns error
   - Client gets clear error when daemon not running

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- daemon
```

## What NOT to do
- Do NOT use TCP — Unix domain sockets only (platform-appropriate)
- Do NOT add HTTP/REST to the daemon — that is roko-serve's job
- Do NOT implement full subscription logic here — just the IPC transport layer
- Do NOT add authentication — daemon IPC is local-only (filesystem permissions suffice)
- Do NOT use a full JSON-RPC library — hand-roll the simple protocol (2 structs)
