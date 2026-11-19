//! A model that hands back while its task's checks still fail, with turns
//! left, goes on in the same run (bug-cbbaa8).
//!
//! The loop ends a run once the model stops calling tools. A plan attempt's
//! implementer that ran its task's checks, saw them fail and stopped then
//! spent its whole try, and the next try started from the tree it left: in
//! Undertow's phase 3 all six strong tries handed back at 13 to 25 of their
//! 120 turns. With a [`HandOffCheck`], the loop asks it when the model hands
//! back; while the checks still fail and the check may send the model back,
//! the loop tells the model what they say, as a user message, and goes on.
//!
//! A model that says the cause lies outside its task's files, with a
//! `Blocked:` line ([`stated_blocker`], bug-3b3bb6), is not sent back: its
//! try ends, and roko hands the failure to the task that owns that file.

use async_trait::async_trait;
use roko_core::tool::ToolContext;

/// The fewest turns a run must have left, past the turn it handed back in,
/// for its model to be sent back.
///
/// That is enough to read what failed, fix it and run the checks again. With
/// fewer, the run ends as before, and the try's checks judge the work it
/// left; a return the model cannot use would only end the run at its turn
/// cap.
pub const HAND_OFF_TURNS_LEFT: usize = 3;

/// What a [`HandOffCheck`] found when its run's model handed back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandOff {
    /// The run ends, as it would without the check: the checks pass, or did
    /// not run. Says which, for the run's live output.
    Ends(String),
    /// The checks still fail: the run goes on, and the model is told this.
    GoesOn(String),
}

/// What a [`super::ToolLoop`] asks when its model hands back (bug-cbbaa8).
///
/// A model hands back when it ends a turn without a tool call. A plan
/// attempt's implementer gets a check that runs its task's checks.
#[async_trait]
pub trait HandOffCheck: Send + Sync + std::fmt::Debug {
    /// Whether the model may be sent back once more. The loop asks before it
    /// runs [`Self::check`].
    fn may_send_back(&self) -> bool;

    /// Run the checks on the work in `ctx`'s checkout.
    async fn check(&self, ctx: &ToolContext) -> HandOff;
}

/// A blocker a model's final message states (bug-3b3bb6). With a line
/// `Blocked: <path> — <why>`, a model that cannot fix a failure inside its
/// task's files names the file outside them that the cause is in, rather than
/// work around it, as phase 5's T4 boosted its demo renders past a level check
/// whose cause was the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatedBlocker {
    /// The file the cause is in, as the line names it.
    pub path: String,
    /// Why, as the line says it; empty when it says nothing more.
    pub why: String,
}

/// The blocker `text`, a model's final message, states ([`StatedBlocker`]):
/// its last line that starts with `Blocked:`, after any list, quote or
/// emphasis mark. Its first word, without quotes or closing punctuation, is
/// the path, which names a file (it has a `/` or a `.`); the reason follows
/// it, after a dash or a colon. `None` without such a line, or when its last
/// one names no file (`Blocked: none`).
#[must_use]
pub fn stated_blocker(text: &str) -> Option<StatedBlocker> {
    let marks = |c: char| c.is_whitespace() || matches!(c, '-' | '*' | '_' | '>' | '#' | '`');
    let rest = text
        .lines()
        .rev()
        .find_map(|line| line.trim_start_matches(marks).strip_prefix("Blocked:"))?;
    let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '*' | '_'));
    let (word, why) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let path = word
        .trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '*' | '_'))
        .trim_end_matches([':', ',', ';', '.', ')']);
    if path.is_empty() || !path.contains(['/', '.']) {
        return None;
    }
    let why = why
        .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '—' | '–' | '-' | ':'))
        .trim();
    Some(StatedBlocker {
        path: path.to_string(),
        why: why.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-3b3bb6: the last `Blocked:` line names the file and why, however
    /// the model marked it up; a line that names no file states no blocker.
    #[test]
    fn a_blocked_line_names_the_file_and_why() {
        let blocker = |path: &str, why: &str| {
            Some(StatedBlocker {
                path: path.to_string(),
                why: why.to_string(),
            })
        };
        for (text, stated) in [
            (
                "The renders are too quiet.\nBlocked: tools/render/main.cpp — it renders init at \
                 -33 dBFS\nLesson: none",
                blocker("tools/render/main.cpp", "it renders init at -33 dBFS"),
            ),
            (
                "**Blocked:** `core/src/presets.cpp` - the presets set the level",
                blocker("core/src/presets.cpp", "the presets set the level"),
            ),
            (
                "- Blocked: src/lib.rs: the trait is defined there.",
                blocker("src/lib.rs", "the trait is defined there."),
            ),
            ("Blocked: CMakeLists.txt", blocker("CMakeLists.txt", "")),
            (
                "Blocked: a/x.cpp — old\nBlocked: b/y.cpp — new",
                blocker("b/y.cpp", "new"),
            ),
            ("Blocked: none", None),
            ("Done.\nLesson: Blocked: means nothing here", None),
            ("Nothing blocked me.", None),
        ] {
            assert_eq!(stated_blocker(text), stated, "{text}");
        }
    }
}
