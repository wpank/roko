//! Durable safety provenance for tool effects (gap-ff95f5).
//!
//! [`crate::dispatcher::ToolDispatcher`] reports every tool call to a
//! host-provided [`SafetyProvenanceSink`]: an intent once every safety stage
//! has passed and before the tool's handler runs, and an outcome when the
//! call ends, or a denial when the dispatcher stopped it first. The sink must
//! acknowledge the intent durably, or the handler does not run (fail closed).
//!
//! Records hold IDs, keyed digests, taint levels and bounded reason codes
//! only: never prompt text, tool arguments, tool output or secrets. The
//! digests are BLAKE3 keyed hashes under the sink's
//! [`SafetyProvenanceSink::digest_key`], so nobody can recover a low-entropy
//! argument by hashing guesses.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use roko_core::ContentHash;
use roko_core::config::fingerprint::canonical_json;
use roko_core::extension::CamelTaintLevel;
use roko_core::tool::ToolResult;
use serde::{Deserialize, Serialize};

use super::taint_propagation::{TaintReason, TaintTracker};

/// Which call a provenance record is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceCall {
    /// The run the call belongs to.
    pub run_id: String,
    /// The task the call belongs to.
    pub task_id: String,
    /// The task's attempt.
    pub attempt_id: String,
    /// The tool-loop turn.
    pub turn_id: String,
    /// The provider's id for the call.
    pub call_id: String,
    /// The tool's name.
    pub tool: String,
    /// Keyed digest of the call's arguments ([`arguments_digest`]).
    pub args_digest: ContentHash,
}

/// The record a sink acknowledges before a tool's handler runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceIntent {
    /// The call.
    pub call: ProvenanceCall,
    /// The turn's trust-origin taint when the call was admitted.
    pub taint: CamelTaintLevel,
}

/// A sink's acknowledgement of a durable intent record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceAck {
    /// The sink's id for the intent record.
    pub record_id: String,
}

/// How a call ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceVerdict {
    /// The handler ran, and the call returned a result.
    Succeeded,
    /// The handler ran, and the call returned an error.
    Failed,
    /// The dispatcher stopped the call before its handler ran.
    Denied,
}

/// The record of how a call ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceOutcome {
    /// The call.
    pub call: ProvenanceCall,
    /// The acknowledged intent this outcome settles; `None` for a call
    /// stopped before its intent.
    pub intent: Option<String>,
    /// How the call ended.
    pub verdict: ProvenanceVerdict,
    /// Bounded reason code of an error or denial, such as
    /// `permission_denied`.
    pub reason: Option<String>,
    /// Keyed digest of the call's result ([`result_digest`]); `None` when
    /// the handler did not run.
    pub result_digest: Option<ContentHash>,
    /// The turn's trust-origin taint after the call.
    pub taint: CamelTaintLevel,
}

/// One record a sink keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum ProvenanceRecord {
    /// An acknowledged intent.
    Intent(ProvenanceIntent),
    /// An outcome or a denial.
    Outcome(ProvenanceOutcome),
}

/// Why a sink could not record a provenance record.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("safety provenance: {0}")]
pub struct ProvenanceError(pub String);

/// Where the dispatcher records safety provenance. A host implements it over
/// its durable stores; [`MemoryProvenanceSink`] keeps the records in memory.
pub trait SafetyProvenanceSink: Send + Sync {
    /// Key of the argument and result digests. Keep it secret, and stable
    /// across restarts: digests made under another key no longer match.
    fn digest_key(&self) -> [u8; 32];

    /// Durably record `intent` and acknowledge it. The dispatcher calls this
    /// after every safety stage has passed and before the tool's handler
    /// runs; on an error the handler does not run.
    ///
    /// # Errors
    ///
    /// Returns an error when the intent is not durably recorded.
    fn record_intent(&self, intent: &ProvenanceIntent) -> Result<ProvenanceAck, ProvenanceError>;

    /// Record how a call ended, or that the dispatcher stopped it. The
    /// dispatcher only logs a failure: the call has already happened.
    ///
    /// # Errors
    ///
    /// Returns an error when the outcome is not recorded.
    fn record_outcome(&self, outcome: &ProvenanceOutcome) -> Result<(), ProvenanceError>;
}

/// Keyed digest of a call's `arguments`: their canonical JSON (RFC 8785), so
/// key order does not change it, hashed under `key`.
#[must_use]
pub fn arguments_digest(key: &[u8; 32], arguments: &serde_json::Value) -> ContentHash {
    keyed_digest(key, b"arguments", canonical_json(arguments).as_bytes())
}

/// Keyed digest of a call's `result`, made the way [`arguments_digest`] is.
#[must_use]
pub fn result_digest(key: &[u8; 32], result: &ToolResult) -> ContentHash {
    let canonical = serde_json::to_value(result)
        .map(|value| canonical_json(&value))
        .unwrap_or_default();
    keyed_digest(key, b"result", canonical.as_bytes())
}

/// BLAKE3 keyed hash of `bytes` under `key`, separated by `domain` so that an
/// argument and a result with the same bytes do not share a digest.
fn keyed_digest(key: &[u8; 32], domain: &[u8], bytes: &[u8]) -> ContentHash {
    let mut input = Vec::with_capacity(domain.len() + 1 + bytes.len());
    input.extend_from_slice(domain);
    input.push(0);
    input.extend_from_slice(bytes);
    ContentHash::keyed(key, &input)
}

/// Track `intent` in `tracker`: arguments admitted in a tainted turn carry
/// the turn's taint.
pub fn track_intent(tracker: &TaintTracker, intent: &ProvenanceIntent) {
    if intent.taint > CamelTaintLevel::Trusted {
        tracker.mark_tainted(
            intent.call.args_digest,
            TaintReason::custom("tool_arguments", intent.call.tool.clone()),
            intent.taint,
        );
    }
}

/// Track `outcome` in `tracker`: a result inherits its arguments' taint and
/// carries the turn's taint after the call.
pub fn track_outcome(tracker: &TaintTracker, outcome: &ProvenanceOutcome) {
    let Some(result) = outcome.result_digest else {
        return;
    };
    tracker.propagate(&[outcome.call.args_digest], result);
    if outcome.taint > CamelTaintLevel::Trusted {
        tracker.mark_tainted(
            result,
            TaintReason::custom("tool_result", outcome.call.tool.clone()),
            outcome.taint,
        );
    }
}

/// A [`SafetyProvenanceSink`] that keeps its records, and the taint they
/// carry, in memory in the order they were written. It is deterministic, and
/// it can refuse intents to show a call failing closed.
#[derive(Default)]
pub struct MemoryProvenanceSink {
    key: [u8; 32],
    records: Mutex<Vec<ProvenanceRecord>>,
    taint: TaintTracker,
    refuse_intents: AtomicBool,
}

impl std::fmt::Debug for MemoryProvenanceSink {
    // The digest key stays out of debug output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryProvenanceSink")
            .field("records", &self.records.lock().len())
            .field("refuse_intents", &self.refuse_intents)
            .finish_non_exhaustive()
    }
}

impl MemoryProvenanceSink {
    /// A sink whose digests are keyed with `key`.
    #[must_use]
    pub fn with_key(key: [u8; 32]) -> Self {
        Self {
            key,
            ..Self::default()
        }
    }

    /// Refuse every later intent, or accept them again.
    pub fn refuse_intents(&self, refuse: bool) {
        self.refuse_intents.store(refuse, Ordering::Release);
    }

    /// The records so far, oldest first.
    #[must_use]
    pub fn records(&self) -> Vec<ProvenanceRecord> {
        self.records.lock().clone()
    }

    /// The taint the records so far carry.
    #[must_use]
    pub const fn taint(&self) -> &TaintTracker {
        &self.taint
    }
}

impl SafetyProvenanceSink for MemoryProvenanceSink {
    fn digest_key(&self) -> [u8; 32] {
        self.key
    }

    fn record_intent(&self, intent: &ProvenanceIntent) -> Result<ProvenanceAck, ProvenanceError> {
        if self.refuse_intents.load(Ordering::Acquire) {
            return Err(ProvenanceError("the sink refuses intents".to_string()));
        }
        let ack = {
            let mut records = self.records.lock();
            records.push(ProvenanceRecord::Intent(intent.clone()));
            ProvenanceAck {
                record_id: format!("intent-{}", records.len()),
            }
        };
        track_intent(&self.taint, intent);
        Ok(ack)
    }

    fn record_outcome(&self, outcome: &ProvenanceOutcome) -> Result<(), ProvenanceError> {
        self.records
            .lock()
            .push(ProvenanceRecord::Outcome(outcome.clone()));
        track_outcome(&self.taint, outcome);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use roko_core::tool::{
        ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolHandler, ToolPermission,
        ToolRegistry, VecToolRegistry,
    };
    use serde_json::json;

    use super::*;
    use crate::dispatcher::{HandlerResolver, ToolDispatcher};

    /// A handler that notes how many provenance records existed when it ran.
    struct ProbeHandler {
        sink: Arc<MemoryProvenanceSink>,
        seen: Arc<StdMutex<Vec<usize>>>,
    }

    #[async_trait]
    impl ToolHandler for ProbeHandler {
        fn name(&self) -> &str {
            "probe"
        }

        async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            let records = self.sink.records().len();
            self.seen.lock().expect("seen lock").push(records);
            ToolResult::text(call.arguments.to_string())
        }
    }

    /// A dispatcher with the one tool `probe`, whose handler reports into
    /// `seen`, recording provenance with `sink`.
    fn probe_dispatcher(
        sink: &Arc<MemoryProvenanceSink>,
        seen: &Arc<StdMutex<Vec<usize>>>,
    ) -> ToolDispatcher {
        let def = ToolDef::new("probe", "x", ToolCategory::Meta, ToolPermission::read_only())
            .with_concurrency(ToolConcurrency::Serial);
        let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(vec![def]));
        let handler: Arc<dyn ToolHandler> = Arc::new(ProbeHandler {
            sink: Arc::clone(sink),
            seen: Arc::clone(seen),
        });
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(move |name: &str| (name == "probe").then(|| Arc::clone(&handler)));
        let provenance = Arc::clone(sink) as Arc<dyn SafetyProvenanceSink>;
        ToolDispatcher::new_unguarded(registry, resolver).with_provenance_sink(provenance)
    }

    #[test]
    fn arguments_digest_ignores_key_order_and_depends_on_the_key() {
        let one = json!({"a": 1, "b": "two"});
        let other_order: serde_json::Value =
            serde_json::from_str(r#"{"b": "two", "a": 1}"#).expect("json");
        assert_eq!(
            arguments_digest(&[3; 32], &one),
            arguments_digest(&[3; 32], &other_order)
        );
        assert_ne!(
            arguments_digest(&[3; 32], &one),
            arguments_digest(&[4; 32], &one)
        );
        let text = ToolResult::text(one.to_string());
        assert_ne!(
            arguments_digest(&[3; 32], &one),
            result_digest(&[3; 32], &text)
        );
    }

    #[tokio::test]
    async fn safety_provenance_intent_recorded_before_handler() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let ctx = ToolContext::testing(workspace.path());
        let sink = Arc::new(MemoryProvenanceSink::default());
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let dispatcher = probe_dispatcher(&sink, &seen);

        let call = ToolCall::new("call-1", "probe", json!({"path": "a.txt"}));
        let result = dispatcher.dispatch(call, &ctx).await;
        assert!(result.is_ok(), "{result:?}");
        // The intent was already recorded when the handler ran.
        assert_eq!(*seen.lock().expect("seen lock"), [1]);
        let records = sink.records();
        let [ProvenanceRecord::Intent(intent), ProvenanceRecord::Outcome(outcome)] =
            records.as_slice()
        else {
            panic!("expected an intent, then an outcome: {records:?}");
        };
        assert_eq!(intent.call.tool, "probe");
        assert_eq!(intent.call.call_id, "call-1");
        assert_eq!(outcome.intent.as_deref(), Some("intent-1"));
        assert_eq!(outcome.verdict, ProvenanceVerdict::Succeeded);
        assert!(outcome.result_digest.is_some());

        // Without an acknowledged intent the handler does not run.
        sink.refuse_intents(true);
        let call = ToolCall::new("call-2", "probe", json!({"path": "b.txt"}));
        let result = dispatcher.dispatch(call, &ctx).await;
        assert!(!result.is_ok(), "the call must fail closed");
        assert_eq!(seen.lock().expect("seen lock").len(), 1);
        let records = sink.records();
        let Some(ProvenanceRecord::Outcome(refused)) = records.last() else {
            panic!("expected the refusal's outcome: {records:?}");
        };
        assert_eq!(refused.verdict, ProvenanceVerdict::Denied);
        assert_eq!(refused.intent, None);
        assert_eq!(refused.reason.as_deref(), Some("provenance_intent_failed"));
        assert_eq!(refused.result_digest, None);
    }

    #[tokio::test]
    async fn safety_provenance_records_hold_no_arguments_or_output() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let ctx = ToolContext::testing(workspace.path());
        let sink = Arc::new(MemoryProvenanceSink::with_key([7; 32]));
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let dispatcher = probe_dispatcher(&sink, &seen);

        let arguments = json!({"note": "quokka-umbrella-7731"});
        let call = ToolCall::new("call-1", "probe", arguments.clone());
        let result = dispatcher.dispatch(call, &ctx).await;
        assert!(result.text_content().contains("quokka-umbrella-7731"));
        let records = sink.records();
        let encoded = serde_json::to_string(&records).expect("records serialize");
        assert!(!encoded.contains("quokka"), "{encoded}");
        let [ProvenanceRecord::Intent(intent), ProvenanceRecord::Outcome(outcome)] =
            records.as_slice()
        else {
            panic!("expected an intent, then an outcome: {records:?}");
        };
        assert_eq!(
            intent.call.args_digest,
            arguments_digest(&[7; 32], &arguments)
        );
        assert_eq!(
            outcome.result_digest,
            Some(result_digest(&[7; 32], &result))
        );
    }

    #[tokio::test]
    async fn safety_provenance_records_a_denied_call() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let mut ctx = ToolContext::testing(workspace.path());
        ctx.denied_tools = Some(vec!["probe".to_string()]);
        let sink = Arc::new(MemoryProvenanceSink::default());
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let dispatcher = probe_dispatcher(&sink, &seen);

        let call = ToolCall::new("call-1", "probe", json!({}));
        let result = dispatcher.dispatch(call, &ctx).await;
        assert!(!result.is_ok());
        assert!(seen.lock().expect("seen lock").is_empty());
        let records = sink.records();
        let [ProvenanceRecord::Outcome(denied)] = records.as_slice() else {
            panic!("expected one denial: {records:?}");
        };
        assert_eq!(denied.verdict, ProvenanceVerdict::Denied);
        assert_eq!(denied.intent, None);
        assert_eq!(denied.reason.as_deref(), Some("permission_denied"));
    }

    #[tokio::test]
    async fn safety_provenance_propagates_taint_to_the_result() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let ctx = ToolContext::testing(workspace.path());
        let ctx = ctx.with_taint_level(CamelTaintLevel::External);
        let sink = Arc::new(MemoryProvenanceSink::default());
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let dispatcher = probe_dispatcher(&sink, &seen);

        let call = ToolCall::new("call-1", "probe", json!({"url": "https://example.com"}));
        assert!(dispatcher.dispatch(call, &ctx).await.is_ok());
        let records = sink.records();
        let Some(ProvenanceRecord::Outcome(outcome)) = records.last() else {
            panic!("expected an outcome: {records:?}");
        };
        let args = outcome.call.args_digest;
        let result = outcome.result_digest.expect("the handler ran");
        let taint = sink.taint();
        assert_eq!(taint.get_level(&args), Some(CamelTaintLevel::External));
        assert_eq!(taint.get_level(&result), Some(CamelTaintLevel::External));
        assert_eq!(taint.derived_from(&result), [args]);
    }
}
