//! Learning subsystem configuration.

use serde::{Deserialize, Serialize};

use super::agent::default_true;

// ---- [learning] ----------------------------------------------------------

/// Default number of gate observations between incremental threshold flushes.
pub const DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL: u64 = 10;

// ---- [learning.dreams] ---------------------------------------------------

/// Configuration for the dreams consolidation subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DreamsConfig {
    /// Automatically trigger dream consolidation after a plan completes.
    ///
    /// The trigger fires only when this (default `true`) and
    /// [`LearningConfig::dream_on_completion`] (default `false`) are both
    /// `true`, so by default no dream runs. Setting this to `false` disables
    /// the automatic trigger; dreams can still be run manually via
    /// `roko knowledge dream run`.
    #[serde(default = "default_true")]
    pub trigger_on_plan_complete: bool,
    /// Maximum number of dream consolidations that ACP sessions run at once
    /// in one process. An ACP turn that finds this many running starts none.
    /// The plan-completion trigger keeps its own limit of one.
    ///
    /// Defaults to `1`. Zero is normalized to one at the runtime boundary
    /// (see [`Self::effective_max_concurrent`]).
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// Let ACP sessions trigger dream consolidation once
    /// [`Self::acp_episode_threshold`] episodes accumulate since the last
    /// dream report.
    ///
    /// Defaults to `false`: each automatic dream costs a model call, so ACP
    /// sessions start none unless this is `true`. It is independent of
    /// [`Self::trigger_on_plan_complete`] and
    /// [`LearningConfig::dream_on_completion`].
    #[serde(default)]
    pub trigger_on_acp_episodes: bool,
    /// Episodes recorded since the last dream report before an ACP session
    /// triggers a dream, when [`Self::trigger_on_acp_episodes`] is `true`.
    ///
    /// Defaults to `10`. Zero is normalized to one at the runtime boundary
    /// (see [`Self::effective_acp_episode_threshold`]), so a dream always
    /// needs at least one new episode.
    #[serde(default = "default_acp_episode_threshold")]
    pub acp_episode_threshold: usize,
}

fn default_max_concurrent() -> usize {
    1
}

const fn default_acp_episode_threshold() -> usize {
    10
}

impl Default for DreamsConfig {
    fn default() -> Self {
        Self {
            trigger_on_plan_complete: true,
            max_concurrent: default_max_concurrent(),
            trigger_on_acp_episodes: false,
            acp_episode_threshold: default_acp_episode_threshold(),
        }
    }
}

impl DreamsConfig {
    /// Return the runtime-safe ACP episode threshold.
    #[must_use]
    pub const fn effective_acp_episode_threshold(&self) -> usize {
        if self.acp_episode_threshold == 0 {
            1
        } else {
            self.acp_episode_threshold
        }
    }

    /// Return the runtime-safe limit on concurrent ACP dream consolidations.
    #[must_use]
    pub const fn effective_max_concurrent(&self) -> usize {
        if self.max_concurrent == 0 {
            1
        } else {
            self.max_concurrent
        }
    }
}

/// Learning subsystem configuration.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningConfig {
    /// Auto-refresh playbook rules after successful tasks.
    #[serde(default = "default_true")]
    pub auto_playbook_refresh: bool,
    /// Inject file difficulty profiles into agent context.
    #[serde(default = "default_true")]
    pub knowledge_file_intel: bool,
    /// Inject knowledge store warnings into agent context.
    #[serde(default = "default_true")]
    pub knowledge_warnings: bool,
    /// Enable cross-task wave context propagation.
    #[serde(default = "default_true")]
    pub knowledge_wave_context: bool,
    /// Enable error signature pattern matching.
    #[serde(default = "default_true")]
    pub knowledge_error_patterns: bool,
    /// Min occurrences before promoting learned rules.
    #[serde(default = "default_learning_min_occ")]
    pub learning_min_occurrences: usize,
    /// Max file-intel entries to inject per task.
    #[serde(default = "default_file_intel_max")]
    pub file_intel_max_entries: usize,
    /// Max warning entries to inject per task.
    #[serde(default = "default_warning_max")]
    pub warning_max_entries: usize,
    /// Run dream consolidation after a plan completes.
    ///
    /// Defaults to `false`: each automatic dream costs a model call, so dreams
    /// run on demand, through `roko knowledge dream run`. An explicit `true`
    /// opts in; [`DreamsConfig::trigger_on_plan_complete`] must also be `true`
    /// for the trigger to fire.
    #[serde(default)]
    pub dream_on_completion: bool,
    /// Dreams consolidation subsystem configuration.
    #[serde(default)]
    pub dreams: DreamsConfig,
    /// Enable the lookahead router for cost-saving tier downgrades.
    /// When true, the cascade router selection is post-filtered through
    /// `LookaheadRouter::route_with_lookahead()` which may downgrade to a
    /// cheaper model when calibration data indicates sufficient success
    /// probability.
    #[serde(default)]
    pub use_lookahead_router: bool,
    /// Success probability threshold for the lookahead router to accept a
    /// cheaper tier (0.0--1.0). Only used when `use_lookahead_router` is true.
    /// Defaults to 0.7.
    #[serde(default = "default_lookahead_threshold")]
    pub lookahead_threshold: f64,
    /// Serve plan tasks that author no verify steps from matching T0 reflex
    /// rules (`.roko/learn/reflexes.jsonl`) instead of dispatching a model.
    ///
    /// Off by default: a reflex skips the provider and every gate, so its
    /// output is unverified and its rule earns no gate pass.
    #[serde(default)]
    pub t0_reflexes: bool,
    /// Write an `hdc_fingerprint` on every persisted episode: one 64-bit hash
    /// of the prompt and outcome expanded into an HDC vector.
    ///
    /// Off by default (9226): it identifies exact inputs, measures no
    /// semantic similarity and only the TUI context view reads it, yet it was
    /// most of each `.roko/episodes.jsonl` row. Rows already written keep it.
    #[serde(default)]
    pub episode_hdc_fingerprint: bool,
    /// Dampening factor for manual model override learning (UX34).
    ///
    /// When a user manually overrides the model via `--model` /
    /// `--force-model` / `--force-backend`, the outcome reward is
    /// multiplied by this factor before being fed into the cascade
    /// router's multi-objective bandit. This prevents a single override
    /// from dominating the learned policy. Range: 0.0--1.0.
    /// Defaults to 0.5 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_learning_dampening: Option<f64>,
    /// Number of gate observations between incremental writes of adaptive
    /// thresholds to `.roko/learn/gate-thresholds.json`.
    ///
    /// Values below one are normalized to one at the runtime boundary so an
    /// invalid zero never disables durability or creates an always-due
    /// comparison. Defaults to 10 for compatibility with the original
    /// hardcoded cadence.
    #[serde(default = "default_gate_threshold_flush_interval")]
    pub gate_threshold_flush_interval: u64,
    /// P3-26: Knowledge tier progression thresholds.
    #[serde(default)]
    pub knowledge: KnowledgeProgressionConfig,
    /// Hold learned state fixed (decision 2218): a frozen run reads learned
    /// state as usual and never writes it, so it starts and ends with the
    /// same state. `roko plan run --frozen-learning` freezes one run.
    ///
    /// Learned state is whatever a later prompt, route, retry budget, gate
    /// threshold or reflex reads: in `.roko/learn/`, the cascade router and
    /// its journal, `gate-thresholds.json`, `playbooks/`, `experiments.json`,
    /// `error-patterns.json`, `post-gate-reflections.json`, the hindsight
    /// adjustments, `reflexes.jsonl`, `holdout-state.json` and the daimon
    /// state; the knowledge store in `.roko/neuro/`, with its access counts;
    /// and `.roko/episodes.jsonl`. A frozen run assigns no prompt
    /// experiments. Telemetry stays on: the run's `.roko/runs/<run_id>/`
    /// files, `learn/costs.jsonl`, `learn/efficiency.jsonl` and
    /// `learn/run-metrics.jsonl`. A frozen run's manifest records
    /// `ablation_flags = ["learning_frozen"]`.
    #[serde(default)]
    pub frozen: bool,
    /// `[learning.audit]`: the M2 loop auditor (S03 §5).
    #[serde(default)]
    pub audit: LearningAuditConfig,
}

// ---- [learning.audit] ----------------------------------------------------

/// The highest holdout rate a loop runs at (S03 §4.3).
pub const MAX_AUDIT_HOLDOUT: f64 = 0.5;

/// The highest all-learning-off rate g (S03 §4.3).
pub const MAX_AUDIT_GLOBAL_OFF: f64 = 0.1;

/// `[learning.audit]` in `roko.toml` (S03 §5): the M2 loop auditor's switch,
/// level, holdout schedule, thresholds and budgets, with S03 §4.10's
/// defaults. Every key is optional; invariant 13 checks the bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LearningAuditConfig {
    /// Whether a demotion switches a loop to its default policy π⁰. Off for
    /// the first 14 days and until C2 and C4 pass (D10), so the auditor only
    /// flags. A loop enforces only when its registry entry allows it too.
    pub enforce: bool,
    /// α, the family-wise rate of false harm-demotions, split over the
    /// enforced loops.
    pub alpha: f64,
    /// The holdout rate h by audit state, and its floor.
    pub h: AuditHoldoutConfig,
    /// g, the share of chains that run every loop on π⁰, at most 0.1.
    pub global_off: f64,
    /// Whether the placebo loop runs.
    pub placebo: bool,
    /// What an assignment epoch is: the UTC day in production.
    pub epoch: AuditEpoch,
    /// N_ε: learned-arm opportunities before ε is judged.
    pub n_eps: u64,
    /// ε_min: the exposure below which a loop is dormant.
    pub eps_min: f64,
    /// N_ι: opportunities before ι is judged.
    pub n_iota: u64,
    /// ι_min, net of the A/A floor.
    pub iota_min: f64,
    /// N_β: opportunities before β is judged.
    pub n_beta: u64,
    /// N_null: opportunities before a narrow null is declared.
    pub n_null: u64,
    /// δ_min: the pass-rate half-width a null's sequence must fit inside.
    pub delta_min: f64,
    /// δ_ni: the non-inferiority margin.
    pub delta_ni: f64,
    /// κ_max, the clip on CUPED's coefficient.
    pub kappa_max: f64,
    /// r_pair: the share of opportunities replayed in pairs for ι_beh.
    pub pair_rate: f64,
    /// The weekly cap on paired-replay spend, in USD.
    pub pair_weekly_usd: f64,
    /// The dwell between two transitions of a loop.
    pub dwell: AuditDwellConfig,
    /// T_re: days in `flagged` or `demoted` before re-probation.
    pub reprobation_days: u32,
    /// N∧: a stratum's opportunities before SPIBB lets it run learned.
    pub spibb_n: u64,
    /// The daily audit spend, in USD; past it the auditor only observes.
    pub audit_budget_usd_per_day: f64,
    /// Loops audited but never demoted, by loop id (`L-know`) or name:
    /// the immune system, quarantine, capability checks and the M4 sensor.
    pub exempt: Vec<String>,
}

/// `[learning.audit.h]`: the holdout rate by audit state (S03 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuditHoldoutConfig {
    /// h on probation.
    pub probation: f64,
    /// h once live.
    pub live: f64,
    /// The floor h_min: never 0, so that every loop stays identifiable.
    pub min: f64,
    /// h while flagged or demoted.
    pub suspect: f64,
}

/// `[learning.audit.dwell]`: the least time and the fewest opportunities
/// between two transitions of a loop (S03 §4.6, guard 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuditDwellConfig {
    /// Hours.
    pub hours: u32,
    /// Opportunities.
    pub opps: u64,
}

/// What an assignment epoch is (S03 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEpoch {
    /// The UTC day, in production.
    #[default]
    UtcDay,
    /// The run id, as a bench campaign uses.
    Run,
}

impl Default for LearningAuditConfig {
    fn default() -> Self {
        Self {
            enforce: false,
            alpha: 0.05,
            h: AuditHoldoutConfig::default(),
            global_off: 0.03,
            placebo: true,
            epoch: AuditEpoch::UtcDay,
            n_eps: 30,
            eps_min: 0.5,
            n_iota: 50,
            iota_min: 0.05,
            n_beta: 200,
            n_null: 800,
            delta_min: 0.03,
            delta_ni: 0.03,
            kappa_max: 2.0,
            pair_rate: 0.03,
            pair_weekly_usd: 5.0,
            dwell: AuditDwellConfig::default(),
            reprobation_days: 14,
            spibb_n: 20,
            audit_budget_usd_per_day: 5.0,
            exempt: ["immune", "quarantine", "capability", "m4_sensor"]
                .map(String::from)
                .to_vec(),
        }
    }
}

impl Default for AuditHoldoutConfig {
    fn default() -> Self {
        Self {
            probation: 0.20,
            live: 0.05,
            min: 0.02,
            suspect: 0.50,
        }
    }
}

impl Default for AuditDwellConfig {
    fn default() -> Self {
        Self {
            hours: 24,
            opps: 100,
        }
    }
}

impl LearningAuditConfig {
    /// Every problem with the section, as (key, problem): the holdout rates
    /// lie between a positive floor and 0.5, g is at most 0.1, shares lie in
    /// [0, 1], counts are positive and budgets are not negative.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        let floor = self.h.min;
        if !(floor > 0.0 && floor <= MAX_AUDIT_HOLDOUT) {
            problems.push((
                "h.min",
                format!(
                    "h.min ({floor}) must lie in (0, {MAX_AUDIT_HOLDOUT}]: a loop with no \
                     holdout cannot be audited"
                ),
            ));
        }
        let holdouts = [
            ("h.probation", self.h.probation),
            ("h.live", self.h.live),
            ("h.suspect", self.h.suspect),
        ];
        for (key, value) in holdouts {
            if !(floor..=MAX_AUDIT_HOLDOUT).contains(&value) {
                let bound = format!("[h.min ({floor}), {MAX_AUDIT_HOLDOUT}]");
                problems.push((key, format!("{key} ({value}) must lie in {bound}")));
            }
        }
        if !(0.0..=MAX_AUDIT_GLOBAL_OFF).contains(&self.global_off) {
            let g = self.global_off;
            problems.push((
                "global_off",
                format!("global_off ({g}) must lie in [0, {MAX_AUDIT_GLOBAL_OFF}]"),
            ));
        }
        if !(self.alpha > 0.0 && self.alpha < 1.0) {
            let alpha = self.alpha;
            problems.push(("alpha", format!("alpha ({alpha}) must lie in (0, 1)")));
        }
        let shares = [
            ("eps_min", self.eps_min),
            ("iota_min", self.iota_min),
            ("delta_min", self.delta_min),
            ("delta_ni", self.delta_ni),
            ("pair_rate", self.pair_rate),
        ];
        for (key, value) in shares {
            if !(0.0..=1.0).contains(&value) {
                problems.push((key, format!("{key} ({value}) must be between 0 and 1")));
            }
        }
        let amounts = [
            ("kappa_max", self.kappa_max),
            ("pair_weekly_usd", self.pair_weekly_usd),
            ("audit_budget_usd_per_day", self.audit_budget_usd_per_day),
        ];
        for (key, value) in amounts {
            if value.is_nan() || value < 0.0 {
                problems.push((key, format!("{key} ({value}) cannot be negative")));
            }
        }
        let counts = [
            ("n_eps", self.n_eps),
            ("n_iota", self.n_iota),
            ("n_beta", self.n_beta),
            ("n_null", self.n_null),
            ("spibb_n", self.spibb_n),
            ("reprobation_days", u64::from(self.reprobation_days)),
        ];
        for (key, value) in counts {
            if value == 0 {
                problems.push((key, format!("{key} must be at least 1")));
            }
        }
        if self.exempt.iter().any(|name| name.trim().is_empty()) {
            problems.push(("exempt", "exempt lists a blank name".to_string()));
        }
        problems
    }

    /// Whether `loop_id` is exempt by this section: the list names it, or
    /// its name after `L-` (`m4` for `L-M4`), ignoring case.
    #[must_use]
    pub fn exempts(&self, loop_id: &str) -> bool {
        let short = loop_id.strip_prefix("L-").unwrap_or(loop_id);
        self.exempt
            .iter()
            .any(|name| name.eq_ignore_ascii_case(loop_id) || name.eq_ignore_ascii_case(short))
    }
}

/// P3-26: Configurable thresholds for knowledge tier promotion/demotion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeProgressionConfig {
    /// Confirmations needed for Transient -> Working promotion.
    #[serde(default = "default_transient_confirmations")]
    pub transient_confirmations: u32,
    /// Distinct contexts needed for Working -> Consolidated promotion.
    #[serde(default = "default_working_contexts")]
    pub working_contexts: u32,
    /// Minimum age in days for Consolidated -> Persistent promotion.
    #[serde(default = "default_consolidated_age_days")]
    pub consolidated_age_days: u32,
    /// Minimum balance for demotion consideration.
    #[serde(default = "default_demotion_balance")]
    pub demotion_balance_threshold: f64,
}

const fn default_transient_confirmations() -> u32 {
    2
}
const fn default_working_contexts() -> u32 {
    3
}
const fn default_consolidated_age_days() -> u32 {
    14
}
fn default_demotion_balance() -> f64 {
    0.1
}

impl Default for KnowledgeProgressionConfig {
    fn default() -> Self {
        Self {
            transient_confirmations: default_transient_confirmations(),
            working_contexts: default_working_contexts(),
            consolidated_age_days: default_consolidated_age_days(),
            demotion_balance_threshold: default_demotion_balance(),
        }
    }
}

fn default_lookahead_threshold() -> f64 {
    0.7
}

const fn default_gate_threshold_flush_interval() -> u64 {
    DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
}

const fn default_learning_min_occ() -> usize {
    2
}

const fn default_file_intel_max() -> usize {
    15
}

const fn default_warning_max() -> usize {
    5
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            auto_playbook_refresh: true,
            knowledge_file_intel: true,
            knowledge_warnings: true,
            knowledge_wave_context: true,
            knowledge_error_patterns: true,
            learning_min_occurrences: default_learning_min_occ(),
            file_intel_max_entries: default_file_intel_max(),
            warning_max_entries: default_warning_max(),
            dream_on_completion: false,
            dreams: DreamsConfig::default(),
            use_lookahead_router: false,
            lookahead_threshold: default_lookahead_threshold(),
            t0_reflexes: false,
            episode_hdc_fingerprint: false,
            override_learning_dampening: None,
            gate_threshold_flush_interval: default_gate_threshold_flush_interval(),
            knowledge: KnowledgeProgressionConfig::default(),
            frozen: false,
            audit: LearningAuditConfig::default(),
        }
    }
}

impl LearningConfig {
    /// Return the runtime-safe gate-threshold flush cadence.
    #[must_use]
    pub const fn effective_gate_threshold_flush_interval(&self) -> u64 {
        if self.gate_threshold_flush_interval == 0 {
            1
        } else {
            self.gate_threshold_flush_interval
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S03 §5: `[learning.audit]`'s defaults, a section that loads with
    /// every key in the schema tree, and invariant 13 on broken bounds.
    #[test]
    fn learning_audit_config_defaults_match_s03() {
        use crate::config::schema::RokoConfig;
        use crate::config::validation::{InvariantSeverity, validate_invariants};

        let audit = LearningAuditConfig::default();
        assert!(!audit.enforce && audit.placebo);
        assert_eq!(audit.alpha, 0.05);
        let h = audit.h;
        assert_eq!(
            (h.probation, h.live, h.min, h.suspect),
            (0.20, 0.05, 0.02, 0.50)
        );
        assert_eq!((audit.global_off, audit.epoch), (0.03, AuditEpoch::UtcDay));
        let counts = (audit.n_eps, audit.n_iota, audit.n_beta, audit.n_null);
        assert_eq!(counts, (30, 50, 200, 800));
        assert_eq!((audit.eps_min, audit.iota_min), (0.5, 0.05));
        let margins = (audit.delta_min, audit.delta_ni, audit.kappa_max);
        assert_eq!(margins, (0.03, 0.03, 2.0));
        assert_eq!((audit.pair_rate, audit.pair_weekly_usd), (0.03, 5.0));
        assert_eq!((audit.dwell.hours, audit.dwell.opps), (24, 100));
        assert_eq!((audit.reprobation_days, audit.spibb_n), (14, 20));
        assert_eq!(audit.audit_budget_usd_per_day, 5.0);
        let exempt = ["immune", "quarantine", "capability", "m4_sensor"];
        assert_eq!(audit.exempt, exempt);
        assert!(audit.problems().is_empty());
        assert!(!audit.exempts("L-M4"));

        let empty = RokoConfig::from_toml("").expect("an empty config");
        assert_eq!(empty.learning.audit, audit);
        let text = toml::to_string(&audit).expect("serialize the defaults");
        let back: LearningAuditConfig = toml::from_str(&text).expect("parse them back");
        assert_eq!(back, audit);

        // A section loads, and the schema tree knows every key it names.
        let section = "[learning.audit]\nenforce = true\nalpha = 0.1\nepoch = \"run\"\n\
                       exempt = [\"L-know\", \"route\"]\n\n[learning.audit.h]\nlive = 0.1\n\n\
                       [learning.audit.dwell]\nhours = 12\n";
        let config = RokoConfig::from_toml(section).expect("the section loads");
        let loaded = &config.learning.audit;
        assert!(loaded.enforce);
        let rates = (loaded.alpha, loaded.h.live, loaded.h.probation);
        assert_eq!(rates, (0.1, 0.1, 0.20));
        assert_eq!((loaded.dwell.hours, loaded.dwell.opps), (12, 100));
        assert_eq!(loaded.epoch, AuditEpoch::Run);
        assert!(loaded.exempts("L-know") && loaded.exempts("L-route"));
        assert!(!loaded.exempts("L-play"));
        let invariants = validate_invariants(&config);
        assert!(invariants.iter().all(|result| result.invariant_id != 13));
        let value: toml::Value = toml::from_str(section).expect("parse the section");
        let unknown = super::super::loader::validate_known_config_paths(&value);
        assert!(unknown.is_empty(), "{unknown:?}");
        assert!(RokoConfig::from_toml("[learning.audit]\nsurprise = 1\n").is_err());

        // Invariant 13: h lies in [h.min, 0.5], h.min is above 0, g is at
        // most 0.1, and counts are positive.
        for (toml_text, key) in [
            ("[learning.audit.h]\nlive = 0.6\n", "learning.audit.h.live"),
            ("[learning.audit.h]\nprobation = 0.01\n", "learning.audit.h.probation"),
            ("[learning.audit.h]\nmin = 0.0\n", "learning.audit.h.min"),
            ("[learning.audit]\nglobal_off = 0.2\n", "learning.audit.global_off"),
            ("[learning.audit]\nn_beta = 0\n", "learning.audit.n_beta"),
        ] {
            let config = RokoConfig::from_toml(toml_text).expect("the section parses");
            let rejected = validate_invariants(&config);
            assert!(
                rejected.iter().any(|result| result.invariant_id == 13
                    && result.config_path == key
                    && result.severity == InvariantSeverity::Error),
                "{toml_text}: {rejected:?}"
            );
        }
    }

    #[test]
    fn gate_threshold_flush_interval_preserves_legacy_default() {
        let config: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert_eq!(
            config.gate_threshold_flush_interval,
            DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
        );
        assert_eq!(
            config.effective_gate_threshold_flush_interval(),
            DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
        );
    }

    #[test]
    fn gate_threshold_flush_interval_accepts_custom_value_and_normalizes_zero() {
        let custom: LearningConfig =
            toml::from_str("gate_threshold_flush_interval = 3").expect("parse custom interval");
        assert_eq!(custom.effective_gate_threshold_flush_interval(), 3);

        let zero: LearningConfig =
            toml::from_str("gate_threshold_flush_interval = 0").expect("parse zero interval");
        assert_eq!(zero.effective_gate_threshold_flush_interval(), 1);
    }

    /// Learning runs unless a config asks to freeze it (decision 2218).
    #[test]
    fn frozen_learning_defaults_to_off() {
        assert!(!LearningConfig::default().frozen);
        let omitted: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert!(!omitted.frozen);
        let frozen: LearningConfig = toml::from_str("frozen = true").expect("parse frozen");
        assert!(frozen.frozen);
    }

    #[test]
    fn dream_on_completion_defaults_to_false() {
        assert!(!LearningConfig::default().dream_on_completion);

        let omitted: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert!(!omitted.dream_on_completion);

        // An explicit opt-in is still honoured.
        let opted_in: LearningConfig =
            toml::from_str("dream_on_completion = true").expect("parse explicit opt-in");
        assert!(opted_in.dream_on_completion);
    }

    #[test]
    fn acp_dream_trigger_defaults_to_off() {
        let defaults = DreamsConfig::default();
        assert!(!defaults.trigger_on_acp_episodes);
        assert_eq!(defaults.acp_episode_threshold, 10);

        let omitted: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert_eq!(omitted.dreams, defaults);

        let opted_in: LearningConfig =
            toml::from_str("[dreams]\ntrigger_on_acp_episodes = true\nacp_episode_threshold = 4")
                .expect("parse ACP dream opt-in");
        assert!(opted_in.dreams.trigger_on_acp_episodes);
        assert_eq!(opted_in.dreams.effective_acp_episode_threshold(), 4);

        let zero: LearningConfig = toml::from_str("[dreams]\nacp_episode_threshold = 0")
            .expect("parse zero ACP threshold");
        assert_eq!(zero.dreams.effective_acp_episode_threshold(), 1);
    }
}
