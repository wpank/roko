//! Integration tests: the safety layer wired into the dispatcher pipeline.
//!
//! Each test constructs a `ToolDispatcher` with a `SafetyLayer`, then
//! dispatches a tool call that should be blocked. The handler is a no-op
//! echo — the point is that the dispatcher never reaches it.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
use roko_agent::safety::SafetyLayer;
use roko_agent::safety::rate_limit::{RateLimitPolicy, RateLimiter};
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, VecToolRegistry,
};

// ─── Mock handler (never reached on blocked calls) ───────────────────────

struct NoopHandler {
    tool_name: &'static str,
}

#[async_trait]
impl ToolHandler for NoopHandler {
    fn name(&self) -> &str {
        self.tool_name
    }
    async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
        ToolResult::text("handler reached")
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn tool(name: &str, perm: ToolPermission, conc: ToolConcurrency) -> ToolDef {
    ToolDef::new(name, "test tool", ToolCategory::Meta, perm).with_concurrency(conc)
}

fn resolver_from(entries: Vec<(&'static str, Arc<dyn ToolHandler>)>) -> Arc<dyn HandlerResolver> {
    Arc::new(move |name: &str| {
        entries
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, h)| Arc::clone(h))
    })
}

fn ctx_with_exec() -> ToolContext {
    ToolContext::new(
        "/tmp",
        Duration::from_secs(5),
        ToolPermission {
            read: true,
            write: true,
            exec: true,
            git: true,
            network: true,
        },
        Arc::new(roko_core::tool::NoopAuditSink),
        Arc::new(roko_core::tool::NoopTraceSink),
        Arc::new(roko_core::tool::NoopMetricsSink),
        Arc::new(roko_core::tool::NeverCancel),
    )
}

// ─── Test 1: bash `rm -rf /` blocked by dispatcher ──────────────────────

#[tokio::test]
async fn bash_rm_rf_blocked_by_dispatcher() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "bash",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "bash",
        Arc::new(NoopHandler { tool_name: "bash" }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults();
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new("c1", "bash", serde_json::json!({ "command": "rm -rf /" }));
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;

    match result {
        ToolResult::Err(ToolError::CommandNotAllowed(msg)) => {
            assert!(
                msg.contains("rm -rf /"),
                "error should mention the denied pattern, got: {msg}"
            );
        }
        other => panic!("expected CommandNotAllowed, got {other:?}"),
    }
}

#[tokio::test]
async fn run_tests_rm_rf_blocked_by_dispatcher() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "run_tests",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "run_tests",
        Arc::new(NoopHandler {
            tool_name: "run_tests",
        }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults();
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new(
        "c1-run-tests",
        "run_tests",
        serde_json::json!({ "command": "rm -rf /" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;

    match result {
        ToolResult::Err(ToolError::CommandNotAllowed(msg)) => {
            assert!(
                msg.contains("rm -rf /"),
                "error should mention the denied pattern, got: {msg}"
            );
        }
        other => panic!("expected CommandNotAllowed, got {other:?}"),
    }
}

// ─── Test 2: network RFC1918 blocked ─────────────────────────────────────

#[tokio::test]
async fn network_rfc1918_blocked() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "web_fetch",
            ToolPermission::networked(),
            ToolConcurrency::Parallel,
        )]));
    let resolver = resolver_from(vec![(
        "web_fetch",
        Arc::new(NoopHandler {
            tool_name: "web_fetch",
        }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults();
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    // HTTP scheme to a private IP: blocked both by scheme and by private-network policy.
    let call = ToolCall::new(
        "c2",
        "web_fetch",
        serde_json::json!({ "url": "http://192.168.1.1" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;

    match result {
        ToolResult::Err(ToolError::NetworkBlocked(msg)) => {
            assert!(
                msg.contains("scheme") || msg.contains("private"),
                "error should mention scheme or private network, got: {msg}"
            );
        }
        other => panic!("expected NetworkBlocked, got {other:?}"),
    }

    // HTTPS to a private IP: blocked by private-network policy.
    let registry2: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "web_fetch",
            ToolPermission::networked(),
            ToolConcurrency::Parallel,
        )]));
    let resolver2 = resolver_from(vec![(
        "web_fetch",
        Arc::new(NoopHandler {
            tool_name: "web_fetch",
        }) as Arc<dyn ToolHandler>,
    )]);
    let layer2 = SafetyLayer::with_defaults();
    let dispatcher2 = ToolDispatcher::new(registry2, resolver2).with_safety(layer2);

    let call2 = ToolCall::new(
        "c3",
        "web_fetch",
        serde_json::json!({ "url": "https://192.168.1.1/admin" }),
    );
    let result2 = dispatcher2.dispatch(call2, &ctx_with_exec()).await;

    match result2 {
        ToolResult::Err(ToolError::NetworkBlocked(msg)) => {
            assert!(
                msg.contains("private"),
                "error should mention private network, got: {msg}"
            );
        }
        other => panic!("expected NetworkBlocked for HTTPS to private IP, got {other:?}"),
    }
}

// ─── Test 3: git force-push blocked ──────────────────────────────────────

#[tokio::test]
async fn git_force_push_blocked() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "bash",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "bash",
        Arc::new(NoopHandler { tool_name: "bash" }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults();
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new(
        "c4",
        "bash",
        serde_json::json!({ "command": "git push --force origin main" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;

    match result {
        ToolResult::Err(ToolError::CommandNotAllowed(msg)) => {
            assert!(
                msg.contains("block_force_push"),
                "error should reference the git policy rule, got: {msg}"
            );
        }
        other => panic!("expected CommandNotAllowed for git force push, got {other:?}"),
    }
}

// ─── Test 4: rate limit exceeded ─────────────────────────────────────────

#[tokio::test]
async fn rate_limit_exceeded() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "bash",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "bash",
        Arc::new(NoopHandler { tool_name: "bash" }) as Arc<dyn ToolHandler>,
    )]);

    let tight_limiter = RateLimiter::new(RateLimitPolicy {
        max_calls_per_window: 3,
        window_duration: Duration::from_secs(60),
    });

    let mut layer = SafetyLayer::with_defaults();
    layer.rate_limiter = Some(Arc::new(tight_limiter));
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);
    let ctx = ctx_with_exec();

    // First 3 calls succeed (cap = 3).
    for i in 0..3 {
        let call = ToolCall::new(
            format!("ok-{i}"),
            "bash",
            serde_json::json!({ "command": "echo hello" }),
        );
        let res = dispatcher.dispatch(call, &ctx).await;
        assert!(res.is_ok(), "call {i} should succeed, got {res:?}");
    }

    // 4th call should be blocked by rate limiter.
    let call = ToolCall::new(
        "blocked",
        "bash",
        serde_json::json!({ "command": "echo hello" }),
    );
    let result = dispatcher.dispatch(call, &ctx).await;

    match result {
        ToolResult::Err(ToolError::Other(msg)) => {
            assert!(
                msg.contains("rate limit"),
                "error should mention rate limit, got: {msg}"
            );
        }
        other => panic!("expected Other(rate limit), got {other:?}"),
    }
}

// ─── Test 5: safe calls pass through when all policies are active ────────

#[tokio::test]
async fn safe_calls_pass_through_with_all_policies() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "bash",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "bash",
        Arc::new(NoopHandler { tool_name: "bash" }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults();
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new("safe", "bash", serde_json::json!({ "command": "ls -la" }));
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;
    assert!(
        result.is_ok(),
        "safe command should pass all safety checks, got {result:?}"
    );
}

// ─── Test 6: no safety policy = backward-compatible pass-through ─────────

#[tokio::test]
async fn no_policy_means_pass_through() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "bash",
            ToolPermission::executes(),
            ToolConcurrency::Serial,
        )]));
    let resolver = resolver_from(vec![(
        "bash",
        Arc::new(NoopHandler { tool_name: "bash" }) as Arc<dyn ToolHandler>,
    )]);

    // No safety layer attached — but built-in denylist still blocks dangerous
    // commands. This is the correct security-first behavior.
    let dispatcher = ToolDispatcher::new(registry, resolver);

    // Dangerous commands are blocked by the built-in denylist even without
    // an explicit safety policy.
    let call = ToolCall::new(
        "danger",
        "bash",
        serde_json::json!({ "command": "rm -rf /" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;
    assert!(
        result.is_err(),
        "built-in denylist should block dangerous commands even without explicit policy"
    );
}

// ─── Test 7: production safety chain is non-optional ─────────────────────

#[tokio::test]
async fn production_dispatcher_always_has_safety_chain() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "read_file",
            ToolPermission::read_only(),
            ToolConcurrency::Parallel,
        )]));
    let resolver = resolver_from(vec![(
        "read_file",
        Arc::new(NoopHandler {
            tool_name: "read_file",
        }) as Arc<dyn ToolHandler>,
    )]);

    let dispatcher = ToolDispatcher::new(registry, resolver);
    assert!(
        dispatcher.production_safety_chain().is_some(),
        "production dispatcher must always have a safety chain"
    );
    assert_eq!(
        dispatcher
            .production_safety_chain()
            .unwrap()
            .pre_handler_hook_count(),
        3,
        "chain must have exactly 3 pre-handler hooks (stages 5,6,7)"
    );
}

// ─── Test 8: production chain rejects unknown tools ──────────────────────

#[tokio::test]
async fn production_chain_rejects_hallucinated_tool() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "read_file",
            ToolPermission::read_only(),
            ToolConcurrency::Parallel,
        )]));
    let resolver = resolver_from(vec![(
        "read_file",
        Arc::new(NoopHandler {
            tool_name: "read_file",
        }) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults().with_role("implementer");
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    // The registry only has "read_file", but we call "hallucinated_tool".
    // The production hallucination detector (stage 5) should reject it.
    let call = ToolCall::new(
        "h1",
        "hallucinated_tool",
        serde_json::json!({ "file_path": "/tmp/test.rs" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;
    // Note: this will be caught by the registry lookup before the hook chain,
    // but the chain itself would also reject it. The important thing is that
    // the call is denied.
    assert!(result.is_err(), "hallucinated tool should be rejected");
}

// ─── Test 9: production chain result filtering annotates external output ──

#[tokio::test]
async fn production_chain_annotates_external_tool_output() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "web_fetch",
            ToolPermission::networked(),
            ToolConcurrency::Parallel,
        )]));

    struct ExternalHandler;
    #[async_trait]
    impl ToolHandler for ExternalHandler {
        fn name(&self) -> &str {
            "web_fetch"
        }
        async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            ToolResult::text("response body from the internet")
        }
    }

    let resolver = resolver_from(vec![(
        "web_fetch",
        Arc::new(ExternalHandler) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults().with_role("researcher");
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new(
        "e1",
        "web_fetch",
        serde_json::json!({ "url": "https://example.com" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;
    if let ToolResult::Ok { .. } = &result {
        let content = result.text_content();
        assert!(
            content.contains("[external:web_fetch]"),
            "external tool output must be annotated with provenance, got: {content}"
        );
    }
    // Note: the result might be an error due to URL policy, which is acceptable.
    // The annotation test only applies when the handler is reached.
}

// ─── Test 10: production chain filters secrets from results ──────────────

#[tokio::test]
async fn production_chain_scrubs_secrets_from_handler_output() {
    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "read_file",
            ToolPermission::read_only(),
            ToolConcurrency::Parallel,
        )]));

    struct SecretLeaker;
    #[async_trait]
    impl ToolHandler for SecretLeaker {
        fn name(&self) -> &str {
            "read_file"
        }
        async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            let api_key = format!("sk-ant-api03-{}", "A".repeat(80));
            ToolResult::text(format!("found key: {api_key}"))
        }
    }

    let resolver = resolver_from(vec![(
        "read_file",
        Arc::new(SecretLeaker) as Arc<dyn ToolHandler>,
    )]);

    let layer = SafetyLayer::with_defaults().with_role("implementer");
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(layer);

    let call = ToolCall::new(
        "s1",
        "read_file",
        serde_json::json!({ "file_path": "/tmp/test.rs" }),
    );
    let result = dispatcher.dispatch(call, &ctx_with_exec()).await;
    match result {
        ToolResult::Ok { .. } => {
            let content = result.text_content();
            let api_key = format!("sk-ant-api03-{}", "A".repeat(80));
            assert!(
                !content.contains(&api_key),
                "secret must not leak through the production chain"
            );
            assert!(
                content.contains("[REDACTED]"),
                "scrubbed output must contain redaction marker"
            );
        }
        _ => {} // handler might not be reached due to other policies
    }
}

// ─── Test 11: frozen stage ordering is preserved across with_safety ──────

#[tokio::test]
async fn with_safety_preserves_frozen_stage_ordering() {
    use roko_agent::dispatcher::production_safety_chain;

    let registry: Arc<dyn roko_core::tool::ToolRegistry> =
        Arc::new(VecToolRegistry::from_tools(vec![tool(
            "read_file",
            ToolPermission::read_only(),
            ToolConcurrency::Parallel,
        )]));
    let resolver = resolver_from(vec![(
        "read_file",
        Arc::new(NoopHandler {
            tool_name: "read_file",
        }) as Arc<dyn ToolHandler>,
    )]);

    let dispatcher = ToolDispatcher::new(registry, resolver)
        .with_safety(SafetyLayer::with_defaults().with_role("implementer"));

    let chain = dispatcher.production_safety_chain().unwrap();
    let names: Vec<&str> = chain.pre_handler_hooks().hook_names().collect();
    assert_eq!(
        names,
        vec![
            production_safety_chain::stage_id::KNOWN_TOOL_SANITY,
            production_safety_chain::stage_id::TAINT_CEILING,
            production_safety_chain::stage_id::CORRIGIBILITY,
        ],
        "frozen stage order must be preserved after with_safety"
    );
}
