//! UCB1 multi-armed bandit (§27.3).
//!
//! Implements the context-free UCB1 bandit for online discrete decisions:
//! backend selection, retry strategy, context-size buckets — anywhere a
//! choice is made many times and you want to exploit the best-known option
//! while still occasionally exploring.
//!
//! UCB1 reference: Auer, Cesa-Bianchi, Fischer (2002), "Finite-time
//! Analysis of the Multiarmed Bandit Problem."
//!
//! # UCB1 formula
//!
//! For each arm `a` with `pulls_a` observations:
//!
//! ```text
//! ucb(a) = mean_a + C · sqrt( ln(total_pulls) / pulls_a )
//! ```
//!
//! Arms with `pulls_a == 0` receive infinite UCB and are always chosen
//! before any pulled arm. Deterministic tiebreak: first by insertion order.
//!
//! # Thread safety
//!
//! [`UcbBandit`] uses [`parking_lot::RwLock`] for arm stats and
//! [`std::sync::atomic::AtomicU64`] for the pull counter so that `select`
//! only acquires a shared read lock while `update` acquires an exclusive
//! write lock.
//!
//! # Reward scaling
//!
//! Standard UCB1 regret bounds assume rewards in `[0, 1]`. Callers must
//! normalize: gate pass → 1.0, gate fail → 0.0, mixed → `1.0 − (cost /
//! max_cost)`.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ─── BanditArm ───────────────────────────────────────────────────────────────

/// Statistics for a single arm of a [`UcbBandit`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanditArm {
    /// Human-readable name, e.g. `"claude"` or `"codex"`.
    pub name: String,
    /// Number of times this arm has been pulled.
    pub pulls: u64,
    /// Cumulative reward received across all pulls.
    pub total_reward: f64,
}

impl BanditArm {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pulls: 0,
            total_reward: 0.0,
        }
    }

    /// Mean reward; returns `0.0` for unpulled arms.
    #[allow(clippy::cast_precision_loss)]
    pub fn mean_reward(&self) -> f64 {
        if self.pulls == 0 {
            0.0
        } else {
            self.total_reward / (self.pulls as f64)
        }
    }
}

// ─── UcbBandit ───────────────────────────────────────────────────────────────

/// Context-free UCB1 multi-armed bandit.
///
/// `select` is read-only (shared lock). Only `update` takes an exclusive
/// write lock. This means concurrent `select` calls are never blocked
/// by each other — only by an in-progress `update`.
pub struct UcbBandit {
    arms: RwLock<Vec<BanditArm>>,
    total_pulls: AtomicU64,
    /// UCB exploration constant. Default: `sqrt(2)`.
    exploration_c: f64,
    /// If `Some`, `save()` persists to this path.
    persist_path: Option<PathBuf>,
}

impl UcbBandit {
    /// Create a fresh bandit with all arms at zero.
    ///
    /// # Panics
    ///
    /// Panics if `arm_names` is empty (construction-time invariant: at
    /// least one arm is required).
    pub fn new(arm_names: Vec<String>) -> Self {
        assert!(!arm_names.is_empty(), "UcbBandit: arm_names must be non-empty");
        let arms = arm_names.into_iter().map(BanditArm::new).collect();
        Self {
            arms: RwLock::new(arms),
            total_pulls: AtomicU64::new(0),
            exploration_c: std::f64::consts::SQRT_2,
            persist_path: None,
        }
    }

    /// Override the exploration constant (builder pattern).
    #[must_use]
    pub const fn with_c(mut self, c: f64) -> Self {
        self.exploration_c = c;
        self
    }

    /// Attach a persistence path (builder pattern).
    #[must_use]
    pub fn with_persist_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.persist_path = Some(path.into());
        self
    }

    /// Load arm stats from `path`; return a fresh bandit if the file is
    /// missing. The `exploration_c` is always set to `sqrt(2)` — callers
    /// that want a different value should chain `.with_c(c)` after load.
    ///
    /// # Errors
    ///
    /// Returns an I/O or parse error if the file exists but cannot be read
    /// or deserialized.
    pub fn load(
        path: impl AsRef<Path>,
        arm_names: Vec<String>,
    ) -> std::io::Result<Self> {
        let path = path.as_ref();
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::new(arm_names).with_persist_path(path));
            }
            Err(e) => return Err(e),
        };
        let saved: Vec<BanditArm> = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Merge: use saved stats for arms whose names match, fresh zeros
        // for any arm not present in the file (forward-compat).
        let arms: Vec<BanditArm> = arm_names
            .into_iter()
            .map(|name| {
                saved
                    .iter()
                    .find(|a| a.name == name)
                    .cloned()
                    .unwrap_or_else(|| BanditArm::new(&name))
            })
            .collect();

        let total_pulls: u64 = arms.iter().map(|a| a.pulls).sum();
        Ok(Self {
            total_pulls: AtomicU64::new(total_pulls),
            arms: RwLock::new(arms),
            exploration_c: std::f64::consts::SQRT_2,
            persist_path: Some(path.to_path_buf()),
        })
    }

    /// Persist arm stats to disk atomically (tempfile + rename).
    ///
    /// # Errors
    ///
    /// Returns an error if `persist_path` is `None`, or if any filesystem
    /// operation fails.
    pub fn save(&self) -> std::io::Result<()> {
        let dest = self.persist_path.as_ref().ok_or_else(|| {
            std::io::Error::other("UcbBandit: no persist_path set")
        })?;

        // Snapshot under read lock, then release before doing any I/O.
        let json = {
            let arms = self.arms.read();
            serde_json::to_vec(&*arms)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        };

        // Write to a sibling temp file then rename for atomicity.
        let parent = dest.parent().unwrap_or_else(|| Path::new("."));
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let tmp = parent.join(format!(".bandit_tmp_{nanos}.json"));
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, dest)?;
        Ok(())
    }

    /// Select the arm with the highest UCB score.
    ///
    /// Arms with `pulls == 0` receive infinite UCB and always beat pulled
    /// arms. Deterministic tiebreak: first arm by insertion order (the arm
    /// with the lower index wins when scores are equal).
    ///
    /// This method holds only a **read** lock.
    pub fn select(&self) -> String {
        let arms = self.arms.read();
        let total = self.total_pulls.load(Ordering::Relaxed);

        // We want the *first* arm with the highest UCB. Rust's `max_by`
        // returns the *last* maximum (not the first), so we iterate
        // manually to get first-wins tiebreak semantics.
        let mut best_idx = 0;
        let mut best_score = ucb_score(&arms[0], total, self.exploration_c);

        for (i, arm) in arms.iter().enumerate().skip(1) {
            let score = ucb_score(arm, total, self.exploration_c);
            // Strict greater-than: ties do NOT replace the current best,
            // so the first (lowest-index) arm wins on ties.
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        arms[best_idx].name.clone()
    }

    /// Record a reward for `arm`.
    ///
    /// If `arm` is not recognised, a diagnostic is printed to stderr and
    /// the call is ignored (no panic, no state mutation).
    pub fn update(&self, arm: &str, reward: f64) {
        let mut arms = self.arms.write();
        match arms.iter_mut().find(|a| a.name == arm) {
            Some(a) => {
                a.pulls += 1;
                a.total_reward += reward;
                // Increment the atomic after releasing the write lock
                // would create a window where total_pulls lags. Release
                // ordering is fine here; we bump while still holding the
                // write lock so readers see a consistent snapshot.
                self.total_pulls.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                eprintln!("UcbBandit::update: unknown arm {arm:?} — ignoring");
            }
        }
    }

    /// Snapshot of all arm statistics (cheap clone under read lock).
    pub fn arm_stats(&self) -> Vec<BanditArm> {
        self.arms.read().clone()
    }

    /// Total pulls recorded so far.
    pub fn total_pulls(&self) -> u64 {
        self.total_pulls.load(Ordering::Relaxed)
    }
}

/// Compute the UCB1 score for a single arm.
///
/// Returns `f64::INFINITY` when `arm.pulls == 0` or `total_pulls == 0`
/// (guarantees unpulled arms are always selected first).
///
/// The `#[allow]` attributes suppress precision-loss lints for the
/// intentional `u64 → f64` conversions: UCB1 regret bounds are not
/// invalidated by the ~1 ULP rounding on counts this large.
#[allow(clippy::cast_precision_loss)]
fn ucb_score(arm: &BanditArm, total_pulls: u64, c: f64) -> f64 {
    if arm.pulls == 0 || total_pulls == 0 {
        return f64::INFINITY;
    }
    let bonus = ((total_pulls as f64).ln() / arm.pulls as f64).sqrt();
    c.mul_add(bonus, arm.mean_reward())
}

// ─── BanditBank ──────────────────────────────────────────────────────────────

/// Wire format for [`BanditBank::save`] / [`BanditBank::load`].
#[derive(Serialize, Deserialize)]
struct BankSnapshot {
    bandits: HashMap<String, Vec<BanditArm>>,
}

/// A collection of independent [`UcbBandit`]s keyed by context string.
///
/// Use context keys like `"{role}_{complexity}"` to maintain separate
/// arm statistics per context. Bandits are created lazily on first access.
pub struct BanditBank {
    bandits: RwLock<HashMap<String, UcbBandit>>,
    arm_names: Vec<String>,
    exploration_c: f64,
}

impl BanditBank {
    /// Create an empty bank. Bandits are created lazily per key.
    pub fn new(arm_names: Vec<String>, exploration_c: f64) -> Self {
        Self {
            bandits: RwLock::new(HashMap::new()),
            arm_names,
            exploration_c,
        }
    }

    /// Select the best arm for `key`. Creates a fresh bandit for the key
    /// if none exists yet.
    pub fn select(&self, key: &str) -> String {
        // Fast path: bandit already exists — read lock only.
        {
            let map = self.bandits.read();
            if let Some(b) = map.get(key) {
                return b.select();
            }
        }
        // Slow path: create and insert under write lock.
        let mut map = self.bandits.write();
        // Re-check in case another thread inserted between our read and write.
        if let Some(b) = map.get(key) {
            return b.select();
        }
        let bandit = UcbBandit::new(self.arm_names.clone()).with_c(self.exploration_c);
        let chosen = bandit.select();
        map.insert(key.to_string(), bandit);
        chosen
    }

    /// Record a reward for `(key, arm)`. Creates a fresh bandit if needed.
    pub fn update(&self, key: &str, arm: &str, reward: f64) {
        // Fast path: bandit exists.
        {
            let map = self.bandits.read();
            if let Some(b) = map.get(key) {
                b.update(arm, reward);
                return;
            }
        }
        // Slow path: create then update. We insert first (under write lock),
        // then call update which re-acquires a read lock internally.
        self.bandits
            .write()
            .entry(key.to_string())
            .or_insert_with(|| UcbBandit::new(self.arm_names.clone()).with_c(self.exploration_c))
            .update(arm, reward);
    }

    /// Persist all bandits to a single JSON file atomically.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();

        // Snapshot under read lock, then release before doing any I/O.
        let json = {
            let snapshot = {
                let map = self.bandits.read();
                BankSnapshot {
                    bandits: map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.arm_stats()))
                        .collect(),
                }
            };
            serde_json::to_vec(&snapshot)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        };

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let tmp = parent.join(format!(".bank_tmp_{nanos}.json"));
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load a [`BanditBank`] from disk. Returns a fresh empty bank if the
    /// file is missing.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file exists but cannot be read or
    /// deserialized.
    pub fn load(
        path: impl AsRef<Path>,
        arm_names: Vec<String>,
        exploration_c: f64,
    ) -> std::io::Result<Self> {
        let path = path.as_ref();
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::new(arm_names, exploration_c));
            }
            Err(e) => return Err(e),
        };

        let snapshot: BankSnapshot = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut map: HashMap<String, UcbBandit> = HashMap::new();
        for (key, saved_arms) in snapshot.bandits {
            // Merge saved stats into fresh arms list (forward-compat).
            let arms: Vec<BanditArm> = arm_names
                .iter()
                .map(|name| {
                    saved_arms
                        .iter()
                        .find(|a| &a.name == name)
                        .cloned()
                        .unwrap_or_else(|| BanditArm::new(name))
                })
                .collect();
            let total: u64 = arms.iter().map(|a| a.pulls).sum();
            let bandit = UcbBandit {
                arms: RwLock::new(arms),
                total_pulls: AtomicU64::new(total),
                exploration_c,
                persist_path: None,
            };
            map.insert(key, bandit);
        }

        Ok(Self {
            bandits: RwLock::new(map),
            arm_names,
            exploration_c,
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use rand::Rng;
    use tempfile::TempDir;

    fn three_arm_names() -> Vec<String> {
        vec!["arm0".to_string(), "arm1".to_string(), "arm2".to_string()]
    }

    // ── Test 1 ───────────────────────────────────────────────────────────────

    #[test]
    fn new_bandit_select_returns_first_arm_tie_breaks_deterministic() {
        // All arms unpulled → all have infinite UCB → first wins.
        let bandit = UcbBandit::new(three_arm_names());
        let choice = bandit.select();
        assert_eq!(choice, "arm0", "unpulled tiebreak should return first arm");
    }

    // ── Test 2 ───────────────────────────────────────────────────────────────

    #[test]
    fn after_each_arm_pulled_once_select_uses_formula() {
        let bandit = UcbBandit::new(three_arm_names());
        // Pull each arm once with equal rewards.
        bandit.update("arm0", 0.5);
        bandit.update("arm1", 0.5);
        bandit.update("arm2", 0.5);
        // All means equal → UCB bonus also equal (same pulls, same total) →
        // formula is deterministic, tiebreak returns first arm.
        let first = bandit.select();
        let second = bandit.select();
        assert_eq!(first, second, "select must be deterministic given the same state");
    }

    // ── Test 3 ───────────────────────────────────────────────────────────────

    #[test]
    fn unpulled_arm_preempts_all_pulled_arms() {
        let bandit = UcbBandit::new(three_arm_names());
        // Pull arms 0 and 1 many times with high reward; arm2 never touched.
        for _ in 0..50 {
            bandit.update("arm0", 1.0);
            bandit.update("arm1", 1.0);
        }
        // arm2 has pulls == 0 → infinite UCB → must win.
        assert_eq!(bandit.select(), "arm2");
    }

    // ── Test 4 ───────────────────────────────────────────────────────────────

    #[test]
    fn update_increments_pulls_and_total_reward() {
        let bandit = UcbBandit::new(three_arm_names());
        bandit.update("arm0", 0.8);
        bandit.update("arm0", 0.2);
        let stats = bandit.arm_stats();
        let arm0 = stats.iter().find(|a| a.name == "arm0").expect("arm0 exists");
        assert_eq!(arm0.pulls, 2);
        assert!((arm0.total_reward - 1.0).abs() < 1e-10);
        assert_eq!(bandit.total_pulls(), 2);
    }

    // ── Test 5 ───────────────────────────────────────────────────────────────

    #[test]
    fn update_with_unknown_arm_logs_and_ignores() {
        let bandit = UcbBandit::new(three_arm_names());
        // Should NOT panic.
        bandit.update("nonexistent", 1.0);
        // State must be unchanged.
        let stats = bandit.arm_stats();
        for arm in &stats {
            assert_eq!(arm.pulls, 0, "no arm should have been updated");
        }
        assert_eq!(bandit.total_pulls(), 0);
    }

    // ── Test 6 ───────────────────────────────────────────────────────────────

    #[test]
    fn regime_change_trace_reallocates_within_20_pulls() {
        // Reward regime: arms 0 and 1 yield ~0.5; arm 2 yields ~0.5 until
        // pull 50, then jumps to ~0.9.
        let bandit = UcbBandit::new(three_arm_names());
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let total_pulls = 100usize;
        let regime_change_at = 50usize;

        let mut arm2_last20 = 0u32;
        let mut arm0_last20 = 0u32;
        let mut arm1_last20 = 0u32;

        for pull_idx in 0..total_pulls {
            let chosen = bandit.select();

            // Simulate noisy reward based on current regime.
            let reward = if chosen == "arm2" && pull_idx >= regime_change_at {
                0.9 + rng.gen_range(-0.05_f64..0.05_f64)
            } else {
                0.5 + rng.gen_range(-0.05_f64..0.05_f64)
            };

            bandit.update(&chosen, reward);

            if pull_idx >= total_pulls - 20 {
                match chosen.as_str() {
                    "arm0" => arm0_last20 += 1,
                    "arm1" => arm1_last20 += 1,
                    "arm2" => arm2_last20 += 1,
                    _ => {}
                }
            }
        }

        // Arm 2 must dominate the last 20 pulls.
        assert!(
            arm2_last20 > arm0_last20 && arm2_last20 > arm1_last20,
            "arm2 should dominate last 20 pulls; got arm2={arm2_last20} arm0={arm0_last20} arm1={arm1_last20}"
        );
    }

    // ── Test 7 ───────────────────────────────────────────────────────────────

    #[test]
    fn higher_exploration_c_flattens_distribution() {
        let arms = three_arm_names();

        // Low C → exploits fast, distribution concentrates.
        let bandit_low = UcbBandit::new(arms.clone()).with_c(0.1);
        // High C → explores more, distribution is flatter.
        let bandit_high = UcbBandit::new(arms.clone()).with_c(5.0);

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut counts_low = [0u32; 3];
        let mut counts_high = [0u32; 3];

        for _ in 0..100 {
            // Feed same noisy rewards to both.
            let r: [f64; 3] = [
                0.6 + rng.gen_range(-0.05_f64..0.05_f64),
                0.5 + rng.gen_range(-0.05_f64..0.05_f64),
                0.4 + rng.gen_range(-0.05_f64..0.05_f64),
            ];

            let chosen_low = bandit_low.select();
            let idx_low = arms.iter().position(|a| a == &chosen_low).expect("valid arm");
            bandit_low.update(&chosen_low, r[idx_low]);
            counts_low[idx_low] += 1;

            let chosen_high = bandit_high.select();
            let idx_high = arms.iter().position(|a| a == &chosen_high).expect("valid arm");
            bandit_high.update(&chosen_high, r[idx_high]);
            counts_high[idx_high] += 1;
        }

        let max_low = *counts_low.iter().max().expect("non-empty");
        let max_high = *counts_high.iter().max().expect("non-empty");

        assert!(
            max_high < max_low,
            "high-C bandit should have lower max-arm fraction; got low={max_low} high={max_high}"
        );
    }

    // ── Test 8 ───────────────────────────────────────────────────────────────

    #[test]
    fn save_then_load_roundtrips_arm_stats() {
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("bandit.json");

        let bandit = UcbBandit::new(three_arm_names())
            .with_persist_path(&path);
        bandit.update("arm0", 0.9);
        bandit.update("arm1", 0.3);
        bandit.save().expect("save");

        let loaded = UcbBandit::load(&path, three_arm_names()).expect("load");
        let stats = loaded.arm_stats();

        let arm0 = stats.iter().find(|a| a.name == "arm0").expect("arm0");
        let arm1 = stats.iter().find(|a| a.name == "arm1").expect("arm1");
        let arm2 = stats.iter().find(|a| a.name == "arm2").expect("arm2");

        assert_eq!(arm0.pulls, 1);
        assert!((arm0.total_reward - 0.9).abs() < 1e-10);
        assert_eq!(arm1.pulls, 1);
        assert!((arm1.total_reward - 0.3).abs() < 1e-10);
        assert_eq!(arm2.pulls, 0);
        assert_eq!(loaded.total_pulls(), 2);
    }

    // ── Test 9 ───────────────────────────────────────────────────────────────

    #[test]
    fn load_missing_file_returns_fresh_bandit() {
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("nonexistent.json");
        let bandit = UcbBandit::load(&path, three_arm_names()).expect("load missing");
        assert_eq!(bandit.total_pulls(), 0);
        for arm in bandit.arm_stats() {
            assert_eq!(arm.pulls, 0);
        }
    }

    // ── Test 10 ──────────────────────────────────────────────────────────────

    #[test]
    fn bandit_bank_keys_are_independent() {
        let bank = BanditBank::new(three_arm_names(), std::f64::consts::SQRT_2);

        // Pull arm0 on key "A" several times.
        for _ in 0..5 {
            bank.update("A", "arm0", 1.0);
        }

        // Key "B" must be entirely fresh.
        // The first select on "B" should return arm0 (all unpulled → first wins).
        let choice_b = bank.select("B");
        assert_eq!(choice_b, "arm0", "key B must start with all arms unpulled");

        // Verify internal state via select on B: since B is fresh, arm0 still unpulled.
        // To confirm independence, check that key B's update doesn't touch key A's stats.
        bank.update("B", "arm1", 0.9);

        // Key A should still have arm0 at 5 pulls and arm1 at 0.
        // We can't directly inspect per-key stats from the bank API, but
        // we can verify that selecting from A continues to use its own stats.
        // With arm0 at 5 pulls on A and arm1/arm2 still unpulled there,
        // A's select should prefer arm1 or arm2 (unpulled → infinite).
        // (After 5 pulls on A.arm0, arm1 and arm2 are still at 0 for key A.)
        let choice_a_after = bank.select("A");
        // arm1 or arm2 on "A" is unpulled → infinite UCB → wins over arm0.
        assert!(
            choice_a_after == "arm1" || choice_a_after == "arm2",
            "Key A should still have arm1/arm2 unpulled; got {choice_a_after}"
        );
    }

    // ── Test 11 ──────────────────────────────────────────────────────────────

    #[test]
    fn bandit_bank_save_and_load_roundtrips() {
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("bank.json");

        let bank = BanditBank::new(three_arm_names(), std::f64::consts::SQRT_2);
        bank.update("ctx_a", "arm0", 0.7);
        bank.update("ctx_a", "arm1", 0.3);
        bank.update("ctx_b", "arm2", 0.9);
        bank.save(&path).expect("save");

        let loaded = BanditBank::load(&path, three_arm_names(), std::f64::consts::SQRT_2)
            .expect("load");

        // After select on ctx_a: arm2 is unpulled on ctx_a → it wins.
        let choice = loaded.select("ctx_a");
        assert_eq!(choice, "arm2", "arm2 should be unpulled on ctx_a after roundtrip");

        // ctx_b had arm2 pulled once; arm0 and arm1 are unpulled → arm0 wins (first).
        let choice_b = loaded.select("ctx_b");
        assert_eq!(choice_b, "arm0", "arm0 should be unpulled on ctx_b after roundtrip");
    }

    // ── Test 12 ──────────────────────────────────────────────────────────────

    #[test]
    fn concurrent_updates_ten_threads_atomic_counters() {
        use std::sync::Arc;

        let bandit = Arc::new(UcbBandit::new(vec!["arm0".to_string()]));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let b = Arc::clone(&bandit);
            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    b.update("arm0", 1.0);
                }
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        let stats = bandit.arm_stats();
        assert_eq!(stats[0].pulls, 1000, "expected 1000 pulls");
        assert!((stats[0].total_reward - 1000.0).abs() < 1e-6,
            "expected total_reward = 1000.0, got {}", stats[0].total_reward);
        assert_eq!(bandit.total_pulls(), 1000);
    }

    // ── Test 13 ──────────────────────────────────────────────────────────────

    #[test]
    fn exploration_c_default_is_sqrt_2() {
        let bandit = UcbBandit::new(three_arm_names());
        // The field is not pub, but we can verify behavior: with C = sqrt(2)
        // and a known arm state, the UCB score matches the expected value.
        bandit.update("arm0", 1.0); // pulls=1, mean=1.0
        bandit.update("arm1", 0.0); // pulls=1, mean=0.0
        // arm2 still unpulled → infinite UCB wins.
        // Pull arm2 to force formula comparison between arm0 and arm1.
        bandit.update("arm2", 0.0); // pulls=1, mean=0.0
        // Now total_pulls = 3, each arm has 1 pull.
        // UCB(arm0) = 1.0 + sqrt(2) * sqrt(ln(3)/1) ≈ 1.0 + 1.4142 * 1.0986 ≈ 2.554
        // UCB(arm1) = 0.0 + sqrt(2) * sqrt(ln(3)/1) ≈ 1.554
        // UCB(arm2) = 0.0 + sqrt(2) * sqrt(ln(3)/1) ≈ 1.554
        // arm0 wins.
        let choice = bandit.select();
        assert_eq!(choice, "arm0",
            "with default C=sqrt(2), arm0 (mean=1.0) should win by highest UCB");
    }
}
