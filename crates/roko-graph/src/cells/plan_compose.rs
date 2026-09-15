//! PlanComposeCell — fan-in compose cell for the production plan topology.
//!
//! Collects the 7 input signals (6 enrichers + 1 task context) produced by the
//! per-task subgraph enrichment wave and merges them into a single composed
//! `Prompt` signal for the downstream `TaskExecutorCell`.
//!
//! Each input signal's text content is extracted and assembled into labeled
//! sections. The output is a single `Signal` of kind `Prompt` whose body
//! contains the merged text, plus a JSON-encoded manifest of which enrichment
//! sources contributed.

use std::time::Duration;

use async_trait::async_trait;
use roko_core::{Body, Kind, ProtocolId, Signal, error::Result};
use tracing::debug;

use crate::cell::{Cell, CellContext, CellVersion};

/// Known enricher source names used to label sections in the composed output.
///
/// The order here determines the section ordering in the final prompt. The
/// task-context signal is always placed first, before these enricher sections.
const ENRICHER_SOURCES: &[&str] = &[
    "knowledge",
    "episodes",
    "playbook",
    "modulation",
    "safety",
    "experiment",
];

/// Fan-in compose cell for the production plan topology.
///
/// Receives 7 input signals (6 enrichers + 1 task context), extracts text
/// content from each, and produces a single composed `Prompt` signal. Empty
/// or failed enricher outputs are omitted from the composed result.
///
/// Protocol: `Compose` (prompt assembly).
pub struct PlanComposeCell;

impl PlanComposeCell {
    /// Create a new `PlanComposeCell`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract displayable text from a signal body.
    ///
    /// Tries `Body::Text` first, then falls back to pretty-printed JSON for
    /// `Body::Json`, and returns `None` for empty or bytes bodies.
    fn extract_text(signal: &Signal) -> Option<String> {
        match &signal.body {
            Body::Text(s) if !s.is_empty() => Some(s.clone()),
            Body::Json(v) if !v.is_null() => serde_json::to_string_pretty(v).ok(),
            _ => None,
        }
    }

    /// Classify a signal into an enricher source name based on its tags or kind.
    ///
    /// Looks for a `"source"` tag first (set by enricher cells), then falls
    /// back to checking the signal's `Kind` custom key for enricher names.
    /// Returns `"context"` for the task-context signal and `None` for
    /// unrecognized signals.
    fn classify_source(signal: &Signal) -> &str {
        // Check the "source" tag first — enricher cells should set this.
        if let Some(source) = signal.tags.get("source") {
            let src = source.as_str();
            if ENRICHER_SOURCES.contains(&src) || src == "context" || src == "task-context" {
                return if src == "task-context" { "context" } else { src };
            }
        }

        // Fall back to custom kind matching.
        if let Kind::Custom(key) = &signal.kind {
            let key = key.as_str();
            for &enricher in ENRICHER_SOURCES {
                if key.contains(enricher) {
                    return enricher;
                }
            }
            if key.contains("context") || key.contains("task-context") {
                return "context";
            }
        }

        // PromptSection signals are treated as context.
        if matches!(signal.kind, Kind::PromptSection) {
            return "context";
        }

        // Unknown source — include as unclassified.
        "unclassified"
    }
}

impl Default for PlanComposeCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for PlanComposeCell {
    fn cell_id(&self) -> &str {
        "plan.compose"
    }

    fn cell_name(&self) -> &str {
        "PlanComposeCell"
    }

    fn cell_version(&self) -> CellVersion {
        (0, 2, 0)
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Compose]
    }

    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0) // Pure data assembly, no LLM call.
    }

    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(5))
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        // Partition inputs by source.
        let mut context_parts: Vec<String> = Vec::new();
        let mut sections: Vec<(&str, String)> = Vec::new();
        let mut source_manifest: Vec<String> = Vec::new();

        for signal in &input {
            let source = Self::classify_source(signal);
            if let Some(text) = Self::extract_text(signal) {
                if source == "context" {
                    context_parts.push(text);
                    source_manifest.push("context".to_string());
                } else {
                    sections.push((source, text));
                    source_manifest.push(source.to_string());
                }
            }
        }

        // Assemble: context first, then enricher sections in stable order.
        let mut composed = String::new();

        // Task context section.
        if !context_parts.is_empty() {
            composed.push_str("## Task Context\n\n");
            for part in &context_parts {
                composed.push_str(part);
                composed.push_str("\n\n");
            }
        }

        // Enricher sections in canonical order.
        for &enricher_name in ENRICHER_SOURCES {
            let enricher_texts: Vec<&str> = sections
                .iter()
                .filter(|(src, _)| *src == enricher_name)
                .map(|(_, text)| text.as_str())
                .collect();

            if enricher_texts.is_empty() {
                continue;
            }

            let section_title = match enricher_name {
                "knowledge" => "Knowledge Context",
                "episodes" => "Episode History",
                "playbook" => "Playbook Guidance",
                "modulation" => "Modulation State",
                "safety" => "Safety Constraints",
                "experiment" => "Experiment Context",
                other => other,
            };

            composed.push_str(&format!("## {section_title}\n\n"));
            for text in enricher_texts {
                composed.push_str(text);
                composed.push_str("\n\n");
            }
        }

        // Any unclassified inputs go at the end.
        let unclassified: Vec<&str> = sections
            .iter()
            .filter(|(src, _)| *src == "unclassified")
            .map(|(_, text)| text.as_str())
            .collect();
        if !unclassified.is_empty() {
            composed.push_str("## Additional Context\n\n");
            for text in unclassified {
                composed.push_str(text);
                composed.push_str("\n\n");
            }
        }

        // Trim trailing whitespace.
        let composed = composed.trim_end().to_string();

        debug!(
            input_count = input.len(),
            sources = ?source_manifest,
            composed_len = composed.len(),
            "PlanComposeCell: assembled {} inputs into composed prompt ({} bytes)",
            input.len(),
            composed.len(),
        );

        // Produce a single Prompt signal with the assembled content.
        let output = Signal::builder(Kind::Prompt)
            .body(Body::text(&composed))
            .tag("cell", "plan.compose")
            .tag("input_count", input.len().to_string())
            .tag("sources", source_manifest.join(","))
            .build();

        Ok(vec![output])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::Body;

    fn make_enricher_signal(source: &str, text: &str) -> Signal {
        Signal::builder(Kind::Custom(format!("plan.enricher.{source}")))
            .body(Body::text(text))
            .tag("source", source)
            .build()
    }

    fn make_context_signal(text: &str) -> Signal {
        Signal::builder(Kind::Custom("plan.task-context".into()))
            .body(Body::text(text))
            .tag("source", "task-context")
            .build()
    }

    #[tokio::test]
    async fn composes_all_seven_inputs() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let input = vec![
            make_context_signal("Implement the auth module"),
            make_enricher_signal("knowledge", "OAuth2 uses bearer tokens"),
            make_enricher_signal("episodes", "Previous attempt timed out"),
            make_enricher_signal("playbook", "Use reqwest for HTTP"),
            make_enricher_signal("modulation", "Normal energy level"),
            make_enricher_signal("safety", "No elevated permissions needed"),
            make_enricher_signal("experiment", "Using claude-sonnet model"),
        ];

        let output = cell.execute(input, &ctx).await.unwrap();

        assert_eq!(output.len(), 1);
        let signal = &output[0];
        assert_eq!(signal.kind, Kind::Prompt);

        let text = signal.body.as_text().unwrap();
        assert!(text.contains("## Task Context"));
        assert!(text.contains("Implement the auth module"));
        assert!(text.contains("## Knowledge Context"));
        assert!(text.contains("OAuth2 uses bearer tokens"));
        assert!(text.contains("## Episode History"));
        assert!(text.contains("Previous attempt timed out"));
        assert!(text.contains("## Playbook Guidance"));
        assert!(text.contains("Use reqwest for HTTP"));
        assert!(text.contains("## Modulation State"));
        assert!(text.contains("Normal energy level"));
        assert!(text.contains("## Safety Constraints"));
        assert!(text.contains("No elevated permissions needed"));
        assert!(text.contains("## Experiment Context"));
        assert!(text.contains("Using claude-sonnet model"));
    }

    #[tokio::test]
    async fn empty_inputs_produce_empty_prompt() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let output = cell.execute(vec![], &ctx).await.unwrap();

        assert_eq!(output.len(), 1);
        let text = output[0].body.as_text().unwrap();
        assert!(text.is_empty());
    }

    #[tokio::test]
    async fn skips_empty_enricher_sections() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let input = vec![
            make_context_signal("Build feature X"),
            make_enricher_signal("knowledge", "relevant docs"),
            // No episodes, playbook, modulation, safety, experiment.
        ];

        let output = cell.execute(input, &ctx).await.unwrap();

        let text = output[0].body.as_text().unwrap();
        assert!(text.contains("## Task Context"));
        assert!(text.contains("## Knowledge Context"));
        assert!(!text.contains("## Episode History"));
        assert!(!text.contains("## Playbook Guidance"));
    }

    #[tokio::test]
    async fn context_only_input() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let input = vec![make_context_signal("Just context, no enrichment")];

        let output = cell.execute(input, &ctx).await.unwrap();

        let text = output[0].body.as_text().unwrap();
        assert!(text.contains("## Task Context"));
        assert!(text.contains("Just context, no enrichment"));
    }

    #[tokio::test]
    async fn output_tags_record_sources() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let input = vec![
            make_context_signal("ctx"),
            make_enricher_signal("safety", "safe"),
        ];

        let output = cell.execute(input, &ctx).await.unwrap();

        let signal = &output[0];
        assert_eq!(signal.tags.get("cell").unwrap(), "plan.compose");
        assert_eq!(signal.tags.get("input_count").unwrap(), "2");

        let sources = signal.tags.get("sources").unwrap();
        assert!(sources.contains("context"));
        assert!(sources.contains("safety"));
    }

    #[tokio::test]
    async fn json_body_signals_are_extracted() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let json_signal = Signal::builder(Kind::Custom("plan.enricher.knowledge".into()))
            .body(Body::Json(serde_json::json!({"key": "value"})))
            .tag("source", "knowledge")
            .build();

        let output = cell.execute(vec![json_signal], &ctx).await.unwrap();

        let text = output[0].body.as_text().unwrap();
        assert!(text.contains("key"));
        assert!(text.contains("value"));
    }

    #[tokio::test]
    async fn empty_body_signals_are_skipped() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        let empty_signal = Signal::builder(Kind::Custom("plan.enricher.episodes".into()))
            .tag("source", "episodes")
            .build();

        let text_signal = make_enricher_signal("knowledge", "has content");

        let output = cell.execute(vec![empty_signal, text_signal], &ctx).await.unwrap();

        let text = output[0].body.as_text().unwrap();
        assert!(!text.contains("## Episode History"));
        assert!(text.contains("## Knowledge Context"));
        assert!(text.contains("has content"));
    }

    #[tokio::test]
    async fn section_ordering_is_canonical() {
        let cell = PlanComposeCell::new();
        let ctx = CellContext::new();

        // Provide enrichers in reverse order.
        let input = vec![
            make_enricher_signal("experiment", "exp"),
            make_enricher_signal("safety", "safe"),
            make_enricher_signal("modulation", "mod"),
            make_enricher_signal("playbook", "play"),
            make_enricher_signal("episodes", "ep"),
            make_enricher_signal("knowledge", "know"),
            make_context_signal("ctx"),
        ];

        let output = cell.execute(input, &ctx).await.unwrap();
        let text = output[0].body.as_text().unwrap();

        // Context should come first, then enrichers in canonical order.
        let ctx_pos = text.find("## Task Context").unwrap();
        let know_pos = text.find("## Knowledge Context").unwrap();
        let ep_pos = text.find("## Episode History").unwrap();
        let play_pos = text.find("## Playbook Guidance").unwrap();
        let mod_pos = text.find("## Modulation State").unwrap();
        let safe_pos = text.find("## Safety Constraints").unwrap();
        let exp_pos = text.find("## Experiment Context").unwrap();

        assert!(ctx_pos < know_pos);
        assert!(know_pos < ep_pos);
        assert!(ep_pos < play_pos);
        assert!(play_pos < mod_pos);
        assert!(mod_pos < safe_pos);
        assert!(safe_pos < exp_pos);
    }

    #[test]
    fn default_impl() {
        let cell = PlanComposeCell::default();
        assert_eq!(cell.cell_id(), "plan.compose");
        assert_eq!(cell.cell_name(), "PlanComposeCell");
        assert!(!cell.is_stub());
    }

    #[test]
    fn cell_metadata() {
        let cell = PlanComposeCell::new();
        assert_eq!(cell.cell_version(), (0, 2, 0));
        assert_eq!(cell.protocols(), vec![ProtocolId::Compose]);
        assert_eq!(cell.estimated_cost(), Some(0.0));
        assert!(cell.estimated_duration().is_some());
    }
}
