# AUDIT: Batch R7_E03 — MCP server mesh polish

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R7_E03`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task
MCP server mesh polish

## Runner Context
You are working in runner `mega-parity`, batch R7_E03.
This batch is part of Runner 7: mori-polish — Complete remaining Mori-like UX polish after core contracts stable.

## Problem
MCP server configuration is manual and opaque. Users need CLI commands to list, test, and add MCP
servers without editing config files directly. Currently there is no `roko config mcp` subcommand.

## Architecture Contract
- Use existing MCP config format (`.roko/mcp-config.json` with `mcpServers.roko` shape)
- No MCP protocol handling — just launch management
- No server lifecycle management (no start/stop/monitor)
- `list` reads from `.roko/mcp-config.json`; `test` spawns briefly then kills; `add` writes file

## Current State

### ConfigCmd enum (crates/roko-cli/src/main.rs, line 1416)

The `ConfigCmd` enum ends at line 1558 with `Secrets`. It has no `Mcp` variant yet:

```rust
// crates/roko-cli/src/main.rs lines 1551–1558
    // ── Secrets ───────────���─────────────────────────────────────────
    /// Manage profile-aware secrets (set, get, list, rotate).
    Secrets {
        #[command(subcommand)]
        cmd: roko_cli::SecretsCmd,
    },
}
```

### dispatch_config function (crates/roko-cli/src/commands/config_cmd.rs, line 16)

The dispatch match ends with the `Secrets` arm which is intercepted before reaching `dispatch_config`.
The `other` arm in main.rs routes remaining variants through `dispatch_config`. The new `Mcp` variant
must be handled in `dispatch_config`.

### McpLaunch struct (crates/roko-agent/src/process/mcp.rs, lines 16–21)
```rust
// crates/roko-agent/src/process/mcp.rs lines 16–21
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpLaunch {
    /// The command to execute (e.g. an absolute path to a binary, or `cargo`).
    pub command: String,
    /// Arguments to pass to the command.
    pub args: Vec<String>,
}
```

### MCP config file format (crates/roko-agent/src/process/mcp.rs lines 181–202)
The canonical config file is `.roko/mcp-config.json` with shape:
```json
{ "mcpServers": { "roko": { "command": "...", "args": [...] } } }
```

The functions `find_mcp_launch(working_dir)` and `write_mcp_config(base, launch)` already exist
in `crates/roko-agent/src/process/mcp.rs`:

```rust
// crates/roko-agent/src/process/mcp.rs line 148
pub fn find_mcp_launch(working_dir: &Path) -> Option<McpLaunch>

// crates/roko-agent/src/process/mcp.rs line 185
pub fn write_mcp_config(base: &Path, launch: &McpLaunch) -> Result<PathBuf, std::io::Error>
```

## Step-by-Step Changes

### Step 1: Add `ConfigMcpCmd` enum and `Mcp` variant to `ConfigCmd` in `main.rs`

Find the end of `ConfigCmd` (line 1558 ends with `}`) and the begin of `ConfigProviderCmd` (line 1561).

Add BEFORE the closing `}` of `ConfigCmd`:

```rust
    // ── MCP servers ────��────────────────────────────────────────────
    /// Manage MCP server configuration (list, test, add).
    Mcp {
        #[command(subcommand)]
        cmd: ConfigMcpCmd,
    },
```

Then add `ConfigMcpCmd` after `ConfigSubscriptionCmd` (around line 1640):

```rust
#[derive(Debug, Subcommand)]
enum ConfigMcpCmd {
    /// List configured MCP servers.
    List {
        /// Directory containing `roko.toml` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Test whether a named MCP server starts successfully.
    Test {
        /// MCP server name (currently only "roko" is used).
        name: String,
        /// Directory containing `.roko/mcp-config.json` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Add an MCP server entry to `.roko/mcp-config.json`.
    Add {
        /// Server name (e.g. "roko").
        name: String,
        /// Launch command (e.g. "/usr/local/bin/roko-mcp").
        command: String,
        /// Optional arguments.
        #[arg(last = true)]
        args: Vec<String>,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}
```

### Step 2: Handle `ConfigCmd::Mcp` in `commands/config_cmd.rs`

In `dispatch_config` (line 16 of `commands/config_cmd.rs`), add a match arm for `ConfigCmd::Mcp`
before the closing of the `match cmd { ... }` block:

```rust
        ConfigCmd::Mcp { cmd } => {
            let workdir = resolve_workdir(cli);
            match cmd {
                ConfigMcpCmd::List { workdir: wd } => {
                    let wd = wd.unwrap_or(workdir);
                    cmd_mcp_list(&wd)?;
                }
                ConfigMcpCmd::Test { name, workdir: wd } => {
                    let wd = wd.unwrap_or(workdir);
                    cmd_mcp_test(&wd, &name)?;
                }
                ConfigMcpCmd::Add { name, command, args, workdir: wd } => {
                    let wd = wd.unwrap_or(workdir);
                    cmd_mcp_add(&wd, &name, &command, args)?;
                }
            }
            Ok(())
        }
```

### Step 3: Add `cmd_mcp_list`, `cmd_mcp_test`, `cmd_mcp_add` to `config_cmd.rs`

Add at the bottom of `commands/config_cmd.rs`:

```rust
// ── MCP server commands ──────────────────────────────────────────────────

use roko_agent::process::mcp::{McpLaunch, find_mcp_launch, write_mcp_config};

fn cmd_mcp_list(workdir: &Path) -> Result<()> {
    match find_mcp_launch(workdir) {
        Some(launch) => {
            println!("{:<12} {}", "name", "command");
            println!("{}", "─".repeat(48));
            let args_display = if launch.args.is_empty() {
                String::new()
            } else {
                format!(" {}", launch.args.join(" "))
            };
            println!("{:<12} {}{}", "roko", launch.command, args_display);
        }
        None => {
            println!("no MCP servers configured");
            println!("  Hint: roko config mcp add roko <command> [args...]");
        }
    }
    Ok(())
}

fn cmd_mcp_test(workdir: &Path, name: &str) -> Result<()> {
    if name != "roko" {
        anyhow::bail!("unknown MCP server '{name}'; only 'roko' is supported");
    }
    let Some(launch) = find_mcp_launch(workdir) else {
        anyhow::bail!("no MCP config found in {}", workdir.display());
    };

    print!("testing {} ({})... ", name, launch.command);
    std::io::stdout().flush().context("flush")?;

    // Spawn the server process briefly, then kill it.
    let mut child = std::process::Command::new(&launch.command)
        .args(&launch.args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn '{}'", launch.command))?;

    // Give it 500ms to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    let still_running = child.try_wait()?.is_none();
    let _ = child.kill();
    let _ = child.wait();

    if still_running {
        println!("OK");
    } else {
        println!("FAILED (process exited immediately)");
        anyhow::bail!("MCP server '{}' exited immediately after spawn", name);
    }
    Ok(())
}

fn cmd_mcp_add(workdir: &Path, name: &str, command: &str, args: Vec<String>) -> Result<()> {
    if name != "roko" {
        // For now, only "roko" is the convention; warn but still write
        eprintln!("warning: only 'roko' is the conventional MCP server name; writing '{name}' anyway");
    }
    let launch = McpLaunch {
        command: command.to_string(),
        args,
    };
    let path = write_mcp_config(workdir, &launch)
        .with_context(|| format!("write MCP config in {}", workdir.display()))?;
    println!("wrote {}", path.display());
    println!("  command: {}", launch.command);
    if !launch.args.is_empty() {
        println!("  args:    {}", launch.args.join(" "));
    }
    Ok(())
}
```

**Important**: `roko_agent::process::mcp` uses the re-export path. Check the actual export path:
```bash
grep -rn 'pub.*mcp\|pub use.*mcp' crates/roko-agent/src/lib.rs crates/roko-agent/src/process/mod.rs | head -10
```
Use whatever the actual public path is. If not yet re-exported, add the import directly:
```rust
use roko_agent::process::mcp::{McpLaunch, find_mcp_launch, write_mcp_config};
```

### Step 4: Ensure `ConfigMcpCmd` is imported in `commands/config_cmd.rs`

The file already has `use crate::*;` at the top (line 4). Since `ConfigMcpCmd` is defined in
`main.rs` inside the binary crate, and `commands/config_cmd.rs` also lives in the binary crate,
the `use crate::*;` should make `ConfigMcpCmd` available — OR you may need to add:
```rust
use crate::ConfigMcpCmd;
```

## Target Implementation

Full `cmd_mcp_list` / `cmd_mcp_test` / `cmd_mcp_add` (for copy-paste into `config_cmd.rs`):

```rust
fn cmd_mcp_list(workdir: &Path) -> Result<()> {
    match roko_agent::process::mcp::find_mcp_launch(workdir) {
        Some(launch) => {
            println!("{:<12} command", "name");
            println!("{}", "─".repeat(48));
            let args_s = launch.args.join(" ");
            println!(
                "{:<12} {}{}",
                "roko",
                launch.command,
                if args_s.is_empty() { String::new() } else { format!(" {args_s}") }
            );
        }
        None => {
            println!("no MCP servers configured");
            println!("  Add one: roko config mcp add roko <command> [-- args...]");
        }
    }
    Ok(())
}

fn cmd_mcp_test(workdir: &Path, name: &str) -> Result<()> {
    anyhow::ensure!(name == "roko", "unknown MCP server '{name}'; only 'roko' is supported");
    let launch = roko_agent::process::mcp::find_mcp_launch(workdir)
        .ok_or_else(|| anyhow::anyhow!("no MCP config in {}", workdir.display()))?;

    print!("testing {} ({})... ", name, launch.command);
    std::io::stdout().flush().context("flush")?;

    let mut child = std::process::Command::new(&launch.command)
        .args(&launch.args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("spawn '{}'", launch.command))?;

    std::thread::sleep(std::time::Duration::from_millis(500));
    let alive = child.try_wait()?.is_none();
    let _ = child.kill();
    let _ = child.wait();

    if alive {
        println!("OK");
        Ok(())
    } else {
        println!("FAILED");
        anyhow::bail!("MCP server '{}' exited immediately after spawn", name);
    }
}

fn cmd_mcp_add(workdir: &Path, _name: &str, command: &str, args: Vec<String>) -> Result<()> {
    let launch = roko_agent::process::mcp::McpLaunch {
        command: command.to_string(),
        args,
    };
    let path = roko_agent::process::mcp::write_mcp_config(workdir, &launch)
        .context("write MCP config")?;
    println!("wrote {}", path.display());
    Ok(())
}
```

## Integration Points

- **ConfigCmd::Mcp** variant in `crates/roko-cli/src/main.rs` lines 1551–1558 (before `Secrets`)
- **ConfigMcpCmd** enum — add to `main.rs` after `ConfigSubscriptionCmd` (~line 1640)
- **dispatch_config** in `crates/roko-cli/src/commands/config_cmd.rs` — match arm for `Mcp`
- **cmd_mcp_list / test / add** — helper functions in `config_cmd.rs`
- **roko_agent::process::mcp** — uses `find_mcp_launch` and `write_mcp_config` already exported
- **No new crate dependencies** needed — `roko-agent` is already a dependency of `roko-cli`

## Grep Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# 1. Build
cargo build -p roko-cli 2>&1 | tail -5

# 2. Lint (must be clean)
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | tail -10

# 3. Verify ConfigMcpCmd in main.rs
grep -n 'ConfigMcpCmd\|Mcp {' crates/roko-cli/src/main.rs

# 4. Verify cmd_mcp_list in config_cmd.rs
grep -n 'cmd_mcp_list\|cmd_mcp_test\|cmd_mcp_add' crates/roko-cli/src/commands/config_cmd.rs

# 5. Verify help shows mcp subcommand
cargo run -p roko-cli -- config --help 2>&1 | grep mcp

# 6. Verify mcp list subcommand is reachable
cargo run -p roko-cli -- config mcp --help 2>&1 | head -15
```

Expected from step 3: at least one hit each for `ConfigMcpCmd` and `Mcp {`.
Expected from step 4: hits for all three handler functions.
Expected from step 5: "mcp" appears in `config --help` output.

## Acceptance Criteria
- [ ] `roko config mcp list` shows configured servers (or "no MCP servers configured")
- [ ] `roko config mcp test roko` spawns and checks the server starts
- [ ] `roko config mcp add roko <command> [args...]` writes `.roko/mcp-config.json`
- [ ] Uses existing `McpLaunch` / `find_mcp_launch` / `write_mcp_config` from `roko-agent`
- [ ] Clean table output for `list`
- [ ] `cargo build -p roko-cli` succeeds
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` passes clean

## Do NOT
- Implement MCP protocol handling (no JSON-RPC, no stdio framing)
- Add server lifecycle management (no start/stop/status daemon tracking)
- Make MCP required (graceful "no config" message is correct)
- Change the `.roko/mcp-config.json` format (use `write_mcp_config` exactly)
- Add a new file — all handler code goes in `commands/config_cmd.rs`

## Evidence
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` lines 1416–1558 — `ConfigCmd` enum
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs` — `dispatch_config`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/process/mcp.rs` lines 16–202 — `McpLaunch`, `find_mcp_launch`, `write_mcp_config`

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/commands/config_cmd.rs` (2740 lines — truncated)

```rust
//! config_cmd command handlers.
#![allow(unused_imports)]

use crate::*;
use roko_agent::process::{McpLaunch, find_mcp_launch, write_mcp_config};
use serde::Serialize;

pub(crate) const fn edit_target(global: bool, project: bool) -> EditTarget {
    if global {
        EditTarget::Global
    } else if project {
        EditTarget::Project
    } else {
        EditTarget::Auto
    }
}

pub(crate) async fn dispatch_config(cli: &Cli, cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Init {
            yes,
            agent,
            model,
            budget,
            role,
            enable_gates,
            path,
            non_interactive,
        } => {
            let mut inputs = WizardInputs {
                agent_command: agent.clone(),
                token_budget: budget,
                model: model.clone(),
                role,
                enable_gates: if enable_gates { Some(true) } else { None },
                yes,
                ..Default::default()
            };
            if let (Some("ollama"), Some(m)) = (agent.as_deref(), model.as_ref()) {
                inputs.agent_args = Some(vec!["run".into(), m.clone()]);
            }
            if non_interactive {
                if inputs.agent_command.is_none() {
                    return Err(anyhow!("--non-interactive requires --agent"));
                }
                inputs.token_budget.get_or_insert(8000);
                inputs
                    .role
                    .get_or_insert_with(|| "You are a Roko agent.".into());
                inputs.enable_gates.get_or_insert(false);
                inputs.yes = true;
                if inputs.agent_args.is_none() {
                    inputs.agent_args = Some(vec![]);
                }
            }
            let _ = run_init_wizard(path, &inputs)?;
            Ok(())
        }
        ConfigCmd::Show { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            config_cmd::cmd_show(&wd)
        }
        ConfigCmd::Path { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            config_cmd::cmd_path(&wd)
        }
        ConfigCmd::Edit {
            global,
            project,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let target = edit_target(global, project);
            config_cmd::cmd_edit(&wd, target)
        }
        ConfigCmd::Set {
            key,
            value,
            global: _,
            project,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let target = if project {
                EditTarget::Project
            } else {
                EditTarget::Global
            };
            config_cmd::cmd_set(&wd, target, &key, &value)
        }
        ConfigCmd::SetSecret { name, value } => config_cmd::cmd_set_secret(&name, &value),
        ConfigCmd::CheckSecrets { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            config_cmd::cmd_check_secrets(&wd)
        }
        ConfigCmd::Validate { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            config_cmd::cmd_validate(&wd).await
        }
        ConfigCmd::Migrate {
            workdir,
            dry_run,
            yes,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            config_cmd::cmd_migrate(&wd, dry_run, yes)
        }
        // ── Providers ───────────────────────────────────────────────
        ConfigCmd::Providers { cmd } => match cmd {
            ConfigProviderCmd::List { workdir } => {
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                cmd_provider_list(&wd).await?;
                Ok(())
            }
            ConfigProviderCmd::Health { workdir } => {
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                cmd_provider_health(&wd)?;
                Ok(())
            }
            ConfigProviderCmd::Test {
                provider,
                all,
                workdir,
            } => {
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                if all {
                    cmd_provider_test_all(&wd, cli.json).await?;
                } else if provider.is_some() || cli.model.is_some() {
                    cmd_provider_test(
                        &wd,
                        provider.as_deref(),
                        cli.model.as_deref(),
                        cli.role.as_deref(),
                        cli.json,
                    )
                    .await?;
                } else {
                    bail!("provide a provider name or use --all");
                }
                Ok(())
            }
        },
        // ── Models ──────────────────────────────────────────────────
        ConfigCmd::Models { cmd } => match cmd {
            ConfigModelCmd::List { workdir } => {
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                cmd_model_list(&wd)?;
                Ok(())
            }
            ConfigModelCmd::Route {
                model,
                explain,
                complexity,
                workdir,
            } => {
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                cmd_model_route(
                    &wd,
                    cli.model.as_deref(),
                    cli.role.as_deref(),
                    &model,
                    explain,
                    complexity.as_deref(),
                )?;
                Ok(())
            }
        },
        // ── Subscriptions ───────────────────────────────────────────
        ConfigCmd::Subscriptions { cmd } => {
            let workdir = resolve_workdir(cli);
            match cmd {
                ConfigSubscriptionCmd::List => {
                    roko_cli::subscriptions::cmd_list(&workdir, cli.json)?
                }
                ConfigSubscriptionCmd::Add { template, trigger } => {
                    roko_cli::subscriptions::cmd_add(&workdir, &template, &trigger)?
                }
                ConfigSubscriptionCmd::Remove { id } => {
                    roko_cli::subscriptions::cmd_remove(&workdir, &id)?
                }
                ConfigSubscriptionCmd::Enable { id } => {
                    roko_cli::subscriptions::cmd_set_enabled(&workdir, &id, true)?
                }
                ConfigSubscriptionCmd::Disable { id } => {
                    roko_cli::subscriptions::cmd_set_enabled(&workdir, &id, false)?
                }
            }
            Ok(())
        }
        ConfigCmd::Mcp { cmd } => {
            let root_workdir = resolve_workdir(cli);
            match cmd {
                ConfigMcpCmd::List { workdir } => {
                    let wd = workdir.unwrap_or_else(|| root_workdir.clone());
                    cmd_mcp_list(&wd)?;
                }
                ConfigMcpCmd::Test { name, workdir } => {
                    let wd = workdir.unwrap_or_else(|| root_workdir.clone());
                    cmd_mcp_test(&wd, &name).await?;
                }
// ... (2340 lines omitted) ...
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86_399 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86_400),
    }
}

pub(crate) fn file_modified_ms(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    system_time_to_ms(modified)
}

pub(crate) fn system_time_to_ms(timestamp: SystemTime) -> Option<i64> {
    timestamp
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
}

pub(crate) fn unix_ms_now() -> i64 {
    system_time_to_ms(SystemTime::now()).unwrap_or(0)
}

pub(crate) fn max_timestamp(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderListRow {
    pub(crate) provider: String,
    pub(crate) kind: String,
    pub(crate) base_url: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelListRow {
    pub(crate) model: String,
    pub(crate) provider: String,
    pub(crate) slug: String,
    pub(crate) context: String,
    pub(crate) tools: String,
    pub(crate) thinking: String,
    pub(crate) vision: String,
    pub(crate) cost: String,
}

pub(crate) const PROVIDER_FAILURE_THRESHOLD: u32 = 3;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderTestAllRow {
    pub(crate) provider: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderHealthRow {
    pub(crate) provider: String,
    pub(crate) state: String,
    pub(crate) fails: String,
    pub(crate) cooldown: String,
    pub(crate) latency_p50: String,
    pub(crate) error_rate: String,
    pub(crate) last_check: String,
}

pub(crate) fn cmd_mcp_list(workdir: &Path) -> Result<()> {
    match find_mcp_launch(workdir) {
        Some(launch) => {
            let name = "roko";
            let args = if launch.args.is_empty() {
                "—".to_string()
            } else {
                launch.args.join(" ")
            };
            let widths = [
                "Name".len().max(name.len()),
                "Command".len().max(launch.command.len()),
                "Args".len().max(args.len()),
            ];

            println!(
                "{:<name_w$}  {:<command_w$}  {:<args_w$}",
                "Name",
                "Command",
                "Args",
                name_w = widths[0],
                command_w = widths[1],
                args_w = widths[2],
            );
            println!("{}", "─".repeat(widths.iter().copied().sum::<usize>() + 4));
            println!(
                "{:<name_w$}  {:<command_w$}  {:<args_w$}",
                name,
                launch.command,
                args,
                name_w = widths[0],
                command_w = widths[1],
                args_w = widths[2],
            );
        }
        None => {
            println!("no MCP servers configured");
            println!("  add one with: roko config mcp add roko <command> [-- args...]");
        }
    }

    Ok(())
}

pub(crate) async fn cmd_mcp_test(workdir: &Path, name: &str) -> Result<()> {
    if name.trim() != "roko" {
        bail!("unknown MCP server '{name}'; only 'roko' is supported");
    }

    let Some(launch) = find_mcp_launch(workdir) else {
        bail!("no MCP config found in {}", workdir.display());
    };

    print!("testing {} ({})... ", name, launch.command);
    let mut stdout = std::io::stdout();
    std::io::Write::flush(&mut stdout).context("flush stdout")?;

    let mut child = std::process::Command::new(&launch.command)
        .args(&launch.args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn '{}'", launch.command))?;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let still_running = child.try_wait()?.is_none();
    let _ = child.kill();
    let _ = child.wait();

    if still_running {
        println!("OK");
        Ok(())
    } else {
        println!("FAILED");
        bail!("MCP server '{}' exited immediately after spawn", name);
    }
}

pub(crate) fn cmd_mcp_add(
    workdir: &Path,
    name: &str,
    command: &str,
    args: Vec<String>,
) -> Result<()> {
    if name.trim() != "roko" {
        bail!("unknown MCP server '{name}'; only 'roko' is supported");
    }

    let launch = McpLaunch {
        command: command.to_string(),
        args,
    };
    let path = write_mcp_config(workdir, &launch)
        .with_context(|| format!("write MCP config in {}", workdir.display()))?;

    println!("wrote {}", path.display());
    println!("  name:    roko");
    println!("  command: {}", launch.command);
    if !launch.args.is_empty() {
        println!("  args:    {}", launch.args.join(" "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_mcp_add_rejects_unknown_server_name() {
        let err = cmd_mcp_add(
            std::path::Path::new("/tmp"),
            "other",
            "roko-mcp",
            Vec::new(),
        )
        .expect_err("unsupported MCP server names should fail");
        assert!(err
            .to_string()
            .contains("only 'roko' is supported"));
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
