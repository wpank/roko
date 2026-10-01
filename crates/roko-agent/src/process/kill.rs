//! Graceful process-tree termination with SIGTERM → SIGKILL escalation.
//!
//! The [`kill_tree`] function implements the full shutdown sequence:
//!
//! 1. Close stdin (EOF signal to the child).
//! 2. Wait up to 1200 ms for graceful exit.
//! 3. SIGTERM the entire process group + descendants.
//! 4. Wait up to 800 ms.
//! 5. SIGKILL if still alive.
//!
//! [`KillTreeOnDrop`] sends the same signals when a run's future is dropped
//! before the run could call [`kill_tree`] itself.

use std::time::Duration;
use tokio::process::Child;

use super::group::collect_descendants;
use super::registry::{register_spawned_descendants, unregister_pid};

/// Default grace period before SIGTERM (milliseconds).
pub const GRACE_STDIN_CLOSE_MS: u64 = roko_core::defaults::DEFAULT_GRACE_STDIN_CLOSE_MS;

/// Grace period between SIGTERM and SIGKILL (milliseconds).
pub const GRACE_SIGTERM_MS: u64 = roko_core::defaults::DEFAULT_GRACE_SIGTERM_MS;

/// Kill a child process and its entire process group using escalation.
///
/// Sequence:
/// 1. Drop/close stdin (triggers EOF in the child).
/// 2. Wait `grace` for the child to exit on its own.
/// 3. SIGTERM the process group + all descendants.
/// 4. Wait 800 ms.
/// 5. SIGKILL the process group + all descendants + `child.kill()`.
///
/// Returns `Ok(())` when the child has exited (or was already dead).
pub async fn kill_tree(child: &mut Child, grace: Duration) -> Result<(), std::io::Error> {
    let root_pid = child.id();
    #[cfg(unix)]
    let descendants = root_pid.map_or_else(Vec::new, collect_descendants);
    #[cfg(not(unix))]
    let descendants = Vec::new();

    // Step 1: close stdin to signal EOF.
    // Taking stdin drops the handle, closing the pipe.
    drop(child.stdin.take());

    // Step 2: wait for graceful exit.
    if wait_for_root(child, grace).await? && wait_for_absence(root_pid, &descendants, grace).await?
    {
        return Ok(());
    }

    // Step 3: SIGTERM the process group.
    #[cfg(unix)]
    {
        register_spawned_descendants(&descendants);
        signal_captured(root_pid, &descendants, libc::SIGTERM)?;
    }

    let term_grace = Duration::from_millis(GRACE_SIGTERM_MS);
    let root_exited = wait_for_root(child, term_grace).await?;
    if root_exited && wait_for_absence(root_pid, &descendants, term_grace).await? {
        return Ok(());
    }

    // Step 5: SIGKILL escalation.
    #[cfg(unix)]
    {
        signal_captured(root_pid, &descendants, libc::SIGKILL)?;
    }

    // Also use tokio's built-in kill (sends SIGKILL on unix, TerminateProcess on Windows).
    if child.try_wait()?.is_none() {
        child.kill().await?;
    }
    if !wait_for_root(child, term_grace).await? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "root process did not exit after SIGKILL",
        ));
    }
    if !wait_for_absence(root_pid, &descendants, term_grace).await? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("process tree still present after SIGKILL: {descendants:?}"),
        ));
    }
    Ok(())
}

/// Terminates a child's process tree when dropped while armed.
///
/// `kill_on_drop` kills only the direct child, so a run whose future is
/// dropped (a cancel, the stall watchdog) would leave the subprocesses that
/// child started, such as shells, test runners and builds, running. Dropping
/// an armed guard sends SIGTERM to the child's process group and its
/// descendants, then SIGKILL after [`GRACE_SIGTERM_MS`] from a detached
/// thread, so the drop never blocks for the grace period. It then removes the
/// root from the PID registry.
///
/// Declare it right after the [`Child`], so it drops first: descendants are
/// found through their parent, which only works while the root is alive.
#[must_use = "a guard dropped at once kills the tree"]
pub struct KillTreeOnDrop {
    root_pid: Option<u32>,
    root_reaped: bool,
}

impl KillTreeOnDrop {
    /// Arm a guard for the tree of a child spawned in its own process group
    /// ([`set_process_group`](super::group::set_process_group)).
    pub const fn new(root_pid: Option<u32>) -> Self {
        Self {
            root_pid,
            root_reaped: false,
        }
    }

    /// The root was waited on, so its PID may be reused: signal only its
    /// group, whose ID stays reserved while any member lives.
    pub const fn root_reaped(&mut self) {
        self.root_reaped = true;
    }

    /// Leave the tree alone on drop: the run finished or killed it itself.
    pub const fn disarm(&mut self) {
        self.root_pid = None;
    }
}

impl Drop for KillTreeOnDrop {
    fn drop(&mut self) {
        let Some(root_pid) = self.root_pid.take() else {
            return;
        };
        #[cfg(unix)]
        {
            let (root, descendants) = if self.root_reaped {
                (None, Vec::new())
            } else {
                (Some(root_pid), collect_descendants(root_pid))
            };
            signal_tree(root_pid, root, &descendants, libc::SIGTERM);
            let grace = Duration::from_millis(GRACE_SIGTERM_MS);
            let escalate = move || {
                std::thread::sleep(grace);
                signal_tree(root_pid, root, &descendants, libc::SIGKILL);
                unregister_pid(root_pid);
            };
            if let Err(error) = std::thread::Builder::new()
                .name("roko-kill-tree".into())
                .spawn(escalate)
            {
                tracing::warn!(root_pid, %error, "could not wait to SIGKILL a dropped process tree");
                signal_tree(root_pid, root, &[], libc::SIGKILL);
                unregister_pid(root_pid);
            }
        }
        #[cfg(not(unix))]
        unregister_pid(root_pid);
    }
}

/// Signal the process group `pgid`, then `root` and `descendants` one by one,
/// which also reaches processes that left the group. Best effort: a process
/// that is gone or out of reach is skipped.
#[cfg(unix)]
fn signal_tree(pgid: u32, root: Option<u32>, descendants: &[u32], signal: i32) {
    let _ = signal_pid(-(pgid as i32), signal);
    for pid in root.iter().chain(descendants) {
        let _ = signal_pid(*pid as i32, signal);
    }
}

async fn wait_for_root(child: &mut Child, duration: Duration) -> std::io::Result<bool> {
    if child.try_wait()?.is_some() {
        return Ok(true);
    }
    match tokio::time::timeout(duration, child.wait()).await {
        Ok(result) => result.map(|_| true),
        Err(_) => Ok(false),
    }
}

#[cfg(unix)]
fn signal_captured(root_pid: Option<u32>, descendants: &[u32], signal: i32) -> std::io::Result<()> {
    if let Some(pid) = root_pid {
        signal_pid(-(pid as i32), signal)?;
        signal_pid(pid as i32, signal)?;
    }
    for pid in descendants {
        signal_pid(*pid as i32, signal)?;
    }
    Ok(())
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub(super) fn signal_pid(pid: i32, signal: i32) -> std::io::Result<()> {
    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(err)
    }
}

async fn wait_for_absence(
    root_pid: Option<u32>,
    descendants: &[u32],
    duration: Duration,
) -> std::io::Result<bool> {
    #[cfg(unix)]
    {
        let deadline = tokio::time::Instant::now() + duration;
        loop {
            let mut alive = Vec::new();
            for pid in root_pid.into_iter().chain(descendants.iter().copied()) {
                if pid_is_alive(pid)? {
                    alive.push(pid);
                }
            }
            if alive.is_empty() {
                return Ok(true);
            }
            if tokio::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (root_pid, descendants, duration);
        Ok(true)
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub(super) fn pid_is_alive(pid: u32) -> std::io::Result<bool> {
    if unsafe { libc::kill(pid as i32, 0) } == 0 {
        return Ok(true);
    }
    let err = std::io::Error::last_os_error();
    match err.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::process::Command;

    #[tokio::test]
    async fn kill_tree_terminates_quick_process() {
        let mut cmd = Command::new("sleep");
        cmd.arg("0.01");
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn().expect("failed to spawn sleep");
        let result = kill_tree(&mut child, Duration::from_millis(2000)).await;
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[allow(unsafe_code)]
    #[tokio::test]
    async fn kill_tree_escalates_to_sigkill() {
        use super::super::group::set_process_group;

        // spawn a process that ignores SIGTERM (trap '' TERM; sleep 30)
        let mut cmd = Command::new("bash");
        cmd.args(["-c", "trap '' TERM; sleep 30"]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        set_process_group(&mut cmd);

        let mut child = cmd.spawn().expect("failed to spawn bash");
        let pid = child.id();

        let result = kill_tree(&mut child, Duration::from_millis(200)).await;
        assert!(result.is_ok());

        // Verify process is actually dead.
        if let Some(pid) = pid {
            // Give OS a moment to reap.
            tokio::time::sleep(Duration::from_millis(100)).await;
            let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
            assert!(!alive, "process {pid} should be dead after kill_tree");
        }
    }

    #[tokio::test]
    async fn kill_tree_handles_already_exited() {
        let mut cmd = Command::new("true");
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn().expect("failed to spawn true");
        // Wait for it to finish naturally.
        let _ = child.wait().await;
        // kill_tree should handle an already-exited child gracefully.
        let result = kill_tree(&mut child, Duration::from_millis(100)).await;
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[allow(unsafe_code)]
    #[tokio::test]
    async fn kill_tree_removes_child_and_grandchild_pids() {
        use super::super::group::set_process_group;

        let dir = tempfile::tempdir().expect("tempdir");
        let child_pid_file = dir.path().join("child.pid");
        let grandchild_pid_file = dir.path().join("grandchild.pid");
        let script = format!(
            "trap '' TERM; bash -c 'trap \"\" TERM; sleep 30 & echo $! > \"$1\"; wait' _ '{}' & echo $! > '{}'; wait",
            grandchild_pid_file.display(),
            child_pid_file.display()
        );

        let mut cmd = Command::new("bash");
        cmd.args(["-c", &script]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        set_process_group(&mut cmd);
        let mut root = cmd.spawn().expect("spawn process tree");
        let root_pid = root.id().expect("root pid");

        let child_pid = read_pid_eventually(&child_pid_file).await;
        let grandchild_pid = read_pid_eventually(&grandchild_pid_file).await;
        assert!(pid_is_alive(root_pid).expect("root liveness"));
        assert!(pid_is_alive(child_pid).expect("child liveness"));
        assert!(pid_is_alive(grandchild_pid).expect("grandchild liveness"));

        kill_tree(&mut root, Duration::from_millis(50))
            .await
            .expect("kill process tree");

        for pid in [root_pid, child_pid, grandchild_pid] {
            assert!(
                !pid_is_alive(pid).expect("post-kill liveness"),
                "pid {pid} survived"
            );
        }
    }

    #[cfg(unix)]
    async fn read_pid_eventually(path: &std::path::Path) -> u32 {
        for _ in 0..100 {
            if let Ok(value) = std::fs::read_to_string(path)
                && let Ok(pid) = value.trim().parse()
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("PID file was not written: {}", path.display());
    }
}
