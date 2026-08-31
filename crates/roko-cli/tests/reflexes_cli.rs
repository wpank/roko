//! End-to-end coverage for `roko learn reflexes` human and JSON output.

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;

fn reflex_rule(index: u32, hit_count: u32) -> serde_json::Value {
    json!({
        "id": format!("00000000-0000-0000-0000-{index:012}"),
        "condition": {
            "tool": "bash",
            "args_pattern": null,
            "context": null,
            "message_type": null,
            "file_ext": null
        },
        "action": {
            "tool": "bash",
            "args": format!("rule-{hit_count}")
        },
        "confidence": 1.0,
        "source_episode": format!("episode-{index}"),
        "promoted_at": "2026-08-31T12:00:00Z",
        "last_fired_at": null,
        "hit_count": hit_count,
        "success_count": hit_count
    })
}

fn write_reflexes(workdir: &std::path::Path, hit_counts: &[u32]) {
    let learn_dir = workdir.join(".roko").join("learn");
    fs::create_dir_all(&learn_dir).expect("create learn directory");
    let contents = hit_counts
        .iter()
        .enumerate()
        .map(|(index, hit_count)| reflex_rule(index as u32 + 1, *hit_count).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(learn_dir.join("reflexes.jsonl"), format!("{contents}\n"))
        .expect("write reflex fixture");
}

fn write_demotion(workdir: &std::path::Path) {
    let learn_dir = workdir.join(".roko").join("learn");
    fs::create_dir_all(&learn_dir).expect("create learn directory");
    let event = roko_learn::efficiency::AgentEfficiencyEvent {
        backend: "t0-reflex".to_string(),
        model: "t0-reflex".to_string(),
        plan_id: "plan-a".to_string(),
        task_id: "task-b".to_string(),
        attempt_id: "attempt-c".to_string(),
        gate_passed: Some(false),
        outcome: "reflex_demoted".to_string(),
        timestamp: "2026-08-31T12:30:00Z".to_string(),
        ..roko_learn::efficiency::AgentEfficiencyEvent::default()
    };
    fs::write(
        learn_dir.join("efficiency.jsonl"),
        format!(
            "{}\n",
            serde_json::to_string(&event).expect("serialize demotion fixture")
        ),
    )
    .expect("write demotion fixture");
}

#[test]
fn learn_reflexes_reports_an_empty_store_friendly() {
    let workdir = tempfile::tempdir().expect("temp workdir");

    Command::cargo_bin("roko")
        .expect("roko binary")
        .args(["learn", "reflexes", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "T0 Reflex Store — 0 rules (max 200)",
        ))
        .stdout(predicate::str::contains("no rules yet"))
        .stdout(predicate::str::contains("Recent demotions:\n  (none)"));
}

#[test]
fn learn_reflexes_reports_total_and_only_the_top_five() {
    let workdir = tempfile::tempdir().expect("temp workdir");
    write_reflexes(workdir.path(), &[1, 30, 10, 50, 40, 20]);
    write_demotion(workdir.path());

    let output = Command::cargo_bin("roko")
        .expect("roko binary")
        .args(["learn", "reflexes", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("UTF-8 stdout");

    assert!(stdout.contains("T0 Reflex Store — 6 rules (max 200)"));
    assert!(stdout.contains("Top rules by hit count:"));
    for hits in [50, 40, 30, 20, 10] {
        assert!(stdout.contains(&format!("rule-{hits}")), "{stdout}");
    }
    assert!(
        !stdout.contains("→ bash rule-1\n"),
        "sixth rule leaked into output: {stdout}"
    );
    assert_eq!(stdout.matches("% conf,").count(), 5, "{stdout}");
    assert!(
        stdout.contains("plan=plan-a task=task-b attempt=attempt-c"),
        "{stdout}"
    );
}

#[test]
fn learn_reflexes_honors_global_json_output() {
    let workdir = tempfile::tempdir().expect("temp workdir");
    write_reflexes(workdir.path(), &[7]);
    write_demotion(workdir.path());

    let output = Command::cargo_bin("roko")
        .expect("roko binary")
        .args(["--json", "learn", "reflexes", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON output");

    assert_eq!(value["reflexes"]["total_rules"], 1);
    assert_eq!(value["reflexes"]["max_rules"], 200);
    assert_eq!(
        value["reflexes"]["top_rules"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        value["reflexes"]["recent_demotions"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        value["reflexes"]["recent_demotions"][0]["outcome"],
        "reflex_demoted"
    );
}
