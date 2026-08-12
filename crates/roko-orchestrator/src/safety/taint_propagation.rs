//! Taint propagation for Roko safety (MORI-PARITY-CHECKLIST §28.9).
//!
//! `TaintTracker` records how tainted information flows through signal
//! lineage. When a signal with `Provenance::is_tainted() == true` is used as
//! an input, any derived signal becomes tainted too. Sinks that need to refuse
//! tainted data (git commits, network egress, signal emits) consult
//! [`TaintTracker::is_tainted`] before proceeding.
//!
//! The tracker stores one [`TaintReason`] per [`ContentHash`] keyed in a
//! `HashMap` behind a `parking_lot::Mutex`, so multiple executor tasks may
//! consult/update it concurrently without deadlock risk.
//!
//! # Lattice-aware propagation (E34-T02)
//!
//! In addition to boolean taint tracking, each hash is assigned a
//! [`TaintLevel`] from the IFC lattice. Propagation is *monotonic*:
//! a child's level is always the join (least upper bound) of all parent
//! levels, ensuring taint can only increase, never decrease.
//!
//! # Example
//!
//! ```
//! use roko_core::{ContentHash, TaintLevel};
//! use roko_orchestrator::safety::taint_propagation::{TaintTracker, TaintReason};
//!
//! let tracker = TaintTracker::new();
//! let source = ContentHash::of(b"user input");
//! let derived = ContentHash::of(b"parsed user input");
//!
//! tracker.mark_tainted(source, TaintReason::external("webhook"));
//! tracker.propagate(&[source], derived);
//!
//! assert!(tracker.is_tainted(&derived));
//!
//! // Lattice-aware query:
//! let (tainted, level) = tracker.is_tainted_with_level(&derived);
//! assert!(tainted);
//! assert!(level.is_some());
//! ```

use parking_lot::Mutex;
use roko_core::{ContentHash, Signal, TaintInfo, TaintLevel};
use std::collections::HashMap;

// ─── propagate_taint ─────────────────────────────────────────────────────────

/// Compute the output [`TaintLevel`] for a derived signal given its input
/// levels.
///
/// A derived signal inherits the **highest** (most restrictive) classification
/// of all its inputs, implementing the *no-read-up* rule of the IFC lattice: if
/// any input is `Confidential`, the output must be at least `Confidential`.
///
/// An empty input slice returns [`TaintLevel::Public`] — a signal with no
/// inputs has no inherited classification.
///
/// # Examples
///
/// ```
/// use roko_core::TaintLevel;
/// use roko_orchestrator::safety::taint_propagation::propagate_taint;
///
/// assert_eq!(
///     propagate_taint(&[TaintLevel::Public, TaintLevel::Confidential]),
///     TaintLevel::Confidential,
/// );
/// assert_eq!(propagate_taint(&[]), TaintLevel::Public);
/// ```
#[must_use]
pub fn propagate_taint(inputs: &[TaintLevel]) -> TaintLevel {
    inputs
        .iter()
        .copied()
        .fold(TaintLevel::Public, TaintLevel::join)
}

/// Why a particular [`ContentHash`] is considered tainted.
///
/// Reasons are informational — they travel with the taint flag so that
/// downstream audits can explain refusals ("refused: came from untrusted
/// webhook"). They are not interpreted semantically by the tracker itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaintReason {
    /// Short machine-readable category (e.g. `"external"`, `"user_input"`,
    /// `"propagated"`).
    pub category: String,
    /// Human-readable explanation; kept brief to stay loggable.
    pub detail: String,
    /// Upstream tainted hash that caused propagation, when known.
    pub inherited_from: Option<ContentHash>,
}

impl TaintReason {
    /// Build a new [`TaintReason`] from a category tag and an explanation.
    pub fn new(category: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            detail: detail.into(),
            inherited_from: None,
        }
    }

    /// Convenience constructor for taint coming from an external source.
    pub fn external(detail: impl Into<String>) -> Self {
        Self::new("external", detail)
    }

    /// Convenience constructor for taint coming from user input.
    pub fn user_input(detail: impl Into<String>) -> Self {
        Self::new("user_input", detail)
    }

    /// Convenience constructor for taint that was inherited from a parent
    /// signal during propagation.
    pub fn propagated(detail: impl Into<String>) -> Self {
        Self::new("propagated", detail)
    }

    /// Convert this tracker-local reason into the shared provenance metadata shape.
    #[must_use]
    pub fn to_taint_info(&self) -> TaintInfo {
        let mut info = TaintInfo::new(self.category.clone(), self.detail.clone());
        info.inherited_from = self.inherited_from;
        info
    }

    fn from_taint_info(info: &TaintInfo) -> Self {
        Self {
            category: info.category.clone(),
            detail: info.detail.clone(),
            inherited_from: info.inherited_from,
        }
    }
}

/// Per-hash entry combining a taint reason with its lattice classification.
#[derive(Clone, Debug)]
struct TaintEntry {
    reason: TaintReason,
    /// IFC lattice level — monotonically non-decreasing across propagations.
    level: TaintLevel,
}

/// Tracks taint status across a signal DAG.
///
/// A [`TaintTracker`] is cheap to create and safe to share across threads
/// (`Arc<TaintTracker>` is the expected sharing pattern). All mutating
/// methods take `&self` because internal state is protected by a
/// `parking_lot::Mutex`.
///
/// # Semantics
///
/// * [`mark_tainted`](Self::mark_tainted) stamps a hash with a reason.
///   Calling it twice overwrites the reason (last writer wins), which is
///   fine — taint is a boolean-with-annotation, not a vote.
/// * [`mark_tainted_with_level`](Self::mark_tainted_with_level) additionally
///   stores an explicit [`TaintLevel`] from the IFC lattice.
/// * [`propagate`](Self::propagate) marks `child` tainted if **any** parent
///   is already tainted. If no parent is tainted, the child is left alone
///   (a clean child must not become tainted by being combined with other
///   clean signals).
/// * [`is_tainted`](Self::is_tainted) is a pure read (bool).
/// * [`is_tainted_with_level`](Self::is_tainted_with_level) returns
///   `(bool, Option<TaintLevel>)` for richer lattice queries.
/// * [`get_level`](Self::get_level) retrieves the stored lattice level.
/// * [`reason`](Self::reason) returns the stored reason, if any.
#[derive(Debug, Default)]
pub struct TaintTracker {
    inner: Mutex<HashMap<ContentHash, TaintEntry>>,
}

impl TaintTracker {
    /// Construct a fresh, empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `hash` tainted with the given `reason`.
    ///
    /// The lattice level defaults to [`TaintLevel::Internal`] when taint is
    /// asserted without an explicit level. Use
    /// [`mark_tainted_with_level`](Self::mark_tainted_with_level) to supply a
    /// specific level. Calling this twice overwrites the prior entry.
    pub fn mark_tainted(&self, hash: ContentHash, reason: TaintReason) {
        self.inner.lock().insert(
            hash,
            TaintEntry {
                reason,
                level: TaintLevel::Internal,
            },
        );
    }

    /// Mark `hash` tainted with both a [`TaintReason`] and an explicit
    /// [`TaintLevel`] from the IFC lattice.
    ///
    /// The level must be at least [`TaintLevel::Internal`] — passing
    /// [`TaintLevel::Public`] would be a no-op taint, so it is silently
    /// promoted to `Internal`.
    pub fn mark_tainted_with_level(
        &self,
        hash: ContentHash,
        reason: TaintReason,
        level: TaintLevel,
    ) {
        let effective_level = level.join(TaintLevel::Internal);
        self.inner.lock().insert(
            hash,
            TaintEntry {
                reason,
                level: effective_level,
            },
        );
    }

    /// Returns `true` if `hash` has been marked tainted at any point.
    #[must_use]
    pub fn is_tainted(&self, hash: &ContentHash) -> bool {
        self.inner.lock().contains_key(hash)
    }

    /// Returns `(is_tainted, level)` for richer lattice queries.
    ///
    /// When the hash is not tainted, returns `(false, None)`. When tainted,
    /// returns `(true, Some(level))` where `level` is the stored lattice
    /// classification.
    #[must_use]
    pub fn is_tainted_with_level(&self, hash: &ContentHash) -> (bool, Option<TaintLevel>) {
        let guard = self.inner.lock();
        match guard.get(hash) {
            Some(entry) => (true, Some(entry.level)),
            None => (false, None),
        }
    }

    /// Retrieve the [`TaintLevel`] stored for `hash`, if any.
    ///
    /// Returns `None` when the hash is not tracked (clean), and
    /// `Some(level)` when it has been marked or had taint propagated to it.
    #[must_use]
    pub fn get_level(&self, hash: &ContentHash) -> Option<TaintLevel> {
        self.inner.lock().get(hash).map(|e| e.level)
    }

    /// Retrieve the [`TaintReason`] stored for `hash`, if any.
    #[must_use]
    pub fn reason(&self, hash: &ContentHash) -> Option<TaintReason> {
        self.inner.lock().get(hash).map(|e| e.reason.clone())
    }

    /// Retrieve structured taint metadata suitable for storing in provenance.
    #[must_use]
    pub fn taint_info(&self, hash: &ContentHash) -> Option<TaintInfo> {
        self.reason(hash).map(|reason| reason.to_taint_info())
    }

    /// Propagate taint from parents to `child`.
    ///
    /// If any parent in `parents` is currently tainted, `child` is marked
    /// tainted with a `"propagated"` reason that names the offending parent.
    /// The child's lattice level is computed as the join (max) of all parent
    /// levels, enforcing monotonicity: taint can only increase, never decrease.
    ///
    /// If `child` was already tainted with a more specific (non-propagated)
    /// reason, that reason is preserved, but the lattice level is still
    /// updated to the join to maintain monotonicity.
    ///
    /// Returns `true` if taint was actually propagated; `false` if no parent
    /// was tainted and `child` was left untouched.
    ///
    /// Emits a `tracing::info!` event for every propagation with structured
    /// fields: `child`, `parents`, and `resulting_level`.
    pub fn propagate(&self, parents: &[ContentHash], child: ContentHash) -> bool {
        let mut guard = self.inner.lock();

        // Collect all tainted parents and join their levels.
        let mut joined_level = TaintLevel::Public;
        let mut first_tainted_parent: Option<ContentHash> = None;
        for p in parents {
            if let Some(entry) = guard.get(p) {
                joined_level = joined_level.join(entry.level);
                if first_tainted_parent.is_none() {
                    first_tainted_parent = Some(*p);
                }
            }
        }

        let Some(parent) = first_tainted_parent else {
            return false;
        };

        // Ensure the joined level is at least Internal when propagating.
        let resulting_level = joined_level.join(TaintLevel::Internal);

        // Preserve any pre-existing, stronger reason (anything that isn't
        // itself just "propagated"), but always monotonically join the level.
        let already_specific = guard
            .get(&child)
            .is_some_and(|e| e.reason.category != "propagated");
        if already_specific {
            // Level is still updated for monotonicity.
            if let Some(existing) = guard.get_mut(&child) {
                existing.level = existing.level.join(resulting_level);
            }
        } else {
            let mut reason = TaintReason::propagated(format!("inherited from {}", parent.short()));
            reason.inherited_from = Some(parent);
            guard.insert(
                child,
                TaintEntry {
                    reason,
                    level: resulting_level,
                },
            );
        }

        tracing::info!(
            child = %child.short(),
            parent_count = parents.len(),
            first_tainted_parent = %parent.short(),
            resulting_level = ?resulting_level,
            "taint propagated to child",
        );

        true
    }

    /// Propagate taint through a linear pipeline of stages.
    ///
    /// Given a slice of input hashes and a corresponding slice of output
    /// hashes (one per stage), each output is tainted with the join of all
    /// inputs' lattice levels plus any previously accumulated taint on
    /// earlier outputs. This models a sequential data-flow pipeline where
    /// contamination at any stage flows forward to all subsequent stages.
    ///
    /// Returns the final effective [`TaintLevel`] after propagating through
    /// all stages, or [`TaintLevel::Public`] if no input was tainted.
    ///
    /// # Example
    ///
    /// ```
    /// use roko_core::{ContentHash, TaintLevel};
    /// use roko_orchestrator::safety::taint_propagation::{TaintTracker, TaintReason};
    ///
    /// let tracker = TaintTracker::new();
    /// let input = ContentHash::of(b"raw input");
    /// let stage1 = ContentHash::of(b"parsed");
    /// let stage2 = ContentHash::of(b"transformed");
    ///
    /// tracker.mark_tainted_with_level(input, TaintReason::external("api"), TaintLevel::Confidential);
    /// let final_level = tracker.propagate_through_pipeline(&[input], &[stage1, stage2]);
    ///
    /// assert_eq!(final_level, TaintLevel::Confidential);
    /// assert!(tracker.is_tainted(&stage2));
    /// ```
    pub fn propagate_through_pipeline(
        &self,
        inputs: &[ContentHash],
        outputs: &[ContentHash],
    ) -> TaintLevel {
        // Compute initial joined level and first-tainted-input attribution
        // from all inputs in a single lock acquisition.
        let (input_level, attr) = {
            let guard = self.inner.lock();
            let joined = inputs
                .iter()
                .filter_map(|h| guard.get(h).map(|e| e.level))
                .fold(TaintLevel::Public, TaintLevel::join);
            let first_tainted = inputs.iter().find(|h| guard.contains_key(h)).copied();
            (joined, first_tainted)
        };

        if input_level == TaintLevel::Public {
            // No tainted inputs — nothing to propagate.
            return TaintLevel::Public;
        }

        // Attribution: use first tainted input; fall back to first input.
        let attribution = attr.or_else(|| inputs.first().copied());

        // Propagate forward through each output stage, accumulating level.
        let mut current_level = input_level;
        for &output in outputs {
            let effective_level = {
                let mut guard = self.inner.lock();
                // Join with any pre-existing level on this output.
                if let Some(e) = guard.get(&output) {
                    current_level = current_level.join(e.level);
                }
                let effective = current_level.join(TaintLevel::Internal);
                let mut reason =
                    TaintReason::propagated(format!("pipeline stage {}", output.short()));
                if let Some(src) = attribution {
                    reason.inherited_from = Some(src);
                    reason.detail = format!("pipeline input {}", src.short());
                }
                let entry = guard.entry(output).or_insert(TaintEntry {
                    reason: reason.clone(),
                    level: effective,
                });
                entry.level = entry.level.join(effective);
                if entry.reason.category == "propagated" {
                    entry.reason = reason;
                }
                effective
            };

            tracing::info!(
                stage = %output.short(),
                resulting_level = ?effective_level,
                "taint propagated through pipeline stage",
            );

            current_level = effective_level;
        }

        current_level
    }

    /// Inspect a [`Signal`] and, if its provenance is tainted, mark it in
    /// the tracker with a reason derived from the provenance's [`Taint`] variant
    /// and the provenance's [`TaintLevel`] lattice classification.
    ///
    /// Returns `true` if the signal was (or already was) tainted, `false`
    /// if the signal's provenance is clean.
    pub fn observe_signal(&self, signal: &Signal) -> bool {
        if signal.provenance.is_tainted() {
            // Prefer the legacy taint_info if present (for old serialized data),
            // otherwise derive from the typed Taint enum.
            let reason = signal.provenance.taint_info.as_ref().map_or_else(
                || {
                    TaintReason::new(
                        signal.provenance.taint.category(),
                        signal
                            .provenance
                            .taint
                            .detail()
                            .unwrap_or(&format!("signal author {}", signal.provenance.author))
                            .to_string(),
                    )
                },
                TaintReason::from_taint_info,
            );
            // Use the signal's effective_taint() for the lattice level, which
            // joins the provenance's taint_level field with the level implied
            // by the Taint variant.
            let level = signal.provenance.effective_taint();
            self.mark_tainted_with_level(signal.id, reason, level);
            true
        } else {
            false
        }
    }

    /// Forget all recorded taint. Useful between isolated runs/tests.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    /// Number of tainted hashes currently tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// `true` if no taint has been recorded (yet).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn h(tag: &[u8]) -> ContentHash {
        ContentHash::of(tag)
    }

    #[test]
    fn mark_tainted_records_hash() {
        let tracker = TaintTracker::new();
        let id = h(b"one");
        assert!(!tracker.is_tainted(&id));
        tracker.mark_tainted(id, TaintReason::user_input("CLI flag"));
        assert!(tracker.is_tainted(&id));
    }

    #[test]
    fn is_tainted_on_clean_returns_false() {
        let tracker = TaintTracker::new();
        let id = h(b"never-touched");
        assert!(!tracker.is_tainted(&id));
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn propagate_via_single_parent() {
        let tracker = TaintTracker::new();
        let parent = h(b"p");
        let child = h(b"c");
        tracker.mark_tainted(parent, TaintReason::external("api"));
        let propagated = tracker.propagate(&[parent], child);
        assert!(propagated);
        assert!(tracker.is_tainted(&child));
    }

    #[test]
    fn propagate_via_multiple_parents_when_one_tainted() {
        let tracker = TaintTracker::new();
        let clean_parent = h(b"clean");
        let dirty_parent = h(b"dirty");
        let child = h(b"child");
        tracker.mark_tainted(dirty_parent, TaintReason::user_input("stdin"));
        let propagated = tracker.propagate(&[clean_parent, dirty_parent], child);
        assert!(propagated);
        assert!(tracker.is_tainted(&child));
        assert!(!tracker.is_tainted(&clean_parent));
    }

    #[test]
    fn propagate_no_ops_when_all_parents_clean() {
        let tracker = TaintTracker::new();
        let p1 = h(b"p1");
        let p2 = h(b"p2");
        let child = h(b"child");
        let propagated = tracker.propagate(&[p1, p2], child);
        assert!(!propagated);
        assert!(!tracker.is_tainted(&child));
    }

    #[test]
    fn propagate_with_empty_parents_is_noop() {
        let tracker = TaintTracker::new();
        let child = h(b"lonely");
        let propagated = tracker.propagate(&[], child);
        assert!(!propagated);
        assert!(!tracker.is_tainted(&child));
    }

    #[test]
    fn reason_retrieval_returns_stored_reason() {
        let tracker = TaintTracker::new();
        let id = h(b"r");
        tracker.mark_tainted(id, TaintReason::external("webhook"));
        let reason = tracker.reason(&id).expect("reason should be present");
        assert_eq!(reason.category, "external");
        assert_eq!(reason.detail, "webhook");
    }

    #[test]
    fn taint_info_roundtrips_from_reason() {
        let tracker = TaintTracker::new();
        let id = h(b"info");
        tracker.mark_tainted(id, TaintReason::external("webhook"));
        let info = tracker
            .taint_info(&id)
            .expect("taint info should be present");
        assert_eq!(info.category, "external");
        assert_eq!(info.detail, "webhook");
        assert_eq!(info.inherited_from, None);
    }

    #[test]
    fn reason_on_clean_returns_none() {
        let tracker = TaintTracker::new();
        assert!(tracker.reason(&h(b"absent")).is_none());
    }

    #[test]
    fn mark_twice_overwrites_reason() {
        let tracker = TaintTracker::new();
        let id = h(b"twice");
        tracker.mark_tainted(id, TaintReason::external("first"));
        tracker.mark_tainted(id, TaintReason::user_input("second"));
        let reason = tracker.reason(&id).expect("reason present");
        assert_eq!(reason.category, "user_input");
        assert_eq!(reason.detail, "second");
    }

    #[test]
    fn propagate_reason_cites_parent() {
        let tracker = TaintTracker::new();
        let parent = h(b"parent");
        let child = h(b"child");
        tracker.mark_tainted(parent, TaintReason::external("api"));
        tracker.propagate(&[parent], child);
        let reason = tracker.reason(&child).expect("child reason");
        assert_eq!(reason.category, "propagated");
        assert!(reason.detail.contains(&parent.short()));
        assert_eq!(reason.inherited_from, Some(parent));
    }

    #[test]
    fn transitive_propagation_spreads_taint() {
        // a -> b -> c: taint at `a` reaches `c` via two propagate calls.
        let tracker = TaintTracker::new();
        let a = h(b"a");
        let b = h(b"b");
        let c = h(b"c");
        tracker.mark_tainted(a, TaintReason::external("root"));
        assert!(tracker.propagate(&[a], b));
        assert!(tracker.propagate(&[b], c));
        assert!(tracker.is_tainted(&a));
        assert!(tracker.is_tainted(&b));
        assert!(tracker.is_tainted(&c));
    }

    #[test]
    fn propagate_preserves_stronger_preexisting_reason() {
        // Child is already marked with a specific reason; propagate must
        // not overwrite it with a weaker "propagated" reason.
        let tracker = TaintTracker::new();
        let parent = h(b"parent");
        let child = h(b"child");
        tracker.mark_tainted(parent, TaintReason::external("api"));
        tracker.mark_tainted(child, TaintReason::user_input("kept"));
        assert!(tracker.propagate(&[parent], child));
        let r = tracker.reason(&child).expect("reason");
        assert_eq!(r.category, "user_input");
        assert_eq!(r.detail, "kept");
    }

    #[test]
    fn propagate_upgrades_from_propagated_reason() {
        // If child's current reason is already "propagated", a new
        // propagate call may refresh the parent citation.
        let tracker = TaintTracker::new();
        let p1 = h(b"p1");
        let p2 = h(b"p2");
        let child = h(b"c");
        tracker.mark_tainted(p1, TaintReason::external("a"));
        tracker.mark_tainted(p2, TaintReason::external("b"));
        tracker.propagate(&[p1], child);
        let r1 = tracker.reason(&child).expect("reason");
        assert_eq!(r1.category, "propagated");
        tracker.propagate(&[p2], child);
        let r2 = tracker.reason(&child).expect("reason");
        assert_eq!(r2.category, "propagated");
        assert!(r2.detail.contains(&p2.short()));
    }

    #[test]
    fn clear_drops_all_state() {
        let tracker = TaintTracker::new();
        tracker.mark_tainted(h(b"x"), TaintReason::external("x"));
        tracker.mark_tainted(h(b"y"), TaintReason::external("y"));
        assert_eq!(tracker.len(), 2);
        tracker.clear();
        assert!(tracker.is_empty());
        assert!(!tracker.is_tainted(&h(b"x")));
        assert!(!tracker.is_tainted(&h(b"y")));
    }

    #[test]
    fn observe_signal_marks_tainted_provenance() {
        use roko_core::{Body, Kind, Signal};

        let tainted_signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("external payload"))
            .provenance(roko_core::Provenance::external("webhook"))
            .build();
        let clean_signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("internal payload"))
            .provenance(roko_core::Provenance::trusted("orchestrator"))
            .build();

        let tracker = TaintTracker::new();
        assert!(tracker.observe_signal(&tainted_signal));
        assert!(!tracker.observe_signal(&clean_signal));

        assert!(tracker.is_tainted(&tainted_signal.id));
        assert!(!tracker.is_tainted(&clean_signal.id));
        let reason = tracker.reason(&tainted_signal.id).expect("has reason");
        assert_eq!(reason.category, "unverified_source");
    }

    #[test]
    fn observe_signal_prefers_provenance_taint_info() {
        use roko_core::{Body, Kind, Provenance, Signal, TaintInfo};

        let tainted_signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("external payload"))
            .provenance(
                Provenance::trusted("gateway")
                    .with_taint_info(TaintInfo::external("webhook payload")),
            )
            .build();

        let tracker = TaintTracker::new();
        assert!(tracker.observe_signal(&tainted_signal));
        let reason = tracker.reason(&tainted_signal.id).expect("has reason");
        assert_eq!(reason.category, "external");
        assert_eq!(reason.detail, "webhook payload");
    }

    #[test]
    fn concurrent_marks_are_safe() {
        use std::sync::Arc;
        use std::thread;

        let tracker = Arc::new(TaintTracker::new());
        let mut handles = Vec::new();
        for i in 0u8..16 {
            let t = Arc::clone(&tracker);
            handles.push(thread::spawn(move || {
                let id = h(&[i]);
                t.mark_tainted(id, TaintReason::external("thread"));
            }));
        }
        for handle in handles {
            handle.join().expect("thread join");
        }
        assert_eq!(tracker.len(), 16);
    }

    #[test]
    fn taint_reason_constructors_set_category() {
        assert_eq!(TaintReason::external("x").category, "external");
        assert_eq!(TaintReason::user_input("x").category, "user_input");
        assert_eq!(TaintReason::propagated("x").category, "propagated");
        assert_eq!(TaintReason::new("custom", "x").category, "custom");
    }

    // ─── propagate_taint tests ────────────────────────────────────────────────

    #[test]
    fn propagate_taint_empty_inputs_returns_public() {
        assert_eq!(propagate_taint(&[]), TaintLevel::Public);
    }

    #[test]
    fn propagate_taint_single_input_passes_through() {
        assert_eq!(
            propagate_taint(&[TaintLevel::Internal]),
            TaintLevel::Internal
        );
        assert_eq!(
            propagate_taint(&[TaintLevel::Confidential]),
            TaintLevel::Confidential
        );
        assert_eq!(propagate_taint(&[TaintLevel::Secret]), TaintLevel::Secret);
    }

    #[test]
    fn propagate_taint_inherits_highest_level() {
        assert_eq!(
            propagate_taint(&[TaintLevel::Public, TaintLevel::Confidential]),
            TaintLevel::Confidential,
        );
        assert_eq!(
            propagate_taint(&[TaintLevel::Internal, TaintLevel::Secret, TaintLevel::Public]),
            TaintLevel::Secret,
        );
    }

    #[test]
    fn propagate_taint_all_public_stays_public() {
        assert_eq!(
            propagate_taint(&[TaintLevel::Public, TaintLevel::Public]),
            TaintLevel::Public,
        );
    }

    #[test]
    fn propagate_taint_is_commutative() {
        let a = &[TaintLevel::Internal, TaintLevel::Confidential];
        let b = &[TaintLevel::Confidential, TaintLevel::Internal];
        assert_eq!(propagate_taint(a), propagate_taint(b));
    }

    // ─── E34-T02: lattice-aware TaintTracker tests ────────────────────────────

    #[test]
    fn mark_tainted_with_level_stores_level() {
        let tracker = TaintTracker::new();
        let id = h(b"sensitive");
        tracker.mark_tainted_with_level(id, TaintReason::external("api"), TaintLevel::Confidential);
        assert!(tracker.is_tainted(&id));
        assert_eq!(tracker.get_level(&id), Some(TaintLevel::Confidential));
    }

    #[test]
    fn mark_tainted_defaults_to_internal_level() {
        // mark_tainted() (no explicit level) should default to at least Internal.
        let tracker = TaintTracker::new();
        let id = h(b"basic");
        tracker.mark_tainted(id, TaintReason::user_input("stdin"));
        let level = tracker.get_level(&id).expect("level must be set");
        assert!(
            level >= TaintLevel::Internal,
            "default level must be >= Internal"
        );
    }

    #[test]
    fn public_level_is_promoted_to_internal_on_mark() {
        // Passing TaintLevel::Public to mark_tainted_with_level is promoted.
        let tracker = TaintTracker::new();
        let id = h(b"promoted");
        tracker.mark_tainted_with_level(id, TaintReason::external("src"), TaintLevel::Public);
        let level = tracker.get_level(&id).expect("level must be set");
        assert!(
            level >= TaintLevel::Internal,
            "Public is promoted to at least Internal"
        );
    }

    #[test]
    fn is_tainted_with_level_returns_correct_pair() {
        let tracker = TaintTracker::new();
        let id = h(b"pair");
        let absent = h(b"absent");

        tracker.mark_tainted_with_level(id, TaintReason::external("api"), TaintLevel::Secret);

        let (tainted, level) = tracker.is_tainted_with_level(&id);
        assert!(tainted);
        assert_eq!(level, Some(TaintLevel::Secret));

        let (tainted2, level2) = tracker.is_tainted_with_level(&absent);
        assert!(!tainted2);
        assert_eq!(level2, None);
    }

    #[test]
    fn get_level_returns_none_for_clean_hash() {
        let tracker = TaintTracker::new();
        assert_eq!(tracker.get_level(&h(b"clean")), None);
    }

    #[test]
    fn propagate_joins_parent_levels_monotonically() {
        let tracker = TaintTracker::new();
        let p1 = h(b"parent1");
        let p2 = h(b"parent2");
        let child = h(b"child");

        tracker.mark_tainted_with_level(p1, TaintReason::external("a"), TaintLevel::Internal);
        tracker.mark_tainted_with_level(p2, TaintReason::external("b"), TaintLevel::Confidential);

        tracker.propagate(&[p1, p2], child);

        // Child must be >= the max of parent levels (Confidential).
        let child_level = tracker.get_level(&child).expect("child must be tainted");
        assert!(
            child_level >= TaintLevel::Confidential,
            "child level must be >= join of parent levels"
        );
    }

    #[test]
    fn propagate_level_is_monotone_cannot_decrease() {
        // Mark child at Confidential, then propagate from an Internal parent.
        // Child's level must remain at least Confidential.
        let tracker = TaintTracker::new();
        let parent = h(b"parent");
        let child = h(b"child");

        tracker.mark_tainted_with_level(parent, TaintReason::external("api"), TaintLevel::Internal);
        tracker.mark_tainted_with_level(
            child,
            TaintReason::user_input("direct"),
            TaintLevel::Confidential,
        );

        tracker.propagate(&[parent], child);

        let child_level = tracker
            .get_level(&child)
            .expect("child must still be tainted");
        assert!(
            child_level >= TaintLevel::Confidential,
            "level must not decrease: got {child_level:?}"
        );
    }

    #[test]
    fn propagate_through_pipeline_propagates_all_stages() {
        let tracker = TaintTracker::new();
        let input = h(b"raw input");
        let stage1 = h(b"stage1 output");
        let stage2 = h(b"stage2 output");
        let stage3 = h(b"stage3 output");

        tracker.mark_tainted_with_level(
            input,
            TaintReason::external("api"),
            TaintLevel::Confidential,
        );

        let final_level = tracker.propagate_through_pipeline(&[input], &[stage1, stage2, stage3]);

        assert!(
            final_level >= TaintLevel::Confidential,
            "final level must be >= input level"
        );
        assert!(
            tracker.is_tainted(&stage1),
            "stage1 must be tainted after pipeline"
        );
        assert!(
            tracker.is_tainted(&stage2),
            "stage2 must be tainted after pipeline"
        );
        assert!(
            tracker.is_tainted(&stage3),
            "stage3 must be tainted after pipeline"
        );
    }

    #[test]
    fn propagate_through_pipeline_with_clean_inputs_noop() {
        let tracker = TaintTracker::new();
        let clean_input = h(b"clean input");
        let output = h(b"output");

        // No taint on the input — pipeline should be a no-op.
        let final_level = tracker.propagate_through_pipeline(&[clean_input], &[output]);

        assert_eq!(final_level, TaintLevel::Public);
        assert!(!tracker.is_tainted(&output));
    }

    #[test]
    fn propagate_through_pipeline_inherits_highest_input_level() {
        let tracker = TaintTracker::new();
        let in1 = h(b"in1");
        let in2 = h(b"in2");
        let out = h(b"out");

        tracker.mark_tainted_with_level(in1, TaintReason::external("a"), TaintLevel::Internal);
        tracker.mark_tainted_with_level(in2, TaintReason::external("b"), TaintLevel::Secret);

        let final_level = tracker.propagate_through_pipeline(&[in1, in2], &[out]);

        // The highest input (Secret) must dominate.
        assert!(
            final_level >= TaintLevel::Secret,
            "must inherit highest input level"
        );
        assert!(tracker.get_level(&out).unwrap() >= TaintLevel::Secret);
    }

    #[test]
    fn observe_signal_stores_lattice_level() {
        use roko_core::{Body, Kind, Provenance, Signal};

        let signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("external"))
            .provenance(Provenance::external("webhook").with_taint_level(TaintLevel::Confidential))
            .build();

        let tracker = TaintTracker::new();
        tracker.observe_signal(&signal);

        let (tainted, level) = tracker.is_tainted_with_level(&signal.id);
        assert!(tainted);
        // effective_taint() for an external source is at least Confidential.
        assert!(level.unwrap() >= TaintLevel::Confidential);
    }

    #[test]
    fn lattice_attribution_tracks_origin() {
        // Verify that the TaintReason's inherited_from field is set during propagation.
        let tracker = TaintTracker::new();
        let source = h(b"origin");
        let derived = h(b"derived");

        tracker.mark_tainted_with_level(source, TaintReason::external("api"), TaintLevel::Secret);
        tracker.propagate(&[source], derived);

        let reason = tracker.reason(&derived).expect("derived must have reason");
        assert_eq!(reason.category, "propagated");
        assert_eq!(reason.inherited_from, Some(source));
        assert_eq!(tracker.get_level(&derived), Some(TaintLevel::Secret));
    }
}
