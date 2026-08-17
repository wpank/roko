# S-config-5: Migrate roko-acp callers to ValidatedConfig

## Task
Update `roko-acp` to bind `ValidatedConfig` instead of bare `RokoConfig` where it loads or accepts config.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-2. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 4.

## Read first

```bash
rg 'load_config\(|RokoConfig|fn new' crates/roko-acp/src/ -n | head -20
```

## Exact changes

`crates/roko-acp/src/session.rs::AcpSession`:

If `AcpSession::new` accepts `Arc<RokoConfig>`, change to `Arc<ValidatedConfig>` and access via `.config()`.

If session config comes from a parent process (the bridge), it's typically pre-loaded; just change the type.

```rust
// Before
pub fn new(config: Arc<RokoConfig>, ...) -> Self { ... }
self.config.providers.get(...)

// After
pub fn new(config: Arc<ValidatedConfig>, ...) -> Self { ... }
self.config.config().providers.get(...)
```

## Write Scope
- `crates/roko-acp/src/session.rs`
- `crates/roko-acp/src/lib.rs` (only if exposing constructors)
- `crates/roko-acp/src/bridge_events.rs` (only if it constructs sessions)

## Read-Only Context
- `crates/roko-core/src/config/provenance.rs`

## Verify

```bash
rg 'Arc<RokoConfig>' crates/roko-acp/src/
# Expect: 0 hits (or only legacy ones marked with TODO)

rg 'ValidatedConfig' crates/roko-acp/src/
# Expect: 2+ hits
```

## Do NOT

- Do NOT bundle with S-config-3/4.
- Do NOT change session-history / approval / trust state semantics.
- Do NOT add provenance display in this batch (S-config-7 owns).
