# Perf Hot-Path Fixes

**Priority**: P2
**Size**: M (2-3 days)

---

## Problem

Four performance anti-patterns add unnecessary latency to every agent dispatch.
Each issue was verified against the actual codebase and is confirmed present.

---

### Issue 1: Sync I/O on Tokio thread in `PromptAssemblyService`

`crates/roko-compose/src/prompt_assembly_service.rs`

`PromptAssemblyService::assemble` is an `async fn` (line 357) that calls two
sync blocking helpers without wrapping them in `tokio::task::spawn_blocking`:

- `collect_source_context_from` (line 741) calls `std::fs::read_dir` (line 752)
  in a recursive loop up to depth 5 and 500 files.
- `read_to_string_if_exists` (line 783) calls `std::fs::read_to_string` (line
  784) for up to 12 source files.

Both are called on the Tokio event-loop thread from within `assemble` via:

- `conventions_for_spec` → `detect_workdir_conventions` → `collect_source_context`
  (line 373 in `assemble`, line 711 in `detect_workdir_conventions`)
- `workspace_map_for_spec` → `collect_source_context` (line 433 in `assemble`,
  line 571 in `workspace_map_for_spec`)

There is no `spawn_blocking` or `tokio::fs` anywhere in the file. On a large
workspace this can block the thread for tens to hundreds of milliseconds.

**Fix**: Wrap `collect_source_context` and `read_to_string_if_exists` calls in
`tokio::task::spawn_blocking` within `assemble`, or convert to `tokio::fs`
equivalents. The simplest path is to move the two sync helpers into a
`tokio::task::spawn_blocking` closure called from the two `assemble` call sites
at lines 373 and 433.

---

### Issue 2: CascadeRouter loaded from disk on every TurnCompleted event

`crates/roko-serve/src/dispatch.rs`

`AppState` holds a cached `cascade_router: RwLock<Option<CascadeRouter>>` and
the `record_template_dispatch_feedback` function (line 1991) correctly checks
this cache when `learn_dir == global_learn_dir` (line 2021–2044). However, two
separate call sites bypass the cache entirely:

**Call site A** — `drain_dispatch_learning_events` (line 2547) fires on every
`AgentEvent::TurnCompleted` (line 2559) and calls
`record_cascade_router_outcome_with_layout` (line 2560), which always calls
`record_cascade_router_observation_at` (line 2668), which always calls
`CascadeRouter::load_or_new` (line 2693). There is no check against the
in-memory cached router here; the cache is ignored unconditionally.

**Call site B** — In `record_template_dispatch_feedback`, when `learn_dir !=
global_learn_dir` (per-repo worktree dispatch), the code falls through to
`ModelCallFeedbackRecorder::from_learn_dir` (line 2048). This constructs the
recorder by calling `CascadeRouter::load_or_new` (in
`crates/roko-learn/src/model_call_feedback.rs`, line 81) before any I/O is
needed.

**Fix**: In `record_cascade_router_outcome_with_layout`, check
`state.cascade_router` first (same pattern as lines 2021–2041 in
`record_template_dispatch_feedback`). When `path == state.layout.cascade_router_path()`,
acquire the read lock, call `observe_model_call_on_router` on the in-memory
router, and save it — exactly matching the existing cache path. Only fall
through to `load_or_new` when the path is repo-specific. For per-repo paths,
consider a bounded LRU of per-repo routers rather than loading from disk each
time.

---

### Issue 3: `RokoConfig` cloned on every model call

`crates/roko-agent/src/model_call_service.rs`

`ModelCallService::call` (line 2076) calls `config_for_model` (line 2278) on
every invocation. `config_for_model` is defined at line 471–473 as:

```rust
fn config_for_model(&self, _model: &str) -> RokoConfig {
    self.config.clone()
}
```

`RokoConfig` (defined in `crates/roko-core/src/config/schema.rs` at line 89) is
a 30+ field struct containing:
- `providers: IndexMap<String, ProviderConfig>` — one entry per configured
  provider
- `models: IndexMap<String, ModelProfile>` — one entry per model profile
- `profiles: HashMap<String, DomainProfile>`
- `subscriptions: Vec<SubscriptionConfig>`
- `agents: Vec<AgentDefinition>`
- `groups: Vec<GroupDefinition>`

The clone allocates new heap storage for all of these on every model call. The
returned config is passed to `ProviderCallCell::new` (line 2297) and only used
to resolve provider configuration. The model does not change between calls for a
given service instance, so the clone is unnecessary.

**Fix**: Change `config` in `ModelCallService` to `Arc<RokoConfig>` and update
`config_for_model` to return `Arc<RokoConfig>`. Update `ProviderCallCell::new`
to accept `Arc<RokoConfig>`. This makes the "clone" a single atomic increment
instead of deep-copying all the maps and vecs.

---

### Issue 4: Unconditional `git diff` subprocess after every agent completion

`crates/roko-runtime/src/effect_driver.rs`

`count_changed_files` (line 708) is called unconditionally after every
successful agent response (line 262):

```rust
let files_changed = count_changed_files(&self.workdir).await;
```

The function (lines 708–723) forks a `git diff --name-only HEAD` subprocess on
every call, regardless of whether the caller will actually use the count or
whether the working directory is even a git repo. For effects that run
frequently (e.g. streaming turns or high-frequency subscriptions), this is one
`git` subprocess per completion.

**Fix**: Add a fast path: check whether `.git` exists in `self.workdir` before
spawning the subprocess. If absent, return 0 immediately. Optionally, cache the
result for a short TTL (1–2 seconds) to absorb burst completions. The result is
described in the doc comment as "best-effort enrichment, not a gate" so
occasional staleness is acceptable.

---

## What already exists

| Component | File | Status |
|---|---|---|
| `AppState.cascade_router` RwLock | `dispatch.rs` | Exists — used in `record_template_dispatch_feedback` |
| Cache path in `record_template_dispatch_feedback` | `dispatch.rs:2021–2044` | Exists — not extended to `drain_dispatch_learning_events` |
| `tokio::task::spawn_blocking` (available) | runtime | Available — not used in `prompt_assembly_service.rs` |
| `Arc<RokoConfig>` pattern | elsewhere | Used in `AppState.config`; not plumbed into `ModelCallService` |

---

## Acceptance criteria

1. `assemble` in `PromptAssemblyService` does not call any `std::fs` function
   directly on the async thread — all blocking I/O is wrapped in
   `spawn_blocking`.
2. `record_cascade_router_outcome_with_layout` reads from `AppState.cascade_router`
   for the global learn path instead of loading from disk.
3. `ModelCallService` stores `Arc<RokoConfig>` and `config_for_model` returns
   `Arc<RokoConfig>` without deep-cloning.
4. `count_changed_files` returns 0 immediately when no `.git` directory is
   present, without forking a subprocess.
5. `cargo test --workspace` passes with zero failures.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## References

- `crates/roko-compose/src/prompt_assembly_service.rs` — sync I/O at lines 752, 784; async call sites at lines 373, 433
- `crates/roko-serve/src/dispatch.rs` — `drain_dispatch_learning_events` (line 2547), `record_cascade_router_observation_at` (line 2682), cached path (line 2021)
- `crates/roko-learn/src/model_call_feedback.rs` — `ModelCallFeedbackRecorder::from_learn_dir` calls `load_or_new` (line 81)
- `crates/roko-agent/src/model_call_service.rs` — `config_for_model` (line 471), call site in `call` (line 2278)
- `crates/roko-core/src/config/schema.rs` — `RokoConfig` struct (line 89), 30+ fields
- `crates/roko-runtime/src/effect_driver.rs` — `count_changed_files` (line 708), call site (line 262)
