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
}
