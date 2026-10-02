//! Best-effort background writes that tests can wait for.
//!
//! Graph dispatch appends its learning and cost rows off the reactor, in
//! spawned tasks, so an attempt never waits on the disk. [`spawn`] counts each
//! such write from the moment it is spawned until it ends, however it ends.
//! [`settled`] waits until no write to a file under a directory is still
//! pending, so a test reads its rows once those writes have finished instead
//! of polling the files against a deadline that a loaded machine can miss
//! (bug-779ae7). In production a plan run waits, for a bounded time, so its
//! rows reach the disk before the process exits (bug-2b1ddc, q-1faa0c).

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use tokio::sync::Notify;

/// Writes spawned and not yet ended, by the file each one writes.
static PENDING: Mutex<BTreeMap<PathBuf, usize>> = Mutex::new(BTreeMap::new());

/// Woken each time a write ends.
static ENDED: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Spawn `write`, a best-effort write to `path`, on the current runtime.
///
/// The write counts as pending from this call until its task ends: it
/// finishes, panics, or is dropped with its runtime.
pub(crate) fn spawn<F>(path: &Path, write: F) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let pending = Pending::register(path);
    tokio::spawn(async move {
        write.await;
        drop(pending);
    })
}

/// Wait until no write that [`spawn`] started for a file under `dir` is
/// pending. Writes spawned after this returns are not waited for.
pub(crate) async fn settled(dir: &Path) {
    loop {
        let ended = ENDED.notified();
        tokio::pin!(ended);
        // Registered before the check, so a write ending in between wakes it.
        ended.as_mut().enable();
        if !pending().keys().any(|path| path.starts_with(dir)) {
            return;
        }
        ended.await;
    }
}

fn pending() -> MutexGuard<'static, BTreeMap<PathBuf, usize>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One pending write; dropping it ends the write.
struct Pending(PathBuf);

impl Pending {
    fn register(path: &Path) -> Self {
        *pending().entry(path.to_path_buf()).or_insert(0) += 1;
        Self(path.to_path_buf())
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        {
            let mut pending = pending();
            if let Some(count) = pending.get_mut(&self.0) {
                *count -= 1;
                if *count == 0 {
                    pending.remove(&self.0);
                }
            }
        }
        ENDED.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn settled_waits_for_the_writes_under_a_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("learn/rows.jsonl");
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let written = path.clone();
        spawn(&path, async move {
            released.await.ok();
            std::fs::create_dir_all(written.parent().expect("parent")).expect("dir");
            std::fs::write(&written, "{}\n").expect("row");
        });

        // Another directory's writes are not waited for.
        settled(&temp.path().join("elsewhere")).await;
        let waiting = tokio::spawn({
            let dir = temp.path().to_path_buf();
            async move { settled(&dir).await }
        });
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished(), "the write is still pending");

        release.send(()).expect("release the write");
        waiting.await.expect("settled");
        assert_eq!(std::fs::read_to_string(&path).expect("row"), "{}\n");
    }

    #[tokio::test]
    async fn a_write_that_panics_still_ends() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("rows.jsonl");
        let task = spawn(&path, async { panic!("the write failed") });
        assert!(task.await.is_err());
        settled(temp.path()).await;
    }
}
