//! Three-layer capability intersection and IFC capability narrowing.
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
//! | Public        | All seven capabilities                    |
//! | Internal      | All except Secrets                        |
//! | Confidential  | ReadFs + WriteFs only                     |
//! | Secret        | ReadFs only                               |
//!
//! Capabilities can only be *narrowed* by moving to a higher classification
//! level — data at `Secret` level never receives `Network` or `Shell`
//! access, even temporarily.

use std::fmt;
use std::ops::BitAnd;

use serde::{Deserialize, Serialize};

use crate::TaintLevel;

// ─── Capability ──────────────────────────────────────────────────────────────

/// One of the seven independent authorities in the E34 security model.
///
/// The discriminants are bit positions in [`CapabilitySet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Capability {
    /// Read files.
    #[serde(alias = "read", alias = "file_system")]
    ReadFs = 0,
    /// Write files.
    #[serde(alias = "write")]
    WriteFs = 1,
    /// Make outbound network calls.
    Network = 2,
    /// Execute shell commands or subprocesses.
    #[serde(alias = "execute")]
    Shell = 3,
    /// Invoke a language model.
    Llm = 4,
    /// Read protected secrets.
    #[serde(alias = "secret")]
    Secrets = 5,
    /// Publish to or consume from the signal bus.
    Bus = 6,
}

impl Capability {
    const ALL: [Self; 7] = [
        Self::ReadFs,
        Self::WriteFs,
        Self::Network,
        Self::Shell,
        Self::Llm,
        Self::Secrets,
        Self::Bus,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    /// Compatibility alias for the former coarse read capability.
    #[allow(non_upper_case_globals)]
    pub const Read: Self = Self::ReadFs;
    /// Compatibility alias for the former coarse write capability.
    #[allow(non_upper_case_globals)]
    pub const Write: Self = Self::WriteFs;
    /// Compatibility alias for the former execute capability.
    #[allow(non_upper_case_globals)]
    pub const Execute: Self = Self::Shell;
    /// Compatibility alias for the former combined filesystem capability.
    #[allow(non_upper_case_globals)]
    pub const FileSystem: Self = Self::ReadFs;
    /// Compatibility alias for the former singular secret capability.
    #[allow(non_upper_case_globals)]
    pub const Secret: Self = Self::Secrets;
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFs => write!(f, "ReadFs"),
            Self::WriteFs => write!(f, "WriteFs"),
            Self::Network => write!(f, "Network"),
            Self::Shell => write!(f, "Shell"),
            Self::Llm => write!(f, "Llm"),
            Self::Secrets => write!(f, "Secrets"),
            Self::Bus => write!(f, "Bus"),
        }
    }
}

// ─── CapabilitySet ───────────────────────────────────────────────────────────

/// Compact seven-bit capability set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct CapabilitySet(u8);

impl<I: IntoIterator<Item = Capability>> From<I> for CapabilitySet {
    fn from(iter: I) -> Self {
        iter.into_iter()
            .fold(Self::default(), |set, cap| Self(set.0 | cap.bit()))
    }
}

impl CapabilitySet {
    /// Empty capability set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Set containing all seven authorities.
    #[must_use]
    pub const fn all() -> Self {
        Self((1 << Capability::ALL.len()) - 1)
    }

    /// Returns `true` if `cap` is present.
    #[must_use]
    pub const fn has(self, cap: Capability) -> bool {
        self.0 & cap.bit() != 0
    }

    /// Backward-compatible membership spelling.
    #[must_use]
    pub const fn contains(&self, cap: Capability) -> bool {
        self.has(cap)
    }

    /// Returns the number of capabilities in the set.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns `true` if the set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Iterate over all capabilities in the set (order is unspecified).
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        Capability::ALL.iter().filter(|cap| self.has(**cap))
    }

    /// Return the intersection of `self` and `other` (most-restrictive wins).
    #[must_use]
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        *self & *other
    }
}

impl BitAnd for CapabilitySet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "CapabilitySet(empty)");
        }
        write!(f, "CapabilitySet({{")?;
        for (index, cap) in self.iter().enumerate() {
            if index != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{cap}")?;
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
            Capability::ReadFs,
            Capability::WriteFs,
            Capability::Network,
            Capability::Shell,
            Capability::Llm,
            Capability::Secrets,
            Capability::Bus,
        ],
        TaintLevel::Internal => &[
            Capability::ReadFs,
            Capability::WriteFs,
            Capability::Network,
            Capability::Shell,
            Capability::Llm,
            Capability::Bus,
        ],
        TaintLevel::Confidential => &[Capability::ReadFs, Capability::WriteFs],
        TaintLevel::Secret => &[Capability::ReadFs],
    };
    CapabilitySet::from(caps.iter().copied())
}

// ─── Three-layer intersection model (E34-T06) ────────────────────────────────

/// What a Cell declares it needs (from the cell manifest).
///
/// The effective set is the intersection of cell, graph, and space layers;
/// cells can only be narrowed by intersection, never widened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellCapabilities(pub CapabilitySet);

/// What a Graph permits cells to do (from the graph configuration).
///
/// Cells operating within a graph can never exceed what the graph allows,
/// even if the cell's manifest declares a wider set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphAllowList(pub CapabilitySet);

/// What the space operator has granted (from the workspace configuration).
///
/// This is the outermost boundary: the space grant constrains both the cell
/// manifest and the graph config.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    cell.0 & graph.0 & space.0
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
        if self.effective().has(cap) {
            Ok(())
        } else {
            Err(format!(
                "capability {cap:?} is not permitted in the effective set"
            ))
        }
    }

    /// Return `true` if `cap` is present in the effective set.
    fn permits(&self, cap: Capability) -> bool {
        self.effective().has(cap)
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

    /// Return the intersection cached at construction time.
    #[must_use]
    pub const fn effective(&self) -> &CapabilitySet {
        &self.effective
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
    pub const fn all() -> Self {
        Self(CapabilitySet::all())
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
    pub const fn all() -> Self {
        Self(CapabilitySet::all())
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
    pub const fn all() -> Self {
        Self(CapabilitySet::all())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_has_all_seven_individual_capabilities() {
        let all = CapabilitySet::all();
        assert_eq!(std::mem::size_of::<CapabilitySet>(), 1);
        assert_eq!(all.len(), 7);
        for capability in Capability::ALL {
            assert!(all.has(capability));
        }
        assert!(CapabilitySet::empty().is_empty());
    }

    #[test]
    fn bitand_is_strict_intersection() {
        let left =
            CapabilitySet::from([Capability::ReadFs, Capability::WriteFs, Capability::Network]);
        let right = CapabilitySet::from([Capability::ReadFs, Capability::Llm]);
        let result = left & right;
        assert!(result.has(Capability::ReadFs));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn effective_capabilities_require_all_three_layers() {
        let cell =
            CellCapabilities::of(&[Capability::ReadFs, Capability::WriteFs, Capability::Llm]);
        let graph = GraphAllowList::of(&[Capability::ReadFs, Capability::Llm, Capability::Bus]);
        let space = SpaceGrant::of(&[Capability::ReadFs, Capability::Bus]);

        let effective = effective_capabilities(&cell, &graph, &space);
        assert_eq!(effective, CapabilitySet::from([Capability::ReadFs]));
        assert!(!effective.has(Capability::WriteFs));
        assert!(!effective.has(Capability::Llm));
        assert!(!effective.has(Capability::Bus));
    }

    #[test]
    fn cached_checker_is_not_widened_by_later_layer_changes() {
        let mut cell = CellCapabilities::of(&[Capability::ReadFs]);
        let graph = GraphAllowList::all();
        let space = SpaceGrant::all();
        let checker = CachedCapabilityChecker::new(&cell, &graph, &space);

        cell = CellCapabilities::all();
        assert!(cell.0.has(Capability::Network));
        assert!(checker.effective().has(Capability::ReadFs));
        assert!(!checker.effective().has(Capability::Network));
    }

    #[test]
    fn taint_mapping_still_only_narrows() {
        let public = capabilities_for_taint(TaintLevel::Public);
        let internal = capabilities_for_taint(TaintLevel::Internal);
        let confidential = capabilities_for_taint(TaintLevel::Confidential);
        let secret = capabilities_for_taint(TaintLevel::Secret);

        assert_eq!(public, CapabilitySet::all());
        assert_eq!(public & internal, internal);
        assert_eq!(internal & confidential, confidential);
        assert_eq!(confidential & secret, secret);
    }
}
