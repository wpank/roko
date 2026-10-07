//! The audit vault (S05 §4.4): where hidden tests, audit keys, the audit
//! ledger, incident records and audit worktrees live, outside every workdir
//! an agent works in.
//!
//! The vault root is `$ROKO_AUDIT_HOME`, else `~/.roko/audit`. It holds one
//! `<workspace_id>/` directory per workspace, mode 0700, with the subpaths
//! [`VAULT_SUBDIRS`]. It is not the workspace's agent-readable `.roko/audit/`
//! log (roko-runtime's `messages.jsonl` and the redacted audit mirror), so
//! [`AuditVault::resolve`] refuses a vault inside the workspace root (its
//! `.roko/worktrees` included) and one under a `HOME` that is the workdir,
//! where `~/.roko/audit` is the workspace's own `.roko/audit` (the case
//! [`crate::child_env::is_key_file`] describes). Paths are compared after
//! symlinks are resolved, so a symlink or a `..` cannot hide a vault inside
//! the workdir, nor reach one from it ([`is_vault_path`]).

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// The environment variable that names the vault root.
pub const AUDIT_HOME_ENV: &str = "ROKO_AUDIT_HOME";

/// The subdirectories of a workspace's vault directory.
pub const VAULT_SUBDIRS: [&str; 5] = ["keys", "ledger", "hidden", "incidents", "worktrees"];

/// Why the vault cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum AuditVaultError {
    /// Neither `ROKO_AUDIT_HOME` nor `HOME` is set.
    #[error("cannot place the audit vault: neither ROKO_AUDIT_HOME nor HOME is set")]
    NoHome,
    /// The vault would sit where agents work.
    #[error(
        "the audit vault {vault} lies inside the workspace {workspace}, where agents can read it; \
         set ROKO_AUDIT_HOME to a directory outside it"
    )]
    InsideWorkspace {
        /// The vault root, symlinks resolved.
        vault: PathBuf,
        /// The workspace root, symlinks resolved.
        workspace: PathBuf,
    },
    /// The vault could not be created or read.
    #[error("cannot prepare the audit vault {path}: {source} (ROKO_AUDIT_HOME names its root)")]
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
}

/// A workspace's audit vault: the canonical root and its `<workspace_id>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditVault {
    root: PathBuf,
    workspace_id: String,
    dir: PathBuf,
}

impl AuditVault {
    /// The vault of the workspace at `workspace_root`, from `ROKO_AUDIT_HOME`
    /// or `~/.roko/audit`, created with mode 0700.
    pub fn resolve(workspace_root: &Path) -> Result<Self, AuditVaultError> {
        let audit_home = std::env::var_os(AUDIT_HOME_ENV).filter(|value| !value.is_empty());
        let home = std::env::var_os("HOME").filter(|value| !value.is_empty());
        Self::resolve_with(
            workspace_root,
            audit_home.as_deref().map(Path::new),
            home.as_deref().map(Path::new),
        )
    }

    /// [`Self::resolve`] with the two variables given: `audit_home` for
    /// `ROKO_AUDIT_HOME` and `home` for `HOME`.
    pub fn resolve_with(
        workspace_root: &Path,
        audit_home: Option<&Path>,
        home: Option<&Path>,
    ) -> Result<Self, AuditVaultError> {
        let workspace = workspace_root
            .canonicalize()
            .map_err(|source| AuditVaultError::Io {
                path: workspace_root.to_path_buf(),
                source,
            })?;
        let wanted = root_path(audit_home, home).ok_or(AuditVaultError::NoHome)?;
        // Refuse before creating anything inside the workspace.
        let inside = |vault: &Path| AuditVaultError::InsideWorkspace {
            vault: vault.to_path_buf(),
            workspace: workspace.clone(),
        };
        let planned = canonical_lenient(&wanted);
        if planned.starts_with(&workspace) {
            return Err(inside(&planned));
        }
        create_private_dir(&wanted)?;
        let root = wanted
            .canonicalize()
            .map_err(|source| AuditVaultError::Io {
                path: wanted.clone(),
                source,
            })?;
        if root.starts_with(&workspace) {
            return Err(inside(&root));
        }
        let workspace_id = workspace_id(&workspace);
        let dir = root.join(&workspace_id);
        create_private_dir(&dir)?;
        Ok(Self {
            root,
            workspace_id,
            dir,
        })
    }

    /// The vault root, symlinks resolved.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The stable id of the workspace: a digest of its canonical root.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The workspace's vault directory, `<root>/<workspace_id>`.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `keys/`: the workspace audit secret.
    #[must_use]
    pub fn keys_dir(&self) -> PathBuf {
        self.dir.join("keys")
    }

    /// `ledger/`: the hash-chained audit ledger.
    #[must_use]
    pub fn ledger_dir(&self) -> PathBuf {
        self.dir.join("ledger")
    }

    /// `hidden/`: the hidden test suites.
    #[must_use]
    pub fn hidden_dir(&self) -> PathBuf {
        self.dir.join("hidden")
    }

    /// `incidents/`: incident records.
    #[must_use]
    pub fn incidents_dir(&self) -> PathBuf {
        self.dir.join("incidents")
    }

    /// `worktrees/`: the audit worker's worktrees.
    #[must_use]
    pub fn worktrees_dir(&self) -> PathBuf {
        self.dir.join("worktrees")
    }

    /// Whether `path`, once symlinks are resolved, lies in this vault.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        canonical_lenient(path).starts_with(&self.root)
    }
}

/// The stable id of a workspace: the first 16 hex digits of the BLAKE3
/// digest of its canonical root.
#[must_use]
pub fn workspace_id(canonical_root: &Path) -> String {
    let digest = blake3::hash(canonical_root.as_os_str().as_encoded_bytes());
    digest.to_hex()[..16].to_string()
}

/// The vault root this process would use, symlinks resolved as far as it
/// exists, without creating it: `ROKO_AUDIT_HOME`, else `~/.roko/audit`.
#[must_use]
pub fn vault_root() -> Option<PathBuf> {
    let audit_home = std::env::var_os(AUDIT_HOME_ENV).filter(|value| !value.is_empty());
    let home = std::env::var_os("HOME").filter(|value| !value.is_empty());
    root_path(
        audit_home.as_deref().map(Path::new),
        home.as_deref().map(Path::new),
    )
    .map(|root| canonical_lenient(&root))
}

/// The roots agents are kept out of: [`vault_root`] and, when
/// `ROKO_AUDIT_HOME` points elsewhere, the default `~/.roko/audit` too,
/// which may still hold an earlier vault.
#[must_use]
pub fn vault_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").filter(|value| !value.is_empty());
    let default = root_path(None, home.as_deref().map(Path::new));
    let mut roots: Vec<PathBuf> = vault_root().into_iter().collect();
    if let Some(root) = default.map(|root| canonical_lenient(&root))
        && !roots.contains(&root)
    {
        roots.push(root);
    }
    roots
}

/// Whether `path`, once symlinks and `..` are resolved, lies in the vault
/// ([`vault_roots`]). The sandbox refuses such a path (7118, 7119).
#[must_use]
pub fn is_vault_path(path: &Path) -> bool {
    vault_roots().iter().any(|root| is_under_root(path, root))
}

/// Whether `path`, once symlinks and `..` are resolved, lies under `root`,
/// itself resolved.
#[must_use]
pub fn is_under_root(path: &Path, root: &Path) -> bool {
    let root = canonical_lenient(root);
    canonical_lenient(path).starts_with(root)
}

/// The vault root as configured: `audit_home`, a leading `~/` read as
/// `home`, else `home/.roko/audit`.
fn root_path(audit_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    match audit_home {
        Some(path) => match path.strip_prefix("~") {
            Ok(rest) => home.map(|home| home.join(rest)),
            Err(_) => Some(path.to_path_buf()),
        },
        None => home.map(|home| home.join(".roko").join("audit")),
    }
}

/// `path` made absolute, with symlinks resolved in the part that exists and
/// `.` and `..` resolved in the rest.
fn canonical_lenient(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut missing: Vec<OsString> = Vec::new();
    let mut existing = absolute.as_path();
    let mut base = loop {
        if let Ok(canonical) = existing.canonicalize() {
            break canonical;
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                existing = parent;
            }
            _ => break existing.to_path_buf(),
        }
    };
    for part in missing.iter().rev() {
        match Path::new(part).components().next() {
            Some(Component::ParentDir) => {
                base.pop();
            }
            Some(Component::CurDir) | None => {}
            Some(_) => base.push(part),
        }
    }
    base
}

/// Create `path` and its missing parents with mode 0700, and make `path`
/// itself 0700.
fn create_private_dir(path: &Path) -> Result<(), AuditVaultError> {
    let io = |source| AuditVaultError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        builder.mode(0o700);
        builder.create(path).map_err(io)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    #[cfg(not(unix))]
    builder.create(path).map_err(io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_vault_refuses_a_home_inside_the_workdir() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        let root = workspace.path();

        // A vault inside the workspace root, or its plan worktrees.
        for inner in [root.join("vault"), root.join(".roko/worktrees/vault")] {
            let error = AuditVault::resolve_with(root, Some(&inner), None).expect_err("inside");
            assert!(
                matches!(error, AuditVaultError::InsideWorkspace { .. }),
                "{error}"
            );
            assert!(error.to_string().contains("ROKO_AUDIT_HOME"), "{error}");
            assert!(!inner.exists(), "nothing is created inside the workspace");
        }

        // One reached through a symlink outside the workspace.
        #[cfg(unix)]
        {
            std::fs::create_dir(root.join("secrets")).expect("dir");
            let link = outside.path().join("link");
            std::os::unix::fs::symlink(root.join("secrets"), &link).expect("symlink");
            let error = AuditVault::resolve_with(root, Some(&link), None).expect_err("symlink");
            assert!(
                matches!(error, AuditVaultError::InsideWorkspace { .. }),
                "{error}"
            );
        }

        // A HOME that is the workdir puts ~/.roko/audit in the workspace.
        let error = AuditVault::resolve_with(root, None, Some(root)).expect_err("home");
        assert!(error.to_string().contains("ROKO_AUDIT_HOME"), "{error}");
        assert!(!root.join(".roko/audit").exists());

        // A directory outside is accepted, private, and stable.
        let wanted = outside.path().join("audit");
        let vault = AuditVault::resolve_with(root, Some(&wanted), Some(root)).expect("outside");
        assert_eq!(vault.root(), wanted.canonicalize().expect("canonical"));
        assert_eq!(vault.dir(), vault.root().join(vault.workspace_id()));
        assert_eq!(vault.workspace_id().len(), 16);
        assert_eq!(vault.hidden_dir(), vault.dir().join("hidden"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for dir in [vault.root(), vault.dir()] {
                let mode = std::fs::metadata(dir)
                    .expect("metadata")
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o700, "{}", dir.display());
            }
        }
        let again = AuditVault::resolve_with(root, Some(&wanted), None).expect("again");
        assert_eq!(again, vault);
        assert!(vault.contains(&vault.hidden_dir().join("suite/../suite.py")));
        assert!(!vault.contains(&root.join(".roko/audit/messages.jsonl")));
    }

    #[test]
    fn vault_paths_are_found_through_dot_dot_and_symlinks() {
        let outside = tempfile::tempdir().expect("outside");
        let vault = outside.path().join("vault");
        std::fs::create_dir_all(vault.join("ws/hidden")).expect("vault");
        let workdir = outside.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        assert!(is_under_root(
            &workdir.join("../vault/ws/hidden/x.py"),
            &vault
        ));
        assert!(is_under_root(&vault.join("not-yet/created"), &vault));
        assert!(!is_under_root(
            &workdir.join(".roko/audit/messages.jsonl"),
            &vault
        ));
        #[cfg(unix)]
        {
            let link = workdir.join("peek");
            std::os::unix::fs::symlink(vault.join("ws"), &link).expect("symlink");
            assert!(is_under_root(&link.join("hidden/x.py"), &vault));
        }
        assert_eq!(
            root_path(Some(Path::new("~/v")), Some(Path::new("/h"))),
            Some(PathBuf::from("/h/v"))
        );
        assert_eq!(
            root_path(None, Some(Path::new("/h"))),
            Some(PathBuf::from("/h/.roko/audit"))
        );
        assert_eq!(root_path(None, None), None);
    }
}
