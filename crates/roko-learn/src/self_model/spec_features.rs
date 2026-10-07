//! S07's spec features for the self-model (S07 §4.4; backlog 3240).
//!
//! Before a run's first attempt of a plan, the spec gate appends one `spec.quality` record per
//! task to the run's `spec.jsonl` (3231). This module reads those records as JSON, so it needs
//! no roko-gate, and joins them to the run's attempts by plan and task: a task's spec is fixed
//! within a run, and the records carry no attempt key. A record answers to its plan id and to
//! its `tasks.toml`'s directory, which is the plan id Graph attempts carry.
//!
//! Each record becomes the S07 §4.4 vector, by name: `sqs` (the score), `rule:<SQ>` for the
//! rule scores, `hard_fail:<HF>` and `hard_fails`, one-hot `band:`, `verify_max_class:`,
//! `red_on_base:`, `role:` and `spec_origin:`, and the counts and flags (`n_verify`,
//! `n_acceptance`, `ac_coverage`, `desc_words`, `vague_density`, `n_read_files`, `n_symbols`,
//! `n_files`, `max_loc`, `refine_rounds`, `has_hidden_hook`, ...). It also carries the names
//! the self-model's feature vector reads ([`SPEC_FEATURES`](super::features::SPEC_FEATURES)):
//! `spec_score` (the score over 100), `has_acceptance_criteria`, `n_acceptance_criteria`,
//! `n_verify_cmds`, `has_hidden_hook`, `ambiguity_terms` (the vague terms found) and
//! `spec_tokens` (the words of goal and description, standing in for tokens).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::telemetry::AttemptKey;

/// The file in a run's directory that holds its spec records (3231).
pub const SPEC_RECORDS_FILE: &str = "spec.jsonl";

/// The `ev` of a spec-quality record.
pub const SPEC_QUALITY_EVENT: &str = "spec.quality";

/// The counts and measures of a record's `features` the vector keeps under their own names.
const MEASURES: [&str; 12] = [
    "n_verify",
    "n_accept",
    "n_acceptance",
    "n_observable",
    "ac_coverage",
    "desc_words",
    "vague_density",
    "n_read_files",
    "n_symbols",
    "n_files",
    "max_loc",
    "refine_rounds",
];

/// The flags of a record's `features` the vector keeps as 0 or 1.
const FLAGS: [&str; 5] = [
    "has_hidden_hook",
    "has_acceptance",
    "has_test_verify",
    "has_non_goals",
    "planner_test",
];

/// One task's S07 §4.4 vector: feature name to value.
pub type SpecVector = BTreeMap<String, f64>;

/// A run's spec vectors, by plan and task.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecFeatureIndex {
    by_task: BTreeMap<(String, String), SpecVector>,
    /// Lines that are not JSON, and spec-quality records without a task.
    pub unreadable: usize,
}

impl SpecFeatureIndex {
    /// The vectors of the `spec.quality` records in `text`, one JSON object a line. Other
    /// events, such as `spec.gate`, are skipped.
    #[must_use]
    pub fn from_records(text: &str) -> Self {
        let mut index = Self::default();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(record) = serde_json::from_str::<Value>(line) else {
                index.unreadable += 1;
                continue;
            };
            if record["ev"] != SPEC_QUALITY_EVENT {
                continue;
            }
            let Some(task) = record["task_id"].as_str() else {
                index.unreadable += 1;
                continue;
            };
            let vector = spec_vector(&record);
            for plan in plan_names(&record) {
                index
                    .by_task
                    .insert((plan, task.to_string()), vector.clone());
            }
        }
        index
    }

    /// The index of the run directory `run_dir` (`.roko/runs/<run_id>`); empty when the run
    /// has no spec records.
    pub fn read_run(run_dir: &Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(run_dir.join(SPEC_RECORDS_FILE)) {
            Ok(text) => Ok(Self::from_records(&text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    /// The vector of `task_id` in the plan `plan_id`.
    #[must_use]
    pub fn get(&self, plan_id: &str, task_id: &str) -> Option<&SpecVector> {
        let key = (plan_id.to_string(), task_id.to_string());
        self.by_task.get(&key)
    }

    /// The vector of the task the attempt `key` belongs to.
    #[must_use]
    pub fn for_attempt(&self, key: &AttemptKey) -> Option<&SpecVector> {
        self.get(&key.plan_id, &key.task_id)
    }

    /// The (plan, task) pairs indexed, a task counted once per name its plan answers to.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_task.len()
    }

    /// Whether no task is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_task.is_empty()
    }
}

/// The plan names a record answers to: its plan id, and its `tasks.toml`'s directory.
fn plan_names(record: &Value) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(plan) = record["plan_id"].as_str().filter(|plan| !plan.is_empty()) {
        names.push(plan.to_string());
    }
    let directory = record["plan_path"]
        .as_str()
        .map(Path::new)
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    if let Some(directory) = directory
        && !names.iter().any(|name| name == directory)
    {
        names.push(directory.to_string());
    }
    names
}

/// A `spec.quality` record as the S07 §4.4 vector (module docs).
#[must_use]
pub fn spec_vector(record: &Value) -> SpecVector {
    let features = &record["features"];
    let mut vector = SpecVector::new();
    if let Some(score) = record["score"].as_f64() {
        vector.insert("sqs".to_string(), score);
        vector.insert("spec_score".to_string(), score / 100.0);
    }
    for (rule, score) in record["rules"].as_object().into_iter().flatten() {
        if let Some(score) = score.as_f64() {
            vector.insert(format!("rule:{rule}"), score);
        }
    }
    let hard_fails: Vec<&str> = record["hard_fail"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for hard_fail in &hard_fails {
        vector.insert(format!("hard_fail:{hard_fail}"), 1.0);
    }
    vector.insert("hard_fails".to_string(), hard_fails.len() as f64);
    let categories = [
        ("band", &record["band"]),
        ("verify_max_class", &features["verify_max_class"]),
        ("red_on_base", &record["red_on_base"]),
        ("role", &record["role"]),
        ("spec_origin", &features["spec_origin"]),
    ];
    for (name, value) in categories {
        if let Some(value) = value.as_str() {
            vector.insert(format!("{name}:{value}"), 1.0);
        }
    }
    for name in MEASURES {
        if let Some(value) = features[name].as_f64() {
            vector.insert(name.to_string(), value);
        }
    }
    for name in FLAGS {
        if let Some(flag) = features[name].as_bool() {
            vector.insert(name.to_string(), f64::from(u8::from(flag)));
        }
    }
    // The names the self-model's feature vector reads.
    let aliases = [
        ("has_acceptance_criteria", "has_acceptance"),
        ("n_acceptance_criteria", "n_acceptance"),
        ("n_verify_cmds", "n_verify"),
        ("spec_tokens", "desc_words"),
    ];
    for (alias, name) in aliases {
        if let Some(&value) = vector.get(name) {
            vector.insert(alias.to_string(), value);
        }
    }
    if let Some(terms) = features["vague_terms"].as_array() {
        vector.insert("ambiguity_terms".to_string(), terms.len() as f64);
    }
    vector
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::self_model::features::SPEC_FEATURES;

    fn fixture_runs() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/self_model/runs")
    }

    /// A `spec.quality` line of `task`, as the spec gate writes it for the plan `plan_id`
    /// whose `tasks.toml` sits in `plans/<directory>/`.
    fn record(plan_id: &str, directory: &str, task: &str) -> String {
        // Split out of the record below: one json! of both overruns the macro
        // recursion limit.
        let features = serde_json::json!({
            "verify_max_class": "test",
            "n_verify": 2,
            "n_accept": 0,
            "n_no_run": 0,
            "n_self_exit": 0,
            "has_test_verify": true,
            "has_acceptance": true,
            "has_acceptance_fields": true,
            "n_acceptance": 3,
            "n_observable": 2,
            "ac_coverage": 0.5,
            "desc_words": 54,
            "vague_density": 1.25,
            "vague_terms": ["robust"],
            "n_read_files": 1,
            "n_read_files_why": 1,
            "n_symbols": 2,
            "n_symbols_anchored": 1,
            "n_files": 3,
            "max_loc": 200,
            "has_non_goals": false,
            "has_hidden_hook": false,
            "planner_test": false,
            "refine_rounds": 0
        });
        serde_json::json!({
            "ev": "spec.quality",
            "linter": "sq-3",
            "mode": "static",
            "plan_id": plan_id,
            "plan_path": format!("plans/{directory}/tasks.toml"),
            "archived": false,
            "task_id": task,
            "role": "implementer",
            "score": 72.5,
            "band": "B",
            "hard_fail": ["HF4"],
            "hard_fail_detail": {"HF4": ["src/missing.rs"]},
            "unknown": ["HF3", "SQ06"],
            "excluded": ["SQ06"],
            "rules": {"SQ01": 1.0, "SQ02": 0.5, "SQ03": 0.25},
            "verify_classes": ["test", "compile"],
            "red_on_base": "unknown",
            "features": features,
            "run_id": "run-a",
            "recorded_at_ms": 1_759_400_000_000_i64
        })
        .to_string()
    }

    /// 3240: on the replayed fixture runs every attempt's task finds its spec vector, through
    /// its plan id or its plan's directory, with the S07 §4.4 features and every name the
    /// self-model's feature vector reads but `example_io_present`, which no record carries.
    #[test]
    fn spec_features_join_every_attempt() {
        let temp = tempfile::tempdir().expect("tempdir");
        // run-b's plan was renamed in its `[meta]`: its directory still names it.
        let runs = [
            (
                "run-a",
                "plan-1",
                "plan-1",
                vec!["T1", "T2", "T3", "T4", "T5"],
            ),
            ("run-b", "renamed", "plan-2", vec!["T1"]),
        ];
        let (mut attempts, mut joined) = (0, 0);
        for (run, plan_id, directory, tasks) in runs {
            let run_dir = temp.path().join(run);
            std::fs::create_dir_all(&run_dir).expect("create the run directory");
            let attempt_log = fixture_runs().join(run).join("attempts.jsonl");
            std::fs::copy(&attempt_log, run_dir.join("attempts.jsonl")).expect("copy the log");
            let mut lines: Vec<String> = tasks
                .iter()
                .map(|task| record(plan_id, directory, task))
                .collect();
            lines.push(r#"{"ev":"spec.gate","task_id":"T1","decision":"allow"}"#.to_string());
            lines.push("not json".to_string());
            let spec_log = run_dir.join(SPEC_RECORDS_FILE);
            std::fs::write(spec_log, lines.join("\n")).expect("write the spec records");

            let index = SpecFeatureIndex::read_run(&run_dir).expect("read the spec records");
            assert_eq!(index.unreadable, 1);
            let log = std::fs::read_to_string(run_dir.join("attempts.jsonl")).expect("the log");
            for line in log.lines() {
                let record: Value = serde_json::from_str(line).expect("an attempt line");
                if record["schema_version"] != "roko.attempt_open/1" {
                    continue;
                }
                let key = record["attempt_key"].as_str().and_then(AttemptKey::parse);
                let key = key.expect("an attempt key");
                attempts += 1;
                joined += usize::from(index.for_attempt(&key).is_some());
            }
        }
        assert_eq!(attempts, 7);
        assert_eq!(joined, attempts, "every attempt gets a spec vector");

        let index = SpecFeatureIndex::from_records(&record("plan-1", "plan-1", "T1"));
        let vector = index.get("plan-1", "T1").expect("T1's vector");
        let expected = [
            ("sqs", 72.5),
            ("spec_score", 0.725),
            ("rule:SQ02", 0.5),
            ("hard_fail:HF4", 1.0),
            ("hard_fails", 1.0),
            ("band:B", 1.0),
            ("verify_max_class:test", 1.0),
            ("red_on_base:unknown", 1.0),
            ("role:implementer", 1.0),
            ("ac_coverage", 0.5),
            ("max_loc", 200.0),
            ("has_hidden_hook", 0.0),
            ("has_acceptance_criteria", 1.0),
            ("n_acceptance_criteria", 3.0),
            ("n_verify_cmds", 2.0),
            ("ambiguity_terms", 1.0),
            ("spec_tokens", 54.0),
        ];
        for (name, value) in expected {
            assert_eq!(vector.get(name), Some(&value), "{name}");
        }
        let missing: Vec<&str> = SPEC_FEATURES
            .into_iter()
            .filter(|name| !vector.contains_key(*name))
            .collect();
        assert_eq!(missing, ["example_io_present"]);
    }
}
