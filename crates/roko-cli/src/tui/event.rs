//! Crossterm event polling, adaptive tick policy, and render-dirty tracking
//! for the TUI shell.
//!
//! The render loop draws only when `RenderDirty` is non-empty after coalescing
//! all ready inputs. The tick interval adapts between 16 ms (active animation),
//! 100 ms (idle-connected), and 250 ms (fully dormant) based on observable
//! application state.

use std::fmt;
use std::io;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};

// ---------------------------------------------------------------------------
// RenderDirty — reason bitflags for why a redraw is needed
// ---------------------------------------------------------------------------

/// Bit-packed reasons why the frame buffer is stale and a redraw is needed.
///
/// The render loop ORs reasons from input, channel drains, and timers.
/// After drawing, only the reasons included in that frame are cleared so
/// that late-arriving reasons survive into the next iteration.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderDirty(u8);

impl RenderDirty {
    /// No reasons — frame buffer is clean.
    pub const NONE: Self = Self(0);
    /// Keyboard or mouse input was received.
    pub const INPUT: Self = Self(1 << 0);
    /// Dashboard snapshot changed (StateHub, disk reload).
    pub const SNAPSHOT: Self = Self(1 << 1);
    /// System metrics (CPU/MEM/disk/net) updated.
    pub const METRICS: Self = Self(1 << 2);
    /// Modal dialog opened, closed, or content changed.
    pub const MODAL: Self = Self(1 << 3);
    /// Toast notification appeared, expired, or was dismissed.
    pub const NOTIFICATION: Self = Self(1 << 4);
    /// Active animation (atmosphere, tab transition, spinners).
    pub const ANIMATION: Self = Self(1 << 5);
    /// Terminal was resized.
    pub const RESIZE: Self = Self(1 << 6);
    /// Periodic health fallback (ensures the TUI draws at least once
    /// every 250 ms even when fully dormant).
    pub const FORCED_HEALTH: Self = Self(1 << 7);

    /// Returns `true` when no dirty bits are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` when any dirty bit is set.
    #[must_use]
    pub const fn is_dirty(self) -> bool {
        self.0 != 0
    }

    /// Returns `true` when `self` contains all bits in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Insert additional dirty bits.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Remove dirty bits.
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Raw bits (for telemetry / debug).
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Construct from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

impl BitOr for RenderDirty {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for RenderDirty {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for RenderDirty {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for RenderDirty {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for RenderDirty {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl fmt::Debug for RenderDirty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "RenderDirty(NONE)");
        }
        let mut first = true;
        let flags = [
            (Self::INPUT, "INPUT"),
            (Self::SNAPSHOT, "SNAPSHOT"),
            (Self::METRICS, "METRICS"),
            (Self::MODAL, "MODAL"),
            (Self::NOTIFICATION, "NOTIFICATION"),
            (Self::ANIMATION, "ANIMATION"),
            (Self::RESIZE, "RESIZE"),
            (Self::FORCED_HEALTH, "FORCED_HEALTH"),
        ];
        write!(f, "RenderDirty(")?;
        for (flag, name) in flags {
            if self.contains(flag) {
                if !first {
                    write!(f, " | ")?;
                }
                write!(f, "{name}")?;
                first = false;
            }
        }
        write!(f, ")")
    }
}

impl fmt::Display for RenderDirty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// ---------------------------------------------------------------------------
// TickPolicy — adaptive tick rate selection
// ---------------------------------------------------------------------------

/// Target tick cadence selected by `next_tick_policy` based on observable
/// application state. The tick interval is a maximum wake latency, not an
/// unconditional frame interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickPolicy {
    /// Active animation, streaming, or recent input: ~60 Hz (16 ms).
    Active,
    /// Idle but connected — agents running, plan in progress: ~10 Hz (100 ms).
    Idle,
    /// Fully dormant — no animation, no active work. Wait for input or
    /// channel change with a 250 ms health fallback.
    Dormant,
}

impl TickPolicy {
    /// The target tick duration for this policy.
    #[must_use]
    pub const fn duration(self) -> Duration {
        match self {
            Self::Active => Duration::from_millis(16),
            Self::Idle => Duration::from_millis(100),
            Self::Dormant => Duration::from_millis(250),
        }
    }
}

/// Observable TUI state consumed by `next_tick_policy`.
///
/// Callers populate this from `App` fields so the policy function remains
/// pure and testable without an `App` reference.
pub struct TickPolicyInputs {
    /// Whether any agent is currently active/running.
    pub has_active_agents: bool,
    /// Whether any plan is currently active/running.
    pub has_active_plans: bool,
    /// Whether a modal dialog is open.
    pub has_modal: bool,
    /// Whether toast notifications are visible.
    pub has_notifications: bool,
    /// Whether a tab transition animation is in progress.
    pub has_tab_transition: bool,
    /// Whether the PostFX animation pipeline is enabled.
    pub has_postfx: bool,
    /// Time since last keyboard/mouse input.
    pub since_last_input: Duration,
}

/// Select the appropriate tick policy based on observable state.
///
/// This is a pure function — no side effects, no `App` dependency.
/// Both the async `run()` and sync `main_loop()` share this function.
#[must_use]
pub fn next_tick_policy(inputs: &TickPolicyInputs) -> TickPolicy {
    // Recent input (< 2s) always keeps the loop at animation rate for
    // responsive feel after interaction.
    const INPUT_ACTIVE_WINDOW: Duration = Duration::from_secs(2);

    if inputs.since_last_input < INPUT_ACTIVE_WINDOW {
        return TickPolicy::Active;
    }

    // Active animations require the fast cadence.
    if inputs.has_tab_transition || inputs.has_notifications {
        return TickPolicy::Active;
    }

    // PostFX with active work needs smooth rendering.
    if inputs.has_postfx
        && (inputs.has_active_agents || inputs.has_active_plans || inputs.has_modal)
    {
        return TickPolicy::Active;
    }

    // Active agents or plans need responsive updates but not 60 Hz.
    if inputs.has_active_agents || inputs.has_active_plans {
        return TickPolicy::Idle;
    }

    // Modal dialogs without active work still need moderate responsiveness.
    if inputs.has_modal {
        return TickPolicy::Idle;
    }

    // Nothing happening — dormant.
    TickPolicy::Dormant
}

// ---------------------------------------------------------------------------
// FrameStats — per-session render telemetry
// ---------------------------------------------------------------------------

/// Lightweight per-session render statistics accumulated without per-frame
/// logging. Read by the status footer or diagnostic commands.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    /// Total frames drawn this session.
    pub frames_drawn: u64,
    /// Number of tick wakes that did not result in a draw (identical state).
    pub skipped_identical: u64,
    /// Cumulative frame time (draw call duration) for p95 computation.
    pub frame_times: FrameTimeAccumulator,
    /// Cumulative input-to-draw latency samples.
    pub input_to_draw: FrameTimeAccumulator,
    /// Last wake reason bitflags (for status footer display).
    pub last_wake_reason: RenderDirty,
    /// Instant of the most recent keyboard or mouse input (used by
    /// `next_tick_policy` for the 2-second active window).
    pub last_input_at: Option<Instant>,
}

impl FrameStats {
    /// Record a completed frame.
    pub fn record_frame(&mut self, draw_duration: Duration, reason: RenderDirty) {
        self.frames_drawn += 1;
        self.last_wake_reason = reason;
        self.frame_times.push(draw_duration);
    }

    /// Record a skipped (no-change) tick.
    pub fn record_skip(&mut self) {
        self.skipped_identical += 1;
    }

    /// Record that input was received (updates last_input_at).
    pub fn record_input(&mut self) {
        self.last_input_at = Some(Instant::now());
    }

    /// Record an input-to-draw latency sample.
    pub fn record_input_to_draw(&mut self, latency: Duration) {
        self.input_to_draw.push(latency);
    }

    /// Duration since the last input event, or a large sentinel if none.
    #[must_use]
    pub fn since_last_input(&self) -> Duration {
        self.last_input_at
            .map_or(Duration::from_secs(3600), |t| t.elapsed())
    }
}

/// Rolling accumulator for frame time / latency samples.
///
/// Keeps the last 256 samples for p95 computation without unbounded growth.
#[derive(Debug, Clone, Default)]
pub struct FrameTimeAccumulator {
    samples: Vec<Duration>,
    write_pos: usize,
    full: bool,
}

impl FrameTimeAccumulator {
    const CAP: usize = 256;

    /// Push a new sample.
    pub fn push(&mut self, d: Duration) {
        if self.samples.len() < Self::CAP {
            self.samples.push(d);
        } else {
            self.samples[self.write_pos] = d;
            self.full = true;
        }
        self.write_pos = (self.write_pos + 1) % Self::CAP;
    }

    /// Compute the p95 of accumulated samples.
    #[must_use]
    pub fn p95(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let idx = ((sorted.len() as f64) * 0.95).ceil() as usize;
        sorted.get(idx.saturating_sub(1)).copied()
    }

    /// Total sample count pushed (including overwritten).
    #[must_use]
    pub fn count(&self) -> usize {
        if self.full {
            // More samples were pushed than CAP, but we only retain CAP.
            // For the counter, report what we have.
            Self::CAP
        } else {
            self.samples.len()
        }
    }
}

// ---------------------------------------------------------------------------
// Event enum + EventHandler
// ---------------------------------------------------------------------------

/// High-level terminal events consumed by the TUI app loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Keyboard input.
    Key(KeyEvent),
    /// Mouse input.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Tick fired when no input arrives before the configured timeout.
    Tick,
}

/// Polls crossterm for keyboard, resize, and tick events.
#[derive(Debug, Clone)]
pub struct EventHandler {
    tick_rate: Duration,
    last_tick: Instant,
}

impl EventHandler {
    /// Create a new handler with the given tick rate.
    #[must_use]
    pub fn new(tick_rate: Duration) -> Self {
        Self {
            tick_rate,
            last_tick: Instant::now(),
        }
    }

    /// Current tick rate.
    #[must_use]
    pub const fn tick_rate(&self) -> Duration {
        self.tick_rate
    }

    /// Update the tick rate.
    pub fn set_tick_rate(&mut self, tick_rate: Duration) {
        self.tick_rate = tick_rate;
    }

    /// Wait for the next keyboard, mouse, resize, or tick event.
    pub fn next(&mut self) -> io::Result<Event> {
        loop {
            let elapsed = self.last_tick.elapsed();
            let timeout = if elapsed >= self.tick_rate {
                Duration::ZERO
            } else {
                self.tick_rate - elapsed
            };

            if event::poll(timeout)? {
                match event::read()? {
                    CrosstermEvent::Key(key)
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                    {
                        return Ok(Event::Key(key));
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        return Ok(Event::Mouse(mouse));
                    }
                    CrosstermEvent::Resize(width, height) => {
                        return Ok(Event::Resize(width, height));
                    }
                    _ => continue,
                }
            }
            self.last_tick = Instant::now();
            return Ok(Event::Tick);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- RenderDirty bitflag tests --

    #[test]
    fn tui_render_policy_empty_is_clean() {
        let d = RenderDirty::NONE;
        assert!(d.is_empty());
        assert!(!d.is_dirty());
    }

    #[test]
    fn tui_render_policy_single_bit_round_trip() {
        let d = RenderDirty::INPUT;
        assert!(d.is_dirty());
        assert!(d.contains(RenderDirty::INPUT));
        assert!(!d.contains(RenderDirty::SNAPSHOT));
    }

    #[test]
    fn tui_render_policy_or_combines_reasons() {
        let d = RenderDirty::INPUT | RenderDirty::RESIZE;
        assert!(d.contains(RenderDirty::INPUT));
        assert!(d.contains(RenderDirty::RESIZE));
        assert!(!d.contains(RenderDirty::SNAPSHOT));
    }

    #[test]
    fn tui_render_policy_insert_and_remove() {
        let mut d = RenderDirty::NONE;
        d.insert(RenderDirty::SNAPSHOT);
        d.insert(RenderDirty::METRICS);
        assert!(d.contains(RenderDirty::SNAPSHOT));
        assert!(d.contains(RenderDirty::METRICS));
        d.remove(RenderDirty::SNAPSHOT);
        assert!(!d.contains(RenderDirty::SNAPSHOT));
        assert!(d.contains(RenderDirty::METRICS));
    }

    #[test]
    fn tui_render_policy_clear_only_drawn_reasons() {
        // Simulates: reasons A+B are dirty, we draw with A+B, then C arrives
        // late. After clearing A+B, C should survive.
        let mut dirty = RenderDirty::INPUT | RenderDirty::SNAPSHOT;
        let drawn = dirty; // snapshot the reasons we drew
        // Simulate late arrival before clear
        dirty.insert(RenderDirty::METRICS);
        // Clear only what was drawn
        dirty.remove(drawn);
        assert!(!dirty.contains(RenderDirty::INPUT));
        assert!(!dirty.contains(RenderDirty::SNAPSHOT));
        assert!(dirty.contains(RenderDirty::METRICS));
    }

    #[test]
    fn tui_render_policy_debug_display() {
        let d = RenderDirty::INPUT | RenderDirty::ANIMATION;
        let s = format!("{d:?}");
        assert!(s.contains("INPUT"));
        assert!(s.contains("ANIMATION"));
    }

    #[test]
    fn tui_render_policy_from_bits_round_trip() {
        let original = RenderDirty::MODAL | RenderDirty::NOTIFICATION;
        let restored = RenderDirty::from_bits(original.bits());
        assert_eq!(original, restored);
    }

    // -- TickPolicy tests --

    #[test]
    fn tui_render_policy_active_on_recent_input() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: false,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_millis(500),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Active);
    }

    #[test]
    fn tui_render_policy_active_on_tab_transition() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: false,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: true,
            has_postfx: false,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Active);
    }

    #[test]
    fn tui_render_policy_active_on_notifications() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: false,
            has_modal: false,
            has_notifications: true,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Active);
    }

    #[test]
    fn tui_render_policy_active_on_postfx_with_work() {
        let inputs = TickPolicyInputs {
            has_active_agents: true,
            has_active_plans: false,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: true,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Active);
    }

    #[test]
    fn tui_render_policy_idle_on_active_agents() {
        let inputs = TickPolicyInputs {
            has_active_agents: true,
            has_active_plans: false,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Idle);
    }

    #[test]
    fn tui_render_policy_idle_on_active_plans() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: true,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Idle);
    }

    #[test]
    fn tui_render_policy_idle_on_modal() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: false,
            has_modal: true,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_secs(10),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Idle);
    }

    #[test]
    fn tui_render_policy_dormant_when_nothing_happening() {
        let inputs = TickPolicyInputs {
            has_active_agents: false,
            has_active_plans: false,
            has_modal: false,
            has_notifications: false,
            has_tab_transition: false,
            has_postfx: false,
            since_last_input: Duration::from_secs(60),
        };
        assert_eq!(next_tick_policy(&inputs), TickPolicy::Dormant);
    }

    #[test]
    fn tui_render_policy_duration_values() {
        assert_eq!(TickPolicy::Active.duration(), Duration::from_millis(16));
        assert_eq!(TickPolicy::Idle.duration(), Duration::from_millis(100));
        assert_eq!(TickPolicy::Dormant.duration(), Duration::from_millis(250));
    }

    // -- FrameStats tests --

    #[test]
    fn tui_render_policy_frame_stats_records() {
        let mut stats = FrameStats::default();
        assert_eq!(stats.frames_drawn, 0);
        assert_eq!(stats.skipped_identical, 0);

        stats.record_frame(Duration::from_millis(5), RenderDirty::INPUT);
        assert_eq!(stats.frames_drawn, 1);
        assert_eq!(stats.last_wake_reason, RenderDirty::INPUT);

        stats.record_skip();
        assert_eq!(stats.skipped_identical, 1);
    }

    #[test]
    fn tui_render_policy_frame_time_p95() {
        let mut acc = FrameTimeAccumulator::default();
        // Push 100 samples: 1ms, 2ms, ..., 100ms
        for i in 1..=100 {
            acc.push(Duration::from_millis(i));
        }
        let p95 = acc.p95().unwrap();
        // p95 of 1..=100 should be around 95ms
        assert!(
            p95 >= Duration::from_millis(94) && p95 <= Duration::from_millis(96),
            "p95 was {p95:?}, expected ~95ms"
        );
    }

    #[test]
    fn tui_render_policy_since_last_input_default() {
        let stats = FrameStats::default();
        // No input ever: should return a large duration.
        assert!(stats.since_last_input() >= Duration::from_secs(3599));
    }

    // -- FrameTimeAccumulator ring buffer --

    #[test]
    fn tui_render_policy_accumulator_ring_buffer() {
        let mut acc = FrameTimeAccumulator::default();
        // Fill past capacity
        for i in 0..300 {
            acc.push(Duration::from_millis(i));
        }
        // Should not grow beyond CAP
        assert_eq!(acc.samples.len(), FrameTimeAccumulator::CAP);
        assert!(acc.full);
    }
}
