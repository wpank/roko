//! Registry mapping canonical tool names to [`ToolHandler`] instances.
//!
//! [`StaticToolRegistry`](super::StaticToolRegistry) describes **what**
//! the built-in tools look like; [`HandlerRegistry`] tells the
//! dispatcher **how** to execute them. All 16 canonical built-ins
//! (§36.b) are shipped, though a few are day-one stubs that return a
//! structured "not yet wired" error until their downstream
//! dependencies (HTTP client, sub-agent supervisor) land.
//!
//! # Chain feature gate
//!
//! When the `chain` cargo feature is enabled, the 17 chain-domain tools
//! (all `chain.*` names) also resolve to handlers here.  Those handlers
//! return a typed [`roko_core::tool::ToolError::Other`] error explaining
//! that the blockchain backend is a Phase 2+ dependency — the feature gate
//! makes the tools _advertised_ (they appear in the catalog) but the
//! execution path is honest about the missing backend.
//!
//! Without the `chain` feature (default), chain tools are absent from the
//! tool catalog entirely and `handler_for` returns `None` for all `chain.*`
//! names.

use std::sync::Arc;

use roko_core::tool::ToolHandler;

use super::builtin::{
    apply_patch, bash, edit_file, exit_plan_mode, glob, grep, isfr, ls, multi_edit, notebook_edit,
    read_file, run_tests, task_agent, todo_write, web_fetch, web_search, write_file,
};

// ─── Chain handler (feature-gated) ────────────────────────────────────────

/// Handler for chain-domain tools when the `chain` cargo feature is enabled.
///
/// The 17 `chain.*` tools require a live blockchain / alloy backend that is
/// a Phase 2+ dependency.  This handler exists so that `handler_for` returns
/// `Some(_)` for every advertised chain tool (ensuring advertised-handler
/// parity) while returning a clear, typed error that explains the missing
/// backend rather than silently succeeding or panicking.
#[cfg(feature = "chain")]
#[derive(Debug, Clone)]
struct ChainToolHandler {
    tool_name: &'static str,
}

#[cfg(feature = "chain")]
#[async_trait::async_trait]
impl ToolHandler for ChainToolHandler {
    fn name(&self) -> &str {
        self.tool_name
    }

    async fn execute(
        &self,
        _call: roko_core::tool::ToolCall,
        _ctx: &roko_core::tool::ToolContext,
    ) -> roko_core::tool::ToolResult {
        roko_core::tool::ToolResult::Err(roko_core::tool::ToolError::Other(format!(
            "chain tool `{}` is not available: the blockchain backend \
             (roko-chain with alloy) is a Phase 2+ dependency and is not wired \
             in this build.  Enable the `chain` cargo feature and provide a \
             live RPC endpoint via `roko.toml` [chain] to use this tool.",
            self.tool_name
        )))
    }
}

/// Build a chain tool handler for the given canonical tool name.
///
/// Looks the name up in the static `CHAIN_TOOL_NAMES` array so that the
/// stored reference is `'static` and `Arc<dyn ToolHandler>` is satisfied.
/// Returns `Some(handler)` only when compiled with `--features chain`.
#[cfg(feature = "chain")]
fn chain_handler_for(name: &str) -> Option<Arc<dyn ToolHandler>> {
    use roko_chain::tools::CHAIN_TOOL_NAMES;
    // Find the matching &'static str from the canonical array so
    // ChainToolHandler.tool_name gets a 'static lifetime.
    let static_name = CHAIN_TOOL_NAMES.iter().copied().find(|n| *n == name)?;
    Some(Arc::new(ChainToolHandler {
        tool_name: static_name,
    }))
}

// ──────────────────────────────────────────────────────────────────────────

/// Look up the concrete handler for a canonical tool `name`.
///
/// Every built-in from [`BUILTIN_TOOL_NAMES`](super::builtin::BUILTIN_TOOL_NAMES)
/// resolves to a handler; a few are day-one stubs that fail with
/// [`roko_core::tool::ToolError::Other`] describing the missing dep.
/// Returns `None` only for genuinely unknown names.
///
/// With the `chain` feature enabled, all `chain.*` names resolve to a
/// [`ChainToolHandler`] that returns a descriptive "Phase 2+ not wired" error.
/// Without the `chain` feature, `chain.*` names return `None` (and are not
/// advertised in the tool catalog, so no model will see them).
#[must_use]
pub fn handler_for(name: &str) -> Option<Arc<dyn ToolHandler>> {
    match name {
        n if n == read_file::NAME => Some(Arc::new(read_file::Handler)),
        n if n == write_file::NAME => Some(Arc::new(write_file::Handler)),
        n if n == edit_file::NAME => Some(Arc::new(edit_file::Handler)),
        n if n == multi_edit::NAME => Some(Arc::new(multi_edit::Handler)),
        n if n == glob::NAME => Some(Arc::new(glob::Handler)),
        n if n == ls::NAME => Some(Arc::new(ls::Handler)),
        n if n == grep::NAME => Some(Arc::new(grep::Handler)),
        n if n == bash::NAME => Some(Arc::new(bash::Handler)),
        n if n == run_tests::NAME => Some(Arc::new(run_tests::Handler)),
        n if n == apply_patch::NAME => Some(Arc::new(apply_patch::Handler)),
        n if n == notebook_edit::NAME => Some(Arc::new(notebook_edit::Handler)),
        n if n == todo_write::NAME => Some(Arc::new(todo_write::Handler)),
        n if n == exit_plan_mode::NAME => Some(Arc::new(exit_plan_mode::Handler)),
        n if n == web_fetch::NAME => Some(Arc::new(web_fetch::Handler)),
        n if n == web_search::NAME => Some(Arc::new(web_search::Handler)),
        n if n == task_agent::NAME => Some(Arc::new(task_agent::Handler)),
        // ISFR tools: handler ships with no keeper; returns a typed error when
        // keeper is absent rather than silently succeeding.
        n if isfr::ISFR_TOOL_NAMES.contains(&n) => Some(Arc::new(isfr::ISFRHandler::new())),
        #[cfg(feature = "chain")]
        other => chain_handler_for(other),
        #[cfg(not(feature = "chain"))]
        _ => None,
    }
}

/// Thin wrapper around [`handler_for`] for trait-object dispatch sites.
#[derive(Debug, Clone, Copy, Default)]
pub struct HandlerRegistry;

impl HandlerRegistry {
    /// Construct a new registry (zero-sized).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Look up a handler by canonical name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        handler_for(name)
    }

    /// Canonical names of handlers that ship unconditionally (16 std + 4 ISFR).
    ///
    /// Chain tool handlers are registered separately via `handler_for` when the
    /// `chain` cargo feature is enabled, but are not listed here so that callers
    /// testing the std handler set don't need to be feature-conditional.
    #[must_use]
    pub fn shipped_names(&self) -> &'static [&'static str] {
        const NAMES: &[&str] = &[
            read_file::NAME,
            write_file::NAME,
            edit_file::NAME,
            multi_edit::NAME,
            glob::NAME,
            grep::NAME,
            ls::NAME,
            bash::NAME,
            run_tests::NAME,
            apply_patch::NAME,
            notebook_edit::NAME,
            todo_write::NAME,
            exit_plan_mode::NAME,
            web_fetch::NAME,
            web_search::NAME,
            task_agent::NAME,
            isfr::ISFR_READ_RATES,
            isfr::ISFR_READ_RATE_HISTORY,
            isfr::ISFR_ORACLE_STATUS,
            isfr::ISFR_SOURCE_STATUS,
        ];
        NAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::builtin::BUILTIN_TOOL_NAMES;

    #[test]
    fn shipped_handlers_are_findable() {
        let reg = HandlerRegistry::new();
        for name in reg.shipped_names() {
            assert!(reg.get(name).is_some(), "expected handler for {name}");
        }
    }

    #[test]
    fn unknown_tool_returns_none() {
        let reg = HandlerRegistry::new();
        assert!(reg.get("not_a_real_tool").is_none());
        assert!(reg.get("").is_none());
    }

    #[test]
    fn handler_names_match_tool_defs() {
        use crate::tool::builtin::isfr::ISFR_TOOL_NAMES;
        let reg = HandlerRegistry::new();
        for name in reg.shipped_names() {
            let handler = reg.get(name).expect("handler present");
            if ISFR_TOOL_NAMES.contains(name) {
                // ISFRHandler dispatches multiple names via a single handler;
                // its canonical name() is "isfr.*". Confirm it resolves only.
                assert!(
                    handler.name().starts_with("isfr"),
                    "unexpected handler name `{}` for ISFR tool `{name}`",
                    handler.name()
                );
            } else {
                assert_eq!(handler.name(), *name);
            }
        }
    }

    #[test]
    fn every_std_builtin_tool_has_a_handler() {
        let reg = HandlerRegistry::new();
        // The 16 std handlers ship unconditionally.
        for name in reg.shipped_names() {
            assert!(
                reg.get(name).is_some(),
                "no handler registered for shipped tool `{name}`"
            );
        }
    }

    #[test]
    fn shipped_names_are_subset_of_builtin_names() {
        use std::collections::HashSet;
        let reg = HandlerRegistry::new();
        let shipped: HashSet<&str> = reg.shipped_names().iter().copied().collect();
        let builtin: HashSet<&str> = BUILTIN_TOOL_NAMES.iter().copied().collect();
        assert!(
            shipped.is_subset(&builtin),
            "shipped_names must be a subset of BUILTIN_TOOL_NAMES"
        );
    }

    #[test]
    fn all_20_handlers_ship() {
        // 16 std built-ins + 4 ISFR tools = 20 shipped handlers.
        assert_eq!(HandlerRegistry::new().shipped_names().len(), 20);
    }

    #[test]
    fn isfr_tools_resolve_to_handlers() {
        use crate::tool::builtin::isfr::ISFR_TOOL_NAMES;
        let reg = HandlerRegistry::new();
        for name in &ISFR_TOOL_NAMES {
            assert!(
                reg.get(name).is_some(),
                "handler_for returned None for ISFR tool `{name}`"
            );
        }
    }

    #[test]
    fn isfr_handler_name_sentinel() {
        // ISFRHandler is a multi-name handler; its canonical name() is "isfr.*".
        // Confirm all 4 ISFR tools resolve and their handler uses the isfr prefix.
        use crate::tool::builtin::isfr::ISFR_TOOL_NAMES;
        let reg = HandlerRegistry::new();
        for name in &ISFR_TOOL_NAMES {
            let handler = reg
                .get(name)
                .unwrap_or_else(|| panic!("no handler for ISFR tool `{name}`"));
            assert!(
                handler.name().starts_with("isfr"),
                "unexpected handler name `{}` for tool `{name}`",
                handler.name()
            );
        }
    }

    /// When the `chain` feature is enabled, every advertised chain tool must
    /// resolve to a handler so that the catalog and dispatch path agree.
    #[cfg(feature = "chain")]
    #[test]
    fn chain_tools_have_handlers_when_feature_enabled() {
        use roko_chain::tools::CHAIN_TOOL_NAMES;
        let reg = HandlerRegistry::new();
        for name in CHAIN_TOOL_NAMES {
            assert!(
                reg.get(name).is_some(),
                "no handler for chain tool `{name}` even though `chain` feature is enabled"
            );
        }
    }

    /// When the `chain` feature is disabled, `handler_for` must return `None`
    /// for chain tool names and those names must not appear in the catalog.
    #[cfg(not(feature = "chain"))]
    #[test]
    fn chain_tools_absent_when_feature_disabled() {
        use std::collections::HashSet;
        const CHAIN_NAMES: &[&str] = &[
            "chain.balance",
            "chain.transfer",
            "chain.approve",
            "chain.swap",
            "chain.add_liquidity",
            "chain.remove_liquidity",
            "chain.get_pool_info",
            "chain.get_position",
            "chain.simulate_tx",
            "chain.gas_estimate",
            "chain.wallet_create",
            "chain.wallet_list",
            "chain.wallet_info",
            "chain.wallet_export_address",
            "chain.post_insight",
            "chain.search_insights",
            "chain.confirm_insight",
        ];
        let reg = HandlerRegistry::new();
        let builtin: HashSet<&str> = BUILTIN_TOOL_NAMES.iter().copied().collect();
        for name in CHAIN_NAMES {
            assert!(
                reg.get(name).is_none(),
                "handler_for(`{name}`) must return None when chain feature is disabled"
            );
            assert!(
                !builtin.contains(name),
                "chain tool `{name}` must not appear in BUILTIN_TOOL_NAMES when chain feature is disabled"
            );
        }
    }
}
