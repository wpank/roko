//! Time-based animation state for TUI effects.

use std::time::Instant;

/// Tracks elapsed time and frame count for driving animations.
#[derive(Debug, Clone)]
pub struct Atmosphere {
    start_time: Instant,
    /// Seconds since start.
    pub elapsed: f64,
    /// Total frames rendered.
    pub frame_count: u64,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            elapsed: 0.0,
            frame_count: 0,
        }
    }
}

impl Atmosphere {
    /// Advance the clock. Call once per frame.
    pub fn tick(&mut self) {
        self.elapsed = self.start_time.elapsed().as_secs_f64();
        self.frame_count += 1;
    }

    /// Breathing brightness oscillation (sine wave, range 0.8..1.0).
    #[must_use]
    pub fn breathing_brightness(&self) -> f64 {
        let phase = (self.elapsed * std::f64::consts::PI * 0.5).sin();
        0.9 + 0.1 * phase
    }

    /// Double-pulse heartbeat pattern.
    /// Returns a value in 0.0..1.0 representing beat intensity.
    #[must_use]
    pub fn heartbeat(&self) -> f64 {
        // Two quick pulses per ~1.5s cycle
        let t = (self.elapsed % 1.5) / 1.5;
        if t < 0.1 {
            // First beat: quick rise
            (t / 0.1).min(1.0)
        } else if t < 0.2 {
            // First beat: quick fall
            1.0 - ((t - 0.1) / 0.1)
        } else if t < 0.3 {
            // Second beat: quick rise
            ((t - 0.2) / 0.1).min(1.0) * 0.7
        } else if t < 0.4 {
            // Second beat: quick fall
            0.7 * (1.0 - ((t - 0.3) / 0.1))
        } else {
            // Rest
            0.0
        }
    }

    /// Current frame count.
    #[must_use]
    pub const fn frame(&self) -> u64 {
        self.frame_count
    }

    /// Elapsed seconds since start.
    #[must_use]
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    /// Approximate FPS based on elapsed time and frame count.
    #[must_use]
    pub fn fps(&self) -> f64 {
        if self.elapsed > 0.0 {
            self.frame_count as f64 / self.elapsed
        } else {
            60.0
        }
    }

    /// Spinner character cycling through animation frames.
    #[must_use]
    pub fn spinner(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[(self.frame_count as usize / 4) % SPINNER.len()]
    }

    /// Ethereal spinner (slower, for subtle animations).
    #[must_use]
    pub fn spinner_ethereal(&self) -> char {
        const SPINNER: &[char] = &['◜', '◝', '◞', '◟'];
        SPINNER[(self.frame_count as usize / 8) % SPINNER.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breathing_in_range() {
        let atm = Atmosphere::default();
        let b = atm.breathing_brightness();
        assert!(b >= 0.79 && b <= 1.01, "breathing={b}");
    }

    #[test]
    fn heartbeat_in_range() {
        let atm = Atmosphere::default();
        let h = atm.heartbeat();
        assert!(h >= 0.0 && h <= 1.0, "heartbeat={h}");
    }
}
