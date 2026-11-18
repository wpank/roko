//! Regression coverage for learn-path alignment.
//!
//! These tests write fixture data to the canonical `.roko/episodes.jsonl`,
//! `.roko/learn/`, and `.roko/neuro/` locations that the `learn` command
//! reads, then assert the CLI summary output reflects the stored data.

mod common;
use common::Isolated;

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use chrono::{DateTime, Utc};
use roko_learn::cascade::CascadeStage;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::{Episode, Usage};
use tempfile::TempDir;

fn utc(timestamp: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(timestamp)
        .expect("valid RFC3339 timestamp")
        .with_timezone(&Utc)
}

fn write_jsonl(path: &Path, lines: &[String]) {
    fs::write(path, format!("{}\n", lines.join("\n"))).expect("write jsonl");
}

fn run_roko(workdir: &Path, args: &[&str]) -> String {
    let assert = Command::cargo_bin("roko")
        .expect("roko binary")
        .isolated()
        .current_dir(workdir)
        .args(args)
        .arg("--workdir")
        .arg(workdir)
        .assert()
        .success();

    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn write_learn_fixture(workdir: &Path) {
    let learn_dir = workdir.join(".roko").join("learn");
    let neuro_dir = workdir.join(".roko").join("neuro");
    fs::create_dir_all(&learn_dir).expect("create learn dir");
    fs::create_dir_all(&neuro_dir).expect("create neuro dir");

    let router_first = utc("2026-04-10T08:00:00Z");
    let router_last = utc("2026-04-10T09:30:00Z");
    let router_path = learn_dir.join("cascade-router.json");
    let router_json = serde_json::json!({
        "model_slugs": ["claude-sonnet-4-6", "claude-haiku-4-5"],
        "total_observations": 75,
        "stage_transitions": [
            {
                "from": CascadeStage::Static,
                "to": CascadeStage::Confidence,
                "observations": 50,
                "timestamp": router_first.clone(),
            },
            {
                "from": CascadeStage::Confidence,
                "to": CascadeStage::Ucb,
                "observations": 75,
                "timestamp": router_last.clone(),
            }
        ]
    });
    fs::write(
        &router_path,
        serde_json::to_string(&router_json).expect("serialize router fixture"),
    )
    .expect("write router fixture");

    let efficiency_first = utc("2026-04-10T10:00:00Z");
    let efficiency_last = utc("2026-04-10T11:15:00Z");
    let mut efficiency_a = AgentEfficiencyEvent::default();
    efficiency_a.agent_id = "agent-a".into();
    efficiency_a.role = "Implementer".into();
    efficiency_a.backend = "claude_cli".into();
    efficiency_a.model = "claude-sonnet-4-6".into();
    efficiency_a.plan_id = "plan-a".into();
    efficiency_a.task_id = "task-a".into();
    efficiency_a.input_tokens = 100;
    efficiency_a.output_tokens = 25;
    efficiency_a.cache_read_tokens = 10;
    efficiency_a.cache_write_tokens = 5;
    efficiency_a.cost_usd = 0.0012;
    efficiency_a.cost_usd_without_cache = 0.0015;
    efficiency_a.total_prompt_tokens = 300;
    efficiency_a.system_prompt_tokens = 120;
    efficiency_a.tools_available = 4;
    efficiency_a.tools_used = 1;
    efficiency_a.wall_time_ms = 850;
    efficiency_a.time_to_first_token_ms = 175;
    efficiency_a.was_warm_start = true;
    efficiency_a.iteration = 1;
    efficiency_a.gate_passed = Some(true);
    efficiency_a.model_used = "claude-sonnet-4-6".into();
    efficiency_a.timestamp = efficiency_first.to_rfc3339();

    let mut efficiency_b = AgentEfficiencyEvent::default();
    efficiency_b.agent_id = "agent-b".into();
    efficiency_b.role = "Reviewer".into();
    efficiency_b.backend = "claude_cli".into();
    efficiency_b.model = "claude-haiku-4-5".into();
    efficiency_b.plan_id = "plan-b".into();
    efficiency_b.task_id = "task-b".into();
    efficiency_b.input_tokens = 80;
    efficiency_b.output_tokens = 20;
    efficiency_b.cache_read_tokens = 0;
    efficiency_b.cache_write_tokens = 0;
    efficiency_b.cost_usd = 0.0034;
    efficiency_b.cost_usd_without_cache = 0.0038;
    efficiency_b.total_prompt_tokens = 220;
    efficiency_b.system_prompt_tokens = 90;
    efficiency_b.tools_available = 2;
    efficiency_b.tools_used = 1;
    efficiency_b.wall_time_ms = 910;
    efficiency_b.time_to_first_token_ms = 200;
    efficiency_b.was_warm_start = false;
    efficiency_b.iteration = 2;
    efficiency_b.gate_passed = Some(false);
    efficiency_b.outcome = "gate_failure".into();
    efficiency_b.model_used = String::new();
    efficiency_b.timestamp = efficiency_last.to_rfc3339();

    let efficiency_path = learn_dir.join("efficiency.jsonl");
    write_jsonl(
        &efficiency_path,
        &[
            serde_json::to_string(&efficiency_a).expect("serialize efficiency fixture a"),
            serde_json::to_string(&efficiency_b).expect("serialize efficiency fixture b"),
        ],
    );

    let episodes_first = utc("2026-04-10T12:00:00Z");
    let episodes_last = utc("2026-04-10T12:45:00Z");
    let mut episode_a = Episode::new("agent-a", "task-a");
    episode_a.kind = "agent_turn".into();
    episode_a.id = "episode-a".into();
    episode_a.episode_id = "episode-a".into();
    episode_a.agent_id = "agent-a".into();
    episode_a.agent_template = "Implementer".into();
    episode_a.model = "claude-sonnet-4-6".into();
    episode_a.backend = "claude_cli".into();
    episode_a.turns = 1;
    episode_a.tokens_used = 125;
    episode_a.success = true;
    episode_a.usage = Usage {
        input_tokens: 100,
        output_tokens: 25,
        cache_read_tokens: 10,
        cache_write_tokens: 5,
        cost_usd: 0.0012,
        cost_usd_without_cache: 0.0015,
        wall_ms: 850,
    };
    episode_a.timestamp = episodes_first.clone();
    episode_a.started_at = episodes_first.clone();
    episode_a.completed_at = episodes_first.clone();

    let mut episode_b = Episode::new("agent-b", "task-b");
    episode_b.kind = "agent_turn".into();
    episode_b.id = "episode-b".into();
    episode_b.episode_id = "episode-b".into();
    episode_b.agent_id = "agent-b".into();
    episode_b.agent_template = "Reviewer".into();
    episode_b.model = "claude-haiku-4-5".into();
    episode_b.backend = "claude_cli".into();
    episode_b.turns = 1;
    episode_b.tokens_used = 100;
    episode_b.success = false;
    episode_b.failure_reason = Some("gate failed".into());
    episode_b.usage = Usage {
        input_tokens: 80,
        output_tokens: 20,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_usd: 0.0022,
        cost_usd_without_cache: 0.0026,
        wall_ms: 910,
    };
    episode_b.timestamp = episodes_last.clone();
    episode_b.started_at = episodes_last.clone();
    episode_b.completed_at = episodes_last.clone();

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    write_jsonl(
        &episodes_path,
        &[
            serde_json::to_string(&episode_a).expect("serialize episode fixture a"),
            serde_json::to_string(&episode_b).expect("serialize episode fixture b"),
        ],
    );

    let knowledge_path = neuro_dir.join("knowledge.jsonl");
    write_jsonl(
        &knowledge_path,
        &[serde_json::to_string(&serde_json::json!({
            "id": "knowledge-1",
            "topic": "fixture"
        }))
        .expect("serialize knowledge fixture")],
    );
}

/// When `timestamp` was, as `roko learn` says it (the design system's §6.3
/// forms: `10 Apr`), so the test reads like the view in any time zone.
fn day(timestamp: DateTime<Utc>) -> String {
    roko_cli::status_view::ago(timestamp.timestamp_millis(), Utc::now().timestamp_millis())
}

/// A span of days as `roko learn` says it: `10 Apr`, or `9 Apr to 10 Apr`.
fn span(first: DateTime<Utc>, last: DateTime<Utc>) -> String {
    let (first, last) = (day(first), day(last));
    if first == last {
        first
    } else {
        format!("{first} to {last}")
    }
}

/// The facts of `roko learn`'s part `label`, one per line: what its labelled
/// line says after the label, then each line under it, up to the next label
/// or the closing line.
fn part(stdout: &str, label: &str) -> Vec<String> {
    let mut lines = stdout.lines().skip_while(|line| {
        line.trim_start()
            .strip_prefix(label)
            .is_none_or(|rest| !rest.starts_with("  "))
    });
    let Some(first) = lines.next() else {
        panic!("no {label} part: {stdout}");
    };
    let indent = first.len() - first.trim_start().len();
    let first = first.trim_start().strip_prefix(label).unwrap_or_default();
    std::iter::once(first)
        .chain(lines.take_while(|line| line.len() - line.trim_start().len() > indent))
        .map(|line| line.trim().to_string())
        .collect()
}

/// The files `roko learn` names on its closing line.
fn read_from(stdout: &str) -> String {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Read from "))
        .unwrap_or_else(|| panic!("no closing line naming the files read: {stdout}"))
        .to_string()
}

/// gap-cb1a87: `roko learn all` gives each fixture's facts in its part, in
/// the run view's style: the router's observations, models and stage changes,
/// the efficiency events and episodes with their span and latest one, and the
/// knowledge entries; the files read are named on its closing line, relative
/// to the workspace, and no part reads empty.
#[test]
fn learn_all_reads_the_expected_fixtures() {
    let tmp = TempDir::new().expect("tempdir");
    let workdir = tmp.path();
    write_learn_fixture(workdir);

    let stdout = run_roko(workdir, &["learn", "all"]);

    let router_first = utc("2026-04-10T08:00:00Z");
    let router_last = utc("2026-04-10T09:30:00Z");
    let efficiency_first = utc("2026-04-10T10:00:00Z");
    let efficiency_last = utc("2026-04-10T11:15:00Z");
    let episodes_first = utc("2026-04-10T12:00:00Z");
    let episodes_last = utc("2026-04-10T12:45:00Z");

    let routing = part(&stdout, "Routing");
    assert_eq!(
        routing[0],
        format!(
            "75 observations · 2 stage changes since {}, the last {}: confidence to ucb after 75 \
             observations",
            day(router_first),
            day(router_last)
        ),
        "router summary missing or incorrect: {stdout}"
    );
    for model in ["claude-sonnet-4-6", "claude-haiku-4-5"] {
        assert!(
            routing.iter().any(|fact| fact.contains(model)),
            "router model {model} missing: {stdout}"
        );
    }

    let efficiency = part(&stdout, "Efficiency");
    assert_eq!(
        efficiency[0],
        format!("2 events, {}", span(efficiency_first, efficiency_last)),
        "efficiency summary missing or incorrect: {stdout}"
    );
    assert_eq!(
        efficiency[1],
        format!(
            "latest: {}, claude-haiku-4-5 on plan-b task task-b, failed, <$0.01",
            day(efficiency_last)
        ),
        "efficiency latest line missing or incorrect: {stdout}"
    );

    let episodes = part(&stdout, "Episodes");
    assert_eq!(
        episodes[0],
        format!("2 episodes, {}", span(episodes_first, episodes_last)),
        "episode summary missing or incorrect: {stdout}"
    );
    assert_eq!(
        episodes[1],
        format!(
            "latest: {}, claude-haiku-4-5 on task task-b, failed, <$0.01",
            day(episodes_last)
        ),
        "episode latest line missing or incorrect: {stdout}"
    );

    assert!(
        part(&stdout, "Knowledge")[0].starts_with("1 live, 0 retired"),
        "knowledge summary missing or incorrect: {stdout}"
    );

    let read = read_from(&stdout);
    for path in [
        ".roko/learn/cascade-router.json",
        ".roko/learn/efficiency.jsonl",
        ".roko/episodes.jsonl",
        ".roko/neuro/knowledge.jsonl",
    ] {
        assert!(
            read.contains(path),
            "{path} missing from the files read: {stdout}"
        );
    }
    for empty in [
        "no router state yet",
        "no observations yet",
        "no events yet",
        "no episodes yet",
        "no lessons yet",
    ] {
        assert!(
            !stdout.contains(empty),
            "fixture should have satisfied every learn path: {stdout}"
        );
    }
}

/// gap-cb1a87: `roko learn efficiency` without an efficiency file says it has
/// no events, names the file it read, and makes up no count.
#[test]
fn learn_missing_efficiency_reports_no_data_instead_of_fake_counts() {
    let tmp = TempDir::new().expect("tempdir");
    let workdir = tmp.path();

    let stdout = run_roko(workdir, &["learn", "efficiency"]);

    assert_eq!(
        part(&stdout, "Efficiency"),
        ["no events yet: each try records what it spent"],
        "missing efficiency file should say it has no events: {stdout}"
    );
    assert!(
        read_from(&stdout).contains(".roko/learn/efficiency.jsonl"),
        "missing efficiency file should report the read path: {stdout}"
    );
    assert!(
        !stdout.contains(" events, ") && !stdout.contains("latest:"),
        "missing efficiency file must not produce a fake summary: {stdout}"
    );
}
