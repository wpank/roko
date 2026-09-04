//! Explicit capability state enum replacing boolean flags (T008).
//!
//! Each provider capability progresses through a lifecycle from
//! [`ProviderCapabilityState::Unavailable`] to [`ProviderCapabilityState::Active`].
//! This replaces scattered `supports_*: bool` flags with a single typed
//! state machine that captures *why* a capability is or isn't available.

use serde::{Deserialize, Serialize};

use crate::translate::capability::ModelCapabilities;

/// Lifecycle state of a single provider capability.
///
/// States form a progression: capabilities start as [`Unavailable`](Self::Unavailable)
/// and advance through configuration and negotiation to become [`Active`](Self::Active).
/// Runtime failures move them to [`Degraded`](Self::Degraded); policy blocks land on
/// [`Denied`](Self::Denied).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapabilityState {
    /// The underlying model/provider advertises the capability.
    Supported,
    /// The capability is configured in `roko.toml` / provider config.
    Configured,
    /// Runtime handshake confirmed the capability works (e.g. MCP tools/list succeeded).
    Negotiated,
    /// The capability is live and in use for the current session.
    Active,
    /// The capability was active but encountered runtime errors (retryable).
    Degraded,
    /// The model or provider does not support this capability at all.
    #[default]
    Unavailable,
    /// Policy (role contract, safety layer, allowlist) explicitly denies this capability.
    Denied,
}

impl ProviderCapabilityState {
    /// Whether the capability can currently be used for dispatch.
    #[must_use]
    pub const fn is_usable(self) -> bool {
        matches!(self, Self::Active | Self::Negotiated)
    }

    /// Whether the capability has been confirmed at any point (even if now degraded).
    #[must_use]
    pub const fn was_confirmed(self) -> bool {
        matches!(self, Self::Active | Self::Negotiated | Self::Degraded)
    }

    /// Stable string tag for logs and metrics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Configured => "configured",
            Self::Negotiated => "negotiated",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
            Self::Denied => "denied",
        }
    }
}

/// Typed capability states for a provider session.
///
/// Replaces the flat `supports_*: bool` fields in [`ModelCapabilities`]
/// with richer lifecycle tracking. Construct from a [`ModelCapabilities`]
/// snapshot via [`ProviderCapabilities::from_model_capabilities`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Native tool-calling support.
    pub tools: ProviderCapabilityState,
    /// Streaming response support.
    pub streaming: ProviderCapabilityState,
    /// Extended thinking / reasoning support.
    pub reasoning: ProviderCapabilityState,
    /// Vision (image input) support.
    pub vision: ProviderCapabilityState,
    /// Server-side code execution support.
    pub code_execution: ProviderCapabilityState,
    /// MCP tool integration support.
    pub mcp: ProviderCapabilityState,
    /// Subagent / multi-agent delegation support.
    pub subagents: ProviderCapabilityState,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            tools: ProviderCapabilityState::Unavailable,
            streaming: ProviderCapabilityState::Unavailable,
            reasoning: ProviderCapabilityState::Unavailable,
            vision: ProviderCapabilityState::Unavailable,
            code_execution: ProviderCapabilityState::Unavailable,
            mcp: ProviderCapabilityState::Unavailable,
            subagents: ProviderCapabilityState::Unavailable,
        }
    }
}

impl ProviderCapabilities {
    /// Project a [`ModelCapabilities`] snapshot into typed capability states.
    ///
    /// Boolean `supports_*` flags map to [`Supported`](ProviderCapabilityState::Supported)
    /// (true) or [`Unavailable`](ProviderCapabilityState::Unavailable) (false).
    /// Callers should advance the states (e.g. to `Configured` or `Active`)
    /// as the session progresses.
    #[must_use]
    pub fn from_model_capabilities(caps: &ModelCapabilities) -> Self {
        let bool_to_state = |supported: bool| {
            if supported {
                ProviderCapabilityState::Supported
            } else {
                ProviderCapabilityState::Unavailable
            }
        };
        Self {
            tools: bool_to_state(caps.supports_tools),
            streaming: bool_to_state(caps.supports_tool_streaming),
            reasoning: bool_to_state(caps.supports_thinking),
            vision: bool_to_state(caps.supports_vision),
            code_execution: ProviderCapabilityState::Unavailable,
            mcp: bool_to_state(caps.supports_mcp_tools),
            subagents: ProviderCapabilityState::Unavailable,
        }
    }

    /// Mark a capability as active.
    pub fn activate(&mut self, cap: CapabilityKind) {
        *self.state_mut(cap) = ProviderCapabilityState::Active;
    }

    /// Mark a capability as degraded.
    pub fn degrade(&mut self, cap: CapabilityKind) {
        *self.state_mut(cap) = ProviderCapabilityState::Degraded;
    }

    /// Mark a capability as denied by policy.
    pub fn deny(&mut self, cap: CapabilityKind) {
        *self.state_mut(cap) = ProviderCapabilityState::Denied;
    }

    /// Get the state of a capability by kind.
    #[must_use]
    pub fn state(&self, cap: CapabilityKind) -> ProviderCapabilityState {
        match cap {
            CapabilityKind::Tools => self.tools,
            CapabilityKind::Streaming => self.streaming,
            CapabilityKind::Reasoning => self.reasoning,
            CapabilityKind::Vision => self.vision,
            CapabilityKind::CodeExecution => self.code_execution,
            CapabilityKind::Mcp => self.mcp,
            CapabilityKind::Subagents => self.subagents,
        }
    }

    /// Get a mutable reference to a capability state.
    fn state_mut(&mut self, cap: CapabilityKind) -> &mut ProviderCapabilityState {
        match cap {
            CapabilityKind::Tools => &mut self.tools,
            CapabilityKind::Streaming => &mut self.streaming,
            CapabilityKind::Reasoning => &mut self.reasoning,
            CapabilityKind::Vision => &mut self.vision,
            CapabilityKind::CodeExecution => &mut self.code_execution,
            CapabilityKind::Mcp => &mut self.mcp,
            CapabilityKind::Subagents => &mut self.subagents,
        }
    }

    /// Returns the list of all capabilities that are currently usable.
    #[must_use]
    pub fn usable_capabilities(&self) -> Vec<CapabilityKind> {
        CapabilityKind::ALL
            .iter()
            .copied()
            .filter(|&kind| self.state(kind).is_usable())
            .collect()
    }

    /// Returns the list of all capabilities that have degraded.
    #[must_use]
    pub fn degraded_capabilities(&self) -> Vec<CapabilityKind> {
        CapabilityKind::ALL
            .iter()
            .copied()
            .filter(|&kind| self.state(kind) == ProviderCapabilityState::Degraded)
            .collect()
    }
}

/// Which capability slot to address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Tools,
    Streaming,
    Reasoning,
    Vision,
    CodeExecution,
    Mcp,
    Subagents,
}

impl CapabilityKind {
    /// All known capability kinds in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::Tools,
        Self::Streaming,
        Self::Reasoning,
        Self::Vision,
        Self::CodeExecution,
        Self::Mcp,
        Self::Subagents,
    ];

    /// Stable string for logs and metrics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Streaming => "streaming",
            Self::Reasoning => "reasoning",
            Self::Vision => "vision",
            Self::CodeExecution => "code_execution",
            Self::Mcp => "mcp",
            Self::Subagents => "subagents",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_capabilities_are_unavailable() {
        let caps = ProviderCapabilities::default();
        for kind in CapabilityKind::ALL {
            assert_eq!(caps.state(*kind), ProviderCapabilityState::Unavailable);
        }
    }

    #[test]
    fn from_model_capabilities_maps_booleans() {
        let model_caps = ModelCapabilities {
            supports_tools: true,
            supports_parallel_tool_calls: true,
            tool_format: roko_core::tool::ToolFormat::AnthropicBlocks,
            max_tools_before_degrade: 32,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: true,
            supports_partial: false,
            supports_tool_streaming: true,
        };

        let caps = ProviderCapabilities::from_model_capabilities(&model_caps);
        assert_eq!(caps.tools, ProviderCapabilityState::Supported);
        assert_eq!(caps.reasoning, ProviderCapabilityState::Supported);
        assert_eq!(caps.vision, ProviderCapabilityState::Unavailable);
        assert_eq!(caps.mcp, ProviderCapabilityState::Supported);
        assert_eq!(caps.streaming, ProviderCapabilityState::Supported);
        assert_eq!(caps.code_execution, ProviderCapabilityState::Unavailable);
        assert_eq!(caps.subagents, ProviderCapabilityState::Unavailable);
    }

    #[test]
    fn activate_and_degrade() {
        let mut caps = ProviderCapabilities::default();
        caps.activate(CapabilityKind::Tools);
        assert!(caps.state(CapabilityKind::Tools).is_usable());

        caps.degrade(CapabilityKind::Tools);
        assert!(!caps.state(CapabilityKind::Tools).is_usable());
        assert!(caps.state(CapabilityKind::Tools).was_confirmed());
    }

    #[test]
    fn deny_overrides_supported() {
        let mut caps = ProviderCapabilities::default();
        caps.activate(CapabilityKind::Mcp);
        assert!(caps.state(CapabilityKind::Mcp).is_usable());

        caps.deny(CapabilityKind::Mcp);
        assert!(!caps.state(CapabilityKind::Mcp).is_usable());
        assert_eq!(
            caps.state(CapabilityKind::Mcp),
            ProviderCapabilityState::Denied
        );
    }

    #[test]
    fn usable_and_degraded_lists() {
        let mut caps = ProviderCapabilities::default();
        caps.activate(CapabilityKind::Tools);
        caps.activate(CapabilityKind::Vision);
        caps.degrade(CapabilityKind::Streaming);

        let usable = caps.usable_capabilities();
        assert_eq!(usable.len(), 2);
        assert!(usable.contains(&CapabilityKind::Tools));
        assert!(usable.contains(&CapabilityKind::Vision));

        let degraded = caps.degraded_capabilities();
        assert_eq!(degraded, vec![CapabilityKind::Streaming]);
    }

    #[test]
    fn capability_state_serde_roundtrip() {
        let states = [
            ProviderCapabilityState::Supported,
            ProviderCapabilityState::Configured,
            ProviderCapabilityState::Negotiated,
            ProviderCapabilityState::Active,
            ProviderCapabilityState::Degraded,
            ProviderCapabilityState::Unavailable,
            ProviderCapabilityState::Denied,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let decoded: ProviderCapabilityState = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, state);
        }
    }

    #[test]
    fn provider_capabilities_serde_roundtrip() {
        let mut caps = ProviderCapabilities::default();
        caps.activate(CapabilityKind::Tools);
        caps.deny(CapabilityKind::Mcp);
        caps.degrade(CapabilityKind::Streaming);

        let json = serde_json::to_string(&caps).unwrap();
        let decoded: ProviderCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, caps);
    }

    #[test]
    fn capability_kind_as_str_stable() {
        assert_eq!(CapabilityKind::Tools.as_str(), "tools");
        assert_eq!(CapabilityKind::Streaming.as_str(), "streaming");
        assert_eq!(CapabilityKind::Reasoning.as_str(), "reasoning");
        assert_eq!(CapabilityKind::Vision.as_str(), "vision");
        assert_eq!(CapabilityKind::CodeExecution.as_str(), "code_execution");
        assert_eq!(CapabilityKind::Mcp.as_str(), "mcp");
        assert_eq!(CapabilityKind::Subagents.as_str(), "subagents");
    }

    #[test]
    fn capability_state_as_str_stable() {
        assert_eq!(ProviderCapabilityState::Supported.as_str(), "supported");
        assert_eq!(ProviderCapabilityState::Configured.as_str(), "configured");
        assert_eq!(ProviderCapabilityState::Negotiated.as_str(), "negotiated");
        assert_eq!(ProviderCapabilityState::Active.as_str(), "active");
        assert_eq!(ProviderCapabilityState::Degraded.as_str(), "degraded");
        assert_eq!(ProviderCapabilityState::Unavailable.as_str(), "unavailable");
        assert_eq!(ProviderCapabilityState::Denied.as_str(), "denied");
    }
}
