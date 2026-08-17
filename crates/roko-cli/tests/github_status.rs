//! Integration tests for the standalone `roko github status` command.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn github_status_without_token_still_reports_local_configuration() {
    let temp = tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("roko.toml"),
        r#"
[github]
owner = "octo"
repo = "roko"
auto_pr = true
merge_method = "squash"
label_prefix = "automation/"
"#,
    )
    .expect("write config");

    Command::cargo_bin("roko")
        .expect("roko binary")
        .args(["github", "status", "--workdir"])
        .arg(temp.path())
        .env_remove("GITHUB_TOKEN")
        .assert()
        .success()
        .stdout(contains("Repository: octo/roko"))
        .stdout(contains("Auto PR: enabled"))
        .stdout(contains("Merge method: squash"))
        .stdout(contains("Token: missing"))
        .stdout(contains("automation/task-failure"));
}

#[test]
fn github_status_json_without_token_is_structured_and_successful() {
    let temp = tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("roko.toml"),
        "[github]\nowner = \"octo\"\nrepo = \"roko\"\n",
    )
    .expect("write config");

    Command::cargo_bin("roko")
        .expect("roko binary")
        .args(["--json", "github", "status", "--workdir"])
        .arg(temp.path())
        .env_remove("GITHUB_TOKEN")
        .assert()
        .success()
        .stdout(contains("\"token\": \"missing\""))
        .stdout(contains("\"merge_method\": \"squash\""))
        .stdout(contains("\"state\": \"skipped\""));
}
