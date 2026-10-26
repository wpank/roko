//! What a run changed in a git checkout it was meant to leave alone
//! (bug-b7d9ad, reg-d2deba).
//!
//! An attempt that runs in a worktree of its own should change only that. A
//! path compiled in from the person's checkout breaks this: a warm build tree
//! keeps what was built with the checkout's path, and a check that writes
//! through it writes into the person's checkout. [`CheckoutState`] is a
//! checkout's dirty paths as `git status` lists them, untracked files
//! included, each with its size and mtime. Two of them, taken before and
//! after something ran, say what it changed ([`CheckoutState::changes_since`]).
//! roko-cli watches the person's checkout over each try and around its
//! gate's steps; [`super::task_checks::run_checks`] watches it around each
//! run of the try's own checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, SystemTime};

/// How long `git status` may run.
const GIT_TIMEOUT: Duration = Duration::from_mins(1);

/// `git status`'s code of an untracked path.
const UNTRACKED: &str = "??";

/// roko's own directory in a checkout, which it writes during a run: never
/// counted as a change the run made to the checkout.
pub const ROKO_DIR: &str = ".roko";

/// One dirty path of a checkout: its `git status` code, and its size and
/// mtime while it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DirtyPath {
    status: String,
    file: Option<(u64, SystemTime)>,
}

/// The dirty paths of a git checkout (bug-b7d9ad).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckoutState {
    paths: BTreeMap<String, DirtyPath>,
}

impl CheckoutState {
    /// The dirty paths of the git checkout at `dir`, untracked files
    /// included, leaving out the directories `skipped` names (relative to
    /// `dir`); `None` when git cannot list them in time.
    pub async fn of(dir: &Path, skipped: &[String]) -> Option<Self> {
        let mut command = tokio::process::Command::new("git");
        command
            .args([
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
            ])
            .current_dir(dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let output = tokio::time::timeout(GIT_TIMEOUT, command.output())
            .await
            .ok()?
            .ok()
            .filter(|output| output.status.success())?;
        let dir = dir.to_path_buf();
        let skipped = skipped.to_vec();
        tokio::task::spawn_blocking(move || Self::parse(&dir, &output.stdout, &skipped))
            .await
            .ok()
    }

    /// The state `git status --porcelain=v1 -z` printed for the checkout at
    /// `dir`: `XY path`, each ended by a NUL, a rename or copy followed by
    /// the path it came from. A path under a directory of `skipped` is left
    /// out, and so is a nested checkout, which git lists as its directory.
    #[must_use]
    pub fn parse(dir: &Path, porcelain: &[u8], skipped: &[String]) -> Self {
        let mut paths = BTreeMap::new();
        let mut entries = porcelain
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty());
        while let Some(entry) = entries.next() {
            if matches!(entry.first(), Some(b'R' | b'C')) {
                entries.next();
            }
            let Some(path) = entry.get(3..) else {
                continue;
            };
            let path = String::from_utf8_lossy(path).into_owned();
            if path.ends_with('/') || skipped.iter().any(|root| is_under(&path, root)) {
                continue;
            }
            let file = std::fs::metadata(dir.join(&path))
                .ok()
                .and_then(|meta| Some((meta.len(), meta.modified().ok()?)));
            let status = String::from_utf8_lossy(&entry[..2]).into_owned();
            paths.insert(path, DirtyPath { status, file });
        }
        Self { paths }
    }

    /// The dirty paths, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.paths.keys().map(String::as_str)
    }

    /// What changed since `before`, by kind.
    #[must_use]
    pub fn changes_since(&self, before: &Self) -> CheckoutChanges {
        let mut changes = CheckoutChanges::default();
        let all: BTreeSet<&String> = self.paths.keys().chain(before.paths.keys()).collect();
        for path in all {
            let (then, now) = (before.paths.get(path), self.paths.get(path));
            if then == now {
                continue;
            }
            match (then, now) {
                // The person's own changes were there already.
                (Some(_), _) => changes.touched.push(path.clone()),
                (None, Some(now)) if now.status == UNTRACKED => changes.added.push(path.clone()),
                (None, _) => changes.changed.push(path.clone()),
            }
        }
        changes
    }
}

/// Whether `path` is the directory `root` or lies under it, both relative to
/// one checkout.
#[must_use]
pub fn is_under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// What changed in a checkout between two of its states
/// ([`CheckoutState::changes_since`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckoutChanges {
    /// Tracked files that were clean before and changed.
    pub changed: Vec<String>,
    /// Files that were not there before and are now, untracked.
    pub added: Vec<String>,
    /// Paths that had changes of the person's own before and changed again,
    /// or went back: dropping them would drop those changes too.
    pub touched: Vec<String>,
}

impl CheckoutChanges {
    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty() && self.touched.is_empty()
    }

    /// Every path that changed, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .changed
            .iter()
            .chain(&self.added)
            .chain(&self.touched)
            .cloned()
            .collect();
        paths.sort();
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// reg-d2deba: a checkout's state comes from git itself, untracked files
    /// and roko's own directory left out as asked, and two states of it name
    /// a file a run added.
    #[tokio::test]
    async fn a_checkout_state_comes_from_git_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        std::fs::write(dir.path().join("tracked.txt"), "a").expect("tracked");
        git(&["add", "tracked.txt"]);
        git(&[
            "-c",
            "user.email=o@example.test",
            "-c",
            "user.name=O",
            "commit",
            "-qm",
            "base",
        ]);
        std::fs::create_dir_all(dir.path().join(".roko")).expect(".roko");
        std::fs::write(dir.path().join(".roko/state.json"), "{}").expect("state");
        let skipped = [ROKO_DIR.to_string()];
        let before = CheckoutState::of(dir.path(), &skipped)
            .await
            .expect("a state");
        assert_eq!(before.paths().count(), 0, "{before:?}");

        std::fs::create_dir_all(dir.path().join("art")).expect("art");
        std::fs::write(dir.path().join("art/shot.png"), "png").expect("shot");
        std::fs::write(dir.path().join("tracked.txt"), "ab").expect("tracked");
        let after = CheckoutState::of(dir.path(), &skipped)
            .await
            .expect("a state");
        let changes = after.changes_since(&before);
        assert_eq!(changes.added, ["art/shot.png"]);
        assert_eq!(changes.changed, ["tracked.txt"]);
        assert!(changes.touched.is_empty());
        assert_eq!(changes.paths(), ["art/shot.png", "tracked.txt"]);
        assert!(after.changes_since(&after).is_empty());
        assert!(is_under("plans/p1/status.json", "plans/p1"));
        assert!(!is_under("plans/p10.txt", "plans/p1"));
    }
}
