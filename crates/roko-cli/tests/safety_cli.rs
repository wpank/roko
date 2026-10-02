//! End-to-end coverage for `roko safety controls` and `roko safety release`.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn roko(workdir: &Path) -> Command {
    let mut command = Command::cargo_bin("roko").expect("roko binary");
    command.current_dir(workdir);
    command
}

/// backlog 1105: an operator sees an immune isolation and lifts it in one
/// command, and the release leaves an audit line.
#[test]
fn safety_release_clears_isolation_control() {
    let workdir = TempDir::new().expect("tempdir");
    roko_agent::isolate_agent(workdir.path(), "live-a/cli", "operator_isolation")
        .expect("seed an isolation control");

    let listed = predicate::str::contains("live-a/cli")
        .and(predicate::str::contains("reason=operator_isolation"));
    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(listed);

    roko(workdir.path())
        .args([
            "safety",
            "release",
            "live-a/cli",
            "--reason",
            "blank answer, not tamper",
            "--by",
            "release-test",
            "--workdir",
        ])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("released isolation control"));

    let controls = roko_agent::list_agent_controls(workdir.path()).expect("read controls");
    assert!(controls.is_empty(), "{controls:?}");
    let audit_path = roko_agent::agent_control_releases_path(workdir.path());
    let audit = std::fs::read_to_string(&audit_path).expect("audit file");
    let lines = audit.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{audit}");
    let record: roko_agent::ReleasedControl = serde_json::from_str(lines[0]).expect("audit line");
    assert_eq!(record.agent_id, "live-a/cli");
    assert_eq!(record.by, "release-test");
    assert_eq!(record.reason, "blank answer, not tamper");

    // Nothing is left to release: the command says so and fails.
    roko(workdir.path())
        .args(["safety", "release", "live-a/cli", "--reason", "again"])
        .arg("--workdir")
        .arg(workdir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no isolation control"));
    let audit_after = std::fs::read_to_string(&audit_path).expect("audit file");
    assert_eq!(audit_after, audit, "a failed release writes no audit line");
}

#[test]
fn safety_controls_on_an_empty_workspace_says_so() {
    let workdir = TempDir::new().expect("tempdir");

    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no isolation controls"));
}
