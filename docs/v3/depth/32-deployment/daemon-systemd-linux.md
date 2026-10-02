# 32-deployment/06 -- Daemon Mode: systemd (Linux)

> systemd user unit file, restart policies, journal integration,
> watchdog, security hardening, lingering, and differences from macOS.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-cli/src/commands/daemon.rs`

---

## 1. Overview

The Linux daemon mode mirrors the macOS launchd path with identical
functionality: repository watching, automatic plan execution, IPC over
Unix socket, and event streaming. The difference is the service manager
-- systemd instead of launchd.

The `roko daemon` subcommand detects the platform at runtime:

```rust
fn daemon_install(config: &DaemonConfig) -> Result<()> {
    #[cfg(target_os = "macos")]
    return install_launchd(config);

    #[cfg(target_os = "linux")]
    return install_systemd(config);

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    anyhow::bail!("Daemon mode requires macOS (launchd) or Linux (systemd)");
}
```

---

## 2. systemd Unit File

The generated unit lives at `~/.config/systemd/user/roko.service`. It
uses the user-level systemd instance (no root required).

```ini
[Unit]
Description=Roko cognitive agent daemon
Documentation=https://github.com/nunchi/roko
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/home/USER/.cargo/bin/roko daemon run \
    --socket %t/roko-daemon.sock

Restart=on-failure
RestartSec=10
RestartMaxDelaySec=300
RestartSteps=5

Environment=RUST_LOG=info
Environment=HOME=/home/USER
EnvironmentFile=-%h/.config/roko/daemon.env

LimitNOFILE=4096

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.roko %h/.local/state/roko %h/.config/roko
PrivateTmp=true

WorkingDirectory=%h
WatchdogSec=60

[Install]
WantedBy=default.target
```

---

## 3. Key Configuration Choices

| Directive | Value | Why |
|-----------|-------|-----|
| `Type=simple` | Main process is the daemon | Fast startup (<1s) |
| `Restart=on-failure` | Restart on non-zero exit only | Matches launchd KeepAlive |
| `RestartSec=10` | Initial backoff delay | Prevents restart storms |
| `RestartMaxDelaySec=300` | 5-minute cap on backoff | Bounded recovery time |
| `RestartSteps=5` | 5 steps from 10s to 300s | Exponential backoff |
| `%t/roko-daemon.sock` | XDG_RUNTIME_DIR socket | Per-user, tmpfs, auto-cleaned |
| `EnvironmentFile=-` | `-` prefix = ignore if missing | Optional daemon.env |
| `WatchdogSec=60` | Ping every 60s or killed | Detects hung daemon |

### Security Hardening

- `NoNewPrivileges=true` -- prevents privilege escalation
- `ProtectSystem=strict` -- mounts entire filesystem read-only except
  explicitly allowed paths
- `ProtectHome=read-only` -- read-only home directory
- `ReadWritePaths=...` -- write access only to Roko state directories
- `PrivateTmp=true` -- private /tmp namespace

### Unit File Generation

```rust
fn generate_systemd_unit(config: &DaemonConfig) -> String {
    let binary_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("roko"));
    let home = std::env::var("HOME")
        .unwrap_or_else(|_| String::from("/tmp"));
    format!(r#"[Unit]
Description=Roko cognitive agent daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={binary} daemon run --socket %t/roko-daemon.sock
Restart=on-failure
RestartSec=10
...
"#, binary = binary_path.display())
}
```

---

## 4. Environment and Secrets

The `EnvironmentFile` directive loads secrets from
`~/.config/roko/daemon.env`:

```bash
# ~/.config/roko/daemon.env
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
```

This file can contain API keys and other variables that should not be in
the unit file. The `-` prefix means systemd continues silently if the
file does not exist.

---

## 5. Lifecycle Commands

### Install and Enable

```bash
$ roko daemon install

[1/4] Generating systemd unit file...
[2/4] Writing to ~/.config/systemd/user/roko.service
[3/4] Enabling service (systemctl --user enable roko)...
[4/4] Starting service (systemctl --user start roko)...

Roko daemon installed and running.
  PID:      12345
  Socket:   /run/user/1000/roko-daemon.sock
  Logs:     journalctl --user-unit roko -f
```

The install function runs `systemctl --user daemon-reload`, then `enable`,
then `start`.

### Uninstall

Stops the service, disables it, removes the unit file, and runs
`daemon-reload`.

### Status

```bash
$ roko daemon status

Roko Daemon Status
  State:          active (running) since Mon 2026-04-10 09:15:00
  PID:            12345
  Memory:         45.2M
  CPU:            0.3%
  Socket:         /run/user/1000/roko-daemon.sock
  Config:         ~/.config/roko/config.toml

Subscriptions:
  ~/dev/project-a    cron: */30 * * * *    last: 2h ago (success)
  ~/dev/project-b    watch: plans/         last: 15m ago (running)
```

---

## 6. Log Management via journald

systemd's journald captures all stdout/stderr. No separate log files
needed -- the journal handles rotation, compression, and retention.

```bash
roko daemon logs                       # journalctl --user-unit roko -f
roko daemon logs --lines 100           # -n 100
roko daemon logs --boot                # -b (since last boot)
journalctl --user-unit roko -o json    # Machine-parseable output
journalctl --user-unit roko --since "2026-04-10" --until "2026-04-11"
```

### Structured Logging

The daemon uses `tracing` with `tracing-journald` for structured entries:

```rust
info!(repo = %repo_path.display(), plan = %plan_name, "Plan execution started");
warn!(subscription = %sub_id, error = %err, "Subscription check failed");
```

Structured fields are queryable via journalctl:

```bash
journalctl --user-unit roko REPO=/home/user/dev/project-a
```

---

## 7. Watchdog Integration

The unit file sets `WatchdogSec=60`. The daemon must ping systemd every
60 seconds or be considered hung and restarted.

```rust
use sd_notify::NotifyState;

// In the main event loop (every tick)
let _ = sd_notify::notify(false, &[NotifyState::Watchdog]);

// On startup, notify ready
let _ = sd_notify::notify(false, &[NotifyState::Ready]);
```

Uses the `sd-notify` crate (lightweight, no C dependency). On macOS,
these calls are no-ops.

---

## 8. Lingering: Daemon Without Login

By default, systemd user services only run while the user has an active
login session. For headless servers or CI machines:

```bash
loginctl enable-linger $USER
```

With lingering enabled, the user's systemd instance starts at boot and
persists after logout. The install command checks and prints a note if
lingering is not enabled:

```
Note: systemd user services only run while you're logged in.
For persistent daemon operation on a server, run:
  sudo loginctl enable-linger $USER
```

---

## 9. Socket Path Differences

| Platform | Socket Path | Why |
|----------|-------------|-----|
| macOS | `/tmp/roko-daemon.sock` | No per-user runtime dir |
| Linux | `/run/user/$UID/roko-daemon.sock` | XDG_RUNTIME_DIR, tmpfs |

The CLI auto-detects:

```rust
fn default_socket_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("roko-daemon.sock");
    }
    PathBuf::from("/tmp/roko-daemon.sock")
}
```

The `%t` specifier in the unit file expands to `$XDG_RUNTIME_DIR`,
typically `/run/user/1000` for UID 1000. This directory is per-user,
tmpfs-backed, and cleaned up on logout.

---

## 10. Comparison: launchd vs systemd

| Feature | launchd (macOS) | systemd (Linux) |
|---------|-----------------|-----------------|
| Unit location | `~/Library/LaunchAgents/` | `~/.config/systemd/user/` |
| Start on login | `RunAtLoad = true` | `WantedBy=default.target` + enable |
| Restart on crash | `KeepAlive.SuccessfulExit = false` | `Restart=on-failure` |
| Restart backoff | Flat `ThrottleInterval` | Exponential `RestartSec` |
| Logs | Custom log files | journald (built-in rotation) |
| Socket path | `/tmp/roko-daemon.sock` | `/run/user/$UID/roko-daemon.sock` |
| Security | Sandbox profiles (limited) | Namespaces, capabilities |
| Watchdog | Not built-in | `WatchdogSec` + `sd_notify` |
| Without login | Always for user agents | Requires `enable-linger` |

The daemon provides identical functionality on both platforms. The
`roko daemon` CLI abstracts the differences.

---

## 11. Implementation Status

> **Implementation status:** The systemd integration is at Tier 3H
> priority, same as launchd. Unit file generation is designed but not
> implemented. The IPC protocol, subscription system, and event streaming
> are shared between both platforms. Implementation depends on the daemon
> event loop being wired. The `sd-notify` crate is not yet a workspace
> dependency; the `tracing-journald` subscriber is designed but not
> integrated.
