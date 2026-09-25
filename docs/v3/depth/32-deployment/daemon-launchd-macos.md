# 32-deployment/05 -- Daemon Mode: launchd (macOS)

> launchd plist generation, lifecycle commands, IPC over Unix socket,
> log management, startup sequence, and graceful shutdown on macOS.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-cli/src/commands/daemon.rs`

---

## 1. Overview

Daemon mode transforms Roko from a CLI tool into a persistent background
service that:

- Watches repositories for changes (filesystem events, webhooks, cron)
- Triggers plan execution automatically when PRDs change or on schedule
- Maintains state across reboots (launchd restarts it automatically)
- Accepts commands via a Unix domain socket IPC interface
- Streams events to connected clients (TUI, web dashboard, CI hooks)

On macOS, this uses launchd, the native service manager. The
`roko daemon` subcommand handles all lifecycle operations.

---

## 2. The `roko daemon` Subcommand

```
roko daemon [COMMAND]

Commands:
  install      Generate launchd plist and load it (starts on login)
  uninstall    Unload and remove launchd plist
  start        Start the daemon (if installed but not running)
  stop         Stop the daemon
  restart      Stop and start the daemon
  status       Show daemon status (PID, uptime, subscriptions)
  logs         Tail daemon logs (stdout + stderr)
  send <cmd>   Send command to the running daemon via IPC
```

---

## 3. launchd Plist

The generated plist lives at
`~/Library/LaunchAgents/dev.nunchi.roko.plist`. It uses the user-level
LaunchAgents directory (no root required).

Key configuration choices:

| Setting | Value | Why |
|---------|-------|-----|
| `RunAtLoad` | `true` | Start on login |
| `KeepAlive.SuccessfulExit` | `false` | Restart on crash only |
| `ThrottleInterval` | `10` | Minimum 10s between restarts |
| `Nice` | `5` | Lower priority than interactive processes |
| `SoftResourceLimits.NumberOfFiles` | `4096` | File watcher headroom |

### Dynamic Generation

The install command generates the plist dynamically, substituting the
current user's home directory, the resolved binary path, and config
overrides:

```rust
fn generate_plist(config: &DaemonConfig) -> String {
    let binary_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("roko"));
    let home = std::env::var("HOME")
        .unwrap_or_else(|_| String::from("/tmp"));
    let state_dir = format!("{}/.local/state/roko", home);
    std::fs::create_dir_all(&state_dir).ok();
    // ... XML plist template substitution ...
}
```

### Plist Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.nunchi.roko</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/USERNAME/.cargo/bin/roko</string>
        <string>daemon</string>
        <string>run</string>
        <string>--socket</string>
        <string>/tmp/roko-daemon.sock</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>10</integer>
    <key>StandardOutPath</key>
    <string>/Users/USERNAME/.local/state/roko/daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/USERNAME/.local/state/roko/daemon.stderr.log</string>
    <key>Nice</key>
    <integer>5</integer>
</dict>
</plist>
```

---

## 4. Lifecycle Commands

### Install

```bash
$ roko daemon install

[1/4] Generating launchd plist...
[2/4] Writing to ~/Library/LaunchAgents/dev.nunchi.roko.plist
[3/4] Loading plist (launchctl load)...
[4/4] Verifying daemon started...

Roko daemon installed and running.
  PID:      12345
  Socket:   /tmp/roko-daemon.sock
  Logs:     ~/.local/state/roko/daemon.log
```

Uses `launchctl load -w` to load and persist the plist. Checks if already
installed and prints a message rather than overwriting.

### Uninstall

Uses `launchctl unload -w` to stop and remove from startup, then deletes
the plist file.

### Status

```bash
$ roko daemon status

Roko Daemon Status
  State:          running
  PID:            12345
  Uptime:         3d 14h 22m
  Socket:         /tmp/roko-daemon.sock
  Config:         ~/.config/roko/config.toml
  Log (stdout):   ~/.local/state/roko/daemon.stdout.log
  Log (stderr):   ~/.local/state/roko/daemon.stderr.log

Subscriptions:
  ~/dev/project-a    cron: */30 * * * *    last: 2h ago (success)
  ~/dev/project-b    watch: .roko/prd/     last: 15m ago (running)
  ~/dev/project-c    webhook: POST /hook   last: never
```

---

## 5. IPC: Unix Domain Socket

The daemon exposes a Unix socket at `/tmp/roko-daemon.sock` for
command-and-control from the CLI, TUI, or other local processes. The
protocol is newline-delimited JSON.

### Commands

| Command | What it does |
|---------|-------------|
| `Status` | Get daemon status |
| `RunPlan { repo_path, plan_dir }` | Trigger a plan run |
| `Subscribe { repo_path, schedule }` | Add a subscription |
| `Unsubscribe { repo_path }` | Remove a subscription |
| `ListSubscriptions` | List all subscriptions |
| `StreamEvents { filter }` | Stream events until disconnect |
| `Shutdown` | Graceful shutdown |

### CLI Client

```bash
roko daemon send status
roko daemon send run-plan --repo ~/dev/my-project
roko daemon send subscribe --repo ~/dev/project --cron "*/30 * * * *"
```

---

## 6. Log Management

Daemon logs go to XDG state directory:

- stdout: `~/.local/state/roko/daemon.stdout.log`
- stderr: `~/.local/state/roko/daemon.stderr.log`

```bash
roko daemon logs                # Tail stdout
roko daemon logs --stderr       # Tail stderr
roko daemon logs --lines 100    # Last 100 lines
```

### Log Rotation

launchd does not provide built-in rotation. The daemon implements its own:

- Maximum log file size: 10MB
- Rename to `.log.1`, `.log.2`, `.log.3` on rotation
- Keep at most 3 rotated files (~40MB total)

On macOS 13+, `os_log` integration with Console.app and the unified
logging system is an alternative.

---

## 7. Startup Sequence

When the daemon starts, it follows 13 initialization steps:

1. Parse CLI args and resolve config paths
2. Load global config (`~/.config/roko/config.toml`)
3. Initialize logging (file + optional stderr)
4. Create or connect to Unix domain socket
5. Load subscription list from config
6. For each subscription:
   a. Validate repo path exists
   b. Load repo-local config (`.roko/config.toml`)
   c. Initialize file watcher (if watch mode)
   d. Initialize cron scheduler (if cron mode)
   e. Initialize webhook listener (if webhook mode)
7. Start IPC server (accept commands on socket)
8. Start event bus (internal pub/sub for daemon events)
9. Start health check loop (periodic self-assessment)
10. Start adaptive clock (Gamma/Theta/Delta frequencies)
11. Log startup complete with PID and socket path
12. Enter main event loop (process subscriptions, IPC commands, events)
13. On SIGTERM/SIGINT: graceful shutdown

---

## 8. Graceful Shutdown

On SIGTERM (from `launchctl unload` or `roko daemon stop`):

1. **Stop accepting**: Mark all subscriptions as paused
2. **Drain**: Wait up to 30 seconds for in-progress plan runs
3. **Save state**: Write subscription states and pending events to disk
4. **Close IPC**: Clean up the socket file
5. **Exit cleanly**: Exit code 0

launchd will not restart after a clean exit (only after crash exits, per
the `SuccessfulExit = false` KeepAlive). Force-kills happen if tasks do
not complete within the 30-second drain period.

---

## 9. Environment Variables

launchd runs daemons in a minimal environment -- no `~/.zshrc` or
`~/.bashrc` sourcing. The `roko daemon install` command detects relevant
env vars from the current shell and includes them in the plist:

- `ANTHROPIC_API_KEY` -- if set, included
- `OPENAI_API_KEY` -- if set, included
- `RUST_LOG` -- always included (default: `info`)
- `HOME` -- always included (launchd may not set it)
- `PATH` -- always included with `~/.cargo/bin` appended

For sensitive keys, the daemon can read from the macOS Keychain at runtime
via the `keyring` crate. See `secret-management.md`.

---

## 10. Implementation Status

> **Implementation status:** The `roko daemon` subcommand structure is
> defined in the CLI. Plist generation is implemented but not tested
> end-to-end. File watching uses the `notify` crate (workspace dependency)
> but is not connected to the daemon event loop. Cron scheduling uses the
> `cron` crate but is not integrated. The IPC protocol and subscription
> system are designed but not wired. Daemon mode depends on the
> subscription configuration system being completed. Daemon mode is at
> Tier 3H priority.
