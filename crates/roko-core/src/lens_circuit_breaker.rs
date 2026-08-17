//! Lens-specific overhead circuit breaker.

/// Runtime degradation stage for a telemetry Lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LensBreakerStage {
    Active,
    Sampled,
    Disabled,
}

/// Direction returned to the Lens runtime after an overhead check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LensBreakerAction {
    Allow,
    Skip,
    Disable,
}

/// Tracks consecutive Lens overhead-budget violations.
#[derive(Debug, Clone)]
pub struct LensCircuitBreaker {
    stage: LensBreakerStage,
    consecutive_violations: u32,
    total_violations: u64,
    budget_pct: f64,
    sample_threshold: u32,
    disable_threshold: u32,
}

impl Default for LensCircuitBreaker {
    fn default() -> Self {
        Self::new(0.01)
    }
}

impl LensCircuitBreaker {
    #[must_use]
    pub fn new(budget_pct: f64) -> Self {
        Self {
            stage: LensBreakerStage::Active,
            consecutive_violations: 0,
            total_violations: 0,
            budget_pct,
            sample_threshold: 3,
            disable_threshold: 10,
        }
    }

    #[must_use]
    pub const fn with_thresholds(mut self, sample: u32, disable: u32) -> Self {
        self.sample_threshold = sample;
        self.disable_threshold = disable;
        self
    }

    pub fn check(&mut self, lens_duration_ms: u64, cell_duration_ms: u64) -> LensBreakerAction {
        let budget_ms = cell_duration_ms as f64 * self.budget_pct;
        if lens_duration_ms as f64 > budget_ms {
            self.consecutive_violations = self.consecutive_violations.saturating_add(1);
            self.total_violations = self.total_violations.saturating_add(1);
            if self.consecutive_violations == 1 {
                tracing::warn!(
                    lens_duration_ms,
                    cell_duration_ms,
                    "Lens exceeded overhead budget"
                );
            }
            if self.consecutive_violations >= self.disable_threshold {
                self.stage = LensBreakerStage::Disabled;
            } else if self.consecutive_violations >= self.sample_threshold {
                self.stage = LensBreakerStage::Sampled;
            }
        } else {
            self.consecutive_violations = 0;
        }

        match self.stage {
            LensBreakerStage::Active => LensBreakerAction::Allow,
            LensBreakerStage::Sampled => LensBreakerAction::Skip,
            LensBreakerStage::Disabled => LensBreakerAction::Disable,
        }
    }

    #[must_use]
    pub const fn should_invoke(&self) -> bool {
        !matches!(self.stage, LensBreakerStage::Disabled)
    }

    pub fn reset(&mut self) {
        self.stage = LensBreakerStage::Sampled;
        self.consecutive_violations = 0;
    }

    #[must_use]
    pub const fn stage(&self) -> LensBreakerStage {
        self.stage
    }

    #[must_use]
    pub const fn total_violations(&self) -> u64 {
        self.total_violations
    }
}
