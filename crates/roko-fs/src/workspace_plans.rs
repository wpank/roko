//! Where a workspace keeps its plans.
//!
//! Plans live in `<workspace>/plans/<plan-id>/`, next to the code they change.
//! Older workspaces keep them in `<workspace>/.roko/plans/`, beside roko's
//! runtime state. [`workspace_plans_dir`] is the one rule for choosing between
//! the two. roko-cli (`plan::plans_dir`) and roko-serve (`routes::plans`) both
//! call it, so a new plan is written where plan listing looks for it.
//!
//! `.roko/plans/` also exists in workspaces that have never held a plan:
//! `roko init` creates it, and the enrichment pipeline keeps per-plan artifacts
//! there. Only a plan inside it makes it the plans directory.

use std::fs;
use std::path::{Path, PathBuf};

/// Deepest level below a plans root at which a plan directory is found:
/// `plans/<set>/<subset>/<plan>/tasks.toml` sits at depth 3. Mirrors
/// `MAX_PLAN_DEPTH` in roko-cli's plan discovery.
const MAX_PLAN_DEPTH: usize = 3;

/// Directory names that are never plans or plan sets (compared
/// case-insensitively). Mirrors roko-cli's plan discovery.
const SKIPPED_DIR_NAMES: &[&str] = &["archive", "archived", "_meta"];

/// Markdown files that sit in a plans directory without being plans.
const NON_PLAN_MARKDOWN: &[&str] = &["index.md", "readme.md", "context.md"];

/// The legacy plans directory, `<workdir>/.roko/plans`.
#[must_use]
pub fn legacy_plans_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("plans")
}

/// Resolve the directory that holds a workspace's plans and receives new ones.
///
/// 1. `<workdir>/plans/` when it is a directory.
/// 2. Otherwise `<workdir>/.roko/plans/` when it holds at least one plan
///    ([`holds_plans`]): a workspace that already keeps its plans there goes on
///    using it.
/// 3. Otherwise `<workdir>/plans/`, which need not exist yet. The code that
///    writes the first plan creates it.
///
/// This only probes the filesystem; it never creates anything.
#[must_use]
pub fn workspace_plans_dir(workdir: &Path) -> PathBuf {
    let top_level = workdir.join("plans");
    if top_level.is_dir() {
        return top_level;
    }
    let legacy = legacy_plans_dir(workdir);
    if holds_plans(&legacy) {
        return legacy;
    }
    top_level
}

/// Does `dir` hold at least one plan?
///
/// A plan is what roko-cli's plan discovery (`find_plan_dirs`,
/// `discover_plans`) lists:
///
/// - a directory holding its own `tasks.toml` or `plan.md`: `dir` itself, or a
///   directory up to three levels below it inside plan sets, skipping
///   `archive/`, `archived/`, `_meta/` and names that do not start with an
///   ASCII letter or digit;
/// - a legacy flat `<name>.md` file directly in `dir`, other than `INDEX.md`,
///   `README.md` and `CONTEXT.md`.
///
/// A missing or unreadable `dir` holds no plans.
#[must_use]
pub fn holds_plans(dir: &Path) -> bool {
    is_plan_dir(dir) || holds_plan_dir(dir, 1) || holds_flat_plan(dir)
}

fn is_plan_dir(dir: &Path) -> bool {
    dir.join("tasks.toml").is_file() || dir.join("plan.md").is_file()
}

/// Is a plan directory among the entries of `dir`, which sits `depth - 1`
/// levels below the plans root, or inside a plan set there?
fn holds_plan_dir(dir: &Path, depth: usize) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !starts_with_plan_prefix(&name)
            || SKIPPED_DIR_NAMES
                .iter()
                .any(|skipped| name.eq_ignore_ascii_case(skipped))
        {
            return false;
        }
        let path = entry.path();
        path.is_dir()
            && (is_plan_dir(&path) || (depth < MAX_PLAN_DEPTH && holds_plan_dir(&path, depth + 1)))
    })
}

fn holds_flat_plan(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        starts_with_plan_prefix(&name)
            && Path::new(&name).extension().is_some_and(|ext| ext == "md")
            && !NON_PLAN_MARKDOWN.contains(&name.as_str())
            && entry.file_type().is_ok_and(|kind| kind.is_file())
    })
}

fn starts_with_plan_prefix(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn new_workspace_uses_top_level_plans_without_creating_it() {
        let tmp = TempDir::new().unwrap();

        assert_eq!(workspace_plans_dir(tmp.path()), tmp.path().join("plans"));
        assert!(!tmp.path().join("plans").exists());
    }

    #[test]
    fn empty_legacy_dir_left_by_init_is_not_the_plans_dir() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(legacy_plans_dir(tmp.path())).unwrap();

        assert_eq!(workspace_plans_dir(tmp.path()), tmp.path().join("plans"));
    }

    #[test]
    fn existing_top_level_dir_wins_over_legacy_plans() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("plans")).unwrap();
        write(
            &legacy_plans_dir(tmp.path()).join("old-plan/tasks.toml"),
            "",
        );

        assert_eq!(workspace_plans_dir(tmp.path()), tmp.path().join("plans"));
    }

    #[test]
    fn legacy_workspace_keeps_its_plans_in_dot_roko() {
        for plan_file in [
            "old-plan/tasks.toml",
            "old-plan/plan.md",
            "programme/01-first/tasks.toml",
            "a/b/at-depth-3/tasks.toml",
            "01-flat-legacy-plan.md",
        ] {
            let tmp = TempDir::new().unwrap();
            let legacy = legacy_plans_dir(tmp.path());
            write(&legacy.join(plan_file), "");

            assert!(holds_plans(&legacy), "{plan_file} is a plan");
            assert_eq!(workspace_plans_dir(tmp.path()), legacy, "{plan_file}");
        }
    }

    #[test]
    fn non_plan_contents_do_not_make_a_legacy_workspace() {
        for non_plan in [
            "INDEX.md",
            "readme.MD",
            "enriched-plan/context.json",
            "archive/old-plan/tasks.toml",
            "Archived/old-plan/tasks.toml",
            "_meta/old-plan/tasks.toml",
            ".hidden/old-plan/tasks.toml",
            "a/b/c/at-depth-4/tasks.toml",
            "notes.txt",
        ] {
            let tmp = TempDir::new().unwrap();
            let legacy = legacy_plans_dir(tmp.path());
            write(&legacy.join(non_plan), "");

            assert!(!holds_plans(&legacy), "{non_plan} is not a plan");
            assert_eq!(
                workspace_plans_dir(tmp.path()),
                tmp.path().join("plans"),
                "{non_plan}"
            );
        }
    }

    #[test]
    fn missing_dir_holds_no_plans() {
        let tmp = TempDir::new().unwrap();

        assert!(!holds_plans(&tmp.path().join("missing")));
    }
}
