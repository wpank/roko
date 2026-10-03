//! "Break a loop" fault flags and their ground-truth writer (S03 §4.9; backlog 5120).
//!
//! A flag breaks one loop in one way ([`FaultKind`]) until its time to live
//! (at most [`MAX_TTL_SECS`]) or its decision budget runs out, whichever
//! comes first. Read sites ask [`active`], the only read API; the auditor's
//! estimators never do. Flags exist only with the `fault-injection` feature,
//! and `set` works only after `enable` (serve's admin route) or
//! `enable_from_env` (`ROKO_FAULTS=1` in the CLI) has named the run's
//! `faults.jsonl`. Each set, hit, expiry and clear appends a `loop.fault` row
//! there ([`FaultRecord`](super::ledger::FaultRecord)), which only the
//! evaluation joins. Without the feature, [`active`] is a `const fn` that
//! returns `None`, and nothing can set a flag.
//!
//! A flag of a dry-run kind (every kind but HARMFUL) touches only the reads
//! made inside [`dry_run`], such as a canary's or E1's plans; a live read
//! sees HARMFUL alone (decision 5101 §9.10), and a read a flag skips is no
//! decision of it.

#[cfg(feature = "fault-injection")]
use std::cell::Cell;
#[cfg(feature = "fault-injection")]
use std::path::PathBuf;
#[cfg(feature = "fault-injection")]
use std::sync::{Mutex, PoisonError};
#[cfg(feature = "fault-injection")]
use std::time::{Duration, Instant};

#[cfg(feature = "fault-injection")]
use super::ledger::{FAULT_SCHEMA, FaultEvent, FaultRecord, append_row};
pub use super::ledger::{FaultActor, FaultKind};

/// The longest a flag lives, in seconds (S03 §4.9).
pub const MAX_TTL_SECS: u64 = 1800;

/// HARMFUL's spend cap per run, in USD (S03 §4.9).
pub const HARMFUL_SPEND_CAP_USD: f64 = 1.50;

/// The variable that enables flags in the CLI when it is `1`.
pub const FAULTS_ENV: &str = "ROKO_FAULTS";

/// A flag to set: the loop it breaks, how, and for how long.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone, PartialEq)]
pub struct FaultSpec {
    /// The loop to break.
    pub loop_id: String,
    /// How it breaks.
    pub kind: FaultKind,
    /// Seconds the flag lives, 1 to [`MAX_TTL_SECS`].
    pub ttl_secs: u64,
    /// Decisions it may affect, at least 1.
    pub max_decisions: u64,
    /// HARMFUL's spend cap for the run, at most [`HARMFUL_SPEND_CAP_USD`],
    /// which is also its default. The other kinds run on dry-run plans and
    /// ignore it.
    pub spend_cap_usd: Option<f64>,
}

/// Why a flag could not be set.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FaultError {
    /// Neither `ROKO_FAULTS=1` nor the admin route enabled flags.
    #[error("fault flags are not enabled: set ROKO_FAULTS=1 or use the admin route")]
    Disabled,
    /// The time to live is outside 1 to [`MAX_TTL_SECS`] seconds.
    #[error("a fault flag lives 1 to 1800 s, not {0} s")]
    Ttl(u64),
    /// The decision budget is 0.
    #[error("a fault flag needs a budget of at least one decision")]
    NoDecisions,
    /// HARMFUL's spend cap is outside (0, [`HARMFUL_SPEND_CAP_USD`]].
    #[error("HARMFUL's spend cap is at most $1.50 per run, not ${0}")]
    SpendCap(f64),
    /// The loop already has a flag.
    #[error("loop {0} already has a fault flag; clear it first")]
    Busy(String),
    /// The ground truth could not be written, so the flag was not set.
    #[error("writing the fault ground truth failed: {0}")]
    GroundTruth(String),
}

/// A set flag and what it has done so far.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone)]
struct Flag {
    fault_id: String,
    spec: FaultSpec,
    armed_at: Instant,
    decisions: u64,
    spent_usd: f64,
}

/// The process's flags and where their ground truth goes.
#[cfg(feature = "fault-injection")]
#[derive(Debug)]
struct Registry {
    actor: FaultActor,
    faults_file: PathBuf,
    flags: Vec<Flag>,
    seq: u64,
}

/// The flags of this process; `None` while they are disabled.
#[cfg(feature = "fault-injection")]
static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

#[cfg(feature = "fault-injection")]
impl Registry {
    /// A registry writing to `faults_file` as `actor`, numbering its rows
    /// after those already in the file.
    fn new(actor: FaultActor, faults_file: PathBuf) -> Self {
        let seq =
            std::fs::read_to_string(&faults_file).map_or(0, |text| text.lines().count() as u64);
        Self {
            actor,
            faults_file,
            flags: Vec::new(),
            seq,
        }
    }

    /// Set the flag `spec` at `now`, once the flags that ran out are gone.
    /// Its id names the row that arms it.
    fn arm(&mut self, spec: FaultSpec, now: Instant) -> Result<String, FaultError> {
        if !(1..=MAX_TTL_SECS).contains(&spec.ttl_secs) {
            return Err(FaultError::Ttl(spec.ttl_secs));
        }
        if spec.max_decisions == 0 {
            return Err(FaultError::NoDecisions);
        }
        let spend_cap_usd = if spec.kind == FaultKind::Harmful {
            let cap = spec.spend_cap_usd.unwrap_or(HARMFUL_SPEND_CAP_USD);
            if !(cap > 0.0 && cap <= HARMFUL_SPEND_CAP_USD) {
                return Err(FaultError::SpendCap(cap));
            }
            Some(cap)
        } else {
            None
        };
        self.expire(now);
        if self.index(&spec.loop_id).is_some() {
            return Err(FaultError::Busy(spec.loop_id));
        }
        let flag = Flag {
            fault_id: format!("fault-{}", self.seq + 1),
            spec: FaultSpec {
                spend_cap_usd,
                ..spec
            },
            armed_at: now,
            decisions: 0,
            spent_usd: 0.0,
        };
        self.write(&flag, FaultEvent::Armed)
            .map_err(|error| FaultError::GroundTruth(error.to_string()))?;
        let fault_id = flag.fault_id.clone();
        self.flags.push(flag);
        Ok(fault_id)
    }

    /// The kind of `loop_id`'s flag at `now`. The call is one decision the
    /// flag affects, and the flag expires after its last.
    fn active(&mut self, loop_id: &str, now: Instant) -> Option<FaultKind> {
        self.expire(now);
        let index = self.index(loop_id)?;
        self.flags[index].decisions += 1;
        let flag = self.flags[index].clone();
        self.record(&flag, FaultEvent::Hit);
        if flag.decisions >= flag.spec.max_decisions {
            self.flags.remove(index);
            self.record(&flag, FaultEvent::Expired);
        }
        Some(flag.spec.kind)
    }

    /// [`Self::active`] for a read made in a dry run or not: a dry-run kind
    /// touches dry runs only, and a read it skips is no decision.
    fn active_in(&mut self, loop_id: &str, now: Instant, dry_run: bool) -> Option<FaultKind> {
        let harmful = self
            .index(loop_id)
            .is_some_and(|index| self.flags[index].spec.kind == FaultKind::Harmful);
        if dry_run || harmful {
            self.active(loop_id, now)
        } else {
            None
        }
    }

    /// Clear `loop_id`'s flag; whether it had one.
    fn clear(&mut self, loop_id: &str) -> bool {
        let Some(index) = self.index(loop_id) else {
            return false;
        };
        let flag = self.flags.remove(index);
        self.record(&flag, FaultEvent::Cleared);
        true
    }

    /// Charge `usd` of HARMFUL spend to `loop_id`'s flag, which expires once
    /// the spend reaches its cap.
    fn charge(&mut self, loop_id: &str, usd: f64) {
        let Some(index) = self.index(loop_id) else {
            return;
        };
        let flag = &mut self.flags[index];
        flag.spent_usd += usd.max(0.0);
        let Some(cap) = flag.spec.spend_cap_usd else {
            return;
        };
        if flag.spent_usd >= cap {
            let flag = self.flags.remove(index);
            self.record(&flag, FaultEvent::Expired);
        }
    }

    /// Expire the flags whose time ran out by `now`.
    fn expire(&mut self, now: Instant) {
        let ran_out = |flag: &Flag| {
            now.saturating_duration_since(flag.armed_at) >= Duration::from_secs(flag.spec.ttl_secs)
        };
        let flags = std::mem::take(&mut self.flags);
        let (expired, live): (Vec<Flag>, Vec<Flag>) = flags.into_iter().partition(ran_out);
        self.flags = live;
        for flag in &expired {
            self.record(flag, FaultEvent::Expired);
        }
    }

    /// The index of `loop_id`'s flag.
    fn index(&self, loop_id: &str) -> Option<usize> {
        self.flags
            .iter()
            .position(|flag| flag.spec.loop_id == loop_id)
    }

    /// Append `flag`'s `event` row to the faults file.
    fn write(&mut self, flag: &Flag, event: FaultEvent) -> std::io::Result<()> {
        self.seq += 1;
        let (fault_id, decisions) = (&flag.fault_id, flag.decisions);
        let identity = format!("{FAULT_SCHEMA}|{fault_id}|{event:?}|{decisions}");
        let record_id = format!("b3:{}", blake3::hash(identity.as_bytes()).to_hex());
        let row = FaultRecord {
            schema_version: FAULT_SCHEMA.to_string(),
            record_id: Some(record_id),
            seq: Some(self.seq),
            ts: Some(chrono::Utc::now().to_rfc3339()),
            kind: "loop.fault".to_string(),
            event,
            fault_id: flag.fault_id.clone(),
            loop_id: flag.spec.loop_id.clone(),
            fault: flag.spec.kind,
            ttl_s: flag.spec.ttl_secs,
            max_decisions: flag.spec.max_decisions,
            decisions_affected: flag.decisions,
            actor: self.actor,
            dry_run: flag.spec.kind != FaultKind::Harmful,
            spend_cap_usd: flag.spec.spend_cap_usd,
        };
        append_row(&self.faults_file, &row)
    }

    /// [`Self::write`], logging a failure: a read site's decision does not
    /// wait on the ground truth.
    fn record(&mut self, flag: &Flag, event: FaultEvent) {
        if let Err(error) = self.write(flag, event) {
            tracing::warn!(%error, fault_id = %flag.fault_id, "fault ground truth not written");
        }
    }
}

/// Run `f` on the registry; `None` while flags are disabled.
#[cfg(feature = "fault-injection")]
fn with_registry<T>(f: impl FnOnce(&mut Registry) -> T) -> Option<T> {
    let mut guard = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    guard.as_mut().map(f)
}

/// Enable flags for this process, their ground truth going to `faults_file`
/// (the run's `faults.jsonl`) as `actor`. Enabling again moves the ground
/// truth and keeps the flags already set.
#[cfg(feature = "fault-injection")]
pub fn enable(actor: FaultActor, faults_file: impl Into<PathBuf>) {
    let mut guard = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    let mut registry = Registry::new(actor, faults_file.into());
    if let Some(previous) = guard.take() {
        registry.flags = previous.flags;
    }
    *guard = Some(registry);
}

/// Enable flags as the CLI does, when `ROKO_FAULTS=1`; whether it did.
#[cfg(feature = "fault-injection")]
pub fn enable_from_env(faults_file: impl Into<PathBuf>) -> bool {
    let on = std::env::var(FAULTS_ENV).is_ok_and(|value| value.trim() == "1");
    if on {
        enable(FaultActor::Env, faults_file);
    }
    on
}

/// Disable flags, clearing every flag still set.
#[cfg(feature = "fault-injection")]
pub fn disable() {
    let mut guard = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(mut registry) = guard.take() {
        for flag in std::mem::take(&mut registry.flags) {
            registry.record(&flag, FaultEvent::Cleared);
        }
    }
}

/// Set a flag; its id.
///
/// # Errors
///
/// [`FaultError::Disabled`] before `enable`, an out-of-bounds TTL, decision
/// budget or HARMFUL spend cap, a loop that already has a flag, or a ground
/// truth that could not be written.
#[cfg(feature = "fault-injection")]
pub fn set(spec: FaultSpec) -> Result<String, FaultError> {
    with_registry(|registry| registry.arm(spec, Instant::now()))
        .unwrap_or(Err(FaultError::Disabled))
}

/// Clear `loop_id`'s flag; whether it had one.
#[cfg(feature = "fault-injection")]
pub fn clear(loop_id: &str) -> bool {
    with_registry(|registry| registry.clear(loop_id)).unwrap_or(false)
}

/// Charge `usd` that a HARMFUL attempt spent to `loop_id`'s flag, which
/// expires once the spend reaches its cap.
#[cfg(feature = "fault-injection")]
pub fn charge(loop_id: &str, usd: f64) {
    with_registry(|registry| registry.charge(loop_id, usd));
}

/// The fault set on `loop_id`, if any: the only read API. Each call is one
/// decision the flag affects (a `hit` row), and the flag expires after its
/// TTL or its last decision, whichever comes first. Outside a [`dry_run`]
/// only a HARMFUL flag is seen.
#[cfg(feature = "fault-injection")]
#[must_use]
pub fn active(loop_id: &str) -> Option<FaultKind> {
    let dry_run = DRY_RUN.with(Cell::get);
    with_registry(|registry| registry.active_in(loop_id, Instant::now(), dry_run)).flatten()
}

#[cfg(feature = "fault-injection")]
thread_local! {
    /// Whether this thread is inside a [`dry_run`].
    static DRY_RUN: Cell<bool> = const { Cell::new(false) };
}

/// Restores the thread's dry-run mark when a [`dry_run`] ends, on a panic
/// too.
#[cfg(feature = "fault-injection")]
struct DryRunMark(bool);

#[cfg(feature = "fault-injection")]
impl Drop for DryRunMark {
    fn drop(&mut self) {
        DRY_RUN.with(|dry_run| dry_run.set(self.0));
    }
}

/// Run `plan` as a dry run: the reads it makes on this thread see a flag of
/// any kind, where a live read sees HARMFUL alone (decision 5101 §9.10). A
/// canary trace and E1's plans run inside one.
pub fn dry_run<T>(plan: impl FnOnce() -> T) -> T {
    #[cfg(feature = "fault-injection")]
    let _mark = DryRunMark(DRY_RUN.with(|dry_run| dry_run.replace(true)));
    plan()
}

/// The fault set on `loop_id`: none, as this build has no fault flags.
#[cfg(not(feature = "fault-injection"))]
#[must_use]
pub const fn active(_loop_id: &str) -> Option<FaultKind> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `text` uses `word` as a word, not inside a longer identifier
    /// (as `faults` sits inside `defaults`).
    fn mentions(text: &str, word: &str) -> bool {
        let identifier = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        text.match_indices(word).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + word.len()..].chars().next();
            !identifier(before) && !identifier(after)
        })
    }

    /// The auditor's estimators never read fault flags: their sources do not
    /// name this module (S03 §4.9).
    fn assert_estimators_blind() {
        assert!(mentions("use super::faults;", "faults"));
        assert!(!mentions("S03 §4.10's defaults", "faults"));
        for (file, source) in [
            ("exposure.rs", include_str!("exposure.rs")),
            ("estimators.rs", include_str!("estimators.rs")),
            ("state.rs", include_str!("state.rs")),
        ] {
            assert!(!mentions(source, "faults"), "{file} mentions faults");
        }
    }

    /// S03 §4.9: a flag expires by count and by time, whichever comes first;
    /// its bounds hold; HARMFUL records its spend cap and expires at it;
    /// every set, hit and expiry leaves a `loop.fault` row; and the
    /// estimators never read flags.
    #[cfg(feature = "fault-injection")]
    #[test]
    fn fault_flags_expire_and_are_invisible_to_estimators() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("runs/gr-faults/faults.jsonl");
        let mut registry = Registry::new(FaultActor::Env, file.clone());
        let t0 = Instant::now();
        let after = |secs| t0 + Duration::from_secs(secs);
        let spec = |loop_id: &str, kind, ttl_secs, max_decisions| FaultSpec {
            loop_id: loop_id.to_string(),
            kind,
            ttl_secs,
            max_decisions,
            spend_cap_usd: None,
        };

        // By count: the third decision is the flag's last.
        let cut = registry
            .arm(spec("L-know", FaultKind::Cut, 600, 3), t0)
            .expect("arm CUT");
        let busy = registry.arm(spec("L-know", FaultKind::Mask, 600, 3), t0);
        assert_eq!(busy, Err(FaultError::Busy("L-know".to_string())));
        for _ in 0..3 {
            assert_eq!(registry.active("L-know", t0), Some(FaultKind::Cut));
        }
        assert_eq!(registry.active("L-know", t0), None);

        // By time: the flag is gone once its TTL has passed.
        registry
            .arm(spec("L-play", FaultKind::Stale, 60, 100), t0)
            .expect("arm STALE");
        assert_eq!(registry.active("L-play", after(30)), Some(FaultKind::Stale));
        assert_eq!(registry.active("L-play", after(60)), None);
        assert_eq!(registry.active("L-other", t0), None);

        // The bounds.
        let arm = |registry: &mut Registry, ttl_secs, max_decisions| {
            registry.arm(spec("L-sec", FaultKind::Cut, ttl_secs, max_decisions), t0)
        };
        assert_eq!(arm(&mut registry, 0, 1), Err(FaultError::Ttl(0)));
        assert_eq!(arm(&mut registry, 1801, 1), Err(FaultError::Ttl(1801)));
        assert_eq!(arm(&mut registry, 60, 0), Err(FaultError::NoDecisions));

        // HARMFUL: a cap above $1.50 is refused; the default cap ends it.
        let mut harmful = spec("L-route", FaultKind::Harmful, 1800, 20);
        harmful.spend_cap_usd = Some(2.0);
        assert_eq!(
            registry.arm(harmful.clone(), t0),
            Err(FaultError::SpendCap(2.0))
        );
        harmful.spend_cap_usd = None;
        registry.arm(harmful, t0).expect("arm HARMFUL");
        registry.charge("L-route", 1.0);
        assert_eq!(registry.active("L-route", t0), Some(FaultKind::Harmful));
        registry.charge("L-route", 0.5);
        assert_eq!(registry.active("L-route", t0), None, "it spent its cap");
        assert!(!registry.clear("L-route"));

        // Ground truth: every set, hit and expiry, in order.
        let text = std::fs::read_to_string(&file).expect("the faults file");
        let rows: Vec<FaultRecord> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("a fault row"))
            .collect();
        let events: Vec<(&str, FaultEvent, u64)> = rows
            .iter()
            .map(|row| (row.loop_id.as_str(), row.event, row.decisions_affected))
            .collect();
        assert_eq!(
            events,
            [
                ("L-know", FaultEvent::Armed, 0),
                ("L-know", FaultEvent::Hit, 1),
                ("L-know", FaultEvent::Hit, 2),
                ("L-know", FaultEvent::Hit, 3),
                ("L-know", FaultEvent::Expired, 3),
                ("L-play", FaultEvent::Armed, 0),
                ("L-play", FaultEvent::Hit, 1),
                ("L-play", FaultEvent::Expired, 1),
                ("L-route", FaultEvent::Armed, 0),
                ("L-route", FaultEvent::Hit, 1),
                ("L-route", FaultEvent::Expired, 1),
            ]
        );
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.seq, Some(index as u64 + 1));
            assert_eq!(row.schema_version, FAULT_SCHEMA);
            assert_eq!(row.actor, FaultActor::Env);
        }
        assert_eq!(rows[0].fault_id, cut);
        assert!(rows[0].dry_run);
        assert!(!rows[8].dry_run);
        assert_eq!(rows[8].spend_cap_usd, Some(HARMFUL_SPEND_CAP_USD));

        assert_estimators_blind();
    }

    /// Without the feature, no flag exists, and `active` is a `const fn`.
    #[cfg(not(feature = "fault-injection"))]
    #[test]
    fn active_is_none_without_the_feature() {
        const NONE: Option<FaultKind> = active("L-know");
        assert_eq!(NONE, None);
        assert_estimators_blind();
    }
}
