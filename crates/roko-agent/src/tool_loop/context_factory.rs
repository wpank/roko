//! `ToolExecutionContextFactory` — production [`ToolContext`] builder for
//! non-test code paths.
//!
//! Before this factory existed, non-test code paths either used
//! `ToolContext::testing` (which uses noop sinks and `NeverCancel`) or manually
//! threaded all seven parameters.  `ToolExecutionContextFactory` closes that gap
//! by holding the shared production sinks once and stamping out per-call
//! contexts with real audit/trace/metrics/cancel wiring.
//!
//! ## Typical usage
//!
//! ```rust,ignore
//! let factory = ToolExecutionContextFactory::new(worktree)
//!     .with_audit_sink(real_audit)
//!     .with_trace_sink(real_trace)
//!     .with_metrics_sink(real_metrics)
//!     .with_cancel_token(cancel)
//!     .with_capabilities(role_caps)
//!     .with_correlation(envelope);
//!
//! // Each call to `build()` produces a fresh ToolContext.
//! let ctx = factory.build();
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_core::extension::CamelTaintLevel;
use roko_core::tool::{
    AuditSink, CancelToken, CorrelationEnvelope, MetricsSink, NeverCancel, NoopAuditSink,
    NoopMetricsSink, NoopTraceSink, ToolContext, ToolPermission, TraceSink,
};

/// Reusable factory that stamps out production [`ToolContext`] instances
/// from shared sinks, cancel tokens, and capability configurations.
///
/// Unlike `ToolContext::testing` (which hard-codes noop sinks and full local
/// capabilities), the factory carries real sinks and per-role capabilities so
/// non-test code paths get a properly wired context on every call.
#[derive(Clone)]
pub struct ToolExecutionContextFactory {
    worktree_path: PathBuf,
    immune_root_path: Option<PathBuf>,
    timeout: Duration,
    max_timeout: Duration,
    capabilities: ToolPermission,
    audit_sink: Arc<dyn AuditSink>,
    trace_sink: Arc<dyn TraceSink>,
    metrics_sink: Arc<dyn MetricsSink>,
    cancel_token: Arc<dyn CancelToken>,
    correlation: CorrelationEnvelope,
    taint_level: CamelTaintLevel,
    env_passthrough: Vec<String>,
}

impl ToolExecutionContextFactory {
    /// Create a factory rooted at the given worktree.
    ///
    /// Starts with safe defaults: the default tool timeout (120 s, a call
    /// asking for its own up to 600 s: `[timeouts] tool_execution_ms` and
    /// `tool_execution_max_ms`), full local capabilities (no network), noop
    /// sinks, never-cancel token, `External` taint. Replace these with the
    /// `with_*` builders before the first `build()`.
    #[must_use]
    pub fn new(worktree_path: impl Into<PathBuf>) -> Self {
        Self {
            worktree_path: worktree_path.into(),
            immune_root_path: None,
            timeout: Duration::from_millis(roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS),
            max_timeout: Duration::from_millis(
                roko_core::config::timeouts::DEFAULT_TOOL_EXECUTION_MAX_MS,
            ),
            capabilities: ToolPermission {
                read: true,
                write: true,
                exec: true,
                git: true,
                network: false,
            },
            audit_sink: Arc::new(NoopAuditSink),
            trace_sink: Arc::new(NoopTraceSink),
            metrics_sink: Arc::new(NoopMetricsSink),
            cancel_token: Arc::new(NeverCancel),
            correlation: CorrelationEnvelope::empty(),
            taint_level: CamelTaintLevel::External,
            env_passthrough: Vec::new(),
        }
    }

    /// Override the canonical root for durable immune controls and evidence.
    #[must_use]
    pub fn with_immune_root(mut self, immune_root: impl Into<PathBuf>) -> Self {
        let root = immune_root.into();
        self.immune_root_path = Some(root.canonicalize().unwrap_or(root));
        self
    }

    /// Override the tool-call timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Let a call that asks for its own budget get up to `max_timeout`
    /// (bug-c22bd9).
    #[must_use]
    pub const fn with_max_timeout(mut self, max_timeout: Duration) -> Self {
        self.max_timeout = max_timeout;
        self
    }

    /// Override the capability flags granted to tool handlers.
    #[must_use]
    pub const fn with_capabilities(mut self, capabilities: ToolPermission) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Attach a real audit sink.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit_sink = sink;
        self
    }

    /// Attach a real trace sink.
    #[must_use]
    pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
        self.trace_sink = sink;
        self
    }

    /// Attach a real metrics sink.
    #[must_use]
    pub fn with_metrics_sink(mut self, sink: Arc<dyn MetricsSink>) -> Self {
        self.metrics_sink = sink;
        self
    }

    /// Wire the active agent cancellation token.
    #[must_use]
    pub fn with_cancel_token(mut self, token: Arc<dyn CancelToken>) -> Self {
        self.cancel_token = token;
        self
    }

    /// Attach correlation metadata for trace/audit joining.
    #[must_use]
    pub fn with_correlation(mut self, correlation: CorrelationEnvelope) -> Self {
        self.correlation = correlation;
        self
    }

    /// Set the initial trust-origin taint level.
    #[must_use]
    pub const fn with_taint_level(mut self, taint_level: CamelTaintLevel) -> Self {
        self.taint_level = taint_level;
        self
    }

    /// Let the commands tool calls run (`bash`, `run_tests`) inherit the
    /// variables matching `patterns` (`[agent] env_passthrough`) besides
    /// what the gate policy admits.
    #[must_use]
    pub fn with_env_passthrough(mut self, patterns: Vec<String>) -> Self {
        self.env_passthrough = patterns;
        self
    }

    /// Stamp out a fresh production [`ToolContext`] using the factory's
    /// configured sinks, cancel token, capabilities, and correlation data.
    #[must_use]
    pub fn build(&self) -> ToolContext {
        ToolContext::production(
            &self.worktree_path,
            self.timeout,
            self.capabilities,
            Arc::clone(&self.audit_sink),
            Arc::clone(&self.trace_sink),
            Arc::clone(&self.metrics_sink),
            Arc::clone(&self.cancel_token),
            self.correlation.clone(),
        )
        .with_immune_root(
            self.immune_root_path
                .as_deref()
                .unwrap_or(&self.worktree_path),
        )
        .with_taint_level(self.taint_level)
        .with_env_passthrough(self.env_passthrough.clone())
        .with_max_timeout(self.max_timeout)
    }
}

impl std::fmt::Debug for ToolExecutionContextFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolExecutionContextFactory")
            .field("worktree_path", &self.worktree_path)
            .field("immune_root_path", &self.immune_root_path)
            .field("timeout", &self.timeout)
            .field("max_timeout", &self.max_timeout)
            .field("capabilities", &self.capabilities)
            .field("taint_level", &self.taint_level)
            .field("correlation", &self.correlation)
            .field("env_passthrough", &self.env_passthrough)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn factory_defaults_produce_valid_context() {
        let factory = ToolExecutionContextFactory::new("/tmp/factory");
        let ctx = factory.build();
        assert_eq!(ctx.worktree(), Path::new("/tmp/factory"));
        assert_eq!(ctx.timeout, Duration::from_secs(120));
        assert_eq!(ctx.max_timeout, Duration::from_secs(600));
        assert!(ctx.capabilities.read);
        assert!(ctx.capabilities.write);
        assert!(ctx.capabilities.exec);
        assert!(ctx.capabilities.git);
        assert!(!ctx.capabilities.network);
        assert!(!ctx.is_cancelled());
        assert_eq!(ctx.taint_level(), CamelTaintLevel::External);
    }

    #[test]
    fn factory_overrides_carry_through() {
        let factory = ToolExecutionContextFactory::new("/tmp/overrides")
            .with_timeout(Duration::from_secs(5))
            .with_capabilities(ToolPermission::read_only())
            .with_taint_level(CamelTaintLevel::Trusted)
            .with_correlation(CorrelationEnvelope {
                run_id: "r1".into(),
                task_id: "t1".into(),
                ..Default::default()
            });

        let ctx = factory.build();
        assert_eq!(ctx.timeout, Duration::from_secs(5));
        assert!(ctx.capabilities.read);
        assert!(!ctx.capabilities.write);
        assert_eq!(ctx.taint_level(), CamelTaintLevel::Trusted);
        assert_eq!(ctx.correlation.run_id, "r1");
    }

    #[test]
    fn factory_build_is_repeatable() {
        let factory =
            ToolExecutionContextFactory::new("/tmp/repeat").with_timeout(Duration::from_secs(10));

        let ctx1 = factory.build();
        let ctx2 = factory.build();

        assert_eq!(ctx1.timeout, ctx2.timeout);
        assert_eq!(ctx1.worktree(), ctx2.worktree());
        assert_eq!(ctx1.capabilities, ctx2.capabilities);
    }

    #[test]
    fn factory_immune_root_is_distinct_from_worktree() {
        let factory = ToolExecutionContextFactory::new("/tmp/attempt-wt")
            .with_immune_root("/tmp/canonical-ws");

        let ctx = factory.build();
        assert_eq!(ctx.worktree(), Path::new("/tmp/attempt-wt"));
        assert_eq!(ctx.immune_root(), Path::new("/tmp/canonical-ws"));
    }

    #[test]
    fn factory_carries_env_passthrough_to_every_context() {
        let factory = ToolExecutionContextFactory::new("/tmp/passthrough")
            .with_env_passthrough(vec!["DATABASE_URL".to_string()]);

        assert_eq!(factory.build().env_passthrough, ["DATABASE_URL"]);
        assert_eq!(factory.build().env_passthrough, ["DATABASE_URL"]);
        let default = ToolExecutionContextFactory::new("/tmp/none").build();
        assert!(default.env_passthrough.is_empty());
    }

    #[test]
    fn factory_debug_does_not_panic() {
        let factory = ToolExecutionContextFactory::new("/tmp/debug");
        let s = format!("{factory:?}");
        assert!(s.contains("ToolExecutionContextFactory"));
    }

    /// The dispatcher and the real `bash` tool, as an agent's tool loop runs
    /// them for a role that may use bash: the default safety layer under a
    /// permissive contract.
    fn bash_dispatcher() -> crate::dispatcher::ToolDispatcher {
        let registry: Arc<dyn roko_core::tool::ToolRegistry> =
            Arc::new(roko_core::tool::VecToolRegistry::from_tools(vec![
                roko_std::tool::builtin::bash::tool_def(),
            ]));
        let resolver: Arc<dyn crate::dispatcher::HandlerResolver> = Arc::new(|name: &str| {
            (name == "bash").then(|| {
                Arc::new(roko_std::tool::builtin::bash::Handler)
                    as Arc<dyn roko_core::tool::ToolHandler>
            })
        });
        crate::dispatcher::ToolDispatcher::new(registry, resolver).with_safety(
            crate::safety::SafetyLayer::with_defaults().with_contract(
                crate::safety::contract::AgentContract::permissive("bug-c22bd9"),
            ),
        )
    }

    /// bug-c22bd9: an agent's bash call that asks for 90 s runs a 70 s
    /// command to its end. Every tool call used to be cut at 60 s, whatever it
    /// asked for, so an agent never saw a JUCE plugin build finish.
    #[tokio::test]
    async fn a_bash_call_runs_past_sixty_seconds_when_asked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = ToolExecutionContextFactory::new(dir.path()).build();
        let call = roko_core::tool::ToolCall::new(
            "c1",
            "bash",
            serde_json::json!({"command": "sleep 70 && echo built", "timeout_ms": 90_000}),
        );
        let started = std::time::Instant::now();

        let result = bash_dispatcher().dispatch(call, &ctx).await;

        assert!(started.elapsed() >= Duration::from_secs(70), "{result:?}");
        assert!(result.text_content().contains("built"), "{result:?}");
    }

    /// bug-c22bd9: a call that asks for more than `[timeouts]
    /// tool_execution_max_ms` runs until the cap and is told so; one that asks
    /// for nothing gets `tool_execution_ms`.
    #[tokio::test]
    async fn a_bash_call_asking_past_the_cap_is_cut_at_it_and_told() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = ToolExecutionContextFactory::new(dir.path())
            .with_timeout(Duration::from_secs(1))
            .with_max_timeout(Duration::from_secs(2))
            .build();
        let asked = roko_core::tool::ToolCall::new(
            "c1",
            "bash",
            serde_json::json!({"command": "sleep 30", "timeout_ms": 600_000}),
        );
        let started = std::time::Instant::now();

        let result = bash_dispatcher().dispatch(asked, &ctx).await;

        assert!(started.elapsed() < Duration::from_secs(10), "{result:?}");
        match result {
            roko_core::tool::ToolResult::Err(roko_core::tool::ToolError::Other(message)) => {
                assert!(message.contains("asked for 600000 ms"), "{message}");
                assert!(message.contains("tool_execution_max_ms"), "{message}");
            }
            other => panic!("expected the cap's timeout, got {other:?}"),
        }
        let plain = roko_core::tool::ToolCall::new(
            "c2",
            "bash",
            serde_json::json!({"command": "sleep 30"}),
        );
        let result = bash_dispatcher().dispatch(plain, &ctx).await;
        assert!(
            matches!(
                result,
                roko_core::tool::ToolResult::Err(roko_core::tool::ToolError::Timeout { .. })
            ),
            "{result:?}"
        );
    }
}
