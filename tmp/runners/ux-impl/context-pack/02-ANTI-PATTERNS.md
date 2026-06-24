# Anti-Patterns To Avoid

The runner greps for these after every batch. A hit fails the batch and
the worktree is reset. Prevent them at write time.

## AP-1: Stubs that silently pass

**BAD**:
```rust
async fn list_pheromones(state: State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "items": [], "total": 0 }))   // looks fine, returns nothing
}
```

**GOOD**:
```rust
async fn list_pheromones(state: State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let samples = state.pheromone_field.list_all();
    Ok(Json(serialize_pheromone_response(samples)))
}
```

If the implementation isn't ready: `anyhow::bail!("pheromone backend not configured")`,
not silent empty.

## AP-2: `block_on` in async

**BAD**: `futures::executor::block_on(other_async()).await` — panics
under tokio.

**GOOD**: `.await` directly, or `tokio::task::spawn_blocking` for sync
work.

## AP-3: Duplicate trait definitions

**BAD**: Defining `pub trait FeedbackSink` in `roko-runtime` when it
already lives in `roko-core/src/foundation.rs`.

**GOOD**: `use roko_core::foundation::FeedbackSink;`

## AP-4: Computed but unused values

**BAD**:
```rust
let modulation = policy.modulate(&context);   // never used
make_request().await
```

**GOOD**: apply the result, or don't compute it.

## AP-5: Shelling out to `claude` / `codex`

**BAD**: `Command::new("claude").arg("--prompt").arg(p).output()` for
runtime inference.

**GOOD**: dispatch via `roko_agent::dispatcher::Dispatcher`.

## AP-6: Inline prompt strings

**BAD**: `format!("You are a helpful research agent...")` inside a
hot path.

**GOOD**: `PromptAssemblyService::compose_for(role, input)`.

## AP-7: `std::sync::Mutex` across `.await`

**BAD**:
```rust
let g = std_mutex.lock().unwrap();
do_async().await;          // guard still held, deadlocks
```

**GOOD**: use `tokio::sync::Mutex`, or drop the guard before `.await`.

## AP-8: Debug strings as event contracts

**BAD**: `writeln!(file, "{event:?}")` then parsing back via regex.

**GOOD**: `serde_json::to_string(&event)` with `Serialize` /
`Deserialize` derives.

## AP-9: Empty-string placeholders in events

**BAD**:
```rust
RuntimeEvent::AgentSpawned { agent_id: String::new(), model: String::new() }
```

**GOOD**: use `Option<String>`, or fill with the real value.

## AP-10: Success variants carrying error state

**BAD**:
```rust
CommitDone { hash: format!("error: {e}") }
```

**GOOD**: `Err(...)` or a typed enum with a `Failed` variant.

## Wave-specific anti-patterns

### Wave M (mirage extraction)

- **Don't keep a "just in case" feature gate.** Once `chain` is
  removed, no `dashboard-api` should remain. Half-deletion creates
  confusing dead branches.
- **Don't move deleted code to a sibling crate "for archiving".** The
  git history is the archive.
- **Don't preserve `chain_*` JSON-RPC methods.** They were bolted-on
  application state, not simulator features.

### Wave AG (aggregator backends)

- **Don't reach into mirage-rs from roko-serve to read pheromone
  state.** Mirage is being deleted; the import becomes a deletion
  blocker.
- **Don't make `/pheromones/decay` a public POST.** Bearer-auth it.
- **Don't unify knowledge and pheromone storage into one struct.**
  Knowledge → chain; pheromones → roko-serve in this phase.
- **Don't run a 1 s decay tick.** 60 s matches mirage and the UI.
- **Don't skip the captured fixtures.** Compat is the whole job.

### Wave CH (chain discovery)

- **Don't trust the capability bitmask alone.** The `"roko"` domain tag
  is a second filter (defence-in-depth).
- **Don't fetch every Agent Card on every aggregator request.** Cache
  with TTL 30 s + `AgentCardUpdated` event invalidation.
- **Don't repurpose reserved bits in the 64-bit capability mask.**
- **Don't poll `eth_getLogs` on every cache miss.** WS subscribe at
  startup; polling is the fallback only.

### Wave TU (TUI event parity)

- **Don't add new polling alongside the new tailers.** The whole point
  is to delete `file_stamp` calls in `dashboard.rs::tick()`.
- **Don't unbound the in-memory tailer Vec.** Cap to ~10 000 entries;
  drop oldest.
- **Don't run the WS client on the render thread.** Spawn a `tokio`
  task; forward via `tokio::sync::mpsc`.
- **Don't read `.roko/state/dashboard-gen.json` on every render.**
  Read once at startup; write on increments.

### Wave BP (backend parity)

- **Don't write parity tests with `tokio::time::sleep` synchronisation.**
  Use `mockito` deterministic responses or `tokio::time::pause`.
- **Don't hit real provider endpoints in default tests.** Use
  `mockito` fakes; reserve a separate `--ignored` smoke layer for real
  calls.
- **Don't change the wire format of `cascade-router.json`** without
  shipping a migration shim.

### Wave HY (hygiene)

- **Don't bless snapshot diffs without reading them.** That defeats
  the test.
- **Don't add `# Panics` that says "Panics if input is invalid"**
  without specifying *what* counts as invalid.
- **Don't fix flakes with retry loops.** A flaky test with retry is a
  silenced bug.
