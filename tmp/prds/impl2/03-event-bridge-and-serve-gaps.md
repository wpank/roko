# Event bridge and serve gaps

Six isolated gaps that prevent the HTTP control plane and CLI from working as a coherent
system. Each gap is self-contained: pick any one and the others stay unchanged.

---

## Scope

| Gap | Area | Net change |
|-----|------|-----------|
| A | EventBus → StateHub bridge (reverse direction) | ~60 lines, 1 new fn in `roko-serve/src/lib.rs` |
| B | `roko chat` sidecar routing | ~80 lines in `roko-cli/src/chat.rs` |
| C | Sidecar `/research` LLM dispatch | ~50 lines in `roko-agent-server/src/state.rs` |
| D | roko-mcp-code auto-discovery | ~70 lines in `roko-cli/src/orchestrate.rs` |
| E | Subscriptions only trigger from WebhookReceived | ~60 lines in `roko-serve/src/dispatch.rs` |
| F | Provider health not wired in serve dispatch | ~40 lines in `roko-serve/src/dispatch.rs` + `state.rs` |

No new crates. No new public APIs beyond what already exists. Each gap is a wiring
problem, not a design problem.

**What is NOT in scope here:**
- Knowledge-informed model selection in `CascadeRouter` (separate PRD)
- Cold substrate archival scheduling
- Chain runtime integration
- Any UI changes to the TUI beyond what StateHub already drives

---

## Implementation checklist

### Gap A: StateHub → EventBus bridge (orchestrator events reach SSE/WS clients)

The problem in one sentence: `roko plan run` publishes to `StateHub` (a watch channel),
but `/api/events` SSE and `/ws` WebSocket clients subscribe to `EventBus<ServerEvent>`. The
two buses have no bridge in that direction.

**What already exists and must NOT be re-created:**

- `start_state_hub_bridge` in `crates/roko-serve/src/lib.rs` at line 318 — this bridge
  runs EventBus → StateHub (one direction: REST-triggered events reach the dashboard
  snapshot). Keep it exactly as is.
- `StateHub.subscribe_events()` returns a broadcast receiver over `DashboardEvent`.
- `AppState.event_bus` is `EventBus<ServerEvent>` with `.publish(ServerEvent)`.
- `AppState.state_hub` is `SharedStateHub` with `.subscribe_events()`.

**The missing direction:** orchestrator publishes `DashboardEvent` to `StateHub`. That
event never reaches `EventBus<ServerEvent>`, so SSE/WS clients see nothing.

- [ ] **A-1** Add function `start_orchestrator_event_bridge` in
  `crates/roko-serve/src/lib.rs` directly below `start_state_hub_bridge`:

  ```rust
  fn start_orchestrator_event_bridge(state: Arc<AppState>) -> JoinHandle<()> {
      // NOTE: subscribe_events() returns broadcast::Receiver<Envelope<DashboardEvent>>,
      // not broadcast::Receiver<DashboardEvent>. The event is wrapped in an Envelope.
      let mut rx = state.state_hub.subscribe_events();
      let bus = state.event_bus.clone();
      tokio::spawn(async move {
          loop {
              match rx.recv().await {
                  Ok(envelope) => {
                      if let Some(server_event) = dashboard_event_to_server(&envelope.payload) {
                          bus.publish(server_event);
                      }
                  }
                  Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                      tracing::warn!(n, "orchestrator bridge lagged behind state hub");
                  }
                  Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
              }
          }
      })
  }
  ```

- [ ] **A-2** Add the conversion function `dashboard_event_to_server` in the same file,
  directly below `server_event_to_dashboard`. This is the inverse of the existing function:

  ```rust
  fn dashboard_event_to_server(event: &roko_core::DashboardEvent) -> Option<ServerEvent> {
      use roko_core::DashboardEvent;
      match event {
          DashboardEvent::PlanStarted { plan_id } =>
              Some(ServerEvent::PlanStarted { plan_id: plan_id.clone() }),
          DashboardEvent::PlanCompleted { plan_id, success } =>
              Some(ServerEvent::PlanCompleted { plan_id: plan_id.clone(), success: *success }),
          DashboardEvent::TaskStarted { plan_id, task_id, phase } =>
              Some(ServerEvent::Execution {
                  plan_id: plan_id.clone(),
                  event: ExecutionEvent::TaskStarted {
                      task_id: task_id.clone(),
                      phase: phase.clone(),
                  },
              }),
          DashboardEvent::TaskCompleted { plan_id, task_id, outcome } =>
              Some(ServerEvent::Execution {
                  plan_id: plan_id.clone(),
                  event: ExecutionEvent::TaskCompleted {
                      task_id: task_id.clone(),
                      outcome: outcome.clone(),
                  },
              }),
          DashboardEvent::TaskPhaseChanged { plan_id, task_id, old_phase, new_phase } =>
              Some(ServerEvent::Execution {
                  plan_id: plan_id.clone(),
                  event: ExecutionEvent::TaskPhaseChanged {
                      task_id: task_id.clone(),
                      old_phase: old_phase.clone(),
                      new_phase: new_phase.clone(),
                  },
              }),
          DashboardEvent::AgentSpawned { agent_id, role } =>
              Some(ServerEvent::AgentSpawned {
                  agent_id: agent_id.clone(),
                  role: role.clone(),
              }),
          DashboardEvent::AgentOutput { agent_id, content } =>
              Some(ServerEvent::AgentOutput {
                  agent_id: agent_id.clone(),
                  run_id: None,
                  content: content.clone(),
                  done: false,
                  metadata: None,
              }),
          DashboardEvent::GateResult { plan_id, task_id, gate, passed } =>
              Some(ServerEvent::GateResult {
                  plan_id: plan_id.clone(),
                  task_id: task_id.clone(),
                  gate: gate.clone(),
                  passed: *passed,
              }),
          DashboardEvent::PhaseTransition { plan_id, from, to } =>
              Some(ServerEvent::PhaseTransition {
                  plan_id: plan_id.clone(),
                  from: from.clone(),
                  to: to.clone(),
              }),
          DashboardEvent::EfficiencyEvent { plan_id, task_id, metric, value } =>
              Some(ServerEvent::EfficiencyEvent {
                  plan_id: plan_id.clone(),
                  task_id: task_id.clone(),
                  metric: metric.clone(),
                  value: *value,
              }),
          DashboardEvent::Error { message } =>
              Some(ServerEvent::Error { message: message.clone() }),
          // All other variants (Diagnosis, ExperimentWinnersUpdated, JobExecutionStarted,
          // JobProgress, etc.) either have no ServerEvent equivalent or are already
          // covered by the forward bridge. Do not add catch-all — explicit is safer.
          _ => None,
      }
  }
  ```

- [ ] **A-3** Wire the bridge into both server startup paths in `crates/roko-serve/src/lib.rs`:

  In `ServerBuilder::run` (around line 207), after the existing `_state_hub_bridge` line:
  ```rust
  let _state_hub_bridge = start_state_hub_bridge(Arc::clone(&state));
  let _orchestrator_bridge = start_orchestrator_event_bridge(Arc::clone(&state)); // ADD
  ```

  In `run_server_with_state` (around line 282), same pattern:
  ```rust
  let _state_hub_bridge = start_state_hub_bridge(Arc::clone(&state));
  let _orchestrator_bridge = start_orchestrator_event_bridge(Arc::clone(&state)); // ADD
  ```

- [ ] **A-4** Guard against double-delivery. The existing `start_state_hub_bridge`
  converts `ServerEvent → DashboardEvent`. The new bridge converts
  `DashboardEvent → ServerEvent`. Events originating from REST routes will now loop:
  REST → EventBus → StateHub (via existing bridge) → EventBus (via new bridge). Add a
  deduplication guard by checking the event origin. The simplest approach: tag events
  that cross from `StateHub → EventBus` with a source label and skip them in the forward
  bridge. Alternatively, accept the minor duplication (REST-originated events are low
  frequency) and document it. **The recommended approach:** accept duplication for now and
  add a `// FIXME: bridge loop` comment with a link to this PRD item. Only revisit if
  event volume causes observable problems in integration tests.

  Anti-pattern to avoid: do not try to solve deduplication with a shared atomic counter or
  a `HashSet<u64>` of sequence numbers. The ring buffer sequence is not stable across the
  two buses. Keep it simple.

**Anti-patterns for Gap A:**
- Do not merge `StateHub` and `EventBus<ServerEvent>` into one type. The two buses serve
  different consumers with different semantics (watch channel for TUI vs broadcast for
  SSE). Merging them breaks the TUI's zero-copy borrow.
- Do not call `state.state_hub.publish()` from inside roko-serve routes. Routes should
  publish to `event_bus` only. The bridge propagates automatically.
- Do not spawn the bridge inside `AppState::new`. The hub must exist before the bridge
  starts, and `AppState::new` is synchronous. The bridge needs a tokio runtime.

---

### Gap B: `roko chat` sidecar routing with session continuity

The problem: `run_chat_repl` in `crates/roko-cli/src/chat.rs` unconditionally POSTs to
`{serve_url}/api/agents/{agent_id}/message`. That route calls `spawn_background_run()`,
which forks a fresh `runtime.run_once()` per message. No session state survives between
turns.

The sidecar (`roko agent serve`) already has a `/message` endpoint in
`crates/roko-agent-server/src/features/messaging.rs`. **Correction:** the
sidecar does NOT maintain conversation history across calls — each call creates
`SessionState::default()`. The `BackendMessageDispatcher` dispatches a single
prompt and returns; there is no accumulated chat history. Session continuity in
the sense of multi-turn conversation requires additional state management that
does not yet exist. The benefit of routing through the sidecar is that the
sidecar runs in-process with the agent's LLM backend, avoiding the
`spawn_background_run` overhead of the roko-serve fallback path.

The sidecar registers its bind address in
`AppState.discovered_agents` via `AgentEndpoints.rest`.

**What already exists:**
- `AppState.discovered_agents` is a `RwLock<HashMap<String, DiscoveredAgent>>`.
- `DiscoveredAgent.endpoints.rest` is `Option<String>` — the sidecar's bind URL.
- `AppState` has `discovered_agent(agent_id)` returning `Option<DiscoveredAgent>`.
- `AppState` exposes this via `GET /api/agents/{id}` already.
- `roko-serve` already proxies requests to sidecars in other routes. The `http_client`
  field on `AppState` is a `reqwest::Client` ready to use.

**What already exists (additional):**
- `roko-serve` already proxies to the sidecar. The `POST /api/agents/{id}/message`
  route already checks `discovered_agent` and forwards to the sidecar's
  `rest_endpoint` if the agent is registered. This means the existing `chat.rs`
  path through `roko-serve` may already reach the sidecar indirectly. The
  direct-to-sidecar routing in this gap is an optimization to bypass the
  roko-serve intermediary, not a new capability.

**What does NOT exist:**
- `run_chat_repl` never queries the discovery registry before choosing a target URL.
- There is no fallback logic in `chat.rs`.

- [ ] **B-1** Add a helper function `resolve_sidecar_url` to `crates/roko-cli/src/chat.rs`
  that queries `GET {serve_url}/api/agents/{agent_id}` and extracts `endpoints.rest`:

  ```rust
  async fn resolve_sidecar_url(
      client: &reqwest::Client,
      serve_url: &str,
      agent_id: &str,
  ) -> Option<String> {
      let url = format!(
          "{}/api/agents/{agent_id}",
          serve_url.trim_end_matches('/')
      );
      let response = client.get(&url).send().await.ok()?;
      if !response.status().is_success() {
          return None;
      }
      let body: serde_json::Value = response.json().await.ok()?;
      // NOTE: The registration payload uses a flat `rest_endpoint` field,
      // not a nested `endpoints.rest` object. Check both for robustness.
      body.get("rest_endpoint")
          .and_then(|v| v.as_str())
          .or_else(|| body.pointer("/endpoints/rest").and_then(|v| v.as_str()))
          .map(str::to_owned)
  }
  ```

- [ ] **B-2** Modify `run_chat_repl` to call `resolve_sidecar_url` before entering the
  REPL loop and store the result:

  ```rust
  pub async fn run_chat_repl(agent_id: &str, serve_url: &str) -> Result<()> {
      // ... existing println setup ...

      let client = reqwest::Client::new();
      let sidecar_base = resolve_sidecar_url(&client, serve_url, agent_id).await;

      if sidecar_base.is_some() {
          println!("[connected to sidecar — session continuity enabled]");
      } else {
          println!("[no sidecar registered — using roko-serve fallback]");
      }

      // ... rest of loop ...
  }
  ```

- [ ] **B-3** Modify the per-message POST in `run_chat_repl` to route to the sidecar when
  available. The sidecar `/message` endpoint expects `{ "prompt": "..." }` (field name
  `prompt`), while the roko-serve endpoint expects `{ "message": "..." }`. Use the correct
  field name per target.

  Replace the single `client.post(...)` block with:

  ```rust
  let (target_url, body) = if let Some(ref base) = sidecar_base {
      (
          format!("{}/message", base.trim_end_matches('/')),
          json!({ "prompt": message }),
      )
  } else {
      (
          format!(
              "{}/api/agents/{agent_id}/message",
              serve_url.trim_end_matches('/')
          ),
          json!({ "message": message }),
      )
  };

  let response = client
      .post(&target_url)
      .json(&body)
      .send()
      .await
      .context("send chat message")?;
  ```

- [ ] **B-4** The sidecar response shape already matches `SendMessageResponse` when it
  returns `{ "response": "...", "reasoning": "..." }`. No change needed to the
  deserialization path. Verify this by reading `messaging.rs` line 51-59 — the response
  body includes `response` and `reasoning` keys. The existing `SendMessageResponse` struct
  and the rendering code in `run_chat_repl` handle this correctly already.

- [ ] **B-5** The fallback path (`run_id` present) must still work when the serve route
  returns a background run. This path is only taken when `sidecar_base.is_none()`. The
  existing `wait_for_run_completion` function handles it. No change needed.

- [ ] **B-6** Add `resolve_sidecar_url` to the existing test module at the bottom of
  `chat.rs`. The test should verify that a missing `endpoints.rest` field returns `None`:

  ```rust
  #[tokio::test]
  async fn resolve_sidecar_url_returns_none_when_rest_absent() {
      // spawn a minimal mock server that returns an agent record without
      // endpoints.rest set
      use axum::{Json, Router, routing::get};
      use tokio::net::TcpListener;

      let app = Router::new().route(
          "/api/agents/agent-1",
          get(|| async { Json(serde_json::json!({ "agent_id": "agent-1", "endpoints": {} })) }),
      );
      let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
      let addr = listener.local_addr().unwrap();
      tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

      let client = reqwest::Client::new();
      let result = resolve_sidecar_url(
          &client,
          &format!("http://{addr}"),
          "agent-1",
      ).await;
      assert!(result.is_none());
  }
  ```

**Anti-patterns for Gap B:**
- Do not hardcode a sidecar port. Always read from the discovery registry.
- Do not remove the fallback to `roko-serve`. The fallback is load-bearing for agents that
  have no running sidecar (e.g., agents dispatched transiently by the orchestrator).
- Do not change the sidecar's `/message` endpoint signature. The existing `{ "prompt": }`
  field name is correct; only `roko-serve`'s route uses `{ "message": }`.
- Do not add authentication headers in this change. Token auth for the sidecar is handled
  by `AgentEndpoints` rotation (already in place) and is out of scope here.

---

### Gap C: Sidecar `/research` LLM dispatch

The problem: `AgentState::research` in `crates/roko-agent-server/src/state.rs` at line
765 returns hardcoded strings. No LLM call is made.

**Audit update (2026-04-22):** the sidecar research/tasks feature routes are mounted and `AgentState::research` can answer from local knowledge, but this gap is still open because the LLM dispatch path and mock-dispatcher tests below are not complete.

- [ ] Finish sidecar `/research` LLM dispatch and tests; local-knowledge fallback alone is not sufficient for this PRD.

**What already exists:**
- `AgentState::dispatch_prompt` at line 566 is the correct dispatch seam — it calls
  `self.message_dispatcher().ok_or(DispatchError::NotConfigured)?` and then
  `dispatcher.dispatch(chat_request(prompt, false)).await`.
- `chat_request` at line 164 builds a `ChatRequest` from a prompt string.
- The research route at `crates/roko-agent-server/src/features/research.rs` calls
  `state.research(request).await` and returns the result as JSON. The route handler itself
  needs no changes.

**What needs to change:** `AgentState::research` should call `dispatch_prompt` with a
structured research prompt, then parse the LLM response into `findings` and `sources`.

- [ ] **C-1** Replace the body of `AgentState::research` in
  `crates/roko-agent-server/src/state.rs`. The function signature stays identical:
  `pub async fn research(&self, request: ResearchRequest) -> ResearchResponse`.

  ```rust
  pub async fn research(&self, request: ResearchRequest) -> ResearchResponse {
      self.metrics.record_request();

      let prompt = format!(
          "You are a research assistant. Investigate the following topic and return your \
           findings as a JSON object with two keys: \
           \"findings\" (array of strings, one insight per entry) and \
           \"sources\" (array of strings, one URL or reference per entry).\n\n\
           Topic: {}\nDepth: {}\n\nReturn ONLY the JSON object, no prose.",
          request.topic, request.depth
      );

      let response = match self.dispatch_prompt(&prompt).await {
          Ok(response) => response,
          Err(_) => return self.stub_research_response(&request),
      };

      // Attempt to extract JSON from the LLM response text.
      let text = response.content.trim();
      // Strip markdown fences if present.
      let json_text = text
          .trim_start_matches("```json")
          .trim_start_matches("```")
          .trim_end_matches("```")
          .trim();

      match serde_json::from_str::<serde_json::Value>(json_text) {
          Ok(value) => {
              let findings = value["findings"]
                  .as_array()
                  .map(|arr| {
                      arr.iter()
                          .filter_map(|v| v.as_str().map(str::to_owned))
                          .collect()
                  })
                  .unwrap_or_default();
              let sources = value["sources"]
                  .as_array()
                  .map(|arr| {
                      arr.iter()
                          .filter_map(|v| v.as_str().map(str::to_owned))
                          .collect()
                  })
                  .unwrap_or_default();
              ResearchResponse { findings, sources }
          }
          Err(_) => {
              // LLM returned prose rather than JSON. Wrap it as a single finding.
              ResearchResponse {
                  findings: vec![text.to_owned()],
                  sources: vec![format!("agent://{}/research", self.agent_id)],
              }
          }
      }
  }

  /// Fallback used when no dispatcher is configured.
  fn stub_research_response(&self, request: &ResearchRequest) -> ResearchResponse {
      ResearchResponse {
          findings: vec![
              format!("{} reviewed topic '{}'", self.agent_id, request.topic),
              format!("requested depth: {}", request.depth),
          ],
          sources: vec![
              format!("agent://{}/capabilities", self.agent_id),
              self.chain_client.as_ref().map_or_else(
                  || "chain://unconfigured".to_string(),
                  |client| format!("chain://{}", client.name()),
              ),
          ],
      }
  }
  ```

  The stub is the exact code that previously lived in `research`. Moving it to a separate
  method means no behavior regression when a dispatcher is not configured (e.g., in
  integration tests that call `AgentState::new` with `llm_backend: None`).

- [ ] **C-2** Remove the `#[allow(clippy::unused_async)]` attribute from `research` — the
  function is now genuinely async (it awaits `dispatch_prompt`).

- [ ] **C-3** Add a unit test in `crates/roko-agent-server/src/state.rs` that verifies the
  stub path is taken when no dispatcher is configured:

  ```rust
  #[tokio::test]
  async fn research_falls_back_to_stub_when_no_dispatcher() {
      let state = AgentState::new(
          "agent-test".to_string(),
          None,
          "0.1.0".to_string(),
          vec!["research".to_string()],
          None,
          None, // no llm_backend
          None,
      );
      let response = state.research(ResearchRequest {
          topic: "Rust ownership".to_string(),
          depth: "shallow".to_string(),
      }).await;
      assert!(!response.findings.is_empty());
      assert!(!response.sources.is_empty());
      // Stub path: findings contain the agent_id and topic
      assert!(response.findings[0].contains("agent-test"));
      assert!(response.findings[0].contains("Rust ownership"));
  }
  ```

- [ ] **C-4** Add a unit test that verifies the LLM dispatch path when a mock dispatcher
  returns well-formed JSON. Use the same `MockDispatcher` pattern already established in
  `crates/roko-agent-server/src/features/messaging.rs` tests. Add it to a `#[cfg(test)]`
  block in `state.rs`:

  ```rust
  #[tokio::test]
  async fn research_parses_llm_json_response() {
      use crate::state::DispatchLike;
      use roko_agent::chat_types::{ChatRequest, ChatResponse, FinishReason};
      use async_trait::async_trait;
      use tokio::sync::mpsc;
      use roko_agent::streaming::StreamChunk;

      #[derive(Clone)]
      struct JsonDispatcher;

      #[async_trait]
      impl DispatchLike for JsonDispatcher {
          async fn dispatch(&self, _: ChatRequest) -> Result<ChatResponse, DispatchError> {
              Ok(ChatResponse {
                  content: r#"{"findings":["ownership prevents dangling pointers"],"sources":["https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html"]}"#.to_string(),
                  finish_reason: FinishReason::Stop,
                  ..Default::default()
              })
          }
      }

      let state = AgentState::new(
          "agent-test".to_string(),
          None,
          "0.1.0".to_string(),
          vec!["research".to_string()],
          None,
          None,
          None,
      ).with_message_dispatcher(Arc::new(JsonDispatcher));

      let response = state.research(ResearchRequest {
          topic: "Rust ownership".to_string(),
          depth: "deep".to_string(),
      }).await;

      assert_eq!(response.findings, vec!["ownership prevents dangling pointers"]);
      assert_eq!(response.sources, vec!["https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html"]);
  }
  ```

**Anti-patterns for Gap C:**
- Do not parse the LLM response with a regex. Use `serde_json::from_str` with a fallback.
- Do not make the JSON schema strict. Findings or sources may be empty arrays; both are
  valid.
- Do not remove the stub. Integration tests that run without a real LLM backend rely on
  the stub path.
- Do not add a new HTTP route. The existing `POST /research` route in
  `crates/roko-agent-server/src/features/research.rs` is correct and unchanged.

---

### Gap D: roko-mcp-code auto-discovery in `setup_mcp`

The problem: `PlanRunner::setup_mcp` in `crates/roko-cli/src/orchestrate.rs` at line 4278
reads MCP servers from either an explicit config path (`config.agent.mcp_config`) or by
walking the directory tree for `.mcp.json`. It never adds `roko-mcp-code` automatically.
Users must manually edit `.mcp.json` to get code-intelligence tools.

**What already exists:**
- `McpConfig` in `roko-agent/src/mcp/` has a `servers: Vec<McpServerConfig>` field.
- `McpServerConfig` has `name`, `command`, and `args` fields.
- `setup_mcp` already constructs `McpConfig` from the discovered or explicit file and
  iterates `mcp_config.servers`. Inserting an entry before iteration is sufficient.
- The `roko-mcp-code` binary name is `roko-mcp-code` (same as the crate name). It accepts
  no positional arguments; it reads from stdin and writes to stdout (stdio transport).

- [ ] **D-1** Add a helper function `find_roko_mcp_code_binary` directly above `setup_mcp`
  in `crates/roko-cli/src/orchestrate.rs`:

  ```rust
  /// Locate the `roko-mcp-code` binary.
  ///
  /// Search order:
  /// 1. `ROKO_MCP_CODE` environment variable (explicit override).
  /// 2. A `roko-mcp-code` binary adjacent to the current executable
  ///    (covers `cargo install` and release builds).
  /// 3. `roko-mcp-code` on `PATH`.
  ///
  /// Returns `None` if the binary cannot be found. Callers must not fail
  /// hard on `None` — auto-discovery is best-effort.
  fn find_roko_mcp_code_binary() -> Option<String> {
      // Explicit override wins.
      if let Ok(env_path) = std::env::var("ROKO_MCP_CODE") {
          if !env_path.is_empty() {
              return Some(env_path);
          }
      }

      // Adjacent to current executable (covers release builds + cargo install).
      if let Ok(exe) = std::env::current_exe() {
          if let Some(dir) = exe.parent() {
              let candidate = dir.join("roko-mcp-code");
              if candidate.exists() {
                  return Some(candidate.to_string_lossy().into_owned());
              }
          }
      }

      // PATH lookup — manual walk since `which` crate is not in the workspace.
      std::env::var_os("PATH")
          .and_then(|paths| {
              std::env::split_paths(&paths)
                  .map(|dir| dir.join("roko-mcp-code"))
                  .find(|candidate| candidate.exists())
                  .map(|p| p.to_string_lossy().into_owned())
          })
  }
  ```

  **NOTE:** The `which` crate is NOT in the workspace dependency tree (verified:
  `grep -r '"which"' Cargo.toml` returns zero results). The code above uses a
  manual PATH walk instead. If you prefer the `which` crate, add it to
  `crates/roko-cli/Cargo.toml` first:
  ```toml
  which = "6"
  ```
  Then replace the manual PATH walk with `which::which("roko-mcp-code").ok().map(...)`.
  The manual walk is preferred to avoid adding a new dependency for a single call site.

- [ ] **D-2** In `setup_mcp`, after the `mcp_config` variable is resolved but before the
  `mcp_config.servers.is_empty()` early return, inject the `roko-mcp-code` entry when:
  - the binary is discoverable, AND
  - no server with `name == "roko-mcp-code"` already exists in the config (idempotent).

  The injection point is the block after line 4310:
  ```rust
  let mcp_config = match mcp_config {
      Some(cfg) if !cfg.servers.is_empty() => cfg,
      _ => return (HashMap::new(), None, Vec::new(), HashMap::new()),
  };
  ```

  Replace with:
  ```rust
  let mut mcp_config = match mcp_config {
      Some(cfg) => cfg,
      None => McpConfig { servers: Vec::new() },
  };

  // Auto-inject roko-mcp-code if discoverable and not already configured.
  if let Some(binary) = find_roko_mcp_code_binary() {
      let already_configured = mcp_config
          .servers
          .iter()
          .any(|s| s.name == "roko-mcp-code");
      if !already_configured {
          // NOTE: McpServerConfig has 5 additional fields beyond name/command/args:
          // transport, env, endpoint, auth_token, tier. Use ..Default::default()
          // to initialize them correctly.
          mcp_config.servers.push(McpServerConfig {
              name: "roko-mcp-code".to_string(),
              command: binary,
              args: Vec::new(),
              ..McpServerConfig::default()
          });
          tracing::debug!("auto-injected roko-mcp-code into MCP server list");
      }
  }

  if mcp_config.servers.is_empty() {
      return (HashMap::new(), None, Vec::new(), HashMap::new());
  }
  ```

  Note the early return condition moves to after injection: if the user had no `.mcp.json`
  and `roko-mcp-code` is not on PATH, `servers` is still empty and we return early. If
  `roko-mcp-code` was injected, `servers` has one entry and we proceed.

- [ ] **D-3** Verify that `McpServerConfig` has `name`, `command`, and `args` fields by
  reading `crates/roko-agent/src/mcp/mod.rs` (or wherever the struct is defined) before
  writing the injection code. If the struct fields differ, adapt accordingly.

- [ ] **D-4** Add an integration test in `crates/roko-cli/src/orchestrate.rs` test module
  (or a dedicated `tests/mcp_discovery.rs` if one exists) that verifies the injection is
  skipped when `roko-mcp-code` is already in the config:

  ```rust
  #[test]
  fn roko_mcp_code_not_injected_twice() {
      // Build a config that already has roko-mcp-code.
      let existing = McpServerConfig {
          name: "roko-mcp-code".to_string(),
          command: "/usr/local/bin/roko-mcp-code".to_string(),
          args: Vec::new(),
          ..McpServerConfig::default()
      };
      let mut config = McpConfig { servers: vec![existing] };

      // Simulate the injection logic.
      let already_configured = config.servers.iter().any(|s| s.name == "roko-mcp-code");
      if !already_configured {
          config.servers.push(McpServerConfig {
              name: "roko-mcp-code".to_string(),
              command: "roko-mcp-code".to_string(),
              args: Vec::new(),
              ..McpServerConfig::default()
          });
      }

      // Must still have exactly one entry.
      assert_eq!(
          config.servers.iter().filter(|s| s.name == "roko-mcp-code").count(),
          1
      );
  }
  ```

- [ ] **D-5** Add a `ROKO_MCP_CODE` entry to the relevant environment variable docs (if
  any) or the `roko.toml` config schema comments. This is a one-line change. If no central
  env var docs exist, add a `// ROKO_MCP_CODE: override path to roko-mcp-code binary`
  comment at the call site of `find_roko_mcp_code_binary`.

**Anti-patterns for Gap D:**
- Do not compile-time embed the path to the binary. The path varies between development
  (cargo target dir), CI, and production installs.
- Do not hard-fail if the binary is not found. Auto-discovery is best-effort. Log at
  `debug` level and continue without it.
- Do not add `roko-mcp-code` to the MCP config when `selected_servers` is specified and
  `roko-mcp-code` is not in the selected set. Respect the caller's explicit selection.
  Add this guard to the injection site:
  ```rust
  let skip_injection = selected_servers
      .map(|names| !names.contains("roko-mcp-code"))
      .unwrap_or(false);
  if !skip_injection {
      // ... injection logic ...
  }
  ```
- Do not modify the `McpConfig` that was loaded from disk. The injection operates on the
  in-memory copy inside `setup_mcp`. The on-disk `.mcp.json` file must remain unchanged.

---

### Gap E: Subscriptions only trigger from WebhookReceived

The problem: The subscription dispatch loop in
`crates/roko-serve/src/dispatch.rs:1424` filters:

```rust
let ServerEvent::WebhookReceived { signal } = envelope.payload else { continue; };
```

This means in-process events (gate failures, plan completions, episode
recordings) do NOT trigger subscriptions. Only external webhook and cron signals
do. Users who create subscriptions expecting to react to gate failures will see
nothing happen.

**What already exists:**
- `ServerEvent` has variants for `GateResult`, `PlanCompleted`, `Episode`,
  `TaskStarted`, `TaskCompleted`, `PhaseTransition`, etc.
- `Subscription` matching logic already supports filter expressions that can
  match on event type and payload fields.
- The subscription store persists to disk and survives restarts.

**What needs to change:** Expand the dispatch loop to also match additional
`ServerEvent` variants beyond `WebhookReceived`. Convert them to synthetic
`Engram` signals that can be matched against subscription filters.

- [ ] **E-1** In `crates/roko-serve/src/dispatch.rs`, at the subscription
  dispatch loop (around line 1424), replace the single `let ... else { continue }`
  with a match that handles multiple event types:

  ```rust
  let signal = match &envelope.payload {
      ServerEvent::WebhookReceived { signal } => signal.clone(),
      ServerEvent::GateResult { plan_id, task_id, gate, passed } => {
          // Synthesize an Engram from the gate result.
          Engram::builder()
              .kind("gate_result")
              .tag("plan_id", plan_id)
              .tag("task_id", task_id)
              .tag("gate", gate)
              .tag("passed", &passed.to_string())
              .build()
      }
      ServerEvent::PlanCompleted { plan_id, success } => {
          Engram::builder()
              .kind("plan_completed")
              .tag("plan_id", plan_id)
              .tag("success", &success.to_string())
              .build()
      }
      ServerEvent::Episode { episode_id, .. } => {
          Engram::builder()
              .kind("episode")
              .tag("episode_id", episode_id)
              .build()
      }
      _ => continue,
  };
  ```

  NOTE: The exact `Engram` builder API may differ. Check
  `crates/roko-core/src/signal.rs` for the actual construction method. The key
  requirement is that each event type produces a signal with a `kind` tag that
  subscription filters can match against.

- [ ] **E-2** Add subscription filter examples to the documentation (or inline
  comments) showing how to subscribe to gate failures:

  ```json
  {
    "filter": { "kind": "gate_result", "tags": { "passed": "false" } },
    "action": { "webhook": "https://example.com/gate-failure" }
  }
  ```

- [ ] **E-3** Add an integration test in `crates/roko-serve/tests/api_integration.rs`:
  - Register a subscription with filter `kind == "gate_result"`.
  - Publish a `ServerEvent::GateResult` to the event bus.
  - Assert the subscription handler is triggered (use a mock webhook endpoint
    or check an internal counter).

- [ ] **E-4** Guard against infinite loops: if a subscription action publishes
  a new event that itself triggers subscriptions, the system could loop. Add a
  `depth` or `source` tag to synthetic signals so the dispatch loop can skip
  events that were themselves generated by subscription actions:

  ```rust
  if signal.tag("_source") == Some("subscription") {
      continue; // do not re-trigger subscriptions from subscription-generated events
  }
  ```

**Anti-patterns for Gap E:**
- Do not convert ALL `ServerEvent` variants to signals. Only convert events
  that users would plausibly want to subscribe to (gate results, plan
  completions, episodes). Leave internal bookkeeping events (e.g.,
  `EfficiencyEvent`) out of the subscription path.
- Do not change the existing `WebhookReceived` handling. It must continue to
  work exactly as before. The new match arms are additive.
- Do not remove the `_ => continue` fallback. New `ServerEvent` variants added
  in the future should be explicitly opted in, not silently subscribed.

---

### Gap F: Provider health not wired in serve dispatch

The problem: Provider health tracking works in the CLI orchestrator path
(`orchestrate.rs:13453` checks `is_healthy()`,
`crates/roko-learn/src/runtime_feedback.rs:832` records success/failure). But
the roko-serve HTTP dispatch loop (`crates/roko-serve/src/dispatch.rs`) never
calls `record_success()` or `record_failure()` after agent runs. Further,
`route_with_health()` exists but is unused in the serve path.
`dispatch.rs:2326` uses `CascadeRouter::load_or_new().route()` (without
health) instead of `route_with_health()`.

**What already exists:**
- `ProviderHealth` type with `is_healthy()`, `record_success()`, and
  `record_failure()` methods.
- `CascadeRouter::route_with_health()` method that consults health before
  selecting a model.
- The CLI orchestrator path already uses both correctly.

**What needs to change:**

- [ ] **F-1** Add `provider_health: Arc<ProviderHealth>` to `AppState` in
  `crates/roko-serve/src/state.rs` if not already present. Initialize it in
  `AppState::new` from config or with defaults.

- [ ] **F-2** In `crates/roko-serve/src/dispatch.rs` at line 2326 (the routing
  call), replace `router.route(...)` with `router.route_with_health(...)`:

  ```rust
  let model = router.route_with_health(&role, &category, &state.provider_health);
  ```

- [ ] **F-3** After each agent run in the serve dispatch loop, record the
  outcome:

  ```rust
  match agent_result {
      Ok(_) => state.provider_health.record_success(&provider_name),
      Err(ref e) => state.provider_health.record_failure(
          &provider_name,
          &format!("{e:#}"),
      ),
  }
  ```

- [ ] **F-4** Add an integration test in `crates/roko-serve/tests/api_integration.rs`
  that verifies health is tracked after dispatch:
  - Dispatch a mock agent run through the serve path.
  - Query the health state and assert it reflects the outcome.

**Anti-patterns for Gap F:**
- Do not create a new `ProviderHealth` per request. It must be shared state.
- Do not skip health recording on timeout. Timeouts are failures and should
  degrade the provider's health score.
- Do not remove the health tracking from the CLI path. Both paths must track
  health independently.

---

## Concrete file touchpoints

| File | Gap | What changes |
|------|-----|-------------|
| `crates/roko-serve/src/lib.rs` | A | Add `start_orchestrator_event_bridge` fn (new); add `dashboard_event_to_server` fn (new); wire both into `ServerBuilder::run` and `run_server_with_state` |
| `crates/roko-cli/src/chat.rs` | B | Add `resolve_sidecar_url` fn (new); modify `run_chat_repl` to call it; modify per-message POST to branch on sidecar presence; update tests |
| `crates/roko-agent-server/src/state.rs` | C | Replace body of `AgentState::research`; extract `stub_research_response`; remove `#[allow(clippy::unused_async)]` from `research`; add two tests |
| `crates/roko-cli/src/orchestrate.rs` | D | Add `find_roko_mcp_code_binary` fn (new); modify `setup_mcp` to inject binary and adjust early-return logic |
| `crates/roko-serve/src/dispatch.rs` | E | Expand subscription dispatch loop to match `GateResult`, `PlanCompleted`, `Episode`; add depth guard |
| `crates/roko-serve/src/dispatch.rs` | F | Replace `route()` with `route_with_health()`; add `record_success`/`record_failure` after agent runs |
| `crates/roko-serve/src/state.rs` | F | Add `provider_health` to `AppState` if missing |
| `crates/roko-serve/tests/api_integration.rs` | E, F | Add integration tests for subscription triggering and health recording |

No files outside this list should need edits. No new crates. No `Cargo.toml` changes
unless `which` is missing (see D-1 note).

---

## Verification checklist

Run these commands from the workspace root `/Users/will/dev/nunchi/roko/roko/`.

### Build and lint

- [ ] `cargo build --workspace` passes with zero errors
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes with zero warnings
- [ ] `cargo +nightly fmt --all -- --check` passes (run `cargo +nightly fmt --all` to fix)

### Unit tests

- [ ] `cargo test -p roko-serve` — Gap A bridge tests pass
- [ ] `cargo test -p roko-cli --lib` — Gap B `resolve_sidecar_url` test passes
- [ ] `cargo test -p roko-agent-server` — Gap C stub and JSON parse tests pass
- [ ] `cargo test -p roko-cli --lib` — Gap D `roko_mcp_code_not_injected_twice` passes
- [ ] `cargo test -p roko-serve` — Gap E subscription trigger and Gap F health recording tests pass

### Integration: Gap A

- [ ] Start `roko serve` in one terminal
- [ ] Connect a WebSocket client to `ws://localhost:6677/ws` or open a curl SSE stream:
  `curl -N http://localhost:6677/api/events`
- [ ] In a second terminal, run `roko plan run plans/` against any plan
- [ ] Observe `gate_result`, `task_started`, and `task_completed` events appearing in the
  SSE/WS stream within seconds of the orchestrator firing them
- [ ] Confirm the existing `roko dashboard` TUI still updates correctly (regression check)

### Integration: Gap B

- [ ] Start a sidecar: `roko agent serve --agent-id test-agent --bind 127.0.0.1:7001`
- [ ] Register it: `curl -X POST http://localhost:6677/api/agents/register -H 'content-type: application/json' -d '{"agent_id":"test-agent","rest_endpoint":"http://127.0.0.1:7001"}'`
  (Note: use flat `rest_endpoint` field, not nested `endpoints.rest` object)
- [ ] Run `roko chat --agent test-agent`
- [ ] Verify the REPL prints `[connected to sidecar — session continuity enabled]`
- [ ] Send two messages and confirm the second message references context from the first
  (session state is preserved in the `BackendMessageDispatcher`)
- [ ] Kill the sidecar, run `roko chat --agent test-agent` again — verify fallback prints
  `[no sidecar registered — using roko-serve fallback]`

### Integration: Gap C

- [ ] Start a sidecar with an LLM backend configured
- [ ] `curl -X POST http://localhost:{sidecar_port}/research -H 'content-type: application/json' -d '{"topic":"Rust borrow checker","depth":"shallow"}'`
- [ ] Confirm response contains non-stub `findings` (actual LLM output, not the
  "agent-X reviewed topic" string)
- [ ] Repeat with a sidecar that has no LLM backend configured — confirm stub response is
  returned with HTTP 200 (not a 5xx)

### Integration: Gap D

- [ ] Build the workspace: `cargo build --workspace`
- [ ] Copy `target/debug/roko-mcp-code` to a directory on PATH or set
  `ROKO_MCP_CODE=target/debug/roko-mcp-code`
- [ ] Run `RUST_LOG=debug roko plan run plans/ 2>&1 | grep 'auto-injected'`
- [ ] Confirm `auto-injected roko-mcp-code into MCP server list` appears in debug output
- [ ] Run with an explicit `.mcp.json` that already contains `roko-mcp-code` — confirm the
  debug log does NOT show `auto-injected` (idempotency)
- [ ] Run with `selected_servers` not including `roko-mcp-code` (e.g., via the `--mcp`
  flag if it exists) — confirm injection is skipped

### Integration: Gap E

- [ ] Start `roko serve` in one terminal
- [ ] Register a subscription via the API:
  `curl -X POST http://localhost:6677/api/subscriptions -H 'content-type: application/json' -d '{"filter":{"kind":"gate_result","tags":{"passed":"false"}},"action":{"webhook":"http://localhost:9999/hook"}}'`
- [ ] Start a mock webhook receiver on port 9999 (e.g., `nc -l 9999` or a simple HTTP server)
- [ ] In a second terminal, run a plan that triggers a gate failure
- [ ] Observe the webhook receiver gets a POST with the gate failure signal
- [ ] Confirm `WebhookReceived` subscriptions still fire correctly (regression check)

### Integration: Gap F

- [ ] Start `roko serve`
- [ ] Dispatch an agent run via the HTTP API:
  `curl -X POST http://localhost:6677/api/agents/test/run -H 'content-type: application/json' -d '{"prompt":"Hello"}'`
- [ ] Check provider health:
  `curl http://localhost:6677/api/health/providers`
- [ ] Confirm the provider used for the run has a health entry with success count > 0
- [ ] Simulate a failure (e.g., configure an invalid API key) and dispatch again
- [ ] Confirm the health entry shows a failure count > 0
- [ ] Confirm `route_with_health()` prefers healthy providers over unhealthy ones in
  subsequent dispatches (configure two providers, mark one unhealthy, verify routing)

---

## Acceptance criteria

All six gaps are accepted when all of the following are true.

**Gap A:**
1. An integration test in `crates/roko-serve/tests/api_integration.rs` publishes a
   `DashboardEvent::GateResult` to `state.state_hub` and asserts that a `ServerEvent`
   with `type == "gate_result"` appears on the WebSocket within 2 seconds.
2. The existing `api_integration.rs` tests still pass (no regressions to SSE or WS routes).
3. `cargo test -p roko-serve` green.

**Gap B:**
1. `cargo test -p roko-cli --lib` includes at least one test for `resolve_sidecar_url`
   covering the no-`endpoints.rest` case and the happy path.
2. Manual test (see verification checklist B) confirmed with a real sidecar.
3. No existing tests broken.

**Gap C:**
1. `cargo test -p roko-agent-server` includes:
   - One test asserting stub response when no dispatcher configured.
   - One test asserting JSON parsing from a mock dispatcher.
2. `POST /research` on a sidecar with a real LLM backend returns LLM-generated content,
   not the hardcoded stub strings.
3. `POST /research` on a sidecar without a backend returns HTTP 200 with stub content
   (not a 500 or 503).

**Gap D:**
1. `cargo test -p roko-cli --lib` includes `roko_mcp_code_not_injected_twice` and passes.
2. `RUST_LOG=debug roko plan run` with `roko-mcp-code` on PATH logs the `auto-injected`
   line exactly once.
3. `roko plan run` with `roko-mcp-code` already in `.mcp.json` does NOT log `auto-injected`.
4. `cargo build --workspace` introduces no new `Cargo.toml` dependencies unless `which` is
   genuinely absent from the workspace.

**Gap E:**
1. `ServerEvent::GateResult` published to the event bus triggers a matching subscription.
2. `ServerEvent::PlanCompleted` published to the event bus triggers a matching subscription.
3. Existing `WebhookReceived` subscriptions continue to work (regression).
4. Subscription-generated events do not re-trigger subscriptions (loop guard verified).
5. An integration test in `api_integration.rs` covers at least one non-webhook event type.

**Gap F:**
1. `route_with_health()` is called in the serve dispatch path (verified via debug logs or
   test assertion).
2. `record_success()` is called after successful agent runs in serve dispatch.
3. `record_failure()` is called after failed agent runs in serve dispatch.
4. `AppState` contains a shared `ProviderHealth` instance.
5. An integration test verifies health data accumulates across requests.

---

## Errata applied

Corrections applied 2026-04-22 based on audit discrepancy report:

1. **BLOCKER FIX: Gap A wrong receiver type.** `subscribe_events()` returns
   `broadcast::Receiver<Envelope<DashboardEvent>>`, not bare `DashboardEvent`.
   Code updated from `Ok(event)` / `dashboard_event_to_server(&event)` to
   `Ok(envelope)` / `dashboard_event_to_server(&envelope.payload)`.

2. **Gap B: Session continuity claim corrected.** The sidecar does NOT maintain
   conversation history -- each call creates `SessionState::default()`. The
   motivation text was overstated. Corrected to describe the actual benefit
   (avoiding `spawn_background_run` overhead).

3. **Gap B: roko-serve proxy noted.** Added documentation that `POST
   /api/agents/{id}/message` already checks `discovered_agent` and forwards
   to the sidecar. Direct-to-sidecar routing is an optimization, not a new
   capability.

4. **Gap B: Registration curl fixed.** Changed from nested `endpoints.rest`
   to flat `rest_endpoint` field. Updated `resolve_sidecar_url` to check both
   field formats for robustness.

5. **Gap D: `which` crate absence documented.** The PRD claimed "verify with
   grep" -- the crate is absent. Replaced `which::which()` with a manual PATH
   walk. Added note on how to add the `which` crate if preferred.

6. **Gap D: `McpServerConfig` init completed.** All three `McpServerConfig`
   literal constructions now use `..McpServerConfig::default()` to initialize
   the 5 extra fields (`transport`, `env`, `endpoint`, `auth_token`, `tier`).
