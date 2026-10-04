//! FeedbackService — concrete implementation of `FeedbackSink`.
//!
//! Records model call feedback, gate results, and workflow outcomes into the
//! existing learning infrastructure as append-only efficiency JSONL events.

use crate::cascade_router::CascadeRouter;
use crate::costs_db::CostRecord;
use crate::efficiency::FEEDBACK_EVENT_SCHEMA;
use crate::episode_logger::{Episode, EpisodeGateVerdict, EpisodeLogger, Usage};
use crate::model_call_feedback::{ModelCallJournal, observe_model_call_on_router};
use crate::section_effect::SectionEffectivenessRegistry;
use crate::telemetry::CostSource;
use async_trait::async_trait;
use chrono::Utc;
use roko_core::foundation::{FeedbackEvent, FeedbackSink};
use roko_core::{Result, RokoError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const KNOWLEDGE_FEEDBACK_FILE: &str = "knowledge-feedback.jsonl";
const KNOWLEDGE_SCORES_FILE: &str = "knowledge-scores.json";
const SECTION_EFFECTS_FILE: &str = "section-effects.json";

/// Outcome assigned to prompt sections and knowledge entries after evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeOutcome {
    /// The prompt/knowledge contributed to a passing outcome.
    Success,
    /// The prompt/knowledge contributed to a failing outcome.
    Failure,
    /// The model call succeeded, but the workflow/gate outcome is not known yet.
    Partial,
}

impl KnowledgeOutcome {
    fn score_delta(self) -> i64 {
        match self {
            Self::Success => 1,
            Self::Failure => -1,
            Self::Partial => 0,
        }
    }

    fn passed(self) -> Option<bool> {
        match self {
            Self::Success => Some(true),
            Self::Failure => Some(false),
            Self::Partial => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ProvenanceRecord {
    run_id: Option<String>,
    request_id: String,
    prompt_section_ids: Vec<String>,
    knowledge_ids: Vec<String>,
    role: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct KnowledgeScoreSnapshot {
    scores: HashMap<String, i64>,
}

/// Service that records feedback events for the learning subsystem.
///
/// This is the canonical way to record workflow feedback. It logs model call
/// metrics, gate results, and workflow outcomes into `.roko/learn`-style data
/// files so downstream learning components can consume them.
pub struct FeedbackService {
    /// Directory for feedback data files.
    data_dir: PathBuf,
    /// In-memory buffer of recent events for batched writes.
    buffer: Mutex<Vec<FeedbackEvent>>,
    /// Maximum buffer size before flushing.
    buffer_capacity: usize,
    /// Optional episode logger for workflow outcome records.
    episode_logger: Option<EpisodeLogger>,
    /// Optional cascade router for eager model-call reward observations.
    cascade_router: Option<Arc<CascadeRouter>>,
    /// Optional WAL journal for those observations (find-0dc1d5).
    cascade_journal: Option<Arc<ModelCallJournal>>,
    /// Cost log each model call that reached a provider is recorded in,
    /// when the caller records its calls' costs here (bug-c1f6b8).
    costs_path: Option<PathBuf>,
    /// Model-call provenance waiting for a gate/workflow outcome.
    provenance: Mutex<HashMap<String, ProvenanceRecord>>,
    /// Durable score for each knowledge entry.
    knowledge_scores: Mutex<HashMap<String, i64>>,
    /// Prompt-section effectiveness registry consumed by prompt assembly.
    section_effectiveness: Mutex<SectionEffectivenessRegistry>,
    /// Whether an outcome changed a knowledge score or a section effect
    /// since they were loaded or last saved; only then does a flush save
    /// them.
    scores_changed: AtomicBool,
}

impl FeedbackService {
    /// Create a new service writing to the given data directory.
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        let knowledge_scores = load_knowledge_scores(&data_dir.join(KNOWLEDGE_SCORES_FILE));
        let section_effectiveness =
            SectionEffectivenessRegistry::load_or_new(&data_dir.join(SECTION_EFFECTS_FILE));
        Self {
            data_dir,
            buffer: Mutex::new(Vec::with_capacity(64)),
            buffer_capacity: 64,
            episode_logger: None,
            cascade_router: None,
            cascade_journal: None,
            costs_path: None,
            provenance: Mutex::new(HashMap::new()),
            knowledge_scores: Mutex::new(knowledge_scores),
            section_effectiveness: Mutex::new(section_effectiveness),
            scores_changed: AtomicBool::new(false),
        }
    }

    /// Create a service from the standard `.roko` directory.
    #[must_use]
    pub fn from_roko_dir(roko_dir: &Path) -> Self {
        Self::new(roko_dir.join("learn"))
    }

    /// Attach an episode logger to record workflow outcomes as episodes.
    ///
    /// When a `WorkflowComplete` event is flushed, the service will also
    /// append an `Episode` record to the logger's JSONL file.
    #[must_use]
    pub fn with_episode_logger(mut self, logger: EpisodeLogger) -> Self {
        self.episode_logger = Some(logger);
        self
    }

    /// Attach a cascade router for bandit reward observations.
    ///
    /// On each `ModelCall` event, the service records the call's outcome on
    /// the router (a success with reward 1, or a failed trial with reward 0)
    /// so the bandit can update its model selection policy.
    #[must_use]
    pub fn with_cascade_router(mut self, router: Arc<CascadeRouter>) -> Self {
        self.cascade_router = Some(router);
        self
    }

    /// Journal the cascade router's model-call observations in the learning
    /// WAL before applying them (find-0dc1d5).
    ///
    /// Whoever saves the router must save it through
    /// [`ModelCallJournal::save`], which truncates the journal once the
    /// snapshot holds its observations, so a replay does not count them twice.
    #[must_use]
    pub fn with_cascade_journal(mut self, journal: Arc<ModelCallJournal>) -> Self {
        self.cascade_journal = Some(journal);
        self
    }

    /// Record each model call that reached a provider as one cost record in
    /// `costs.jsonl` under the data directory, where `roko status` and the
    /// daily budget read spend (bug-c1f6b8).
    ///
    /// Only for callers whose model calls nothing else costs, so no call is
    /// recorded twice. A response served from a cache cost nothing and gets
    /// no record, nor does a call that reported no usage: its cost is
    /// unknown, not zero.
    #[must_use]
    pub fn with_cost_records(mut self) -> Self {
        self.costs_path = Some(self.data_dir.join("costs.jsonl"));
        self
    }

    /// Create a service from the `.roko` directory with episode recording enabled.
    #[must_use]
    pub fn from_roko_dir_with_episodes(roko_dir: &Path) -> Self {
        let episodes_path = roko_dir.join("episodes.jsonl");
        let logger = EpisodeLogger::new(episodes_path);
        Self::from_roko_dir(roko_dir).with_episode_logger(logger)
    }

    /// Flush buffered events to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer lock is poisoned, the data directory
    /// cannot be created, or the JSONL file cannot be written.
    pub fn flush(&self) -> Result<()> {
        let events = {
            let mut buf = self
                .buffer
                .lock()
                .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
            std::mem::take(&mut *buf)
        };

        if events.is_empty() {
            self.persist_score_snapshots()?;
            return Ok(());
        }

        std::fs::create_dir_all(&self.data_dir)?;
        // This file is shared with the AgentEfficiencyEvent writers
        // (Graph dispatch). Feedback rows carry an explicit `schema`
        // discriminator so readers can classify them instead of silently
        // skipping foreign rows (audit #23).
        let efficiency_path = self.data_dir.join("efficiency.jsonl");

        let mut records = String::new();

        for event in &events {
            let ts = Utc::now().to_rfc3339();
            let json = match event {
                FeedbackEvent::ModelCall {
                    run_id,
                    request_id,
                    prompt_section_ids,
                    knowledge_ids,
                    model,
                    provider,
                    token_usage,
                    cost,
                    role,
                    input_tokens,
                    output_tokens,
                    cost_usd,
                    latency_ms,
                    success,
                    error_class,
                    model_reported,
                    attempt_key,
                    cache_hit,
                } => serde_json::json!({
                    "kind": "model_call",
                    "schema": FEEDBACK_EVENT_SCHEMA,
                    "run_id": run_id,
                    "request_id": request_id,
                    "prompt_section_ids": prompt_section_ids,
                    "knowledge_ids": knowledge_ids,
                    "model": model,
                    "provider": provider,
                    "token_usage": token_usage,
                    "cost": cost,
                    "role": role,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "cost_usd": cost_usd,
                    "latency_ms": latency_ms,
                    "success": success,
                    "error_class": error_class,
                    // `null` when the provider named no model, or the
                    // call belongs to no attempt.
                    "model_reported": model_reported,
                    "attempt_key": attempt_key,
                    // A cached response, which cost nothing.
                    "cache_hit": cache_hit,
                    "ts": ts,
                }),
                FeedbackEvent::GateResult {
                    run_id,
                    gate_name,
                    passed,
                    duration_ms,
                } => serde_json::json!({
                    "kind": "gate_result",
                    "schema": FEEDBACK_EVENT_SCHEMA,
                    "run_id": run_id,
                    "gate_name": gate_name,
                    "passed": passed,
                    "duration_ms": duration_ms,
                    "ts": ts,
                }),
                FeedbackEvent::WorkflowComplete {
                    event_type,
                    run_id,
                    model,
                    success,
                    outcome,
                    total_cost_usd,
                    total_tokens,
                    duration_ms,
                } => serde_json::json!({
                    "kind": event_type,
                    "schema": FEEDBACK_EVENT_SCHEMA,
                    "event_type": event_type,
                    "run_id": run_id,
                    "model": model,
                    "success": success,
                    "outcome": outcome,
                    "total_cost_usd": total_cost_usd,
                    "total_tokens": total_tokens,
                    "duration_ms": duration_ms,
                    "ts": ts,
                }),
            };
            records.push_str(&json.to_string());
            records.push('\n');
        }

        roko_fs::log_rotation::append_jsonl_line_sync(
            &efficiency_path,
            records.as_bytes(),
            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
        )?;

        self.persist_score_snapshots()?;
        Ok(())
    }

    /// Record evaluated prompt-section and knowledge provenance for a model request.
    ///
    /// # Errors
    ///
    /// Returns an error if `request_id` is empty, the data directory cannot be
    /// created, the JSONL event cannot be written, or scores cannot be updated.
    pub fn record_prompt_knowledge_outcome(
        &self,
        request_id: &str,
        prompt_section_ids: &[String],
        knowledge_ids: &[String],
        outcome: KnowledgeOutcome,
    ) -> Result<()> {
        self.record_prompt_knowledge_outcome_for(
            None,
            Some("model_call"),
            request_id,
            prompt_section_ids,
            knowledge_ids,
            outcome,
        )
    }

    /// Return learned prompt-section weights for prompt assembly.
    #[must_use]
    pub fn section_effectiveness(&self) -> HashMap<String, f64> {
        self.section_effectiveness
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .lift_weights()
    }

    /// Return signed knowledge scores keyed by knowledge entry id.
    #[must_use]
    pub fn knowledge_scores(&self) -> HashMap<String, i64> {
        self.knowledge_scores
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Record which knowledge entries influenced a routing/prompt decision
    /// and whether the subsequent gate check passed.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory cannot be created, the feedback
    /// file cannot be opened, or the JSONL record cannot be written.
    pub fn record_knowledge_usage(
        &self,
        run_id: &str,
        knowledge_ids: Vec<String>,
        gate_passed: bool,
        model_slug: &str,
    ) -> Result<()> {
        if knowledge_ids.is_empty() {
            return Ok(());
        }

        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.data_dir.join(KNOWLEDGE_FEEDBACK_FILE);
        let timestamp = Utc::now().to_rfc3339();
        let json = serde_json::json!({
            "type": "knowledge_usage",
            "run_id": run_id,
            "knowledge_ids": knowledge_ids,
            "gate_passed": gate_passed,
            "model_slug": model_slug,
            "timestamp": timestamp,
        });
        self.apply_knowledge_outcome(
            &knowledge_ids,
            if gate_passed {
                KnowledgeOutcome::Success
            } else {
                KnowledgeOutcome::Failure
            },
        )?;

        roko_fs::log_rotation::append_jsonl_line_sync(
            &path,
            json.to_string().as_bytes(),
            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
        )?;

        tracing::debug!(
            run_id,
            model_slug,
            gate_passed,
            knowledge_count = json
                .get("knowledge_ids")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len),
            "recorded knowledge usage feedback"
        );

        Ok(())
    }

    fn record_prompt_knowledge_outcome_for(
        &self,
        run_id: Option<&str>,
        role: Option<&str>,
        request_id: &str,
        prompt_section_ids: &[String],
        knowledge_ids: &[String],
        outcome: KnowledgeOutcome,
    ) -> Result<()> {
        if request_id.trim().is_empty() {
            return Err(RokoError::Invalid(
                "request_id is required for knowledge provenance feedback".to_string(),
            ));
        }
        if prompt_section_ids.is_empty() && knowledge_ids.is_empty() {
            return Ok(());
        }

        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.data_dir.join(KNOWLEDGE_FEEDBACK_FILE);
        let timestamp = Utc::now().to_rfc3339();
        let score_delta = outcome.score_delta();
        let json = serde_json::json!({
            "type": "knowledge_outcome",
            "run_id": run_id,
            "request_id": request_id,
            "prompt_section_ids": prompt_section_ids,
            "knowledge_ids": knowledge_ids,
            "outcome": outcome,
            "score_delta": score_delta,
            "timestamp": timestamp,
        });

        roko_fs::log_rotation::append_jsonl_line_sync(
            &path,
            json.to_string().as_bytes(),
            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
        )?;

        self.apply_knowledge_outcome(knowledge_ids, outcome)?;
        if let Some(passed) = outcome.passed() {
            self.apply_section_outcome(prompt_section_ids, role.unwrap_or("model_call"), passed)?;
        }

        Ok(())
    }

    fn apply_knowledge_outcome(
        &self,
        knowledge_ids: &[String],
        outcome: KnowledgeOutcome,
    ) -> Result<()> {
        let delta = outcome.score_delta();
        if delta == 0 || knowledge_ids.is_empty() {
            return Ok(());
        }

        let mut scores = self
            .knowledge_scores
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
        for knowledge_id in knowledge_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
        {
            let score = scores.entry(knowledge_id.to_string()).or_insert(0);
            *score = score.saturating_add(delta);
            self.scores_changed.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    fn apply_section_outcome(
        &self,
        prompt_section_ids: &[String],
        role: &str,
        passed: bool,
    ) -> Result<()> {
        if prompt_section_ids.is_empty() {
            return Ok(());
        }

        let mut registry = self
            .section_effectiveness
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
        for section_id in prompt_section_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
        {
            registry.record_outcome(section_id, role.trim(), true, passed);
            self.scores_changed.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    fn remember_model_call_provenance(
        &self,
        run_id: Option<&String>,
        request_id: Option<&String>,
        prompt_section_ids: &[String],
        knowledge_ids: &[String],
        role: &str,
    ) -> Result<Option<ProvenanceRecord>> {
        if prompt_section_ids.is_empty() && knowledge_ids.is_empty() {
            return Ok(None);
        }
        let Some(request_id) = request_id.filter(|id| !id.trim().is_empty()) else {
            return Ok(None);
        };

        let record = ProvenanceRecord {
            run_id: run_id.cloned(),
            request_id: request_id.clone(),
            prompt_section_ids: prompt_section_ids.to_vec(),
            knowledge_ids: knowledge_ids.to_vec(),
            role: role.to_string(),
        };

        let mut provenance = self
            .provenance
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
        if provenance.contains_key(request_id) {
            return Ok(None);
        }
        provenance.insert(request_id.clone(), record.clone());
        Ok(Some(record))
    }

    fn take_provenance_for_run(&self, run_id: &str) -> Result<Vec<ProvenanceRecord>> {
        let mut provenance = self
            .provenance
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
        let request_ids: Vec<_> = provenance
            .iter()
            .filter(|(_, record)| record.run_id.as_deref() == Some(run_id))
            .map(|(request_id, _)| request_id.clone())
            .collect();

        Ok(request_ids
            .into_iter()
            .filter_map(|request_id| provenance.remove(&request_id))
            .collect())
    }

    fn record_outcome_for_run(&self, run_id: &str, outcome: KnowledgeOutcome) -> Result<()> {
        for record in self.take_provenance_for_run(run_id)? {
            self.record_prompt_knowledge_outcome_for(
                record.run_id.as_deref(),
                Some(&record.role),
                &record.request_id,
                &record.prompt_section_ids,
                &record.knowledge_ids,
                outcome,
            )?;
        }
        Ok(())
    }

    /// Save the knowledge scores and section effects when an outcome changed
    /// them. A service that recorded none, such as one that only logs a
    /// provider call, rewrites neither file, so it cannot overwrite what
    /// another writer saved since this one loaded them, and a run that holds
    /// learned state fixed (decision 2218) leaves both as it found them.
    fn persist_score_snapshots(&self) -> Result<()> {
        if !self.scores_changed.swap(false, Ordering::Relaxed) {
            return Ok(());
        }
        let saved = self.save_score_snapshots();
        if saved.is_err() {
            // The next flush tries again.
            self.scores_changed.store(true, Ordering::Relaxed);
        }
        saved
    }

    fn save_score_snapshots(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;

        let knowledge_scores = self
            .knowledge_scores
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?
            .clone();
        let snapshot = KnowledgeScoreSnapshot {
            scores: knowledge_scores,
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(self.data_dir.join(KNOWLEDGE_SCORES_FILE), json)?;

        let registry = self
            .section_effectiveness
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
        registry.save(&self.data_dir.join(SECTION_EFFECTS_FILE))?;
        Ok(())
    }

    /// Read the knowledge feedback log and compute per-entry success rates.
    ///
    /// Returns a map of knowledge_id -> (successes, total_uses).
    ///
    /// # Errors
    ///
    /// Returns an error if the feedback file cannot be read or a non-empty
    /// JSONL line cannot be parsed as JSON.
    pub fn compute_knowledge_scores(&self) -> Result<HashMap<String, (u32, u32)>> {
        let path = self.data_dir.join(KNOWLEDGE_FEEDBACK_FILE);
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let mut scores: HashMap<String, (u32, u32)> = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(trimmed)?;
            let gate_passed =
                if let Some(outcome) = value.get("outcome").and_then(serde_json::Value::as_str) {
                    match outcome {
                        "success" => Some(true),
                        "failure" => Some(false),
                        _ => None,
                    }
                } else {
                    value
                        .get("gate_passed")
                        .and_then(serde_json::Value::as_bool)
                };
            let Some(gate_passed) = gate_passed else {
                continue;
            };
            let Some(knowledge_ids) = value
                .get("knowledge_ids")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };

            for knowledge_id in knowledge_ids
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|id| !id.is_empty())
            {
                let (successes, total_uses) =
                    scores.entry(knowledge_id.to_string()).or_insert((0, 0));
                *total_uses += 1;
                if gate_passed {
                    *successes += 1;
                }
            }
        }

        Ok(scores)
    }

    /// Flush buffered events to disk and append workflow-complete episodes.
    ///
    /// The synchronous [`Self::flush`] path remains suitable for `Drop` and
    /// callers without an async runtime; this async path adds best-effort
    /// episode recording after the efficiency JSONL write succeeds.
    ///
    /// # Errors
    ///
    /// Returns any error from the synchronous efficiency JSONL flush. Episode
    /// append errors are logged and do not fail the flush.
    pub async fn flush_async(&self) -> Result<()> {
        let episodes = if self.episode_logger.is_some() {
            self.pending_workflow_episodes()?
        } else {
            Vec::new()
        };

        self.flush()?;

        if let Some(ref logger) = self.episode_logger {
            for episode in episodes {
                if let Err(err) = logger.append(&episode).await {
                    tracing::warn!("failed to append episode: {err}");
                }
            }
        }

        Ok(())
    }

    fn pending_workflow_episodes(&self) -> Result<Vec<Episode>> {
        let buf = self
            .buffer
            .lock()
            .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;

        Ok(buf
            .iter()
            .filter_map(|event| {
                if let FeedbackEvent::WorkflowComplete {
                    run_id,
                    model,
                    success,
                    outcome,
                    total_cost_usd,
                    total_tokens,
                    duration_ms,
                    ..
                } = event
                {
                    let gate_verdicts = buf
                        .iter()
                        .filter_map(|event| match event {
                            FeedbackEvent::GateResult {
                                run_id: gate_run_id,
                                gate_name,
                                passed,
                                ..
                            } if gate_run_id == run_id => {
                                Some(EpisodeGateVerdict::new(gate_name.clone(), *passed))
                            }
                            _ => None,
                        })
                        .collect();
                    Some(build_episode_from_workflow(
                        run_id,
                        model.as_deref(),
                        *success,
                        outcome,
                        *total_cost_usd,
                        *total_tokens,
                        *duration_ms,
                        gate_verdicts,
                    ))
                } else {
                    None
                }
            })
            .collect())
    }

    fn observe_model_call(&self, model: &str, success: bool, role: &str, latency_ms: u64) {
        let Some(ref router) = self.cascade_router else {
            return;
        };

        match &self.cascade_journal {
            Some(journal) => journal.observe_model_call(router, model, role, success, latency_ms),
            None => observe_model_call_on_router(router, model, role, success, latency_ms),
        }
    }
}

#[async_trait]
impl FeedbackSink for FeedbackService {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        if let Some(costs_path) = &self.costs_path
            && let Some(cost_record) = model_call_cost_record(&event)
        {
            append_cost_record(costs_path, &cost_record).await;
        }
        match &event {
            FeedbackEvent::ModelCall {
                run_id,
                request_id,
                prompt_section_ids,
                knowledge_ids,
                model: Some(model),
                role,
                latency_ms,
                success,
                cache_hit,
                ..
            } => {
                // A cache hit called no model, so it is no router trial
                // (bug-982600).
                if !*cache_hit {
                    self.observe_model_call(model, *success, role, *latency_ms);
                }
                if *success {
                    if let Some(record) = self.remember_model_call_provenance(
                        run_id.as_ref(),
                        request_id.as_ref(),
                        prompt_section_ids,
                        knowledge_ids,
                        role,
                    )? {
                        self.record_prompt_knowledge_outcome_for(
                            record.run_id.as_deref(),
                            Some(&record.role),
                            &record.request_id,
                            &record.prompt_section_ids,
                            &record.knowledge_ids,
                            KnowledgeOutcome::Partial,
                        )?;
                    }
                }
            }
            FeedbackEvent::ModelCall { .. } => {}
            FeedbackEvent::GateResult { run_id, passed, .. } => {
                self.record_outcome_for_run(
                    run_id,
                    if *passed {
                        KnowledgeOutcome::Success
                    } else {
                        KnowledgeOutcome::Failure
                    },
                )?;
            }
            FeedbackEvent::WorkflowComplete {
                run_id, success, ..
            } => {
                self.record_outcome_for_run(
                    run_id,
                    if *success {
                        KnowledgeOutcome::Success
                    } else {
                        KnowledgeOutcome::Failure
                    },
                )?;
            }
        }

        let should_flush = {
            let mut buf = self
                .buffer
                .lock()
                .map_err(|error| RokoError::Invalid(format!("lock poisoned: {error}")))?;
            buf.push(event);
            buf.len() >= self.buffer_capacity
        };

        if should_flush {
            self.flush_async().await?;
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.flush_async().await
    }
}

fn load_knowledge_scores(path: &Path) -> HashMap<String, i64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<KnowledgeScoreSnapshot>(&contents).ok())
        .map(|snapshot| snapshot.scores)
        .unwrap_or_default()
}

/// The cost record of a model call that reached a provider: `None` for any
/// other event, and for a response served from a cache, which cost nothing.
fn model_call_cost_record(event: &FeedbackEvent) -> Option<CostRecord> {
    let FeedbackEvent::ModelCall {
        run_id,
        request_id,
        model,
        provider,
        role,
        input_tokens,
        output_tokens,
        cost_usd,
        latency_ms,
        success,
        cache_hit: false,
        ..
    } = event
    else {
        return None;
    };
    // A call that reported no usage gets no record: a $0 row would read as
    // a free call, when its cost is unknown.
    if *input_tokens == 0 && *output_tokens == 0 && *cost_usd <= 0.0 {
        return None;
    }
    Some(CostRecord {
        timestamp: Utc::now().to_rfc3339(),
        model: model.clone().unwrap_or_default(),
        provider: provider
            .clone()
            .unwrap_or_else(|| "unknown-provider".to_string()),
        role: role.clone(),
        plan_id: String::new(),
        // The request id is the one the call's gateway event carries.
        task_id: request_id.clone().unwrap_or_default(),
        complexity_band: "standard".to_string(),
        input_tokens: *input_tokens,
        output_tokens: *output_tokens,
        cached_tokens: 0,
        cost_usd: *cost_usd,
        duration_ms: *latency_ms,
        success: *success,
        session_id: run_id.clone().unwrap_or_default(),
        // A model-call event does not say where its usage came from.
        cost_source: CostSource::Unknown,
        priced: None,
        api_equiv_usd: None,
        price_snapshot_id: None,
    })
}

/// Append `record` to the cost log at `path`. Best-effort: a failed write is
/// logged and never fails the feedback.
async fn append_cost_record(path: &Path, record: &CostRecord) {
    let line = match serde_json::to_string(record) {
        Ok(line) => line,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "cost record serialization failed");
            return;
        }
    };
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    let written = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || roko_fs::log_rotation::append_jsonl_line_sync(&path, line.as_bytes(), max_mb)
    })
    .await;
    if let Err(error) = written
        .map_err(std::io::Error::other)
        .and_then(|result| result.map(|_| ()))
    {
        tracing::warn!(path = %path.display(), %error, "cost record write failed (best-effort)");
    }
}

impl Drop for FeedbackService {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn build_episode_from_workflow(
    run_id: &str,
    model: Option<&str>,
    success: bool,
    outcome: &str,
    total_cost_usd: f64,
    total_tokens: u64,
    duration_ms: u64,
    gate_verdicts: Vec<EpisodeGateVerdict>,
) -> Episode {
    let mut episode = Episode::new("workflow", run_id);
    episode.kind = "workflow_complete".to_string();
    episode.id = format!("ep-{run_id}");
    episode.episode_id = episode.id.clone();
    episode.model = model.unwrap_or_default().to_string();
    episode.trigger_kind = "workflow_complete".to_string();
    episode.success = success;
    episode.duration_secs = duration_ms as f64 / 1000.0;
    episode.tokens_used = total_tokens;
    episode.gate_verdicts = gate_verdicts;
    episode.usage = Usage {
        cost_usd: total_cost_usd,
        cost_usd_without_cache: total_cost_usd,
        wall_ms: duration_ms,
        ..Usage::default()
    };
    episode
        .extra
        .insert("run_id".to_string(), serde_json::json!(run_id));
    if !episode.success {
        episode.failure_reason = Some(outcome.to_string());
    }
    episode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_model_call() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        svc.flush().unwrap();

        let content = std::fs::read_to_string(dir.path().join("efficiency.jsonl")).unwrap();
        assert!(content.contains("model_call"));
        assert!(content.contains("sonnet"));
    }

    /// bug-c1f6b8: with cost records on, a model call that reached a
    /// provider gets one cost record. A cached answer cost nothing, and a
    /// call that reported no usage has an unknown cost: neither gets one.
    #[tokio::test]
    async fn cost_records_skip_cached_answers_and_calls_without_usage() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf()).with_cost_records();
        let call = |input_tokens: u64, cost_usd: f64, cache_hit: bool| FeedbackEvent::ModelCall {
            run_id: Some("serve-run".into()),
            request_id: Some(format!("serve-run:{input_tokens}:{cache_hit}")),
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: Some("anthropic".into()),
            token_usage: None,
            cost: None,
            role: "model_call".into(),
            input_tokens,
            output_tokens: input_tokens / 10,
            cost_usd,
            latency_ms: 900,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit,
        };

        svc.record(call(1_000, 0.02, false)).await.unwrap();
        svc.record(call(1_000, 0.02, true)).await.unwrap();
        svc.record(call(0, 0.0, false)).await.unwrap();

        let costs = std::fs::read_to_string(dir.path().join("costs.jsonl")).unwrap();
        let rows: Vec<CostRecord> = costs
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 1, "{costs}");
        let row = &rows[0];
        assert_eq!(
            (row.model.as_str(), row.provider.as_str()),
            ("sonnet", "anthropic")
        );
        assert_eq!((row.input_tokens, row.output_tokens), (1_000, 100));
        assert!((row.cost_usd - 0.02).abs() < 1e-12);
        assert_eq!(row.task_id, "serve-run:1000:false");
        assert_eq!(row.session_id, "serve-run");
    }

    #[tokio::test]
    async fn records_gate_result() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record(FeedbackEvent::GateResult {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            passed: true,
            duration_ms: 3000,
        })
        .await
        .unwrap();

        svc.flush().unwrap();

        let content = std::fs::read_to_string(dir.path().join("efficiency.jsonl")).unwrap();
        assert!(content.contains("gate_result"));
    }

    #[tokio::test]
    async fn flush_writes_schema_discriminator() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record(FeedbackEvent::GateResult {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            passed: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.01,
            latency_ms: 200,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        svc.flush().unwrap();

        let content = std::fs::read_to_string(dir.path().join("efficiency.jsonl")).unwrap();
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(
                value["schema"], FEEDBACK_EVENT_SCHEMA,
                "every feedback row carries the schema discriminator: {line}"
            );
        }
        assert!(content.contains("gate_result"));
        assert!(content.contains("model_call"));
    }

    #[tokio::test]
    async fn records_episode_on_workflow_complete() {
        let dir = tempfile::tempdir().unwrap();
        let episodes_path = dir.path().join("episodes.jsonl");
        let logger = EpisodeLogger::new(&episodes_path);
        let svc = FeedbackService::new(dir.path().join("learn")).with_episode_logger(logger);

        svc.record(FeedbackEvent::GateResult {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            passed: true,
            duration_ms: 3000,
        })
        .await
        .unwrap();
        svc.record(FeedbackEvent::WorkflowComplete {
            event_type: "workflow_completed".into(),
            run_id: "r1".into(),
            model: Some("sonnet".into()),
            success: true,
            outcome: "success".into(),
            total_cost_usd: 0.02,
            total_tokens: 1200,
            duration_ms: 2500,
        })
        .await
        .unwrap();

        svc.flush_async().await.unwrap();

        let episodes = EpisodeLogger::read_all(&episodes_path).await.unwrap();
        assert!(
            episodes
                .iter()
                .any(|episode| episode.kind == "workflow_complete")
        );
        let episode = episodes
            .iter()
            .find(|episode| episode.kind == "workflow_complete")
            .unwrap();
        assert_eq!(episode.model, "sonnet");
        assert_eq!(episode.extra.get("run_id"), Some(&serde_json::json!("r1")));
        assert_eq!(
            episode.gate_verdicts,
            vec![EpisodeGateVerdict::new("compile", true)]
        );
    }

    #[test]
    fn records_knowledge_usage() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record_knowledge_usage(
            "r1",
            vec!["knowledge-a".into(), "knowledge-b".into()],
            true,
            "sonnet",
        )
        .unwrap();

        let content = std::fs::read_to_string(dir.path().join(KNOWLEDGE_FEEDBACK_FILE)).unwrap();
        let value: serde_json::Value = serde_json::from_str(content.trim()).unwrap();

        assert_eq!(value["type"], "knowledge_usage");
        assert_eq!(value["run_id"], "r1");
        assert_eq!(value["gate_passed"], true);
        assert_eq!(value["model_slug"], "sonnet");
        assert_eq!(
            value["knowledge_ids"],
            serde_json::json!(["knowledge-a", "knowledge-b"])
        );
        assert!(value["timestamp"].as_str().is_some());
    }

    #[test]
    fn skips_empty_knowledge_usage() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record_knowledge_usage("r1", Vec::new(), true, "sonnet")
            .unwrap();

        assert!(!dir.path().join(KNOWLEDGE_FEEDBACK_FILE).exists());
    }

    #[test]
    fn computes_knowledge_scores() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record_knowledge_usage(
            "r1",
            vec!["knowledge-a".into(), "knowledge-b".into()],
            true,
            "sonnet",
        )
        .unwrap();
        svc.record_knowledge_usage("r2", vec!["knowledge-a".into()], false, "haiku")
            .unwrap();

        let scores = svc.compute_knowledge_scores().unwrap();

        assert_eq!(scores.get("knowledge-a"), Some(&(1, 2)));
        assert_eq!(scores.get("knowledge-b"), Some(&(1, 1)));
    }

    #[tokio::test]
    async fn sync_flush_still_works_without_episodes() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());

        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        svc.flush().unwrap();

        let efficiency_path = dir.path().join("efficiency.jsonl");
        assert!(efficiency_path.exists());
        let content = std::fs::read_to_string(efficiency_path).unwrap();
        assert!(content.contains("model_call"));
    }

    /// A flush saves the knowledge scores and section effects only when an
    /// outcome changed them. A provider call logged without prompt sections
    /// or knowledge, as a provider bridge logs each call, writes neither
    /// file, flushed or dropped; a knowledge outcome saves the scores.
    #[tokio::test]
    async fn flush_saves_scores_only_when_an_outcome_changed_them() {
        let dir = tempfile::tempdir().unwrap();
        let svc = FeedbackService::new(dir.path().to_path_buf());
        svc.record(FeedbackEvent::ModelCall {
            run_id: None,
            request_id: Some("dispatch-v2-agent".into()),
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "dispatch_v2".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();
        svc.flush().unwrap();
        drop(svc);

        assert!(dir.path().join("efficiency.jsonl").exists());
        let scores = dir.path().join(KNOWLEDGE_SCORES_FILE);
        assert!(!scores.exists(), "no outcome changed a score");
        let effects = dir.path().join(SECTION_EFFECTS_FILE);
        assert!(!effects.exists(), "no outcome changed a section effect");

        let svc = FeedbackService::new(dir.path().to_path_buf());
        svc.record_knowledge_usage("r1", vec!["knowledge-a".into()], true, "sonnet")
            .unwrap();
        svc.flush().unwrap();
        let reloaded = FeedbackService::new(dir.path().to_path_buf());
        assert_eq!(reloaded.knowledge_scores().get("knowledge-a"), Some(&1));
    }

    #[tokio::test]
    async fn observes_model_call_to_cascade_router() {
        let dir = tempfile::tempdir().unwrap();
        let router = Arc::new(CascadeRouter::new(vec!["sonnet".into(), "opus".into()]));
        let svc =
            FeedbackService::new(dir.path().to_path_buf()).with_cascade_router(Arc::clone(&router));

        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        assert_eq!(router.total_observations(), 1);
    }

    #[tokio::test]
    async fn observes_failed_model_call_as_failed_trial() {
        // bug-8da8ba: a failed call reaches LinUCB like a success does, and
        // counts as a trial without a success.
        let dir = tempfile::tempdir().unwrap();
        let router = Arc::new(CascadeRouter::new(vec!["sonnet".into(), "opus".into()]));
        let svc =
            FeedbackService::new(dir.path().to_path_buf()).with_cascade_router(Arc::clone(&router));

        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: false,
            error_class: Some("timeout".into()),
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        assert_eq!(router.total_observations(), 1);
        assert_eq!(router.confidence_snapshot()["sonnet"], (1, 0));
    }

    /// bug-982600: a cache hit answers from the cache without calling a
    /// model, so the router counts only the call that reached one.
    #[tokio::test]
    async fn cache_hits_are_not_router_trials() {
        let dir = tempfile::tempdir().unwrap();
        let router = Arc::new(CascadeRouter::new(vec!["sonnet".into(), "opus".into()]));
        let svc =
            FeedbackService::new(dir.path().to_path_buf()).with_cascade_router(Arc::clone(&router));
        let call = |cache_hit: bool| FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("sonnet".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit,
        };

        svc.record(call(false)).await.unwrap();
        svc.record(call(true)).await.unwrap();
        svc.record(call(true)).await.unwrap();

        assert_eq!(router.total_observations(), 1);
        assert_eq!(router.confidence_snapshot()["sonnet"], (1, 1));
    }

    #[tokio::test]
    async fn skips_observation_for_unknown_model() {
        let dir = tempfile::tempdir().unwrap();
        let router = Arc::new(CascadeRouter::new(vec!["sonnet".into()]));
        let svc =
            FeedbackService::new(dir.path().to_path_buf()).with_cascade_router(Arc::clone(&router));

        svc.record(FeedbackEvent::ModelCall {
            run_id: Some("r1".into()),
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some("mystery-model".into()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 2000,
            success: true,
            error_class: None,
            model_reported: None,
            attempt_key: None,
            cache_hit: false,
        })
        .await
        .unwrap();

        assert_eq!(router.total_observations(), 0);
    }
}
