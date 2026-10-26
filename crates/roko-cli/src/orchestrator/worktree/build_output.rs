//! Build output an attempt leaves behind, which its commit leaves out
//! (bug-ba0c40).
//!
//! An agent that builds or tests from a subdirectory leaves output where the
//! repository's `.gitignore` may not reach: cargo run in `crates/alpha` under
//! a relative `CARGO_TARGET_DIR` writes `crates/alpha/target/`, which a
//! `/target` rule does not ignore. Acceptance stages everything the checkout
//! holds, so that output used to be committed and delivered.
//!
//! A path the attempt adds is build output when it lies in a build
//! directory: one that holds no file of the commit the attempt builds on, and
//! is named for build output ([`BUILD_OUTPUT_DIRS`]: `target`,
//! `node_modules`, `__pycache__`, `.pytest_cache`) or marked by the tool that
//! wrote it (a `CACHEDIR.TAG`, which cargo writes in its target directory
//! whatever that is called, or CMake's `CMakeCache.txt`). The outermost such
//! directory counts. A compiled Python file ([`BUILD_OUTPUT_EXTENSIONS`]) is
//! build output on its own. A directory the base commit has files in holds
//! sources, whatever its name.
//!
//! Build output stays out of the commit unless the task's `files` name it:
//! an entry that reaches into its build directory (`crates/a/target/doc/`,
//! `**/node_modules/**`) keeps the files it covers. An entry for a directory
//! above it (`crates/a/`) names the sources there, not their build output,
//! except under a `target` directory no tool marked, which may be a source
//! module of that name. Everything else the attempt adds is committed as
//! before.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read as _;
use std::path::Path;

use roko_gate::attempt_diff::{BUILD_OUTPUT_DIRS, BUILD_OUTPUT_EXTENSIONS, covers};

/// What a `CACHEDIR.TAG` starts with (the Cache Directory Tagging
/// Specification).
const CACHEDIR_TAG_SIGNATURE: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";

/// CMake's cache, at the top of every build tree it configures.
const CMAKE_CACHE: &str = "CMakeCache.txt";

/// Most paths the commit message's note names.
const NOTED_PATHS: usize = 10;

/// The build output an attempt added that its commit leaves out (see the
/// module docs). It stays in the attempt's checkout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeftOut {
    /// Build directories left out whole, relative to the checkout.
    pub dirs: Vec<String>,
    /// Files left out one by one: compiled Python files, and the files of a
    /// build directory whose other files the task's `files` name.
    pub files: Vec<String>,
    /// How many new files the two hold.
    pub count: usize,
}

impl LeftOut {
    /// Whether nothing was left out.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty() && self.files.is_empty()
    }

    /// Each path left out, a directory's with a trailing `/`.
    pub fn paths(&self) -> impl Iterator<Item = String> {
        self.dirs
            .iter()
            .map(|dir| format!("{dir}/"))
            .chain(self.files.iter().cloned())
    }

    /// The paths left out as literal pathspecs, each ended by a NUL, for
    /// `--pathspec-from-file` with `--pathspec-file-nul`.
    pub(super) fn pathspecs(&self) -> Vec<u8> {
        let mut specs = Vec::new();
        for path in self.dirs.iter().chain(&self.files) {
            specs.extend_from_slice(b":(literal)");
            specs.extend_from_slice(path.as_bytes());
            specs.push(0);
        }
        specs
    }

    /// The paragraph of the commit's message that says what it left out.
    pub(super) fn note(&self) -> String {
        let paths: Vec<String> = self.paths().collect();
        let mut lines = vec![format!(
            "Not committed: {} new file(s) of build output that the task's files do not name:",
            self.count
        )];
        lines.extend(
            paths
                .iter()
                .take(NOTED_PATHS)
                .map(|path| format!("- {path}")),
        );
        if paths.len() > NOTED_PATHS {
            lines.push(format!("- and {} more", paths.len() - NOTED_PATHS));
        }
        lines.join("\n")
    }
}

/// `git` arguments that list the paths the index adds to commit `base`, each
/// ended by a NUL.
pub(super) fn added_paths_args(base: &str) -> [&str; 8] {
    [
        "diff",
        "--cached",
        "--name-only",
        "--diff-filter=A",
        "--no-renames",
        "--no-color",
        "-z",
        base,
    ]
}

/// `git` arguments that list the directories of commit `base`, each ended by
/// a NUL.
pub(super) fn base_dirs_args(base: &str) -> [&str; 6] {
    ["ls-tree", "-r", "-d", "--name-only", "-z", base]
}

/// `git` arguments that take the paths of the pathspec file
/// `pathspec_flag` names (`--pathspec-from-file=<file>`, written from
/// [`LeftOut::pathspecs`]) out of the index, and leave the files on disk.
pub(super) fn unstage_args(pathspec_flag: &str) -> [&str; 8] {
    [
        "rm",
        "--cached",
        "-r",
        "-q",
        "--force",
        "--ignore-unmatch",
        pathspec_flag,
        "--pathspec-file-nul",
    ]
}

/// The build output among the paths an attempt adds, `added`, that the
/// task's `files` do not name (see the module docs). `added` and `base_dirs`,
/// the directories of the commit it adds them to, are NUL-separated paths
/// relative to `checkout`, as [`added_paths_args`] and [`base_dirs_args`]
/// list them. A path that is not UTF-8 is never left out.
pub(super) fn left_out(
    checkout: &Path,
    added: &[u8],
    base_dirs: &[u8],
    files: &[String],
) -> LeftOut {
    let base_dirs: HashSet<&str> = nul_separated(base_dirs).collect();
    let mut marked = HashMap::new();
    let mut roots: BTreeMap<&str, RootFiles<'_>> = BTreeMap::new();
    for path in nul_separated(added) {
        let Some(root) = build_root(checkout, path, &base_dirs, &mut marked) else {
            continue;
        };
        let entry = roots.entry(root.path).or_insert_with(|| RootFiles {
            dir: root.dir,
            left: Vec::new(),
            kept: false,
        });
        if named(files, path, &root) {
            entry.kept = true;
        } else {
            entry.left.push(path);
        }
    }
    let mut left_out = LeftOut::default();
    for (root, under) in roots {
        left_out.count += under.left.len();
        if under.dir && !under.kept {
            left_out.dirs.push(root.to_string());
        } else {
            left_out
                .files
                .extend(under.left.iter().map(ToString::to_string));
        }
    }
    left_out
}

/// The build directory, or compiled file, that a new path belongs to.
struct BuildRoot<'a> {
    /// Its path, relative to the checkout.
    path: &'a str,
    /// It is a directory.
    dir: bool,
    /// Nothing under it can be sources: a tool marked it, or its name says
    /// so. A `target` directory no tool marked may be a source module.
    certain: bool,
}

/// What the new files under one [`BuildRoot`] come to.
struct RootFiles<'a> {
    /// The root is a directory.
    dir: bool,
    /// The files the task's `files` do not name.
    left: Vec<&'a str>,
    /// The task's `files` name some file under it.
    kept: bool,
}

/// The build root of `path`, a path the attempt adds: the outermost
/// directory above it that holds no file of the base (`base_dirs`) and is
/// named or marked as build output, else the path itself when it is a
/// compiled file. `marked` remembers which directories a tool marked.
fn build_root<'a>(
    checkout: &Path,
    path: &'a str,
    base_dirs: &HashSet<&str>,
    marked: &mut HashMap<&'a str, bool>,
) -> Option<BuildRoot<'a>> {
    for (end, _) in path.match_indices('/') {
        let dir = &path[..end];
        if base_dirs.contains(dir) {
            continue;
        }
        let name = dir.rsplit('/').next().unwrap_or(dir);
        let tool_marked = *marked
            .entry(dir)
            .or_insert_with(|| marked_by_its_tool(&checkout.join(dir)));
        if tool_marked || BUILD_OUTPUT_DIRS.contains(&name) {
            return Some(BuildRoot {
                path: dir,
                dir: true,
                certain: tool_marked || name != "target",
            });
        }
    }
    let compiled = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            BUILD_OUTPUT_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        });
    compiled.then_some(BuildRoot {
        path,
        dir: false,
        certain: true,
    })
}

/// Whether one of the task's `files` names `path`, build output under
/// `root`: it covers `path` and, unless `root` may hold sources, reaches into
/// `root` rather than covering the directory above it as well.
fn named(files: &[String], path: &str, root: &BuildRoot<'_>) -> bool {
    let above = root.path.rsplit_once('/').map_or("", |(parent, _)| parent);
    files
        .iter()
        .any(|entry| covers(entry, path) && (!root.certain || !covers(entry, above)))
}

/// Whether the tool that wrote the directory `dir` marked it as its own: a
/// regular `CACHEDIR.TAG` with the tag's signature (cargo's target
/// directory, pytest's cache), or CMake's cache.
fn marked_by_its_tool(dir: &Path) -> bool {
    let is_file = |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file());
    let tag = dir.join("CACHEDIR.TAG");
    let tagged = is_file(&tag)
        && std::fs::File::open(&tag).is_ok_and(|file| {
            let mut start = Vec::with_capacity(CACHEDIR_TAG_SIGNATURE.len());
            let limit = u64::try_from(CACHEDIR_TAG_SIGNATURE.len()).unwrap_or(u64::MAX);
            file.take(limit).read_to_end(&mut start).is_ok() && start == CACHEDIR_TAG_SIGNATURE
        });
    tagged || is_file(&dir.join(CMAKE_CACHE))
}

/// The UTF-8 paths of NUL-separated git output.
fn nul_separated(output: &[u8]) -> impl Iterator<Item = &str> {
    output
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .filter_map(|path| std::str::from_utf8(path).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NUL-separated `paths`, as git lists them.
    fn listed(paths: &[&str]) -> Vec<u8> {
        let mut listed = Vec::new();
        for path in paths {
            listed.extend_from_slice(path.as_bytes());
            listed.push(0);
        }
        listed
    }

    fn files(entries: &[&str]) -> Vec<String> {
        entries.iter().map(ToString::to_string).collect()
    }

    /// Write `path` under `root`, with its directories.
    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directories");
        std::fs::write(path, text).expect("write a file");
    }

    const CARGO_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n# cargo\n";

    /// bug-ba0c40: what counts as build output, and what of it the task's
    /// `files` keep.
    #[test]
    fn build_output_is_left_out_unless_the_tasks_files_reach_into_it() {
        let checkout = tempfile::tempdir().expect("tempdir");
        let root = checkout.path();
        write(root, "crates/a/target/CACHEDIR.TAG", CARGO_TAG);
        write(root, "out/CACHEDIR.TAG", "not a cache directory tag\n");
        write(
            root,
            "build/CMakeCache.txt",
            "# This is the CMakeCache file.\n",
        );
        let base_dirs = listed(&["crates", "crates/a", "crates/a/src", "lib", "lib/target"]);
        let cargo = [
            "crates/a/target/CACHEDIR.TAG",
            "crates/a/target/debug/deps/a-0123",
            "crates/a/target/doc/a/index.html",
        ];
        let left_out_of = |added: &[&str], entries: &[&str]| {
            left_out(root, &listed(added), &base_dirs, &files(entries))
        };

        // The golden path's T7: cargo's target directory beside the declared
        // change, which stays.
        let mut added = cargo.to_vec();
        added.push("crates/a/tests/accept.rs");
        let left = left_out_of(&added, &["crates/a/src/lib.rs", "crates/a/tests/accept.rs"]);
        assert_eq!(left.dirs, ["crates/a/target"]);
        assert!(left.files.is_empty(), "{left:?}");
        assert_eq!(left.count, 3);
        assert_eq!(
            left.paths().collect::<Vec<_>>(),
            ["crates/a/target/".to_string()]
        );
        // A directory above it names the crate's sources, not its build
        // output, and neither does a glob over the crate.
        for entry in ["crates/a/", "crates/**", "**"] {
            assert_eq!(left_out_of(&cargo, &[entry]).dirs, ["crates/a/target"]);
        }
        // An entry inside it keeps what it covers, and the rest goes one by
        // one.
        let left = left_out_of(&cargo, &["crates/a/target/doc/"]);
        assert!(left.dirs.is_empty(), "{left:?}");
        assert_eq!(
            left.files,
            [
                "crates/a/target/CACHEDIR.TAG",
                "crates/a/target/debug/deps/a-0123"
            ]
        );
        assert!(left_out_of(&cargo, &["crates/*/target/**"]).is_empty());

        // An unmarked `target` may be a new source module: an entry above it
        // keeps it, and nothing keeps it undeclared.
        let module = ["src/target/mod.rs"];
        assert!(left_out_of(&module, &["src/"]).is_empty());
        assert_eq!(left_out_of(&module, &["src/lib.rs"]).dirs, ["src/target"]);
        // A directory the base has files in holds sources, whatever its name.
        assert!(left_out_of(&["lib/target/new.rs"], &[]).is_empty());

        // Named and marked build directories, whatever the task's files.
        let left = left_out_of(
            &[
                "build/CMakeCache.txt",
                "build/main.o",
                "web/node_modules/x/index.js",
                "py/__pycache__/m.cpython-312.pyc",
                "py/old.pyc",
            ],
            &["web/", "py/"],
        );
        assert_eq!(left.dirs, ["build", "py/__pycache__", "web/node_modules"]);
        assert_eq!(left.files, ["py/old.pyc"]);
        assert_eq!(left.count, 5);
        assert!(left_out_of(&["web/node_modules/x/index.js"], &["**/node_modules/**"]).is_empty());

        // Neither a file merely called `CACHEDIR.TAG` nor an undeclared
        // source file is build output.
        assert!(left_out_of(&["out/CACHEDIR.TAG", "out/x", "notes.md"], &["src/"]).is_empty());
    }

    #[test]
    fn the_note_and_pathspecs_name_what_was_left_out() {
        let left = LeftOut {
            dirs: (0..11).map(|n| format!("crates/c{n:02}/target")).collect(),
            files: vec!["py/old.pyc".to_string()],
            count: 40,
        };
        let note = left.note();
        assert!(
            note.starts_with("Not committed: 40 new file(s) of build output"),
            "{note}"
        );
        assert!(note.contains("\n- crates/c00/target/"), "{note}");
        assert!(note.ends_with("\n- and 2 more"), "{note}");
        let specs = left.pathspecs();
        assert!(specs.starts_with(b":(literal)crates/c00/target\0"));
        assert!(specs.ends_with(b":(literal)py/old.pyc\0"));
        assert!(LeftOut::default().is_empty());
    }
}
