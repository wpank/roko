//! Durable checkpoints for CLI plan execution through the Graph Engine.
//!
//! The Graph Engine records successful Activity outputs as JSONL.  This module
//! adds the small amount of run metadata needed to resume those recordings
//! safely: a schema version, a stable fingerprint of the converted graph, and
//! the run ID that scopes every record.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use roko_graph::{ActivityRecorder, ActivityReplayer, Graph, graph_execution_fingerprint};
use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA_VERSION: u32 = 2;
const COST_LEDGER_SCHEMA_VERSION: u32 = 1;
const DEFAULT_RUNNER_RESUME_PATH: &str = ".roko/state/executor.json";

/// Lifecycle state persisted beside a Graph Activity recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphCheckpointStatus {
    /// The process may have stopped before every node reached a terminal state.
    Running,
    /// Every graph node completed successfully.
    Succeeded,
    /// At least one graph node failed.
    Failed,
}

/// Versioned metadata that makes an Activity JSONL file safe to resume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphCheckpointManifest {
    /// On-disk schema version.
    pub schema_version: u32,
    /// Plan/graph identifier.
    pub plan_id: String,
    /// SHA-256 of the execution-relevant converted graph definition.
    pub graph_fingerprint: String,
    /// Run ID stored in every Activity record.
    pub run_id: String,
    /// Activity log filename, relative to this manifest.
    pub activity_log: String,
    /// Actual-provider-cost ledger filename, relative to this manifest.
    #[serde(default)]
    pub cost_ledger: String,
    /// Last known run state.
    pub status: GraphCheckpointStatus,
    /// Last manifest update as Unix milliseconds.
    pub updated_at_ms: u128,
}

/// Resolved files for one plan's Graph checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCheckpointPaths {
    /// Versioned JSON manifest.
    pub manifest: PathBuf,
    /// Append-only Activity output log.
    pub activities: PathBuf,
    /// Atomically replaced actual-provider-cost state.
    pub costs: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GraphCostLedgerState {
    schema_version: u32,
    plan_id: String,
    graph_fingerprint: String,
    run_id: String,
    spent_micro_usd: u64,
    reserved_micro_usd: u64,
}

/// Durable actual-provider-cost state bound to one Graph checkpoint identity.
#[derive(Debug, Clone)]
pub struct GraphCostLedgerCheckpoint {
    path: PathBuf,
    identity: GraphCostLedgerState,
}

impl GraphCostLedgerCheckpoint {
    /// Restored actual spend in millionths of one USD.
    #[must_use]
    pub const fn spent_micro_usd(&self) -> u64 {
        self.identity.spent_micro_usd
    }

    /// Persist the latest actual spend atomically.
    pub(crate) fn persist(&self, spent_micro_usd: u64, reserved_micro_usd: u64) -> Result<()> {
        let mut state = self.identity.clone();
        state.spent_micro_usd = spent_micro_usd;
        state.reserved_micro_usd = reserved_micro_usd;
        write_cost_ledger_atomic(&self.path, &state)
    }

    fn load(
        path: PathBuf,
        manifest: &GraphCheckpointManifest,
    ) -> Result<GraphCostLedgerCheckpoint> {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read Graph cost ledger {}", path.display()))?;
        let state: GraphCostLedgerState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph cost ledger {}", path.display()))?;
        validate_cost_ledger_state(&state, manifest)
            .with_context(|| format!("validate Graph cost ledger {}", path.display()))?;
        Ok(Self {
            path,
            identity: state,
        })
    }
}

/// Recorder/replayer pair prepared for one Graph execution.
pub struct PreparedGraphCheckpoint {
    paths: GraphCheckpointPaths,
    manifest: GraphCheckpointManifest,
    recorder: Option<ActivityRecorder>,
    replayer: Option<ActivityReplayer>,
    replayed_entries: usize,
    cost_ledger: Option<GraphCostLedgerCheckpoint>,
}

impl PreparedGraphCheckpoint {
    /// Run ID shared by telemetry, the manifest, and JSONL records.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.manifest.run_id
    }

    /// Number of previously completed Activities available for replay.
    #[must_use]
    pub const fn replayed_entries(&self) -> usize {
        self.replayed_entries
    }

    /// Resolved checkpoint paths.
    #[must_use]
    pub const fn paths(&self) -> &GraphCheckpointPaths {
        &self.paths
    }

    /// Move the prepared recorder into a Graph Engine.
    pub fn take_recorder(&mut self) -> ActivityRecorder {
        self.recorder
            .take()
            .expect("Graph checkpoint recorder may only be taken once")
    }

    /// Move the optional replayer into a Graph Engine.
    pub fn take_replayer(&mut self) -> Option<ActivityReplayer> {
        self.replayer.take()
    }

    /// Move the restored durable cost ledger into the task dispatcher.
    pub fn take_cost_ledger(&mut self) -> GraphCostLedgerCheckpoint {
        self.cost_ledger
            .take()
            .expect("Graph cost ledger may only be taken once")
    }

    /// Persist the terminal state after Graph execution.
    pub fn finish(&mut self, succeeded: bool) -> Result<()> {
        self.manifest.status = if succeeded {
            GraphCheckpointStatus::Succeeded
        } else {
            GraphCheckpointStatus::Failed
        };
        self.manifest.updated_at_ms = unix_ms();
        write_manifest_atomic(&self.paths.manifest, &self.manifest)
    }
}

/// Prepare a fresh or resumed checkpoint for one converted plan graph.
///
/// `requested_path` behaves as a checkpoint directory unless it has a `.json`
/// extension.  A file path is only unambiguous for a single-plan invocation.
/// The CLI's historical bare `--resume-plan` default is mapped to the canonical
/// Graph checkpoint root instead of overwriting Runner v2's executor snapshot.
pub fn prepare_graph_checkpoint(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
    graph: &Graph,
    fresh: bool,
    force_resume: bool,
) -> Result<PreparedGraphCheckpoint> {
    let paths = resolve_checkpoint_paths(workdir, requested_path, plan_id, plan_count)?;
    let fingerprint = graph_execution_fingerprint(graph).context("fingerprint Graph")?;

    if fresh {
        archive_checkpoint_files(&paths)?;
    }

    if !fresh && paths.manifest.exists() {
        let bytes = std::fs::read(&paths.manifest)
            .with_context(|| format!("read Graph checkpoint {}", paths.manifest.display()))?;
        let mut manifest: GraphCheckpointManifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph checkpoint {}", paths.manifest.display()))?;
        let mismatch = checkpoint_mismatch(&manifest, plan_id, &fingerprint, &paths);

        if let Some(reason) = mismatch {
            if !force_resume {
                bail!(
                    "cannot resume Graph checkpoint {}: {reason}; use --fresh to archive it or --force-resume to start a new run",
                    paths.manifest.display()
                );
            }
            archive_checkpoint_files(&paths)?;
        } else {
            if (!paths.activities.is_file() || !paths.costs.is_file()) && !force_resume {
                bail!(
                    "Graph checkpoint {} references missing Activity log or cost ledger ({}, {}); use --fresh or --force-resume",
                    paths.manifest.display(),
                    paths.activities.display(),
                    paths.costs.display(),
                );
            }
            if !paths.activities.is_file() || !paths.costs.is_file() {
                archive_checkpoint_files(&paths)?;
                return create_fresh_checkpoint(paths, plan_id, fingerprint);
            }
            let cost_ledger = match GraphCostLedgerCheckpoint::load(paths.costs.clone(), &manifest)
            {
                Ok(cost_ledger) => cost_ledger,
                Err(_) if force_resume => {
                    archive_checkpoint_files(&paths)?;
                    return create_fresh_checkpoint(paths, plan_id, fingerprint);
                }
                Err(error) => return Err(error),
            };
            let replayer =
                ActivityReplayer::load_scoped(&paths.activities, plan_id, &manifest.run_id)
                    .with_context(|| {
                        format!(
                            "load Graph Activity checkpoint {}",
                            paths.activities.display()
                        )
                    })?;
            let replayed_entries = replayer.entry_count();
            let recorder = ActivityRecorder::create(&manifest.run_id, &paths.activities)
                .with_context(|| {
                    format!(
                        "open Graph Activity checkpoint {}",
                        paths.activities.display()
                    )
                })?;
            manifest.status = GraphCheckpointStatus::Running;
            manifest.updated_at_ms = unix_ms();
            write_manifest_atomic(&paths.manifest, &manifest)?;
            return Ok(PreparedGraphCheckpoint {
                paths,
                manifest,
                recorder: Some(recorder),
                replayer: Some(replayer),
                replayed_entries,
                cost_ledger: Some(cost_ledger),
            });
        }
    } else if !fresh && (paths.activities.exists() || paths.costs.exists()) {
        if !force_resume {
            bail!(
                "found Graph Activity log or cost ledger without its manifest ({}, {}); use --fresh or --force-resume",
                paths.activities.display(),
                paths.costs.display(),
            );
        }
        archive_checkpoint_files(&paths)?;
    }

    create_fresh_checkpoint(paths, plan_id, fingerprint)
}

fn create_fresh_checkpoint(
    paths: GraphCheckpointPaths,
    plan_id: &str,
    fingerprint: String,
) -> Result<PreparedGraphCheckpoint> {
    let parent = paths
        .manifest
        .parent()
        .context("Graph checkpoint manifest has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let run_id = format!("graph-{plan_id}-{}", uuid::Uuid::new_v4());
    let activity_log = paths
        .activities
        .file_name()
        .and_then(|name| name.to_str())
        .context("Graph Activity checkpoint filename is not valid UTF-8")?
        .to_string();
    let cost_ledger = paths
        .costs
        .file_name()
        .and_then(|name| name.to_str())
        .context("Graph cost ledger filename is not valid UTF-8")?
        .to_string();
    let manifest = GraphCheckpointManifest {
        schema_version: CHECKPOINT_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        graph_fingerprint: fingerprint.clone(),
        run_id: run_id.clone(),
        activity_log,
        cost_ledger,
        status: GraphCheckpointStatus::Running,
        updated_at_ms: unix_ms(),
    };
    let recorder = ActivityRecorder::create_fresh(&run_id, &paths.activities)
        .with_context(|| format!("create Graph Activity log {}", paths.activities.display()))?;
    let cost_ledger = GraphCostLedgerCheckpoint {
        path: paths.costs.clone(),
        identity: GraphCostLedgerState {
            schema_version: COST_LEDGER_SCHEMA_VERSION,
            plan_id: plan_id.to_string(),
            graph_fingerprint: fingerprint.clone(),
            run_id,
            spent_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    };
    cost_ledger.persist(0, 0)?;
    write_manifest_atomic(&paths.manifest, &manifest)?;
    Ok(PreparedGraphCheckpoint {
        paths,
        manifest,
        recorder: Some(recorder),
        replayer: None,
        replayed_entries: 0,
        cost_ledger: Some(cost_ledger),
    })
}

fn resolve_checkpoint_paths(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
) -> Result<GraphCheckpointPaths> {
    let canonical_graph_root = workdir.join(".roko/state/graph");
    let requested = requested_path.map(|path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            workdir.join(path)
        }
    });
    let runner_default = workdir.join(DEFAULT_RUNNER_RESUME_PATH);
    let base = match requested {
        None => canonical_graph_root,
        Some(path) if path == runner_default => canonical_graph_root,
        Some(path) => path,
    };

    let is_manifest = base
        .extension()
        .is_some_and(|extension| extension == "json")
        && !base.is_dir();
    if is_manifest {
        if plan_count != 1 {
            bail!(
                "a Graph checkpoint file can only resume one plan; pass a directory when running {plan_count} plans"
            );
        }
        let stem = base
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("Graph checkpoint filename is not valid UTF-8")?;
        let activities = base.with_file_name(format!("{stem}.activities.jsonl"));
        let costs = base.with_file_name(format!("{stem}.costs.json"));
        return Ok(GraphCheckpointPaths {
            manifest: base,
            activities,
            costs,
        });
    }

    let plan_dir = base.join(safe_plan_component(plan_id));
    Ok(GraphCheckpointPaths {
        manifest: plan_dir.join("checkpoint.json"),
        activities: plan_dir.join("activities.jsonl"),
        costs: plan_dir.join("costs.json"),
    })
}

fn checkpoint_mismatch(
    manifest: &GraphCheckpointManifest,
    plan_id: &str,
    fingerprint: &str,
    paths: &GraphCheckpointPaths,
) -> Option<String> {
    if manifest.schema_version != CHECKPOINT_SCHEMA_VERSION {
        return Some(format!(
            "schema version {} is unsupported (expected {CHECKPOINT_SCHEMA_VERSION})",
            manifest.schema_version
        ));
    }
    if manifest.plan_id != plan_id {
        return Some(format!(
            "manifest is for plan '{}' rather than '{plan_id}'",
            manifest.plan_id
        ));
    }
    if manifest.graph_fingerprint != fingerprint {
        return Some("the converted plan graph has changed".to_string());
    }
    if paths.activities.file_name().and_then(|name| name.to_str())
        != Some(manifest.activity_log.as_str())
    {
        return Some("manifest references a different Activity log".to_string());
    }
    if paths.costs.file_name().and_then(|name| name.to_str()) != Some(manifest.cost_ledger.as_str())
    {
        return Some("manifest references a different cost ledger".to_string());
    }
    None
}

fn validate_cost_ledger_state(
    state: &GraphCostLedgerState,
    manifest: &GraphCheckpointManifest,
) -> Result<()> {
    if state.schema_version != COST_LEDGER_SCHEMA_VERSION {
        bail!(
            "cost ledger schema version {} is unsupported (expected {COST_LEDGER_SCHEMA_VERSION})",
            state.schema_version
        );
    }
    if state.plan_id != manifest.plan_id {
        bail!(
            "cost ledger is for plan '{}' rather than '{}'",
            state.plan_id,
            manifest.plan_id
        );
    }
    if state.graph_fingerprint != manifest.graph_fingerprint {
        bail!("cost ledger graph fingerprint does not match the checkpoint manifest");
    }
    if state.run_id != manifest.run_id {
        bail!("cost ledger run ID does not match the checkpoint manifest");
    }
    if state.reserved_micro_usd > 0 {
        bail!(
            "cost ledger contains {} unresolved reserved micro-USD from an interrupted provider call",
            state.reserved_micro_usd
        );
    }
    Ok(())
}

fn archive_checkpoint_files(paths: &GraphCheckpointPaths) -> Result<()> {
    let timestamp = unix_ms();
    for path in [&paths.manifest, &paths.activities, &paths.costs] {
        if path.exists() {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("state");
            let backup = path.with_extension(format!("{extension}.bak.{timestamp}"));
            std::fs::rename(path, &backup).with_context(|| {
                format!(
                    "archive Graph checkpoint {} to {}",
                    path.display(),
                    backup.display()
                )
            })?;
        }
    }
    Ok(())
}

fn write_cost_ledger_atomic(path: &Path, state: &GraphCostLedgerState) -> Result<()> {
    let parent = path
        .parent()
        .context("Graph cost ledger has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(state).context("serialize Graph cost ledger")?;
    std::fs::write(&temporary, bytes)
        .with_context(|| format!("write Graph cost ledger {}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .with_context(|| format!("commit Graph cost ledger {}", path.display()))
}

fn write_manifest_atomic(path: &Path, manifest: &GraphCheckpointManifest) -> Result<()> {
    let parent = path
        .parent()
        .context("Graph checkpoint manifest has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(manifest).context("serialize Graph checkpoint")?;
    std::fs::write(&temporary, bytes)
        .with_context(|| format!("write Graph checkpoint {}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .with_context(|| format!("commit Graph checkpoint {}", path.display()))
}

fn safe_plan_component(plan_id: &str) -> String {
    let safe: String = plan_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() || safe == "." || safe == ".." {
        "plan".to_string()
    } else {
        safe
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_graph::{GraphMetadata, Node};
    use tempfile::tempdir;

    fn graph(name: &str, config_value: i64) -> Graph {
        let mut graph = Graph::new(GraphMetadata {
            name: name.to_string(),
            ..GraphMetadata::default()
        });
        graph
            .add_node(Node {
                id: "task-1".to_string(),
                cell_type: "task-executor".to_string(),
                config: toml::Value::Table(toml::map::Map::from_iter([(
                    "value".to_string(),
                    toml::Value::Integer(config_value),
                )])),
                inputs: Vec::new(),
                outputs: Vec::new(),
                execution_class: roko_graph::ExecutionClass::Activity,
            })
            .expect("node");
        graph
    }

    #[test]
    fn fingerprint_changes_with_execution_config() {
        assert_ne!(
            graph_execution_fingerprint(&graph("p", 1)).expect("fingerprint"),
            graph_execution_fingerprint(&graph("p", 2)).expect("fingerprint")
        );
    }

    #[test]
    fn fresh_checkpoint_then_resume_roundtrips_run_identity() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let run_id = fresh.run_id().to_string();
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh.finish(false).expect("finish");

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        assert_eq!(resumed.run_id(), run_id);
        assert_eq!(resumed.replayed_entries(), 1);
    }

    #[test]
    fn actual_provider_cost_roundtrips_across_resume() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let ledger = fresh.take_cost_ledger();
        ledger.persist(375_000, 0).expect("persist cost");
        fresh.finish(false).expect("finish");

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");

        assert_eq!(resumed.take_cost_ledger().spent_micro_usd(), 375_000);
    }

    #[test]
    fn corrupt_cost_ledger_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let costs = checkpoint.paths().costs.clone();
        drop(checkpoint);
        std::fs::write(&costs, b"not-json").expect("corrupt cost ledger");

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("corrupt cost ledger must fail closed");

        assert!(error.to_string().contains("parse Graph cost ledger"));
    }

    #[test]
    fn mismatched_cost_ledger_identity_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let costs = checkpoint.paths().costs.clone();
        drop(checkpoint);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&costs).expect("read cost ledger"))
                .expect("parse cost ledger");
        value["run_id"] = serde_json::Value::String("wrong-run".to_string());
        std::fs::write(
            &costs,
            serde_json::to_vec_pretty(&value).expect("serialize mismatch"),
        )
        .expect("write mismatched ledger");

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("mismatched cost ledger must fail closed");

        assert!(error.to_string().contains("validate Graph cost ledger"));
    }

    #[test]
    fn unresolved_crash_reservation_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        checkpoint
            .take_cost_ledger()
            .persist(100_000, 250_000)
            .expect("persist unresolved reservation");
        drop(checkpoint);

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("unresolved reservation must fail closed");

        assert!(format!("{error:#}").contains("unresolved reserved micro-USD"));
    }

    #[test]
    fn graph_drift_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph_v1 = graph("p", 1);
        let checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph_v1, false, false)
                .expect("fresh checkpoint");
        drop(checkpoint);

        let error =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph("p", 2), false, false)
                .err()
                .expect("drift must fail");
        assert!(error.to_string().contains("graph has changed"));
    }

    #[test]
    fn force_resume_archives_drifted_state_and_starts_new_run() {
        let dir = tempdir().expect("tempdir");
        let graph_v1 = graph("p", 1);
        let old = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph_v1, false, false)
            .expect("fresh checkpoint");
        let old_run = old.run_id().to_string();
        drop(old);

        let replacement =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph("p", 2), false, true)
                .expect("replacement checkpoint");
        assert_ne!(replacement.run_id(), old_run);
        assert_eq!(replacement.replayed_entries(), 0);
    }

    #[test]
    fn explicit_manifest_is_rejected_for_multiple_plans() {
        let dir = tempdir().expect("tempdir");
        let error =
            resolve_checkpoint_paths(dir.path(), Some(Path::new("checkpoint.json")), "p", 2)
                .expect_err("ambiguous file must fail");
        assert!(error.to_string().contains("only resume one plan"));
    }
}
