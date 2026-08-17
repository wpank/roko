# Chain integration: wire roko-chain into the agent runtime

## Scope

Seven discrete gaps prevent agents from calling `chain.*` tools at runtime. All
of the underlying code exists. None of it is connected. This checklist closes
each gap in dependency order so the system compiles and passes integration tests
after every step.

Target state: an agent dispatched by `orchestrate.rs` can call `chain.balance`
to read the deployer's ETH balance on mirage and call `chain.transfer` to send
a native transfer — both going through the `AlloyChainClient` / `AlloyChainWallet`
Alloy backend talking to `https://mirage-devnet.up.railway.app` (Chain ID 1).

Workspace root: `/Users/will/dev/nunchi/roko/roko/`

---

## Implementation checklist

### Gap 1 — Enable `alloy-backend` in roko-cli

- [ ] **1.1** Open `crates/roko-cli/Cargo.toml`.
  Add a dependency entry directly below the existing `roko-` crate block (around
  line 35):
  ```toml
  roko-chain = { path = "../roko-chain", features = ["alloy-backend"] }
  ```
  Do not add `roko-chain` to `[dev-dependencies]`; it belongs in
  `[dependencies]` because chain handler code will live in non-test modules.

- [ ] **1.2** Verify the feature gate compiles cleanly:
  ```bash
  cargo build -p roko-cli 2>&1 | head -20
  ```
  Expected: no `unresolved import roko_chain` errors. There will be unused-import
  warnings until Gap 3 is closed; ignore them for now.

  Anti-pattern: do not add `alloy-backend` to `roko-std`'s `Cargo.toml`. The
  handler will live in `roko-cli`, which already owns the chain config.

---

### Gap 2 — Add `[chain]` section to `RokoConfig`

- [ ] **2.1** Open `crates/roko-core/src/config/schema.rs`.
  After the `oneirography: OneirographyConfig` field (line 175) and before the
  closing brace of `RokoConfig`, add:
  ```rust
  /// EVM chain connection settings for chain-domain tools.
  #[serde(default)]
  pub chain: ChainConfig,
  ```

- [ ] **2.2** In the same file, after the `OneirographyConfig` struct definition
  (search for `pub struct OneirographyConfig`), add the new struct:
  ```rust
  /// Chain connection settings used by the `chain.*` tool domain.
  ///
  /// ```toml
  /// [chain]
  /// rpc_url = "https://mirage-devnet.up.railway.app"
  /// chain_id = 1
  /// wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
  /// identity_registry   = "0x84eA74d481Ee0A5332c457a4d796187F6Ba67fEB"
  /// reputation_registry = "0x9E545E3C0baAB3E08CdfD552C960A1050f373042"
  /// validation_registry = "0xa82fF9aFd8f496c3d6ac40E2a0F282E47488CFc9"
  /// deployer            = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  /// ```
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
  pub struct ChainConfig {
      /// HTTP JSON-RPC endpoint (e.g. `https://mirage-devnet.up.railway.app`).
      #[serde(default)]
      pub rpc_url: Option<String>,
      /// Chain ID. Must match the endpoint. Mirage uses 1.
      #[serde(default)]
      pub chain_id: Option<u64>,
      /// Hex-encoded private key (0x-prefixed or bare). Used to sign txs.
      /// Load from an env var in production; never commit a real key.
      #[serde(default)]
      pub wallet_key: Option<String>,
      /// ERC-8004 IdentityRegistry contract address.
      #[serde(default)]
      pub identity_registry: Option<String>,
      /// ERC-8004 ReputationRegistry contract address.
      #[serde(default)]
      pub reputation_registry: Option<String>,
      /// ERC-8004 ValidationRegistry contract address.
      #[serde(default)]
      pub validation_registry: Option<String>,
      /// Deployer / funder address (Anvil account #0 on mirage).
      #[serde(default)]
      pub deployer: Option<String>,
  }
  ```

- [ ] **2.3** Update `RokoConfig::default()` (around line 186) to include the
  new field:
  ```rust
  chain: ChainConfig::default(),
  ```

- [ ] **2.4** Confirm `roko-core` still compiles and its existing tests pass:
  ```bash
  cargo test -p roko-core 2>&1 | tail -5
  ```

  Anti-pattern: do not put `wallet_key` in a non-`Option` field. An absent key
  means read-only mode; a missing field should not force every project to supply
  a dummy key.

---

### Gap 3 — Register chain tools in the tool registry

The agent runtime consults `ROKO_BUILTIN_TOOLS` to build the tool list it sends
to the LLM. `CHAIN_DOMAIN_TOOLS` is never merged in, so the model never sees
`chain.*` tool names.

- [ ] **3.1** Open `crates/roko-std/src/tool/builtin/mod.rs`.
  Change `TOOL_COUNT` from `16` to `30` (16 existing + 14 chain tools):
  ```rust
  pub const TOOL_COUNT: usize = 30;
  ```

- [ ] **3.2** In the same file, add an import at the top of the `use` block:
  ```rust
  use roko_chain::tools::CHAIN_DOMAIN_TOOLS;
  ```
  This requires `roko-chain` as a dependency of `roko-std`. Open
  `crates/roko-std/Cargo.toml` and add (no feature flag needed here — just the
  tool *definitions*, which have no alloy dependency):
  ```toml
  roko-chain = { path = "../roko-chain" }
  ```

- [ ] **3.3** Replace the `ROKO_BUILTIN_TOOLS` `LazyLock` body in
  `crates/roko-std/src/tool/builtin/mod.rs`. The current array has 16 entries.
  Chain the 14 chain tools after them. Because Rust fixed-size arrays cannot be
  concatenated with `+`, collect into a `Box<[ToolDef]>` instead, or change the
  type to `Vec<ToolDef>` — the `ToolRegistry` trait's `all()` returns `&[ToolDef]`
  so a `Vec` works fine.

  Preferred approach: change `ROKO_BUILTIN_TOOLS` to `LazyLock<Vec<ToolDef>>`:
  ```rust
  pub static ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>> = LazyLock::new(|| {
      let mut tools = vec![
          read_file::tool_def(),
          // ... remaining 15 existing tools (16 total) ...
      ];
      tools.extend(CHAIN_DOMAIN_TOOLS.iter().cloned());
      tools
  });
  ```
  Update `TOOL_COUNT` to remain a named constant but remove the `usize` assertion
  against the array length; instead assert `ROKO_BUILTIN_TOOLS.len() == TOOL_COUNT`
  in the existing test.

- [ ] **3.4** Update `StaticToolRegistry::all()` in
  `crates/roko-std/src/tool/registry.rs` (currently returns
  `ROKO_BUILTIN_TOOLS.as_slice()`). With `Vec<ToolDef>` the same call still
  works — no change needed.

- [ ] **3.5** Update `BUILTIN_TOOL_NAMES` in
  `crates/roko-std/src/tool/builtin/mod.rs` to include the chain tool names.
  Append:
  ```rust
  // append at end of BUILTIN_TOOL_NAMES array
  roko_chain::tools::CHAIN_TOOL_NAMES[0],  // "chain.balance"
  roko_chain::tools::CHAIN_TOOL_NAMES[1],  // "chain.transfer"
  // ... through index 13
  ```
  Or replace with a `Vec<&'static str>` constructed at `LazyLock` time — match
  whichever approach was chosen for `ROKO_BUILTIN_TOOLS`.

- [ ] **3.6** Update the following tests that will break when the tool count
  changes from 16 to 30:
  - `all_16_builtins_ship_handlers` — rename and update expected count to 30
  - `shipped_names_and_builtin_names_agree` — will fail because new chain tool
    names are added to `BUILTIN_TOOL_NAMES` but shipped handler names do not
    yet include them (handlers live in `roko-cli`, not `roko-std`)
  - `for_role_preserves_allowlist_invariants` — may need updated role tool
    allowlists if chain tools are role-gated
  - `all_len_matches_tool_count` — update `TOOL_COUNT` assertion from 16 to 30

  Confirm all pass after updates:
  ```bash
  cargo test -p roko-std 2>&1 | tail -10
  ```

  Anti-pattern: do not create a separate `ChainToolRegistry` struct. There is
  already one `StaticToolRegistry`; extend it rather than splitting registries.

---

### Gap 4 — Implement `ChainToolHandler` and wire it into `handler_for()`

The tool dispatcher calls `handler_for(name)` to get an executor. All `chain.*`
names currently fall through to `_ => None`.

- [ ] **4.1** Create `crates/roko-cli/src/chain_handler.rs` (new file, not in
  `roko-std` — it needs the `alloy-backend` feature from `roko-chain`).

  The handler needs access to a `Arc<dyn ChainClient>` for reads and an
  `Option<Arc<dyn ChainWallet>>` for writes. These are passed in at construction
  time and stored in the handler struct.

  Minimal skeleton:
  ```rust
  //! Handler for chain.* tool calls. Routes JSON args to AlloyChainClient /
  //! AlloyChainWallet.

  use std::sync::Arc;
  use async_trait::async_trait;
  use roko_chain::{ChainClient, ChainWallet, TxRequest};
  use roko_core::tool::{ToolCall, ToolContext, ToolError, ToolHandler, ToolResult};

  pub struct ChainToolHandler {
      pub client: Arc<dyn ChainClient>,
      pub wallet: Option<Arc<dyn ChainWallet>>,
      pub name: &'static str,
  }

  #[async_trait]
  impl ToolHandler for ChainToolHandler {
      async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
          let args = call.arguments.clone();
          match self.name {
              "chain.balance"   => handle_balance(&*self.client, args).await,
              "chain.transfer"  => {
                  let w = self.wallet.as_ref()
                      .ok_or_else(|| ToolError::Other("no wallet configured".into()))?;
                  handle_transfer(&**w, args).await
              }
              "chain.simulate_tx" => handle_simulate(&*self.client, args).await,
              "chain.gas_estimate" => handle_gas_estimate(&*self.client, args).await,
              // stub remaining tools with a clear error until implemented:
              other => Err(ToolError::Other(format!("chain tool not yet implemented: {other}"))),
          }
      }
  }
  ```

  **Important:** `ToolResult` is an enum with variants `Ok { content, is_structured, artifacts }`
  and `Err(ToolError)`, NOT `Result<String, ToolError>`. All handler functions
  (`handle_balance`, `handle_transfer`, etc.) must return `ToolResult::Ok { .. }` on
  success, not `Ok(string)`. For example:
  ```rust
  ToolResult::Ok {
      content: serde_json::to_string(&json!({ "balance_wei": balance })).unwrap(),
      is_structured: true,
      artifacts: vec![],
  }
  ```

- [ ] **4.2** Implement `handle_balance`. Extract `address: String` and optional
  `token: Option<String>` from `args`. If `token` is absent, call
  `client.eth_call` with the ERC-20 `balanceOf(address)` ABI selector
  (`0x70a08231`) for ERC-20, or use `client.eth_call` with an empty `data` field
  plus `value = 0` to read native balance via the provider. For native balance
  specifically, the `AlloyChainClient` does not expose `get_balance` directly —
  encode a call to a balance-reading precompile or add a `get_balance(addr)`
  method to `ChainClient` trait.

  Simplest correct approach for native balance: add `get_balance` to
  `ChainClient` trait in `crates/roko-chain/src/client.rs`:
  ```rust
  /// Native token balance at `block` (or latest if `None`), in wei.
  async fn get_balance(
      &self,
      address: &str,
      block: Option<BlockNumber>,
  ) -> ChainResult<u128>;
  ```
  Implement it on `AlloyChainClient` (call `self.provider.get_balance(addr).await`)
  and on `MockChainClient` (return a seeded value). **Note:** `MockChainClient`
  needs a `balances: HashMap<String, u128>` field to serve `get_balance` queries.
  Add this field and populate it via the `paired_mocks(initial_balance)` factory
  so the mock can return address-specific balances. The existing mock state struct
  does not have a balance field.

- [ ] **4.3** Implement `handle_transfer`. Extract `to: String`, `amount: String`
  (parse as `u128` wei), optional `token`. For native transfer, construct a
  `TxRequest { to: Some(to), value: amount_u128, ..Default::default() }` and
  call `wallet.sign_and_submit(tx).await`. Return the tx hash as a JSON string.
  ERC-20 transfer ABI encoding: `bytes4(keccak256("transfer(address,uint256)"))` =
  `0xa9059cbb`, then ABI-encode `(to_address, amount)` as two 32-byte words and
  put the result in `TxRequest.data`.

- [ ] **4.4** Implement `handle_simulate_tx`. Parse `to`, `data` (hex string to
  bytes), optional `value` and `from` from `args`. Build a `TxRequest` and call
  `client.eth_call(&req, block).await`. Return `{ "output": "0x...", "gas_used": N }`.

- [ ] **4.5** Implement `handle_gas_estimate`. For now, call `handle_simulate_tx`
  and return the gas figure from the result, multiplied by 1.2 as specified in
  the tool definition. Mark as `// TODO: use eth_estimateGas` so a future pass
  can swap in the real RPC method.

- [ ] **4.6** Add a module declaration in `crates/roko-cli/src/lib.rs`:
  ```rust
  pub mod chain_handler;
  ```

- [ ] **4.7** The roko-std `handler_for()` function in
  `crates/roko-std/src/tool/handlers.rs` cannot call `ChainToolHandler` because
  it does not have access to a live `ChainClient` — `handler_for` takes only
  `name: &str`. The chain handlers are stateful (they need an RPC connection).

  Solution: add a second lookup path in `crates/roko-cli`. Create
  `crates/roko-cli/src/chain_registry.rs`:
  ```rust
  //! Merges chain tool handlers into the dispatcher at construction time.

  use std::collections::HashMap;
  use std::sync::Arc;
  use roko_chain::{ChainClient, ChainWallet};
  use roko_chain::tools::CHAIN_TOOL_NAMES;
  use roko_core::tool::ToolHandler;
  use crate::chain_handler::ChainToolHandler;

  /// Build a map of chain tool name → handler, given live client/wallet.
  pub fn chain_handlers(
      client: Arc<dyn ChainClient>,
      wallet: Option<Arc<dyn ChainWallet>>,
  ) -> HashMap<&'static str, Arc<dyn ToolHandler>> {
      CHAIN_TOOL_NAMES
          .iter()
          .map(|&name| {
              let h: Arc<dyn ToolHandler> = Arc::new(ChainToolHandler {
                  client: Arc::clone(&client),
                  wallet: wallet.clone(),
                  name,
              });
              (name, h)
          })
          .collect()
  }
  ```

  Anti-pattern: do not add a `dyn ChainClient` field to `HandlerRegistry` in
  `roko-std`. That crate must stay chain-agnostic so mock-only consumers do not
  pull in alloy. Keep chain handler construction in `roko-cli`.

---

### Gap 5 — Build chain context in `orchestrate.rs` and pass it to agents

- [ ] **5.1** Open `crates/roko-cli/src/orchestrate.rs`. Find the import block
  near the top. Add:
  ```rust
  use roko_chain::alloy_impl::{AlloyChainClient, AlloyChainWallet};
  use roko_chain::{ChainClient, ChainWallet};
  use crate::chain_registry::chain_handlers;
  ```

- [ ] **5.2** Find the `PlanRunner` struct (the orchestration state struct).
  Add two fields:
  ```rust
  /// Read-only chain client. None if [chain] rpc_url is not configured.
  chain_client: Option<Arc<dyn ChainClient>>,
  /// Signing wallet. None if wallet_key is not configured.
  chain_wallet: Option<Arc<dyn ChainWallet>>,
  ```

- [ ] **5.3** Find where `PlanRunner` is constructed. **Note:** there is no
  simple `PlanRunner::new` constructor. Construction is a long inline block
  within the three factory methods (`from_plans_dir`, `from_snapshot`,
  `from_snapshots`). The chain client/wallet initialization must be added
  to each of these factory methods. After the `RokoConfig` is loaded, add:
  ```rust
  let chain_client = config.chain.rpc_url.as_deref().map(|url| {
      AlloyChainClient::http(url)
          .map(|c| Arc::new(c) as Arc<dyn ChainClient>)
          .unwrap_or_else(|e| {
              tracing::warn!(error = %e, "chain client failed to initialize; chain tools disabled");
              // Leave chain_client as None by returning early via Option
              panic!("unreachable — handled below")
          })
  });
  // Cleaner: use a match
  let chain_client: Option<Arc<dyn ChainClient>> = match config.chain.rpc_url.as_deref() {
      Some(url) => match AlloyChainClient::http(url) {
          Ok(c) => Some(Arc::new(c)),
          Err(e) => {
              tracing::warn!(error = %e, "chain rpc_url is set but client failed; chain tools disabled");
              None
          }
      },
      None => None,
  };

  let chain_wallet: Option<Arc<dyn ChainWallet>> = match (
      config.chain.rpc_url.as_deref(),
      config.chain.wallet_key.as_deref(),
  ) {
      (Some(url), Some(key)) => {
          let chain_id = config.chain.chain_id.unwrap_or(1);
          match AlloyChainWallet::from_hex_key(url, key, chain_id) {
              Ok(w) => Some(Arc::new(w)),
              Err(e) => {
                  tracing::warn!(error = %e, "wallet_key invalid; sign-and-submit disabled");
                  None
              }
          }
      }
      _ => None,
  };
  ```
  Store both into the new `PlanRunner` fields.

- [ ] **5.4** Fix the hardcoded `chain_connected: false` at lines 6325-6326 in
  the status snapshot method:
  ```rust
  chain_connected: self.chain_client.is_some(),
  chain_expected: self.chain_client.is_some(),
  ```

- [ ] **5.5** Find `dispatch_agent_with`. It creates a `SpawnAgentSpec` and
  calls `spawn_agent_with_layer`. The agent that runs does not yet receive chain
  tool handlers. After the agent is created but before it runs, build the chain
  handler map and register it with the agent's tool dispatcher.

  If the agent's `run()` method pulls handlers from a shared `HandlerRegistry`
  that was passed at construction time, the cleanest approach is to construct a
  compound registry. The exact mechanism depends on whether `roko-agent`'s
  dispatcher accepts runtime-supplied extra handlers.

  **IMPORTANT:** Neither `AgentOptions` (at `crates/roko-agent/src/provider/mod.rs:422-438`)
  nor `SpawnAgentSpec` (at `crates/roko-cli/src/agent_spawn.rs:11-42`) has an
  `extra_handlers` field. There is no existing handler injection mechanism in
  `roko-agent`. This must be added as a prerequisite step.

  **5.5a** Add `with_extra_handlers()` to `ToolDispatcher` in
  `crates/roko-agent/src/dispatcher/mod.rs`:

  ```rust
  impl ToolDispatcher {
      /// Register additional tool handlers that will be consulted before the
      /// default handler_for() lookup. Chain tool handlers use this path.
      pub fn with_extra_handlers(
          mut self,
          handlers: HashMap<String, Arc<dyn ToolHandler>>,
      ) -> Self {
          self.extra_handlers = handlers;
          self
      }
  }
  ```

  Add the corresponding field to the `ToolDispatcher` struct:
  ```rust
  pub extra_handlers: HashMap<String, Arc<dyn ToolHandler>>,
  ```

  And update `ToolDispatcher::dispatch()` to check `self.extra_handlers.get(name)`
  before falling through to the existing `handler_for(name)` lookup.

  **5.5b** Thread the extra handlers through `SpawnAgentSpec` or pass them
  directly to the dispatcher at construction time. The cleanest approach is to
  add an `extra_handlers: HashMap<String, Arc<dyn ToolHandler>>` field to
  `SpawnAgentSpec` and forward it in `spawn_agent_with_layer` when building
  the `ToolDispatcher`.

  **5.5c** Then in `dispatch_agent_with`, after building `SpawnAgentSpec`:
  ```rust
  if let Some(client) = &self.chain_client {
      let handlers = chain_handlers(
          Arc::clone(client),
          self.chain_wallet.clone(),
      );
      // inject into spawn spec
      for (name, handler) in handlers {
          spec.extra_handlers.insert(name.to_string(), handler);
      }
  }
  ```

  Anti-pattern: do not pass chain credentials as environment variables to the
  child process. The wallet key must never leave the roko-cli process boundary;
  handlers must run in-process, not as subprocess calls.

---

### Gap 6 — Activate `ChainConfig` in `agent_serve.rs`

`agent_serve.rs` already parses `--chain-rpc-url` and `--wallet-key` into a
local `ChainConfig` struct, then logs them and does nothing. Wire them up.

- [ ] **6.1** Open `crates/roko-cli/src/agent_serve.rs`. Add imports:
  ```rust
  use roko_chain::alloy_impl::{AlloyChainClient, AlloyChainWallet};
  use roko_chain::{ChainClient, ChainWallet};
  use crate::chain_registry::chain_handlers;
  ```

- [ ] **6.2** Find `AgentServeRuntimeConfig::build_server()` (around line 145).
  Replace the log-only chain block in the `on_start` callback. Before calling
  `builder.build()`, construct the chain context:
  ```rust
  let chain_context: Option<(Arc<dyn ChainClient>, Option<Arc<dyn ChainWallet>>)> =
      self.chain.as_ref().and_then(|cfg| {
          let url = cfg.rpc_url.as_deref()?;
          let client = AlloyChainClient::http(url)
              .map_err(|e| tracing::warn!(error = %e, "chain client init failed"))
              .ok()?;
          let client = Arc::new(client) as Arc<dyn ChainClient>;
          // NOTE: The local ChainConfig struct in agent_serve.rs has no `chain_id`
          // field. Chain ID is hardcoded to 1 (mirage default) here. If multi-chain
          // support is needed, add `chain_id: Option<u64>` to the local ChainConfig
          // or read it from RokoConfig's [chain] section.
          let chain_id = 1u64;
          let wallet = cfg.wallet_key.as_deref().map(|key| {
              AlloyChainWallet::from_hex_key(url, key, chain_id)
                  .map(|w| Arc::new(w) as Arc<dyn ChainWallet>)
                  .map_err(|e| tracing::warn!(error = %e, "wallet init failed"))
                  .ok()
          }).flatten();
          Some((client, wallet))
      });
  ```

- [ ] **6.3** Remove the `warn!("signing hooks are not wired in this batch")` line
  (currently at line 193). Replace the entire `if let Some(chain) = &startup.chain`
  block in the `on_start` callback with:
  ```rust
  if let Some((client, wallet)) = chain_context {
      info!(
          agent_id = %startup.agent_id,
          chain_backend = %client.name(),
          has_wallet = wallet.is_some(),
          "chain tools active"
      );
      // chain_handlers are injected into the dispatcher separately (see Gap 5)
  }
  ```

  Anti-pattern: do not spawn the `roko-chain-watcher` binary from here. That is
  a separate concern (Gap 7). This gap only activates the tool handlers.

---

### Gap 7 — Spawn `roko-chain-watcher` from `roko serve`

`apps/roko-chain-watcher/` is a standalone binary. `roko serve` does not start
it. Wire it in.

- [ ] **7.1** Confirm the watcher binary exists and builds:
  ```bash
  cargo build -p roko-chain-watcher 2>&1 | tail -5
  ```

- [ ] **7.2** Open `crates/roko-serve/src/state.rs` (or wherever the serve
  startup logic lives). Find the startup sequence. After all routes are
  registered and before `axum::serve` is called, add a conditional watcher
  spawn:
  ```rust
  if let Some(rpc_url) = &config.chain.rpc_url {
      let rpc_url = rpc_url.clone();
      let app_state = Arc::clone(&state);
      tokio::spawn(async move {
          // Resolve the watcher binary path relative to the current executable.
          let watcher = std::env::current_exe()
              .ok()
              .and_then(|p| p.parent().map(|d| d.join("roko-chain-watcher")))
              .unwrap_or_else(|| std::path::PathBuf::from("roko-chain-watcher"));

          let status = tokio::process::Command::new(&watcher)
              .arg("--rpc-url")
              .arg(&rpc_url)
              .status()
              .await;

          match status {
              Ok(s) => tracing::info!(exit = %s, "chain-watcher exited"),
              Err(e) => tracing::warn!(error = %e, "chain-watcher failed to start"),
          }
      });
  }
  ```

- [ ] **7.3** If `roko-chain-watcher` does not accept `--rpc-url` as a flag,
  check its `main.rs` argument structure and pass the correct flag name. Do not
  change the watcher's interface; adapt the caller.

- [ ] **7.4** This is a best-effort spawn. If the binary is absent (e.g. in
  unit test environments), `roko serve` must still start cleanly. The `Err(e)`
  arm in step 7.2 handles this — it logs a warning and continues.

---

## Concrete file touchpoints

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Add `roko-chain = { path = "../roko-chain", features = ["alloy-backend"] }` |
| `crates/roko-std/Cargo.toml` | Add `roko-chain = { path = "../roko-chain" }` (no feature) |
| `crates/roko-core/src/config/schema.rs` | Add `ChainConfig` struct + `chain: ChainConfig` field to `RokoConfig` |
| `crates/roko-std/src/tool/builtin/mod.rs` | Extend `ROKO_BUILTIN_TOOLS` with `CHAIN_DOMAIN_TOOLS`; update count |
| `crates/roko-std/src/tool/registry.rs` | No change if `Vec<ToolDef>` approach used; update length test |
| `crates/roko-std/src/tool/handlers.rs` | No change (chain handlers are stateful; they live in roko-cli) |
| `crates/roko-cli/src/chain_handler.rs` | New file: `ChainToolHandler` with balance, transfer, simulate, gas |
| `crates/roko-cli/src/chain_registry.rs` | New file: `chain_handlers()` factory |
| `crates/roko-cli/src/lib.rs` | Add `pub mod chain_handler; pub mod chain_registry;` |
| `crates/roko-cli/src/orchestrate.rs` | Add chain_client/chain_wallet fields to PlanRunner; construct in `new`; inject into dispatch |
| `crates/roko-cli/src/agent_serve.rs` | Replace log-only chain block with `AlloyChainClient` + `AlloyChainWallet` construction |
| `crates/roko-chain/src/client.rs` | Add `get_balance()` method to `ChainClient` trait |
| `crates/roko-chain/src/alloy_impl.rs` | Implement `get_balance` on `AlloyChainClient` |
| `crates/roko-chain/src/mock.rs` | Implement `get_balance` on `MockChainClient` |
| `crates/roko-serve/src/state.rs` (or equivalent) | Spawn `roko-chain-watcher` on startup when `chain.rpc_url` is set |

---

## Verification checklist

- [ ] `cargo build --workspace` passes with no errors.
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean.
- [ ] `cargo test --workspace` passes — all pre-existing tests green.
- [ ] `cargo test -p roko-std` — registry length test passes with count 30.
- [ ] `cargo test -p roko-core` — `RokoConfig` round-trips through TOML with a
  `[chain]` section present.
- [ ] Manual smoke test: add the following block to a local `roko.toml`, run
  `roko serve`, and confirm the log line "chain tools active" appears:
  ```toml
  [chain]
  rpc_url   = "https://mirage-devnet.up.railway.app"
  chain_id  = 1
  wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
  deployer  = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  ```
- [ ] `chain.balance` tool call returns the deployer's balance (nonzero on
  mirage) when invoked against the live endpoint.
- [ ] `chain.transfer` tool call sends 1 wei from the deployer to a second
  address and returns a tx hash starting with `0x`.
- [ ] `chain_connected: true` appears in the output of `roko status` when chain
  config is present.

---

## Acceptance criteria

These are the integration tests that must pass for this work to be considered
complete. Write them in `crates/roko-cli/tests/chain_integration.rs` (or
`crates/roko-serve/tests/chain_integration.rs` if the test requires a running
server).

### Test 1 — `chain.balance` against a mock RPC

```rust
/// Confirm that the ChainToolHandler routes `chain.balance` through the client
/// and returns a JSON-serializable wei amount.
#[tokio::test]
async fn chain_balance_handler_returns_wei() {
    use roko_chain::mock::{MockChainClient, MockChainWallet, paired_mocks};
    use crate::chain_handler::ChainToolHandler;
    use roko_core::tool::{ToolCall, ToolContext, ToolHandler, ToolResult};
    use std::sync::Arc;

    // Seed the mock so get_balance returns a known value.
    let (client, _wallet) = paired_mocks(1_000_000_000_000_000_000u128); // 1 ETH
    let handler = ChainToolHandler {
        client: Arc::new(client) as Arc<dyn roko_chain::ChainClient>,
        wallet: None,
        name: "chain.balance",
    };

    let call = ToolCall {
        name: "chain.balance".to_string(),
        arguments: serde_json::json!({
            "address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
        }),
        ..ToolCall::default()
    };
    let ctx = ToolContext::default();
    let result = handler.execute(call, &ctx).await;
    // ToolResult is an enum: ToolResult::Ok { content, is_structured, artifacts }
    match result {
        ToolResult::Ok { content, .. } => {
            let balance: serde_json::Value = serde_json::from_str(&content).unwrap();
            let wei = balance["balance_wei"].as_u64().unwrap_or(0);
            assert!(wei > 0, "expected nonzero wei balance from mock");
        }
        ToolResult::Err(e) => panic!("balance call must succeed, got: {e:?}"),
    }
}
```

### Test 2 — `chain.transfer` against a mock RPC

```rust
/// Confirm that the ChainToolHandler routes `chain.transfer` through the wallet,
/// submits a tx, and returns a tx hash.
#[tokio::test]
async fn chain_transfer_handler_returns_tx_hash() {
    use roko_chain::mock::paired_mocks;
    use crate::chain_handler::ChainToolHandler;
    use roko_core::tool::{ToolCall, ToolContext, ToolHandler, ToolResult};
    use std::sync::Arc;

    let (client, wallet) = paired_mocks(10_000_000_000_000_000_000u128); // 10 ETH
    let handler = ChainToolHandler {
        client: Arc::new(client.clone()),
        wallet: Some(Arc::new(wallet.clone())),
        name: "chain.transfer",
    };

    let call = ToolCall {
        name: "chain.transfer".to_string(),
        arguments: serde_json::json!({
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "amount": "1000000000000000000"  // 1 ETH in wei
        }),
        ..ToolCall::default()
    };
    let ctx = ToolContext::default();
    let result = handler.execute(call, &ctx).await;
    match result {
        ToolResult::Ok { content, .. } => {
            let resp: serde_json::Value = serde_json::from_str(&content).unwrap();
            let hash = resp["tx_hash"].as_str().unwrap_or("");
            assert!(hash.starts_with("0x"), "tx hash must be 0x-prefixed hex");
        }
        ToolResult::Err(e) => panic!("transfer must succeed, got: {e:?}"),
    }

    // The mock wallet should have recorded one submitted tx.
    assert_eq!(wallet.submitted().len(), 1);
}
```

### Test 3 — End-to-end against mirage (skipped unless `MIRAGE_RPC_URL` is set)

```rust
/// Integration test against a live or CI-provided mirage endpoint.
/// Skipped when MIRAGE_RPC_URL is not set.
///
/// Requires: MIRAGE_RPC_URL, MIRAGE_WALLET_KEY, MIRAGE_DEPLOYER env vars.
/// Mirage endpoint: https://mirage-devnet.up.railway.app (Chain ID 1)
/// Deployer (Anvil #0): 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
#[tokio::test]
async fn chain_balance_and_transfer_against_mirage() {
    let Some(rpc_url) = std::env::var("MIRAGE_RPC_URL").ok() else {
        eprintln!("MIRAGE_RPC_URL not set; skipping live chain test");
        return;
    };
    let wallet_key = std::env::var("MIRAGE_WALLET_KEY")
        .unwrap_or_else(|_| {
            // Anvil account #0 private key — safe to use on mirage devnet only.
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()
        });
    let deployer = std::env::var("MIRAGE_DEPLOYER")
        .unwrap_or_else(|_| "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string());
    let recipient = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"; // Anvil account #1

    use roko_chain::alloy_impl::{AlloyChainClient, AlloyChainWallet};
    use crate::chain_handler::ChainToolHandler;
    use roko_core::tool::{ToolCall, ToolContext, ToolHandler, ToolResult};
    use std::sync::Arc;

    let client = AlloyChainClient::http(&rpc_url).expect("http client");
    let wallet = AlloyChainWallet::from_hex_key(&rpc_url, &wallet_key, 1).expect("wallet");
    let client = Arc::new(client) as Arc<dyn roko_chain::ChainClient>;
    let wallet = Arc::new(wallet) as Arc<dyn roko_chain::ChainWallet>;
    let ctx = ToolContext::default();

    // Step 1: read the deployer balance.
    let balance_handler = ChainToolHandler {
        client: Arc::clone(&client),
        wallet: None,
        name: "chain.balance",
    };
    let bal_call = ToolCall {
        name: "chain.balance".to_string(),
        arguments: serde_json::json!({ "address": deployer }),
        ..ToolCall::default()
    };
    let bal_result = balance_handler.execute(bal_call, &ctx).await;
    let bal_content = match bal_result {
        ToolResult::Ok { content, .. } => content,
        ToolResult::Err(e) => panic!("balance call failed: {e:?}"),
    };
    let bal: serde_json::Value = serde_json::from_str(&bal_content).unwrap();
    let balance_before = bal["balance_wei"]
        .as_str()
        .and_then(|s| s.parse::<u128>().ok())
        .expect("balance_wei must be a numeric string");
    assert!(balance_before > 0, "deployer must have ETH on mirage");

    // Step 2: send 1 wei to recipient.
    let transfer_handler = ChainToolHandler {
        client: Arc::clone(&client),
        wallet: Some(Arc::clone(&wallet)),
        name: "chain.transfer",
    };
    let tx_call = ToolCall {
        name: "chain.transfer".to_string(),
        arguments: serde_json::json!({
            "to": recipient,
            "amount": "1"
        }),
        ..ToolCall::default()
    };
    let tx_result = transfer_handler.execute(tx_call, &ctx).await;
    let tx_content = match tx_result {
        ToolResult::Ok { content, .. } => content,
        ToolResult::Err(e) => panic!("transfer call failed: {e:?}"),
    };
    let tx: serde_json::Value = serde_json::from_str(&tx_content).unwrap();
    let hash = tx["tx_hash"].as_str().expect("tx_hash field");
    assert!(hash.starts_with("0x"), "hash must be 0x-prefixed");
    assert_eq!(hash.len(), 66, "tx hash must be 32 bytes (66 chars with 0x prefix)");

    // Step 3: verify the receipt landed on mirage.
    let receipt = wallet
        .wait_for_receipt(&roko_chain::TxHash::new(hash), 30_000)
        .await
        .expect("receipt within 30s");
    assert!(receipt.status, "transfer must succeed on mirage");
    assert!(receipt.block_number > 0, "tx must be mined into a block");
}
```

The test at step 3 is the definitive acceptance gate. It will be run in CI with
`MIRAGE_RPC_URL=https://mirage-devnet.up.railway.app`. It passes when the
receipt comes back with `status: true` and `block_number > 0`.

---

## Errata applied

Corrections applied 2026-04-22 based on audit discrepancy report:

1. **BLOCKER FIX: Wrong `ToolHandler` trait signature.** All code snippets in Gap 4
   and all integration tests updated from `async fn call(&self, args: Value) -> ToolResult`
   to the real signature: `async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult`.
   `ToolResult` is documented as an enum (`Ok { content, is_structured, artifacts }` /
   `Err(ToolError)`), not `Result<String, ToolError>`.

2. **BLOCKER FIX: `AgentOptions` has no `extra_handlers` field.** Gap 5.5 expanded
   with a detailed three-step plan (5.5a, 5.5b, 5.5c) to add
   `with_extra_handlers()` to `ToolDispatcher`, thread it through `SpawnAgentSpec`,
   and inject chain handlers at dispatch time. The original text assumed the
   injection mechanism already existed.

3. **`ROKO_BUILTIN_TOOLS` count corrected.** Comment changed from "remaining 15"
   to "remaining 15 existing tools (16 total)" to reflect the actual 16 builtin
   tools.

4. **Test breakage documented.** Gap 3.6 now lists the four tests that will break:
   `all_16_builtins_ship_handlers`, `shipped_names_and_builtin_names_agree`,
   `for_role_preserves_allowlist_invariants`, `all_len_matches_tool_count`.

5. **`MockChainClient` `balances` field documented.** Gap 4.2 now explicitly notes
   that `MockChainClient` needs a `balances: HashMap<String, u128>` field for the
   new `get_balance` trait method.

6. **`ChainConfig` in `agent_serve.rs` chain_id documented.** Gap 6.2 now notes
   the local `ChainConfig` struct has no `chain_id` field and documents the
   hardcoded `chain_id = 1` approach.

7. **`PlanRunner` constructor clarified.** Gap 5.3 now notes there is no simple
   `PlanRunner::new` constructor; construction is a long inline block within three
   factory methods (`from_plans_dir`, `from_snapshot`, `from_snapshots`).
