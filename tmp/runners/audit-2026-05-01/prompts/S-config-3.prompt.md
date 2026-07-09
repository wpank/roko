# S-config-3: Migrate roko-cli callers to ValidatedConfig

## Task
Update every `roko-cli` caller of `load_config()` (or its result) to bind `ValidatedConfig` and access `RokoConfig` via `.config()`.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-2. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 4.

## Read first

```bash
rg 'load_config\(' crates/roko-cli/src/ -n
```

## Exact changes

For each caller:

```rust
// Before
let config: RokoConfig = roko_core::config::load_config(&workdir)?;
config.providers.get("anthropic")...

// After
let validated: ValidatedConfig = roko_core::config::load_config(&workdir)?;
let config: &RokoConfig = validated.config();
config.providers.get("anthropic")...
```

If the caller stored `config: RokoConfig` as a struct field, change the field type to `ValidatedConfig` (preferably) or keep it as `RokoConfig` derived once via `validated.config().clone()` (cheaper to migrate; loses provenance).

For now, **prefer `ValidatedConfig`** in the field; provenance / overrides become accessible to consumers (e.g. `roko config doctor` from S-config-7).

For pieces that just need a snapshot of `RokoConfig` (e.g. handed to `Arc<RokoConfig>`-wrapped state), use `validated.config().clone()` once and store the `Arc<RokoConfig>`. Document with a comment that provenance was discarded for that consumer.

## Write Scope
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`
- (Plus any other roko-cli file that calls `load_config`)

## Verify

```bash
rg 'load_config\(' crates/roko-cli/src/
# Each call binds ValidatedConfig

rg ': RokoConfig = roko_core::config::load_config' crates/roko-cli/src/
# Expect: 0 hits
```

## Do NOT

- Do NOT bundle with S-config-4/5 (other crates).
- Do NOT change config schema fields.
- Do NOT skip `.config()` and try to dereference `ValidatedConfig` directly.
- Do NOT create a global `OnceCell<ValidatedConfig>` here; let each caller own its handle.
