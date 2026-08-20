# 93 — Plugin Runtime Re-Verification and WASM Panic Boundary

**Priority**: P1 — Security gap: installed plugins are not re-verified on load, and a WASM panic can permanently poison the WASM runtime mutex for all subsequent hooks
**Size**: S (1 day)
**Crates**: `crates/roko-plugin/` (`src/manifest.rs`, `src/registry.rs`), `crates/roko-cli/` (`src/runner/extension_loader.rs`, `src/runner/wasm_extension.rs`)
**Depends on**: None

---

## Background

Roko supports WASM plugins that are installed from a registry package. During installation (`roko plugin install`) and during relay verification (`validate_resolved_registry_graph`), the package undergoes full Ed25519 signature verification plus SHA-256 checksum validation. This is correct.

However, when the runner loads plugins from disk at startup, it reads TOML manifests and WASM binaries directly without any re-verification. A plugin binary that was modified after installation — by disk corruption, accidental overwrite, or a malicious actor with filesystem access — would be loaded and executed without detection.

The second issue is a stability hazard. The WASM execution path in `wasm_extension.rs` wraps the invocation in `tokio::task::spawn_blocking`, but does not wrap `invoke_json` in `std::panic::catch_unwind`. If wasmtime panics internally (a rare but possible occurrence with malformed or corrupted modules), the panic aborts the blocking worker thread. The `Arc<Mutex<WasmRuntime>>` is then poisoned, and every subsequent WASM hook invocation for that plugin returns `"WASM runtime mutex is poisoned"` for the lifetime of the process. The only recovery is a restart.

## Current State

### Plugin loading path (no re-verification)

1. **`resolve_plugin_tool_catalog`** in `crates/roko-cli/src/runner/extension_loader.rs`, lines 1144–1223. This is the function called for each plan run. It calls `roko_plugin::manifest::discover_plugins(dir)` for three directories (`layout.extensions_dir()`, `workdir/plugins`, `workdir/.roko/plugins`) and then calls `roko_plugin::manifest::resolve_plugins(discovered)`. Neither call verifies signatures or checksums.

2. **`discover_plugins`** in `crates/roko-plugin/src/manifest.rs`, line 858. Reads `plugin.toml` from disk; returns `LoadedPlugin` structs. No checksum check.

3. **`resolve_plugins`** in `crates/roko-plugin/src/manifest.rs`, lines 693–727. Selects the highest version when multiple copies of the same plugin are found. No signature check. The comment in the original backlog item ("no signature check") is still accurate.

4. **`LoadedPlugin` struct**: In `crates/roko-plugin/src/manifest.rs`, a `LoadedPlugin` has a `base_dir: PathBuf` and a `manifest: PluginManifest`. There is no `sha256` field on `LoadedPlugin` itself; the SHA-256 checksum lives on the `RegistryPackage` struct in `crates/roko-plugin/src/registry.rs` line 74–75 (`pub sha256: String`). The plugin manifest TOML (`plugin.toml`) does not currently store the expected checksum.

### Signature/checksum verification (install-time only)

5. **`validate_signed_package`** in `crates/roko-plugin/src/registry.rs`, lines 160–239. Performs full Ed25519 + SHA-256 verification. Called during `validate_resolved_registry_graph` (line 286) and during the `install` CLI flow. Not called on startup plugin load.

6. **`package_checksum`** in `crates/roko-plugin/src/registry.rs`, line 362. Computes a SHA-256 over all package files. This is the function to reuse.

7. **`constant_time_eq`** in `crates/roko-plugin/src/registry.rs`, lines 878–886. XOR accumulation without `black_box`. Used for the checksum comparison.

### WASM execution boundary (no catch_unwind)

8. **`invoke_value`** in `crates/roko-cli/src/runner/wasm_extension.rs`, lines 263–288. The `spawn_blocking` task (line 272–277) locks the mutex and calls `invoke_json`. The `JoinError` path (line 281: `Ok(Err(error))`) catches task panics and returns an error, but by the time `JoinError` is caught, the mutex is already poisoned. The distinction is:
   - A task panic inside `spawn_blocking` propagates as a `JoinError` when awaited.
   - But before the `JoinError` reaches the caller, the mutex is poisoned because the lock guard was held across the panic.
   - All subsequent calls to `invoke_value` fail at `runtime.lock().map_err(|_| "WASM runtime mutex is poisoned")` (line 274–275).

9. **`invoke_json`** in `crates/roko-cli/src/runner/wasm_extension.rs`, lines 75–139. The actual wasmtime invocation happens here. Wasmtime fuel traps and memory traps return `Err(...)`, not panics, under normal conditions. A panic here would be from internal wasmtime logic on corrupted state.

## Implementation Plan

### Part A: Checksum verification on plugin load

The cleanest approach given the current data model is to store the expected SHA-256 of the WASM binary in the `plugin.toml` manifest during installation, then verify it at load time. This requires two changes: write the checksum during install, and verify it during discover/load.

#### A1. Add `wasm_sha256` field to the plugin manifest

In `crates/roko-plugin/src/manifest.rs`, find the `PluginManifest` struct (search for `struct PluginManifest`). Add an optional field:
```rust
/// SHA-256 hex digest of the WASM binary, written during installation.
/// Verified at startup to detect tampering or corruption.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub wasm_sha256: Option<String>,
```

This is optional so that hand-authored `plugin.toml` files (e.g. in `plugins/` or `.roko/extensions/`) continue to work without a checksum field.

#### A2. Write the checksum during installation

In the install flow (find where `plugin.toml` is written to `.roko/plugins/`), after writing the WASM binary:
```rust
let wasm_bytes = std::fs::read(&wasm_path)?;
let actual_sha256 = sha256_hex(&wasm_bytes);
manifest.wasm_sha256 = Some(actual_sha256);
// Then serialize manifest to TOML and write plugin.toml.
```

For the SHA-256 computation, use the same approach as `package_checksum` in `registry.rs`. You can factor it out into a small helper:
```rust
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}
```

(The `sha2` and `hex` crates are already in the workspace.)

#### A3. Verify the checksum in `discover_plugins`

In `crates/roko-plugin/src/manifest.rs`, inside `discover_plugins` (line 858), after loading a `plugin.toml` and identifying the WASM module path:

```rust
if let Some(ref expected_sha256) = loaded.manifest.wasm_sha256 {
    match std::fs::read(&wasm_path) {
        Ok(wasm_bytes) => {
            let actual = sha256_hex(&wasm_bytes);
            if !constant_time_eq(actual.as_bytes(), expected_sha256.as_bytes()) {
                tracing::error!(
                    plugin = %name,
                    path = %wasm_path.display(),
                    "WASM checksum mismatch — plugin may be corrupted or tampered; skipping"
                );
                continue; // skip this plugin
            }
        }
        Err(err) => {
            tracing::error!(
                plugin = %name,
                path = %wasm_path.display(),
                error = %err,
                "cannot read WASM binary for checksum verification; skipping"
            );
            continue;
        }
    }
}
```

Move `constant_time_eq` from `registry.rs` to a shared location within `roko-plugin` (e.g. a `crypto` submodule or directly in `manifest.rs`) so both modules can use it without duplicating it.

### Part B: catch_unwind around WASM invocation

In `crates/roko-cli/src/runner/wasm_extension.rs`, in the `invoke_value` method (line 263), wrap the `invoke_json` call:

```rust
let task = tokio::task::spawn_blocking(move || {
    let mut guard = runtime
        .lock()
        .map_err(|_| "WASM runtime mutex is poisoned".to_string())?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        guard.invoke_json(export, input)
    }))
    .map_err(|_| "WASM hook panicked during execution".to_string())?
});
```

The key difference from the original code is that `catch_unwind` prevents the panic from propagating out of the closure, so the mutex guard is dropped cleanly before the `Err` is returned. The mutex is never poisoned.

The `AssertUnwindSafe` wrapper is required because `guard` contains non-`UnwindSafe` types (the wasmtime `Store`). This is safe here because we are mapping any panic to an `Err` return and not retaining any reference to `guard` after the catch — the `guard` is dropped at the end of the `catch_unwind` closure.

## Acceptance Criteria

1. A plugin whose WASM binary is modified on disk after installation is rejected at startup with an error log and skipped (does not crash the runner).
2. A plugin without a `wasm_sha256` field in its manifest (hand-authored or pre-installation) is loaded normally (backward compatible).
3. A WASM module that causes `invoke_json` to panic returns an `Err` to the caller and does not poison the mutex for subsequent calls.
4. All existing plugin manifest and registry tests pass.
5. New test: write a plugin with a known checksum, modify the WASM bytes, call `discover_plugins` — the plugin must not appear in the results.
6. New test: a wasmtime invocation that panics (simulate with a closure that panics inside `catch_unwind`) returns `Err` and allows a subsequent call to succeed.

## Verification Checklist

- [ ] `cargo test -p roko-plugin` passes
- [ ] `cargo test -p roko-cli -- extension` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes (check `AssertUnwindSafe` usage)
- [ ] `grep -rn "constant_time_eq" crates/roko-plugin/` shows one definition, not two
- [ ] Install a plugin via `roko plugin install`, then corrupt the `.wasm` file, then `roko plan run` — confirm the plugin is logged as rejected

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-plugin/src/manifest.rs` | Add `wasm_sha256: Option<String>` to `PluginManifest`; add checksum verification in `discover_plugins`; add `sha256_hex` helper |
| `crates/roko-plugin/src/registry.rs` | Move `constant_time_eq` to a shared location if needed; write `wasm_sha256` to manifest during install |
| `crates/roko-cli/src/runner/wasm_extension.rs` | Wrap `invoke_json` in `std::panic::catch_unwind(AssertUnwindSafe(...))` |
