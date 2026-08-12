use std::env;
use std::path::PathBuf;

/// Launchd label for the Roko daemon.
pub const LABEL: &str = "dev.nunchi.roko";

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap()
}

fn exe_path() -> PathBuf {
    env::current_exe().unwrap()
}

fn cargo_bin_path() -> PathBuf {
    home_dir().join(".cargo").join("bin")
}

fn logs_dir() -> PathBuf {
    home_dir().join(".roko").join("logs")
}

fn plist_value(value: impl AsRef<str>) -> String {
    xml_escape(value.as_ref())
}

/// Return the macOS LaunchAgents plist path for the daemon.
#[must_use]
pub fn plist_path() -> PathBuf {
    home_dir()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

/// Generate the launchd plist for the daemon at the given port.
///
/// The plist includes:
/// - `KeepAlive` / `RunAtLoad` for auto-restart on failure and at login.
/// - `WorkingDirectory` set to the user home directory.
/// - `StandardOutPath` / `StandardErrorPath` for structured log capture.
/// - `EnvironmentVariables` with `RUST_LOG=info` and an augmented `PATH`
///   that includes `~/.cargo/bin` so the daemon can invoke cargo-installed
///   tools without a shell sourcing profile files.
#[must_use]
pub fn generate_plist(port: u16) -> String {
    let exe = plist_value(exe_path().display().to_string());
    let home = plist_value(home_dir().display().to_string());
    let stdout_path = plist_value(logs_dir().join("daemon.log").display().to_string());
    let stderr_path = plist_value(logs_dir().join("daemon.err").display().to_string());
    let cargo_bin = cargo_bin_path();
    let path_value = match env::var("PATH") {
        Ok(current_path) if !current_path.is_empty() => {
            format!("{}:{}", cargo_bin.display(), current_path)
        }
        _ => cargo_bin.display().to_string(),
    };
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>daemon</string>
    <string>start</string>
    <string>--foreground</string>
    <string>--port</string>
    <string>{port}</string>
  </array>
  <key>KeepAlive</key>
  <true/>
  <key>RunAtLoad</key>
  <true/>
  <key>WorkingDirectory</key>
  <string>{home}</string>
  <key>StandardOutPath</key>
  <string>{stdout_path}</string>
  <key>StandardErrorPath</key>
  <string>{stderr_path}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>{path}</string>
    <key>RUST_LOG</key>
    <string>{rust_log}</string>
  </dict>
</dict>
</plist>
"#,
        label = LABEL,
        exe = exe,
        port = port,
        home = home,
        stdout_path = stdout_path,
        stderr_path = stderr_path,
        path = plist_value(path_value),
        rust_log = plist_value(rust_log),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_required_keys() {
        let plist = generate_plist(6677);
        assert!(plist.contains("<key>Label</key>"), "Label key missing");
        assert!(plist.contains(LABEL), "label value dev.nunchi.roko missing");
        assert!(
            plist.contains("<string>daemon</string>"),
            "program argument 'daemon' missing"
        );
        assert!(
            plist.contains("<string>start</string>"),
            "program argument 'start' missing"
        );
        assert!(
            plist.contains("<string>--foreground</string>"),
            "program argument '--foreground' missing"
        );
        assert!(
            plist.contains("<string>--port</string>"),
            "program argument '--port' missing"
        );
        assert!(
            plist.contains("<string>6677</string>"),
            "port value missing"
        );
        assert!(
            plist.contains("<key>KeepAlive</key>"),
            "KeepAlive key missing"
        );
        assert!(
            plist.contains("<key>RunAtLoad</key>"),
            "RunAtLoad key missing"
        );
        assert!(
            plist.contains("<key>WorkingDirectory</key>"),
            "WorkingDirectory key missing"
        );
        assert!(
            plist.contains("<key>StandardOutPath</key>"),
            "StandardOutPath key missing"
        );
        assert!(
            plist.contains("<key>StandardErrorPath</key>"),
            "StandardErrorPath key missing"
        );
        assert!(
            plist.contains("daemon.log"),
            "stdout log path daemon.log missing"
        );
        assert!(
            plist.contains("daemon.err"),
            "stderr log path daemon.err missing"
        );
    }

    #[test]
    fn plist_contains_rust_log_env_var() {
        let plist = generate_plist(6677);
        assert!(
            plist.contains("<key>EnvironmentVariables</key>"),
            "EnvironmentVariables dict missing"
        );
        assert!(
            plist.contains("<key>RUST_LOG</key>"),
            "RUST_LOG key missing from EnvironmentVariables"
        );
        assert!(
            plist.contains("<key>PATH</key>"),
            "PATH key missing from EnvironmentVariables"
        );
    }

    #[test]
    fn plist_is_valid_xml_plist_structure() {
        let plist = generate_plist(9090);
        assert!(
            plist.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
            "XML declaration missing"
        );
        assert!(plist.contains("<!DOCTYPE plist"), "plist DOCTYPE missing");
        assert!(
            plist.contains("<plist version=\"1.0\">"),
            "plist tag missing"
        );
        assert!(plist.contains("</plist>"), "closing plist tag missing");
        assert!(plist.contains("<dict>"), "root dict missing");
        assert!(plist.contains("</dict>"), "closing root dict missing");
    }

    #[test]
    fn plist_path_is_in_launch_agents() {
        let path = plist_path();
        assert!(
            path.to_string_lossy().contains("LaunchAgents"),
            "plist path should be under LaunchAgents"
        );
        assert!(
            path.to_string_lossy().contains(LABEL),
            "plist path should contain label"
        );
        assert!(
            path.extension().map_or(false, |e| e == "plist"),
            "plist path should have .plist extension"
        );
    }

    #[test]
    fn xml_escape_handles_special_chars() {
        assert_eq!(xml_escape("a&b"), "a&amp;b");
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(xml_escape("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }
}
