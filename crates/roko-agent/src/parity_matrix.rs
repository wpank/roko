//! Provider capability parity matrix for the tool audit.
//!
//! Maps each [`ProviderKind`] to the set of capabilities it supports,
//! derived from contract test results rather than static declarations.
//! The matrix is used by CI to generate a human-readable parity report
//! and to flag regressions when a provider loses a capability.

use roko_core::agent::ProviderKind;
use std::collections::BTreeMap;
use std::fmt;

/// State of a single capability for a provider family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityState {
    /// The capability is verified to work via contract tests.
    Supported,
    /// The provider does not offer this capability.
    Unavailable,
    /// The capability has not been tested yet.
    Untested,
    /// The capability exists but has known limitations.
    Degraded,
}

impl CapabilityState {
    /// Short label for report columns.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unavailable => "unavailable",
            Self::Untested => "untested",
            Self::Degraded => "degraded",
        }
    }

    /// Single-character symbol for compact reports.
    pub fn as_symbol(self) -> char {
        match self {
            Self::Supported => 'Y',
            Self::Unavailable => 'N',
            Self::Untested => '?',
            Self::Degraded => '~',
        }
    }
}

impl fmt::Display for CapabilityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_label())
    }
}

/// Canonical capability dimensions tested across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Tool definition translation (canonical -> provider format -> canonical).
    Tools,
    /// Streaming response support.
    Streaming,
    /// Reasoning/thinking content support.
    Reasoning,
    /// Vision/image input support.
    Vision,
    /// Code execution support.
    CodeExecution,
    /// MCP tool support (native or bridged).
    Mcp,
    /// Parallel tool call support.
    ParallelTools,
    /// Accurate usage/token reporting.
    UsageReporting,
    /// Request cancellation support.
    Cancellation,
}

impl Capability {
    /// All capability dimensions in canonical order.
    pub const ALL: &'static [Self] = &[
        Self::Tools,
        Self::Streaming,
        Self::Reasoning,
        Self::Vision,
        Self::CodeExecution,
        Self::Mcp,
        Self::ParallelTools,
        Self::UsageReporting,
        Self::Cancellation,
    ];

    /// Column header for the parity report.
    pub fn column_name(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Streaming => "streaming",
            Self::Reasoning => "reasoning",
            Self::Vision => "vision",
            Self::CodeExecution => "code_exec",
            Self::Mcp => "mcp",
            Self::ParallelTools => "parallel",
            Self::UsageReporting => "usage",
            Self::Cancellation => "cancel",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.column_name())
    }
}

/// One row in the parity matrix: a provider and its capability states.
#[derive(Debug, Clone)]
pub struct ProviderCapabilityRow {
    pub provider: ProviderKind,
    pub capabilities: BTreeMap<Capability, CapabilityState>,
}

impl ProviderCapabilityRow {
    /// Create a new row with all capabilities set to [`CapabilityState::Untested`].
    pub fn untested(provider: ProviderKind) -> Self {
        let capabilities = Capability::ALL
            .iter()
            .map(|c| (*c, CapabilityState::Untested))
            .collect();
        Self {
            provider,
            capabilities,
        }
    }

    /// Set the state for a capability.
    pub fn set(&mut self, capability: Capability, state: CapabilityState) {
        self.capabilities.insert(capability, state);
    }

    /// Get the state for a capability.
    pub fn get(&self, capability: Capability) -> CapabilityState {
        self.capabilities
            .get(&capability)
            .copied()
            .unwrap_or(CapabilityState::Untested)
    }
}

/// The full parity matrix mapping every provider to its capabilities.
#[derive(Debug, Clone)]
pub struct ProviderCapabilityMatrix {
    pub rows: BTreeMap<String, ProviderCapabilityRow>,
}

impl ProviderCapabilityMatrix {
    /// All provider kinds in the canonical enumeration order.
    pub const ALL_PROVIDERS: &'static [ProviderKind] = &[
        ProviderKind::AnthropicApi,
        ProviderKind::ClaudeCli,
        ProviderKind::OpenAiCompat,
        ProviderKind::CursorAcp,
        ProviderKind::CursorCli,
        ProviderKind::PerplexityApi,
        ProviderKind::GeminiApi,
        ProviderKind::GeminiCli,
        ProviderKind::CerebrasApi,
        ProviderKind::Hermes,
        ProviderKind::OpenClaw,
    ];

    /// Create a matrix with all providers set to [`CapabilityState::Untested`].
    pub fn untested() -> Self {
        let rows = Self::ALL_PROVIDERS
            .iter()
            .map(|p| {
                (
                    provider_label(*p).to_string(),
                    ProviderCapabilityRow::untested(*p),
                )
            })
            .collect();
        Self { rows }
    }

    /// Create the known-good static baseline based on adapter implementations.
    ///
    /// This reflects what each adapter is *designed* to support based on code
    /// review. Contract tests override these states with verified results.
    pub fn static_baseline() -> Self {
        let mut matrix = Self::untested();

        // AnthropicApi: full-featured HTTP API
        matrix.set_row(
            ProviderKind::AnthropicApi,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Supported),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Unavailable),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // ClaudeCli: subprocess protocol
        matrix.set_row(
            ProviderKind::ClaudeCli,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // OpenAiCompat: covers OpenAI, Ollama, GLM, Kimi, OpenRouter
        matrix.set_row(
            ProviderKind::OpenAiCompat,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Supported),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Unavailable),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // CursorAcp: ACP protocol
        matrix.set_row(
            ProviderKind::CursorAcp,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Unavailable),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Degraded),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // CursorCli: subprocess ACP
        matrix.set_row(
            ProviderKind::CursorCli,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Unavailable),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Degraded),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // PerplexityApi: search-focused, limited tool support
        matrix.set_row(
            ProviderKind::PerplexityApi,
            &[
                (Capability::Tools, CapabilityState::Unavailable),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Unavailable),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Unavailable),
                (Capability::ParallelTools, CapabilityState::Unavailable),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Unavailable),
            ],
        );

        // GeminiApi: full-featured HTTP API
        matrix.set_row(
            ProviderKind::GeminiApi,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Supported),
                (Capability::CodeExecution, CapabilityState::Supported),
                (Capability::Mcp, CapabilityState::Unavailable),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // GeminiCli: subprocess protocol
        matrix.set_row(
            ProviderKind::GeminiCli,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // CerebrasApi: OpenAI-compatible, ultra-fast inference
        matrix.set_row(
            ProviderKind::CerebrasApi,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Unavailable),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Unavailable),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Unavailable),
            ],
        );

        // Hermes: gateway (HTTP, CLI one-shot, or ACP)
        matrix.set_row(
            ProviderKind::Hermes,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Degraded),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        // OpenClaw: inference runtime (CLI one-shot or ACP)
        matrix.set_row(
            ProviderKind::OpenClaw,
            &[
                (Capability::Tools, CapabilityState::Supported),
                (Capability::Streaming, CapabilityState::Supported),
                (Capability::Reasoning, CapabilityState::Supported),
                (Capability::Vision, CapabilityState::Unavailable),
                (Capability::CodeExecution, CapabilityState::Unavailable),
                (Capability::Mcp, CapabilityState::Supported),
                (Capability::ParallelTools, CapabilityState::Supported),
                (Capability::UsageReporting, CapabilityState::Supported),
                (Capability::Cancellation, CapabilityState::Supported),
            ],
        );

        matrix
    }

    /// Set multiple capabilities on a single provider row.
    fn set_row(&mut self, provider: ProviderKind, entries: &[(Capability, CapabilityState)]) {
        let label = provider_label(provider).to_string();
        if let Some(row) = self.rows.get_mut(&label) {
            for (cap, state) in entries {
                row.set(*cap, *state);
            }
        }
    }

    /// Record a contract test result, overriding the static baseline.
    pub fn record_result(
        &mut self,
        provider: ProviderKind,
        capability: Capability,
        state: CapabilityState,
    ) {
        let label = provider_label(provider).to_string();
        if let Some(row) = self.rows.get_mut(&label) {
            row.set(capability, state);
        }
    }

    /// Generate a markdown parity report suitable for CI output.
    pub fn to_markdown_report(&self) -> String {
        let mut report = String::from("# Provider Capability Parity Matrix\n\n");
        report.push_str("Legend: Y=supported, N=unavailable, ?=untested, ~=degraded\n\n");

        // Header row
        report.push_str("| Provider |");
        for cap in Capability::ALL {
            report.push_str(&format!(" {} |", cap.column_name()));
        }
        report.push('\n');

        // Separator
        report.push_str("|---|");
        for _ in Capability::ALL {
            report.push_str("---|");
        }
        report.push('\n');

        // Data rows
        for (label, row) in &self.rows {
            report.push_str(&format!("| {label} |"));
            for cap in Capability::ALL {
                let state = row.get(*cap);
                report.push_str(&format!(" {} |", state.as_symbol()));
            }
            report.push('\n');
        }

        // Summary
        let total = self.rows.len() * Capability::ALL.len();
        let supported = self
            .rows
            .values()
            .flat_map(|row| row.capabilities.values())
            .filter(|s| **s == CapabilityState::Supported)
            .count();
        let unavailable = self
            .rows
            .values()
            .flat_map(|row| row.capabilities.values())
            .filter(|s| **s == CapabilityState::Unavailable)
            .count();
        let untested = self
            .rows
            .values()
            .flat_map(|row| row.capabilities.values())
            .filter(|s| **s == CapabilityState::Untested)
            .count();
        let degraded = self
            .rows
            .values()
            .flat_map(|row| row.capabilities.values())
            .filter(|s| **s == CapabilityState::Degraded)
            .count();

        report.push_str(&format!(
            "\n**Coverage:** {supported}/{total} supported, {unavailable} unavailable, \
             {degraded} degraded, {untested} untested\n"
        ));

        report
    }

    /// Count cells in a given state across all providers.
    pub fn count_state(&self, state: CapabilityState) -> usize {
        self.rows
            .values()
            .flat_map(|row| row.capabilities.values())
            .filter(|s| **s == state)
            .count()
    }
}

/// Short stable label for a provider kind, used as matrix row keys.
pub fn provider_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::AnthropicApi => "anthropic_api",
        ProviderKind::ClaudeCli => "claude_cli",
        ProviderKind::OpenAiCompat => "openai_compat",
        ProviderKind::CursorAcp => "cursor_acp",
        ProviderKind::CursorCli => "cursor_cli",
        ProviderKind::PerplexityApi => "perplexity_api",
        ProviderKind::GeminiApi => "gemini_api",
        ProviderKind::GeminiCli => "gemini_cli",
        ProviderKind::CerebrasApi => "cerebras_api",
        ProviderKind::Hermes => "hermes",
        ProviderKind::OpenClaw => "openclaw",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untested_matrix_has_all_providers() {
        let matrix = ProviderCapabilityMatrix::untested();
        assert_eq!(
            matrix.rows.len(),
            ProviderCapabilityMatrix::ALL_PROVIDERS.len()
        );
        for provider in ProviderCapabilityMatrix::ALL_PROVIDERS {
            let label = provider_label(*provider);
            assert!(matrix.rows.contains_key(label), "missing provider: {label}");
        }
    }

    #[test]
    fn untested_matrix_all_cells_untested() {
        let matrix = ProviderCapabilityMatrix::untested();
        let total = matrix.rows.len() * Capability::ALL.len();
        assert_eq!(matrix.count_state(CapabilityState::Untested), total);
    }

    #[test]
    fn static_baseline_has_no_untested_cells() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        assert_eq!(
            matrix.count_state(CapabilityState::Untested),
            0,
            "static baseline must classify every cell"
        );
    }

    #[test]
    fn record_result_overrides_baseline() {
        let mut matrix = ProviderCapabilityMatrix::static_baseline();
        // PerplexityApi.Tools is Unavailable in baseline
        assert_eq!(
            matrix.rows["perplexity_api"].get(Capability::Tools),
            CapabilityState::Unavailable,
        );
        matrix.record_result(
            ProviderKind::PerplexityApi,
            Capability::Tools,
            CapabilityState::Supported,
        );
        assert_eq!(
            matrix.rows["perplexity_api"].get(Capability::Tools),
            CapabilityState::Supported,
        );
    }

    #[test]
    fn markdown_report_contains_all_providers() {
        let matrix = ProviderCapabilityMatrix::static_baseline();
        let report = matrix.to_markdown_report();
        for provider in ProviderCapabilityMatrix::ALL_PROVIDERS {
            let label = provider_label(*provider);
            assert!(report.contains(label), "report missing provider: {label}");
        }
        assert!(report.contains("Coverage:"));
    }

    #[test]
    fn capability_state_display() {
        assert_eq!(CapabilityState::Supported.as_symbol(), 'Y');
        assert_eq!(CapabilityState::Unavailable.as_symbol(), 'N');
        assert_eq!(CapabilityState::Untested.as_symbol(), '?');
        assert_eq!(CapabilityState::Degraded.as_symbol(), '~');
    }

    #[test]
    fn all_capabilities_covered() {
        assert_eq!(Capability::ALL.len(), 9);
    }

    #[test]
    fn provider_label_covers_all_kinds() {
        for provider in ProviderCapabilityMatrix::ALL_PROVIDERS {
            let label = provider_label(*provider);
            assert!(!label.is_empty());
        }
    }
}
