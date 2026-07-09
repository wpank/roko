# W6-A: Unify 11 load_roko_config Functions → 1

**Priority**: P1 — architecture cleanup
**Effort**: 2-3 hours
**Files to modify**: 11+ files
**Dependencies**: W5-A (auth detection already loads config)

## Problem

11 separate `load_roko_config` function definitions across the codebase, each with different behavior. Only the one in `orchestrate.rs` caches via `OnceLock`. Others call `load_config_unified` directly every time.

## All 11 Definitions (with exact locations)

| # | File | Line | Signature | Notes |
|---|------|------|-----------|-------|
| 1 | `crates/roko-cli/src/orchestrate.rs` | 863 | `fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Has OnceLock cache |
| 2 | `crates/roko-cli/src/config_helpers.rs` | 121 | `pub(crate) fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Public crate |
| 3 | `crates/roko-cli/src/main.rs` | 2480 | `pub(crate) fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Public crate |
| 4 | `crates/roko-serve/src/lib.rs` | 437 | `fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Private |
| 5 | `crates/roko-cli/src/agent_serve.rs` | 559 | `fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Private |
| 6 | `crates/roko-cli/src/subscriptions.rs` | 249 | `fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Private |
| 7 | `crates/roko-cli/src/vision_loop/orchestrator.rs` | 305 | `fn load_roko_config(roko_dir: &Path) -> Result<RokoConfig>` | Takes .roko dir, extracts parent |
| 8 | `crates/roko-cli/src/event_sources.rs` | 77 | `fn load_roko_config(workdir: &Path) -> Result<RokoConfig>` | Private |
| 9 | `crates/roko-cli/src/serve_runtime.rs` | 474 | `fn load_roko_config_file(path: &Path) -> Result<Option<RokoConfig>>` | File-only variant |
| 10 | `crates/roko-cli/src/run.rs` | 2983 | `fn load_roko_config_models(workdir: &Path) -> Vec<String>` | Models-only variant |
| 11 | `crates/roko-acp/src/config.rs` | 48 | `pub fn load_roko_config(&self) -> RokoConfig` | ACP-specific, method on AcpConfig |

## Fix Strategy

### Step 1: Designate ONE canonical loader

`roko_core::config::loader::load_config_unified(workdir)` already exists. All 11 functions ultimately call it (or a variant). Make this the ONLY entry point.

### Step 2: Delete duplicate definitions

For each of the 11 functions:
1. Replace the function body with a call to the canonical loader
2. OR delete the function entirely and update all callers

**Recommendation**: Delete functions 2-8 and 11. Keep function 1 (orchestrate.rs) as a caching wrapper since orchestrate is hot-path. Keep function 9 (serve_runtime.rs) since it has a different signature (file-only). Keep function 10 (run.rs) since it's a projection (models-only).

### Step 3: For each deleted function, update callers

```bash
# Find all callers of each function
grep -rn 'load_roko_config(' crates/roko-cli/src/ --include='*.rs' | grep -v 'fn load_roko_config'
```

Replace each call with:
```rust
roko_core::config::loader::load_config_unified(workdir)?
```

### Step 4: Special case — vision_loop/orchestrator.rs (function 7)

This one takes `.roko/` dir and extracts the parent. Change the caller to pass the workdir directly:
```rust
// BEFORE: load_roko_config(&roko_dir)  // where roko_dir = foo/.roko
// AFTER: roko_core::config::loader::load_config_unified(roko_dir.parent().unwrap_or(&roko_dir))?
```

### Step 5: Special case — ACP (function 11)

This is a method on `AcpConfig`. Change to:
```rust
impl AcpConfig {
    pub fn load_roko_config(&self) -> RokoConfig {
        roko_core::config::loader::load_config_unified(&self.workdir)
            .unwrap_or_default()
    }
}
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-A-config-unify.md and implement all changes described in it. Delete the 8 duplicate load_roko_config functions listed. Update all callers to use roko_core::config::loader::load_config_unified. Keep the cached wrapper in orchestrate.rs (function 1), the file-only variant in serve_runtime.rs (function 9), and the models-only variant in run.rs (function 10). Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 6 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: `cargo build --workspace` succeeds with no unresolved references to deleted functions.

## Checklist

- [x] Identify canonical loader: `roko_core::config::loader::load_config_unified`
- [x] Delete `config_helpers.rs:121` definition → update callers
- [x] Delete `main.rs:2480` definition → update callers
- [x] Delete `roko-serve/src/lib.rs:437` definition → update callers
- [x] Delete `agent_serve.rs:559` definition → update callers
- [x] Delete `subscriptions.rs:249` definition → update callers
- [x] Delete `vision_loop/orchestrator.rs:305` definition → update callers (fix parent dir)
- [x] Delete `event_sources.rs:77` definition → update callers
- [x] Simplify ACP's `load_roko_config` to delegate (already properly delegates; kept as-is)
- [x] Keep orchestrate.rs cached wrapper (performance)
- [x] Keep serve_runtime.rs file-only variant (different signature)
- [x] Keep run.rs models-only variant (projection)
- [ ] Full workspace builds and tests pass
- [ ] Pre-commit checks pass
