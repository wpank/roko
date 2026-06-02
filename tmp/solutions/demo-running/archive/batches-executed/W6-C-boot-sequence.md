# W6-C: RokoBootstrap Struct for Unified Startup

**Priority**: P2 — reduces ad-hoc startup duplication
**Effort**: 2-3 hours
**Files to modify**: 4-5 files
**Dependencies**: W5-A (config-aware auth), W6-A (unified config loading)

## Problem

Each entry point (CLI chat, ACP, serve, plan run) implements ad-hoc startup with different config loading, workspace init, signal handling, error reporting.

## Fix

Create a single `RokoBootstrap` struct that all entry points use.

### File: `crates/roko-cli/src/bootstrap.rs` (new file)

```rust
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use roko_core::config::schema::RokoConfig;

pub struct BootOpts {
    pub require_workspace: bool,
    pub require_provider: bool,
    pub acquire_lock: bool,
}

impl Default for BootOpts {
    fn default() -> Self {
        Self {
            require_workspace: true,
            require_provider: false,
            acquire_lock: false,
        }
    }
}

pub struct RokoBootstrap {
    pub config: RokoConfig,
    pub workdir: PathBuf,
    pub workspace_ready: bool,
}

impl RokoBootstrap {
    pub fn new(workdir: &Path, opts: BootOpts) -> Result<Self> {
        // 1. Canonicalize workdir
        let workdir = workdir.canonicalize().unwrap_or_else(|_| workdir.to_path_buf());

        // 2. Ensure workspace (.roko/) exists if required
        let roko_dir = workdir.join(".roko");
        let workspace_ready = roko_dir.is_dir();
        if opts.require_workspace && !workspace_ready {
            anyhow::bail!(
                "No roko workspace found at {}.\n  hint: run `roko init`",
                workdir.display()
            );
        }

        // 3. Load unified config
        let config = roko_core::config::loader::load_config_unified(&workdir)
            .unwrap_or_default();

        // 4. Validate providers if required
        if opts.require_provider {
            crate::preflight::preflight_providers(&config)?;
        }

        Ok(Self {
            config,
            workdir,
            workspace_ready,
        })
    }
}
```

### Usage in entry points

```rust
// Chat entry:
let boot = RokoBootstrap::new(&workdir, BootOpts {
    require_provider: true,
    ..Default::default()
})?;

// Plan run entry:
let boot = RokoBootstrap::new(&workdir, BootOpts {
    require_workspace: true,
    require_provider: true,
    acquire_lock: true,
    ..Default::default()
})?;

// Status (read-only):
let boot = RokoBootstrap::new(&workdir, BootOpts {
    require_workspace: true,
    ..Default::default()
})?;
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-C-boot-sequence.md and implement all changes described in it. Create crates/roko-cli/src/bootstrap.rs with RokoBootstrap struct. Wire into chat, plan run, and serve entry points. This depends on W5-A (config-aware auth) and W6-A (unified config loading) being done first. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 6 batches together. Do not commit individually.

## Checklist

- [x] Create `bootstrap.rs` with `RokoBootstrap` struct
- [x] Implement `new()` with canonical workdir, workspace check, config load, provider validation
- [x] Wire into chat entry point
- [x] Wire into plan run entry point
- [x] Wire into serve entry point
- [ ] Verify: all entry points use consistent startup (deferred)
