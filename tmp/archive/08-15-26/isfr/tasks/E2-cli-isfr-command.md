# E2: Add `roko isfr` CLI Subcommand

## Context

The ISFR keeper is started and managed via CLI: `roko isfr start|status|sources`. This
follows the same pattern as other command groups (e.g. `roko job`, `roko bench`).

**Existing CLI structure** (verified against codebase):
- `Command` enum is in `crates/roko-cli/src/main.rs` at line 313
- `dispatch_subcommand()` is at line 2209
- Handler files are in `crates/roko-cli/src/commands/`
- `commands/mod.rs` has a merge conflict — the HEAD section contains the full module list;
  use that section as the authoritative list when adding `pub mod isfr;`

**Dependencies already in `crates/roko-cli/Cargo.toml`**:
- `roko-chain = { path = "../roko-chain", features = ["alloy-backend"] }` — line 49
- `tokio-util = { workspace = true }` — line 55
- `uuid = { workspace = true }` — line 73

**Do NOT add** these to Cargo.toml — they are already present.

**`ISFRKeeperConfig`** has a `relay_url` field and a `chain_id` field (defined by C2).
**`ChainConfig`** already has `rpc_url` and `chain_id` (`Option<u64>`) fields.
**`ChainConfig::profile`** is the new field added by E1.
**`ISFRSection`** and **`ISFRSourceConfig`** are added by E1.

## Files to Create

- `crates/roko-cli/src/commands/isfr.rs` (NEW)

## Files to Modify

- `crates/roko-cli/src/main.rs` — add `Isfr` variant to `Command` enum + dispatch arm
- `crates/roko-cli/src/commands/mod.rs` — add `pub mod isfr;`

## Pre-Check

```bash
# Verify isfr.rs does not already exist.
ls /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/isfr.rs 2>/dev/null \
  && echo "EXISTS"

# Find where to add the Command enum variant (look for Job, Bench nearby).
grep -n "Job {\|Bench {" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs

# Confirm the last arm before closing brace of dispatch_subcommand().
grep -n "Logout\|Whoami" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs
```

## Implementation

### Step 1: Create `crates/roko-cli/src/commands/isfr.rs`

**Config access pattern**: The CLI uses `resolve_config()` or `resolve_config_for_workdir()`
which may return a custom `Config` type (not `RokoConfig` from roko-core). Verify the return
type:

```bash
grep -n "fn resolve_config\|fn resolve_config_for_workdir" crates/roko-cli/src/main.rs | head -5
```

If it returns `roko_core::config::RokoConfig` or wraps it, access `.isfr`, `.chain`, `.relay`
directly. If it returns a CLI-specific Config type without these fields, you need to load
`RokoConfig` separately:

```rust
use roko_core::config::RokoConfig;
let config = RokoConfig::load_from_dir(&wd)?;
```

Adapt the code below to match whatever pattern the codebase actually uses.

```rust
//! `roko isfr` subcommand — manage the ISFR keeper.

use crate::*;
use clap::Subcommand;

/// ISFR keeper management.
#[derive(Debug, Subcommand)]
pub enum IsfrCmd {
    /// Start the ISFR keeper (foreground, Ctrl-C to stop).
    Start {
        /// Override poll interval in seconds.
        #[arg(long)]
        poll_interval: Option<u64>,
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show ISFR configuration and status.
    Status {
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// List configured ISFR rate sources.
    Sources {
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_isfr(cli: &Cli, cmd: IsfrCmd) -> Result<i32> {
    match cmd {
        IsfrCmd::Start { poll_interval, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let config = resolve_config_for_workdir(cli, &wd)?;

            if !config.isfr.enabled {
                println!("ISFR is not enabled. Set [isfr] enabled = true in roko.toml.");
                return Ok(EXIT_FAILURE);
            }

            // Build ISFRKeeperConfig from the [isfr] and [chain] sections.
            let keeper_config = roko_chain::isfr_keeper::ISFRKeeperConfig {
                poll_interval_secs: poll_interval
                    .unwrap_or(config.isfr.poll_interval_secs),
                epoch_duration_secs: config.isfr.epoch_duration_secs,
                min_submissions: config.isfr.min_submissions,
                outlier_sigma: config.isfr.outlier_sigma,
                relay_url: config.relay.url.clone(),
                chain_id: config.chain.chain_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "31337".to_string()),
            };

            // Convert [[isfr.sources]] entries to SourceConfig.
            let source_configs: Vec<roko_chain::isfr_keeper::SourceConfig> = config
                .isfr
                .sources
                .iter()
                .map(|s| roko_chain::isfr_keeper::SourceConfig {
                    name: s.name.clone(),
                    kind: s.kind.clone(),
                    weight: s.weight,
                    class: s.class.clone(),
                    rate_bps: s.rate_bps,
                    jitter_bps: s.jitter_bps,
                    rpc_url: s.rpc_url.clone(),
                    pool_address: s.pool_address.clone(),
                })
                .collect();

            let keeper_id =
                format!("isfr-keeper-{}", &uuid::Uuid::new_v4().to_string()[..8]);

            let keeper = if source_configs.is_empty() {
                println!("No sources configured, using 4 default mock sources.");
                roko_chain::isfr_keeper::ISFRKeeper::mock_keeper(
                    &keeper_id,
                    keeper_config,
                )
            } else {
                roko_chain::isfr_keeper::ISFRKeeper::from_config(
                    &keeper_id,
                    keeper_config,
                    &source_configs,
                )
            };

            // Relay publish stub — full wiring happens in A6.
            if let Some(ref relay_url) = config.relay.url {
                println!("Relay: {relay_url} (publish stub — full wiring in task A6)");
                let publish_fn: roko_chain::isfr_keeper::PublishFn =
                    std::sync::Arc::new(|topic: &str, msg_type: &str, _payload| {
                        tracing::info!(
                            topic,
                            msg_type,
                            "isfr: relay publish (stub)"
                        );
                    });
                keeper.set_publish_fn(publish_fn);
            }

            println!(
                "ISFR keeper '{}' starting (poll every {}s)...",
                keeper_id,
                config.isfr.poll_interval_secs
            );
            println!("Press Ctrl-C to stop.\n");

            let cancel = tokio_util::sync::CancellationToken::new();
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                cancel_clone.cancel();
            });

            keeper.run(cancel).await;
            println!("\nISFR keeper stopped.");
            Ok(EXIT_SUCCESS)
        }

        IsfrCmd::Status { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let config = resolve_config_for_workdir(cli, &wd)?;

            if !config.isfr.enabled {
                println!("ISFR: disabled");
                return Ok(EXIT_SUCCESS);
            }

            println!("ISFR: enabled");
            println!("  chain:    {}", config.chain.profile);
            println!(
                "  relay:    {}",
                config.relay.url.as_deref().unwrap_or("not configured")
            );
            println!("  sources:  {}", config.isfr.sources.len());
            println!("  poll:     {}s", config.isfr.poll_interval_secs);
            println!("  epoch:    {}s", config.isfr.epoch_duration_secs);

            if !config.isfr.sources.is_empty() {
                println!("\n  Sources:");
                for src in &config.isfr.sources {
                    println!(
                        "    {} ({}, {}, weight={:.2})",
                        src.name, src.kind, src.class, src.weight
                    );
                }
            }

            // TODO (A6): Query /api/isfr/current from serve if running.
            Ok(EXIT_SUCCESS)
        }

        IsfrCmd::Sources { workdir } => {
            let _wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let config = resolve_config_for_workdir(cli, &_wd)?;

            if config.isfr.sources.is_empty() {
                println!("No ISFR sources configured.");
                return Ok(EXIT_SUCCESS);
            }

            println!(
                "{:<20} {:<10} {:<12} {:>6} {:>8}",
                "NAME", "KIND", "CLASS", "WEIGHT", "RATE_BPS"
            );
            for src in &config.isfr.sources {
                println!(
                    "{:<20} {:<10} {:<12} {:>6.2} {:>8}",
                    src.name, src.kind, src.class, src.weight, src.rate_bps
                );
            }
            Ok(EXIT_SUCCESS)
        }
    }
}
```

### Step 2: Add `Isfr` variant to `Command` enum in `main.rs`

The `Job` and `Bench` variants are at lines 520–529. Insert the `Isfr` variant after `Job`:

```rust
/// ISFR keeper management (start, status, sources).
Isfr {
    #[command(subcommand)]
    cmd: commands::isfr::IsfrCmd,
},
```

### Step 3: Add dispatch arm to `dispatch_subcommand()`

The function closes at line 2695 with `Command::Whoami => ...`. Add before that closing brace:

```rust
Command::Isfr { cmd } => commands::isfr::cmd_isfr(cli, cmd).await,
```

### Step 4: Add `pub mod isfr;` to `commands/mod.rs`

`commands/mod.rs` currently has a merge conflict. The HEAD section (lines 3–24) contains
the full module list. Resolve the conflict by keeping the HEAD section and adding `isfr`
in alphabetical order (between `init` and `job`).

**IMPORTANT**: The module list also includes `pub mod graph;` — do NOT remove it. The
final list after resolution should include ALL existing modules plus `isfr`:

```rust
pub mod agent;
pub mod auth;
pub mod bench;
pub mod config_cmd;
pub mod dashboard;
pub mod dev;
pub mod do_cmd;
pub mod experiment;
pub mod graph;       // KEEP — already exists
pub mod init;
pub mod isfr;        // ADD HERE
pub mod job;
pub mod knowledge;
pub mod learn;
pub mod plan;
pub mod prd;
pub mod research;
pub mod server;
pub mod show;
pub mod status;
pub mod think;
pub mod tune;
pub mod util;
```

## Verification

```bash
cargo build -p roko-cli

# Smoke-test — requires roko.toml present (or works without one via defaults)
cargo run -p roko-cli -- isfr status
cargo run -p roko-cli -- isfr sources

# With [isfr] enabled = true in roko.toml:
# cargo run -p roko-cli -- isfr start
# Expected: "ISFR keeper 'isfr-keeper-XXXXXXXX' starting (poll every 10s)..."
```

## Dependencies

- C1 (`ISFRSource`, `MockSource`, `SourceReading`, `CompositeRate`)
- C2 (`ISFRKeeper`, `ISFRKeeperConfig`, `SourceConfig`, `PublishFn`)
- E1 (`ISFRSection`, `ISFRSourceConfig` in `RokoConfig`; `profile` field on `ChainConfig`)
