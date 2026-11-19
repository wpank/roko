//! Background distillation of completed episodes into durable knowledge.
//!
//! Only an episode that passed a gate is distilled, unless `[learning]
//! distill_ungated` says every episode, and a frozen workspace distils none
//! (bug-f8655e). Under `[learning] guarded_commit` = `observe` or `enforce`,
//! a distillation's entries go through the knowledge store's guard as a batch
//! of their own, `distill:<episode>`, proposed as soon as they are written:
//! the batch commits only when the episode succeeded and failed no gate, and
//! `roko learn commits` lists it. `off` writes them straight to the store.
//!
//! A plan run's start rolls back every batch but its own as an orphan, so a
//! run that starts in the moment between a distillation's write and its
//! proposal rolls that batch back. That loses one distillation, and the row
//! says why. No distillation joins a run's batch: no plan run distils.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use roko_core::config::learning::{GuardedCommitMode, LearningConfig};
use roko_core::foundation::{
    CachePolicy, ChatMessage, MessageRole, ModelCallRequest, ModelCaller, caller,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::Episode;
use roko_learn::guarded_commit::{CommitDecision, GuardMode, Proposer};
use tokio::task;

use crate::distiller::episode_source_id;
use crate::knowledge_store::commit::{
    Delivery, KNOWLEDGE_STORE, KnowledgeChecks, load_knowledge_anchors, propose_batch,
};
use crate::{DistillationBackend, Distiller, KnowledgeEntry, KnowledgeStore};

/// Spawn background distillation for one completed episode.
///
/// The work is intentionally detached from the caller so episode
/// persistence can finish without waiting on model inference or store
/// writes.
pub fn spawn_episode_distillation(
    workdir: PathBuf,
    episode: Episode,
    model_caller: Option<Arc<dyn ModelCaller>>,
) {
    tokio::spawn(async move {
        if let Err(error) = distill_episode(workdir, episode, model_caller).await {
            tracing::warn!(error = %error, "episode distillation failed");
        }
    });
}

/// Role under which the spend of an episode-distillation call is recorded.
pub const DISTILLATION_ROLE: &str = "episode-distiller";

/// [`spawn_episode_distillation`] that records the call's spend.
///
/// For a `model_caller` that records nothing itself, such as a bare
/// `ModelCallService`, the spend goes through [`DistillationSpend`]. The CLI's
/// capture paths and ACP distil this way.
pub fn spawn_recorded_episode_distillation(
    workdir: PathBuf,
    episode: Episode,
    model_caller: Arc<dyn ModelCaller>,
) {
    let recorded: Arc<dyn ModelCaller> =
        Arc::new(DistillationSpend::new(&workdir, &episode, model_caller));
    spawn_episode_distillation(workdir, episode, Some(recorded));
}

/// A [`ModelCaller`] that records the spend of each call it makes to distil
/// one episode.
///
/// [`spawn_episode_distillation`] calls the model detached from whatever
/// logged the episode, so nothing else sees that call's usage. Each call that
/// returns appends a cost record to `.roko/learn/costs.jsonl` and an
/// efficiency row to `.roko/learn/efficiency.jsonl`, under role
/// [`DISTILLATION_ROLE`] and the episode's plan and task ids.
pub struct DistillationSpend {
    inner: Arc<dyn ModelCaller>,
    workdir: PathBuf,
    episode_id: String,
    plan_id: String,
    task_id: String,
}

impl DistillationSpend {
    /// Record, in `workdir`'s learning logs, the calls `inner` makes to distil
    /// `episode`.
    #[must_use]
    pub fn new(workdir: &Path, episode: &Episode, inner: Arc<dyn ModelCaller>) -> Self {
        let plan_id = episode
            .extra
            .get("plan_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        Self {
            inner,
            workdir: workdir.to_path_buf(),
            episode_id: episode.id.clone(),
            plan_id,
            task_id: episode.task_id.clone(),
        }
    }

    /// Append the cost record and the efficiency row of one returned call.
    /// Best-effort: a failed write is logged and never fails the call.
    async fn record(&self, response: &roko_core::foundation::ModelCallResponse, duration_ms: u64) {
        let config =
            roko_core::config::loader::load_config_unified(&self.workdir).unwrap_or_default();
        let provider = roko_core::agent::resolve_model(&config, &response.model)
            .profile
            .map(|profile| profile.provider)
            .filter(|provider| !provider.trim().is_empty())
            .unwrap_or_else(|| "unknown-provider".to_string());
        let usage = &response.usage;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let cost_record = CostRecord {
            timestamp: timestamp.clone(),
            model: response.model.clone(),
            provider: provider.clone(),
            role: DISTILLATION_ROLE.to_string(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            complexity_band: "standard".to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cached_tokens: 0,
            cost_usd: usage.cost_usd,
            duration_ms,
            success: true,
            session_id: String::new(),
            // A `ModelCallResponse` does not say where its usage came from.
            cost_source: roko_learn::telemetry::CostSource::Unknown,
            priced: None,
            api_equiv_usd: None,
            price_snapshot_id: None,
        };
        let efficiency_event = AgentEfficiencyEvent {
            agent_id: format!("{}/{DISTILLATION_ROLE}", self.episode_id),
            role: DISTILLATION_ROLE.to_string(),
            backend: provider,
            model: response.model.clone(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            attempt_id: format!("{}/{DISTILLATION_ROLE}", self.episode_id),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_usd: usage.cost_usd,
            cost_usd_without_cache: usage.cost_usd,
            total_prompt_tokens: usage.input_tokens,
            wall_time_ms: duration_ms,
            duration_ms,
            iteration: 1,
            outcome: "success".to_string(),
            model_used: response.model.clone(),
            frequency: roko_core::OperatingFrequency::Theta,
            timestamp,
            ..AgentEfficiencyEvent::default()
        };

        let learn_dir = roko_fs::RokoLayout::for_project(&self.workdir).learn_dir();
        for (file_name, line) in [
            ("costs.jsonl", serde_json::to_string(&cost_record)),
            ("efficiency.jsonl", serde_json::to_string(&efficiency_event)),
        ] {
            let path = learn_dir.join(file_name);
            if let Err(error) = append_learn_line(&path, line).await {
                tracing::warn!(
                    path = %path.display(),
                    task_id = %self.task_id,
                    %error,
                    "distillation spend write failed (best-effort)"
                );
            }
        }
    }
}

#[async_trait]
impl ModelCaller for DistillationSpend {
    async fn call(
        &self,
        req: ModelCallRequest,
    ) -> roko_core::Result<roko_core::foundation::ModelCallResponse> {
        let started = std::time::Instant::now();
        let response = self.inner.call(req).await?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.record(&response, duration_ms).await;
        Ok(response)
    }
}

/// Append one serialized record to a `.roko/learn` JSONL log, rotating it at
/// the default size.
async fn append_learn_line(path: &Path, line: serde_json::Result<String>) -> std::io::Result<()> {
    let line = line.map_err(std::io::Error::other)?;
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    let path = path.to_path_buf();
    task::spawn_blocking(move || {
        roko_fs::log_rotation::append_jsonl_line_sync(&path, line.as_bytes(), max_mb)
    })
    .await
    .map_err(std::io::Error::other)?
    .map(|_| ())
}

/// The prefix of a distillation's knowledge batch: `distill:<episode>`.
pub const DISTILL_BATCH_PREFIX: &str = "distill:";

async fn distill_episode(
    workdir: PathBuf,
    episode: Episode,
    model_caller: Option<Arc<dyn ModelCaller>>,
) -> Result<()> {
    let Some(model_caller) = model_caller else {
        tracing::debug!("no ModelCaller provided; skipping episode distillation");
        return Ok(());
    };
    let config = roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();
    let Some(delivery) = distillation_delivery(&episode, &config.learning) else {
        return Ok(());
    };

    // Use an empty model string so the ModelCaller resolves via its own
    // default — this respects whatever model the workspace has configured
    // instead of hardcoding a specific provider.
    let distiller =
        Distiller::with_backend(Arc::new(GatewayDistillationBackend::new(model_caller, "")));

    let source = episode_source_id(&episode).to_string();
    let episodes = [episode];
    let entries = distiller
        .distill(&episodes)
        .await
        .context("distill completed episode")?;

    let mode = guard_mode(config.learning.guarded_commit);
    task::spawn_blocking(move || write_distilled(&workdir, &source, entries, mode, delivery))
        .await
        .context("join knowledge-store writer")??;

    Ok(())
}

/// Whether to distil `episode`, and if so what it delivered: an episode is
/// distilled when it succeeded and passed a gate, failing none, or, under
/// `distill_ungated`, whatever it ran (bug-f8655e). `None` in a frozen
/// workspace, and for an episode whose distillation the guard would roll back
/// whatever it taught: one that failed, while the guard is on.
fn distillation_delivery(episode: &Episode, learning: &LearningConfig) -> Option<Delivery> {
    let id = episode_source_id(episode);
    if learning.frozen {
        tracing::debug!(
            episode = id,
            "learning is frozen; the episode is not distilled"
        );
        return None;
    }
    let passed = episode
        .gate_verdicts
        .iter()
        .filter(|gate| gate.passed)
        .count();
    let failed = episode.gate_verdicts.len() - passed;
    let delivered = episode.success && failed == 0;
    let evidence = delivered && passed > 0;
    if !evidence && !learning.distill_ungated {
        tracing::debug!(
            episode = id,
            passed,
            failed,
            success = episode.success,
            "the episode passed no gate to learn from; it is not distilled"
        );
        return None;
    }
    if delivered {
        let what = match passed {
            0 => format!("episode {id} succeeded and ran no gate"),
            1 => format!("episode {id} passed its gate"),
            gates => format!("episode {id} passed its {gates} gates"),
        };
        return Some(Delivery::Delivered(what));
    }
    if learning.guarded_commit != GuardedCommitMode::Off {
        tracing::debug!(
            episode = id,
            "the knowledge guard keeps nothing a failed episode taught; it is not distilled"
        );
        return None;
    }
    let why = if failed > 0 {
        format!(
            "episode {id} failed {failed} of its {} gates",
            passed + failed
        )
    } else {
        format!("episode {id} failed")
    };
    Some(Delivery::Undelivered(why))
}

/// What a failed guard check does to a distillation's batch; `None` for
/// `off`, which writes the entries straight to the store.
const fn guard_mode(mode: GuardedCommitMode) -> Option<GuardMode> {
    match mode {
        GuardedCommitMode::Observe => Some(GuardMode::Observe),
        GuardedCommitMode::Enforce => Some(GuardMode::Enforce),
        GuardedCommitMode::Off => None,
    }
}

/// Write the entries distilled from `episode` into `workdir`'s knowledge
/// store: under a guard, as the batch `distill:<episode>`, proposed at once
/// and decided by the guard's checks and `delivery`; with none, straight to
/// the store. Returns the guard's decision; `None` for no guard or no entry.
fn write_distilled(
    workdir: &Path,
    episode: &str,
    entries: Vec<KnowledgeEntry>,
    mode: Option<GuardMode>,
    delivery: Delivery,
) -> Result<Option<CommitDecision>> {
    let store = KnowledgeStore::for_workdir(workdir);
    let Some(mode) = mode else {
        for entry in entries {
            store.add(entry)?;
        }
        return Ok(None);
    };
    if entries.is_empty() {
        return Ok(None);
    }
    let batch = format!("{DISTILL_BATCH_PREFIX}{episode}");
    let batched = store.clone().with_commit_batch(&batch);
    // Propose whatever was written, even when a write failed part-way.
    let written = entries.into_iter().try_for_each(|entry| batched.add(entry));
    let layout = roko_fs::RokoLayout::for_project(workdir);
    let anchors = load_knowledge_anchors(layout.root()).unwrap_or_else(|problem| {
        tracing::warn!(%problem, "knowledge anchors unread; the distillation checks none");
        Vec::new()
    });
    let checks = KnowledgeChecks::new(&store, &batch, anchors, delivery);
    let proposer = Proposer::evolved(
        DISTILLER_ACTOR,
        KNOWLEDGE_STORE,
        format!("knowledge:{batch}"),
    );
    let decision = propose_batch(
        &store,
        &batch,
        &layout.learn_dir(),
        mode,
        &checks,
        &proposer,
    )
    .with_context(|| format!("propose the knowledge batch {batch}"))?;
    written?;
    Ok(decision)
}

/// The actor of a distillation's guard rows.
const DISTILLER_ACTOR: &str = "distiller";

struct GatewayDistillationBackend {
    model_caller: Arc<dyn ModelCaller>,
    model: String,
}

impl std::fmt::Debug for GatewayDistillationBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayDistillationBackend")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl GatewayDistillationBackend {
    fn new(model_caller: Arc<dyn ModelCaller>, model: impl Into<String>) -> Self {
        Self {
            model_caller,
            model: model.into(),
        }
    }
}

#[async_trait]
impl DistillationBackend for GatewayDistillationBackend {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self
            .model_caller
            .call(ModelCallRequest {
                model: self.model.clone(),
                system: None,
                messages: vec![ChatMessage {
                    role: MessageRole::User,
                    content: prompt.to_string(),
                }],
                input_messages: Vec::new(),
                max_tokens: None,
                temperature: None,
                role: Some("episode-distiller".to_string()),
                caller: Some(caller::RESEARCH.to_string()),
                run_id: None,
                prompt_section_ids: Vec::new(),
                knowledge_ids: Vec::new(),
                budget: None,
                budget_remaining: None,
                routing_hints: Vec::new(),
                cache_policy: CachePolicy::Default,
                tools: Vec::new(),
                generation_settings: None,
                mcp_config: None,
                thinking: None,
            })
            .await
            .context("call gateway distillation model")?;
        Ok(response.content)
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use roko_core::foundation::{ModelCallResponse, TokenUsage};
    use roko_learn::episode_logger::EpisodeGateVerdict;
    use roko_learn::guarded_commit::{CommitDecision, GuardMode, GuardedStore, Proposer};

    use super::*;
    use crate::knowledge_store::commit::{KNOWLEDGE_STORE, LaterBatches};

    /// What the fake distiller learns: a fact about the repository.
    const LESSON: &str = "core/src/presets.cpp: a preset's master_gain trim keeps the loudness \
                          check near -18 dBFS RMS.";

    /// A distillation model that states [`LESSON`] and counts its calls.
    #[derive(Default)]
    struct Distil {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelCaller for Distil {
        async fn call(&self, _req: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let entry = serde_json::json!({
                "kind": "insight",
                "content": LESSON,
                "confidence": 0.8,
                "tags": ["core/src/presets.cpp"],
            });
            Ok(ModelCallResponse {
                content: serde_json::json!({ "entries": [entry] }).to_string(),
                model: "fake-distiller".to_string(),
                usage: TokenUsage::default(),
                stop_reason: Some("end_turn".to_string()),
                request_id: None,
            })
        }
    }

    /// A workspace whose `roko.toml` sets `[learning]` to `learning`.
    fn workspace(learning: &str) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("roko.toml"),
            format!("[learning]\n{learning}\n"),
        )
        .expect("write roko.toml");
        temp
    }

    /// An episode that succeeded or not, with one gate verdict per `gates`.
    fn episode(id: &str, success: bool, gates: &[bool]) -> Episode {
        let mut episode = Episode::new("implementer", "T1");
        episode.id = id.to_string();
        episode.episode_id = id.to_string();
        episode.success = success;
        episode.gate_verdicts = gates
            .iter()
            .enumerate()
            .map(|(step, passed)| EpisodeGateVerdict::new(format!("verify[{step}:test]"), *passed))
            .collect();
        episode
    }

    /// Distil `episode` in `workdir` through a fresh [`Distil`]; returns how
    /// many calls it made.
    async fn distil(workdir: &Path, episode: Episode) -> usize {
        let model = Arc::new(Distil::default());
        let caller: Arc<dyn ModelCaller> = model.clone();
        distill_episode(workdir.to_path_buf(), episode, Some(caller))
            .await
            .expect("distilled");
        model.calls.load(Ordering::SeqCst)
    }

    /// The knowledge store's guard in `workdir`.
    fn guard(workdir: &Path) -> GuardedStore {
        let learn_dir = roko_fs::RokoLayout::for_project(workdir).learn_dir();
        GuardedStore::open(&learn_dir, KNOWLEDGE_STORE, GuardMode::Observe).expect("the guard")
    }

    /// bug-f8655e: what an episode that passed its gates taught goes through
    /// the knowledge store's guard as a batch of its own, proposed at once:
    /// it commits with a row of its own, and a rollback removes it.
    #[tokio::test]
    async fn a_distilled_entry_goes_through_the_guard() {
        let temp = workspace("guarded_commit = \"observe\"");
        let workdir = temp.path();
        let calls = distil(workdir, episode("ep-dispatch-1", true, &[true, true])).await;
        assert_eq!(calls, 1);

        let store = KnowledgeStore::for_workdir(workdir);
        let entries = store.read_all().expect("read");
        let [entry] = entries.as_slice() else {
            panic!("one lesson, one entry: {entries:#?}");
        };
        assert_eq!(entry.content, LESSON);
        assert_eq!(entry.source_episodes, ["ep-dispatch-1"]);
        assert_eq!(entry.commit_batch, None, "the guard committed it");

        let mut guarded = guard(workdir);
        let rows = guarded.rows().expect("the guard's rows");
        assert_eq!(rows.len(), 2, "the baseline, then the batch: {rows:#?}");
        let row = &rows[1];
        assert_eq!(row.decision, CommitDecision::Committed);
        assert_eq!(row.actor, "distiller");
        let reason = row.reason.as_deref().unwrap_or_default();
        assert_eq!(reason, "knowledge:distill:ep-dispatch-1");
        let delivered = &row.checks[0];
        assert_eq!(delivered.check, "delivered");
        assert_eq!(delivered.reason, "episode ep-dispatch-1 passed its 2 gates");

        // `roko learn rollback knowledge --to 1` takes it out again.
        let mut later = LaterBatches::after(&guarded, 1, store.clone()).expect("the later batches");
        let person = Proposer::human(KNOWLEDGE_STORE, "roko learn rollback knowledge --to 1");
        guarded
            .rollback(&mut later, 1, &person)
            .expect("rolled back");
        assert!(store.read_all().expect("read").is_empty());
    }

    /// bug-f8655e: a plan write, like every capture episode, runs no gate:
    /// it verified nothing, so it is not distilled. No model call, no entry,
    /// no guard row.
    #[tokio::test]
    async fn a_plan_write_distills_no_entry_without_evidence() {
        let temp = workspace("guarded_commit = \"observe\"");
        let workdir = temp.path();
        let mut write = episode("ep_9f2f838fc5c90dfe", true, &[]);
        write.task_id = "plan:generate:p5-1-the-renders-as-they-play".to_string();

        assert_eq!(distil(workdir, write).await, 0);
        assert!(
            KnowledgeStore::for_workdir(workdir)
                .read_all()
                .expect("read")
                .is_empty()
        );
        assert!(guard(workdir).rows().expect("the guard's rows").is_empty());
    }

    /// bug-f8655e: `distill_ungated = true` distils a gate-less episode, as
    /// roko did before, still through the guard. A failed episode is not
    /// distilled under the guard at all: its batch could never commit.
    #[tokio::test]
    async fn distill_ungated_distils_a_gate_less_episode_through_the_guard() {
        let temp = workspace("guarded_commit = \"observe\"\ndistill_ungated = true");
        let workdir = temp.path();
        assert_eq!(distil(workdir, episode("ep-write-1", true, &[])).await, 1);
        assert_eq!(distil(workdir, episode("ep-write-2", false, &[])).await, 0);
        assert_eq!(
            distil(workdir, episode("ep-write-3", true, &[true, false])).await,
            0
        );

        let entries = KnowledgeStore::for_workdir(workdir)
            .read_all()
            .expect("read");
        assert_eq!(entries.len(), 1, "{entries:#?}");
        assert_eq!(entries[0].commit_batch, None);
        let rows = guard(workdir).rows().expect("the guard's rows");
        assert_eq!(rows.len(), 2, "{rows:#?}");
        let delivered = &rows[1].checks[0];
        assert_eq!(
            delivered.reason,
            "episode ep-write-1 succeeded and ran no gate"
        );
    }

    /// bug-f8655e: with `guarded_commit = "off"`, an episode that passed its
    /// gate is distilled straight into the store, as before, and a frozen
    /// workspace distils nothing.
    #[tokio::test]
    async fn off_writes_straight_to_the_store_and_frozen_distils_nothing() {
        let temp = workspace("guarded_commit = \"off\"");
        let workdir = temp.path();
        assert_eq!(distil(workdir, episode("ep-off-1", true, &[true])).await, 1);
        let entries = KnowledgeStore::for_workdir(workdir)
            .read_all()
            .expect("read");
        assert_eq!(entries.len(), 1, "{entries:#?}");
        assert!(guard(workdir).rows().expect("the guard's rows").is_empty());

        let temp = workspace("frozen = true");
        let workdir = temp.path();
        assert_eq!(
            distil(workdir, episode("ep-frozen-1", true, &[true])).await,
            0
        );
        assert!(
            KnowledgeStore::for_workdir(workdir)
                .read_all()
                .expect("read")
                .is_empty()
        );
    }
}
