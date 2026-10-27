//! Integration coverage for `roko plan validate`.

mod common;
use common::Isolated;

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_plan(root: &Path, plan_id: &str, tasks_toml: &str) -> PathBuf {
    let plan_dir = write_tasks(root, plan_id, tasks_toml);
    // The fixtures' checks run cargo, which needs a manifest: without one,
    // `plan validate` warns that they cannot pass (PLAN_059).
    let manifest = root.join("Cargo.toml");
    if !manifest.exists() {
        fs::write(manifest, "[workspace]\n").unwrap();
    }
    plan_dir
}

/// Write `plans/<plan_id>/tasks.toml` under `root`, and nothing else.
fn write_tasks(root: &Path, plan_id: &str, tasks_toml: &str) -> PathBuf {
    let plan_dir = root.join("plans").join(plan_id);
    fs::create_dir_all(&plan_dir).unwrap();
    fs::write(plan_dir.join("tasks.toml"), tasks_toml).unwrap();
    plan_dir
}

fn run_validate(temp: &TempDir, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .current_dir(temp.path())
        .arg("plan")
        .arg("validate")
        .args(args)
        .assert()
}

fn write_model_registry(root: &Path) {
    fs::write(
        root.join("roko.toml"),
        r#"
[models.claude-haiku-4-5]
provider = "claude_cli"
slug = "claude-haiku-4-5"

[models.claude-sonnet-4-6]
provider = "claude_cli"
slug = "claude-sonnet-4-6"

[models.claude-opus-4-6]
provider = "claude_cli"
slug = "claude-opus-4-6"
"#,
    )
    .unwrap();
}

#[test]
fn plan_validate_help_shows_new_flags() {
    let assert = Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .arg("plan")
        .arg("validate")
        .arg("--help")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("--strict"),
        "missing --strict flag: {stdout}"
    );
    assert!(stdout.contains("--json"), "missing --json flag: {stdout}");
    assert!(
        stdout.contains("[DIR]") || stdout.contains("[dir]"),
        "missing directory argument in help: {stdout}"
    );
}

#[test]
fn plan_validate_succeeds_for_well_formed_plan() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "good",
        r#"
[meta]
plan = "good"

[[task]]
id = "T1"
title = "Implement the validator"
role = "implementer"
files = ["src/lib.rs"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("0 diagnostics in 1 plan"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn plan_validate_strict_allows_missing_task_outputs() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "new-output",
        r#"
[meta]
plan = "new-output"

[[task]]
id = "T1"
title = "Create a new output"
role = "implementer"
files = ["crates/new-crate/src/lib.rs", "docs/generated.md"]
depends_on = []
verify = [{ phase = "structural", command = "test -f docs/generated.md" }]
"#,
    );

    let assert = run_validate(&temp, &["plans", "--strict"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("0 diagnostics in 1 plan"),
        "missing outputs should be accepted: {stdout}"
    );
}

#[test]
fn plan_validate_strict_rejects_missing_context_prerequisites() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "missing-input",
        r#"
[meta]
plan = "missing-input"

[[task]]
id = "T1"
title = "Read a required input"
role = "implementer"
files = ["docs/generated.md"]
depends_on = []
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "docs/missing.md", why = "required source" }]
"#,
    );

    let assert = run_validate(&temp, &["plans", "--strict"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_031"), "missing PLAN_031: {stdout}");
    assert!(
        stdout.contains("requires prerequisite 'docs/missing.md'"),
        "missing prerequisite detail: {stdout}"
    );
}

#[test]
fn plan_validate_strict_accepts_dependency_created_prerequisite() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "generated-input",
        r#"
[meta]
plan = "generated-input"

[[task]]
id = "T1"
title = "Create shared input"
role = "implementer"
files = ["generated/shared.md"]
depends_on = []
verify = [{ phase = "structural", command = "test -f generated/shared.md" }]

[[task]]
id = "T2"
title = "Consume shared input"
role = "implementer"
files = ["src/lib.rs"]
depends_on = ["T1"]
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/shared.md", why = "T1 output" }]
"#,
    );

    let assert = run_validate(&temp, &["plans", "--strict"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("0 diagnostics in 1 plan"),
        "dependency-created prerequisite should pass: {stdout}"
    );
}

#[test]
fn plan_validate_reports_schema_validation_errors() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "schema",
        r#"
[meta]
plan = "schema"

[[task]]
id = "T1"
title = "Missing implementer files"
role = "implementer"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_035"), "missing PLAN_035: {stdout}");
    assert!(
        stdout.contains("missing 'files'"),
        "missing files diagnostic: {stdout}"
    );
}

#[test]
fn plan_validate_warns_on_known_model_aliases() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(temp.path().join("src/lib.rs"), "").unwrap();
    write_model_registry(temp.path());
    write_plan(
        temp.path(),
        "aliases",
        r#"
[meta]
plan = "aliases"
# One task at a time: the three tasks share a file, and this test is about
# model aliases, not about tasks that could run together.
max_parallel = 1

[[task]]
id = "T1"
title = "Mechanical alias"
role = "implementer"
files = ["src/lib.rs"]
model_hint = "haiku"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[[task]]
id = "T2"
title = "Focused alias"
role = "implementer"
files = ["src/lib.rs"]
model_hint = "sonnet"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[[task]]
id = "T3"
title = "Architectural alias"
role = "implementer"
files = ["src/lib.rs"]
model_hint = "opus"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Each task gets two warnings: PLAN_012 for naming its model by an
    // alias, and PLAN_041 because a `model_hint` pins the model past the
    // routing ladder (on by default) however it is spelled; the full name
    // clears only the first.
    assert!(
        stdout.contains("6 diagnostics in 1 plan"),
        "unexpected stdout: {stdout}"
    );
    assert_eq!(stdout.matches("PLAN_012").count(), 3, "{stdout}");
    assert!(
        stdout.contains("uses model alias 'haiku'; prefer the full name 'claude-haiku-4-5'"),
        "missing haiku alias warning: {stdout}"
    );
    assert!(
        stdout.contains("uses model alias 'sonnet'; prefer the full name 'claude-sonnet-4-6'"),
        "missing sonnet alias warning: {stdout}"
    );
    assert!(
        stdout.contains("uses model alias 'opus'; prefer the full name 'claude-opus-4-6'"),
        "missing opus alias warning: {stdout}"
    );
    assert_eq!(stdout.matches("PLAN_041").count(), 3, "{stdout}");
    for (task, alias) in [("T1", "haiku"), ("T2", "sonnet"), ("T3", "opus")] {
        assert!(
            stdout.contains(&format!(
                "task '{task}' sets model_hint = '{alias}', which pins a model and bypasses the \
                 routing ladder"
            )),
            "missing PLAN_041 for {task}: {stdout}"
        );
    }
}

#[test]
fn plan_validate_preserves_unknown_model_warning() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(temp.path().join("src/lib.rs"), "").unwrap();
    write_model_registry(temp.path());
    write_plan(
        temp.path(),
        "mystery-model",
        r#"
[meta]
plan = "mystery-model"

[[task]]
id = "T1"
title = "Unknown model"
role = "implementer"
files = ["src/lib.rs"]
model_hint = "definitely-not-a-model"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // PLAN_009 for the unknown model, and PLAN_041 because the hint still
    // pins the task's model past the routing ladder (on by default).
    assert!(
        stdout.contains("2 diagnostics in 1 plan"),
        "unexpected stdout: {stdout}"
    );
    assert!(stdout.contains("PLAN_009"), "missing PLAN_009: {stdout}");
    assert!(
        stdout.contains("uses model 'definitely-not-a-model' which is not configured in roko.toml"),
        "missing original mystery-model warning: {stdout}"
    );
    assert!(stdout.contains("PLAN_041"), "missing PLAN_041: {stdout}");
    assert!(
        stdout.contains(
            "task 'T1' sets model_hint = 'definitely-not-a-model', which pins a model and \
             bypasses the routing ladder"
        ),
        "missing PLAN_041 pin warning: {stdout}"
    );
}

#[test]
fn plan_validate_reports_cycles() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "cycle",
        r#"
[meta]
plan = "cycle"

[[task]]
id = "T1"
title = "First"
role = "implementer"
files = ["src/lib.rs"]
depends_on = ["T2"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[[task]]
id = "T2"
title = "Second"
role = "implementer"
files = ["src/lib.rs"]
depends_on = ["T1"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_006"), "missing PLAN_006: {stdout}");
}

#[test]
fn plan_validate_reports_missing_role_templates() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "missing-template",
        r#"
[meta]
plan = "missing-template"

[[task]]
id = "T1"
title = "Validate a missing template"
role = "researcher"
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans", "--strict"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_008"), "missing PLAN_008: {stdout}");
}

#[test]
fn plan_validate_reports_invalid_gate_rungs() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "bad-rung",
        r#"
[meta]
plan = "bad-rung"

[[task]]
id = "T1"
title = "Use an invalid rung"
role = "implementer"
files = ["src/lib.rs"]
gate_rung = 9
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_007"), "missing PLAN_007: {stdout}");
}

/// 3206: a key that `plan run` does not read is a PLAN_043 warning naming
/// its table. A context key at the top level of a task says where it
/// belongs, and a typo says which key it most likely means. Both are
/// warnings, so only `--strict` fails.
#[test]
fn plan_validate_warns_on_unknown_task_key() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "keys",
        r#"
[meta]
plan = "keys"

[[task]]
id = "T1"
title = "Slugify titles"
role = "implementer"
files = ["src/slug.py"]
depends_on = []
read_files = ["tests/test_slug.py"]
verify = [{ phase = "test", command = "python3 -m unittest tests.test_slug" }]

[[task]]
id = "T2"
title = "Document slugify"
role = "implementer"
files = ["docs/slug.md"]
depends_on = ["T1"]
verify = [{ phase = "structural", command = "test -f docs/slug.md" }]
verfy = [{ phase = "structural", command = "grep -q slugify docs/slug.md" }]
"#,
    );
    // The test module T1 reads and its check runs.
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("tests/test_slug.py"),
        "import unittest\n\n\nclass SlugTest(unittest.TestCase):\n    def test_slug(self):\n        pass\n",
    )
    .unwrap();

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.matches("PLAN_043").count(), 2, "{stdout}");
    assert!(
        stdout.contains(
            "task 'T1' sets `read_files` at the top level of [[task]], where plan run ignores \
             it; move it under [task.context]"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "task 'T2' [[task]] has unknown key `verfy`, which plan run ignores; did you mean \
             `verify`?"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("2 diagnostics in 1 plan"), "{stdout}");

    let strict = run_validate(&temp, &["plans", "--strict"]).failure();
    assert_eq!(strict.get_output().status.code(), Some(1));
}

/// 3207: the TSS v1 example plan sets every field of the task-spec
/// standard and passes `plan validate --strict` without a PLAN_043. It runs
/// on a copy, with the context files its task reads, so the test leaves the
/// checkout alone.
#[test]
fn tss_v1_example_passes_strict_validation() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let example = fs::read_to_string(repo.join("plans/_fixtures/tss-v1-example/tasks.toml"))
        .expect("read the TSS v1 example");
    let parsed: toml::Value = toml::from_str(&example).expect("parse the example");
    let task = &parsed["task"][0];
    for key in [
        "goal",
        "non_goals",
        "assumptions",
        "open_questions",
        "hidden",
        "acceptance",
    ] {
        assert!(task.get(key).is_some(), "the example sets `{key}`");
    }
    let step = &task["verify"][0];
    for key in ["covers", "expect"] {
        assert!(step.get(key).is_some(), "the verify step sets `{key}`");
    }

    let temp = TempDir::new().unwrap();
    write_plan(temp.path(), "tss-v1-example", &example);
    for entry in task["context"]["read_files"].as_array().unwrap() {
        let path = entry["path"].as_str().unwrap();
        let copy = temp.path().join(path);
        fs::create_dir_all(copy.parent().unwrap()).unwrap();
        fs::copy(repo.join(path), copy).expect("copy a context file");
    }

    let assert = run_validate(&temp, &["plans", "--strict"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.contains("PLAN_043"), "{stdout}");
    assert!(stdout.contains("0 diagnostics in 1 plan"), "{stdout}");
}

/// 3212: a task of any role without a verify step can only end unverified,
/// so it is a PLAN_037 error without `--strict`. A plan that sets `[meta]
/// allow_unverified = true` says it means that, and passes.
#[test]
fn plan_validate_rejects_verify_less_researcher_task() {
    let temp = TempDir::new().unwrap();
    let plan = |meta: &str| {
        format!(
            r#"
[meta]
plan = "survey"
{meta}
[[task]]
id = "T1"
title = "Survey the retry callers"
role = "researcher"
depends_on = []
"#
        )
    };

    write_plan(temp.path(), "survey", &plan(""));
    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("error PLAN_037 task 'T1' has no verify steps"),
        "{stdout}"
    );

    write_plan(temp.path(), "survey", &plan("allow_unverified = true\n"));
    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("warn  PLAN_037 task 'T1' has no verify steps"),
        "{stdout}"
    );
}

#[test]
fn plan_validate_accepts_typed_acceptance_contract() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "contract",
        r#"
[meta]
plan = "contract"

[[task]]
id = "T1"
title = "Define a done gate"
role = "implementer"
files = ["crates/roko-gate/src/acceptance_contract.rs"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-gate" }]

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
command = "cargo check -p roko-gate"

[task.acceptance_contract.no_stub]
production_paths = ["crates/roko-gate/src"]

[task.acceptance_contract.agent_output]
schema = "roko.acceptance.agent_output.v1"

[task.acceptance_contract.review_verdict]
reviewer_role_id = "quick-reviewer"
min_confidence = 0.6

[task.acceptance_contract.recovery]
retry = true
reflection = true
replan = true

[task.acceptance_contract.parity_ledger]

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "RT00.done-gate"
source_ref = "tmp/architecture-plans/08-end-to-end-acceptance.md"
evidence_ref = "crates/roko-gate/src/acceptance_contract.rs"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // The contract is accepted: its one finding is the PLAN_046 warning
    // that contracts are not enforced at run time (3230).
    assert!(
        stdout.contains("1 diagnostics in 1 plan") && stdout.contains("PLAN_046"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn plan_validate_fails_closed_for_malformed_acceptance_contract() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "bad-contract",
        r#"
[meta]
plan = "bad-contract"

[[task]]
id = "T1"
title = "Define a bad done gate"
role = "implementer"
files = ["crates/roko-gate/src/acceptance_contract.rs"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-gate" }]

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("ACCEPT_003"),
        "missing ACCEPT_003: {stdout}"
    );
}

#[test]
fn plan_validate_accepts_architecture_queue_packets() {
    let temp = TempDir::new().unwrap();
    // The architecture source is a declared context.read_files prerequisite.
    std::fs::create_dir_all(temp.path().join("tmp/architecture-plans")).unwrap();
    std::fs::write(
        temp.path()
            .join("tmp/architecture-plans/06-architecture-implementation.md"),
        "# source plan\n",
    )
    .unwrap();
    write_plan(
        temp.path(),
        "architecture",
        r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Implement one architecture packet"
role = "implementer"
files = ["crates/roko-core/src/config/schema.rs"]
depends_on = []
acceptance = ["AC1: roko-core compiles with the packet's schema change"]
verify = [{ phase = "compile", command = "cargo check -p roko-core", covers = ["AC1"] }]

[task.context]
read_files = [
  { path = "tmp/architecture-plans/06-architecture-implementation.md", why = "source plan" },
]

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
command = "cargo check -p roko-core"

[task.acceptance_contract.agent_output]
schema = "roko.architecture_packet.v1"

[task.acceptance_contract.review_verdict]
reviewer_role_id = "quick-reviewer"
min_confidence = 0.6

[task.acceptance_contract.recovery]
retry = true
reflection = true

[task.acceptance_contract.parity_ledger]

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "ARCH-Q1"
source_ref = "tmp/architecture-plans/06-architecture-implementation.md"
evidence_ref = "crates/roko-core/src/config/schema.rs"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Checked acceptance satisfies PLAN_024, and beside it the contract
    // (kept for its parity rows) draws no PLAN_046 warning (bug-ac2a51).
    assert!(
        stdout.contains("0 diagnostics in 1 plan") && !stdout.contains("PLAN_046"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn plan_validate_fails_closed_for_incomplete_architecture_queue_packets() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "architecture",
        r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Incomplete architecture packet"
role = "implementer"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for rule in ["PLAN_020", "PLAN_021", "PLAN_022", "PLAN_023", "PLAN_024"] {
        assert!(stdout.contains(rule), "missing {rule}: {stdout}");
    }
}

#[test]
fn plan_validate_requires_parity_rows_for_architecture_queue_packets() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "architecture",
        r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Architecture packet without parity closure"
role = "implementer"
files = ["crates/roko-gate/src/acceptance_contract.rs"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-gate" }]

[task.context]
read_files = [
  { path = "tmp/architecture-plans/06-architecture-implementation.md", why = "source plan" },
]

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
command = "cargo check -p roko-gate"

[task.acceptance_contract.agent_output]
schema = "roko.architecture_packet.v1"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_025"), "missing PLAN_025: {stdout}");
}

#[test]
fn plan_validate_requires_complete_architecture_deferral_metadata() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "architecture",
        r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Deferred advanced packet"
role = "implementer"
files = ["plans/architecture-core-queue/tasks.toml"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[task.context]
read_files = [
  { path = "docs/08-chain/INDEX.md", why = "advanced source inventory" },
]

[task.deferral]
rationale = "Advanced surface must wait for trustworthy execution."

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
command = "cargo check -p roko-cli"

[task.acceptance_contract.agent_output]
schema = "roko.architecture_packet.v1"

[task.acceptance_contract.review_verdict]
reviewer_role_id = "quick-reviewer"
min_confidence = 0.6

[task.acceptance_contract.recovery]
retry = true
reflection = true

[task.acceptance_contract.parity_ledger]

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "ARCH-DEFER"
source_ref = "docs/08-chain/INDEX.md"
evidence_ref = "plans/architecture-core-queue/tasks.toml"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("PLAN_026"), "missing PLAN_026: {stdout}");
    assert!(
        stdout.contains("deferral.prerequisite_runtime_policy_gates"),
        "missing prerequisite field diagnostic: {stdout}"
    );
}

#[test]
fn plan_validate_accepts_complete_architecture_deferral_metadata() {
    let temp = TempDir::new().unwrap();
    // Add a second valid plan to exercise recursive multi-plan validation. Its
    // declared stub/lib.rs output is intentionally absent before execution.
    std::fs::create_dir_all(temp.path().join("plans/architecture-core-queue")).unwrap();
    std::fs::create_dir_all(temp.path().join("docs/08-chain")).unwrap();
    std::fs::write(temp.path().join("docs/08-chain/INDEX.md"), "# chain docs\n").unwrap();
    std::fs::write(
        temp.path().join("plans/architecture-core-queue/tasks.toml"),
        "[meta]\nplan = \"architecture-core-queue\"\n\n[[task]]\nid = \"S1\"\ntitle = \"Stub\"\nrole = \"implementer\"\nfiles = [\"stub/lib.rs\"]\ndepends_on = []\nverify = [{ phase = \"compile\", command = \"echo ok\" }]\n",
    )
    .unwrap();
    write_plan(
        temp.path(),
        "architecture",
        r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Deferred advanced packet"
role = "implementer"
files = ["plans/architecture-core-queue/tasks.toml"]
depends_on = []
acceptance = ["AC1: roko-cli compiles with the deferred packet recorded"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli", covers = ["AC1"] }]

[task.context]
read_files = [
  { path = "docs/08-chain/INDEX.md", why = "advanced source inventory" },
]

[task.deferral]
rationale = "Advanced surface must wait for trustworthy execution."
prerequisite_runtime_policy_gates = ["structured verdicts pass"]
acceptance_gates = ["cargo check -p roko-cli"]
risk_notes = ["Do not ship behavior before runtime gates are durable."]
parity_requirements = ["ARCH-DEFER"]

[task.acceptance_contract]
version = 1

[[task.acceptance_contract.gates]]
id = "compile"
kind = "compile"
command = "cargo check -p roko-cli"

[task.acceptance_contract.agent_output]
schema = "roko.architecture_packet.v1"

[task.acceptance_contract.review_verdict]
reviewer_role_id = "quick-reviewer"
min_confidence = 0.6

[task.acceptance_contract.recovery]
retry = true
reflection = true

[task.acceptance_contract.parity_ledger]

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "ARCH-DEFER"
source_ref = "docs/08-chain/INDEX.md"
evidence_ref = "plans/architecture-core-queue/tasks.toml"
"#,
    );

    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Both discovered plans validate, including the intentionally absent
    // output. The contract sits beside checked acceptance, so it draws no
    // PLAN_046 warning (bug-ac2a51).
    assert!(
        stdout.contains("0 diagnostics in 2 plans") && !stdout.contains("PLAN_046"),
        "unexpected stdout: {stdout}"
    );
}

/// bug-ac2a51: PLAN_024 asks an architecture-queue task for checked
/// acceptance (decision 3205), and PLAN_046 warns only when a contract is the
/// task's only acceptance, so a packet that keeps its contract for the parity
/// rows (PLAN_025) validates clean. A contract alone still fails PLAN_024 and
/// draws the PLAN_046 warning.
#[test]
fn architecture_queue_task_with_acceptance_criteria_has_no_contract_warning() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("tmp/architecture-plans")).unwrap();
    std::fs::write(
        temp.path()
            .join("tmp/architecture-plans/06-architecture-implementation.md"),
        "# source plan\n",
    )
    .unwrap();
    let plan = |acceptance: &str| {
        format!(
            r#"
[meta]
plan = "architecture"
queue_kind = "architecture_implementation"

[[task]]
id = "Q1"
title = "Implement one architecture packet"
role = "implementer"
files = ["crates/roko-gate/src/acceptance_contract.rs"]
depends_on = []
{acceptance}
[task.context]
read_files = [
  {{ path = "tmp/architecture-plans/06-architecture-implementation.md", why = "source plan" }},
]

[task.acceptance_contract]
version = 1
gates = [{{ id = "test", kind = "test", command = "cargo test -p roko-gate acceptance" }}]

[task.acceptance_contract.parity_ledger]

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "ARCH-Q1"
source_ref = "tmp/architecture-plans/06-architecture-implementation.md"
evidence_ref = "crates/roko-gate/src/acceptance_contract.rs"
"#
        )
    };

    write_plan(
        temp.path(),
        "architecture",
        &plan(
            r#"acceptance = ["AC1: the contract module's tests pass"]
verify = [{ phase = "test", command = "cargo test -p roko-gate acceptance", covers = ["AC1"] }]
"#,
        ),
    );
    let assert = run_validate(&temp, &["plans"]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for rule in ["PLAN_024", "PLAN_046"] {
        assert!(!stdout.contains(rule), "unexpected {rule}: {stdout}");
    }
    assert!(stdout.contains("0 diagnostics in 1 plan"), "{stdout}");

    write_plan(
        temp.path(),
        "architecture",
        &plan(
            r#"verify = [{ phase = "test", command = "cargo test -p roko-gate acceptance" }]
"#,
        ),
    );
    let assert = run_validate(&temp, &["plans"]).failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for rule in ["PLAN_024", "PLAN_046"] {
        assert!(stdout.contains(rule), "missing {rule}: {stdout}");
    }
}

#[test]
fn spec_quality_flag_reports_scores_and_hard_fails() {
    let temp = TempDir::new().unwrap();
    // T1's verify step can never fail (HF2); T2 runs a scoped test. Both pass plain validation.
    write_plan(
        temp.path(),
        "spec",
        r#"
[meta]
plan = "spec"

[[task]]
id = "T1"
title = "Retry limit"
description = "Add the retry limit to `parse_config`."
role = "implementer"
files = ["src/config.rs"]
max_loc = 40
depends_on = []
verify = [{ phase = "test", command = "echo ok" }]

[[task]]
id = "T2"
title = "Retry limit"
description = "Add the retry limit to `parse_config`."
role = "implementer"
files = ["src/retry.rs"]
max_loc = 40
depends_on = []
verify = [{ phase = "test", command = "cargo test -p fixture --lib config" }]
"#,
    );

    // Without the flag nothing changes, and --strict still passes.
    let plain = run_validate(&temp, &["plans", "--strict"]).success();
    let plain_stdout = String::from_utf8_lossy(&plain.get_output().stdout).into_owned();
    assert!(
        plain_stdout.contains("0 diagnostics in 1 plan"),
        "{plain_stdout}"
    );
    assert!(!plain_stdout.contains("spec quality"), "{plain_stdout}");
    let plain_json = run_validate(&temp, &["plans", "--json"]).success();
    let plain_json: serde_json::Value =
        serde_json::from_slice(&plain_json.get_output().stdout).unwrap();
    assert!(plain_json.get("spec_quality").is_none(), "{plain_json}");

    // With it, the text gains a section: per task the score, band, rules and hard fails.
    let scored = run_validate(&temp, &["plans", "--spec-quality"]).success();
    let stdout = String::from_utf8_lossy(&scored.get_output().stdout);
    assert!(stdout.starts_with(&plain_stdout), "{stdout}");
    assert!(
        stdout.contains("spec quality (sq-3, static: HF3 and SQ06 not evaluated)\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\nplans/spec/tasks.toml\n"), "{stdout}");
    assert!(
        stdout.contains(
            "  T1  21.25 D  SQ01=0 SQ02=0 SQ03=0 SQ04=0 SQ05=0 SQ06=0 SQ07=0 SQ08=1 SQ09=0 SQ10=1 \
             SQ11=1 SQ12=0  hard=HF2\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("HF2: step 1: the step only runs `echo ok`"),
        "{stdout}"
    );
    assert!(stdout.contains("  T2  46.25 C  SQ01=0"), "{stdout}");
    assert!(
        stdout.contains("2 tasks: 0 A, 0 B, 1 C, 1 D; 1 with hard fails"),
        "{stdout}"
    );

    // A hard fail exits 1 only under --strict.
    let strict = run_validate(&temp, &["plans", "--spec-quality", "--strict"]).failure();
    assert_eq!(strict.get_output().status.code(), Some(1));

    // JSON keeps the validation report and adds the per-task records.
    let json = run_validate(&temp, &["plans", "--spec-quality", "--json"]).success();
    let json: serde_json::Value = serde_json::from_slice(&json.get_output().stdout).unwrap();
    assert_eq!(json["plans"], plain_json["plans"]);
    assert_eq!(json["totals"], plain_json["totals"]);
    let spec = &json["spec_quality"];
    assert_eq!(spec["linter"], "sq-3");
    let tasks = spec["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    let t1 = &tasks[0];
    assert_eq!(t1["task_id"], "T1");
    assert_eq!(t1["plan_path"], "plans/spec/tasks.toml");
    assert_eq!(t1["score"], 21.25);
    assert_eq!(t1["band"], "D");
    assert_eq!(t1["hard_fail"], serde_json::json!(["HF2"]));
    assert_eq!(
        t1["hard_fail_detail"]["HF2"],
        serde_json::json!(["step 1: the step only runs `echo ok`"])
    );
    assert_eq!(t1["unknown"], serde_json::json!(["HF3", "SQ06"]));
    assert_eq!(t1["rules"].as_object().unwrap().len(), 12);
    assert_eq!(t1["rules"]["SQ04"], 0.0);
    assert_eq!(t1["verify_classes"], serde_json::json!(["vacuous"]));
    let t2 = &tasks[1];
    assert_eq!(t2["score"], 46.25);
    assert_eq!(t2["hard_fail"], serde_json::json!([]));
    assert_eq!(t2["rules"]["SQ05"], 1.0);
    assert_eq!(t2["verify_classes"], serde_json::json!(["test"]));
}

/// 3230 (decision 3205): the AcceptanceContract evaluator is retired, so
/// `plan validate` warns that a task's contract is not enforced (PLAN_046)
/// without failing the plan, and an archived plan with contracts still
/// parses.
#[test]
fn acceptance_contract_is_reported_as_not_enforced() {
    let temp = TempDir::new().unwrap();
    write_plan(
        temp.path(),
        "contract",
        r#"
[meta]
plan = "contract"

[[task]]
id = "T1"
title = "Implement the validator"
role = "implementer"
files = ["src/lib.rs"]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[task.acceptance_contract]
version = 1
gates = [{ id = "compile", kind = "compile", command = "cargo check -p roko-cli" }]
"#,
    );

    let assert = run_validate(&temp, &["plans", "--json"]).success();
    let report: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("JSON report");
    let warnings: Vec<&serde_json::Value> = report["plans"]
        .as_array()
        .expect("plans")
        .iter()
        .flat_map(|plan| plan["diagnostics"].as_array().expect("diagnostics"))
        .filter(|diagnostic| diagnostic["rule_id"] == "PLAN_046")
        .collect();
    assert_eq!(warnings.len(), 1, "{report}");
    assert_eq!(warnings[0]["severity"], "warning", "{report}");
    let message = warnings[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("not enforced at run time"), "{message}");

    let archived = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plans/archive/architecture-core-queue/tasks.toml");
    let plan = roko_cli::task_parser::TasksFile::parse(&archived).expect("parse the archived plan");
    assert!(
        plan.tasks
            .iter()
            .any(|task| task.acceptance_contract.is_some())
    );
}

/// bug-261c02: five tracked plans failed `roko plan validate` with staleness unrelated to PK14's
/// own work (which only noticed them). Three (two line ranges in
/// `portal-programme/08d-portal-legibility` and `08f-final-polish`, one symbol anchor in
/// `workspace-doctor-improvements`) named code that moved, shrank or was renamed, and are fixed in
/// place: `plan validate` passes against the real tree, since their `read_files` entries point at
/// files scattered across the repository, not a synthetic fixture. The other two
/// (`portal-plan-execution`, `wire-http-plan-execute`) named code and files that were never built
/// as the plans described -- a portal editor page with a `StatusLED` atom and `TaskEditorRow`
/// component that do not exist anywhere under `apps/portal/src`, and an `execute_plan` HTTP
/// handler already switched to the graph engine by a later, unrelated refactor -- and are marked
/// archived or done/superseded instead, per this item's own "or deliberately archived" allowance,
/// rather than patched with fabricated content.
#[test]
fn five_stale_plans_pass_plan_validate() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    for dir in [
        "plans/portal-programme/08d-portal-legibility",
        "plans/portal-programme/08f-final-polish",
        "plans/workspace-doctor-improvements",
    ] {
        let assert = Command::cargo_bin("roko")
            .unwrap()
            .isolated()
            .current_dir(&repo)
            .arg("plan")
            .arg("validate")
            .arg(dir)
            .assert();
        let output = assert.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{dir} should pass `roko plan validate`:\n{stdout}"
        );
    }

    for (old_dir, archived_dir, meta_status) in [
        (
            "plans/portal-plan-execution",
            "plans/archive/portal-plan-execution",
            "archived",
        ),
        (
            "plans/wire-http-plan-execute",
            "plans/archive/wire-http-plan-execute",
            "done",
        ),
    ] {
        assert!(
            !repo.join(old_dir).exists(),
            "{old_dir} should have moved to plans/archive/, not stayed live and unvalidatable"
        );
        let tasks_toml = fs::read_to_string(repo.join(archived_dir).join("tasks.toml"))
            .unwrap_or_else(|err| panic!("read {archived_dir}/tasks.toml: {err}"));
        let needle = format!("status = \"{meta_status}\"");
        assert!(
            tasks_toml.contains(&needle),
            "{archived_dir}/tasks.toml should set [meta] {needle}"
        );
    }
}

/// `roko plan run` with `args` in `temp`, with the mock agents first on
/// `PATH`, so a run that got past its checks could reach no real model.
fn run_plan(temp: &TempDir, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .arg("plan")
        .arg("run")
        .args(args)
        .assert()
}

/// The small Python project the specs epics' exit checks plan work in:
/// `load_settings` in `src/config.py`, and the planner-written tests the
/// plans' checks run, each red until its task writes the module it imports.
const RETRY_PROJECT: [(&str, &str); 4] = [
    (
        "src/config.py",
        r#"# The CLI's settings: `key = value` lines over the defaults.
DEFAULTS = {"retries": "3"}


def load_settings(text):
    settings = dict(DEFAULTS)
    for line in text.splitlines():
        key, sep, value = line.partition("=")
        if sep:
            settings[key.strip()] = value.strip()
    return settings
"#,
    ),
    (
        "tests/test_retry.py",
        r#"import unittest

from src.retry import parse_retry_limit


class ParseRetryLimitTest(unittest.TestCase):
    def test_returns_the_limit(self):
        self.assertEqual(parse_retry_limit(" 5 "), 5)

    def test_rejects_zero(self):
        with self.assertRaises(ValueError):
            parse_retry_limit("0")
"#,
    ),
    (
        "tests/test_backoff.py",
        r#"import unittest

from src.backoff import backoff_delay


class BackoffDelayTest(unittest.TestCase):
    def test_doubles_half_a_second_per_attempt(self):
        self.assertEqual(backoff_delay(3), 2.0)
"#,
    ),
    (
        "tests/test_config.py",
        r#"import unittest

from src.config import load_settings


class LoadSettingsTest(unittest.TestCase):
    def test_names_the_limit_retry_limit(self):
        self.assertEqual(load_settings("")["retry_limit"], "3")
"#,
    ),
];

fn write_retry_project(root: &Path) {
    for (path, text) in RETRY_PROJECT {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

/// Whether a run in `root` started an attempt: dispatching a task opens its
/// run's `attempts.jsonl`.
fn any_attempt_started(root: &Path) -> bool {
    fs::read_dir(root.join(".roko/runs"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|run| run.path().join("attempts.jsonl").exists())
}

/// The check of 3200's exact plan: the planner's tests of both criteria.
const RETRY_CHECK: &str = "python3 -m unittest tests.test_retry";

/// 3200's exact, test-backed plan with `check` as its one check: a goal, a
/// description that names the function and its file, observable criteria the
/// check covers, the files to read and why, a non-goal and a size bound.
fn exact_plan(plan: &str, check: &str) -> String {
    format!(
        r#"[meta]
plan = "{plan}"

[[task]]
id = "T1"
title = "Parse the retry limit"
goal = "`parse_retry_limit` turns the `retries` setting into a whole number of at least 1, so a bad value stops the CLI before its first request."
description = """Add `parse_retry_limit(text)` to src/retry.py. It strips the text, converts it with
`int()` and returns the number when it is 1 or more. For 0, a negative number or text that is not
a number it raises `ValueError` naming the value. The planner's tests in tests/test_retry.py check
both criteria."""
role = "implementer"
tier = "focused"
files = ["src/retry.py"]
max_loc = 20
depends_on = []
non_goals = ["Do not change `load_settings` or the defaults in src/config.py"]
acceptance = [
  'AC1: `parse_retry_limit(" 5 ")` returns 5',
  'AC2: `parse_retry_limit("0")` raises `ValueError`',
]

[task.context]
read_files = [
  {{ path = "src/config.py", why = "`load_settings` returns each setting as text" }},
  {{ path = "tests/test_retry.py", why = "the planner's tests of both criteria" }},
]

[[task.verify]]
phase = "test"
command = "{check}"
covers = ["AC1", "AC2"]
"#
    )
}

/// Backlog epic 3200's exit check: an exact, test-backed plan scores band B
/// or better and passes the spec gate `roko plan run` applies before any
/// dispatch, and `roko plan run` refuses its twin, whose one check ends in
/// `|| true` and so cannot fail (HF2), before any agent starts, naming the
/// rule. The gate runs in a dry run too, so the exact plan's dry run passes
/// the gate rather than skipping it.
#[test]
fn spec_gate_passes_an_exact_plan_and_refuses_its_vacuous_twin() {
    let temp = TempDir::new().unwrap();
    write_retry_project(temp.path());
    write_tasks(temp.path(), "exact", &exact_plan("exact", RETRY_CHECK));
    let vacuous = format!("{RETRY_CHECK} || true");
    write_tasks(temp.path(), "twin", &exact_plan("twin", &vacuous));

    // The exact plan validates clean, even under --strict, and scores band B
    // or better.
    let validated = run_validate(
        &temp,
        &["plans/exact", "--strict", "--spec-quality", "--json"],
    )
    .success();
    let report: serde_json::Value = serde_json::from_slice(&validated.get_output().stdout).unwrap();
    assert_eq!(report["totals"]["errors"], 0, "{report}");
    assert_eq!(report["totals"]["warnings"], 0, "{report}");
    let task = &report["spec_quality"]["tasks"][0];
    assert!(matches!(task["band"].as_str(), Some("A" | "B")), "{task}");
    assert_eq!(task["hard_fail"], serde_json::json!([]), "{task}");
    assert_eq!(
        task["verify_classes"],
        serde_json::json!(["test"]),
        "{task}"
    );

    // `plan run` lets it through the gate; the dry run stops before dispatch.
    let passed = run_plan(&temp, &["plans/exact", "--dry-run"]).success();
    let stdout = String::from_utf8_lossy(&passed.get_output().stdout);
    let stderr = String::from_utf8_lossy(&passed.get_output().stderr);
    assert!(
        stdout.contains("Dry run: 1 plan(s), 1 task(s)"),
        "{stdout}\n{stderr}"
    );
    assert!(!stderr.contains("spec gate"), "{stderr}");

    // It refuses the twin, in a dry run and in a real run, before any agent
    // starts.
    let runs: [&[&str]; 2] = [&["plans/twin", "--dry-run"], &["plans/twin"]];
    for args in runs {
        let refused = run_plan(&temp, args).failure();
        assert_eq!(refused.get_output().status.code(), Some(1), "{args:?}");
        let stderr = String::from_utf8_lossy(&refused.get_output().stderr);
        assert!(
            stderr.contains("spec gate: plan refused before dispatch"),
            "{args:?}: {stderr}"
        );
        assert!(
            stderr.contains("plans/twin/tasks.toml T1: HF2: step 1: ends in `|| true`"),
            "{args:?}: {stderr}"
        );
    }
    assert!(!any_attempt_started(temp.path()), "a twin's task started");
}

/// spec-e57870's four-task plan, with `t1_verify` as T1's verify steps and
/// T3's tier and dependencies as given: T1 parses the retry limit, T2 adds
/// the backoff delay, T3 renames the `retries` setting in four files, and T4
/// logs each retry, checked by a pinned acceptance test whose source is the
/// plan's `accept/test_log.py`.
fn four_task_plan(plan: &str, t1_verify: &str, t3_tier: &str, t3_depends_on: &str) -> String {
    format!(
        r#"[meta]
plan = "{plan}"

[[task]]
id = "T1"
title = "Parse the retry limit"
description = "Add `parse_retry_limit(text)` to src/retry.py: it returns the limit as an int and raises `ValueError` for 0."
role = "implementer"
files = ["src/retry.py"]
depends_on = []
verify = [{t1_verify}]

[[task]]
id = "T2"
title = "Add the backoff delay"
description = "Add `backoff_delay(attempt)` to src/backoff.py, half a second doubled per attempt, and list it in docs/settings.md."
role = "implementer"
files = ["src/backoff.py", "docs/settings.md"]
depends_on = []
verify = [{{ phase = "test", command = "python3 -m unittest tests.test_backoff" }}]

[[task]]
id = "T3"
title = "Rename the retries setting"
description = "Rename the `retries` setting to `retry_limit` in `load_settings`, src/cli.py, docs/settings.md and README.md."
role = "implementer"
tier = "{t3_tier}"
files = ["src/config.py", "src/cli.py", "docs/settings.md", "README.md"]
depends_on = {t3_depends_on}
verify = [{{ phase = "test", command = "python3 -m unittest tests.test_config" }}]

[[task]]
id = "T4"
title = "Log each retry"
description = "Add `log_retry(attempt, delay)` to src/log.py: `log_retry(2, 0.5)` returns `retry 2 after 0.5s`."
role = "implementer"
files = ["src/log.py", "tests/test_log.py"]
depends_on = []

[task.accept]
files = [
  {{ src = "accept/test_log.py", dest = "tests/test_log.py", runner = "python3 -m unittest {{dest}}", count = 2 }},
]
"#
    )
}

/// T4's pinned acceptance test, which only the fixed twin ships.
const TEST_LOG_PY: &str = r#"import unittest

from src.log import log_retry


class LogRetryTest(unittest.TestCase):
    def test_names_the_attempt_and_the_delay(self):
        self.assertEqual(log_retry(2, 0.5), "retry 2 after 0.5s")

    def test_counts_attempts_from_one(self):
        self.assertEqual(log_retry(1, 0.25), "retry 1 after 0.25s")
"#;

/// The rule, task (`-` for the plan) and message of each diagnostic in a
/// `plan validate --json` report, sorted.
fn diagnostics(report: &serde_json::Value) -> Vec<(&str, &str, &str)> {
    let mut found: Vec<(&str, &str, &str)> = report["plans"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|plan| plan["diagnostics"].as_array().unwrap())
        .map(|diagnostic| {
            (
                diagnostic["rule_id"].as_str().unwrap(),
                diagnostic["task_id"].as_str().unwrap_or("-"),
                diagnostic["message"].as_str().unwrap(),
            )
        })
        .collect();
    found.sort_unstable();
    found
}

/// spec-e57870's exit check: `plan validate --strict --spec-quality` rejects
/// a weak plan, with a code for each of its four defects, and passes its
/// fixed twin. The weak plan checks T1 with a grep alone, which a stub
/// passes: the spec-quality rules class the step `structural` and score its
/// verify strength (SQ04) 0.25, since find-70edcb's decision keeps such a
/// file-scoped step a score rather than a warning. T2 and T3 can run at the
/// same time and both write `docs/settings.md` (PLAN_CONCURRENT_OVERLAP); T3
/// is a mechanical task that writes four files, one more than its tier
/// allows (PLAN_TIER_SIZE); and the source of T4's pinned acceptance test is
/// missing from the plan (PLAN_038). The fixed twin runs T1's
/// planner-written test, has T3 wait for T2 at the focused tier, and ships
/// T4's test.
#[test]
fn spec_lints_reject_a_weak_plan_and_pass_its_fixed_twin() {
    let temp = TempDir::new().unwrap();
    write_retry_project(temp.path());
    let grep =
        r#"{ phase = "structural", command = "grep -q 'def parse_retry_limit' src/retry.py" }"#;
    let test = r#"{ phase = "test", command = "python3 -m unittest tests.test_retry" }"#;
    write_tasks(
        temp.path(),
        "weak",
        &four_task_plan("weak", grep, "mechanical", "[]"),
    );
    let fixed = write_tasks(
        temp.path(),
        "fixed",
        &four_task_plan("fixed", test, "focused", r#"["T2"]"#),
    );
    fs::create_dir_all(fixed.join("accept")).unwrap();
    fs::write(fixed.join("accept/test_log.py"), TEST_LOG_PY).unwrap();
    let args = |plan| [plan, "--strict", "--spec-quality", "--json"];

    let weak = run_validate(&temp, &args("plans/weak")).failure();
    assert_eq!(weak.get_output().status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&weak.get_output().stdout).unwrap();
    let found = diagnostics(&report);
    let codes: Vec<(&str, &str)> = found.iter().map(|(rule, task, _)| (*rule, *task)).collect();
    assert_eq!(
        codes,
        [
            ("PLAN_038", "T4"),
            ("PLAN_CONCURRENT_OVERLAP", "-"),
            ("PLAN_TIER_SIZE", "T3"),
        ],
        "{report}"
    );
    let messages = [
        "task 'T4' [task.accept] src `accept/test_log.py` is missing",
        "tasks T2 and T3 can run at the same time and both write `docs/settings.md`",
        "mechanical task is over its tier's size limits: 4 files (limit 3)",
    ];
    for ((rule, _, message), expected) in found.iter().zip(messages) {
        assert!(message.contains(expected), "{rule}: {message}");
    }
    let tasks = report["spec_quality"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 4, "{report}");
    assert_eq!(tasks[0]["task_id"], "T1");
    assert_eq!(
        tasks[0]["verify_classes"],
        serde_json::json!(["structural"])
    );
    assert_eq!(tasks[0]["rules"]["SQ04"], 0.25);
    for task in tasks {
        assert_eq!(task["hard_fail"], serde_json::json!([]), "{task}");
    }

    let fixed = run_validate(&temp, &args("plans/fixed")).success();
    let report: serde_json::Value = serde_json::from_slice(&fixed.get_output().stdout).unwrap();
    assert!(diagnostics(&report).is_empty(), "{report}");
    assert_eq!(report["totals"]["errors"], 0, "{report}");
    assert_eq!(report["totals"]["warnings"], 0, "{report}");
    let tasks = report["spec_quality"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 4, "{report}");
    assert_eq!(tasks[0]["verify_classes"], serde_json::json!(["test"]));
    assert_eq!(tasks[0]["rules"]["SQ04"], 1.0);
    for task in tasks {
        assert_eq!(task["hard_fail"], serde_json::json!([]), "{task}");
    }
}
