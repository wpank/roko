# PAI_01: Consolidate config assembly to single canonical load path

## Task
Reduce 12+ independent `load_config()` call sites to a single config load at startup, passed by reference to all consumers.

## Runner Context
Runner PAI (Config & Infrastructure), batch 1 of 4. No dependencies.

## Problem
CI-1 anti-pattern: "12 independent config loads from disk." Every subsystem calls `load_config()` independently, re-parsing roko.toml each time. This means:
- Config changes mid-run aren't seen by subsystems that loaded earlier
- No single point to validate config consistency
- Hot-reload (PU_01) needs to update all consumers

## Current Config Load Sites (VERIFIED)

12+ `load_config()` call sites:
- `orchestrate.rs:866`
- `unified.rs:42,170`
- `run.rs:421,1843,2410,2722`
- `daemon.rs:320`
- `prd.rs:730`
- `worker/cloud.rs:453`
- `agent_exec.rs:93`
- `acp/config.rs:44`
- `dreams/runner.rs:69,141`

Also 5 config synthesis functions in `agent_config.rs:78,88,98`.

## Exact Changes

### Step 1: Create RuntimeConfig wrapper

```rust
// In roko-core or roko-cli:
/// Shared runtime config handle. Loaded once, shared by reference.
/// Hot-reload updates the inner config atomically.
pub struct RuntimeConfig {
    inner: Arc<ArcSwap<RokoConfig>>,
}

impl RuntimeConfig {
    pub fn load(workdir: &Path) -> Result<Self> {
        let config = load_config(workdir)?;
        Ok(Self {
            inner: Arc::new(ArcSwap::from_pointee(config)),
        })
    }

    pub fn get(&self) -> arc_swap::Guard<Arc<RokoConfig>> {
        self.inner.load()
    }

    /// Hot-reload: swap the inner config atomically
    pub fn swap(&self, new: RokoConfig) {
        self.inner.store(Arc::new(new));
    }
}
```

Note: `arc-swap` is a lightweight crate. If it's not a dependency, use `Arc<RwLock<RokoConfig>>` instead:

```rust
pub struct RuntimeConfig {
    inner: Arc<RwLock<RokoConfig>>,
}
```

### Step 2: Load config once at CLI entry point

In `main.rs` or the top-level command handler:

```rust
let runtime_config = RuntimeConfig::load(&workdir)?;
```

Pass `&RuntimeConfig` to all subcommands and subsystems.

### Step 3: Replace load_config() calls with runtime_config.get()

For each of the 12+ sites:

```rust
// BEFORE:
let config = load_config(&workdir)?;

// AFTER:
let config = runtime_config.get();
```

### Step 4: Wire hot-reload to use RuntimeConfig::swap()

PU_01's config watcher should call:

```rust
runtime_config.swap(new_config);
```

Instead of each subsystem reloading independently.

## Write Scope
- `crates/roko-core/src/config/mod.rs` or new file (RuntimeConfig)
- All 12+ call sites in roko-cli (replace load_config with runtime_config.get)

## Read-Only Context
- `crates/roko-core/src/config/schema.rs` (RokoConfig struct)
- `crates/roko-core/src/config/mod.rs` (load_config function)


## Verify
```bash
cargo build -p roko-core 2>&1 | head -30
cargo test -p roko-core 2>&1 | tail -20
```
## Acceptance Criteria
- Config loaded from disk exactly once per process startup
- All consumers receive the same config instance
- Hot-reload updates all consumers atomically via swap()
- No `load_config()` calls remain in non-startup code
- Existing behavior unchanged

## Do NOT
- Change the RokoConfig struct
- Change the load_config() function (it's still used by RuntimeConfig::load)
- Add config validation beyond what already exists
