//! A symbol a task names as an anchor in a file it does not read
//! (bug-417c4a). Round v7's phase-4.2 plan write anchored
//! `Look::drawStaticLayer`, then `UndertowEditor::initialiseControls`, in
//! files its T1 owned but did not read, and each draft went back to the
//! planner for a whole new one (PLAN_CONTEXT_SYMBOL): two gpt-5.4 calls.
//! Generation reads the file that holds such a symbol into the task, or makes
//! an anchor no file holds yet a note.

use std::collections::HashMap;
use std::path::Path;

use super::runnable::NOT_THE_PROJECT;
use crate::plan_policy::{
    PlanExecutionPolicy, contains_identifier, explicit_symbol_anchor, find_anchor_line,
    symbol_note as note, transitive_dependency_outputs,
};
use crate::task_parser::{ReadFile, TaskDef, TasksFile};

/// The most directory entries a search of the workspace for a symbol visits.
const WORKSPACE_ENTRIES: usize = 50_000;

/// The largest file a search of the workspace for a symbol reads.
const MAX_SEARCHED_BYTES: u64 = 512 * 1024;

/// The extensions of the source files a symbol is searched for in: code, not
/// the specs, plans and notes that may name it too.
const CODE_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "cs", "cxx", "go", "h", "hh", "hpp", "hxx", "java", "js", "jsx", "kt", "m",
    "mm", "py", "rb", "rs", "scala", "swift", "ts", "tsx",
];

/// The lines a read added for a symbol shows before the symbol's line, when
/// the whole file is longer than a range may be.
const LINES_BEFORE: usize = 40;

/// Where a symbol a task names is.
enum Found {
    /// In one file the task may read.
    One(String),
    /// In several files of the workspace, none of them the task's own.
    Several,
    /// In no file yet: a symbol the task will write.
    Nowhere,
}

/// For each symbol of each task of `plan` written as an anchor that no file
/// it declares holds (PLAN_CONTEXT_SYMBOL), read the file that holds it into
/// the task: its own existing file that holds it, the qualified name
/// (`Look::drawStaticLayer`) before the last part alone, else the workspace's
/// one source file that holds it. The read is whole when the file fits
/// `policy`'s range limit, else a range from a little before the symbol. A
/// symbol no file holds becomes a note, and so does one whose file the task
/// has no room to read ([`PlanExecutionPolicy::max_read_files_for`]; the
/// plan's sources do not count). A symbol several files of the workspace
/// hold is left to the check. A task that reads a file a task it waits for
/// writes is left alone, as the check leaves it. Returns one line per
/// symbol repaired.
pub(super) fn repair_symbol_anchors(
    plan: &mut TasksFile,
    workdir: &Path,
    policy: PlanExecutionPolicy,
) -> Vec<String> {
    let mut workspace: Option<Vec<(String, String)>> = None;
    let mut repaired = Vec::new();
    for index in 0..plan.tasks.len() {
        let task = &plan.tasks[index];
        let Some(context) = &task.context else {
            continue;
        };
        let task_by_id: HashMap<&str, &TaskDef> = plan
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task))
            .collect();
        let outputs = transitive_dependency_outputs(task, &task_by_id);
        if context
            .read_files
            .iter()
            .any(|read| outputs.contains(&read.path))
        {
            continue;
        }
        let mut read_texts: Vec<String> = context
            .read_files
            .iter()
            .filter_map(|read| std::fs::read_to_string(workdir.join(&read.path)).ok())
            .collect();
        let mut reads = context
            .read_files
            .iter()
            .filter(|read| !plan.meta.sources.contains(&read.path))
            .count();
        let room = policy.max_read_files_for(task);
        let mut added: Vec<ReadFile> = Vec::new();
        let mut notes: Vec<(usize, String)> = Vec::new();
        for (number, symbol) in context.symbols.iter().enumerate() {
            let Some(anchor) = explicit_symbol_anchor(symbol) else {
                continue;
            };
            if read_texts
                .iter()
                .any(|text| find_anchor_line(text, &anchor).is_some())
            {
                continue;
            }
            match holder(task, &anchor, workdir, &mut workspace) {
                Found::One(path) if reads < room => {
                    let text = std::fs::read_to_string(workdir.join(&path)).unwrap_or_default();
                    let lines = text.lines().count();
                    let range = (lines > policy.max_range_lines).then(|| {
                        let line = qualified_line(&text, &anchor)
                            .or_else(|| find_anchor_line(&text, &anchor))
                            .unwrap_or(1);
                        let start = line.saturating_sub(LINES_BEFORE).max(1);
                        let end = (start + policy.max_range_lines - 1).min(lines);
                        format!("{start}-{end}")
                    });
                    repaired.push(format!(
                        "{}: reads `{path}`{}, which holds `{anchor}`, a symbol it names",
                        task.id,
                        range
                            .as_deref()
                            .map(|range| format!(" (lines {range})"))
                            .unwrap_or_default()
                    ));
                    added.push(ReadFile {
                        path,
                        lines: range,
                        why: format!("Holds `{anchor}`, a symbol the task names."),
                    });
                    read_texts.push(text);
                    reads += 1;
                }
                Found::One(path) => {
                    notes.push((number, note(symbol, &anchor)));
                    repaired.push(format!(
                        "{}: `{anchor}` is in `{path}`, but the task reads as many files as it \
                         may, so its symbol entry is a note, not an anchor",
                        task.id
                    ));
                }
                Found::Nowhere => {
                    notes.push((number, note(symbol, &anchor)));
                    repaired.push(format!(
                        "{}: no file holds `{anchor}` yet, so its symbol entry is a note, not an \
                         anchor",
                        task.id
                    ));
                }
                Found::Several => {}
            }
        }
        if let Some(context) = plan.tasks[index].context.as_mut() {
            context.read_files.extend(added);
            for (number, note) in notes {
                context.symbols[number] = note;
            }
        }
    }
    repaired
}

/// Where the symbol `anchor` that `task` names is: the task's own existing
/// file that holds its qualified name, else its last part; else the one
/// source file of the workspace that holds the qualified name, or, when
/// none does, the one that holds its last part.
fn holder(
    task: &TaskDef,
    anchor: &str,
    workdir: &Path,
    workspace: &mut Option<Vec<(String, String)>>,
) -> Found {
    let own: Vec<(&String, String)> = task
        .files
        .iter()
        .filter_map(|file| {
            std::fs::read_to_string(workdir.join(file))
                .ok()
                .map(|text| (file, text))
        })
        .collect();
    if let Some((file, _)) = own
        .iter()
        .find(|(_, text)| qualified_line(text, anchor).is_some())
        .or_else(|| {
            own.iter()
                .find(|(_, text)| find_anchor_line(text, anchor).is_some())
        })
    {
        return Found::One((*file).clone());
    }
    let files = workspace.get_or_insert_with(|| source_files(workdir));
    let qualified: Vec<&String> = files
        .iter()
        .filter(|(_, text)| qualified_line(text, anchor).is_some())
        .map(|(path, _)| path)
        .collect();
    let named: Vec<&String> = if qualified.is_empty() {
        files
            .iter()
            .filter(|(_, text)| find_anchor_line(text, anchor).is_some())
            .map(|(path, _)| path)
            .collect()
    } else {
        qualified
    };
    match named.as_slice() {
        [] => Found::Nowhere,
        [path] => Found::One((*path).clone()),
        _ => Found::Several,
    }
}

/// The first line of `text` that names `anchor` whole, qualified as it is
/// (`Look::drawStaticLayer`), not its last part alone.
fn qualified_line(text: &str, anchor: &str) -> Option<usize> {
    text.lines()
        .position(|line| contains_identifier(line, anchor))
        .map(|index| index + 1)
}

/// The workspace's source files ([`CODE_EXTENSIONS`]) with their text,
/// outside hidden directories, build output and vendored code, by path.
fn source_files(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut pending = vec![std::path::PathBuf::new()];
    let mut visited = 0;
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&dir)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            visited += 1;
            if visited > WORKSPACE_ENTRIES {
                return files;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = dir.join(&name);
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !name.starts_with('.') && !NOT_THE_PROJECT.contains(&name.as_str()) {
                    pending.push(path);
                }
                continue;
            }
            let code = Path::new(&name)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| CODE_EXTENSIONS.contains(&extension));
            let small = entry
                .metadata()
                .is_ok_and(|meta| meta.len() <= MAX_SEARCHED_BYTES);
            if !code || !small {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(root.join(&path)) {
                files.push((path.to_string_lossy().replace('\\', "/"), text));
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 4.2's editor, as its base had it: the functions T1's drafts
    /// anchored, each defined in one file and named in others.
    fn workspace() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        let filler = "// layout\n".repeat(90);
        for (path, text) in [
            (
                "plugin/ui/Look.h",
                "class Look {\n    void drawStaticLayer(juce::Graphics&, float);\n};\n".to_string(),
            ),
            (
                "plugin/ui/Look.cpp",
                format!(
                    "{filler}    drawStaticLayer(ig, scale);\n\
                     void Look::drawStaticLayer(juce::Graphics& g, float) {{\n}}\n"
                ),
            ),
            (
                "plugin/PluginEditor.h",
                "class UndertowEditor {\n    void initialiseControls();\n};\n".to_string(),
            ),
            (
                "plugin/PluginEditor.cpp",
                format!(
                    "{filler}    initialiseControls();\n\
                     void UndertowEditor::initialiseControls() {{\n}}\n"
                ),
            ),
            (
                "plugin/ui/Theme.h",
                "constexpr Panel controlPanels[] = {};\n".to_string(),
            ),
            (
                "docs/specs/phase-4.2-layout.md",
                "The controls and the static layer follow the design.\n".to_string(),
            ),
        ] {
            let path = temp.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, text).expect("a file");
        }
        temp
    }

    /// T1 of phase 4.2's first draft: six files, three reads with the spec,
    /// and three symbols, one of them in a file it does not read.
    fn draft(symbols: &[&str], reads: &[&str]) -> TasksFile {
        let quoted = |items: &[&str]| {
            items
                .iter()
                .map(|item| format!("{item:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let reads: String = reads
            .iter()
            .map(|path| format!("\n[[task.context.read_files]]\npath = {path:?}\nwhy = \"code\"\n"))
            .collect();
        TasksFile::parse_str(&format!(
            "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\
             sources = [\"docs/specs/phase-4.2-layout.md\"]\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"Lay out the editor\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"architectural\"\n\
             files = [\"plugin/ui/Theme.h\", \"plugin/ui/Texts.h\", \"plugin/PluginEditor.h\", \
             \"plugin/PluginEditor.cpp\", \"plugin/ui/Look.h\", \"plugin/ui/Look.cpp\"]\n\n\
             [task.context]\nsymbols = [{}]\n{reads}",
            quoted(symbols)
        ))
        .expect("a plan")
    }

    /// bug-417c4a: phase 4.2's first draft anchored `Look::drawStaticLayer`
    /// while reading the spec, Theme.h and PluginEditor.cpp. The task owns
    /// Look.cpp, which defines it (Look.h and a call name it too), so
    /// Look.cpp joins its reads, whole; a symbol a read holds stays as it is.
    #[test]
    fn a_symbol_in_a_file_the_task_owns_reads_that_file() {
        let temp = workspace();
        let mut plan = draft(
            &[
                "UndertowEditor::initialiseControls — the control loop",
                "undertow::ui::theme::controlPanels — the panel table",
                "Look::drawStaticLayer — current static text and stamp drawing site",
            ],
            &[
                "docs/specs/phase-4.2-layout.md",
                "plugin/ui/Theme.h",
                "plugin/PluginEditor.cpp",
            ],
        );
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_symbol_anchors(&mut plan, temp.path(), policy);
        assert_eq!(
            lines,
            [
                "T1: reads `plugin/ui/Look.cpp`, which holds `Look::drawStaticLayer`, a symbol it names"
            ]
        );
        let context = plan.tasks[0].context.as_ref().expect("a context");
        let added = context.read_files.last().expect("a read");
        assert_eq!(added.path, "plugin/ui/Look.cpp");
        assert_eq!(added.lines, None);
        assert_eq!(
            context.symbols[2],
            "Look::drawStaticLayer — current static text and stamp drawing site"
        );
        // Run again, nothing is left to repair.
        assert!(repair_symbol_anchors(&mut plan, temp.path(), policy).is_empty());
    }

    /// A file longer than a range may be is read from a little before the
    /// symbol's definition; a symbol no file holds becomes a note; and with
    /// no room left, a symbol in an unread file becomes a note too. A symbol
    /// several source files of the workspace hold, none of them the task's,
    /// is left to the check.
    #[test]
    fn a_long_file_gets_a_range_and_a_symbol_without_a_file_becomes_a_note() {
        let temp = workspace();
        let root = temp.path();
        let long = format!(
            "{}void UndertowEditor::initialiseControls() {{\n}}\n{}",
            "// a\n".repeat(300),
            "// b\n".repeat(300)
        );
        std::fs::write(root.join("plugin/PluginEditor.cpp"), long).expect("a long file");
        let policy = PlanExecutionPolicy::generated(8);
        let mut plan = draft(
            &[
                "UndertowEditor::initialiseControls — the control loop",
                "`texts::recordBox` writes each text box",
                "exact:drawnTexts",
            ],
            &["docs/specs/phase-4.2-layout.md", "plugin/ui/Theme.h"],
        );
        let lines = repair_symbol_anchors(&mut plan, root, policy);
        assert_eq!(
            lines,
            [
                "T1: reads `plugin/PluginEditor.cpp` (lines 261-500), which holds \
                 `UndertowEditor::initialiseControls`, a symbol it names",
                "T1: no file holds `texts::recordBox` yet, so its symbol entry is a note, not an \
                 anchor",
                "T1: no file holds `drawnTexts` yet, so its symbol entry is a note, not an anchor",
            ]
        );
        let context = plan.tasks[0].context.as_ref().expect("a context");
        assert_eq!(
            context.symbols[1..],
            [
                "texts::recordBox (in no file the task reads): writes each text box",
                "drawnTexts (in no file the task reads)",
            ]
        );
        for symbol in &context.symbols[1..] {
            assert_eq!(explicit_symbol_anchor(symbol), None, "{symbol}");
        }

        // No room: the task reads as many files as it owns, the spec aside.
        let mut full = draft(
            &["Look::drawStaticLayer — the static layer"],
            &[
                "docs/specs/phase-4.2-layout.md",
                "plugin/ui/Theme.h",
                "plugin/ui/Texts.h",
                "plugin/PluginEditor.h",
                "plugin/PluginEditor.cpp",
                "plugin/ui/Unit.h",
                "plugin/ui/Panel.h",
            ],
        );
        assert_eq!(
            repair_symbol_anchors(&mut full, root, policy),
            [
                "T1: `Look::drawStaticLayer` is in `plugin/ui/Look.cpp`, but the task reads as many \
              files as it may, so its symbol entry is a note, not an anchor"
            ]
        );

        // Several files hold it, none the task's own: the check decides.
        for path in ["core/src/a.cpp", "core/src/b.cpp"] {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, "int Engine::render() { return 0; }\n").expect("a file");
        }
        let mut shared = draft(&["Engine::render — the block loop"], &["plugin/ui/Theme.h"]);
        assert!(repair_symbol_anchors(&mut shared, root, policy).is_empty());
    }
}
