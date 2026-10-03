#![cfg(unix)]

//! Canary drill (S05 §7.5, SC4; backlog 7130): a hidden suite whose canary
//! reaches an attempt's composed prompt is burned, and a replacement audits
//! the task's next unit.
//!
//! The vault holds an active suite for task T1, written by a GLM model for a
//! Claude implementer. T1's context points its agent at a fixture file and
//! quotes the canary the file holds, so the canary reaches T1's prompt.
//! Running the plan logs `audit.leak_canary` at the prompt and moves the
//! suite to exposed, then retired. T1's attempt is green and, with no verify
//! steps, drawn at π = 1; phase B always runs (`[audit] phase_b_rate = 1`).
//! B1 then asks the configured GLM model, which the scripted provider's
//! default turn plays, for a replacement, validates it on the base tree, and
//! audits T1's unit with it.

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use roko_core::audit_home::AuditVault;
use roko_gate::audit::hidden::{HiddenStore, SuiteDraft, SuiteState};
use roko_gate::audit::ledger::{AuditEvent, AuditLedger, records, verify_chain};

/// The suite the GLM model writes: `src/answer.sh` prints 42.
const SUITE: &str = "[ \"$(sh src/answer.sh)\" = 42 ]";

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=drill",
            "-c",
            "user.email=drill@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// The plan: T1 makes `src/answer.sh` print 42, has no verify steps, and
/// reads `notes/context.md`, whose quoted line holds `canary`.
fn plan(repo: &Path, canary: &str) {
    let dir = repo.join("plans/drill");
    write(&dir.join("plan.md"), "# Plan: drill\n\nThe canary drill.\n");
    write(
        &dir.join("tasks.toml"),
        &format!(
            r#"[meta]
plan = "drill"
iteration = 1
total = 1
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Make answer print 42"
description = "Change src/answer.sh so that it prints 42."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "drill-model"
files = ["src/answer.sh"]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = []
depends_on_plan = []
acceptance = []
verify = []
timeout_secs = 30
max_retries = 0

[task.context]
read_files = [{{ path = "notes/context.md", why = "the release note quotes {canary}" }}]
"#
        ),
    );
}

#[test]
fn a_planted_canary_retires_its_suite_and_a_replacement_audits_the_next_unit() {
    let temp = tempfile::tempdir().expect("tempdir");
    // Canonical, so the vault the test opens is the one roko opens.
    let root = temp.path().canonicalize().expect("canonical tempdir");
    let repo = root.join("repo");
    let vault_home = root.join("vault");
    fs::create_dir_all(root.join("home")).expect("home dir");
    write(&repo.join("src/answer.sh"), "echo 41\n");

    // 1. An active suite for T1, by another family than its implementer's.
    let vault = AuditVault::resolve_with(&repo, Some(&vault_home), None).expect("the vault");
    let store = HiddenStore::open(&vault).expect("the hidden store");
    let mut ledger = AuditLedger::open(&vault).expect("the ledger");
    let draft = SuiteDraft {
        task_id: "T1".to_string(),
        spec_hash: "sha256:drill".to_string(),
        author_model: "glm-4.6".to_string(),
        author_family: "zhipu".to_string(),
        implementer_family: "anthropic".to_string(),
        file_name: "suite.sh".to_string(),
        comment: "#".to_string(),
        body: format!("{SUITE}\n"),
    };
    let leaked = store.draft(&mut ledger, draft).expect("a drafted suite");
    for to in [SuiteState::Validated, SuiteState::Active] {
        store
            .transition(&mut ledger, &leaked.suite_id, to, "drill")
            .expect("a move");
    }
    drop(ledger);

    // 2. Its canary reaches T1's prompt through a fixture context file.
    write(
        &repo.join("notes/context.md"),
        &format!("# Release note\n\n{}\n", leaked.canary),
    );
    plan(&repo, &leaked.canary);
    let script = Script::new()
        .task(
            "T1",
            [Turn::reply()
                .write("src/answer.sh", "echo 42\n")
                .text("src/answer.sh prints 42.")
                .usage(100, 50, 1.0)],
        )
        .otherwise(Turn::reply().text(&format!("```sh\n{SUITE}\n```")));
    let provider = ScriptedProvider::install(&root.join("provider"), &script);
    let agent = provider.command().display().to_string();
    write(&repo.join(".gitignore"), ".roko/\n");
    write(
        &repo.join("roko.toml"),
        &format!(
            r#"[agent]
default_model = "drill-model"
command = {agent:?}
bare_mode = false

[providers.drill-cli]
kind = "claude_cli"
command = {agent:?}

[models.drill-model]
provider = "drill-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[models.author-model]
provider = "drill-cli"
slug = "glm-4.6"
context_window = 200000

[gates]
cargo_fix_enabled = false

[runner]
worktree_per_task = false

[audit]
enabled = true
home = {vault_home:?}
phase_b_rate = 1.0
"#,
            vault_home = vault_home.display().to_string(),
        ),
    );
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["add", "--all"]);
    git(&repo, &["commit", "--quiet", "-m", "seed"]);

    // 3. The run logs the leak at the prompt and burns the suite.
    let workdir = repo.display().to_string();
    let output = Command::new(cargo_bin("roko"))
        .current_dir(&repo)
        .args(["--json", "plan", "run", "plans/drill", "--engine", "graph"])
        .args(["--workdir", workdir.as_str()])
        .env("HOME", root.join("home"))
        .env("ROKO_AUDIT_HOME", &vault_home)
        .env("ROKO_LOG", "roko=warn")
        .env_remove("RUST_LOG")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .expect("run roko");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the plan runs: {stderr}");
    let prompts: Vec<String> = provider
        .calls_for("T1")
        .into_iter()
        .map(|call| call.prompt)
        .collect();
    assert!(
        prompts.iter().any(|prompt| prompt.contains(&leaked.canary)),
        "the canary reached T1's prompt: {prompts:?}"
    );
    let ledger_dir = vault.ledger_dir();
    verify_chain(&ledger_dir).expect("an unbroken ledger");
    let all = records(&ledger_dir).expect("the ledger");
    let leaks: Vec<(&str, &str)> = all
        .iter()
        .filter_map(|record| match &record.event {
            AuditEvent::LeakCanary { place, suite_id } => Some((place.as_str(), suite_id.as_str())),
            _ => None,
        })
        .collect();
    assert!(
        leaks.contains(&("prompt", leaked.suite_id.as_str())),
        "{leaks:?}"
    );
    let moves: Vec<&str> = all
        .iter()
        .filter_map(|record| match &record.event {
            AuditEvent::HiddenSuite { suite_id, to, .. } if *suite_id == leaked.suite_id => {
                Some(to.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        ["drafted", "validated", "active", "exposed", "retired"]
    );
    let burned = store.meta(&leaked.suite_id).expect("its meta");
    assert_eq!(burned.state, SuiteState::Retired);

    // 4. A replacement by the other family audits T1's unit.
    let replacement = store
        .active_for("T1")
        .expect("the store")
        .expect("a replacement suite");
    assert_ne!(replacement.suite_id, leaked.suite_id);
    assert_eq!(replacement.author_family, "zhipu");
    assert_eq!(replacement.implementer_family, "anthropic");
    let selected: Vec<&str> = all
        .iter()
        .filter_map(|record| match &record.event {
            AuditEvent::Selection {
                sel_id,
                task_id,
                selected: true,
                ..
            } if task_id == "T1" => Some(sel_id.as_str()),
            _ => None,
        })
        .collect();
    let (checks, labels) = all
        .iter()
        .find_map(|record| match &record.event {
            AuditEvent::Result {
                sel_id,
                checks,
                labels,
                ..
            } if selected.contains(&sel_id.as_str()) => Some((checks, labels)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("an audit of T1's unit: {all:#?}"));
    assert_eq!(
        checks["b1"]["suite_id"],
        replacement.suite_id.as_str(),
        "{checks}"
    );
    assert_eq!(labels.y, Some(false), "the answer is 42: {checks}");
}
