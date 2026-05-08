# Batch ACP01 — Scaffold `roko-acp` crate + workspace wire

## Goal

Create the `roko-acp` crate skeleton with all module stubs and wire it into the workspace.

## Target files

- `crates/roko-acp/Cargo.toml` — New crate manifest
- `crates/roko-acp/src/lib.rs` — Module declarations + re-exports
- `crates/roko-acp/src/*.rs` — Stub files for all modules
- `Cargo.toml` — Add `roko-acp` to workspace members

## Implementation details

### Cargo.toml

```toml
[package]
name = "roko-acp"
version = "0.1.0"
edition = "2024"
description = "ACP (Agent Client Protocol) server for Roko"
license = "MIT OR Apache-2.0"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
```

### lib.rs

Declare all modules:

```rust
//! ACP (Agent Client Protocol) server for Roko.
//!
//! Implements the ACP JSON-RPC 2.0 protocol over stdio, enabling Roko
//! to work as a coding agent from any ACP-compatible editor (JetBrains,
//! Zed, Neovim, VS Code, etc.).

pub mod types;
pub mod transport;
pub mod handler;
pub mod session;
pub mod config;
pub mod config_options;
pub mod commands;
pub mod elicitation;
pub mod permissions;
pub mod bridge_fs;
pub mod bridge_terminal;
pub mod bridge_events;
pub mod bridge_plan;
pub mod bridge_gates;
pub mod bridge_usage;
```

### Stub files

Create each module file with a doc comment and a placeholder:

```rust
//! <Module description>

// Stubbed — implementation in batch ACPnn
```

Module descriptions:
- `types.rs` — ACP protocol types (JSON-RPC messages, session types, update types)
- `transport.rs` — Stdio transport layer for JSON-RPC messages
- `handler.rs` — Main ACP dispatch loop
- `session.rs` — ACP session state management
- `config.rs` — ACP server configuration
- `config_options.rs` — Session config options (mode, model, thinking, etc.)
- `commands.rs` — Slash command definitions and dispatch
- `elicitation.rs` — Structured form dialogs via elicitation/create
- `permissions.rs` — Permission request/response bridge
- `bridge_fs.rs` — File system bridge (editor-mediated I/O)
- `bridge_terminal.rs` — Terminal bridge (editor-mediated shell commands)
- `bridge_events.rs` — Cognitive event to session/update streaming
- `bridge_plan.rs` — Plan phase to plan notification mapping
- `bridge_gates.rs` — Gate results to tool call card mapping
- `bridge_usage.rs` — Token/cost to usage notification mapping

### Workspace wire

Add `"crates/roko-acp"` to the `members` list in the root `Cargo.toml`.

### IMPORTANT: Fix pre-existing workspace error

The workspace has a pre-existing bug: `crates/roko-cli/Cargo.toml` contains a **duplicate** `roko-learn` dependency entry. This causes `cargo check` to fail for the entire workspace. You MUST fix this as part of this batch:

1. Open `crates/roko-cli/Cargo.toml`
2. Find the duplicate `roko-learn = { path = "../roko-learn" }` lines
3. Remove the duplicate (keep exactly one)

Without this fix, `cargo check -p roko-acp` will fail because Cargo validates the entire workspace.

### Allowed write scope

This batch is allowed to modify:
- `crates/roko-acp/` (all files — new crate)
- `Cargo.toml` (root workspace — add member)
- `crates/roko-cli/Cargo.toml` (fix duplicate dep only)

## Verification

```bash
cargo check -p roko-acp
```

## Done when

- `crates/roko-acp/` exists with all files
- Root `Cargo.toml` includes `"crates/roko-acp"` in members
- `crates/roko-cli/Cargo.toml` has no duplicate deps
- `cargo check -p roko-acp` succeeds
- All 16 module files exist under `src/`
