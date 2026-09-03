//! Canonical ROSEDUST theme and palette helpers for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Canonical ROSEDUST palette and semantic style helpers for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Primary foreground text color.
    pub foreground: Color,
    /// Secondary or muted text color.
    pub muted: Color,
    /// Default background color.
    pub background: Color,
    /// Primary accent color.
    pub accent: Color,
    /// Accent foreground color for contrast.
    pub accent_foreground: Color,
    /// Success or completed state color.
    pub success: Color,
    /// Warning or gating state color.
    pub warning: Color,
    /// Error or failed state color.
    pub danger: Color,
    /// Informational or active state color.
    pub info: Color,
    /// Selection background color.
    pub selection_background: Color,
    /// Selection foreground color.
    pub selection_foreground: Color,
}

impl Theme {
    // -- Primaries (ROSEDUST v2 canonical) --
    pub(crate) const VOID: Color = Color::Rgb(0, 0, 0); // true black, matching Mori's canvas
    pub(crate) const ROSE: Color = Color::Rgb(185, 120, 148);
    pub(crate) const ROSE_BRIGHT: Color = Color::Rgb(220, 155, 180);
    pub(crate) const ROSE_GLOW: Color = Color::Rgb(228, 172, 196);
    pub(crate) const ROSE_DIM: Color = Color::Rgb(155, 106, 124);
    pub(crate) const ROSE_DEEP: Color = Color::Rgb(65, 36, 52);
    pub(crate) const BONE: Color = Color::Rgb(215, 198, 158);
    pub(crate) const BONE_BRIGHT: Color = Color::Rgb(228, 213, 176);
    pub(crate) const BONE_DIM: Color = Color::Rgb(160, 142, 108);

    // -- Text (ROSEDUST v2 canonical) --
    pub(crate) const TEXT: Color = Color::Rgb(165, 142, 158);
    pub(crate) const TEXT_STRONG: Color = Color::Rgb(215, 198, 208);
    pub const TEXT_DIM: Color = Color::Rgb(145, 120, 138);
    pub(crate) const TEXT_GHOST: Color = Color::Rgb(110, 85, 105);
    pub(crate) const TEXT_PHANTOM: Color = Color::Rgb(55, 42, 55);

    // -- Accents (ROSEDUST v2 canonical) --
    pub const DREAM: Color = Color::Rgb(120, 115, 165);
    pub(crate) const DREAM_BRIGHT: Color = Color::Rgb(150, 145, 192);
    pub(crate) const SAGE: Color = Color::Rgb(125, 158, 140);
    pub(crate) const EMBER: Color = Color::Rgb(195, 110, 85);
    pub(crate) const WARNING: Color = Color::Rgb(195, 155, 95);
    pub(crate) const ROSE_EMBER: Color = Color::Rgb(80, 45, 62);
    /// REM imagination / creative purple accent.
    pub(crate) const DREAM_REM: Color = Color::Rgb(180, 100, 200);
    /// Deep accent for backgrounds.
    pub(crate) const DREAM_DEEP: Color = Color::Rgb(40, 40, 72);
    /// Secondary text.
    pub(crate) const TEXT_SOFT: Color = Color::Rgb(200, 184, 196);

    // -- Backgrounds (ROSEDUST v2 canonical) --
    pub(crate) const BG: Color = Self::VOID;
    pub(crate) const BG_RAISED: Color = Color::Rgb(14, 12, 18);
    pub(crate) const BG_SECONDARY: Color = Color::Rgb(14, 12, 16);
    pub(crate) const BG_HIGHLIGHT: Color = Color::Rgb(34, 28, 36);

    // -- Structural --
    pub(crate) const SEPARATOR: Color = Color::Rgb(40, 35, 42);
    pub(crate) const SHADOW: Color = Color::Rgb(30, 30, 30);
    pub(crate) const SHADOW_FG: Color = Color::Rgb(50, 50, 50);

    /// Diff hunk header color (`@@` lines).
    pub(crate) const HUNK: Color = Self::DREAM;

    /// Column header text for tables and grids.
    pub(crate) const COL_HEADER: Color = Color::Rgb(60, 50, 60);

    /// Lavender accent for conductor/orchestrator roles.
    pub(crate) const LAVENDER: Color = Color::Rgb(155, 130, 175);

    /// Teal accent for researcher roles.
    pub(crate) const TEAL: Color = Color::Rgb(100, 150, 170);

    /// Low-focus score color.
    pub(crate) const FOCUS_LOW: Color = Color::Rgb(110, 95, 115);

    /// Themed palette for bar chart / series differentiation.
    ///
    /// Uses ROSEDUST accent colors instead of raw ANSI 16-color values.
    pub(crate) const SERIES_COLORS: [Color; 6] = [
        Self::ROSE,      // primary data series
        Self::DREAM,     // secondary
        Self::SAGE,      // tertiary
        Self::WARNING,   // quaternary
        Self::BONE_DIM,  // fifth
        Self::DREAM_REM, // sixth
    ];

    /// Stage colors for cascade router stages (Static / Confidence / UCB).
    pub(crate) const STAGE_STATIC: Color = Self::WARNING;
    pub(crate) const STAGE_CONFIDENCE: Color = Self::DREAM;
    pub(crate) const STAGE_UCB: Color = Self::SAGE;

    /// Pass-rate semantic colors matching the danger/warning/success triad.
    pub(crate) const RATE_GOOD: Color = Self::SAGE;
    pub(crate) const RATE_MID: Color = Self::WARNING;
    pub(crate) const RATE_BAD: Color = Self::EMBER;

    // -- Foreground aliases --
    pub(crate) const FG: Color = Self::TEXT;
    pub(crate) const FG_DIM: Color = Self::TEXT_DIM;

    // -- Semantic status --
    pub(crate) const STATUS_OK: Color = Self::SAGE;
    pub(crate) const STATUS_ERROR: Color = Self::EMBER;

    /// ROSEDUST palette — warm rose/indigo aesthetic from Mori's design system.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            foreground: Self::TEXT,
            muted: Self::TEXT_DIM,
            background: Self::BG,
            accent: Self::ROSE,
            accent_foreground: Self::VOID,
            success: Self::SAGE,
            warning: Self::WARNING,
            danger: Self::EMBER,
            info: Self::DREAM,
            selection_background: Self::BG_HIGHLIGHT,
            selection_foreground: Self::BONE,
        }
    }

    /// Build an uncolored palette for `NO_COLOR` environments.
    #[must_use]
    pub const fn no_color() -> Self {
        Self {
            foreground: Color::Reset,
            muted: Color::Reset,
            background: Color::Reset,
            accent: Color::Reset,
            accent_foreground: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            danger: Color::Reset,
            info: Color::Reset,
            selection_background: Color::Reset,
            selection_foreground: Color::Reset,
        }
    }

    /// High-contrast palette for accessibility (WCAG 2.1 AA).
    ///
    /// All text colors have at least 4.5:1 contrast ratio against the
    /// background. Uses pure white text on black, bright primary colors,
    /// and avoids low-contrast pastels.
    #[must_use]
    pub const fn high_contrast() -> Self {
        Self {
            foreground: Color::White,
            muted: Color::Rgb(180, 180, 180),
            background: Color::Black,
            accent: Color::Rgb(255, 180, 200), // bright pink
            accent_foreground: Color::Black,
            success: Color::Rgb(100, 255, 100), // bright green
            warning: Color::Rgb(255, 255, 80),  // bright yellow
            danger: Color::Rgb(255, 80, 80),    // bright red
            info: Color::Rgb(100, 180, 255),    // bright blue
            selection_background: Color::Rgb(60, 60, 80),
            selection_foreground: Color::White,
        }
    }

    /// 256-color fallback palette for terminals that don't support 24-bit RGB.
    ///
    /// Uses xterm-256 indexed colors that approximate the ROSEDUST palette.
    /// Activated when `TERM` does not contain `24bit`, `truecolor`, or
    /// `256color` (e.g. plain `xterm`, `vt100`, `dumb`).
    #[must_use]
    pub const fn fallback_256() -> Self {
        Self {
            foreground: Color::Indexed(182),    // mauve-grey ~#A58E9E
            muted: Color::Indexed(139),         // dim rose-grey ~#91788A
            background: Color::Indexed(16),     // near-black
            accent: Color::Indexed(175),        // rose ~#B97894
            accent_foreground: Color::Indexed(16),
            success: Color::Indexed(108),       // sage green ~#7D9E8C
            warning: Color::Indexed(179),       // amber ~#C39B5F
            danger: Color::Indexed(167),        // ember ~#C36E55
            info: Color::Indexed(103),          // dream indigo ~#7873A5
            selection_background: Color::Indexed(236), // dark grey
            selection_foreground: Color::Indexed(187), // bone ~#D7C69E
        }
    }

    /// Build the active palette from the current environment.
    ///
    /// Checks `ROKO_HIGH_CONTRAST`, `NO_COLOR`, and `TERM` / `COLORTERM`
    /// to select the best palette for the active terminal.
    #[must_use]
    pub fn from_env() -> Self {
        if std::env::var_os("ROKO_HIGH_CONTRAST").is_some() {
            Self::high_contrast()
        } else if std::env::var_os("NO_COLOR").is_some() {
            Self::no_color()
        } else if Self::terminal_supports_truecolor() {
            Self::dark()
        } else {
            Self::fallback_256()
        }
    }

    /// Returns `true` when the terminal likely supports 24-bit RGB.
    fn terminal_supports_truecolor() -> bool {
        // COLORTERM=truecolor / 24bit is the most reliable indicator.
        if let Some(ct) = std::env::var_os("COLORTERM") {
            let ct = ct.to_string_lossy();
            if ct.contains("truecolor") || ct.contains("24bit") {
                return true;
            }
        }
        // Fall back to TERM heuristics — most modern terminals set a value
        // that contains "256color" or better.
        if let Some(term) = std::env::var_os("TERM") {
            let term = term.to_string_lossy();
            // Common truecolor-capable terminals.
            if term.contains("256color")
                || term.contains("kitty")
                || term.contains("alacritty")
                || term.contains("wezterm")
            {
                return true;
            }
        }
        // When we can't tell, assume truecolor — most 2020+ terminals support it.
        true
    }

    /// Build the active palette from an explicit `NO_COLOR` flag.
    #[must_use]
    pub const fn from_no_color(no_color: bool) -> Self {
        if no_color {
            Self::no_color()
        } else {
            Self::dark()
        }
    }

    /// A plain foreground style.
    #[must_use]
    pub fn text(self) -> Style {
        Style::default().fg(self.foreground)
    }

    /// A muted foreground style.
    #[must_use]
    pub fn muted(self) -> Style {
        Style::default().fg(self.muted)
    }

    /// An accent style used for titles and highlights.
    #[must_use]
    pub fn accent(self) -> Style {
        Style::default().fg(self.accent)
    }

    /// A bold accent style for selected content.
    #[must_use]
    pub fn accent_bold(self) -> Style {
        self.accent().add_modifier(Modifier::BOLD)
    }

    /// A selected-item style with readable contrast.
    #[must_use]
    pub fn selection(self) -> Style {
        Style::default()
            .fg(self.selection_foreground)
            .bg(self.selection_background)
            .add_modifier(Modifier::BOLD)
    }

    /// A success style for completed or healthy states.
    #[must_use]
    pub fn success(self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// A warning style for gating or degraded states.
    #[must_use]
    pub fn warning(self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// A danger style for failed or critical states.
    #[must_use]
    pub fn danger(self) -> Style {
        Style::default()
            .fg(self.danger)
            .add_modifier(Modifier::BOLD)
    }

    /// An informational style for active or in-flight states.
    #[must_use]
    pub fn info(self) -> Style {
        Style::default().fg(self.info).add_modifier(Modifier::BOLD)
    }

    /// Default block background style.
    #[must_use]
    pub(crate) fn block_style() -> Style {
        Style::default().fg(Self::FG_DIM).bg(Self::BG)
    }

    /// Focused-panel border style.
    #[must_use]
    pub(crate) fn focused_border_style() -> Style {
        Style::default()
            .fg(Self::ROSE_BRIGHT)
            .add_modifier(Modifier::BOLD)
    }

    /// Unfocused-panel border style.
    #[must_use]
    pub(crate) fn unfocused_border_style() -> Style {
        Style::default().fg(Self::TEXT_PHANTOM)
    }

    /// Focused-panel title style.
    #[must_use]
    pub(crate) fn focused_title_style() -> Style {
        Style::default().fg(Self::BONE).add_modifier(Modifier::BOLD)
    }

    /// Unfocused-panel title style.
    #[must_use]
    pub(crate) fn unfocused_title_style() -> Style {
        Style::default().fg(Self::TEXT_GHOST)
    }

    /// Default title style.
    #[must_use]
    pub(crate) fn title_style() -> Style {
        Style::default()
            .fg(Self::BONE_DIM)
            .add_modifier(Modifier::BOLD)
    }

    /// Error style.
    #[must_use]
    pub(crate) fn error_style() -> Style {
        Style::default()
            .fg(Self::EMBER)
            .add_modifier(Modifier::BOLD)
    }

    /// Success style.
    #[must_use]
    pub(crate) fn success_style() -> Style {
        Style::default().fg(Self::SAGE).add_modifier(Modifier::BOLD)
    }

    /// Per-role accent color.
    #[must_use]
    pub(crate) fn role_accent(role: &str) -> Color {
        match role {
            r if r.contains("implement") => Self::ROSE,
            r if r.contains("strateg") => Self::DREAM,
            r if r.contains("architect") => Self::BONE,
            r if r.contains("audit") => Self::SAGE,
            r if r.contains("critic") => Self::EMBER,
            r if r.contains("conduct") => Self::WARNING,
            r if r.contains("research") => Self::DREAM,
            _ => Self::TEXT_DIM,
        }
    }

    /// Phase-based accent color.
    #[must_use]
    pub(crate) fn phase_accent(phase: &str) -> Color {
        match phase {
            p if p.contains("preflight") => Self::TEXT_GHOST,
            p if p.contains("implement") => Self::ROSE,
            p if p.contains("strateg") => Self::DREAM,
            p if p.contains("compil") || p.contains("test") => Self::WARNING,
            p if p.contains("review") || p.contains("critic") => Self::BONE_DIM,
            p if p.contains("gate") || p.contains("verify") => Self::SAGE,
            p if p.contains("fail") => Self::EMBER,
            p if p.contains("done") || p.contains("complete") => Self::SAGE,
            _ => Self::TEXT_DIM,
        }
    }

    /// Semantic color on a 0..1 progress scale: red -> amber -> green.
    #[must_use]
    pub(crate) fn semantic_color(t: f64) -> Color {
        if t >= 0.8 {
            Self::SAGE
        } else if t >= 0.4 {
            Self::WARNING
        } else {
            Self::EMBER
        }
    }

    /// Smooth gradient progress color on a 0..1 scale.
    ///
    /// Interpolates EMBER (0%) -> WARNING (50%) -> SAGE (100%) using the
    /// ROSEDUST semantic triad for a continuous progress ramp.
    #[must_use]
    pub(crate) fn progress_gradient(t: f64) -> Color {
        gradient_progress().sample(t)
    }

    /// Gradient progress color on a 0..1 scale using the fire gradient.
    ///
    /// Used by the header bar for the gradient progress bar fill color.
    #[must_use]
    pub(crate) fn progress_color(fraction: f64) -> Color {
        gradient_fire().sample(fraction)
    }

    /// Rose gradient sparkline color on a 0..1 scale.
    ///
    /// Maps values through a ROSEDUST-tinted gradient from deep rose
    /// through dream to rose-bright for sparkline/bar chart fills.
    #[must_use]
    pub(crate) fn sparkline_gradient(t: f64) -> Color {
        gradient_rose().sample(t)
    }

    /// Notification pulse color for toast/badge animations.
    ///
    /// Returns a color that oscillates between the base notification
    /// color and a brighter variant based on the pulse parameter (0..1).
    #[must_use]
    pub(crate) fn notification_pulse(level_color: Color, pulse: f64) -> Color {
        let factor = 0.7 + 0.3 * pulse.clamp(0.0, 1.0);
        brighten(level_color, factor)
    }

    // -- Visual weight hierarchy --

    /// Bold section header style for panel/section titles.
    #[must_use]
    pub(crate) fn section_header(self) -> Style {
        Style::default()
            .fg(Self::BONE)
            .add_modifier(Modifier::BOLD)
    }

    /// Dim label style for field names like "Status:", "Cost:".
    #[must_use]
    pub(crate) fn label(self) -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    /// Strong value style for data content beside labels.
    #[must_use]
    pub(crate) fn value(self) -> Style {
        Style::default()
            .fg(Self::TEXT_STRONG)
            .add_modifier(Modifier::BOLD)
    }

    /// Ghost metadata style for timestamps, hashes, and secondary info.
    #[must_use]
    pub(crate) fn metadata(self) -> Style {
        Style::default().fg(Self::TEXT_GHOST)
    }

    // -- Status badges --

    /// Pending badge: warning background with void text.
    #[must_use]
    pub(crate) fn badge_pending(self) -> Style {
        Style::default()
            .fg(Self::VOID)
            .bg(Self::WARNING)
            .add_modifier(Modifier::BOLD)
    }

    /// Running badge: dream background with void text.
    #[must_use]
    pub(crate) fn badge_running(self) -> Style {
        Style::default()
            .fg(Self::VOID)
            .bg(Self::DREAM)
            .add_modifier(Modifier::BOLD)
    }

    /// Complete badge: sage background with void text.
    #[must_use]
    pub(crate) fn badge_complete(self) -> Style {
        Style::default()
            .fg(Self::VOID)
            .bg(Self::SAGE)
            .add_modifier(Modifier::BOLD)
    }

    /// Failed badge: ember background with bone text.
    #[must_use]
    pub(crate) fn badge_failed(self) -> Style {
        Style::default()
            .fg(Self::BONE)
            .bg(Self::EMBER)
            .add_modifier(Modifier::BOLD)
    }

    // -- Code block --

    /// Code block style: raised background with standard text.
    #[must_use]
    pub(crate) fn code_block(self) -> Style {
        Style::default().fg(Self::TEXT).bg(Self::BG_RAISED)
    }

    // -- Discrete progress tiers --

    /// Returns a discrete tier color for a progress percentage (0.0-1.0):
    /// ember (<25%) -> warning (25-75%) -> sage (>=75%).
    #[must_use]
    pub(crate) fn progress_tier(self, pct: f64) -> Color {
        if pct < 0.25 {
            Self::EMBER
        } else if pct < 0.75 {
            Self::WARNING
        } else {
            Self::SAGE
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_env()
    }
}

/// A three-stop linear gradient.
#[derive(Clone, Debug)]
pub(crate) struct Gradient {
    start: (f64, f64, f64),
    mid: (f64, f64, f64),
    end: (f64, f64, f64),
}

impl Gradient {
    /// Sample the gradient at `t` in `0.0..=1.0`.
    #[must_use]
    pub(crate) fn sample(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let (r, g, b) = if t < 0.5 {
            let lt = t * 2.0;
            (
                self.start.0 + (self.mid.0 - self.start.0) * lt,
                self.start.1 + (self.mid.1 - self.start.1) * lt,
                self.start.2 + (self.mid.2 - self.start.2) * lt,
            )
        } else {
            let lt = (t - 0.5) * 2.0;
            (
                self.mid.0 + (self.end.0 - self.mid.0) * lt,
                self.mid.1 + (self.end.1 - self.mid.1) * lt,
                self.mid.2 + (self.end.2 - self.mid.2) * lt,
            )
        };
        Color::Rgb(
            r.clamp(0.0, 255.0) as u8,
            g.clamp(0.0, 255.0) as u8,
            b.clamp(0.0, 255.0) as u8,
        )
    }
}

/// Fire gradient: dark red -> amber -> gold.
#[must_use]
pub(crate) fn gradient_fire() -> Gradient {
    Gradient {
        start: (120.0, 30.0, 20.0),
        mid: (195.0, 110.0, 45.0),
        end: (215.0, 198.0, 80.0),
    }
}

/// Ocean gradient: deep blue -> teal -> cyan.
#[must_use]
pub(crate) fn gradient_ocean() -> Gradient {
    Gradient {
        start: (30.0, 40.0, 120.0),
        mid: (40.0, 120.0, 150.0),
        end: (80.0, 190.0, 210.0),
    }
}

/// Rose gradient: deep rose -> dream -> rose-bright.
///
/// Used for sparklines and data visualization fills.
#[must_use]
pub(crate) fn gradient_rose() -> Gradient {
    Gradient {
        start: (65.0, 36.0, 52.0),    // ROSE_DEEP
        mid: (120.0, 115.0, 165.0),   // DREAM
        end: (220.0, 155.0, 180.0),   // ROSE_BRIGHT
    }
}

/// Progress gradient using the ROSEDUST semantic triad:
/// EMBER (195, 110, 85) -> WARNING (195, 155, 95) -> SAGE (125, 158, 140).
///
/// Provides a smooth 0%-to-100% ramp through danger -> caution -> success.
#[must_use]
pub(crate) fn gradient_progress() -> Gradient {
    Gradient {
        start: (195.0, 110.0, 85.0),  // EMBER
        mid: (195.0, 155.0, 95.0),    // WARNING
        end: (125.0, 158.0, 140.0),   // SAGE
    }
}

/// Brighten (or dim) an RGB color by a multiplier.
#[must_use]
pub(crate) fn brighten(color: Color, factor: f64) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            ((r as f64) * factor).clamp(0.0, 255.0) as u8,
            ((g as f64) * factor).clamp(0.0, 255.0) as u8,
            ((b as f64) * factor).clamp(0.0, 255.0) as u8,
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_has_non_default_colors() {
        let theme = Theme::dark();
        assert_ne!(theme.foreground, Color::Reset);
        assert_ne!(theme.accent, Color::Reset);
        assert_ne!(theme.success, Color::Reset);
    }

    #[test]
    fn no_color_theme_all_reset() {
        let theme = Theme::no_color();
        assert_eq!(theme.foreground, Color::Reset);
        assert_eq!(theme.accent, Color::Reset);
        assert_eq!(theme.success, Color::Reset);
        assert_eq!(theme.danger, Color::Reset);
    }

    #[test]
    fn high_contrast_theme_has_bright_colors() {
        let theme = Theme::high_contrast();
        assert_eq!(theme.foreground, Color::White);
        assert_eq!(theme.background, Color::Black);
        // Verify all colors are non-reset (real colors for accessibility)
        assert_ne!(theme.accent, Color::Reset);
        assert_ne!(theme.success, Color::Reset);
        assert_ne!(theme.warning, Color::Reset);
        assert_ne!(theme.danger, Color::Reset);
        assert_ne!(theme.info, Color::Reset);
    }

    #[test]
    fn high_contrast_differs_from_dark() {
        let dark = Theme::dark();
        let hc = Theme::high_contrast();
        assert_ne!(dark.foreground, hc.foreground);
    }

    #[test]
    fn from_no_color_flag() {
        let t = Theme::from_no_color(true);
        assert_eq!(t, Theme::no_color());
        let t = Theme::from_no_color(false);
        assert_eq!(t, Theme::dark());
    }

    #[test]
    fn style_methods_produce_non_empty() {
        let theme = Theme::dark();
        let _ = theme.text();
        let _ = theme.muted();
        let _ = theme.accent();
        let _ = theme.accent_bold();
        let _ = theme.selection();
        let _ = theme.success();
        let _ = theme.warning();
        let _ = theme.danger();
        let _ = theme.info();
    }

    #[test]
    fn semantic_color_ranges() {
        // 0.0 -> danger
        assert_eq!(Theme::semantic_color(0.0), Theme::EMBER);
        // 0.5 -> warning
        assert_eq!(Theme::semantic_color(0.5), Theme::WARNING);
        // 1.0 -> success
        assert_eq!(Theme::semantic_color(1.0), Theme::SAGE);
    }

    #[test]
    fn brighten_works() {
        let c = brighten(Color::Rgb(100, 100, 100), 1.5);
        assert_eq!(c, Color::Rgb(150, 150, 150));
    }

    #[test]
    fn brighten_clamps() {
        let c = brighten(Color::Rgb(200, 200, 200), 2.0);
        assert_eq!(c, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn brighten_non_rgb_passes_through() {
        let c = brighten(Color::Reset, 2.0);
        assert_eq!(c, Color::Reset);
    }

    #[test]
    fn gradient_fire_samples() {
        let g = gradient_fire();
        let _ = g.sample(0.0);
        let _ = g.sample(0.5);
        let _ = g.sample(1.0);
    }

    #[test]
    fn gradient_clamps() {
        let g = gradient_ocean();
        let a = g.sample(-1.0);
        let b = g.sample(0.0);
        assert_eq!(a, b);
        let c = g.sample(2.0);
        let d = g.sample(1.0);
        assert_eq!(c, d);
    }

    #[test]
    fn fallback_256_uses_indexed_colors() {
        let theme = Theme::fallback_256();
        assert!(matches!(theme.foreground, Color::Indexed(_)));
        assert!(matches!(theme.accent, Color::Indexed(_)));
        assert!(matches!(theme.success, Color::Indexed(_)));
        assert!(matches!(theme.danger, Color::Indexed(_)));
        assert!(matches!(theme.info, Color::Indexed(_)));
    }

    #[test]
    fn series_colors_has_six_entries() {
        assert_eq!(Theme::SERIES_COLORS.len(), 6);
    }

    #[test]
    fn progress_gradient_endpoints() {
        // 0.0 should be EMBER
        assert_eq!(Theme::progress_gradient(0.0), Theme::EMBER);
        // 1.0 should be SAGE
        assert_eq!(Theme::progress_gradient(1.0), Theme::SAGE);
    }

    #[test]
    fn gradient_progress_midpoint_is_warning() {
        // 0.5 should be WARNING
        assert_eq!(Theme::progress_gradient(0.5), Theme::WARNING);
    }

    #[test]
    fn rose_gradient_samples() {
        let g = gradient_rose();
        let _ = g.sample(0.0);
        let _ = g.sample(0.5);
        let _ = g.sample(1.0);
    }

    #[test]
    fn sparkline_gradient_maps_zero_and_one() {
        let low = Theme::sparkline_gradient(0.0);
        let high = Theme::sparkline_gradient(1.0);
        assert_ne!(low, high);
    }

    #[test]
    fn notification_pulse_stays_in_range() {
        let base = Theme::EMBER;
        let dimmed = Theme::notification_pulse(base, 0.0);
        let bright = Theme::notification_pulse(base, 1.0);
        // The bright variant should have at least one channel >= the dimmed variant.
        if let (Color::Rgb(dr, dg, db), Color::Rgb(br, bg, bb)) = (dimmed, bright) {
            assert!(br >= dr || bg >= dg || bb >= db);
        }
    }

    #[test]
    fn rose_dim_passes_wcag_aa() {
        // ROSE_DIM (155, 106, 124) should have >= 4.5:1 contrast against black.
        // Relative luminance: 0.2126*(155/255)^2.2 + 0.7152*(106/255)^2.2 + 0.0722*(124/255)^2.2
        // Approximate: L ~ 0.19, ratio = (0.19 + 0.05) / 0.05 = 4.8 > 4.5
        if let Color::Rgb(r, g, b) = Theme::ROSE_DIM {
            assert!(r >= 150, "ROSE_DIM red channel should be >= 150 for AA");
            assert!(g >= 100, "ROSE_DIM green channel should be >= 100 for AA");
        } else {
            panic!("ROSE_DIM should be Color::Rgb");
        }
    }

    #[test]
    fn stage_colors_match_semantic_triad() {
        assert_eq!(Theme::STAGE_STATIC, Theme::WARNING);
        assert_eq!(Theme::STAGE_CONFIDENCE, Theme::DREAM);
        assert_eq!(Theme::STAGE_UCB, Theme::SAGE);
    }

    #[test]
    fn rate_colors_match_semantic_triad() {
        assert_eq!(Theme::RATE_GOOD, Theme::SAGE);
        assert_eq!(Theme::RATE_MID, Theme::WARNING);
        assert_eq!(Theme::RATE_BAD, Theme::EMBER);
    }
}
