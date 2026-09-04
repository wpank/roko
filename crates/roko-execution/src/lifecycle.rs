//! Runner lifecycle event types for the canonical #208 envelope.
//!
//! These are layer-3 event types that wrap runner-specific state
//! transitions. The actual envelope wrapping happens in the CLI (layer 4)
//! via a thin adapter that does not add Runner variants to core.

use serde::Serialize;

/// Runner lifecycle events published through the canonical #208 envelope.
///
/// The CLI's layer-4 adapter wraps these into `RuntimeEventEnvelope`
/// without adding Runner-specific variants to `roko-core`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum RunnerLifecycleEvent {
    /// Runner started with the given profile and plan count.
    Started {
        profile: String,
        plan_count: usize,
        task_count: usize,
    },
    /// Runner service construction completed.
    ServicesReady {
        profile: String,
        factory_init_ms: u64,
    },
    /// A plan within the run completed.
    PlanCompleted {
        plan_id: String,
        passed: bool,
        tasks_passed: usize,
        tasks_failed: usize,
    },
    /// The entire run completed.
    RunCompleted {
        plans_passed: usize,
        plans_failed: usize,
        total_cost_usd: f64,
        duration_secs: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_event_serializes() {
        let event = RunnerLifecycleEvent::Started {
            profile: "FullPlan".to_string(),
            plan_count: 2,
            task_count: 10,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"started\""));
        assert!(json.contains("\"plan_count\":2"));
    }

    #[test]
    fn services_ready_event_serializes() {
        let event = RunnerLifecycleEvent::ServicesReady {
            profile: "GraphPlan".to_string(),
            factory_init_ms: 42,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"services_ready\""));
        assert!(json.contains("\"factory_init_ms\":42"));
    }

    #[test]
    fn plan_completed_event_serializes() {
        let event = RunnerLifecycleEvent::PlanCompleted {
            plan_id: "my-plan".to_string(),
            passed: true,
            tasks_passed: 5,
            tasks_failed: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"plan_completed\""));
        assert!(json.contains("\"passed\":true"));
    }

    #[test]
    fn run_completed_event_serializes() {
        let event = RunnerLifecycleEvent::RunCompleted {
            plans_passed: 3,
            plans_failed: 1,
            total_cost_usd: 1.23,
            duration_secs: 120,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"run_completed\""));
        assert!(json.contains("\"plans_passed\":3"));
        assert!(json.contains("\"duration_secs\":120"));
    }
}
