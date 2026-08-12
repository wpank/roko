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

    /// Return the intersection of `self` and `other` (most-restrictive wins).
    #[must_use]
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            inner: self.inner.intersection(&other.inner).copied().collect(),
        }
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

// ─── Three-layer intersection model (E34-T06) ────────────────────────────────

/// What a Cell declares it needs (from the cell manifest).
///
/// The effective set is the intersection of cell, graph, and space layers;
/// cells can only be narrowed by intersection, never widened.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellCapabilities(pub CapabilitySet);

/// What a Graph permits cells to do (from the graph configuration).
///
/// Cells operating within a graph can never exceed what the graph allows,
/// even if the cell's manifest declares a wider set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphAllowList(pub CapabilitySet);

/// What the space operator has granted (from the workspace configuration).
///
/// This is the outermost boundary: the space grant constrains both the cell
/// manifest and the graph config.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceGrant(pub CapabilitySet);

/// Compute the effective [`CapabilitySet`] for a cell operating inside a graph
/// inside a space.
///
/// The result is the intersection of all three layers (most-restrictive wins).
/// This is the canonical entry point for capability enforcement.
#[must_use]
pub fn effective_capabilities(
    cell: &CellCapabilities,
    graph: &GraphAllowList,
    space: &SpaceGrant,
) -> CapabilitySet {
    cell.0.intersect(&graph.0).intersect(&space.0)
}

/// Runtime enforcement interface for capability checks.
///
/// Implementors hold a cached [`CapabilitySet`] and expose a single check
/// method. The trait is intentionally synchronous and pure — no I/O.
pub trait CapabilityCheck {
    /// Return the effective (pre-computed intersection) capability set.
    fn effective(&self) -> &CapabilitySet;

    /// Return `Ok(())` if `cap` is present in the effective set, or an
    /// explanatory error string on denial.
    fn require(&self, cap: Capability) -> Result<(), String> {
        if self.effective().contains(cap) {
            Ok(())
        } else {
            Err(format!(
                "capability {cap:?} is not permitted in the effective set"
            ))
        }
    }

    /// Return `true` if `cap` is present in the effective set.
    fn permits(&self, cap: Capability) -> bool {
        self.effective().contains(cap)
    }
}

/// A cached capability enforcer that stores the pre-computed intersection from
/// the three layers at construction time.
#[derive(Debug, Clone)]
pub struct CachedCapabilityChecker {
    effective: CapabilitySet,
}

impl CachedCapabilityChecker {
    /// Compute and cache the effective capabilities from the three layers.
    #[must_use]
    pub fn new(cell: &CellCapabilities, graph: &GraphAllowList, space: &SpaceGrant) -> Self {
        Self {
            effective: effective_capabilities(cell, graph, space),
        }
    }
}

impl CapabilityCheck for CachedCapabilityChecker {
    fn effective(&self) -> &CapabilitySet {
        &self.effective
    }
}

impl CellCapabilities {
    /// Construct a `CellCapabilities` from a slice of capabilities.
    #[must_use]
    pub fn of(caps: &[Capability]) -> Self {
        Self(CapabilitySet::from(caps.iter().copied()))
    }

    /// The full set — all capabilities granted.
    #[must_use]
    pub fn all() -> Self {
        Self(CapabilitySet::from([
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ]))
    }
}

impl GraphAllowList {
    /// Construct a `GraphAllowList` from a slice of capabilities.
    #[must_use]
    pub fn of(caps: &[Capability]) -> Self {
        Self(CapabilitySet::from(caps.iter().copied()))
    }

    /// The full allow-list — permits everything.
    #[must_use]
    pub fn all() -> Self {
        Self(CapabilitySet::from([
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ]))
    }
}

impl SpaceGrant {
    /// Construct a `SpaceGrant` from a slice of capabilities.
    #[must_use]
    pub fn of(caps: &[Capability]) -> Self {
        Self(CapabilitySet::from(caps.iter().copied()))
    }

    /// The full space grant — no operator restrictions.
    #[must_use]
    pub fn all() -> Self {
        Self(CapabilitySet::from([
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ]))
    }
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
