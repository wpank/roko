//! Guards bundle -- safety, permissions, budget, and process supervision.
//!
//! Required for all profiles. Encapsulates the runtime safety and resource
//! control handles that every execution surface must have.

use std::sync::Arc;

use roko_agent::rate_limit::ProviderRateLimiter;
use roko_runtime::process::ProcessSupervisor;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// Safety, permission, budget, and process supervision handles.
///
/// Required for all profiles. Each field represents a long-lived handle
/// constructed once at run start and shared across all dispatch calls.
#[derive(Debug, Clone)]
pub struct GuardsBundle {
    /// Run-scoped cancellation token.
    pub cancel: CancellationToken,
    /// Process supervisor for agent lifecycle management.
    pub process_supervisor: Option<Arc<ProcessSupervisor>>,
    /// Per-provider rate limiter shared across concurrent dispatches.
    pub rate_limiter: Arc<ProviderRateLimiter>,
    /// Cost ledger for the run (accumulated USD spend).
    pub cost_ledger: Arc<CostLedger>,
    /// Budget ceiling in USD (None = unlimited).
    pub budget_ceiling_usd: Option<f64>,
}

/// Simple atomic cost accumulator.
///
/// The runner records per-task costs here; the guards bundle checks
/// against the budget ceiling before dispatching new tasks.
#[derive(Debug)]
pub struct CostLedger {
    total_usd: std::sync::atomic::AtomicU64,
}

impl CostLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self {
            total_usd: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record a cost in USD (as f64 encoded in bits).
    pub fn record(&self, usd: f64) {
        let bits = usd.to_bits();
        // Relaxed is fine: we only need eventual visibility, not ordering.
        let old = self
            .total_usd
            .load(std::sync::atomic::Ordering::Relaxed);
        let old_f64 = f64::from_bits(old);
        let new = old_f64 + usd;
        self.total_usd
            .store(new.to_bits(), std::sync::atomic::Ordering::Relaxed);
        drop(bits); // suppress unused
    }

    /// Read the current total cost.
    pub fn total_usd(&self) -> f64 {
        f64::from_bits(
            self.total_usd
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Check whether the total exceeds a ceiling.
    pub fn exceeds(&self, ceiling_usd: f64) -> bool {
        self.total_usd() >= ceiling_usd
    }
}

impl Default for CostLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable summary of the guards bundle for diagnostics.
#[derive(Debug, Serialize, Deserialize)]
pub struct GuardsBundleSummary {
    pub has_process_supervisor: bool,
    pub has_budget_ceiling: bool,
    pub budget_ceiling_usd: Option<f64>,
    pub current_cost_usd: f64,
}

impl GuardsBundle {
    /// Create a minimal guards bundle for testing.
    pub fn for_test() -> Self {
        Self {
            cancel: CancellationToken::new(),
            process_supervisor: None,
            rate_limiter: Arc::new(ProviderRateLimiter::default()),
            cost_ledger: Arc::new(CostLedger::new()),
            budget_ceiling_usd: None,
        }
    }

    /// Produce a serializable summary for diagnostics / snapshot tests.
    pub fn summary(&self) -> GuardsBundleSummary {
        GuardsBundleSummary {
            has_process_supervisor: self.process_supervisor.is_some(),
            has_budget_ceiling: self.budget_ceiling_usd.is_some(),
            budget_ceiling_usd: self.budget_ceiling_usd,
            current_cost_usd: self.cost_ledger.total_usd(),
        }
    }

    /// Check whether the budget has been exceeded.
    pub fn budget_exceeded(&self) -> bool {
        match self.budget_ceiling_usd {
            Some(ceiling) => self.cost_ledger.exceeds(ceiling),
            None => false,
        }
    }

    /// Remaining budget in USD. Returns `None` when no ceiling is set.
    pub fn budget_remaining_usd(&self) -> Option<f64> {
        self.budget_ceiling_usd
            .map(|ceiling| (ceiling - self.cost_ledger.total_usd()).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_ledger_accumulates() {
        let ledger = CostLedger::new();
        assert_eq!(ledger.total_usd(), 0.0);
        ledger.record(1.50);
        // Note: relaxed ordering means we might not see the exact value
        // in all edge cases, but in single-threaded test this is fine.
        assert!((ledger.total_usd() - 1.50).abs() < f64::EPSILON);
        ledger.record(0.25);
        assert!((ledger.total_usd() - 1.75).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_ledger_exceeds() {
        let ledger = CostLedger::new();
        assert!(!ledger.exceeds(1.0));
        ledger.record(1.5);
        assert!(ledger.exceeds(1.0));
    }

    #[test]
    fn guards_budget_tracking() {
        let guards = GuardsBundle {
            budget_ceiling_usd: Some(5.0),
            ..GuardsBundle::for_test()
        };
        assert!(!guards.budget_exceeded());
        assert_eq!(guards.budget_remaining_usd(), Some(5.0));

        guards.cost_ledger.record(3.0);
        assert!(!guards.budget_exceeded());
        assert!((guards.budget_remaining_usd().unwrap() - 2.0).abs() < f64::EPSILON);

        guards.cost_ledger.record(3.0);
        assert!(guards.budget_exceeded());
        assert_eq!(guards.budget_remaining_usd(), Some(0.0));
    }

    #[test]
    fn guards_no_ceiling_never_exceeded() {
        let guards = GuardsBundle::for_test();
        guards.cost_ledger.record(1000.0);
        assert!(!guards.budget_exceeded());
        assert!(guards.budget_remaining_usd().is_none());
    }
}
