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
