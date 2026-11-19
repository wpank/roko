//! Workspace server endpoint advertisement.
//!
//! When a `roko serve` process starts it can optionally write a small JSON
//! file at `.roko/runtime/serve.json` so that CLI commands in the same
//! workspace can discover the running server without guessing its port, and
//! the same record into the machine's serve registry
//! (`~/.roko/runtime/serves/<key>.json`, [`registry_dir`]) so that
//! `roko serve status --all` finds every serve.
//!
//! The files are written atomically (temp-file + rename) to avoid partial
//! reads. They are removed when the server exits, and only removed if the PID
//! in the file matches the current process — so a stale file from a previous
//! crashed server is overwritten on the next start but never removed by a
//! sibling process.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Lightweight record that a running server writes to its workspace so that
/// local CLI commands can discover it.
///
/// It also tells the server apart from any other (gap-fb1594): `GET /health`
/// on a loopback bind answers with the same `instance_id`.
///
/// **No secret data** belongs in this file; callers authenticate separately.
/// Every field after `started_at` is missing from a file an older roko wrote,
/// so each one reads as its default then.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeEndpoint {
    /// OS process ID of the server.
    pub pid: u32,
    /// A URL that local clients can connect to (loopback when bound to
    /// `0.0.0.0` or `::`).
    pub url: String,
    /// Canonical absolute path to the workspace directory.
    pub workdir: PathBuf,
    /// RFC-3339 UTC timestamp when the server started.
    pub started_at: String,
    /// A UUID v4 naming this server process; empty in an older file.
    #[serde(default)]
    pub instance_id: String,
    /// The port the listener is bound to.
    #[serde(default)]
    pub port: u16,
    /// The roko version the server runs.
    #[serde(default)]
    pub version: String,
    /// The commit the server binary was built from, when known.
    #[serde(default)]
    pub git_sha: Option<String>,
    /// Whether tracked files differed from that commit at build time, when
    /// known.
    #[serde(default)]
    pub git_dirty: Option<bool>,
    /// Absolute path of the server's executable.
    #[serde(default)]
    pub exe: Option<PathBuf>,
    /// `--idle-exit`: the server exits after this many idle seconds; `None`
    /// never.
    #[serde(default)]
    pub idle_exit_secs: Option<u64>,
    /// What the server was started with, which a restart starts it with again.
    #[serde(default)]
    pub args: ServeArgs,
}

/// What `roko serve` was started with, which `roko serve restart` starts it
/// with again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeArgs {
    /// The address the listener was bound to (`--bind` or `[server] bind`).
    #[serde(default)]
    pub bind: String,
    /// The `--port` the server was given: a port, `0` (the OS picked one), or
    /// `None` (no flag: the configured port).
    #[serde(default)]
    pub port_requested: Option<u16>,
    /// `--enable-terminal`.
    #[serde(default)]
    pub enable_terminal: bool,
    /// `roko --config <file>`: the config file the server reads instead of
    /// the workspace's.
    #[serde(default)]
    pub config: Option<PathBuf>,
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
    write_record(&endpoint_path(workdir), endpoint)
}

/// Atomically write `endpoint` to `dest`, creating its directory.
fn write_record(dest: &Path, endpoint: &ServeEndpoint) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(endpoint)?;

    // Write to a sibling temp file first, then rename for atomicity.
    let tmp = dest.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, dest)?;

    Ok(())
}

/// Read the workspace endpoint file, returning `None` when it is absent or
/// cannot be parsed.
pub fn read_endpoint(workdir: &Path) -> Option<ServeEndpoint> {
    read_record(&endpoint_path(workdir))
}

/// Read one endpoint record, returning `None` when it is absent or cannot be
/// parsed.
pub fn read_record(path: &Path) -> Option<ServeEndpoint> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Remove the workspace endpoint file, but **only** when the file records
/// `pid` as the owner.
///
/// This prevents a server that replaces a stale file from also removing the
/// new owner's advertisement on its own exit.
pub fn remove_endpoint_if_owned(workdir: &Path, pid: u32) {
    remove_record_if_owned(&endpoint_path(workdir), pid);
}

/// Remove the endpoint record at `path` when it records `pid`.
fn remove_record_if_owned(path: &Path, pid: u32) {
    if read_record(path).is_some_and(|endpoint| endpoint.pid == pid) {
        let _ = std::fs::remove_file(path);
    }
}

// ---------------------------------------------------------------------------
// Machine registry (~/.roko/runtime/serves/<key>.json)
// ---------------------------------------------------------------------------

/// The machine's serve registry, `$HOME/.roko/runtime/serves`; `None` when
/// `HOME` is not set.
///
/// It holds one entry per running server that advertises itself, with the
/// same content as its `serve.json`, so `roko serve status --all` finds every
/// serve without knowing the projects.
#[must_use]
pub fn registry_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
    Some(
        PathBuf::from(home)
            .join(".roko")
            .join("runtime")
            .join("serves"),
    )
}

/// The registry key of `workdir`: the first 16 hex digits of the SHA-256 of
/// its (canonical) path.
#[must_use]
pub fn registry_key(workdir: &Path) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(workdir.as_os_str().as_encoded_bytes());
    let mut key = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

/// Where `workdir`'s entry lives in the registry at `dir`.
#[must_use]
pub fn registry_entry_path(dir: &Path, workdir: &Path) -> PathBuf {
    dir.join(format!("{}.json", registry_key(workdir)))
}

/// Atomically write `endpoint` as its workdir's entry in the registry at
/// `dir`.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the file cannot be
/// written, or the rename fails.
pub fn write_registry_entry(dir: &Path, endpoint: &ServeEndpoint) -> anyhow::Result<()> {
    write_record(&registry_entry_path(dir, &endpoint.workdir), endpoint)
}

/// Remove `workdir`'s entry from the registry at `dir`, but **only** when it
/// records `pid`, as [`remove_endpoint_if_owned`] does for `serve.json`.
pub fn remove_registry_entry_if_owned(dir: &Path, workdir: &Path, pid: u32) {
    remove_record_if_owned(&registry_entry_path(dir, workdir), pid);
}

/// Every entry file in the registry at `dir`, sorted, with its record; an
/// entry that cannot be read comes with `None`.
#[must_use]
pub fn read_registry(dir: &Path) -> Vec<(PathBuf, Option<ServeEndpoint>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let record = read_record(&path);
            (path, record)
        })
        .collect()
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
/// The token is written atomically to a sibling temp file that is created
/// with mode 0600, so neither file is ever readable by other users, not even
/// between creation and rename. The temp file is synced before the rename.
///
/// The token **never** goes into `serve.json`; it is a separate file so
/// endpoint payloads remain free of secrets.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the file cannot be
/// created or written, or the rename fails.
pub fn write_token(workdir: &Path, token: &str) -> anyhow::Result<()> {
    use std::io::Write as _;

    let dest = token_path(workdir);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = dest.with_extension("token.tmp");
    // A leftover from a crashed start, possibly with wider permissions:
    // `create_owner_only` refuses to reuse it, so remove it first.
    let _ = std::fs::remove_file(&tmp);
    let mut file = create_owner_only(&tmp)?;
    file.write_all(token.as_bytes())?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, &dest)?;
    Ok(())
}

/// Create `path` for writing, readable and writable only by its owner from
/// the moment it exists (mode 0600 on Unix, narrowed further by the umask).
///
/// Fails if `path` already exists, so it never reuses a file, or follows a
/// link, that someone else put there.
fn create_owner_only(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Remove the workspace token file when the current process owns the endpoint.
///
/// Reads `serve.json` to verify PID ownership before removing `serve.token`.
/// **Must be called before [`remove_endpoint_if_owned`]** so the PID can still
/// be read from the endpoint file.
pub fn remove_token_if_owned(workdir: &Path, pid: u32) {
    if read_endpoint(workdir).is_some_and(|endpoint| endpoint.pid == pid) {
        let _ = std::fs::remove_file(token_path(workdir));
    }
}

/// Remove what a server advertised for `workdir`, each file only while it
/// still records `pid`.
///
/// That is `serve.token`, `serve.json` and its registry entry in `registry`.
/// The token goes first, since its ownership is read from `serve.json`.
pub fn remove_advertisement_if_owned(workdir: &Path, registry: Option<&Path>, pid: u32) {
    remove_token_if_owned(workdir, pid);
    remove_endpoint_if_owned(workdir, pid);
    if let Some(dir) = registry {
        remove_registry_entry_if_owned(dir, workdir, pid);
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
            ..Default::default()
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
            ..Default::default()
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

    /// The token's temp file is owner-only from creation, and a readable
    /// leftover temp file is replaced rather than reused (bug-af3cf4).
    #[cfg(unix)]
    #[test]
    fn write_token_never_leaves_a_readable_temp_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_workdir();
        let runtime = dir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&runtime).expect("create runtime dir");

        // Owner-only from the moment the file exists, before any chmod.
        let probe = runtime.join("probe.tmp");
        drop(create_owner_only(&probe).expect("create probe"));
        let mode = std::fs::metadata(&probe)
            .expect("probe metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "group and other bits set: {mode:o}");
        assert!(
            create_owner_only(&probe).is_err(),
            "an existing file must never be reused"
        );

        // A world-readable leftover from a crashed start.
        let tmp = token_path(dir.path()).with_extension("token.tmp");
        std::fs::write(&tmp, "stale").expect("write stale temp file");
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))
            .expect("widen stale temp file");

        write_token(dir.path(), "tok-secret").expect("write token");
        let path = token_path(dir.path());
        assert_eq!(
            std::fs::read_to_string(&path).expect("read token"),
            "tok-secret"
        );
        let mode = std::fs::metadata(&path)
            .expect("token metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "group and other bits set: {mode:o}");
        assert!(!tmp.exists(), "the temp file must be renamed into place");
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
            ..Default::default()
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
            ..Default::default()
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

    /// gap-fb1594: a `serve.json` an older roko wrote, without the identity
    /// fields, still parses; a new one keeps every field.
    #[test]
    fn identity_fields_round_trip_and_old_files_still_parse() {
        let dir = temp_workdir();
        let old = r#"{"pid": 7, "url": "http://127.0.0.1:6677", "workdir": "/w",
                      "started_at": "2026-10-14T00:00:00Z"}"#;
        std::fs::create_dir_all(endpoint_path(dir.path()).parent().expect("parent"))
            .expect("runtime dir");
        std::fs::write(endpoint_path(dir.path()), old).expect("old serve.json");
        let read = read_endpoint(dir.path()).expect("an old file parses");
        assert_eq!(read.pid, 7);
        assert!(read.instance_id.is_empty());
        assert_eq!(read.args, ServeArgs::default());

        let endpoint = ServeEndpoint {
            pid: 42,
            url: "http://127.0.0.1:6801".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-10-19T10:00:00Z".to_string(),
            instance_id: "6f1c0e2a-0000-4000-8000-000000000000".to_string(),
            port: 6801,
            version: "0.1.0".to_string(),
            git_sha: Some("e31f4d52c".to_string()),
            git_dirty: Some(false),
            exe: Some(PathBuf::from("/usr/local/bin/roko")),
            idle_exit_secs: Some(3600),
            args: ServeArgs {
                bind: "127.0.0.1".to_string(),
                port_requested: Some(0),
                enable_terminal: false,
                config: None,
            },
        };
        write_endpoint(dir.path(), &endpoint).expect("write");
        assert_eq!(read_endpoint(dir.path()), Some(endpoint));
    }

    /// gap-fb1594: the registry keys an entry by the first 16 hex digits of
    /// the SHA-256 of the workdir, and only the owner removes it.
    #[test]
    fn registry_entry_is_keyed_by_workdir_and_removed_only_by_its_owner() {
        let registry = temp_workdir();
        let workdir = PathBuf::from("/Users/x/project");
        let key = registry_key(&workdir);
        assert_eq!(key.len(), 16);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()), "{key}");
        assert_eq!(key, "4faf1baab2320279", "sha256 of the path's bytes");
        assert_ne!(key, registry_key(Path::new("/Users/x/project2")));

        let endpoint = ServeEndpoint {
            pid: 4242,
            url: "http://127.0.0.1:6802".to_string(),
            workdir: workdir.clone(),
            started_at: "2026-10-19T10:00:00Z".to_string(),
            ..Default::default()
        };
        write_registry_entry(registry.path(), &endpoint).expect("write entry");
        let path = registry_entry_path(registry.path(), &workdir);
        assert_eq!(path, registry.path().join(format!("{key}.json")));
        assert_eq!(
            read_registry(registry.path()),
            vec![(path.clone(), Some(endpoint.clone()))]
        );

        remove_registry_entry_if_owned(registry.path(), &workdir, 4243);
        assert!(path.exists(), "another pid must not remove the entry");
        remove_registry_entry_if_owned(registry.path(), &workdir, 4242);
        assert!(!path.exists());
        assert!(read_registry(registry.path()).is_empty());
    }

    /// gap-fb1594: a server's exit removes its token, `serve.json` and
    /// registry entry together, and only while they are its own.
    #[test]
    fn remove_advertisement_takes_token_endpoint_and_registry_entry() {
        let dir = temp_workdir();
        let registry = temp_workdir();
        let endpoint = ServeEndpoint {
            pid: 31337,
            url: "http://127.0.0.1:6803".to_string(),
            workdir: dir.path().to_path_buf(),
            started_at: "2026-10-19T10:00:00Z".to_string(),
            ..Default::default()
        };
        write_endpoint(dir.path(), &endpoint).expect("serve.json");
        write_token(dir.path(), "token").expect("serve.token");
        write_registry_entry(registry.path(), &endpoint).expect("entry");

        remove_advertisement_if_owned(dir.path(), Some(registry.path()), 1);
        assert!(read_endpoint(dir.path()).is_some());
        assert!(token_path(dir.path()).exists());
        assert_eq!(read_registry(registry.path()).len(), 1);

        remove_advertisement_if_owned(dir.path(), Some(registry.path()), 31337);
        assert!(read_endpoint(dir.path()).is_none());
        assert!(!token_path(dir.path()).exists());
        assert!(read_registry(registry.path()).is_empty());
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
