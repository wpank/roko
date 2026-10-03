//! `[experiments]`: how a run randomises its learning loops on real work
//! (S02 §4.1, decision 4115).
//!
//! Graph dispatch draws each task chain's arm set at attempt open: whether
//! the chain's prompts get knowledge, playbooks, the sections a bandit may
//! drop, a prompt variant, and the placebo's arm. This table turns that off
//! for must-succeed runs, or pins arms.
//!
//! ```toml
//! [experiments]
//! maximize = true
//! force_arms = { knowledge = "default" }
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `[experiments]`: maximize mode and forced arms.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExperimentsConfig {
    /// Maximize mode, for runs that must succeed: every holdout rate and the
    /// routing exploration rate are 0, so no loop is withheld and no route
    /// explores. Decisions are still logged. `roko plan run --no-holdout`
    /// turns it on for one run.
    pub maximize: bool,
    /// Layers pinned to an arm, by layer name (`knowledge`) or loop id
    /// (`L-know`): `learned`, `default` or `global_off`. Estimates leave the
    /// chains of such a run out.
    pub force_arms: BTreeMap<String, String>,
}
