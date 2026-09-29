//! Attempt diff check: what an agent's attempt changed that it must not have
//! (S05 check A1, run inline before the verify steps).
//!
//! A pure check over the paths an attempt changed and, where it matters,
//! their text before and after. It finds:
//!
//! - **weakened tests**: a test file deleted, renamed where test runners no
//!   longer find it, or emptied; fewer tests or fewer asserts across the
//!   attempt's test code (tests may move between files); an `#[ignore]` or
//!   another skip marker added. Adding and strengthening tests is fine;
//! - **edited checks**: a script the task's verify steps run, a
//!   `tasks.toml`, CI config, a test snapshot, test-runner config, or the
//!   `[gates]` table of `roko.toml`, unless the task's `files` name the path;
//! - **edited pinned acceptance tests**: anything under a plan's `accept/`,
//!   a `[task.accept]` source, or its `dest` holding other text than the
//!   pinned test, whatever `files` says;
//! - **out of scope**: a path the task's `files` do not name.
//!
//! Every kind but the last is tampering ([`DiffFindingKind::is_tamper`]).
//! The caller decides what a finding costs: roko's Graph dispatch fails the
//! attempt on tampering, and on scope findings only under
//! `[gates] diff_scope = "enforce"`.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

/// How a path changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The path is new.
    Added,
    /// The path's content or mode changed.
    Modified,
    /// The path is gone.
    Deleted,
    /// The path moved, from [`AttemptChange::old_path`].
    Renamed,
}

/// One path an attempt changed, relative to the working tree and
/// `/`-separated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptChange {
    /// How it changed.
    pub kind: ChangeKind,
    /// The path now; for a deletion, the deleted path.
    pub path: String,
    /// The path before a rename.
    pub old_path: Option<String>,
    /// Its text before the change: `None` when it did not exist, or its text
    /// was not read (binary, too large, or not needed, see
    /// [`AttemptDiffPolicy::needs_text`]).
    pub before: Option<String>,
    /// Its text after the change, likewise.
    pub after: Option<String>,
}

impl AttemptChange {
    /// A change of `kind` to `path`, with no text read.
    #[must_use]
    pub fn new(kind: ChangeKind, path: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
            old_path: None,
            before: None,
            after: None,
        }
    }

    /// The path before the change: the old path of a rename, else the path.
    #[must_use]
    pub fn old_path(&self) -> &str {
        self.old_path.as_deref().unwrap_or(&self.path)
    }
}

/// A planner-written acceptance test pinned outside the agent's reach
/// (`[task.accept]`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinnedTest {
    /// The planner's source, relative to the working tree; `None` when the
    /// plan directory is outside it.
    pub src: Option<String>,
    /// Where the harness copies the pinned test before running it.
    pub dest: String,
    /// The pinned text, which the harness itself writes to `dest`. `None`
    /// when unknown: then any change to `dest` counts.
    pub text: Option<String>,
}

/// What an attempt may change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttemptDiffPolicy {
    /// The task's `files`: paths, directories or `*` globs relative to the
    /// working tree. Empty: the task has no scope to check.
    pub task_files: Vec<String>,
    /// Scripts the task's verify steps run ([`scripts_run_by`]).
    pub verify_scripts: Vec<String>,
    /// The task's pinned acceptance tests.
    pub pinned_tests: Vec<PinnedTest>,
    /// Directories of planner-written acceptance tests, such as the plan's
    /// `accept/`: any change under them counts.
    pub accept_dirs: Vec<String>,
}

impl AttemptDiffPolicy {
    /// Whether the check reads the text of `path` before and after: test
    /// code, Rust files (which hold inline tests), `roko.toml` and pinned
    /// test destinations. Callers may leave the text of other paths out.
    #[must_use]
    pub fn needs_text(&self, path: &str) -> bool {
        is_test_path(path)
            || path.ends_with(".rs")
            || file_name(path) == "roko.toml"
            || self.pinned_tests.iter().any(|pinned| pinned.dest == path)
    }

    /// Whether the task's `files` name `path`.
    fn declares(&self, path: &str) -> bool {
        self.task_files
            .iter()
            .any(|declared| covers(declared, path))
    }
}

/// What a [`DiffFinding`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiffFindingKind {
    /// A test file was deleted.
    TestDeleted,
    /// A test file was renamed where test runners no longer find it.
    TestRenamed,
    /// A test file was left with nothing but blank lines and comments.
    TestEmptied,
    /// The attempt's test code holds fewer tests than before.
    TestsRemoved,
    /// The attempt's test code holds fewer asserts than before.
    AssertsRemoved,
    /// An `#[ignore]` or another skip marker was added.
    SkipAdded,
    /// A script the task's verify steps run was edited.
    VerifyScriptEdited,
    /// A `tasks.toml`, which holds verify steps, was edited.
    TasksTomlEdited,
    /// A pinned acceptance test was edited.
    AcceptEdited,
    /// Gate configuration was edited: `roko.toml`'s `[gates]`, CI config, a
    /// test snapshot, or test-runner config.
    GateConfigEdited,
    /// A path the task's `files` do not name was changed.
    OutsideScope,
}

impl DiffFindingKind {
    /// Whether the finding is tampering with a check, rather than a scope
    /// finding.
    #[must_use]
    pub const fn is_tamper(self) -> bool {
        !matches!(self, Self::OutsideScope)
    }

    /// Stable snake_case name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TestDeleted => "test_deleted",
            Self::TestRenamed => "test_renamed",
            Self::TestEmptied => "test_emptied",
            Self::TestsRemoved => "tests_removed",
            Self::AssertsRemoved => "asserts_removed",
            Self::SkipAdded => "skip_added",
            Self::VerifyScriptEdited => "verify_script_edited",
            Self::TasksTomlEdited => "tasks_toml_edited",
            Self::AcceptEdited => "accept_edited",
            Self::GateConfigEdited => "gate_config_edited",
            Self::OutsideScope => "outside_scope",
        }
    }
}

/// One thing an attempt changed that it must not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFinding {
    /// What was found.
    pub kind: DiffFindingKind,
    /// Where.
    pub path: String,
    /// One line on what changed.
    pub detail: String,
}

impl DiffFinding {
    /// A finding of `kind` at `path`.
    #[must_use]
    pub fn new(kind: DiffFindingKind, path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for DiffFinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} `{}`: {}",
            self.kind.label(),
            self.path,
            self.detail
        )
    }
}

/// Check what an attempt changed against what its task may change. Findings
/// come in the order of `changes`, then the counts over all test code.
#[must_use]
pub fn check_attempt_diff(
    changes: &[AttemptChange],
    policy: &AttemptDiffPolicy,
) -> Vec<DiffFinding> {
    use DiffFindingKind as Kind;

    let mut findings = Vec::new();
    // Test and assert counts per file, before and after, for the totals.
    let mut tallies: Vec<(&str, Tally, Tally)> = Vec::new();
    // Files already found deleted, renamed away or emptied.
    let mut gone: BTreeSet<&str> = BTreeSet::new();

    for change in changes {
        let path = change.path.as_str();
        let old_path = change.old_path();

        // Pinned acceptance tests are the planner's, whatever `files` says.
        let pinned_src = policy
            .pinned_tests
            .iter()
            .filter_map(|pinned| pinned.src.as_deref())
            .chain(policy.accept_dirs.iter().map(String::as_str))
            .any(|pinned| covers(pinned, path) || covers(pinned, old_path));
        if pinned_src {
            findings.push(DiffFinding::new(
                Kind::AcceptEdited,
                path,
                "a planner-written acceptance test changed; it is pinned, not the task's to edit",
            ));
            continue;
        }
        if let Some(pinned) = policy
            .pinned_tests
            .iter()
            .find(|pinned| pinned.dest == path || pinned.dest == old_path)
        {
            // The harness copies the pinned text here itself.
            let copied = !matches!(change.kind, ChangeKind::Deleted | ChangeKind::Renamed)
                && pinned.text.is_some()
                && change.after == pinned.text;
            if !copied {
                findings.push(DiffFinding::new(
                    Kind::AcceptEdited,
                    pinned.dest.as_str(),
                    "the destination of a pinned acceptance test no longer holds the pinned test",
                ));
            }
            continue;
        }

        if is_test_path(old_path) {
            match change.kind {
                ChangeKind::Deleted => {
                    gone.insert(path);
                    findings.push(DiffFinding::new(
                        Kind::TestDeleted,
                        path,
                        "a test file was deleted",
                    ));
                }
                ChangeKind::Renamed if !is_test_path(path) => {
                    gone.insert(path);
                    findings.push(DiffFinding::new(
                        Kind::TestRenamed,
                        old_path,
                        format!("a test file was renamed to `{path}`, where test runners miss it"),
                    ));
                }
                _ => {}
            }
            let substantive = |text: &Option<String>| {
                text.as_deref()
                    .map(|text| text.lines().filter(|line| is_substantive(line)).count())
            };
            if change.kind != ChangeKind::Deleted
                && substantive(&change.before).is_some_and(|lines| lines > 0)
                && substantive(&change.after) == Some(0)
            {
                gone.insert(path);
                findings.push(DiffFinding::new(
                    Kind::TestEmptied,
                    path,
                    "a test file was left with only blank lines and comments",
                ));
            }
        }

        let declared = policy.declares(path) || policy.declares(old_path);
        if !declared {
            if policy
                .verify_scripts
                .iter()
                .any(|script| script == path || script == old_path)
            {
                findings.push(DiffFinding::new(
                    Kind::VerifyScriptEdited,
                    path,
                    "a script the task's verify steps run was edited",
                ));
            } else if file_name(path) == "tasks.toml" || file_name(old_path) == "tasks.toml" {
                findings.push(DiffFinding::new(
                    Kind::TasksTomlEdited,
                    path,
                    "a plan's tasks.toml, which holds its verify steps, was edited",
                ));
            } else if let Some(detail) = gate_config_edit(change) {
                findings.push(DiffFinding::new(Kind::GateConfigEdited, path, detail));
            }
        }

        let before = change
            .before
            .as_deref()
            .map_or_else(Tally::default, |text| Tally::of(old_path, text));
        let after = change
            .after
            .as_deref()
            .map_or_else(Tally::default, |text| Tally::of(path, text));
        if after.skips > before.skips {
            findings.push(DiffFinding::new(
                Kind::SkipAdded,
                path,
                format!(
                    "{} skip marker(s) added, such as `#[ignore]`",
                    after.skips - before.skips
                ),
            ));
        }
        tallies.push((path, before, after));

        if !policy.task_files.is_empty() && !declared && !is_generated(path) {
            findings.push(DiffFinding::new(
                Kind::OutsideScope,
                path,
                "the task's files do not name it",
            ));
        }
    }

    // Totals over all the test code the attempt touched: a test or an assert
    // may move to another file, but the attempt may not end with fewer.
    let totals: [(Kind, &str, fn(&Tally) -> u32); 2] = [
        (Kind::TestsRemoved, "test", |tally| tally.tests),
        (Kind::AssertsRemoved, "assert", |tally| tally.asserts),
    ];
    for (kind, noun, count) in totals {
        let before: u32 = tallies.iter().map(|(_, before, _)| count(before)).sum();
        let after: u32 = tallies.iter().map(|(_, _, after)| count(after)).sum();
        if after >= before {
            continue;
        }
        for (path, before, after) in &tallies {
            let lost = count(before).saturating_sub(count(after));
            if lost > 0 && !gone.contains(path) {
                findings.push(DiffFinding::new(
                    kind,
                    *path,
                    format!("{lost} {noun}(s) fewer than before"),
                ));
            }
        }
    }
    findings
}

/// Files a verify command runs as scripts: the first word of each command
/// when it is a relative path (`./check.sh`, `scripts/verify`), and the first
/// argument of an interpreter (`bash`, `sh`, `python3`, `node`, …). Paths
/// come back without a leading `./`.
#[must_use]
pub fn scripts_run_by(command: &str) -> Vec<String> {
    const INTERPRETERS: &[&str] = &[
        "sh", "bash", "zsh", "dash", "python", "python3", "node", "deno", "bun", "ruby", "perl",
        "tsx", "ts-node",
    ];
    let mut scripts = Vec::new();
    for segment in command.split(['\n', ';', '|', '&', '(', ')']) {
        let mut words = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
            .filter(|word| !word.is_empty())
            .skip_while(|word| is_env_assignment(word));
        let Some(first) = words.next() else {
            continue;
        };
        let script = if INTERPRETERS.contains(&file_name(first)) {
            words.find(|word| !word.starts_with('-'))
        } else {
            first.contains('/').then_some(first)
        };
        if let Some(script) = script.map(|script| script.trim_start_matches("./"))
            && !script.is_empty()
            && !script.starts_with(['/', '$', '~'])
            && !script.contains("..")
        {
            scripts.push(script.to_string());
        }
    }
    scripts.sort();
    scripts.dedup();
    scripts
}

/// Whether `path` is test code by the usual layouts: under a `tests`,
/// `test`, `__tests__`, `spec` or `specs` directory, or named `*_test.*`,
/// `*_tests.*`, `test_*.py`, `*.test.*`, `*.spec.*`, `tests.rs` or
/// `conftest.py`.
#[must_use]
pub fn is_test_path(path: &str) -> bool {
    let mut components: Vec<&str> = path.split('/').collect();
    let name = components.pop().unwrap_or_default();
    if components
        .iter()
        .any(|dir| matches!(*dir, "tests" | "test" | "__tests__" | "spec" | "specs"))
    {
        return true;
    }
    let stem = name.split('.').next().unwrap_or(name);
    (name.starts_with("test_") && name.ends_with(".py"))
        || stem.ends_with("_test")
        || stem.ends_with("_tests")
        || name.contains(".test.")
        || name.contains(".spec.")
        || matches!(name, "tests.rs" | "conftest.py")
}

/// Why a change to `change.path` edits gate configuration, if it does.
fn gate_config_edit(change: &AttemptChange) -> Option<String> {
    let path = change.path.as_str();
    let name = file_name(path);
    let ci = [
        ".github/workflows/",
        ".circleci/",
        ".buildkite/",
        ".gitlab/",
    ]
    .iter()
    .any(|dir| path.starts_with(dir))
        || matches!(
            name,
            ".gitlab-ci.yml" | "Jenkinsfile" | "azure-pipelines.yml" | ".travis.yml"
        );
    if ci {
        return Some("CI configuration was edited".to_string());
    }
    let snapshot = name.ends_with(".snap")
        || path.starts_with("__snapshots__/")
        || path.contains("/__snapshots__/");
    if snapshot && change.kind != ChangeKind::Added {
        return Some("a test snapshot was edited".to_string());
    }
    let runner_config = matches!(name, "pytest.ini" | "nextest.toml" | ".mocharc.yml")
        || ["jest.config.", "vitest.config.", "karma.conf."]
            .iter()
            .any(|prefix| name.starts_with(prefix));
    if runner_config {
        return Some("test-runner configuration was edited".to_string());
    }
    if name != "roko.toml" {
        return None;
    }
    let gates = |text: &Option<String>| -> Result<Option<toml::Value>, ()> {
        match text.as_deref() {
            None => Ok(None),
            Some(text) => toml::from_str::<toml::Table>(text)
                .map(|mut table| table.remove("gates"))
                .map_err(drop),
        }
    };
    let before = gates(&change.before);
    let after = if change.kind == ChangeKind::Deleted {
        Ok(None)
    } else {
        gates(&change.after)
    };
    (before != after).then(|| "the [gates] table of roko.toml was edited".to_string())
}

/// Tests, asserts and skip markers in one file's test code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tally {
    tests: u32,
    asserts: u32,
    skips: u32,
}

/// How tests, asserts and skips look in one language.
struct Markers {
    /// Prefixes of a line that starts a test.
    tests: &'static [&'static str],
    asserts: &'static LazyLock<Regex>,
    /// Text that skips a test.
    skips: &'static [&'static str],
}

static RUST_ASSERTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bassert\w*!|\bprop_assert\w*!").expect("valid Rust assert regex")
});
static PYTHON_ASSERTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bassert\b|\bself\.assert\w+\(|\bpytest\.raises\(")
        .expect("valid Python assert regex")
});
static JS_ASSERTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bexpect\(|\bassert(?:\.\w+)?\(").expect("valid JavaScript assert regex")
});
static GO_ASSERTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bt\.(?:Error|Errorf|Fatal|Fatalf)\(|\b(?:assert|require)\.\w+\(")
        .expect("valid Go assert regex")
});

impl Markers {
    fn for_path(path: &str) -> Option<Self> {
        let extension = file_name(path).rsplit_once('.')?.1;
        Some(match extension {
            "rs" => Self {
                tests: &[
                    "#[test]",
                    "#[tokio::test",
                    "#[rstest",
                    "#[test_case",
                    "#[async_std::test",
                    "#[sqlx::test",
                ],
                asserts: &RUST_ASSERTS,
                skips: &["#[ignore"],
            },
            "py" => Self {
                tests: &["def test", "async def test"],
                asserts: &PYTHON_ASSERTS,
                skips: &[
                    "@pytest.mark.skip",
                    "@pytest.mark.xfail",
                    "@unittest.skip",
                    "pytest.skip(",
                    ".skipTest(",
                ],
            },
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => Self {
                tests: &["it(", "it.", "test(", "test."],
                asserts: &JS_ASSERTS,
                skips: &[".skip(", "xit(", "xdescribe(", "xtest(", ".todo("],
            },
            "go" => Self {
                tests: &["func Test"],
                asserts: &GO_ASSERTS,
                skips: &["t.Skip"],
            },
            _ => return None,
        })
    }
}

impl Tally {
    /// Count the test code in `text`, the content of `path`: all of a test
    /// file, and what follows `#[cfg(test)]` in other Rust files.
    fn of(path: &str, text: &str) -> Self {
        let Some(markers) = Markers::for_path(path) else {
            return Self::default();
        };
        let region = if is_test_path(path) {
            text
        } else if path.ends_with(".rs") {
            text.find("#[cfg(test)]").map_or("", |start| &text[start..])
        } else {
            ""
        };
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Self {
            tests: count(
                region
                    .lines()
                    .filter(|line| {
                        let line = line.trim_start();
                        markers.tests.iter().any(|prefix| line.starts_with(prefix))
                    })
                    .count(),
            ),
            asserts: count(markers.asserts.find_iter(region).count()),
            skips: count(
                markers
                    .skips
                    .iter()
                    .map(|skip| region.matches(skip).count())
                    .sum(),
            ),
        }
    }
}

/// Whether `line` holds code rather than blank space or a comment.
fn is_substantive(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty()
        && !["//", "/*", "*", "#"]
            .iter()
            .any(|marker| line.starts_with(marker))
}

/// Whether `path` is output a build or test run leaves behind.
fn is_generated(path: &str) -> bool {
    path.ends_with(".pyc")
        || path.split('/').any(|dir| {
            matches!(
                dir,
                "__pycache__" | ".pytest_cache" | "node_modules" | "target"
            )
        })
}

/// Whether `word` is a shell `NAME=value` prefix of a command.
fn is_env_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
    })
}

/// The last component of `path`.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Whether the `files` entry `declared` covers `path`: the same path, a
/// directory above it, or a glob (`*`, `**`, `?`) matching it.
fn covers(declared: &str, path: &str) -> bool {
    let declared = declared.trim();
    let declared = declared.strip_prefix("./").unwrap_or(declared);
    let declared = declared.trim_end_matches('/');
    if declared.is_empty() {
        return false;
    }
    if declared.contains('*') || declared.contains('?') {
        return glob_matches(declared.as_bytes(), path.as_bytes());
    }
    path == declared
        || path
            .strip_prefix(declared)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Match `path` against a glob: `*` within one path component, `**` across
/// components, `?` one character.
fn glob_matches(pattern: &[u8], path: &[u8]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((b'*', rest)) => {
            let (any_depth, rest) = match rest.split_first() {
                Some((b'*', rest)) => (true, rest.strip_prefix(b"/").unwrap_or(rest)),
                _ => (false, rest),
            };
            (0..=path.len()).any(|split| {
                (any_depth || !path[..split].contains(&b'/')) && glob_matches(rest, &path[split..])
            })
        }
        Some((b'?', rest)) => path
            .split_first()
            .is_some_and(|(byte, path)| *byte != b'/' && glob_matches(rest, path)),
        Some((byte, rest)) => path
            .split_first()
            .is_some_and(|(other, path)| byte == other && glob_matches(rest, path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(
        kind: ChangeKind,
        path: &str,
        before: Option<&str>,
        after: Option<&str>,
    ) -> AttemptChange {
        AttemptChange {
            before: before.map(str::to_string),
            after: after.map(str::to_string),
            ..AttemptChange::new(kind, path)
        }
    }

    fn kinds(findings: &[DiffFinding]) -> Vec<(DiffFindingKind, &str)> {
        findings
            .iter()
            .map(|finding| (finding.kind, finding.path.as_str()))
            .collect()
    }

    const RUST_TEST: &str = "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n    assert!(add(0, 0) == 0);\n}\n";

    /// One fixture per finding kind; each is found alone, at its path.
    #[test]
    fn attempt_diff_flags_each_tamper_kind() {
        let policy = AttemptDiffPolicy {
            task_files: vec!["src/".to_string(), "docs/*.md".to_string()],
            verify_scripts: scripts_run_by("cargo check && bash scripts/verify.sh --fast"),
            pinned_tests: vec![PinnedTest {
                src: Some("plans/p/accept/add_test.rs".to_string()),
                dest: "tests/accept_add.rs".to_string(),
                text: Some(RUST_TEST.to_string()),
            }],
            accept_dirs: vec!["plans/p/accept".to_string()],
        };
        let weakened = RUST_TEST.replace("    assert!(add(0, 0) == 0);\n", "");
        let ignored = RUST_TEST.replace("#[test]\n", "#[test]\n#[ignore]\n");
        let inline = |body: &str| {
            format!(
                "pub fn add(a: u8, b: u8) -> u8 {{ a + b }}\n\n#[cfg(test)]\nmod tests {{\n{body}}}\n"
            )
        };
        let cases: Vec<(AttemptChange, DiffFindingKind, &str)> = vec![
            (
                change(ChangeKind::Deleted, "tests/math.rs", Some(RUST_TEST), None),
                DiffFindingKind::TestDeleted,
                "tests/math.rs",
            ),
            (
                AttemptChange {
                    old_path: Some("tests/math.rs".to_string()),
                    ..change(
                        ChangeKind::Renamed,
                        "src/math_checks.rs",
                        Some(RUST_TEST),
                        Some(RUST_TEST),
                    )
                },
                DiffFindingKind::TestRenamed,
                "tests/math.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "tests/math.rs",
                    Some(RUST_TEST),
                    Some("// nothing to see\n\n"),
                ),
                DiffFindingKind::TestEmptied,
                "tests/math.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "src/lib.rs",
                    Some(&inline(
                        "#[test]\nfn a() { assert!(true); }\n#[test]\nfn b() {}\n",
                    )),
                    Some(&inline("#[test]\nfn a() { assert!(true); }\n")),
                ),
                DiffFindingKind::TestsRemoved,
                "src/lib.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "tests/math.rs",
                    Some(RUST_TEST),
                    Some(&weakened),
                ),
                DiffFindingKind::AssertsRemoved,
                "tests/math.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "tests/math.rs",
                    Some(RUST_TEST),
                    Some(&ignored),
                ),
                DiffFindingKind::SkipAdded,
                "tests/math.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "tests/test_calc.py",
                    Some("def test_a():\n    assert 1\n"),
                    Some("@pytest.mark.skip\ndef test_a():\n    assert 1\n"),
                ),
                DiffFindingKind::SkipAdded,
                "tests/test_calc.py",
            ),
            (
                change(ChangeKind::Modified, "scripts/verify.sh", None, None),
                DiffFindingKind::VerifyScriptEdited,
                "scripts/verify.sh",
            ),
            (
                change(ChangeKind::Modified, "plans/q/tasks.toml", None, None),
                DiffFindingKind::TasksTomlEdited,
                "plans/q/tasks.toml",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "plans/p/accept/add_test.rs",
                    None,
                    None,
                ),
                DiffFindingKind::AcceptEdited,
                "plans/p/accept/add_test.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "tests/accept_add.rs",
                    Some(RUST_TEST),
                    Some(&weakened),
                ),
                DiffFindingKind::AcceptEdited,
                "tests/accept_add.rs",
            ),
            (
                change(
                    ChangeKind::Modified,
                    "roko.toml",
                    Some("[agent]\nbare_mode = false\n\n[gates]\nmode = \"full\"\n"),
                    Some("[agent]\nbare_mode = false\n\n[gates]\nmode = \"none\"\n"),
                ),
                DiffFindingKind::GateConfigEdited,
                "roko.toml",
            ),
            (
                change(ChangeKind::Modified, ".github/workflows/ci.yml", None, None),
                DiffFindingKind::GateConfigEdited,
                ".github/workflows/ci.yml",
            ),
            (
                change(ChangeKind::Modified, "snapshots/cli__help.snap", None, None),
                DiffFindingKind::GateConfigEdited,
                "snapshots/cli__help.snap",
            ),
            (
                change(ChangeKind::Modified, "README.md", None, None),
                DiffFindingKind::OutsideScope,
                "README.md",
            ),
        ];
        for (change, kind, path) in cases {
            let findings = check_attempt_diff(std::slice::from_ref(&change), &policy);
            let only_scope_besides = findings
                .iter()
                .filter(|finding| finding.kind != kind)
                .all(|finding| finding.kind == DiffFindingKind::OutsideScope);
            assert!(
                findings
                    .iter()
                    .any(|finding| finding.kind == kind && finding.path == path)
                    && only_scope_besides,
                "{kind:?} at {path}: {findings:?}"
            );
            assert_eq!(kind.is_tamper(), kind != DiffFindingKind::OutsideScope);
        }
    }

    #[test]
    fn legitimate_changes_find_nothing() {
        let policy = AttemptDiffPolicy {
            task_files: vec![
                "src/lib.rs".to_string(),
                "tests/".to_string(),
                "roko.toml".to_string(),
            ],
            verify_scripts: vec!["scripts/verify.sh".to_string()],
            pinned_tests: vec![PinnedTest {
                src: None,
                dest: "tests/accept_add.rs".to_string(),
                text: Some(RUST_TEST.to_string()),
            }],
            accept_dirs: Vec::new(),
        };
        let more = RUST_TEST.replace(
            "}\n",
            "}\n\n#[test]\nfn more() {\n    assert_eq!(add(2, 2), 4);\n}\n",
        );
        let changes = [
            // A new test, and a stronger existing one.
            change(ChangeKind::Added, "tests/new.rs", None, Some(RUST_TEST)),
            change(
                ChangeKind::Modified,
                "tests/math.rs",
                Some(RUST_TEST),
                Some(&more),
            ),
            // Product code losing a debug assert is not test code.
            change(
                ChangeKind::Modified,
                "src/lib.rs",
                Some("fn f() { assert!(x); }\n"),
                Some("fn f() {}\n"),
            ),
            // The harness's own copy of the pinned test.
            change(
                ChangeKind::Added,
                "tests/accept_add.rs",
                None,
                Some(RUST_TEST),
            ),
            // roko.toml outside [gates], and a new snapshot.
            change(
                ChangeKind::Modified,
                "roko.toml",
                Some("[agent]\na = 1\n"),
                Some("[agent]\na = 2\n"),
            ),
            change(ChangeKind::Added, "src/snapshots/new.snap", None, None),
        ];
        let findings = check_attempt_diff(&changes, &policy);
        assert_eq!(
            kinds(&findings),
            [(DiffFindingKind::OutsideScope, "src/snapshots/new.snap")]
        );

        // A test moved between files is not a test removed.
        let moved = [
            change(
                ChangeKind::Modified,
                "tests/a.rs",
                Some(RUST_TEST),
                Some("use super::*;\n#[test]\nfn keep() { assert!(true); }\n"),
            ),
            change(
                ChangeKind::Modified,
                "tests/b.rs",
                Some("use super::*;\n#[test]\nfn keep() { assert!(true); }\n"),
                Some(RUST_TEST),
            ),
        ];
        assert!(check_attempt_diff(&moved, &policy).is_empty());
    }

    #[test]
    fn scripts_and_scopes_are_read_like_a_shell_and_a_glob() {
        assert_eq!(
            scripts_run_by(
                "cd web && ./node_modules/.bin/tsc; FOO=1 python3 -u tools/check.py | tee out\nsh 'ci/run tests.sh'"
            ),
            ["ci/run", "node_modules/.bin/tsc", "tools/check.py"]
        );
        assert!(scripts_run_by("cargo test -p roko-cli && grep -q foo src/lib.rs").is_empty());

        assert!(covers("src/", "src/a/b.rs"));
        assert!(covers("./src/lib.rs", "src/lib.rs"));
        assert!(!covers("src", "srcs/lib.rs"));
        assert!(covers("crates/*/src/lib.rs", "crates/x/src/lib.rs"));
        assert!(!covers("crates/*.rs", "crates/x/lib.rs"));
        assert!(covers("crates/**/*.rs", "crates/x/src/lib.rs"));
        assert!(covers("docs/**", "docs/a/b.md"));

        assert!(is_test_path("crates/roko-cli/tests/smoke.rs"));
        assert!(is_test_path("app/src/view.test.tsx"));
        assert!(is_test_path("pkg/calc_test.go"));
        assert!(is_test_path("test_calc.py"));
        assert!(!is_test_path("src/latest.rs"));
        assert!(!is_test_path("src/contest.py"));
    }
}
