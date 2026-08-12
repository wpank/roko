//! Information-Flow Control capability checking for the orchestrator safety layer.
//!
//! This module ties the [`TaintLevel`] lattice (from `roko-core`) to the
//! [`Capability`] set model: given a [`Signal`], determine whether a particular
//! capability is permitted based on the signal's IFC classification.
//!
//! # Usage
//!
//! ```
//! use roko_core::{Body, Kind, Signal, TaintLevel};
//! use roko_core::capabilities::Capability;
//! use roko_orchestrator::safety::ifc::check_capability;
//!
//! let signal = Signal::builder(Kind::AgentOutput)
//!     .body(Body::text("hello"))
//!     .build();
//!
//! // A default (Public) signal permits Network access.
//! assert!(check_capability(&signal, Capability::Network).is_ok());
//! ```
//!
//! # Errors
//!
//! [`check_capability`] returns [`CapabilityDenied`] when the required
//! capability is not included in the set granted to the signal's taint level.

use roko_core::Signal;
use roko_core::capabilities::{Capability, capabilities_for_taint};
use thiserror::Error;

// ─── CapabilityDenied ────────────────────────────────────────────────────────

/// Error returned when a required capability is not allowed at the signal's
/// current [`TaintLevel`] classification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "capability {required} denied for signal {signal_id}: taint level {taint_level:?} does not permit {required}"
)]
pub struct CapabilityDenied {
    /// Short hexadecimal identifier of the signal that was checked.
    pub signal_id: String,
    /// The capability that was required but not granted.
    pub required: Capability,
    /// The signal's current taint-level classification.
    pub taint_level: roko_core::TaintLevel,
}

// ─── check_capability ────────────────────────────────────────────────────────

/// Check whether `required` is permitted for `signal`'s IFC classification.
///
/// Retrieves the signal's [`TaintLevel`] from its provenance and looks up
/// the allowed [`CapabilitySet`] for that level. Returns `Ok(())` if
/// `required` is in the set, or a [`CapabilityDenied`] error otherwise.
///
/// # Examples
///
/// ```
/// use roko_core::{Body, Kind, Provenance, Signal, TaintLevel};
/// use roko_core::capabilities::Capability;
/// use roko_orchestrator::safety::ifc::check_capability;
///
/// // Build a secret-classified signal.
/// let mut signal = Signal::builder(Kind::AgentOutput)
///     .body(Body::text("top secret payload"))
///     .build();
/// signal.provenance.taint_level = TaintLevel::Secret;
///
/// // Read is permitted at Secret level.
/// assert!(check_capability(&signal, Capability::Read).is_ok());
///
/// // Network is not permitted at Secret level.
/// let err = check_capability(&signal, Capability::Network).unwrap_err();
/// assert_eq!(err.required, Capability::Network);
/// assert_eq!(err.taint_level, TaintLevel::Secret);
/// ```
///
/// # Errors
///
/// Returns [`CapabilityDenied`] when the required capability is not in the
/// allowed set for the signal's taint level.
pub fn check_capability(signal: &Signal, required: Capability) -> Result<(), CapabilityDenied> {
    let level = signal.provenance.taint_level;
    let allowed = capabilities_for_taint(level);
    if allowed.contains(required) {
        Ok(())
    } else {
        Err(CapabilityDenied {
            signal_id: signal.id.short(),
            required,
            taint_level: level,
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use roko_core::{Body, Kind, TaintLevel};

    use super::*;

    fn signal_with_level(level: TaintLevel) -> Signal {
        let mut s = Signal::builder(Kind::AgentOutput)
            .body(Body::text("test"))
            .build();
        s.provenance.taint_level = level;
        s
    }

    #[test]
    fn public_signal_allows_all_capabilities() {
        let signal = signal_with_level(TaintLevel::Public);
        for cap in [
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ] {
            assert!(
                check_capability(&signal, cap).is_ok(),
                "Public should allow {cap}"
            );
        }
    }

    #[test]
    fn internal_signal_denies_secret() {
        let signal = signal_with_level(TaintLevel::Internal);
        assert!(check_capability(&signal, Capability::Read).is_ok());
        assert!(check_capability(&signal, Capability::Network).is_ok());
        let err = check_capability(&signal, Capability::Secret).unwrap_err();
        assert_eq!(err.required, Capability::Secret);
        assert_eq!(err.taint_level, TaintLevel::Internal);
    }

    #[test]
    fn confidential_signal_allows_only_read_write() {
        let signal = signal_with_level(TaintLevel::Confidential);
        assert!(check_capability(&signal, Capability::Read).is_ok());
        assert!(check_capability(&signal, Capability::Write).is_ok());
        for cap in [
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ] {
            let err = check_capability(&signal, cap).unwrap_err();
            assert_eq!(err.required, cap);
            assert_eq!(err.taint_level, TaintLevel::Confidential);
        }
    }

    #[test]
    fn secret_signal_allows_only_read() {
        let signal = signal_with_level(TaintLevel::Secret);
        assert!(check_capability(&signal, Capability::Read).is_ok());
        for cap in [
            Capability::Write,
            Capability::Execute,
            Capability::Network,
            Capability::FileSystem,
            Capability::Secret,
        ] {
            let err = check_capability(&signal, cap).unwrap_err();
            assert_eq!(err.required, cap);
            assert_eq!(err.taint_level, TaintLevel::Secret);
        }
    }

    #[test]
    fn default_signal_is_public() {
        // Default provenance has TaintLevel::Public.
        let signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("default"))
            .build();
        assert_eq!(signal.provenance.taint_level, TaintLevel::Public);
        assert!(check_capability(&signal, Capability::Network).is_ok());
    }

    #[test]
    fn error_carries_signal_id_and_capability() {
        let signal = signal_with_level(TaintLevel::Secret);
        let err = check_capability(&signal, Capability::Execute).unwrap_err();
        assert_eq!(err.required, Capability::Execute);
        assert_eq!(err.taint_level, TaintLevel::Secret);
        // Error message should mention the capability.
        let msg = err.to_string();
        assert!(msg.contains("Execute"));
        assert!(msg.contains("Secret"));
    }
}
