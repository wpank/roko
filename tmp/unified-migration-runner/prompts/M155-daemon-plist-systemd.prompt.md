# M155 — Generate Platform Service Files for Daemon Install

## Objective
Implement `roko daemon install` to generate platform-specific service files. On macOS, write a LaunchAgent plist to `~/Library/LaunchAgents/dev.nunchi.roko.plist` with KeepAlive and log paths. On Linux, write a systemd user unit to `~/.config/systemd/user/roko.service` with Restart=on-failure and WatchdogSec. Wire into the `roko daemon install` and `roko daemon uninstall` CLI commands.

## Scope
- Crates: `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs` (install/uninstall logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon/launchd.rs` (macOS plist, if exists)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon/systemd.rs` (Linux unit, if exists)
- Depth doc: `tmp/unified-depth/14-deployment/` (daemon lifecycle)

## Steps
1. Read existing daemon install infrastructure:
   ```bash
   grep -n 'install\|uninstall\|plist\|systemd\|launchd\|LaunchAgent' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs | head -15
   ls /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon/ 2>/dev/null
   ```

2. Check for existing launchd/systemd modules:
   ```bash
   grep -rn 'mod launchd\|mod systemd\|pub mod launchd\|pub mod systemd' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -5
   ```

3. Implement macOS plist generation:
   ```rust
   #[cfg(target_os = "macos")]
   pub fn generate_plist(binary_path: &Path, log_dir: &Path) -> String {
       format!(r#"<?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
       <key>Label</key>
       <string>dev.nunchi.roko</string>
       <key>ProgramArguments</key>
       <array>
           <string>{binary}</string>
           <string>daemon</string>
           <string>start</string>
           <string>--headless</string>
       </array>
       <key>KeepAlive</key>
       <true/>
       <key>RunAtLoad</key>
       <true/>
       <key>StandardOutPath</key>
       <string>{log_dir}/roko-daemon.stdout.log</string>
       <key>StandardErrorPath</key>
       <string>{log_dir}/roko-daemon.stderr.log</string>
       <key>WorkingDirectory</key>
       <string>{cwd}</string>
   </dict>
   </plist>"#, binary = binary_path.display(), log_dir = log_dir.display(), cwd = "~")
   }
   ```

4. Implement Linux systemd unit generation:
   ```rust
   #[cfg(target_os = "linux")]
   pub fn generate_systemd_unit(binary_path: &Path) -> String {
       format!(r#"[Unit]
   Description=Roko Agent Daemon
   After=network.target

   [Service]
   Type=simple
   ExecStart={binary} daemon start --headless
   Restart=on-failure
   RestartSec=5
   WatchdogSec=60
   Environment=RUST_LOG=info

   [Install]
   WantedBy=default.target
   "#, binary = binary_path.display())
   }
   ```

5. Implement `install` command:
   ```rust
   pub async fn daemon_install() -> Result<()> {
       let binary = std::env::current_exe()?;

       #[cfg(target_os = "macos")]
       {
           let plist_dir = dirs::home_dir().unwrap().join("Library/LaunchAgents");
           fs::create_dir_all(&plist_dir)?;
           let plist_path = plist_dir.join("dev.nunchi.roko.plist");
           let log_dir = dirs::home_dir().unwrap().join("Library/Logs/roko");
           fs::create_dir_all(&log_dir)?;
           fs::write(&plist_path, generate_plist(&binary, &log_dir))?;
           // launchctl load
           Command::new("launchctl").args(["load", plist_path.to_str().unwrap()]).status()?;
       }

       #[cfg(target_os = "linux")]
       {
           let unit_dir = dirs::config_dir().unwrap().join("systemd/user");
           fs::create_dir_all(&unit_dir)?;
           let unit_path = unit_dir.join("roko.service");
           fs::write(&unit_path, generate_systemd_unit(&binary))?;
           // systemctl --user daemon-reload && enable && start
           Command::new("systemctl").args(["--user", "daemon-reload"]).status()?;
           Command::new("systemctl").args(["--user", "enable", "roko"]).status()?;
       }
       Ok(())
   }
   ```

6. Implement `uninstall` command:
   - macOS: `launchctl unload` + remove plist file
   - Linux: `systemctl --user disable roko` + remove unit file + daemon-reload

7. Write tests:
   - Plist generation produces valid XML structure
   - Systemd unit generation produces valid INI structure
   - Binary path is correctly interpolated
   - Install/uninstall are idempotent (running twice doesn't error)

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- daemon
```

## What NOT to do
- Do NOT start the daemon as part of install — only write the service file and register it
- Do NOT add Windows service support — Unix only for now
- Do NOT hardcode paths — use dirs crate and env vars
- Do NOT require root/sudo — user-level services only (LaunchAgents, systemd --user)
- Do NOT add log rotation — the OS handles that for LaunchAgents/systemd
