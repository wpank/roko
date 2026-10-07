#![cfg(unix)]

//! Integration test C1 (gap-cd3529): one fixture run shows the same honest
//! verdicts on every surface. It is the exit check of epic spec-e9d7ec.
//!
//! The run holds five independent tasks for a scripted fake Claude CLI:
//!
//! - T1 passes its verify step;
//! - T2 has no verify step;
//! - T3's role is disabled;
//! - T4's verify step fails;
//! - T5 has no verify step and is served by a T0 reflex rule.
//!
//! Only T1 passed, and every surface must say so: the Graph checkpoints,
//! the run's JSON report, `roko plan status`, the run metrics, the dashboard,
//! the episodes and the reflex store. The dashboard is rebuilt from the
//! run's `--log-file` events, the hub events that the TUI, SSE and portal
//! clients receive.
//!
//! T1, T2 and T5 form one plan and T3 and T4 another, so that each fix of
//! the epic changes what some surface shows: with no failed task, the first
//! plan is unverified, not succeeded (gap-29a84b). T5 checks that a reflex
//! rule earns no gate pass for an output nothing verified (bug-94151f).
//!
//! No model runs: the agent is the shared scripted provider, and the run
//! takes seconds.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Output;

use assert_cmd::cargo::cargo_bin;
use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};
use roko_cli::graph_checkpoint::{GATE_VERDICT_EXTENSION, TASK_OUTCOME_EXTENSION};
use roko_core::DashboardEvent;
use roko_core::dashboard_snapshot::{DashboardSnapshot, TaskOutcomeClass, classify_task_outcome};
use serde_json::{Value, json};

/// T1, which passes its verify step, and T2 and T5, which have none.
const CHECKED_PLAN: &str = "verdicts-checked";
/// T3, whose role is disabled, and T4, whose verify step fails.
const REJECTED_PLAN: &str = "verdicts-rejected";

/// The agent: every call appends a line to `NOTES.md`, so each attempt
/// changes the file its task names, and reports one finished turn. Each
/// task's verify step alone decides its verdict.
fn agent() -> Script {
    Script::new().otherwise(Turn::reply().append("NOTES.md", "attempt\n"))
}

/// `roko.toml` additions: T3's role is off, T0 reflexes are on, and no
/// verification waits for sibling tasks to settle.
const CONFIG: &str = "agent.roles.researcher.enabled = false
learning.t0_reflexes = true
gates.sibling_settle_secs = 0
";

/// The reflex rule that serves T5, the only task whose title contains
/// `Task T5`. It has three matches, each followed by a passing gate.
fn reflex_rule() -> Value {
    json!({
        "id": "6f1c7a52-8c1e-4f5a-9d0b-3c2e1a4b5c6d",
        "condition": {
            "tool": null,
            "args_pattern": null,
            "context": "Task T5",
            "message_type": null,
            "file_ext": null
        },
        "action": { "tool": "reply", "args": "cached reply" },
        "confidence": 1.0,
        "source_episode": "c1-fixture",
        "promoted_at": "2026-09-30T00:00:00Z",
        "last_fired_at": null,
        "hit_count": 3,
        "success_count": 3
    })
}

/// `tasks.toml` of plan `plan`, one task per `(id, role, verify command)`.
/// A task without a verify command is a `scribe` or `researcher` task: an
/// `implementer` task must declare one. Every task but a `researcher`, which
/// may write no files, names `NOTES.md`. Plan validation refuses a verify-less
/// task of any role unless the plan sets `allow_unverified` (PLAN_037), which
/// changes no verdict: the task still ends unverified.
fn tasks_toml(plan: &str, tasks: &[(&str, &str, Option<&str>)]) -> String {
    let mut toml = format!(
        "[meta]\nplan = \"{plan}\"\nmax_parallel = 1\nskip_enrichment = true\nallow_unverified = true\n"
    );
    for (id, role, verify) in tasks {
        let files = if *role == "researcher" {
            "[]"
        } else {
            "[\"NOTES.md\"]"
        };
        let verify = verify.map_or_else(String::new, |command| {
            format!("verify = [{{ phase = \"structural\", command = {command:?} }}]\n")
        });
        toml.push_str(&format!(
            r#"
[[task]]
id = "{id}"
title = "Task {id}"
description = "Report a finished turn."
role = "{role}"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = {files}
timeout_secs = 60
max_retries = 0
{verify}"#
        ));
    }
    toml
}

/// Run `roko` in the workspace's repository as its user: provider keys,
/// `ROKO_*` variables and the invoking environment's log and config
/// variables are removed, as `ScriptedPlanWorkspace::run_plan` does.
fn roko(workspace: &ScriptedPlanWorkspace, args: &[&str]) -> Output {
    let mut command = std::process::Command::new(cargo_bin("roko"));
    command
        .current_dir(&workspace.repo)
        .args(args)
        .env("HOME", &workspace.home);
    for name in roko_core::child_env::PROVIDER_KEY_VARS {
        command.env_remove(name);
    }
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("ROKO_") {
            command.env_remove(name);
        }
    }
    for name in ["RUST_LOG", "XDG_CONFIG_HOME", "CLAUDECODE"] {
        command.env_remove(name);
    }
    // Warnings and errors on stderr, so a failure explains itself.
    command.env("ROKO_LOG", "warn");
    command.output().expect("run roko")
}

/// Parse `bytes` as JSON, or fail with `context`.
fn parse(bytes: &[u8], context: &dyn Fn() -> String) -> Value {
    serde_json::from_slice(bytes)
        .unwrap_or_else(|error| panic!("parse JSON: {error}\n{}", context()))
}

/// The JSON lines of `path`.
fn json_lines(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON line"))
        .collect()
}

#[test]
fn honest_verdicts_canary() {
    let (workspace, _provider) = ScriptedPlanWorkspace::with_provider(
        CHECKED_PLAN,
        &tasks_toml(
            CHECKED_PLAN,
            &[
                ("T1", "implementer", Some("test -d .")),
                ("T2", "scribe", None),
                ("T5", "scribe", None),
            ],
        ),
        &agent(),
        CONFIG,
    );
    let rejected_dir = workspace.repo.join("plans").join(REJECTED_PLAN);
    std::fs::create_dir_all(&rejected_dir).expect("create plan directory");
    std::fs::write(
        rejected_dir.join("plan.md"),
        format!("# Plan: {REJECTED_PLAN}\n"),
    )
    .expect("write plan.md");
    std::fs::write(
        rejected_dir.join("tasks.toml"),
        tasks_toml(
            REJECTED_PLAN,
            &[
                ("T3", "researcher", None),
                ("T4", "implementer", Some("false")),
            ],
        ),
    )
    .expect("write tasks.toml");
    workspace.git(&["add", "--all"]);
    workspace.git(&["commit", "--quiet", "-m", "second plan"]);
    let reflexes = workspace.repo.join(".roko/learn/reflexes.jsonl");
    std::fs::create_dir_all(reflexes.parent().expect("learn dir")).expect("create learn dir");
    std::fs::write(&reflexes, format!("{}\n", reflex_rule())).expect("write reflex rule");

    let events = workspace.fixtures.join("events.jsonl");
    let repo = workspace.repo.display().to_string();
    let output = roko(
        &workspace,
        &[
            "--json",
            "plan",
            "run",
            "plans",
            "--no-tui",
            "--workdir",
            &repo,
            "--log-file",
            &events.display().to_string(),
        ],
    );
    let context = || {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail = stderr
            .get(stderr.len().saturating_sub(4000)..)
            .unwrap_or(&stderr);
        format!(
            "exit: {}\nstdout:\n{}\nstderr (tail):\n{tail}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        )
    };

    // ── The run's JSON report: neither plan succeeded ─────────────────────
    assert!(!output.status.success(), "the run succeeded\n{}", context());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_start = stdout.rfind("\n{").map_or(0, |index| index + 1);
    let report = parse(stdout[report_start..].as_bytes(), &context);
    assert_eq!(report["succeeded"], false, "{}", context());
    assert_eq!(
        report["plan_outcomes"],
        json!({ CHECKED_PLAN: "unverified", REJECTED_PLAN: "failed" }),
        "{}",
        context()
    );

    // ── Checkpoints: T1 passed, T2 and T5 are unverified, T3 and T4 failed ─
    let checkpoint = |plan: &str| {
        let path = workspace
            .repo
            .join(".roko/state/graph")
            .join(plan)
            .join("checkpoint.json");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}\n{}", path.display(), context()));
        parse(&bytes, &context)
    };
    let checked = checkpoint(CHECKED_PLAN);
    assert_eq!(checked["status"], "unverified", "{checked:#}");
    assert_eq!(
        checked["extensions"][GATE_VERDICT_EXTENSION]["value"]["verdicts"],
        json!({ "T1": "passed", "T2": "unverified", "T5": "unverified" }),
        "{checked:#}"
    );
    let rejected = checkpoint(REJECTED_PLAN);
    assert_eq!(rejected["status"], "failed", "{rejected:#}");
    assert_eq!(
        rejected["extensions"][TASK_OUTCOME_EXTENSION]["value"]["failed"],
        json!(["T3", "T4"]),
        "{rejected:#}"
    );
    // Neither failed task left a recorded output with a verdict.
    assert!(
        rejected["extensions"][GATE_VERDICT_EXTENSION]["value"]["verdicts"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty),
        "{rejected:#}"
    );

    // ── `roko plan status`: the plan's status, from its checkpoint ─────────
    for (plan, expected) in [(CHECKED_PLAN, "unverified"), (REJECTED_PLAN, "failed")] {
        let status = roko(
            &workspace,
            &[
                "--json",
                "plan",
                "status",
                &format!("plans/{plan}"),
                "--workdir",
                &repo,
            ],
        );
        let status = parse(&status.stdout, &context);
        assert_eq!(status["status"], expected, "{status:#}");
        assert_eq!(status["completed"], false, "{status:#}");
    }

    // ── Run metrics: one passed task ───────────────────────────────────────
    let metrics = json_lines(&workspace.repo.join(".roko/learn/run-metrics.jsonl"));
    let [metrics] = metrics.as_slice() else {
        panic!("expected one run-metrics row: {metrics:#?}");
    };
    let counts = |row: &Value| {
        [
            "tasks_completed",
            "tasks_unverified",
            "tasks_failed",
            "tasks_skipped",
        ]
        .map(|key| row[key].as_u64().unwrap_or(u64::MAX))
    };
    assert_eq!(counts(metrics), [1, 2, 2, 0], "{metrics:#}");
    let plan_rows: BTreeMap<&str, &Value> = metrics["plans"]
        .as_array()
        .expect("plans")
        .iter()
        .map(|row| (row["plan_id"].as_str().unwrap_or_default(), row))
        .collect();
    assert_eq!(counts(plan_rows[CHECKED_PLAN]), [1, 2, 0, 0], "{metrics:#}");
    assert_eq!(
        counts(plan_rows[REJECTED_PLAN]),
        [0, 0, 2, 0],
        "{metrics:#}"
    );
    assert!(
        plan_rows.values().all(|row| row["completed"] == false),
        "{metrics:#}"
    );

    // ── Dashboard: the run's hub events count one pass ─────────────────────
    let lines = json_lines(&events);
    let mut snapshot = DashboardSnapshot::default();
    for line in &lines {
        let kind = line["type"].as_str().unwrap_or_default();
        if matches!(
            kind,
            "dashboard.plan_set_loaded"
                | "dashboard.plan_started"
                | "dashboard.task_started"
                | "dashboard.task_completed"
                | "dashboard.plan_completed"
        ) {
            let event: DashboardEvent =
                serde_json::from_value(line["event"].clone()).expect("dashboard event");
            snapshot.apply(&event);
        }
    }
    let outcomes: BTreeMap<&str, (&str, TaskOutcomeClass)> = snapshot
        .tasks
        .values()
        .map(|task| {
            let outcome = task.outcome.as_deref().unwrap_or("none");
            (
                task.task_id.as_str(),
                (outcome, classify_task_outcome(outcome)),
            )
        })
        .collect();
    assert_eq!(
        outcomes,
        BTreeMap::from([
            ("T1", ("passed", TaskOutcomeClass::Passed)),
            ("T2", ("unverified", TaskOutcomeClass::Unverified)),
            ("T3", ("failed", TaskOutcomeClass::Failed)),
            ("T4", ("failed", TaskOutcomeClass::Failed)),
            ("T5", ("unverified", TaskOutcomeClass::Unverified)),
        ])
    );
    let stats = &snapshot.stats;
    assert_eq!(
        (
            stats.tasks_completed,
            stats.tasks_unverified,
            stats.tasks_failed,
            stats.tasks_skipped
        ),
        (1, 2, 2, 0)
    );
    let checked_plan = &snapshot.plans[CHECKED_PLAN];
    assert_eq!(
        (checked_plan.tasks_passed, checked_plan.tasks_unverified),
        (1, 2)
    );
    for plan in [CHECKED_PLAN, REJECTED_PLAN] {
        assert_eq!(snapshot.plans[plan].phase, "failed", "{plan}");
    }
    let end = lines.last().expect("run.completed line");
    assert_eq!(end["type"], "run.completed", "{end:#}");
    assert_eq!(
        end["task_outcomes"],
        json!({ "passed": 1, "unverified": 2, "failed": 2 }),
        "{end:#}"
    );

    // ── Episodes: only T1's attempt is labelled a pass ─────────────────────
    // Learners read `learning_label` (S01 §4.1): `1` is a pass, `0` a
    // failure, and `null` teaches nothing. `success` keeps its older meaning
    // (the provider call succeeded and no verify step failed), so it is true
    // for T2 as well.
    let mut episodes: Vec<(String, Value, Value)> =
        json_lines(&workspace.repo.join(".roko/episodes.jsonl"))
            .into_iter()
            .map(|episode| {
                let extra = &episode["extra"];
                (
                    episode["task_id"].as_str().unwrap_or_default().to_string(),
                    extra["outcome"].clone(),
                    extra["learning_label"].clone(),
                )
            })
            .collect();
    episodes.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        episodes,
        [
            ("T1".to_string(), json!("passed"), json!(1)),
            ("T2".to_string(), json!("unverified"), Value::Null),
            ("T4".to_string(), json!("gate_failed"), json!(0)),
        ],
        "T3's role is disabled and a reflex served T5, so neither has an agent turn"
    );

    // ── Reflex store: T5's unverified output earned its rule no gate pass ──
    let rules = json_lines(&reflexes);
    let [rule] = rules.as_slice() else {
        panic!("expected the one reflex rule: {rules:#?}");
    };
    assert_eq!(rule["success_count"], 3, "{rule:#}");
}
