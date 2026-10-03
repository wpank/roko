//! Prompt-section effectiveness tracking for gate-to-scaffold feedback.
//!
//! This module tracks whether including a prompt section for a given role
//! correlates with higher gate pass rates. The registry is keyed by
//! `(section_name, role)` and persisted as JSON so prompt assembly can later
//! adjust section priorities using the learned lift.
//!
//! [`SectionBandit`] is the learner that replaces it on the Graph path (S02
//! L9): it leaves droppable sections out at a logged, random rate, so a
//! section that does not help can be found out.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;

use crate::model_router::sample_beta;
use crate::telemetry::Assignment;

/// Default relative path used to persist section-effect snapshots.
pub const DEFAULT_SECTION_EFFECTS_PATH: &str = ".roko/learn/section-effects.json";

/// Suggested priority adjustment for a prompt section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriorityChange {
    /// Increase the section's priority because it improves pass rate.
    Increase,
    /// Decrease the section's priority because it appears to hurt pass rate.
    Decrease,
    /// Keep the section's current priority.
    NoChange,
    /// Not enough trials have accumulated to make a recommendation.
    InsufficientData,
}

/// Inclusion/exclusion outcome statistics for one prompt section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionEffect {
    /// Prompt section name, such as `"workspace_map"`.
    pub section_name: String,
    /// Number of trials where the section was included.
    pub included_trials: u64,
    /// Number of successful trials where the section was included.
    pub included_passes: u64,
    /// Number of trials where the section was excluded.
    pub excluded_trials: u64,
    /// Number of successful trials where the section was excluded.
    pub excluded_passes: u64,
}

impl SectionEffect {
    /// Create a new zeroed tracker for `section_name`.
    #[must_use]
    pub fn new(section_name: impl Into<String>) -> Self {
        Self {
            section_name: section_name.into(),
            included_trials: 0,
            included_passes: 0,
            excluded_trials: 0,
            excluded_passes: 0,
        }
    }

    /// Record one task outcome for the section.
    pub fn record(&mut self, included: bool, passed: bool) {
        if included {
            self.included_trials = self.included_trials.saturating_add(1);
            if passed {
                self.included_passes = self.included_passes.saturating_add(1);
            }
        } else {
            self.excluded_trials = self.excluded_trials.saturating_add(1);
            if passed {
                self.excluded_passes = self.excluded_passes.saturating_add(1);
            }
        }
    }

    /// Pass-rate lift when the section is included versus excluded.
    #[must_use]
    pub fn lift(&self) -> f64 {
        let included_rate = self.included_passes as f64 / self.included_trials.max(1) as f64;
        let excluded_rate = self.excluded_passes as f64 / self.excluded_trials.max(1) as f64;
        included_rate - excluded_rate
    }

    /// Multiplicative budget weight derived from the measured lift.
    ///
    /// The weight is centered at `1.0`, with low-lift sections floored at
    /// `0.5` and strong sections capped at `1.5`.
    #[must_use]
    pub fn lift_weight(&self) -> f64 {
        (1.0 + self.lift()).clamp(0.5, 1.5)
    }

    /// Return the lift weight in a one-entry map keyed by section name.
    #[must_use]
    pub fn lift_weights(&self) -> HashMap<String, f64> {
        HashMap::from([(self.section_name.clone(), self.lift_weight())])
    }

    /// Recommend whether the section priority should change.
    #[must_use]
    pub fn recommend_priority_change(&self) -> PriorityChange {
        if self.included_trials < 20 || self.excluded_trials < 5 {
            return PriorityChange::InsufficientData;
        }

        let lift = self.lift();
        if lift > 0.05 {
            PriorityChange::Increase
        } else if lift < -0.02 {
            PriorityChange::Decrease
        } else {
            PriorityChange::NoChange
        }
    }
}

/// Thread-agnostic registry of section effectiveness keyed by `(section, role)`.
#[derive(Debug, Clone, Default)]
pub struct SectionEffectivenessRegistry {
    effects: HashMap<(String, String), SectionEffect>,
}

impl SectionEffectivenessRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the registry from `path`, or return an empty registry if missing
    /// or invalid.
    #[must_use]
    pub fn load_or_new(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|contents| {
                serde_json::from_str::<SectionEffectivenessSnapshot>(&contents).ok()
            })
            .map(Self::from_snapshot)
            .unwrap_or_default()
    }

    /// Save the registry to `path` using an atomic rename.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry cannot be serialized or if the
    /// snapshot cannot be written to disk.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let snapshot = self.snapshot();
        roko_fs::atomic_write_json(path, &snapshot)
    }

    /// Record one included/excluded outcome for `section_name` scoped to `role`.
    pub fn record_outcome(
        &mut self,
        section_name: impl Into<String>,
        role: impl Into<String>,
        included: bool,
        passed: bool,
    ) {
        let section_name = section_name.into();
        let role = role.into();
        let key = (section_name.clone(), role);
        let effect = self
            .effects
            .entry(key)
            .or_insert_with(|| SectionEffect::new(section_name));
        effect.record(included, passed);
    }

    /// Return the tracked effect for one `(section, role)` pair.
    #[must_use]
    pub fn get(&self, section_name: &str, role: &str) -> Option<&SectionEffect> {
        self.effects
            .get(&(section_name.to_owned(), role.to_owned()))
    }

    /// Recommend how prompt assembly should adjust this section's priority.
    #[must_use]
    pub fn recommend_priority_change(&self, section_name: &str, role: &str) -> PriorityChange {
        self.get(section_name, role).map_or(
            PriorityChange::InsufficientData,
            SectionEffect::recommend_priority_change,
        )
    }

    /// Return sections for `role` whose inclusion currently shows positive lift.
    ///
    /// Results are sorted by descending lift, then by section name.
    #[must_use]
    pub fn positive_lift_sections(&self, role: &str) -> Vec<SectionEffect> {
        let mut sections: Vec<_> = self
            .effects
            .iter()
            .filter(|((_, effect_role), effect)| {
                effect_role == role && effect.lift().is_sign_positive() && effect.lift() > 0.0
            })
            .map(|(_, effect)| effect.clone())
            .collect();

        sections.sort_by(|a, b| {
            b.lift()
                .total_cmp(&a.lift())
                .then(a.section_name.cmp(&b.section_name))
        });
        sections
    }

    /// Return multiplicative prompt-budget weights keyed by section name.
    ///
    /// When the same section appears under multiple roles, the strongest
    /// observed lift wins so the composer can bias toward the highest-signal
    /// version without double-counting.
    #[must_use]
    pub fn lift_weights(&self) -> HashMap<String, f64> {
        let mut weights = HashMap::new();
        for effect in self.effects.values() {
            weights
                .entry(effect.section_name.clone())
                .and_modify(|weight: &mut f64| *weight = (*weight).max(effect.lift_weight()))
                .or_insert_with(|| effect.lift_weight());
        }
        weights
    }

    fn snapshot(&self) -> SectionEffectivenessSnapshot {
        let mut entries: Vec<_> = self
            .effects
            .iter()
            .map(|((section, role), effect)| SectionEffectivenessEntry {
                section_name: section.clone(),
                role: role.clone(),
                effect: effect.clone(),
            })
            .collect();
        entries.sort_by(|a, b| {
            a.role
                .cmp(&b.role)
                .then(a.section_name.cmp(&b.section_name))
        });
        SectionEffectivenessSnapshot { entries }
    }

    fn from_snapshot(snapshot: SectionEffectivenessSnapshot) -> Self {
        let effects = snapshot
            .entries
            .into_iter()
            .map(|entry| ((entry.section_name, entry.role), entry.effect))
            .collect();
        Self { effects }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SectionEffectivenessSnapshot {
    entries: Vec<SectionEffectivenessEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SectionEffectivenessEntry {
    section_name: String,
    role: String,
    effect: SectionEffect,
}

// ── The section bandit (S02 L9) ──────────────────────────────────────────

/// Where the section bandit keeps its state: a file of its own, beside
/// serve's display registry ([`DEFAULT_SECTION_EFFECTS_PATH`]).
pub const SECTION_BANDIT_PATH: &str = ".roko/learn/section-bandit.json";
/// The lowest probability of leaving a droppable section out.
pub const SECTION_EXCLUSION_FLOOR: f64 = 0.05;
/// The highest probability of leaving a section out over its first
/// [`SECTION_EARLY_OPPORTUNITIES`] opportunities (decision 4115).
pub const SECTION_EXCLUSION_CAP_EARLY: f64 = 0.2;
/// The highest probability of leaving a section out after them.
pub const SECTION_EXCLUSION_CAP: f64 = 0.5;
/// How many opportunities a section spends under the early cap.
pub const SECTION_EARLY_OPPORTUNITIES: u64 = 100;
/// The posterior draws behind one exclusion probability.
pub const SECTION_MONTE_CARLO_DRAWS: u32 = 64;

/// A Beta posterior over a pass rate, from the uniform prior Beta(1, 1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BetaPosterior {
    /// One plus the verified passes.
    pub alpha: f64,
    /// One plus the failures of the agent's work.
    pub beta: f64,
}

impl Default for BetaPosterior {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

impl BetaPosterior {
    /// Beta(`alpha`, `beta`).
    #[must_use]
    pub const fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    /// Count one labelled outcome.
    pub fn observe(&mut self, passed: bool) {
        if passed {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }
}

/// One droppable section's two arms, and how often it was an opportunity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionArms {
    /// The pass rate of attempts whose prompt included the section.
    #[serde(default)]
    pub included: BetaPosterior,
    /// The pass rate of attempts whose prompt left it out.
    #[serde(default)]
    pub excluded: BetaPosterior,
    /// The decisions recorded for the section, labelled or not.
    #[serde(default)]
    pub opportunities: u64,
}

impl SectionArms {
    /// p_max, the cap on the exclusion probability:
    /// [`SECTION_EXCLUSION_CAP_EARLY`] over the first
    /// [`SECTION_EARLY_OPPORTUNITIES`] opportunities, then
    /// [`SECTION_EXCLUSION_CAP`].
    #[must_use]
    pub const fn exclusion_cap(&self) -> f64 {
        if self.opportunities < SECTION_EARLY_OPPORTUNITIES {
            SECTION_EXCLUSION_CAP_EARLY
        } else {
            SECTION_EXCLUSION_CAP
        }
    }
}

/// Whether one prompt leaves a droppable section out, and the propensity of
/// that choice, to log beside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionDecision {
    /// The section.
    pub section: String,
    /// p_ex, the probability of leaving it out.
    pub p_exclude: f64,
    /// Whether the prompt leaves it out.
    pub excluded: bool,
    /// P(the realised choice): `p_exclude` when excluded, else its
    /// complement.
    pub propensity: f64,
}

/// The section bandit (S02 L9, S02.P1-10). Each droppable section has a
/// Beta posterior for the prompts that include it and one for the prompts
/// that leave it out, and a prompt leaves it out with probability
/// p_ex = clip(P(θ_ex > θ_in), [`SECTION_EXCLUSION_FLOOR`], p_max).
///
/// p_ex is estimated from [`SECTION_MONTE_CARLO_DRAWS`] posterior draws, and
/// the choice is drawn after them, all from one generator seeded by the
/// section's assignment hash ([`assignment_seed`]): the same state and seed
/// always give the same decision, so its logged propensity is exact and
/// replays. Only the realised arm learns, and only from a learning label.
/// Pinned sections are never offered to it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionBandit {
    /// Each droppable section's arms, by section id.
    #[serde(default)]
    pub sections: BTreeMap<String, SectionArms>,
}

impl SectionBandit {
    /// Read the bandit at `path`; a missing file is an empty bandit.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> io::Result<Self> {
        roko_fs::read_json_or_default_strict(path)
    }

    /// Read, change and save the bandit at `path` in one transaction under
    /// its sibling lock, so that writers in other processes lose nothing.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed or written.
    pub fn transaction<R>(path: &Path, change: impl FnOnce(&mut Self) -> R) -> io::Result<R> {
        roko_fs::with_locked_json_transaction::<Self, R, io::Error, _>(path, |bandit| {
            Ok(change(bandit))
        })
    }

    /// p_ex of `section`, from the posterior draws of the generator seeded
    /// with `seed`.
    #[must_use]
    pub fn exclusion_probability(&self, section: &str, seed: u64) -> f64 {
        self.decide(section, seed).p_exclude
    }

    /// Decide whether a prompt leaves `section` out. `seed` is the section's
    /// assignment hash for the unit ([`assignment_seed`]).
    #[must_use]
    pub fn decide(&self, section: &str, seed: u64) -> SectionDecision {
        let arms = self.sections.get(section).cloned().unwrap_or_default();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let excluded_wins = (0..SECTION_MONTE_CARLO_DRAWS)
            .filter(|_| {
                let included = sample_beta(arms.included.alpha, arms.included.beta, &mut rng);
                let excluded = sample_beta(arms.excluded.alpha, arms.excluded.beta, &mut rng);
                excluded > included
            })
            .count();
        let estimate = excluded_wins as f64 / f64::from(SECTION_MONTE_CARLO_DRAWS);
        let p_exclude = estimate.clamp(SECTION_EXCLUSION_FLOOR, arms.exclusion_cap());
        let excluded = rng.gen::<f64>() < p_exclude;
        let p_include = 1.0 - p_exclude;
        let propensity = if excluded { p_exclude } else { p_include };
        SectionDecision {
            section: section.to_string(),
            p_exclude,
            excluded,
            propensity,
        }
    }

    /// Record a prompt that included `section` or, when `excluded`, left it
    /// out: one more opportunity and, with a learning label (S01 §4.1: 1 for
    /// a verified pass, 0 for a failure of the agent's work), one more
    /// outcome on the realised arm alone. A null label teaches nothing.
    pub fn record(&mut self, section: &str, excluded: bool, learning_label: Option<u8>) {
        let arms = self.sections.entry(section.to_string()).or_default();
        arms.opportunities = arms.opportunities.saturating_add(1);
        if let Some(label) = learning_label {
            let arm = if excluded {
                &mut arms.excluded
            } else {
                &mut arms.included
            };
            arm.observe(label == 1);
        }
    }
}

/// The seed of a section decision: the top 53 bits of the assignment hash
/// behind `assignment` (S01 §4.6), recovered from its logged `u`, so the
/// decision replays from its decision row.
#[must_use]
pub fn assignment_seed(assignment: &Assignment) -> u64 {
    (assignment.u * 9_007_199_254_740_992.0) as u64
}

#[cfg(test)]
mod tests {
    use super::{
        BetaPosterior, DEFAULT_SECTION_EFFECTS_PATH, PriorityChange, SECTION_BANDIT_PATH,
        SECTION_EXCLUSION_CAP, SECTION_EXCLUSION_CAP_EARLY, SECTION_EXCLUSION_FLOOR, SectionArms,
        SectionBandit, SectionEffect, SectionEffectivenessRegistry, assignment_seed,
    };
    use crate::telemetry::{Assignment, AssignmentUnit, AttemptKey, LayerSpec, assign};

    #[test]
    fn section_effectiveness_lift_and_priority_change_follow_thresholds() {
        let mut effect = SectionEffect::new("workspace_map");
        for _ in 0..24 {
            effect.record(true, true);
        }
        for _ in 0..6 {
            effect.record(true, false);
        }
        for _ in 0..2 {
            effect.record(false, true);
        }
        for _ in 0..8 {
            effect.record(false, false);
        }

        assert!((effect.lift() - 0.6).abs() < 1e-12);
        assert_eq!(effect.recommend_priority_change(), PriorityChange::Increase);
    }

    #[test]
    fn section_effectiveness_returns_insufficient_data_until_thresholds_are_met() {
        let mut effect = SectionEffect::new("plan_brief");
        for _ in 0..19 {
            effect.record(true, true);
        }
        for _ in 0..5 {
            effect.record(false, false);
        }

        assert_eq!(
            effect.recommend_priority_change(),
            PriorityChange::InsufficientData
        );
    }

    #[test]
    fn section_effectiveness_lift_weights_are_clamped_and_aggregated() {
        let mut effect = SectionEffect::new("workspace_map");
        for _ in 0..30 {
            effect.record(true, true);
        }
        for _ in 0..2 {
            effect.record(false, false);
        }

        let weights = effect.lift_weights();
        assert_eq!(weights.len(), 1);
        assert!(weights["workspace_map"] >= 1.0);
        assert!(weights["workspace_map"] <= 1.5);
    }

    #[test]
    fn section_effectiveness_registry_identifies_positive_lift_after_50_events() {
        let mut registry = SectionEffectivenessRegistry::new();

        for _ in 0..30 {
            registry.record_outcome("workspace_map", "Implementer", true, true);
        }
        for _ in 0..10 {
            registry.record_outcome("workspace_map", "Implementer", true, false);
        }
        for _ in 0..5 {
            registry.record_outcome("workspace_map", "Implementer", false, true);
        }
        for _ in 0..5 {
            registry.record_outcome("workspace_map", "Implementer", false, false);
        }

        for _ in 0..20 {
            registry.record_outcome("plan_brief", "Implementer", true, false);
        }
        for _ in 0..5 {
            registry.record_outcome("plan_brief", "Implementer", false, true);
        }
        for _ in 0..5 {
            registry.record_outcome("plan_brief", "Implementer", false, false);
        }

        let positive = registry.positive_lift_sections("Implementer");
        assert_eq!(positive.len(), 1);
        assert_eq!(positive[0].section_name, "workspace_map");
        assert!(positive[0].lift() > 0.0);
        assert_eq!(
            registry.recommend_priority_change("workspace_map", "Implementer"),
            PriorityChange::Increase
        );
        assert_eq!(
            registry.recommend_priority_change("plan_brief", "Implementer"),
            PriorityChange::Decrease
        );
    }

    #[test]
    fn section_effectiveness_registry_persists_and_loads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(DEFAULT_SECTION_EFFECTS_PATH);

        let mut registry = SectionEffectivenessRegistry::new();
        registry.record_outcome("workspace_map", "Reviewer", true, true);
        registry.record_outcome("workspace_map", "Reviewer", false, false);
        registry.save(&path).expect("save");

        let loaded = SectionEffectivenessRegistry::load_or_new(&path);
        let effect = loaded
            .get("workspace_map", "Reviewer")
            .expect("persisted effect");

        assert_eq!(effect.included_trials, 1);
        assert_eq!(effect.included_passes, 1);
        assert_eq!(effect.excluded_trials, 1);
        assert_eq!(effect.excluded_passes, 0);
    }

    /// S02 L9: a new section is left out at most at the early cap, and each
    /// decision carries the propensity of its choice and replays from its
    /// seed; only the realised arm learns, and only from a label; a section
    /// that helps stays at the floor, and one that hurts reaches the cap,
    /// which rises after 100 opportunities; the state persists under its
    /// lock.
    #[test]
    fn section_bandit_excludes_with_logged_propensity_and_updates_posterior() {
        const DECISIONS: u64 = 2_000;
        let near = |p: f64, q: f64| (p - q).abs() < 1e-12;
        let bandit = SectionBandit::default();
        let (mut excluded, mut p_sum) = (0_u32, 0.0);
        for seed in 0..DECISIONS {
            let decision = bandit.decide("workspace_map", seed);
            let p = decision.p_exclude;
            assert!(p <= SECTION_EXCLUSION_CAP_EARLY, "{decision:?}");
            let expected = if decision.excluded { p } else { 1.0 - p };
            assert!(near(decision.propensity, expected), "{decision:?}");
            let replay = bandit.decide("workspace_map", seed);
            assert_eq!(replay, decision, "it replays");
            excluded += u32::from(decision.excluded);
            p_sum += p;
        }
        // Both arms start at Beta(1, 1), so P(θ_ex > θ_in) is 0.5 and the cap
        // binds.
        let mean = p_sum / DECISIONS as f64;
        assert!(mean > 0.19, "mean p_ex {mean}");
        let share = f64::from(excluded) / DECISIONS as f64;
        assert!((0.16..0.24).contains(&share), "excluded share {share}");

        // The seed is read from the logged assignment, so a decision replays
        // from its row.
        let layer = LayerSpec {
            run_seed: 7,
            layer: "sections".to_string(),
            epoch: "2026-10-03".to_string(),
            unit: AssignmentUnit::Chain,
            h: 0.2,
            g: 0.03,
        };
        let drawn = assign(&layer, &AttemptKey::new("gr-sec", "plan", "T1", 1));
        let row = serde_json::to_value(&drawn).expect("log the assignment");
        let logged: Assignment = serde_json::from_value(row).expect("read the row");
        assert_eq!(assignment_seed(&logged), assignment_seed(&drawn));

        // Only the realised arm learns, and a null label teaches nothing.
        let mut bandit = SectionBandit::default();
        bandit.record("workspace_map", true, Some(1));
        bandit.record("workspace_map", false, Some(0));
        bandit.record("workspace_map", false, None);
        let arms = &bandit.sections["workspace_map"];
        assert_eq!(arms.excluded, BetaPosterior::new(2.0, 1.0));
        assert_eq!(arms.included, BetaPosterior::new(1.0, 2.0));
        assert_eq!(arms.opportunities, 3);

        // A section that helps stays at the floor. One that hurts reaches the
        // early cap, and the later cap after its first 100 opportunities.
        let helps = SectionArms {
            included: BetaPosterior::new(40.0, 1.0),
            excluded: BetaPosterior::new(1.0, 40.0),
            opportunities: 99,
        };
        let hurts = SectionArms {
            included: helps.excluded,
            excluded: helps.included,
            ..helps.clone()
        };
        let mut bandit = SectionBandit::default();
        bandit.sections.insert("helps".to_string(), helps);
        bandit.sections.insert("hurts".to_string(), hurts);
        let floor = bandit.exclusion_probability("helps", 11);
        assert!(near(floor, SECTION_EXCLUSION_FLOOR), "{floor}");
        let early = bandit.exclusion_probability("hurts", 11);
        assert!(near(early, SECTION_EXCLUSION_CAP_EARLY), "{early}");
        bandit.record("hurts", false, None);
        let later = bandit.exclusion_probability("hurts", 11);
        assert!(near(later, SECTION_EXCLUSION_CAP), "{later}");

        // The state is read and written under the file's sibling lock.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SECTION_BANDIT_PATH);
        let snapshot = bandit.clone();
        SectionBandit::transaction(&path, |saved| *saved = snapshot).expect("save the bandit");
        SectionBandit::transaction(&path, |saved| saved.record("helps", true, Some(0)))
            .expect("record under the lock");
        let saved = SectionBandit::load(&path).expect("load the bandit");
        let helps = &saved.sections["helps"];
        assert_eq!(helps.excluded, BetaPosterior::new(1.0, 41.0));
        assert_eq!(helps.opportunities, 100);
        assert_eq!(saved.sections["hurts"], bandit.sections["hurts"]);
    }
}
