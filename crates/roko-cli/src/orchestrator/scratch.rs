//! scratch_dir attempt workspaces (9135): a copy of a task's data outside
//! git, for large or binary data and for data git does not hold.
//!
//! [`ScratchLease::acquire`] copies what the task's `files` name, relative
//! to the workspace (a file, a directory with everything below it, or a glob
//! with what it matches), into the attempt's directory at the same relative
//! paths. `std::fs::copy` clones a file where the file system can (APFS
//! `clonefile` on macOS; Linux `copy_file_range`, which reflinks on btrfs
//! and XFS) and copies it otherwise. It never hard-links, so an agent's
//! edit never reaches the original. The lease records the SHA-256 of every
//! copy in a base manifest beside the directory, and
//! [`ScratchLease::finish`] records the result manifest after the attempt,
//! with what changed. A retry resumes in the same directory, as a worktree
//! attempt's retry resumes in its checkout.
//!
//! An accepted attempt's result goes back into the workspace through
//! [`ScratchLease::copy_back`] (9136): each file it changed, added or
//! deleted is replaced atomically, created or deleted there, but only while
//! the workspace still holds the base the copy was made from. A file another
//! writer changed since is a conflict, and nothing is copied. One copy-back
//! runs at a time in a process, so the check and the writes are one step.
//! (A keyed-row merge of CSV data is a later item: this is a single writer.)

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// The SHA-256 of each file of a scratch workspace, in hex, by its path
/// relative to the workspace.
pub type ScratchManifest = BTreeMap<String, String>;

/// What an attempt changed in its scratch workspace, by relative path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScratchChanges {
    /// Files whose content changed.
    pub changed: Vec<String>,
    /// Files the attempt created.
    pub added: Vec<String>,
    /// Files the attempt deleted.
    pub removed: Vec<String>,
}

/// A file an accepted scratch attempt wrote back into the workspace
/// (9136): its path, relative to the workspace, and its SHA-256 there, or
/// `None` for a file the attempt deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopiedFile {
    /// The file's path, relative to the workspace.
    pub path: String,
    /// Its SHA-256 in hex after the copy-back; `None` once deleted.
    pub sha256: Option<String>,
}

/// Why a scratch attempt's result was not copied back (9136).
#[derive(Debug)]
pub enum CopyBackError {
    /// The workspace changed these files, relative to it, since the copy was
    /// made: copying back would overwrite another writer's work. Nothing was
    /// copied.
    Conflict(Vec<String>),
    /// A file could not be read or written.
    Io(anyhow::Error),
}

impl std::fmt::Display for CopyBackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict(paths) => write!(
                formatter,
                "the workspace changed {} since the copy was made",
                paths.join(", ")
            ),
            Self::Io(error) => write!(formatter, "{error:#}"),
        }
    }
}

/// One copy-back at a time in this process, so that checking that the
/// workspace still holds the base and writing the result are one step.
static COPY_BACK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A scratch_dir attempt workspace ([module docs](self)).
#[derive(Debug)]
pub struct ScratchLease {
    dir: PathBuf,
    base: ScratchManifest,
}

impl ScratchLease {
    /// The scratch workspace at `dir` holding copies of the data `files`
    /// names in the workspace at `workdir`. A `dir` that already has its base
    /// manifest is a retry's: it is taken as it is.
    ///
    /// # Errors
    ///
    /// Fails when `files` is empty, names a path outside the workspace or
    /// one that does not exist, or a copy or the manifest cannot be written.
    pub fn acquire(workdir: &Path, dir: PathBuf, files: &[String]) -> Result<Self> {
        let base_path = manifest_path(&dir, "base");
        if dir.is_dir() && base_path.is_file() {
            let base = serde_json::from_slice(&std::fs::read(&base_path)?)
                .with_context(|| format!("read {}", base_path.display()))?;
            return Ok(Self { dir, base });
        }
        if files.is_empty() {
            bail!("a scratch_dir task names its data in `files`, and this one names none");
        }
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        for relative in declared_files(workdir, files)? {
            let target = dir.join(&relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(workdir.join(&relative), &target)
                .with_context(|| format!("copy {} into {}", relative.display(), dir.display()))?;
        }
        let base = manifest(&dir)?;
        write_manifest(&base_path, &base)?;
        Ok(Self { dir, base })
    }

    /// The workspace's directory: the attempt's working directory, where its
    /// verify steps run.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The SHA-256 of each copy when the workspace was leased.
    #[must_use]
    pub fn base(&self) -> &ScratchManifest {
        &self.base
    }

    /// Record the result manifest beside the directory after an attempt, and
    /// return what the attempt changed.
    ///
    /// # Errors
    ///
    /// Fails when a file cannot be read or the manifest cannot be written.
    pub fn finish(&self) -> Result<ScratchChanges> {
        let result = manifest(&self.dir)?;
        write_manifest(&manifest_path(&self.dir, "result"), &result)?;
        Ok(changes(&self.base, &result))
    }

    /// Copy what the attempt changed back into the workspace at `workdir`
    /// (9136): a changed or added file replaces the workspace's atomically
    /// (written to a temp file beside it, then renamed over it), and a
    /// deleted one is deleted. It happens only when every one of those files
    /// in the workspace is still the base the copy was made from, or, for an
    /// added file, is absent or already the result. Returns the files written
    /// back with their new hashes, by path.
    ///
    /// # Errors
    ///
    /// [`CopyBackError::Conflict`] names the files the workspace changed
    /// since the copy was made, and nothing is copied;
    /// [`CopyBackError::Io`] when a file cannot be read or written.
    pub fn copy_back(&self, workdir: &Path) -> std::result::Result<Vec<CopiedFile>, CopyBackError> {
        let _one_at_a_time = COPY_BACK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = manifest(&self.dir).map_err(CopyBackError::Io)?;
        let changes = changes(&self.base, &result);
        let in_workspace = |path: &str| file_hash(&workdir.join(path));
        let mut conflicts: Vec<String> = changes
            .changed
            .iter()
            .chain(&changes.removed)
            .filter(|path| in_workspace(path).as_ref() != self.base.get(*path))
            .cloned()
            .collect();
        for path in &changes.added {
            let present = in_workspace(path);
            if present.is_some() && present.as_ref() != result.get(path) {
                conflicts.push(path.clone());
            }
        }
        if !conflicts.is_empty() {
            conflicts.sort();
            return Err(CopyBackError::Conflict(conflicts));
        }
        let mut copied = Vec::new();
        for path in changes.changed.iter().chain(&changes.added) {
            replace_atomically(&self.dir.join(path), &workdir.join(path))
                .map_err(CopyBackError::Io)?;
            copied.push(CopiedFile {
                path: path.clone(),
                sha256: result.get(path).cloned(),
            });
        }
        for path in &changes.removed {
            let target = workdir.join(path);
            std::fs::remove_file(&target)
                .with_context(|| format!("delete {}", target.display()))
                .map_err(CopyBackError::Io)?;
            copied.push(CopiedFile {
                path: path.clone(),
                sha256: None,
            });
        }
        copied.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(copied)
    }

    /// Delete the workspace and its manifests.
    ///
    /// # Errors
    ///
    /// Fails when the directory cannot be removed.
    pub fn remove(self) -> Result<()> {
        std::fs::remove_dir_all(&self.dir)
            .with_context(|| format!("remove {}", self.dir.display()))?;
        for stage in ["base", "result"] {
            let _ = std::fs::remove_file(manifest_path(&self.dir, stage));
        }
        Ok(())
    }
}

/// The manifest file of `stage` (`base` or `result`) for the workspace at
/// `dir`: beside it, out of the agent's reach.
fn manifest_path(dir: &Path, stage: &str) -> PathBuf {
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    dir.with_file_name(format!("{name}.{stage}.json"))
}

fn write_manifest(path: &Path, manifest: &ScratchManifest) -> Result<()> {
    let json = serde_json::to_vec_pretty(manifest)?;
    std::fs::write(path, json).with_context(|| format!("write {}", path.display()))
}

/// The files, relative to `workdir`, that `files` names there: each entry is
/// a file, a directory with every file below it, or a glob.
fn declared_files(workdir: &Path, files: &[String]) -> Result<Vec<PathBuf>> {
    let mut declared = Vec::new();
    for entry in files {
        let relative = Path::new(entry.trim());
        let inside = relative
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
        if !inside {
            bail!("`{entry}` is not a path inside the workspace");
        }
        let path = workdir.join(relative);
        if path.is_file() {
            declared.push(relative.to_path_buf());
        } else if path.is_dir() {
            for file in files_below(&path)? {
                declared.push(file.strip_prefix(workdir)?.to_path_buf());
            }
        } else {
            let pattern = format!(
                "{}/{}",
                glob::Pattern::escape(&workdir.to_string_lossy()),
                relative.to_string_lossy()
            );
            let matched: Vec<PathBuf> = glob::glob(&pattern)
                .with_context(|| format!("`{entry}` is not a valid glob"))?
                .flatten()
                .filter(|path| path.is_file())
                .collect();
            if matched.is_empty() {
                bail!("`{entry}` names nothing in the workspace");
            }
            for file in matched {
                declared.push(file.strip_prefix(workdir)?.to_path_buf());
            }
        }
    }
    declared.sort();
    declared.dedup();
    Ok(declared)
}

/// Every file below `dir`, recursively, in no set order.
fn files_below(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(files_below(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

/// The SHA-256 of every file below `dir`, by its path relative to `dir`.
fn manifest(dir: &Path) -> Result<ScratchManifest> {
    let mut manifest = ScratchManifest::new();
    for file in files_below(dir)? {
        let relative = file.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
        manifest.insert(relative, hash(&file)?);
    }
    Ok(manifest)
}

/// The SHA-256 of the file at `path`, in hex.
fn hash(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut reader =
        std::fs::File::open(path).with_context(|| format!("read {}", path.display()))?;
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// The SHA-256 of the file at `path`, or `None` when no file is there or it
/// cannot be read.
fn file_hash(path: &Path) -> Option<String> {
    path.is_file().then(|| hash(path).ok()).flatten()
}

/// Replace `target` with a copy of `source`: written to a temp file beside
/// `target`, then renamed over it, so a reader sees the old file or the new
/// one, never part of either.
fn replace_atomically(source: &Path, target: &Path) -> Result<()> {
    let dir = target
        .parent()
        .with_context(|| format!("{} has no directory", target.display()))?;
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.roko-scratch-{}", std::process::id()));
    std::fs::copy(source, &temp).with_context(|| format!("write {}", temp.display()))?;
    if let Err(error) = std::fs::rename(&temp, target) {
        let _ = std::fs::remove_file(&temp);
        return Err(error).with_context(|| format!("replace {}", target.display()));
    }
    Ok(())
}

/// What changed from `base` to `result`.
fn changes(base: &ScratchManifest, result: &ScratchManifest) -> ScratchChanges {
    let mut changes = ScratchChanges::default();
    for (path, hash) in result {
        match base.get(path) {
            None => changes.added.push(path.clone()),
            Some(before) if before != hash => changes.changed.push(path.clone()),
            Some(_) => {}
        }
    }
    changes.removed = base
        .keys()
        .filter(|path| !result.contains_key(*path))
        .cloned()
        .collect();
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9135: a lease holds copies of exactly the files the task declares,
    /// its base manifest holds their hashes, an edit of a copy leaves the
    /// original as it was, and the result manifest shows the changed file.
    #[test]
    fn scratch_dir_attempt_gets_cow_copy_and_manifest() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path();
        std::fs::create_dir_all(workdir.join("data/raw")).expect("data directory");
        std::fs::write(workdir.join("data/export.csv"), "id,name\n1,ada\n").expect("export");
        std::fs::write(workdir.join("data/raw/a.bin"), [0_u8, 1, 2]).expect("raw file");
        std::fs::write(workdir.join("notes.md"), "not declared\n").expect("notes");
        let dir = workdir.join(".roko/scratch/run-1/T1/0");
        let files = ["data/export.csv".to_string(), "data/raw".to_string()];

        let lease = ScratchLease::acquire(workdir, dir.clone(), &files).expect("the lease");
        let copied: Vec<&str> = lease.base().keys().map(String::as_str).collect();
        assert_eq!(copied, ["data/export.csv", "data/raw/a.bin"]);
        let expected = format!("{:x}", Sha256::digest(b"id,name\n1,ada\n"));
        assert_eq!(lease.base()["data/export.csv"], expected);
        assert!(manifest_path(&dir, "base").is_file());

        std::fs::write(dir.join("data/export.csv"), "id,name\n1,grace\n").expect("edit");
        let original = std::fs::read_to_string(workdir.join("data/export.csv")).expect("original");
        assert_eq!(
            original, "id,name\n1,ada\n",
            "the edit reached the original"
        );
        let changes = lease.finish().expect("the result manifest");
        assert_eq!(changes.changed, ["data/export.csv"]);
        assert!(
            changes.added.is_empty() && changes.removed.is_empty(),
            "{changes:?}"
        );
        assert!(manifest_path(&dir, "result").is_file());

        let resumed = ScratchLease::acquire(workdir, dir.clone(), &files).expect("a retry");
        assert_eq!(resumed.base(), lease.base());
        let none = ScratchLease::acquire(workdir, workdir.join(".roko/scratch/x"), &[]);
        assert!(none.is_err());
        resumed.remove().expect("remove");
        assert!(!dir.exists());
    }

    /// 9136: an accepted attempt's changed, added and deleted files reach the
    /// workspace while its base is unchanged; once another writer changes a
    /// file after the copy was made, the copy-back is refused, names that
    /// file and leaves the workspace as that writer left it.
    #[test]
    fn scratch_dir_accept_copies_back_and_refuses_conflicts() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path();
        std::fs::create_dir_all(workdir.join("data")).expect("data directory");
        std::fs::write(workdir.join("data/a.csv"), "id\n1\n").expect("a.csv");
        std::fs::write(workdir.join("data/old.csv"), "id\n0\n").expect("old.csv");
        let files = ["data".to_string()];

        let first = workdir.join(".roko/scratch/run-1/T1/0");
        let lease = ScratchLease::acquire(workdir, first.clone(), &files).expect("the lease");
        std::fs::write(first.join("data/a.csv"), "id\n1\n2\n").expect("edit");
        std::fs::write(first.join("data/new.csv"), "id\n3\n").expect("add");
        std::fs::remove_file(first.join("data/old.csv")).expect("delete");
        let copied = lease
            .copy_back(workdir)
            .expect("an unchanged base takes the result");
        let paths: Vec<&str> = copied.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["data/a.csv", "data/new.csv", "data/old.csv"]);
        let a = std::fs::read_to_string(workdir.join("data/a.csv")).expect("a.csv");
        assert_eq!(a, "id\n1\n2\n");
        assert_eq!(
            copied[0].sha256,
            Some(format!("{:x}", Sha256::digest(b"id\n1\n2\n")))
        );
        assert!(workdir.join("data/new.csv").is_file());
        assert!(!workdir.join("data/old.csv").exists());
        assert_eq!(copied[2].sha256, None);

        let second = workdir.join(".roko/scratch/run-1/T2/0");
        let lease = ScratchLease::acquire(workdir, second.clone(), &files).expect("the lease");
        std::fs::write(second.join("data/a.csv"), "id\n9\n").expect("edit");
        std::fs::write(workdir.join("data/a.csv"), "id\n1\n2\n4\n").expect("another writer");
        let refused = lease
            .copy_back(workdir)
            .expect_err("a moved base refuses the result");
        assert!(
            matches!(&refused, CopyBackError::Conflict(paths) if paths == &["data/a.csv"]),
            "{refused}"
        );
        let a = std::fs::read_to_string(workdir.join("data/a.csv")).expect("a.csv");
        assert_eq!(a, "id\n1\n2\n4\n", "the refused copy-back touched the file");
    }
}
