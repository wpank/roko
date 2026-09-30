//! The run manifest of a Graph plan run (S01 §5.1, P0-2).
//!
//! Every checkpoint run keeps `.roko/runs/<run_id>/manifest.json` beside its
//! `attempts.jsonl`. It says which harness build, which config (by its
//! secret-redacted fingerprint) and which invocations produced the run's
//! records. [`RunManifests::open`] records an invocation when a plan's run
//! starts or resumes, and [`RunManifests::close`] records how it ended and
//! how many attempts it opened, settled and abandoned. A manifest that
//! cannot be written is logged; it never stops a run.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use roko_core::config::schema::RokoConfig;
use roko_fs::RokoLayout;
use roko_learn::telemetry::records::{
    ConfigHashProvenance, HarnessProvenance, RunClosed, RunInvocation, WorkspaceProvenance,
    b3_digest,
};
use roko_learn::telemetry::{AttemptTally, RunProvenanceManifest, TelemetryWriterStats};
use sha2::Digest as _;

use crate::graph_checkpoint::GraphCheckpointStatus;

/// `kind` of the manifests a Graph plan run writes.
const PLAN_RUN_KIND: &str = "plan_run";

/// The provenance every run of one plan-run invocation shares, and the runs
/// it reopened.
#[derive(Debug)]
pub struct RunManifests {
    /// `.roko/runs`.
    runs_dir: PathBuf,
    harness: HarnessProvenance,
    config: ConfigHashProvenance,
    workspace: WorkspaceProvenance,
    /// `sha256` of this process's command-line arguments.
    args_sha256: String,
    /// Records an earlier close of a run counted as dropped, per run this
    /// process reopened: its close adds them to its own.
    carried_drops: parking_lot::Mutex<HashMap<String, u64>>,
}

impl RunManifests {
    /// Capture the harness build, the fingerprint of `config` and the commit
    /// of the workspace at `workdir`, whose `.roko/runs` holds the manifests.
    #[must_use]
    pub fn capture(workdir: &Path, config: &RokoConfig) -> Self {
        let config = match roko_core::config::fingerprint(config) {
            Ok(fingerprint) => ConfigHashProvenance {
                hash: fingerprint.hash,
                params_digest: None,
                redacted_keys: fingerprint.redacted_keys,
            },
            Err(error) => {
                tracing::warn!(%error, "config fingerprint failed; run manifests record no config hash");
                ConfigHashProvenance::default()
            }
        };
        let base_commit = git_output(workdir, &["rev-parse", "HEAD"])
            .map(|head| String::from_utf8_lossy(&head).trim().to_string())
            .filter(|head| !head.is_empty());
        Self {
            runs_dir: RokoLayout::for_project(workdir).runs_dir(),
            harness: harness_provenance(),
            config,
            workspace: WorkspaceProvenance { base_commit },
            args_sha256: args_sha256(),
            carried_drops: parking_lot::Mutex::default(),
        }
    }

    /// Record that run `run_id` of plan `plan_id` starts in this process: a
    /// new manifest for a new run, one more invocation (a resume) for a run
    /// that has one. Returns the invocation's ordinal, or `None` when the
    /// manifest could not be read or written.
    pub fn open(&self, run_id: &str, plan_id: &str) -> Option<u32> {
        let run_dir = self.runs_dir.join(run_id);
        let mut manifest = match RunProvenanceManifest::load(&run_dir) {
            Ok(Some(manifest)) => {
                if manifest.harness.sha != self.harness.sha
                    || manifest.config.hash != self.config.hash
                {
                    tracing::warn!(
                        run_id,
                        "run resumed under another harness build or config; its manifest keeps \
                         the provenance of the invocation that started it"
                    );
                }
                manifest
            }
            Ok(None) => {
                let mut manifest = RunProvenanceManifest::new(run_id, PLAN_RUN_KIND);
                manifest.harness = self.harness.clone();
                manifest.config = self.config.clone();
                manifest.workspace = self.workspace.clone();
                manifest
            }
            Err(error) => {
                tracing::warn!(run_id, %error, "run manifest unreadable; not recording this invocation");
                return None;
            }
        };
        if !manifest.plan_ids.iter().any(|id| id == plan_id) {
            manifest.plan_ids.push(plan_id.to_string());
        }
        if let Some(closed) = &manifest.closed {
            self.carried_drops
                .lock()
                .insert(run_id.to_string(), closed.telemetry_dropped);
        }
        let inv = manifest.begin_invocation(RunInvocation {
            inv: 0,
            started_at: now_iso(),
            resumed: false,
            pid: std::process::id(),
            host: "local".to_string(),
            args_sha256: Some(self.args_sha256.clone()),
        });
        match manifest.store(&run_dir) {
            Ok(()) => Some(inv),
            Err(error) => {
                tracing::warn!(run_id, %error, "run manifest not written");
                None
            }
        }
    }

    /// Record that run `run_id` ended with `status`, with the attempts its
    /// `attempts.jsonl` holds and the records its attempt writer dropped
    /// (`writer`, from [`GraphTaskDispatcher::close_run_attempts`]).
    ///
    /// [`GraphTaskDispatcher::close_run_attempts`]: crate::graph_task_dispatch::GraphTaskDispatcher::close_run_attempts
    pub fn close(
        &self,
        run_id: &str,
        status: GraphCheckpointStatus,
        writer: Option<TelemetryWriterStats>,
    ) {
        let run_dir = self.runs_dir.join(run_id);
        let carried = self.carried_drops.lock().remove(run_id).unwrap_or(0);
        let mut manifest = match RunProvenanceManifest::load(&run_dir) {
            Ok(Some(manifest)) => manifest,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(run_id, %error, "run manifest unreadable; not closing it");
                return;
            }
        };
        let tally = match AttemptTally::read(&run_dir) {
            Ok(tally) => tally,
            Err(error) => {
                tracing::warn!(run_id, %error, "attempt log unreadable; the run manifest stays open");
                return;
            }
        };
        manifest.closed = Some(RunClosed {
            ended_at: now_iso(),
            status: status.as_str().to_string(),
            attempts_opened: tally.opened,
            attempts_settled: tally.settled,
            abandoned: tally.abandoned,
            telemetry_dropped: carried + writer.map_or(0, |stats| stats.dropped),
            runaway_guard_trips: 0,
        });
        if let Err(error) = manifest.store(&run_dir) {
            tracing::warn!(run_id, %error, "run manifest not closed");
        }
    }
}

/// The harness build: its commit and build-time dirty flag, and a digest of
/// the uncommitted changes of the source tree it was built from. Edits after
/// a build do not rerun the build script, so the tree is checked here too.
/// A build whose cleanliness is unknown counts as dirty.
fn harness_provenance() -> HarnessProvenance {
    let sha = env!("ROKO_GIT_HASH");
    let dirty_digest = source_tree_diff_digest(sha);
    let built_clean = option_env!("ROKO_GIT_DIRTY") == Some("false");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    HarnessProvenance {
        sha: sha.to_string(),
        dirty: !built_clean || dirty_digest.is_some(),
        dirty_digest,
        rustc: Some(env!("ROKO_RUSTC_VERSION").to_string()),
        profile: Some(profile.to_string()),
    }
}

/// `b3:` digest of `git diff HEAD` in the source tree this binary was built
/// from, when that tree is still at the build's commit and has uncommitted
/// changes. `None` when it is clean, has moved on, or is gone.
fn source_tree_diff_digest(build_sha: &str) -> Option<String> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let head = git_output(source, &["rev-parse", "HEAD"])?;
    if build_sha == "unknown" || !String::from_utf8_lossy(&head).trim().starts_with(build_sha) {
        return None;
    }
    let diff = git_output(
        source,
        &[
            "--no-optional-locks",
            "diff",
            "HEAD",
            "--no-ext-diff",
            "--binary",
        ],
    )?;
    (!diff.is_empty()).then(|| b3_digest(&diff))
}

/// Standard output of `git -C <dir> <args>`, when it succeeds.
fn git_output(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

/// `sha256` of this process's arguments, each followed by a NUL byte.
fn args_sha256() -> String {
    let mut hasher = sha2::Sha256::new();
    for arg in std::env::args_os() {
        hasher.update(arg.as_encoded_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
