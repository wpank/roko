# PAI_03: Add plugin panic boundary

## Task
Wrap plugin execution in `catch_unwind` to prevent a panicking plugin from crashing the host runtime.

## Runner Context
Runner PAI (Config & Infrastructure), batch 3 of 4. No dependencies.

## Problem
CI-3 safety gap: Plugins run via `tokio::spawn()` (roko-plugin/src/lib.rs:746,817,875,950) with no panic boundary. A panicking plugin will crash the tokio task and potentially the runtime. There is no `catch_unwind` or `AssertUnwindSafe` in roko-plugin.

The capability tier system (`PluginTier` in roko-agent/src/safety/capabilities.rs:16) gates tool access but doesn't provide process isolation.

## Exact Changes

### Step 1: Wrap plugin execution in catch_unwind

At each `tokio::spawn` site in `roko-plugin/src/lib.rs`:

```rust
// BEFORE (at ~L746):
tokio::spawn(async move {
    plugin.execute(args).await
});

// AFTER:
tokio::spawn(async move {
    match tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // If execute is sync:
            plugin.execute(args)
        }))
    }).await {
        Ok(Ok(result)) => result,
        Ok(Err(panic_info)) => {
            error!(
                plugin = %plugin_name,
                "plugin panicked — isolating: {:?}",
                panic_info
            );
            Err(PluginError::Panicked(format!("{:?}", panic_info)))
        }
        Err(join_err) => {
            error!(plugin = %plugin_name, %join_err, "plugin task failed");
            Err(PluginError::TaskFailed(join_err.to_string()))
        }
    }
});
```

For async plugin execution:

```rust
// For async plugins, catch_unwind on the Future:
tokio::spawn(async move {
    let result = std::panic::AssertUnwindSafe(plugin.execute(args))
        .catch_unwind()
        .await;
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(PluginError::ExecutionFailed(err.to_string())),
        Err(panic_info) => {
            error!(plugin = %plugin_name, "plugin panicked: {:?}", panic_info);
            Err(PluginError::Panicked(format!("{:?}", panic_info)))
        }
    }
});
```

### Step 2: Add PluginError::Panicked variant

```rust
pub enum PluginError {
    // ... existing variants ...
    Panicked(String),
    TaskFailed(String),
}
```

### Step 3: Quarantine panicking plugins

After a panic, mark the plugin as quarantined to prevent re-execution:

```rust
if matches!(&result, Err(PluginError::Panicked(_))) {
    plugin_registry.quarantine(&plugin_name);
    warn!(plugin = %plugin_name, "plugin quarantined after panic — will not run again this session");
}
```

## Write Scope
- `crates/roko-plugin/src/lib.rs` (wrap spawn sites in catch_unwind)
- `crates/roko-plugin/src/` (PluginError variant, quarantine support)

## Read-Only Context
- `crates/roko-agent/src/safety/capabilities.rs` (PluginTier — reference only)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Panicking plugin caught by `catch_unwind` (no runtime crash)
- Error logged with plugin name
- Panicking plugin quarantined for rest of session
- Non-panicking plugins unaffected
- `PluginError::Panicked` variant available for error handling

## Do NOT
- Add subprocess isolation (that's a larger architectural change)
- Change the plugin manifest format
- Change the PluginTier capability system
