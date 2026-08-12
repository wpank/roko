//! IFC capability set — maps TaintLevel classifications to allowed operations.
//!
//! This module implements the access-control half of the Information Flow
//! Control lattice. [`TaintLevel`] answers *how secret* the data is;
//! [`capabilities_for_taint`] answers *what operations* are permitted on
//! data at that classification level.
//!
//! # Rules
//!
//! | TaintLevel    | Allowed operations                        |
//! |---------------|-------------------------------------------|
//! | Public        | All capabilities (Read, Write, Execute, Network, FileSystem, Secret) |
//! | Internal      | All except Secret                         |
//! | Confidential  | Read + Write only                         |
//! | Secret        | Read only                                 |
//!
//! Capabilities can only be *narrowed* by moving to a higher classification
//! level — data at `Secret` level never receives `Network` or `Execute`
//! access, even temporarily.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TaintLevel;

// ─── Capability ──────────────────────────────────────────────────────────────

/// An individual privilege that code may need to exercise on a signal.
///
/// Capabilities are granted as a set by [`capabilities_for_taint`] based on
/// the signal's [`TaintLevel`] classification. A call-site that needs, say,
/// `Network` access on a `Confidential` signal will be denied because
/// `capabilities_for_taint(TaintLevel::Confidential)` does not include
/// `Network`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Read the signal's content (always permitted at all levels).
    Read,
    /// Write or mutate derived data.
    Write,
    /// Execute code or spawn processes using this signal as input.
    Execute,
    /// Make outbound network calls using this signal's content.
    Network,
    /// Read from or write to the filesystem using this signal's content.
    FileSystem,
    /// Access or propagate secrets derived from this signal.
    Secret,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "Read"),
            Self::Write => write!(f, "Write"),
            Self::Execute => write!(f, "Execute"),
            Self::Network => write!(f, "Network"),
            Self::FileSystem => write!(f, "FileSystem"),
            Self::Secret => write!(f, "Secret"),
        }
    }
}

// ─── CapabilitySet ───────────────────────────────────────────────────────────

/// A set of capabilities granted to a signal at a given [`TaintLevel`].
///
/// Constructed by [`capabilities_for_taint`]; callers check membership with
/// [`CapabilitySet::contains`] before performing privileged operations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    inner: HashSet<Capability>,
}

impl<I: IntoIterator<Item = Capability>> From<I> for CapabilitySet {
    fn from(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl CapabilitySet {
    /// Returns `true` if `cap` is in the set.
    #[must_use]
    pub fn contains(&self, cap: Capability) -> bool {
        self.inner.contains(&cap)
    }

    /// Returns the number of capabilities in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate over all capabilities in the set (order is unspecified).
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.inner.iter()
    }
}

impl fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inner.is_empty() {
            return write!(f, "CapabilitySet(empty)");
        }
        // Sort for deterministic display output.
        let mut caps: Vec<&Capability> = self.inner.iter().collect();
        caps.sort_by_key(|c| format!("{c}"));
        write!(f, "CapabilitySet({{")?;
        let mut first = true;
        for cap in caps {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{cap}")?;
            first = false;
        }
        write!(f, "}})")
    }
}

// ─── capabilities_for_taint ──────────────────────────────────────────────────

/// Return the set of capabilities permitted for data at `level`.
///
/// The mapping enforces the principle of least privilege: as the classification
/// level rises (data becomes more sensitive), the allowed operations shrink.
///
/// ```
/// use roko_core::capabilities::{Capability, capabilities_for_taint};
/// use roko_core::TaintLevel;
///
/// // Public data has full access.
/// let public_caps = capabilities_for_taint(TaintLevel::Public);
/// assert!(public_caps.contains(Capability::Secret));
/// assert!(public_caps.contains(Capability::Network));
///
/// // Secret data is read-only.
/// let secret_caps = capabilities_for_taint(TaintLevel::Secret);
/// assert!(secret_caps.contains(Capability::Read));
/// assert!(!secret_caps.contains(Capability::Network));
/// assert!(!secret_caps.contains(Capability::Execute));
/// ```
#[must_use]
pub fn capabilities_for_taint(level: TaintLevel) -> CapabilitySet {
    let caps: &[Capability] = match level {
        TaintLevel::Public => &[
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ],
        TaintLevel::Internal => &[
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            // Secret is withheld — internal signals must not propagate secrets.
        ],
        TaintLevel::Confidential => &[
            Capability::Read,
            Capability::Write,
            // Execute, Network, FileSystem, Secret withheld.
        ],
        TaintLevel::Secret => &[
            Capability::Read,
            // Everything else withheld — read-only access only.
        ],
    };
    CapabilitySet::from(caps.iter().copied())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_has_all_capabilities() {
        let caps = capabilities_for_taint(TaintLevel::Public);
        assert!(caps.contains(Capability::Read));
        assert!(caps.contains(Capability::Write));
        assert!(caps.contains(Capability::Execute));
        assert!(caps.contains(Capability::Network));
        assert!(caps.contains(Capability::FileSystem));
        assert!(caps.contains(Capability::Secret));
        assert_eq!(caps.len(), 6);
    }

    #[test]
    fn internal_has_all_except_secret() {
        let caps = capabilities_for_taint(TaintLevel::Internal);
        assert!(caps.contains(Capability::Read));
        assert!(caps.contains(Capability::Write));
        assert!(caps.contains(Capability::Execute));
        assert!(caps.contains(Capability::Network));
        assert!(caps.contains(Capability::FileSystem));
        assert!(!caps.contains(Capability::Secret));
        assert_eq!(caps.len(), 5);
    }

    #[test]
    fn confidential_has_read_and_write_only() {
        let caps = capabilities_for_taint(TaintLevel::Confidential);
        assert!(caps.contains(Capability::Read));
        assert!(caps.contains(Capability::Write));
        assert!(!caps.contains(Capability::Execute));
        assert!(!caps.contains(Capability::Network));
        assert!(!caps.contains(Capability::FileSystem));
        assert!(!caps.contains(Capability::Secret));
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn secret_has_read_only() {
        let caps = capabilities_for_taint(TaintLevel::Secret);
        assert!(caps.contains(Capability::Read));
        assert!(!caps.contains(Capability::Write));
        assert!(!caps.contains(Capability::Execute));
        assert!(!caps.contains(Capability::Network));
        assert!(!caps.contains(Capability::FileSystem));
        assert!(!caps.contains(Capability::Secret));
        assert_eq!(caps.len(), 1);
    }

    #[test]
    fn capabilities_narrow_as_level_rises() {
        let public_count = capabilities_for_taint(TaintLevel::Public).len();
        let internal_count = capabilities_for_taint(TaintLevel::Internal).len();
        let confidential_count = capabilities_for_taint(TaintLevel::Confidential).len();
        let secret_count = capabilities_for_taint(TaintLevel::Secret).len();
        assert!(public_count > internal_count);
        assert!(internal_count > confidential_count);
        assert!(confidential_count > secret_count);
    }

    #[test]
    fn capability_set_is_empty_works() {
        let empty = CapabilitySet::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(!empty.contains(Capability::Read));
    }

    #[test]
    fn capability_set_display_non_empty() {
        let caps = capabilities_for_taint(TaintLevel::Secret);
        let s = caps.to_string();
        assert!(s.contains("Read"));
        assert!(s.starts_with("CapabilitySet("));
    }

    #[test]
    fn capability_set_display_empty() {
        let empty = CapabilitySet::default();
        assert_eq!(empty.to_string(), "CapabilitySet(empty)");
    }

    #[test]
    fn capability_display_names() {
        assert_eq!(Capability::Read.to_string(), "Read");
        assert_eq!(Capability::Write.to_string(), "Write");
        assert_eq!(Capability::Execute.to_string(), "Execute");
        assert_eq!(Capability::Network.to_string(), "Network");
        assert_eq!(Capability::FileSystem.to_string(), "FileSystem");
        assert_eq!(Capability::Secret.to_string(), "Secret");
    }

    #[test]
    fn capability_set_iter_covers_all() {
        let caps = capabilities_for_taint(TaintLevel::Public);
        let collected: HashSet<_> = caps.iter().copied().collect();
        assert_eq!(collected.len(), 6);
    }
}
