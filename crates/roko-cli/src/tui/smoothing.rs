//! Exponential moving average for smoothing TUI display metrics.
//!
//! Prevents visual jumps when raw values update in large steps.

/// Default smoothing factor — each frame the displayed value moves 12% toward the raw value.
pub const DEFAULT_ALPHA: f64 = 0.12;

/// Exponentially smoothed scalar value.
///
/// `update(raw)` applies `display = alpha * raw + (1 - alpha) * display`.
#[derive(Debug, Clone)]
pub struct SmoothedValue {
    value: f64,
    alpha: f64,
}

impl SmoothedValue {
    /// Create a new smoother starting at zero.
    pub fn new(alpha: f64) -> Self {
        Self {
            value: 0.0,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Create a new smoother starting at `initial`.
    pub fn with_initial(alpha: f64, initial: f64) -> Self {
        Self {
            value: initial,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Push a new raw value. The internal display value moves toward it.
    pub fn update(&mut self, raw: f64) {
        self.value = self.alpha * raw + (1.0 - self.alpha) * self.value;
    }

    /// Return the current smoothed display value.
    pub fn get(&self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_toward_raw() {
        let mut s = SmoothedValue::new(DEFAULT_ALPHA);
        for _ in 0..50 {
            s.update(100.0);
        }
        assert!(
            (s.get() - 100.0).abs() < 0.5,
            "should converge to 100, got {}",
            s.get()
        );
    }

    #[test]
    fn with_initial_seeds_value() {
        let s = SmoothedValue::with_initial(DEFAULT_ALPHA, 50.0);
        assert!((s.get() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn alpha_one_tracks_instantly() {
        let mut s = SmoothedValue::new(1.0);
        s.update(42.0);
        assert!((s.get() - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stable_input_stays_stable() {
        let mut s = SmoothedValue::with_initial(DEFAULT_ALPHA, 100.0);
        for _ in 0..10 {
            s.update(100.0);
        }
        assert!((s.get() - 100.0).abs() < f64::EPSILON);
    }
}
