# M019 — Fix routing context derivation in gateway.rs

## Objective
The `select_model_via_router()` function in gateway.rs builds a `RoutingContext` with hardcoded defaults (`crate_familiarity: 0.5`, `has_prior_failure: false`, `conductor_load: 0.0`, etc.) instead of deriving values from request metadata and runtime state. Fix these to use actual runtime data where available, producing more accurate routing decisions.

## Scope
- Crates: `roko-serve`
- Files:
  - `crates/roko-serve/src/routes/gateway.rs` (lines ~799-837, `select_model_via_router`)
  - `crates/roko-serve/src/state.rs` (AppState — may hold conductor/agent state)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.2
- Audit ref: `tmp/roko-trustworthy/AUDIT.md` §B4

## Steps
1. Read the current hardcoded RoutingContext construction:
   ```bash
   grep -n -A 20 'RoutingContext {' crates/roko-serve/src/routes/gateway.rs
   ```

2. Examine what AppState already tracks:
   ```bash
   grep -n 'pub struct AppState' crates/roko-serve/src/state.rs
   grep -n 'agent\|conductor\|load\|failure\|familiarity' crates/roko-serve/src/state.rs
   ```

3. Check the RoutingContext struct to understand all fields:
   ```bash
   grep -n -A 30 'pub struct RoutingContext' crates/roko-learn/src/model_router.rs
   ```

4. For each hardcoded field, determine if runtime data is available:

   | Field | Current | Fix |
   |---|---|---|
   | `crate_familiarity` | `0.5` | If request includes a `crate_name` hint, look up familiarity from episode history; else keep 0.5 |
   | `has_prior_failure` | `false` | Check recent gate results in AppState (if tracked) or request `iteration` > 1 |
   | `conductor_load` | `0.0` | Read from conductor metrics if available in AppState |
   | `active_agents` | `0` | Count active agents from operations map |
   | `ready_queue_depth` | `0` | Read from task queue if available |
   | `max_queue_wait_hours` | `0.0` | Compute from queue timestamps if available |

5. Extend `RoutingHints` to accept optional fields the caller can provide:
   ```rust
   pub struct RoutingHints {
       // existing:
       pub task_category: Option<String>,
       pub complexity: Option<String>,
       pub role: Option<String>,
       pub iteration: Option<u32>,
       // new:
       pub crate_name: Option<String>,
       pub has_prior_failure: Option<bool>,
   }
   ```

6. Update the `CompletionRequest` body to accept these optional hints (backward-compatible with `#[serde(default)]`).

7. In `select_model_via_router`, derive what you can from AppState:
   ```rust
   let active_agents = state.operations.read().await.len() as u32;
   let has_prior_failure = hints.has_prior_failure.unwrap_or(hints.iteration.unwrap_or(1) > 1);
   ```

8. Add a test that verifies different request metadata produces different routing contexts.

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- gateway
# Confirm hardcoded values are reduced:
grep -c '0\.5\|false\|0\.0' crates/roko-serve/src/routes/gateway.rs
```

## What NOT to do
- Do NOT change the RoutingContext struct in roko-learn — only change how it's populated
- Do NOT make any of the new request fields required — all must have sensible defaults
- Do NOT add expensive queries (DB lookups, file reads) in the hot path — use cached state from AppState
- Do NOT break the existing `/inference/complete` API — new fields are optional
