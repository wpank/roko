//! What a Graph task's verify run teaches the persisted gate learning
//! (find-4b4344), beyond the per-rung pass-rate EMAs in
//! `.roko/learn/gate-thresholds.json`:
//!
//! - P1-10: rungs with no observations yet start from the priors of the
//!   task's `ThresholdProfile`.
//! - P1-08: a test rung's EMA tightens by how far the CodingOracle's forecast
//!   missed ([`GateThresholds::observe_verify_steps`]).
//! - P1-11: a regression is a verify step that passed on an earlier attempt
//!   and fails now, told apart per step rather than per rung
//!   ([`super::step_ratchet`], gap-6dba88).
//! - P1-12: the temperament skip advisory is logged and never acted on.
//!   Skipping an authored verify step would let an unverified change pass,
//!   so every step runs.
//! - P2-22: each verify verdict is recorded in the gate pipeline's metrics.
//!
//! None of it changes which verify steps run or what their verdicts mean.

use std::sync::{Mutex, PoisonError};

use roko_core::{TaskDomain, Temperament};
use roko_gate::adaptive_threshold::ThresholdProfile;

use super::*;

/// This process's lock on the gate learning files. Parallel tasks of a run
/// each load, update and save them, and without it one task's save could drop
/// another's update.
static GATE_LEARNING_FILES: Mutex<()> = Mutex::new(());

/// The `ThresholdProfile` whose priors a task's unobserved rungs start from
/// (P1-10): `research` for a research domain or role, `security` for a
/// security role, `coding` otherwise. The roles are the ones Runner-v2 mapped.
pub(super) fn threshold_profile(task: &TaskDef) -> ThresholdProfile {
    let name = if matches!(task.domain, Some(TaskDomain::Research)) {
        "research"
    } else {
        match task.role.as_deref().unwrap_or("implementer") {
            "researcher" | "strategist" | "pre-planner" => "research",
            "security-reviewer" | "security" => "security",
            _ => "coding",
        }
    };
    ThresholdProfile::by_name(name)
}

/// Record a verify step's verdict in the metrics the gate pipeline emits
/// (P2-22): `roko_gate_verdicts_total` and `roko_gate_duration_seconds`, as
/// the same tracing metric fields `run_gate_once` uses.
pub(super) fn record_gate_verdict_metrics(phase: &str, verdict: &roko_core::Verdict) {
    let rung = rung_for_gate_name(phase).map(|rung| rung.as_index());
    tracing::info!(
        monotonic_counter.roko_gate_verdicts_total = 1_u64,
        result = if verdict.passed { "pass" } else { "fail" },
        rung,
        "gate verdict recorded"
    );
    tracing::info!(
        histogram.roko_gate_duration_seconds = verdict.duration_ms as f64 / 1000.0,
        rung,
        "gate duration recorded"
    );
}

/// What one verify run taught the gate learning.
#[derive(Debug, Default, PartialEq)]
pub(super) struct GateLearning {
    /// `(rung, residual)` of each oracle residual observed (P1-08).
    pub(super) residuals: Vec<(u32, f64)>,
    /// `(rung, passed)` of each step the temperament advisory would have
    /// skipped (P1-12). Every one of them ran.
    pub(super) would_skip: Vec<(u32, bool)>,
}

/// Feed one verify run of a Graph task, `(phase, passed)` per step that ran,
/// into the gate learning.
///
/// `profile` seeds the rungs of `thresholds` that have no observations yet.
/// Each step whose phase names a canonical rung then updates that rung's EMA,
/// and a test step also the oracle residual of `test_pass_forecast`, which
/// was taken before the steps ran. Skip advice is read from the thresholds as
/// they were before this run: a step `thresholds` would let `temperament`
/// skip.
pub(super) fn update_graph_gate_thresholds(
    thresholds: &mut GateThresholds,
    profile: &ThresholdProfile,
    temperament: Temperament,
    step_outcomes: &[(String, bool)],
    test_pass_forecast: Option<(f64, f64)>,
) -> GateLearning {
    thresholds.apply_profile(profile);
    let mut learning = GateLearning::default();
    for (phase, passed) in step_outcomes {
        let Some(rung) = rung_for_gate_name(phase).map(|rung| rung.as_index()) else {
            continue;
        };
        if thresholds.should_skip_rung_for_temperament(rung, temperament) {
            learning.would_skip.push((rung, *passed));
        }
    }
    learning.residuals = thresholds.observe_verify_steps(step_outcomes, test_pass_forecast);
    learning
}

impl GraphTaskDispatcher {
    /// Settle what an attempt's verify run teaches the persisted gate
    /// learning: load `gate-thresholds.json`, update it
    /// ([`update_graph_gate_thresholds`]), save it, and tell the dashboard.
    /// A file that fails to load or save is logged and left to the next
    /// task's update.
    pub(super) fn settle_gate_learning(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        step_outcomes: &[(String, bool)],
        test_pass_forecast: Option<(f64, f64)>,
    ) {
        let Some(thresholds_path) = &self.feedback.gate_thresholds_path else {
            return;
        };
        let profile = threshold_profile(task);
        let temperament = self
            .config
            .agent
            .temperament_for_role(task.role.as_deref().unwrap_or("implementer"));

        let files = GATE_LEARNING_FILES
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut thresholds =
            GateThresholds::load_or_default(thresholds_path).unwrap_or_else(|err| {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    error = %err,
                    "P2-LRN-6 Loop 1: gate threshold load failed (non-fatal)"
                );
                GateThresholds::default()
            });
        let learning = update_graph_gate_thresholds(
            &mut thresholds,
            &profile,
            temperament,
            step_outcomes,
            test_pass_forecast,
        );
        let saved = thresholds.save(thresholds_path);
        drop(files);

        if !learning.residuals.is_empty() {
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                residuals = ?learning.residuals,
                forecast = ?test_pass_forecast,
                "P1-08: oracle residual fed to adaptive gate thresholds"
            );
        }
        if !learning.would_skip.is_empty() {
            let failed = learning
                .would_skip
                .iter()
                .filter(|(_, passed)| !passed)
                .count();
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                temperament = temperament.label(),
                rungs = ?learning.would_skip,
                would_skip = learning.would_skip.len(),
                failed_of_those = failed,
                "P1-12: the skip advisory would skip these rungs; every verify step ran \
                 (advisory only)"
            );
        }
        match saved {
            Ok(()) => {
                // The learning tab shows the updated per-rung EMAs at once.
                if let Some(tui) = &self.tui_bridge {
                    if let Ok(json) = serde_json::to_string(&thresholds) {
                        tui.gate_thresholds_updated(&json);
                    }
                }
                tracing::debug!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    steps = step_outcomes.len(),
                    profile = %profile.name,
                    "P2-LRN-6 Loop 1: gate thresholds updated"
                );
            }
            Err(err) => {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    error = %err,
                    "P2-LRN-6 Loop 1: gate threshold save failed (non-fatal)"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn steps(outcomes: &[(&str, bool)]) -> Vec<(String, bool)> {
        outcomes
            .iter()
            .map(|(phase, passed)| ((*phase).to_string(), *passed))
            .collect()
    }

    /// Two attempts of one task through the file a Graph verify run writes:
    /// the first passes compile and clippy and fails its tests, the second
    /// fails compile. The thresholds get the profile's priors, each step's
    /// observation and the residual.
    #[test]
    fn graph_verify_feeds_gate_thresholds() {
        let dir = tempdir().expect("tempdir");
        let thresholds_path = dir.path().join("gate-thresholds.json");
        let profile = ThresholdProfile::research();
        let attempt = |outcomes: &[(&str, bool)], forecast| {
            let mut thresholds =
                GateThresholds::load_or_default(&thresholds_path).expect("load thresholds");
            let learning = update_graph_gate_thresholds(
                &mut thresholds,
                &profile,
                Temperament::Balanced,
                &steps(outcomes),
                forecast,
            );
            thresholds.save(&thresholds_path).expect("save thresholds");
            (thresholds, learning)
        };

        let (thresholds, first) = attempt(
            &[("compile", true), ("clippy", true), ("test", false)],
            Some((0.8, 0.5)),
        );
        assert!(first.would_skip.is_empty(), "{first:?}");
        // The forecast said 0.8 and the tests failed.
        assert_eq!(first.residuals.len(), 1, "{first:?}");
        assert_eq!(first.residuals[0].0, 2);
        assert!((first.residuals[0].1 - 0.8).abs() < 1e-9, "{first:?}");
        // An unobserved rung holds the research prior, not the default one.
        let symbol = &thresholds.rungs[&3];
        assert_eq!(symbol.total_count, 0);
        assert!((symbol.ema_pass_rate - 0.40).abs() < 1e-9, "{symbol:?}");
        assert_eq!(thresholds.rungs[&0].total_count, 1);
        assert_eq!(thresholds.rungs[&2].pass_count, 0);

        let (thresholds, second) = attempt(&[("compile", false)], None);
        assert!(second.residuals.is_empty(), "{second:?}");
        assert_eq!(thresholds.rungs[&0].total_count, 2);
        assert_eq!(thresholds.rungs[&0].pass_count, 1);
    }

    /// A residual tightens a test rung that only ever passed.
    #[test]
    fn a_confident_miss_tightens_a_passing_test_rung() {
        let mut thresholds = GateThresholds::default();
        for _ in 0..4 {
            thresholds.observe(2, true);
        }
        let learning = update_graph_gate_thresholds(
            &mut thresholds,
            &ThresholdProfile::coding(),
            Temperament::Balanced,
            &steps(&[("test", true)]),
            Some((0.5, 0.9)),
        );
        assert_eq!(learning.residuals.len(), 1, "{learning:?}");
        assert!(thresholds.rungs[&2].ema_pass_rate < 1.0, "{thresholds:?}");
    }

    /// The skip advisory names a long-passing rung under a balanced
    /// temperament, never under a conservative one, and reports whether the
    /// step it would have skipped failed.
    #[test]
    fn the_skip_advisory_only_reports() {
        let mut thresholds = GateThresholds::default();
        for _ in 0..25 {
            thresholds.observe(0, true);
        }
        let run = steps(&[("compile", false)]);
        let advise = |temperament| {
            update_graph_gate_thresholds(
                &mut thresholds.clone(),
                &ThresholdProfile::coding(),
                temperament,
                &run,
                None,
            )
            .would_skip
        };
        assert_eq!(advise(Temperament::Balanced), vec![(0, false)]);
        assert!(advise(Temperament::Conservative).is_empty());
    }

    /// Research tasks get the research priors, security roles the security
    /// ones, and every other task the coding ones.
    #[test]
    fn the_profile_follows_the_task_domain_and_role() {
        let plan = crate::task_parser::TasksFile::parse_str(
            r#"
[meta]
plan = "profiles"

[[task]]
id = "PLAIN"
title = "no role or domain"

[[task]]
id = "RESEARCH"
title = "research domain"
domain = "research"

[[task]]
id = "RESEARCHER"
title = "researcher role"
role = "researcher"

[[task]]
id = "SECURITY"
title = "security role"
role = "security-reviewer"

[[task]]
id = "CODE"
title = "implementer in the code domain"
role = "implementer"
domain = "code"
"#,
        )
        .expect("fixture plan");
        let profiles: Vec<(String, String)> = plan
            .tasks
            .iter()
            .map(|task| (task.id.clone(), threshold_profile(task).name))
            .collect();
        let expected = [
            ("PLAIN", "coding"),
            ("RESEARCH", "research"),
            ("RESEARCHER", "research"),
            ("SECURITY", "security"),
            ("CODE", "coding"),
        ]
        .map(|(id, name)| (id.to_string(), name.to_string()));
        assert_eq!(profiles, expected);
    }
}
