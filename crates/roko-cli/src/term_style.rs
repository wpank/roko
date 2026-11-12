//! One terminal style for roko's command output (gap-cf3940).
//!
//! [`Style`] says how one output stream looks: whether it gets colour, whether
//! live output may redraw lines in place, Unicode or ASCII glyphs, and how
//! wide the terminal is. [`Style::stdout`] and [`Style::stderr`] read the
//! process's `--color` choice ([`set_color_choice`]) and its environment.
//!
//! Colour follows, in order: `--color always|never`; `NO_COLOR` set and not
//! empty turns it off; `CLICOLOR_FORCE` set and not `0` turns it on;
//! `CLICOLOR=0` and `TERM=dumb` turn it off; otherwise the stream must be a
//! terminal. Live redraw needs a terminal, a `TERM` other than `dumb`, and no
//! `CI`. Without colour or redraw the words and their order stay the same.
//!
//! The colour roles are roko's design system's. Text roles use the 16 ANSI
//! colours, so they follow the person's light or dark terminal theme; the
//! black glass behind changing numbers is the one fixed colour.

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
            ci: var("CI"),
            columns: var("COLUMNS"),
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

/// A colour role of roko's design system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The roko mark.
    Mark,
    /// Something running now.
    Live,
    /// Passed.
    Pass,
    /// Failed.
    Fail,
    /// What the person should do: the next step.
    You,
    /// Something roko learned.
    Learn,
    /// Secondary text.
    Dim,
    /// The quietest text: rule ids, gutters.
    Faint,
}

impl Role {
    fn sgr(self) -> &'static str {
        match self {
            Self::Mark | Self::Live => "35",
            Self::Pass => "32",
            Self::Fail => "31",
            Self::You => "33",
            Self::Learn => "34",
            Self::Dim => "2",
            Self::Faint => "2;90",
        }
    }
}

/// A glyph of roko's output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    /// The roko mark.
    Mark,
    /// Passed.
    Pass,
    /// Failed.
    Fail,
    /// A failed try that roko tries again.
    Retry,
    /// A task waiting to start.
    Waiting,
    /// A task starting (in appended output).
    Start,
    /// Between items on a line.
    Dot,
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_ASCII: [&str; 4] = ["-", "\\", "|", "/"];

/// How one output stream looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    colour: bool,
    live: bool,
    unicode: bool,
    width: usize,
}

/// The narrowest layout roko draws.
pub const MIN_WIDTH: usize = 60;
/// The widest layout roko draws.
pub const MAX_WIDTH: usize = 120;
/// The width roko assumes when it can't ask the terminal.
pub const DEFAULT_WIDTH: usize = 100;

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
        let width = if is_terminal {
            crossterm::terminal::size().map_or(DEFAULT_WIDTH, |(cols, _)| usize::from(cols))
        } else {
            env.columns
                .as_deref()
                .and_then(|cols| cols.parse().ok())
                .unwrap_or(DEFAULT_WIDTH)
        };
        Self {
            colour: colour_for(color_choice(), &env, is_terminal),
            live: live_for(&env, is_terminal),
            unicode: !env.dumb(),
            width: width.clamp(MIN_WIDTH, MAX_WIDTH),
        }
    }

    /// No colour, no redraw, Unicode glyphs, [`DEFAULT_WIDTH`] columns: what a
    /// pipe gets, and what tests compare against.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            colour: false,
            live: false,
            unicode: true,
            width: DEFAULT_WIDTH,
        }
    }

    /// The same style, with colour on or off.
    #[must_use]
    pub const fn with_colour(mut self, colour: bool) -> Self {
        self.colour = colour;
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

    /// `text` in `role`'s colour.
    #[must_use]
    pub fn paint(self, role: Role, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[{}m{text}\x1b[0m", role.sgr())
        } else {
            text.to_string()
        }
    }

    /// `text` in `role`'s colour, bold.
    #[must_use]
    pub fn paint_bold(self, role: Role, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[1;{}m{text}\x1b[0m", role.sgr())
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

    /// A number that changes, on black glass: bright digits on a near-black
    /// chip, a space either side. Without colour, the number alone.
    #[must_use]
    pub fn glass(self, text: impl Display) -> String {
        if self.colour {
            format!("\x1b[48;2;12;11;16m\x1b[1;38;2;239;233;243m {text} \x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// `glyph` in this stream's glyph set.
    #[must_use]
    pub const fn glyph(self, glyph: Glyph) -> &'static str {
        match (glyph, self.unicode) {
            (Glyph::Mark, true) => "◆",
            (Glyph::Mark, false) => "*",
            (Glyph::Pass, true) => "✓",
            (Glyph::Pass, false) => "ok",
            (Glyph::Fail, true) => "✗",
            (Glyph::Fail, false) => "x",
            (Glyph::Retry, true) => "↻",
            (Glyph::Retry, false) => "~",
            (Glyph::Waiting, true) => "·",
            (Glyph::Waiting, false) => ".",
            (Glyph::Start, true) => "▸",
            (Glyph::Start, false) => ">",
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

/// How long something took: `0.4s`, `16.9s`, `4m 05s`, `1h 02m`.
#[must_use]
pub fn duration(elapsed: Duration) -> String {
    let tenths = elapsed.as_millis() / 100;
    if tenths < 600 {
        return format!("{}.{}s", tenths / 10, tenths % 10);
    }
    let secs = elapsed.as_secs();
    if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// A dollar amount: `$0.44`, or four decimals below a cent (`$0.0042`).
#[must_use]
pub fn money(usd: f64) -> String {
    if usd > 0.0 && usd < 0.01 {
        format!("${usd:.4}")
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
            // An SGR sequence: ESC '[' params 'm'.
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
        assert_eq!(visible_width(&colour.glass("1/3")), 5);
    }

    #[test]
    fn durations_read_at_a_glance() {
        assert_eq!(duration(Duration::from_millis(0)), "0.0s");
        assert_eq!(duration(Duration::from_millis(5_349)), "5.3s");
        assert_eq!(duration(Duration::from_millis(16_900)), "16.9s");
        assert_eq!(duration(Duration::from_millis(59_999)), "59.9s");
        assert_eq!(duration(Duration::from_secs(60)), "1m 00s");
        assert_eq!(duration(Duration::from_secs(245)), "4m 05s");
        assert_eq!(duration(Duration::from_secs(3_720)), "1h 02m");
    }

    #[test]
    fn money_keeps_two_decimals_and_four_below_a_cent() {
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(0.4363), "$0.44");
        assert_eq!(money(0.0042), "$0.0042");
        assert_eq!(money(3.0), "$3.00");
        assert_eq!(money(-0.0), "$0.00");
    }

    #[test]
    fn widths_ignore_escapes_and_truncation_ends_in_an_ellipsis() {
        assert_eq!(visible_width("\x1b[1;35m◆\x1b[0m roko"), 6);
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
        assert_eq!(ascii.spinner(1), "\\");
        assert_eq!(Style::plain().glyph(Glyph::Pass), "✓");
    }
}
