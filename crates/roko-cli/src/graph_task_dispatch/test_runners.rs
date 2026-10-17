//! The test runners whose failures a verify step can be judged by
//! (gap-161be1, bug-5089fb).
//!
//! A step that runs a whole suite fails on any failing test. To tell the
//! tests a change broke from those that failed before it, the baseline
//! judgement ([`super::baseline_verify`]) needs the runner the step runs and
//! the name of every test that failed:
//!
//! - `cargo test`: each libtest binary prints its failures and a `test
//!   result:` line, and cargo names the binary on stderr;
//! - `python -m unittest`: an `ERROR:` or `FAIL:` header per failed test,
//!   then `Ran N tests` and `FAILED (failures=…, errors=…)`. A test module
//!   that cannot be imported fails as a test of its own, named after the
//!   module;
//! - `pytest` (`python -m pytest`, `py.test`): a `FAILED <node>` or `ERROR
//!   <node>` line per failure in its short summary, then its counts. A test
//!   module that cannot be imported stops the whole run ("Interrupted: N
//!   errors during collection"), so no test of it is known to pass or fail.
//!
//! Output that does not name every failure it counts names none: the
//! judgement then cannot tell one failure from another.

use std::collections::BTreeSet;
use std::path::Path;

use roko_gate::BuildSystem;

/// A test runner the baseline judgement reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TestRunner {
    /// `cargo test`, whose test binaries are libtest's.
    Cargo,
    /// `python -m unittest`.
    Unittest,
    /// `pytest`, `py.test` or `python -m pytest`.
    Pytest,
}

/// The words of `command` with the byte offset each ends at. Whitespace and
/// shell punctuation (`&|;(){}`) separate them.
fn words(command: &str) -> Vec<(&str, usize)> {
    let mut words = Vec::new();
    let mut start = None;
    for (index, character) in command.char_indices() {
        let separator = character.is_whitespace() || "&|;(){}".contains(character);
        match (separator, start) {
            (true, Some(begin)) => {
                words.push((&command[begin..index], index));
                start = None;
            }
            (false, None) => start = Some(index),
            _ => {}
        }
    }
    if let Some(begin) = start {
        words.push((&command[begin..], command.len()));
    }
    words
}

/// The last component of a program's path.
fn basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Whether `word` starts a Python interpreter: `python`, `python3` or
/// `python3.12`, by name or by path.
fn is_python(word: &str) -> bool {
    basename(word)
        .strip_prefix("python")
        .is_some_and(|version| version.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// The short flags a word holds (`-qx` holds `q` and `x`); empty for a long
/// flag or a word that is not a flag.
fn short_flags(word: &str) -> &str {
    word.strip_prefix('-')
        .filter(|flags| !flags.starts_with('-'))
        .unwrap_or_default()
}

impl TestRunner {
    /// Where each run of a test runner in `command` ends, as a byte offset:
    /// after `cargo test` (a toolchain may come between: `cargo +nightly
    /// test`), `python -m unittest`, `python -m pytest`, or the `pytest`
    /// program.
    fn runs(command: &str) -> Vec<(Self, usize)> {
        let words = words(command);
        let mut runs = Vec::new();
        for (index, &(word, end)) in words.iter().enumerate() {
            let after = |offset: usize| words.get(index + offset).copied();
            if word == "cargo" {
                let at = if after(1).is_some_and(|(word, _)| word.starts_with('+')) {
                    2
                } else {
                    1
                };
                if let Some(("test", end)) = after(at) {
                    runs.push((Self::Cargo, end));
                }
            } else if word == "-m" && words[..index].iter().any(|(word, _)| is_python(word)) {
                match after(1) {
                    Some(("unittest", end)) => runs.push((Self::Unittest, end)),
                    Some(("pytest", end)) => runs.push((Self::Pytest, end)),
                    _ => {}
                }
            } else if matches!(basename(word), "pytest" | "py.test")
                && (index == 0 || words[index - 1].0 != "-m")
            {
                runs.push((Self::Pytest, end));
            }
        }
        runs
    }

    /// The runner `part`, one `&&` part of a step's command, runs, and where
    /// the runner's own words end, where its options start; `None` when the
    /// part runs no test runner, or more than one.
    pub(super) fn find(part: &str) -> Option<(Self, usize)> {
        match Self::runs(part).as_slice() {
            &[run] => Some(run),
            _ => None,
        }
    }

    /// Whether `command` runs any test runner.
    pub(super) fn any_in(command: &str) -> bool {
        !Self::runs(command).is_empty()
    }

    /// `part` as the judgement runs it, so that every failure is named: with
    /// cargo's `--no-fail-fast` after `end` unless it is there, since cargo
    /// stops at the first failing test target. `None` when the part tells
    /// unittest or pytest to stop at the first failure (`-f`, `--failfast`;
    /// `-x`, `--exitfirst`, `--maxfail`), which no flag undoes.
    pub(super) fn without_fail_fast(self, part: &str, end: usize) -> Option<String> {
        let options = part[end..].split_whitespace();
        match self {
            Self::Cargo if part.contains("--no-fail-fast") => Some(part.to_string()),
            Self::Cargo => Some(format!("{} --no-fail-fast{}", &part[..end], &part[end..])),
            Self::Unittest => {
                let mut options = options;
                (!options.any(|word| word == "--failfast" || short_flags(word).contains('f')))
                    .then(|| part.to_string())
            }
            Self::Pytest => {
                let mut options = options;
                (!options.any(|word| {
                    word == "--exitfirst"
                        || word.starts_with("--maxfail")
                        || short_flags(word).contains('x')
                }))
                .then(|| part.to_string())
            }
        }
    }

    /// The tests `output` names as failed, or `None` unless it names every
    /// failure the runner counted: libtest's by target (`unittests
    /// src/lib.rs: tests::adds`), unittest's by test id
    /// (`test_env.EnvTest.test_level`, or the module's name, `test_osc`, for
    /// a module that could not be imported), pytest's by node id
    /// (`tests/test_env.py::test_level`).
    pub(super) fn failed_tests(self, output: &str) -> Option<BTreeSet<String>> {
        match self {
            Self::Cargo => libtest_failed_tests(output),
            Self::Unittest => unittest_failed_tests(output),
            Self::Pytest => pytest_failed_tests(output),
        }
    }

    /// Whether `output` shows that the step's tests ran and some failed, or,
    /// for pytest, that a test module could not be collected. Either shows
    /// that the parts of the command before the tests passed; a build that
    /// failed or a crash shows neither.
    pub(super) fn tests_failed(self, output: &str) -> bool {
        match self {
            Self::Cargo => roko_gate::parse_test_counts(output, BuildSystem::Cargo)
                .is_some_and(|counts| counts.failed > 0),
            Self::Unittest => unittest_summary(output).is_some_and(|summary| summary.failed() > 0),
            Self::Pytest => pytest_summary(output)
                .is_some_and(|summary| summary.interrupted || summary.failed + summary.errors > 0),
        }
    }

    /// How many tests `output` shows ran and passed, skipped tests and a
    /// module that could not be loaded left out; `None` without the runner's
    /// counts.
    pub(super) fn passed(self, output: &str) -> Option<u32> {
        match self {
            Self::Cargo => {
                roko_gate::parse_test_counts(output, BuildSystem::Cargo).map(|counts| counts.passed)
            }
            Self::Unittest => unittest_summary(output).map(|summary| summary.passed()),
            Self::Pytest => pytest_summary(output).map(|summary| summary.passed),
        }
    }

    /// How many tests `output` shows passed, all of them: `None` unless the
    /// runner finished with no failure. A run of no test passes nothing.
    pub(super) fn all_passed(self, output: &str) -> Option<u32> {
        let finished = match self {
            Self::Cargo => roko_gate::parse_test_counts(output, BuildSystem::Cargo)
                .is_some_and(|counts| counts.failed == 0),
            Self::Unittest => unittest_summary(output).is_some_and(|summary| summary.failed() == 0),
            Self::Pytest => pytest_summary(output).is_some_and(|summary| {
                !summary.interrupted && summary.failed + summary.errors == 0
            }),
        };
        if finished { self.passed(output) } else { None }
    }

    /// Where `test`, a failure as [`Self::failed_tests`] names it, is in
    /// `file`, a test file relative to the step's directory: `Some(true)`
    /// when it is the file's own failure, as unittest names a module it could
    /// not import and pytest a file it could not collect; `Some(false)` when
    /// it is one of the file's tests; `None` when it is not in the file.
    /// unittest names a module by its dotted path below the directory it
    /// discovers from, so any tail of the file's dotted path matches.
    pub(super) fn failure_in(self, test: &str, file: &str) -> Option<bool> {
        match self {
            Self::Cargo => None,
            Self::Unittest => {
                let module = file.strip_suffix(".py")?.replace('/', ".");
                std::iter::once(module.as_str())
                    .chain(module.match_indices('.').map(|(at, _)| &module[at + 1..]))
                    .find_map(|tail| {
                        let rest = test.strip_prefix(tail)?;
                        if rest.is_empty() {
                            Some(true)
                        } else {
                            rest.starts_with('.').then_some(false)
                        }
                    })
            }
            Self::Pytest => {
                let (node, item) = match test.split_once("::") {
                    Some((node, _)) => (node, true),
                    None => (test, false),
                };
                (node == file
                    || file.ends_with(&format!("/{node}"))
                    || node.ends_with(&format!("/{file}")))
                .then_some(!item)
            }
        }
    }

    /// Whether the runner's tests can be run file by file, as a check whose
    /// suite names no failure is judged by the test files a change adds or
    /// touches (bug-5089fb): unittest's and pytest's, not cargo's.
    pub(super) fn runs_files(self) -> bool {
        matches!(self, Self::Unittest | Self::Pytest)
    }

    /// Whether `path` is a test file the runner collects by default:
    /// `test*.py` for unittest, `test_*.py` and `*_test.py` for pytest.
    pub(super) fn is_test_file(self, path: &str) -> bool {
        let name = basename(path);
        let Some(stem) = name.strip_suffix(".py") else {
            return false;
        };
        match self {
            Self::Cargo => false,
            Self::Unittest => stem.starts_with("test"),
            Self::Pytest => stem.starts_with("test_") || stem.ends_with("_test"),
        }
    }

    /// The command that runs only `files`, test files relative to the
    /// step's directory, the way `part` runs the runner (bug-5089fb): `part`
    /// up to the runner's words, then for pytest its options, without the
    /// paths that exist in `workdir`, then `files`. Pipes and redirections
    /// after the runner are dropped, so its whole output is judged. `None`
    /// for cargo.
    pub(super) fn scoped(
        self,
        part: &str,
        end: usize,
        files: &[String],
        workdir: &Path,
    ) -> Option<String> {
        if !self.runs_files() || files.is_empty() {
            return None;
        }
        let mut command = part[..end].trim_end().to_string();
        if self == Self::Pytest {
            for word in part[end..].split_whitespace() {
                if word.starts_with('|') || word.contains('>') || word.starts_with('<') {
                    break;
                }
                if word.starts_with('-') || !workdir.join(word).exists() {
                    command.push(' ');
                    command.push_str(word);
                }
            }
        }
        for file in files {
            command.push(' ');
            command.push_str(file);
        }
        Some(command)
    }
}

// ── libtest ──────────────────────────────────────────────────────────────

/// The test target a cargo header line names: `Running unittests src/lib.rs
/// (target/debug/deps/demo-0f1e2d)` names `unittests src/lib.rs`, without
/// the build hash, which differs between checkouts, and `Doc-tests demo`
/// names `doc-tests demo`.
fn test_target(line: &str) -> Option<String> {
    if let Some(running) = line.strip_prefix("Running ") {
        let (source, binary) = running.rsplit_once(" (")?;
        binary.ends_with(')').then(|| source.to_string())
    } else {
        line.strip_prefix("Doc-tests ")
            .map(|name| format!("doc-tests {name}"))
    }
}

/// Whether `line` starts a test binary's run: `running 3 tests`.
fn starts_test_run(line: &str) -> bool {
    line.strip_prefix("running ")
        .and_then(|rest| {
            rest.strip_suffix(" tests")
                .or_else(|| rest.strip_suffix(" test"))
        })
        .is_some_and(|count| !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()))
}

/// The failed tests in a `cargo test` run's output, each named by its test
/// target (`unittests src/lib.rs: tests::adds`). `None` unless every test
/// binary that started printed its result, with its failures named one by
/// one, and the binaries pair up with cargo's target headers.
///
/// Cargo prints the headers on stderr and the test binaries print on
/// stdout, which the gate output keeps apart, so they pair up in order.
fn libtest_failed_tests(output: &str) -> Option<BTreeSet<String>> {
    let mut targets = Vec::new();
    // Per test binary that started: the tests it failed, and whether it
    // printed its result.
    let mut runs: Vec<(Vec<&str>, bool)> = Vec::new();
    for line in output.lines().map(str::trim) {
        if let Some(target) = test_target(line) {
            targets.push(target);
        } else if starts_test_run(line) {
            if runs.last().is_some_and(|(_, finished)| !finished) {
                return None;
            }
            runs.push((Vec::new(), false));
        } else if line.starts_with("test result:") {
            let (failed, finished) = runs.last_mut().filter(|(_, finished)| !*finished)?;
            let counts = roko_gate::parse_test_counts(line, BuildSystem::Cargo)?;
            if counts.failed as usize != failed.len() {
                return None;
            }
            *finished = true;
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.strip_suffix(" ... FAILED"))
        {
            let (failed, _) = runs.last_mut().filter(|(_, finished)| !*finished)?;
            failed.push(name);
        }
    }
    if runs.is_empty() || runs.len() != targets.len() || runs.iter().any(|(_, finished)| !finished)
    {
        return None;
    }
    Some(
        targets
            .iter()
            .zip(&runs)
            .flat_map(|(target, (failed, _))| {
                failed.iter().map(move |name| format!("{target}: {name}"))
            })
            .collect(),
    )
}

// ── unittest ─────────────────────────────────────────────────────────────

/// unittest's closing lines: how many tests ran, and its counts of
/// failures, errors, unexpected successes, skipped tests and expected
/// failures.
#[derive(Debug, Default, PartialEq, Eq)]
struct UnittestSummary {
    ran: u32,
    failures: u32,
    errors: u32,
    unexpected: u32,
    skipped: u32,
    expected: u32,
}

impl UnittestSummary {
    fn failed(&self) -> u32 {
        self.failures + self.errors + self.unexpected
    }

    fn passed(&self) -> u32 {
        self.ran
            .saturating_sub(self.failed() + self.skipped + self.expected)
    }
}

/// unittest's summary in `output`: `Ran N tests in …s`, then `OK`, `OK
/// (skipped=1)` or `FAILED (failures=1, errors=2, skipped=1)`; `None`
/// without both.
fn unittest_summary(output: &str) -> Option<UnittestSummary> {
    let mut summary = None;
    for line in output.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Ran ") {
            let mut words = rest.split_whitespace();
            let ran = words.next().and_then(|count| count.parse().ok());
            let tests = words.next().is_some_and(|word| word.starts_with("test"));
            if let (Some(ran), true) = (ran, tests) {
                summary = Some((
                    UnittestSummary {
                        ran,
                        ..UnittestSummary::default()
                    },
                    false,
                ));
            }
            continue;
        }
        let Some((summary, ended)) = summary.as_mut().filter(|(_, ended)| !*ended) else {
            continue;
        };
        if line == "OK" {
            *ended = true;
        } else if let Some(counts) = line
            .strip_prefix("OK (")
            .or_else(|| line.strip_prefix("FAILED ("))
            .and_then(|counts| counts.strip_suffix(')'))
        {
            for count in counts.split(", ") {
                let Some((label, value)) = count.split_once('=') else {
                    continue;
                };
                let value = value.parse().unwrap_or(0);
                match label {
                    "failures" => summary.failures = value,
                    "errors" => summary.errors = value,
                    "unexpected successes" => summary.unexpected = value,
                    "skipped" => summary.skipped = value,
                    "expected failures" => summary.expected = value,
                    _ => {}
                }
            }
            *ended = true;
        }
    }
    summary
        .filter(|(_, ended)| *ended)
        .map(|(summary, _)| summary)
}

/// How unittest names a failed test: by its id (`test_env.EnvTest.test_x`;
/// Python before 3.11 prints the class, `test_env.EnvTest`, so the method
/// is added), or by the module's name for a module that could not be
/// imported or loaded (`unittest.loader._FailedTest.test_osc`).
fn unittest_name(name: &str, id: &str) -> String {
    if id.starts_with("unittest.loader.") || id.starts_with("unittest.suite.") {
        name.to_string()
    } else if id == name || id.ends_with(&format!(".{name}")) {
        id.to_string()
    } else {
        format!("{id}.{name}")
    }
}

/// The failed tests in a unittest run's output, from its `ERROR:`, `FAIL:`
/// and `UNEXPECTED SUCCESS:` headers. `None` unless its summary counts
/// exactly the failures the headers name.
fn unittest_failed_tests(output: &str) -> Option<BTreeSet<String>> {
    let summary = unittest_summary(output)?;
    let mut failed = BTreeSet::new();
    let mut named = 0;
    for line in output.lines().map(str::trim_end) {
        let Some(header) = ["ERROR: ", "FAIL: ", "UNEXPECTED SUCCESS: "]
            .iter()
            .find_map(|kind| line.strip_prefix(kind))
        else {
            continue;
        };
        let (name, id) = header.rsplit_once(" (")?;
        failed.insert(unittest_name(name, id.strip_suffix(')')?));
        named += 1;
    }
    (named == summary.failed()).then_some(failed)
}

// ── pytest ───────────────────────────────────────────────────────────────

/// pytest's closing counts, and whether it stopped at collection.
#[derive(Debug, Default, PartialEq, Eq)]
struct PytestSummary {
    passed: u32,
    failed: u32,
    errors: u32,
    /// "Interrupted: N errors during collection": no test ran.
    interrupted: bool,
}

/// pytest's summary in `output`: its last count line (`2 failed, 1 passed,
/// 1 error in 0.02s`, plain with `-q`, between `=` runs otherwise, or `no
/// tests ran in …s`), and whether it was interrupted. `None` without one.
fn pytest_summary(output: &str) -> Option<PytestSummary> {
    let mut summary = None;
    let mut interrupted = false;
    for line in output.lines() {
        let line = line.trim().trim_matches('=').trim();
        if line.contains("Interrupted: ") {
            interrupted = true;
            continue;
        }
        let Some((counts, time)) = line.rsplit_once(" in ") else {
            continue;
        };
        let time = time.split_whitespace().next().unwrap_or_default();
        if !time.ends_with('s') || time.trim_end_matches('s').parse::<f64>().is_err() {
            continue;
        }
        let mut found = PytestSummary::default();
        let mut counted = false;
        for count in counts.split(", ") {
            let (value, label) = match count.split_once(' ') {
                Some(("no", "tests ran")) => {
                    counted = true;
                    continue;
                }
                Some(pair) => pair,
                None => continue,
            };
            let Ok(value) = value.parse::<u32>() else {
                continue;
            };
            counted = true;
            match label {
                "passed" => found.passed = value,
                "failed" => found.failed = value,
                "error" | "errors" => found.errors = value,
                _ => {}
            }
        }
        if counted {
            summary = Some(found);
        }
    }
    summary.map(|summary| PytestSummary {
        interrupted,
        ..summary
    })
}

/// The failed tests in a pytest run's output, by the node ids of its short
/// summary's `FAILED` and `ERROR` lines. `None` for a run interrupted at
/// collection, and unless its counts match the lines.
fn pytest_failed_tests(output: &str) -> Option<BTreeSet<String>> {
    let summary = pytest_summary(output).filter(|summary| !summary.interrupted)?;
    let mut failed = BTreeSet::new();
    let (mut failures, mut errors) = (0, 0);
    for line in output.lines() {
        let (node, count) = if let Some(rest) = line.strip_prefix("FAILED ") {
            (rest, &mut failures)
        } else if let Some(rest) = line.strip_prefix("ERROR ") {
            (rest, &mut errors)
        } else {
            continue;
        };
        let node = node.split(" - ").next().unwrap_or(node).trim();
        if node.is_empty() {
            return None;
        }
        failed.insert(node.to_string());
        *count += 1;
    }
    (failures == summary.failed && errors == summary.errors).then_some(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// unittest's output for pocket-synth's seed (bug-5089fb): four test
    /// modules whose modules under test are not written yet, as `ShellGate`
    /// keeps it (unittest writes to stderr).
    const UNITTEST_RED_BASE: &str = "$ PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s \
         tests -q\n\n---stderr---\n\
         ======================================================================\n\
         ERROR: test_env (unittest.loader._FailedTest.test_env)\n\
         ----------------------------------------------------------------------\n\
         ImportError: Failed to import test module: test_env\n\
         Traceback (most recent call last):\n    from synth.env import ADSR\n\
         ModuleNotFoundError: No module named 'synth.env'\n\n\n\
         ======================================================================\n\
         ERROR: test_osc (unittest.loader._FailedTest.test_osc)\n\
         ----------------------------------------------------------------------\n\
         ImportError: Failed to import test module: test_osc\n\n\
         ======================================================================\n\
         ERROR: test_svf (unittest.loader._FailedTest.test_svf)\n\
         ----------------------------------------------------------------------\n\
         ImportError: Failed to import test module: test_svf\n\n\
         ----------------------------------------------------------------------\n\
         Ran 3 tests in 0.000s\n\nFAILED (errors=3)\n";

    /// The same suite once `synth/env.py` exists, one of its tests failing.
    const UNITTEST_AFTER: &str = "---stderr---\n\
         ======================================================================\n\
         ERROR: test_osc (unittest.loader._FailedTest.test_osc)\n\
         ----------------------------------------------------------------------\n\
         ImportError: Failed to import test module: test_osc\n\n\
         ======================================================================\n\
         FAIL: test_adsr_shape (test_env.ADSRTest.test_adsr_shape)\n\
         ----------------------------------------------------------------------\n\
         AssertionError: 0.5 != 1.0\n\n\
         ----------------------------------------------------------------------\n\
         Ran 6 tests in 0.016s\n\nFAILED (failures=1, errors=1)\n";

    /// pytest's `-q` output for a run with failures and an error.
    const PYTEST_FAILURES: &str = "F.F\n=================================== FAILURES \
         ===================================\n___ test_bad ___\ntests/test_a.py:5: AssertionError\n\
         =========================== short test summary info ============================\n\
         FAILED tests/test_a.py::test_bad - assert 1 == 2\n\
         FAILED tests/test_a.py::TestK::test_m - ValueError: boom\n\
         ERROR tests/test_b.py::test_uses - RuntimeError: fixture\n\
         2 failed, 1 passed, 1 error in 0.02s\n";

    /// pytest's `-q` output for pocket-synth's seed: collection stops.
    const PYTEST_INTERRUPTED: &str = "==================================== ERRORS \
         ====================================\n\
         ______________________ ERROR collecting tests/test_env.py ______________________\n\
         ImportError while importing test module 'tests/test_env.py'.\n\
         =========================== short test summary info ============================\n\
         ERROR tests/test_env.py\nERROR tests/test_osc.py\n\
         !!!!!!!!!!!!!!!!!!! Interrupted: 2 errors during collection !!!!!!!!!!!!!!!!!!!!\n\
         2 errors in 0.14s\n";

    fn names(failed: Option<BTreeSet<String>>) -> Option<Vec<String>> {
        failed.map(|failed| failed.into_iter().collect())
    }

    #[test]
    fn runners_are_found_with_where_their_words_end() {
        fn find(part: &str) -> Option<(TestRunner, &str)> {
            TestRunner::find(part).map(|(runner, end)| (runner, &part[..end]))
        }
        assert_eq!(
            find("cargo +nightly test -p demo"),
            Some((TestRunner::Cargo, "cargo +nightly test"))
        );
        assert_eq!(
            find("PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -q"),
            Some((
                TestRunner::Unittest,
                "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest"
            ))
        );
        assert_eq!(
            find("/usr/bin/python3.12 -m pytest -q tests"),
            Some((TestRunner::Pytest, "/usr/bin/python3.12 -m pytest"))
        );
        assert_eq!(
            find("uv run pytest tests/test_env.py"),
            Some((TestRunner::Pytest, "uv run pytest"))
        );
        for part in [
            "cargo clippy -p demo",
            "cargo nextest run -p demo",
            "python3 tests/run.py",
            "npm test",
            "cargo test -p a; cargo test -p b",
        ] {
            assert_eq!(TestRunner::find(part), None, "{part}");
        }
        assert!(TestRunner::any_in("test -f x && python -m unittest"));
        assert!(!TestRunner::any_in("cargo clippy -p demo -- -D warnings"));
    }

    #[test]
    fn fail_fast_is_turned_off_or_refused() {
        let off = |part: &str| {
            let (runner, end) = TestRunner::find(part).expect("a runner");
            runner.without_fail_fast(part, end)
        };
        assert_eq!(
            off("cargo test -p demo").as_deref(),
            Some("cargo test --no-fail-fast -p demo")
        );
        assert_eq!(
            off("python3 -m unittest discover -q").as_deref(),
            Some("python3 -m unittest discover -q")
        );
        for part in [
            "python3 -m unittest -f",
            "python3 -m unittest --failfast",
            "python3 -m unittest -qf",
            "pytest -x",
            "pytest -qx tests",
            "pytest --exitfirst",
            "pytest --maxfail=2",
        ] {
            assert_eq!(off(part), None, "{part}");
        }
    }

    #[test]
    fn unittest_failures_are_named_by_test_id_or_module() {
        assert_eq!(
            names(TestRunner::Unittest.failed_tests(UNITTEST_RED_BASE)),
            Some(vec![
                "test_env".to_string(),
                "test_osc".to_string(),
                "test_svf".to_string()
            ])
        );
        assert_eq!(
            names(TestRunner::Unittest.failed_tests(UNITTEST_AFTER)),
            Some(vec![
                "test_env.ADSRTest.test_adsr_shape".to_string(),
                "test_osc".to_string()
            ])
        );
        // Python before 3.11 prints the class, not the method.
        let older =
            UNITTEST_AFTER.replace("(test_env.ADSRTest.test_adsr_shape)", "(test_env.ADSRTest)");
        assert_eq!(
            names(TestRunner::Unittest.failed_tests(&older)),
            names(TestRunner::Unittest.failed_tests(UNITTEST_AFTER))
        );
        let ok = "......\n----\nRan 6 tests in 0.010s\n\nOK\n";
        assert_eq!(
            names(TestRunner::Unittest.failed_tests(ok)),
            Some(Vec::new())
        );
        assert_eq!(TestRunner::Unittest.all_passed(ok), Some(6));
        let skipped = "..s\n----\nRan 3 tests in 0.010s\n\nOK (skipped=1)\n";
        assert_eq!(TestRunner::Unittest.all_passed(skipped), Some(2));
        // Of six tests, the module that did not import and the failure did
        // not pass.
        assert_eq!(TestRunner::Unittest.passed(UNITTEST_AFTER), Some(4));
        assert_eq!(TestRunner::Unittest.passed(UNITTEST_RED_BASE), Some(0));
        assert_eq!(TestRunner::Unittest.all_passed(UNITTEST_AFTER), None);
        assert!(TestRunner::Unittest.tests_failed(UNITTEST_AFTER));
        assert!(!TestRunner::Unittest.tests_failed(ok));

        // A count the headers do not account for, or no summary at all.
        let unnamed = UNITTEST_AFTER.replace("FAIL: test_adsr_shape", "test_adsr_shape");
        assert_eq!(TestRunner::Unittest.failed_tests(&unnamed), None);
        let crashed = "ImportError: Start directory is not importable: 'missing'\n";
        assert_eq!(TestRunner::Unittest.failed_tests(crashed), None);
        assert!(!TestRunner::Unittest.tests_failed(crashed));
    }

    #[test]
    fn pytest_failures_are_named_by_node_and_collection_stops_name_none() {
        assert_eq!(
            names(TestRunner::Pytest.failed_tests(PYTEST_FAILURES)),
            Some(vec![
                "tests/test_a.py::TestK::test_m".to_string(),
                "tests/test_a.py::test_bad".to_string(),
                "tests/test_b.py::test_uses".to_string(),
            ])
        );
        assert!(TestRunner::Pytest.tests_failed(PYTEST_FAILURES));
        assert_eq!(TestRunner::Pytest.all_passed(PYTEST_FAILURES), None);
        // Collection stopped: tests failed in a sense, but none is named.
        assert_eq!(TestRunner::Pytest.failed_tests(PYTEST_INTERRUPTED), None);
        assert!(TestRunner::Pytest.tests_failed(PYTEST_INTERRUPTED));
        let passed = "....\n4 passed in 0.01s\n";
        assert_eq!(
            names(TestRunner::Pytest.failed_tests(passed)),
            Some(Vec::new())
        );
        assert_eq!(TestRunner::Pytest.all_passed(passed), Some(4));
        let decorated =
            "============================== 4 passed in 0.01s ===============================\n";
        assert_eq!(TestRunner::Pytest.all_passed(decorated), Some(4));
        let none = "no tests ran in 0.01s\n";
        assert_eq!(TestRunner::Pytest.all_passed(none), Some(0));
        assert_eq!(TestRunner::Pytest.passed(PYTEST_FAILURES), Some(1));
        assert_eq!(TestRunner::Pytest.passed(PYTEST_INTERRUPTED), Some(0));
        // A summary that names fewer failures than it counts.
        let short =
            PYTEST_FAILURES.replace("FAILED tests/test_a.py::test_bad - assert 1 == 2\n", "");
        assert_eq!(TestRunner::Pytest.failed_tests(&short), None);
    }

    #[test]
    fn scoped_commands_run_only_the_given_test_files() {
        let workdir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(workdir.path().join("tests")).expect("tests dir");
        let scoped = |part: &str| {
            let (runner, end) = TestRunner::find(part).expect("a runner");
            runner.scoped(
                part,
                end,
                &["tests/test_env.py".to_string()],
                workdir.path(),
            )
        };
        assert_eq!(
            scoped("PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -q 2>&1")
                .as_deref(),
            Some("PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/test_env.py")
        );
        assert_eq!(
            scoped("python3 -m pytest -q -p no:cacheprovider tests | tail -5").as_deref(),
            Some("python3 -m pytest -q -p no:cacheprovider tests/test_env.py")
        );
        assert_eq!(scoped("cargo test -p demo"), None);
        assert!(TestRunner::Unittest.is_test_file("tests/test_env.py"));
        assert!(!TestRunner::Unittest.is_test_file("synth/env.py"));
        assert!(TestRunner::Pytest.is_test_file("pkg/env_test.py"));
        assert!(!TestRunner::Pytest.is_test_file("tests/testing.py"));
    }

    #[test]
    fn failures_are_placed_in_their_test_files() {
        let unittest = |test: &str| TestRunner::Unittest.failure_in(test, "tests/test_env.py");
        assert_eq!(unittest("test_env"), Some(true));
        assert_eq!(unittest("tests.test_env"), Some(true));
        assert_eq!(unittest("test_env.ADSRTest.test_level"), Some(false));
        assert_eq!(unittest("tests.test_env.ADSRTest.test_level"), Some(false));
        assert_eq!(unittest("test_envelope.T.test_x"), None);
        assert_eq!(unittest("test_osc"), None);
        let pytest = |test: &str| TestRunner::Pytest.failure_in(test, "tests/test_env.py");
        assert_eq!(pytest("tests/test_env.py"), Some(true));
        assert_eq!(pytest("tests/test_env.py::test_level"), Some(false));
        assert_eq!(pytest("test_env.py::T::test_level"), Some(false));
        assert_eq!(pytest("tests/test_osc.py::test_saw"), None);
        assert_eq!(
            TestRunner::Cargo.failure_in("unittests src/lib.rs: a", "src/lib.rs"),
            None
        );
    }
}
