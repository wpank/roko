//! Observation bundle -- event publication and telemetry handles.
//!
//! This bundle supports injection of canonical event publishers without
//! depending on StateHub, projection, or TUI types (which live in layer 4).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Observation and telemetry publication handles.
///
/// Required for all profiles. The bundle carries an optional event publisher
/// trait object so that execution surfaces can emit structured events without
/// depending on the concrete StateHub or TUI infrastructure.
#[derive(Debug, Clone)]
pub struct ObservationBundle {
    /// Canonical event publisher, when configured.
    ///
    /// The publisher trait is object-safe and defined in roko-runtime;
    /// concrete implementations (StateHub, logging, test stubs) are
    /// injected at construction time.
    pub event_publisher: Option<Arc<dyn ObservationPublisher>>,
    /// Whether structured telemetry sampling is enabled.
    pub telemetry_enabled: bool,
}

/// Object-safe trait for publishing execution observations.
///
/// Concrete implementations live in higher layers (roko-runtime, roko-serve).
/// This trait is intentionally minimal to keep the layer-3 boundary clean.
pub trait ObservationPublisher: Send + Sync + std::fmt::Debug {
    /// Publish a named event with a JSON-serializable payload.
    fn publish(&self, event_name: &str, payload: &str);
}

/// Serializable summary of the observation bundle for diagnostics.
#[derive(Debug, Serialize, Deserialize)]
pub struct ObservationBundleSummary {
    pub has_event_publisher: bool,
    pub telemetry_enabled: bool,
}

impl ObservationBundle {
    /// Create a minimal observation bundle for testing.
    pub fn for_test() -> Self {
        Self {
            event_publisher: None,
            telemetry_enabled: false,
        }
    }

    /// Produce a serializable summary for diagnostics / snapshot tests.
    pub fn summary(&self) -> ObservationBundleSummary {
        ObservationBundleSummary {
            has_event_publisher: self.event_publisher.is_some(),
            telemetry_enabled: self.telemetry_enabled,
        }
    }
}
