//! Planner-written acceptance tests in a generated plan (3221, G24).
//!
//! Beside its `tasks.toml` block, the planner may write each test it
//! proposes as a fenced block whose info string is `accept:<path>`:
//!
//! ````text
//! ```accept:accept/test_slug.py
//! import unittest
//! ...
//! ```
//! ````
//!
//! and name it as the `src` of a `[task.accept]` entry, which pins it out of
//! the implementer's reach when the plan runs (gap-d14a43). [`extract`] takes
//! the blocks from the planner's output: a path must be relative and under
//! `accept/`, with no `..`, and a plan carries at most [`MAX_FILES`] blocks
//! of at most [`MAX_BYTES`] each. [`check_sources`] holds every
//! `[task.accept]` entry to `accept_issues` against the emitted tests, and
//! [`write`] puts them beside `tasks.toml` once the plan is written. On
//! regeneration a test the plan directory already holds is kept unless the
//! planner emits it again.

use std::ffi::OsStr;
use std::path::{Component, Path};

use crate::task_accept::accept_issues;
use crate::task_parser::TasksFile;

/// The most accept blocks one plan may carry.
pub const MAX_FILES: usize = 8;

/// The largest accept block, in bytes.
pub const MAX_BYTES: usize = 64 * 1024;

/// One planner-written test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptBlock {
    /// Its path, relative to the plan directory, under `accept/`.
    pub path: String,
    /// Its text.
    pub text: String,
}

/// The `accept:<path>` blocks of `output`, in order, or why one is refused:
/// a path that is absolute, outside `accept/` or climbs with `..`, a
/// duplicate, a block that is not closed, one over [`MAX_BYTES`], or more
/// than [`MAX_FILES`].
pub fn extract(output: &str) -> Result<Vec<AcceptBlock>, String> {
    let mut blocks: Vec<AcceptBlock> = Vec::new();
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        let Some(path) = line
            .trim_start()
            .strip_prefix("```")
            .and_then(|info| info.trim().strip_prefix("accept:"))
        else {
            continue;
        };
        let path = path.trim().to_string();
        let mut text = String::new();
        let mut closed = false;
        for body in lines.by_ref() {
            if body.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            text.push_str(body);
            text.push('\n');
        }
        if !closed {
            return Err(format!("accept block `{path}` is not closed with ```"));
        }
        check_path(&path)?;
        if text.len() > MAX_BYTES {
            return Err(format!(
                "accept block `{path}` is refused: it has {} bytes, more than {MAX_BYTES}",
                text.len()
            ));
        }
        if blocks.iter().any(|block| block.path == path) {
            return Err(format!("accept block `{path}` is emitted twice"));
        }
        blocks.push(AcceptBlock { path, text });
        if blocks.len() > MAX_FILES {
            return Err(format!(
                "the plan emits more than {MAX_FILES} accept blocks"
            ));
        }
    }
    Ok(blocks)
}

/// Refuse a path that is not relative, not under `accept/`, or not plain
/// file names after it.
fn check_path(path: &str) -> Result<(), String> {
    let refused = |why: &str| Err(format!("accept block `{path}` is refused: {why}"));
    let mut components = Path::new(path).components();
    if components.next() != Some(Component::Normal(OsStr::new("accept"))) {
        return refused("its path must be relative and under accept/");
    }
    let rest: Vec<Component> = components.collect();
    if rest.is_empty() {
        return refused("it names no file under accept/");
    }
    if rest
        .iter()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return refused("its path must not contain `..`");
    }
    Ok(())
}

/// Whether every `[task.accept]` entry of `plan_toml` passes
/// `accept_issues` against the tests `blocks` emits, with the tests
/// `plan_dir` already holds where `blocks` emits no new copy. The error
/// lists each blocking issue and says how to emit a test.
pub fn check_sources(
    plan_toml: &str,
    blocks: &[AcceptBlock],
    plan_dir: &Path,
) -> Result<(), String> {
    let plan = TasksFile::parse_str(plan_toml).map_err(|error| format!("{error:#}"))?;
    if plan.tasks.iter().all(|task| task.accept.is_none()) {
        return Ok(());
    }
    let staging = tempfile::tempdir().map_err(|error| error.to_string())?;
    write(staging.path(), blocks).map_err(|error| error.to_string())?;
    // On regeneration, a test the plan already has stays unless re-emitted.
    for entry in plan
        .tasks
        .iter()
        .flat_map(|task| task.accept.iter().flat_map(|accept| &accept.files))
    {
        let (kept, staged) = (plan_dir.join(&entry.src), staging.path().join(&entry.src));
        if check_path(&entry.src).is_ok() && kept.is_file() && !staged.exists() {
            if let Some(parent) = staged.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::copy(&kept, &staged).map_err(|error| error.to_string())?;
        }
    }
    let problems: Vec<String> = plan
        .tasks
        .iter()
        .flat_map(|task| accept_issues(task, staging.path()))
        .filter(|issue| issue.blocking)
        .map(|issue| format!("  - {}", issue.message))
        .collect();
    if problems.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the plan's [task.accept] entries need their tests: write each one as a fenced \
         ```accept:accept/<file> block after the tasks.toml block, and name that path as its \
         src:\n{}",
        problems.join("\n")
    ))
}

/// Write `blocks` under `dir`, creating directories as needed; a file
/// there that no block names is left as it is.
pub fn write(dir: &Path, blocks: &[AcceptBlock]) -> std::io::Result<()> {
    for block in blocks {
        let path = dir.join(&block.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &block.text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3221: blocks come out with their paths and text; a path outside
    /// `accept/`, an absolute one, one that climbs, a duplicate and an
    /// oversized block are refused.
    #[test]
    fn accept_blocks_are_extracted_and_paths_outside_accept_refused() {
        let output = "```toml\n[meta]\nplan = \"p\"\n```\n\n\
                      ```accept:accept/test_slug.py\nimport unittest\n```\n\
                      ```accept: accept/nested/test_more.py \nprint(1)\n```\n";
        let blocks = extract(output).expect("two blocks");
        assert_eq!(
            blocks,
            [
                AcceptBlock {
                    path: "accept/test_slug.py".to_string(),
                    text: "import unittest\n".to_string(),
                },
                AcceptBlock {
                    path: "accept/nested/test_more.py".to_string(),
                    text: "print(1)\n".to_string(),
                },
            ]
        );
        assert_eq!(extract("```toml\nx = 1\n```\n"), Ok(Vec::new()));

        for path in [
            "tests/test_slug.py",
            "/tmp/accept/test.py",
            "accept/../src/lib.rs",
            "accept",
            "../accept/x.py",
        ] {
            let error = extract(&format!("```accept:{path}\nx\n```\n")).expect_err(path);
            assert!(error.contains("is refused"), "{path}: {error}");
        }
        let twice = "```accept:accept/a.py\nx\n```\n```accept:accept/a.py\ny\n```\n";
        assert!(extract(twice).expect_err("twice").contains("twice"));
        let open = "```accept:accept/a.py\nx\n";
        assert!(extract(open).expect_err("open").contains("not closed"));
        let big = format!("```accept:accept/a.py\n{}\n```\n", "x".repeat(MAX_BYTES));
        assert!(extract(&big).expect_err("big").contains("bytes"));
        let many: String = (0..=MAX_FILES)
            .map(|index| format!("```accept:accept/t{index}.py\nx\n```\n"))
            .collect();
        assert!(extract(&many).expect_err("many").contains("more than"));
    }
}
