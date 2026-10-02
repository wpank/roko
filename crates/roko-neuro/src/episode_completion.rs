//! Background distillation of completed episodes into durable knowledge.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use roko_core::foundation::{
    CachePolicy, ChatMessage, MessageRole, ModelCallRequest, ModelCaller, caller,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::Episode;
use tokio::task;

use crate::{DistillationBackend, Distiller, KnowledgeStore};

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

async fn distill_episode(
    workdir: PathBuf,
    episode: Episode,
    model_caller: Option<Arc<dyn ModelCaller>>,
) -> Result<()> {
    let Some(model_caller) = model_caller else {
        tracing::debug!("no ModelCaller provided; skipping episode distillation");
        return Ok(());
    };

    // Use an empty model string so the ModelCaller resolves via its own
    // default — this respects whatever model the workspace has configured
    // instead of hardcoding a specific provider.
    let distiller =
        Distiller::with_backend(Arc::new(GatewayDistillationBackend::new(model_caller, "")));

    let episodes = [episode];
    let entries = distiller
        .distill(&episodes)
        .await
        .context("distill completed episode")?;

    let store = KnowledgeStore::for_workdir(&workdir);
    task::spawn_blocking(move || -> Result<()> {
        for entry in entries {
            store.add(entry)?;
        }
        Ok(())
    })
    .await
    .context("join knowledge-store writer")??;

    Ok(())
}

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
