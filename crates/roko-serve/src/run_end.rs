//! A run's end, written into the run's own event index when the run itself
//! could not write it (bug-dceafd, bug-9a5970).
//!
//! A Graph run records its events in `.roko/events-by-run/<sha256(run id)>.jsonl`
//! as it goes and ends them with `run_completed`. Two kinds of run never get
//! that far: one whose roko process died ([`crate::interrupted_runs`]), and
//! one roko refused to start, such as a run that found the workspace's runner
//! lock taken ([`refused_rows`]). Serve writes their end into the run's own
//! index. It never writes it into the shared hub or `.roko/events.jsonl`,
//! whose readers would take it for the end of whichever run is live
//! (bug-09e21e).

use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::Path;

use serde_json::{Value, json};

/// The `run_completed` outcome of a run roko refused to start. A run summary
/// reads it as `failed`, with the refusal under `refused`.
pub const REFUSED: &str = "refused";

/// The `stage` of a run refused before any plan of it started.
pub const START_STAGE: &str = "start";

/// The `stage` of a run whose plans the spec gate refused before any
/// dispatch.
pub const SPEC_GATE_STAGE: &str = "spec_gate";

/// How much of the end of a run's index is searched for an end it already
/// has.
const END_SCAN_BYTES: u64 = 64 * 1024;

/// What a run's own index holds at its end.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct IndexEnd {
    /// The index exists.
    pub exists: bool,
    /// It records the run's end.
    pub ended: bool,
    /// Its last row is incomplete: its writer was stopped in the middle.
    pub partial: bool,
}

/// What run `run_id`'s own index beside `global_log` holds at its end.
pub(crate) fn index_end(global_log: &Path, run_id: &str) -> std::io::Result<IndexEnd> {
    let path =
        roko_fs::run_index::run_index_path(global_log, run_id).map_err(std::io::Error::other)?;
    let mut index = match roko_fs::run_index::open_existing_run_index(&path) {
        Ok(index) => index,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(IndexEnd::default());
        }
        Err(error) => return Err(error),
    };
    let length = index.metadata()?.len();
    index.seek(SeekFrom::Start(length.saturating_sub(END_SCAN_BYTES)))?;
    let mut tail = Vec::new();
    index.read_to_end(&mut tail)?;
    let ended = tail.split(|byte| *byte == b'\n').any(|line| {
        serde_json::from_slice::<Value>(line).is_ok_and(|row| {
            matches!(
                row.get("type").and_then(Value::as_str),
                Some("run_completed" | "run.completed" | "workflow_completed")
            )
        })
    });
    Ok(IndexEnd {
        exists: true,
        ended,
        partial: tail.last().is_some_and(|byte| *byte != b'\n'),
    })
}

/// Append `rows` to run `run_id`'s own index beside `global_log`, creating
/// the index when the run has none. A `partial` last row (see [`IndexEnd`])
/// stays on a line of its own. Each line gets the secret scrubbing the run's
/// own writer applies: an error message can quote a key.
pub(crate) fn append(
    global_log: &Path,
    run_id: &str,
    rows: &[Value],
    partial: bool,
) -> std::io::Result<()> {
    let mut text = String::new();
    if partial {
        text.push('\n');
    }
    for row in rows {
        let line = serde_json::to_string(row).map_err(std::io::Error::other)?;
        text.push_str(&roko_core::obs::scrub_secrets_in_jsonl(&line));
        text.push('\n');
    }
    let (_, mut index) = roko_fs::run_index::open_run_index_append(global_log, run_id)?;
    index.write_all(text.as_bytes())?;
    index.sync_data()
}

/// The rows that end run `run_id` when roko refused to start it (bug-9a5970).
///
/// They are an `error` row with `message`, then a `run_completed` row with
/// outcome [`REFUSED`], stage [`START_STAGE`] and `reason`, after
/// `duration_ms`. Serve writes them for a plan run, and `roko serve`'s
/// runtime for a prompt run.
#[must_use]
pub fn refused_rows(run_id: &str, message: &str, reason: &str, duration_ms: u64) -> Vec<Value> {
    let now = chrono::Utc::now().timestamp_millis();
    let error = json!({
        "type": "error",
        "run_id": run_id,
        "message": message,
        "ts_millis": now,
    });
    let mut end = serde_json::to_value(roko_core::DashboardEvent::RunCompleted {
        run_id: Some(run_id.to_string()),
        outcome: REFUSED.to_string(),
        duration_ms,
        cleanup_degraded: false,
        surviving_agent_ids: Vec::new(),
        surviving_agent_pids: Vec::new(),
    })
    .unwrap_or_else(|_| json!({"type": "run_completed", "run_id": run_id, "outcome": REFUSED}));
    if let Some(fields) = end.as_object_mut() {
        fields.insert("stage".into(), json!(START_STAGE));
        fields.insert("reason".into(), json!(reason));
        fields.insert("ts_millis".into(), json!(now));
    }
    vec![error, end]
}

/// How much of the start of a run's index is searched for what the spec gate
/// refused, which a run records before its first dispatch.
const GATE_SCAN_BYTES: u64 = 256 * 1024;

/// What the spec gate refused in run `run_id`, from the `spec.gate.refused`
/// entries near the start of its own index beside `global_log`: one
/// `<task>: <rule>: <detail>` per finding, or none when the gate refused
/// nothing (bug-9a5970).
pub(crate) fn spec_gate_refusals(global_log: &Path, run_id: &str) -> Vec<String> {
    let Ok(path) = roko_fs::run_index::run_index_path(global_log, run_id) else {
        return Vec::new();
    };
    let Ok(index) = roko_fs::run_index::open_existing_run_index(&path) else {
        return Vec::new();
    };
    let mut head = Vec::new();
    if index.take(GATE_SCAN_BYTES).read_to_end(&mut head).is_err() {
        return Vec::new();
    }
    let mut refusals = Vec::new();
    for line in head.split(|byte| *byte == b'\n') {
        let Ok(row) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if row["type"] != "event_log_entry" || row["event_type"] != "spec.gate.refused" {
            continue;
        }
        let message = row["message"].as_str().unwrap_or_default();
        let refusal = match row["task_id"].as_str().filter(|task| !task.is_empty()) {
            Some(task) => format!("{task}: {message}"),
            None => message.to_string(),
        };
        if !refusal.is_empty() && !refusals.contains(&refusal) {
            refusals.push(refusal);
        }
    }
    refusals
}

/// The first line of `message` that is not blank, trimmed: an error's own
/// words without the hint after them.
#[must_use]
pub fn first_line(message: &str) -> &str {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusals_reason_is_its_first_line() {
        let message = "A roko plan runner is already active in this workspace (PID 42).\n  \
                       hint: wait for it to finish, or kill it with `kill 42`";
        assert_eq!(
            first_line(message),
            "A roko plan runner is already active in this workspace (PID 42)."
        );
        assert_eq!(first_line("\n  \n"), "");
    }

    /// The rows of a refused start: its error, then its end, outcome
    /// `refused` with its stage and reason; an index that has them records
    /// the run's end.
    #[test]
    fn a_refused_start_ends_its_runs_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global_log = dir.path().join(".roko/events.jsonl");
        std::fs::create_dir_all(global_log.parent().unwrap()).expect("create .roko");
        assert_eq!(
            index_end(&global_log, "r1").expect("no index"),
            IndexEnd::default()
        );

        let rows = refused_rows("r1", "plan execution failed for p: busy", "busy", 7);
        assert_eq!(rows[0]["type"], "error");
        assert_eq!(rows[0]["message"], "plan execution failed for p: busy");
        assert_eq!(rows[1]["type"], "run_completed");
        assert_eq!(rows[1]["outcome"], REFUSED);
        assert_eq!(rows[1]["stage"], START_STAGE);
        assert_eq!(rows[1]["reason"], "busy");
        assert_eq!(rows[1]["run_id"], "r1");
        assert_eq!(rows[1]["duration_ms"], 7);

        append(&global_log, "r1", &rows, false).expect("append");
        let end = index_end(&global_log, "r1").expect("index");
        assert!(end.exists && end.ended && !end.partial, "{end:?}");
        assert!(spec_gate_refusals(&global_log, "r1").is_empty());
    }

    /// What the spec gate refused, read from a run's index: one line per
    /// finding, each once.
    #[test]
    fn a_runs_spec_gate_refusals_come_from_its_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global_log = dir.path().join(".roko/events.jsonl");
        std::fs::create_dir_all(global_log.parent().unwrap()).expect("create .roko");
        let refused = |task: &str, message: &str| {
            json!({"type": "event_log_entry", "event_type": "spec.gate.refused", "run_id": "r2",
                   "task_id": task, "message": message})
        };
        let rows = [
            json!({"type": "plan_set_loaded", "run_id": "r2", "plans": []}),
            refused("T1", "HF2: step 1: ends in `|| true`"),
            refused("T2", "HF3: verify passes on the unchanged base"),
            refused("T1", "HF2: step 1: ends in `|| true`"),
            json!({"type": "run_completed", "run_id": "r2", "outcome": "failed"}),
        ];
        append(&global_log, "r2", &rows, false).expect("append");
        assert_eq!(
            spec_gate_refusals(&global_log, "r2"),
            [
                "T1: HF2: step 1: ends in `|| true`",
                "T2: HF3: verify passes on the unchanged base"
            ]
        );
        assert!(spec_gate_refusals(&global_log, "no-such-run").is_empty());
    }
}
