# 22 — Dispatch Streaming Completion (T5-36, T5-37 expanded)

The migration from "every surface owns its provider HTTP" to "all dispatch
goes through `ModelCallService`" is partial. ACP and CLI chat API are done;
serve and legacy paths remain.

This plan describes the per-surface migration, the deprecation path for
`dispatch_direct.rs`, and the static check that prevents regression.

Source: doc 35 § Dispatch and provider execution; doc 41 T5-36, T5-37.

---

## Today's State (verified 2026-05-01)

Four LLM dispatch paths exist:

1. **`ModelCallService`** (`crates/roko-agent/src/model_call_service.rs`)
   — the canonical path. ACP uses it (D6); CLI chat API uses it (D8);
   serve provider-test uses it (D9).
2. **`DispatchResolver`** (`crates/roko-agent/src/dispatch_resolver.rs`)
   — picks model+provider; calls into `ModelCallService`. Validation is
   `Unvalidated` (plan 21 ACP-1 fixes that).
3. **`dispatch_direct.rs`** (`crates/roko-cli/src/dispatch_direct.rs`) —
   legacy. Still used in `chat_inline`, `lib`, `unified`,
   `marketplace.rs`, `identity_economy_markets.rs`.
4. **Route-local `reqwest::Client`** in serve routes — non-LLM HTTP is
   fine; the issue is LLM dispatch routes that build their own client.

After this plan, only paths 1 and 2 remain in production code; path 3 is
feature-gated; path 4 is gone.

---

## Anti-Patterns

1. **No fifth dispatch path.** If a route needs new behavior, extend
   `ModelCallService` or add a provider adapter.
2. **No silent fallback** from `ModelCallService` to `dispatch_direct`.
3. **No raw `reqwest::Client::new()` in `roko-serve` routes that
   execute models.** Non-LLM HTTP (GitHub, Linear, Slack, Railway,
   chain) keeps `reqwest`.
4. **One route migration per commit.** Easy to bisect.

---

## Phase 1: Migrate Serve LLM Routes (T5-36)

### Inventory

```bash
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/ -l
```

Cross-reference with the route's purpose. **Migrate** routes whose
`reqwest` usage is for LLM dispatch. **Leave alone** routes whose
`reqwest` is for:

- GitHub / Linear / Slack / generic webhook-style external services
- Railway deploy API
- Chain RPC (eth_call, etc.)
- Relay registration
- Health probes to other roko instances

### Migration template (per-route)

For each LLM dispatch route:

1. **Read the existing flow**:
   ```rust
   async fn handler(req: Request) -> Response {
       let client = reqwest::Client::new();
       let body = json!({"model": req.model, "messages": ...});
       let resp = client.post("https://api.openai.com/v1/chat/completions")
           .header("Authorization", format!("Bearer {}", env_key))
           .json(&body).send().await?;
       let parsed: OpenAiResponse = resp.json().await?;
       // ...
   }
   ```

2. **Replace with shared dispatch**:
   ```rust
   async fn handler(
       State(state): State<Arc<AppState>>,
       Json(req): Json<RequestBody>,
   ) -> Response {
       let model_call_req = ModelCallRequest {
           model: req.model.clone(),
           // provider resolved from model registry
           messages: req.messages.iter().map(Message::from).collect(),
           max_tokens: req.max_tokens.unwrap_or(2048),
           temperature: req.temperature,
           tools: vec![],
           streaming: false,
       };
       let resp = state.model_call_service.call(model_call_req).await
           .map_err(|e| ApiError::internal(format!("dispatch: {e}")))?;
       Ok(Json(json!({
           "text": resp.text,
           "model": resp.model,
           "usage": resp.usage,
       })))
   }
   ```

3. **For streaming routes**, use `model_call_service.stream(...)` and
   forward `ModelStreamEvent`s as SSE:
   ```rust
   use axum::response::sse::{Event, Sse};
   use futures::StreamExt;

   async fn handler(...) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
       let stream = state.model_call_service.stream(req).await?;
       let sse_stream = stream.map(|ev| match ev {
           ModelStreamEvent::TextDelta(t) => Ok(Event::default().event("delta").data(t)),
           ModelStreamEvent::Completed { text, usage } => Ok(
               Event::default().event("done").json_data(json!({"text": text, "usage": usage})).unwrap()
           ),
           ModelStreamEvent::Failed { reason, code } => Ok(
               Event::default().event("error").json_data(json!({"reason": reason, "code": code})).unwrap()
           ),
       });
       Sse::new(sse_stream)
   }
   ```

4. **Remove the unused imports** if `reqwest` is no longer needed
   (often `reqwest` stays imported for other handlers in the same file —
   that's fine).

5. **Tests**:
   ```rust
   #[tokio::test]
   async fn route_uses_shared_dispatch() {
       let stub_service = StubModelCallService::with_response("hello");
       let state = test_state_with_dispatch(stub_service);
       let resp = post(&app(state), "/api/inference/complete",
           json!({"model": "any", "messages": [{"role": "user", "content": "hi"}]})).await;
       assert_eq!(resp.status(), 200);
       let body: Value = resp.json().await.unwrap();
       assert_eq!(body["text"], "hello");
   }
   ```

### Routes to migrate (suggested order, easiest first)

| Order | Route | File | Notes |
|---|---|---|---|
| 1 | `POST /api/inference/complete` | `routes/inference.rs` | Probably the most-used; smallest blast radius |
| 2 | `POST /api/research/complete` | `routes/research.rs` | Perplexity Sonar dispatch |
| 3 | `POST /api/agents/{id}/messages` | `routes/agents.rs` | Per-agent model invocation |
| 4 | `POST /api/diagnosis/{kind}` | `routes/diagnosis.rs` | One model call per diagnosis |
| 5 | (any remaining, route-by-route) | various | confirm via inventory |

Each is one commit. Each commit:

- Migrates one route.
- Adds a stub-based test.
- Removes the route's unused HTTP client construction.

### Verify per route

```bash
rg 'reqwest::Client::new' crates/roko-serve/src/routes/<route>.rs
# Empty for migrated routes
cargo test -p roko-serve <route>::<test_name> --lib
```

### Final verification

```bash
# Inventory all remaining reqwest::Client::new in routes
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/ -n

# Each match must be in a non-LLM route. Audit.
```

---

## Phase 2: Quarantine `dispatch_direct` (T5-37)

### Step 1: Feature-gate the module

`crates/roko-cli/Cargo.toml`:

```toml
[features]
default = []
legacy-direct-dispatch = []
```

`crates/roko-cli/src/lib.rs`:

```rust
#[cfg(feature = "legacy-direct-dispatch")]
pub mod dispatch_direct;
```

### Step 2: Migrate production callers

```bash
rg 'dispatch_direct' crates/ -g '*.rs'
```

For each caller:

1. **`crates/roko-cli/src/chat_inline.rs`** — historically used
   `dispatch_direct` for the single-turn `roko run "..."` path. The
   migration to `ChatAgentSession` (R3_D02) is partial. Confirm by
   reading. If `dispatch_direct` calls remain, replace with:
   ```rust
   let stream = self.model_call_service.stream(req).await?;
   ```
2. **`crates/roko-cli/src/unified.rs`** — same migration.
3. **`crates/roko-chain/src/marketplace.rs`** and
   **`identity_economy_markets.rs`** — these are chain-side. They likely
   call dispatch_direct for AI-judging marketplace decisions. Migrate to
   `model_call_service`. If the chain crate doesn't have access to
   `ModelCallService`, this is a layering bug — pass it in via dependency
   injection from the caller.
4. **`crates/roko-cli/src/lib.rs`** — re-exports `dispatch_direct`. Move
   re-export under `#[cfg(feature = "legacy-direct-dispatch")]`.

### Step 3: Verify default build excludes the module

```bash
cargo build --workspace
cargo build --workspace --release
# Both must succeed without legacy-direct-dispatch feature.

cargo build --workspace --features legacy-direct-dispatch
# Must also succeed (for tests that need it).

cargo test --workspace
cargo test --workspace --features legacy-direct-dispatch
# Both must pass.
```

### Step 4: Static check (plan 27 will promote to CI)

Add to `scripts/roko-fitness-checks.sh`:

```bash
echo "[fitness] checking dispatch_direct usage..."
violations=$(rg 'dispatch_direct' crates/ -g '*.rs' \
  | rg -v '^crates/roko-cli/src/dispatch_direct\.rs:' \
  | rg -v '#\[cfg\(feature = "legacy-direct-dispatch"\)\]' \
  | rg -v '#\[cfg_attr\(.*feature = "legacy-direct-dispatch"\)' )
if [ -n "$violations" ]; then
    echo "FAIL: dispatch_direct used outside feature gate:"
    echo "$violations"
    exit 1
fi
```

### Step 5: Plan deletion follow-up

Track T5-37b as "delete `dispatch_direct.rs` entirely after 30 days
green." Not part of this plan.

---

## Phase 3: Add Generation Settings to `ModelCallService`

**Why**: Today `ModelCallService::call(req)` uses hardcoded defaults for
`max_tokens`, `temperature`, `timeout`. The audit (doc 35 § Dispatch)
flags these as "should come from model/profile/request config."

### Implementation

Extend `ModelCallRequest`:

```rust
pub struct ModelCallRequest {
    pub model: String,
    pub provider: Option<String>,    // optional override
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Vec<String>,
    pub timeout: Option<Duration>,
    pub tools: Vec<ToolSpec>,
    pub streaming: bool,
}
```

Wire from `RokoConfig`:

- Per-model defaults: `cfg.models[<slug>].max_tokens`,
  `cfg.models[<slug>].temperature`, etc.
- Per-profile overrides: `cfg.agent.profile_overrides`.
- Per-request override: caller's explicit values.

Resolution order: caller → profile → model → service default.

Implement in `DispatchResolver::resolve_existing` (or a sibling method):

```rust
pub fn resolve_generation_settings(
    &self,
    plan: &mut DispatchPlan,
    profile: Option<&str>,
    overrides: GenerationOverrides,
) {
    let model_cfg = self.models.get(&plan.model);
    let profile_cfg = profile.and_then(|p| self.profiles.get(p));
    plan.max_tokens = overrides.max_tokens
        .or(profile_cfg.and_then(|p| p.max_tokens))
        .or(model_cfg.and_then(|m| m.max_tokens))
        .unwrap_or(2048);
    // ... same for temperature, top_p, timeout
}
```

### Verify

```bash
cargo test -p roko-agent generation_settings --lib
rg 'max_tokens: 2048' crates/roko-agent/src/  # only in the resolver default
```

**Estimated effort**: 4-6 hours.

---

## Combined Verification

After all phases:

```bash
# Phase 1: serve dispatch consolidated
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/ \
  | rg -v '(connectors|webhooks|integrations|chain|deploy|relay|providers/health)'
# Empty

# Phase 2: dispatch_direct quarantined
cargo build --workspace
rg 'dispatch_direct' crates/ -g '*.rs' \
  | rg -v 'crates/roko-cli/src/dispatch_direct\.rs:' \
  | rg -v '#\[cfg\(feature = "legacy-direct-dispatch"\)\]'
# Empty (in non-test, non-feature-gated code)

# Phase 3: generation settings flow through
rg 'ModelCallRequest' crates/roko-agent/src/model_call_service.rs
# Carries max_tokens, temperature, etc. from config
```

---

## Status

- [ ] Phase 1 — Serve LLM routes migrated to `ModelCallService` (one commit per route)
- [ ] Phase 2 — `dispatch_direct` feature-gated, production callers migrated
- [ ] Phase 3 — Generation settings threaded through `ModelCallService`
