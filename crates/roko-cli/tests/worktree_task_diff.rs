#![cfg(unix)]

//! Proof case 2 (gap-415c54): a real `roko plan run --worktree-per-task`
//! whose agent edits a file in its task worktree. With no manual step, the
//! verified edit is committed on the plan branch, delivered into the run's
//! batch branch and promoted into the target, the run summary names where
//! it landed, and the attempt's worktree is removed while its branch stays
//! for inspection. An edit that fails its verify step is kept in its
//! worktree and merged nowhere.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;

/// A provider that writes `feature.txt` in its working directory, the
/// attempt's worktree, and reports success.
const WRITES_FEATURE: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf 'feature\n' > feature.txt
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"proof","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

/// Run git in `dir` and return its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A repository on `main` with `provider` as its only model, a one-task plan
/// `proof` whose task must pass `verify`, and a `release` branch for the run
/// to be promoted into. Returns the base commit.
fn seed_repo(repo: &Path, provider: &Path, verify: &str) -> String {
    fs::write(provider, WRITES_FEATURE).expect("write the provider");
    let mut permissions = fs::metadata(provider).expect("provider").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(provider, permissions).expect("make the provider executable");
    let config = format!(
        "[agent]\ndefault_model = \"fake\"\n\n[providers.fake]\nkind = \"claude_cli\"\n\
         command = {:?}\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n",
        provider.display().to_string()
    );
    let tasks = format!(
        "[meta]\nplan = \"proof\"\nmax_parallel = 1\nskip_enrichment = true\n\n\
         [[task]]\nid = \"T1\"\ntitle = \"Write feature.txt\"\n\
         description = \"Write feature.txt.\"\nrole = \"implementer\"\nstatus = \"ready\"\n\
         tier = \"focused\"\nfiles = [\"feature.txt\"]\nmax_retries = 0\n\n\
         [[task.verify]]\nphase = \"structural\"\ncommand = {verify:?}\n"
    );
    for (path, contents) in [
        ("roko.toml", config.as_str()),
        (".gitignore", ".roko/\n"),
        ("plans/proof/tasks.toml", tasks.as_str()),
    ] {
        let path = repo.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        fs::write(path, contents).expect("write fixture");
    }
    git(repo, &["init", "--quiet", "--initial-branch=main"]);
    for (key, value) in [
        ("user.name", "Operator"),
        ("user.email", "operator@example.test"),
        ("commit.gpgsign", "false"),
    ] {
        git(repo, &["config", key, value]);
    }
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", "fixture"]);
    git(repo, &["branch", "release"]);
    git(repo, &["rev-parse", "HEAD"])
}

/// The operator's checkout: its HEAD, its index and its status, leaving out
/// `plans/INDEX.md`, which roko rewrites after every plan command.
fn operator_state(repo: &Path) -> [String; 3] {
    let status = git(repo, &["status", "--porcelain", "--untracked-files=all"])
        .lines()
        .filter(|line| !line.ends_with(" plans/INDEX.md"))
        .collect::<Vec<_>>()
        .join("\n");
    [
        git(repo, &["rev-parse", "HEAD"]),
        git(repo, &["ls-files", "--stage"]),
        status,
    ]
}

/// `roko --json plan run plans/proof --worktree-per-task --promote release`
/// in `repo`.
fn run_plan(repo: &Path, target: &Path) -> Output {
    Command::new(cargo_bin("roko"))
        .current_dir(repo)
        .args([
            "--json",
            "plan",
            "run",
            "plans/proof",
            "--worktree-per-task",
        ])
        .args(["--promote", "release", "--workdir"])
        .arg(repo)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("run roko")
}

/// The JSON run summary: the last stdout line that opens an object, through
/// the end of stdout.
fn summary(output: &Output, log: &str) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let start = stdout.rfind("\n{").map_or(0, |index| index + 1);
    serde_json::from_str(&stdout[start..]).unwrap_or_else(|error| panic!("{error}: {log}"))
}

/// The paths of the repository's worktrees, the operator's first.
fn worktrees(repo: &Path) -> Vec<String> {
    git(repo, &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(str::to_string)
        .collect()
}

/// What roko logged in `repo`: its stderr is quiet unless asked, so warnings
/// go to the daily `.roko/roko.log.<date>` files.
fn roko_logs(repo: &Path) -> String {
    fs::read_dir(repo.join(".roko"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("roko.log"))
                .filter_map(|entry| fs::read_to_string(entry.path()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The branches under `prefix`, such as `refs/heads/roko/attempt/`.
fn branches(repo: &Path, prefix: &str) -> Vec<String> {
    git(repo, &["for-each-ref", "--format=%(refname:short)", prefix])
        .lines()
        .map(str::to_string)
        .collect()
}

/// gap-415c54: the agent's edit passes its verify step, is committed on the
/// plan branch, delivered into the batch branch and promoted into `release`,
/// and the summary and checkpoint name the commits. The attempt's worktree
/// is gone afterwards, its branch is kept (`[runner]
/// delete_attempt_branches` is off by default), and the operator's checkout
/// never changed.
#[test]
fn worktree_task_diff_is_gated_merged_and_cleaned() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let base = seed_repo(
        &repo,
        &temp.path().join("provider.sh"),
        "test -f feature.txt",
    );
    let before = operator_state(&repo);

    let output = run_plan(&repo, &temp.path().join("target"));
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{log}");
    let report = summary(&output, &log);
    assert_eq!(report["succeeded"], true, "{report:#}");

    // Gated and committed: one commit on the plan branch, holding the edit.
    let plan_tip = git(&repo, &["rev-parse", "roko/plan/proof"]);
    assert_eq!(
        git(&repo, &["rev-list", &format!("{base}..{plan_tip}")]),
        plan_tip,
        "{log}"
    );
    assert_eq!(
        git(&repo, &["show", "--name-only", "--format=", &plan_tip]),
        "feature.txt"
    );
    assert_eq!(
        git(&repo, &["show", &format!("{plan_tip}:feature.txt")]),
        "feature"
    );
    let activities = fs::read_to_string(repo.join(".roko/state/graph/proof/activities.jsonl"))
        .expect("activity log");
    assert!(
        activities.contains(&format!("\"workspace.accepted_commit\":\"{plan_tip}\"")),
        "the checkpoint does not name {plan_tip}"
    );

    // Merged: delivered into the batch branch, then promoted into `release`,
    // and the summary names the commits.
    let batches = branches(&repo, "refs/heads/roko/batch/");
    assert_eq!(batches.len(), 1, "{batches:?}");
    let batch = &batches[0];
    let batch_tip = git(&repo, &["rev-parse", batch]);
    assert_eq!(
        git(&repo, &["show", &format!("{batch_tip}:feature.txt")]),
        "feature"
    );
    let delivery = &report["batch"]["deliveries"][0];
    assert_eq!(report["batch"]["branch"], batch.as_str(), "{report:#}");
    assert_eq!(delivery["plan_id"], "proof", "{report:#}");
    assert_eq!(delivery["state"], "delivered", "{report:#}");
    assert_eq!(delivery["merge_commit"], batch_tip.as_str(), "{report:#}");
    let release = git(&repo, &["rev-parse", "release"]);
    assert_eq!(release, batch_tip, "{log}");
    let promotion = &report["batch"]["promotion"];
    assert_eq!(promotion["target"], "release", "{report:#}");
    assert_eq!(promotion["moved"], true, "{report:#}");
    assert_eq!(promotion["commit"], release.as_str(), "{report:#}");

    // Cleaned: no attempt worktree is left, and the attempt's branch, still
    // at the commit the plan branch took, stays and is named in the summary.
    assert_eq!(worktrees(&repo).len(), 1, "{:?}", worktrees(&repo));
    let attempt_branches = branches(&repo, "refs/heads/roko/attempt/");
    assert_eq!(attempt_branches.len(), 1, "{attempt_branches:?}");
    assert_eq!(git(&repo, &["rev-parse", &attempt_branches[0]]), plan_tip);
    assert_eq!(
        delivery["attempt_cleanup"]["kept_branches"],
        serde_json::json!(attempt_branches),
        "{report:#}"
    );
    assert_eq!(
        delivery["attempt_cleanup"]["removed_checkouts"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "{report:#}"
    );
    let checkouts = fs::read_dir(repo.join(".roko/worktrees"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("attempt-"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(checkouts.is_empty(), "{checkouts:?}");
    let logs = roko_logs(&repo);
    for warning in [
        "kept a delivered attempt's checkout",
        "kept the delivered plan's attempt checkouts",
        "could not keep the attempt's worktree",
    ] {
        assert!(!logs.contains(warning), "{warning}:\n{logs}");
    }

    // The operator's checkout never changed.
    assert_eq!(operator_state(&repo), before);
}

/// gap-415c54: an edit that fails its verify step stays in its worktree, on
/// its attempt branch, for post-mortem, and lands nowhere: no plan branch,
/// an unmoved batch branch and an unmoved `release`.
#[test]
fn worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let base = seed_repo(
        &repo,
        &temp.path().join("provider.sh"),
        "grep -q missing feature.txt",
    );
    let before = operator_state(&repo);

    let output = run_plan(&repo, &temp.path().join("target"));
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(1), "{log}");
    let report = summary(&output, &log);
    assert_eq!(report["succeeded"], false, "{report:#}");
    assert_eq!(
        report["batch"]["deliveries"],
        serde_json::json!([]),
        "{report:#}"
    );

    // Nothing landed anywhere.
    assert_eq!(
        branches(&repo, "refs/heads/roko/plan/"),
        Vec::<String>::new()
    );
    let batches = branches(&repo, "refs/heads/roko/batch/");
    assert_eq!(batches.len(), 1, "{batches:?}");
    assert_eq!(git(&repo, &["rev-parse", &batches[0]]), base);
    assert_eq!(git(&repo, &["rev-parse", "release"]), base);

    // The attempt's worktree is kept, with the edit, on its attempt branch.
    let checkouts = worktrees(&repo);
    assert_eq!(checkouts.len(), 2, "{checkouts:?}");
    let kept = Path::new(&checkouts[1]);
    assert_eq!(
        fs::read_to_string(kept.join("feature.txt")).expect("the edit is kept"),
        "feature\n"
    );
    assert_eq!(branches(&repo, "refs/heads/roko/attempt/").len(), 1);

    assert_eq!(operator_state(&repo), before);
}
