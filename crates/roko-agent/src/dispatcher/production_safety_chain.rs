//! Frozen production safety chain with nine mandatory stages (#350).
//!
//! Every production [`ToolDispatcher`](super::ToolDispatcher) must hold a
//! non-optional `ProductionSafetyChain`. The chain enforces a fixed logical
//! order where each enforcement behavior has exactly **one** owner:
//!
//! | Stage | Owner | Short-circuits? |
//! |-------|-------|-----------------|
//! | 1. Sandbox / role+task allow-deny | `SafetyLayer` inline | Yes |
//! | 2. Tool-definition / MCP tier     | `SafetyLayer` inline | Yes |
//! | 3. Rate / budget check            | `SafetyLayer` inline | Yes |
//! | 4. Bash/git/network/path/contract | `SafetyLayer` inline | Yes |
//! | 5. Known-tool / parameter sanity  | `HallucinationDetector` hook | Yes |
//! | 6. Taint ceiling                  | `TaintLevelHook` hook | Yes |
//! | 7. Five-head corrigibility        | `CorrigibilityHook` hook | Yes |
//! | 8. Handler execution              | Dispatcher | N/A |
//! | 9. Output filter + final scrub    | `ResultFilter::sanitize` | No |
//!
//! Stages 1-4 are owned by the synchronous `SafetyLayer::check_pre_execution_with_def`
//! path and are **not** duplicated as hooks. Stages 5-7 are hooks in the
//! production chain. Stage 8 is the handler. Stage 9 runs after the handler
//! via [`ProductionSafetyChain::filter_result`].
//!
//! Denial at any pre-handler stage (1-7) short-circuits before the handler
//! executes. Post-handler filtering (stage 9) runs even when the handler
//! returns an error payload.
//!
//! Configured [`AgentWarrant`] and [`TemporalMonitor`] are preserved and
//! enforced when present, but this module does **not** synthesize either.
//! A separate follow-up must define issuer/expiry and temporal property
//! semantics before they become mandatory defaults.

use std::sync::Arc;

use roko_core::extension::CamelTaintLevel;
use roko_core::tool::{ToolContext, ToolDef, ToolError, ToolRegistry};

use crate::safety::hooks::{HookDecision, SafetyAuditRecord};
use crate::safety::result_filter::ResultFilter;
use crate::safety::{CorrigibilityHook, HallucinationDetector, SafetyLayer, TaintLevelHook};

use super::hook_chain::SafetyHookChain;

/// Stable stage identifiers for denial audit events (#136).
///
/// These are constant strings embedded in denial records so that downstream
/// consumers can match on stage identity without parsing human-readable text.
pub mod stage_id {
    pub const SANDBOX_ROLE_ALLOW_DENY: &str = "stage:1:sandbox_role";
    pub const TOOL_DEFINITION_MCP_TIER: &str = "stage:2:tool_def_tier";
    pub const RATE_BUDGET: &str = "stage:3:rate_budget";
    pub const BASH_GIT_NETWORK_PATH: &str = "stage:4:policy_checks";
    pub const KNOWN_TOOL_SANITY: &str = "stage:5:known_tool_sanity";
    pub const TAINT_CEILING: &str = "stage:6:taint_ceiling";
    pub const CORRIGIBILITY: &str = "stage:7:corrigibility";
    pub const RESULT_FILTER: &str = "stage:9:result_filter";
}

/// Non-optional production safety chain that every production dispatcher must hold.
///
/// The chain cannot be constructed with an empty default: the builder requires
/// a resolved [`SafetyLayer`] and tool registry to derive the known-tool set
/// for the `HallucinationDetector`.
#[derive(Debug)]
pub struct ProductionSafetyChain {
    /// The pre-handler hook chain (stages 5-7).
    pre_handler_hooks: SafetyHookChain,
    /// Post-handler result filter (stage 9).
    result_filter: ResultFilter,
    /// The taint ceiling applied to the taint-level hook.
    max_taint_level: CamelTaintLevel,
}

impl ProductionSafetyChain {
    /// Build a chain from resolved context.
    ///
    /// # Arguments
    ///
    /// * `safety` - The resolved `SafetyLayer` supplying contract, warrant, etc.
    /// * `registry` - The tool registry used to derive the known-tool set for
    ///   the hallucination detector.
    pub fn build(safety: &SafetyLayer, registry: &dyn ToolRegistry) -> Self {
        let known_tools: Vec<String> = registry.all().iter().map(|t| t.name.clone()).collect();
        Self::build_with_known_tools(safety, known_tools)
    }

    /// Build a chain with an explicit list of known tool names.
    ///
    /// Useful when the registry is not available (e.g. tests) or when the
    /// caller wants to supply a curated list.
    pub fn build_with_known_tools(safety: &SafetyLayer, known_tools: Vec<String>) -> Self {
        let mut pre_handler_hooks = SafetyHookChain::new();

        // Stage 5: known-tool / parameter sanity (HallucinationDetector).
        pre_handler_hooks.push(
            stage_id::KNOWN_TOOL_SANITY,
            Arc::new(HallucinationDetector::with_known_tools(known_tools)),
        );

        // Stage 6: taint ceiling (TaintLevelHook).
        pre_handler_hooks.push(
            stage_id::TAINT_CEILING,
            Arc::new(TaintLevelHook::new(safety.contract.max_taint_level)),
        );

        // Stage 7: five-head corrigibility (CorrigibilityHook).
        pre_handler_hooks.push(stage_id::CORRIGIBILITY, Arc::new(CorrigibilityHook));

        Self {
            pre_handler_hooks,
            result_filter: ResultFilter::with_defaults(),
            max_taint_level: safety.contract.max_taint_level,
        }
    }

    /// Evaluate the pre-handler hook stages (5-7).
    ///
    /// Stages 1-4 are evaluated by `SafetyLayer::check_pre_execution_with_def`
    /// before this method is called. Returns the (possibly modified) parameters
    /// and audit trail on success, or the rejection error and partial audit trail
    /// on failure.
    pub async fn evaluate_pre_handler(
        &self,
        tool: &ToolDef,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<(serde_json::Value, Vec<SafetyAuditRecord>), (ToolError, Vec<SafetyAuditRecord>)>
    {
        self.pre_handler_hooks.evaluate(tool, params, ctx).await
    }

    /// Apply post-handler output filtering (stage 9).
    ///
    /// Runs `ResultFilter::sanitize` for size/source annotation, followed by
    /// the deep `SafetyLayer` scrub which is the final secret scrub. This
    /// method runs on **both** success and error payloads.
    #[must_use]
    pub fn filter_result(&self, content: &str, tool_name: &str) -> String {
        self.result_filter.sanitize(content, tool_name)
    }

    /// Access the underlying pre-handler hook chain (for audit/inspection).
    #[must_use]
    pub const fn pre_handler_hooks(&self) -> &SafetyHookChain {
        &self.pre_handler_hooks
    }

    /// Number of hooks in the pre-handler chain (stages 5-7).
    #[must_use]
    pub fn pre_handler_hook_count(&self) -> usize {
        self.pre_handler_hooks.len()
    }

    /// The taint ceiling configured for stage 6.
    #[must_use]
    pub const fn max_taint_level(&self) -> CamelTaintLevel {
        self.max_taint_level
    }

    /// Access the result filter (for audit/inspection).
    #[must_use]
    pub const fn result_filter(&self) -> &ResultFilter {
        &self.result_filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::contract::AgentContract;
    use crate::safety::hooks::HookDecision;
    use roko_core::tool::{ToolCategory, ToolContext, ToolDef, ToolPermission, VecToolRegistry};

    fn test_ctx() -> ToolContext {
        ToolContext::testing("/tmp/worktree")
    }

    fn test_tool(name: &str) -> ToolDef {
        ToolDef::new(
            name,
            "test tool",
            ToolCategory::Read,
            ToolPermission::read_only(),
        )
    }

    fn write_tool(name: &str) -> ToolDef {
        ToolDef::new(
            name,
            "write tool",
            ToolCategory::Write,
            ToolPermission::writes(),
        )
    }

    fn make_registry(tools: Vec<ToolDef>) -> Arc<dyn ToolRegistry> {
        Arc::new(VecToolRegistry::from_tools(tools))
    }

    // ── Construction ─────────────────────────────────────────────────────

    #[test]
    fn chain_has_exactly_three_pre_handler_hooks() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file"), write_tool("write_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());
        assert_eq!(
            chain.pre_handler_hook_count(),
            3,
            "stages 5 (hallucination), 6 (taint), 7 (corrigibility)"
        );
    }

    #[test]
    fn chain_has_frozen_stage_order() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());
        let hooks = chain.pre_handler_hooks();
        let names: Vec<&str> = hooks.hook_names().collect();
        assert_eq!(
            names,
            vec![
                stage_id::KNOWN_TOOL_SANITY,
                stage_id::TAINT_CEILING,
                stage_id::CORRIGIBILITY,
            ]
        );
    }

    // ── Stage 5: known-tool / parameter sanity ───────────────────────────

    #[tokio::test]
    async fn stage5_rejects_unknown_tool() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let unknown = test_tool("hallucinated_tool");
        let result = chain
            .evaluate_pre_handler(&unknown, serde_json::json!({}), &test_ctx())
            .await;
        assert!(result.is_err());
        let (err, audit) = result.unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(msg) if msg.contains(stage_id::KNOWN_TOOL_SANITY)));
        // Short-circuited at stage 5 — only one audit record.
        assert_eq!(audit.len(), 1);
    }

    #[tokio::test]
    async fn stage5_rejects_null_bytes_in_path() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("read_file");
        let result = chain
            .evaluate_pre_handler(
                &tool,
                serde_json::json!({"file_path": "/tmp/\0bad"}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stage5_rejects_negative_offset() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("read_file");
        let result = chain
            .evaluate_pre_handler(
                &tool,
                serde_json::json!({"file_path": "/tmp/test.rs", "offset": -1}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_err());
    }

    // ── Stage 6: taint ceiling ───────────────────────────────────────────

    #[tokio::test]
    async fn stage6_rejects_untrusted_taint_on_privileged_tool() {
        let mut contract = AgentContract::permissive("writer");
        contract.max_taint_level = CamelTaintLevel::External;
        let safety = SafetyLayer::with_defaults().with_contract(contract);
        let registry = make_registry(vec![write_tool("write_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = write_tool("write_file");
        let ctx = ToolContext::testing("/tmp").with_taint_level(CamelTaintLevel::Untrusted);
        let result = chain
            .evaluate_pre_handler(&tool, serde_json::json!({}), &ctx)
            .await;
        assert!(result.is_err());
        let (_, audit) = result.unwrap_err();
        // Stage 5 passed (known tool), stage 6 rejected.
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].hook_name, stage_id::KNOWN_TOOL_SANITY);
        assert!(matches!(audit[0].decision, HookDecision::Allow));
        assert_eq!(audit[1].hook_name, stage_id::TAINT_CEILING);
        assert!(matches!(audit[1].decision, HookDecision::Reject(_)));
    }

    #[tokio::test]
    async fn stage6_allows_read_tool_under_untrusted_taint() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("read_file"); // read-only, no privileged bits
        let ctx = ToolContext::testing("/tmp").with_taint_level(CamelTaintLevel::Untrusted);
        let result = chain
            .evaluate_pre_handler(&tool, serde_json::json!({}), &ctx)
            .await;
        assert!(result.is_ok());
    }

    // ── Stage 7: corrigibility ───────────────────────────────────────────

    #[tokio::test]
    async fn stage7_rejects_audit_disable() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let exec_tool = ToolDef::new(
            "bash",
            "shell",
            ToolCategory::Exec,
            ToolPermission::executes(),
        );
        let registry = make_registry(vec![exec_tool.clone()]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let result = chain
            .evaluate_pre_handler(
                &exec_tool,
                serde_json::json!({"command": "disable audit logging"}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_err());
        let (_, audit) = result.unwrap_err();
        // Stages 5 and 6 passed, stage 7 rejected.
        assert_eq!(audit.len(), 3);
        assert_eq!(audit[2].hook_name, stage_id::CORRIGIBILITY);
        assert!(matches!(audit[2].decision, HookDecision::Reject(_)));
    }

    // ── Stage 9: result filtering ────────────────────────────────────────

    #[test]
    fn stage9_truncates_oversized_output() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let big = "x".repeat(200_000);
        let filtered = chain.filter_result(&big, "read_file");
        assert!(filtered.len() < 200_000);
        assert!(filtered.contains("[OUTPUT TRUNCATED"));
    }

    #[test]
    fn stage9_scrubs_secrets_from_output() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let api_key = format!("sk-ant-api03-{}", "A".repeat(80));
        let content = format!("found key: {api_key}");
        let filtered = chain.filter_result(&content, "read_file");
        assert!(!filtered.contains(&api_key));
        assert!(filtered.contains("[REDACTED]"));
    }

    #[test]
    fn stage9_annotates_external_tool_output() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("web_fetch")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let filtered = chain.filter_result("response body", "web_fetch");
        assert!(filtered.starts_with("[external:web_fetch]"));
    }

    #[test]
    fn stage9_does_not_annotate_internal_tool_output() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let filtered = chain.filter_result("file content", "read_file");
        assert!(!filtered.starts_with("[external:"));
    }

    // ── All stages pass for valid calls ──────────────────────────────────

    #[tokio::test]
    async fn all_stages_pass_for_valid_known_tool() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("read_file");
        let result = chain
            .evaluate_pre_handler(
                &tool,
                serde_json::json!({"file_path": "/tmp/test.rs"}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_ok());
        let (_, audit) = result.unwrap();
        assert_eq!(audit.len(), 3, "all three hooks evaluated");
        assert!(audit.iter().all(|r| matches!(r.decision, HookDecision::Allow)));
    }

    // ── Denial audit records never contain raw arguments ─────────────────

    #[tokio::test]
    async fn denial_audit_uses_hash_not_raw_args() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("hallucinated_tool");
        let secret = "super-secret-value-12345";
        let result = chain
            .evaluate_pre_handler(
                &tool,
                serde_json::json!({"secret_param": secret}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_err());
        let (_, audit) = result.unwrap_err();
        for record in &audit {
            assert!(
                record.params_hash.starts_with("hash:"),
                "params should be hashed, not raw"
            );
            // The raw secret must not appear in any field.
            let serialized = serde_json::to_string(record).unwrap();
            assert!(
                !serialized.contains(secret),
                "raw arguments must not leak into denial audit: {serialized}"
            );
        }
    }

    // ── Short-circuit: handler not reached on deny ───────────────────────

    #[tokio::test]
    async fn short_circuit_at_stage5_skips_stages_6_and_7() {
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let tool = test_tool("nonexistent");
        let result = chain
            .evaluate_pre_handler(&tool, serde_json::json!({}), &test_ctx())
            .await;
        assert!(result.is_err());
        let (_, audit) = result.unwrap_err();
        assert_eq!(audit.len(), 1, "short-circuited at stage 5");
        assert_eq!(audit[0].hook_name, stage_id::KNOWN_TOOL_SANITY);
    }

    // ── Overlapping checks have single owner ─────────────────────────────

    #[test]
    fn taint_check_is_only_in_hook_chain_not_duplicated() {
        // The TaintLevelHook in the production chain is the sole owner of
        // taint enforcement in the hook path. The SafetyLayer's inline
        // check_pre_execution_with_def also checks taint, but it runs in
        // stage 4 (inline) and is the sole owner there. Together they
        // enforce taint exactly once at the hook boundary and once at the
        // inline boundary, with no overlap.
        let safety = SafetyLayer::with_defaults().with_role("implementer");
        let registry = make_registry(vec![test_tool("read_file")]);
        let chain = ProductionSafetyChain::build(&safety, registry.as_ref());

        let taint_hooks: Vec<_> = chain
            .pre_handler_hooks()
            .hook_names()
            .filter(|n| n.contains("taint"))
            .collect();
        assert_eq!(
            taint_hooks.len(),
            1,
            "exactly one taint hook in the chain"
        );
    }
}
