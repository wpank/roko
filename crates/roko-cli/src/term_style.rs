//! One terminal style for roko's command output (gap-cf3940).
//!
//! [`Style`] says how one output stream looks: whether it gets colour, how
//! many colours the terminal takes, whether live output may redraw lines in
//! place, Unicode or ASCII glyphs, and how wide the terminal is.
//! [`Style::stdout`] and [`Style::stderr`] read the process's `--color`
//! choice ([`set_color_choice`]) and its environment.
//!
//! Colour follows, in order: `--color always|never`; `NO_COLOR` set and not
//! empty turns it off; `CLICOLOR_FORCE` set and not `0` turns it on;
//! `CLICOLOR=0` and `TERM=dumb` turn it off; otherwise the stream must be a
//! terminal. Live redraw needs a terminal, a `TERM` other than `dumb`, and no
//! `CI`. Without colour or redraw the words and their order stay the same.
//!
//! The colour roles, glyphs and number formats are roko's design system's
//! (`tmp/hermes-roko-v8/design/roko-design-system.md`, §5.3 and §6.3): four
//! signal colours (live blue, pass green, you orange, fail red) in mid-tones
//! that read on dark and light terminals, in truecolor, xterm-256 or the 16
//! ANSI colours as the terminal has them; faint for context; the terminal's
//! own foreground for everything else; and black glass behind the figures
//! that change while a run is live. No other background is ever set.

use std::fmt::Display;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use unicode_width::UnicodeWidthChar;

/// What `--color` asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorChoice {
    /// Decide from the environment and the stream.
    #[default]
    Auto,
    /// Always colour.
    Always,
    /// Never colour.
    Never,
}

static COLOR_CHOICE: AtomicU8 = AtomicU8::new(0);

/// Record the process's `--color` choice, once, before any output.
pub fn set_color_choice(choice: ColorChoice) {
    let value = match choice {
        ColorChoice::Auto => 0,
        ColorChoice::Always => 1,
        ColorChoice::Never => 2,
    };
    COLOR_CHOICE.store(value, Ordering::Relaxed);
}

/// The process's `--color` choice.
#[must_use]
pub fn color_choice() -> ColorChoice {
    match COLOR_CHOICE.load(Ordering::Relaxed) {
        1 => ColorChoice::Always,
        2 => ColorChoice::Never,
        _ => ColorChoice::Auto,
    }
}

/// The environment variables the decisions read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TermEnv {
    /// `NO_COLOR`.
    pub no_color: Option<String>,
    /// `CLICOLOR_FORCE`.
    pub clicolor_force: Option<String>,
    /// `CLICOLOR`.
    pub clicolor: Option<String>,
    /// `TERM`.
    pub term: Option<String>,
    /// `COLORTERM`: `truecolor` or `24bit` when the terminal takes 24-bit
    /// colour.
    pub colorterm: Option<String>,
    /// `CI`.
    pub ci: Option<String>,
    /// `COLUMNS`: the width a pipe's output is laid out for.
    pub columns: Option<String>,
}

impl TermEnv {
    /// This process's environment.
    #[must_use]
    pub fn from_process() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            no_color: var("NO_COLOR"),
            clicolor_force: var("CLICOLOR_FORCE"),
            clicolor: var("CLICOLOR"),
            term: var("TERM"),
            colorterm: var("COLORTERM"),
            ci: var("CI"),
            columns: var("COLUMNS"),
        }
    }

    /// How many colours the terminal takes.
    #[must_use]
    pub fn depth(&self) -> Depth {
        if matches!(self.colorterm.as_deref(), Some("truecolor" | "24bit")) {
            Depth::True
        } else if self
            .term
            .as_deref()
            .is_some_and(|term| term.contains("256color"))
        {
            Depth::Ansi256
        } else {
            Depth::Ansi16
        }
    }

    fn dumb(&self) -> bool {
        self.term.as_deref() == Some("dumb")
    }

    fn in_ci(&self) -> bool {
        self.ci
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value != "0" && value != "false")
    }
}

/// Whether output to a stream gets colour (see the module docs for the order).
#[must_use]
pub fn colour_for(choice: ColorChoice, env: &TermEnv, is_terminal: bool) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if env
                .no_color
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            {
                false
            } else if env
                .clicolor_force
                .as_deref()
                .is_some_and(|value| value != "0")
            {
                true
            } else if env.clicolor.as_deref() == Some("0") || env.dumb() {
                false
            } else {
                is_terminal
            }
        }
    }
}

/// Whether live output may redraw lines in place, rather than append one line
/// per event.
#[must_use]
pub fn live_for(env: &TermEnv, is_terminal: bool) -> bool {
    is_terminal && !env.dumb() && !env.in_ci()
}

/// How many colours a terminal takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// 24-bit colour (`COLORTERM=truecolor`).
    True,
    /// xterm's 256 colours.
    Ansi256,
    /// The 16 ANSI colours.
    Ansi16,
}

/// A colour role of roko's design system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The ◆ of `◆ roko`: live's blue.
    Mark,
    /// roko is working.
    Live,
    /// Passed.
    Pass,
    /// Failed.
    Fail,
    /// The person's turn: the Next line, a waiting approval.
    You,
    /// Context, times, hints, anything waiting (faint).
    Dim,
}

impl Role {
    const fn sgr(self, depth: Depth) -> &'static str {
        match (self, depth) {
            (Self::Mark | Self::Live, Depth::True) => "38;2;68;121;255",
            (Self::Mark | Self::Live, Depth::Ansi256) => "38;5;69",
            (Self::Mark | Self::Live, Depth::Ansi16) => "34",
            (Self::Pass, Depth::True) => "38;2;28;154;85",
            (Self::Pass, Depth::Ansi256) => "38;5;29",
            (Self::Pass, Depth::Ansi16) => "32",
            (Self::You, Depth::True) => "38;2;236;95;24",
            (Self::You, Depth::Ansi256) => "38;5;202",
            (Self::You, Depth::Ansi16) => "33",
            (Self::Fail, Depth::True) => "38;2;224;69;60",
            (Self::Fail, Depth::Ansi256) => "38;5;160",
            (Self::Fail, Depth::Ansi16) => "31",
            (Self::Dim, _) => "2",
        }
    }
}

/// A glyph of roko's output, with its ASCII form for `TERM=dumb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    /// The roko mark.
    Mark,
    /// Passed.
    Pass,
    /// Failed.
    Fail,
    /// Trying again.
    Retry,
    /// Running.
    Running,
    /// Waiting to start.
    Waiting,
    /// Stopped.
    Stopped,
    /// roko itself stopped; it can resume.
    Interrupted,
    /// Finished, but no check ran.
    NotChecked,
    /// Not needed, or not run.
    Skipped,
    /// Next.
    Next,
    /// Between items on a line.
    Dot,
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_ASCII: [&str; 4] = ["-", "\\", "|", "/"];

/// How one output stream looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    colour: bool,
    depth: Depth,
    live: bool,
    unicode: bool,
    width: usize,
    height: usize,
}

/// The narrowest layout roko draws.
pub const MIN_WIDTH: usize = 60;
/// The widest layout roko draws: it fits 80 columns, and uses up to 100.
pub const MAX_WIDTH: usize = 100;
/// The width roko assumes when it can't ask the terminal.
pub const DEFAULT_WIDTH: usize = 100;
/// The height roko assumes when it can't ask the terminal.
pub const DEFAULT_HEIGHT: usize = 24;

impl Style {
    /// Standard output's style.
    #[must_use]
    pub fn stdout() -> Self {
        Self::for_stream(std::io::stdout().is_terminal())
    }

    /// Standard error's style: live output goes there.
    #[must_use]
    pub fn stderr() -> Self {
        Self::for_stream(std::io::stderr().is_terminal())
    }

    fn for_stream(is_terminal: bool) -> Self {
        let env = TermEnv::from_process();
        let (width, height) = if is_terminal {
            crossterm::terminal::size().map_or((DEFAULT_WIDTH, DEFAULT_HEIGHT), |(cols, rows)| {
                (usize::from(cols), usize::from(rows))
            })
        } else {
            let cols = env
                .columns
                .as_deref()
                .and_then(|cols| cols.parse().ok())
                .unwrap_or(DEFAULT_WIDTH);
            (cols, DEFAULT_HEIGHT)
        };
        Self {
            colour: colour_for(color_choice(), &env, is_terminal),
            depth: env.depth(),
            live: live_for(&env, is_terminal),
            unicode: !env.dumb(),
            width: width.clamp(MIN_WIDTH, MAX_WIDTH),
            height: height.max(DEFAULT_HEIGHT / 2),
        }
    }

    /// No colour, no redraw, Unicode glyphs, [`DEFAULT_WIDTH`] columns: what a
    /// pipe gets, and what tests compare against.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            colour: false,
            depth: Depth::Ansi16,
            live: false,
            unicode: true,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }

    /// The same style, with colour on or off.
    #[must_use]
    pub const fn with_colour(mut self, colour: bool) -> Self {
        self.colour = colour;
        self
    }

    /// The same style, with `depth` colours.
    #[must_use]
    pub const fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }

    /// The same style, redrawing in place or appending lines.
    #[must_use]
    pub const fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// The same style, with Unicode or ASCII glyphs.
    #[must_use]
    pub const fn with_unicode(mut self, unicode: bool) -> Self {
        self.unicode = unicode;
        self
    }

    /// The same style, laid out for `width` columns (clamped to
    /// [`MIN_WIDTH`]..=[`MAX_WIDTH`]).
    #[must_use]
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width.clamp(MIN_WIDTH, MAX_WIDTH);
        self
    }

    /// The same style, for a terminal of `height` rows.
    #[must_use]
    pub const fn with_height(mut self, height: usize) -> Self {
        self.height = height;
        self
    }

    /// Whether this stream gets colour.
    #[must_use]
    pub const fn colour(self) -> bool {
        self.colour
    }

    /// Whether live output may redraw lines in place.
    #[must_use]
    pub const fn live(self) -> bool {
        self.live
    }

    /// How many columns the layout may use.
    #[must_use]
    pub const fn width(self) -> usize {
        self.width
    }

    /// How many rows the terminal has.
    #[must_use]
    pub const fn height(self) -> usize {
        self.height
    }

    /// `text` in `role`'s colour.
    #[must_use]
    pub fn paint(self, role: Role, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[{}m{text}\x1b[0m", role.sgr(self.depth))
        } else {
            text.to_string()
        }
    }

    /// `text` in `role`'s colour, bold.
    #[must_use]
    pub fn paint_bold(self, role: Role, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[1;{}m{text}\x1b[0m", role.sgr(self.depth))
        } else {
            text.to_string()
        }
    }

    /// `text` in bold.
    #[must_use]
    pub fn bold(self, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[1m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// A figure that changes while a run is live, on black glass: bold bone
    /// digits on a near-black chip, a space either side (reverse video in 16
    /// colours). Without colour, the figure alone.
    #[must_use]
    pub fn glass(self, text: impl Display) -> String {
        if !self.colour {
            return text.to_string();
        }
        let sgr = match self.depth {
            Depth::True => "1;38;2;242;240;233;48;2;11;12;13",
            Depth::Ansi256 => "1;38;5;255;48;5;232",
            Depth::Ansi16 => "1;7",
        };
        format!("\x1b[{sgr}m {text} \x1b[0m")
    }

    /// `glyph` in this stream's glyph set.
    #[must_use]
    pub const fn glyph(self, glyph: Glyph) -> &'static str {
        match (glyph, self.unicode) {
            (Glyph::Mark, true) => "◆",
            (Glyph::Mark | Glyph::Running, false) => "*",
            (Glyph::Pass, true) => "✓",
            (Glyph::Pass, false) => "ok",
            (Glyph::Fail, true) => "✗",
            (Glyph::Fail, false) => "x",
            (Glyph::Retry, true) => "↻",
            (Glyph::Retry, false) => "~",
            (Glyph::Running, true) => "●",
            (Glyph::Waiting, true) => "○",
            (Glyph::Waiting, false) => "o",
            (Glyph::Stopped, true) => "■",
            (Glyph::Stopped, false) => "#",
            (Glyph::Interrupted, true) => "‖",
            (Glyph::Interrupted, false) => "||",
            (Glyph::NotChecked, _) => "?",
            (Glyph::Skipped, true) => "⊘",
            (Glyph::Skipped, false) => "-",
            (Glyph::Next, true) => "→",
            (Glyph::Next, false) => "->",
            (Glyph::Dot, true) => "·",
            (Glyph::Dot, false) => "-",
        }
    }

    /// Frame `frame` of the spinner shown beside running work.
    #[must_use]
    pub fn spinner(self, frame: usize) -> &'static str {
        if self.unicode {
            SPINNER[frame % SPINNER.len()]
        } else {
            SPINNER_ASCII[frame % SPINNER_ASCII.len()]
        }
    }
}

/// How long something took: `5.3s` under ten seconds, `42s` under a minute,
/// `4m 05s` under an hour, then `1h 12m`.
#[must_use]
pub fn duration(elapsed: Duration) -> String {
    let tenths = elapsed.as_millis() / 100;
    if tenths < 100 {
        return format!("{}.{}s", tenths / 10, tenths % 10);
    }
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// A clock on the glass: `0:42`, `4:05`, then `1H 12M`.
#[must_use]
pub fn clock(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    if secs < 3600 {
        format!("{}:{:02}", secs / 60, secs % 60)
    } else {
        format!("{}H {:02}M", secs / 3600, (secs % 3600) / 60)
    }
}

/// A dollar amount, always two decimals: `$0.44`; `<$0.01` below a cent
/// when it isn't zero.
#[must_use]
pub fn money(usd: f64) -> String {
    if usd > 0.0 && usd < 0.005 {
        "<$0.01".to_string()
    } else {
        format!("${:.2}", usd.max(0.0))
    }
}

/// `count` and the noun: `1 try`, `2 tries`.
#[must_use]
pub fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// How many columns `text` takes on a terminal, ANSI escapes left out.
#[must_use]
pub fn visible_width(text: &str) -> usize {
    let mut width = 0;
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // A CSI sequence: ESC '[' params, then a letter.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += ch.width().unwrap_or(0);
        }
    }
    width
}

/// `text` without its ANSI escapes.
#[must_use]
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// `text` cut to `max` columns, ending in `…` when it was longer. Plain text
/// only: cut before styling.
#[must_use]
pub fn truncate(text: &str, max: usize) -> String {
    if visible_width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if width + w + 1 > max {
            break;
        }
        out.push(ch);
        width += w;
    }
    out.push('…');
    out
}

/// `text` padded with spaces to `width` columns (ANSI escapes don't count).
#[must_use]
pub fn pad(text: &str, width: usize) -> String {
    let gap = width.saturating_sub(visible_width(text));
    format!("{text}{}", " ".repeat(gap))
}

/// `text` right-aligned in `width` columns (ANSI escapes don't count).
#[must_use]
pub fn pad_left(text: &str, width: usize) -> String {
    let gap = width.saturating_sub(visible_width(text));
    format!("{}{text}", " ".repeat(gap))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> TermEnv {
        let mut env = TermEnv::default();
        for (name, value) in pairs {
            let value = Some((*value).to_string());
            match *name {
                "NO_COLOR" => env.no_color = value,
                "CLICOLOR_FORCE" => env.clicolor_force = value,
                "CLICOLOR" => env.clicolor = value,
                "TERM" => env.term = value,
                "COLORTERM" => env.colorterm = value,
                "CI" => env.ci = value,
                other => panic!("unknown variable {other}"),
            }
        }
        env
    }

    #[test]
    fn colour_follows_the_flag_then_no_color_then_the_terminal() {
        let auto = ColorChoice::Auto;
        // The flag wins over everything.
        assert!(colour_for(
            ColorChoice::Always,
            &env(&[("NO_COLOR", "1")]),
            false
        ));
        assert!(!colour_for(
            ColorChoice::Never,
            &env(&[("CLICOLOR_FORCE", "1")]),
            true
        ));
        // NO_COLOR, when not empty.
        assert!(!colour_for(auto, &env(&[("NO_COLOR", "1")]), true));
        assert!(colour_for(auto, &env(&[("NO_COLOR", "")]), true));
        // CLICOLOR_FORCE colours a pipe; CLICOLOR=0 and TERM=dumb turn colour off.
        assert!(colour_for(auto, &env(&[("CLICOLOR_FORCE", "1")]), false));
        assert!(!colour_for(auto, &env(&[("CLICOLOR_FORCE", "0")]), false));
        assert!(!colour_for(auto, &env(&[("CLICOLOR", "0")]), true));
        assert!(!colour_for(auto, &env(&[("TERM", "dumb")]), true));
        // Otherwise: a terminal gets colour, a pipe doesn't, CI or not.
        assert!(colour_for(auto, &env(&[("TERM", "xterm-256color")]), true));
        assert!(!colour_for(auto, &env(&[]), false));
        assert!(colour_for(auto, &env(&[("CI", "true")]), true));
    }

    #[test]
    fn live_redraw_needs_a_terminal_and_no_ci() {
        assert!(live_for(&env(&[("TERM", "xterm-256color")]), true));
        assert!(!live_for(&env(&[]), false));
        assert!(!live_for(&env(&[("CI", "true")]), true));
        assert!(live_for(&env(&[("CI", "false")]), true));
        assert!(!live_for(&env(&[("TERM", "dumb")]), true));
    }

    #[test]
    fn terminals_say_how_many_colours_they_take() {
        assert_eq!(env(&[("COLORTERM", "truecolor")]).depth(), Depth::True);
        assert_eq!(env(&[("TERM", "xterm-256color")]).depth(), Depth::Ansi256);
        assert_eq!(env(&[("TERM", "xterm")]).depth(), Depth::Ansi16);
    }

    #[test]
    fn plain_text_carries_no_escape_codes() {
        let plain = Style::plain();
        for text in [
            plain.paint(Role::Pass, "ok"),
            plain.paint_bold(Role::Fail, "no"),
            plain.bold("b"),
            plain.glass("1/3"),
        ] {
            assert!(!text.contains('\x1b'), "{text:?}");
        }
        let colour = plain.with_colour(true);
        assert_eq!(colour.paint(Role::Pass, "ok"), "\x1b[32mok\x1b[0m");
        assert_eq!(colour.glass("1/3"), "\x1b[1;7m 1/3 \x1b[0m");
        assert_eq!(visible_width(&colour.glass("1/3")), 5);
        assert_eq!(
            colour.with_depth(Depth::True).paint(Role::Live, "●"),
            "\x1b[38;2;68;121;255m●\x1b[0m"
        );
        assert_eq!(
            colour
                .with_depth(Depth::Ansi256)
                .paint_bold(Role::You, "Next"),
            "\x1b[1;38;5;202mNext\x1b[0m"
        );
    }

    #[test]
    fn durations_read_at_a_glance() {
        assert_eq!(duration(Duration::from_millis(0)), "0.0s");
        assert_eq!(duration(Duration::from_millis(5_349)), "5.3s");
        assert_eq!(duration(Duration::from_millis(9_999)), "9.9s");
        assert_eq!(duration(Duration::from_millis(16_900)), "16s");
        assert_eq!(duration(Duration::from_secs(42)), "42s");
        assert_eq!(duration(Duration::from_secs(60)), "1m 00s");
        assert_eq!(duration(Duration::from_secs(245)), "4m 05s");
        assert_eq!(duration(Duration::from_secs(4_320)), "1h 12m");
        assert_eq!(clock(Duration::from_secs(42)), "0:42");
        assert_eq!(clock(Duration::from_secs(245)), "4:05");
        assert_eq!(clock(Duration::from_secs(4_320)), "1H 12M");
    }

    #[test]
    fn money_keeps_two_decimals() {
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(0.4363), "$0.44");
        assert_eq!(money(0.0042), "<$0.01");
        assert_eq!(money(3.0), "$3.00");
        assert_eq!(money(-0.0), "$0.00");
    }

    #[test]
    fn widths_ignore_escapes_and_truncation_ends_in_an_ellipsis() {
        assert_eq!(visible_width("\x1b[1;34m◆\x1b[0m roko"), 6);
        assert_eq!(strip_ansi("\x1b[1;34m◆\x1b[0m roko"), "◆ roko");
        assert_eq!(
            truncate("Add the command-line entry point", 12),
            "Add the com…"
        );
        assert_eq!(truncate("short", 12), "short");
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad_left("ab", 4), "  ab");
        assert_eq!(plural(1, "try", "tries"), "1 try");
        assert_eq!(plural(2, "try", "tries"), "2 tries");
    }

    #[test]
    fn a_dumb_terminal_gets_ascii_glyphs() {
        let ascii = Style::plain().with_unicode(false);
        assert_eq!(ascii.glyph(Glyph::Pass), "ok");
        assert_eq!(ascii.glyph(Glyph::Next), "->");
        assert_eq!(ascii.spinner(1), "\\");
        assert_eq!(Style::plain().glyph(Glyph::Pass), "✓");
        assert_eq!(Style::plain().glyph(Glyph::Waiting), "○");
    }
}
