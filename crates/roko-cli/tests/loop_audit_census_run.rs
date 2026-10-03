#![cfg(unix)]

//! S03 §7 A3 (backlog 5131): a fixture run logs each chain's arms before
//! `plan()`, and the measured census sees the loops' exposure.
//!
//! `fixture_run_logs_arms_before_plan_and_measures_exposure` runs a plan of
//! [`CHAINS`] one-task chains through the real `roko` binary with a scripted
//! provider, in a workspace whose one knowledge entry and one playbook every
//! task's prompt retrieves, and then reads the run's files alone:
//! - the knowledge layer splits the chains near 80/20 (h = 0.2 on probation,
//!   g = 0.03 all-off), with an SRM e-value below 20;
//! - every decision row with an arm was assigned before it was decided;
//! - every knowledge and playbook row that included an item carries a receipt
//!   with the rendered sections' hashes, and every verdict names the model
//!   the provider reported;
//! - the measured census (5123) gives L-know and L-play exposure above 0.

use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin;
use roko_learn::loop_audit::census::measure;
use roko_learn::telemetry::records::AuditFields;
use roko_learn::telemetry::report::{RunRecords, srm_check};
use roko_learn::telemetry::{Arm, ContentDecisionPoint};

/// The chains of the fixture plan.
const CHAINS: usize = 200;

/// The stand-in `claude_cli` provider: it ignores its prompt and reports a
/// finished free turn on `claude-sonnet-4-6`, so each task's verify step
/// alone decides its outcome.
const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"census","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":3,"output_tokens":2},"is_error":false}'
"#;

/// The knowledge entry every task's prompt retrieves: it holds four of their
/// topic words.
const KNOWLEDGE_ENTRY: &str = "Check the verify step before the chain finishes";

/// A workspace whose one model runs on [`PROVIDER`], with the ladder off and
/// the domain context, which carries knowledge and playbooks, pinned, so the
/// section bandit never leaves it out. Nothing forces an arm: every chain
/// draws its own.
fn write_workspace(workdir: &Path) {
    let provider = workdir.join("fake-provider.sh");
    fs::write(&provider, PROVIDER).expect("write provider script");
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o755))
        .expect("make provider executable");
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "census-model"
command = {provider:?}
bare_mode = false

[providers.census-cli]
kind = "claude_cli"
command = {provider:?}

[models.census-model]
provider = "census-cli"
slug = "claude-sonnet-4-6"
context_window = 200000
cost_input_per_m = 0.1
cost_output_per_m = 0.1
supports_tools = false

[routing.ladder]
enabled = false

[sections]
pinned = ["domain_context"]

[gates]
sibling_settle_secs = 0
"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko.toml");
    fs::write(workdir.join("README.md"), "# loop census run\n").expect("write README");

    let neuro = workdir.join(".roko/neuro");
    fs::create_dir_all(&neuro).expect("create the knowledge store's directory");
    let entry = serde_json::json!({
        "id": "kn-verify",
        "content": KNOWLEDGE_ENTRY,
        "confidence": 0.8,
        "created_at": chrono::Utc::now(),
    });
    fs::write(neuro.join("knowledge.jsonl"), format!("{entry}\n")).expect("seed the knowledge");
    let playbooks = workdir.join(".roko/learn/playbooks");
    fs::create_dir_all(&playbooks).expect("create the playbook directory");
    let playbook = roko_learn::playbook::Playbook::new("pb-verify", "Check the verify step");
    let playbook = serde_json::to_string(&playbook).expect("serialize the playbook");
    fs::write(playbooks.join("pb-verify.json"), playbook).expect("seed the playbook");
}

/// The fixture plan: [`CHAINS`] independent tasks whose titles share the
/// knowledge entry's and the playbook's topic words, each passing its
/// verify step on its one attempt.
fn tasks_toml() -> String {
    let mut toml = String::from(
        "[meta]\nplan = \"loop-census-run\"\nmax_parallel = 8\nskip_enrichment = true\n",
    );
    for chain in 0..CHAINS {
        let _ = write!(
            toml,
            r#"
[[task]]
id = "C{chain}"
title = "Check the verify step of chain {chain}"
description = "Its verify step passes."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "census-model"
files = ["c{chain}.txt"]
verify = [{{ phase = "structural", command = "test -d ." }}]
timeout_secs = 60
max_retries = 0
"#
        );
    }
    toml
}

/// Run the fixture plan through the real `roko` binary in a fresh
/// workspace. Returns the workspace and the run's directory.
fn run_fixture() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(workdir);
    let plan_dir = workdir.join("plans/loop-census-run");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("tasks.toml"), tasks_toml()).expect("write tasks.toml");

    let output = std::process::Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args([
            "--json",
            "plan",
            "run",
            "plans/loop-census-run",
            "--workdir",
        ])
        .arg(workdir)
        .env("HOME", workdir)
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("ROKO_CONFIG")
        .output()
        .expect("run roko plan run");
    let log = format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "every chain passes: {log}");

    let mut run_dirs: Vec<PathBuf> = fs::read_dir(workdir.join(".roko/runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("run directory").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(run_dirs.len(), 1, "{run_dirs:?}\n{log}");
    let run_dir = run_dirs.remove(0);
    (temp, run_dir)
}

/// Whether a row's S03 fields put its arm's draw before its decision; `None`
/// when the row carries no arm.
fn assigned_first(audit: &AuditFields) -> Option<bool> {
    let assigned_at = audit.assignment.as_ref()?.assigned_at;
    Some(assigned_at < audit.decided_at?)
}

#[test]
fn fixture_run_logs_arms_before_plan_and_measures_exposure() {
    let (_temp, run_dir) = run_fixture();
    let run = RunRecords::load(&run_dir).expect("read the run's records");
    assert!(run.invalid.is_empty(), "{:?}", run.invalid);
    assert_eq!(run.verdicts.len(), CHAINS, "one settled attempt per chain");

    // The knowledge layer: one draw per chain, near 80/20, and no SRM alarm.
    let knowledge: Vec<_> = run
        .content_decisions
        .iter()
        .map(|line| &line.record)
        .filter(|row| row.decision_point == ContentDecisionPoint::Knowledge)
        .collect();
    assert_eq!(knowledge.len(), CHAINS, "one knowledge row per chain");
    let learned = knowledge
        .iter()
        .filter_map(|row| row.audit.assignment.as_ref())
        .filter(|assignment| assignment.draw.arm == Arm::Learned)
        .count();
    let share = learned as f64 / CHAINS as f64;
    assert!((0.65..=0.90).contains(&share), "learned share {share}");
    let srm = srm_check(std::slice::from_ref(&run));
    let layer = srm
        .layers
        .iter()
        .find(|layer| layer.layer == "knowledge")
        .expect("the knowledge layer was checked");
    assert!(layer.e_value < 20.0 && !layer.mismatch, "{layer:?}");

    // Every arm was drawn before its decision: route and content rows alike.
    let route = run.decisions.iter().map(|line| &line.record.audit);
    let content = run.content_decisions.iter().map(|line| &line.record.audit);
    let ordered: Vec<bool> = route.chain(content).filter_map(assigned_first).collect();
    let carried = ordered.len();
    assert!(carried >= 2 * CHAINS, "{carried} rows carry an arm");
    let all_first = ordered.iter().all(|first| *first);
    assert!(all_first, "an arm was drawn after its decision");

    // Receipts: the rendered sections' hashes, and the provider's model.
    for line in &run.content_decisions {
        let row = &line.record;
        if row.chosen.is_empty() || !row.audit.present() {
            continue;
        }
        let receipt = row.audit.receipt.as_ref();
        let hashed =
            receipt.is_some_and(|receipt| receipt.ok && !receipt.exposure_hashes.is_empty());
        assert!(hashed, "an included item's receipt: {receipt:?}");
    }
    let reported = run
        .verdicts
        .iter()
        .filter(|line| line.record.executed.model_reported.is_some())
        .count();
    assert_eq!(reported, CHAINS, "every verdict names the provider's model");

    // The measured census sees both content loops reach their decisions.
    let measured = measure(std::slice::from_ref(&run));
    for loop_id in ["L-know", "L-play"] {
        let eps = measured.get(loop_id).map(|loop_| loop_.eps.est);
        assert!(eps.is_some_and(|eps| eps > 0.0), "{loop_id}: ε {eps:?}");
    }
}
