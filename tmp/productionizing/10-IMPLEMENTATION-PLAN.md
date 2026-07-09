# Implementation Plan: Productionize Roko

This is the master implementation plan. Each task is self-contained with enough context
for an agent without codebase knowledge to execute it.

**Read `07-ANTI-PATTERNS.md` before starting any task.**

## Task Dependency Graph

```
P1 (provider availability) ──┐
                              ├── P3 (cascade router filter) ── P4 (startup banner)
P2 (config available_models) ─┘

P5 (file locking) ── standalone

P6 (auth + path traversal) ── standalone

P7 (HTTP hardening) ── standalone

P8 (error logging) ── standalone

P9 (context overflow) ── standalone

P10 (expect/unwrap cleanup) ── standalone (large, can be split)

P11 (log rotation) ── standalone

P12 (eprintln → tracing) ── standalone

P13 (Dockerfile.optimized) ── standalone

P14 (railway.toml + fly.toml) ── depends on P13

P15 (production roko.toml) ── depends on P1

P16 (Dockerfile.runtime) ── standalone

P17 (deploy script) ── depends on P16

P18 (CI auto-deploy) ── depends on P13 or P16
```

Tasks P1-P4 must be done in order. All others are independent.
P16-P18 are the "fast build/deploy" tasks — see `08-FAST-BUILD-DEPLOY.md` for full context.

---

## P1: Add provider availability checking

**Priority**: CRITICAL
**Estimated scope**: ~50 lines across 2 files
**Depends on**: nothing
**Blocks**: P2, P3, P4, P15

### Context

Providers are configured in `roko.toml` under `[[providers]]`. Each has an optional
`api_key_env` field (e.g., `"ANTHROPIC_API_KEY"`). Currently, nothing checks whether that
env var is actually set before routing to the provider.

### What to do

**File: `crates/roko-core/src/config/provider.rs`**

Add a method to `ProviderConfig`:

```rust
impl ProviderConfig {
    /// Returns true if this provider can be used (has API key or doesn't need one).
    pub fn is_available(&self) -> bool {
        match &self.api_key_env {
            Some(env_name) => std::env::var(env_name).is_ok(),
            None => true, // Local providers (ollama) don't need keys
        }
    }
}
```

**File: `crates/roko-core/src/config/mod.rs`**

Add methods to `RokoConfig`:

```rust
impl RokoConfig {
    /// Returns provider slug for a given model slug, or None.
    pub fn provider_for_model(&self, model_slug: &str) -> Option<&str> {
        self.models.iter()
            .find(|m| m.slug == model_slug)
            .map(|m| m.provider.as_str())
    }

    /// Returns true if the provider for this model has a valid API key.
    pub fn provider_available_for_model(&self, model_slug: &str) -> bool {
        let Some(provider_slug) = self.provider_for_model(model_slug) else {
            return false;
        };
        self.providers.iter()
            .find(|p| p.slug == provider_slug)
            .map(|p| p.is_available())
            .unwrap_or(false)
    }

    /// Returns names of all providers that have valid API keys.
    pub fn available_provider_names(&self) -> Vec<&str> {
        self.providers.iter()
            .filter(|p| p.is_available())
            .map(|p| p.slug.as_str())
            .collect()
    }
}
```

### How to find these files

```bash
# Provider config struct:
grep -n "pub struct ProviderConfig" crates/roko-core/src/config/provider.rs

# RokoConfig struct:
grep -n "pub struct RokoConfig" crates/roko-core/src/config/mod.rs

# Existing model config:
grep -n "pub struct ModelConfig" crates/roko-core/src/config/
```

### What NOT to do

- DO NOT change the `ProviderConfig` struct fields — only add methods
- DO NOT make `is_available()` async — env var checks are instant
- DO NOT cache the result — keys can be set/unset at runtime
- DO NOT panic if a provider is unavailable — return false

### Verification

```bash
cargo test -p roko-core -- provider
cargo clippy -p roko-core --no-deps -- -D warnings
```

---

## P2: Filter models to available providers in config

**Priority**: CRITICAL
**Estimated scope**: ~30 lines, 1 file
**Depends on**: P1

### Context

`RokoConfig` has a `models` collection (Vec or map of `ModelConfig`). Currently all models
are treated as available. After P1, we can filter.

### What to do

**File: `crates/roko-core/src/config/mod.rs`**

Add:

```rust
impl RokoConfig {
    /// Returns only models whose provider has a valid API key.
    pub fn available_models(&self) -> Vec<&ModelConfig> {
        self.models.iter()
            .filter(|m| self.provider_available_for_model(&m.slug))
            .collect()
    }

    /// Returns available model slugs as a HashSet for fast lookup.
    pub fn available_model_set(&self) -> std::collections::HashSet<String> {
        self.available_models().iter().map(|m| m.slug.clone()).collect()
    }
}
```

### What NOT to do

- DO NOT mutate the config — return filtered views, not modified state
- DO NOT remove unavailable models from the config struct — other code may need the full list
- DO NOT introduce new dependencies

### Verification

```bash
cargo test -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
```

---

## P3: Make CascadeRouter only consider available models

**Priority**: CRITICAL
**Estimated scope**: ~40 lines, 2 files
**Depends on**: P1, P2

### Context

The CascadeRouter in `crates/roko-learn/src/cascade_router.rs` selects models based on
learned quality/cost/latency stats (UCB1 bandit). It has NO concept of "this provider
doesn't have a key." It can route to Gemini when no `GEMINI_API_KEY` exists.

The router has 3 stages based on observation count:
1. Static (< 50 obs): hardcoded role→model table
2. Confidence (50-200 obs): empirical pass rates
3. UCB1 (> 200 obs): contextual bandit

All 3 stages need to filter candidates.

### What to do

**File: `crates/roko-learn/src/cascade_router.rs`**

Find the main routing method (likely `pub fn route(...)` or `pub fn select_model(...)`).
Add an `available_models: &HashSet<String>` parameter. Filter candidates before scoring.

```rust
// In the routing method:
let candidates: Vec<_> = all_candidates.iter()
    .filter(|m| available_models.contains(&m.slug))
    .collect();

if candidates.is_empty() {
    // Fall back to the config's default model (which was already validated)
    return CascadeModel::new(config.agent.default_model.clone());
}
```

**File: `crates/roko-cli/src/model_selection.rs`**

Find where CascadeRouter is called. Pass the available model set:

```rust
let available = config.available_model_set();
let cascade_choice = cascade_router.route(&ctx, &available);
```

**File: `crates/roko-agent/src/dispatch_resolver.rs`**

Find `fallback_candidates()`. Filter by availability:

```rust
fn fallback_candidates(config: &RokoConfig, primary: &str) -> Vec<String> {
    let mut fallbacks = /* ... existing logic ... */;
    fallbacks.retain(|model| config.provider_available_for_model(model));
    fallbacks
}
```

### How to find the routing method

```bash
grep -n "pub fn route\|pub fn select_model\|pub fn pick" crates/roko-learn/src/cascade_router.rs
grep -n "cascade_router\.\|CascadeRouter" crates/roko-cli/src/model_selection.rs
grep -n "fn fallback_candidates" crates/roko-agent/src/dispatch_resolver.rs
```

### What NOT to do

- DO NOT delete stats for unavailable models — they may become available later
- DO NOT change the UCB1 algorithm itself — only filter its input candidates
- DO NOT make the router depend on roko-core config directly — pass the available set as a param
- DO NOT change the function signature of public methods without updating all callers

### Verification

```bash
cargo test -p roko-learn -- cascade
cargo test -p roko-cli -- model_selection
cargo clippy --workspace --no-deps -- -D warnings
```

---

## P4: Pre-dispatch validation + startup banner

**Priority**: CRITICAL
**Estimated scope**: ~60 lines, 2 files
**Depends on**: P1, P2, P3

### Context

The orchestrator dispatches agents in `dispatch_agent_with()` in `orchestrate.rs`.
Currently it proceeds without checking if the resolved model's provider has a key.

Also: there's no startup output showing which providers are available, so the user
doesn't know what's usable until dispatch fails.

### What to do

**File: `crates/roko-cli/src/orchestrate.rs`**

Find `dispatch_agent_with` or the model selection block (around line 14630). Add validation
BEFORE dispatch:

```rust
// After model is selected but BEFORE agent is spawned:
if !self.config.provider_available_for_model(&selected_model) {
    let provider = self.config.provider_for_model(&selected_model)
        .unwrap_or("unknown");
    let env_var = self.config.providers.iter()
        .find(|p| p.slug == provider)
        .and_then(|p| p.api_key_env.as_deref())
        .unwrap_or("(unknown env var)");
    return Err(anyhow!(
        "Model '{}' requires provider '{}' but {} is not set. \
         Available providers: {:?}",
        selected_model, provider, env_var,
        self.config.available_provider_names()
    ));
}
```

Also remove the 7 hardcoded `"claude-sonnet-4-6"` fallbacks. Replace with:

```rust
// BEFORE (in ~7 locations):
.unwrap_or_else(|| "claude-sonnet-4-6".into())

// AFTER:
// Just use config.agent.default_model — it's already validated at startup
```

**File: `crates/roko-cli/src/orchestrate.rs` or `crates/roko-serve/src/lib.rs`**

Add a startup banner (in the `serve` or `plan run` entry point):

```rust
fn log_provider_status(config: &RokoConfig) {
    let available: Vec<_> = config.providers.iter()
        .filter(|p| p.is_available())
        .map(|p| {
            let model_count = config.models.iter()
                .filter(|m| m.provider == p.slug)
                .count();
            format!("{} ({} models)", p.slug, model_count)
        })
        .collect();

    let unavailable: Vec<_> = config.providers.iter()
        .filter(|p| !p.is_available())
        .map(|p| p.slug.as_str())
        .collect();

    tracing::info!(
        available = %available.join(", "),
        default_model = %config.agent.default_model,
        "Provider status"
    );
    if !unavailable.is_empty() {
        tracing::warn!(
            providers = %unavailable.join(", "),
            "Providers without API keys (models disabled)"
        );
    }
}
```

### How to find the hardcoded fallbacks

```bash
grep -n '"claude-sonnet-4-6"' crates/roko-cli/src/orchestrate.rs
grep -n '"claude-sonnet-4-6"' crates/roko-core/src/config/agent.rs
grep -n '"claude-sonnet-4-6"' crates/roko-cli/src/model_selection.rs
```

### What NOT to do

- DO NOT remove the `default_model` config field — just validate it has a working provider
- DO NOT make the startup banner a hard error if some providers are unavailable — just warn
- DO NOT change the dispatch flow beyond adding the validation check
- DO NOT log actual API key values — only log the env var NAME

### Verification

```bash
# Test that dispatch fails cleanly without a key:
ANTHROPIC_API_KEY= cargo run -p roko-cli -- run "test" 2>&1 | grep "not set"

# Test that serve shows the banner:
cargo run -p roko-cli -- serve 2>&1 | head -20
```

---

## P5: File locking for `.roko/` state writes

**Priority**: CRITICAL
**Estimated scope**: ~80 lines, 1 new utility + 4 call sites
**Depends on**: nothing

### Context

All state files in `.roko/` (episodes.jsonl, efficiency.jsonl, cascade-router.json,
executor.json) can be written by multiple processes concurrently (e.g., `roko serve`
and `roko plan run` running at the same time). Currently, only process-local
`tokio::sync::Mutex` is used, which does nothing for inter-process safety.

### What to do

**New utility in `crates/roko-fs/src/flock.rs`** (or add to existing file):

```rust
use std::fs::{File, OpenOptions};
use std::path::Path;

/// Acquire an exclusive file lock. Returns a guard that releases on drop.
pub fn exclusive_lock(path: &Path) -> std::io::Result<FileLockGuard> {
    let lock_path = path.with_extension("lock");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)?;

    // Unix: use flock
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(FileLockGuard { _file: file, lock_path })
}

pub struct FileLockGuard {
    _file: File, // Held open = lock held. Drop closes = lock released.
    lock_path: std::path::PathBuf,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        // Lock is released when file is closed (automatic).
        // Optionally remove the lock file:
        let _ = std::fs::remove_file(&self.lock_path);
    }
}
```

**Call sites to add locking**:

1. `crates/roko-learn/src/episode_logger.rs` — around the `append()` method
2. `crates/roko-learn/src/feedback_service.rs` — around `flush()`
3. `crates/roko-learn/src/cascade_router.rs` — around `save()`
4. `crates/roko-cli/src/orchestrate.rs` — around `save_snapshot_atomic()`

At each site, acquire the lock before writing:

```rust
let _lock = roko_fs::flock::exclusive_lock(&file_path)?;
// ... existing write logic ...
// Lock released when _lock is dropped
```

### What NOT to do

- DO NOT use `fd-lock` crate if you can avoid the dependency — libc `flock()` is sufficient
- DO NOT hold the lock across async `.await` points — flock is not async-safe. Do the write synchronously inside a `tokio::task::spawn_blocking` if needed.
- DO NOT remove the existing `tokio::sync::Mutex` — keep it for intra-process coordination. The flock is for inter-process only.
- DO NOT make the lock path configurable — use `{original_path}.lock` convention

### Verification

```bash
# Test concurrent writes don't corrupt:
# Terminal 1:
cargo run -p roko-cli -- serve &
# Terminal 2:
cargo run -p roko-cli -- plan run plans/ --resume .roko/state/executor.json

# Check for corruption:
python3 -c "
import json, sys
for line in open('.roko/episodes.jsonl'):
    try: json.loads(line)
    except: print('CORRUPT:', line[:80]); sys.exit(1)
print('OK')
"
```

---

## P6: Auth hardening + path traversal fix

**Priority**: CRITICAL (auth) + MEDIUM (path traversal)
**Estimated scope**: ~40 lines, 2 files
**Depends on**: nothing

### Context

Auth is disabled by default. When deploying publicly, this leaves all 85+ API routes
unprotected, including terminal (arbitrary command execution).

The SPA file serving in `embedded.rs` uses `dir.join(path)` without verifying the result
stays within the intended directory.

### What to do

**File: `crates/roko-serve/src/lib.rs`**

Find the bind validation logic (around line 641, `validate_bind_safety`). Add a hard
warning when auth is disabled on non-loopback:

```rust
// In the startup path, after config is loaded:
if !config.serve.auth.enabled && !is_loopback(&config.server.bind) {
    tracing::error!(
        "⚠ Auth is DISABLED but server is binding to {}. \
         ALL routes are unprotected including terminal (shell access). \
         Set [serve.auth] enabled = true in roko.toml or use --bind 127.0.0.1",
        config.server.bind
    );
    // Don't block startup, but make it VERY visible
}
```

**File: `crates/roko-serve/src/embedded.rs`**

Find `read_from_disk()` (around line 50). Add path canonicalization:

```rust
fn read_from_disk(path: &str) -> Option<(Vec<u8>, String)> {
    let dir = disk_dist_dir()?;
    let full = dir.join(path);

    // Prevent path traversal
    let canonical = full.canonicalize().ok()?;
    let dir_canonical = dir.canonicalize().ok()?;
    if !canonical.starts_with(&dir_canonical) {
        tracing::warn!(path = %path, "path traversal attempt blocked");
        return None;
    }

    if canonical.is_file() {
        let content = std::fs::read(&canonical).ok()?;
        let mime = mime_for(&canonical);
        return Some((content, mime));
    }

    // SPA fallback
    let index = dir.join("index.html");
    std::fs::read(&index).ok().map(|c| (c, "text/html".into()))
}
```

### What NOT to do

- DO NOT block server startup when auth is disabled — just warn loudly
- DO NOT change the auth middleware itself — it already works when enabled
- DO NOT remove the `unsafe_public_cors` option — it's useful for development
- DO NOT use `path.contains("..")` as the ONLY check — use canonicalize for robustness

### Verification

```bash
# Test path traversal block:
cargo run -p roko-cli -- serve &
curl -s http://localhost:6677/../../../etc/passwd | head -1
# Should return index.html (SPA fallback), NOT /etc/passwd

# Test auth warning:
# Edit roko.toml: bind = "0.0.0.0", auth.enabled = false
cargo run -p roko-cli -- serve 2>&1 | grep "Auth is DISABLED"
```

---

## P7: HTTP server hardening (timeouts, limits, health codes)

**Priority**: HIGH
**Estimated scope**: ~50 lines, 1-2 files
**Depends on**: nothing

### Context

The Axum HTTP server has no per-request timeouts, no body size limits, and no rate
limiting. The health endpoint returns 200 even when providers are down.

### What to do

**File: `crates/roko-serve/src/lib.rs`**

Find where the router is constructed (the main `Router::new()` chain). Add tower layers:

```rust
use tower_http::timeout::TimeoutLayer;
use tower_http::limit::RequestBodyLimitLayer;

let app = Router::new()
    // ... existing routes ...
    .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
    .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MiB
    ;
```

Check that `tower-http` is already a dependency. If not, add it to `crates/roko-serve/Cargo.toml`:
```toml
tower-http = { version = "0.6", features = ["timeout", "limit"] }
```

**File: `crates/roko-serve/src/routes/status/health.rs`**

Find the health handler. Change the HTTP status code based on provider health:

```rust
// After computing status:
let http_status = match status.as_str() {
    "ok" => StatusCode::OK,
    "degraded" => StatusCode::OK, // Still serving, just degraded
    "down" | _ => StatusCode::SERVICE_UNAVAILABLE,
};

(http_status, Json(health_response))
```

### What NOT to do

- DO NOT set the timeout too low — some routes (plan execution) can take minutes.
  Use 30s for general routes and exempt long-running ones with `.route_layer()`.
- DO NOT add rate limiting in this task — that's a bigger change (needs IP tracking).
  Just add the timeout + body limit.
- DO NOT make the body limit too small — config uploads and plan files can be large.
  10 MiB is reasonable.

### Verification

```bash
# Test timeout (send a request to a route that would hang):
timeout 35 curl -s http://localhost:6677/api/plans/run -d '{}' -H 'Content-Type: application/json'
# Should return within 30s with a timeout error

# Test body limit:
dd if=/dev/zero bs=1M count=20 | curl -s -X POST http://localhost:6677/api/config -d @-
# Should return 413 Payload Too Large

# Test health status code:
# With all providers available:
curl -s -o /dev/null -w "%{http_code}" http://localhost:6677/api/health  # 200
```

---

## P8: Replace silent error swallowing with logging

**Priority**: HIGH
**Estimated scope**: ~60 lines across 3-4 files
**Depends on**: nothing

### Context

25+ locations silently swallow errors with `let _ = ...` or `.ok()`. In production,
these make failures invisible. The most critical are in roko-serve startup and
security-related code.

### What to do

Fix ONLY the high-impact silent swallows. Don't touch every `.ok()` in the codebase.

**File: `crates/roko-serve/src/lib.rs`**

```rust
// Line ~830 — bootstrap
// BEFORE:
let _ = state.state_hub.bootstrap_from_workdir(&state.workdir);
// AFTER:
if let Err(e) = state.state_hub.bootstrap_from_workdir(&state.workdir) {
    tracing::warn!(error = %e, "StateHub bootstrap failed, starting with empty state");
}

// Line ~1668 — event sources
// BEFORE:
let _ = start_event_source_group(state, sources);
// AFTER:
if let Err(e) = start_event_source_group(state, sources) {
    tracing::error!(error = %e, "Failed to start event sources");
}
```

**File: `crates/roko-serve/src/jwks.rs`**

```rust
// Line ~145 — JWKS refresh (SECURITY CRITICAL)
// BEFORE:
let _ = self.refresh_jwks().await;
// AFTER:
if let Err(e) = self.refresh_jwks().await {
    tracing::error!(error = %e, "JWKS refresh failed — JWT validation may use stale keys");
}
```

**File: `crates/roko-serve/src/terminal.rs`**

Add logging to the 5+ silent terminal operation failures. Pattern:

```rust
// BEFORE:
let _ = state.terminal_sessions.resize(...);
// AFTER:
if let Err(e) = state.terminal_sessions.resize(...) {
    tracing::warn!(error = %e, session_id = %id, "terminal resize failed");
}
```

### How to find all locations

```bash
grep -n 'let _ = ' crates/roko-serve/src/*.rs crates/roko-serve/src/**/*.rs | grep -v test | grep -v '//'
```

### What NOT to do

- DO NOT change `.ok()` in match arms where the None case is intentionally handled
- DO NOT add error logging to intentionally-ignored cleanup operations (e.g., removing temp files)
- DO NOT change the return type of functions — just add logging before the swallow
- DO NOT add `tracing::error!` for non-critical failures — use `tracing::warn!` for recoverable ones

### Verification

```bash
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve
# Start server and check logs for new warning messages:
RUST_LOG=debug cargo run -p roko-cli -- serve 2>&1 | grep -i "warn\|error"
```

---

## P9: Handle context overflow before dispatch

**Priority**: HIGH
**Estimated scope**: ~50 lines, 2 files
**Depends on**: P1 (needs model config for context_window)

### Context

When a prompt exceeds a model's context window, dispatch fails permanently (ContextOverflow
is non-retryable). There's no pre-check. The system has tool output truncation
(`dispatcher/truncate.rs`) but no input/prompt truncation.

### What to do

**File: `crates/roko-cli/src/orchestrate.rs`**

Before dispatch, estimate prompt size and check against model context window:

```rust
// After model is selected, before dispatch:
let prompt_estimate_tokens = prompt_text.len() / 4; // rough char-to-token ratio
let model_config = self.config.models.iter().find(|m| m.slug == selected_model);

if let Some(mc) = model_config {
    let threshold = (mc.context_window as f64 * 0.85) as usize; // 85% to leave room for response
    if prompt_estimate_tokens > threshold {
        tracing::warn!(
            model = %selected_model,
            prompt_tokens = prompt_estimate_tokens,
            context_window = mc.context_window,
            "Prompt may exceed context window, attempting truncation"
        );
        // Try a model with larger context
        let bigger = self.config.available_models().iter()
            .filter(|m| m.context_window > mc.context_window)
            .min_by_key(|m| m.context_window) // smallest model that fits
            .map(|m| m.slug.clone());

        if let Some(bigger_model) = bigger {
            tracing::info!(fallback_model = %bigger_model, "Falling back to larger context model");
            selected_model = bigger_model;
        } else {
            tracing::warn!("No larger model available, dispatch may fail with ContextOverflow");
        }
    }
}
```

### What NOT to do

- DO NOT implement a full tokenizer — `len() / 4` is good enough for a pre-check
- DO NOT silently truncate the user's prompt — log a warning and try a bigger model
- DO NOT make this a hard error — let the dispatch attempt proceed (it might work)
- DO NOT change the retry policy for ContextOverflow — that's a separate concern

### Verification

```bash
# Create a very long prompt and test:
python3 -c "print('x ' * 100000)" | cargo run -p roko-cli -- run --model claude-haiku-4-5
# Should see a warning about context window
```

---

## P10: Replace critical `expect()` and `unwrap()` calls

**Priority**: HIGH
**Estimated scope**: ~200 lines across 3 files (focus on critical paths only)
**Depends on**: nothing

### Context

92 `expect()` calls in orchestrate.rs + 11 mutex `unwrap()`/`expect()` calls that will
permanently crash on lock poisoning. Focus on the ones in hot paths.

### What to do

**Priority 1: Mutex lock poisoning (will cascade-crash)**

**File: `crates/roko-cli/src/dispatch/warm_pool.rs`**

Replace all 4 locations:

```rust
// BEFORE:
self.inner.lock().expect("poisoned")
// AFTER:
self.inner.lock().unwrap_or_else(|poisoned| {
    tracing::warn!("warm pool lock was poisoned, recovering");
    poisoned.into_inner()
})
```

**File: `crates/roko-cli/src/orchestrate.rs`**

Replace enrichment stats lock (2 locations around line 1437-1441):

```rust
// BEFORE:
self.stats.lock().expect("enrichment stats lock")
// AFTER:
self.stats.lock().unwrap_or_else(|p| p.into_inner())
```

Replace gate sink lock (line ~17388):

```rust
// BEFORE:
sink.lock().expect("recorded gate sink poisoned")
// AFTER:
sink.lock().unwrap_or_else(|p| p.into_inner())
```

**Priority 2: Serialization expects (lines ~17589, 17617)**

```rust
// BEFORE:
Body::from_json(&manifest).expect("SymbolManifest serializes")
// AFTER:
Body::from_json(&manifest).map_err(|e| anyhow!("failed to serialize SymbolManifest: {e}"))?
```

**Priority 3: The 92 generic `expect()` calls**

These are lower priority but should be addressed over time. For now, focus on the ones
in the dispatch and gate verification paths. Use this grep to find them:

```bash
grep -n '\.expect(' crates/roko-cli/src/orchestrate.rs | grep -v test | head -20
```

Replace with `?` + `anyhow::Context`:

```rust
// BEFORE:
.expect("filtered acceptance contract")
// AFTER:
.context("filtered acceptance contract missing")?
```

### What NOT to do

- DO NOT try to fix all 92 in one PR — focus on mutex + serialization (priority 1 & 2)
- DO NOT change the behavior when recovering from poisoned mutex — just log and recover
- DO NOT add `#[allow(clippy::expect_used)]` to suppress — fix them
- DO NOT convert to `unwrap()` — that's worse

### Verification

```bash
cargo test -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

---

## P11: Auto-rotate unbounded log files

**Priority**: MEDIUM
**Estimated scope**: ~40 lines, 1 file
**Depends on**: nothing

### Context

`episodes.jsonl`, `signals.jsonl`, and `efficiency.jsonl` grow unbounded. GC exists
(`roko-fs/src/gc.rs`) but must be called manually. A long-running server will fill disk.

### What to do

**File: `crates/roko-cli/src/orchestrate.rs` or `crates/roko-serve/src/lib.rs`**

Add a periodic check after state writes:

```rust
/// Check if .roko/ is getting large and trigger GC if needed.
fn maybe_auto_gc(workdir: &Path, config: &RokoConfig) {
    let roko_dir = workdir.join(".roko");
    let threshold_mb = config.gc.auto_threshold_mb.unwrap_or(500);

    // Quick check: if the dir exists and is large
    if let Ok(size) = dir_size_mb(&roko_dir) {
        if size > threshold_mb {
            tracing::info!(size_mb = size, threshold_mb, "Auto-GC triggered");
            if let Err(e) = roko_fs::gc::compact_all(&roko_dir) {
                tracing::warn!(error = %e, "Auto-GC failed");
            }
        }
    }
}
```

Call this after every plan completion or every N tasks.

### What NOT to do

- DO NOT run GC on every single write — too expensive
- DO NOT delete files without compaction — use the existing GC logic in roko-fs
- DO NOT make the threshold too low — 500 MB is reasonable for a workspace

### Verification

```bash
# Check current .roko/ size:
du -sh .roko/

# After running some tasks, verify GC triggers:
cargo run -p roko-cli -- knowledge gc
```

---

## P12: Replace `eprintln!()` with structured logging

**Priority**: MEDIUM
**Estimated scope**: ~80 lines, 1 file
**Depends on**: nothing

### Context

~50 `eprintln!()` calls in `orchestrate.rs` bypass structured logging. These are in the
conductor feedback loop, health checks, fleet status, and progress output.

### What to do

**File: `crates/roko-cli/src/orchestrate.rs`**

Find and replace. Pattern:

```bash
grep -n 'eprintln!' crates/roko-cli/src/orchestrate.rs | grep -v test | grep -v '//'
```

Replace each one contextually:

```rust
// BEFORE (health check alert):
eprintln!("[ALERT] Agent {} unhealthy: {}", agent_id, reason);
// AFTER:
tracing::warn!(agent_id = %agent_id, reason = %reason, "agent unhealthy");

// BEFORE (progress):
eprintln!("  Task {}/{} complete", done, total);
// AFTER:
tracing::info!(done, total, "task complete");

// BEFORE (conductor decision):
eprintln!("[CONDUCTOR] {}", decision);
// AFTER:
tracing::info!(decision = %decision, "conductor decision");
```

### What NOT to do

- DO NOT remove `eprintln!()` calls that are in `main.rs` for direct user output
- DO NOT use `tracing::error!` for non-errors — match the log level to severity
- DO NOT add structured fields for everything — keep it readable
- DO NOT change the actual logic, only the output mechanism

### Verification

```bash
cargo clippy -p roko-cli --no-deps -- -D warnings
# Run a plan and verify logs are structured:
RUST_LOG=info cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -c "eprintln"
# Should be 0
```

---

## P13: Create Dockerfile.optimized (cargo-chef + sccache for CI builds)

**Priority**: MEDIUM
**Estimated scope**: 1 new file, ~60 lines
**Depends on**: nothing

### Context

This is the Tier 2 build — used when you can't cross-compile locally (CI, reproducible
builds). Uses cargo-chef to cache dependency compilation and sccache for per-unit caching.
Source-only changes rebuild in 15-30 seconds instead of 10-15 minutes.

See `08-FAST-BUILD-DEPLOY.md` for the full Dockerfile.optimized content.

### What to do

Create `Dockerfile.optimized` at the repo root with the cargo-chef + sccache pattern
from `08-FAST-BUILD-DEPLOY.md`. Key stages:

1. **Frontend** (parallel): `node:22-bookworm-slim` → `npm ci && npm run build`
2. **Planner**: `cargo-chef:latest-rust-1.91` → `cargo chef prepare --recipe-path recipe.json`
3. **Builder**: Cook deps (cached) → copy source → `cargo build --release --bin roko`
4. **Runtime**: `debian:bookworm-slim` → copy binary

### Also update the root Dockerfile

The root `Dockerfile` should use the same optimized pattern since it's what Railway
uses by default. Either:
- Replace `Dockerfile` contents with `Dockerfile.optimized`
- Or update `railway.toml` to use `dockerfilePath = "Dockerfile.optimized"`

### Verification

```bash
# Clean build (~3-5 min):
docker build -f Dockerfile.optimized -t roko:chef .

# Source-only change (~15-30s with warm cache):
touch crates/roko-cli/src/main.rs
docker build -f Dockerfile.optimized -t roko:chef .

# Verify it runs:
docker run --rm -p 6677:6677 -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY roko:chef &
sleep 5
curl -s http://localhost:6677/api/health
curl -s http://localhost:6677/ | head -1
```

---

## P14: Fix railway.toml and fly.toml

**Priority**: MEDIUM
**Estimated scope**: ~10 lines, 2 files
**Depends on**: P13

### What to do

**File: `railway.toml`**

```toml
[build]
builder = "dockerfile"
dockerfilePath = "Dockerfile"

[deploy]
healthcheckPath = "/api/health"
healthcheckTimeout = 300
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 3
```

**File: `fly.toml`**

Fix the port mismatch. Find `internal_port` and change from 3000 to 6677:

```toml
[[services]]
  internal_port = 6677
```

Or change the Dockerfile CMD to use port 3000. Either way, they must match.

### Verification

```bash
# Validate railway.toml syntax:
cat railway.toml

# Validate fly.toml:
fly config validate  # if flyctl installed
```

---

## P15: Create production roko.toml

**Priority**: MEDIUM
**Estimated scope**: Config file edit
**Depends on**: P1 (for validation to work)

### What to do

See `05-ROKO-TOML-PRODUCTION.md` for the full config. Key changes:
- Remove all provider entries without keys (moonshot, zhipu, cerebras, openrouter, zai)
- Remove all model entries for removed providers
- Set `serve.auth.enabled = true`
- Keep: anthropic, openai, perplexity, gemini, ollama

Create a `roko.production.toml` alongside the main one. Use it via:
```bash
ROKO_CONFIG=roko.production.toml roko serve
```

### What NOT to do

- DO NOT modify the main `roko.toml` — it's used for development
- DO NOT hardcode API keys in the TOML file — use `api_key_env` references
- DO NOT remove the routing config — just ensure it references available models

---

## P16: Create Dockerfile.runtime (binary-only image for fast deploys)

**Priority**: HIGH
**Estimated scope**: 1 new file, ~15 lines
**Depends on**: nothing

### Context

This is the key to Tier 1 speed (45-second deploys). Instead of building Rust inside Docker
(10-15 minutes), you cross-compile locally with `cargo zigbuild` and copy the pre-built
binary into a minimal image. Docker build takes ~5 seconds.

### What to do

**Create `Dockerfile.runtime` at the repo root:**

```dockerfile
# Minimal image — no Rust toolchain, just the pre-built binary
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 git curl \
    && rm -rf /var/lib/apt/lists/*

# Copy cross-compiled binary (built with: cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko)
COPY target/x86_64-unknown-linux-gnu/release/roko /usr/local/bin/roko

# Copy frontend assets (pre-built with: cd demo/demo-app && npm run build)
COPY demo/demo-app/dist/ /usr/share/roko/spa/

# Copy config
COPY roko.toml /workspace/roko.toml

WORKDIR /workspace
ENV RUST_LOG=info,roko=debug
ENV ROKO_SPA_DIR=/usr/share/roko/spa
EXPOSE 6677
CMD ["roko", "serve", "--bind", "0.0.0.0", "--port", "6677"]
```

### Prerequisites (one-time setup)

```bash
brew install zig
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-gnu
```

### How to use

```bash
# 1. Build frontend
cd demo/demo-app && npm run build && cd ../..

# 2. Cross-compile for Linux (incremental: ~30s, clean: ~3min)
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko

# 3. Build Docker image (~5s)
docker build -f Dockerfile.runtime -t ghcr.io/nunchi-trade/roko:latest .

# 4. Push + deploy (~15s)
docker push ghcr.io/nunchi-trade/roko:latest
railway up --image ghcr.io/nunchi-trade/roko:latest
```

### If cargo zigbuild fails with OpenSSL errors

roko depends on `openssl-sys`. Options:
1. Add `features = ["vendored"]` to the openssl dependency in the relevant Cargo.toml
2. Use musl target instead: `rustup target add x86_64-unknown-linux-musl && cargo zigbuild --release --target x86_64-unknown-linux-musl --bin roko`
3. Fall back to `cross` tool: `cargo install cross && cross build --release --target x86_64-unknown-linux-gnu --bin roko`

### What NOT to do

- DO NOT include `target/` in .dockerignore for this Dockerfile — it needs the cross-compiled binary
- DO NOT use `scratch` base image — roko needs glibc, libssl, git, and curl at runtime
- DO NOT skip the frontend build — ROKO_SPA_DIR expects the dist/ to exist

### Verification

```bash
docker run --rm -p 6677:6677 \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/nunchi-trade/roko:latest &
sleep 3
curl -s http://localhost:6677/api/health
curl -s http://localhost:6677/ | head -5  # Should be HTML
```

---

## P17: Create deploy.sh convenience script

**Priority**: MEDIUM
**Estimated scope**: 1 new file, ~40 lines
**Depends on**: P16

### What to do

**Create `deploy.sh` at the repo root:**

```bash
#!/bin/bash
set -euo pipefail

TARGET="${1:-railway}"
REGISTRY="ghcr.io/nunchi-trade/roko"
TAG="${2:-latest}"

echo "=== Building frontend ==="
(cd demo/demo-app && npm run build)

echo "=== Cross-compiling for Linux ==="
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko

echo "=== Building Docker image ==="
docker buildx build \
  -f Dockerfile.runtime \
  -t "$REGISTRY:$TAG" \
  --platform linux/amd64 \
  --push .

echo "=== Deploying to $TARGET ==="
case "$TARGET" in
  railway)
    railway up --image "$REGISTRY:$TAG"
    ;;
  fly)
    fly deploy --image "$REGISTRY:$TAG"
    ;;
  local)
    docker run -p 6677:6677 \
      -e ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY:-}" \
      -e OPENAI_API_KEY="${OPENAI_API_KEY:-}" \
      -e PERPLEXITY_API_KEY="${PERPLEXITY_API_KEY:-}" \
      -e GEMINI_API_KEY="${GEMINI_API_KEY:-}" \
      -v roko-state:/workspace/.roko \
      "$REGISTRY:$TAG"
    ;;
  *)
    echo "Usage: ./deploy.sh [railway|fly|local] [tag]"
    exit 1
    ;;
esac

echo "=== Done ==="
```

```bash
chmod +x deploy.sh
```

### What NOT to do

- DO NOT hardcode API keys in the script
- DO NOT `git push` or commit from the script — that's the user's responsibility

---

## P18: Add CI auto-deploy to Railway

**Priority**: MEDIUM
**Estimated scope**: ~20 lines added to existing workflow
**Depends on**: P13 or P16

### Context

The existing `.github/workflows/docker-publish.yml` already builds and pushes images to
GHCR on push to main. Adding a deploy step makes it fully automatic.

### What to do

**File: `.github/workflows/docker-publish.yml`**

Add a job after the existing build job:

```yaml
deploy-railway:
  needs: build
  if: github.ref == 'refs/heads/main'
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install Railway CLI
      run: npm i -g @railway/cli
    - name: Deploy from GHCR image
      env:
        RAILWAY_TOKEN: ${{ secrets.RAILWAY_TOKEN }}
      run: railway up --image ghcr.io/nunchi-trade/roko:latest
```

### Prerequisites

- Add `RAILWAY_TOKEN` to GitHub repo secrets (get from Railway dashboard → Account → Tokens)

### What NOT to do

- DO NOT deploy on every PR — only on main
- DO NOT put the Railway token in the workflow file — use GitHub secrets
- DO NOT make deploy a blocking step for CI — use `needs: build` but don't block other jobs

---

## Execution Order

For a single agent doing everything sequentially:

```
P1 → P2 → P3 → P4  (model routing, ~4 tasks, CRITICAL)
P6                   (auth + path traversal, CRITICAL)
P5                   (file locking, CRITICAL)
P16                  (Dockerfile.runtime for fast deploys, HIGH)
P17                  (deploy script, MEDIUM)
P13 → P14            (Dockerfile.optimized + deploy configs, MEDIUM)
P15                  (production config, MEDIUM)
P7                   (HTTP hardening, HIGH)
P8                   (error logging, HIGH)
P10                  (expect/unwrap, HIGH — do priority 1 & 2 only)
P9                   (context overflow, HIGH)
P18                  (CI auto-deploy, MEDIUM)
P12                  (eprintln, MEDIUM)
P11                  (log rotation, MEDIUM)
```

For parallel execution, these groups are independent:

- **Group A**: P1 → P2 → P3 → P4
- **Group B**: P5, P6, P7, P8
- **Group C**: P9, P10, P11, P12
- **Group D**: P13 → P14, P15
- **Group E**: P16 → P17, P18
