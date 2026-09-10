//! Safety incident records for the F5 Logs safety sub-tab (P2-06).

/// Safety incident record for the F5 Logs safety sub-tab (P2-06).
#[derive(Debug, Clone)]
pub struct SafetyIncident {
    /// Milliseconds since UNIX epoch.
    pub timestamp_ms: u64,
    /// Event category: quarantine, taint, immune, denial, etc.
    pub event_type: String,
    /// Severity: info, warning, critical.
    pub severity: String,
    /// Human-readable description.
    pub description: String,
}
