# W2-A: Route Tracing to File, Add `--verbose` Flag

**Priority**: P1 — makes demo presentable
**Effort**: 1-2 hours
**Files to modify**: 1 file
**Dependencies**: None

## Problem

Every CLI command dumps tracing INFO/WARN lines to stderr:
```
2026-05-04T08:03:03.280711Z  WARN roko_core::config::schema: roko.toml uses config version 1
2026-05-04T08:03:03.283842Z  INFO roko_agent::provider: creating agent via provider adapter
```
This makes demo output noisy and hard to read. There's no `--verbose` flag — only `--quiet` exists (line 253 of main.rs).

## Root Cause

The tracing subscriber in `crates/roko-cli/src/main.rs` (lines 1800-1894) writes to stderr by default using a `RedactingFormat` wrapper. There's no option to route to a file.

## Exact Code to Change

### File: `crates/roko-cli/src/main.rs`

### Change 1: Add `--verbose` flag to Cli struct

Find the `Cli` struct (search for `#[derive(Parser)]` or `struct Cli`). Near the existing `--quiet` flag (line ~253):

```rust
/// Suppress non-essential output.
#[arg(long, global = true)]
quiet: bool,
```

Add:
```rust
/// Enable verbose tracing output to stderr. Without this, tracing goes only to .roko/roko.log.
#[arg(long, short = 'v', global = true)]
verbose: bool,
```

### Change 2: Modify tracing subscriber setup (lines 1800-1894)

The current code sets up tracing to stderr always. Change it to:
1. **Always** write to `.roko/roko.log` (file layer)
2. **Only** write to stderr when `--verbose` is set OR `RUST_LOG` env var is present

Find the tracing setup section (around line 1800, likely in a function called during main() setup). The current pattern is:

```rust
// Current: always writes to stderr
tracing_subscriber::fmt()
    .with_target(false)
    .with_ansi(ansi_logs)
    .event_format(RedactingFormat::new(
        tracing_subscriber::fmt::format(),
        scrubber,
    ))
    .with_env_filter(filter)
    .init();
```

Replace with a dual-layer approach:

```rust
use tracing_subscriber::prelude::*;

// File layer: always write to .roko/roko.log
let log_dir = resolve_workdir(&cli).join(".roko");
let file_layer = if let Ok(()) = std::fs::create_dir_all(&log_dir) {
    let log_path = log_dir.join("roko.log");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok()
        .map(|file| {
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(file)
        })
} else {
    None
};

// Stderr layer: only when --verbose or RUST_LOG is set
let stderr_layer = if cli.verbose || std::env::var("RUST_LOG").is_ok() {
    let scrubber = build_log_scrubber(&startup_env_redactions);
    Some(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(ansi_logs)
            .event_format(RedactingFormat::new(
                tracing_subscriber::fmt::format(),
                scrubber,
            ))
            .with_writer(std::io::stderr)
    )
} else {
    None
};

tracing_subscriber::registry()
    .with(file_layer)
    .with(stderr_layer)
    .with(filter)
    .init();
```

**Important**: The `tracing_subscriber` crate's `Layer` trait supports `Option<L>` — an `Option::None` layer is a no-op. This is the standard pattern for conditional layers.

### Change 3: Handle TUI mode

The existing code has a TUI-specific tracing path (for ratatui). Make sure the file layer is also applied there. Search for the TUI branch in the tracing setup — it likely uses `tui_logger` or a custom layer. Add the file layer alongside it.

### Cargo.toml check

Ensure `tracing-subscriber` has the `env-filter` feature enabled in `crates/roko-cli/Cargo.toml`:
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
```

The `registry()` + `.with()` pattern requires the "registry" or "fmt" feature.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-A-tracing-to-file.md and implement all changes described in it. Focus on main.rs — the Cli struct (add --verbose) and the tracing subscriber setup around line 1800. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 2 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: default mode produces no tracing on stderr; `--verbose` shows tracing; `.roko/roko.log` always populated.

## Checklist

- [x] Add `--verbose` (`-v`) flag to `Cli` struct in main.rs
- [x] Modify tracing setup to use dual-layer (file always + stderr conditional)
- [x] File layer writes to `.roko/roko.log` (append mode)
- [x] Stderr layer only active when `--verbose` or `RUST_LOG` set
- [x] TUI mode also gets file layer
- [ ] Verify: default mode produces no tracing on stderr
- [ ] Verify: `--verbose` shows tracing on stderr
- [ ] Verify: log file always populated
- [ ] Pre-commit checks pass
