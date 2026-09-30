//! Overlap between nodes' exclusive paths.
//!
//! A node's [`Node::exclusive`](crate::types::Node::exclusive) paths are the
//! files and directories it writes in a working tree shared with other nodes.
//! The engine's ready-queue scheduler never runs two nodes whose paths overlap
//! at the same time, and `roko plan validate` flags plan tasks that could.

use std::path::{Component, Path, PathBuf};

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
