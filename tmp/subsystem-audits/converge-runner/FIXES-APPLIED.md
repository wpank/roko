# Converge Runner — Fixes Applied

All fixes applied on branch `wp-arch2` after merging converge commits.
These changes are currently uncommitted in the working tree.

## Pre-Merge Compile Fixes (commit `db0df777`, previous session)

Before the converge branch could be merged, `cargo check --workspace` failed.
These fixes were committed to `wp-arch2` before the merge:

1. **tokio Mutex migration**: Replaced `std::sync::Mutex` with `tokio::sync::Mutex`
   in files where the guard was held across `.await` points (required for `Send`
   bounds in async contexts).

2. **`#[path]` state_hub references**: Updated imports from `roko_core::state_hub::*`
   to `crate::state_hub::*` in roko-cli and roko-serve (the `state_hub.rs` module
   is included via `#[path]`, not exported from roko-core's `lib.rs`).

3. **Missing fields/imports**: Added fields and imports that were introduced in
   one converge batch but not propagated to all usage sites.

## Critical Audit Fixes (this session, uncommitted)

### Fix 1: block_on panic → spawn_blocking (CRIT-01)

**File**: `crates/roko-cli/src/run.rs` (~line 382)

**Before**:
```rust
fn persist(&self) -> RuntimeBoxFuture<'_, RuntimeResult<()>> {
    let inner = Arc::clone(&self.inner);
    Box::pin(async move {
        // PANICS: "Cannot start a runtime from within a runtime"
        futures::executor::block_on(async move {
            if let Ok(policy) = inner.lock() {
                let _ = policy.persist().await;
            }
        });
        Ok(())
    })
}
```

**After**:
```rust
fn persist(&self) -> RuntimeBoxFuture<'_, RuntimeResult<()>> {
    let inner = Arc::clone(&self.inner);
    Box::pin(async move {
        let result = tokio::task::spawn_blocking(move || {
            if let Ok(policy) = inner.lock() {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(policy.persist())
            } else {
                tracing::warn!("affect policy lock poisoned; skipping persist");
                Ok(())
            }
        })
        .await;
        match result {
            Ok(Err(e)) => tracing::warn!(%e, "affect policy persist failed"),
            Err(e) => tracing::warn!(%e, "affect policy persist task panicked"),
            _ => {}
        }
        Ok(())
    })
}
```

**Why**: `futures::executor::block_on` creates a new single-threaded runtime.
When called from within a tokio context (which this always is — it's a BoxFuture
polled by tokio), it panics. `spawn_blocking` moves to a blocking thread pool
where `Handle::current().block_on()` can safely bridge back to the tokio runtime.

### Fix 2: Layer violation (CRIT-02)

**File**: `crates/roko-runtime/Cargo.toml`

**Before**: `layer = 0`
**After**: `layer = 1`

**Why**: roko-runtime depends on roko-core (layer 1). A layer-0 crate cannot
depend on layer-1 crates. roko-primitives is the true layer-0 crate.

### Fix 3: Wrong PipelineInput on commit error (CRIT-03)

**File**: `crates/roko-runtime/src/effect_driver.rs`

**Before**: `git add` and `git commit` failures returned `PipelineInput::AgentFailed`.
**After**: All error paths return `PipelineInput::CommitDone { hash: "error: ..." }`.

**Why**: The state machine transitions from `Phase::Committing` only handle
`CommitDone`. An `AgentFailed` input would cause an unhandled transition, likely
resulting in the pipeline hanging or panicking.

### Fix 4: Silent stub judge gate (CRIT-04)

**File**: `crates/roko-gate/src/gate_service.rs`

**Before**: `Verdict::pass("stub-llm-judge").with_detail("LLM judge stub: always passes")`
**After**: `Verdict::fail("stub-llm-judge", "LLM judge gate not yet implemented — enable a real judge or remove from enabled_gates")`

**Why**: A stub that silently passes is dangerous — users get false confidence.
Failing explicitly forces users to either implement the judge or remove it from
their gate config.

## Test & Compilation Fixes (this session, uncommitted)

### Fix 5: Safety tests after X01 fail-closed change

**File**: `crates/roko-agent/src/safety/mod.rs`

X01 changed `SafetyLayer::with_defaults()` from using a permissive contract to a
restricted (deny-all) contract. This broke 10 unit tests that relied on default
behavior being permissive.

**Fix**: Added `permissive_layer()` test helper:
```rust
fn permissive_layer() -> SafetyLayer {
    SafetyLayer::with_defaults()
        .with_contract(AgentContract::permissive("test"))
}
```

Updated tests: `safety_layer_allows_safe_bash`, `safety_layer_no_safety_means_passthrough`,
`rate_limiter_eventually_blocks`, `taint_escalates_network_to_allow_with_confirm`,
`taint_escalates_file_write_to_allow_with_confirm`, `taint_escalates_bash_to_allow_with_confirm`,
`no_taint_means_normal_allow`, `inactive_taint_means_normal_allow`,
`safety_budget_blocks_after_limit_is_spent`, `temporal_monitor_blocks_never_pattern_in_safety_layer`.

### Fix 6: Contract integration test after X01

**File**: `crates/roko-agent/tests/contracts.rs`

Renamed `no_contract_means_permissive_default` → `no_contract_means_restricted_default`.
Updated assertions to verify restricted (deny-all) behavior:
```rust
assert!(safety.contract.allowed_tools.as_ref().is_some_and(|t| t.is_empty()));
assert!(result.is_err(), "restricted fallback should deny call");
```

### Fix 7: `--allowedTools` → `--tools` flag rename

**Files**: `crates/roko-agent/src/claude_cli_agent.rs` (line 778),
`crates/roko-agent/src/provider/claude_cli.rs` (line 274)

Converge renamed the CLI flag but tests/provider still referenced the old name.

### Fix 8: ContentHash Display vs to_hex mismatch

**File**: `crates/roko-agent/src/file_cache.rs` (line 104-106)

`entry_path()` used `format!("{key}.json")` which calls `Display::fmt` →
`short()` (8 hex chars). But `keys()` calls `ContentHash::from_hex()` which
requires 64 hex chars. Fixed to use `key.to_hex()`.

### Fix 9: Missing `request_id` field in test mock

**File**: `crates/roko-runtime/src/workflow_engine.rs` (line 595)

`ModelCallResponse` gained a `request_id: Option<String>` field from converge
but the test mock didn't include it. Added `request_id: None`.

### Fix 10: StateHub type mismatch in orchestrate.rs test

**File**: `crates/roko-cli/src/orchestrate.rs` (~line 20065)

Test was calling `state.state_hub.sender()` which returns
`roko_serve::StateHubSender`, but `set_state_hub()` expects
`roko_cli::state_hub::StateHubSender`. Fixed by creating a local hub:
```rust
let local_hub = crate::state_hub::shared_state_hub();
runner.set_state_hub(local_hub.sender());
```

### Fix 11: TUI app StateHub import

**File**: `crates/roko-cli/src/tui/app.rs` (line 3262)

Fixed `roko_core::shared_state_hub()` → `crate::state_hub::shared_state_hub()`.

### Fix 12: Clippy fixes across roko-runtime (27 warnings)

**Files**: Multiple files in `crates/roko-runtime/src/`

Fixed: `too_many_lines`, structure name repetition in methods, `map_or` on
Option, match on boolean, `expect` on Result, temporaries with significant Drop,
format string variables, empty line after doc comments, `floor_char_boundary`
MSRV incompatibility (replaced with manual helper).

### Fix 13: roko-serve build.rs missing_docs

**File**: `crates/roko-serve/build.rs`

Added `#![allow(missing_docs)]` to build script (build scripts don't need docs).

## Build Verification

After all fixes:
```
cargo check --workspace           # PASS
cargo clippy --workspace --no-deps -- -D warnings  # PASS
cargo test --workspace            # PASS (all tests green)
```
