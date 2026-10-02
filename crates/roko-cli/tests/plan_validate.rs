//! Integration coverage for `roko plan validate`.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_plan(root: &Path, plan_id: &str, tasks_toml: &str) -> PathBuf {
    let plan_dir = root.join("plans").join(plan_id);
    fs::create_dir_all(&plan_dir).unwrap();
    fs::write(plan_dir.join("tasks.toml"), tasks_toml).unwrap();
    plan_dir
}

fn run_validate(temp: &TempDir, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("roko")
        .unwrap()
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
    assert!(
        stdout.contains("3 diagnostics in 1 plan"),
        "unexpected stdout: {stdout}"
    );
    assert!(stdout.contains("PLAN_012"), "missing PLAN_012: {stdout}");
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
    assert!(
        stdout.contains("1 diagnostics in 1 plan"),
        "unexpected stdout: {stdout}"
    );
    assert!(stdout.contains("PLAN_009"), "missing PLAN_009: {stdout}");
    assert!(
        stdout.contains("uses model 'definitely-not-a-model' which is not configured in roko.toml"),
        "missing original mystery-model warning: {stdout}"
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
    assert!(
        stdout.contains("0 diagnostics in 1 plan"),
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
verify = [{ phase = "compile", command = "cargo check -p roko-core" }]

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
    assert!(
        stdout.contains("0 diagnostics in 1 plan"),
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
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

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
    // Both discovered plans validate, including the intentionally absent output.
    assert!(
        stdout.contains("0 diagnostics in 2 plans"),
        "unexpected stdout: {stdout}"
    );
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
        stdout.contains("spec quality (sq-2, static: HF3 and SQ06 not evaluated)\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\nplans/spec/tasks.toml\n"), "{stdout}");
    assert!(
        stdout.contains(
            "  T1  17.00 D  SQ01=0 SQ02=0 SQ03=0 SQ04=0 SQ05=0 SQ06=0 SQ07=0 SQ08=1 SQ09=0 SQ10=1 \
             SQ11=1 SQ12=0  hard=HF2\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("HF2: step 1: the step only runs `echo ok`"),
        "{stdout}"
    );
    assert!(stdout.contains("  T2  37.00 D  SQ01=0"), "{stdout}");
    assert!(
        stdout.contains("2 tasks: 0 A, 0 B, 0 C, 2 D; 1 with hard fails"),
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
    assert_eq!(spec["linter"], "sq-2");
    let tasks = spec["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    let t1 = &tasks[0];
    assert_eq!(t1["task_id"], "T1");
    assert_eq!(t1["plan_path"], "plans/spec/tasks.toml");
    assert_eq!(t1["score"], 17.0);
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
    assert_eq!(t2["score"], 37.0);
    assert_eq!(t2["hard_fail"], serde_json::json!([]));
    assert_eq!(t2["rules"]["SQ05"], 1.0);
    assert_eq!(t2["verify_classes"], serde_json::json!(["test"]));
}
