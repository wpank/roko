//! Git command wrappers, process management, and low-level I/O helpers
//! used by [`super::WorktreeManager`].

use std::ffi::OsString;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use tokio::sync::{OwnedMutexGuard, oneshot};
use tokio_util::sync::CancellationToken;

use super::{
    OperationLifecycle, RuntimeShutdownOwner, WorktreeConfig, WorktreeError, WorktreeManager,
    WorktreeOperationError,
};

// ── Git command methods on WorktreeManager ──

impl WorktreeManager {
    pub(super) async fn git_remove(
        &self,
        path: &Path,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let path_str = path.to_string_lossy().into_owned();
        let output = self
            .git_mutation_output(&["worktree", "remove", "--force", &path_str], lifecycle)
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    }

    pub(super) async fn common_git_dir(&self) -> Result<PathBuf, WorktreeError> {
        let output = self
            .git_policy_output(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        std::fs::canonicalize(raw).map_err(WorktreeError::IoError)
    }

    pub(super) async fn git_ref_oid(
        &self,
        reference: &str,
        required: bool,
    ) -> Result<Option<String>, WorktreeError> {
        let output = self
            .git_policy_output(&["rev-parse", "--verify", reference])
            .await?;
        if !output.status.success() {
            if required {
                return Err(WorktreeError::GitFailed {
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            return Ok(None);
        }
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(WorktreeError::GitFailed {
                stderr: "git returned a malformed object id".to_string(),
            });
        }
        Ok(Some(oid))
    }

    pub(super) fn register_linked_worktree(
        &self,
        path: &Path,
        admin_dir: &Path,
        expected_ref: &str,
    ) -> std::io::Result<()> {
        let admin_parent = admin_dir
            .parent()
            .ok_or_else(|| std::io::Error::other("worktree admin directory has no parent"))?;
        std::fs::create_dir_all(admin_parent)?;
        std::fs::create_dir(admin_dir)?;
        write_new_synced_file(&admin_dir.join("locked"), b"roko create in progress\n")?;
        write_new_synced_file(&admin_dir.join("commondir"), b"../..\n")?;
        write_new_synced_file(
            &admin_dir.join("HEAD"),
            format!("ref: {expected_ref}\n").as_bytes(),
        )?;
        write_new_synced_file(
            &admin_dir.join("gitdir"),
            format!("{}\n", path.join(".git").display()).as_bytes(),
        )?;
        sync_directory(admin_dir)?;
        sync_directory(admin_parent)?;

        std::fs::create_dir(path)?;
        write_new_synced_file(
            &path.join(".git"),
            format!("gitdir: {}\n", admin_dir.display()).as_bytes(),
        )?;
        sync_directory(path)?;
        if let Some(parent) = path.parent() {
            sync_directory(parent)?;
        }
        Ok(())
    }

    pub(super) fn remove_registered_worktree(
        &self,
        path: &Path,
        admin_dir: &Path,
    ) -> std::io::Result<()> {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(std::io::Error::other(
                    "owned incomplete worktree path changed type before rollback",
                ));
            }
            let dot_git = std::fs::read_to_string(path.join(".git"))?;
            if dot_git.trim() != format!("gitdir: {}", admin_dir.display()) {
                return Err(std::io::Error::other(
                    "owned incomplete worktree .git link changed before rollback",
                ));
            }
            std::fs::remove_dir_all(path)?;
        }
        if let Ok(metadata) = std::fs::symlink_metadata(admin_dir) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(std::io::Error::other(
                    "owned worktree admin path changed type before rollback",
                ));
            }
            let gitdir = std::fs::read_to_string(admin_dir.join("gitdir"))?;
            if gitdir.trim() != path.join(".git").to_string_lossy() {
                return Err(std::io::Error::other(
                    "owned worktree admin link changed before rollback",
                ));
            }
            std::fs::remove_dir_all(admin_dir)?;
        }
        if path.exists() || admin_dir.exists() {
            return Err(std::io::Error::other(
                "incomplete worktree remains registered after rollback",
            ));
        }
        Ok(())
    }

    pub(super) async fn validate_git_policy(&self, checkout: bool) -> Result<(), WorktreeError> {
        roko_agent::process::validate_no_descendant_context().map_err(|error| {
            WorktreeError::UnsafeGitExecution {
                reason: error.to_string(),
            }
        })?;
        validate_trusted_executable(&self.git_executable()?, &self.config)?;

        if !checkout {
            return Ok(());
        }

        let config = self
            .git_policy_output(&[
                "config",
                "--includes",
                "--get-regexp",
                "^(filter\\..*\\.(process|smudge)|core\\.fsmonitor|extensions\\.partialclone|remote\\..*\\.promisor)$",
            ])
            .await?;
        if config.status.success() {
            for line in String::from_utf8_lossy(&config.stdout).lines() {
                let mut fields = line.splitn(2, char::is_whitespace);
                let key = fields.next().unwrap_or_default().to_ascii_lowercase();
                let value = fields.next().unwrap_or_default().trim();
                // Managed Git commands override this setting on the command
                // line, so repository/user fsmonitor configuration is inert.
                let harmless_fsmonitor = matches!(key.as_str(), "core.fsmonitor");
                let harmless_promisor = key.ends_with(".promisor")
                    && matches!(value, "false" | "no" | "off" | "0" | "");
                if !harmless_fsmonitor && !harmless_promisor {
                    return Err(WorktreeError::UnsafeGitExecution {
                        reason: format!("unsupported checkout extension `{key}`"),
                    });
                }
            }
        } else if config.status.code() != Some(1) {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&config.stderr).into_owned(),
            });
        }

        let hook = self
            .git_policy_output(&["rev-parse", "--git-path", "hooks/post-checkout"])
            .await?;
        if !hook.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&hook.stderr).into_owned(),
            });
        }
        let hook_path = PathBuf::from(String::from_utf8_lossy(&hook.stdout).trim());
        if is_executable_file(&hook_path) {
            return Err(WorktreeError::UnsafeGitExecution {
                reason: format!("executable post-checkout hook `{}`", hook_path.display()),
            });
        }
        Ok(())
    }

    pub(super) async fn git_policy_output(&self, args: &[&str]) -> Result<Output, WorktreeError> {
        self.git_probe_output_at(&self.config.repo_root, args).await
    }

    pub(super) async fn git_probe_output_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<Output, WorktreeError> {
        let executable = self.git_executable()?;
        validate_trusted_executable(&executable, &self.config)?;
        let mut command = Command::new(executable);
        command
            .current_dir(current_dir)
            .arg("--no-pager")
            .args(["-c", "core.fsmonitor=false"])
            .args(args)
            .stdin(Stdio::null())
            .kill_on_drop(true);
        #[cfg(test)]
        for (key, value) in self.git_probe_environment.lock().iter() {
            command.env(key, value);
        }
        sanitize_git_environment(&mut command);
        roko_agent::process::configure_no_descendant_process(&mut command).map_err(|error| {
            WorktreeError::UnsafeGitExecution {
                reason: error.to_string(),
            }
        })?;
        command.output().await.map_err(WorktreeError::IoError)
    }

    pub(super) async fn git_probe_stdout_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<String, WorktreeError> {
        let output = self.git_probe_output_at(current_dir, args).await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let value = String::from_utf8(output.stdout).map_err(|_| {
            WorktreeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "git probe returned non-UTF-8 output",
            ))
        })?;
        let value = value.trim().to_string();
        if value.is_empty() {
            Err(WorktreeError::GitFailed {
                stderr: "git probe returned empty output".to_string(),
            })
        } else {
            Ok(value)
        }
    }

    pub(super) async fn git_probe_canonical_path_at(
        &self,
        current_dir: &Path,
        args: &[&str],
    ) -> Result<PathBuf, WorktreeError> {
        let value = self.git_probe_stdout_at(current_dir, args).await?;
        let path = PathBuf::from(value);
        let absolute = if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        };
        std::fs::canonicalize(absolute).map_err(WorktreeError::IoError)
    }

    pub(super) async fn git_probe_common_dir_at(
        &self,
        current_dir: &Path,
    ) -> Result<PathBuf, WorktreeError> {
        self.git_probe_canonical_path_at(current_dir, &["rev-parse", "--git-common-dir"])
            .await
    }

    pub(super) async fn git_mutation_output(
        &self,
        args: &[&str],
        lifecycle: &OperationLifecycle,
    ) -> std::io::Result<Output> {
        self.git_mutation_output_at(&self.config.repo_root, args, lifecycle, true)
            .await
    }

    pub(super) async fn git_mutation_output_at(
        &self,
        current_dir: &Path,
        args: &[&str],
        lifecycle: &OperationLifecycle,
        observe_cancellation: bool,
    ) -> std::io::Result<Output> {
        let executable = self.git_executable()?;

        let mut stdout = CommandCapture::new("stdout")?;
        let mut stderr = CommandCapture::new("stderr")?;
        let mut command = Command::new(executable);
        command
            .current_dir(current_dir)
            .arg("--no-pager")
            .args([
                "-c",
                "index.threads=1",
                "-c",
                "checkout.workers=1",
                "-c",
                "core.preloadIndex=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
                "-c",
                "submodule.recurse=false",
            ])
            .args(args)
            .stdin(Stdio::null())
            .stdout(stdout.child_stdio()?)
            .stderr(stderr.child_stdio()?)
            .kill_on_drop(true);
        sanitize_git_environment(&mut command);
        roko_agent::process::configure_no_descendant_process(&mut command)?;
        let mut child = command.spawn()?;

        let status = loop {
            if observe_cancellation && lifecycle.is_cancel_requested() {
                let cleanup = terminate_direct_child(&mut child).await;
                if let Err(error) = cleanup {
                    lifecycle.mark_cleanup_unproved();
                    return Err(error);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "git mutation cancelled because its caller runtime shut down",
                ));
            }

            let child_status = match child.try_wait() {
                Ok(status) => status,
                Err(wait_error) => {
                    if let Err(cleanup_error) = terminate_direct_child(&mut child).await {
                        lifecycle.mark_cleanup_unproved();
                        return Err(std::io::Error::new(
                            cleanup_error.kind(),
                            format!(
                                "Git wait failed ({wait_error}); direct-child cleanup also failed: {cleanup_error}"
                            ),
                        ));
                    }
                    return Err(wait_error);
                }
            };
            if let Some(status) = child_status {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };

        Ok(Output {
            status,
            stdout: stdout.read_all()?,
            stderr: stderr.read_all()?,
        })
    }

    pub(super) fn git_executable(&self) -> std::io::Result<PathBuf> {
        if let Some(executable) = self.resolved_git_executable.lock().clone() {
            return Ok(executable);
        }
        #[cfg(test)]
        let requested = self.git_binary.lock().clone();
        #[cfg(not(test))]
        let requested = PathBuf::from("git");
        let executable = resolve_executable(&requested)?;
        *self.resolved_git_executable.lock() = Some(executable.clone());
        Ok(executable)
    }

    #[cfg(test)]
    pub(super) async fn test_phase_checkpoint(
        &self,
        phase: super::CreationPhase,
        lifecycle: &OperationLifecycle,
    ) -> std::io::Result<()> {
        let barrier = self.phase_barrier.lock().clone();
        let Some(barrier) = barrier.filter(|barrier| barrier.phase == phase) else {
            return Ok(());
        };
        std::fs::write(&barrier.started, b"started")?;
        while !barrier.release.exists() {
            if lifecycle.is_cancel_requested() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "create phase cancelled by caller-runtime shutdown",
                ));
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok(())
    }

    #[cfg(not(test))]
    pub(super) async fn test_phase_checkpoint(
        &self,
        _phase: super::CreationPhase,
        _lifecycle: &OperationLifecycle,
    ) -> std::io::Result<()> {
        Ok(())
    }
}

// ── CommandCapture ──

pub(super) struct CommandCapture {
    path: PathBuf,
    file: std::fs::File,
}

impl CommandCapture {
    pub(super) fn new(stream: &str) -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "roko-git-{}-{stream}.capture",
            uuid::Uuid::new_v4()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok(Self { path, file })
    }

    pub(super) fn child_stdio(&self) -> std::io::Result<Stdio> {
        self.file.try_clone().map(Stdio::from)
    }

    pub(super) fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        self.file.seek(std::io::SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl Drop for CommandCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Free functions ──

pub(super) async fn terminate_direct_child(
    child: &mut tokio::process::Child,
) -> std::io::Result<()> {
    if child.try_wait()?.is_none() {
        child.start_kill()?;
        let _ = child.wait().await?;
    }
    Ok(())
}

pub(super) fn ensure_git_success(output: Output) -> Result<(), WorktreeError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(WorktreeError::GitFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

pub(super) fn resolve_executable(requested: &Path) -> std::io::Result<PathBuf> {
    if requested.components().count() > 1 {
        return std::fs::canonicalize(requested);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(requested);
        if candidate.is_file() {
            return std::fs::canonicalize(candidate);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Git executable `{}` not found on PATH", requested.display()),
    ))
}

pub(super) fn validate_trusted_executable(
    executable: &Path,
    config: &WorktreeConfig,
) -> Result<(), WorktreeError> {
    let metadata = std::fs::symlink_metadata(executable)?;
    if !metadata.file_type().is_file() {
        return Err(WorktreeError::UnsafeGitExecution {
            reason: format!(
                "Git executable `{}` is not a regular file",
                executable.display()
            ),
        });
    }
    if executable.starts_with(&config.repo_root) || executable.starts_with(&config.worktrees_root) {
        return Err(WorktreeError::UnsafeGitExecution {
            reason: "Git executable is inside the managed repository/worktree root".to_string(),
        });
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o6000 != 0 {
            return Err(WorktreeError::UnsafeGitExecution {
                reason: "setuid/setgid Git executables are unsupported".to_string(),
            });
        }
    }
    Ok(())
}

pub(super) fn sanitize_git_environment(command: &mut Command) {
    let mut git_keys: Vec<OsString> = std::env::vars_os()
        .filter_map(|(key, _)| key.to_string_lossy().starts_with("GIT_").then_some(key))
        .collect();
    git_keys.extend(
        command
            .as_std()
            .get_envs()
            .filter(|(key, _)| key.to_string_lossy().starts_with("GIT_"))
            .map(|(key, _)| key.to_os_string()),
    );
    git_keys.sort();
    git_keys.dedup();
    for key in git_keys {
        command.env_remove(key);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat");
}

pub(super) fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = metadata;
        false
    }
}

pub(super) fn sync_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

pub(super) fn write_new_synced_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

pub(super) fn canonicalish(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    let Some(parent) = path.parent() else {
        return path.to_path_buf();
    };
    match std::fs::canonicalize(parent) {
        Ok(parent) => path
            .file_name()
            .map_or(parent.clone(), |name| parent.join(name)),
        Err(_) => path.to_path_buf(),
    }
}

pub(super) fn worktree_list_contains_path(stdout: &[u8], path: &Path) -> bool {
    let expected = canonicalish(path);
    String::from_utf8_lossy(stdout).lines().any(|line| {
        line.strip_prefix("worktree ")
            .is_some_and(|listed| canonicalish(Path::new(listed)) == expected)
    })
}

pub(super) fn worktree_list_contains_branch(stdout: &[u8], branch_ref: &str) -> bool {
    String::from_utf8_lossy(stdout)
        .lines()
        .any(|line| line.strip_prefix("branch ") == Some(branch_ref))
}

pub(super) fn reattach_rejected(id: &str, reason: impl Into<String>) -> WorktreeError {
    WorktreeError::ReattachRejected {
        id: id.to_string(),
        reason: reason.into(),
    }
}

pub(super) async fn await_owned_operation<T, F, Fut>(
    operation: OwnedMutexGuard<()>,
    operation_fn: F,
) -> Result<T, WorktreeError>
where
    T: Send + 'static,
    F: FnOnce(Arc<OperationLifecycle>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, WorktreeError>> + Send + 'static,
{
    let (_lifecycle, result_rx) = start_owned_operation(operation, operation_fn)?;
    receive_owned_operation(result_rx).await
}

pub(super) async fn await_owned_operation_controlled<T, F, Fut>(
    operation: OwnedMutexGuard<()>,
    operation_fn: F,
    cancel: &CancellationToken,
    deadline: Option<tokio::time::Instant>,
) -> Result<T, WorktreeOperationError>
where
    T: Send + 'static,
    F: FnOnce(Arc<OperationLifecycle>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, WorktreeError>> + Send + 'static,
{
    let (lifecycle, mut result_rx) = start_owned_operation(operation, operation_fn)?;
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            lifecycle.request_cancel();
            Err(WorktreeOperationError::Cancelled)
        }
        _ = await_optional_deadline(deadline) => {
            lifecycle.request_cancel();
            Err(WorktreeOperationError::Deadline)
        }
        result = &mut result_rx => receive_owned_operation_result(result).map_err(Into::into),
    }
}

pub(super) async fn await_optional_deadline(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending::<()>().await,
    }
}

fn start_owned_operation<T, F, Fut>(
    operation: OwnedMutexGuard<()>,
    operation_fn: F,
) -> Result<
    (
        Arc<OperationLifecycle>,
        oneshot::Receiver<Result<T, WorktreeError>>,
    ),
    WorktreeError,
>
where
    T: Send + 'static,
    F: FnOnce(Arc<OperationLifecycle>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, WorktreeError>> + Send + 'static,
{
    let lifecycle = Arc::new(OperationLifecycle::default());
    let worker_lifecycle = Arc::clone(&lifecycle);
    let (result_tx, result_rx) = oneshot::channel();
    let (done_tx, done_rx) = oneshot::channel();

    std::thread::Builder::new()
        .name("roko-worktree-mutation".to_string())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(WorktreeError::IoError)?
                    .block_on(operation_fn(Arc::clone(&worker_lifecycle)))
            }))
            .unwrap_or_else(|_| {
                worker_lifecycle.mark_cleanup_unproved();
                Err(WorktreeError::IoError(std::io::Error::other(
                    "runtime-independent worktree mutation worker panicked",
                )))
            });

            if worker_lifecycle.cleanup_was_unproved() {
                // The process tree could not be proved absent. Permanently
                // retain manager ownership rather than permitting a later Git
                // mutation to overlap an unowned descendant.
                std::mem::forget(operation);
            } else {
                drop(operation);
            }
            worker_lifecycle.mark_complete();
            let _ = done_tx.send(());
            let _ = result_tx.send(result);
        })
        .map_err(WorktreeError::IoError)?;

    let sentinel_lifecycle = Arc::clone(&lifecycle);
    tokio::spawn(async move {
        let mut shutdown_owner = RuntimeShutdownOwner::new(sentinel_lifecycle);
        let _ = done_rx.await;
        shutdown_owner.disarm();
    });

    Ok((lifecycle, result_rx))
}

async fn receive_owned_operation<T>(
    result_rx: oneshot::Receiver<Result<T, WorktreeError>>,
) -> Result<T, WorktreeError> {
    receive_owned_operation_result(result_rx.await)
}

fn receive_owned_operation_result<T>(
    result: Result<Result<T, WorktreeError>, oneshot::error::RecvError>,
) -> Result<T, WorktreeError> {
    result.map_err(|_| {
        WorktreeError::IoError(std::io::Error::other(
            "runtime-independent worktree mutation worker ended without a result",
        ))
    })?
}

pub(super) fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn validate_id(id: &str) -> Result<(), WorktreeError> {
    if id.is_empty() {
        return Err(WorktreeError::InvalidId("id is empty".to_string()));
    }
    if id.starts_with('.') || id.starts_with('-') {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not start with `.` or `-`"
        )));
    }
    if id.contains("..") {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not contain `..`"
        )));
    }
    // Characters that would break either a filesystem path or a git ref:
    // `/`, `\`, NUL, whitespace, and the git-ref-forbidden set
    // (`~`, `^`, `:`, `?`, `*`, `[`, ASCII control). Also reject `@{`.
    for ch in id.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | '\0' | '~' | '^' | ':' | '?' | '*' | '[' | '@'
        ) || ch.is_whitespace()
            || ch.is_control();
        if bad {
            return Err(WorktreeError::InvalidId(format!(
                "id `{id}` contains forbidden character `{ch}`"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_reflex_replay_id(id: &str) -> Result<(), WorktreeError> {
    validate_id(id)?;
    let Some(suffix) = id.strip_prefix("reflex-") else {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` is outside the reflex replay namespace"
        )));
    };
    if suffix.len() != 32 || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` is not an exact reflex replay identifier"
        )));
    }
    Ok(())
}

/// Read the `gitdir:` pointer from a worktree's `.git` file.
///
/// In a worktree `.git` is a regular file (not a directory) containing
/// a single line like `gitdir: /repo/.git/worktrees/<name>`. Returns
/// `None` if the file is missing, is a directory, or is unparseable.
pub(super) fn read_gitdir(worktree_path: &Path) -> Option<PathBuf> {
    let git_file = worktree_path.join(".git");
    if git_file.is_dir() {
        // Main repo — the gitdir is the .git directory itself.
        return Some(git_file);
    }
    let content = std::fs::read_to_string(&git_file).ok()?;
    let path_str = content.trim().strip_prefix("gitdir: ")?;
    let p = PathBuf::from(path_str);
    if p.is_absolute() {
        Some(p)
    } else {
        Some(worktree_path.join(p))
    }
}

/// A lock file is considered stale when its mtime is ≥ [`STALE_LOCK_SECS`]
/// seconds in the past.
pub(super) fn is_stale_lock(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    std::time::SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age.as_secs() >= super::STALE_LOCK_SECS)
}

/// G08: Copy isolated config directories from the main repo into a new worktree
/// so concurrent agents do not contend on shared config files.
///
/// We copy (not symlink) `.cursor/` so that each worktree has its own MCP
/// configuration. Failure is non-fatal -- the agent can still run without
/// isolated config; we just log a debug warning.
pub(super) fn isolate_worktree_config(repo_root: &Path, worktree_path: &Path) {
    // Directories to copy for config isolation.
    const ISOLATION_DIRS: &[&str] = &[".cursor"];

    for dir_name in ISOLATION_DIRS {
        let source = repo_root.join(dir_name);
        let target = worktree_path.join(dir_name);
        if !source.is_dir() || target.exists() {
            continue;
        }
        if let Err(err) = copy_dir_shallow(&source, &target) {
            tracing::debug!(
                dir = %dir_name,
                error = %err,
                "failed to copy config directory to worktree (non-fatal)"
            );
        }
    }
}

/// Shallow copy of a directory: creates the target directory and copies all
/// regular files (non-recursively). Subdirectories are created but their
/// contents are not copied -- for `.cursor/` we only need the top-level
/// `mcp.json` and similar config files.
fn copy_dir_shallow(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dest = target.join(entry.file_name());
        if ft.is_file() {
            std::fs::copy(entry.path(), &dest)?;
        } else if ft.is_dir() {
            // Create subdirectory and copy its files too (one level deep).
            std::fs::create_dir_all(&dest)?;
            if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                for sub_entry in sub_entries.flatten() {
                    if sub_entry.file_type().map_or(false, |t| t.is_file()) {
                        let _ = std::fs::copy(sub_entry.path(), dest.join(sub_entry.file_name()));
                    }
                }
            }
        }
    }
    Ok(())
}
