//! The documents a plan is written from, read by every one of its tasks
//! (bug-60ad88). Round v7's phase-3 plan came from
//! `docs/specs/phase-3-sequencer.md` through `spec_path`, yet its tasks read
//! only the tests and the PRD: four of six tries never opened the spec, which
//! states the step rule they got wrong. The planner chooses each task's
//! context, so generation adds the source documents itself.

use std::path::Path;

use crate::task_parser::{ReadFile, TaskContext, TasksFile};

/// Why a task reads a source document.
const WHY: &str = "the document this plan is written from: read it first, it states the rules, the \
                   interfaces and the checks this task must meet";

/// The workspace path of `path`, the file a plan's source text came from,
/// when it is a file inside `workdir`.
pub(super) fn workspace_document(path: &Path, workdir: &Path) -> Option<String> {
    let file = path.canonicalize().ok()?;
    let root = workdir.canonicalize().ok()?;
    let relative = file.strip_prefix(&root).ok()?;
    file.is_file()
        .then(|| relative.to_string_lossy().replace('\\', "/"))
}

/// Make each task of `plan` read each of `documents` (workspace paths of the
/// files the plan is written from) whole, and record them as the plan's
/// `[meta] sources`, which `plan validate` holds the tasks to (PLAN_061). A
/// task that reads one whole already, with no range or one that runs the
/// whole file, keeps it as the draft has it (phase 3's re-plan put the spec
/// first itself). A read of only part of a document widens to the whole of
/// it; the declared
/// context cuts a long file to fit, never fails the task over it. The
/// documents count toward no generated task's read budget
/// (`PLAN_BUDGET_READ_FILES`), but a task that already reads `max_reads`
/// files, the most the lane that runs it allows, gets no more. Returns one
/// line per change.
pub(super) fn repair_source_reads(
    plan: &mut TasksFile,
    workdir: &Path,
    documents: &[String],
    max_reads: usize,
) -> Vec<String> {
    let documents: Vec<&String> = documents
        .iter()
        .filter(|document| workdir.join(document).is_file())
        .collect();
    let mut lines = Vec::new();
    if documents.is_empty() {
        return lines;
    }
    // A read with no range, or one that runs the whole file, reads it whole.
    let line_counts: Vec<usize> = documents
        .iter()
        .map(|document| {
            std::fs::read_to_string(workdir.join(document)).map_or(0, |text| text.lines().count())
        })
        .collect();
    let whole = |read: &ReadFile, line_count: usize| {
        read.lines.as_deref().is_none_or(|range| {
            crate::plan_policy::parse_line_range(range, line_count)
                .is_ok_and(|(start, end)| start <= 1 && end.is_none_or(|end| end >= line_count))
        })
    };
    for task in &mut plan.tasks {
        let context = task.context.get_or_insert_with(TaskContext::default);
        for (document, line_count) in documents.iter().zip(&line_counts) {
            let reads: Vec<usize> = context
                .read_files
                .iter()
                .enumerate()
                .filter(|(_, read)| read.path == **document)
                .map(|(index, _)| index)
                .collect();
            if reads
                .iter()
                .any(|index| whole(&context.read_files[*index], *line_count))
            {
                continue;
            }
            if let Some((first, rest)) = reads.split_first() {
                let was = context.read_files[*first].lines.take().unwrap_or_default();
                for index in rest.iter().rev() {
                    context.read_files.remove(*index);
                }
                lines.push(format!(
                    "{}: reads all of `{document}`, the document its plan is written from, not \
                     only lines {was}",
                    task.id
                ));
            } else if context.read_files.len() < max_reads {
                context.read_files.insert(
                    0,
                    ReadFile {
                        path: (*document).clone(),
                        lines: None,
                        why: WHY.to_string(),
                    },
                );
                lines.push(format!(
                    "{}: reads `{document}`, the document its plan is written from",
                    task.id
                ));
            } else {
                lines.push(format!(
                    "{}: does not read `{document}`, the document its plan is written from: it \
                     reads {} files already, the most a task may",
                    task.id,
                    context.read_files.len()
                ));
            }
        }
    }
    let sources: Vec<String> = documents.into_iter().cloned().collect();
    if plan.meta.sources != sources {
        lines.push(format!(
            "`[meta] sources` names `{}`, the documents the plan is written from",
            sources.join("`, `")
        ));
        plan.meta.sources = sources;
    }
    lines
}

/// The documents of `plan`'s `[meta] sources` that `plan.tasks[index]` does
/// not read (bug-60ad88, PLAN_061): its agent may never see what they state.
#[must_use]
pub fn unread_sources(plan: &TasksFile, index: usize) -> Vec<String> {
    let task = &plan.tasks[index];
    plan.meta
        .sources
        .iter()
        .filter(|source| {
            !task
                .context
                .iter()
                .flat_map(|context| &context.read_files)
                .any(|read| read.path == **source)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 3's shape: T1 reads the tests and the PRD, T2 a part of the spec.
    fn phase_3() -> TasksFile {
        TasksFile::parse_str(
            "[meta]\nplan = \"p3\"\ntotal = 2\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"The sequencer\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"architectural\"\nfiles = [\"core/src/sequencer.cpp\"]\n\n\
             [[task.context.read_files]]\npath = \"tests/spec/p3/sequencer_test.cpp\"\n\
             why = \"the tests\"\n\n\
             [[task.context.read_files]]\npath = \"docs/PRD.md\"\nlines = \"60-170\"\n\
             why = \"the product\"\n\n\
             [[task]]\nid = \"T2\"\ntitle = \"The engine\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"integrative\"\nfiles = [\"core/src/engine.cpp\"]\n\n\
             [[task.context.read_files]]\npath = \"docs/specs/phase-3-sequencer.md\"\n\
             lines = \"4-8\"\nwhy = \"the engine wiring\"\n\n\
             [[task]]\nid = \"T3\"\ntitle = \"The panel\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"integrative\"\nfiles = [\"plugin/Panel.cpp\"]\n\n\
             [[task.context.read_files]]\npath = \"docs/specs/phase-3-sequencer.md\"\n\
             lines = \"1-10\"\nwhy = \"the whole spec\"\n",
        )
        .expect("a plan")
    }

    /// bug-60ad88: round v7's phase-3 plan came from its spec, yet T1 read
    /// only the tests and the PRD. Each task now reads the spec whole: T1 gets
    /// it first, T2's part of it widens, and the plan records it as a source,
    /// which `plan validate` holds every task to. A task at the most reads a
    /// task may gets none, and says so; a document the workspace lacks is no
    /// source.
    #[test]
    fn every_task_of_a_spec_sourced_plan_reads_the_spec() {
        let temp = tempfile::tempdir().expect("tempdir");
        let spec = "docs/specs/phase-3-sequencer.md";
        std::fs::create_dir_all(temp.path().join("docs/specs")).expect("docs");
        let ten_lines: String = (1..=10).map(|n| format!("line {n}\n")).collect();
        std::fs::write(temp.path().join(spec), ten_lines).expect("the spec");
        assert_eq!(
            workspace_document(&temp.path().join(spec), temp.path()).as_deref(),
            Some(spec)
        );
        assert_eq!(
            workspace_document(&temp.path().join("docs/specs/missing.md"), temp.path()),
            None
        );

        let mut plan = phase_3();
        assert_eq!(unread_sources(&plan, 0), Vec::<String>::new());
        let documents = [spec.to_string(), "docs/specs/missing.md".to_string()];
        let lines = repair_source_reads(&mut plan, temp.path(), &documents, 16);
        assert_eq!(
            lines,
            [
                "T1: reads `docs/specs/phase-3-sequencer.md`, the document its plan is written \
                 from",
                "T2: reads all of `docs/specs/phase-3-sequencer.md`, the document its plan is \
                 written from, not only lines 4-8",
                "`[meta] sources` names `docs/specs/phase-3-sequencer.md`, the documents the plan \
                 is written from",
            ]
        );
        for (index, task) in plan.tasks.iter().enumerate() {
            let reads = &task.context.as_ref().expect("a context").read_files;
            let read = reads
                .iter()
                .find(|read| read.path == spec)
                .expect("the spec is read");
            // T3 read it whole already, by its range, and keeps it so.
            let whole = if task.id == "T3" { Some("1-10") } else { None };
            assert_eq!(read.lines.as_deref(), whole, "{}", task.id);
            assert_eq!(unread_sources(&plan, index), Vec::<String>::new());
        }
        assert_eq!(
            plan.tasks[0]
                .context
                .as_ref()
                .expect("a context")
                .read_files[0]
                .path,
            spec
        );
        assert_eq!(plan.meta.sources, [spec]);
        // Again, nothing changes.
        assert!(repair_source_reads(&mut plan, temp.path(), &documents, 16).is_empty());

        // The spec is no part of a generated task's read budget: T1 reads
        // three files of its own beside it.
        plan.tasks[0]
            .context
            .as_mut()
            .expect("a context")
            .read_files
            .push(ReadFile {
                path: "core/include/undertow/engine.h".to_string(),
                lines: None,
                why: "the engine".to_string(),
            });
        let over_budget = |plan: &TasksFile| {
            crate::plan_policy::validate_plan_budgets(
                plan,
                crate::plan_policy::PlanExecutionPolicy::generated(8),
            )
            .iter()
            .filter(|issue| issue.code == "PLAN_BUDGET_READ_FILES")
            .count()
        };
        assert_eq!(over_budget(&plan), 0);
        let mut unsourced = plan.clone();
        unsourced.meta.sources.clear();
        assert_eq!(over_budget(&unsourced), 1);

        // A task that may read no more keeps its reads, and the lint sees it.
        let mut full = phase_3();
        let lines = repair_source_reads(&mut full, temp.path(), &documents, 2);
        assert_eq!(
            lines[0],
            "T1: does not read `docs/specs/phase-3-sequencer.md`, the document its plan is \
             written from: it reads 2 files already, the most a task may"
        );
        assert_eq!(unread_sources(&full, 0), [spec]);
    }

    /// bug-60ad88: `plan validate` warns about a task that does not read a
    /// document its plan is written from (PLAN_061), and only then.
    #[test]
    fn plan_validate_warns_about_a_task_that_never_reads_its_spec() {
        let temp = tempfile::tempdir().expect("tempdir");
        let spec = "docs/specs/phase-3-sequencer.md";
        std::fs::create_dir_all(temp.path().join("docs/specs")).expect("docs");
        std::fs::write(temp.path().join(spec), "# Phase 3\n").expect("the spec");
        let plan_dir = temp.path().join("plans/p3");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        let mut plan = phase_3();
        plan.meta.sources = vec![spec.to_string()];
        std::fs::write(
            plan_dir.join("tasks.toml"),
            toml::to_string_pretty(&plan).expect("serialize"),
        )
        .expect("tasks.toml");
        let found = || {
            crate::plan_validate::validate_plans_dir_with_workdir(
                &plan_dir,
                None,
                Some(temp.path()),
            )
            .expect("validate")
            .plans
            .iter()
            .flat_map(|plan| &plan.diagnostics)
            .filter(|diagnostic| diagnostic.rule_id == "PLAN_061")
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
        };
        assert_eq!(
            found(),
            [
                "task 'T1' does not read `docs/specs/phase-3-sequencer.md`, which its plan is \
                 written from ([meta] sources): its agent may never see what it states; add it \
                 to the task's read_files"
            ]
        );
        repair_source_reads(&mut plan, temp.path(), &[spec.to_string()], 16);
        std::fs::write(
            plan_dir.join("tasks.toml"),
            toml::to_string_pretty(&plan).expect("serialize"),
        )
        .expect("tasks.toml");
        assert_eq!(found(), Vec::<String>::new());
    }
}
