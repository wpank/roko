//! Append-only JSONL persistence for routing decisions.
//!
//! Each routing decision is written once when the decision is made and may be
//! written again with outcome fields populated after the task completes. The
//! latest record for a given `trace_id` is therefore the canonical view.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::loop_audit::arm_set::ArmSet;
use crate::telemetry::records::{AuditFields, DecisionSource};

/// `decision_point` of every route decision row (S01 §5.3).
pub const ROUTE_DECISION_POINT: &str = "route";

/// Persisted routing-decision record.
///
/// It is also the route decision row of the S01 run telemetry
/// (`roko.decision/1`, see [`crate::telemetry`]): the fields after
/// `outcome_latency_ms` were added for it, and rows written before them
/// still parse. It keeps its older names for S01 §5.3's `chosen`
/// (`selected_model`) and a candidate's `id` (`model`), and reads S01's
/// spellings too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecisionLog {
    /// RFC 3339 timestamp for when the record was written.
    #[serde(default)]
    pub timestamp: String,
    /// Deterministic trace identifier for the task dispatch.
    #[serde(default)]
    pub trace_id: String,
    /// Stable task identifier within the plan.
    pub task_id: String,
    /// Originally requested model before routing/override logic.
    #[serde(default)]
    pub requested_model: String,
    /// Agent role requesting the model.
    #[serde(default)]
    pub role: String,
    /// Task complexity label used for routing.
    #[serde(default)]
    pub task_complexity: String,
    /// Task category label used for routing and calibration.
    #[serde(default)]
    pub task_category: String,
    /// Final provider selected for dispatch.
    #[serde(default)]
    pub selected_provider: String,
    /// Final model selected for dispatch (S01's `chosen`).
    #[serde(alias = "chosen")]
    pub selected_model: String,
    /// Routing stage responsible for the base decision.
    #[serde(default)]
    pub routing_stage: String,
    /// Human-readable machine-parsable reason for the final decision.
    #[serde(default)]
    pub routing_reason: String,
    /// Candidate set considered during routing.
    pub candidates: Vec<CandidateEntry>,
    /// Whether the routed turn ultimately succeeded.
    pub outcome_success: Option<bool>,
    /// Final observed turn cost in USD.
    pub outcome_cost_usd: Option<f64>,
    /// Final observed turn latency in milliseconds.
    pub outcome_latency_ms: Option<u64>,
    /// Attempt the decision belongs to (`AttemptKey::attempt_key`). Rows
    /// written before S01 have none.
    #[serde(default)]
    pub attempt_key: Option<String>,
    /// Who produced `selected_model` (S01 §5.3). A guard that rewrites the
    /// router's pick is `fallback`, not `router`.
    #[serde(default)]
    pub source: Option<DecisionSource>,
    /// The configured default the decision falls back to. It mirrors
    /// `proposals.default` for one release.
    #[serde(default)]
    pub default_model: Option<String>,
    /// Probability the logging policy gave `selected_model`
    /// (`chosen_propensity`); off-policy estimates need it. Rows written
    /// before that name call it `propensity`.
    #[serde(default, rename = "chosen_propensity", alias = "propensity")]
    pub propensity: Option<f64>,
    /// The decision point: always [`ROUTE_DECISION_POINT`].
    #[serde(default = "route_decision_point")]
    pub decision_point: String,
    /// What each policy proposed. `learned` apart from `selected_model` is
    /// what lets a reader count masked routes.
    #[serde(default)]
    pub proposals: RouteProposals,
    /// Why a guard replaced the cascade's pick, when `source` is `fallback`:
    /// `provider_unconfigured`, `provider_disabled` or `no_tool_support`.
    #[serde(default)]
    pub fallback_reason: Option<String>,
    /// Inputs besides the router's own state that could move its pick, and
    /// whether each was in force.
    #[serde(default)]
    pub influences: Vec<RouteInfluence>,
    /// The learned state the router decided from (S01 P0-10); `None`
    /// without a router.
    #[serde(default)]
    pub state: Option<DecisionState>,
    /// The arms of the attempt's chain (S02.P1-14), which every decision row
    /// of the attempt carries; `None` for a row written outside an attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm_set: Option<ArmSet>,
    /// S03's fields (A-DEC, S01 §5.3).
    #[serde(flatten)]
    pub audit: AuditFields,
}

fn route_decision_point() -> String {
    ROUTE_DECISION_POINT.to_string()
}

impl RoutingDecisionLog {
    /// Return a clone with terminal outcome fields populated.
    #[must_use]
    pub fn with_outcome(mut self, success: bool, cost_usd: f64, latency_ms: u64) -> Self {
        self.outcome_success = Some(success);
        self.outcome_cost_usd = Some(cost_usd);
        self.outcome_latency_ms = Some(latency_ms);
        self
    }
}

/// What each policy proposed for a route decision (S01 §5.3 `proposals`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RouteProposals {
    /// The cascade router's own pick, before any guard. With the ladder on
    /// it is the router's shadow pick. `None` when no cascade pick was made
    /// (an override, a task hint, the default).
    pub learned: Option<String>,
    /// The configured default model.
    pub default: Option<String>,
    /// The `[routing.ladder]` rung's model, when the ladder routed the task.
    pub ladder: Option<String>,
    /// A second, independent draw of a stochastic learned policy (S03's A/A
    /// floor); `None` for a deterministic one.
    pub aa: Option<String>,
}

/// An input besides the router's own state that could move its pick (S01
/// §5.3 `influences[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteInfluence {
    /// What the input is: `knowledge_weighting` or `provider_health`.
    pub kind: String,
    /// Whether it was in force for this pick.
    pub applied: bool,
}

/// The learned state a decision read (S01 §5.3 `state`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionState {
    /// Whether the state had learned anything when it was read.
    pub read: bool,
    /// Version label, e.g. `cr:obs=412`.
    pub version: String,
    /// `b3:` digest of the canonical learned state.
    pub digest: String,
    /// Seconds since the state last learned, when known.
    #[serde(default)]
    pub age_s: Option<u64>,
    /// Observations the state holds.
    pub n_obs: u64,
}

/// One candidate model score from the routing decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEntry {
    /// Candidate model slug (S01's `id`).
    #[serde(alias = "id")]
    pub model: String,
    /// Provider backing the model.
    #[serde(default)]
    pub provider: String,
    /// Stage-specific candidate score.
    pub score: f64,
    /// Whether the guards let the policy pick this candidate. Rows written
    /// before S01 have none and read as eligible.
    #[serde(default = "default_true")]
    pub eligible: bool,
    /// Optional reason the candidate could not be selected. Rows written
    /// before S01 call it `disqualified`.
    #[serde(default, alias = "disqualified")]
    pub ineligible_reason: Option<String>,
    /// Probability the logging policy gave this candidate; a decision's
    /// candidates sum to 1.
    #[serde(default)]
    pub p: Option<f64>,
}

impl CandidateEntry {
    /// A candidate with no probability yet, eligible unless
    /// `ineligible_reason` says why not.
    #[must_use]
    pub fn new(
        model: impl Into<String>,
        provider: impl Into<String>,
        score: f64,
        ineligible_reason: Option<String>,
    ) -> Self {
        Self {
            model: model.into(),
            provider: provider.into(),
            score,
            eligible: ineligible_reason.is_none(),
            ineligible_reason,
            p: None,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Per-decision metadata attached to a routing log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecisionMeta {
    /// Deterministic trace identifier for the task dispatch.
    pub trace_id: String,
    /// Stable task identifier within the plan.
    pub task_id: String,
    /// Originally requested model before routing/override logic.
    pub requested_model: String,
    /// Agent role requesting the model.
    pub role: String,
    /// Task complexity label used for routing.
    pub task_complexity: String,
    /// Task category label used for routing and calibration.
    pub task_category: String,
    /// Routing stage responsible for the base decision.
    pub routing_stage: String,
    /// Human-readable machine-parsable reason for the final decision.
    pub routing_reason: String,
}

/// Append-only JSONL log for [`RoutingDecisionLog`] values.
#[derive(Debug, Clone)]
pub struct RoutingDecisionLogStore {
    path: PathBuf,
    fsync: bool,
}

impl RoutingDecisionLogStore {
    /// Construct a log at `path`.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fsync: true,
        }
    }

    /// Create parent directories and return a log at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when parent directories cannot be created.
    pub async fn open_creating(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(Self { path, fsync: true })
    }

    /// Create parent directories and return a log at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when parent directories cannot be created.
    pub fn open_creating_blocking(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path, fsync: true })
    }

    /// Path to the underlying JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Disable fsync after appends.
    #[must_use]
    pub const fn without_fsync(mut self) -> Self {
        self.fsync = false;
        self
    }

    /// Append one [`RoutingDecisionLog`] as one JSON line.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append(&self, record: &RoutingDecisionLog) -> io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        if self.fsync {
            file.sync_data().await?;
        }
        Ok(())
    }

    /// Append one [`RoutingDecisionLog`] as one JSON line using blocking I/O.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub fn append_blocking(&self, record: &RoutingDecisionLog) -> io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        use std::io::Write as _;
        file.write_all(line.as_bytes())?;
        if self.fsync {
            file.sync_data()?;
        }
        Ok(())
    }

    /// Read all valid records; malformed lines are skipped.
    ///
    /// # Errors
    ///
    /// Returns an error only for file open/read failures.
    pub async fn read_all(&self) -> io::Result<Vec<RoutingDecisionLog>> {
        let file = match tokio::fs::File::open(&self.path).await {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut out = Vec::new();
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<RoutingDecisionLog>(trimmed) {
                out.push(record);
            }
        }
        Ok(out)
    }
}

/// Synchronous helper for recording routing decisions during model selection.
#[derive(Debug, Clone)]
pub struct RoutingLogger {
    store: RoutingDecisionLogStore,
    model_providers: HashMap<String, String>,
    disqualifications: HashMap<String, String>,
}

impl RoutingLogger {
    /// Create a new logger backed by `store`.
    #[must_use]
    pub fn new(store: RoutingDecisionLogStore) -> Self {
        Self {
            store,
            model_providers: HashMap::new(),
            disqualifications: HashMap::new(),
        }
    }

    /// Create parent directories and return a logger at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when parent directories cannot be created.
    pub fn open_creating(path: impl Into<PathBuf>) -> io::Result<Self> {
        Ok(Self::new(RoutingDecisionLogStore::open_creating_blocking(
            path,
        )?))
    }

    /// Seed the logger with a model -> provider mapping.
    #[must_use]
    pub fn with_model_providers(mut self, model_providers: HashMap<String, String>) -> Self {
        self.model_providers = model_providers;
        self
    }

    /// Seed the logger with explicit candidate disqualification reasons.
    #[must_use]
    pub fn with_disqualifications(mut self, disqualifications: HashMap<String, String>) -> Self {
        self.disqualifications = disqualifications;
        self
    }

    /// Return the backing store so callers can append completion records later.
    #[must_use]
    pub fn store(&self) -> RoutingDecisionLogStore {
        self.store.clone()
    }

    /// Resolve the provider for `model`, falling back to the model slug.
    #[must_use]
    pub fn provider_for_model(&self, model: &str) -> String {
        self.model_providers
            .get(model)
            .cloned()
            .unwrap_or_else(|| model.to_string())
    }

    /// Return the disqualification reason for `model`, if any.
    #[must_use]
    pub fn disqualified_reason(&self, model: &str) -> Option<String> {
        self.disqualifications.get(model).cloned()
    }

    /// Append a routing decision immediately.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub fn append(&self, record: &RoutingDecisionLog) -> io::Result<()> {
        self.store.append_blocking(record)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateEntry, DecisionState, ROUTE_DECISION_POINT, RouteProposals, RoutingDecisionLog,
        RoutingDecisionLogStore,
    };
    use crate::telemetry::records::{DECISION_SCHEMA, DecisionSource, Stamped};
    use tempfile::TempDir;

    /// S01 §5.3's route decision example, with its elisions filled in.
    const S01_ROUTE_DECISION: &str = r#"{
        "schema_version": "roko.decision/1", "record_id": "b3:5a0f", "seq": 19,
        "ts": "2026-10-02T14:03:11.418Z",
        "run_id": "gr-7f3c2a91", "plan_id": "loop-census", "task_id": "T4", "node_id": "task:T4",
        "attempt": 1, "inv": 1, "attempt_key": "gr-7f3c2a91:loop-census:T4:1",
        "chain_key": "gr-7f3c2a91:loop-census:T4",
        "decision_point": "route", "loop_id": "L-route", "loop_ids": ["L-route", "L-M3"],
        "layer": "route",
        "opportunity": {"eligible": true, "reason": "no_override_no_hint_ge2_eligible"},
        "assignment": {
            "unit": "chain", "unit_key": "gr-7f3c2a91:loop-census:T4", "layer": "route",
            "salt_id": "route@2026-10-02", "audit_epoch": "2026-10-02", "u": 0.7312, "h": 0.0,
            "g": 0.0, "global_off": false, "arm": "learned", "propensity": 1.0,
            "assigned_at": 1759413791410
        },
        "policy": "cascade_linucb_argmax",
        "candidates": [
            {"id": "kimi-k2", "score": 0.84, "eligible": false,
             "ineligible_reason": "provider_unconfigured", "p": 0.0},
            {"id": "gpt-oss-120b", "score": 0.81, "eligible": true, "p": 1.0},
            {"id": "glm-4.6", "score": 0.77, "eligible": true, "p": 0.0}
        ],
        "proposals": {"learned": "kimi-k2", "default": "gpt-oss-120b", "ladder": null, "aa": null},
        "chosen": "gpt-oss-120b", "chosen_propensity": 1.0, "source": "fallback",
        "fallback_reason": "provider_unconfigured",
        "state": {
            "read": true, "version": "cr:obs=412", "digest": "b3:77e0", "age_s": 5400,
            "n_obs": 412
        },
        "influences": [
            {"kind": "knowledge_weighting", "applied": false},
            {"kind": "provider_health", "applied": true}
        ],
        "decided_at": 1759413791418,
        "receipt": {"kind": "route", "ok": true, "request_hash": "b3:5e1c", "exposure_hashes": []},
        "params_digest": "b3:11aa", "prediction_id": null
    }"#;

    /// A route decision row as the cascade router wrote it before S01.
    const PRE_S01_ROW: &str = r#"{
        "timestamp": "2026-04-12T08:30:00Z", "trace_id": "trace-123", "task_id": "task-2m13",
        "requested_model": "kimi-k2.5", "role": "implementer",
        "task_complexity": "architectural", "task_category": "implementation",
        "selected_provider": "zai", "selected_model": "glm-5.1", "routing_stage": "ucb",
        "routing_reason": "highest_ucb_score",
        "candidates": [
            {"model": "glm-5.1", "provider": "zai", "score": 0.91, "disqualified": null},
            {"model": "kimi-k2.5", "provider": "moonshot", "score": 0.77,
             "disqualified": "provider_unhealthy"}
        ],
        "outcome_success": null, "outcome_cost_usd": null, "outcome_latency_ms": null
    }"#;

    fn record() -> RoutingDecisionLog {
        RoutingDecisionLog {
            timestamp: "2026-04-12T08:30:00Z".to_string(),
            trace_id: "trace-123".to_string(),
            task_id: "task-2m13".to_string(),
            requested_model: "kimi-k2.5".to_string(),
            role: "implementer".to_string(),
            task_complexity: "architectural".to_string(),
            task_category: "implementation".to_string(),
            selected_provider: "zai".to_string(),
            selected_model: "glm-5.1".to_string(),
            routing_stage: "ucb".to_string(),
            routing_reason: "highest_ucb_score".to_string(),
            candidates: vec![
                CandidateEntry::new("glm-5.1", "zai", 0.91, None),
                CandidateEntry::new(
                    "kimi-k2.5",
                    "moonshot",
                    0.77,
                    Some("provider_unhealthy".to_string()),
                ),
            ],
            outcome_success: None,
            outcome_cost_usd: None,
            outcome_latency_ms: None,
            attempt_key: None,
            source: None,
            default_model: None,
            propensity: None,
            decision_point: ROUTE_DECISION_POINT.to_string(),
            proposals: RouteProposals::default(),
            fallback_reason: None,
            influences: Vec::new(),
            state: None,
            arm_set: None,
            audit: Default::default(),
        }
    }

    #[tokio::test]
    async fn routing_decision_log_append_and_read_roundtrip() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("routing.jsonl");
        let log = RoutingDecisionLogStore::at(&path).without_fsync();
        let pending = record();
        let completed = pending.clone().with_outcome(true, 0.42, 1_250);

        log.append(&pending).await.expect("append pending");
        log.append(&completed).await.expect("append completed");

        let all = log.read_all().await.expect("read all");
        assert_eq!(all, vec![pending, completed]);
    }

    #[test]
    fn route_decision_row_round_trips_s01_example() {
        let line: Stamped<RoutingDecisionLog> =
            serde_json::from_str(S01_ROUTE_DECISION).expect("S01 §5.3's example");
        assert_eq!(line.schema_version, DECISION_SCHEMA);
        assert_eq!(line.seq, 19);
        let row = &line.record;
        assert_eq!(row.decision_point, ROUTE_DECISION_POINT);
        assert_eq!(
            row.attempt_key.as_deref(),
            Some("gr-7f3c2a91:loop-census:T4:1")
        );
        assert_eq!(row.selected_model, "gpt-oss-120b", "S01's `chosen`");
        assert_eq!(row.propensity, Some(1.0));
        assert_eq!(row.source, Some(DecisionSource::Fallback));
        assert_eq!(
            row.fallback_reason.as_deref(),
            Some("provider_unconfigured")
        );
        assert_eq!(
            row.proposals,
            RouteProposals {
                learned: Some("kimi-k2".to_string()),
                default: Some("gpt-oss-120b".to_string()),
                ladder: None,
                aa: None,
            }
        );
        let ineligible = &row.candidates[0];
        assert_eq!(ineligible.model, "kimi-k2", "S01's `id`");
        assert!(!ineligible.eligible);
        assert_eq!(
            ineligible.ineligible_reason.as_deref(),
            Some("provider_unconfigured")
        );
        assert!(row.candidates[1].eligible);
        let total: f64 = row.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "candidate p sums to {total}");
        assert_eq!(
            row.state,
            Some(DecisionState {
                read: true,
                version: "cr:obs=412".to_string(),
                digest: "b3:77e0".to_string(),
                age_s: Some(5_400),
                n_obs: 412,
            })
        );
        let influences: Vec<(&str, bool)> = row
            .influences
            .iter()
            .map(|influence| (influence.kind.as_str(), influence.applied))
            .collect();
        assert_eq!(
            influences,
            [("knowledge_weighting", false), ("provider_health", true)]
        );

        // Written back, the row keeps its own spellings and reads the same.
        let json = serde_json::to_value(&line).expect("serialize");
        assert_eq!(json["selected_model"], "gpt-oss-120b");
        assert_eq!(json["chosen_propensity"], 1.0);
        assert!(json.get("propensity").is_none());
        assert_eq!(
            json["candidates"][0]["ineligible_reason"],
            "provider_unconfigured"
        );
        assert_eq!(json["proposals"]["learned"], "kimi-k2");
        let back: Stamped<RoutingDecisionLog> = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, line);

        // A row written before S01 still parses, and so does one that names
        // `chosen_propensity` by its earlier name.
        let mut old: serde_json::Value = serde_json::from_str(PRE_S01_ROW).expect("old JSON");
        let pre_s01: RoutingDecisionLog = serde_json::from_value(old.clone()).expect("pre-S01 row");
        assert_eq!(pre_s01.decision_point, ROUTE_DECISION_POINT);
        assert_eq!(pre_s01.proposals, RouteProposals::default());
        assert!(pre_s01.influences.is_empty());
        assert_eq!(pre_s01.state, None);
        assert!(pre_s01.candidates[0].eligible);
        assert_eq!(pre_s01.candidates[0].p, None);
        assert_eq!(
            pre_s01.candidates[1].ineligible_reason.as_deref(),
            Some("provider_unhealthy")
        );
        old["propensity"] = serde_json::json!(0.25);
        let earlier: RoutingDecisionLog = serde_json::from_value(old).expect("earlier row");
        assert_eq!(earlier.propensity, Some(0.25));
    }

    #[tokio::test]
    async fn routing_decision_log_read_all_skips_malformed_lines() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("routing.jsonl");
        let record = record();

        tokio::fs::write(
            &path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(&record).expect("serialize"),
                "{ bad json",
                serde_json::to_string(&record.with_outcome(false, 0.1, 250)).expect("serialize"),
            ),
        )
        .await
        .expect("write log");

        let log = RoutingDecisionLogStore::at(&path).without_fsync();
        let all = log.read_all().await.expect("read all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].trace_id, "trace-123");
        assert_eq!(all[1].outcome_success, Some(false));
    }
}
