//! Runs whose roko process is gone (bug-dceafd).
//!
//! A run's manifest, `.roko/runs/<run>/manifest.json`, names the process that
//! runs it: one invocation per start or resume, with its pid and when it
//! started. When that process dies before the run ends (a machine restart, a
//! crash, `kill -9`), nothing ends the run: its manifest stays open and its
//! event index never records an end. It would read as running forever, hold
//! its host's budget, and answer a cancel with "no active execution".
//!
//! [`settle`] ends such a run as [`INTERRUPTED`]. Its manifest records the
//! interruption and closes, and its own event index, when it has one, gets a
//! `run_completed` row with that outcome and [`INTERRUPTED_REASON`], stamped
//! at the run's last sign of life. The row goes to that index only
//! ([`crate::run_end`]), never to the shared state hub or `.roko/events.jsonl`,
//! as a refused run's rows do (bug-09e21e): a live run's dashboard would take
//! it for its own end. Serve
//! settles the open runs when it starts ([`settle_open_runs`]), and a run it
//! holds no handle of when its summary, detail or cancel is asked for.
//!
//! A process counts as gone only when that is certain: no process has its
//! pid, or the one that has it started after the invocation did, so the pid
//! was recycled. An invocation on another host, or a process whose start this
//! host cannot tell, counts as alive. Nothing is ever signalled.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use roko_fs::RokoLayout;
use roko_learn::telemetry::records::{MANIFEST_FILE, RunClosed, RunInvocation};
use roko_learn::telemetry::{AttemptTally, RunInterruption, RunProvenanceManifest};
use serde_json::json;

/// The state and `run_completed` outcome of a run whose process was gone.
pub const INTERRUPTED: &str = "interrupted";

/// What happened to such a run, in words a host can show its person.
pub const INTERRUPTED_REASON: &str =
    "roko stopped while this run was going (machine restart or crash)";

/// How many runs serve settles when it starts, the most recently written
/// manifests first.
const MAX_SETTLED_AT_START: usize = 256;

/// How much later than its invocation a process may have started and still
/// be the invocation's own. The invocation is recorded after its process
/// starts; `ps` reports a start to the second, and a clock may step.
const START_SLACK_MS: i64 = 5_000;

/// One settle at a time, so no run gets two ends.
static SETTLING: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// How a run whose process was gone ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interrupted {
    /// The run's last sign of life (RFC 3339, UTC).
    pub at: String,
    /// What happened ([`INTERRUPTED_REASON`]).
    pub reason: String,
    /// Whether this call ended the run; `false` when it had ended so before.
    pub settled_now: bool,
}

/// End run `run_id` of the workspace at `workdir` as [`INTERRUPTED`] when its
/// manifest is open and the process its last invocation names is gone.
///
/// Returns how the run ended when it ended interrupted, now or before;
/// `None` when it has no manifest, ended some other way, or its process may
/// still be running.
pub fn settle(workdir: &Path, run_id: &str) -> Option<Interrupted> {
    roko_fs::run_index::validate_scoped_id(run_id).ok()?;
    let _settling = SETTLING.lock();
    let layout = RokoLayout::for_project(workdir);
    let run_dir = layout.run_dir(run_id);
    let mut manifest = match RunProvenanceManifest::load(&run_dir) {
        Ok(manifest) => manifest?,
        // Asked on every read of the run's summary: a run that cannot be told
        // about stays as its index says, without a warning each time.
        Err(error) => {
            tracing::debug!(
                run_id,
                %error,
                "run manifest unreadable; cannot tell whether the run is still going"
            );
            return None;
        }
    };
    if let Some(closed) = &manifest.closed {
        if closed.status != INTERRUPTED {
            return None;
        }
        let last = manifest.interruptions.last();
        return Some(Interrupted {
            at: last.map_or_else(|| closed.ended_at.clone(), |last| last.at.clone()),
            reason: last.map_or_else(
                || INTERRUPTED_REASON.to_string(),
                |last| last.reason.clone(),
            ),
            settled_now: false,
        });
    }
    let owner = manifest
        .invocations
        .iter()
        .max_by_key(|invocation| invocation.inv)?
        .clone();
    if !process_gone(&owner) {
        return None;
    }

    let global_log = layout.events_jsonl_path();
    let index = roko_fs::run_index::run_index_path(&global_log, run_id).ok();
    let started_ms = parse_ms(&owner.started_at);
    let at_ms = last_sign_of_life(&run_dir, index.as_deref(), started_ms);
    let at = rfc3339(at_ms);
    let tally = AttemptTally::read(&run_dir).unwrap_or_default();
    manifest.interruptions.push(RunInterruption {
        at: at.clone(),
        detected_at: rfc3339(now_ms()),
        inv: owner.inv,
        pid: owner.pid,
        reason: INTERRUPTED_REASON.to_string(),
    });
    manifest.closed = Some(RunClosed {
        ended_at: at.clone(),
        status: INTERRUPTED.to_string(),
        attempts_opened: tally.opened,
        attempts_settled: tally.settled,
        abandoned: tally.abandoned,
        ..RunClosed::default()
    });
    if let Err(error) = manifest.store(&run_dir) {
        tracing::warn!(
            run_id,
            %error,
            "run manifest not closed; only the run's index records its end"
        );
    }
    let first_ms = manifest
        .invocations
        .iter()
        .filter_map(|invocation| parse_ms(&invocation.started_at))
        .min()
        .unwrap_or(at_ms);
    let duration_ms = u64::try_from(at_ms.saturating_sub(first_ms)).unwrap_or(0);
    if let Err(error) = record_end(&global_log, run_id, at_ms, duration_ms) {
        tracing::warn!(run_id, %error, "the run's index does not record its end");
    }
    tracing::warn!(
        run_id,
        pid = owner.pid,
        last_seen = %at,
        "the roko process running this run is gone; closed the run as interrupted"
    );
    Some(Interrupted {
        at,
        reason: INTERRUPTED_REASON.to_string(),
        settled_now: true,
    })
}

/// [`settle`] each open run of the workspace at `workdir`, the
/// [`MAX_SETTLED_AT_START`] most recently written manifests first, as serve
/// does when it starts. Returns the runs it ended.
pub fn settle_open_runs(workdir: &Path) -> Vec<String> {
    let runs_dir = RokoLayout::for_project(workdir).runs_dir();
    let Ok(entries) = std::fs::read_dir(&runs_dir) else {
        return Vec::new();
    };
    let mut runs: Vec<(SystemTime, String)> = entries
        .flatten()
        .filter_map(|entry| {
            let run_id = entry.file_name().into_string().ok()?;
            let written = std::fs::metadata(entry.path().join(MANIFEST_FILE))
                .and_then(|metadata| metadata.modified())
                .ok()?;
            Some((written, run_id))
        })
        .collect();
    runs.sort_by(|left, right| right.cmp(left));
    runs.into_iter()
        .take(MAX_SETTLED_AT_START)
        .filter_map(|(_, run_id)| {
            settle(workdir, &run_id)
                .filter(|ended| ended.settled_now)
                .map(|_| run_id)
        })
        .collect()
}

/// Whether the process `invocation` names is certainly gone.
fn process_gone(invocation: &RunInvocation) -> bool {
    if invocation.host != "local" || invocation.pid == 0 {
        return false;
    }
    let Some(invoked_ms) = parse_ms(&invocation.started_at) else {
        return false;
    };
    match pid_holder(invocation.pid) {
        PidHolder::None => true,
        // A process that started after the invocation did has a recycled pid.
        PidHolder::StartedAt(started_ms) => started_ms > invoked_ms.saturating_add(START_SLACK_MS),
        PidHolder::Unknown => false,
    }
}

/// What this host can tell of the process that has a pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidHolder {
    /// No process has it.
    None,
    /// The process that has it started then (ms since the Unix epoch).
    StartedAt(i64),
    /// A process may have it, but its start cannot be told.
    Unknown,
}

fn pid_holder(pid: u32) -> PidHolder {
    if let Some(started_ms) = roko_agent::process::process_identity(pid)
        .and_then(|identity| identity.started_at_ms)
        .and_then(|started_ms| i64::try_from(started_ms).ok())
    {
        return PidHolder::StartedAt(started_ms);
    }
    // The kernel probe cannot inspect another user's process; `ps` lists
    // every process with how long it has been running.
    let Ok(output) = Command::new("ps")
        .args(["-o", "etime=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return PidHolder::Unknown;
    };
    let listed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if listed.is_empty() {
        // `ps` exits 1 when it lists no process.
        return if output.status.code() == Some(1) {
            PidHolder::None
        } else {
            PidHolder::Unknown
        };
    }
    parse_elapsed_secs(&listed)
        .and_then(|secs| secs.checked_mul(1_000))
        .map_or(PidHolder::Unknown, |elapsed_ms| {
            PidHolder::StartedAt(now_ms().saturating_sub(elapsed_ms))
        })
}

/// Seconds from `ps`'s `etime`, `[[dd-]hh:]mm:ss`.
fn parse_elapsed_secs(etime: &str) -> Option<i64> {
    let (days, clock) = match etime.split_once('-') {
        Some((days, clock)) => (days.parse::<i64>().ok()?, clock),
        None => (0, etime),
    };
    let parts = clock
        .split(':')
        .map(|part| part.parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (0, *minutes, *seconds),
        [hours, minutes, seconds] => (*hours, *minutes, *seconds),
        _ => return None,
    };
    Some(((days * 24 + hours) * 60 + minutes) * 60 + seconds)
}

/// The run's last sign of life (ms since the Unix epoch): the latest write
/// to its index or its attempt and manifest files, and no earlier than its
/// last invocation's start.
fn last_sign_of_life(run_dir: &Path, index: Option<&Path>, started_ms: Option<i64>) -> i64 {
    let written = [
        index.map(Path::to_path_buf),
        Some(run_dir.join("attempts.jsonl")),
        Some(run_dir.join(MANIFEST_FILE)),
    ]
    .into_iter()
    .flatten()
    .filter_map(|path: PathBuf| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    })
    .filter_map(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
    .filter_map(|since| i64::try_from(since.as_millis()).ok())
    .max();
    written.max(started_ms).unwrap_or_else(now_ms)
}

/// Append run `run_id`'s `run_completed` row, outcome [`INTERRUPTED`], to its
/// own index beside `global_log`, when it has an index and that has no end
/// yet. A run that never wrote an event gets no index.
fn record_end(
    global_log: &Path,
    run_id: &str,
    at_ms: i64,
    duration_ms: u64,
) -> std::io::Result<()> {
    let end = crate::run_end::index_end(global_log, run_id)?;
    if !end.exists || end.ended {
        return Ok(());
    }
    let mut row = serde_json::to_value(roko_core::DashboardEvent::RunCompleted {
        run_id: Some(run_id.to_string()),
        outcome: INTERRUPTED.to_string(),
        duration_ms,
        cleanup_degraded: false,
        surviving_agent_ids: Vec::new(),
        surviving_agent_pids: Vec::new(),
    })
    .map_err(std::io::Error::other)?;
    if let Some(fields) = row.as_object_mut() {
        fields.insert("reason".into(), json!(INTERRUPTED_REASON));
        fields.insert("ts_millis".into(), json!(at_ms));
        fields.insert("detected_at_ms".into(), json!(now_ms()));
    }
    crate::run_end::append(global_log, run_id, &[row], end.partial)
}

fn parse_ms(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.timestamp_millis())
}

fn rfc3339(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    /// An open manifest for run `run_id` in `workdir` whose one invocation
    /// is process `pid`, started at `started_at`.
    fn write_open_run(workdir: &Path, run_id: &str, pid: u32, started_at: &str) {
        let mut manifest = RunProvenanceManifest::new(run_id, "plan_run");
        manifest.plan_ids.push("p1".to_string());
        manifest.begin_invocation(RunInvocation {
            started_at: started_at.to_string(),
            pid,
            host: "local".to_string(),
            ..RunInvocation::default()
        });
        manifest
            .store(&RokoLayout::for_project(workdir).run_dir(run_id))
            .expect("store the manifest");
    }

    /// A pid no process has: a child that has exited and been reaped.
    fn exited_pid() -> u32 {
        let mut child = Command::new("true").spawn().expect("spawn true");
        let pid = child.id();
        child.wait().expect("reap true");
        pid
    }

    #[test]
    fn elapsed_times_parse_as_ps_prints_them() {
        assert_eq!(parse_elapsed_secs("00:07"), Some(7));
        assert_eq!(parse_elapsed_secs("01:02:03"), Some(3_723));
        assert_eq!(parse_elapsed_secs("2-01:02:03"), Some(2 * 86_400 + 3_723));
        assert_eq!(parse_elapsed_secs("soon"), None);
    }

    /// A live process is never taken for gone: this test's own process, and
    /// an invocation on another host or with no pid.
    #[test]
    fn a_live_process_or_one_that_cannot_be_checked_is_not_gone() {
        let me = RunInvocation {
            started_at: rfc3339(now_ms()),
            pid: std::process::id(),
            host: "local".to_string(),
            ..RunInvocation::default()
        };
        assert!(!process_gone(&me), "this process is running");
        let elsewhere = RunInvocation {
            pid: exited_pid(),
            host: "build-02".to_string(),
            ..me.clone()
        };
        assert!(!process_gone(&elsewhere), "another host's process");
        let unknown = RunInvocation { pid: 0, ..me };
        assert!(!process_gone(&unknown), "no pid");
    }

    /// A pid that now names a process started after the invocation was
    /// recycled: the invocation's own process is gone.
    #[test]
    fn a_recycled_pid_is_gone() {
        let recycled = RunInvocation {
            started_at: rfc3339(now_ms() - 3_600_000),
            pid: std::process::id(),
            host: "local".to_string(),
            ..RunInvocation::default()
        };
        assert!(process_gone(&recycled));
        let exited = RunInvocation {
            pid: exited_pid(),
            ..recycled
        };
        assert!(process_gone(&exited));
    }

    /// Settling ends an open run whose process is gone once: the manifest
    /// closes interrupted with the interruption, the run's index gets one
    /// `run_completed` row, and settling again reports the same end without
    /// writing another. A run whose process runs is left alone.
    #[test]
    fn settling_ends_a_run_whose_process_is_gone_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = RokoLayout::for_project(dir.path());
        std::fs::create_dir_all(layout.root()).expect("create .roko");
        let gone = exited_pid();
        write_open_run(dir.path(), "r-gone", gone, "2026-10-15T14:58:36.254Z");
        write_open_run(dir.path(), "r-live", std::process::id(), &rfc3339(now_ms()));
        let index = roko_fs::run_index::run_index_path(&layout.events_jsonl_path(), "r-gone")
            .expect("index path");
        std::fs::create_dir_all(index.parent().unwrap()).expect("index dir");
        // The writer was killed in the middle of a row.
        let started = json!({"type": "task_started", "run_id": "r-gone", "ts_millis": 1});
        std::fs::write(&index, format!("{started}\n{{\"type\":\"agent_out"))
            .expect("write the index");

        assert_eq!(settle_open_runs(dir.path()), ["r-gone"]);
        let ended = settle(dir.path(), "r-gone").expect("ended interrupted");
        assert!(!ended.settled_now, "it ended at start");
        assert_eq!(ended.reason, INTERRUPTED_REASON);
        assert!(settle(dir.path(), "r-live").is_none(), "its process runs");
        assert!(settle(dir.path(), "no-such-run").is_none());

        let manifest = RunProvenanceManifest::load(&layout.run_dir("r-gone"))
            .expect("load")
            .expect("manifest");
        let closed = manifest.closed.expect("closed");
        assert_eq!(closed.status, INTERRUPTED);
        assert_eq!(closed.ended_at, ended.at);
        assert_eq!(manifest.interruptions.len(), 1);
        assert_eq!(manifest.interruptions[0].pid, gone);
        assert_eq!(manifest.interruptions[0].inv, 1);

        let text = std::fs::read_to_string(&index).expect("read the index");
        let ends: Vec<Value> = text
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|row| row["type"] == "run_completed")
            .collect();
        assert_eq!(ends.len(), 1, "{text}");
        assert_eq!(ends[0]["outcome"], INTERRUPTED);
        assert_eq!(ends[0]["reason"], INTERRUPTED_REASON);
        assert_eq!(ends[0]["run_id"], "r-gone");
        assert!(
            text.contains("\"agent_out\n{"),
            "the partial row stays apart: {text}"
        );
    }
}
