//! The git tree of a checkout as it stands, hashed through a copy of its
//! index (gap-fefdf5, gap-56f761): what a try's tree was when its checks
//! judged it, kept so it can be put back.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// How long one git command of a snapshot may run.
const GIT_TIMEOUT: Duration = Duration::from_mins(1);

/// Tree id of `worktree`'s working tree as it is now; `None` outside a git
/// work tree, or when git fails.
///
/// The tree holds the tracked files and the untracked ones `.gitignore` lets
/// in, hashed into the object store through a copy of the repository's
/// index, which is left untouched. The copy keeps the index's stat cache, so
/// `git add` rehashes only what changed: about 0.1 s on a C++ checkout whose
/// build trees `.gitignore` leaves out.
pub async fn snapshot_tree(worktree: &Path) -> Option<String> {
    let index = git(worktree, &["rev-parse", "--git-path", "index"], None).await?;
    let scratch = scratch_index();
    // A repository without an index yet starts from an empty one.
    let _ = tokio::fs::copy(worktree.join(index.trim()), &scratch).await;
    let tree = async {
        git(worktree, &["add", "--all", "--", "."], Some(&scratch)).await?;
        git(worktree, &["write-tree"], Some(&scratch)).await
    }
    .await;
    let _ = tokio::fs::remove_file(&scratch).await;
    tree.map(|tree| tree.trim().to_string())
        .filter(|tree| !tree.is_empty())
}

/// Stdout of git run in `worktree`, against the index at `index` when
/// given; `None` when git cannot start, fails, or runs past
/// [`GIT_TIMEOUT`].
async fn git(worktree: &Path, args: &[&str], index: Option<&Path>) -> Option<String> {
    let mut command = tokio::process::Command::new("git");
    command
        .current_dir(worktree)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = tokio::time::timeout(GIT_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A fresh path for a scratch index in the system's temporary directory.
fn scratch_index() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("roko-tree-{}-{n}.index", std::process::id()))
}
