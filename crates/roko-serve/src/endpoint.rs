//! Workspace server endpoint advertisement.
//!
//! When a `roko serve` process starts it can optionally write a small JSON
//! file at `.roko/runtime/serve.json` so that CLI commands in the same
//! workspace can discover the running server without guessing its port.
//!
//! The file is written atomically (temp-file + rename) to avoid partial reads.
//! It is removed when the server exits, and only removed if the PID in the
//! file matches the current process — so a stale file from a previous crashed
//! server is overwritten on the next start but never removed by a sibling
//! process.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Lightweight record that a running server writes to its workspace so that
/// local CLI commands can discover it.
///
/// **No secret data** belongs in this file; callers authenticate separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeEndpoint {
    /// OS process ID of the server.
    pub pid: u32,
    /// A URL that local clients can connect to (loopback when bound to
    /// `0.0.0.0` or `::`).
    pub url: String,
    /// Absolute path to the workspace directory.
    pub workdir: PathBuf,
    /// RFC-3339 UTC timestamp when the server started.
    pub started_at: String,
}

/// Return the canonical path where the endpoint file lives.
///
/// Resolves to `<workdir>/.roko/runtime/serve.json`.
pub fn endpoint_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("runtime").join("serve.json")
}

/// Atomically write `endpoint` to the workspace endpoint file.
///
/// The parent directory is created if it does not already exist.  The write is
/// atomic: a temp file is written next to the destination and then renamed,
/// so readers never see a partial file.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the file cannot be
/// written, or the rename fails.
pub fn write_endpoint(workdir: &Path, endpoint: &ServeEndpoint) -> anyhow::Result<()> {
    let dest = endpoint_path(workdir);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(endpoint)?;

    // Write to a sibling temp file first, then rename for atomicity.
    let tmp = dest.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, &dest)?;

    Ok(())
}

/// Read the workspace endpoint file, returning `None` when it is absent or
/// cannot be parsed.
pub fn read_endpoint(workdir: &Path) -> Option<ServeEndpoint> {
    let path = endpoint_path(workdir);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Remove the workspace endpoint file, but **only** when the file records
/// `pid` as the owner.
///
/// This prevents a server that replaces a stale file from also removing the
/// new owner's advertisement on its own exit.
pub fn remove_endpoint_if_owned(workdir: &Path, pid: u32) {
    let path = endpoint_path(workdir);
    if let Some(endpoint) = read_endpoint(workdir) {
        if endpoint.pid == pid {
            let _ = std::fs::remove_file(&path);
        }
    }
}

// ---------------------------------------------------------------------------
// Launch token file (.roko/runtime/serve.token)
// ---------------------------------------------------------------------------

/// Return the canonical path where the launch token file lives.
///
/// Resolves to `<workdir>/.roko/runtime/serve.token`, next to `serve.json`.
pub fn token_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("runtime").join("serve.token")
}

/// Write the launch token to `.roko/runtime/serve.token` with mode 0600.
///
/// The token is written atomically: a temp file is written next to the
/// destination and restricted to owner-read/write before being renamed so
/// it is never world-readable, even transiently.
///
/// The token **never** goes into `serve.json`; it is a separate file so
/// endpoint payloads remain free of secrets.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the file cannot be
/// written, permissions cannot be set, or the rename fails.
pub fn write_token(workdir: &Path, token: &str) -> anyhow::Result<()> {
    let dest = token_path(workdir);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to a sibling temp file, restrict permissions, then rename for atomicity.
    let tmp = dest.with_extension("token.tmp");
    std::fs::write(&tmp, token)?;

    // Restrict to owner-read/write only before renaming.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::rename(&tmp, &dest)?;
    Ok(())
}

/// Remove the workspace token file when the current process owns the endpoint.
///
/// Reads `serve.json` to verify PID ownership before removing `serve.token`.
/// **Must be called before [`remove_endpoint_if_owned`]** so the PID can still
/// be read from the endpoint file.
pub fn remove_token_if_owned(workdir: &Path, pid: u32) {
    if let Some(endpoint) = read_endpoint(workdir) {
        if endpoint.pid == pid {
            let _ = std::fs::remove_file(token_path(workdir));
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build a loopback URL from a bound socket address
// ---------------------------------------------------------------------------

/// Given the `SocketAddr` that the server is listening on and the scheme
/// (`"http"` or `"https"`), return a URL that a local client can use.
///
/// When the server bound to an unspecified address (`0.0.0.0` or `::`) the
/// loopback address (`127.0.0.1` or `::1`) is substituted so the URL is
/// always reachable from the same machine.
pub(crate) fn local_url(addr: std::net::SocketAddr, scheme: &str) -> String {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    let port = addr.port();
    let host: std::net::IpAddr = match addr.ip() {
        IpAddr::V4(v4) if v4.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(v6) if v6.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        other => other,
    };

    match host {
        IpAddr::V4(v4) => format!("{scheme}://{v4}:{port}"),
        IpAddr::V6(v6) => format!("{scheme}://[{v6}]:{port}"),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    #[test]
    fn round_trip_write_and_read() {
        let dir = temp_workdir();
        let endpoint = ServeEndpoint {
            pid: 12345,
            url: "http://127.0.0.1:6677".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-09-28T00:00:00Z".to_string(),
        };

        write_endpoint(dir.path(), &endpoint).expect("write_endpoint should succeed");

        let read = read_endpoint(dir.path()).expect("read_endpoint should return Some");
        assert_eq!(read.pid, 12345);
        assert_eq!(read.url, "http://127.0.0.1:6677");
        assert_eq!(read.workdir, dir.path());
    }

    #[test]
    fn remove_endpoint_if_owned_only_removes_own_file() {
        let dir = temp_workdir();
        let other_pid: u32 = 99999;
        let my_pid: u32 = 11111;

        // Write a file owned by `other_pid`.
        let endpoint = ServeEndpoint {
            pid: other_pid,
            url: "http://127.0.0.1:6677".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-09-28T00:00:00Z".to_string(),
        };
        write_endpoint(dir.path(), &endpoint).expect("write_endpoint should succeed");

        // `my_pid` should NOT remove the file because it doesn't own it.
        remove_endpoint_if_owned(dir.path(), my_pid);
        assert!(
            read_endpoint(dir.path()).is_some(),
            "file should still exist after wrong-pid remove"
        );

        // The actual owner removes the file.
        remove_endpoint_if_owned(dir.path(), other_pid);
        assert!(
            read_endpoint(dir.path()).is_none(),
            "file should be gone after correct-pid remove"
        );
    }

    #[test]
    fn write_token_creates_file_with_restricted_permissions() {
        let dir = temp_workdir();
        write_token(dir.path(), "my-launch-token").expect("write_token should succeed");

        let path = token_path(dir.path());
        assert!(path.exists(), "token file should exist after write");

        let contents = std::fs::read_to_string(&path).expect("read token file");
        assert_eq!(
            contents, "my-launch-token",
            "token file must contain the exact token"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "token file must have mode 0600 (owner read/write only)"
            );
        }
    }

    #[test]
    fn remove_token_if_owned_removes_token_with_correct_pid() {
        let dir = temp_workdir();
        let pid: u32 = 42424;

        let ep = ServeEndpoint {
            pid,
            url: "http://127.0.0.1:6677".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-09-28T00:00:00Z".to_string(),
        };
        write_endpoint(dir.path(), &ep).expect("write endpoint");
        write_token(dir.path(), "secret").expect("write token");

        assert!(
            token_path(dir.path()).exists(),
            "token should exist before removal attempt"
        );

        // Wrong PID must not remove the token.
        remove_token_if_owned(dir.path(), 99999);
        assert!(
            token_path(dir.path()).exists(),
            "token must survive a wrong-pid remove attempt"
        );

        // Correct PID removes the token.
        remove_token_if_owned(dir.path(), pid);
        assert!(
            !token_path(dir.path()).exists(),
            "token must be removed after a correct-pid remove"
        );
    }

    #[test]
    fn token_is_removed_before_endpoint_on_shutdown_sequence() {
        let dir = temp_workdir();
        let pid: u32 = 55555;

        let ep = ServeEndpoint {
            pid,
            url: "http://127.0.0.1:6677".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-09-28T00:00:00Z".to_string(),
        };
        write_endpoint(dir.path(), &ep).expect("write endpoint");
        write_token(dir.path(), "shutdown-token").expect("write token");

        // Simulate the shutdown sequence: token first, then endpoint.
        remove_token_if_owned(dir.path(), pid);
        remove_endpoint_if_owned(dir.path(), pid);

        assert!(
            !token_path(dir.path()).exists(),
            "serve.token must be gone after shutdown"
        );
        assert!(
            read_endpoint(dir.path()).is_none(),
            "serve.json must be gone after shutdown"
        );
    }

    #[test]
    fn local_url_replaces_unspecified_v4() {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 6677));
        assert_eq!(local_url(addr, "http"), "http://127.0.0.1:6677");
    }

    #[test]
    fn local_url_replaces_unspecified_v6() {
        use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
        let addr = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 6677, 0, 0));
        assert_eq!(local_url(addr, "https"), "https://[::1]:6677");
    }

    #[test]
    fn local_url_keeps_specific_address() {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 8080));
        assert_eq!(local_url(addr, "http"), "http://10.0.0.1:8080");
    }
}
