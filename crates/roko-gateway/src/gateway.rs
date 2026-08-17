//! Nine-stage inference pipeline assembly and bounded background loop.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use roko_agent::{GatewayEvent, GatewayEventWriter};
use roko_core::agent::AgentRole;
use roko_core::foundation::ModelCaller;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::cost_table::CostTable;
use roko_learn::model_router::RoutingContext;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::backpressure::{BackpressureConfig, BackpressureGuard, BackpressureStats};
use crate::cache::{CacheStats, InferenceCache, simhash};
use crate::convergence::{ConvergenceDetector, ConvergenceStats};
use crate::cost_track::{CostRecord, CostTracker};
use crate::handle::{InferenceEnvelope, InferenceHandle, InferenceReply};
use crate::loop_detect::{LoopDetector, LoopStats};
use crate::output_budget::{OutputBudgetStats, OutputBudgeter};
use crate::provider::{ModelCallerBackend, ProviderBackend};
use crate::thinking_cap::{ThinkingCapStats, ThinkingCapper};
use crate::tool_prune::{PruneStats, ToolPruner};
use crate::{
    GatewayError, GatewayResult, InferenceChunk, InferenceClient, InferenceMeta, InferenceRequest,
    InferenceResponse, ProviderFailureKind, Tier, TokenUsage,
};

const DEFAULT_CHANNEL_CAPACITY: usize = 200;
const DEFAULT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_FALLBACKS: usize = 2;

/// Stable pipeline order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    /// Retry/oscillation/drift guidance.
    LoopDetect,
    /// Exact then semantic cache.
    CacheLookup,
    /// Observation-gated tool filtering.
    ToolPrune,
    /// Learned maximum output.
    OutputBudget,
    /// Family-specific thinking default.
    ThinkingCap,
    /// Repetitive response guidance.
    ConvergenceDetect,
    /// Live provider and fallback dispatch.
    ProviderCall,
    /// Exact and semantic cache publication.
    CacheStore,
    /// Cost calculation, attribution, persistence, and budget deduction.
    CostTrack,
}

impl PipelineStage {
    /// Ordered list used for cache-route traces and structural assertions.
    pub const ALL: [Self; 9] = [
        Self::LoopDetect,
        Self::CacheLookup,
        Self::ToolPrune,
        Self::OutputBudget,
        Self::ThinkingCap,
        Self::ConvergenceDetect,
        Self::ProviderCall,
        Self::CacheStore,
        Self::CostTrack,
    ];
}

/// Runtime configuration and injected boundaries.
pub struct GatewayConfig {
    /// Shared learned model router.
    pub cascade_router: Arc<CascadeRouter>,
    /// Ordered registered provider backends.
    pub providers: Vec<Arc<dyn ProviderBackend>>,
    /// Canonical pricing table.
    pub cost_table: CostTable,
    /// Queue/concurrency limits.
    pub backpressure: BackpressureConfig,
    /// Provider attempt timeout.
    pub provider_timeout: Duration,
    /// Maximum fallback models after the primary.
    pub max_fallbacks: usize,
    /// Agent handle channel capacity.
    pub channel_capacity: usize,
    /// Optional durable event sink.
    pub event_writer: Option<Arc<GatewayEventWriter>>,
}

impl GatewayConfig {
    /// Construct with safe bounded defaults.
    #[must_use]
    pub fn new(
        cascade_router: Arc<CascadeRouter>,
        providers: Vec<Arc<dyn ProviderBackend>>,
        cost_table: CostTable,
    ) -> Self {
        Self {
            cascade_router,
            providers,
            cost_table,
            backpressure: BackpressureConfig::default(),
            provider_timeout: DEFAULT_PROVIDER_TIMEOUT,
            max_fallbacks: DEFAULT_MAX_FALLBACKS,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            event_writer: None,
        }
    }

    /// Adapt the existing live ModelCaller service into ordered built-in
    /// provider families plus a configured-provider catch-all.
    #[must_use]
    pub fn from_model_caller(
        cascade_router: Arc<CascadeRouter>,
        caller: Arc<dyn ModelCaller>,
        cost_table: CostTable,
    ) -> Self {
        let providers: Vec<Arc<dyn ProviderBackend>> = vec![
            Arc::new(ModelCallerBackend::anthropic(Arc::clone(&caller))),
            Arc::new(ModelCallerBackend::openai(Arc::clone(&caller))),
            Arc::new(ModelCallerBackend::catch_all("configured", caller)),
        ];
        Self::new(cascade_router, providers, cost_table)
    }

    /// Enable append-only durable event publication.
    #[must_use]
    pub fn with_event_writer(mut self, writer: Arc<GatewayEventWriter>) -> Self {
        self.event_writer = Some(writer);
        self
    }
}

/// Public gateway telemetry used by the HTTP stats adapter.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayStats {
    /// Requests entering the pipeline.
    pub total_requests: u64,
    /// Successful provider or cache responses.
    pub completed_requests: u64,
    /// Failed requests.
    pub failed_requests: u64,
    /// Responses served without a provider call.
    pub cache_hits: u64,
    /// Actual cost accumulated in USD.
    pub actual_cost_usd: f64,
    /// Avoided cost accumulated in USD.
    pub savings_usd: f64,
    /// Cache internals.
    pub cache: CacheStats,
    /// Loop guidance counters.
    pub loops: LoopStats,
    /// Convergence guidance counters.
    pub convergence: ConvergenceStats,
    /// Tool-pruning totals.
    pub tool_pruning: PruneStats,
    /// Output-budget totals.
    pub output_budget: OutputBudgetStats,
    /// Thinking-budget totals.
    pub thinking_cap: ThinkingCapStats,
    /// Three-level queue state.
    pub backpressure: BackpressureStats,
    /// Invocation count for every pipeline stage.
    pub stage_invocations: HashMap<PipelineStage, u64>,
}

struct GatewayCounters {
    total_requests: AtomicU64,
    completed_requests: AtomicU64,
    failed_requests: AtomicU64,
    cache_hits: AtomicU64,
    actual_cost_nanos: AtomicU64,
    savings_nanos: AtomicU64,
    stages: HashMap<PipelineStage, AtomicU64>,
}

#[derive(Clone, Copy)]
struct RequestExecutionContext<'a> {
    request_id: &'a str,
    session_id: &'a str,
    agent_id: &'a str,
    budget: &'a AtomicU64,
}

impl Default for GatewayCounters {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            completed_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            actual_cost_nanos: AtomicU64::new(0),
            savings_nanos: AtomicU64::new(0),
            stages: PipelineStage::ALL
                .into_iter()
                .map(|stage| (stage, AtomicU64::new(0)))
                .collect(),
        }
    }
}

impl GatewayCounters {
    fn stage(&self, stage: PipelineStage) {
        if let Some(counter) = self.stages.get(&stage) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_cost(&self, record: &CostRecord) {
        self.actual_cost_nanos
            .fetch_add(usd_to_nanos(record.cost.actual_cost), Ordering::Relaxed);
        if record.cost.savings > 0.0 {
            self.savings_nanos
                .fetch_add(usd_to_nanos(record.cost.savings), Ordering::Relaxed);
        }
    }
}

/// Live nine-stage inference gateway.
pub struct InferenceGateway {
    cascade_router: Arc<CascadeRouter>,
    providers: Vec<Arc<dyn ProviderBackend>>,
    loop_detector: LoopDetector,
    cache: InferenceCache,
    tool_pruner: ToolPruner,
    output_budgeter: OutputBudgeter,
    thinking_capper: ThinkingCapper,
    convergence_detector: ConvergenceDetector,
    cost_tracker: CostTracker,
    backpressure: BackpressureGuard,
    provider_timeout: Duration,
    max_fallbacks: usize,
    event_writer: Option<Arc<GatewayEventWriter>>,
    sender: mpsc::Sender<InferenceEnvelope>,
    receiver: Mutex<Option<mpsc::Receiver<InferenceEnvelope>>>,
    counters: GatewayCounters,
    traces: Mutex<HashMap<String, Vec<PipelineStage>>>,
}

impl InferenceGateway {
    /// Initialize all nine stages and the bounded handle channel.
    #[must_use]
    pub fn new(config: GatewayConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.channel_capacity.max(1));
        Self {
            cascade_router: config.cascade_router,
            providers: config.providers,
            loop_detector: LoopDetector::default(),
            cache: InferenceCache::default(),
            tool_pruner: ToolPruner::default(),
            output_budgeter: OutputBudgeter::default(),
            thinking_capper: ThinkingCapper::default(),
            convergence_detector: ConvergenceDetector::default(),
            cost_tracker: CostTracker::new(config.cost_table),
            backpressure: BackpressureGuard::new(config.backpressure),
            provider_timeout: config.provider_timeout,
            max_fallbacks: config.max_fallbacks,
            event_writer: config.event_writer,
            sender,
            receiver: Mutex::new(Some(receiver)),
            counters: GatewayCounters::default(),
            traces: Mutex::new(HashMap::new()),
        }
    }

    /// Create a key-isolated agent handle with an independent budget counter.
    #[must_use]
    pub fn create_handle(
        &self,
        agent_id: impl Into<String>,
        budget_microdollars: u64,
    ) -> InferenceHandle {
        InferenceHandle::new(self.sender.clone(), agent_id.into(), budget_microdollars)
    }

    /// Start the bounded gateway receiver exactly once.
    pub fn spawn_gateway_loop(self: &Arc<Self>) -> GatewayResult<tokio::task::JoinHandle<()>> {
        let mut receiver = self
            .receiver
            .lock()
            .map_err(|_| GatewayError::AlreadyStarted)?
            .take()
            .ok_or(GatewayError::AlreadyStarted)?;
        let gateway = Arc::clone(self);
        Ok(tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                let gateway = Arc::clone(&gateway);
                tokio::spawn(async move {
                    gateway.process(envelope).await;
                });
            }
        }))
    }

    /// Process an envelope and answer its selected response transport.
    pub async fn process(&self, envelope: InferenceEnvelope) {
        match envelope.reply {
            InferenceReply::Complete(respond_to) => {
                let result = self
                    .process_request(envelope.request, &envelope.agent_id, &envelope.budget)
                    .await;
                let _ = respond_to.send(result);
            }
            InferenceReply::Stream(respond_to) => {
                // The full completion pipeline still runs once; chunking happens
                // after controls/store/accounting so stream callers cannot bypass
                // any stage or budget deduction.
                match self
                    .process_request(envelope.request, &envelope.agent_id, &envelope.budget)
                    .await
                {
                    Ok(response) => {
                        if !response.text.is_empty() {
                            let _ = respond_to
                                .send(Ok(InferenceChunk {
                                    delta: response.text.clone(),
                                    model: response.model.clone(),
                                    ..InferenceChunk::default()
                                }))
                                .await;
                        }
                        let _ = respond_to
                            .send(Ok(InferenceChunk {
                                usage: Some(response.usage),
                                stop_reason: Some(response.stop_reason),
                                model: response.model,
                                done: true,
                                ..InferenceChunk::default()
                            }))
                            .await;
                    }
                    Err(error) => {
                        let _ = respond_to.send(Err(error)).await;
                    }
                }
            }
        }
    }

    /// Execute routing plus the complete nine-stage pipeline.
    pub async fn process_request(
        &self,
        mut request: InferenceRequest,
        agent_id: &str,
        budget: &AtomicU64,
    ) -> GatewayResult<InferenceResponse> {
        self.counters.total_requests.fetch_add(1, Ordering::Relaxed);
        request.metadata.agent_id = agent_id.to_string();
        request.metadata.budget_remaining = budget.load(Ordering::Relaxed);
        let session_id = request.metadata.session_id.clone();
        let request_id = uuid::Uuid::new_v4().to_string();
        let execution = RequestExecutionContext {
            request_id: &request_id,
            session_id: &session_id,
            agent_id,
            budget,
        };
        let mut trace = Vec::with_capacity(PipelineStage::ALL.len());
        let route = self.route(&request);
        let original_model = route
            .first()
            .cloned()
            .unwrap_or_else(|| request.model.clone());
        request.model.clone_from(&original_model);

        self.enter(PipelineStage::LoopDetect, &mut trace);
        if let Some(marker) = request.metadata.progress_marker.clone() {
            self.loop_detector.record_progress(&session_id, marker);
        }
        for tool_call in &request.metadata.tool_calls {
            self.loop_detector.record_call(
                &session_id,
                &tool_call.tool_name,
                tool_call.arguments_hash(),
                0,
            );
            self.tool_pruner
                .record_tool_use(&session_id, &tool_call.tool_name);
        }
        if let Some(guidance) = self.loop_detector.take_guidance(&session_id) {
            request.prepend_system_guidance(&guidance);
        }

        self.enter(PipelineStage::CacheLookup, &mut trace);
        if let Some(hit) = self.cache.lookup_with_layer(&request) {
            let response = hit.entry.response()?;
            self.counters.cache_hits.fetch_add(1, Ordering::Relaxed);
            // A cache route bypasses mutation/I/O but every stage remains
            // observable in the pipeline trace.
            for stage in PipelineStage::ALL.into_iter().skip(2) {
                self.enter(stage, &mut trace);
            }
            self.write_event(&request_id, &request, "cache", &response, 0.0, true, None)?;
            self.finish_trace(&session_id, trace);
            self.counters
                .completed_requests
                .fetch_add(1, Ordering::Relaxed);
            return Ok(response);
        }

        self.prepare_provider_request(&mut request, &session_id, &mut trace);

        if let Err(error) = self.preflight_budget(&request, budget) {
            self.counters
                .failed_requests
                .fetch_add(1, Ordering::Relaxed);
            self.finish_trace(&session_id, trace);
            return Err(error);
        }

        self.enter(PipelineStage::ProviderCall, &mut trace);
        let started = Instant::now();
        let result = self
            .call_with_fallbacks(&request, &route, agent_id, &original_model)
            .await;
        let (mut response, provider) = match result {
            Ok(response) => response,
            Err(error) => {
                self.counters
                    .failed_requests
                    .fetch_add(1, Ordering::Relaxed);
                let failure_response = InferenceResponse {
                    model: original_model,
                    ..InferenceResponse::default()
                };
                self.write_event(
                    &request_id,
                    &request,
                    "gateway",
                    &failure_response,
                    0.0,
                    false,
                    Some(error.to_string()),
                )?;
                self.finish_trace(&session_id, trace);
                return Err(error);
            }
        };
        response.latency_ms = started.elapsed().as_millis() as u64;
        self.finalize_provider_response(&request, response, &provider, execution, trace)
    }

    fn prepare_provider_request(
        &self,
        request: &mut InferenceRequest,
        session_id: &str,
        trace: &mut Vec<PipelineStage>,
    ) {
        self.enter(PipelineStage::ToolPrune, trace);
        if let Some(tools) = &request.tools {
            request.tools = Some(self.tool_pruner.prune(session_id, tools).0);
        }

        self.enter(PipelineStage::OutputBudget, trace);
        if let Some(cap) = self
            .output_budgeter
            .apply_budget(&request.model, request.max_tokens)
        {
            request.max_tokens = Some(cap);
        }

        self.enter(PipelineStage::ThinkingCap, trace);
        self.thinking_capper
            .apply(&request.model, &mut request.thinking);

        self.enter(PipelineStage::ConvergenceDetect, trace);
        if let Some(guidance) = self.convergence_detector.take_guidance(session_id) {
            request.prepend_system_guidance(&guidance);
        }
    }

    fn finalize_provider_response(
        &self,
        request: &InferenceRequest,
        response: InferenceResponse,
        provider: &str,
        execution: RequestExecutionContext<'_>,
        mut trace: Vec<PipelineStage>,
    ) -> GatewayResult<InferenceResponse> {
        self.enter(PipelineStage::CacheStore, &mut trace);
        let computed_cost = self.cost_tracker.compute_cost(
            &response.usage,
            &response.model,
            request.metadata.is_batch,
        );
        self.cache
            .store_with_cost(request, &response, computed_cost.actual_cost);

        self.enter(PipelineStage::CostTrack, &mut trace);
        let record = self.cost_tracker.record(
            &response.usage,
            &response.model,
            request.metadata.is_batch,
            execution.agent_id,
            execution.session_id,
        );
        self.counters.record_cost(&record);
        deduct_budget(
            execution.budget,
            usd_to_microdollars(record.cost.actual_cost),
        );
        self.output_budgeter
            .record_output(&response.model, response.usage.output_tokens);
        self.loop_detector
            .record_output(execution.session_id, response.usage.output_tokens);
        self.convergence_detector
            .record_response(execution.session_id, simhash(&response.text));
        self.write_event(
            execution.request_id,
            request,
            provider,
            &response,
            record.cost.actual_cost,
            false,
            None,
        )?;
        self.finish_trace(execution.session_id, trace);
        self.counters
            .completed_requests
            .fetch_add(1, Ordering::Relaxed);
        Ok(response)
    }

    /// Last pipeline trace for a session.
    #[must_use]
    pub fn last_trace(&self, session_id: &str) -> Vec<PipelineStage> {
        self.traces
            .lock()
            .ok()
            .and_then(|traces| traces.get(session_id).cloned())
            .unwrap_or_default()
    }

    /// Complete stats snapshot for an HTTP or telemetry adapter.
    #[must_use]
    pub fn stats(&self) -> GatewayStats {
        GatewayStats {
            total_requests: self.counters.total_requests.load(Ordering::Relaxed),
            completed_requests: self.counters.completed_requests.load(Ordering::Relaxed),
            failed_requests: self.counters.failed_requests.load(Ordering::Relaxed),
            cache_hits: self.counters.cache_hits.load(Ordering::Relaxed),
            actual_cost_usd: nanos_to_usd(self.counters.actual_cost_nanos.load(Ordering::Relaxed)),
            savings_usd: nanos_to_usd(self.counters.savings_nanos.load(Ordering::Relaxed)),
            cache: self.cache.stats(),
            loops: self.loop_detector.stats(),
            convergence: self.convergence_detector.stats(),
            tool_pruning: self.tool_pruner.totals(),
            output_budget: self.output_budgeter.stats(),
            thinking_cap: self.thinking_capper.stats(),
            backpressure: self.backpressure.stats(),
            stage_invocations: self
                .counters
                .stages
                .iter()
                .map(|(stage, counter)| (*stage, counter.load(Ordering::Relaxed)))
                .collect(),
        }
    }

    fn route(&self, request: &InferenceRequest) -> Vec<String> {
        let routed = self
            .cascade_router
            .route(&routing_context(&request.metadata));
        let explicit = !request.model.trim().is_empty() && request.model != "auto";
        let primary = if explicit {
            request.model.clone()
        } else {
            routed.primary.slug.clone()
        };
        let mut models = vec![primary.clone()];
        if routed.primary.slug != primary {
            models.push(routed.primary.slug);
        }
        models.extend(routed.fallback_chain.into_iter().map(|model| model.slug));
        let mut seen = std::collections::HashSet::new();
        models.retain(|model| seen.insert(model.clone()));
        models
    }

    async fn call_with_fallbacks(
        &self,
        request: &InferenceRequest,
        route: &[String],
        agent_id: &str,
        original_model: &str,
    ) -> GatewayResult<(InferenceResponse, String)> {
        let mut failures = Vec::new();
        let mut provider_attempt = 0_usize;
        for model in route {
            let Some(provider) = self
                .providers
                .iter()
                .find(|provider| provider.supports_model(model))
            else {
                failures.push(format!("{model}: no backend"));
                continue;
            };
            if provider_attempt > self.max_fallbacks {
                break;
            }
            let attempt = provider_attempt;
            provider_attempt = provider_attempt.saturating_add(1);
            let _permit = match self.backpressure.acquire(provider.name(), agent_id).await {
                Ok(permit) => permit,
                Err(error) => return Err(error.into()),
            };
            let mut attempt_request = request.clone();
            attempt_request.model.clone_from(model);
            let result =
                tokio::time::timeout(self.provider_timeout, provider.complete(&attempt_request))
                    .await
                    .map_err(|_| GatewayError::Provider {
                        provider: provider.name().to_string(),
                        kind: ProviderFailureKind::Timeout,
                        message: format!("model {model} exceeded provider timeout"),
                    })
                    .and_then(std::convert::identity);
            match result {
                Ok(mut response) => {
                    self.cascade_router.record_confidence_outcome(model, true);
                    response.model = model.clone();
                    response.fallback = attempt > 0;
                    response.original_model = (attempt > 0).then(|| original_model.to_string());
                    return Ok((response, provider.name().to_string()));
                }
                Err(error) => {
                    self.cascade_router.record_confidence_outcome(model, false);
                    let retryable = match &error {
                        GatewayError::Provider { kind, .. } => {
                            if *kind == ProviderFailureKind::RateLimited {
                                provider.rotate_key();
                            }
                            kind.is_retryable()
                        }
                        GatewayError::Backpressure(_) | GatewayError::NoProvider(_) => true,
                        _ => false,
                    };
                    failures.push(format!("{model}: {error}"));
                    if !retryable {
                        return Err(error);
                    }
                }
            }
        }
        Err(GatewayError::ProvidersExhausted(failures.join("; ")))
    }

    fn preflight_budget(
        &self,
        request: &InferenceRequest,
        budget: &AtomicU64,
    ) -> GatewayResult<()> {
        let remaining = budget.load(Ordering::Relaxed);
        let estimated_input = request.semantic_text().chars().count().div_ceil(4) as u64;
        let estimated = self.cost_tracker.compute_cost(
            &TokenUsage {
                input_tokens: estimated_input,
                output_tokens: u64::from(request.max_tokens.unwrap_or(2_048)),
                ..TokenUsage::default()
            },
            &request.model,
            request.metadata.is_batch,
        );
        if usd_to_microdollars(estimated.actual_cost) > remaining {
            return Err(GatewayError::BudgetExceeded {
                remaining_microdollars: remaining,
            });
        }
        Ok(())
    }

    fn enter(&self, stage: PipelineStage, trace: &mut Vec<PipelineStage>) {
        trace.push(stage);
        self.counters.stage(stage);
    }

    fn finish_trace(&self, session_id: &str, trace: Vec<PipelineStage>) {
        if let Ok(mut traces) = self.traces.lock() {
            traces.insert(session_id.to_string(), trace);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_event(
        &self,
        request_id: &str,
        request: &InferenceRequest,
        provider: &str,
        response: &InferenceResponse,
        cost_usd: f64,
        cache_hit: bool,
        error: Option<String>,
    ) -> GatewayResult<()> {
        let Some(writer) = &self.event_writer else {
            return Ok(());
        };
        writer.write(&GatewayEvent {
            request_id: request_id.to_string(),
            caller: format!(
                "{}:{}",
                request.metadata.agent_id, request.metadata.session_id
            ),
            model: response.model.clone(),
            provider: Some(provider.to_string()),
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            cost_usd,
            latency_ms: response.latency_ms,
            cache_hit,
            success: error.is_none(),
            error,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(())
    }
}

#[async_trait]
impl InferenceClient for InferenceGateway {
    async fn complete(&self, request: InferenceRequest) -> GatewayResult<InferenceResponse> {
        let budget = AtomicU64::new(request.metadata.budget_remaining);
        let agent_id = request.metadata.agent_id.clone();
        self.process_request(request, &agent_id, &budget).await
    }

    async fn stream(
        &self,
        request: InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>> {
        let response = self.complete(request).await?;
        Ok(Box::pin(stream::iter(vec![
            Ok(InferenceChunk {
                delta: response.text,
                model: response.model.clone(),
                ..InferenceChunk::default()
            }),
            Ok(InferenceChunk {
                usage: Some(response.usage),
                stop_reason: Some(response.stop_reason),
                model: response.model,
                done: true,
                ..InferenceChunk::default()
            }),
        ])))
    }
}

fn routing_context(metadata: &InferenceMeta) -> RoutingContext {
    RoutingContext {
        task_category: parse_task_category(metadata.task_category.as_deref()),
        complexity: parse_complexity(metadata.complexity.as_deref(), metadata.tier),
        iteration: metadata.iteration,
        role: parse_agent_role(metadata.agent_role.as_deref()),
        ..RoutingContext::default()
    }
}

fn parse_task_category(value: Option<&str>) -> TaskCategory {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "scaffolding" => TaskCategory::Scaffolding,
        "integration" => TaskCategory::Integration,
        "verification" | "test" | "testing" => TaskCategory::Verification,
        "research" => TaskCategory::Research,
        "refactor" => TaskCategory::Refactor,
        "infra" | "infrastructure" => TaskCategory::Infra,
        "docs" | "documentation" => TaskCategory::Docs,
        _ => TaskCategory::Implementation,
    }
}

fn parse_complexity(value: Option<&str>, tier: Tier) -> TaskComplexityBand {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "fast" => TaskComplexityBand::Fast,
        "complex" => TaskComplexityBand::Complex,
        "standard" => TaskComplexityBand::Standard,
        _ => match tier {
            Tier::T0 => TaskComplexityBand::Fast,
            Tier::T1 => TaskComplexityBand::Standard,
            Tier::T2 => TaskComplexityBand::Complex,
        },
    }
}

fn parse_agent_role(value: Option<&str>) -> AgentRole {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "strategist" => AgentRole::Strategist,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" | "reviewer" => AgentRole::Auditor,
        "scribe" => AgentRole::Scribe,
        "critic" => AgentRole::Critic,
        "refactorer" => AgentRole::Refactorer,
        "integration-tester" => AgentRole::IntegrationTester,
        _ => AgentRole::Implementer,
    }
}

fn deduct_budget(budget: &AtomicU64, cost_microdollars: u64) {
    let _ = budget.fetch_update(Ordering::AcqRel, Ordering::Relaxed, |remaining| {
        Some(remaining.saturating_sub(cost_microdollars))
    });
}

fn usd_to_microdollars(usd: f64) -> u64 {
    if !usd.is_finite() || usd <= 0.0 {
        0
    } else {
        (usd * 1_000_000.0).ceil().min(u64::MAX as f64) as u64
    }
}

fn usd_to_nanos(usd: f64) -> u64 {
    if !usd.is_finite() || usd <= 0.0 {
        0
    } else {
        (usd * 1_000_000_000.0).round().min(u64::MAX as f64) as u64
    }
}

fn nanos_to_usd(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    use futures::{StreamExt, stream};
    use roko_core::foundation::{
        ChatMessage, MessageRole, ModelCallRequest, ModelCallResponse, TokenUsage as CoreTokenUsage,
    };
    use roko_learn::cost_table::ModelPricing;

    use super::*;
    use crate::StopReason;

    struct ScriptedProvider {
        name: String,
        model: String,
        result: Mutex<Vec<Result<InferenceResponse, ProviderFailureKind>>>,
        calls: AtomicUsize,
        rotations: AtomicUsize,
    }

    impl ScriptedProvider {
        fn success(name: &str, model: &str, text: &str) -> Arc<Self> {
            Arc::new(Self {
                name: name.into(),
                model: model.into(),
                result: Mutex::new(vec![Ok(InferenceResponse {
                    text: text.into(),
                    stop_reason: StopReason::EndTurn,
                    usage: TokenUsage {
                        input_tokens: 100,
                        output_tokens: 10,
                        ..TokenUsage::default()
                    },
                    model: model.into(),
                    ..InferenceResponse::default()
                })]),
                calls: AtomicUsize::new(0),
                rotations: AtomicUsize::new(0),
            })
        }

        fn failure(name: &str, model: &str, kind: ProviderFailureKind) -> Arc<Self> {
            Arc::new(Self {
                name: name.into(),
                model: model.into(),
                result: Mutex::new(vec![Err(kind)]),
                calls: AtomicUsize::new(0),
                rotations: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl ProviderBackend for ScriptedProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn supports_model(&self, model: &str) -> bool {
            model == self.model
        }

        async fn complete(&self, _request: &InferenceRequest) -> GatewayResult<InferenceResponse> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.result.lock().unwrap().remove(0) {
                Ok(response) => Ok(response),
                Err(kind) => Err(GatewayError::Provider {
                    provider: self.name.clone(),
                    kind,
                    message: "scripted failure".into(),
                }),
            }
        }

        async fn stream(
            &self,
            request: &InferenceRequest,
        ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>> {
            let response = self.complete(request).await?;
            Ok(Box::pin(stream::iter(vec![Ok(InferenceChunk {
                delta: response.text,
                model: response.model,
                done: true,
                ..InferenceChunk::default()
            })])))
        }

        fn rotate_key(&self) {
            self.rotations.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn cost_table() -> CostTable {
        CostTable {
            models: HashMap::from([
                (
                    "primary".into(),
                    ModelPricing {
                        input_per_m: 1.0,
                        output_per_m: 2.0,
                        cache_read_per_m: 0.1,
                        cache_write_per_m: 1.25,
                        tokenizer_ratio: 1.0,
                    },
                ),
                (
                    "fallback".into(),
                    ModelPricing {
                        input_per_m: 1.0,
                        output_per_m: 2.0,
                        cache_read_per_m: 0.1,
                        cache_write_per_m: 1.25,
                        tokenizer_ratio: 1.0,
                    },
                ),
            ]),
        }
    }

    fn request(model: &str, session: &str) -> InferenceRequest {
        InferenceRequest {
            model: model.into(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "implement the gateway".into(),
            }],
            metadata: InferenceMeta {
                session_id: session.into(),
                agent_id: "spoofed".into(),
                budget_remaining: 1_000_000,
                ..InferenceMeta::default()
            },
            ..InferenceRequest::default()
        }
    }

    #[tokio::test]
    async fn gateway_pipeline_executes_nine_stages_and_cache_bypasses_provider() {
        let provider = ScriptedProvider::success("p", "primary", "provider response");
        let config = GatewayConfig::new(
            Arc::new(CascadeRouter::new(vec!["primary".into()])),
            vec![provider.clone()],
            cost_table(),
        );
        let gateway = Arc::new(InferenceGateway::new(config));
        let loop_task = gateway.spawn_gateway_loop().unwrap();
        let handle = gateway.create_handle("agent", 1_000_000);

        let first = handle.infer(request("primary", "s")).await.unwrap();
        assert_eq!(first.text, "provider response");
        assert_eq!(gateway.last_trace("s"), PipelineStage::ALL);
        let second = handle.infer(request("primary", "s")).await.unwrap();
        assert_eq!(second.text, "provider response");
        assert_eq!(provider.calls.load(Ordering::Relaxed), 1);
        assert_eq!(gateway.stats().cache_hits, 1);
        assert!(handle.remaining_budget() < 1_000_000);

        let chunks = handle
            .infer_stream(request("primary", "s"))
            .await
            .unwrap()
            .collect::<Vec<_>>()
            .await;
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].as_ref().unwrap().delta, "provider response");
        assert!(chunks[1].as_ref().unwrap().done);
        loop_task.abort();
    }

    #[tokio::test]
    async fn gateway_falls_through_rate_limit_rotates_and_marks_metadata() {
        let primary = ScriptedProvider::failure(
            "primary-provider",
            "primary",
            ProviderFailureKind::RateLimited,
        );
        let fallback = ScriptedProvider::success("fallback-provider", "fallback", "fallback ok");
        let router = Arc::new(CascadeRouter::new(vec![
            "primary".into(),
            "fallback".into(),
        ]));
        let mut config = GatewayConfig::new(
            router,
            vec![primary.clone(), fallback.clone()],
            cost_table(),
        );
        config.max_fallbacks = 2;
        let gateway = InferenceGateway::new(config);
        let budget = AtomicU64::new(1_000_000);
        let response = gateway
            .process_request(request("primary", "fallback-session"), "agent", &budget)
            .await
            .unwrap();
        assert_eq!(response.model, "fallback");
        assert!(response.fallback);
        assert_eq!(response.original_model.as_deref(), Some("primary"));
        assert_eq!(primary.rotations.load(Ordering::Relaxed), 1);
        assert_eq!(fallback.calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn gateway_all_fallbacks_exhausted_returns_bounded_error() {
        let primary = ScriptedProvider::failure(
            "primary-provider",
            "primary",
            ProviderFailureKind::Unavailable,
        );
        let fallback = ScriptedProvider::failure(
            "fallback-provider",
            "fallback",
            ProviderFailureKind::Timeout,
        );
        let config = GatewayConfig::new(
            Arc::new(CascadeRouter::new(vec![
                "primary".into(),
                "fallback".into(),
            ])),
            vec![primary, fallback],
            cost_table(),
        );
        let gateway = InferenceGateway::new(config);
        let budget = AtomicU64::new(1_000_000);
        assert!(matches!(
            gateway
                .process_request(request("primary", "failed"), "agent", &budget)
                .await,
            Err(GatewayError::ProvidersExhausted(_))
        ));
        assert_eq!(gateway.stats().failed_requests, 1);
    }

    #[tokio::test]
    async fn gateway_reaches_second_fallback_after_two_retryable_failures() {
        let primary = ScriptedProvider::failure(
            "primary-provider",
            "primary",
            ProviderFailureKind::Unavailable,
        );
        let fallback_one = ScriptedProvider::failure(
            "fallback-one-provider",
            "fallback-one",
            ProviderFailureKind::Timeout,
        );
        let fallback_two =
            ScriptedProvider::success("fallback-two-provider", "fallback-two", "third works");
        let config = GatewayConfig::new(
            Arc::new(CascadeRouter::new(vec![
                "primary".into(),
                "fallback-one".into(),
                "fallback-two".into(),
            ])),
            vec![primary, fallback_one, fallback_two.clone()],
            cost_table(),
        );
        let gateway = InferenceGateway::new(config);
        let response = gateway
            .process_request(
                request("primary", "second-fallback"),
                "agent",
                &AtomicU64::new(1_000_000),
            )
            .await
            .unwrap();
        assert_eq!(response.model, "fallback-two");
        assert_eq!(response.original_model.as_deref(), Some("primary"));
        assert_eq!(fallback_two.calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn gateway_budget_preflight_rejects_before_provider() {
        let provider = ScriptedProvider::success("p", "primary", "provider response");
        let gateway = InferenceGateway::new(GatewayConfig::new(
            Arc::new(CascadeRouter::new(vec!["primary".into()])),
            vec![provider.clone()],
            cost_table(),
        ));
        let budget = AtomicU64::new(0);
        assert!(matches!(
            gateway
                .process_request(request("primary", "budget"), "agent", &budget)
                .await,
            Err(GatewayError::BudgetExceeded { .. })
        ));
        assert_eq!(provider.calls.load(Ordering::Relaxed), 0);
        assert_eq!(gateway.stats().failed_requests, 1);
    }

    #[derive(Default)]
    struct LiveCallerBoundary {
        calls: AtomicUsize,
        last_request: Mutex<Option<ModelCallRequest>>,
    }

    #[async_trait]
    impl ModelCaller for LiveCallerBoundary {
        async fn call(&self, request: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let model = request.model.clone();
            *self.last_request.lock().unwrap() = Some(request);
            Ok(ModelCallResponse {
                content: "live boundary response".into(),
                model,
                usage: CoreTokenUsage {
                    input_tokens: 40,
                    output_tokens: 12,
                    total_tokens: 52,
                    cost_usd: 0.0,
                },
                stop_reason: Some("end_turn".into()),
                request_id: Some("live-request".into()),
            })
        }
    }

    #[tokio::test]
    async fn gateway_from_model_caller_uses_existing_live_dispatch_boundary() {
        let caller = Arc::new(LiveCallerBoundary::default());
        let live: Arc<dyn ModelCaller> = caller.clone();
        let gateway = InferenceGateway::new(GatewayConfig::from_model_caller(
            Arc::new(CascadeRouter::new(vec!["claude-live".into()])),
            live,
            CostTable {
                models: HashMap::new(),
            },
        ));
        let response = gateway
            .process_request(
                request("claude-live", "live-boundary"),
                "authoritative-agent",
                &AtomicU64::new(1_000_000),
            )
            .await
            .unwrap();
        assert_eq!(response.text, "live boundary response");
        assert_eq!(caller.calls.load(Ordering::Relaxed), 1);
        let observed = caller.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(observed.model, "claude-live");
        assert_eq!(observed.caller.as_deref(), Some("authoritative-agent"));
        assert_eq!(observed.run_id.as_deref(), Some("live-boundary"));
    }

    #[tokio::test]
    async fn gateway_persists_attributed_cost_event_after_provider_success() {
        let directory = tempfile::tempdir().unwrap();
        let writer = Arc::new(GatewayEventWriter::new(
            directory.path().join("gateway.jsonl"),
        ));
        let config = GatewayConfig::new(
            Arc::new(CascadeRouter::new(vec!["primary".into()])),
            vec![ScriptedProvider::success("p", "primary", "persisted")],
            cost_table(),
        )
        .with_event_writer(Arc::clone(&writer));
        let gateway = InferenceGateway::new(config);
        gateway
            .process_request(
                request("primary", "durable-session"),
                "durable-agent",
                &AtomicU64::new(1_000_000),
            )
            .await
            .unwrap();

        let projection = writer.projection().unwrap();
        assert_eq!(projection.total_events(), 1);
        assert!(projection.total_cost_usd() > 0.0);
        let callers = projection.stats_by_caller();
        let caller = callers.get("durable-agent:durable-session").unwrap();
        assert_eq!(caller.count, 1);
    }
}
