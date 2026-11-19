//! Overlap between nodes' exclusive paths.
//!
//! A node's [`Node::exclusive`](crate::types::Node::exclusive) paths are the
//! files and directories it writes in a working tree shared with other nodes.
//! The engine's ready-queue scheduler never runs two nodes whose paths overlap
//! at the same time, and `roko plan validate` flags plan tasks that could.
//!
//! A build-registration file ([`is_registration_file`]) is a list of sources
//! or modules, such as a `CMakeLists.txt`, that several tasks each add an
//! entry to (bug-daa54e). Plan generation lets them all list it
//! ([`first_overlap_except_registration`]). The scheduler still never runs
//! two nodes that name it at the same time in one working tree, where one
//! write could overwrite the other; with a worktree per task, nothing is
//! serialized, and their edits meet at merge.

use std::path::{Component, Path, PathBuf};

/// The file names of build-registration files: lists of sources or modules
/// that parallel tasks each add an entry to (bug-daa54e).
pub const REGISTRATION_FILES: &[&str] = &[
    "BUILD",
    "BUILD.bazel",
    "CMakeLists.txt",
    "Cargo.toml",
    "__init__.py",
    "index.js",
    "index.ts",
    "meson.build",
    "mod.rs",
];

/// Whether `path` names a build-registration file, by its file name
/// ([`REGISTRATION_FILES`]).
///
/// Plan generation lets several tasks list one, and a merge may combine both
/// sides' added lines in one. In a shared working tree, the scheduler still
/// runs the nodes that name one at different times.
#[must_use]
pub fn is_registration_file(path: &str) -> bool {
    lexical(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| REGISTRATION_FILES.contains(&name))
}

/// The first path in `wanted` that overlaps a path in `held`, together with
/// the path it overlaps.
///
/// Two paths overlap when they name the same file, or when one names a
/// directory that holds the other.
#[must_use]
pub fn first_overlap<'a>(wanted: &'a [String], held: &'a [String]) -> Option<(&'a str, &'a str)> {
    wanted.iter().find_map(|path| {
        held.iter()
            .find(|other| overlaps(path, other))
            .map(|other| (path.as_str(), other.as_str()))
    })
}

/// [`first_overlap`], except that two paths naming the same
/// build-registration file do not overlap ([`is_registration_file`]).
///
/// Plan generation's checks use it, so several tasks may list the file they
/// each add to (bug-daa54e); the scheduler uses [`first_overlap`]. A directory
/// that holds a registration file still overlaps it.
#[must_use]
pub fn first_overlap_except_registration<'a>(
    wanted: &'a [String],
    held: &'a [String],
) -> Option<(&'a str, &'a str)> {
    wanted.iter().find_map(|path| {
        held.iter()
            .find(|other| overlaps(path, other) && !shared_registration(path, other))
            .map(|other| (path.as_str(), other.as_str()))
    })
}

/// Whether two declared paths name the same build-registration file, which
/// tasks share rather than hold.
fn shared_registration(left: &str, right: &str) -> bool {
    lexical(left) == lexical(right) && is_registration_file(left)
}

/// Whether two declared paths name the same file, or one names a directory
/// that holds the other.
///
/// Paths compare lexically and component by component, as
/// `sibling_settle::declares` in roko-cli does: `src/app` covers
/// `src/app/view.tsx` but not `src/app.rs`. Unlike `declares`, a path is
/// never matched by its suffix, since both paths are declared relative to the
/// same working tree. A path that is empty once normalized declares nothing.
fn overlaps(left: &str, right: &str) -> bool {
    let (left, right) = (lexical(left), lexical(right));
    if left.as_os_str().is_empty() || right.as_os_str().is_empty() {
        return false;
    }
    left.starts_with(&right) || right.starts_with(&left)
}

/// `path` trimmed, without `.` components, and with each `..` removing the
/// component before it.
fn lexical(path: &str) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in Path::new(path.trim()).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(paths: &[&str]) -> Vec<String> {
        paths.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_same_file_overlaps() {
        assert!(overlaps("src/lib.rs", "src/lib.rs"));
        assert!(overlaps("./src/lib.rs", "src//lib.rs"));
        assert!(overlaps("src/old/../lib.rs", " src/lib.rs "));
    }

    #[test]
    fn a_directory_overlaps_the_paths_inside_it() {
        assert!(overlaps("web/src/stage", "web/src/stage/PlanView.tsx"));
        assert!(overlaps("web/src/stage/PlanView.tsx", "web/src/stage/"));
        assert!(overlaps("web", "web/src/stage/PlanView.tsx"));
    }

    #[test]
    fn paths_overlap_by_component_not_by_text() {
        assert!(!overlaps("src/app", "src/app.rs"));
        assert!(!overlaps("crates/a/src/lib.rs", "crates/ab/src/lib.rs"));
        assert!(!overlaps("src/lib.rs", "crates/a/src/lib.rs"));
    }

    #[test]
    fn an_empty_path_declares_nothing() {
        assert!(!overlaps("", "src/lib.rs"));
        assert!(!overlaps(".", "src/lib.rs"));
        assert!(!overlaps(" ", " "));
    }

    /// bug-daa54e: phase 1's three parallel tasks each add their source to
    /// `core/CMakeLists.txt`. For plan generation, naming the same
    /// registration file is no overlap; a directory that holds one still
    /// overlaps it, and so does a second, overlapping path of the same nodes.
    /// The scheduler's own overlap counts the shared file.
    #[test]
    fn a_shared_registration_file_is_no_overlap() {
        let oscillator = paths(&["core/src/oscillator.cpp", "core/CMakeLists.txt"]);
        let filter = paths(&["core/src/filter.cpp", "./core/CMakeLists.txt"]);
        assert_eq!(
            first_overlap_except_registration(&oscillator, &filter),
            None
        );
        assert_eq!(
            first_overlap(&oscillator, &filter),
            Some(("core/CMakeLists.txt", "./core/CMakeLists.txt"))
        );
        assert_eq!(
            first_overlap_except_registration(&oscillator, &paths(&["core"])),
            Some(("core/src/oscillator.cpp", "core"))
        );
        let both = paths(&["core/CMakeLists.txt", "core/src/oscillator.cpp"]);
        assert_eq!(
            first_overlap_except_registration(&oscillator, &both),
            Some(("core/src/oscillator.cpp", "core/src/oscillator.cpp"))
        );
        for path in [
            "CMakeLists.txt",
            "crates/x/src/mod.rs",
            "pkg/__init__.py",
            "web/src/index.ts",
        ] {
            assert!(is_registration_file(path), "{path}");
        }
        for path in ["core/src/engine.cpp", "crates/x/src/lib.rs", "package.json"] {
            assert!(!is_registration_file(path), "{path}");
        }
    }

    #[test]
    fn first_overlap_names_both_paths() {
        let wanted = paths(&["docs/guide.md", "web/src/stage/PlanView.tsx"]);
        let held = paths(&["web/src/api.ts", "web/src/stage"]);
        assert_eq!(
            first_overlap(&wanted, &held),
            Some(("web/src/stage/PlanView.tsx", "web/src/stage"))
        );
        assert_eq!(first_overlap(&wanted, &paths(&["web/src/api.ts"])), None);
        assert_eq!(first_overlap(&[], &held), None);
    }
}
