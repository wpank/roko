//! What a Graph task changed in its working tree, for the pre-verify screen
//! (`red_flags`).
//!
//! Before the provider runs, dispatch records the tree the task starts from
//! ([`GraphTaskDispatcher::record_diff_base`]), once per task and run: the
//! worktree lease's base revision, or else a snapshot of the shared working
//! tree hashed through a throwaway copy of the index, so the repository's own
//! index is never touched. [`GraphTaskDispatcher::attempt_diff`] diffs that
//! base against a snapshot of the tree as the attempt left it. The diff spans
//! the task's attempts, so an edit an earlier attempt left in the tree still
//! counts. In a shared tree, paths declared by sibling tasks whose attempts
//! overlapped the task's are left out, since those edits are theirs, and so
//! is roko's own `.roko/` state.
//!
//! Git trouble never fails an attempt: without a base or a snapshot, the
//! screen skips its diff checks.

use std::process::Stdio;

use tokio::io::AsyncReadExt as _;

use super::sibling_settle::{WriterMark, declares};
use super::*;

/// Longest a snapshot or diff command may run.
const GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Most bytes of `git diff --raw` output read.
const RAW_LIMIT: usize = 16 * 1024 * 1024;

/// Most bytes of patch text the stub check reads.
const PATCH_LIMIT: usize = 4 * 1024 * 1024;

/// Most bytes of one file's text read.
const BLOB_LIMIT: usize = 2 * 1024 * 1024;

/// Most changed paths the patch for the stub check names.
const PATCH_PATHS: usize = 500;

/// The tree a task's attempts are diffed against.
struct DiffBase {
    workdir: PathBuf,
    tree: String,
    /// Where the window of sibling edits in a shared tree starts; `None` for
    /// a worktree lease base, which no sibling edits.
    siblings: Option<WriterMark>,
    /// The attempt that snapshotted the base; `None` for a lease base.
    snapshot_by: Option<String>,
}

/// Every task's base in this process, keyed by its attempt chain
/// (`"{run}:{plan}:{task}"`).
#[derive(Default)]
pub(super) struct DiffBases {
    bases: parking_lot::Mutex<HashMap<String, DiffBase>>,
}

/// One path a task changed, as `git diff --raw` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChangedPath {
    /// Git's status letter: `A`, `C`, `D`, `M`, `R` or `T`.
    pub(super) status: char,
    /// The path now, or the deleted path.
    pub(super) path: String,
    /// The path before a rename or copy.
    pub(super) old_path: Option<String>,
    /// The blob before the change, when the path existed.
    pub(super) old_blob: Option<String>,
    /// The blob after the change, when the path exists.
    pub(super) new_blob: Option<String>,
}

/// What a task changed since its base, as the screen sees it.
pub(super) struct AttemptDiff {
    workdir: PathBuf,
    base: String,
    result: String,
    /// The changes attributed to the task, in git's order.
    pub(super) changes: Vec<ChangedPath>,
    /// Changed paths left out because a sibling task declares them.
    pub(super) sibling_paths: Vec<String>,
    /// This attempt snapshotted the base, so no earlier attempt of the task
    /// ran in this process: the work of an earlier process is in the base.
    pub(super) base_is_this_attempts: bool,
}

impl GraphTaskDispatcher {
    /// Record the tree the task of `attempt_key` starts from in `workdir`,
    /// unless an earlier attempt of the task in this run did: the lease's
    /// `lease_base` revision when there is one, else a snapshot of the tree.
    pub(super) async fn record_diff_base(
        &self,
        attempt_key: &str,
        workdir: &Path,
        lease_base: Option<&str>,
    ) {
        let chain = attempt_chain(attempt_key).to_string();
        if self
            .diff_bases
            .bases
            .lock()
            .get(&chain)
            .is_some_and(|base| base.workdir == workdir)
        {
            return;
        }
        let lease_tree = match lease_base.map(str::trim).filter(|rev| !rev.is_empty()) {
            Some(rev) => {
                let tree = format!("{rev}^{{tree}}");
                git_line(
                    workdir,
                    &["rev-parse", "--verify", "--quiet", tree.as_str()],
                    None,
                )
                .await
            }
            None => None,
        };
        let base = if let Some(tree) = lease_tree {
            DiffBase {
                workdir: workdir.to_path_buf(),
                tree,
                siblings: None,
                snapshot_by: None,
            }
        } else {
            // Taken before the snapshot: a sibling that starts meanwhile
            // may already be editing the tree.
            let siblings = self.in_flight.mark();
            let Some(tree) = snapshot_tree(workdir).await else {
                tracing::debug!(
                    workdir = %workdir.display(),
                    "no git snapshot of the working tree; the pre-verify screen skips its diff checks"
                );
                return;
            };
            DiffBase {
                workdir: workdir.to_path_buf(),
                tree,
                siblings: Some(siblings),
                snapshot_by: Some(attempt_key.to_string()),
            }
        };
        self.diff_bases.bases.lock().insert(chain, base);
    }

    /// Forget the base of `attempt_key`'s task, once the task has passed.
    pub(super) fn forget_diff_base(&self, attempt_key: &str) {
        self.diff_bases
            .bases
            .lock()
            .remove(attempt_chain(attempt_key));
    }

    /// What the task of `attempt_key` changed in `workdir` since its base.
    /// `None` when dispatch recorded no base (the streaming path records
    /// none yet), or git could not snapshot or diff the tree.
    pub(super) async fn attempt_diff(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        workdir: &Path,
    ) -> Option<AttemptDiff> {
        let (base, siblings, base_is_this_attempts) = {
            let bases = self.diff_bases.bases.lock();
            let base = bases
                .get(attempt_chain(attempt_key))
                .filter(|base| base.workdir == workdir)?;
            (
                base.tree.clone(),
                base.siblings.clone(),
                base.snapshot_by.as_deref() == Some(attempt_key),
            )
        };
        let result = snapshot_tree(workdir).await?;
        let (raw, _) = git_bytes(
            workdir,
            &[
                "diff",
                "--raw",
                "-z",
                "-M",
                "--no-abbrev",
                "--relative",
                base.as_str(),
                result.as_str(),
            ],
            None,
            RAW_LIMIT,
        )
        .await?;
        let task_key = format!("{}/{}", spec.plan_id, task.id);
        let sibling_files = siblings
            .map(|mark| {
                self.in_flight
                    .sibling_files_since(&mark, &task_key, workdir)
            })
            .unwrap_or_default();
        let mut changes = Vec::new();
        let mut sibling_paths = Vec::new();
        for change in parse_raw_diff(&raw) {
            if is_roko_state(&change.path) {
                continue;
            }
            let siblings_own = std::iter::once(&change.path)
                .chain(&change.old_path)
                .any(|path| {
                    sibling_files
                        .iter()
                        .any(|declared| declares(declared, Path::new(path)))
                });
            if siblings_own {
                sibling_paths.push(change.path);
            } else {
                changes.push(change);
            }
        }
        Some(AttemptDiff {
            workdir: workdir.to_path_buf(),
            base,
            result,
            changes,
            sibling_paths,
            base_is_this_attempts,
        })
    }
}

impl AttemptDiff {
    /// Unified diff, without context, of the changes attributed to the task;
    /// `None` when there are none, too many to read, or git fails.
    pub(super) async fn patch(&self) -> Option<String> {
        if self.changes.is_empty() || self.changes.len() > PATCH_PATHS {
            return None;
        }
        let mut args: Vec<String> = [
            "diff",
            "-p",
            "-M",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=0",
            "--relative",
        ]
        .map(String::from)
        .to_vec();
        args.extend([self.base.clone(), self.result.clone(), "--".to_string()]);
        // A rename's old path too, so git pairs it with the new one.
        for change in &self.changes {
            for path in std::iter::once(&change.path).chain(&change.old_path) {
                args.push(format!(":(literal){path}"));
            }
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let (patch, _) = git_bytes(&self.workdir, &args, None, PATCH_LIMIT).await?;
        Some(String::from_utf8_lossy(&patch).into_owned())
    }

    /// Text of `blob`, or `None` when it is binary, too large, or unreadable.
    pub(super) async fn blob_text(&self, blob: &str) -> Option<String> {
        let (bytes, truncated) =
            git_bytes(&self.workdir, &["cat-file", "blob", blob], None, BLOB_LIMIT).await?;
        if truncated || bytes.contains(&0) {
            return None;
        }
        String::from_utf8(bytes).ok()
    }
}

/// The attempt chain `"{run}:{plan}:{task}"` of an attempt key
/// (`"{run}:{plan}:{task}:{n}"`).
fn attempt_chain(attempt_key: &str) -> &str {
    attempt_key
        .rsplit_once(':')
        .map_or(attempt_key, |(chain, _)| chain)
}

/// Whether `path` is roko's own state, which the harness writes throughout
/// a run.
fn is_roko_state(path: &str) -> bool {
    path == ".roko" || path.starts_with(".roko/")
}

/// Tree id of `workdir`'s working tree as it is now: the tracked files and
/// the untracked ones `.gitignore` keeps, hashed into the object store
/// through a copy of the repository's index, which is left untouched.
/// `None` outside a git work tree, or when git fails.
async fn snapshot_tree(workdir: &Path) -> Option<String> {
    let index = git_line(workdir, &["rev-parse", "--git-path", "index"], None).await?;
    let scratch = tempfile::tempdir().ok()?;
    let scratch_index = scratch.path().join("index");
    // The copy keeps the index's stat cache, so `git add` rehashes only what
    // changed. A repository without an index yet starts from an empty one.
    let _ = tokio::fs::copy(workdir.join(index), &scratch_index).await;
    git_bytes(
        workdir,
        &["add", "--all", "--", "."],
        Some(&scratch_index),
        RAW_LIMIT,
    )
    .await?;
    git_line(workdir, &["write-tree"], Some(&scratch_index)).await
}

/// Trimmed stdout of git in `workdir`, or `None` when it fails or prints
/// nothing.
async fn git_line(workdir: &Path, args: &[&str], index: Option<&Path>) -> Option<String> {
    let (stdout, _) = git_bytes(workdir, args, index, 64 * 1024).await?;
    let line = String::from_utf8_lossy(&stdout).trim().to_string();
    (!line.is_empty()).then_some(line)
}

/// Up to `limit` bytes of the stdout of git run in `workdir`, against the
/// index at `index` when given, and whether the output was cut there (git is
/// then killed). `None` when git cannot start, fails, or runs past
/// [`GIT_TIMEOUT`].
async fn git_bytes(
    workdir: &Path,
    args: &[&str],
    index: Option<&Path>,
    limit: usize,
) -> Option<(Vec<u8>, bool)> {
    let mut command = crate::runner::merge::git_command(workdir);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let run = async {
        let mut child = command.spawn().ok()?;
        let stdout = child.stdout.take()?;
        let mut reader = stdout.take(u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1));
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.ok()?;
        drop(reader);
        let truncated = bytes.len() > limit;
        if truncated {
            bytes.truncate(limit);
            let _ = child.start_kill();
        }
        let status = child.wait().await.ok()?;
        (truncated || status.success()).then_some((bytes, truncated))
    };
    tokio::time::timeout(GIT_TIMEOUT, run).await.ok().flatten()
}

/// Parse `git diff --raw -z`: a `:<mode> <mode> <blob> <blob> <status>`
/// header, then one path, or two for a rename or copy, each NUL-terminated.
fn parse_raw_diff(raw: &[u8]) -> Vec<ChangedPath> {
    let mut fields = raw
        .split(|byte| *byte == 0)
        .map(|field| String::from_utf8_lossy(field).into_owned());
    let mut changes = Vec::new();
    while let Some(header) = fields.next() {
        let Some(header) = header.strip_prefix(':') else {
            continue;
        };
        let parts: Vec<&str> = header.split(' ').collect();
        let [_, _, old_blob, new_blob, status] = parts.as_slice() else {
            break;
        };
        let status = status.chars().next().unwrap_or('M');
        let Some(first) = fields.next() else {
            break;
        };
        let (old_path, path) = if matches!(status, 'R' | 'C') {
            let Some(second) = fields.next() else {
                break;
            };
            (Some(first), second)
        } else {
            (None, first)
        };
        changes.push(ChangedPath {
            status,
            path,
            old_path,
            old_blob: blob_id(old_blob),
            new_blob: blob_id(new_blob),
        });
    }
    changes
}

/// A blob id, or `None` for git's all-zero "no blob".
fn blob_id(id: &str) -> Option<String> {
    (!id.is_empty() && !id.bytes().all(|byte| byte == b'0')).then(|| id.to_string())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Run git in `dir` and return its trimmed stdout.
    pub(in crate::graph_task_dispatch) fn git(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=roko-test",
                "-c",
                "user.email=roko@test.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Make `dir` a git repository whose one commit holds `files`.
    pub(in crate::graph_task_dispatch) fn commit_repo(dir: &Path, files: &[(&str, &str)]) {
        git(dir, &["init", "--quiet"]);
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, text).expect("write file");
        }
        git(dir, &["add", "--all"]);
        git(dir, &["commit", "--quiet", "-m", "seed"]);
    }

    #[test]
    fn raw_diff_parses_renames_additions_and_deletions() {
        let old = "1".repeat(40);
        let new = "2".repeat(40);
        let zero = "0".repeat(40);
        let raw = format!(
            ":100644 100644 {old} {new} M\0src/lib.rs\0\
             :100644 100644 {old} {new} R087\0tests/a.rs\0tests/b.rs\0\
             :000000 100644 {zero} {new} A\0new file.rs\0\
             :100644 000000 {old} {zero} D\0gone.rs\0"
        );
        let changes = parse_raw_diff(raw.as_bytes());
        let summary: Vec<(char, &str, Option<&str>, bool, bool)> = changes
            .iter()
            .map(|change| {
                (
                    change.status,
                    change.path.as_str(),
                    change.old_path.as_deref(),
                    change.old_blob.is_some(),
                    change.new_blob.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ('M', "src/lib.rs", None, true, true),
                ('R', "tests/b.rs", Some("tests/a.rs"), true, true),
                ('A', "new file.rs", None, false, true),
                ('D', "gone.rs", None, true, false),
            ]
        );
        assert_eq!(attempt_chain("run:plan:T1:3"), "run:plan:T1");
    }

    #[tokio::test]
    async fn snapshots_see_untracked_files_and_leave_the_index_alone() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        commit_repo(repo, &[("src/lib.rs", "pub fn a() {}\n")]);
        let before = snapshot_tree(repo).await.expect("snapshot before");

        std::fs::write(repo.join("src/lib.rs"), "pub fn a() { b() }\n").expect("edit");
        std::fs::write(repo.join("notes.md"), "new\n").expect("add");
        let after = snapshot_tree(repo).await.expect("snapshot after");

        let (raw, _) = git_bytes(
            repo,
            &[
                "diff",
                "--raw",
                "-z",
                "--no-abbrev",
                before.as_str(),
                after.as_str(),
            ],
            None,
            RAW_LIMIT,
        )
        .await
        .expect("diff");
        let paths: Vec<String> = parse_raw_diff(&raw)
            .into_iter()
            .map(|change| change.path)
            .collect();
        assert_eq!(paths, ["notes.md", "src/lib.rs"]);
        // The repository's own index is untouched: nothing staged, and the
        // new file still untracked.
        assert_eq!(git(repo, &["diff", "--cached", "--name-only"]), "");
        assert_eq!(
            git(repo, &["ls-files", "--others", "--exclude-standard"]),
            "notes.md"
        );

        let outside = tempfile::tempdir().expect("tempdir");
        assert!(snapshot_tree(outside.path()).await.is_none());
    }
}
