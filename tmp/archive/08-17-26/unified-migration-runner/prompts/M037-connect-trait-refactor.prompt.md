# M037 — Define Connect trait and refactor existing connectors

## Objective
Define the `Connect` trait in roko-core as a formal protocol for external system connections (MCP servers, APIs, chain clients, etc.). Then refactor the MCP client in roko-agent to implement Connect, establishing the pattern for all future connector implementations.

## Scope
- Crates: `roko-core`, `roko-agent`
- Files:
  - `crates/roko-core/src/traits.rs` (Connect trait definition)
  - `crates/roko-core/src/connector.rs` (existing ConnectorKind, ConnectorConfig — align with Connect)
  - `crates/roko-agent/src/` (MCP client — primary refactor target)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.12
- Spec ref: `tmp/unified/12-CONNECTIVITY.md` §1-3 (Connect protocol), §4 (Exoskeleton bindings)
- Architecture ref: `tmp/architecture/04-connectivity.md` (if exists)

## Steps
1. Read the existing connector types:
   ```bash
   cat crates/roko-core/src/connector.rs
   ```

2. Read the unified connectivity spec:
   ```bash
   head -80 tmp/unified/12-CONNECTIVITY.md
   ```

3. Find the MCP client implementation:
   ```bash
   grep -rn 'mcp\|MCP\|McpClient\|mcp_config' crates/roko-agent/src/ --include='*.rs' | head -15
   ```

4. Define the Connect trait in `crates/roko-core/src/traits.rs`:
   ```rust
   /// Handle to an active connection.
   #[derive(Debug, Clone, PartialEq, Eq, Hash)]
   pub struct ConnectionHandle(pub String);

   /// Health status of a connection.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ConnectionHealth {
       pub is_connected: bool,
       pub latency_ms: Option<u64>,
       pub last_checked_at: DateTime<Utc>,
       pub error: Option<String>,
   }

   /// Reconnection strategy for failed connections.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ReconnectStrategy {
       /// No automatic reconnection.
       None,
       /// Exponential backoff with max retries.
       ExponentialBackoff { max_retries: u32, base_delay_ms: u64 },
       /// Fixed interval reconnection.
       FixedInterval { interval_ms: u64, max_retries: u32 },
   }

   /// Configuration for establishing a connection.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ConnectConfig {
       /// Connection identifier.
       pub id: String,
       /// Kind of connector (MCP, HTTP, Chain, etc.).
       pub kind: ConnectorKind,
       /// Connection-specific configuration (JSON).
       pub params: serde_json::Value,
       /// Reconnection strategy.
       #[serde(default)]
       pub reconnect: ReconnectStrategy,
   }

   /// Protocol for connecting to external systems.
   ///
   /// See: tmp/unified/12-CONNECTIVITY.md §1-3.
   #[async_trait::async_trait]
   pub trait Connect: Send + Sync {
       /// Establish a connection. Returns a handle for subsequent operations.
       async fn connect(&mut self, config: &ConnectConfig) -> Result<ConnectionHandle>;

       /// Check connection health.
       async fn health(&self, handle: &ConnectionHandle) -> ConnectionHealth;

       /// Close a connection.
       async fn disconnect(&mut self, handle: &ConnectionHandle) -> Result<()>;

       /// The kind of connections this connector supports.
       fn supported_kinds(&self) -> &[ConnectorKind];
   }
   ```

5. Align existing `ConnectorKind` with the new trait:
   ```bash
   grep -n 'ConnectorKind' crates/roko-core/src/connector.rs
   ```
   Add any missing variants (MCP, HTTP, WebSocket, Chain).

6. Implement Connect for the MCP client wrapper:
   ```rust
   /// MCP connector implementing the Connect trait.
   pub struct McpConnect {
       connections: HashMap<String, McpConnectionState>,
   }

   #[async_trait]
   impl Connect for McpConnect {
       async fn connect(&mut self, config: &ConnectConfig) -> Result<ConnectionHandle> {
           // Parse MCP-specific params from config.params
           // Start MCP server process
           // Return handle
       }

       async fn health(&self, handle: &ConnectionHandle) -> ConnectionHealth {
           // Check if MCP process is alive, measure latency
       }

       async fn disconnect(&mut self, handle: &ConnectionHandle) -> Result<()> {
           // Stop MCP server process
       }

       fn supported_kinds(&self) -> &[ConnectorKind] {
           &[ConnectorKind::Mcp]
       }
   }
   ```

7. Add a `ConnectorKind::Mcp` variant if not present.

8. Add tests:
   ```rust
   #[tokio::test]
   async fn mcp_connect_lifecycle() {
       let mut connector = McpConnect::new();
       let config = ConnectConfig { id: "test".into(), kind: ConnectorKind::Mcp, ... };
       let handle = connector.connect(&config).await.unwrap();
       let health = connector.health(&handle).await;
       assert!(health.is_connected);
       connector.disconnect(&handle).await.unwrap();
   }
   ```

9. Deprecate the old `ConnectorRegistry` in favor of the Connect trait:
   ```rust
   #[deprecated(note = "Use the Connect trait instead")]
   pub struct ConnectorRegistry { ... }
   ```

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-agent
cargo clippy --workspace --no-deps -- -D warnings
cargo test -p roko-core -- connect
cargo test -p roko-agent -- mcp
```

## What NOT to do
- Do NOT refactor roko-chain connectors — that's Phase 2+
- Do NOT make Connect required for agent dispatch — MCP passthrough should still work without it
- Do NOT implement query/execute methods on Connect yet — start with lifecycle (connect/health/disconnect)
- Do NOT break existing MCP config in roko.toml — the Connect wrapper should parse the same config format
- Do NOT remove the existing MCP passthrough code — wrap it, don't replace it
