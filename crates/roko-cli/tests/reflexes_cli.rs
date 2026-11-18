//! End-to-end coverage for `roko learn reflexes` human and JSON output.

mod common;
use common::Isolated;

use std::fs;

use assert_cmd::Command;
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

/// The facts of the Reflexes part of `roko learn reflexes` (gap-cb1a87), one
/// per line: what its labelled line says after the label, then each line
/// under it.
fn reflexes_part(stdout: &str) -> Vec<String> {
    let mut lines = stdout.lines().skip_while(|line| {
        line.trim_start()
            .strip_prefix("Reflexes")
            .is_none_or(|rest| !rest.starts_with("  "))
    });
    let Some(first) = lines.next() else {
        panic!("no Reflexes part: {stdout}");
    };
    let indent = first.len() - first.trim_start().len();
    let first = first
        .trim_start()
        .strip_prefix("Reflexes")
        .unwrap_or_default();
    std::iter::once(first)
        .chain(lines.take_while(|line| line.len() - line.trim_start().len() > indent))
        .map(|line| line.trim().to_string())
        .collect()
}

#[test]
fn learn_reflexes_reports_an_empty_store_friendly() {
    let workdir = tempfile::tempdir().expect("temp workdir");

    let output = Command::cargo_bin("roko")
        .expect("roko binary")
        .isolated()
        .current_dir(workdir.path())
        .args(["learn", "reflexes", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("UTF-8 stdout");

    assert_eq!(
        reflexes_part(&stdout),
        [
            "0 rules (max 200) · off: [learning] t0_reflexes is false, so runs add no rules",
            "recent demotions: none",
        ],
        "{stdout}"
    );
    assert!(
        stdout.contains("Read from .roko/learn/reflexes.jsonl."),
        "{stdout}"
    );
}

#[test]
fn learn_reflexes_reports_total_and_only_the_top_five() {
    let workdir = tempfile::tempdir().expect("temp workdir");
    write_reflexes(workdir.path(), &[1, 30, 10, 50, 40, 20]);
    write_demotion(workdir.path());

    let output = Command::cargo_bin("roko")
        .expect("roko binary")
        .isolated()
        .current_dir(workdir.path())
        .args(["learn", "reflexes", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("UTF-8 stdout");

    let facts = reflexes_part(&stdout);
    // The total and the cap, then the top five by hits, in order.
    assert_eq!(
        facts[0], "6 rules (max 200), the top ones by hits:",
        "{stdout}"
    );
    for (rank, hits) in (1..).zip([50, 40, 30, 20, 10]) {
        assert_eq!(
            facts[rank],
            format!("{rank}. 100% confident, {hits} hits: \"bash\" → bash rule-{hits}"),
            "{stdout}"
        );
    }
    assert!(
        !stdout.contains("→ bash rule-1\n"),
        "sixth rule leaked into output: {stdout}"
    );
    assert_eq!(stdout.matches("% confident, ").count(), 5, "{stdout}");
    // The demotion, with its plan, task and try.
    assert_eq!(facts[6], "recent demotions:", "{stdout}");
    assert!(
        facts[7].ends_with(": plan plan-a task task-b try attempt-c"),
        "{stdout}"
    );
    assert_eq!(facts.len(), 8, "{stdout}");
}

#[test]
fn learn_reflexes_honors_global_json_output() {
    let workdir = tempfile::tempdir().expect("temp workdir");
    write_reflexes(workdir.path(), &[7]);
    write_demotion(workdir.path());

    let output = Command::cargo_bin("roko")
        .expect("roko binary")
        .isolated()
        .current_dir(workdir.path())
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
