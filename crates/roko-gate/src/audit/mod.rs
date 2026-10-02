//! M4, random deep audits (S05). The Python reference in
//! `benchmarks/viabilitybench/audit/` fixes every number, and the tests here
//! read its `fixtures/estimators.json`, the one source of truth for both.
//!
//! - [`policy`]: the inclusion probability π_i, the keyed draw, the run key,
//!   its commitment and the reveal check (S05 §4.2, DP1);
//! - [`estimate`]: Horvitz–Thompson, Hájek, v̂, Kish's n_eff, the Wilson
//!   interval at n_eff and the betting confidence sequence (S05 §4.5);
//! - [`ledger`]: the SHA-256 hash chain of audit events in the vault, and its
//!   redacted mirror in the workspace (S05 §5);
//! - [`hidden`]: hidden test suites in the vault and their lifecycle
//!   (S05 §4.4);
//! - [`canary`]: the scanner that exposes a suite whose canary appears where
//!   an agent's words go (SC4).
//!
//! Plain types other crates read (verify depth, labels, strata, the
//! `vs.label` row) live in `roko_core::audit_types`, and the vault in
//! `roko_core::audit_home`.

pub mod canary;
pub mod estimate;
pub mod hidden;
pub mod ledger;
pub mod policy;

/// Why an audit computation refused its input.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AuditError {
    /// A number lies outside the range it must.
    #[error("{name} ({value}) must lie in [{low}, {high}]")]
    OutOfRange {
        /// What the number is.
        name: &'static str,
        /// The number.
        value: f64,
        /// The lowest it may be.
        low: f64,
        /// The highest it may be.
        high: f64,
    },
    /// Anything else wrong with the input.
    #[error("{0}")]
    Invalid(String),
}

/// `value` when it lies in [`low`, `high`], else [`AuditError::OutOfRange`].
pub(crate) fn within(
    name: &'static str,
    value: f64,
    low: f64,
    high: f64,
) -> Result<f64, AuditError> {
    if (low..=high).contains(&value) {
        Ok(value)
    } else {
        Err(AuditError::OutOfRange {
            name,
            value,
            low,
            high,
        })
    }
}

/// `value` when it lies in the open interval (`low`, `high`), else an
/// error.
pub(crate) fn strictly_within(
    name: &'static str,
    value: f64,
    low: f64,
    high: f64,
) -> Result<f64, AuditError> {
    if value > low && value < high {
        Ok(value)
    } else {
        Err(AuditError::Invalid(format!(
            "{name} ({value}) must lie strictly between {low} and {high}"
        )))
    }
}

/// `bytes` random bytes from the operating system, as lowercase hex.
pub(crate) fn random_hex(bytes: usize) -> std::io::Result<String> {
    use std::io::Read as _;
    let mut buffer = vec![0_u8; bytes];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buffer)?;
    Ok(policy::hex(&buffer))
}

/// 7107's reference inputs and outputs, the one source of truth for the
/// Python and Rust audit numbers.
#[cfg(test)]
pub(crate) fn fixture() -> serde_json::Value {
    const FIXTURE: &str =
        include_str!("../../../../benchmarks/viabilitybench/audit/fixtures/estimators.json");
    serde_json::from_str(FIXTURE).expect("the estimators fixture parses")
}

/// The number `name` of a fixture entry.
#[cfg(test)]
pub(crate) fn number(entry: &serde_json::Value, name: &str) -> f64 {
    entry[name]
        .as_f64()
        .unwrap_or_else(|| panic!("{name} in {entry}"))
}
