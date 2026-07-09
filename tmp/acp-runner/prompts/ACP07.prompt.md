# Batch ACP07 — `roko acp` CLI subcommand

## Goal

Add the `roko acp` subcommand that launches the ACP server.

## Target files

- `crates/roko-cli/src/main.rs` — Add Acp variant to Commands enum
- `crates/roko-acp/src/lib.rs` — Ensure `run_acp_server` is exported

## Implementation details

### CLI args

Add to the `Commands` enum in main.rs:

```rust
/// Start ACP (Agent Client Protocol) server for editor integration
Acp {
    /// Working directory
    #[arg(long, default_value = ".")]
    workdir: PathBuf,

    /// Configuration profile
    #[arg(long, default_value = "default")]
    profile: String,

    /// Path to roko.toml config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Log file path (stdout is the protocol channel)
    #[arg(long, default_value = ".roko/acp.log")]
    log_file: PathBuf,
},
```

### Command handler

In the match arm for `Commands::Acp`:

```rust
Commands::Acp { workdir, profile, config, log_file } => {
    let acp_config = roko_acp::AcpConfig {
        workdir,
        profile,
        config_path: config,
        log_file,
    };
    roko_acp::run_acp_server(acp_config).await?;
}
```

### Export from roko-acp

Ensure `lib.rs` re-exports:
```rust
pub use config::AcpConfig;
pub use handler::run_acp_server;
```

### Dependency

Add `roko-acp` as a dependency in `crates/roko-cli/Cargo.toml`:
```toml
roko-acp = { path = "../roko-acp" }
```

### Important

- All tracing in the ACP server goes to the log file, NOT stdout
- stdout is exclusively for JSON-RPC protocol messages
- stderr can be used for fatal startup errors only

## Verification

```bash
cargo check -p roko-cli
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- `roko acp` compiles
- `--workdir`, `--profile`, `--config`, `--log-file` args work
- `roko_acp::run_acp_server` is called with correct config
- No logging on stdout
