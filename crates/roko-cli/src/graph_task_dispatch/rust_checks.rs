//! The task's Rust checks (find-fc5cdd; Will's decision of 2026-10-28): for
//! the crates an attempt changed, `cargo fmt --check`, clippy with warnings
//! denied and the doc tests, whatever the plan authors, once the attempt's
//! own checks passed and before its verify depth's. A task that leaves its
//! crate unformatted, with a warning or with a doc example that does not
//! compile is refused at its own task, instead of a later task that edits the
//! same file repairing it (and being charged for it).
//!
//! The checks run at every verify depth, in the attempt's working tree, on
//! the compile lock, as V1's clippy does ([`super::verify_depth`]), and fail
//! fast. They run only for a Rust change in a Cargo workspace, not in a FAST
//! run (a FAST task has exactly one authored check), and not when `[gates]
//! clippy_enabled = false`. `cargo fmt --check` counts only the files the
//! attempt changed, so a crate's older formatting fails nothing; the doc tests
//! cover the changed crates that have a library. A check that cannot tell (no
//! `rustfmt`, a crate directory that is not a package name, no library) is
//! recorded as skipped and fails nothing. At V1 and deeper the depth's own
//! clippy does not run again once these passed it.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use roko_learn::telemetry::VerifyStepVerdict;

use super::verify_depth::{Checked, RustScope, rust_scope, tail};
use super::*;

/// What a failed fmt check tells the attempt, after the files.
const FMT_FAILED: &str = "Format the crate as `cargo fmt` does, and keep the result.";

/// What a failed clippy check tells the attempt.
const CLIPPY_FAILED: &str = "Fix every clippy warning in the code the task changed; warnings are \
                             denied, as CI denies them.";

/// What a failed doc-test check tells the attempt.
const DOC_FAILED: &str = "Every doc example must compile and pass as a doc test: qualify the names \
                          it uses with the crate's name (`crate_name::function`), and show what it \
                          returns.";

/// One of the task's Rust checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustCheck {
    /// `cargo fmt --check` over the changed crates.
    Fmt,
    /// Clippy over the changed crates, warnings denied.
    Clippy,
    /// The changed crates' doc tests.
    Doc,
}

impl RustCheck {
    /// Its verdict rung, `rust:<name>`.
    const fn rung(self) -> &'static str {
        match self {
            Self::Fmt => "rust:fmt",
            Self::Clippy => "rust:clippy",
            Self::Doc => "rust:doc",
        }
    }

    /// The verify phase it runs as.
    const fn phase(self) -> &'static str {
        match self {
            Self::Fmt | Self::Clippy => "lint",
            Self::Doc => "test",
        }
    }
}

/// What the task's Rust checks found.
#[derive(Debug, Default)]
pub(super) struct RustChecked {
    /// How many ran.
    pub(super) checks: usize,
    /// The failure of the one that failed, if one did.
    pub(super) failure: Option<String>,
    /// Whether clippy ran and passed, so that V1 does not run it again.
    pub(super) clippy_passed: bool,
}

impl GraphTaskDispatcher {
    /// Whether the task's Rust checks may run in `workdir`: a Cargo
    /// workspace, outside a FAST run, with `[gates] clippy_enabled`.
    fn rust_checks_on(&self, workdir: &Path) -> bool {
        self.fast.is_none()
            && self.config.gates.clippy_enabled
            && workdir.join("Cargo.toml").is_file()
    }

    /// find-fc5cdd: the Rust checks of the attempt `attempt_key` at `task`
    /// in `workdir`, whose own checks passed, over the files it changed; each
    /// check adds its row to `step_verdicts`.
    ///
    /// # Errors
    ///
    /// The plan run began to stop.
    pub(super) async fn rust_task_checks(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        workdir: &Path,
        step_verdicts: &mut Vec<VerifyStepVerdict>,
    ) -> Result<RustChecked> {
        let mut checked = RustChecked::default();
        if !self.rust_checks_on(workdir) {
            return Ok(checked);
        }
        // The tree as its checks left it, which an auto-fix may have changed
        // since the pre-verify screen read it.
        if self
            .attempt_diff(spec, task, attempt_key, workdir)
            .await
            .is_none()
        {
            return Ok(checked);
        }
        let changed = self.changed_files(attempt_key);
        let Some(scope) = rust_scope(workdir, &changed) else {
            return Ok(checked);
        };
        for (check, command) in rust_commands(workdir, &scope) {
            if checked.failure.is_some() {
                let skipped = Checked::Skipped("fail_fast".to_string());
                step_verdicts.push(rust_row(check, &skipped, None));
                continue;
            }
            let Some(command) = command else {
                let skipped = Checked::Skipped("no changed crate has a library".to_string());
                step_verdicts.push(rust_row(check, &skipped, None));
                continue;
            };
            let started = Instant::now();
            let ran = self
                .run_check_command(
                    spec,
                    task,
                    workdir,
                    check.rung(),
                    check.phase(),
                    &command,
                    crate::task_parser::default_verify_timeout(),
                )
                .await?;
            checked.checks += 1;
            let outcome = judged(check, &command, &ran, &changed, workdir);
            step_verdicts.push(rust_row(check, &outcome, Some(started.elapsed())));
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = check.rung(),
                checked = ?outcome,
                "the task's Rust check settled"
            );
            if let (Some(tui), Some(passed)) = (&self.tui_bridge, outcome.passed()) {
                tui.gate_result(&spec.plan_id, &task.id, check.rung(), passed);
            }
            match outcome {
                Checked::Passed if check == RustCheck::Clippy => checked.clippy_passed = true,
                Checked::Failed(message) => {
                    checked.failure = Some(format!(
                        "{} (the task's Rust checks): {message}",
                        check.rung()
                    ));
                }
                _ => {}
            }
        }
        Ok(checked)
    }
}

/// Each check's command for `scope` in `workdir`, in the order they run:
/// the doc tests' is `None` when no changed crate has a library.
fn rust_commands(workdir: &Path, scope: &RustScope) -> [(RustCheck, Option<String>); 3] {
    let (fmt, clippy, doc) = match scope {
        RustScope::Workspace => (
            "--all".to_string(),
            "--workspace".to_string(),
            Some("--workspace".to_string()),
        ),
        RustScope::Packages(packages) => {
            let flags = |names: &mut dyn Iterator<Item = &String>| {
                names
                    .map(|name| format!("-p {name}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let libraries: Vec<&String> = packages
                .iter()
                .filter(|name| {
                    workdir
                        .join("crates")
                        .join(name)
                        .join("src/lib.rs")
                        .is_file()
                })
                .collect();
            let doc = (!libraries.is_empty()).then(|| flags(&mut libraries.into_iter()));
            (
                flags(&mut packages.iter()),
                flags(&mut packages.iter()),
                doc,
            )
        }
    };
    [
        (RustCheck::Fmt, Some(format!("cargo fmt --check {fmt}"))),
        (
            RustCheck::Clippy,
            Some(format!("cargo clippy {clippy} --no-deps -- -D warnings")),
        ),
        (
            RustCheck::Doc,
            doc.map(|scope| format!("cargo test {scope} --doc")),
        ),
    ]
}

/// What `check`'s run `ran` of `command` means for an attempt that changed
/// `changed` in `workdir`.
fn judged(
    check: RustCheck,
    command: &str,
    ran: &super::verify_depth::CheckRun,
    changed: &[String],
    workdir: &Path,
) -> Checked {
    if ran.passed {
        return Checked::Passed;
    }
    let output = &ran.output;
    if output.contains("did not match any packages") {
        return Checked::Skipped(format!(
            "`{command}`: a changed crate's directory is not its package's name"
        ));
    }
    match check {
        RustCheck::Fmt => fmt_judged(command, output, changed, workdir),
        RustCheck::Doc if output.contains("no library targets found") => {
            Checked::Skipped("no changed crate has a library".to_string())
        }
        RustCheck::Clippy | RustCheck::Doc => {
            let guidance = if check == RustCheck::Clippy {
                CLIPPY_FAILED
            } else {
                DOC_FAILED
            };
            Checked::Failed(format!(
                "`{command}`: {}\n{}\n{guidance}",
                ran.reason,
                tail(output)
            ))
        }
    }
}

/// What a failed `cargo fmt --check` with `output` means: a failure when a
/// file the attempt `changed` is not formatted, a pass when only files it did
/// not change are (their formatting is older than the attempt), and no
/// verdict when it named no file (no `rustfmt`, or one that could not parse).
fn fmt_judged(command: &str, output: &str, changed: &[String], workdir: &Path) -> Checked {
    let unformatted = fmt_diff_paths(output, workdir);
    if unformatted.is_empty() {
        return Checked::Skipped(format!(
            "`{command}` failed without naming a file:\n{}",
            tail(output)
        ));
    }
    let changed: BTreeSet<&str> = changed.iter().map(String::as_str).collect();
    let theirs: Vec<&str> = unformatted
        .iter()
        .map(String::as_str)
        .filter(|path| changed.contains(path))
        .collect();
    if theirs.is_empty() {
        return Checked::Passed;
    }
    Checked::Failed(format!(
        "`{command}`: the attempt left {} unformatted.\n{}\n{FMT_FAILED}",
        theirs.join(", "),
        tail(output)
    ))
}

/// The files a `cargo fmt --check` `output` shows a diff of, relative to
/// `workdir`: its `Diff in <path>:<line>:` and older `Diff in <path> at line
/// <n>:` headers.
fn fmt_diff_paths(output: &str, workdir: &Path) -> BTreeSet<String> {
    let root = workdir.to_string_lossy();
    output
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("Diff in "))
        .map(|rest| {
            let rest = rest.trim_end().trim_end_matches(':');
            let path = match rest.split_once(" at line ") {
                Some((path, _)) => path,
                None => rest
                    .rsplit_once(':')
                    .filter(|(_, line)| line.chars().all(|c| c.is_ascii_digit()))
                    .map_or(rest, |(path, _)| path),
            };
            let path = path.strip_prefix(root.as_ref()).unwrap_or(path);
            path.trim_start_matches('/').to_string()
        })
        .collect()
}

/// The verdict row of `check` as it settled; `took` is how long it ran,
/// `None` when it did not.
fn rust_row(check: RustCheck, checked: &Checked, took: Option<Duration>) -> VerifyStepVerdict {
    let skip_reason = match checked {
        Checked::Skipped(why) => Some(why.clone()),
        Checked::Passed | Checked::Failed(_) => None,
    };
    VerifyStepVerdict {
        rung: check.rung().to_string(),
        passed: checked.passed(),
        duration_ms: took.map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX)),
        skipped: skip_reason.is_some(),
        skip_reason,
        ..VerifyStepVerdict::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher, no_auto_fix,
        verify_step,
    };
    use crate::graph_task_dispatch::verify_depth::CheckRun;

    /// A workspace of one library crate, `beta`, formatted.
    const MANIFEST: &str = "[workspace]\nmembers = [\"crates/beta\"]\nresolver = \"2\"\n";
    const BETA: &str =
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n";
    const LIB: &str = "//! Farewells.\n\n/// A goodbye for `name`.\npub fn farewell(name: &str) -> String {\n    format!(\"GOODBYE, {name}.\")\n}\n";

    fn changed(paths: &[&str]) -> Vec<String> {
        paths.iter().map(ToString::to_string).collect()
    }

    fn failed(output: &str) -> CheckRun {
        CheckRun {
            passed: false,
            reason: "exit 1".to_string(),
            output: output.to_string(),
        }
    }

    /// The checks cover each changed crate; the doc tests only those with a
    /// library; Rust elsewhere in the workspace takes the whole workspace.
    #[test]
    fn rust_commands_cover_the_changed_crates() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("Cargo.toml"), "[workspace]\n").expect("a manifest");
        std::fs::create_dir_all(temp.path().join("crates/beta/src")).expect("beta");
        std::fs::write(temp.path().join("crates/beta/src/lib.rs"), LIB).expect("its library");
        let scope = rust_scope(
            temp.path(),
            &changed(&["crates/beta/src/lib.rs", "crates/tool/src/main.rs", "a.md"]),
        )
        .expect("a Rust change");
        let commands = rust_commands(temp.path(), &scope);
        let commands: Vec<Option<&str>> = commands
            .iter()
            .map(|(_, command)| command.as_deref())
            .collect();
        assert_eq!(
            commands,
            [
                Some("cargo fmt --check -p beta -p tool"),
                Some("cargo clippy -p beta -p tool --no-deps -- -D warnings"),
                Some("cargo test -p beta --doc"),
            ]
        );
        let elsewhere = rust_scope(temp.path(), &changed(&["src/main.rs"])).expect("Rust");
        let commands = rust_commands(temp.path(), &elsewhere);
        assert_eq!(commands[0].1.as_deref(), Some("cargo fmt --check --all"));
        assert_eq!(
            commands[2].1.as_deref(),
            Some("cargo test --workspace --doc")
        );
        let binary = rust_scope(temp.path(), &changed(&["crates/tool/src/main.rs"])).expect("Rust");
        assert_eq!(rust_commands(temp.path(), &binary)[2].1, None);
        assert!(rust_scope(temp.path(), &changed(&["README.md"])).is_none());
    }

    /// fmt fails the attempt only for a file it changed: older formatting
    /// elsewhere in the crate passes, and a failure that names no file gives
    /// no verdict.
    #[test]
    fn fmt_counts_only_the_files_the_attempt_changed() {
        let root = Path::new("/work/tree");
        let output = "Diff in /work/tree/crates/beta/src/lib.rs:32:\n     }\n-\n }\n\
                      Diff in /work/tree/crates/beta/src/old.rs at line 4:\n";
        assert_eq!(
            fmt_diff_paths(output, root),
            BTreeSet::from([
                "crates/beta/src/lib.rs".to_string(),
                "crates/beta/src/old.rs".to_string()
            ])
        );
        let command = "cargo fmt --check -p beta";
        let mine = changed(&["crates/beta/src/lib.rs"]);
        let Checked::Failed(message) =
            judged(RustCheck::Fmt, command, &failed(output), &mine, root)
        else {
            panic!("the attempt's own file is unformatted");
        };
        assert!(
            message.contains("left crates/beta/src/lib.rs unformatted"),
            "{message}"
        );
        assert!(!message.contains("old.rs unformatted"), "{message}");
        let theirs = changed(&["crates/beta/src/new.rs"]);
        assert_eq!(
            judged(RustCheck::Fmt, command, &failed(output), &theirs, root),
            Checked::Passed
        );
        let crashed = failed("error: rustfmt is not installed for the toolchain\n");
        assert!(matches!(
            judged(RustCheck::Fmt, command, &crashed, &mine, root),
            Checked::Skipped(_)
        ));
        let unnamed = failed("error: package ID specification `tool` did not match any packages\n");
        assert!(matches!(
            judged(RustCheck::Clippy, command, &unnamed, &mine, root),
            Checked::Skipped(_)
        ));
        let doc = failed("error[E0425]: cannot find function `farewell` in this scope\n");
        let Checked::Failed(message) = judged(RustCheck::Doc, command, &doc, &mine, root) else {
            panic!("a doc example that does not compile fails");
        };
        assert!(message.contains("qualify the names"), "{message}");
    }

    /// The `rust:` rows of a verdict, as `rung passed`.
    fn rust_rows(verdict: &serde_json::Value) -> String {
        let steps = verdict["steps"].as_array().cloned().unwrap_or_default();
        let rows: Vec<String> = steps
            .iter()
            .filter_map(|step| step["rung"].as_str().map(|rung| (rung, &step["passed"])))
            .filter(|(rung, _)| rung.starts_with("rust:"))
            .map(|(rung, passed)| format!("{rung} {passed}"))
            .collect();
        rows.join(", ")
    }

    /// Dispatch one attempt whose provider writes `lib` to `beta`'s library,
    /// under `config`, in the run `run`; its result and its verdict's `rust:`
    /// rows.
    async fn attempt_writing(
        temp: &tempfile::TempDir,
        lib: &str,
        run: &str,
        config: impl FnOnce(&mut RokoConfig),
    ) -> (Result<Vec<Signal>>, String) {
        std::fs::write(temp.path().join("crates/beta/src/lib.rs"), LIB).expect("restore");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let write =
            format!("cat >/dev/null\ncat > crates/beta/src/lib.rs <<'ROKO_LIB'\n{lib}ROKO_LIB\n");
        let (dispatcher, mut task) = make_test_dispatcher(
            temp,
            &VERIFY_PROVIDER.replace("cat >/dev/null\n", &write),
            |roko| {
                no_auto_fix(roko);
                config(roko);
            },
            feedback,
        )
        .await;
        task.files = vec!["crates/beta/src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "check",
            "grep -q 'fn wave' crates/beta/src/lib.rs",
        )];
        let ctx = CellContext::new().with_run_id(run.to_string());
        let result = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await;
        let _ = GraphTaskDispatcher::close_run_attempts(&dispatcher, run);
        drop(dispatcher);
        let attempts = runs_dir.join(run).join("attempts.jsonl");
        let verdicts = jsonl_rows_where(&attempts, 1, |row| {
            row["schema_version"] == "roko.verdict/1"
        })
        .await;
        (result, verdicts.first().map(rust_rows).unwrap_or_default())
    }

    /// find-fc5cdd: an attempt that passes its own check but leaves its crate
    /// unformatted is refused by `rust:fmt` at its own task; the same change
    /// formatted passes fmt, clippy and the doc tests; and with `[gates]
    /// clippy_enabled = false` none of them runs.
    #[tokio::test]
    async fn rust_task_checks_refuse_an_unformatted_change() {
        let temp = tempfile::tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[
                ("Cargo.toml", MANIFEST),
                ("crates/beta/Cargo.toml", BETA),
                ("crates/beta/src/lib.rs", LIB),
            ],
        );
        let unformatted =
            format!("{LIB}\n/// A wave.\npub fn wave() -> &'static str {{   \"o/\" }}\n");
        let formatted =
            format!("{LIB}\n/// A wave.\npub fn wave() -> &'static str {{\n    \"o/\"\n}}\n");

        let (result, rows) = attempt_writing(&temp, &unformatted, "run-fmt", |_| {}).await;
        let message = result
            .expect_err("rust:fmt refuses the unformatted change")
            .to_string();
        assert!(message.contains("rust:fmt"), "{message}");
        assert!(
            message.contains("left crates/beta/src/lib.rs unformatted"),
            "{message}"
        );
        assert_eq!(rows, "rust:fmt false, rust:clippy null, rust:doc null");

        let (result, rows) = attempt_writing(&temp, &formatted, "run-clean", |_| {}).await;
        result.expect("a formatted change with no warning and no broken doc example passes");
        assert_eq!(rows, "rust:fmt true, rust:clippy true, rust:doc true");

        let off = |roko: &mut RokoConfig| roko.gates.clippy_enabled = false;
        let (result, rows) = attempt_writing(&temp, &unformatted, "run-off", off).await;
        result.expect("with clippy_enabled = false the Rust checks do not run");
        assert_eq!(rows, "");
    }
}
