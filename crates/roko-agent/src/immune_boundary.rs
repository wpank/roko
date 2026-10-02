//! Effectful immune screening for provider final outputs.
//!
//! This module owns one deliberately narrow automatic trust boundary: the
//! primary [`AgentResult`] returned by agents constructed through the canonical
//! provider factory. It does not claim visibility into provider-owned internal
//! tool calls or results that were never surfaced as Signals.
//!
//! Every returned result is scored only from host-observable structural facts,
//! then passed through the fixed five-stage `roko-graph` immune Graph. Suspicious
//! output is withheld from the caller and persisted in a dedicated file-backed
//! quarantine Store. High and Critical findings also create a deterministic
//! isolation control record checked before subsequent provider execution.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use roko_core::{
    AnomalyScore, Body, ContentHash, Context, ImmunePipeline, ImmunePipelineResult,
    IncidentRelation, Kind, Provenance, QuarantineDecision, Signal, Store, ThreatSeverity,
    error::Result,
};
use roko_graph::NodeStatus;
use roko_graph::cells::ImmunePipelineGraph;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentResult};
use crate::dispatcher::truncate::{bounded_json_bytes, bounded_serialized_bytes};
use crate::immune_evidence::{
    AGENT_ISOLATION_CONTROL_KIND as AGENT_ISOLATION_CONTROL_KIND_VALUE, get_agent_control,
    persist_agent_control, persist_evidence_signals, validate_boundary_label,
};
use crate::live_output::{LiveAgentEvent, LiveOutput, tool_step_target};
use crate::tool_immune::update_vault;
use crate::tool_loop::{StreamEvent, StreamEventKind};

/// Relative workspace directory containing the quarantine Store.
pub const QUARANTINE_STORE_RELATIVE_PATH: &str = ".roko/immune/quarantine";
/// Signal kind for durable provider-boundary containment evidence.
pub const PROVIDER_BOUNDARY_RECORD_KIND: &str = "roko.security.immune.provider_boundary_record";
/// Signal kind for durable agent-isolation control state.
pub const AGENT_ISOLATION_CONTROL_KIND: &str = AGENT_ISOLATION_CONTROL_KIND_VALUE;

const IMMUNE_STAGE_ORDER: [&str; 5] = [
    "immune-perception",
    "immune-assessment",
    "immune-containment",
    "immune-validation",
    "immune-escalation",
];
const MAX_PROVIDER_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROVIDER_SECURITY_METADATA_BYTES: usize = 64 * 1024;
const MAX_PROVIDER_STREAM_CHUNKS: usize = 4_096;
const MAX_PROVIDER_STREAM_BYTES: usize = 4 * 1024 * 1024;

/// Resolve the dedicated quarantine Store beneath a workspace root.
#[must_use]
pub fn quarantine_store_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(QUARANTINE_STORE_RELATIVE_PATH)
}

/// Effects this boundary will attempt after the decision Graph completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderBoundaryEffect {
    /// The suspect result is not returned to its caller.
    DeliveryDenied,
    /// The exact suspect output is retained only in the quarantine Store.
    QuarantineEvidencePersisted,
    /// The output hash is indexed in the strict review vault.
    QuarantineVaultIndexed,
    /// A durable pre-dispatch isolation marker is created for this agent ID.
    AgentIsolation,
}

/// Durable evidence for one automatic provider final-output screening.
///
/// No timestamp participates in this record, so the content hash is stable for
/// an identical agent/input/output decision and Store writes are idempotent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderBoundaryRecord {
    /// Record schema version.
    pub schema_version: u32,
    /// Stable agent identity used for durable isolation lookup.
    pub agent_id: String,
    /// Prompt or task Signal passed to the provider.
    pub input: ContentHash,
    /// Exact provider output retained in the quarantine Store.
    pub output: ContentHash,
    /// Digest of the complete persisted output provenance. The Signal's
    /// content hash intentionally omits trust and IFC metadata, so the
    /// boundary receipt binds those security-relevant fields separately.
    pub output_security_metadata: ContentHash,
    /// Host-observable anomaly evidence supplied to stage 1.
    pub anomaly: AnomalyScore,
    /// Complete five-stage decision.
    pub decision: ImmunePipelineResult,
    /// Completed Graph nodes in execution order.
    pub stage_order: Vec<String>,
    /// Effects completed before this receipt was published. This is
    /// historical audit evidence, not a guarantee that a bounded vault still
    /// retains the entry at replay time.
    pub effects: Vec<ProviderBoundaryEffect>,
    /// Deterministic isolation marker, when isolation is required.
    pub isolation_control: Option<ContentHash>,
}

/// Durable agent-control state checked before a provider process or request is
/// started. Its identity is deterministic for one `agent_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIsolationControl {
    /// Record schema version.
    pub schema_version: u32,
    /// Agent denied at the provider boundary.
    pub agent_id: String,
    /// Stable state label.
    pub state: String,
    /// Stable reason code, intentionally excluding suspect provider text.
    pub reason: String,
}

/// Compute anomaly evidence solely from facts visible in an [`AgentResult`].
///
/// This detector does not infer truthfulness, hallucination, intent, or the
/// behavior of opaque provider-owned tools. Dimension names state the exact
/// observed invariant. The maximum observed dimension is the overall score.
#[must_use]
pub fn detect_provider_output_anomaly(input: &Signal, result: &AgentResult) -> AnomalyScore {
    detect_provider_output_evidence_anomaly(input.id, &result.output)
}

fn detect_provider_output_evidence_anomaly(input: ContentHash, output: &Signal) -> AnomalyScore {
    let mut anomaly = AnomalyScore::clean();

    let mut observe = |dimension: &str, score: f64| {
        anomaly.score = anomaly.score.max(score);
        anomaly
            .dimensions
            .insert(dimension.to_string(), score.clamp(0.0, 1.0));
    };

    if output.kind != Kind::AgentOutput {
        observe("unexpected_primary_output_kind", 0.95);
    }
    match &output.body {
        Body::Empty => observe("empty_primary_output_body", 0.9),
        Body::Text(text) if text.trim().is_empty() => {
            observe("blank_primary_output_text", 0.9);
        }
        Body::Bytes(bytes) if bytes.is_empty() => observe("empty_primary_output_bytes", 0.9),
        Body::Text(_) | Body::Json(_) | Body::Bytes(_) => {}
    }
    if !provider_body_within_limit(&output.body) {
        observe("oversized_primary_output", 0.9);
    }
    if !output.lineage.contains(&input) {
        // Several established provider adapters predate direct lineage and
        // construct otherwise valid AgentOutput Signals from scratch. Keep
        // that compatibility gap visible without treating it as grounds to
        // contain an output on its own.
        observe("missing_direct_input_lineage", 0.1);
    }
    if output.id != output.content_hash() {
        observe("content_hash_mismatch", 1.0);
    }
    if output
        .attestation
        .as_ref()
        .is_some_and(|attestation| !roko_core::attestation::verify(output, attestation))
    {
        observe("invalid_output_attestation", 1.0);
    }

    anomaly
}

enum BoundaryStore {
    Durable { workspace_root: PathBuf },
    Injected(Arc<dyn Store>),
}

impl BoundaryStore {
    fn durable(workspace_root: PathBuf) -> Self {
        Self::Durable { workspace_root }
    }

    async fn get_isolation(
        &self,
        agent_id: &str,
        marker_id: &ContentHash,
    ) -> Result<Option<Signal>> {
        match self {
            Self::Durable { workspace_root } => get_agent_control(workspace_root, agent_id)
                .map_err(|error| roko_core::RokoError::Store(error.to_string())),
            Self::Injected(store) => store.get(marker_id).await,
        }
    }

    async fn put_isolation(&self, signal: &Signal) -> Result<()> {
        match self {
            Self::Durable { workspace_root } => persist_agent_control(workspace_root, signal)
                .map_err(|error| roko_core::RokoError::Store(error.to_string())),
            Self::Injected(store) => store.put(signal.clone()).await.map(|_| ()),
        }
    }

    async fn put_all(&self, signals: &[Signal]) -> Result<()> {
        match self {
            Self::Durable { workspace_root } => persist_evidence_signals(workspace_root, signals)
                .map_err(|error| roko_core::RokoError::Store(error.to_string())),
            Self::Injected(store) => {
                for signal in signals {
                    store.put(signal.clone()).await?;
                }
                Ok(())
            }
        }
    }

    fn vault_path(&self) -> Option<PathBuf> {
        match self {
            Self::Durable { workspace_root } => {
                Some(workspace_root.join(crate::tool_immune::QUARANTINE_VAULT_RELATIVE_PATH))
            }
            Self::Injected(_) => None,
        }
    }
}

/// Agent wrapper installed automatically by the canonical provider factory.
///
/// Clean results are returned unchanged. Any non-Accept immune decision is
/// durably quarantined and replaced with a generic denied result. Store or
/// Graph errors also deny the output, keeping the boundary fail closed.
pub struct ImmuneScreenedAgent {
    inner: Box<dyn Agent>,
    /// Valid identities are retained verbatim; invalid identities are replaced
    /// at construction with a fixed-format digest that is safe for every
    /// subsequent log, tag, and control lookup.
    agent_id: String,
    agent_id_valid: bool,
    store: BoundaryStore,
    pipeline: ImmunePipelineGraph,
    live_output: Option<LiveOutput>,
    /// Directory live tool-step targets are shown relative to.
    tool_step_root: Option<PathBuf>,
}

/// Validate a provider agent identity without reflecting its contents.
///
/// This is public so orchestration boundaries can reject malformed dispatch
/// requests before resolving providers or producing runtime events.
pub fn validate_provider_agent_id(agent_id: &str) -> std::result::Result<(), &'static str> {
    validate_boundary_label(agent_id, "agent ID").map_err(|_| "invalid provider agent identity")?;
    if crate::safety::scrub::scrub_secrets(agent_id, &crate::safety::scrub::ScrubPolicy::default())
        != agent_id
    {
        return Err("invalid provider agent identity");
    }
    Ok(())
}

pub(crate) fn safe_provider_agent_identity(agent_id: &str) -> (String, bool) {
    if validate_provider_agent_id(agent_id).is_ok() {
        (agent_id.to_string(), true)
    } else {
        (
            format!("invalid-agent-{}", ContentHash::of(agent_id.as_bytes())),
            false,
        )
    }
}

impl ImmuneScreenedAgent {
    /// Construct a production boundary rooted at the supplied workspace.
    #[must_use]
    pub fn durable(
        inner: Box<dyn Agent>,
        agent_id: impl Into<String>,
        workspace_root: impl AsRef<Path>,
    ) -> Self {
        let requested_agent_id = agent_id.into();
        let (agent_id, agent_id_valid) = safe_provider_agent_identity(&requested_agent_id);
        let workspace_root = workspace_root.as_ref();
        let workspace_root = workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf());
        Self {
            inner,
            agent_id,
            agent_id_valid,
            store: BoundaryStore::durable(workspace_root),
            pipeline: ImmunePipelineGraph::default(),
            live_output: None,
            tool_step_root: None,
        }
    }

    /// Construct with an injected Store, primarily for embedded runtimes and
    /// deterministic failure testing.
    #[must_use]
    pub fn with_store(
        inner: Box<dyn Agent>,
        agent_id: impl Into<String>,
        store: Arc<dyn Store>,
    ) -> Self {
        let requested_agent_id = agent_id.into();
        let (agent_id, agent_id_valid) = safe_provider_agent_identity(&requested_agent_id);
        Self {
            inner,
            agent_id,
            agent_id_valid,
            store: BoundaryStore::Injected(store),
            pipeline: ImmunePipelineGraph::default(),
            live_output: None,
            tool_step_root: None,
        }
    }

    async fn isolation_marker(&self) -> Result<Signal> {
        isolation_marker_for(&self.agent_id)
    }

    async fn is_isolated(&self) -> Result<Option<ContentHash>> {
        let marker = self.isolation_marker().await?;
        self.store
            .get_isolation(&self.agent_id, &marker.id)
            .await
            .map(|stored| stored.map(|_| marker.id))
    }

    fn denied_result(
        &self,
        input: &Signal,
        original: Option<&AgentResult>,
        reason_code: &str,
        record: Option<ContentHash>,
    ) -> AgentResult {
        let mut output = input
            .derive(
                Kind::AgentOutput,
                Body::text("provider result denied by immune boundary"),
            )
            .provenance(Provenance::trusted("immune-provider-boundary"))
            .tag("immune_denied", "true")
            .tag("immune_reason", reason_code)
            .tag("agent_id", &self.agent_id);
        if let Some(record) = record {
            output = output.tag("immune_record", record.to_string());
        }
        let output = output.build();

        match original {
            None => AgentResult::fail(output),
            Some(original) => AgentResult {
                output,
                // Suspect trace content is withheld with the primary output.
                trace: Vec::new(),
                usage: original.usage,
                usage_obs: original.usage_obs.clone(),
                success: false,
                ttft_ms: original.ttft_ms,
            },
        }
    }

    /// Attach a live output channel to this boundary. When set and the inner
    /// agent supports streaming, `run` and `run_streaming` tap the provider
    /// event stream and forward qualifying events before the final result is
    /// screened.
    #[must_use]
    pub fn with_live_output(mut self, live_output: LiveOutput) -> Self {
        self.live_output = Some(live_output);
        self
    }

    /// Show live tool-step targets inside `root`, the directory the provider
    /// runs in, relative to it (see [`tool_step_target`]).
    #[must_use]
    pub fn with_tool_step_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.tool_step_root = Some(root.into());
        self
    }

    /// Shared inner loop: buffers provider stream events, counts against the
    /// limits, and forwards to the live output channel when one is present.
    /// The first model output on the stream sets the result's
    /// [`AgentResult::ttft_ms`], unless the provider measured its own.
    ///
    /// Returns `(AgentResult, stream_limit_exceeded)`.
    async fn drive_streaming_inner(&self, input: &Signal, ctx: &Context) -> (AgentResult, bool) {
        let (buffer_tx, mut buffer_rx) = mpsc::channel::<StreamEvent>(1);
        let live_sink = self
            .live_output
            .as_ref()
            .map(|lo| (lo.sink.clone(), lo.trusted));
        let tool_step_root = self.tool_step_root.as_deref();
        let started = Instant::now();
        let collect = async move {
            let mut chunk_count = 0_usize;
            let mut byte_count = 0_usize;
            let mut exceeded = false;
            let mut first_output = None;
            while let Some(event) = buffer_rx.recv().await {
                if first_output.is_none() && is_model_output(&event.kind) {
                    first_output = Some(started.elapsed());
                }
                chunk_count = chunk_count.saturating_add(1);
                byte_count = byte_count.saturating_add(stream_event_bytes(&event));
                exceeded |= chunk_count > MAX_PROVIDER_STREAM_CHUNKS
                    || byte_count > MAX_PROVIDER_STREAM_BYTES;
                if let Some((ref sink, trusted)) = live_sink {
                    match &event.kind {
                        StreamEventKind::ToolCallEnd { id, name, args } => {
                            let target = tool_step_target(name, args, tool_step_root);
                            let _ = sink.try_send(LiveAgentEvent::ToolStep {
                                id: id.clone(),
                                name: name.clone(),
                                target,
                            });
                            if trusted {
                                let _ =
                                    sink.try_send(LiveAgentEvent::Unscreened(event.kind.clone()));
                            }
                        }
                        StreamEventKind::TextDelta(_)
                        | StreamEventKind::ReasoningDelta(_)
                        | StreamEventKind::ToolResult { .. }
                            if trusted =>
                        {
                            let _ = sink.try_send(LiveAgentEvent::Unscreened(event.kind.clone()));
                        }
                        // Token counts and a model call's end carry no
                        // content: the stall watchdog records what a call it
                        // cancels used from them (bug-aa2044).
                        StreamEventKind::Usage(_) | StreamEventKind::Done { .. } => {
                            let _ = sink.try_send(LiveAgentEvent::Unscreened(event.kind.clone()));
                        }
                        _ => {}
                    }
                }
            }
            (exceeded, first_output)
        };
        let (mut result, (exceeded, first_output)) =
            tokio::join!(self.inner.run_streaming(input, ctx, buffer_tx), collect);
        if result.ttft_ms.is_none() {
            result.ttft_ms =
                first_output.map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX));
        }
        (result, exceeded)
    }

    async fn preflight(&self, input: &Signal) -> Option<AgentResult> {
        if !self.agent_id_valid {
            return Some(self.denied_result(input, None, "invalid_agent_identity", None));
        }
        match self.is_isolated().await {
            Ok(Some(marker)) => {
                Some(self.denied_result(input, None, "agent_isolated", Some(marker)))
            }
            Ok(None) => None,
            Err(error) => {
                tracing::error!(
                    agent_id = %self.agent_id,
                    error = %error,
                    "immune isolation state unavailable; denying provider dispatch"
                );
                Some(self.denied_result(input, None, "isolation_state_unavailable", None))
            }
        }
    }

    async fn persist_containment(
        &self,
        input: &Signal,
        original: &AgentResult,
        anomaly: AnomalyScore,
        decision: ImmunePipelineResult,
        stage_order: Vec<String>,
    ) -> Result<ContentHash> {
        let vault_path = self.store.vault_path();
        let severity = decision.validation.containment.assessment.severity;
        let requires_isolation =
            matches!(severity, ThreatSeverity::High | ThreatSeverity::Critical);
        let isolation = if requires_isolation {
            Some(self.isolation_marker().await?)
        } else {
            None
        };
        if let Some(isolation) = &isolation {
            // Commit enforcement before any fallible evidence/index work.
            self.store.put_isolation(isolation).await?;
        }
        let mut effects = vec![
            ProviderBoundaryEffect::DeliveryDenied,
            ProviderBoundaryEffect::QuarantineEvidencePersisted,
        ];
        if vault_path.is_some() {
            effects.push(ProviderBoundaryEffect::QuarantineVaultIndexed);
        }
        if isolation.is_some() {
            effects.push(ProviderBoundaryEffect::AgentIsolation);
        }

        let record = ProviderBoundaryRecord {
            schema_version: 1,
            agent_id: self.agent_id.clone(),
            input: input.id,
            output: original.output.id,
            output_security_metadata: provider_output_security_metadata_hash(&original.output)
                .map_err(roko_core::RokoError::Store)?,
            anomaly: anomaly.clone(),
            decision: decision.clone(),
            stage_order,
            effects,
            isolation_control: isolation.as_ref().map(|signal| signal.id),
        };
        let record_signal =
            Signal::builder(Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()))
                .body(Body::from_json(&record)?)
                .provenance(Provenance::trusted("immune-provider-boundary"))
                .lineage([input.id, original.output.id])
                .tag("agent_id", &self.agent_id)
                .tag(
                    "quarantine_decision",
                    match record.decision.validation.containment.decision {
                        QuarantineDecision::Accept => "accept",
                        QuarantineDecision::Quarantine => "quarantine",
                        QuarantineDecision::Reject => "reject",
                    },
                )
                .tag("boundary", "provider_final_output")
                .build();
        validate_provider_boundary_receipt(
            &record_signal,
            Some(&original.output),
            vault_path.is_some(),
        )
        .map_err(roko_core::RokoError::Store)?;

        // A receipt is published only after every effect it claims succeeds.
        self.store
            .put_all(std::slice::from_ref(&original.output))
            .await?;
        if let Some(vault_path) = vault_path {
            update_vault(
                &vault_path,
                original.output.id,
                anomaly,
                decision.escalation_required,
                &self.agent_id,
                IncidentRelation::SameSession,
            )
            .map_err(|error| {
                roko_core::RokoError::Store(format!(
                    "provider quarantine vault update failed: {error}"
                ))
            })?;
        }
        self.store
            .put_all(std::slice::from_ref(&record_signal))
            .await?;
        Ok(record_signal.id)
    }

    async fn screen_result(&self, input: &Signal, result: AgentResult) -> AgentResult {
        let anomaly = detect_provider_output_anomaly(input, &result);
        let graph = match self
            .pipeline
            .screen(result.output.id, anomaly.clone(), Vec::new())
            .await
        {
            Ok(graph) => graph,
            Err(error) => {
                tracing::error!(
                    agent_id = %self.agent_id,
                    error = %error,
                    "provider output immune Graph failed; denying result"
                );
                return self.denied_result(input, Some(&result), "immune_graph_failed", None);
            }
        };
        let stage_order = graph
            .graph
            .node_results
            .iter()
            .filter(|node| node.status == NodeStatus::Complete)
            .map(|node| node.node_id.clone())
            .collect::<Vec<_>>();
        if stage_order != IMMUNE_STAGE_ORDER.map(str::to_string) {
            tracing::error!(
                agent_id = %self.agent_id,
                ?stage_order,
                "provider output immune Graph violated fixed stage order; denying result"
            );
            return self.denied_result(input, Some(&result), "immune_stage_order_invalid", None);
        }

        if graph.result.validation.containment.decision == QuarantineDecision::Accept {
            return result;
        }

        match self
            .persist_containment(input, &result, anomaly, graph.result, stage_order)
            .await
        {
            Ok(record) => self.denied_result(
                input,
                Some(&result),
                "provider_output_quarantined",
                Some(record),
            ),
            Err(error) => {
                tracing::error!(
                    agent_id = %self.agent_id,
                    error = %error,
                    "provider output containment persistence failed; denying result"
                );
                self.denied_result(input, Some(&result), "containment_persistence_failed", None)
            }
        }
    }
}

fn isolation_marker_for(agent_id: &str) -> Result<Signal> {
    validate_boundary_label(agent_id, "agent ID")
        .map_err(|error| roko_core::RokoError::Store(error.to_string()))?;
    let control = AgentIsolationControl {
        schema_version: 1,
        agent_id: agent_id.to_string(),
        state: "isolated".to_string(),
        reason: "provider_output_immune_containment".to_string(),
    };
    Ok(
        Signal::builder(Kind::Custom(AGENT_ISOLATION_CONTROL_KIND.to_string()))
            .body(Body::from_json(&control)?)
            .provenance(Provenance::trusted("immune-provider-boundary"))
            .tag("agent_id", agent_id)
            .tag("control_state", "isolated")
            .build(),
    )
}

pub(crate) fn validate_provider_boundary_receipt(
    signal: &Signal,
    evidence: Option<&Signal>,
    vault_expected: bool,
) -> std::result::Result<ProviderBoundaryRecord, String> {
    // This proves internal ledger consistency and verifies any output
    // attestation. Authenticating a wholesale, self-consistent ledger rewrite
    // still requires an external anchor or independently held signing key.
    if signal.id != signal.content_hash()
        || !signal.is(&Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()))
    {
        return Err("provider boundary receipt has an invalid identity".to_string());
    }
    let record: ProviderBoundaryRecord = decode_exact_json_body(&signal.body)?;
    if record.schema_version != 1
        || signal.attestation.is_some()
        || signal.provenance != Provenance::trusted("immune-provider-boundary")
        || signal.tag("agent_id") != Some(record.agent_id.as_str())
        || signal.tag("boundary") != Some("provider_final_output")
        || signal.tag("quarantine_decision")
            != Some(match record.decision.validation.containment.decision {
                QuarantineDecision::Accept => "accept",
                QuarantineDecision::Quarantine => "quarantine",
                QuarantineDecision::Reject => "reject",
            })
        || signal.tags.len() != 3
        || signal.lineage != vec![record.input, record.output]
        || record.stage_order != IMMUNE_STAGE_ORDER.map(str::to_string)
    {
        return Err("provider boundary receipt metadata is inconsistent".to_string());
    }
    validate_boundary_label(&record.agent_id, "agent ID").map_err(|error| error.to_string())?;
    let isolation_expected = matches!(
        record.decision.validation.containment.assessment.severity,
        ThreatSeverity::High | ThreatSeverity::Critical
    );
    let mut expected_effects = vec![
        ProviderBoundaryEffect::DeliveryDenied,
        ProviderBoundaryEffect::QuarantineEvidencePersisted,
    ];
    if vault_expected {
        expected_effects.push(ProviderBoundaryEffect::QuarantineVaultIndexed);
    }
    if isolation_expected {
        expected_effects.push(ProviderBoundaryEffect::AgentIsolation);
    }
    if record.effects != expected_effects {
        return Err("provider boundary receipt effects do not match its decision".to_string());
    }
    let evidence =
        evidence.ok_or_else(|| "provider boundary receipt evidence is missing".to_string())?;
    if evidence.id != record.output
        || evidence.id != evidence.content_hash()
        || !provider_body_within_limit(&evidence.body)
        || provider_output_security_metadata_hash(evidence)? != record.output_security_metadata
    {
        return Err("provider boundary evidence does not match its receipt".to_string());
    }
    let expected_anomaly = detect_provider_output_evidence_anomaly(record.input, evidence);
    let expected_decision =
        ImmunePipeline::default().run(record.output, expected_anomaly.clone(), Vec::new());
    if record.anomaly != expected_anomaly
        || record.decision != expected_decision
        || record.decision.validation.containment.decision == QuarantineDecision::Accept
    {
        return Err("provider boundary receipt decision is not bound to its evidence".to_string());
    }
    if isolation_expected {
        let marker = isolation_marker_for(&record.agent_id).map_err(|error| error.to_string())?;
        if record.isolation_control != Some(marker.id) {
            return Err("provider isolation binding is invalid".to_string());
        }
    } else if record.isolation_control.is_some() {
        return Err("provider receipt unexpectedly references isolation".to_string());
    }
    Ok(record)
}

fn provider_output_security_metadata_hash(
    output: &Signal,
) -> std::result::Result<ContentHash, String> {
    #[derive(Serialize)]
    struct SecurityMetadata<'a> {
        provenance: &'a Provenance,
        attestation: &'a Option<roko_core::Attestation>,
    }

    // Signal content_hash already binds kind/body/author+tainted-bit/lineage/
    // tags. Bind the excluded fields that carry security authority here:
    // complete provenance and cryptographic attestation. Mutable scoring,
    // decay, lifecycle, access, and display metadata are deliberately not
    // security inputs to this boundary and remain outside the receipt.
    let metadata = SecurityMetadata {
        provenance: &output.provenance,
        attestation: &output.attestation,
    };
    let encoded = bounded_serialized_bytes(&metadata, MAX_PROVIDER_SECURITY_METADATA_BYTES)
        .map_err(|_| "provider output security metadata exceeds byte budget".to_string())?;
    Ok(ContentHash::of(&encoded))
}

fn provider_body_within_limit(body: &Body) -> bool {
    match body {
        Body::Empty => true,
        Body::Text(text) => text.len() <= MAX_PROVIDER_OUTPUT_BYTES,
        Body::Bytes(bytes) => bytes.len() <= MAX_PROVIDER_OUTPUT_BYTES,
        Body::Json(value) => bounded_json_bytes(value, MAX_PROVIDER_OUTPUT_BYTES).is_ok(),
    }
}

fn decode_exact_json_body<T>(body: &Body) -> std::result::Result<T, String>
where
    T: DeserializeOwned + Serialize,
{
    let Body::Json(raw) = body else {
        return Err("provider boundary body must use canonical JSON".to_string());
    };
    let decoded: T = serde_json::from_value(raw.clone()).map_err(|error| error.to_string())?;
    let canonical = serde_json::to_value(&decoded).map_err(|error| error.to_string())?;
    if canonical != *raw {
        return Err("provider boundary body contains a non-canonical nested schema".to_string());
    }
    Ok(decoded)
}

#[async_trait::async_trait]
impl Agent for ImmuneScreenedAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult {
        if let Some(denied) = self.preflight(input).await {
            return denied;
        }
        if self.live_output.is_some() && self.inner.supports_streaming() {
            let (result, exceeded) = self.drive_streaming_inner(input, ctx).await;
            if exceeded {
                return self.denied_result(
                    input,
                    Some(&result),
                    "provider_stream_limit_exceeded",
                    None,
                );
            }
            return self.screen_result(input, result).await;
        }
        let result = self.inner.run(input, ctx).await;
        self.screen_result(input, result).await
    }

    fn name(&self) -> &str {
        &self.agent_id
    }

    fn backend_id(&self) -> &'static str {
        if self.agent_id_valid {
            self.inner.backend_id()
        } else {
            "invalid-provider-identity"
        }
    }

    fn supports_streaming(&self) -> bool {
        self.agent_id_valid && self.inner.supports_streaming()
    }

    async fn run_streaming(
        &self,
        input: &Signal,
        ctx: &Context,
        event_tx: mpsc::Sender<StreamEvent>,
    ) -> AgentResult {
        if let Some(denied) = self.preflight(input).await {
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::Done {
                    finish_reason: "error: provider stream denied by immune boundary".to_string(),
                }))
                .await;
            return denied;
        }

        // Drive the inner stream through the shared counting+forwarding loop.
        // Provider events are buffered, limits are enforced, and qualifying
        // events reach the live output channel (when set) before screening.
        let (result, stream_limit_exceeded) = self.drive_streaming_inner(input, ctx).await;
        if stream_limit_exceeded {
            let denied =
                self.denied_result(input, Some(&result), "provider_stream_limit_exceeded", None);
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::Done {
                    finish_reason: "error: provider stream denied by immune boundary".to_string(),
                }))
                .await;
            return denied;
        }
        let screened = self.screen_result(input, result).await;
        if screened.success {
            // Provider events are not replayed: only the accepted canonical
            // final body can cross this boundary, so divergent reasoning,
            // tool-call, or content deltas cannot escape.
            if let Body::Text(text) = &screened.output.body
                && !text.is_empty()
            {
                let _ = event_tx
                    .send(StreamEvent::now(StreamEventKind::TextDelta(text.clone())))
                    .await;
            }
            if screened.usage.total_tokens() > 0 {
                let _ = event_tx
                    .send(StreamEvent::now(StreamEventKind::Usage(screened.usage)))
                    .await;
            }
        } else {
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::Done {
                    finish_reason: "error: provider stream denied by immune boundary".to_string(),
                }))
                .await;
        }
        screened
    }
}

/// Whether `kind` is model output, whose first arrival ends a call's time to
/// first token: text, reasoning or a tool call (gap-7a8474). Usage, the end
/// of a turn and a tool's own result are not.
fn is_model_output(kind: &StreamEventKind) -> bool {
    matches!(
        kind,
        StreamEventKind::TextDelta(_)
            | StreamEventKind::ReasoningDelta(_)
            | StreamEventKind::ToolCallStart { .. }
            | StreamEventKind::ToolCallDelta { .. }
            | StreamEventKind::ToolCallEnd { .. }
    )
}

fn stream_event_bytes(event: &StreamEvent) -> usize {
    match &event.kind {
        StreamEventKind::ReasoningDelta(text) | StreamEventKind::TextDelta(text) => text.len(),
        StreamEventKind::ToolCallStart { id, name } => id.len() + name.len(),
        StreamEventKind::ToolCallDelta { id, json_fragment } => id.len() + json_fragment.len(),
        StreamEventKind::ToolCallEnd { id, name, args } => {
            id.len() + name.len() + args.to_string().len()
        }
        StreamEventKind::ToolResult { id, output, .. } => id.len() + output.len(),
        StreamEventKind::Usage(_) | StreamEventKind::Done { .. } => 0,
    }
}

/// Wrap one factory-created agent at the automatic provider boundary.
///
/// `working_dir` is the directory the provider runs in; live tool steps show
/// paths inside it relative to it.
#[must_use]
pub(crate) fn wrap_provider_agent(
    agent: Box<dyn Agent>,
    requested_agent_id: &str,
    immune_root: Option<&Path>,
    working_dir: Option<&Path>,
    live_output: Option<LiveOutput>,
) -> Box<dyn Agent> {
    let workspace_root = immune_root
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut boundary = ImmuneScreenedAgent::durable(agent, requested_agent_id, workspace_root);
    if let Some(live) = live_output {
        boundary = boundary.with_live_output(live);
        if let Some(working_dir) = working_dir {
            boundary = boundary.with_tool_step_root(working_dir);
        }
    }
    Box::new(boundary)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use roko_core::{
        DEFAULT_QUARANTINE_VAULT_CAPACITY, QuarantineStatus, QuarantineVault, Query,
        ResponseAction, RokoError,
    };
    use roko_std::MemorySubstrate;
    use tempfile::tempdir;

    use super::*;
    use crate::immune_evidence::query_evidence_signals;

    struct CountingAgent {
        calls: Arc<AtomicUsize>,
        blank: bool,
        preserve_lineage: bool,
    }

    #[async_trait::async_trait]
    impl Agent for CountingAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let body = if self.blank {
                Body::text("   ")
            } else {
                Body::text("provider output")
            };
            let mut builder = Signal::builder(Kind::AgentOutput).body(body);
            if self.preserve_lineage {
                builder = builder.lineage([input.id]);
            }
            AgentResult::ok(builder.build())
        }

        fn name(&self) -> &str {
            "counting-agent"
        }
    }

    fn prompt() -> Signal {
        Signal::builder(Kind::Prompt)
            .body(Body::text("test prompt"))
            .build()
    }

    #[test]
    fn detector_reports_only_observed_structural_facts() {
        let input = prompt();
        let clean = AgentResult::ok(input.derive(Kind::AgentOutput, Body::text("ok")).build());
        assert_eq!(
            detect_provider_output_anomaly(&input, &clean),
            AnomalyScore::clean()
        );

        let mut validly_attested = clean.output.clone();
        let key = roko_core::attestation::SigningKey::from_bytes(&[13; 32]);
        validly_attested.attestation = Some(roko_core::attestation::sign(&validly_attested, &key));
        assert_eq!(
            detect_provider_output_anomaly(&input, &AgentResult::ok(validly_attested)),
            AnomalyScore::clean()
        );

        let missing_lineage = AgentResult::ok(
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("ok"))
                .build(),
        );
        let anomaly = detect_provider_output_anomaly(&input, &missing_lineage);
        assert_eq!(anomaly.score, 0.1);
        assert_eq!(anomaly.dimensions["missing_direct_input_lineage"], 0.1);
    }

    #[tokio::test]
    async fn clean_provider_result_crosses_boundary_without_effects() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: false,
                preserve_lineage: true,
            }),
            "clean-agent",
            store.clone(),
        );

        let result = boundary.run(&prompt(), &Context::now()).await;
        assert!(result.success);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(store.len().await.expect("store length"), 0);
    }

    #[tokio::test]
    async fn legacy_provider_result_without_lineage_remains_usable() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: false,
                preserve_lineage: false,
            }),
            "legacy-provider-agent",
            store.clone(),
        );

        let result = boundary.run(&prompt(), &Context::now()).await;
        assert!(result.success);
        assert_eq!(
            result.output.body.as_text().expect("text output"),
            "provider output"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(store.len().await.expect("store length"), 0);
    }

    #[tokio::test]
    async fn invalid_provider_attestation_is_denied_live_and_cannot_replay_as_accept() {
        struct FixedOutputAgent {
            output: Signal,
        }

        #[async_trait::async_trait]
        impl Agent for FixedOutputAgent {
            async fn run(&self, _input: &Signal, _ctx: &Context) -> AgentResult {
                AgentResult::ok(self.output.clone())
            }

            fn name(&self) -> &str {
                "invalid-attestation-agent"
            }
        }

        let workspace = tempdir().unwrap();
        let input = prompt();
        let mut output = input
            .derive(Kind::AgentOutput, Body::text("otherwise clean output"))
            .build();
        output.attestation = Some(roko_core::Attestation {
            signature: roko_core::Ed25519Signature([7; 64]),
            public_key: roko_core::PublicKey([3; 32]),
            chain_attestation: None,
        });
        assert_eq!(output.id, output.content_hash());
        let anomaly = detect_provider_output_anomaly(&input, &AgentResult::ok(output.clone()));
        assert_eq!(anomaly.dimensions["invalid_output_attestation"], 1.0);

        let boundary = ImmuneScreenedAgent::durable(
            Box::new(FixedOutputAgent { output }),
            "invalid-attestation-agent",
            workspace.path(),
        );
        let denied = boundary.run(&input, &Context::now()).await;
        assert!(!denied.success);

        let receipts = query_evidence_signals(
            workspace.path(),
            &Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()),
            Some(("agent_id", "invalid-attestation-agent")),
            1,
        )
        .unwrap();
        let mut accepted_receipt = receipts[0].clone();
        let mut accepted_record: ProviderBoundaryRecord = accepted_receipt.body.as_json().unwrap();
        let evidence =
            crate::immune_evidence::get_evidence_signal(workspace.path(), &accepted_record.output)
                .unwrap()
                .unwrap();
        accepted_record.anomaly = AnomalyScore::clean();
        accepted_record.decision = ImmunePipeline::default().run(
            accepted_record.output,
            accepted_record.anomaly.clone(),
            Vec::new(),
        );
        assert_eq!(
            accepted_record.decision.validation.containment.decision,
            QuarantineDecision::Accept
        );
        accepted_record.effects = vec![
            ProviderBoundaryEffect::DeliveryDenied,
            ProviderBoundaryEffect::QuarantineEvidencePersisted,
            ProviderBoundaryEffect::QuarantineVaultIndexed,
        ];
        accepted_record.isolation_control = None;
        accepted_receipt.body = Body::from_json(&accepted_record).unwrap();
        accepted_receipt
            .tags
            .insert("quarantine_decision".to_string(), "accept".to_string());
        accepted_receipt.id = accepted_receipt.content_hash();
        assert!(
            validate_provider_boundary_receipt(&accepted_receipt, Some(&evidence), true).is_err(),
            "replay must recompute invalid attestation evidence rather than trust an Accept record"
        );
    }

    #[tokio::test]
    async fn high_anomaly_is_denied_persisted_ordered_and_durably_isolated() {
        let workspace = tempdir().expect("temp workspace");
        let calls = Arc::new(AtomicUsize::new(0));
        let input = prompt();
        let boundary = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: true,
                preserve_lineage: true,
            }),
            "isolated-agent",
            workspace.path(),
        );

        let first = boundary.run(&input, &Context::now()).await;
        assert!(!first.success);
        assert_eq!(first.output.tag("immune_denied"), Some("true"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let records = query_evidence_signals(
            workspace.path(),
            &Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()),
            Some(("agent_id", "isolated-agent")),
            DEFAULT_QUARANTINE_VAULT_CAPACITY,
        )
        .expect("query records");
        assert_eq!(records.len(), 1);
        let record: ProviderBoundaryRecord = records[0].body.as_json().expect("typed record");
        assert_eq!(record.stage_order, IMMUNE_STAGE_ORDER.map(str::to_string));
        assert_eq!(
            record.decision.validation.containment.assessment.severity,
            ThreatSeverity::High
        );
        assert_eq!(
            record.decision.validation.containment.action,
            Some(ResponseAction::IsolateAgent)
        );
        assert!(record.isolation_control.is_some());
        assert!(
            record
                .effects
                .contains(&ProviderBoundaryEffect::QuarantineVaultIndexed)
        );
        let vault =
            QuarantineVault::load(crate::tool_immune::quarantine_vault_path(workspace.path()))
                .expect("load provider quarantine vault");
        assert_eq!(vault.count(), 1);
        assert_eq!(
            vault
                .get(&record.output)
                .expect("provider vault entry")
                .status,
            QuarantineStatus::Escalated
        );
        let evidence =
            crate::immune_evidence::get_evidence_signal(workspace.path(), &record.output)
                .unwrap()
                .unwrap();
        validate_provider_boundary_receipt(&records[0], Some(&evidence), true).unwrap();
        let mut forged_trust = evidence.clone();
        forged_trust.provenance.trust = 0.25;
        forged_trust.provenance.taint_info = Some(roko_core::TaintInfo::external("forged"));
        forged_trust.provenance.session = Some("forged-session".to_string());
        forged_trust.provenance.taint_level = roko_core::TaintLevel::Secret;
        forged_trust.provenance.trust_origin =
            roko_core::provenance::TrustOriginTaintLevel::External;
        forged_trust.id = forged_trust.content_hash();
        assert_eq!(forged_trust.id, evidence.id);
        assert!(
            validate_provider_boundary_receipt(&records[0], Some(&forged_trust), true).is_err(),
            "receipt must bind exact trust and taint metadata omitted by Signal content_hash"
        );
        let mut forged_attestation = evidence.clone();
        forged_attestation.attestation = Some(roko_core::Attestation {
            signature: roko_core::Ed25519Signature([7; 64]),
            public_key: roko_core::PublicKey([3; 32]),
            chain_attestation: None,
        });
        forged_attestation.id = forged_attestation.content_hash();
        assert_eq!(forged_attestation.id, evidence.id);
        assert!(
            validate_provider_boundary_receipt(&records[0], Some(&forged_attestation), true)
                .is_err(),
            "receipt must bind cryptographic attestation omitted by Signal content_hash"
        );
        let mut tampered_receipt = records[0].clone();
        tampered_receipt
            .tags
            .insert("quarantine_decision".to_string(), "accept".to_string());
        tampered_receipt.id = tampered_receipt.content_hash();
        assert!(
            validate_provider_boundary_receipt(&tampered_receipt, Some(&evidence), true).is_err()
        );
        let mut coupled = records[0].clone();
        let mut coupled_record: ProviderBoundaryRecord = coupled.body.as_json().unwrap();
        coupled_record.anomaly = AnomalyScore::from_score(1.0);
        coupled_record.decision = ImmunePipeline::default().run(
            coupled_record.output,
            coupled_record.anomaly.clone(),
            Vec::new(),
        );
        coupled.body = Body::from_json(&coupled_record).unwrap();
        coupled.id = coupled.content_hash();
        assert!(
            validate_provider_boundary_receipt(&coupled, Some(&evidence), true).is_err(),
            "receipt anomaly must be recomputed from persisted output evidence"
        );
        let mut rehashed_provenance = records[0].clone();
        rehashed_provenance.provenance = Provenance::trusted("forged-boundary");
        rehashed_provenance.id = rehashed_provenance.content_hash();
        assert!(
            validate_provider_boundary_receipt(&rehashed_provenance, Some(&evidence), true)
                .is_err(),
            "boundary-authored provenance must be exact even after rehashing"
        );

        // Same wrapper is blocked before the provider, without duplicate writes.
        let second = boundary.run(&input, &Context::now()).await;
        assert!(!second.success);
        assert_eq!(second.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            query_evidence_signals(
                workspace.path(),
                &Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()),
                Some(("agent_id", "isolated-agent")),
                DEFAULT_QUARANTINE_VAULT_CAPACITY,
            )
            .expect("query idempotent records")
            .len(),
            1
        );
        drop(boundary);

        // Isolation survives wrapper recreation and remains idempotent.
        let recreated = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: false,
                preserve_lineage: true,
            }),
            "isolated-agent",
            workspace.path(),
        );
        let third = recreated.run(&input, &Context::now()).await;
        assert!(!third.success);
        assert_eq!(third.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            query_evidence_signals(
                workspace.path(),
                &Kind::Custom(PROVIDER_BOUNDARY_RECORD_KIND.to_string()),
                Some(("agent_id", "isolated-agent")),
                DEFAULT_QUARANTINE_VAULT_CAPACITY,
            )
            .expect("query stable records")
            .len(),
            1
        );
    }

    #[tokio::test]
    async fn full_evidence_ledger_cannot_reenable_isolated_provider() {
        let workspace = tempdir().expect("temp workspace");
        let filler = (0..crate::immune_evidence::MAX_IMMUNE_EVIDENCE_SIGNALS)
            .map(|index| {
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text(format!("evidence-capacity-{index}")))
                    .tag("source", "capacity-test")
                    .build()
            })
            .collect::<Vec<_>>();
        persist_evidence_signals(workspace.path(), &filler).expect("fill evidence ledger");
        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: true,
                preserve_lineage: true,
            }),
            "capacity-isolated-agent",
            workspace.path(),
        );

        let first = boundary.run(&prompt(), &Context::now()).await;
        assert!(!first.success, "current suspect output stays denied");
        assert_eq!(
            first.output.tag("immune_reason"),
            Some("containment_persistence_failed")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = boundary.run(&prompt(), &Context::now()).await;
        assert!(!second.success);
        assert_eq!(second.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "provider was not invoked");
    }

    #[tokio::test]
    async fn provider_isolation_survives_attempt_deletion_without_cross_workspace_collision() {
        let authority_a = tempdir().unwrap();
        let authority_b = tempdir().unwrap();
        let attempt = authority_a.path().join("attempt-worktree");
        std::fs::create_dir_all(&attempt).unwrap();
        let input = prompt();
        let calls_a = Arc::new(AtomicUsize::new(0));
        let first = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls_a),
                blank: true,
                preserve_lineage: true,
            }),
            "shared-agent-id",
            authority_a.path(),
        );
        assert!(!first.run(&input, &Context::now()).await.success);
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert!(!attempt.join(".roko/immune").exists());
        std::fs::remove_dir_all(&attempt).unwrap();

        let later_same_root = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls_a),
                blank: false,
                preserve_lineage: true,
            }),
            "shared-agent-id",
            authority_a.path(),
        );
        let denied = later_same_root.run(&input, &Context::now()).await;
        assert_eq!(denied.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);

        let calls_b = Arc::new(AtomicUsize::new(0));
        let separate_workspace = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls_b),
                blank: false,
                preserve_lineage: true,
            }),
            "shared-agent-id",
            authority_b.path(),
        );
        assert!(
            separate_workspace
                .run(&input, &Context::now())
                .await
                .success
        );
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn malformed_evidence_cannot_prevent_agent_isolation_commit() {
        let workspace = tempdir().unwrap();
        let evidence_path = crate::immune_evidence::immune_evidence_path(workspace.path());
        std::fs::create_dir_all(evidence_path.parent().unwrap()).unwrap();
        std::fs::write(&evidence_path, b"{malformed").unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = ImmuneScreenedAgent::durable(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: true,
                preserve_lineage: true,
            }),
            "malformed-evidence-agent",
            workspace.path(),
        );

        let first = boundary.run(&prompt(), &Context::now()).await;
        assert!(!first.success);
        assert_eq!(
            first.output.tag("immune_reason"),
            Some("containment_persistence_failed")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let second = boundary.run(&prompt(), &Context::now()).await;
        assert!(!second.success);
        assert_eq!(second.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    struct FailingPutStore;

    #[async_trait::async_trait]
    impl Store for FailingPutStore {
        async fn put(&self, _signal: Signal) -> Result<ContentHash> {
            Err(RokoError::Store("injected write failure".to_string()))
        }

        async fn get(&self, _id: &ContentHash) -> Result<Option<Signal>> {
            Ok(None)
        }

        async fn query(&self, _query: &Query, _ctx: &Context) -> Result<Vec<Signal>> {
            Ok(Vec::new())
        }

        async fn prune(&self, _threshold: f32, _ctx: &Context) -> Result<usize> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn containment_write_failure_denies_without_leaking_suspect_output() {
        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(CountingAgent {
                calls: Arc::clone(&calls),
                blank: true,
                preserve_lineage: true,
            }),
            "write-failure-agent",
            Arc::new(FailingPutStore),
        );

        let result = boundary.run(&prompt(), &Context::now()).await;
        assert!(!result.success);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.output.tag("immune_reason"),
            Some("containment_persistence_failed")
        );
        assert_eq!(
            result.output.body.as_text().expect("safe denial"),
            "provider result denied by immune boundary"
        );
        assert!(result.trace.is_empty());
    }

    struct StreamingBlankAgent;

    #[async_trait::async_trait]
    impl Agent for StreamingBlankAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            AgentResult::ok(input.derive(Kind::AgentOutput, Body::text(" ")).build())
        }

        fn name(&self) -> &str {
            "streaming-blank-agent"
        }

        fn supports_streaming(&self) -> bool {
            true
        }

        async fn run_streaming(
            &self,
            input: &Signal,
            _ctx: &Context,
            event_tx: mpsc::Sender<StreamEvent>,
        ) -> AgentResult {
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::TextDelta(
                    "suspect streamed content".to_string(),
                )))
                .await;
            AgentResult::ok(input.derive(Kind::AgentOutput, Body::text(" ")).build())
        }
    }

    #[tokio::test]
    async fn suspicious_stream_is_buffered_and_never_released() {
        let store = Arc::new(MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(StreamingBlankAgent),
            "streaming-blank-agent",
            store,
        );
        let (tx, mut rx) = mpsc::channel(8);
        let result = boundary.run_streaming(&prompt(), &Context::now(), tx).await;

        assert!(!result.success);
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(), 1);
        assert!(
            matches!(&chunks[0].kind, StreamEventKind::Done { finish_reason } if finish_reason.starts_with("error:"))
        );
    }

    struct DivergentStreamingAgent {
        overflow: bool,
    }

    #[async_trait::async_trait]
    impl Agent for DivergentStreamingAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            AgentResult::ok(
                input
                    .derive(Kind::AgentOutput, Body::text("accepted final output"))
                    .build(),
            )
        }

        fn name(&self) -> &str {
            "divergent-stream-agent"
        }

        fn supports_streaming(&self) -> bool {
            true
        }

        async fn run_streaming(
            &self,
            input: &Signal,
            _ctx: &Context,
            event_tx: mpsc::Sender<StreamEvent>,
        ) -> AgentResult {
            let count = if self.overflow {
                MAX_PROVIDER_STREAM_CHUNKS + 1
            } else {
                1
            };
            for _ in 0..count {
                let _ = event_tx
                    .send(StreamEvent::now(StreamEventKind::TextDelta(
                        "divergent streamed content".to_string(),
                    )))
                    .await;
            }
            AgentResult::ok(
                input
                    .derive(Kind::AgentOutput, Body::text("accepted final output"))
                    .build(),
            )
        }
    }

    #[tokio::test]
    async fn accepted_stream_emits_only_exact_canonical_final_output() {
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(DivergentStreamingAgent { overflow: false }),
            "divergent-stream-agent",
            Arc::new(MemorySubstrate::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let result = boundary.run_streaming(&prompt(), &Context::now(), tx).await;
        assert!(result.success);
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(), 1);
        assert!(matches!(
            &chunks[0].kind,
            StreamEventKind::TextDelta(content) if content == "accepted final output"
        ));
    }

    #[tokio::test]
    async fn provider_stream_count_overflow_is_denied_without_replay() {
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(DivergentStreamingAgent { overflow: true }),
            "overflow-stream-agent",
            Arc::new(MemorySubstrate::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let result = boundary.run_streaming(&prompt(), &Context::now(), tx).await;
        assert!(!result.success);
        assert_eq!(
            result.output.tag("immune_reason"),
            Some("provider_stream_limit_exceeded")
        );
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(), 1);
        assert!(
            matches!(&chunks[0].kind, StreamEventKind::Done { finish_reason } if finish_reason.starts_with("error:"))
        );
    }

    /// Streams a usage update, then text after `delay`, and returns the time
    /// to first token it measured itself, if any.
    struct TimedStreamingAgent {
        delay: std::time::Duration,
        own_ttft_ms: Option<u64>,
    }

    #[async_trait::async_trait]
    impl Agent for TimedStreamingAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            AgentResult::ok(
                input
                    .derive(Kind::AgentOutput, Body::text("timed output"))
                    .build(),
            )
        }

        fn name(&self) -> &str {
            "timed-stream-agent"
        }

        fn supports_streaming(&self) -> bool {
            true
        }

        async fn run_streaming(
            &self,
            input: &Signal,
            _ctx: &Context,
            event_tx: mpsc::Sender<StreamEvent>,
        ) -> AgentResult {
            let usage = crate::usage::Usage::zero();
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::Usage(usage)))
                .await;
            tokio::time::sleep(self.delay).await;
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::TextDelta(
                    "timed output".to_string(),
                )))
                .await;
            let output = input
                .derive(Kind::AgentOutput, Body::text("timed output"))
                .build();
            AgentResult {
                ttft_ms: self.own_ttft_ms,
                ..AgentResult::ok(output)
            }
        }
    }

    /// gap-7a8474: the boundary times a stream's first model output, not a
    /// usage update before it, as the call's time to first token, and keeps
    /// one the provider measured itself.
    #[tokio::test]
    async fn streamed_first_output_sets_time_to_first_token() {
        let run = |own_ttft_ms: Option<u64>| async move {
            let boundary = ImmuneScreenedAgent::with_store(
                Box::new(TimedStreamingAgent {
                    delay: std::time::Duration::from_millis(20),
                    own_ttft_ms,
                }),
                "timed-stream-agent",
                Arc::new(MemorySubstrate::new()),
            );
            let (tx, _rx) = mpsc::channel(8);
            boundary.run_streaming(&prompt(), &Context::now(), tx).await
        };

        let timed = run(None).await;
        assert!(timed.success);
        let ttft_ms = timed.ttft_ms.expect("the stream showed model output");
        assert!(ttft_ms >= 20, "{ttft_ms} ms");
        assert_eq!(run(Some(7)).await.ttft_ms, Some(7));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn canonical_provider_factory_installs_boundary_automatically() {
        use crate::provider::{AgentOptions, create_agent_for_model};
        use roko_core::config::schema::RokoConfig;

        let workspace = tempdir().expect("temp workspace");
        let attempt = workspace.path().join("attempt-worktree");
        std::fs::create_dir_all(&attempt).unwrap();
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.command = None;
        let options = AgentOptions {
            command: Some("/bin/sh".to_string()),
            extra_args: vec!["-c".to_string(), "exit 0".to_string()],
            working_dir: Some(attempt.clone()),
            immune_root: Some(workspace.path().to_path_buf()),
            name: "factory-immune-agent".to_string(),
            ..AgentOptions::default()
        };
        let agent = create_agent_for_model(&config, "missing-model", options)
            .expect("create fallback provider agent");
        let result = agent.run(&prompt(), &Context::now()).await;

        assert!(!result.success, "blank provider output must be denied");
        assert_eq!(
            result.output.tag("immune_reason"),
            Some("provider_output_quarantined")
        );
        assert!(crate::immune_evidence::immune_evidence_path(workspace.path()).is_file());
        assert!(!attempt.join(".roko/immune").exists());
        std::fs::remove_dir_all(&attempt).unwrap();

        let later_attempt = workspace.path().join("later-attempt");
        std::fs::create_dir_all(&later_attempt).unwrap();
        let later_options = AgentOptions {
            command: Some("/bin/sh".to_string()),
            extra_args: vec!["-c".to_string(), "exit 7".to_string()],
            working_dir: Some(later_attempt.clone()),
            immune_root: Some(workspace.path().to_path_buf()),
            name: "factory-immune-agent".to_string(),
            ..AgentOptions::default()
        };
        let later = create_agent_for_model(&config, "missing-model", later_options)
            .expect("recreate fallback provider agent")
            .run(&prompt(), &Context::now())
            .await;
        assert_eq!(later.output.tag("immune_reason"), Some("agent_isolated"));
        assert!(!later_attempt.join(".roko/immune").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn provider_factory_rejects_invalid_agent_ids_before_inner_or_persistence() {
        use crate::provider::{AgentOptions, create_agent_for_model};
        use roko_core::config::schema::RokoConfig;

        let workspace = tempdir().expect("temp workspace");
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.command = None;
        let invalid_ids = [
            String::new(),
            "agent\nPASSWORD=identity-secret".to_string(),
            "x".repeat(257),
            "PASSWORD=identity-secret".to_string(),
            format!("sk-proj-{}", "A".repeat(32)),
        ];

        for (index, invalid_id) in invalid_ids.into_iter().enumerate() {
            let attempt = workspace.path().join(format!("attempt-{index}"));
            std::fs::create_dir_all(&attempt).unwrap();
            let sentinel = attempt.join("inner-was-invoked");
            let options = AgentOptions {
                command: Some("/bin/sh".to_string()),
                extra_args: vec!["-c".to_string(), format!("touch {}", sentinel.display())],
                working_dir: Some(attempt),
                immune_root: Some(workspace.path().to_path_buf()),
                name: invalid_id.clone(),
                ..AgentOptions::default()
            };
            let agent = create_agent_for_model(&config, "missing-model", options)
                .expect("construct fail-closed provider boundary");
            assert!(agent.name().starts_with("invalid-agent-"));
            if !invalid_id.is_empty() {
                assert!(!agent.name().contains(&invalid_id));
            }

            let result = agent.run(&prompt(), &Context::now()).await;
            assert!(!result.success);
            assert_eq!(
                result.output.tag("immune_reason"),
                Some("invalid_agent_identity")
            );
            let visible = serde_json::to_string(&result.output).expect("serialize denied result");
            if !invalid_id.is_empty() {
                assert!(!visible.contains(&invalid_id));
            }
            assert!(!visible.contains("identity-secret"));
            assert!(
                !sentinel.exists(),
                "invalid identity invoked inner provider"
            );
        }

        assert!(!crate::immune_evidence::immune_evidence_path(workspace.path()).exists());
        assert!(!crate::immune_evidence::agent_controls_path(workspace.path()).exists());
    }

    // ─── T16: live output tests ──────────────────────────────────────────────────

    /// An inner agent that emits one ToolCallEnd event, then waits for a
    /// release signal before returning its result. Used to prove the live
    /// event arrives *before* the run completes.
    struct PausingToolAgent {
        /// Fired by the agent after emitting the tool event.
        paused_notify: Arc<tokio::sync::Notify>,
        /// Fired by the test to let the agent finish.
        resume_notify: Arc<tokio::sync::Notify>,
        /// When true, return a blank body so the boundary will deny the result.
        deny: bool,
    }

    #[async_trait::async_trait]
    impl Agent for PausingToolAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            let body = if self.deny {
                Body::text("   ")
            } else {
                Body::text("clean output")
            };
            AgentResult::ok(input.derive(Kind::AgentOutput, body).build())
        }

        fn name(&self) -> &str {
            "pausing-tool-agent"
        }

        fn supports_streaming(&self) -> bool {
            true
        }

        async fn run_streaming(
            &self,
            input: &Signal,
            _ctx: &Context,
            event_tx: mpsc::Sender<StreamEvent>,
        ) -> AgentResult {
            // Emit a ToolCallEnd.
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::ToolCallEnd {
                    id: "call-1".to_string(),
                    name: "Write".to_string(),
                    args: serde_json::json!({ "file_path": "src/main.rs" }),
                }))
                .await;
            // Notify the test we are paused and wait for release.
            self.paused_notify.notify_one();
            self.resume_notify.notified().await;
            let body = if self.deny {
                Body::text("   ")
            } else {
                Body::text("clean output")
            };
            AgentResult::ok(input.derive(Kind::AgentOutput, body).build())
        }
    }

    #[tokio::test]
    async fn live_tool_step_arrives_before_inner_agent_finishes() {
        let paused = Arc::new(tokio::sync::Notify::new());
        let resume = Arc::new(tokio::sync::Notify::new());
        let store = Arc::new(roko_std::MemorySubstrate::new());
        let _boundary = ImmuneScreenedAgent::with_store(
            Box::new(PausingToolAgent {
                paused_notify: Arc::clone(&paused),
                resume_notify: Arc::clone(&resume),
                deny: false,
            }),
            "live-tool-step-agent",
            store,
        )
        .with_live_output(LiveOutput {
            sink: {
                let (tx, _rx) = mpsc::channel(16);
                // We'll keep our own reference below.
                tx
            },
            trusted: false,
        });

        // Rebuild with a channel we can inspect.
        let (live_tx, mut live_rx) = mpsc::channel::<LiveAgentEvent>(16);
        let paused2 = Arc::clone(&paused);
        let resume2 = Arc::clone(&resume);
        let store2 = Arc::new(roko_std::MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(PausingToolAgent {
                paused_notify: Arc::clone(&paused2),
                resume_notify: Arc::clone(&resume2),
                deny: false,
            }),
            "live-tool-step-agent",
            store2,
        )
        .with_live_output(LiveOutput {
            sink: live_tx,
            trusted: false,
        });

        let run_handle = tokio::spawn(async move {
            let (tx, _rx) = mpsc::channel(16);
            boundary.run_streaming(&prompt(), &Context::now(), tx).await
        });

        // Wait for the agent to pause after its tool call.
        paused2.notified().await;

        // Tool step must already be in the live channel.
        let event = live_rx
            .try_recv()
            .expect("ToolStep must arrive before agent finishes");
        assert!(
            matches!(event, LiveAgentEvent::ToolStep { ref name, ref target, .. }
                if name == "Write" && target == "src/main.rs"),
            "unexpected event: {event:?}"
        );

        // Release the agent and wait for the run to complete.
        resume2.notify_one();
        let result = run_handle.await.expect("run task");
        assert!(result.success, "clean output must be accepted");
    }

    #[tokio::test]
    async fn no_unscreened_event_without_trusted_flag() {
        let paused = Arc::new(tokio::sync::Notify::new());
        let resume = Arc::new(tokio::sync::Notify::new());
        let (live_tx, mut live_rx) = mpsc::channel::<LiveAgentEvent>(16);
        let store = Arc::new(roko_std::MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(PausingToolAgent {
                paused_notify: Arc::clone(&paused),
                resume_notify: Arc::clone(&resume),
                deny: false,
            }),
            "untrusted-live-agent",
            store,
        )
        .with_live_output(LiveOutput {
            sink: live_tx,
            trusted: false,
        });

        let paused2 = Arc::clone(&paused);
        let resume2 = Arc::clone(&resume);
        let run_handle = tokio::spawn(async move {
            let (tx, _rx) = mpsc::channel(16);
            boundary.run_streaming(&prompt(), &Context::now(), tx).await
        });

        paused2.notified().await;
        resume2.notify_one();
        let _ = run_handle.await.expect("run task");

        // Collect all live events.
        let mut events = Vec::new();
        while let Ok(e) = live_rx.try_recv() {
            events.push(e);
        }
        // ToolStep must appear (always).
        assert!(
            events
                .iter()
                .any(|e| matches!(e, LiveAgentEvent::ToolStep { .. })),
            "ToolStep must appear even without trusted"
        );
        // Unscreened must NOT appear.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, LiveAgentEvent::Unscreened(_))),
            "Unscreened must not appear when trusted=false; got: {events:?}"
        );
    }

    #[tokio::test]
    async fn screened_result_identical_with_and_without_live_output() {
        let store_a = Arc::new(roko_std::MemorySubstrate::new());
        let store_b = Arc::new(roko_std::MemorySubstrate::new());

        let make_agent = |store: Arc<roko_std::MemorySubstrate>| {
            ImmuneScreenedAgent::with_store(
                Box::new(PausingToolAgent {
                    paused_notify: Arc::new(tokio::sync::Notify::new()),
                    resume_notify: {
                        let n = Arc::new(tokio::sync::Notify::new());
                        n.notify_one(); // pre-release so it never blocks
                        n
                    },
                    deny: false,
                }),
                "screened-result-agent",
                store,
            )
        };

        let without_live = make_agent(store_a);
        let (live_tx, _live_rx) = mpsc::channel(16);
        let with_live = make_agent(store_b).with_live_output(LiveOutput {
            sink: live_tx,
            trusted: true,
        });

        let input = prompt();
        let result_a = without_live.run(&input, &Context::now()).await;
        let result_b = with_live.run(&input, &Context::now()).await;

        assert_eq!(result_a.success, result_b.success);
        assert_eq!(result_a.output.body, result_b.output.body);
    }

    #[tokio::test]
    async fn denied_result_is_denied_regardless_of_live_output() {
        let (live_tx, mut live_rx) = mpsc::channel::<LiveAgentEvent>(16);
        let store = Arc::new(roko_std::MemorySubstrate::new());
        let boundary = ImmuneScreenedAgent::with_store(
            Box::new(PausingToolAgent {
                paused_notify: Arc::new(tokio::sync::Notify::new()),
                resume_notify: {
                    let n = Arc::new(tokio::sync::Notify::new());
                    n.notify_one();
                    n
                },
                deny: true, // blank body → boundary denies
            }),
            "denied-with-live-agent",
            store,
        )
        .with_live_output(LiveOutput {
            sink: live_tx,
            trusted: true,
        });

        let result = boundary.run(&prompt(), &Context::now()).await;
        assert!(!result.success, "blank output must still be denied");
        assert_eq!(result.output.tag("immune_denied"), Some("true"));

        // The live channel may have received a ToolStep, but the screened result is still denied.
        let mut live_events = Vec::new();
        while let Ok(e) = live_rx.try_recv() {
            live_events.push(e);
        }
        assert!(
            live_events
                .iter()
                .any(|e| matches!(e, LiveAgentEvent::ToolStep { .. })),
            "ToolStep must have been sent even for a denied run"
        );
    }

    /// An inner agent that streams one `Write` call with the given arguments,
    /// then returns clean output.
    struct WritingAgent {
        args: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl Agent for WritingAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            AgentResult::ok(
                input
                    .derive(Kind::AgentOutput, Body::text("clean output"))
                    .build(),
            )
        }

        fn name(&self) -> &str {
            "writing-agent"
        }

        fn supports_streaming(&self) -> bool {
            true
        }

        async fn run_streaming(
            &self,
            input: &Signal,
            ctx: &Context,
            event_tx: mpsc::Sender<StreamEvent>,
        ) -> AgentResult {
            let _ = event_tx
                .send(StreamEvent::now(StreamEventKind::ToolCallEnd {
                    id: "call-1".to_string(),
                    name: "Write".to_string(),
                    args: self.args.clone(),
                }))
                .await;
            self.run(input, ctx).await
        }
    }

    #[tokio::test]
    async fn factory_boundary_shows_tool_steps_relative_to_the_working_dir() {
        let workspace = tempdir().expect("temp workspace");
        let attempt = workspace.path().join("attempt-worktree");
        std::fs::create_dir_all(&attempt).unwrap();
        let (live_tx, mut live_rx) = mpsc::channel::<LiveAgentEvent>(16);
        let agent = wrap_provider_agent(
            Box::new(WritingAgent {
                args: serde_json::json!({ "file_path": attempt.join("hello/main.rs") }),
            }),
            "relative-step-agent",
            Some(workspace.path()),
            Some(&attempt),
            Some(LiveOutput {
                sink: live_tx,
                trusted: false,
            }),
        );

        let result = agent.run(&prompt(), &Context::now()).await;
        assert!(result.success, "clean output must be accepted");
        let event = live_rx.try_recv().expect("ToolStep");
        assert!(
            matches!(&event, LiveAgentEvent::ToolStep { target, .. } if target == "hello/main.rs"),
            "unexpected event: {event:?}"
        );
    }
}
