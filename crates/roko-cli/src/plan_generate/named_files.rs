//! The files a plan's source tells its tasks to delete or change
//! (bug-aa93e9). Round v8's phase-1 spec retires twelve core files and nine
//! window files and cuts two headers down; the plan written from it listed
//! none of them in any task's `files`, and an implementer changes only its
//! task's files (rule 5 of its prompt), so the core it rewrote could not
//! compile. A generated task may own eight files, and T1 owned its eight
//! edits: the planner had met that limit by leaving the deletions out.
//! Generation puts each such file in the files of the task that changes it,
//! and a file a task deletes counts toward none of its size limits: removing
//! a file is no work for a tier to size.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use roko_gate::attempt_diff::covers;

use super::sizes::code_span_files;
use crate::plan_policy::PlanPolicyViolation;
use crate::task_parser::{TaskDef, TasksFile, role_capabilities};

/// The files the sources of a plan tell its tasks to delete or change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedFiles {
    /// The files a sentence tells a task to delete: "Delete these nine:
    /// `plugin/ui/Look.h`, …", "Deletes: …".
    pub deleted: BTreeSet<String>,
    /// The files a sentence tells a task to change and none tells it to
    /// delete: "Edits: …", "Update `tools/install.sh` …".
    pub changed: BTreeSet<String>,
    /// For each named file, the other files the sentences that name it name:
    /// which task changes it follows them.
    neighbours: BTreeMap<String, BTreeSet<String>>,
}

impl NamedFiles {
    /// Whether the sources name no file for change.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deleted.is_empty() && self.changed.is_empty()
    }
}

/// What a sentence tells a task to do with the files it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Change {
    Delete,
    Edit,
}

/// The files `sources` tell a task to delete or change, among the files the
/// workspace at `workdir` has and git does not ignore. A sentence outside
/// code blocks names them when its first word, once its list marker,
/// emphasis and a lead such as "`plugin/ui/`:" are off, is a verb of change
/// ("Delete", "Deletes:", "Edits:", "Update", "Cut") or the label "Files:".
/// Its files are the full paths in its code spans before any word that keeps
/// a file as it is ("never", "not", "keep", "as it is"), so "cut
/// `plugin/ui/Theme.h` down …, and keep `plugin/ui/Texts.h` as it is" names
/// only the first. A sentence led by anything else names none: a history
/// ("bug-x changed `a.rs`") or a context line is no instruction.
#[must_use]
pub fn named_files(sources: &[&str], workdir: &Path) -> NamedFiles {
    let mut named = NamedFiles::default();
    let mut exists: BTreeMap<String, bool> = BTreeMap::new();
    for sentence in sources.iter().flat_map(|source| sentences(source)) {
        let named_here: Vec<(Change, String)> = instruction(&sentence)
            .into_iter()
            .filter(|(_, path)| {
                *exists
                    .entry(path.clone())
                    .or_insert_with(|| workspace_file(workdir, path))
            })
            .collect();
        let paths: Vec<String> = named_here.iter().map(|(_, path)| path.clone()).collect();
        for (change, path) in &named_here {
            match change {
                Change::Delete => {
                    named.changed.remove(path);
                    named.deleted.insert(path.clone());
                }
                Change::Edit if !named.deleted.contains(path) => {
                    named.changed.insert(path.clone());
                }
                Change::Edit => {}
            }
            named
                .neighbours
                .entry(path.clone())
                .or_default()
                .extend(paths.iter().filter(|other| *other != path).cloned());
        }
    }
    named
}

/// The files the documents of `plan`'s `[meta] sources`, read from
/// `workdir`, tell its tasks to delete or change ([`named_files`]).
#[must_use]
pub fn named_files_of_plan(plan: &TasksFile, workdir: &Path) -> NamedFiles {
    let texts: Vec<String> = plan
        .meta
        .sources
        .iter()
        .filter_map(|source| std::fs::read_to_string(workdir.join(source)).ok())
        .collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    named_files(&texts, workdir)
}

/// The files `task` deletes (bug-4e89b8): each of its `files` that a
/// sentence of the documents of `plan`'s `[meta] sources`, read from
/// `workdir`, tells a task to delete, as [`named_files`] reads them, whether
/// or not the file is still there: a try may have deleted it. Its gate checks
/// they are gone.
#[must_use]
pub fn files_task_deletes(plan: &TasksFile, task: &TaskDef, workdir: &Path) -> Vec<String> {
    let mut deleted = BTreeSet::new();
    for source in &plan.meta.sources {
        let Ok(text) = std::fs::read_to_string(workdir.join(source)) else {
            continue;
        };
        for sentence in sentences(&text) {
            deleted.extend(
                instruction(&sentence)
                    .into_iter()
                    .filter(|(change, path)| *change == Change::Delete && plain_relative(path))
                    .map(|(_, path)| path),
            );
        }
    }
    task.files
        .iter()
        .map(|file| file.trim_start_matches("./"))
        .filter(|file| deleted.contains(*file))
        .map(str::to_string)
        .collect()
}

/// Whether `path` names a file of the workspace at `workdir` that git does
/// not ignore ([`plain_relative`]).
fn workspace_file(workdir: &Path, path: &str) -> bool {
    plain_relative(path)
        && workdir.join(path).is_file()
        && !super::runnable::git_ignores(workdir, path)
}

/// Whether `path` is a plain path inside a workspace: relative, with no glob,
/// and never one that climbs out.
fn plain_relative(path: &str) -> bool {
    !path.starts_with(['/', '~'])
        && !path.contains(['*', '?', '[', ':', '\\'])
        && path.split('/').all(|part| !part.is_empty() && part != "..")
}

/// The sentences of `source` outside its code blocks, each with its code
/// spans. A list item (its marker off) and the lines indented under it are
/// one text, and so are the lines of a paragraph; a heading or a table row is
/// a text of its own. A text ends a sentence at `.`, `!` or `?` before a
/// space, outside a code span.
fn sentences(source: &str) -> Vec<String> {
    static MARKER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(\s*)(?:[-*+]|\d+[.)])\s+").expect("valid list marker regex")
    });
    let mut texts: Vec<String> = Vec::new();
    // The text being read, with the indent of its list marker (`None` for a
    // paragraph).
    let mut open: Option<(Option<usize>, String)> = None;
    let mut fence: Option<&str> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = ["```", "~~~"]
            .into_iter()
            .find(|marker| trimmed.starts_with(marker))
        {
            texts.extend(open.take().map(|(_, text)| text));
            fence = Some(marker);
            continue;
        }
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() || trimmed.starts_with(['#', '|']) {
            texts.extend(open.take().map(|(_, text)| text));
            if !trimmed.is_empty() {
                texts.push(trimmed.to_string());
            }
        } else if let Some(marker) = MARKER.captures(line) {
            texts.extend(open.take().map(|(_, text)| text));
            let item = &line[marker.get(0).map_or(0, |all| all.end())..];
            open = Some((Some(marker[1].len()), item.to_string()));
        } else if let Some((item_indent, text)) = &mut open
            && item_indent.is_none_or(|item_indent| indent > item_indent)
        {
            text.push(' ');
            text.push_str(trimmed);
        } else {
            texts.extend(open.take().map(|(_, text)| text));
            open = Some((None, trimmed.to_string()));
        }
    }
    texts.extend(open.map(|(_, text)| text));
    texts
        .iter()
        .flat_map(|text| split_sentences(text))
        .collect()
}

/// `text` cut into sentences at `.`, `!` or `?` before a space, outside its
/// code spans.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let mut in_code = false;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code
            && matches!(c, '.' | '!' | '?')
            && chars.peek().is_some_and(|(_, next)| next.is_whitespace())
        {
            sentences.push(text[start..=at].trim().to_string());
            start = at + 1;
        }
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        sentences.push(rest.to_string());
    }
    sentences
}

/// What `sentence` tells a task to do with each path it names for that, in
/// order, when its first word is a verb of change (see [`named_files`]).
/// "Delete" names its files for deletion, except one it names after "from",
/// "in" or "of", which it changes ("Remove the line from `roko.toml`"). A
/// verb with a colon leads a list only as a label ("Edits:", "Delete:"), not
/// as a note's ("Update:", "Fix:"), and a verb before a date is a note's
/// ("Update 2026-09-28, …"). When a code span comes between the verb and a
/// colon, what follows the colon explains it ("Add the test `t`: a workspace
/// whose `roko.toml` …"), so only the spans before the colon count.
fn instruction(sentence: &str) -> Vec<(Change, String)> {
    static SPAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`[^`]*`").expect("valid code span regex"));
    static KEEPS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:never|not|no|none|nothing|nor|unchanged|untouched|intact|stays?|keeps?|kept|leaves?|alone|read-only)\b|n't\b|\bas (?:it|they) (?:is|are)\b|\bas is\b",
        )
        .expect("valid keep regex")
    });
    static WITHIN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:from|in|inside|within|of|into)\b").expect("valid preposition regex")
    });
    // The words that lead a sentence of change, and those of them that lead
    // a list with a colon.
    const DELETE: &[&str] = &[
        "delete", "deletes", "deleted", "remove", "removes", "removed", "retire", "retires",
        "retired",
    ];
    const EDIT: &[&str] = &[
        "edit", "edits", "edited", "change", "changes", "changed", "modify", "modifies",
        "modified", "update", "updates", "updated", "rewrite", "rewrites", "replace", "replaces",
        "cut", "cuts", "add", "adds", "fix", "fixes", "extend", "extends", "rename", "renames",
        "move", "moves",
    ];
    const LABELS: &[&str] = &[
        "edits",
        "edit",
        "changes",
        "deletes",
        "delete",
        "deletions",
        "removes",
        "remove",
        "retires",
        "retire",
        "files",
        "writes",
        "owns",
    ];
    // The prose: the sentence with each code span blanked, the same length,
    // so that a word's place is the place of the span after it.
    let mut prose = sentence.to_string();
    for span in SPAN.find_iter(sentence) {
        prose.replace_range(span.range(), &" ".repeat(span.len()));
    }
    // The lead: emphasis, and a code span ending in a colon ("`core/`:").
    let lead = prose
        .find(|c: char| !(c.is_whitespace() || matches!(c, '*' | '_' | ':')))
        .unwrap_or(prose.len());
    let word_end = prose[lead..]
        .find(|c: char| !c.is_ascii_alphabetic())
        .map_or(prose.len(), |end| lead + end);
    let word = prose[lead..word_end].to_ascii_lowercase();
    let after = prose[word_end..].trim_start_matches('*');
    let delete = DELETE.contains(&word.as_str());
    let named = if after.starts_with(':') {
        LABELS.contains(&word.as_str())
    } else {
        (delete || EDIT.contains(&word.as_str()))
            && !after.trim_start().starts_with(|c: char| c.is_ascii_digit())
    };
    if !named {
        return Vec::new();
    }
    // Where the spans that count end: at a word that keeps a file as it is,
    // and at a colon after a span, other than a directory's, that follows
    // the verb.
    let mut end = KEEPS.find(&prose).map_or(prose.len(), |keep| keep.start());
    let spans: Vec<regex::Match<'_>> = SPAN.find_iter(sentence).collect();
    if let Some(colon) = prose[word_end..].find(':').map(|at| word_end + at)
        && spans.iter().any(|span| {
            span.start() > word_end
                && span.start() < colon
                && !span.as_str().trim_matches('`').ends_with('/')
        })
    {
        end = end.min(colon);
    }
    let mut named = Vec::new();
    // Where the prose since the verb, or since the span before, starts.
    let mut since = word_end;
    for span in spans.into_iter().filter(|span| span.start() < end) {
        let change = if delete && !WITHIN.is_match(&prose[since..span.start()]) {
            Change::Delete
        } else {
            Change::Edit
        };
        since = span.end();
        named.extend(
            code_span_files(span.as_str())
                .into_iter()
                .map(|path| (change, path.trim_start_matches("./").to_string())),
        );
    }
    named
}

/// Whether some task of `plan` may change `path`: a writing task lists it
/// in `files`, or lists no files, which leaves it free to change any.
fn listed(plan: &TasksFile, path: &str) -> bool {
    plan.tasks.iter().any(|task| {
        writes(task) && (task.files.is_empty() || task.files.iter().any(|file| covers(file, path)))
    })
}

/// Whether `task`'s role may write files.
fn writes(task: &TaskDef) -> bool {
    role_capabilities(task.role.as_deref().unwrap_or("implementer")).write
}

/// The task of `plan` that changes `path`, a file `named` names that no task
/// lists: of the writing tasks with files, the one that lists most of the
/// files the sentences naming it name ([`NamedFiles::neighbours`]), then
/// the one whose files share the deepest directory with it, then one that
/// has a file of its stem beside it (`PluginEditor.cpp` for
/// `PluginEditor.h`), then the first. `None` when no task writes files.
fn owner(plan: &TasksFile, named: &NamedFiles, path: &str) -> Option<usize> {
    let neighbours = named.neighbours.get(path);
    let (dir, stem) = dir_and_stem(path);
    plan.tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| !task.files.is_empty() && writes(task))
        .map(|(index, task)| {
            let shared = neighbours.map_or(0, |neighbours| {
                neighbours
                    .iter()
                    .filter(|neighbour| task.files.iter().any(|file| covers(file, neighbour)))
                    .count()
            });
            let (mut depth, mut beside) = (0, false);
            for file in &task.files {
                let (file_dir, file_stem) = dir_and_stem(file);
                let common = dir
                    .iter()
                    .zip(&file_dir)
                    .take_while(|(left, right)| left == right)
                    .count();
                depth = depth.max(common);
                beside |= file_dir == dir && file_stem == stem;
            }
            // `max_by_key` keeps the last of equals: the reversed index
            // makes it the first.
            ((shared, depth, beside, usize::MAX - index), index)
        })
        .max_by_key(|(key, _)| *key)
        .map(|(_, index)| index)
}

/// The directories of `path`, outermost first, and its file name up to its
/// first `.`.
fn dir_and_stem(path: &str) -> (Vec<&str>, &str) {
    let path = path.trim_start_matches("./");
    let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
    let dir = dir.split('/').filter(|part| !part.is_empty()).collect();
    (dir, name.split('.').next().unwrap_or(name))
}

/// Put each file `named` tells a task to delete or change, that no task of
/// `plan` lists, in the `files` of the task that changes it ([`owner`]), and
/// with no planner call: the planner left it out to meet the files limit,
/// or missed it. Returns one line per task and kind of change.
pub(super) fn repair_named_files(plan: &mut TasksFile, named: &NamedFiles) -> Vec<String> {
    let mut gained: BTreeMap<(usize, Change), Vec<String>> = BTreeMap::new();
    let kinds = named
        .deleted
        .iter()
        .map(|path| (path, Change::Delete))
        .chain(named.changed.iter().map(|path| (path, Change::Edit)));
    for (path, change) in kinds {
        if listed(plan, path) {
            continue;
        }
        let Some(index) = owner(plan, named, path) else {
            continue;
        };
        plan.tasks[index].files.push(path.clone());
        gained
            .entry((index, change))
            .or_default()
            .push(format!("`{path}`"));
    }
    gained
        .into_iter()
        .map(|((index, change), paths)| {
            let them = if paths.len() == 1 { "it" } else { "them" };
            let what = match change {
                Change::Delete => "delete",
                Change::Edit => "change",
            };
            format!(
                "{}: `files` gains {}: the source says to {what} {them}",
                plan.tasks[index].id,
                paths.join(", ")
            )
        })
        .collect()
}

/// The files `named` tells a task to delete, then those it tells a task to
/// change, that no task of `plan` lists in its `files` (PLAN_062).
#[must_use]
pub fn unlisted_named_files(plan: &TasksFile, named: &NamedFiles) -> (Vec<String>, Vec<String>) {
    let unlisted = |paths: &BTreeSet<String>| {
        paths
            .iter()
            .filter(|path| !listed(plan, path))
            .cloned()
            .collect::<Vec<String>>()
    };
    (unlisted(&named.deleted), unlisted(&named.changed))
}

/// The PLAN_062 warning for `plan`: the files `named`, read from its
/// sources, tells a task to delete or change that no task lists. `None` when
/// every one is listed.
#[must_use]
pub fn unlisted_named_files_line(plan: &TasksFile, named: &NamedFiles) -> Option<String> {
    let (deleted, changed) = unlisted_named_files(plan, named);
    let quoted = |paths: &[String]| {
        paths
            .iter()
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let said = match (deleted.is_empty(), changed.is_empty()) {
        (true, true) => return None,
        (false, true) => format!("delete {}", quoted(&deleted)),
        (true, false) => format!("change {}", quoted(&changed)),
        (false, false) => format!(
            "delete {} and to change {}",
            quoted(&deleted),
            quoted(&changed)
        ),
    };
    let them = if deleted.len() + changed.len() == 1 {
        "it"
    } else {
        "them"
    };
    Some(format!(
        "the documents this plan is written from ([meta] sources) say to {said}, but no task \
         lists {them} in its files, and an agent changes only its own task's files: add each to \
         the files of the task that changes it"
    ))
}

/// The tasks of `plan`, as its size limits count it ([`without_deleted`]),
/// that `issues` find over their files limit while they change files `named`
/// names, one line each for the planner: such a task is split, never cut
/// down, since [`repair_named_files`] puts back each named file it leaves out.
pub(super) fn crowded_tasks<'a>(
    plan: &TasksFile,
    named: &NamedFiles,
    issues: impl IntoIterator<Item = &'a PlanPolicyViolation>,
) -> Vec<String> {
    let mut crowded: Vec<String> = Vec::new();
    for issue in issues {
        let over_files = issue.code == "PLAN_BUDGET_FILES"
            || (issue.code == "PLAN_TIER_SIZE" && issue.message.contains(" files (limit "));
        let Some(task) = issue
            .task_id
            .as_deref()
            .filter(|_| over_files)
            .filter(|id| {
                !crowded
                    .iter()
                    .any(|line| line.starts_with(&format!("{id} ")))
            })
            .and_then(|id| plan.tasks.iter().find(|task| task.id == id))
        else {
            continue;
        };
        let changes: Vec<String> = task
            .files
            .iter()
            .filter(|file| named.changed.contains(file.trim_start_matches("./")))
            .map(|file| format!("`{file}`"))
            .collect();
        if !changes.is_empty() {
            crowded.push(format!(
                "{} changes {} file(s) its source names ({}) among its {} files",
                task.id,
                changes.len(),
                changes.join(", "),
                task.files.len()
            ));
        }
    }
    crowded
}

/// `plan` as its size limits count it: each task's `files` without the files
/// `deleted` names, which a task removes and does not write.
#[must_use]
pub fn without_deleted(plan: &TasksFile, deleted: &BTreeSet<String>) -> TasksFile {
    let mut sized = plan.clone();
    if !deleted.is_empty() {
        for task in &mut sized.tasks {
            task.files
                .retain(|file| !deleted.contains(file.trim_start_matches("./")));
        }
    }
    sized
}

/// Run `size`, which sets tasks' tiers and line budgets by their size, on
/// `plan` as its size limits count it ([`without_deleted`]), and keep the
/// tiers and line budgets it set.
pub(super) fn sized<R>(
    plan: &mut TasksFile,
    deleted: &BTreeSet<String>,
    size: impl FnOnce(&mut TasksFile) -> R,
) -> R {
    if deleted.is_empty() {
        return size(plan);
    }
    let mut sized = without_deleted(plan, deleted);
    let result = size(&mut sized);
    for (task, sized) in plan.tasks.iter_mut().zip(&sized.tasks) {
        task.tier.clone_from(&sized.tier);
        task.max_loc = sized.max_loc;
    }
    result
}

/// The texts of the documents of `plan`'s `[meta] sources`, read from
/// `workdir`; a document that cannot be read is left out.
fn source_texts(plan: &TasksFile, workdir: &Path) -> Vec<String> {
    plan.meta
        .sources
        .iter()
        .filter_map(|source| std::fs::read_to_string(workdir.join(source)).ok())
        .collect()
}

/// Each shorthand of `sources` that names files together (gap-708931), with
/// the files it names: `plugin/PluginEditor.h/.cpp` names
/// `plugin/PluginEditor.h` and `plugin/PluginEditor.cpp`, and
/// `engine.h/.cpp` the two files of those names. Each once.
fn shorthand_pairs(sources: &[&str]) -> Vec<(String, Vec<String>)> {
    static SHORTHAND: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`[^`\s]*[^`\s/]/\.[^`\s]+`").expect("valid shorthand regex"));
    let mut pairs: Vec<(String, Vec<String>)> = Vec::new();
    for found in sources
        .iter()
        .flat_map(|source| SHORTHAND.find_iter(source))
    {
        let files: Vec<String> = code_span_files(found.as_str())
            .into_iter()
            .map(|file| file.trim_start_matches("./").to_string())
            .collect();
        if files.len() > 1 && !pairs.iter().any(|(_, pair)| *pair == files) {
            pairs.push((found.as_str().trim_matches('`').to_string(), files));
        }
    }
    pairs
}

/// The files the writing tasks of `plan` must own too, for each shorthand of
/// `sources` they own one file of and no task owns the others (gap-708931):
/// `(task index, path, shorthand)`. A member named by its path is that file;
/// one named bare is the file of that name beside the task's own file of the
/// shorthand, else the workspace's one file of that name. A member with no
/// such file in `workdir` is none.
fn shorthand_partners(
    plan: &TasksFile,
    sources: &[&str],
    workdir: &Path,
) -> Vec<(usize, String, String)> {
    let member_of = |file: &str, member: &str| {
        let file = file.trim_start_matches("./");
        if member.contains('/') {
            file == member
        } else {
            Path::new(file).file_name().and_then(|name| name.to_str()) == Some(member)
        }
    };
    let mut by_name = None;
    let mut partners: Vec<(usize, String, String)> = Vec::new();
    for (shorthand, members) in shorthand_pairs(sources) {
        for (index, task) in plan.tasks.iter().enumerate() {
            if task.files.is_empty() || !writes(task) {
                continue;
            }
            let Some(owned) = task.files.iter().find(|file| {
                members
                    .iter()
                    .any(|member| member_of(file.as_str(), member.as_str()))
            }) else {
                continue;
            };
            for member in &members {
                if task
                    .files
                    .iter()
                    .any(|file| member_of(file.as_str(), member.as_str()))
                {
                    continue;
                }
                let beside = Path::new(owned.trim_start_matches("./"))
                    .with_file_name(member)
                    .to_string_lossy()
                    .replace('\\', "/");
                let path = if member.contains('/') {
                    Some(member.clone())
                } else if workdir.join(&beside).is_file() {
                    Some(beside)
                } else {
                    let by_name =
                        by_name.get_or_insert_with(|| super::file_paths::workspace_files(workdir));
                    match by_name.get(member.as_str()).map(Vec::as_slice) {
                        Some([only]) => Some(only.clone()),
                        _ => None,
                    }
                };
                let Some(path) = path.filter(|path| {
                    plain_relative(path) && workdir.join(path).is_file() && !listed(plan, path)
                }) else {
                    continue;
                };
                if !partners.iter().any(|(_, known, _)| *known == path) {
                    partners.push((index, path, shorthand.clone()));
                }
            }
        }
    }
    partners
}

/// Give each task of `plan` the files a shorthand of `sources` names beside
/// one it owns ([`shorthand_partners`], gap-708931): round v8's phase-1 plan
/// listed `plugin/PluginEditor.cpp` but not the header of the spec's
/// `plugin/PluginEditor.h/.cpp`. Returns one line per file added.
pub(super) fn repair_shorthand_pairs(
    plan: &mut TasksFile,
    sources: &[&str],
    workdir: &Path,
) -> Vec<String> {
    shorthand_partners(plan, sources, workdir)
        .into_iter()
        .map(|(index, path, shorthand)| {
            let task = &mut plan.tasks[index];
            task.files.push(path.clone());
            format!(
                "{}: `files` gains `{path}`: the source names it with a file of the task in \
                 `{shorthand}`",
                task.id
            )
        })
        .collect()
}

/// The files the shorthands of `plan`'s sources, read from `workdir`, name
/// beside a file a task owns, that no task lists (gap-708931, PLAN_062).
#[must_use]
pub fn shorthand_partners_of_plan(plan: &TasksFile, workdir: &Path) -> Vec<String> {
    let texts = source_texts(plan, workdir);
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    shorthand_partners(plan, &texts, workdir)
        .into_iter()
        .map(|(_, path, _)| path)
        .collect()
}

/// What the sentences of `sources` that say files are deleted without naming
/// one by its path say, around their word of deletion (gap-708931): a word of
/// deletion with "file" or "files", or with a code span that names a file
/// only by its name (`Displays.h/.cpp`). Round v8's first phase-1 spec said
/// "and the twelve retired files deleted" and "Delete the rest:
/// `Displays.h/.cpp`, …", and named its 21 deletions by path nowhere.
fn vague_deletions(sources: &[&str]) -> Vec<String> {
    static DELETES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:delete[sd]?|deleting|remove[sd]?|removing|retire[sd]?|retiring)\b")
            .expect("valid deletion regex")
    });
    static FILES: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bfiles?\b").expect("valid files regex"));
    static SPAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`[^`]*`").expect("valid code span regex"));
    let mut found = Vec::new();
    for sentence in sources.iter().flat_map(|source| sentences(source)) {
        // The prose: each code span blanked, the same length.
        let mut prose = sentence.clone();
        for span in SPAN.find_iter(&sentence) {
            prose.replace_range(span.range(), &" ".repeat(span.len()));
        }
        let Some(word) = DELETES.find(&prose) else {
            continue;
        };
        let names_bare = SPAN
            .find_iter(&sentence)
            .flat_map(|span| code_span_files(span.as_str()))
            .any(|file| !file.contains('/'));
        let names_a_path = instruction(&sentence)
            .iter()
            .any(|(change, path)| *change == Change::Delete && path.contains('/'));
        if (FILES.is_match(&prose) || names_bare) && !names_a_path {
            let start = sentence.floor_char_boundary(word.start().saturating_sub(60));
            let end = sentence.ceil_char_boundary((word.end() + 40).min(sentence.len()));
            found.push(format!(
                "{}{}{}",
                if start > 0 { "…" } else { "" },
                sentence[start..end].trim(),
                if end < sentence.len() { "…" } else { "" }
            ));
        }
    }
    found
}

/// The PLAN_063 warning for `plan` (gap-708931): its sources, read from
/// `workdir`, say files are deleted ([`vague_deletions`]) and name no
/// deletion by its path anywhere, so roko can neither give the files to a
/// task nor check they are gone. `None` otherwise.
#[must_use]
pub fn vague_deletions_line(plan: &TasksFile, workdir: &Path) -> Option<String> {
    let texts = source_texts(plan, workdir);
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    let named_by_path = texts
        .iter()
        .flat_map(|text| sentences(text))
        .any(|sentence| {
            instruction(&sentence)
                .iter()
                .any(|(change, path)| *change == Change::Delete && plain_relative(path))
        });
    let vague = vague_deletions(&texts);
    if named_by_path || vague.is_empty() {
        return None;
    }
    let quoted: Vec<String> = vague
        .iter()
        .take(3)
        .map(|said| format!("\"{said}\""))
        .collect();
    let more = match vague.len().saturating_sub(3) {
        0 => String::new(),
        n => format!(", and {n} more"),
    };
    Some(format!(
        "the documents this plan is written from ([meta] sources) say files are deleted but name \
         none of them by path ({}{more}): write each one in a sentence that deletes it, as \
         \"Deletes: `core/src/lfo.cpp`, …\", so that roko gives it to the task that deletes it \
         and checks it is gone",
        quoted.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_policy::{PlanExecutionPolicy, validate_plan_budgets, validate_tier_sizes};

    /// The twelve core files phase 1 retires.
    const CORE_DELETED: [&str; 12] = [
        "core/include/undertow/oscillator.h",
        "core/src/oscillator.cpp",
        "core/include/undertow/filter.h",
        "core/src/filter.cpp",
        "core/include/undertow/envelope.h",
        "core/src/envelope.cpp",
        "core/include/undertow/lfo.h",
        "core/src/lfo.cpp",
        "core/include/undertow/voice.h",
        "core/src/voice.cpp",
        "core/include/undertow/sequencer.h",
        "core/src/sequencer.cpp",
    ];

    /// The nine window files phase 1 retires.
    const UI_DELETED: [&str; 9] = [
        "plugin/ui/Displays.h",
        "plugin/ui/Displays.cpp",
        "plugin/ui/Look.h",
        "plugin/ui/Look.cpp",
        "plugin/ui/Status.h",
        "plugin/ui/Status.cpp",
        "plugin/ui/UnitArray.h",
        "plugin/ui/UnitArray.cpp",
        "plugin/ui/UiFrame.h",
    ];

    /// The files of phase 1's plan, as written, and the ones it names but
    /// does not change.
    const OTHERS: [&str; 26] = [
        "core/include/undertow/params.h",
        "core/src/params.cpp",
        "core/include/undertow/engine.h",
        "core/src/engine.cpp",
        "core/include/undertow/output_stage.h",
        "core/src/output_stage.cpp",
        "core/include/undertow/presets.h",
        "core/src/presets.cpp",
        "core/include/undertow/rumble.h",
        "core/src/rumble.cpp",
        "core/CMakeLists.txt",
        "plugin/CMakeLists.txt",
        "plugin/PluginProcessor.h",
        "plugin/PluginProcessor.cpp",
        "plugin/PluginEditor.h",
        "plugin/PluginEditor.cpp",
        "plugin/ui/Theme.h",
        "plugin/ui/Texts.h",
        "tools/render/main.cpp",
        "tools/render-demos.sh",
        "tools/install.sh",
        "tools/spec.sh",
        "tools/check.sh",
        "docs/PRD.md",
        "docs/specs/phase-1-foundation.md",
        "tests/spec/p1/params_test.cpp",
    ];

    /// A copy of Undertow's tree as phase 1 found it: every file its spec
    /// names, as an empty file.
    fn undertow() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        for path in CORE_DELETED.iter().chain(&UI_DELETED).chain(&OTHERS) {
            let path = temp.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, "\n").expect("a file");
        }
        temp
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/p1_named_files")
            .join(name)
    }

    fn strings(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_string()).collect()
    }

    /// bug-aa93e9: round v8's phase-1 spec, with every path written out in
    /// each task's notes (Undertow e7131fb), retires 21 files and changes 17,
    /// and its plan as written (op 19bfbbb8) lists none of the 21 and two of
    /// the 17 nowhere: `plugin/PluginEditor.h` and `plugin/ui/Theme.h`.
    /// Each joins the files of the task that changes its neighbours: the
    /// core's to T1, the window's to T2. The deletions count toward no size
    /// limit, so T1 keeps within the 8 files a generated task may own; T2's
    /// nine edits are one over it, for its spec to split.
    #[test]
    fn a_specs_named_deletions_and_edits_join_their_tasks_files() {
        let temp = undertow();
        let root = temp.path();
        let spec = std::fs::read_to_string(fixture("request.md")).expect("the spec");
        let named = named_files(&[spec.as_str()], root);
        let deleted: Vec<&str> = CORE_DELETED.iter().chain(&UI_DELETED).copied().collect();
        assert_eq!(named.deleted, strings(&deleted));
        assert_eq!(
            named.changed,
            strings(&[
                "core/include/undertow/params.h",
                "core/src/params.cpp",
                "core/include/undertow/engine.h",
                "core/src/engine.cpp",
                "core/include/undertow/output_stage.h",
                "core/src/output_stage.cpp",
                "core/include/undertow/presets.h",
                "core/src/presets.cpp",
                "plugin/CMakeLists.txt",
                "plugin/PluginProcessor.h",
                "plugin/PluginProcessor.cpp",
                "plugin/PluginEditor.h",
                "plugin/PluginEditor.cpp",
                "plugin/ui/Theme.h",
                "tools/render/main.cpp",
                "tools/render-demos.sh",
                "tools/install.sh",
            ])
        );

        let mut plan = TasksFile::parse(&fixture("tasks.toml")).expect("the plan as written");
        let (unlisted_deleted, unlisted_changed) = unlisted_named_files(&plan, &named);
        assert_eq!(unlisted_deleted.len(), 21);
        assert_eq!(
            unlisted_changed,
            ["plugin/PluginEditor.h", "plugin/ui/Theme.h"]
        );

        let repairs = repair_named_files(&mut plan, &named);
        assert_eq!(
            repairs,
            [
                format!(
                    "T1: `files` gains {}: the source says to delete them",
                    CORE_DELETED
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        .iter()
                        .map(|path| format!("`{path}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                format!(
                    "T2: `files` gains {}: the source says to delete them",
                    UI_DELETED
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        .iter()
                        .map(|path| format!("`{path}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                "T2: `files` gains `plugin/PluginEditor.h`, `plugin/ui/Theme.h`: the source says \
                 to change them"
                    .to_string(),
            ]
        );
        assert_eq!(plan.tasks[0].files.len(), 20);
        assert_eq!(plan.tasks[1].files.len(), 18);
        assert_eq!(
            unlisted_named_files(&plan, &named),
            (Vec::new(), Vec::new())
        );
        assert!(repair_named_files(&mut plan, &named).is_empty(), "once");

        // What the size limits count: T1's eight edits, T2's nine.
        let policy = PlanExecutionPolicy::generated(8);
        let sized = without_deleted(&plan, &named.deleted);
        let files_over: Vec<String> = validate_plan_budgets(&sized, policy)
            .into_iter()
            .filter(|issue| issue.code == "PLAN_BUDGET_FILES")
            .map(|issue| issue.to_string())
            .collect();
        assert_eq!(
            files_over,
            ["PLAN_BUDGET_FILES [T2]: files has 9 entries; limit is 8"]
        );
        let sizes: Vec<String> = validate_tier_sizes(&sized, policy)
            .into_iter()
            .map(|issue| issue.to_string())
            .collect();
        assert_eq!(sizes.len(), 1, "only T2: {sizes:?}");
        assert!(sizes[0].contains("9 files (limit 8)"), "{sizes:?}");
        // T2 is told to split, not to cut its files down.
        let issues = validate_plan_budgets(&sized, policy);
        let tiers = validate_tier_sizes(&sized, policy);
        assert_eq!(
            crowded_tasks(&sized, &named, issues.iter().chain(&tiers)),
            [
                "T2 changes 9 file(s) its source names (`plugin/CMakeLists.txt`, \
                 `plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`, \
                 `plugin/PluginEditor.cpp`, `tools/render/main.cpp`, `tools/render-demos.sh`, \
                 `tools/install.sh`, `plugin/PluginEditor.h`, `plugin/ui/Theme.h`) among its 9 \
                 files"
            ]
        );
        // Counted with its deletions, T1 would be over every tier.
        assert!(
            validate_tier_sizes(&plan, policy)
                .iter()
                .any(|issue| issue.message.contains("20 files (limit 8)")),
            "deletions counted"
        );
    }

    /// bug-4e89b8: the files each task of phase 1's repaired plan deletes,
    /// which its gate checks are gone: T1's twelve core files and T2's nine
    /// window files, its edits not among them, and still after a try deleted
    /// them.
    #[test]
    fn the_files_a_task_deletes_come_from_its_spec() {
        let temp = undertow();
        let root = temp.path();
        std::fs::copy(
            fixture("request.md"),
            root.join("docs/specs/phase-1-foundation.md"),
        )
        .expect("the spec");
        let mut plan = TasksFile::parse(&fixture("tasks.toml")).expect("the plan as written");
        let named = named_files_of_plan(&plan, root);
        repair_named_files(&mut plan, &named);
        let deletes = |index: usize| -> BTreeSet<String> {
            files_task_deletes(&plan, &plan.tasks[index], root)
                .into_iter()
                .collect()
        };
        assert_eq!(deletes(0), strings(&CORE_DELETED));
        assert_eq!(deletes(1), strings(&UI_DELETED));
        for path in CORE_DELETED.iter().chain(&UI_DELETED) {
            std::fs::remove_file(root.join(path)).expect("a try deletes it");
        }
        assert_eq!(deletes(0).len(), 12);
        assert_eq!(deletes(1).len(), 9);
        // A plan with no sources deletes nothing.
        plan.meta.sources.clear();
        assert!(files_task_deletes(&plan, &plan.tasks[0], root).is_empty());
    }

    fn vague_fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/p1_vague")
            .join(name)
    }

    /// gap-708931: round v8's first phase-1 spec (Undertow 19943c9) named its
    /// file pairs in shorthand, and its plan as written listed
    /// `plugin/PluginEditor.cpp` but not the header of `PluginEditor.h/.cpp`.
    /// T2 gains it, beside the file it lists; a pair a task owns whole stays;
    /// a member named bare whose file sits in another directory is the
    /// workspace's one file of that name.
    #[test]
    fn a_shorthand_pair_stays_in_one_task() {
        let temp = undertow();
        let root = temp.path();
        let spec = std::fs::read_to_string(vague_fixture("request.md")).expect("the spec");
        let mut plan = TasksFile::parse(&vague_fixture("tasks.toml")).expect("the plan as written");
        assert_eq!(
            repair_shorthand_pairs(&mut plan, &[spec.as_str()], root),
            [
                "T2: `files` gains `plugin/PluginEditor.h`: the source names it with a file of \
                 the task in `PluginEditor.h/.cpp`"
            ]
        );
        assert!(
            plan.tasks[1]
                .files
                .contains(&"plugin/PluginEditor.h".to_string())
        );
        assert!(
            repair_shorthand_pairs(&mut plan, &[spec.as_str()], root).is_empty(),
            "once"
        );

        plan.tasks[0]
            .files
            .retain(|file| file != "core/include/undertow/engine.h");
        assert_eq!(
            repair_shorthand_pairs(&mut plan, &[spec.as_str()], root),
            [
                "T1: `files` gains `core/include/undertow/engine.h`: the source names it with a \
                 file of the task in `engine.h/.cpp`"
            ]
        );
    }

    /// gap-708931: round v8's first phase-1 spec said its files were deleted
    /// only in words ("and the retired files removed", "(delete them)"), and
    /// its plan as written listed none of them: `plan validate` warns
    /// (PLAN_063), quoting what it said, and PLAN_062 names the header its
    /// shorthand pairs with T2's editor. The fixed spec, which names every
    /// deletion by path, gets no PLAN_063.
    #[test]
    fn a_spec_that_deletes_files_in_words_is_warned_about() {
        let temp = undertow();
        let root = temp.path();
        let plan_dir = root.join("plans/p1-the-foundation");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::copy(vague_fixture("tasks.toml"), plan_dir.join("tasks.toml")).expect("the plan");
        let found = |rule: &str| {
            crate::plan_validate::validate_plans_dir_with_workdir(&plan_dir, None, Some(root))
                .expect("validate")
                .plans
                .iter()
                .flat_map(|plan| &plan.diagnostics)
                .filter(|diagnostic| diagnostic.rule_id == rule)
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
        };
        std::fs::copy(
            vague_fixture("request.md"),
            root.join("docs/specs/phase-1-foundation.md"),
        )
        .expect("the first spec");
        let warned = found("PLAN_063");
        assert_eq!(warned.len(), 1, "{warned:?}");
        for said in [
            "and the retired files removed",
            "and the retired window files removed",
            "The retired core files** (delete them)",
        ] {
            assert!(warned[0].contains(said), "{said}: {warned:?}");
        }
        assert!(warned[0].contains("more): write each one"), "{warned:?}");
        let changed = found("PLAN_062");
        assert_eq!(changed.len(), 1, "{changed:?}");
        assert!(
            changed[0].contains("`plugin/PluginEditor.h`"),
            "{changed:?}"
        );

        std::fs::copy(
            fixture("request.md"),
            root.join("docs/specs/phase-1-foundation.md"),
        )
        .expect("the fixed spec");
        assert_eq!(found("PLAN_063"), Vec::<String>::new());
    }

    /// bug-aa93e9: `plan validate` warns once about phase 1's plan as
    /// written, which lists none of the files its spec deletes and two it
    /// edits (PLAN_062), and not once its tasks list them; and T2, an
    /// integrative task of 9 edits and 9 deletions, is within its tier's 10
    /// files (PLAN_TIER_SIZE).
    #[test]
    fn plan_validate_warns_of_a_file_the_sources_change_that_no_task_lists() {
        let temp = undertow();
        let root = temp.path();
        std::fs::copy(
            fixture("request.md"),
            root.join("docs/specs/phase-1-foundation.md"),
        )
        .expect("the spec");
        let plan_dir = root.join("plans/p1-the-foundation");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        let mut plan = TasksFile::parse(&fixture("tasks.toml")).expect("the plan as written");
        let write = |plan: &TasksFile| {
            std::fs::write(
                plan_dir.join("tasks.toml"),
                toml::to_string_pretty(plan).expect("serialize"),
            )
            .expect("tasks.toml");
        };
        let found = |rule: &str| {
            crate::plan_validate::validate_plans_dir_with_workdir(&plan_dir, None, Some(root))
                .expect("validate")
                .plans
                .iter()
                .flat_map(|plan| &plan.diagnostics)
                .filter(|diagnostic| diagnostic.rule_id == rule)
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
        };
        write(&plan);
        let warned = found("PLAN_062");
        assert_eq!(warned.len(), 1, "{warned:?}");
        for path in CORE_DELETED
            .iter()
            .chain(&UI_DELETED)
            .chain(&["plugin/PluginEditor.h", "plugin/ui/Theme.h"])
        {
            assert!(
                warned[0].contains(&format!("`{path}`")),
                "{path}: {warned:?}"
            );
        }
        assert!(
            warned[0].starts_with(
                "the documents this plan is written from ([meta] sources) say to delete \
                 `core/include/undertow/envelope.h`, "
            ),
            "{warned:?}"
        );

        let named = named_files_of_plan(&plan, root);
        assert_eq!(named.deleted.len(), 21);
        repair_named_files(&mut plan, &named);
        write(&plan);
        assert_eq!(found("PLAN_062"), Vec::<String>::new());
        assert_eq!(found("PLAN_TIER_SIZE"), Vec::<String>::new());
    }

    /// The sentences that name files for change, and the ones that do not: a
    /// file a sentence keeps, a history, a note, what a colon after a span
    /// explains, a context line and a code block name none. A line removed
    /// from a file changes the file, and a deletion stays a deletion when
    /// another sentence edits the file.
    #[test]
    fn only_a_sentence_of_change_names_its_files() {
        let temp = undertow();
        let root = temp.path();
        let source = "\
# Spec

## Context

- The spec tests: `tests/spec/p1/params_test.cpp`. Change the code, never the tests.
- **What stays, and works as it is:** `core/CMakeLists.txt` and `core/src/rumble.cpp`.
- Don't touch `core/include/undertow/rumble.h`; it works as it is.
- bug-123 changed `tools/check.sh` last week.
- Read `docs/PRD.md` first.

## What to build

1. **The plugin** (`plugin/CMakeLists.txt`, `plugin/PluginProcessor.h/.cpp`)
   - `plugin/ui/`: cut `plugin/ui/Theme.h` down to its tokens, and keep `plugin/ui/Texts.h` as it is.
   - Delete these two: `plugin/ui/Look.h`,
     `plugin/ui/Look.cpp`.
   - **Edits:** `plugin/ui/Look.h` and `tools/install.sh` (no new files).
   - Update `tools/render-demos.sh`. Then run `tools/spec.sh` once.
   - Files: `core/src/params.cpp`, `core/src/missing.cpp`, `/etc/hosts`, `../outside.cpp`.

```sh
rm core/src/voice.cpp  # Delete `core/src/voice.cpp`.
```

Remove `core/src/lfo.cpp`, but never `core/src/engine.cpp`.

- Remove the duplicate line from `tools/check.sh`.
- Delete the files in `plugin/ui/`: `plugin/ui/Status.h`.
- Update 2026-09-28, root cause on main: `tools/install.sh` lists the old bundle.
- Update: the fix landed in `core/src/voice.cpp`.
- Add the test `daemon_test`: a workspace whose `core/src/filter.cpp` binds.
";
        let named = named_files(&[source], root);
        assert_eq!(
            named.deleted,
            strings(&[
                "core/src/lfo.cpp",
                "plugin/ui/Look.cpp",
                "plugin/ui/Look.h",
                "plugin/ui/Status.h",
            ])
        );
        assert_eq!(
            named.changed,
            strings(&[
                "core/src/params.cpp",
                "plugin/ui/Theme.h",
                "tools/check.sh",
                "tools/install.sh",
                "tools/render-demos.sh",
            ])
        );
    }

    /// A file no writing task lists goes to the task that lists most of its
    /// neighbours, then to the one sharing the deepest directory; a task
    /// that lists no files may change any already; and the sizes of
    /// [`sized`] count no deletion.
    #[test]
    fn a_named_file_goes_to_the_task_beside_it() {
        let plan = |tasks: &[(&str, &str, &[&str])]| {
            let mut toml = format!("[meta]\nplan = \"p\"\ntotal = {}\n\n", tasks.len());
            for (id, role, files) in tasks {
                let files: Vec<String> = files.iter().map(|file| format!("{file:?}")).collect();
                toml.push_str(&format!(
                    "[[task]]\nid = \"{id}\"\ntitle = \"{id}\"\nrole = \"{role}\"\n\
                     tier = \"focused\"\nfiles = [{}]\n\n",
                    files.join(", ")
                ));
            }
            TasksFile::parse_str(&toml).expect("a plan")
        };
        let named = |deleted: &[&str], changed: &[&str], sentence: &[&str]| {
            let mut named = NamedFiles {
                deleted: strings(deleted),
                changed: strings(changed),
                ..NamedFiles::default()
            };
            for path in deleted.iter().chain(changed) {
                named.neighbours.insert(
                    (*path).to_string(),
                    sentence
                        .iter()
                        .filter(|other| *other != path)
                        .map(|other| (*other).to_string())
                        .collect(),
                );
            }
            named
        };

        // Neighbours first: `src/a/x.rs` is named beside T2's file.
        let mut by_neighbour = plan(&[
            ("T1", "implementer", &["src/a/y.rs"]),
            ("T2", "implementer", &["src/b/z.rs"]),
        ]);
        let found = named(&[], &["src/a/x.rs"], &["src/a/x.rs", "src/b/z.rs"]);
        assert_eq!(
            repair_named_files(&mut by_neighbour, &found),
            ["T2: `files` gains `src/a/x.rs`: the source says to change it"]
        );

        // Then the deepest shared directory, and a file of its stem beside.
        let mut by_dir = plan(&[
            ("T1", "implementer", &["src/a.rs"]),
            ("T2", "implementer", &["src/b/c.rs"]),
            ("T3", "implementer", &["src/b/old.cpp"]),
            ("T4", "scribe", &["src/b/old.md"]),
        ]);
        let found = named(&["src/b/old.h"], &[], &["src/b/old.h"]);
        repair_named_files(&mut by_dir, &found);
        assert_eq!(by_dir.tasks[2].files, ["src/b/old.cpp", "src/b/old.h"]);
        // A tie goes to the first; a read-only task owns nothing.
        let mut tie = plan(&[
            ("T1", "researcher", &["lib/x.rs"]),
            ("T2", "implementer", &["src/a.rs"]),
            ("T3", "implementer", &["src/b.rs"]),
        ]);
        let found = named(&["lib/x.rs", "src/c.rs"], &[], &[]);
        repair_named_files(&mut tie, &found);
        assert_eq!(tie.tasks[1].files, ["src/a.rs", "lib/x.rs", "src/c.rs"]);
        assert_eq!(tie.tasks[2].files, ["src/b.rs"]);
        // A writing task with no files may change any file already.
        let mut free = plan(&[
            ("T1", "implementer", &[]),
            ("T2", "implementer", &["src/a.rs"]),
        ]);
        assert!(repair_named_files(&mut free, &found).is_empty());

        // Sizing counts no deletion and keeps what it set.
        let mut plan = plan(&[("T1", "implementer", &["a.rs", "b.rs", "c.rs", "d.rs"])]);
        let deleted = strings(&["c.rs", "d.rs"]);
        let counted = sized(&mut plan, &deleted, |sized| {
            sized.tasks[0].tier = "mechanical".to_string();
            sized.tasks[0].files.len()
        });
        assert_eq!(counted, 2);
        assert_eq!(plan.tasks[0].tier, "mechanical");
        assert_eq!(plan.tasks[0].files.len(), 4);
    }
}
