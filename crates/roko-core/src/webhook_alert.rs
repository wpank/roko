//! P4-15: Webhook alerting for critical Inbox items.
//!
//! Defines webhook alert payloads and a simple dispatcher that sends
//! HTTP POST requests to configured webhook URLs when critical conditions
//! (quarantine escalation, budget overrun, safety violations) are detected.

use serde::{Deserialize, Serialize};

/// Severity levels for webhook alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// Informational alert.
    Info,
    /// Warning that may require attention.
    Warning,
    /// Critical alert requiring immediate action.
    Critical,
}

/// A webhook alert payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAlert {
    /// Alert identifier (unique per alert).
    pub alert_id: String,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Alert category (e.g., "quarantine", "budget", "safety").
    pub category: String,
    /// Human-readable summary.
    pub summary: String,
    /// Detailed description.
    pub details: String,
    /// Workspace identifier.
    pub workspace: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Optional structured data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl WebhookAlert {
    /// Create a new alert.
    #[must_use]
    pub fn new(
        category: impl Into<String>,
        severity: AlertSeverity,
        summary: impl Into<String>,
    ) -> Self {
        let category = category.into();
        let summary_str = summary.into();
        Self {
            alert_id: format!(
                "alert-{}-{}",
                &category,
                chrono::Utc::now().timestamp_millis()
            ),
            severity,
            category,
            summary: summary_str,
            details: String::new(),
            workspace: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        }
    }

    /// Attach detailed description.
    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }

    /// Attach workspace identifier.
    #[must_use]
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = workspace.into();
        self
    }

    /// Attach structured metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Webhook configuration for a single endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL to POST to.
    pub url: String,
    /// Minimum severity to trigger this webhook.
    pub min_severity: AlertSeverity,
    /// Optional categories filter (empty = all categories).
    #[serde(default)]
    pub categories: Vec<String>,
    /// Whether the webhook is currently enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl WebhookConfig {
    /// Whether this webhook should fire for the given alert.
    #[must_use]
    pub fn should_fire(&self, alert: &WebhookAlert) -> bool {
        if !self.enabled {
            return false;
        }
        let severity_ok = matches!(
            (self.min_severity, alert.severity),
            (AlertSeverity::Critical, AlertSeverity::Critical)
                | (
                    AlertSeverity::Warning,
                    AlertSeverity::Critical | AlertSeverity::Warning
                )
                | (AlertSeverity::Info, _)
        );
        let category_ok = self.categories.is_empty() || self.categories.contains(&alert.category);
        severity_ok && category_ok
    }
}

/// Registry of webhook endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebhookRegistry {
    /// Configured webhook endpoints.
    pub webhooks: Vec<WebhookConfig>,
}

impl WebhookRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a webhook endpoint.
    pub fn add(&mut self, config: WebhookConfig) {
        self.webhooks.push(config);
    }

    /// Get all webhooks that should fire for a given alert.
    #[must_use]
    pub fn matching_webhooks(&self, alert: &WebhookAlert) -> Vec<&WebhookConfig> {
        self.webhooks
            .iter()
            .filter(|w| w.should_fire(alert))
            .collect()
    }

    /// Load from disk, or return empty if missing.
    #[must_use]
    pub fn load_or_new(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_creation() {
        let alert = WebhookAlert::new("safety", AlertSeverity::Critical, "Quarantine escalation")
            .with_details("Vault 80% full")
            .with_workspace("test-ws");

        assert_eq!(alert.category, "safety");
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }

    #[test]
    fn webhook_config_filtering() {
        let config = WebhookConfig {
            url: "https://hooks.example.com/alert".into(),
            min_severity: AlertSeverity::Warning,
            categories: vec!["safety".into()],
            enabled: true,
        };

        let critical_safety = WebhookAlert::new("safety", AlertSeverity::Critical, "test");
        let info_safety = WebhookAlert::new("safety", AlertSeverity::Info, "test");
        let critical_budget = WebhookAlert::new("budget", AlertSeverity::Critical, "test");

        assert!(config.should_fire(&critical_safety));
        assert!(!config.should_fire(&info_safety)); // Below severity.
        assert!(!config.should_fire(&critical_budget)); // Wrong category.
    }

    #[test]
    fn disabled_webhook_never_fires() {
        let config = WebhookConfig {
            url: "https://hooks.example.com/alert".into(),
            min_severity: AlertSeverity::Info,
            categories: Vec::new(),
            enabled: false,
        };
        let alert = WebhookAlert::new("safety", AlertSeverity::Critical, "test");
        assert!(!config.should_fire(&alert));
    }
}
