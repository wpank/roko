//! Shared state for per-agent routes with optional disk persistence.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use parking_lot::Mutex;
use roko_agent::chat_types::{
    ChatRequest, ChatResponse, FinishReason, RequestOptions, ResponseMetadata, SessionState,
    ToolChoice,
};
use roko_agent::tool_loop::{LlmBackend, StreamEvent, TurnConfig, collect_stream_to_response};
use roko_agent::translate::{BackendResponse, RenderedTools, normalize_finish_reason};
#[cfg(feature = "chain")]
use roko_chain::ChainClient;

/// Type alias so that the chain client parameter compiles with or without
/// the `chain` feature. When the feature is off, callers pass `None::<()>`.
#[cfg(feature = "chain")]
type OptionalChainClient = Option<Arc<dyn ChainClient>>;
#[cfg(not(feature = "chain"))]
type OptionalChainClient = Option<()>;
use roko_core::obs::LogScrubber;
use roko_core::obs::metrics::{MetricSnapshot, MetricValue};
use roko_core::obs::schema::{self, CanonicalMetricSchema, MetricDescriptor, MetricSchema};
use roko_neuro::KnowledgeStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::registration::{AgentCard, AgentEndpoints};

// ---------------------------------------------------------------------------
// Durable state store
// ---------------------------------------------------------------------------

/// Current schema version for the persisted state envelope.
const STATE_SCHEMA_VERSION: u32 = 1;

/// Versioned on-disk envelope for agent sidecar state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEnvelope {
    /// Agent identifier that owns this state.
    pub agent_id: String,
    /// Schema version for forward-compatibility rejection.
    pub schema_version: u32,
    /// Persisted predictions.
    pub predictions: Vec<AgentPrediction>,
    /// Persisted task queue.
    pub tasks: VecDeque<TaskEntry>,
    /// Idempotency keys for task creation (key -> fingerprint + task_id).
    #[serde(default)]
    pub idempotency_keys: HashMap<String, IdempotencyEntry>,
}

/// Persisted idempotency record for task creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyEntry {
    /// Fingerprint of the normalized request body.
    pub fingerprint: String,
    /// Task ID that was created for this key.
    pub task_id: u64,
}

/// Errors produced by state store operations.
#[derive(Debug)]
pub enum StateStoreError {
    /// The state file belongs to a different agent.
    AgentMismatch {
        /// Expected agent identifier.
        expected: String,
        /// Agent identifier found in the file.
        found: String,
    },
    /// The state file uses a newer schema version.
    NewerSchema {
        /// Maximum supported schema version.
        expected: u32,
        /// Schema version found in the file.
        found: u32,
    },
    /// The state file could not be deserialized.
    Corrupt(String),
    /// An I/O error occurred.
    Io(io::Error),
}

impl std::fmt::Display for StateStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentMismatch { expected, found } => {
                write!(
                    f,
                    "state agent mismatch: expected {expected}, found {found}"
                )
            }
            Self::NewerSchema { expected, found } => {
                write!(
                    f,
                    "state schema version {found} is newer than supported {expected}"
                )
            }
            Self::Corrupt(msg) => write!(f, "corrupt state file: {msg}"),
            Self::Io(err) => write!(f, "state store I/O error: {err}"),
        }
    }
}

impl std::error::Error for StateStoreError {}

impl From<io::Error> for StateStoreError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Trait for durable agent state persistence.
pub trait AgentStateStore: Send + Sync {
    /// Load persisted state for the given agent, returning `None` if no
    /// state file exists.
    fn load(&self, agent_id: &str) -> Result<Option<StateEnvelope>, StateStoreError>;

    /// Persist the current predictions, tasks, and idempotency keys atomically.
    fn persist(
        &self,
        agent_id: &str,
        predictions: &[AgentPrediction],
        tasks: &VecDeque<TaskEntry>,
        idempotency_keys: &HashMap<String, IdempotencyEntry>,
    ) -> Result<(), StateStoreError>;
}

/// File-backed state store using atomic write-then-rename.
pub struct FileStateStore {
    path: PathBuf,
}

impl FileStateStore {
    /// Create a new file-backed store writing to `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the configured state file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AgentStateStore for FileStateStore {
    fn load(&self, agent_id: &str) -> Result<Option<StateEnvelope>, StateStoreError> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(StateStoreError::Io(err)),
        };

        let envelope: StateEnvelope = serde_json::from_slice(&data)
            .map_err(|err| StateStoreError::Corrupt(err.to_string()))?;

        if envelope.agent_id != agent_id {
            return Err(StateStoreError::AgentMismatch {
                expected: agent_id.to_string(),
                found: envelope.agent_id,
            });
        }

        if envelope.schema_version > STATE_SCHEMA_VERSION {
            return Err(StateStoreError::NewerSchema {
                expected: STATE_SCHEMA_VERSION,
                found: envelope.schema_version,
            });
        }

        Ok(Some(envelope))
    }

    fn persist(
        &self,
        agent_id: &str,
        predictions: &[AgentPrediction],
        tasks: &VecDeque<TaskEntry>,
        idempotency_keys: &HashMap<String, IdempotencyEntry>,
    ) -> Result<(), StateStoreError> {
        let envelope = StateEnvelope {
            agent_id: agent_id.to_string(),
            schema_version: STATE_SCHEMA_VERSION,
            predictions: predictions.to_vec(),
            tasks: tasks.clone(),
            idempotency_keys: idempotency_keys.clone(),
        };

        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|err| StateStoreError::Corrupt(err.to_string()))?;

        roko_fs::atomic_write_bytes(&self.path, json.as_bytes())?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Heartbeat snapshot
// ---------------------------------------------------------------------------

/// Consistent snapshot of task/metric state for heartbeat payloads.
#[derive(Debug, Clone)]
pub struct HeartbeatSnapshot {
    /// Tasks in Open or Accepted state.
    pub active_tasks: usize,
    /// Tasks in Completed state.
    pub completed_tasks: usize,
    /// Always zero: the current `TaskState` enum has no failed variant.
    pub failed_tasks: usize,
    /// Bounded allowlisted metric counters.
    pub metrics: HashMap<String, f64>,
}

// ---------------------------------------------------------------------------
// Existing types (unchanged)
// ---------------------------------------------------------------------------

/// Opaque message context payload that round-trips caller JSON as-is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageContext(serde_json::Value);

/// Errors returned by the message dispatch seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarDispatchError {
    /// No dispatcher was configured for this request.
    NotConfigured,
    /// Dispatch failed after reaching a configured backend.
    DispatchFailed(String),
}

impl std::fmt::Display for SidecarDispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => f.write_str("no configured dispatcher"),
            Self::DispatchFailed(reason) => write!(f, "dispatch failed: {reason}"),
        }
    }
}

impl std::error::Error for SidecarDispatchError {}

/// Message dispatch abstraction used by messaging routes.
#[async_trait]
pub trait DispatchLike: Send + Sync {
    /// Dispatch a non-streaming message turn.
    async fn dispatch(&self, request: ChatRequest) -> Result<ChatResponse, SidecarDispatchError>;

    /// Dispatch a streaming message turn.
    ///
    /// `event_tx` is bounded: `send(..).await` waits while the stream's reader
    /// is behind, and fails once the reader is gone, so it cannot deadlock.
    async fn dispatch_streaming(
        &self,
        request: ChatRequest,
        event_tx: mpsc::Sender<StreamEvent>,
    ) -> Result<ChatResponse, SidecarDispatchError> {
        let _ = event_tx;
        self.dispatch(request).await
    }
}

struct BackendMessageDispatcher {
    backend: Arc<dyn LlmBackend>,
}

impl BackendMessageDispatcher {
    fn new(backend: Arc<dyn LlmBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl DispatchLike for BackendMessageDispatcher {
    async fn dispatch(&self, request: ChatRequest) -> Result<ChatResponse, SidecarDispatchError> {
        let messages = request
            .messages
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SidecarDispatchError::DispatchFailed(error.to_string()))?;
        let response = self
            .backend
            .send_turn(
                &messages,
                &RenderedTools::JsonArray(serde_json::json!([])),
                &SessionState::default(),
            )
            .await
            .map_err(|error| SidecarDispatchError::DispatchFailed(error.to_string()))?;
        Ok(chat_response_from_backend(&*self.backend, &response))
    }

    async fn dispatch_streaming(
        &self,
        request: ChatRequest,
        _event_tx: mpsc::Sender<StreamEvent>,
    ) -> Result<ChatResponse, SidecarDispatchError> {
        let messages = request
            .messages
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SidecarDispatchError::DispatchFailed(error.to_string()))?;

        let config = TurnConfig::default();
        let session = SessionState::default();
        let tools = RenderedTools::JsonArray(serde_json::json!([]));

        let stream = self
            .backend
            .stream_turn(&messages, &tools, &session, &config)
            .await
            .map_err(|error| SidecarDispatchError::DispatchFailed(error.to_string()))?;

        let request_start = std::time::Instant::now();
        let response = collect_stream_to_response(stream, request_start)
            .await
            .map_err(|error| SidecarDispatchError::DispatchFailed(error.to_string()))?;

        Ok(chat_response_from_backend(&*self.backend, &response))
    }
}

fn chat_response_from_backend(
    backend: &dyn LlmBackend,
    response: &BackendResponse,
) -> ChatResponse {
    let finish_reason = response_finish_reason(response).unwrap_or(FinishReason::Stop);

    ChatResponse {
        content: response.extract_text(),
        reasoning: response.extract_reasoning(),
        tool_calls: Vec::new(),
        usage: response.extract_usage(),
        finish_reason,
        metadata: ResponseMetadata::default(),
        raw_assistant_message: None,
        session: backend.extract_session(response),
    }
}

/// The finish reason a response names, read through the canonical mapping.
/// Gemini names its reasons in upper case (`STOP`, `MAX_TOKENS`); they read
/// lower-cased, as `gemini::native` reads them, not as errors (bug-e3940b).
fn response_finish_reason(response: &BackendResponse) -> Option<FinishReason> {
    match response {
        BackendResponse::Json(value) => value
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .map(normalize_finish_reason)
            .or_else(|| {
                value
                    .pointer("/candidates/0/finishReason")
                    .and_then(Value::as_str)
                    .map(|reason| normalize_finish_reason(&reason.to_ascii_lowercase()))
            }),
        BackendResponse::StreamJson(_) | BackendResponse::Text(_) => None,
    }
}

fn chat_request(prompt: &str, stream: bool) -> ChatRequest {
    ChatRequest {
        messages: vec![
            serde_json::from_value(serde_json::json!({
                "role": "user",
                "content": prompt,
            }))
            .unwrap_or_else(|error| panic!("valid message request: {error}")),
        ],
        model_slug: String::new(),
        tools: Vec::new(),
        tool_choice: ToolChoice::Auto,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        stream,
        options: RequestOptions::default(),
    }
}

/// Internal and exported agent metrics.
#[derive(Debug, Default)]
pub struct AgentMetrics {
    request_count: AtomicU64,
    message_count: AtomicU64,
}

impl AgentMetrics {
    /// Record an inbound request.
    pub fn record_request(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message request.
    pub fn record_message(&self) {
        self.record_request();
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Export current counters.
    #[must_use]
    pub fn snapshot(&self) -> serde_json::Value {
        let request_count = self.request_count.load(Ordering::Relaxed);
        let message_count = self.message_count.load(Ordering::Relaxed);
        serde_json::json!({
            "schema_version": CanonicalMetricSchema::schema_version(),
            "families": [
                counter_snapshot(
                    &schema::ROKO_AGENT_SERVER_REQUESTS_TOTAL_DESCRIPTOR,
                    request_count,
                ),
                counter_snapshot(
                    &schema::ROKO_AGENT_SERVER_MESSAGE_REQUESTS_TOTAL_DESCRIPTOR,
                    message_count,
                ),
            ],
            "requests": request_count,
            "messages": message_count,
        })
    }
}

fn counter_snapshot(descriptor: &MetricDescriptor, value: u64) -> MetricSnapshot {
    debug_assert_eq!(descriptor.kind, roko_core::obs::MetricKind::Counter);
    debug_assert!(descriptor.labels.is_empty());
    MetricSnapshot {
        name: descriptor.name.to_string(),
        help: descriptor.help.to_string(),
        kind: descriptor.kind,
        labels: Vec::new(),
        value: MetricValue::Counter(value),
    }
}

/// Stats payload shaped to resemble the legacy mirage agent stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRuntimeStats {
    /// Number of confirmations.
    pub confirmations_given: u64,
    /// Number of challenges.
    pub challenges_given: u64,
    /// Number of warnings.
    pub warnings_posted: u64,
    /// Number of insights posted.
    pub insights_posted: u64,
    /// Number of tasks completed.
    pub tasks_completed: u64,
    /// Number of failed tasks.
    pub tasks_failed: u64,
    /// Number of cognitive cycles.
    pub delta_cycles: u64,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Total token usage.
    pub total_tokens: u64,
}

/// Minimal prediction record exposed by the predictions feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrediction {
    /// Unique identifier.
    pub id: String,
    /// Source agent identifier.
    pub agent_id: String,
    /// Market or question label.
    pub market: String,
    /// Free-form category.
    #[serde(default)]
    pub category: String,
    /// Direction label.
    pub direction: String,
    /// Confidence score.
    pub confidence: f64,
    /// Predicted numeric value.
    #[serde(default)]
    pub predicted_value: f64,
    /// Optional interval width.
    #[serde(default)]
    pub interval_width: f64,
    /// Optional observed value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<f64>,
    /// Unix timestamp.
    pub ts: u64,
}

/// Request payload for `POST /predictions`.
#[derive(Debug, Clone, Deserialize)]
pub struct PredictionCreateRequest {
    /// Market or question label.
    pub market: String,
    /// Direction label.
    pub direction: String,
    /// Confidence score.
    #[serde(default)]
    pub confidence: f64,
    /// Optional category.
    #[serde(default)]
    pub category: String,
    /// Optional numeric prediction.
    #[serde(default)]
    pub predicted_value: f64,
    /// Optional interval width.
    #[serde(default)]
    pub interval_width: f64,
    /// Optional observed value.
    #[serde(default)]
    pub actual_value: Option<f64>,
}

/// Per-prediction residual summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPredictionResidual {
    /// Prediction identifier.
    pub id: String,
    /// Residual value.
    pub residual: f64,
}

/// Request payload for the research route.
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchRequest {
    /// Topic to investigate.
    pub topic: String,
    /// Depth hint.
    #[serde(default = "default_research_depth")]
    pub depth: String,
    /// Research mode. Defaults to `local_knowledge`.
    #[serde(default)]
    pub mode: ResearchMode,
}

fn default_research_depth() -> String {
    "shallow".to_string()
}

/// Research response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResponse {
    /// Research mode used for this response.
    pub mode: ResearchMode,
    /// Main findings.
    pub findings: Vec<String>,
    /// Source descriptors.
    pub sources: Vec<String>,
}

/// Task priority labels.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    /// Low priority.
    Low,
    /// Medium priority.
    #[default]
    Medium,
    /// High priority.
    High,
    /// Critical priority.
    Critical,
}

/// Task lifecycle labels.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Open for work.
    #[default]
    Open,
    /// Accepted by the agent.
    Accepted,
    /// Completed successfully.
    Completed,
}

/// Task artifact payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskArtifact {
    /// Artifact kind.
    #[serde(default)]
    pub kind: String,
    /// Artifact label.
    #[serde(default)]
    pub label: String,
    /// Stable content hash.
    #[serde(default)]
    pub content_hash: String,
    /// Optional URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Condensed task summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskSummary {
    /// Human-readable completion summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Agent task entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    /// Unique task identifier.
    pub id: u64,
    /// Task title.
    pub title: String,
    /// Task kind.
    #[serde(default)]
    pub kind: String,
    /// Priority.
    #[serde(default)]
    pub priority: TaskPriority,
    /// Lifecycle state.
    #[serde(default)]
    pub state: TaskState,
    /// Optional bounty amount.
    #[serde(default)]
    pub bounty: u64,
    /// Optional assigned agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Creation timestamp.
    pub created_at: u64,
    /// Optional completion timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    /// Optional artifacts.
    #[serde(default)]
    pub artifacts: Vec<TaskArtifact>,
    /// Optional summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Completion payload for a task.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TaskCompletionRequest {
    /// Produced artifacts.
    #[serde(default)]
    pub artifacts: Vec<TaskArtifact>,
    /// Optional completion summary.
    #[serde(default, alias = "proof")]
    pub summary: Option<String>,
}

/// Maximum length for task title.
const MAX_TASK_TITLE_LEN: usize = 512;

/// Maximum length for task kind.
const MAX_TASK_KIND_LEN: usize = 128;

/// Maximum length for client request ID (idempotency key).
const MAX_CLIENT_REQUEST_ID_LEN: usize = 128;

/// Maximum bounty value.
const MAX_BOUNTY: u64 = 1_000_000;

/// Request payload for `POST /tasks`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    /// Client-provided idempotency key. Repeated identical requests
    /// return the same task; same key with different body returns 409.
    #[serde(default)]
    pub client_request_id: Option<String>,
    /// Task title (required, max 512 chars).
    pub title: String,
    /// Task kind (optional, max 128 chars).
    #[serde(default)]
    pub kind: String,
    /// Priority (defaults to medium).
    #[serde(default)]
    pub priority: TaskPriority,
    /// Bounty amount (optional, max 1_000_000).
    #[serde(default)]
    pub bounty: u64,
}

/// Result of a task creation attempt.
#[derive(Debug)]
pub enum CreateTaskResult {
    /// A new task was created and persisted.
    Created(TaskEntry),
    /// The idempotency key matched an identical prior request.
    Duplicate(TaskEntry),
    /// The idempotency key was reused with a different body.
    Conflict,
    /// The request failed validation.
    Invalid(String),
    /// No durable state store is attached; task creation is unavailable.
    Unavailable,
}

/// Research mode labels.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchMode {
    /// Local knowledge store lookup only.
    #[default]
    LocalKnowledge,
    /// Active web/LLM research (not yet supported).
    Active,
}

/// Resolved dispatch profile injected once at server construction (#283).
///
/// Captures model/role/cost parameters that were previously resolved inline
/// per call. Both HTTP messaging and relay routes reuse this profile. These
/// paths do not generate plans.
#[derive(Debug, Clone)]
pub struct DispatchProfile {
    /// Resolved model key for this agent's dispatch.
    pub model: Option<String>,
    /// Agent role.
    pub role: String,
    /// Cost accounting label.
    pub cost_label: Option<String>,
}

impl DispatchProfile {
    /// Create a new dispatch profile with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            model: None,
            role: "implementer".to_string(),
            cost_label: None,
        }
    }
}

impl Default for DispatchProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state for agent-server routes.
pub struct AgentSidecarState {
    agent_id: String,
    owner: Option<String>,
    version: String,
    capabilities: Vec<String>,
    log_path: PathBuf,
    routes: Vec<String>,
    started_at: Instant,
    registered_at: u64,
    #[cfg_attr(not(feature = "chain"), allow(dead_code))]
    chain_client: OptionalChainClient,
    // Held as Arc owner: message_dispatcher clones this at construction;
    // the field keeps the backend alive for the sidecar lifetime.
    #[allow(dead_code)]
    llm_backend: Option<Arc<dyn LlmBackend>>,
    message_dispatcher: Option<Arc<dyn DispatchLike>>,
    knowledge_store: Option<Arc<KnowledgeStore>>,
    dispatch_profile: DispatchProfile,
    predictions: Mutex<Vec<AgentPrediction>>,
    tasks: Mutex<VecDeque<TaskEntry>>,
    idempotency_keys: Mutex<HashMap<String, IdempotencyEntry>>,
    next_task_id: AtomicU64,
    stats: Mutex<AgentRuntimeStats>,
    metrics: AgentMetrics,
    state_store: Option<Arc<dyn AgentStateStore>>,
}

/// Backward-compatible alias for [`AgentSidecarState`].
pub type AgentState = AgentSidecarState;

impl AgentSidecarState {
    /// Build a new shared state instance.
    #[must_use]
    pub fn new(
        agent_id: String,
        owner: Option<String>,
        version: String,
        capabilities: Vec<String>,
        chain_client: OptionalChainClient,
        llm_backend: Option<Arc<dyn LlmBackend>>,
        knowledge_store: Option<Arc<KnowledgeStore>>,
    ) -> Self {
        let routes = build_routes(&capabilities);
        let message_dispatcher = llm_backend.as_ref().map(|backend| {
            Arc::new(BackendMessageDispatcher::new(Arc::clone(backend))) as Arc<dyn DispatchLike>
        });
        let log_path = default_log_path(&agent_id);
        Self {
            agent_id,
            owner,
            version,
            capabilities,
            log_path,
            routes,
            started_at: Instant::now(),
            registered_at: now_secs(),
            chain_client,
            llm_backend,
            message_dispatcher,
            knowledge_store,
            dispatch_profile: DispatchProfile::default(),
            predictions: Mutex::new(Vec::new()),
            tasks: Mutex::new(VecDeque::new()),
            idempotency_keys: Mutex::new(HashMap::new()),
            next_task_id: AtomicU64::new(1),
            stats: Mutex::new(AgentRuntimeStats::default()),
            metrics: AgentMetrics::default(),
            state_store: None,
        }
    }

    /// Return the configured agent identifier.
    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Return the start instant.
    #[must_use]
    pub const fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Return the path used for the sidecar log file.
    #[must_use]
    pub fn log_path(&self) -> &Path {
        self.log_path.as_path()
    }

    /// Append one scrubbed line to the sidecar log file.
    pub async fn append_log_line(&self, line: impl Into<String>) {
        let log_path = self.log_path.clone();
        let display_path = self.log_path.display().to_string();
        let line = line.into();

        match tokio::task::spawn_blocking(move || append_log_line_sync(&log_path, &line)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::warn!(path = %display_path, %error, "failed to append sidecar log line");
            }
            Err(error) => {
                tracing::warn!(path = %display_path, %error, "sidecar log append task failed");
            }
        }
    }

    /// Borrow the metrics registry.
    #[must_use]
    pub const fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }

    /// Borrow the configured LLM backend, if one is attached.
    #[must_use]
    pub fn llm_backend(&self) -> Option<&Arc<dyn LlmBackend>> {
        self.llm_backend.as_ref()
    }

    /// Borrow the configured message dispatcher, if one is attached.
    #[must_use]
    pub fn message_dispatcher(&self) -> Option<Arc<dyn DispatchLike>> {
        self.message_dispatcher.as_ref().map(Arc::clone)
    }

    /// Dispatch one non-streaming prompt through the configured message seam.
    ///
    /// # Errors
    ///
    /// Returns an error when no dispatcher is configured or when the selected
    /// backend fails to complete the turn.
    pub async fn dispatch_prompt(
        &self,
        prompt: &str,
    ) -> Result<ChatResponse, SidecarDispatchError> {
        self.metrics.record_message();
        let dispatcher = self
            .message_dispatcher()
            .ok_or(SidecarDispatchError::NotConfigured)?;
        let response = dispatcher.dispatch(chat_request(prompt, false)).await;
        let status = if response.is_ok() { "ok" } else { "error" };
        self.append_log_line(format!("message prompt={prompt:?} status={status}"))
            .await;
        response
    }

    /// Attach a dispatcher used to service message routes.
    #[must_use]
    pub fn with_message_dispatcher(mut self, dispatcher: Arc<dyn DispatchLike>) -> Self {
        self.message_dispatcher = Some(dispatcher);
        self
    }

    /// Inject a resolved dispatch profile for model/role/cost resolution (#283).
    ///
    /// Both HTTP and relay messaging routes reuse this profile instead of
    /// resolving factory/model parameters per call.
    #[must_use]
    pub fn with_dispatch_profile(mut self, profile: DispatchProfile) -> Self {
        self.dispatch_profile = profile;
        self
    }

    /// Borrow the dispatch profile.
    #[must_use]
    pub fn dispatch_profile(&self) -> &DispatchProfile {
        &self.dispatch_profile
    }

    /// Override the path used for the sidecar log file.
    #[must_use]
    pub fn with_log_path(mut self, log_path: impl Into<PathBuf>) -> Self {
        self.log_path = log_path.into();
        self
    }

    /// Attach a durable state store for prediction/task persistence.
    #[must_use]
    pub fn with_state_store(mut self, store: Arc<dyn AgentStateStore>) -> Self {
        self.state_store = Some(store);
        self
    }

    /// Restore persisted predictions and tasks from the attached store.
    ///
    /// Must be called before serving routes. Returns the number of
    /// restored predictions and tasks, or an error if the state is
    /// corrupt/mismatched.
    ///
    /// # Errors
    ///
    /// Returns an error if the store returns `AgentMismatch`,
    /// `NewerSchema`, or `Corrupt`. Missing state (first boot) is not
    /// an error.
    pub fn restore_state(&self) -> Result<(usize, usize), StateStoreError> {
        let store = match &self.state_store {
            Some(s) => s,
            None => return Ok((0, 0)),
        };
        match store.load(&self.agent_id)? {
            Some(envelope) => {
                let pred_count = envelope.predictions.len();
                let task_count = envelope.tasks.len();
                // Set next_task_id to one past the highest existing task ID.
                let max_id = envelope.tasks.iter().map(|t| t.id).max().unwrap_or(0);
                self.next_task_id.store(max_id + 1, Ordering::Relaxed);
                *self.predictions.lock() = envelope.predictions;
                *self.tasks.lock() = envelope.tasks;
                *self.idempotency_keys.lock() = envelope.idempotency_keys;
                tracing::info!(
                    agent_id = %self.agent_id,
                    predictions = pred_count,
                    tasks = task_count,
                    "restored durable sidecar state"
                );
                Ok((pred_count, task_count))
            }
            None => Ok((0, 0)),
        }
    }

    /// Persist current predictions, tasks, and idempotency keys to the attached store.
    fn persist_state(&self) {
        if let Some(store) = &self.state_store {
            let predictions = self.predictions.lock().clone();
            let tasks = self.tasks.lock().clone();
            let idem_keys = self.idempotency_keys.lock().clone();
            if let Err(err) = store.persist(&self.agent_id, &predictions, &tasks, &idem_keys) {
                tracing::warn!(
                    agent_id = %self.agent_id,
                    error = %err,
                    "failed to persist sidecar state"
                );
            }
        }
    }

    /// Take a consistent snapshot for heartbeat payloads.
    ///
    /// Active = Open + Accepted; Completed = Completed; Failed = 0
    /// (no failed state exists in the current `TaskState` enum).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn heartbeat_snapshot(&self) -> HeartbeatSnapshot {
        let tasks = self.tasks.lock();
        let mut active: usize = 0;
        let mut completed: usize = 0;
        for task in tasks.iter() {
            match task.state {
                TaskState::Open | TaskState::Accepted => active += 1,
                TaskState::Completed => completed += 1,
            }
        }
        drop(tasks);

        let request_count = self.metrics.request_count.load(Ordering::Relaxed);
        let message_count = self.metrics.message_count.load(Ordering::Relaxed);
        let mut metrics = HashMap::new();
        metrics.insert("requests_total".to_string(), request_count as f64);
        metrics.insert("message_requests_total".to_string(), message_count as f64);

        HeartbeatSnapshot {
            active_tasks: active,
            completed_tasks: completed,
            failed_tasks: 0,
            metrics,
        }
    }

    /// Build the public capabilities manifest.
    #[must_use]
    pub fn capabilities_manifest(&self) -> serde_json::Value {
        let skills = self
            .capabilities
            .iter()
            .map(|capability| {
                (
                    capability.clone(),
                    serde_json::json!({
                        "enabled": true,
                        "config": {},
                    }),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();

        serde_json::json!({
            "agent_id": self.agent_id,
            "features": self.capabilities,
            "routes": self.routes,
            "owner": self.owner,
            "registered_at": self.registered_at,
            "skills": skills,
            "stats": self.stats_payload(),
        })
    }

    /// Export the current stats payload.
    #[must_use]
    pub fn stats_payload(&self) -> serde_json::Value {
        let stats = self.stats.lock().clone();

        #[cfg(feature = "chain")]
        let chain_backend = self
            .chain_client
            .as_ref()
            .map(|client| client.name().to_string());
        #[cfg(not(feature = "chain"))]
        let chain_backend: Option<String> = None;

        let freq = operating_frequency(stats.tasks_completed + stats.tasks_failed);
        let metrics_snap = self.metrics.snapshot();
        serde_json::json!({
            "agent_id": self.agent_id,
            "owner": self.owner,
            "confirmations_given": stats.confirmations_given,
            "challenges_given": stats.challenges_given,
            "warnings_posted": stats.warnings_posted,
            "insights_posted": stats.insights_posted,
            "tasks_completed": stats.tasks_completed,
            "tasks_failed": stats.tasks_failed,
            "delta_cycles": stats.delta_cycles,
            "total_cost_usd": stats.total_cost_usd,
            "total_tokens": stats.total_tokens,
            "registered_at": self.registered_at,
            "operating_frequency": freq,
            "metrics": metrics_snap,
            "chain_backend": chain_backend,
        })
    }

    /// Build the agent card corresponding to this runtime.
    #[must_use]
    pub fn build_agent_card(&self, addr: std::net::SocketAddr) -> AgentCard {
        let host = if addr.ip().is_unspecified() {
            "127.0.0.1".to_string()
        } else {
            addr.ip().to_string()
        };
        let rest = format!("http://{host}:{}", addr.port());
        let websocket = format!("ws://{host}:{}/stream", addr.port());
        AgentCard {
            name: self.agent_id.clone(),
            capabilities: self.capabilities.clone(),
            endpoints: AgentEndpoints {
                rest: Some(rest),
                websocket: Some(websocket),
                a2a: None,
                mcp: None,
            },
            domain_tags: vec!["roko".to_string()],
            version: self.version.clone(),
        }
    }

    /// Create a prediction entry. Persists to disk before returning.
    #[allow(clippy::unused_async)]
    pub async fn create_prediction(&self, request: PredictionCreateRequest) -> AgentPrediction {
        self.metrics.record_request();
        let prediction = AgentPrediction {
            id: format!("pred-{}", Uuid::new_v4()),
            agent_id: self.agent_id.clone(),
            market: request.market,
            category: request.category,
            direction: request.direction,
            confidence: request.confidence,
            predicted_value: request.predicted_value,
            interval_width: request.interval_width,
            actual_value: request.actual_value,
            ts: now_secs(),
        };
        self.predictions.lock().push(prediction.clone());
        self.persist_state();
        prediction
    }

    /// Return all stored predictions.
    #[allow(clippy::unused_async)]
    pub async fn list_predictions(&self) -> Vec<AgentPrediction> {
        self.metrics.record_request();
        self.predictions.lock().clone()
    }

    /// Fetch a prediction by identifier.
    #[allow(clippy::unused_async)]
    pub async fn get_prediction(&self, id: &str) -> Option<AgentPrediction> {
        self.metrics.record_request();
        self.predictions
            .lock()
            .iter()
            .find(|prediction| prediction.id == id)
            .cloned()
    }

    /// Summarize prediction residuals.
    #[allow(
        clippy::cast_precision_loss,
        clippy::significant_drop_tightening,
        clippy::unused_async
    )]
    pub async fn prediction_residuals(&self) -> serde_json::Value {
        self.metrics.record_request();
        let predictions = self.predictions.lock();
        let residuals: Vec<AgentPredictionResidual> = predictions
            .iter()
            .filter_map(|prediction| {
                prediction
                    .actual_value
                    .map(|actual| AgentPredictionResidual {
                        id: prediction.id.clone(),
                        residual: (prediction.predicted_value - actual).abs(),
                    })
            })
            .collect();
        let mse = if residuals.is_empty() {
            0.0
        } else {
            residuals
                .iter()
                .map(|residual| residual.residual.powi(2))
                .sum::<f64>()
                / residuals.len() as f64
        };
        let hit_rate = if residuals.is_empty() {
            0.0
        } else {
            residuals
                .iter()
                .filter(|residual| residual.residual <= 0.1)
                .count() as f64
                / residuals.len() as f64
        };
        serde_json::json!({
            "mse": mse,
            "hit_rate": hit_rate,
            "residuals": residuals,
        })
    }

    /// Run a sidecar-local research lookup against the attached knowledge store.
    ///
    /// Only `local_knowledge` mode is supported. `active` mode returns `None`
    /// to signal 501.
    #[allow(clippy::unused_async)]
    pub async fn research(&self, request: ResearchRequest) -> Option<ResearchResponse> {
        self.metrics.record_request();

        if request.mode == ResearchMode::Active {
            return None;
        }

        let topic = request.topic.trim();
        let mut findings = Vec::new();
        let mut sources = Vec::new();

        if let Some(store) = &self.knowledge_store {
            match store.query(topic, 5) {
                Ok(entries) => {
                    for entry in entries {
                        if !entry.content.trim().is_empty() {
                            findings.push(entry.content);
                        }
                        if let Some(source) = entry.source {
                            sources.push(source);
                        } else if !entry.id.is_empty() {
                            sources.push(entry.id);
                        }
                    }
                }
                Err(error) => {
                    findings.push(format!("local knowledge query failed: {error}"));
                }
            }
        }

        if findings.is_empty() {
            findings.push(format!(
                "no local knowledge findings for topic '{topic}' at {} depth",
                request.depth
            ));
        }

        Some(ResearchResponse {
            mode: ResearchMode::LocalKnowledge,
            findings,
            sources,
        })
    }

    /// Whether a durable state store is attached (controls route availability).
    #[must_use]
    pub fn has_state_store(&self) -> bool {
        self.state_store.is_some()
    }

    /// Create a validated task entry with optional idempotency.
    ///
    /// If `client_request_id` is provided, repeated identical requests return
    /// `Duplicate` and mismatched bodies return `Conflict`. Persists before
    /// returning `Created`.
    #[allow(clippy::unused_async)]
    pub async fn create_task(&self, request: CreateTaskRequest) -> CreateTaskResult {
        self.metrics.record_request();

        // Validate fields.
        let title = request.title.trim();
        if title.is_empty() {
            return CreateTaskResult::Invalid("title must not be empty".to_string());
        }
        if title.len() > MAX_TASK_TITLE_LEN {
            return CreateTaskResult::Invalid(format!(
                "title exceeds {MAX_TASK_TITLE_LEN} characters"
            ));
        }
        if request.kind.len() > MAX_TASK_KIND_LEN {
            return CreateTaskResult::Invalid(format!(
                "kind exceeds {MAX_TASK_KIND_LEN} characters"
            ));
        }
        if request.bounty > MAX_BOUNTY {
            return CreateTaskResult::Invalid(format!("bounty exceeds maximum {MAX_BOUNTY}"));
        }
        if let Some(ref key) = request.client_request_id
            && (key.is_empty() || key.len() > MAX_CLIENT_REQUEST_ID_LEN)
        {
            return CreateTaskResult::Invalid(format!(
                "client_request_id must be 1-{MAX_CLIENT_REQUEST_ID_LEN} characters"
            ));
        }

        // Compute normalized fingerprint for idempotency.
        let fingerprint =
            task_request_fingerprint(title, &request.kind, request.priority, request.bounty);

        // Check idempotency key.
        if let Some(key) = &request.client_request_id {
            let idem = self.idempotency_keys.lock();
            if let Some(entry) = idem.get(key) {
                if entry.fingerprint == fingerprint {
                    // Identical request -- return the existing task.
                    let tasks = self.tasks.lock();
                    if let Some(task) = tasks.iter().find(|t| t.id == entry.task_id) {
                        return CreateTaskResult::Duplicate(task.clone());
                    }
                }
                return CreateTaskResult::Conflict;
            }
        }

        // Allocate ID and create entry.
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let entry = TaskEntry {
            id,
            title: title.to_string(),
            kind: request.kind,
            priority: request.priority,
            state: TaskState::Open,
            bounty: request.bounty,
            assignee: None,
            created_at: now_secs(),
            completed_at: None,
            artifacts: Vec::new(),
            summary: None,
        };

        self.tasks.lock().push_back(entry.clone());
        if let Some(key) = request.client_request_id {
            self.idempotency_keys.lock().insert(
                key,
                IdempotencyEntry {
                    fingerprint,
                    task_id: id,
                },
            );
        }
        self.persist_state();
        CreateTaskResult::Created(entry)
    }

    /// List the in-memory sidecar task queue.
    #[allow(clippy::unused_async)]
    pub async fn list_tasks(&self) -> Vec<TaskEntry> {
        self.metrics.record_request();
        self.tasks.lock().iter().cloned().collect()
    }

    /// Mark a queued task as accepted by this agent.
    ///
    /// Idempotent: re-accepting an already-accepted task returns the same
    /// entry. Accepting a completed task returns `None` (conflict).
    #[allow(clippy::unused_async, clippy::significant_drop_tightening)]
    pub async fn accept_task(&self, id: u64) -> Option<TaskEntry> {
        self.metrics.record_request();
        let mut tasks = self.tasks.lock();
        let task = tasks.iter_mut().find(|task| task.id == id)?;
        match task.state {
            TaskState::Accepted => return Some(task.clone()),
            TaskState::Completed => return None,
            TaskState::Open => {}
        }
        task.state = TaskState::Accepted;
        task.assignee = Some(self.agent_id.clone());
        let result = task.clone();
        drop(tasks);
        self.persist_state();
        Some(result)
    }

    /// Mark a queued task as completed and attach its artifacts.
    ///
    /// Idempotent: re-completing with the same payload returns the entry.
    #[allow(clippy::unused_async, clippy::significant_drop_tightening)]
    pub async fn complete_task(
        &self,
        id: u64,
        request: TaskCompletionRequest,
    ) -> Option<TaskEntry> {
        self.metrics.record_request();
        let mut tasks = self.tasks.lock();
        let task = tasks.iter_mut().find(|task| task.id == id)?;
        if task.state == TaskState::Completed {
            return Some(task.clone());
        }
        task.state = TaskState::Completed;
        task.completed_at = Some(now_secs());
        task.artifacts = request.artifacts;
        task.summary = request.summary;
        let result = task.clone();
        drop(tasks);
        self.persist_state();
        Some(result)
    }
}

/// Compute a deterministic fingerprint from the normalized request fields.
fn task_request_fingerprint(
    title: &str,
    kind: &str,
    priority: TaskPriority,
    bounty: u64,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\x00");
    hasher.update(kind.as_bytes());
    hasher.update(b"\x00");
    hasher.update(format!("{priority:?}").as_bytes());
    hasher.update(b"\x00");
    hasher.update(bounty.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

fn build_routes(capabilities: &[String]) -> Vec<String> {
    let mut routes = vec![
        "/health".to_string(),
        "/capabilities".to_string(),
        "/stats".to_string(),
        "/logs".to_string(),
    ];
    if capabilities
        .iter()
        .any(|capability| capability == "messaging")
    {
        routes.push("/message".to_string());
        routes.push("/stream".to_string());
    }
    if capabilities
        .iter()
        .any(|capability| capability == "predictions")
    {
        routes.push("/predictions".to_string());
        routes.push("/predictions/{id}".to_string());
        routes.push("/predictions/residuals".to_string());
    }
    if capabilities
        .iter()
        .any(|capability| capability == "research")
    {
        routes.push("/research".to_string());
    }
    if capabilities.iter().any(|capability| capability == "tasks") {
        routes.push("/tasks".to_string());
        routes.push("/tasks/{id}/accept".to_string());
        routes.push("/tasks/{id}/complete".to_string());
    }
    routes
}

const fn operating_frequency(task_count: u64) -> &'static str {
    match task_count {
        0 => "idle",
        1..=2 => "reactive",
        3..=5 => "active",
        _ => "intensive",
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn default_log_path(agent_id: &str) -> PathBuf {
    PathBuf::from(".roko")
        .join("agents")
        .join(agent_id)
        .join("log")
}

fn append_log_line_sync(path: &Path, line: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let scrubbed = LogScrubber::default().scrub(line);
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{scrubbed}")?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_agent::tool_loop::LlmError;

    fn make_state(store: Arc<dyn AgentStateStore>) -> AgentState {
        AgentState::new(
            "test-agent".to_string(),
            None,
            "0.1.0".to_string(),
            vec!["predictions".to_string(), "tasks".to_string()],
            None,
            None,
            None,
        )
        .with_state_store(store)
    }

    fn make_prediction_request() -> PredictionCreateRequest {
        PredictionCreateRequest {
            market: "ETH-USD".to_string(),
            direction: "up".to_string(),
            confidence: 0.85,
            category: String::new(),
            predicted_value: 3000.0,
            interval_width: 0.0,
            actual_value: None,
        }
    }

    fn seed_task(state: &AgentState, title: &str) -> u64 {
        let id = now_secs();
        let entry = TaskEntry {
            id,
            title: title.to_string(),
            kind: String::new(),
            priority: TaskPriority::Medium,
            state: TaskState::Open,
            bounty: 0,
            assignee: None,
            created_at: now_secs(),
            completed_at: None,
            artifacts: Vec::new(),
            summary: None,
        };
        state.tasks.lock().push_back(entry);
        state.persist_state();
        id
    }

    // -----------------------------------------------------------------------
    // FileStateStore unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn state_store_persist_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileStateStore::new(dir.path().join("state.json"));

        let predictions = vec![AgentPrediction {
            id: "pred-1".to_string(),
            agent_id: "test-agent".to_string(),
            market: "BTC-USD".to_string(),
            category: String::new(),
            direction: "up".to_string(),
            confidence: 0.9,
            predicted_value: 50_000.0,
            interval_width: 0.0,
            actual_value: None,
            ts: 1,
        }];
        let mut tasks = VecDeque::new();
        tasks.push_back(TaskEntry {
            id: 42,
            title: "test task".to_string(),
            kind: String::new(),
            priority: TaskPriority::High,
            state: TaskState::Open,
            bounty: 0,
            assignee: None,
            created_at: 1,
            completed_at: None,
            artifacts: Vec::new(),
            summary: None,
        });

        store
            .persist("test-agent", &predictions, &tasks, &HashMap::new())
            .expect("persist");
        let envelope = store.load("test-agent").expect("load").expect("some");

        assert_eq!(envelope.agent_id, "test-agent");
        assert_eq!(envelope.schema_version, STATE_SCHEMA_VERSION);
        assert_eq!(envelope.predictions.len(), 1);
        assert_eq!(envelope.predictions[0].id, "pred-1");
        assert_eq!(envelope.tasks.len(), 1);
        assert_eq!(envelope.tasks[0].id, 42);
    }

    #[test]
    fn state_store_load_missing_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileStateStore::new(dir.path().join("nonexistent.json"));
        assert!(store.load("test-agent").expect("load").is_none());
    }

    #[test]
    fn state_store_rejects_agent_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileStateStore::new(dir.path().join("state.json"));

        store
            .persist("agent-a", &[], &VecDeque::new(), &HashMap::new())
            .expect("persist");
        let err = store.load("agent-b").expect_err("mismatch");
        assert!(
            matches!(err, StateStoreError::AgentMismatch { .. }),
            "expected AgentMismatch, got: {err}"
        );
    }

    #[test]
    fn state_store_rejects_newer_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state.json");
        let envelope = serde_json::json!({
            "agent_id": "test-agent",
            "schema_version": STATE_SCHEMA_VERSION + 1,
            "predictions": [],
            "tasks": [],
        });
        fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();

        let store = FileStateStore::new(path);
        let err = store.load("test-agent").expect_err("newer schema");
        assert!(
            matches!(err, StateStoreError::NewerSchema { .. }),
            "expected NewerSchema, got: {err}"
        );
    }

    #[test]
    fn state_store_rejects_corrupt_data() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state.json");
        fs::write(&path, "not valid json {{{{").unwrap();

        let store = FileStateStore::new(path);
        let err = store.load("test-agent").expect_err("corrupt");
        assert!(
            matches!(err, StateStoreError::Corrupt(_)),
            "expected Corrupt, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // AgentState integration tests with FileStateStore
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn state_store_prediction_survives_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("state.json");

        // Create state, add a prediction.
        let state = make_state(Arc::new(FileStateStore::new(store_path.clone())));
        let pred = state.create_prediction(make_prediction_request()).await;

        // Simulate restart: new state, same store path.
        let state2 = make_state(Arc::new(FileStateStore::new(store_path)));
        let (preds, _tasks) = state2.restore_state().expect("restore");
        assert_eq!(preds, 1);

        let restored = state2.list_predictions().await;
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, pred.id);
        assert_eq!(restored[0].market, "ETH-USD");
    }

    #[tokio::test]
    async fn state_store_task_transitions_survive_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("state.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path.clone())));
        let task_id = seed_task(&state, "implement feature");
        state.accept_task(task_id).await.expect("accept");

        // Restart.
        let state2 = make_state(Arc::new(FileStateStore::new(store_path)));
        state2.restore_state().expect("restore");

        let tasks = state2.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].state, TaskState::Accepted);
        assert_eq!(tasks[0].assignee.as_deref(), Some("test-agent"));
    }

    #[tokio::test]
    async fn state_store_accept_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("state.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path)));
        let task_id = seed_task(&state, "idempotent task");

        let first = state.accept_task(task_id).await.expect("first accept");
        let second = state.accept_task(task_id).await.expect("second accept");
        assert_eq!(first.state, TaskState::Accepted);
        assert_eq!(second.state, TaskState::Accepted);
    }

    #[tokio::test]
    async fn state_store_complete_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("state.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path)));
        let task_id = seed_task(&state, "complete task");

        state.accept_task(task_id).await;
        let req = TaskCompletionRequest {
            artifacts: Vec::new(),
            summary: Some("done".to_string()),
        };
        let first = state
            .complete_task(task_id, req.clone())
            .await
            .expect("first complete");
        let second = state
            .complete_task(task_id, req)
            .await
            .expect("second complete");
        assert_eq!(first.state, TaskState::Completed);
        assert_eq!(second.state, TaskState::Completed);
    }

    #[tokio::test]
    async fn state_store_accept_completed_task_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("state.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path)));
        let task_id = seed_task(&state, "done task");

        state.accept_task(task_id).await;
        state
            .complete_task(
                task_id,
                TaskCompletionRequest {
                    artifacts: Vec::new(),
                    summary: None,
                },
            )
            .await;

        assert!(
            state.accept_task(task_id).await.is_none(),
            "accepting a completed task must return None"
        );
    }

    // -----------------------------------------------------------------------
    // Heartbeat snapshot tests
    // -----------------------------------------------------------------------

    #[test]
    fn heartbeat_snapshot_counts_tasks_correctly() {
        let state = AgentState::new(
            "hb-agent".to_string(),
            None,
            "0.1.0".to_string(),
            vec!["tasks".to_string()],
            None,
            None,
            None,
        );

        // Empty state.
        let snap = state.heartbeat_snapshot();
        assert_eq!(snap.active_tasks, 0);
        assert_eq!(snap.completed_tasks, 0);
        assert_eq!(snap.failed_tasks, 0);

        // Add tasks in various states.
        {
            let mut tasks = state.tasks.lock();
            tasks.push_back(TaskEntry {
                id: 1,
                title: "open".to_string(),
                kind: String::new(),
                priority: TaskPriority::Medium,
                state: TaskState::Open,
                bounty: 0,
                assignee: None,
                created_at: 1,
                completed_at: None,
                artifacts: Vec::new(),
                summary: None,
            });
            tasks.push_back(TaskEntry {
                id: 2,
                title: "accepted".to_string(),
                kind: String::new(),
                priority: TaskPriority::Medium,
                state: TaskState::Accepted,
                bounty: 0,
                assignee: Some("hb-agent".to_string()),
                created_at: 1,
                completed_at: None,
                artifacts: Vec::new(),
                summary: None,
            });
            tasks.push_back(TaskEntry {
                id: 3,
                title: "completed".to_string(),
                kind: String::new(),
                priority: TaskPriority::Medium,
                state: TaskState::Completed,
                bounty: 0,
                assignee: Some("hb-agent".to_string()),
                created_at: 1,
                completed_at: Some(2),
                artifacts: Vec::new(),
                summary: None,
            });
        }

        let snap = state.heartbeat_snapshot();
        assert_eq!(snap.active_tasks, 2, "open + accepted = active");
        assert_eq!(snap.completed_tasks, 1);
        assert_eq!(snap.failed_tasks, 0, "no failed state exists");
    }

    #[test]
    fn heartbeat_snapshot_includes_bounded_metrics() {
        let state = AgentState::new(
            "hb-agent".to_string(),
            None,
            "0.1.0".to_string(),
            Vec::new(),
            None,
            None,
            None,
        );

        state.metrics.record_request();
        state.metrics.record_request();
        state.metrics.record_message();

        let snap = state.heartbeat_snapshot();
        assert_eq!(snap.metrics.get("requests_total"), Some(&3.0));
        assert_eq!(snap.metrics.get("message_requests_total"), Some(&1.0));
        assert_eq!(snap.metrics.len(), 2, "only allowlisted metrics");
    }

    // -----------------------------------------------------------------------
    // No-store mode (backward compat)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn state_without_store_still_works() {
        let state = AgentState::new(
            "no-store-agent".to_string(),
            None,
            "0.1.0".to_string(),
            vec!["predictions".to_string()],
            None,
            None,
            None,
        );

        let pred = state.create_prediction(make_prediction_request()).await;
        assert!(!pred.id.is_empty());
        assert_eq!(state.list_predictions().await.len(), 1);
        assert_eq!(state.restore_state().unwrap(), (0, 0));
    }

    // -----------------------------------------------------------------------
    // Task creation tests (#348)
    // -----------------------------------------------------------------------

    fn make_create_task_request(title: &str) -> CreateTaskRequest {
        CreateTaskRequest {
            client_request_id: None,
            title: title.to_string(),
            kind: String::new(),
            priority: TaskPriority::Medium,
            bounty: 0,
        }
    }

    #[tokio::test]
    async fn create_task_returns_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let result = state
            .create_task(make_create_task_request("build feature"))
            .await;
        assert!(matches!(result, CreateTaskResult::Created(ref t) if t.title == "build feature"));
        assert_eq!(state.list_tasks().await.len(), 1);
    }

    #[tokio::test]
    async fn create_task_rejects_empty_title() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let result = state.create_task(make_create_task_request("")).await;
        assert!(matches!(result, CreateTaskResult::Invalid(_)));
    }

    #[tokio::test]
    async fn create_task_rejects_oversized_title() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let long_title = "x".repeat(MAX_TASK_TITLE_LEN + 1);
        let result = state
            .create_task(make_create_task_request(&long_title))
            .await;
        assert!(matches!(result, CreateTaskResult::Invalid(_)));
    }

    #[tokio::test]
    async fn create_task_rejects_excessive_bounty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let mut req = make_create_task_request("task");
        req.bounty = MAX_BOUNTY + 1;
        let result = state.create_task(req).await;
        assert!(matches!(result, CreateTaskResult::Invalid(_)));
    }

    #[tokio::test]
    async fn create_task_idempotency_returns_duplicate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let mut req = make_create_task_request("idempotent task");
        req.client_request_id = Some("req-1".to_string());
        let first = state.create_task(req.clone()).await;
        let second = state.create_task(req).await;

        let first_id = match &first {
            CreateTaskResult::Created(t) => t.id,
            _ => panic!("expected Created"),
        };
        let second_id = match &second {
            CreateTaskResult::Duplicate(t) => t.id,
            _ => panic!("expected Duplicate"),
        };
        assert_eq!(first_id, second_id);
        assert_eq!(state.list_tasks().await.len(), 1);
    }

    #[tokio::test]
    async fn create_task_idempotency_conflict_on_different_body() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = make_state(Arc::new(FileStateStore::new(dir.path().join("s.json"))));

        let mut req1 = make_create_task_request("task A");
        req1.client_request_id = Some("req-1".to_string());
        state.create_task(req1).await;

        let mut req2 = make_create_task_request("task B");
        req2.client_request_id = Some("req-1".to_string());
        let result = state.create_task(req2).await;
        assert!(matches!(result, CreateTaskResult::Conflict));
        assert_eq!(state.list_tasks().await.len(), 1);
    }

    #[tokio::test]
    async fn create_accept_complete_lifecycle_survives_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("s.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path.clone())));
        let task = match state
            .create_task(make_create_task_request("lifecycle task"))
            .await
        {
            CreateTaskResult::Created(t) => t,
            _ => panic!("expected Created"),
        };

        state.accept_task(task.id).await.expect("accept");
        state
            .complete_task(
                task.id,
                TaskCompletionRequest {
                    artifacts: Vec::new(),
                    summary: Some("done".to_string()),
                },
            )
            .await
            .expect("complete");

        // Restart.
        let state2 = make_state(Arc::new(FileStateStore::new(store_path)));
        state2.restore_state().expect("restore");

        let tasks = state2.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].state, TaskState::Completed);
        assert_eq!(tasks[0].summary.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn create_task_idempotency_survives_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("s.json");

        let state = make_state(Arc::new(FileStateStore::new(store_path.clone())));
        let mut req = make_create_task_request("restart idem");
        req.client_request_id = Some("idem-key".to_string());
        state.create_task(req.clone()).await;

        // Restart, replay same key.
        let state2 = make_state(Arc::new(FileStateStore::new(store_path)));
        state2.restore_state().expect("restore");

        let result = state2.create_task(req).await;
        assert!(matches!(result, CreateTaskResult::Duplicate(_)));
        assert_eq!(state2.list_tasks().await.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Research mode tests (#348)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn research_local_knowledge_returns_response() {
        let state = AgentState::new(
            "r-agent".to_string(),
            None,
            "0.1.0".to_string(),
            vec!["research".to_string()],
            None,
            None,
            None,
        );

        let result = state
            .research(ResearchRequest {
                topic: "test topic".to_string(),
                depth: "shallow".to_string(),
                mode: ResearchMode::LocalKnowledge,
            })
            .await;
        let resp = result.expect("should return Some for local_knowledge");
        assert_eq!(resp.mode, ResearchMode::LocalKnowledge);
        assert!(!resp.findings.is_empty());
    }

    #[tokio::test]
    async fn research_active_mode_returns_none() {
        let state = AgentState::new(
            "r-agent".to_string(),
            None,
            "0.1.0".to_string(),
            vec!["research".to_string()],
            None,
            None,
            None,
        );

        let result = state
            .research(ResearchRequest {
                topic: "test topic".to_string(),
                depth: "deep".to_string(),
                mode: ResearchMode::Active,
            })
            .await;
        assert!(result.is_none(), "active mode must return None for 501");
    }

    #[tokio::test]
    async fn research_default_mode_is_local_knowledge() {
        let req: ResearchRequest =
            serde_json::from_str(r#"{"topic":"test"}"#).expect("deserialize");
        assert_eq!(req.mode, ResearchMode::LocalKnowledge);
    }

    /// Replays OpenAI-compatible SSE lines through roko-agent's stream
    /// parser, as the OpenAI-compatible backend streams a turn.
    struct SseBackend {
        lines: Vec<String>,
    }

    #[async_trait]
    impl LlmBackend for SseBackend {
        async fn send_turn(
            &self,
            _messages: &[Value],
            _tools: &RenderedTools,
            _session: &SessionState,
        ) -> Result<BackendResponse, LlmError> {
            Err(LlmError::Backend("this backend only streams".to_string()))
        }

        async fn stream_turn(
            &self,
            _messages: &[Value],
            _tools: &RenderedTools,
            _session: &SessionState,
            _config: &TurnConfig,
        ) -> Result<futures::stream::BoxStream<'static, Result<StreamEvent, LlmError>>, LlmError>
        {
            let events: Vec<Result<StreamEvent, LlmError>> = self
                .lines
                .iter()
                .flat_map(|line| roko_agent::streaming::parse_sse_line(line))
                .map(Ok)
                .collect();
            Ok(Box::pin(futures::stream::iter(events)))
        }
    }

    /// bug-e3940b: a streamed finish reason keeps its meaning in the
    /// sidecar's response. The SSE parser wrote `Debug` names, which read
    /// back as errors: a `length` finish became `Error("Length")`, and even a
    /// normal `stop` an error.
    #[tokio::test]
    async fn streamed_finish_reason_keeps_its_meaning() {
        for (wire, expected) in [
            ("length", FinishReason::Length),
            ("stop", FinishReason::Stop),
            ("tool_calls", FinishReason::ToolCalls),
        ] {
            let chunk = serde_json::json!({
                "choices": [{"delta": {"content": "cut"}, "finish_reason": wire}]
            });
            let backend = SseBackend {
                lines: vec![format!("data: {chunk}"), "data: [DONE]".to_string()],
            };
            let (event_tx, _event_rx) = mpsc::channel(8);
            let response = BackendMessageDispatcher::new(Arc::new(backend))
                .dispatch_streaming(chat_request("hi", true), event_tx)
                .await
                .expect("the stream dispatches");
            assert_eq!(response.finish_reason, expected, "{wire}");
        }
    }

    /// Gemini's upper-case finish reasons read as the canonical reasons, not
    /// as errors named `STOP` or `MAX_TOKENS` (bug-e3940b).
    #[test]
    fn gemini_finish_reasons_read_lower_cased() {
        let gemini = |reason: &str| {
            BackendResponse::Json(serde_json::json!({
                "candidates": [{"finishReason": reason}]
            }))
        };
        assert_eq!(
            response_finish_reason(&gemini("STOP")),
            Some(FinishReason::Stop)
        );
        assert_eq!(
            response_finish_reason(&gemini("MAX_TOKENS")),
            Some(FinishReason::Length)
        );
    }
}
