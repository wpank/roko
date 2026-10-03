//! The audit lottery (S05 §4.2, DP1): which green units get a deep audit, by
//! a draw anyone can re-check once the run's key is revealed. The Python
//! reference is `benchmarks/viabilitybench/audit/lottery.py`.
//!
//! - **π_i** = clip(ρ·[(1−λ) + λ·r_i^α/r̄], ε_floor, π_max)
//!   ([`inclusion_probability`]), where r_i is M3's predicted
//!   P(false green) and r̄ the window's mean of r^α. Without a risk π_i = ρ.
//!   The floor ε_floor = 5% is locked (S05 §4.9, decision 7102): it is a
//!   constant, so every π_i is at least 0.05 and Horvitz–Thompson stays
//!   unbiased.
//! - **The tilt** λ = λ_max·max(0, 1 − ECE/ECE_ref), and 0 until M3 has
//!   [`MIN_TILT_LABELS`] audited labels ([`tilt`]).
//! - **The draw** x_i is the first 8 bytes, big-endian, of
//!   HMAC-SHA256(K_run, fields) over [`DRAW_TAG`], run id, task id, attempt
//!   id and accepted commit ([`draw`]). The unit is selected when
//!   x_i < π_i·2^64 ([`is_selected`]), so π = 1 always selects.
//! - **The key**: K_run = HMAC-SHA256(workspace secret, ([`KEY_TAG`], run
//!   id)) ([`RunKey::derive`]); the secret lives in the vault's `keys/`
//!   ([`workspace_secret`]). The run commits to its key at open
//!   ([`commitment`]) and reveals it at close; [`verify_reveal`] then
//!   recomputes every draw.
//!
//! The field encoding is `common.hmac_seed`'s, shared with Python: each
//! field is UTF-8, prefixed by its byte length as a 4-byte big-endian
//! integer, and the first field is a domain tag.

use std::io::Write as _;

use hmac::{Hmac, Mac};
use roko_core::audit_home::AuditVault;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{AuditError, strictly_within, within};

/// ε_floor, the locked floor of every π_i.
pub const EPS_FLOOR: f64 = 0.05;
/// ρ for real work (decision 7103).
pub const DEFAULT_RHO: f64 = 0.10;
/// The highest ρ.
pub const RHO_MAX: f64 = 0.5;
/// The default π_max.
pub const PI_MAX: f64 = 1.0;
/// The default exponent α on the risk.
pub const ALPHA: f64 = 1.0;
/// The default λ_max.
pub const LAMBDA_MAX: f64 = 0.8;
/// The default ECE_ref.
pub const ECE_REF: f64 = 0.10;
/// Audited labels M3 needs before it may tilt the draw.
pub const MIN_TILT_LABELS: u32 = 50;
/// The domain tag of a draw.
pub const DRAW_TAG: &str = "roko.audit.draw/1";
/// The domain tag of a run key.
pub const KEY_TAG: &str = "roko.audit.key/1";
/// Bytes in a run key, and the fewest in a workspace secret.
pub const KEY_BYTES: usize = 32;
/// The workspace secret's file name in the vault's `keys/`.
pub const SECRET_FILE: &str = "audit-secret";

/// 2^64, exactly.
const TWO_64: f64 = 18_446_744_073_709_551_616.0;

/// A run's key K_run. Its `Debug` never shows the bytes: the key is not
/// logged before `audit.key_reveal`.
#[derive(Clone, PartialEq, Eq)]
pub struct RunKey([u8; KEY_BYTES]);

impl RunKey {
    /// K_run = HMAC-SHA256(`secret`, ([`KEY_TAG`], `run_id`)).
    ///
    /// # Errors
    ///
    /// A secret shorter than [`KEY_BYTES`].
    pub fn derive(secret: &[u8], run_id: &str) -> Result<Self, AuditError> {
        if secret.len() < KEY_BYTES {
            return Err(AuditError::Invalid(format!(
                "the audit secret must hold at least {KEY_BYTES} bytes"
            )));
        }
        Ok(Self(hmac_fields(secret, &[KEY_TAG, run_id])))
    }

    /// The key from its revealed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self(bytes)
    }

    /// The key from 64 hex digits, as `audit.key_reveal` records it.
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = decode_hex(text)?;
        bytes.try_into().ok().map(Self)
    }

    /// The key's bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; KEY_BYTES] {
        &self.0
    }

    /// The key as 64 lowercase hex digits, for `audit.key_reveal` only.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

impl std::fmt::Debug for RunKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RunKey(<unrevealed>)")
    }
}

/// The knobs of [`inclusion_probability`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InclusionParams {
    /// ρ, the base rate, in [[`EPS_FLOOR`], [`RHO_MAX`]].
    pub rho: f64,
    /// λ, the tilt, in [0, 1].
    pub lam: f64,
    /// α, the exponent on the risk, above 0.
    pub alpha: f64,
    /// π_max, in [[`EPS_FLOOR`], 1].
    pub pi_max: f64,
}

impl Default for InclusionParams {
    fn default() -> Self {
        Self {
            rho: DEFAULT_RHO,
            lam: 0.0,
            alpha: ALPHA,
            pi_max: PI_MAX,
        }
    }
}

/// One green unit's draw, as `audit.selection` records it (S05 §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    /// The run.
    pub run_id: String,
    /// The task.
    pub task_id: String,
    /// The attempt (S01's attempt key).
    pub attempt_id: String,
    /// The accepted commit, or the result tree standing in for it.
    pub accepted_commit: String,
    /// π_i.
    pub pi: f64,
    /// x_i as "0x" and 16 hex digits ([`prf_hex`]).
    pub prf_u: String,
    /// Whether the unit was selected.
    pub selected: bool,
}

/// λ for M3's risk tilt: λ_max·max(0, 1 − ECE/ECE_ref), and 0 without an
/// ECE or below [`MIN_TILT_LABELS`] labels.
///
/// # Errors
///
/// `lambda_max` outside [0, 1], `ece_ref` not above 0, or an ECE outside
/// [0, 1].
pub fn tilt(
    ece: Option<f64>,
    n_labels: u32,
    lambda_max: f64,
    ece_ref: f64,
) -> Result<f64, AuditError> {
    within("lambda_max", lambda_max, 0.0, 1.0)?;
    strictly_within("ece_ref", ece_ref, 0.0, f64::INFINITY)?;
    let Some(ece) = ece.filter(|_| n_labels >= MIN_TILT_LABELS) else {
        return Ok(0.0);
    };
    within("ece", ece, 0.0, 1.0)?;
    Ok(lambda_max * (1.0 - ece / ece_ref).max(0.0))
}

/// π = clip(ρ·[(1−λ) + λ·r^α/r̄], [`EPS_FLOOR`], π_max); uniform (π = ρ
/// before the clip) without a risk, a mean risk above 0, or a tilt.
///
/// # Errors
///
/// A knob or a risk outside its range ([`InclusionParams`]; risks lie in
/// [0, 1]).
pub fn inclusion_probability(
    params: &InclusionParams,
    risk: Option<f64>,
    mean_risk: Option<f64>,
) -> Result<f64, AuditError> {
    let InclusionParams {
        rho,
        lam,
        alpha,
        pi_max,
    } = *params;
    within("rho", rho, EPS_FLOOR, RHO_MAX)?;
    within("lambda", lam, 0.0, 1.0)?;
    strictly_within("alpha", alpha, 0.0, f64::INFINITY)?;
    within("pi_max", pi_max, EPS_FLOOR, 1.0)?;
    for (name, value) in [("risk", risk), ("mean_risk", mean_risk)] {
        if let Some(value) = value {
            within(name, value, 0.0, 1.0)?;
        }
    }
    let weight = match (risk, mean_risk) {
        (Some(risk), Some(mean)) if lam > 0.0 && mean > 0.0 => {
            (1.0 - lam) + lam * risk.powf(alpha) / mean
        }
        _ => 1.0,
    };
    Ok((rho * weight).max(EPS_FLOOR).min(pi_max))
}

/// The run-start commitment to K_run: "sha256:" + hex(SHA-256(K_run ‖
/// `run_id`)).
#[must_use]
pub fn commitment(key: &RunKey, run_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.update(run_id.as_bytes());
    format!("sha256:{}", hex(&hasher.finalize()))
}

/// x_i: the first 8 bytes, big-endian, of HMAC-SHA256(K_run, ([`DRAW_TAG`],
/// run id, task id, attempt id, accepted commit)).
#[must_use]
pub fn draw(
    key: &RunKey,
    run_id: &str,
    task_id: &str,
    attempt_id: &str,
    accepted_commit: &str,
) -> u64 {
    let digest = hmac_fields(
        key.as_bytes(),
        &[DRAW_TAG, run_id, task_id, attempt_id, accepted_commit],
    );
    let mut first = [0_u8; 8];
    first.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(first)
}

/// u_i = x_i / 2^64, for display; [`is_selected`] compares exactly.
#[must_use]
pub fn u_value(x: u64) -> f64 {
    x as f64 / TWO_64
}

/// S_i = 1[x_i < π_i·2^64]. π_i·2^64 is an exact integer for every π_i of
/// at least 2^-11, so for every π above the floor the comparison is exact
/// and agrees with Python's.
#[must_use]
pub fn is_selected(x: u64, pi: f64) -> bool {
    u128::from(x) < (pi * TWO_64) as u128
}

/// x_i as the ledger records it: "0x" and 16 lowercase hex digits.
#[must_use]
pub fn prf_hex(x: u64) -> String {
    format!("0x{x:016x}")
}

/// Draw one green unit at probability `pi`.
///
/// # Errors
///
/// A `pi` below [`EPS_FLOOR`] or above 1.
pub fn select(
    key: &RunKey,
    run_id: &str,
    task_id: &str,
    attempt_id: &str,
    accepted_commit: &str,
    pi: f64,
) -> Result<Selection, AuditError> {
    within("pi", pi, EPS_FLOOR, 1.0)?;
    let x = draw(key, run_id, task_id, attempt_id, accepted_commit);
    Ok(Selection {
        run_id: run_id.to_string(),
        task_id: task_id.to_string(),
        attempt_id: attempt_id.to_string(),
        accepted_commit: accepted_commit.to_string(),
        pi,
        prf_u: prf_hex(x),
        selected: is_selected(x, pi),
    })
}

/// Every mismatch between a revealed key and a run's ledger: the
/// commitment, and each selection's draw, π and outcome. Empty when the
/// key opens the commitment and reproduces every selection.
#[must_use]
pub fn verify_reveal(
    key: &RunKey,
    run_id: &str,
    committed: &str,
    selections: &[Selection],
) -> Vec<String> {
    let mut mismatches = Vec::new();
    let opened = commitment(key, run_id);
    if opened != committed {
        mismatches.push(format!(
            "commitment: the revealed key opens {opened}, but the run committed to {committed}"
        ));
    }
    for (index, row) in selections.iter().enumerate() {
        let (task, attempt) = (&row.task_id, &row.attempt_id);
        let at = format!("selection {index} ({task}, attempt {attempt})");
        if row.run_id != run_id {
            mismatches.push(format!("{at}: run_id {} is not {run_id}", row.run_id));
            continue;
        }
        let x = draw(key, run_id, task, attempt, &row.accepted_commit);
        let (pi, recorded, drawn) = (row.pi, &row.prf_u, prf_hex(x));
        if !(EPS_FLOOR..=1.0).contains(&pi) {
            mismatches.push(format!("{at}: pi {pi} is outside [0.05, 1]"));
        }
        if *recorded != drawn {
            mismatches.push(format!("{at}: prf_u {recorded}, but the key draws {drawn}"));
        }
        if row.selected != is_selected(x, pi) {
            let selected = row.selected;
            mismatches.push(format!("{at}: selected {selected}, but the draw differs"));
        }
    }
    mismatches
}

/// The workspace audit secret in the vault's `keys/`: 32 random bytes,
/// mode 0600, made on first use. Every run key derives from it.
///
/// # Errors
///
/// The vault's `keys/` cannot be read or written, or holds a secret shorter
/// than [`KEY_BYTES`].
pub fn workspace_secret(vault: &AuditVault) -> std::io::Result<Vec<u8>> {
    let dir = vault.keys_dir();
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(&dir)?;
    let path = dir.join(SECRET_FILE);
    let checked = |bytes: Vec<u8>| {
        if bytes.len() < KEY_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{} holds fewer than {KEY_BYTES} bytes", path.display()),
            ));
        }
        Ok(bytes)
    };
    match std::fs::read(&path) {
        Ok(bytes) => return checked(bytes),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        Err(_) => {}
    }
    let secret = fresh_secret()?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    match options.open(&path) {
        Ok(mut file) => {
            file.write_all(&secret)?;
            file.sync_all()?;
            Ok(secret.to_vec())
        }
        // Another process made it first: use theirs.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            checked(std::fs::read(&path)?)
        }
        Err(error) => Err(error),
    }
}

/// 32 bytes from the operating system's random source.
fn fresh_secret() -> std::io::Result<[u8; KEY_BYTES]> {
    use std::io::Read as _;
    let mut secret = [0_u8; KEY_BYTES];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut secret)?;
    Ok(secret)
}

/// HMAC-SHA256 over `fields`, each UTF-8 and prefixed by its byte length as
/// a 4-byte big-endian integer (`common.hmac_seed`'s encoding).
fn hmac_fields(key: &[u8], fields: &[&str]) -> [u8; 32] {
    // HMAC takes a key of any length.
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key) else {
        unreachable!("HMAC accepts every key length")
    };
    for field in fields {
        let length = u32::try_from(field.len()).unwrap_or(u32::MAX);
        mac.update(&length.to_be_bytes());
        mac.update(field.as_bytes());
    }
    mac.finalize().into_bytes().into()
}

/// Lowercase hex digits of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The bytes of an even run of hex digits.
pub(crate) fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::Value;

    use super::super::{fixture, number};
    use super::*;

    fn key(entry: &Value) -> RunKey {
        RunKey::from_hex(text(entry, "key_hex")).expect("a 32-byte key")
    }

    fn text<'a>(entry: &'a Value, name: &str) -> &'a str {
        entry[name]
            .as_str()
            .unwrap_or_else(|| panic!("{name} in {entry}"))
    }

    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-12
    }

    #[test]
    fn lottery_matches_the_python_fixtures() {
        let fixture = fixture();
        let constants = &fixture["constants"];
        assert!(close(number(constants, "eps_floor"), EPS_FLOOR));
        assert_eq!(constants["draw_tag"], DRAW_TAG);
        assert_eq!(constants["key_tag"], KEY_TAG);

        for entry in fixture["run_key"].as_array().expect("run_key") {
            let secret = decode_hex(text(entry, "secret_hex")).expect("secret hex");
            let derived = RunKey::derive(&secret, text(entry, "run_id")).expect("a key");
            assert_eq!(derived.to_hex(), text(entry, "key_hex"));
            assert_eq!(format!("{derived:?}"), "RunKey(<unrevealed>)");
        }
        assert!(RunKey::derive(&[0; 31], "run").is_err());
        for entry in fixture["commitment"].as_array().expect("commitment") {
            let opened = commitment(&key(entry), text(entry, "run_id"));
            assert_eq!(opened, text(entry, "commitment"));
        }
        for entry in fixture["draws"].as_array().expect("draws") {
            let x = draw(
                &key(entry),
                text(entry, "run_id"),
                text(entry, "task_id"),
                text(entry, "attempt_id"),
                text(entry, "accepted_commit"),
            );
            assert_eq!(prf_hex(x), text(entry, "prf_u"), "{entry}");
            assert!(close(u_value(x), number(entry, "u")), "{entry}");
            for (pi, selected) in entry["selected"].as_object().expect("selected") {
                let pi: f64 = pi.parse().expect("a pi key");
                let expected = selected.as_bool();
                assert_eq!(Some(is_selected(x, pi)), expected, "{entry} at {pi}");
            }
        }
        for entry in fixture["is_selected"].as_array().expect("is_selected") {
            let digits = text(entry, "prf_u").trim_start_matches("0x");
            let x = u64::from_str_radix(digits, 16).expect("a draw");
            let (pi, expected) = (number(entry, "pi"), entry["selected"].as_bool());
            assert_eq!(Some(is_selected(x, pi)), expected, "{entry}");
        }
        for entry in fixture["inclusion_probability"].as_array().expect("inclusion") {
            let params = InclusionParams {
                rho: number(entry, "rho"),
                lam: number(entry, "lam"),
                alpha: number(entry, "alpha"),
                pi_max: number(entry, "pi_max"),
            };
            let (risk, mean_risk) = (entry["risk"].as_f64(), entry["mean_risk"].as_f64());
            let pi = inclusion_probability(&params, risk, mean_risk).expect("a pi");
            assert!(close(pi, number(entry, "pi")), "{entry}");
        }
        for entry in fixture["tilt"].as_array().expect("tilt") {
            let labels = u32::try_from(entry["n_labels"].as_u64().expect("labels")).expect("u32");
            let (lambda_max, ece_ref) = (number(entry, "lambda_max"), number(entry, "ece_ref"));
            let lam = tilt(entry["ece"].as_f64(), labels, lambda_max, ece_ref).expect("a tilt");
            assert!(close(lam, number(entry, "lam")), "{entry}");
        }

        // A revealed key reproduces every selection; a wrong one does not.
        let entry = &fixture["commitment"][0];
        let (run_key, run_id) = (key(entry), text(entry, "run_id"));
        let selections: Vec<Selection> = [("T1", 0.05), ("T2", 0.3), ("T3", 1.0)]
            .iter()
            .map(|(task, pi)| select(&run_key, run_id, task, &format!("a:{task}:1"), "tree", *pi))
            .collect::<Result<_, _>>()
            .expect("selections");
        assert!(selections[2].selected, "pi = 1 always selects");
        let committed = commitment(&run_key, run_id);
        let mismatches = verify_reveal(&run_key, run_id, &committed, &selections);
        assert!(mismatches.is_empty(), "{mismatches:?}");
        let other = RunKey::from_bytes([7; KEY_BYTES]);
        let mismatches = verify_reveal(&other, run_id, &committed, &selections);
        assert!(!mismatches.is_empty());
        let mut flipped = selections;
        flipped[0].selected = !flipped[0].selected;
        let mismatches = verify_reveal(&run_key, run_id, &committed, &flipped);
        assert_eq!(mismatches.len(), 1, "{mismatches:?}");
        assert!(select(&run_key, run_id, "T", "a", "tree", 0.04).is_err());
    }

    #[test]
    fn the_workspace_secret_is_made_once_and_kept_private() {
        let workspace = tempfile::tempdir().expect("workspace");
        let home = tempfile::tempdir().expect("home");
        let wanted = home.path().join("vault");
        let vault =
            AuditVault::resolve_with(workspace.path(), Some(&wanted), None).expect("a vault");
        let secret = workspace_secret(&vault).expect("a secret");
        assert_eq!(secret.len(), KEY_BYTES);
        assert_eq!(workspace_secret(&vault).expect("the same secret"), secret);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(vault.keys_dir().join(SECRET_FILE))
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let key = RunKey::derive(&secret, "run-1").expect("a key");
        assert_ne!(key, RunKey::derive(&secret, "run-2").expect("another key"));
    }

    proptest! {
        #[test]
        fn pi_never_drops_below_the_floor(
            rho in EPS_FLOOR..=RHO_MAX,
            lam in 0.0_f64..=1.0,
            alpha in 0.1_f64..=4.0,
            pi_max in EPS_FLOOR..=1.0,
            risk in proptest::option::of(0.0_f64..=1.0),
            mean_risk in proptest::option::of(0.0_f64..=1.0),
        ) {
            let params = InclusionParams { rho, lam, alpha, pi_max };
            let pi = inclusion_probability(&params, risk, mean_risk).expect("in range");
            prop_assert!(pi >= EPS_FLOOR, "pi {pi}");
            prop_assert!(pi <= pi_max, "pi {pi}");
        }
    }
}
