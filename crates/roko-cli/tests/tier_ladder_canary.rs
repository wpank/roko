#![cfg(unix)]

//! Canary C8 (W8 gate G8), the exit check of epic spec-98f76d.
//!
//! One `roko plan run` through the built binary, with a scripted provider
//! serving three `[routing.ladder]` rungs, shows that:
//!
//! - each task starts on its tier's rung, or on the rung its `rung` hint names;
//! - two failed attempts move a task one rung up;
//! - a pinned model never moves;
//! - every attempt's verdict records its rung, model and reason (`ladder`).

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use serde_json::Value;

/// Wire slugs of the ladder's rungs, cheapest first.
const CHEAP: &str = "claude-haiku-4-5";
const MID: &str = "claude-sonnet-4-6";
const TOP: &str = "claude-opus-4-1";

/// The script of the `claude_cli` provider behind every rung: each call
/// leaves a `ran-<task>-<model>` marker for T2's verify step and reports
/// serving the model it was asked for.
fn ladder_script() -> Script {
    Script::new().otherwise(
        Turn::reply()
            .usage(3, 2, 0.0)
            .write("ran-@TASK@-@MODEL@", ""),
    )
}

/// The script of the provider of `routing.fast_task_model`, which takes the
/// helper calls after a failed verify step, so the ladder's provider sees
/// attempts only.
fn helper_script() -> Script {
    Script::new().otherwise(
        Turn::reply()
            .text("helper")
            .model("claude-helper")
            .usage(3, 2, 0.0),
    )
}

/// Five tasks, run one at a time:
///
/// - T1 (mechanical) starts on its tier's rung and passes;
/// - T2 (mechanical) passes only once the rung above its start has run it;
/// - T3 pins the cheap rung's model with `model_hint`, and always fails;
/// - T4 (mechanical) starts on the top rung, which its `rung` hint names;
/// - T5 (integrative) starts on its tier's rung and passes.
const TASKS: &str = r#"[meta]
plan = "tier-ladder"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "T1 starts on its tier's rung"
description = "A mechanical task with no hint. Its verify step passes."
role = "implementer"
status = "ready"
tier = "mechanical"
files = ["t1.txt"]
verify = [{ phase = "structural", command = "test -d ." }]
timeout_secs = 60

[[task]]
id = "T2"
title = "T2 climbs one rung after two failures"
description = "Its verify step passes once the mid rung's model has run it."
role = "implementer"
status = "ready"
tier = "mechanical"
files = ["t2.txt"]
verify = [{ phase = "structural", command = "test -f ran-T2-claude-sonnet-4-6", fail_msg = "T2 has not run on the mid rung" }]
timeout_secs = 60
max_retries = 3

[[task]]
id = "T3"
title = "T3 stays on its pinned model"
description = "Its model_hint pins the cheap rung's model. Its verify step always fails."
role = "implementer"
status = "ready"
tier = "mechanical"
model_hint = "cheap-model"
files = ["t3.txt"]
verify = [{ phase = "structural", command = "false", fail_msg = "T3 always fails" }]
timeout_secs = 60
max_retries = 2

[[task]]
id = "T4"
title = "T4 starts on the rung its hint names"
description = "A mechanical task whose rung hint names the top rung. Its verify step passes."
role = "implementer"
status = "ready"
tier = "mechanical"
rung = "top"
files = ["t4.txt"]
verify = [{ phase = "structural", command = "test -d ." }]
timeout_secs = 60

[[task]]
id = "T5"
title = "T5 starts on its tier's rung"
description = "An integrative task with no hint. Its verify step passes."
role = "implementer"
status = "ready"
tier = "integrative"
files = ["t5.txt"]
verify = [{ phase = "structural", command = "test -d ." }]
timeout_secs = 60
"#;

/// A workspace whose three ladder rungs (cheap, mid, top) all run on
/// `provider`, with the D11 start rungs, `helper` serving the helper calls,
/// and the plan of [`TASKS`].
fn write_workspace(workdir: &Path, provider: &ScriptedProvider, helper: &ScriptedProvider) {
    let (provider, helper) = (provider.command(), helper.command());
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "cheap-model"
command = {provider:?}
bare_mode = false

[providers.ladder-cli]
kind = "claude_cli"
command = {provider:?}

[providers.helper-cli]
kind = "claude_cli"
command = {helper:?}

[models.cheap-model]
provider = "ladder-cli"
slug = "{CHEAP}"
context_window = 200000

[models.mid-model]
provider = "ladder-cli"
slug = "{MID}"
context_window = 200000

[models.top-model]
provider = "ladder-cli"
slug = "{TOP}"
context_window = 200000

[models.helper-model]
provider = "helper-cli"
slug = "claude-helper"
context_window = 200000

[routing]
fast_task_model = "helper-model"

[routing.ladder]
rungs = [
  {{ name = "cheap", model = "cheap-model" }},
  {{ name = "mid", model = "mid-model" }},
  {{ name = "top", model = "top-model" }},
]

[gates]
sibling_settle_secs = 0
"#,
            provider = provider.display().to_string(),
            helper = helper.display().to_string(),
        ),
    )
    .expect("write roko.toml");
    let plan_dir = workdir.join("plans/tier-ladder");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("tasks.toml"), TASKS).expect("write tasks.toml");
}

/// The rows of the JSONL file at `path` (none when it does not exist).
fn jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a JSON row"))
        .collect()
}

/// Where a verdict ran and why: `<model> <rung>+<step> <reason> <outcome>`,
/// with `-` for a pinned attempt's rung.
fn place(verdict: &Value) -> String {
    let ladder = &verdict["ladder"];
    format!(
        "{} {}+{} {} {}",
        verdict["executed"]["model_dispatched"]
            .as_str()
            .unwrap_or("?"),
        ladder["rung"].as_str().unwrap_or("-"),
        ladder["step"]
            .as_u64()
            .map_or("?".to_string(), |step| step.to_string()),
        ladder["reason"].as_str().unwrap_or("?"),
        verdict["outcome"].as_str().unwrap_or("?"),
    )
}

/// `rows` grouped by task, each task's entries in order.
fn by_task(rows: impl IntoIterator<Item = (String, String)>) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (task, entry) in rows {
        grouped.entry(task).or_default().push(entry);
    }
    grouped
}

/// `roko plan run` of the workspace's plan, through the built binary: whether
/// it succeeded, and its output for failure messages.
fn run_plan(workdir: &Path) -> (bool, String) {
    let output = std::process::Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["--json", "plan", "run", "plans/tier-ladder", "--workdir"])
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
    (output.status.success(), log)
}

/// The models `provider` served, by task.
fn provider_calls(provider: &ScriptedProvider) -> BTreeMap<String, Vec<String>> {
    by_task(provider.calls().into_iter().map(|call| {
        (
            call.task.unwrap_or_else(|| "-".to_string()),
            call.model.unwrap_or_else(|| "unknown".to_string()),
        )
    }))
}

/// The verdict lines of the run's attempt log.
fn verdicts(roko: &Path) -> Vec<Value> {
    let run_dirs: Vec<_> = fs::read_dir(roko.join("runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("run directory").path())
        .collect();
    assert_eq!(run_dirs.len(), 1, "{run_dirs:?}");
    jsonl(&run_dirs[0].join("attempts.jsonl"))
        .into_iter()
        .filter(|line| line["schema_version"] == "roko.verdict/1")
        .collect()
}

/// The JSON file at `path`.
fn json_file(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("the run did not write {}: {error}", path.display()));
    serde_json::from_str(&text).expect("a JSON file")
}

#[test]
fn tier_ladder_canary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = &temp.path().join("work");
    fs::create_dir_all(workdir).expect("create the workdir");
    let provider = ScriptedProvider::install(&temp.path().join("ladder"), &ladder_script());
    let helper = ScriptedProvider::install(&temp.path().join("helper"), &helper_script());
    write_workspace(workdir, &provider, &helper);
    let (succeeded, log) = run_plan(workdir);
    assert!(!succeeded, "T3 fails, so the plan does: {log}");

    // The provider served each task on its rung, in order: T2 climbed one
    // rung after two failures, and pinned T3 never moved.
    let expected_calls = by_task(
        [
            ("T1", CHEAP),
            ("T2", CHEAP),
            ("T2", CHEAP),
            ("T2", MID),
            ("T3", CHEAP),
            ("T3", CHEAP),
            ("T3", CHEAP),
            ("T4", TOP),
            ("T5", MID),
        ]
        .map(|(task, model)| (task.to_string(), model.to_string())),
    );
    assert_eq!(provider_calls(&provider), expected_calls, "{log}");

    // T3 alone failed.
    let roko = workdir.join(".roko");
    let checkpoint = json_file(&roko.join("state/graph/tier-ladder/checkpoint.json"));
    assert_eq!(
        checkpoint["extensions"]["roko.task.outcome@1"]["value"]["failed"],
        serde_json::json!(["T3"]),
        "{checkpoint:#}"
    );

    // Every attempt's verdict says where it ran and why.
    let verdicts = verdicts(&roko);
    for verdict in &verdicts {
        assert!(verdict["cost"]["source"].is_string(), "{verdict}");
    }
    let places = by_task(verdicts.iter().map(|verdict| {
        let task = verdict["task_id"].as_str().unwrap_or("?").to_string();
        (task, place(verdict))
    }));
    let expected_places = by_task(
        [
            ("T1", format!("{CHEAP} cheap+0 start passed")),
            ("T2", format!("{CHEAP} cheap+0 start gate_failed")),
            ("T2", format!("{CHEAP} cheap+0 start gate_failed")),
            ("T2", format!("{MID} mid+1 escalated passed")),
            ("T3", format!("{CHEAP} -+0 pinned gate_failed")),
            ("T3", format!("{CHEAP} -+0 pinned gate_failed")),
            ("T3", format!("{CHEAP} -+0 pinned gate_failed")),
            ("T4", format!("{TOP} top+0 hint passed")),
            ("T5", format!("{MID} mid+0 start passed")),
        ]
        .map(|(task, place)| (task.to_string(), place)),
    );
    assert_eq!(places, expected_places, "{verdicts:#?}");

    // The router is credited only for its own picks (decision 4111): a rung
    // or a pinned model teaches it nothing, so no model gained trials.
    let router = json_file(&roko.join("learn/cascade-router.json"));
    let trained: Vec<&str> = [CHEAP, MID, TOP]
        .into_iter()
        .filter(|model| {
            router["confidence_stats"][model]["trials"]
                .as_u64()
                .is_some_and(|trials| trials > 0)
        })
        .collect();
    assert!(trained.is_empty(), "{trained:?}: {router:#}");
}
