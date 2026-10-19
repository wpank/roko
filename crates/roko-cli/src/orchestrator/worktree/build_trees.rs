//! Warm build trees for new checkouts (gap-6754ab).
//!
//! A checkout roko makes for a check, such as an attempt's worktree, starts
//! without the project's build tree, so its first check configures and builds
//! the whole project: on a C++ project with JUCE about 50 s, where the same
//! check on a warm tree takes 4 s. [`give_build_trees`] gives the new checkout
//! a copy-on-write clone of each build tree of the repository's checkout,
//! made to work at the new path:
//!
//! - **Relocated.** CMake names the source and build directories by absolute
//!   path in its cache, makefiles and dependency files, and refuses a cache
//!   made for another directory. The checkout's path is rewritten to the new
//!   checkout's in every text file of the clone, keeping the file's times, so
//!   make sees the timestamps the build left.
//! - **Rule hashes.** CMake deletes the output of a custom command whose
//!   command line, absolute paths included, no longer hashes as
//!   `CMakeFiles/CMakeRuleHashes.txt` recorded. A FetchContent or
//!   ExternalProject populate step would then download and extract its
//!   sources again, with new mtimes on every file: a full rebuild. Those
//!   steps' lines are dropped, since their url info file reruns them when
//!   their source changes. Every other rule keeps its hash.
//! - **Unchanged sources.** A new checkout's files are all newer than any
//!   build, so make would rebuild everything. Each tracked file whose content
//!   is the repository checkout's (it is not in `git diff` against the new
//!   checkout's commit) gets that file's times; a changed file keeps its own
//!   and rebuilds.
//!
//! A tree that names a path under the repository checkout which the new
//! checkout lacks (an untracked dependency, `.git/index`) would build against
//! the person's files, or fail, so it is not given. Neither is a tree whose
//! binary files hold paths a text rewrite cannot reach (a Ninja deps log, a
//! precompiled header). Nothing is ever written to the repository checkout,
//! and a tree that cannot be given leaves the new checkout to build cold, as
//! before.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::git_ops::{clone_tree, ignored_dirs};

/// How deep under a git-ignored directory a build tree may be: `build/`
/// itself, a preset's `build/<preset>/`, or `out/build/<preset>/`.
const CMAKE_CACHE_DEPTH: u8 = 2;

/// The `CMakeCache.txt` entry naming the source directory a tree builds.
const CMAKE_HOME_ENTRY: &str = "CMAKE_HOME_DIRECTORY:INTERNAL=";

/// Ignored directories that are never build trees: roko's own state, and
/// what package managers install (given as dependency directories).
const NOT_BUILD_TREES: &[&str] = &[".git", ".roko", "node_modules"];

/// Files larger than this are left as they are: no build system keeps the
/// paths it reads back in one.
const MAX_TEXT_FILE_BYTES: u64 = 16 << 20;

/// A file with a NUL byte in its first this many bytes is binary.
const BINARY_SNIFF_BYTES: usize = 8 << 10;

/// Files whose binary content names absolute paths a text rewrite cannot
/// reach: a tree holding one is not given.
const UNRELOCATABLE_FILES: &[&str] = &[".ninja_deps"];

/// Extensions of such files: precompiled headers and Clang modules.
const UNRELOCATABLE_EXTENSIONS: &[&str] = &["pch", "gch", "pcm"];

/// The CMake file whose custom-command hashes force reruns.
const RULE_HASHES_FILE: &str = "CMakeRuleHashes.txt";

/// One build tree [`give_build_trees`] gave a checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GivenBuildTree {
    /// The tree's directory, relative to the checkout.
    pub dir: PathBuf,
    /// Text files whose paths were rewritten.
    pub rewritten: usize,
    /// How long cloning and moving the tree took, in ms.
    pub elapsed_ms: u64,
}

/// The build trees of the checkout at `source`, relative to it: the
/// directories `configured` names (`[gates] warm_build_dirs`) that exist and
/// git ignores, or, when it is `None`, every CMake build tree in a git-ignored
/// directory, at most [`CMAKE_CACHE_DEPTH`] levels down, whose
/// `CMakeCache.txt` builds a project of the checkout: `source` itself, or a
/// directory under it outside the ignored ones (not a scratch project in an
/// ignored `tmp/`). Empty when `source` is no git checkout.
#[must_use]
pub fn warm_build_dirs(source: &Path, configured: Option<&[String]>) -> Vec<PathBuf> {
    let ignored = ignored_dirs(source);
    if let Some(names) = configured {
        return names
            .iter()
            .map(|name| PathBuf::from(name.trim().trim_end_matches('/')))
            .filter(|dir| is_inside(dir) && source.join(dir).is_dir())
            .filter(|dir| ignored.iter().any(|ignored| dir.starts_with(ignored)))
            .collect();
    }
    // Compared canonically: CMake may record `/tmp/...` for a checkout under
    // `/private/tmp`.
    let real = canonical(source);
    let builds_here = |home: &Path| {
        canonical(home).strip_prefix(&real).is_ok_and(|project| {
            !ignored
                .iter()
                .any(|dir| !dir.as_os_str().is_empty() && project.starts_with(dir))
        })
    };
    let mut trees = Vec::new();
    for dir in &ignored {
        let skipped = dir
            .components()
            .any(|part| NOT_BUILD_TREES.contains(&part.as_os_str().to_string_lossy().as_ref()));
        if !skipped {
            find_cmake_trees(source, dir, &builds_here, CMAKE_CACHE_DEPTH, &mut trees);
        }
    }
    trees
}

/// Add to `trees` each directory at most `depth` levels down from `dir`
/// (relative to `source`) whose `CMakeCache.txt` names a source directory
/// `builds_here` accepts; a tree's own subdirectories (a FetchContent
/// sub-build, JUCE's tools) are part of it.
fn find_cmake_trees(
    source: &Path,
    dir: &Path,
    builds_here: &dyn Fn(&Path) -> bool,
    depth: u8,
    trees: &mut Vec<PathBuf>,
) {
    let cache = source.join(dir).join("CMakeCache.txt");
    if cache.is_file() {
        if cmake_home(&cache).is_some_and(|home| builds_here(&home)) {
            trees.push(dir.to_path_buf());
        }
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(source.join(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            find_cmake_trees(
                source,
                &dir.join(entry.file_name()),
                builds_here,
                depth - 1,
                trees,
            );
        }
    }
}

/// The source directory the CMake cache at `cache` builds.
fn cmake_home(cache: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(cache).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix(CMAKE_HOME_ENTRY))
        .map(|home| PathBuf::from(home.trim()))
}

/// The forms of `source`'s path that the build tree at `tree` may name, the
/// longest first: the path as given, its canonical form, and the form the
/// tree's CMake cache recorded, which on macOS keeps `/tmp` for
/// `/private/tmp`. With them, the recorded form (the canonical one when the
/// tree has no CMake cache).
fn recorded_roots(source: &Path, tree: &Path) -> (Vec<PathBuf>, PathBuf) {
    let mut forms = vec![source.to_path_buf(), canonical(source)];
    let mut recorded = canonical(source);
    // The cache names the project it builds: the checkout's root, or a
    // directory under it, in the form CMake chose.
    if let Some(home) = cmake_home(&tree.join("CMakeCache.txt")) {
        let real_home = canonical(&home);
        if let Ok(project) = real_home.strip_prefix(canonical(source)) {
            let mut root = home.clone();
            let mut whole = true;
            for _ in project.components() {
                whole &= root.pop();
            }
            if whole {
                recorded.clone_from(&root);
                forms.push(root);
            }
        }
    }
    forms.sort_by_key(|form| std::cmp::Reverse(form.as_os_str().len()));
    forms.dedup();
    (forms, recorded)
}

/// `real_checkout` in the form CMake names it by, as it named the source,
/// canonically `real_source`, by `recorded`: when they differ by a prefix
/// (`/tmp/x/repo` for `/private/tmp/x/repo`), the checkout's canonical path
/// with the same prefix replaced; else the canonical path.
fn mirrored_form(real_source: &Path, recorded: &Path, real_checkout: &Path) -> PathBuf {
    let real: Vec<_> = real_source.components().collect();
    let named: Vec<_> = recorded.components().collect();
    let common = real
        .iter()
        .rev()
        .zip(named.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let real_prefix: PathBuf = real[..real.len() - common].iter().collect();
    let named_prefix: PathBuf = named[..named.len() - common].iter().collect();
    match real_checkout.strip_prefix(&real_prefix) {
        Ok(rest) if real_prefix != named_prefix => named_prefix.join(rest),
        _ => real_checkout.to_path_buf(),
    }
}

/// `path`'s canonical form, or `path` when it has none.
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Give the new checkout at `checkout` a warm copy of each build tree of the
/// repository's checkout at `source` ([`warm_build_dirs`]), unless it has one
/// there already, and return the trees it gave. A tree that cannot be cloned
/// or moved is logged and left out: the checkout builds it cold.
pub fn give_build_trees(
    source: &Path,
    checkout: &Path,
    configured: Option<&[String]>,
) -> Vec<GivenBuildTree> {
    let dirs = warm_build_dirs(source, configured);
    if dirs.is_empty() {
        return Vec::new();
    }
    let mut given = Vec::new();
    for dir in &dirs {
        let started = Instant::now();
        let target = checkout.join(dir);
        // Every form of the source's path the tree names becomes the
        // checkout's, in the form CMake will name it by: a form CMake does
        // not regenerate the same would rewrite its makefiles, and rebuild.
        let (roots, recorded) = recorded_roots(source, &source.join(dir));
        let to = mirrored_form(&canonical(source), &recorded, &canonical(checkout));
        match give_tree(source, &roots, checkout, &to, dir, &dirs) {
            Ok(Some(rewritten)) => {
                let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                tracing::info!(
                    dir = %dir.display(),
                    rewritten,
                    elapsed_ms,
                    "gave the checkout a warm copy of the repository checkout's build tree"
                );
                given.push(GivenBuildTree {
                    dir: dir.clone(),
                    rewritten,
                    elapsed_ms,
                });
            }
            Ok(None) => {}
            Err(why) => {
                let _ = std::fs::remove_dir_all(&target);
                tracing::info!(
                    dir = %dir.display(),
                    %why,
                    "left the checkout to build the tree cold"
                );
            }
        }
    }
    if !given.is_empty() {
        match keep_unchanged_times(source, checkout) {
            Ok(kept) => tracing::debug!(kept, "gave unchanged files the checkout's times"),
            Err(error) => tracing::warn!(
                %error,
                "could not give the checkout's unchanged files their times; its first build \
                 rebuilds them"
            ),
        }
    }
    given
}

/// Clone `dir` of `source` into `checkout` and move it there. `Ok(None)` when
/// there is nothing to do: the checkout has the directory already, or its
/// parent is not there, or no copy-on-write clone can be made. `Err` (why)
/// when the clone was made but cannot be used, which the caller removes.
fn give_tree(
    source: &Path,
    roots: &[PathBuf],
    checkout: &Path,
    to: &Path,
    dir: &Path,
    trees: &[PathBuf],
) -> Result<Option<usize>, String> {
    let from = source.join(dir);
    let target = checkout.join(dir);
    let Ok(metadata) = std::fs::symlink_metadata(&from) else {
        return Ok(None);
    };
    if !metadata.is_dir() || std::fs::symlink_metadata(&target).is_ok() {
        return Ok(None);
    }
    // A preset's tree (`build/<preset>`) sits in a directory the checkout
    // does not track.
    let Some(parent) = target.parent() else {
        return Ok(None);
    };
    if std::fs::create_dir_all(parent).is_err() {
        return Ok(None);
    }
    // A clone, never a copy: a copy across devices would cost the attempt
    // the tree's size in time and disk.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if std::fs::metadata(parent).map_or(true, |parent| parent.dev() != metadata.dev()) {
            return Ok(None);
        }
    }
    match clone_tree(&from, &target) {
        Ok(true) => {}
        Ok(false) => return Ok(None),
        Err(error) => return Err(format!("the clone failed: {error}")),
    }
    let rewritten = relocate(&target, roots, source, checkout, to, trees)?;
    drop_populate_rule_hashes(&target).map_err(|error| format!("{RULE_HASHES_FILE}: {error}"))?;
    // The checkout's git must ignore it too, or the attempt's commit adds it.
    let ignored = std::process::Command::new("git")
        .args(["check-ignore", "-q"])
        .arg(format!("{}/", dir.display()))
        .current_dir(checkout)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !ignored {
        return Err("the checkout's git does not ignore it".to_string());
    }
    Ok(Some(rewritten))
}

/// Rewrite every absolute path under `source` in the text files of `tree` (a
/// clone in `checkout`) to the same path under `to`, the checkout's canonical
/// path, keeping each file's times; returns how many files changed. `Err`
/// when the tree cannot work at the new path: it names a path under `source`
/// that the checkout lacks, or holds a file that names paths in binary.
fn relocate(
    tree: &Path,
    roots: &[PathBuf],
    source: &Path,
    checkout: &Path,
    to: &Path,
    trees: &[PathBuf],
) -> Result<usize, String> {
    let alternatives: Vec<String> = roots
        .iter()
        .map(|root| regex::escape(&root.to_string_lossy()))
        .collect();
    // A root, then the rest of the path up to the next character that ends a
    // path in a cache, makefile, dependency file or CMake script.
    let pattern = format!(
        r#"(?:{})(?P<rest>(?:/(?-u:[^\x00-\x20"';:,()<>|\\])*)?)"#,
        alternatives.join("|")
    );
    let paths = regex::bytes::Regex::new(&pattern).map_err(|error| error.to_string())?;
    let to = to.to_string_lossy().into_owned();
    let mut checked: HashMap<Vec<u8>, bool> = HashMap::new();
    let mut rewritten = 0;
    let mut pending = vec![tree.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|error| error.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if unrelocatable(&path) {
                return Err(format!(
                    "{} names paths in binary",
                    path.strip_prefix(checkout).unwrap_or(&path).display()
                ));
            }
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            if metadata.len() > MAX_TEXT_FILE_BYTES {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
            if bytes[..bytes.len().min(BINARY_SNIFF_BYTES)].contains(&0) {
                continue;
            }
            let mut out = Vec::with_capacity(bytes.len());
            let mut last = 0;
            for found in paths.captures_iter(&bytes) {
                let whole = found.get(0).expect("the whole match");
                let rest = found.name("rest").map_or(&[][..], |rest| rest.as_bytes());
                // A root must end at a path boundary: `/a/repo` is not in
                // `/a/repo-2`.
                if rest.is_empty()
                    && bytes
                        .get(whole.end())
                        .is_some_and(|next| next.is_ascii_alphanumeric() || b"._-+@".contains(next))
                {
                    continue;
                }
                if !rest.is_empty() {
                    let missing = *checked
                        .entry(rest.to_vec())
                        .or_insert_with(|| missing_from_checkout(rest, source, checkout, trees));
                    if missing {
                        return Err(format!(
                            "it names {}, which the checkout lacks",
                            source
                                .join(String::from_utf8_lossy(&rest[1..]).as_ref())
                                .display()
                        ));
                    }
                }
                out.extend_from_slice(&bytes[last..whole.start()]);
                out.extend_from_slice(to.as_bytes());
                out.extend_from_slice(rest);
                last = whole.end();
            }
            if last == 0 {
                continue;
            }
            out.extend_from_slice(&bytes[last..]);
            write_keeping_times(&path, &out, &metadata).map_err(|error| error.to_string())?;
            rewritten += 1;
        }
    }
    Ok(rewritten)
}

/// Whether the path `rest` (`/` and a path relative to the checkout) names
/// something the repository checkout at `source` has and the new checkout
/// lacks. Paths inside the given build trees are there by construction.
fn missing_from_checkout(rest: &[u8], source: &Path, checkout: &Path, trees: &[PathBuf]) -> bool {
    let relative = PathBuf::from(String::from_utf8_lossy(&rest[1..]).as_ref());
    if relative.as_os_str().is_empty() || trees.iter().any(|tree| relative.starts_with(tree)) {
        return false;
    }
    std::fs::symlink_metadata(checkout.join(&relative)).is_err()
        && std::fs::symlink_metadata(source.join(&relative)).is_ok()
}

/// Whether the file at `path` names paths in a binary form.
fn unrelocatable(path: &Path) -> bool {
    let name = path.file_name().map(|name| name.to_string_lossy());
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy());
    name.is_some_and(|name| UNRELOCATABLE_FILES.contains(&name.as_ref()))
        || extension.is_some_and(|extension| UNRELOCATABLE_EXTENSIONS.contains(&extension.as_ref()))
}

/// Drop the lines of a FetchContent or ExternalProject populate step from
/// every `CMakeFiles/CMakeRuleHashes.txt` under `tree`, keeping each file's
/// times.
fn drop_populate_rule_hashes(tree: &Path) -> std::io::Result<()> {
    let mut pending = vec![tree.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if !file_type.is_file()
                || entry.file_name() != RULE_HASHES_FILE
                || dir.file_name().is_none_or(|name| name != "CMakeFiles")
            {
                continue;
            }
            let path = entry.path();
            let metadata = entry.metadata()?;
            let text = std::fs::read_to_string(&path)?;
            let kept: String = text
                .split_inclusive('\n')
                .filter(|line| !is_populate_stamp(line))
                .collect();
            if kept.len() != text.len() {
                write_keeping_times(&path, kept.as_bytes(), &metadata)?;
            }
        }
    }
    Ok(())
}

/// Whether a rule-hash line names a populate step's stamp:
/// `<hash> <name>-populate-prefix/src/<name>-populate-stamp/<step>`.
fn is_populate_stamp(line: &str) -> bool {
    line.split_once("-populate-prefix/src/")
        .is_some_and(|(_, after)| after.contains("-populate-stamp/"))
}

/// Write `bytes` to the file at `path`, then give it back the times
/// `metadata` recorded.
fn write_keeping_times(
    path: &Path,
    bytes: &[u8],
    metadata: &std::fs::Metadata,
) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;
    let times = std::fs::FileTimes::new()
        .set_accessed(metadata.accessed()?)
        .set_modified(metadata.modified()?);
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_times(times)
}

/// Give each tracked file of `checkout` whose content is the repository
/// checkout's at `source` that file's times, and return how many it gave.
/// The files `git diff` lists against the checkout's commit, and every file
/// that is no regular file in both, keep their own.
fn keep_unchanged_times(source: &Path, checkout: &Path) -> std::io::Result<usize> {
    let head = git_stdout(checkout, &["rev-parse", "HEAD"])?;
    let head = String::from_utf8_lossy(&head).trim().to_string();
    let source_top = git_stdout(source, &["rev-parse", "--show-toplevel"])?;
    let source_top = PathBuf::from(String::from_utf8_lossy(&source_top).trim());
    let changed: BTreeSet<Vec<u8>> = git_stdout(
        source,
        &["diff", "--name-only", "--no-renames", "-z", head.as_str()],
    )?
    .split(|byte| *byte == 0)
    .filter(|entry| !entry.is_empty())
    .map(<[u8]>::to_vec)
    .collect();
    let tracked = git_stdout(checkout, &["ls-files", "--full-name", "-z"])?;
    let checkout_top = git_stdout(checkout, &["rev-parse", "--show-toplevel"])?;
    let checkout_top = PathBuf::from(String::from_utf8_lossy(&checkout_top).trim());
    let mut kept = 0;
    for entry in tracked.split(|byte| *byte == 0) {
        if entry.is_empty() || changed.contains(entry) {
            continue;
        }
        let relative = String::from_utf8_lossy(entry);
        let Ok(original) = std::fs::symlink_metadata(source_top.join(relative.as_ref())) else {
            continue;
        };
        let path = checkout_top.join(relative.as_ref());
        let Ok(own) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !original.is_file() || !own.is_file() {
            continue;
        }
        let times = std::fs::FileTimes::new()
            .set_accessed(original.accessed()?)
            .set_modified(original.modified()?);
        std::fs::File::open(&path)?.set_times(times)?;
        kept += 1;
    }
    Ok(kept)
}

/// The standard output of `git <args>` in `dir`; an error when git fails.
fn git_stdout(dir: &Path, args: &[&str]) -> std::io::Result<Vec<u8>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "git {} failed",
            args.join(" ")
        )));
    }
    Ok(output.stdout)
}

/// Whether the filesystem at `dir` can make the copy-on-write clones
/// [`give_build_trees`] needs. APFS, Btrfs and XFS can; ext4, which CI's
/// runners use, cannot, and there no tree is given and checkouts build cold.
#[cfg(test)]
pub(crate) fn clones_on(dir: &Path) -> bool {
    let probe = dir.join(".clone-probe");
    let copy = dir.join(".clone-probe-copy");
    let cloned =
        std::fs::create_dir_all(&probe).is_ok() && clone_tree(&probe, &copy).unwrap_or(false);
    let _ = std::fs::remove_dir_all(&probe);
    let _ = std::fs::remove_dir_all(&copy);
    cloned
}

/// Whether the relative path `dir` stays inside the directory it is relative
/// to.
fn is_inside(dir: &Path) -> bool {
    !dir.as_os_str().is_empty()
        && dir
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIT: [&str; 8] = [
        "-c",
        "user.name=roko-test",
        "-c",
        "user.email=roko-test@localhost",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "core.hooksPath=/dev/null",
    ];

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(GIT)
            .args(args)
            .current_dir(dir)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A repository with an ignored `build/` and two sources, and a linked
    /// checkout of its commit at `<temp>/checkout`. Paths are canonical, as
    /// CMake records them.
    fn repo_and_checkout(temp: &Path) -> (PathBuf, PathBuf) {
        let repo = temp.join("repo");
        std::fs::create_dir_all(repo.join("src")).expect("src dir");
        std::fs::write(repo.join(".gitignore"), "build/\ndeps/\n").expect("gitignore");
        std::fs::write(repo.join("src/a.c"), "int a(void) { return 1; }\n").expect("a.c");
        std::fs::write(repo.join("src/b.c"), "int b(void) { return 2; }\n").expect("b.c");
        git(&repo, &["init", "--quiet", "--initial-branch=main"]);
        git(&repo, &["add", "--all"]);
        git(&repo, &["commit", "--quiet", "-m", "base"]);
        let repo = repo.canonicalize().expect("canonical repo");
        let checkout = repo.parent().expect("temp").join("checkout");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                checkout.to_str().expect("utf-8"),
                "HEAD",
            ],
        );
        (repo, checkout.canonicalize().expect("canonical checkout"))
    }

    /// The cache entry that makes a tree a build of `repo`.
    fn home_entry(repo: &Path) -> String {
        format!("{CMAKE_HOME_ENTRY}{}\n", repo.display())
    }

    fn mtime(path: &Path) -> std::time::SystemTime {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .expect("mtime")
    }

    fn set_mtime(path: &Path, secs: u64) {
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        std::fs::File::options()
            .write(true)
            .open(path)
            .expect("open")
            .set_times(std::fs::FileTimes::new().set_modified(at).set_accessed(at))
            .expect("set times");
    }

    /// A build tree's text files name the new checkout and keep their
    /// times; a binary file and the repository's own tree are left alone;
    /// a sibling path that only starts with the repository's is untouched.
    #[test]
    fn warm_build_tree_is_relocated_with_its_times() {
        let temp = tempfile::tempdir().expect("tempdir");
        if !clones_on(temp.path()) {
            eprintln!("this filesystem makes no copy-on-write clones; skipping");
            return;
        }
        let (repo, checkout) = repo_and_checkout(temp.path());
        let root = repo.display().to_string();
        std::fs::create_dir_all(repo.join("build/plugins/CMakeFiles/core.dir")).expect("tree");
        let cache = repo.join("build/plugins/CMakeCache.txt");
        std::fs::write(
            &cache,
            format!(
                "CMAKE_HOME_DIRECTORY:INTERNAL={root}\nCMAKE_CACHEFILE_DIR:INTERNAL={root}/build/plugins\nOTHER:PATH={root}-2/lib\n"
            ),
        )
        .expect("cache");
        let depend = repo.join("build/plugins/CMakeFiles/core.dir/compiler_depend.make");
        std::fs::write(
            &depend,
            format!("core.dir/a.c.o: {root}/src/a.c \\\n  {root}/src/b.c\n"),
        )
        .expect("depend");
        let object = repo.join("build/plugins/CMakeFiles/core.dir/a.c.o");
        std::fs::write(&object, [b"\0\x01".as_slice(), root.as_bytes()].concat()).expect("object");
        for file in [&cache, &depend, &object] {
            set_mtime(file, 1_700_000_000);
        }

        let given = give_build_trees(&repo, &checkout, None);

        assert_eq!(given.len(), 1, "{given:?}");
        assert_eq!(given[0].dir, PathBuf::from("build/plugins"));
        assert_eq!(given[0].rewritten, 2, "{given:?}");
        let to = checkout.display().to_string();
        let moved = std::fs::read_to_string(checkout.join("build/plugins/CMakeCache.txt"))
            .expect("moved cache");
        assert!(
            moved.contains(&format!("CMAKE_HOME_DIRECTORY:INTERNAL={to}\n")),
            "{moved}"
        );
        assert!(moved.contains(&format!("{to}/build/plugins\n")), "{moved}");
        assert!(
            moved.contains(&format!("OTHER:PATH={root}-2/lib")),
            "{moved}"
        );
        let moved_depend = std::fs::read_to_string(
            checkout.join("build/plugins/CMakeFiles/core.dir/compiler_depend.make"),
        )
        .expect("moved depend");
        assert!(
            moved_depend.contains(&format!("{to}/src/a.c \\")),
            "{moved_depend}"
        );
        for file in ["CMakeCache.txt", "CMakeFiles/core.dir/compiler_depend.make"] {
            let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
            assert_eq!(
                mtime(&checkout.join("build/plugins").join(file)),
                at,
                "{file}"
            );
        }
        let object_bytes =
            std::fs::read(checkout.join("build/plugins/CMakeFiles/core.dir/a.c.o")).expect("obj");
        assert!(
            object_bytes.ends_with(root.as_bytes()),
            "a binary file was rewritten"
        );
        let original = std::fs::read_to_string(&cache).expect("original cache");
        assert!(
            original.contains(&format!("INTERNAL={root}\n")),
            "the source tree changed"
        );
        assert!(
            git(&checkout, &["status", "--porcelain"]).is_empty(),
            "the given tree shows in the checkout's status"
        );
    }

    /// Only a populate step's rule hashes are dropped; every other rule keeps
    /// its hash, and the file its times.
    #[test]
    fn warm_build_tree_drops_only_populate_rule_hashes() {
        let temp = tempfile::tempdir().expect("tempdir");
        if !clones_on(temp.path()) {
            eprintln!("this filesystem makes no copy-on-write clones; skipping");
            return;
        }
        let (repo, checkout) = repo_and_checkout(temp.path());
        let hashes_dir = repo.join("build/_deps/juce-subbuild/CMakeFiles");
        std::fs::create_dir_all(&hashes_dir).expect("subbuild");
        std::fs::write(repo.join("build/CMakeCache.txt"), home_entry(&repo)).expect("cache");
        let hashes = hashes_dir.join(RULE_HASHES_FILE);
        std::fs::write(
            &hashes,
            "0123456789abcdef0123456789abcdef juce-populate-prefix/src/juce-populate-stamp/juce-populate-download\n\
             fedcba9876543210fedcba9876543210 generated/BinaryData.cpp\n\
             0123456789abcdef0123456789abcdef juce-populate-prefix/src/juce-populate-stamp/juce-populate-patch\n",
        )
        .expect("hashes");
        set_mtime(&hashes, 1_700_000_000);

        let given = give_build_trees(&repo, &checkout, None);

        assert_eq!(given.len(), 1, "{given:?}");
        let moved = checkout
            .join("build/_deps/juce-subbuild/CMakeFiles")
            .join(RULE_HASHES_FILE);
        assert_eq!(
            std::fs::read_to_string(&moved).expect("moved hashes"),
            "fedcba9876543210fedcba9876543210 generated/BinaryData.cpp\n"
        );
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        assert_eq!(mtime(&moved), at);
    }

    /// A tracked file whose content is the repository checkout's gets that
    /// file's times; a file changed in the repository checkout keeps the new
    /// checkout's own, so it rebuilds.
    #[test]
    fn warm_build_tree_gives_unchanged_files_their_times() {
        let temp = tempfile::tempdir().expect("tempdir");
        if !clones_on(temp.path()) {
            eprintln!("this filesystem makes no copy-on-write clones; skipping");
            return;
        }
        let (repo, checkout) = repo_and_checkout(temp.path());
        std::fs::create_dir_all(repo.join("build")).expect("build");
        std::fs::write(repo.join("build/CMakeCache.txt"), home_entry(&repo)).expect("cache");
        set_mtime(&repo.join("src/a.c"), 1_600_000_000);
        std::fs::write(repo.join("src/b.c"), "int b(void) { return 3; }\n").expect("change b.c");
        set_mtime(&repo.join("src/b.c"), 1_600_000_000);
        let fresh = mtime(&checkout.join("src/b.c"));

        let given = give_build_trees(&repo, &checkout, None);

        assert_eq!(given.len(), 1, "{given:?}");
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        assert_eq!(mtime(&checkout.join("src/a.c")), at);
        assert_eq!(
            mtime(&checkout.join("src/b.c")),
            fresh,
            "a changed file got old times"
        );
    }

    /// A tree that names a path only the repository checkout has, such as an
    /// ignored dependency checkout, is not given: the checkout builds cold.
    /// Neither is a tree with a precompiled header. A configured list names
    /// the trees, and `[]` gives none.
    #[test]
    fn warm_build_tree_is_refused_when_it_cannot_move() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (repo, checkout) = repo_and_checkout(temp.path());
        let root = repo.display().to_string();
        std::fs::create_dir_all(repo.join("deps/juce")).expect("deps");
        std::fs::write(repo.join("deps/juce/CMakeLists.txt"), "project(JUCE)\n").expect("juce");
        std::fs::create_dir_all(repo.join("build")).expect("build");
        std::fs::write(
            repo.join("build/CMakeCache.txt"),
            format!(
                "{}JUCE_SOURCE_DIR:STATIC={root}/deps/juce\n",
                home_entry(&repo)
            ),
        )
        .expect("cache");

        assert!(give_build_trees(&repo, &checkout, None).is_empty());
        assert!(
            !checkout.join("build").exists(),
            "a refused tree was left behind"
        );

        std::fs::write(repo.join("build/CMakeCache.txt"), home_entry(&repo)).expect("cache");
        std::fs::write(repo.join("build/cmake_pch.hxx.pch"), b"\0pch").expect("pch");
        assert!(give_build_trees(&repo, &checkout, None).is_empty());
        assert!(
            !checkout.join("build").exists(),
            "a tree with a PCH was given"
        );

        std::fs::remove_file(repo.join("build/cmake_pch.hxx.pch")).expect("drop the pch");
        assert!(give_build_trees(&repo, &checkout, Some(&[])).is_empty());
        let named = give_build_trees(&repo, &checkout, Some(&["build/".to_string()]));
        if clones_on(temp.path()) {
            assert_eq!(named.len(), 1, "{named:?}");
        }
        assert_eq!(
            warm_build_dirs(&repo, Some(&["../repo".to_string()])),
            Vec::<PathBuf>::new()
        );

        // A scratch project's build in an ignored directory is no build tree
        // of the checkout's.
        std::fs::create_dir_all(repo.join("deps/scratch/build")).expect("scratch build");
        std::fs::write(
            repo.join("deps/scratch/build/CMakeCache.txt"),
            home_entry(&repo.join("deps/scratch")),
        )
        .expect("scratch cache");
        assert_eq!(warm_build_dirs(&repo, None), vec![PathBuf::from("build")]);
    }

    /// The real thing, when cmake is installed: a CMake project built in the
    /// repository checkout, then a new checkout of its commit given the warm
    /// tree, builds nothing on its first build, and rebuilds a file it
    /// changes.
    #[test]
    fn seeded_worktree_builds_nothing() {
        let Some(cmake) = which("cmake") else {
            eprintln!("cmake is not installed; skipping");
            return;
        };
        let temp = tempfile::tempdir().expect("tempdir");
        if !clones_on(temp.path()) {
            eprintln!("this filesystem makes no copy-on-write clones; skipping");
            return;
        }
        let (repo, _) = repo_and_checkout(temp.path());
        std::fs::write(
            repo.join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.16)\nproject(warm C)\nadd_executable(warm src/main.c src/a.c src/b.c)\n",
        )
        .expect("CMakeLists.txt");
        std::fs::write(
            repo.join("src/main.c"),
            "int a(void);\nint b(void);\nint main(void) { return a() + b() - 3; }\n",
        )
        .expect("main.c");
        git(&repo, &["add", "--all"]);
        git(&repo, &["commit", "--quiet", "-m", "cmake project"]);
        let cmake_run = |dir: &Path, args: &[&str]| -> String {
            let output = std::process::Command::new(&cmake)
                .args(args)
                .current_dir(dir)
                .env_remove("MAKEFLAGS")
                .output()
                .expect("run cmake");
            assert!(
                output.status.success(),
                "cmake {args:?}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        cmake_run(&repo, &["-S", ".", "-B", "build", "-G", "Unix Makefiles"]);
        cmake_run(&repo, &["--build", "build"]);
        let checkout = temp.path().join("seeded");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                checkout.to_str().expect("utf-8"),
                "HEAD",
            ],
        );
        let checkout = checkout.canonicalize().expect("canonical checkout");

        let given = give_build_trees(&repo, &checkout, None);

        assert_eq!(given.len(), 1, "{given:?}");
        let configure = cmake_run(&checkout, &["-S", ".", "-B", "build"]);
        // CMake names the build dir in its own form of the path (`/tmp` for
        // `/private/tmp`), so only its tail is compared.
        assert!(configure.contains("seeded/build"), "{configure}");
        let first = cmake_run(&checkout, &["--build", "build"]);
        assert!(
            !first.contains("Building C object"),
            "the first build compiled: {first}"
        );
        std::fs::write(
            checkout.join("src/b.c"),
            "int b(void) { return 2; } /* changed */\n",
        )
        .expect("change b.c");
        let second = cmake_run(&checkout, &["--build", "build"]);
        assert!(
            second.contains("b.c.o"),
            "a changed file did not rebuild: {second}"
        );
        assert!(
            !second.contains("a.c.o"),
            "an unchanged file rebuilt: {second}"
        );
    }

    fn which(program: &str) -> Option<PathBuf> {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(program))
                .find(|candidate| candidate.is_file())
        })
    }
}
