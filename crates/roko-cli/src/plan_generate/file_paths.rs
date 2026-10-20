//! A task's `files` entry that names a path the workspace lacks where the
//! plan means a file it has (bug-7c6f1b). Round v7's phase-2.1 plan listed
//! `core/src/envelope.h` for `core/include/undertow/envelope.h`, in `files`
//! and in the task's own text, while the task's `read_files` named the real
//! header, under a spec that says "No new files". Generation points such an
//! entry at the file, and when the source says the work adds no file, sends
//! the planner back each entry the workspace still lacks.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::runnable::NOT_THE_PROJECT;
use crate::task_parser::{TaskDef, TasksFile};

/// The most directory entries a search of the workspace for a file name
/// visits.
const WORKSPACE_ENTRIES: usize = 50_000;

/// Whether `source` says its work adds no file: "No new files", "adds no
/// files", "creates no new file".
pub(super) fn says_no_new_files(source: &str) -> bool {
    static NO_NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bno new files?\b|\b(?:adds?|creates?) no (?:new )?files?\b")
            .expect("valid no-new-files regex")
    });
    NO_NEW.is_match(source)
}

/// Point each `files` entry the workspace lacks, and that no other task
/// lists, at the existing file of the same name that its task reads (its
/// `read_files`). When `no_new_files` (the source adds no file), an entry
/// whose task reads no such file takes the one file of that name the
/// workspace has. The task's own text follows: its title, description,
/// acceptance, goal, non-goals, assumptions and open questions. Returns one
/// line per entry changed.
pub(super) fn repair_file_paths(
    plan: &mut TasksFile,
    workdir: &Path,
    no_new_files: bool,
) -> Vec<String> {
    let mut by_name: Option<HashMap<String, Vec<String>>> = None;
    let mut repaired = Vec::new();
    for index in 0..plan.tasks.len() {
        let task = &plan.tasks[index];
        let mut moves = Vec::new();
        for path in &task.files {
            if !is_plain_path(path)
                || workdir.join(path).exists()
                || listed_elsewhere(plan, index, path)
            {
                continue;
            }
            let Some(name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let read: Vec<&str> = task
                .context
                .iter()
                .flat_map(|context| &context.read_files)
                .map(|read| read.path.as_str())
                .filter(|read| {
                    *read != path
                        && Path::new(read).file_name().and_then(|name| name.to_str()) == Some(name)
                        && workdir.join(read).is_file()
                })
                .collect();
            let found = match read.as_slice() {
                [read] => Some(((*read).to_string(), "the file of that name it reads")),
                [] if no_new_files => {
                    let by_name = by_name.get_or_insert_with(|| workspace_files(workdir));
                    match by_name.get(name).map(Vec::as_slice) {
                        Some([only]) if only != path => Some((
                            only.clone(),
                            "the one file of that name in the workspace, and the source adds no \
                             file",
                        )),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some((to, why)) = found {
                moves.push((path.clone(), to, why));
            }
        }
        let task = &mut plan.tasks[index];
        for (from, to, why) in moves {
            if task.files.contains(&to) {
                task.files.retain(|file| *file != from);
            } else {
                for file in &mut task.files {
                    if *file == from {
                        file.clone_from(&to);
                    }
                }
            }
            replace_in_text(task, &from, &to);
            repaired.push(format!(
                "{}: `{from}` became `{to}` in its files and its text: `{from}` does not exist, \
                 and `{to}` is {why}",
                task.id
            ));
        }
    }
    repaired
}

/// The `files` entries of `plan` the workspace lacks, one line each for the
/// planner, for a source that adds no file (bug-7c6f1b).
pub(super) fn new_files(plan: &TasksFile, workdir: &Path) -> Vec<String> {
    plan.tasks
        .iter()
        .flat_map(|task| {
            task.files
                .iter()
                .filter(|path| is_plain_path(path) && !workdir.join(path).exists())
                .map(move |path| {
                    format!(
                        "{} lists `{path}` in its files, which does not exist, and the source \
                         says no new files: name the existing file it changes",
                        task.id
                    )
                })
        })
        .collect()
}

/// Whether `path` names one file: not a directory (`core/src/`) or a glob.
fn is_plain_path(path: &str) -> bool {
    !path.is_empty() && !path.ends_with('/') && !path.contains(['*', '?', '['])
}

/// Whether a task of `plan` other than `plan.tasks[index]` lists `path`, so
/// that the plan may create it.
fn listed_elsewhere(plan: &TasksFile, index: usize, path: &str) -> bool {
    plan.tasks
        .iter()
        .enumerate()
        .any(|(other, task)| other != index && task.files.iter().any(|file| file == path))
}

/// The workspace's files by name, relative, outside hidden directories,
/// build output and vendored code.
fn workspace_files(root: &Path) -> HashMap<String, Vec<String>> {
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let mut pending = vec![std::path::PathBuf::new()];
    let mut visited = 0;
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&dir)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            visited += 1;
            if visited > WORKSPACE_ENTRIES {
                return by_name;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = dir.join(&name);
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                if !name.starts_with('.') && !NOT_THE_PROJECT.contains(&name.as_str()) {
                    pending.push(path);
                }
            } else {
                by_name
                    .entry(name)
                    .or_default()
                    .push(path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    by_name
}

/// `task`'s text with each mention of the path `from` made `to`.
fn replace_in_text(task: &mut TaskDef, from: &str, to: &str) {
    let replace = |text: &mut String| *text = replace_path(text, from, to);
    replace(&mut task.title);
    if let Some(description) = &mut task.description {
        replace(description);
    }
    if let Some(goal) = &mut task.spec.goal {
        replace(goal);
    }
    for text in task
        .acceptance
        .iter_mut()
        .chain(&mut task.spec.non_goals)
        .chain(&mut task.spec.assumptions)
        .chain(&mut task.spec.open_questions)
    {
        replace(text);
    }
}

/// `text` with each mention of the path `from` that is a path of its own,
/// not a part of a longer path or name, replaced by `to`.
fn replace_path(text: &str, from: &str, to: &str) -> String {
    let part_of = |c: char| c.is_alphanumeric() || matches!(c, '/' | '_' | '-');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let before = rest[..at].chars().next_back();
        let after = &rest[at + from.len()..];
        // A `.` after it ends a sentence unless more of a name follows.
        let continues = after.chars().next().is_some_and(|c| {
            part_of(c) || (c == '.' && after[1..].starts_with(|c: char| part_of(c)))
        });
        let own = !before.is_some_and(|c| part_of(c) || c == '.') && !continues;
        out.push_str(&rest[..at]);
        out.push_str(if own { to } else { from });
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A workspace with the real headers under `core/include/undertow/`.
    fn workspace() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        for path in [
            "core/include/undertow/envelope.h",
            "core/src/envelope.cpp",
            "core/include/undertow/voice.h",
            "core/src/voice.cpp",
        ] {
            let path = temp.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, "// code\n").expect("a file");
        }
        temp
    }

    /// A one-task plan writing `files`, which reads `reads`.
    fn plan(files: &[&str], reads: &[&str]) -> TasksFile {
        let quoted = |paths: &[&str]| {
            paths
                .iter()
                .map(|path| format!("{path:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let reads: String = reads
            .iter()
            .map(|path| {
                format!("\n[[task.context.read_files]]\npath = {path:?}\nwhy = \"the code\"\n")
            })
            .collect();
        TasksFile::parse_str(&format!(
            "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"Smooth sustain\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"mechanical\"\nfiles = [{}]\n\
             description = \"Update the envelope internals in core/src/envelope.h and \
             core/src/envelope.cpp, not core/src/envelope.hpp.\"\n\
             acceptance = [\"AC1: `core/src/envelope.h` keeps its API.\"]\n{reads}",
            quoted(files)
        ))
        .expect("a plan")
    }

    /// bug-7c6f1b: round v7's phase-2.1 plan listed `core/src/envelope.h`
    /// where its own `read_files` named `core/include/undertow/envelope.h`.
    /// The entry takes the path it reads, in `files` and in the task's text
    /// (a longer name stays); without that read, a missing entry may be a
    /// file the task creates and stays, unless the source adds no file and
    /// the workspace has one file of that name. An entry another task lists
    /// stays, and so do two reads of that name.
    #[test]
    fn a_missing_files_path_takes_the_read_files_path_of_the_same_name() {
        let temp = workspace();
        let root = temp.path();
        let mut repaired = plan(
            &["core/src/envelope.h", "core/src/envelope.cpp"],
            &["core/include/undertow/envelope.h", "core/src/envelope.cpp"],
        );
        let lines = repair_file_paths(&mut repaired, root, false);
        assert_eq!(
            lines,
            [
                "T1: `core/src/envelope.h` became `core/include/undertow/envelope.h` in its files \
                 and its text: `core/src/envelope.h` does not exist, and \
                 `core/include/undertow/envelope.h` is the file of that name it reads"
            ]
        );
        let task = &repaired.tasks[0];
        assert_eq!(
            task.files,
            ["core/include/undertow/envelope.h", "core/src/envelope.cpp"]
        );
        assert_eq!(
            task.description.as_deref(),
            Some(
                "Update the envelope internals in core/include/undertow/envelope.h and \
                 core/src/envelope.cpp, not core/src/envelope.hpp."
            )
        );
        assert_eq!(
            task.acceptance,
            ["AC1: `core/include/undertow/envelope.h` keeps its API."]
        );
        assert_eq!(new_files(&repaired, root), Vec::<String>::new());

        // Without the read, the entry may be new: it stays unless the source
        // adds no file, and then the workspace's one file of that name.
        let mut kept = plan(&["core/src/envelope.h"], &[]);
        assert!(repair_file_paths(&mut kept, root, false).is_empty());
        assert_eq!(kept.tasks[0].files, ["core/src/envelope.h"]);
        let mut found = plan(&["core/src/envelope.h"], &[]);
        assert_eq!(repair_file_paths(&mut found, root, true).len(), 1);
        assert_eq!(found.tasks[0].files, ["core/include/undertow/envelope.h"]);

        // A file nobody has is new: under "no new files", the planner hears.
        let unknown = plan(&["core/src/glide.h"], &[]);
        assert_eq!(
            new_files(&unknown, root),
            [
                "T1 lists `core/src/glide.h` in its files, which does not exist, and the source \
                 says no new files: name the existing file it changes"
            ]
        );

        // Two reads of that name are no answer.
        std::fs::create_dir_all(root.join("legacy")).expect("legacy");
        std::fs::write(root.join("legacy/envelope.h"), "// old\n").expect("a second header");
        let mut two = plan(
            &["core/src/envelope.h"],
            &["core/include/undertow/envelope.h", "legacy/envelope.h"],
        );
        assert!(repair_file_paths(&mut two, root, true).is_empty());

        // An entry another task lists may be one the plan creates.
        let mut shared = plan(
            &["core/src/envelope.h"],
            &["core/include/undertow/envelope.h"],
        );
        let mut other = shared.tasks[0].clone();
        other.id = "T2".to_string();
        shared.tasks.push(other);
        assert!(repair_file_paths(&mut shared, root, false).is_empty());
    }

    /// The phrases that say a source's work adds no file.
    #[test]
    fn a_source_can_say_it_adds_no_file() {
        for source in [
            "Fixes inside `core/src/envelope.cpp`. No new files, and no change to any API.",
            "This phase adds no files.",
            "It creates no new file.",
        ] {
            assert!(says_no_new_files(source), "{source}");
        }
        for source in ["Add a new file `core/src/lfo.cpp`.", "No new features."] {
            assert!(!says_no_new_files(source), "{source}");
        }
    }
}
