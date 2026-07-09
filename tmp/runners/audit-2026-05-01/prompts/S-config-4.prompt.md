# S-config-4: Migrate roko-serve callers to ValidatedConfig

## Task
Update every `roko-serve` caller of `load_config()` to bind `ValidatedConfig`. Server bootstrap (`lib.rs`), state (`state.rs`), and config routes (`routes/config.rs`) are the main sites.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-2. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 4.

## Read first

```bash
rg 'load_config\(|load_roko_config\(' crates/roko-serve/src/ -n
```

`AppState` likely caches a config snapshot. Decide whether to:

(a) Store `Arc<ValidatedConfig>` and expose `state.validated_config()` + `state.load_roko_config()` (back-compat).
(b) Keep current shape (`Arc<RokoConfig>`) and only migrate the entry-point `load_config` call.

**Prefer (a)** so `roko config doctor` (S-config-7) over HTTP can show provenance.

## Exact changes

### 1. `AppState`

```rust
// crates/roko-serve/src/state.rs
pub struct AppState {
    // ... existing fields
    validated_config: ArcSwap<ValidatedConfig>,
    // legacy accessor
}

impl AppState {
    pub fn validated_config(&self) -> Arc<ValidatedConfig> { self.validated_config.load_full() }
    pub fn load_roko_config(&self) -> Arc<RokoConfig> {
        Arc::new(self.validated_config().config().clone())
    }
}
```

### 2. Bootstrap

`crates/roko-serve/src/lib.rs:780+ build_app_state`:

```rust
fn build_app_state(workdir: PathBuf, runtime: Arc<dyn CliRuntime>, mut roko_config: RokoConfig) -> Result<AppState> {
    // Construct ValidatedConfig from the passed-in roko_config + workdir.
    // (If the caller passed roko_config without provenance, wrap as Default
    // provenance.)
    let validated = ValidatedConfig::from_runtime(...).or_else(|_| ValidatedConfig::from_inline(roko_config.clone()))?;
    // ...
}
```

If the bootstrap path doesn't currently call `load_config`, leave that path; only migrate the explicit `roko-serve` reload routes.

### 3. Config reload route

`crates/roko-serve/src/routes/config.rs::reload_config_from_disk`:

```rust
fn reload_config_from_disk(state: &AppState) -> Result<Vec<String>, LoadConfigError> {
    let validated = roko_core::config::load_config(&state.workdir)?;
    state.store_validated_config(validated);
    // ... return any warnings
}
```

`store_validated_config` writes to `ArcSwap<ValidatedConfig>`.

### 4. Tests

```rust
#[tokio::test]
async fn config_reload_validates_semantically() {
    // Write an invalid roko.toml (e.g. duplicate model slug) into workdir.
    // POST /api/config/reload; assert 400 with the validation error.
}
```

## Write Scope
- `crates/roko-serve/src/lib.rs`
- `crates/roko-serve/src/state.rs`
- `crates/roko-serve/src/routes/config.rs`

## Verify

```bash
rg 'ValidatedConfig|validated_config' crates/roko-serve/src/
# Expect: 4+ hits (state field + accessors + reload)
```

## Do NOT

- Do NOT bundle with S-config-3/5.
- Do NOT remove `load_roko_config()` legacy accessor — many routes use it.
- Do NOT change `mask_secret_fields` or any T0-2-related code.
- Do NOT introduce a separate ArcSwap for `RokoConfig` AND `ValidatedConfig`. One canonical store.
