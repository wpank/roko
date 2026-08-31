# 58 — Performance Hot-Path Fixes

> **Status: SOURCE-IMPLEMENTED / FINAL VERIFICATION PENDING** (2026-08-31,
> `88c724744`). All four hot-path changes are present in the dev-audit integration. The later
> cache lane (`97f897200` + `8c82c5b1b`) keeps cleanup off dispatch and retains warm targets unless
> explicitly pruned; the fixed-SHA runner (`d1b94b139`) can measure the result. This docs lane did
> not compile, test, or run the scorecard; the coordinator's final batch and real repetitions remain
> the verification authority.

**Priority**: P2 — four concrete latency regressions, each verified in production code
**Size**: M (2-3 days)
**Crates**: `crates/roko-compose/`, `crates/roko-serve/`, `crates/roko-agent/`, `crates/roko-runtime/`
**Depends on**: None

---

## Background

Roko runs agents in an async Tokio runtime. Every agent dispatch starts by assembling a prompt, routing to a model, spawning an agent, and recording the result. Four performance anti-patterns have been confirmed in the current code that add latency to every dispatch.

The first two are async-correctness issues: synchronous blocking I/O on a Tokio event-loop thread stalls the entire executor, and a disk-backed model router is loaded from disk unconditionally instead of using an existing in-memory cache. The third is an unnecessary heap allocation: the entire `RokoConfig` struct (which contains multiple maps and vecs) is deep-cloned on every LLM call when a reference-counted pointer would serve. The fourth spawns a `git` subprocess unconditionally after every agent completion even in directories with no git repository.

All four issues have been manually verified against the source code. None require design changes; they are local fixes.

## Audited baseline (historical)

1. `PromptAssemblyService::assemble` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs:357` is `async fn` but calls two synchronous filesystem helpers directly on the Tokio thread:
   - `collect_source_context_from` at line 741 calls `std::fs::read_dir` (line 752) in a recursive loop up to depth 5, scanning up to 500 files.
   - `read_to_string_if_exists` at line 783-784 calls `std::fs::read_to_string`.
   - These are reachable via: `assemble` (line 373) → `conventions_for_spec` (line 562) → `detect_workdir_conventions` (line 709) → `collect_source_context` (line 728); and `assemble` (line 433) → `workspace_map_for_spec` (line 569) → `collect_source_context` (line 571).
   - There is no `spawn_blocking` or `tokio::fs` usage anywhere in the file.

2. `crates/roko-serve/src/dispatch.rs` has an in-memory `AppState.cascade_router: RwLock<Option<CascadeRouter>>` cache that is correctly used by `record_template_dispatch_feedback` (line 2022) but bypassed by the main learning path:
   - `drain_dispatch_learning_events` (line 2547) fires on every `AgentEvent::TurnCompleted` (line 2559) and calls `record_cascade_router_outcome_with_layout` (line 2560).
   - `record_cascade_router_outcome_with_layout` (line 2639) → `record_cascade_router_observation_at` (line 2682) → `CascadeRouter::load_or_new` (line 2693) — which loads and parses the router from disk every time, ignoring the cache entirely.
   - The cache is initialized at line 3214 but never consulted in this code path.

3. `ModelCallService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs:81` stores `config: RokoConfig` (line 85). `config_for_model` at line 471-473 returns `self.config.clone()` on every call. `call` at line 2278 invokes it on every LLM request. `RokoConfig` (in `crates/roko-core/src/config/schema.rs`) is a deep struct with multiple `IndexMap`, `HashMap`, and `Vec` fields. The clone is passed to `ProviderCallCell::new` at line 2297, which also stores `config: RokoConfig` (line 1804). `ProviderCallCell` is constructed fresh on every call, so the clone exists only for that call's duration.

4. `count_changed_files` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs:708` is called unconditionally from line 262 after every successful agent completion. It always forks a `tokio::process::Command::new("git")` subprocess (line 709-710), even when the `workdir` is not a git repository. The doc comment on line 706-707 says this is "best-effort enrichment, not a gate," so occasional staleness is acceptable.

## Implementation Plan

**Fix 1: Wrap blocking I/O in `spawn_blocking` in `PromptAssemblyService`**

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`

The two call sites that invoke sync filesystem work inside `assemble` are at lines 373 and 433. The cleanest fix is to extract the sync work into closures passed to `tokio::task::spawn_blocking`:

```rust
// At line 373 (inside assemble), change:
let conventions = conventions_for_spec(&spec, self.default_conventions.as_deref());

// To:
let spec_clone = spec.clone();
let default_conv = self.default_conventions.clone();
let conventions = tokio::task::spawn_blocking(move || {
    conventions_for_spec(&spec_clone, default_conv.as_deref())
})
.await
.unwrap_or(None);
```

Apply the same pattern at line 433 for `workspace_map_for_spec`. If `spec` does not implement `Clone`, extract only the fields needed by each helper before the `spawn_blocking` call.

Alternatively, convert `collect_source_context_from` and `read_to_string_if_exists` to use `tokio::fs::read_dir` and `tokio::fs::read_to_string` and make the two helpers async — but this requires making `conventions_for_spec` and `workspace_map_for_spec` async as well, which is a larger change.

**Fix 2: Use the in-memory cascade router cache in `drain_dispatch_learning_events`**

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/dispatch.rs`

In `record_cascade_router_outcome_with_layout` (line 2639), add a check that mirrors the existing pattern in `record_template_dispatch_feedback` (lines 2022-2046):

```rust
async fn record_cascade_router_outcome_with_layout(
    state: &AppState,
    template: &DispatchTemplate,
    success: bool,
    layout: Option<&RokoLayout>,
) -> Result<()> {
    let path = layout
        .map(RokoLayout::cascade_router_path)
        .unwrap_or_else(|| RokoLayout::for_project(&state.workdir).cascade_router_path());

    // Use the in-memory cache when the path matches the global learn dir.
    if path == state.layout.cascade_router_path() {
        let mut router_guard = state.cascade_router.write().await;
        if let Some(ref mut router) = *router_guard {
            let model_slugs: Vec<String> = /* same derivation as below */;
            if router.record_confidence_outcome(&template.model, success) {
                router.save(&path)?;
            }
            return Ok(());
        }
    }

    // Fallback: per-repo path or cache not populated.
    record_cascade_router_observation_at(&path, model_slugs, &template.model, success)?;
    Ok(())
}
```

The `model_slugs` derivation should match whatever `record_cascade_router_observation_at` currently passes at line 2693. Check the call site for the exact argument.

**Fix 3: Store `Arc<RokoConfig>` in `ModelCallService` and `ProviderCallCell`**

Files:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
- Any types that call `ModelCallService::new` or `with_config`

Change the `config` field in `ModelCallService` (line 85) from `RokoConfig` to `Arc<RokoConfig>`. Update `with_config` (line 190) to accept `Arc<RokoConfig>` or wrap the value. Change `config_for_model` (line 471) to return `Arc<RokoConfig>` instead of `RokoConfig`. Change `ProviderCallCell.config` (line 1804) from `RokoConfig` to `Arc<RokoConfig>` and update `ProviderCallCell::new` (line 1818-1819) accordingly.

All callers of `with_config` that currently pass `config.clone()` (a deep copy) will instead pass `Arc::clone(&config)` (a pointer increment).

Search for all callers:
```bash
grep -rn "with_config\|ModelCallService::new" crates/roko-agent/src/ --include='*.rs'
grep -rn "ModelCallService" crates/ --include='*.rs' | grep -v target/ | grep -v ".rs:pub struct"
```

Update each caller to wrap with `Arc::new(config)` or `Arc::clone(&config)` as appropriate.

**Fix 4: Short-circuit `count_changed_files` when `.git` is absent**

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`

At the top of `count_changed_files` (line 708), add a fast path:

```rust
async fn count_changed_files(workdir: &std::path::Path) -> u32 {
    // Fast path: skip subprocess if workdir is not a git repository.
    if !workdir.join(".git").exists() {
        return 0;
    }

    let result = tokio::process::Command::new("git")
        // ... existing code unchanged ...
```

The `.git` existence check is synchronous but is a single `stat` syscall and is negligible compared to forking a subprocess.

## Acceptance Criteria

- [x] `PromptAssemblyService::assemble` moves selected synchronous workspace collection to
      `tokio::task::spawn_blocking`.
- [x] `record_cascade_router_outcome_with_layout` reads the in-memory `AppState.cascade_router`
      for the global learn path and falls back to disk for per-repo paths.
- [x] `ModelCallService.config` and `ProviderCallCell.config` are `Arc<RokoConfig>` and
      `config_for_model` pointer-clones the `Arc`.
- [x] `count_changed_files` returns `0` before forking Git when `workdir/.git` is absent.
- [ ] `cargo test --workspace` passes with zero failures — final batch pending.
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean — final batch pending.

## Verification Checklist

- [x] Move blocking prompt workspace/context collection off the Tokio worker.
- [x] Bound/cache workspace-map collection in the same off-thread prompt-context path.
- [x] Route global cascade outcomes through the in-memory router; per-repo paths retain disk fallback.
- [ ] Confirm `record_template_dispatch_feedback` still works correctly (existing code path, not changed)
- [x] Change `ModelCallService.config` and `ProviderCallCell.config` to `Arc<RokoConfig>`.
- [ ] Run `cargo test -p roko-agent` and confirm no regressions — final batch pending.
- [ ] Add/run a subprocess-observing non-Git fixture for `count_changed_files`; the source fast path
      is present, but the original checklist asked for runtime proof.
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-compose/src/prompt_assembly_service.rs` | Wrap `conventions_for_spec` and `workspace_map_for_spec` calls in `tokio::task::spawn_blocking` |
| `crates/roko-serve/src/dispatch.rs` | Use `AppState.cascade_router` cache in `record_cascade_router_outcome_with_layout` |
| `crates/roko-agent/src/model_call_service.rs` | Change `config: RokoConfig` to `config: Arc<RokoConfig>`; update `config_for_model`, `ProviderCallCell` |
| `crates/roko-runtime/src/effect_driver.rs` | Add `.git` existence check at top of `count_changed_files` |
| Callers of `ModelCallService::with_config` | Wrap config in `Arc::new()` or `Arc::clone()` as needed |
