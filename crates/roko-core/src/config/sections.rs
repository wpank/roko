//! `[sections]`: the prompt sections the section bandit never leaves out
//! (S02 L9).
//!
//! On the learned arm of its chain, an attempt's prompt may leave a
//! droppable section out, so that the section's effect on verified passes
//! can be measured. The role's identity, the task spec, the verify
//! instructions, the tool policy and the safety rules are always pinned;
//! this table pins more.
//!
//! ```toml
//! [sections]
//! pinned = ["conventions"]
//! ```

use serde::{Deserialize, Serialize};

/// `[sections]`: prompt sections pinned on top of the built-in ones.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SectionsConfig {
    /// Canonical section names (`conventions`, `domain_context`, ...) the
    /// section bandit never leaves out of a prompt.
    pub pinned: Vec<String>,
}
