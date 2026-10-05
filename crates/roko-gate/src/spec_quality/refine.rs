//! The self-model's refine requests, as S07's spec gate reads them (6133; gap-2b0575).
//!
//! When M3 forecasts that a task's spec, scored below s_min, keeps every cheap rung short of the
//! success target, it lets the attempt run and appends a `spec.refine_requested` record to the
//! run's spec ledger (`.roko/runs/<run_id>/spec.jsonl`). The plan-load gate of a later run reads
//! every run's requests ([`read_refine_requests`]), and [`refine_verdict`] says what a task's open
//! requests mean in the gate's mode. S07 §4.3 sends a weak spec to REFINE before it runs, so
//! under `enforce` a request from a self-model that routes (active mode, its calibration gate
//! holding) stops the task until its spec is refined. Under `advise`, and for a request from a
//! self-model that only forecasts, the gate advises.
//!
//! A request stays open until the task's spec scores higher than when the request was made: S07
//! re-scores a spec after each refinement round, and a spec that scores no higher is the spec the
//! self-model judged.

use std::path::Path;

use roko_core::config::SpecQualityMode;
use roko_learn::self_model::spec_features::SPEC_RECORDS_FILE;
use serde::Deserialize;

use super::SpecQualityRecord;

/// The `ev` of a refine request in a run's spec ledger.
pub const REFINE_REQUESTED: &str = "spec.refine_requested";

/// How much higher, in score points, a spec must score than a request's score to count as
/// refined: half the 0.01 a score is rounded to, so that a request's score, stored over 1,
/// compares as equal to the score it came from.
const REFINED_BY: f64 = 0.005;

/// A `spec.refine_requested` record: the self-model asks for a task's spec to be refined before
/// the task runs (6133). The fields the gate does not read are skipped.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct RefineRequest {
    /// The run whose attempt the self-model forecast.
    pub run_id: String,
    /// The plan id of the attempt.
    pub plan_id: String,
    /// The task id.
    pub task_id: String,
    /// The self-model's `[self_model] mode`: `active` or `shadow`.
    #[serde(default)]
    pub mode: String,
    /// Whether the self-model routed when it asked: active, with its calibration gate holding.
    /// A record without the field counts as not routing.
    #[serde(default)]
    pub acting: bool,
    /// S07's score of the spec the self-model saw, over 1. A record without one is skipped: the
    /// self-model asks only about a scored spec.
    pub spec_score: f64,
    /// Unix ms the request was made.
    #[serde(default)]
    pub recorded_at_ms: i64,
}

/// What a task's open refine requests make of it at the spec gate.
#[derive(Clone, Debug, PartialEq)]
pub struct RefineVerdict {
    /// Whether the task is stopped until its spec is refined: under `enforce`, on a request from
    /// a self-model that routed. Otherwise the gate advises.
    pub blocks: bool,
    /// What the gate reports: the request it rests on, and the spec's score then and now.
    pub detail: String,
}

/// The refine requests in `text`, a spec ledger with one JSON record a line, in file order.
/// Other records, and lines that are not a whole request, are skipped.
#[must_use]
pub fn refine_requests(text: &str) -> Vec<RefineRequest> {
    text.lines()
        .filter(|line| line.contains(REFINE_REQUESTED))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["ev"] == REFINE_REQUESTED)
        .filter_map(|record| serde_json::from_value::<RefineRequest>(record).ok())
        .collect()
}

/// The refine requests in the spec ledger of every run under `runs_dir`
/// (`<runs_dir>/<run_id>/spec.jsonl`), oldest first. A run without a ledger adds none, and so
/// does a ledger that cannot be read.
#[must_use]
pub fn read_refine_requests(runs_dir: &Path) -> Vec<RefineRequest> {
    let Ok(runs) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut requests: Vec<RefineRequest> = runs
        .filter_map(Result::ok)
        .filter_map(|run| std::fs::read_to_string(run.path().join(SPEC_RECORDS_FILE)).ok())
        .flat_map(|text| refine_requests(&text))
        .collect();
    requests.sort_by_key(|request| request.recorded_at_ms);
    requests
}

/// The gate's verdict in `mode` on the open refine requests for the task `record` scores
/// (gap-2b0575): `None` when the gate is off, or when no request names the task with a score
/// its spec has not risen above since. Under `enforce`, an open request from a self-model that
/// routed blocks the task; any other open request is advice. A request answers to the task's
/// plan id and to its `tasks.toml`'s directory, the plan id Graph attempts carry, as the
/// self-model's spec features do (3240).
#[must_use]
pub fn refine_verdict(
    mode: SpecQualityMode,
    record: &SpecQualityRecord,
    requests: &[RefineRequest],
) -> Option<RefineVerdict> {
    if mode == SpecQualityMode::Off {
        return None;
    }
    let plans = plan_names(record);
    let open = requests.iter().filter(|request| {
        request.task_id == record.task_id
            && plans.contains(&request.plan_id.as_str())
            && record.score < request.spec_score.mul_add(100.0, REFINED_BY)
    });
    let latest = open.clone().max_by_key(|request| request.recorded_at_ms)?;
    let routed = open
        .filter(|request| request.acting)
        .max_by_key(|request| request.recorded_at_ms)
        .filter(|_| mode == SpecQualityMode::Enforce);
    let request = routed.unwrap_or(latest);
    let detail = format!(
        "the self-model asked for the spec to be refined (run {}, score {:.2}) and it scores \
         {:.2} now: refine its acceptance criteria, verify steps and files to read",
        request.run_id,
        request.spec_score * 100.0,
        record.score
    );
    Some(RefineVerdict {
        blocks: routed.is_some(),
        detail,
    })
}

/// The plan names a record answers to: its plan id, and its `tasks.toml`'s directory.
fn plan_names(record: &SpecQualityRecord) -> Vec<&str> {
    let directory = Path::new(&record.plan_path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    std::iter::once(record.plan_id.as_str())
        .chain(directory)
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_quality::lint_files;

    /// A plan with one task, kept in `plans/weak/` under the workspace root.
    const PLAN: &str = r#"[meta]
plan = "retry-limit"

[[task]]
id = "T1"
title = "Retry limit"
description = "Add the retry limit to `parse_config`."
role = "implementer"
files = ["src/config.rs"]
verify = [{ phase = "test", command = "cargo test -p demo --lib retry" }]
"#;

    /// gap-2b0575: an earlier run's ledger holds the self-model's request to refine T1's spec,
    /// written as 6133 writes it. Under `enforce`, a request from a self-model that routed stops
    /// the task; under `advise` it is advice, and with the gate off nothing reads it. A request
    /// from a self-model that only forecast is advice under `enforce` too. A request answers to
    /// the plan's id and to its directory, and to its task only; once the spec scores higher
    /// than the request saw, it has been refined.
    #[test]
    fn s07_enforce_mode_acts_on_spec_refine_requested() {
        let temp = tempfile::tempdir().expect("tempdir");
        let plan = temp.path().join("plans/weak/tasks.toml");
        std::fs::create_dir_all(temp.path().join("plans/weak")).expect("the plan's directory");
        std::fs::write(&plan, PLAN).expect("write the plan");
        let report = lint_files(&[plan], temp.path());
        assert!(report.parse_errors.is_empty(), "{:?}", report.parse_errors);
        let record = &report.tasks[0];

        let request = |plan: &str, task: &str, acting: bool, at: i64| {
            let mode = if acting { "active" } else { "shadow" };
            serde_json::json!({
                "ev": REFINE_REQUESTED,
                "run_id": "run-1",
                "plan_id": plan,
                "task_id": task,
                "attempt_key": format!("run-1/{plan}/{task}/1"),
                "source": "self_model",
                "mode": mode,
                "acting": acting,
                "spec_score": record.score / 100.0,
                "p_vs_max": 0.4,
                "recorded_at_ms": at,
            })
            .to_string()
        };
        // The run's ledger: its spec.quality line, a line that is not JSON, and the request.
        let runs = temp.path().join(".roko/runs");
        let run = runs.join("run-1");
        std::fs::create_dir_all(&run).expect("the run's directory");
        let quality = serde_json::json!({
            "ev": "spec.quality",
            "plan_id": "retry-limit",
            "task_id": "T1",
            "score": record.score,
        });
        let ledger = format!("{quality}\nnot json\n{}\n", request("weak", "T1", true, 1));
        std::fs::write(run.join(SPEC_RECORDS_FILE), ledger).expect("write the ledger");
        let requests = read_refine_requests(&runs);
        assert_eq!(requests.len(), 1, "{requests:?}");
        assert!(requests[0].acting, "{requests:?}");

        // Under enforce the request stops the task until its spec is refined.
        let verdict = refine_verdict(SpecQualityMode::Enforce, record, &requests);
        let verdict = verdict.expect("an open request");
        assert!(verdict.blocks, "{verdict:?}");
        assert!(verdict.detail.contains("run run-1"), "{verdict:?}");

        // Under advise it is advice only; with the gate off nothing reads it.
        let advice = refine_verdict(SpecQualityMode::Advise, record, &requests);
        assert!(
            advice.as_ref().is_some_and(|advice| !advice.blocks),
            "{advice:?}"
        );
        assert_eq!(
            refine_verdict(SpecQualityMode::Off, record, &requests),
            None
        );

        // A self-model that only forecast is advice under enforce too, unless a request from one
        // that routed is open as well.
        let forecast = refine_requests(&request("weak", "T1", false, 2));
        let verdict = refine_verdict(SpecQualityMode::Enforce, record, &forecast);
        assert!(
            verdict.as_ref().is_some_and(|verdict| !verdict.blocks),
            "{verdict:?}"
        );
        let both: Vec<RefineRequest> = requests.iter().chain(&forecast).cloned().collect();
        let verdict = refine_verdict(SpecQualityMode::Enforce, record, &both);
        assert!(verdict.is_some_and(|verdict| verdict.blocks));

        // The plan's id names it as well as its directory; another task or plan does not.
        let by_id = refine_requests(&request("retry-limit", "T1", true, 3));
        let verdict = refine_verdict(SpecQualityMode::Enforce, record, &by_id);
        assert!(verdict.is_some_and(|verdict| verdict.blocks));
        for (plan, task) in [("weak", "T2"), ("other", "T1")] {
            let other = refine_requests(&request(plan, task, true, 4));
            assert_eq!(
                refine_verdict(SpecQualityMode::Enforce, record, &other),
                None
            );
        }

        // A spec that scores higher than the request saw has been refined.
        let refined = SpecQualityRecord {
            score: record.score + 5.0,
            ..record.clone()
        };
        assert_eq!(
            refine_verdict(SpecQualityMode::Enforce, &refined, &requests),
            None
        );
    }
}
