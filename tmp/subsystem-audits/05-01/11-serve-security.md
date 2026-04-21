# 11 — HTTP Serve: Security & Stability Issues

## HIGH: Path traversal in agent creation

**File:** `crates/roko-serve/src/routes/agents.rs:609`

```rust
let agents_dir = state.workdir.join(".roko").join("agents").join(&req.name);
```

`req.name` comes directly from the request body. A malicious name like `../../etc/passwd` or `../../../home/user/.ssh/id_rsa` would write outside `.roko/agents/`.

The codebase HAS `validate_path_segment()` in the error module — it's just not called here.

---

## HIGH: TOML injection in agent manifest

**File:** `crates/roko-serve/src/routes/agents.rs:630-637`

```rust
let manifest_toml = format!(
    r#"[core.domain.{domain}]"#,
    domain = req.domain,
);
```

If `req.domain` contains `"foo]\n[evil]\nmalicious = true"`, it breaks the TOML structure. The code uses `toml_quote()` for the prompt field but NOT for the domain field.

---

## HIGH: Global static state in vision loop

**File:** `crates/roko-serve/src/routes/vision_loop.rs:22-23`

```rust
static VISION_LOOPS: std::sync::LazyLock<RwLock<HashMap<String, VisionLoopHandle>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));
```

Problems:
- Tests can't run in parallel (shared mutable state)
- Multiple roko-serve instances collide
- Abandoned handles are never cleaned up (memory leak)
- Should be in `AppState`, not a global static

---

## HIGH: Silent persistence failures in deployments

**File:** `crates/roko-serve/src/routes/deployments.rs:477-488`

```rust
let _ = tokio::fs::create_dir_all(path.parent().unwrap_or(&state.workdir)).await;
if let Ok(json) = serde_json::to_string_pretty(&entries) {
    let tmp = path.with_extension("json.tmp");
    if tokio::fs::write(&tmp, &json).await.is_ok() {
        let _ = tokio::fs::rename(&tmp, &path).await;
    }
}
```

Every step silently swallows errors. If persistence fails, deployments are lost on restart with zero indication.

---

## HIGH: No timeout on spawned plan execution

**File:** `crates/roko-serve/src/routes/plans.rs:216-237`

```rust
let handle = tokio::spawn({
    async move {
        let success = match runtime.run_once(&workdir, &prompt).await {
            // No timeout!
```

A runaway agent task consumes CPU/memory indefinitely. No mechanism to kill hung tasks. Combined with `max_restarts: 0` in the supervisor, a stuck task blocks the plan forever.

---

## MEDIUM: CORS allows ANY methods and headers

**File:** `crates/roko-serve/src/routes/middleware.rs:432-463`

```rust
CorsLayer::new()
    .allow_methods(Any)     // DELETE, PATCH, etc. on ALL routes
    .allow_headers(Any)     // Arbitrary headers accepted
```

Even in non-public mode, `allow_methods(Any)` permits DELETE/PATCH on every route. No X-Content-Type-Options, X-Frame-Options, or CSP headers are set.

---

## MEDIUM: Inconsistent route parameter types

**File:** `crates/roko-serve/src/routes/agents.rs:960-981`

```rust
async fn get_agent(Path(id): Path<String>) { ... }
async fn stop_agent(Path(id): Path<u64>) { ... }
```

GET takes String, POST takes u64 for the same resource. Callers get confusing 400 errors when types don't match.

---

## MEDIUM: Plan pause silently drops state

**File:** `crates/roko-serve/src/routes/plans.rs:303-316`

```rust
let _ = tokio::fs::create_dir_all(&snapshot_dir).await;
let _ = tokio::fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot).unwrap_or_default()).await;
```

If snapshot write fails, the plan is paused but can never be resumed. The `handle.handle.abort()` on line 298 kills the task immediately with no graceful shutdown.

---

## MEDIUM: SWE-Bench stubs in production routes

**File:** `crates/roko-serve/src/routes/bench.rs:796-827`

```rust
pub fn format_greeting(name: &str) -> String {
    todo!("format the greeting")
}
pub fn wrap_result<T, E>(value: Result<T, E>) -> Result<T, E> {
    unimplemented!("wrap_result should return the input Result unchanged")
}
```

These `todo!()` and `unimplemented!()` macros will panic if called. They're in the bench module (intentional for benchmark tasks) but pollute the production binary.

---

## MEDIUM: Agent server binds to 0.0.0.0 by default

**File:** `crates/roko-agent-server/src/lib.rs:349`

```rust
let bind = self.bind.unwrap_or_else(|| "0.0.0.0:0".to_string());
```

Per-agent sidecar listens on all interfaces by default. Should default to `127.0.0.1:0`.

---

## LOW: Auth middleware assumed but not verified per-route

Routes like `DELETE /secrets/{namespace}/{key}` and `DELETE /connectors/{name}` don't show per-handler auth checks. They rely on the outer Router's middleware layer, but this coupling is implicit and easy to break when adding new routes.
