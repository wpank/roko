use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs2::FileExt;

/// Acquires an exclusive advisory lock on the workspace.
/// Returns a guard that releases the lock on drop.
/// Fails immediately if another process holds the lock.
pub fn acquire_workspace_lock(roko_dir: &Path) -> Result<WorkspaceLockGuard> {
    let lock_dir = roko_dir.join("runtime");
    fs::create_dir_all(&lock_dir)
        .with_context(|| format!("create lock dir: {}", lock_dir.display()))?;

    let lock_path = lock_dir.join("roko.lock");

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open lock file: {}", lock_path.display()))?;

    match file.try_lock_exclusive() {
        Ok(()) => {
            // Replace stale diagnostics only after acquiring the lock. Opening
            // with `truncate(true)` before `try_lock_exclusive` lets a losing
            // contender erase the live owner's PID.
            file.set_len(0)
                .with_context(|| format!("truncate lock file: {}", lock_path.display()))?;
            let mut f = &file;
            f.seek(SeekFrom::Start(0))
                .with_context(|| format!("seek lock file: {}", lock_path.display()))?;
            writeln!(f, "{}", std::process::id())
                .with_context(|| format!("write lock file: {}", lock_path.display()))?;
            f.sync_data()
                .with_context(|| format!("sync lock file: {}", lock_path.display()))?;
            Ok(WorkspaceLockGuard { file })
        }
        Err(_) => {
            // Read diagnostics after the failed lock attempt so the message
            // reflects the current owner as closely as possible.
            let existing_pid = fs::read_to_string(&lock_path)
                .unwrap_or_default()
                .trim()
                .to_string();
            let pid = if existing_pid.is_empty() {
                "unknown".to_string()
            } else {
                existing_pid
            };
            bail!(
                "Another roko process is running in this workspace (PID {pid}).\n  \
                 hint: wait for it to finish, or kill it with `kill {pid}`"
            );
        }
    }
}

/// RAII guard that releases the file lock on drop.
#[must_use = "lock is released when the guard is dropped"]
pub struct WorkspaceLockGuard {
    file: File,
}

impl Drop for WorkspaceLockGuard {
    fn drop(&mut self) {
        // Clear the diagnostic while the advisory lock is still held. A crash
        // can leave a stale PID, but normal shutdown must not do so.
        let _ = self.file.set_len(0);
        let _ = self.file.sync_data();
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command};
    use std::thread;
    use std::time::{Duration, Instant};

    const HELPER_TEST: &str = "workspace_lock::tests::workspace_lock_process_helper";

    struct LockProcess {
        child: Child,
        release_path: std::path::PathBuf,
        finished: bool,
    }

    impl LockProcess {
        fn spawn(roko_dir: &Path, role: &str) -> Self {
            let release_path = roko_dir.join(format!("{role}-release"));
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", HELPER_TEST, "--ignored", "--nocapture"])
                .env("ROKO_LOCK_TEST_DIR", roko_dir)
                .env("ROKO_LOCK_TEST_ROLE", role)
                .spawn()
                .unwrap();
            Self {
                child,
                release_path,
                finished: false,
            }
        }

        fn pid(&self) -> u32 {
            self.child.id()
        }

        fn finish(mut self) {
            fs::write(&self.release_path, b"release").unwrap();
            let status = self.child.wait().unwrap();
            self.finished = true;
            assert!(status.success(), "lock helper failed with {status}");
        }
    }

    impl Drop for LockProcess {
        fn drop(&mut self) {
            if self.finished {
                return;
            }
            let _ = fs::write(&self.release_path, b"release");
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                match self.child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(_) => break,
                }
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn wait_for(path: &Path, child: &mut Child) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if path.exists() {
                return;
            }
            if let Some(status) = child.try_wait().unwrap() {
                panic!("lock helper exited before {}: {status}", path.display());
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for lock helper at {}", path.display());
    }

    #[test]
    fn contention_does_not_erase_live_owner_pid() {
        let dir = tempfile::tempdir().unwrap();
        let guard = acquire_workspace_lock(dir.path()).unwrap();
        let lock_path = dir.path().join("runtime/roko.lock");
        let owner_pid = fs::read_to_string(&lock_path).unwrap();

        let error = acquire_workspace_lock(dir.path()).err().unwrap();

        assert!(error.to_string().contains(owner_pid.trim()));
        assert_eq!(fs::read_to_string(&lock_path).unwrap(), owner_pid);
        drop(guard);
        assert_eq!(fs::read_to_string(lock_path).unwrap(), "");
    }

    #[test]
    fn separate_process_contenders_never_truncate_owner_pid() {
        let dir = tempfile::tempdir().unwrap();
        let ready_path = dir.path().join("owner-ready");
        let mut owner = LockProcess::spawn(dir.path(), "owner");
        wait_for(&ready_path, &mut owner.child);
        let lock_path = dir.path().join("runtime/roko.lock");
        let owner_pid = format!("{}\n", owner.pid());
        assert_eq!(fs::read_to_string(&lock_path).unwrap(), owner_pid);

        for _ in 0..32 {
            let error = acquire_workspace_lock(dir.path()).err().unwrap();
            assert!(error.to_string().contains(owner_pid.trim()));
            assert_eq!(fs::read_to_string(&lock_path).unwrap(), owner_pid);
        }

        owner.finish();
        assert_eq!(fs::read_to_string(lock_path).unwrap(), "");
    }

    #[test]
    fn normal_release_cannot_clear_the_next_owners_pid() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("runtime/roko.lock");
        let first_owner = acquire_workspace_lock(dir.path()).unwrap();
        let ready_path = dir.path().join("contender-ready");
        let acquired_path = dir.path().join("contender-acquired");
        let mut contender = LockProcess::spawn(dir.path(), "contender");
        wait_for(&ready_path, &mut contender.child);

        drop(first_owner);
        wait_for(&acquired_path, &mut contender.child);
        assert_eq!(
            fs::read_to_string(&lock_path).unwrap(),
            format!("{}\n", contender.pid())
        );

        contender.finish();
        assert_eq!(fs::read_to_string(lock_path).unwrap(), "");
    }

    #[test]
    fn stale_pid_is_replaced_after_lock_is_acquired() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        let lock_path = runtime.join("roko.lock");
        fs::write(&lock_path, "999999\n").unwrap();

        let guard = acquire_workspace_lock(dir.path()).unwrap();

        assert_eq!(
            fs::read_to_string(&lock_path).unwrap(),
            format!("{}\n", std::process::id())
        );
        drop(guard);
        assert_eq!(fs::read_to_string(lock_path).unwrap(), "");
    }

    #[test]
    #[ignore = "subprocess helper; exercised by the parent process tests"]
    fn workspace_lock_process_helper() {
        let roko_dir = std::path::PathBuf::from(
            std::env::var_os("ROKO_LOCK_TEST_DIR").expect("ROKO_LOCK_TEST_DIR"),
        );
        let role = std::env::var("ROKO_LOCK_TEST_ROLE").expect("ROKO_LOCK_TEST_ROLE");
        let ready_path = roko_dir.join(format!("{role}-ready"));
        let release_path = roko_dir.join(format!("{role}-release"));
        let acquired_path = roko_dir.join(format!("{role}-acquired"));

        let guard = if role == "owner" {
            let guard = acquire_workspace_lock(&roko_dir).unwrap();
            fs::write(&ready_path, b"ready").unwrap();
            guard
        } else {
            fs::write(&ready_path, b"ready").unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if let Ok(guard) = acquire_workspace_lock(&roko_dir) {
                    break guard;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out acquiring workspace lock"
                );
                thread::sleep(Duration::from_millis(1));
            }
        };
        fs::write(acquired_path, b"acquired").unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        while !release_path.exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for release signal"
            );
            thread::sleep(Duration::from_millis(10));
        }
        drop(guard);
    }
}
