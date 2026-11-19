//! The tests a task's checks run and the headers of the sources it changes,
//! read by the task (gap-5db3d9). Round v7's plans declared 1 of the 73 test
//! files their checks ran in phases 4.3 to 5.2, and none of the headers of
//! the sources they read, though each spec names its tests ("The spec test:
//! `tests/spec/p5_2/presets_test.cpp`"). Phase 5.2's only try read its spec
//! tests and `presets.h` itself before its first edit, in call 7. Generation
//! declares them now. Neither counts toward the task's read budget
//! ([`crate::plan_policy::counts_toward_read_budget`]), and both come after
//! the task's other reads, so the room the prompt gives declared files cuts
//! them first, with a line naming what it leaves out.

use std::cell::OnceCell;
use std::path::Path;

use super::file_paths::workspace_files;
use super::sizes::code_span_files;
use crate::plan_policy::{
    C_HEADERS, PlanExecutionPolicy, header_beside, test_affix_stem, test_run_by,
};
use crate::task_parser::{ReadFile, TaskContext, TasksFile, VerifyExpect};

/// Give each task of `plan` a read of:
/// - the header of each C or C++ source it changes and reads
///   ([`header_beside`]), when the workspace has one: whole, or its first
///   `policy.max_range_lines` lines;
/// - the test each of its checks runs ([`test_run_by`]), when exactly one of
///   the test files `sources` name in code spans, by path or by file name, is
///   that check's. A check marked to pass on the base gets none: a
///   regression check's test is one the task keeps passing, not one it works
///   to.
///
/// Two headers or two tests that fit alike give no read, never a guess, and a
/// line names them. A file the task reads already is left as it is, and a task
/// that reads `max_reads` files, the most the lane that runs it allows, gets no
/// more. Returns one line per read added or left out.
pub(super) fn repair_check_reads(
    plan: &mut TasksFile,
    workdir: &Path,
    sources: &[&str],
    policy: PlanExecutionPolicy,
    max_reads: usize,
) -> Vec<String> {
    let by_name = OnceCell::new();
    let named = |name: &str| {
        by_name
            .get_or_init(|| workspace_files(workdir))
            .get(name)
            .cloned()
            .unwrap_or_default()
    };
    let mut tests: Vec<String> = Vec::new();
    for file in sources.iter().flat_map(|source| code_span_files(source)) {
        let file = file.trim_start_matches("./");
        if test_affix_stem(file).is_none() {
            continue;
        }
        let found = if file.contains('/') {
            if workdir.join(file).is_file() {
                vec![file.to_string()]
            } else {
                Vec::new()
            }
        } else {
            named(file)
        };
        for path in found {
            if !tests.contains(&path) {
                tests.push(path);
            }
        }
    }
    tests.sort();
    let mut lines = Vec::new();
    for task in &mut plan.tasks {
        let reads: Vec<String> = task
            .context
            .iter()
            .flat_map(|context| &context.read_files)
            .map(|read| read.path.clone())
            .collect();
        let mut added: Vec<(ReadFile, String)> = Vec::new();
        let new = |path: &str, added: &[(ReadFile, String)]| {
            !reads.iter().any(|read| read == path)
                && !added.iter().any(|(read, _)| read.path == path)
        };
        for source in task.files.iter().filter(|file| reads.contains(*file)) {
            let Some(stem) = Path::new(source).file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let mut headers: Vec<String> = C_HEADERS
                .iter()
                .flat_map(|extension| named(&format!("{stem}.{extension}")))
                .filter(|header| header_beside(header, std::slice::from_ref(source)))
                .collect();
            headers.sort();
            let [header] = headers.as_slice() else {
                if headers.len() > 1 {
                    lines.push(format!(
                        "{}: reads no header of `{source}`: {} fit it alike",
                        task.id,
                        listed(&headers)
                    ));
                }
                continue;
            };
            if !new(header, &added) {
                continue;
            }
            let length = std::fs::read_to_string(workdir.join(header))
                .map_or(0, |text| text.lines().count());
            let range =
                (length > policy.max_range_lines).then(|| format!("1-{}", policy.max_range_lines));
            let part = range
                .as_deref()
                .map(|range| format!("lines {range} of "))
                .unwrap_or_default();
            added.push((
                ReadFile {
                    path: header.clone(),
                    lines: range,
                    why: format!(
                        "the header of `{source}`, which this task changes: the declarations its \
                         code must match"
                    ),
                },
                format!(
                    "{}: reads {part}`{header}`, the header of `{source}`, which it changes",
                    task.id
                ),
            ));
        }
        for step in &task.verify {
            if step.expect == Some(VerifyExpect::PassOnBase) {
                continue;
            }
            let runs: Vec<String> = tests
                .iter()
                .filter(|test| test_run_by(&step.command, test))
                .cloned()
                .collect();
            let [test] = runs.as_slice() else {
                if runs.len() > 1 {
                    lines.push(format!(
                        "{}: reads no test for its check `{}`: {} match it alike",
                        task.id,
                        step.command,
                        listed(&runs)
                    ));
                }
                continue;
            };
            if !new(test, &added) {
                continue;
            }
            added.push((
                ReadFile {
                    path: test.clone(),
                    lines: None,
                    why: format!(
                        "the test its check `{}` runs: what this task's work must pass",
                        step.command
                    ),
                },
                format!(
                    "{}: reads `{test}`, the test its check `{}` runs",
                    task.id, step.command
                ),
            ));
        }
        if added.is_empty() {
            continue;
        }
        let context = task.context.get_or_insert_with(TaskContext::default);
        for (read, line) in added {
            if context.read_files.len() < max_reads {
                context.read_files.push(read);
                lines.push(line);
            } else {
                lines.push(format!(
                    "{}: does not read `{}`: it reads {} files already, the most a task may",
                    task.id,
                    read.path,
                    context.read_files.len()
                ));
            }
        }
    }
    lines
}

/// `paths` quoted and listed: `a`, `b` and `c`.
fn listed(paths: &[String]) -> String {
    let quoted: Vec<String> = paths.iter().map(|path| format!("`{path}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_policy::validate_plan_budgets;

    /// A workspace of `files`, each `(path, lines)`.
    fn workspace(files: &[(&str, usize)]) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        for (path, lines) in files {
            let path = temp.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, "// code\n".repeat(*lines)).expect("a file");
        }
        temp
    }

    /// The paths `plan.tasks[0]` reads, in order.
    fn reads(plan: &TasksFile) -> Vec<&str> {
        plan.tasks[0]
            .context
            .iter()
            .flat_map(|context| &context.read_files)
            .map(|read| read.path.as_str())
            .collect()
    }

    /// gap-5db3d9: phase 5.2's plan (Undertow ac16976) had T1 read its spec
    /// and `core/src/presets.cpp`, and its one try read the spec tests and
    /// `presets.h` itself before its first edit, in call 7. T1 now reads
    /// `core/include/undertow/presets.h`, the header of the `presets.cpp` it
    /// changes, and `tests/spec/p5_2/presets_test.cpp`, the test its new check
    /// `tools/spec.sh 5_2 presets` runs. The tests of its regression checks
    /// (`tools/spec.sh 5 presets` and the rest, which pass on the base) stay
    /// unread, though a source names them too, `render_test.cpp` and
    /// `demos_test.cpp` by file name alone. Neither read counts toward T1's
    /// read budget, and a second pass adds nothing.
    #[test]
    fn a_task_reads_the_tests_its_new_checks_run() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p5_2_criteria");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let kept = "The checks it keeps passing: `tests/spec/p5/presets_test.cpp`, \
                    `render_test.cpp` and `demos_test.cpp`.";
        let mut plan = TasksFile::parse(&fixture.join("draft.toml")).expect("the draft");
        plan.meta.sources = vec!["docs/specs/phase-5.2-presets.md".to_string()];
        let temp = workspace(&[
            ("docs/specs/phase-5.2-presets.md", 139),
            ("core/src/presets.cpp", 251),
            ("core/include/undertow/presets.h", 16),
            ("core/include/undertow/params.h", 41),
            ("tests/spec/p5_2/presets_test.cpp", 127),
            ("tests/spec/p5/presets_test.cpp", 161),
            ("tests/spec/p5/render_test.cpp", 120),
            ("tests/spec/p5/demos_test.cpp", 90),
            ("tests/spec/p5_1/render_test.cpp", 149),
            ("tests/spec/p5_1/demos_test.cpp", 80),
        ]);
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_check_reads(&mut plan, temp.path(), &[&spec, kept], policy, 16);
        assert_eq!(
            lines,
            [
                "T1: reads `core/include/undertow/presets.h`, the header of \
                 `core/src/presets.cpp`, which it changes",
                "T1: reads `tests/spec/p5_2/presets_test.cpp`, the test its check \
                 `tools/spec.sh 5_2 presets` runs",
            ]
        );
        assert_eq!(
            reads(&plan),
            [
                "docs/specs/phase-5.2-presets.md",
                "core/src/presets.cpp",
                "core/include/undertow/presets.h",
                "tests/spec/p5_2/presets_test.cpp",
            ]
        );
        let header = &plan.tasks[0]
            .context
            .as_ref()
            .expect("a context")
            .read_files[2];
        assert_eq!(header.lines, None, "a 16-line header is read whole");
        assert!(
            repair_check_reads(&mut plan, temp.path(), &[&spec, kept], policy, 16).is_empty(),
            "a second pass adds nothing"
        );
        // T1 may read one file besides its spec, and reads presets.cpp.
        let tight = PlanExecutionPolicy {
            max_read_files_per_task: 1,
            reads_cover_owned_files: false,
            ..policy
        };
        assert!(
            validate_plan_budgets(&plan, tight)
                .iter()
                .all(|issue| issue.code != "PLAN_BUDGET_READ_FILES"),
            "{:?}",
            validate_plan_budgets(&plan, tight)
        );
    }

    /// A header longer than the policy's range limit is read from its top to
    /// the limit, and a task at its lane's read limit gets no more reads: the
    /// line says which it does without.
    #[test]
    fn a_long_header_is_read_in_part_and_a_full_task_reads_no_more() {
        let toml = r#"
[meta]
plan = "widget"
total = 1
status = "ready"

[[task]]
id = "T1"
title = "The widget"
status = "ready"
role = "implementer"
tier = "focused"
files = ["src/widget.cpp"]

[[task.context.read_files]]
path = "docs/spec.md"

[[task.context.read_files]]
path = "src/widget.cpp"

[[task.verify]]
command = "tools/spec.sh 2 widget"
expect = "fail_on_base"
"#;
        let temp = workspace(&[
            ("docs/spec.md", 20),
            ("src/widget.cpp", 50),
            ("include/widget.h", 300),
            ("tests/spec/p2/widget_test.cpp", 40),
        ]);
        let spec = "The spec test: `tests/spec/p2/widget_test.cpp`.";
        let policy = PlanExecutionPolicy::generated(8);
        let mut plan = TasksFile::parse_str(toml).expect("the plan");
        let lines = repair_check_reads(&mut plan, temp.path(), &[spec], policy, 3);
        assert_eq!(
            lines,
            [
                "T1: reads lines 1-240 of `include/widget.h`, the header of `src/widget.cpp`, \
                 which it changes",
                "T1: does not read `tests/spec/p2/widget_test.cpp`: it reads 3 files already, \
                 the most a task may",
            ]
        );
        let header = &plan.tasks[0]
            .context
            .as_ref()
            .expect("a context")
            .read_files[2];
        assert_eq!(header.lines.as_deref(), Some("1-240"));
    }

    /// A header the task changes as well as its source is read too: it
    /// changes both, and the try reads the header before its first edit.
    #[test]
    fn a_header_the_task_changes_is_read_too() {
        let temp = workspace(&[("src/widget.cpp", 40), ("src/widget.h", 12)]);
        let mut plan = one_task("src/widget.cpp", &["src/widget.cpp"], "tools/check.sh");
        plan.tasks[0].files.push("src/widget.h".to_string());
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_check_reads(&mut plan, temp.path(), &[], policy, 16);
        assert_eq!(
            lines,
            ["T1: reads `src/widget.h`, the header of `src/widget.cpp`, which it changes"]
        );
        assert_eq!(reads(&plan), ["src/widget.cpp", "src/widget.h"]);
    }

    /// One task that changes `file`, reads `reads` and runs `check`, a new
    /// check.
    fn one_task(file: &str, reads: &[&str], check: &str) -> TasksFile {
        let reads: String = reads
            .iter()
            .map(|path| format!("[[task.context.read_files]]\npath = \"{path}\"\n\n"))
            .collect();
        TasksFile::parse_str(&format!(
            "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"The work\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"focused\"\nfiles = [\"{file}\"]\n\n{reads}\
             [[task.verify]]\ncommand = \"{check}\"\nexpect = \"fail_on_base\"\n"
        ))
        .expect("a plan")
    }

    /// Nothing is read on a guess (the lead's negative cases): a check whose
    /// command names none of the tests the sources name adds no read.
    #[test]
    fn a_check_that_names_no_test_adds_no_read() {
        let temp = workspace(&[("src/widget.cpp", 40), ("tests/widget_test.cpp", 30)]);
        let spec = "The test: `tests/widget_test.cpp`.";
        let check = "cmake --build build && ctest --test-dir build";
        let mut plan = one_task("src/widget.cpp", &[], check);
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_check_reads(&mut plan, temp.path(), &[spec], policy, 16);
        assert!(lines.is_empty(), "{lines:?}");
        assert!(reads(&plan).is_empty());
    }

    /// A source that is not C or C++ gets no header read, even with a C
    /// header of its name in the workspace's include tree (a generated
    /// binding, say); its check's test is read as for any language.
    #[test]
    fn a_workspace_without_c_or_cpp_sources_gets_no_header_read() {
        let temp = workspace(&[
            ("src/widget.rs", 40),
            ("include/widget.h", 10),
            ("tests/widget_tests.rs", 30),
        ]);
        let spec = "The test: `tests/widget_tests.rs`.";
        let check = "cargo test --test widget_tests";
        let mut plan = one_task("src/widget.rs", &["src/widget.rs"], check);
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_check_reads(&mut plan, temp.path(), &[spec], policy, 16);
        assert_eq!(
            lines,
            [
                "T1: reads `tests/widget_tests.rs`, the test its check `cargo test --test \
              widget_tests` runs"
            ]
        );
        assert_eq!(reads(&plan), ["src/widget.rs", "tests/widget_tests.rs"]);
    }

    /// Two tests that match a check alike give no read, and a line names
    /// both: `ctest -R render_test` runs either `render_test.cpp`. Two
    /// headers that fit a source alike give none either.
    #[test]
    fn two_files_that_fit_alike_give_no_read() {
        let temp = workspace(&[
            ("src/render.cpp", 40),
            ("src/render.h", 10),
            ("include/render.h", 10),
            ("tests/spec/p5/render_test.cpp", 30),
            ("tests/spec/p5_1/render_test.cpp", 30),
        ]);
        let spec = "The spec tests: `render_test.cpp`.";
        let check = "ctest --test-dir build -R render_test";
        let mut plan = one_task("src/render.cpp", &["src/render.cpp"], check);
        let policy = PlanExecutionPolicy::generated(8);
        let lines = repair_check_reads(&mut plan, temp.path(), &[spec], policy, 16);
        assert_eq!(
            lines,
            [
                "T1: reads no header of `src/render.cpp`: `include/render.h` and `src/render.h` \
                 fit it alike",
                "T1: reads no test for its check `ctest --test-dir build -R render_test`: \
                 `tests/spec/p5/render_test.cpp` and `tests/spec/p5_1/render_test.cpp` match it \
                 alike",
            ]
        );
        assert_eq!(reads(&plan), ["src/render.cpp"]);
    }
}
