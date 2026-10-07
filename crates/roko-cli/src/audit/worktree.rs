//! The audit worktree (S05 §4.3, 7122): where an audit re-checks a green
//! attempt, out of every agent's reach.
//!
//! [`AuditWorktree::create`] makes a commit of the selection's pinned result
//! tree in the workspace repository and checks it out, detached, at
//! `<vault>/<workspace_id>/worktrees/<sel_id>/`: never in an agent's
//! worktree, and never under `.roko/worktrees`, where plan worktrees live.
//! [`AuditWorktree::restore_tests`] then puts back every test file the
//! attempt changed, by `roko_gate::attempt_diff::is_test_path`, as it was in
//! the base tree, and removes a test file the attempt added, so the checks
//! run against the tests the task was given. Dropping the worktree removes
//! it.

use std::path::{Path, PathBuf};

use roko_gate::attempt_diff::is_test_path;

use super::git;

/// One change between two trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeChange {
    /// `A`dded, `M`odified, `D`eleted, or another git status letter.
    pub status: char,
    /// The path, relative to the tree's root.
    pub path: String,
}

/// A detached worktree of the audit vault.
#[derive(Debug)]
pub struct AuditWorktree {
    repo: PathBuf,
    path: PathBuf,
    removed: bool,
}

impl AuditWorktree {
    /// Check out `tree`, a tree of the repository at `repo`, at `path`.
    ///
    /// # Errors
    ///
    /// Git cannot make the commit or the worktree.
    pub fn create(repo: &Path, path: &Path, tree: &str, message: &str) -> anyhow::Result<Self> {
        let commit = git(repo, &["commit-tree", tree, "-m", message])?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let target = path.to_string_lossy();
        git(
            repo,
            &["worktree", "add", "--detach", "--force", &target, &commit],
        )?;
        Ok(Self {
            repo: repo.to_path_buf(),
            path: path.to_path_buf(),
            removed: false,
        })
    }

    /// The worktree's root.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Put every test file the attempt changed from `base` to `result` back
    /// as `base` held it, and remove a test file the attempt added. Returns
    /// the paths restored or removed.
    ///
    /// # Errors
    ///
    /// Git cannot list the changes or restore a file.
    pub fn restore_tests(&self, base: &str, result: &str) -> anyhow::Result<Vec<String>> {
        let mut restored = Vec::new();
        for change in changed_paths(&self.repo, base, result)? {
            if !is_test_path(&change.path) {
                continue;
            }
            if change.status == 'A' {
                let file = self.path.join(&change.path);
                if file.exists() {
                    std::fs::remove_file(&file)?;
                }
            } else {
                git(&self.path, &["checkout", base, "--", &change.path])?;
            }
            restored.push(change.path);
        }
        Ok(restored)
    }

    /// Remove the worktree and its administrative files.
    ///
    /// # Errors
    ///
    /// Git cannot remove it.
    pub fn remove(mut self) -> anyhow::Result<()> {
        self.removed = true;
        self.remove_now()
    }

    fn remove_now(&self) -> anyhow::Result<()> {
        let target = self.path.to_string_lossy();
        git(&self.repo, &["worktree", "remove", "--force", &target])?;
        git(&self.repo, &["worktree", "prune"])?;
        Ok(())
    }
}

impl Drop for AuditWorktree {
    fn drop(&mut self) {
        if !self.removed
            && let Err(error) = self.remove_now()
        {
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "an audit worktree was not removed"
            );
        }
    }
}

/// The paths that differ between the trees `base` and `result` of the
/// repository at `repo`, renames as a deletion and an addition.
///
/// # Errors
///
/// Git cannot compare the trees.
pub fn changed_paths(repo: &Path, base: &str, result: &str) -> anyhow::Result<Vec<TreeChange>> {
    let listing = git(
        repo,
        &["diff", "--name-status", "--no-renames", base, result],
    )?;
    Ok(listing
        .lines()
        .filter_map(|line| {
            let (status, path) = line.split_once('\t')?;
            Some(TreeChange {
                status: status.chars().next()?,
                path: path.to_string(),
            })
        })
        .collect())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A git repository at `dir` whose base commit holds `files`; returns
    /// its base tree.
    pub(crate) fn repo_with(dir: &Path, files: &[(&str, &str)]) -> String {
        std::fs::create_dir_all(dir).expect("mkdir repo");
        git(dir, &["init", "-q"]).expect("git init");
        for (path, text) in files {
            write(dir, path, text);
        }
        let tree = tree_of(dir);
        git(dir, &["commit", "-q", "-m", "base"]).expect("git commit");
        tree
    }

    /// Write `text` at `path` under `dir`.
    pub(crate) fn write(dir: &Path, path: &str, text: &str) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("mkdir");
        std::fs::write(file, text).expect("write");
    }

    /// The tree of `dir`'s working files, as an attempt would leave it.
    pub(crate) fn tree_of(dir: &Path) -> String {
        git(dir, &["add", "-A"]).expect("git add");
        git(dir, &["write-tree"]).expect("git write-tree")
    }

    #[test]
    fn an_audit_worktree_restores_the_tests_the_attempt_changed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let base = repo_with(
            &repo,
            &[
                ("src/answer.sh", "echo 41\n"),
                ("tests/check.sh", "[ \"$(sh src/answer.sh)\" = 42 ]\n"),
            ],
        );
        write(&repo, "tests/check.sh", "exit 0\n");
        write(&repo, "tests/extra_test.sh", "exit 0\n");
        write(&repo, "src/answer.sh", "echo 41 # tuned\n");
        let result = tree_of(&repo);

        let path = temp.path().join("vault/ws/worktrees/sel-1");
        let worktree =
            AuditWorktree::create(&repo, &path, &result, "audit sel-1").expect("a worktree");
        let mut restored = worktree.restore_tests(&base, &result).expect("restore");
        restored.sort();
        assert_eq!(restored, ["tests/check.sh", "tests/extra_test.sh"]);
        let check = std::fs::read_to_string(path.join("tests/check.sh")).expect("check");
        assert!(check.contains("= 42"), "{check}");
        assert!(!path.join("tests/extra_test.sh").exists());
        let answer = std::fs::read_to_string(path.join("src/answer.sh")).expect("answer");
        assert!(answer.contains("tuned"), "product code is the attempt's");
        worktree.remove().expect("remove");
        assert!(!path.exists());
    }
}
