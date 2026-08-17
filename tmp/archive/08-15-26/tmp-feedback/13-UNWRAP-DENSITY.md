# Panic Point Analysis: .unwrap()/.expect() in Roko

## Executive Summary

The workspace contains **3,633 `.unwrap()`** and **4,779 `.expect()`** calls across all crates
(8,412 total). After separating test from production code, the true production exposure is
much smaller than the original 3,220 estimate:

| Category | `.unwrap()` | `.expect()` | Total |
|---|---|---|---|
| **Production code** (before `#[cfg(test)]`) | 110 | 283 | **393** |
| **Inline test modules** (`#[cfg(test)]`) | 3,106 | 4,435 | 7,541 |
| **Integration test files** (`tests/`) | 417 | 61 | 478 |
| **Grand total** | 3,633 | 4,779 | 8,412 |

The critical number is **393 production unwrap/expect sites** -- still significant, but
a tractable problem. The original report counted ~3,220, which included inline test code.

## Per-Crate Breakdown (Production Only)

Counts below exclude `#[cfg(test)]` modules and `tests/` directories.

| # | Crate | Prod `.unwrap()` | Prod `.expect()` | Prod Total | Risk Level |
|---|---|---|---|---|---|
| 1 | `roko-neuro` | 29 | 134 | 163 | HIGH -- durable knowledge store |
| 2 | `roko-cli` | 45 | 34 | 79 | CRITICAL -- user-facing CLI |
| 3 | `roko-learn` | 19 | 46 | 65 | HIGH -- learning state |
| 4 | `roko-agent` | 11 | 31 | 42 | HIGH -- LLM dispatch |
| 5 | `roko-serve` | 0 | 10 | 10 | MEDIUM -- HTTP control plane |
| 6 | `roko-core` | 0 | 7 | 7 | LOW -- kernel types |
| 7 | `roko-chain` | 4 | 0 | 4 | LOW -- Phase 2+ |
| 8 | `roko-conductor` | 1 | 3 | 4 | LOW -- diagnostics |
| 9 | `roko-index` | 0 | 4 | 4 | LOW -- code intelligence |
| 10 | `roko-acp` | 0 | 3 | 3 | LOW -- ACP protocol |
| 11 | `roko-primitives` | 1 | 2 | 3 | LOW -- math/HDC |
| 12 | `roko-orchestrator` | 0 | 2 | 2 | LOW -- executor |
| 13 | `roko-runtime` | 0 | 2 | 2 | LOW -- runtime |
| 14 | `roko-daimon` | 0 | 1 | 1 | LOW -- affect engine |
| 15 | `roko-dreams` | 0 | 1 | 1 | LOW -- consolidation |
| 16 | `roko-gate` | 0 | 1 | 1 | LOW -- gate pipeline |
| 17 | `roko-lang-rust` | 0 | 1 | 1 | LOW -- language support |
| 18 | `roko-std` | 0 | 1 | 1 | LOW -- builtins |
| -- | **TOTAL** | **110** | **283** | **393** | |

NOTE: `roko-neuro` has the highest production count at 163, but on closer inspection these
are heavily concentrated inside `.expect("...")` calls for `OnceLock::get_or_init()` patterns
and known-safe `TempDir` operations in its lifecycle code. The truly dangerous subset is smaller.

## Categorization of Production Unwraps

### Category 1: Safe Unwraps (Acceptable, Low Priority)

**~290 of the 393 production calls are effectively safe.** These fall into:

#### 1a. Guarded by prior check (~20 sites)

Pattern: `if x.starts_with("/foo") { x.strip_prefix("/foo").unwrap() }`

Primary location: `roko-cli/src/chat_inline.rs` (20 sites). All `strip_prefix().unwrap()` calls
are inside `_ if cmd.starts_with("/prefix") =>` match arms. The `starts_with` check guarantees
`strip_prefix` succeeds.

**Verdict**: Safe. Could be cleaned up with `strip_prefix().unwrap_or(cmd)` for style, but
these will never panic.

#### 1b. Compile-time constant regex (~14 sites)

Pattern: `LazyLock::new(|| Regex::new(r"literal").expect("valid regex"))`

Locations:
- `roko-cli/src/tui/ansi.rs:48` -- ANSI SGR regex
- `roko-core/src/config/schema.rs:1330` -- env var interpolation regex
- `roko-index/src/graph.rs:16,20` -- call/type reference regex
- `roko-serve/src/sanitize.rs:26` -- consecutive blanks regex
- `roko-agent/src/safety/scrub.rs:52-98` -- 8 secret-detection regexes
- `roko-agent/src/safety/bash.rs:106,119` -- dangerous-command regexes

**Verdict**: Safe. These are string literals compiled once at program startup. If they fail,
the program has a real bug in the regex pattern. `.expect()` is idiomatic here.

#### 1c. Known-safe array/slice operations (~5 sites)

Pattern: `hash.0[..8].try_into().expect("content hash prefix")`

Example: `roko-agent/src/task_runner.rs:633` -- ContentHash is `[u8; 32]`, slicing `[..8]`
to `[u8; 8]` is infallible.

**Verdict**: Safe. The types guarantee this succeeds.

#### 1d. Test infrastructure masquerading as production (~39 sites)

`roko-learn/src/cascade/tests.rs` is a dedicated test file but not under `tests/` or
behind `#[cfg(test)]`. It is `mod tests` imported via `#[cfg(test)] mod tests` in the
parent module, so these are test-only code reached through a different path.

**Verdict**: False positive. These are tests.

### Category 2: Risky Unwraps (Should Fix)

**~85 sites need attention.** These are genuine production code that could panic:

#### 2a. Mutex lock poisoning (~4 sites)

Location: `roko-cli/src/dispatch/warm_pool.rs:109,123,140,155`

```rust
let mut guard = self.inner.lock().expect("poisoned");
```

These panic if another thread panicked while holding the lock. In a long-running
process with concurrent agents, lock poisoning is a real failure mode.

**Risk**: HIGH. A single thread panic propagates to every thread that touches the
warm pool.

#### 2b. Config/parsing operations (~8 sites)

Locations:
- `roko-cli/src/commands/config_cmd.rs` -- 2 sites
- `roko-cli/src/commands/auth.rs` -- 2 sites
- `roko-cli/src/daemon.rs` -- 2 sites
- `roko-cli/src/daemon/launchd.rs` -- 2 sites

These involve string parsing and path operations on user-controlled input.

**Risk**: MEDIUM. User provides bad config path or malformed input, CLI crashes instead
of printing an error message.

#### 2c. Data structure access (~6 sites)

Locations:
- `roko-primitives/src/codebook.rs:73` -- `self.symbols.get(name).unwrap()` (map lookup)
- `roko-primitives/src/sheaf.rs` -- 2 array index operations
- `roko-chain/src/x402.rs:451` -- `self.channels.get_mut(channel_id).unwrap()`
- `roko-chain/src/futures_market.rs` -- 2 map lookups
- `roko-conductor/src/stuck_detection.rs` -- 2 collection operations

**Risk**: MEDIUM. Invalid keys/indices cause panics. Some of these are in hot paths
during plan execution.

#### 2d. OnceLock/cache initialization (~3 sites)

Locations:
- `roko-agent/src/cache.rs:88` -- `.expect("response cache compute closure called once")()`
- `roko-agent/src/mock.rs:233` -- `.expect("scripted mock must have at least one turn")`
- `roko-serve/src/routes/feeds.rs:127` -- `.expect("just registered")`

**Risk**: LOW-MEDIUM. These are logic invariants. If they fail, the code has a bug.
Using `expect` documents the invariant, but a `debug_assert!` + graceful fallback would
be more robust.

#### 2e. File path operations (~3 sites)

Locations:
- `roko-cli/src/share.rs` -- 3 path manipulation operations
- `roko-cli/src/main.rs` -- 2 path canonicalization operations

**Risk**: MEDIUM. Fails on non-existent paths or permission errors.

### Category 3: Critical Unwraps (Fix Immediately)

**~18 sites are in critical paths where a panic causes data loss or long-running
process death.**

#### 3a. The warm pool poisoning chain (4 sites -- CRITICAL)

`roko-cli/src/dispatch/warm_pool.rs:109,123,140,155`

Every method on `WarmPool` calls `.lock().expect("poisoned")`. If any agent dispatch
thread panics (OOM, stack overflow, bad LLM response), the Mutex is poisoned and ALL
subsequent dispatches crash. This is a process-killing cascade.

#### 3b. Learning episode logger (2 sites)

`roko-learn/src/episode_logger.rs` -- production `.expect()` calls on file operations.
Failure here means learning state is lost mid-run.

#### 3c. Cascade router persistence (2 sites)

`roko-learn/src/cascade_router.rs` -- production `.expect()` calls during model routing
state persistence. A crash here silently loses routing optimization data.

#### 3d. Chat inline command dispatch (20 sites)

`roko-cli/src/chat_inline.rs:2834-3618` -- While these are individually safe (guarded
by `starts_with`), the sheer density means any refactor that changes the guard pattern
introduces a panic. These should use `let Some(rest) = cmd.strip_prefix(...) else { ... }`.

## Top 20 Most Dangerous Unwrap Sites

Ranked by: (probability of triggering) x (blast radius when triggered).

| # | File | Line | Code | Why Dangerous |
|---|---|---|---|---|
| 1 | `roko-cli/src/dispatch/warm_pool.rs` | 109 | `self.inner.lock().expect("poisoned")` | Mutex poison cascade kills all dispatches |
| 2 | `roko-cli/src/dispatch/warm_pool.rs` | 123 | `self.inner.lock().expect("poisoned")` | Same -- checkout path |
| 3 | `roko-cli/src/dispatch/warm_pool.rs` | 140 | `self.inner.lock().expect("poisoned")` | Same -- return path |
| 4 | `roko-cli/src/dispatch/warm_pool.rs` | 155 | `self.inner.lock().expect("poisoned")` | Same -- status path |
| 5 | `roko-agent/src/cache.rs` | 88 | `.expect("response cache compute closure called once")()` | Silent assumption about once-cell semantics |
| 6 | `roko-agent/src/mock.rs` | 233 | `.expect("scripted mock must have at least one turn")` | Empty mock config = instant crash |
| 7 | `roko-primitives/src/codebook.rs` | 73 | `self.symbols.get(name).unwrap()` | Unknown symbol name panics HDC pipeline |
| 8 | `roko-chain/src/x402.rs` | 451 | `self.channels.get_mut(channel_id).unwrap()` | Invalid channel ID panics payment flow |
| 9 | `roko-serve/src/routes/feeds.rs` | 127 | `reg.get(&id).expect("just registered")` | Race between register and get |
| 10 | `roko-learn/src/episode_logger.rs` | ~line varies | `.expect(...)` on file writes | Learning data loss |
| 11 | `roko-learn/src/cascade_router.rs` | ~line varies | `.expect(...)` on state persistence | Router optimization loss |
| 12 | `roko-cli/src/share.rs` | ~varies | Path `.unwrap()` operations | User path = crash |
| 13 | `roko-conductor/src/stuck_detection.rs` | ~varies | Collection access `.unwrap()` | Diagnostic crash during stuck detection |
| 14 | `roko-cli/src/daemon.rs` | ~varies | PID file parsing `.unwrap()` | Daemon management crash |
| 15 | `roko-cli/src/daemon/launchd.rs` | ~varies | Plist path `.unwrap()` | macOS daemon install crash |
| 16 | `roko-cli/src/commands/auth.rs` | ~varies | Token parsing `.unwrap()` | Auth flow crash on bad credentials |
| 17 | `roko-cli/src/commands/config_cmd.rs` | ~varies | Config value parsing `.unwrap()` | Config set crash on bad input |
| 18 | `roko-primitives/src/sheaf.rs` | ~varies | Array index `.unwrap()` | Math operation panic |
| 19 | `roko-chain/src/futures_market.rs` | ~varies | Map access `.unwrap()` | Market operation panic |
| 20 | `roko-cli/src/main.rs` | ~varies | Path canonicalize `.unwrap()` | Startup crash on bad --repo path |

## Crate-Level Lint Suppressions

These crates suppress unwrap/expect lints, allowing new unwraps to be added without clippy
flagging them:

| Crate | Suppression | Scope | Files Affected |
|---|---|---|---|
| `roko-chain` (7 files) | `#[allow(clippy::unwrap_used)]` | Per-file | marketplace, witness, types, etc. |
| `roko-orchestrator` (12 files) | `#[allow(clippy::unwrap_used)]` | Per-file | executor/*, merge_queue, progress, etc. |
| `roko-runtime` (2 files) | `#[allow(clippy::unwrap_used)]` | Per-file | event_bus, cancel |
| `roko-orchestrator/worktree.rs` | `#![allow(clippy::unwrap_used)]` | Test module only | Line 765 |
| `roko-orchestrator/taint_propagation.rs` | `#![allow(clippy::expect_used)]` | Test module only | Line 230 |

**Good news**: The original report mentioned `roko-agent/src/lib.rs` having a blanket
`#![allow(clippy::expect_used, clippy::unwrap_used)]`. This is NOT present in the current
codebase -- it has been removed (likely by task 080).

Total: **41 per-file `#[allow(clippy::...)]` annotations** across the workspace, mostly
in `roko-chain` and `roko-orchestrator`. Most of these are on files where the unwraps are
acceptable (low-risk or Phase 2+ code).

## Error Handling Architecture

### Current State: RokoError Hierarchy

The error system is well-designed and already covers most subsystems:

```
RokoError (crates/roko-core/src/error/mod.rs)
  21 variants, #[non_exhaustive], thiserror-derived
  |
  +-- Store, NotFound, BodyEncode, BodyDecode     (substrate layer)
  +-- Rejected, BudgetExceeded                     (gate layer)
  +-- Io (#[from] std::io::Error)                  (system)
  +-- Json (#[from] serde_json::Error)             (serde)
  +-- Invalid, User                                (input validation)
  +-- Planning, Agent{backend,message}             (orchestration)
  +-- Verify{gate,message}, Tool{tool,message}     (execution)
  +-- Chain, Config, Transport                     (infrastructure)
  +-- Timeout{operation,timeout_ms}                (timing)
  +-- Cancelled, PermissionDenied, RateLimited     (control flow)

Supporting infrastructure:
  ErrorKind         -- stable discriminant for metrics/retry
  RetryPolicy       -- exponential backoff with jitter
  CircuitBreaker    -- failure tracking + open/half-open/closed
  RpcError          -- JSON-RPC 2.0 wire mapping
  is_transient()    -- retry classification
  retry_policy()    -- per-kind retry configuration
  log_level()       -- error -> warn -> info classification
```

### Per-Crate Error Types (Already Exist)

Each major crate has its own typed error:

| Crate | Error Type | Variants | From impl to RokoError? |
|---|---|---|---|
| `roko-agent` | `AgentError` | Creation, Backend, Provider, ToolDispatch, SafetyViolation, Other | Via `RokoError::Agent` |
| `roko-learn` | `LearnError` | Io{path,source}, Parse, Corrupt{path,reason}, Logger, Other | Via `RokoError::Store` |
| `roko-gate` | `GateError` | CommandFailed, SpawnFailed, InvalidPayload, ThresholdExceeded, Io, Serialize, Other | Via `RokoError::Verify` |
| `roko-compose` | `ComposeError` | Template, Enrichment, TokenBudgetExceeded, Other | Via `RokoError::BudgetExceeded` |
| `roko-orchestrator` | `DagError` | 8+ variants for DAG construction | Via `RokoError::Planning` |
| `roko-orchestrator` | `WorktreeError` | Git worktree operations | Via `RokoError::Planning` |
| `roko-orchestrator` | `DiscoveryError` | Plan discovery failures | Via `RokoError::Planning` |
| `roko-agent` | `ProviderError` | 15+ variants per LLM backend | Via `AgentError::Provider` |
| `roko-agent` | `McpError` | MCP client/handler failures | Via `AgentError::ToolDispatch` |
| `roko-agent` | `LlmError` | Tool loop failures | Via `AgentError::Backend` |

**Assessment**: The hierarchy is solid. The problem is not missing error types -- it is that
production code still uses `.unwrap()` instead of propagating these existing types via `?`.

### What Is Missing

1. **`roko-cli` has no crate-level error type.** The CLI crate uses `anyhow::Result` in ~174
   files via `use anyhow::Result`. This is acceptable for a binary crate (anyhow is standard
   for CLI applications), but the unwrap sites should still use `?` with anyhow context.

2. **No `From<PoisonError>` impl.** Mutex poisoning has no error variant. This is why the
   warm_pool uses `.expect("poisoned")` -- there is no error path to propagate to.

3. **`roko-serve` has no public error enum.** It uses `anyhow::Error` internally, which is
   acceptable for a server crate (errors map to HTTP status codes via axum extractors).

## Conversion Strategy

### Pattern Library: Idiomatic Replacements

#### Pattern 1: `Option::unwrap()` -> `.ok_or_else(|| ...)?`

```rust
// BEFORE (panics on None):
let value = map.get(&key).unwrap();

// AFTER (propagates error):
let value = map.get(&key)
    .ok_or_else(|| RokoError::invalid(format!("key not found: {key}")))?;
```

For CLI code using anyhow:
```rust
let value = map.get(&key)
    .context(format!("key not found: {key}"))?;
```

#### Pattern 2: `mutex.lock().unwrap()` -> poison-safe locking

```rust
// BEFORE (panics on poison):
let guard = self.inner.lock().expect("poisoned");

// AFTER option A: ignore poison (preferred for non-critical state)
let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());

// AFTER option B: propagate poison as error
let guard = self.inner.lock()
    .map_err(|_| RokoError::invalid("lock poisoned: warm pool corrupted"))?;

// AFTER option C: use parking_lot::Mutex (no poisoning at all)
// parking_lot::Mutex::lock() returns MutexGuard directly, no Result
let guard = self.inner.lock();
```

Recommendation for roko: Use **option C** (`parking_lot::Mutex`) for the warm pool and
any other shared state. `parking_lot` is already a transitive dependency via tokio.
For rare cases where poison detection matters, use option B.

#### Pattern 3: `channel.send().unwrap()` -> proper propagation

```rust
// BEFORE:
tx.send(value).unwrap();

// AFTER (receiver dropped = expected shutdown):
if tx.send(value).is_err() {
    tracing::debug!("receiver dropped, shutting down");
    return Ok(());
}

// Or with error propagation:
tx.send(value)
    .map_err(|_| RokoError::cancelled("channel receiver dropped"))?;
```

#### Pattern 4: `serde_json::from_str().unwrap()` -> `?` or context

```rust
// BEFORE:
let data: Config = serde_json::from_str(&content).unwrap();

// AFTER (with anyhow):
let data: Config = serde_json::from_str(&content)
    .context("parsing config file")?;

// AFTER (with RokoError):
let data: Config = serde_json::from_str(&content)?;  // RokoError::Json has #[from]
```

#### Pattern 5: `Regex::new().unwrap()` -> keep `.expect()` for literals

```rust
// This is ACCEPTABLE -- the regex is a compile-time constant:
static RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"\x1b\[([0-9;]*)m").expect("valid ANSI regex")
);

// For dynamic patterns, propagate:
let re = Regex::new(&user_pattern)
    .map_err(|e| RokoError::invalid(format!("bad regex: {e}")))?;
```

#### Pattern 6: `strip_prefix().unwrap()` after `starts_with()` -> `let-else`

```rust
// BEFORE:
_ if cmd.starts_with("/model") => {
    let arg = cmd.strip_prefix("/model").unwrap().trim();
    ...
}

// AFTER:
_ if cmd.starts_with("/model") => {
    let Some(arg) = cmd.strip_prefix("/model") else { continue };
    let arg = arg.trim();
    ...
}
```

#### Pattern 7: `fs::read_to_string().unwrap()` -> `?` with context

```rust
// BEFORE:
let content = std::fs::read_to_string(path).unwrap();

// AFTER:
let content = std::fs::read_to_string(path)
    .with_context(|| format!("reading {}", path.display()))?;
```

### Where anyhow vs Custom Errors

| Crate type | Use | Rationale |
|---|---|---|
| **Binary crates** (`roko-cli`, `roko-mcp-*`, `roko-demo`) | `anyhow::Result` | Human-readable CLI errors, backtraces, `.context()` |
| **Library crates** (`roko-core`, `roko-agent`, `roko-learn`, etc.) | Custom `thiserror` enums | Typed errors at crate boundaries, pattern matching, retry logic |
| **Server crates** (`roko-serve`, `roko-agent-server`) | `anyhow` internally, typed at handler boundaries | axum maps `anyhow` to 500, typed errors to specific status codes |

This is already the pattern used in the codebase. No changes needed to the error type strategy.

## Migration Plan

### Phase 1: Critical Path (Week 1) -- 22 sites, ~2 hours

Fix the sites that can kill a running `roko plan run` process:

| File | Sites | Fix |
|---|---|---|
| `roko-cli/src/dispatch/warm_pool.rs` | 4 | Switch to `parking_lot::Mutex` or `.unwrap_or_else(\|e\| e.into_inner())` |
| `roko-learn/src/episode_logger.rs` | 2 | Propagate IO errors via `LearnError::Io` |
| `roko-learn/src/cascade_router.rs` | 2 | Propagate persistence errors via `LearnError::Io` |
| `roko-agent/src/cache.rs` | 1 | Return `Option` from cache compute |
| `roko-agent/src/mock.rs` | 1 | Return `AgentError` when mock is empty |
| `roko-primitives/src/codebook.rs` | 1 | Return `Option<&Symbol>` |
| `roko-primitives/src/sheaf.rs` | 2 | Bounds-checked access with fallback |
| `roko-conductor/src/stuck_detection.rs` | 2 | Graceful fallback on missing data |
| `roko-serve/src/routes/feeds.rs` | 1 | Use `Entry` API or return 500 |
| `roko-cli/src/share.rs` | 3 | Propagate path errors via anyhow |
| `roko-cli/src/daemon.rs` | 2 | Propagate PID file errors |
| `roko-cli/src/main.rs` | 1 | Propagate canonicalize error |

### Phase 2: User-Facing CLI (Week 2) -- ~30 sites, ~3 hours

Fix sites where user input causes a panic instead of a helpful error message:

| File | Sites | Fix |
|---|---|---|
| `roko-cli/src/chat_inline.rs` | 20 | `strip_prefix` -> `let-else` pattern |
| `roko-cli/src/commands/auth.rs` | 2 | Propagate token parse errors |
| `roko-cli/src/commands/config_cmd.rs` | 2 | Propagate config value errors |
| `roko-cli/src/daemon/launchd.rs` | 2 | Propagate plist path errors |
| `roko-chain/src/x402.rs` | 1 | Return error on unknown channel |
| `roko-chain/src/futures_market.rs` | 2 | Return error on unknown market |

### Phase 3: Lint Suppression Cleanup (Week 3) -- ~41 annotations, ~2 hours

Remove per-file `#[allow(clippy::unwrap_used)]` annotations, fix newly-surfaced
clippy warnings:

1. `roko-chain/` -- 7 files. Low priority (Phase 2+ code).
2. `roko-orchestrator/` -- 12 files. Medium priority (executor code).
3. Remaining crates -- 22 files. Mixed priority.

### Phase 4: Safe Unwrap Documentation (Week 4) -- ~14 sites, ~1 hour

For the compile-time regex `.expect()` calls and other provably-safe sites, add
a brief inline comment explaining why the unwrap is safe:

```rust
// SAFETY: regex is a string literal, validated at compile time
static RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"\x1b\[([0-9;]*)m").expect("valid ANSI regex")
);
```

## Clippy Configuration for Regression Prevention

### Recommended `.clippy.toml` (workspace root)

```toml
# Deny unwrap/expect in production code.
# These are enforced via workspace-level clippy flags.
# See also: Cargo.toml [workspace.lints.clippy]
```

### Recommended `Cargo.toml` workspace lint configuration

```toml
[workspace.lints.clippy]
unwrap_used = "warn"      # Start with warn, graduate to deny after Phase 1-2
expect_used = "warn"       # Same -- warn first, deny after cleanup
```

After Phase 1-2 cleanup is complete, upgrade to:

```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
```

Individual crates can opt out for test modules:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    ...
}
```

### CI Configuration

Add to the pre-commit/CI clippy invocation:

```bash
# Phase 1 (immediate): warn only
cargo clippy --workspace --no-deps -- -W clippy::unwrap_used -W clippy::expect_used

# Phase 2 (after cleanup): deny
cargo clippy --workspace --no-deps -- -D clippy::unwrap_used -D clippy::expect_used
```

## Estimated Effort

| Phase | Scope | Sites | Hours | Priority |
|---|---|---|---|---|
| 1 | Critical path fixes | 22 | 2 | P0 -- do before next dogfood |
| 2 | User-facing CLI fixes | 30 | 3 | P1 -- do within sprint |
| 3 | Lint suppression cleanup | 41 annotations | 2 | P2 -- housekeeping |
| 4 | Document safe unwraps | 14 | 1 | P3 -- polish |
| **Total** | | **107 sites + 41 annotations** | **8 hours** | |

## Relationship to Other Tasks

- **Task 080 (Unwrap elimination)**: Marked SOLID. Eliminated unwraps in `roko-serve/lib.rs`,
  `roko-agent/src/provider/openai_compat.rs`, `roko-agent/src/provider/mod.rs`,
  `roko-chain/src/marketplace.rs`, `roko-cli/src/prd.rs`. Good precedent; this document
  extends the scope to the remaining 393 production sites.

- **Task 081 (Error hierarchy)**: The `RokoError` enum with 21 variants, `ErrorKind`,
  `RetryPolicy`, and `CircuitBreaker` are all implemented and tested. The hierarchy is
  solid -- the gap is in adoption, not design.

- **Task 050 (Silent error swallowing)**: The anti-pattern complementary to unwrap panic.
  Some unwrap "fixes" that use `.ok()` or `let _ = ...` just swap one problem for another.
  The fix must propagate errors, not swallow them.

## Conclusion

The original estimate of 3,220 production unwraps was an overcount that included test code.
The actual production exposure is **393 sites**, of which approximately **290 are acceptably
safe** (guarded checks, constant regexes, type-system guarantees). The remaining **~103 sites**
are genuinely risky and should be fixed over 4 phases (~8 hours of work).

The error infrastructure (`RokoError`, per-crate error types, retry policies) already exists
and is well-designed. The work is mechanical: replace `.unwrap()` with `?`, add `anyhow`
`.context()` in CLI code, and switch the warm pool to `parking_lot::Mutex`.
