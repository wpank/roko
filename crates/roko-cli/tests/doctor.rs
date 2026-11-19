//! Integration tests for `roko doctor`.

mod common;
use common::Isolated;

use assert_cmd::Command;
use predicates::str::contains;
use roko_fs::{LayoutVersion, RokoLayout};
use tempfile::tempdir;

fn bootstrap_layout(workdir: &std::path::Path) {
    let layout = RokoLayout::for_project(workdir);
    std::fs::create_dir_all(layout.root()).unwrap();
    for dir in layout.top_level_dirs() {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(
        layout.version_file(),
        format!("{}\n", LayoutVersion::CURRENT.as_u32()),
    )
    .unwrap();
}

#[test]
fn doctor_json_exits_nonzero_when_workspace_is_not_bootstrapped() {
    let temp = tempdir().unwrap();

    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .args(["doctor", "--workdir"])
        .arg(temp.path())
        .arg("--json")
        .assert()
        .failure()
        .stdout(contains("\"healthy\": false"))
        .stdout(contains("\"id\": \"config\""))
        .stdout(contains("\"id\": \"layout\""));
}

#[test]
fn doctor_human_output_succeeds_for_bootstrapped_workspace() {
    let temp = tempdir().unwrap();
    std::fs::write(
        temp.path().join("roko.toml"),
        "[agent]\ncommand = \"echo\"\n\n[serve.auth]\nenabled = false\n",
    )
    .unwrap();
    bootstrap_layout(temp.path());

    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .args(["doctor", "--workdir"])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(contains("doctor: ok"))
        .stdout(contains("[ok] workdir"))
        .stdout(contains("[ok] layout"))
        .stdout(contains("[skipped] serve_health"));
}

#[test]
fn doctor_network_json_produces_summary_and_checks() {
    let temp = tempdir().unwrap();
    std::fs::write(
        temp.path().join("roko.toml"),
        "[agent]\ncommand = \"echo\"\n",
    )
    .unwrap();
    bootstrap_layout(temp.path());

    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .args(["doctor", "network", "--workdir"])
        .arg(temp.path())
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"summary\""))
        .stdout(contains("\"checks\""));
}

#[test]
fn doctor_disk_fix_reports_leftover_checkouts() {
    let temp = tempdir().unwrap();
    std::fs::write(
        temp.path().join("roko.toml"),
        "[agent]\ncommand = \"echo\"\n",
    )
    .unwrap();
    bootstrap_layout(temp.path());

    // The exit code follows the disk report, which depends on the machine.
    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .args(["doctor", "disk", "--fix", "--workdir"])
        .arg(temp.path())
        .assert()
        .stdout(contains("leftover attempt checkouts: 0 removed, 0 kept"))
        .stdout(contains("doctor disk"));
}

#[test]
fn doctor_fix_is_refused_outside_disk() {
    let temp = tempdir().unwrap();

    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(temp.path())
        .args(["doctor", "network", "--fix", "--workdir"])
        .arg(temp.path())
        .assert()
        .failure()
        .stderr(contains("--fix applies only to `roko doctor disk`"));
}

/// A scratch git repository, as `roko init` finds a project.
fn git_repo() -> tempfile::TempDir {
    let temp = tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(temp.path())
        .status()
        .expect("run git");
    assert!(status.success(), "git init");
    temp
}

/// `roko init` of the workspace at `root`, isolated, with no keys.
fn init(root: &std::path::Path) {
    Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(root)
        .arg("init")
        .arg(root)
        .assert()
        .success();
}

/// What `roko doctor --json` says of check `id` in the workspace at `root`.
fn doctor_status(root: &std::path::Path, id: &str) -> String {
    let output = Command::cargo_bin("roko")
        .unwrap()
        .isolated()
        .with_mock_agents()
        .current_dir(root)
        .args(["doctor", "--workdir"])
        .arg(root)
        .arg("--json")
        .output()
        .expect("run roko doctor");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor --json prints JSON");
    report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == id))
        .and_then(|check| check["status"].as_str())
        .unwrap_or_else(|| panic!("no {id} check: {report}"))
        .to_string()
}

/// bug-a1c99a: a fresh `roko init` leaves no legacy `engrams.jsonl` and no
/// `.roko/plans/`, so once the quick start puts a plan in `plans/`, `roko
/// doctor` passes its legacy-file and plans-directory checks.
#[test]
fn fresh_init_leaves_no_legacy_files() {
    let repo = git_repo();
    let root = repo.path();

    init(root);

    assert!(
        !root.join(".roko/engrams.jsonl").exists(),
        "init wrote the legacy signal log"
    );
    assert!(!root.join(".roko/plans").exists(), "init made .roko/plans/");
    std::fs::create_dir_all(root.join("plans/hello")).unwrap();
    std::fs::write(
        root.join("plans/hello/tasks.toml"),
        "[meta]\nplan = \"hello\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Say hello\"\nfiles = [\"hello.txt\"]\n\
         verify = [{ phase = \"check\", command = \"test -f hello.txt\" }]\n",
    )
    .unwrap();
    assert_eq!(doctor_status(root, "state_legacy_files"), "ok");
    assert_eq!(doctor_status(root, "plans_dir_conflict"), "ok");
}

/// A signal for the log, its body `n`.
fn signal(n: u32) -> roko_core::Signal {
    roko_core::Signal::builder(roko_core::Kind::Task)
        .body(roko_core::Body::text(format!("signal {n}")))
        .created_at_ms(i64::from(n))
        .build()
}

/// bug-a1c99a: `roko init` in a workspace whose signal log is
/// `signals.jsonl` leaves the log there. It used to move the rows into
/// `engrams.jsonl`, which the log replays only while `signals.jsonl` is
/// absent, so the first write after it hid them all.
#[tokio::test]
async fn reinit_keeps_the_signal_log() {
    use roko_core::Store as _;
    use roko_fs::file_substrate::FileSubstrate;

    let repo = git_repo();
    let root = repo.path();
    let log = FileSubstrate::open(root.join(".roko")).await.unwrap();
    for n in 0..3 {
        log.put(signal(n)).await.unwrap();
    }
    drop(log);

    init(root);

    assert!(
        root.join(".roko/signals.jsonl").is_file(),
        "init moved the signal log"
    );
    assert!(!root.join(".roko/engrams.jsonl").exists());
    let log = FileSubstrate::open(root.join(".roko")).await.unwrap();
    assert_eq!(log.len().await.unwrap(), 3);
    log.put(signal(3)).await.unwrap();
    drop(log);
    let log = FileSubstrate::open(root.join(".roko")).await.unwrap();
    assert_eq!(
        log.len().await.unwrap(),
        4,
        "the signals before init are gone after the first write"
    );
}

/// bug-a1c99a: `roko init` moves a legacy `engrams.jsonl` into
/// `signals.jsonl`, rows the log gained since kept after it, so a workspace
/// an earlier init split gets its whole history back, and `roko doctor`'s
/// legacy-file check passes.
#[tokio::test]
async fn init_moves_a_legacy_signal_log_into_signals_jsonl() {
    use roko_core::Store as _;
    use roko_fs::file_substrate::FileSubstrate;

    let repo = git_repo();
    let root = repo.path();
    let state = root.join(".roko");
    // Two signals in the legacy log, as an earlier init left them, then one
    // written since, which went to signals.jsonl.
    let log = FileSubstrate::open(&state).await.unwrap();
    for n in 0..2 {
        log.put(signal(n)).await.unwrap();
    }
    drop(log);
    std::fs::rename(state.join("signals.jsonl"), state.join("engrams.jsonl")).unwrap();
    let log = FileSubstrate::open(&state).await.unwrap();
    assert_eq!(log.len().await.unwrap(), 2, "the log reads the legacy file");
    log.put(signal(2)).await.unwrap();
    drop(log);
    assert_eq!(
        FileSubstrate::open(&state)
            .await
            .unwrap()
            .len()
            .await
            .unwrap(),
        1,
        "the split: the log no longer reads the legacy file"
    );

    init(root);

    assert!(
        !state.join("engrams.jsonl").exists(),
        "the legacy log stayed"
    );
    assert_eq!(
        FileSubstrate::open(&state)
            .await
            .unwrap()
            .len()
            .await
            .unwrap(),
        3,
        "the log holds every signal again"
    );
    assert_eq!(doctor_status(root, "state_legacy_files"), "ok");
}
