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
/// the same tracing metric fields `run_gate_once` uses, and in `registry`
/// when the run has one (gap-d8c39a). Its series carry the canonical labels:
/// `gate`, the step's rung (`other` for a step that names none), and
/// `verdict`; never a plan or task id.
pub(super) fn record_gate_verdict_metrics(
    registry: Option<&roko_core::obs::metrics::MetricRegistry>,
    phase: &str,
    verdict: &roko_core::Verdict,
) {
    use roko_core::obs::histograms::LLM_LATENCY_BUCKETS;
    use roko_core::obs::metrics::LabelSet;
    use roko_core::obs::schema::{LABEL_GATE, LABEL_VERDICT, ROKO_GATE_VERDICTS_TOTAL_DESCRIPTOR};

    let rung = rung_for_gate_name(phase);
    let rung_index = rung.map(|rung| rung.as_index());
    let result = if verdict.passed { "pass" } else { "fail" };
    let seconds = verdict.duration_ms as f64 / 1000.0;
    tracing::info!(
        monotonic_counter.roko_gate_verdicts_total = 1_u64,
        result,
        rung = rung_index,
        "gate verdict recorded"
    );
    tracing::info!(
        histogram.roko_gate_duration_seconds = seconds,
        rung = rung_index,
        "gate duration recorded"
    );

    let Some(registry) = registry else {
        return;
    };
    let gate = rung.map_or("other", |rung| rung.label());
    let labels = LabelSet::from_pairs(&[(LABEL_GATE, gate), (LABEL_VERDICT, result)]);
    let verdicts = &ROKO_GATE_VERDICTS_TOTAL_DESCRIPTOR;
    registry
        .register_counter(verdicts.name, verdicts.help, labels.clone())
        .inc();
    registry
        .register_histogram(
            GATE_DURATION_SECONDS,
            "Verify step duration in seconds, by gate and verdict",
            labels,
            LLM_LATENCY_BUCKETS.to_vec(),
        )
        .observe(seconds);
}

/// The verify step duration histogram's name.
const GATE_DURATION_SECONDS: &str = "roko_gate_duration_seconds";

/// What one verify run taught the gate learning.
#[derive(Debug, Default, PartialEq)]
pub(super) struct GateLearning {
    /// `(rung, residual)` of each oracle residual observed (P1-08).
    pub(super) residuals: Vec<(u32, f64)>,
    /// `(rung, passed)` of each step the temperament advisory would have
    /// skipped (P1-12). Every one of them ran.
    pub(super) would_skip: Vec<(u32, bool)>,
}

/// What one verify run of a Graph task, `(phase, passed)` per step that ran,
/// gets at once from `thresholds` as they were before it: the steps they
/// would let `temperament` skip (P1-12). The thresholds take the run later
/// ([`VerifyRun::apply`]). Regressions are the step history's
/// ([`super::step_ratchet`]).
fn skip_advice(
    thresholds: &GateThresholds,
    temperament: Temperament,
    step_outcomes: &[(String, bool)],
) -> GateLearning {
    let mut learning = GateLearning::default();
    for (phase, passed) in step_outcomes {
        let Some(rung) = rung_for_gate_name(phase).map(|rung| rung.as_index()) else {
            continue;
        };
        if thresholds.should_skip_rung_for_temperament(rung, temperament) {
            learning.would_skip.push((rung, *passed));
        }
    }
    learning
}

/// A verify run's share of `gate-thresholds.json`, held until it is written
/// ([`GateThresholdWrites`]).
struct VerifyRun {
    /// Seeds the rungs that have no observations yet (P1-10).
    profile: ThresholdProfile,
    /// `(phase, passed)` per step that ran.
    step_outcomes: Vec<(String, bool)>,
    /// The CodingOracle's test pass-rate forecast, taken before the steps
    /// ran.
    test_pass_forecast: Option<(f64, f64)>,
    /// The pass-rate EMAs' smoothing factor, `[gates] ema_alpha`.
    ema_alpha: f64,
}

impl VerifyRun {
    /// The observations the run adds: its steps whose phase names a
    /// canonical rung.
    fn observations(&self) -> u64 {
        self.step_outcomes
            .iter()
            .filter(|(phase, _)| rung_for_gate_name(phase).is_some())
            .count() as u64
    }

    /// Fold the run into `thresholds`: the profile's priors for rungs with no
    /// observations yet, then each step's observation, and for a test step
    /// the oracle residual of the forecast (P1-08), which this returns.
    fn apply(&self, thresholds: &mut GateThresholds) -> Vec<(u32, f64)> {
        thresholds.apply_profile(&self.profile);
        let forecast = self.test_pass_forecast;
        thresholds.observe_verify_steps(&self.step_outcomes, forecast, self.ema_alpha)
    }
}

/// The verify runs a dispatcher has not written to `gate-thresholds.json`
/// yet. Once they add up to `[learning] gate_threshold_flush_interval`
/// observations they are written together. What is left is written before a
/// plan's retry budgets are read ([`Self::flush`]) and when the dispatcher is
/// dropped at the end of its run (reg-c7ecf6).
pub(super) struct GateThresholdWrites {
    interval: u64,
    pending: parking_lot::Mutex<PendingRuns>,
}

/// Runs waiting to be written, and where they go.
#[derive(Default)]
struct PendingRuns {
    path: Option<PathBuf>,
    runs: Vec<VerifyRun>,
    observations: u64,
}

impl GateThresholdWrites {
    /// Write every `interval` observations, at least one.
    pub(super) fn new(interval: u64) -> Self {
        Self {
            interval: interval.max(1),
            pending: parking_lot::Mutex::default(),
        }
    }

    /// Hold `run`, bound for `path`. Returns the runs to write now, this one
    /// last, once they add up to the interval.
    fn add(&self, path: &Path, run: VerifyRun) -> Option<Vec<VerifyRun>> {
        let mut pending = self.pending.lock();
        pending.observations += run.observations();
        pending.runs.push(run);
        pending.path = Some(path.to_path_buf());
        if pending.observations < self.interval {
            return None;
        }
        pending.observations = 0;
        Some(std::mem::take(&mut pending.runs))
    }

    /// Write the runs held so far.
    pub(super) fn flush(&self) {
        let pending = std::mem::take(&mut *self.pending.lock());
        write_held_runs(pending);
    }
}

impl Drop for GateThresholdWrites {
    fn drop(&mut self) {
        write_held_runs(std::mem::take(self.pending.get_mut()));
    }
}

/// Write `pending`'s runs to their `gate-thresholds.json`, logging a failure.
fn write_held_runs(pending: PendingRuns) {
    if pending.runs.is_empty() {
        return;
    }
    let Some(path) = pending.path else {
        return;
    };
    let _files = GATE_LEARNING_FILES
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let written = GateThresholds::update_locked(&path, |thresholds| {
        for run in &pending.runs {
            run.apply(thresholds);
        }
    });
    if let Err(err) = written {
        tracing::warn!(
            path = %path.display(),
            error = %err,
            "P2-LRN-6 Loop 1: held gate thresholds were not written (non-fatal)"
        );
    }
}

impl GraphTaskDispatcher {
    /// Settle what an attempt's verify run teaches the persisted gate
    /// learning. Its skip advice is read at once ([`skip_advice`]). The
    /// thresholds take the run with the runs before it once they add up to
    /// the flush interval ([`GateThresholdWrites`]), and the dashboard then
    /// hears of them. The file is read and written in one read-modify-write
    /// under its lock ([`GateThresholds::update_locked`]), so tasks and
    /// processes that update it at once lose nothing (bug-e0f472). A file
    /// that fails to load or save is logged and left to the next task's
    /// update. A frozen run records nothing.
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
        // A frozen run (decision 2218) records nothing in the thresholds;
        // retry budgets still read them as the run found them.
        if self.learning_frozen() {
            return;
        }
        let profile = threshold_profile(task);
        let temperament = self
            .config
            .agent
            .temperament_for_role(task.role.as_deref().unwrap_or("implementer"));

        let run = VerifyRun {
            profile: profile.clone(),
            step_outcomes: step_outcomes.to_vec(),
            test_pass_forecast,
            ema_alpha: self.config.gates.effective_ema_alpha(),
        };
        let due = self.gate_threshold_writes.add(thresholds_path, run);

        let files = GATE_LEARNING_FILES
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let updated = GateThresholds::update_locked(thresholds_path, |thresholds| {
            let mut learning = skip_advice(thresholds, temperament, step_outcomes);
            for run in due.iter().flatten() {
                learning.residuals.extend(run.apply(thresholds));
            }
            learning
        });
        drop(files);
        let (thresholds, learning) = match updated {
            Ok(updated) => updated,
            Err(err) => {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    error = %err,
                    "P2-LRN-6 Loop 1: gate threshold update failed (non-fatal)"
                );
                return;
            }
        };

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
        let Some(written) = due else {
            return;
        };
        // The learning tab shows the per-rung EMAs as saved, at once.
        if let Some(tui) = &self.tui_bridge {
            if let Ok(json) = serde_json::to_string(&thresholds) {
                tui.gate_thresholds_updated(&json);
            }
        }
        tracing::debug!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            runs = written.len(),
            profile = %profile.name,
            "P2-LRN-6 Loop 1: gate thresholds updated"
        );
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };

    fn steps(outcomes: &[(&str, bool)]) -> Vec<(String, bool)> {
        outcomes
            .iter()
            .map(|(phase, passed)| ((*phase).to_string(), *passed))
            .collect()
    }

    /// One verify run through the gate learning at once, as a flush interval
    /// of one writes it: the advice, then the thresholds.
    fn update_graph_gate_thresholds(
        thresholds: &mut GateThresholds,
        profile: &ThresholdProfile,
        temperament: Temperament,
        step_outcomes: &[(String, bool)],
        test_pass_forecast: Option<(f64, f64)>,
    ) -> GateLearning {
        let mut learning = skip_advice(thresholds, temperament, step_outcomes);
        let run = VerifyRun {
            profile: profile.clone(),
            step_outcomes: step_outcomes.to_vec(),
            test_pass_forecast,
            ema_alpha: roko_core::config::GatesConfig::default().ema_alpha,
        };
        learning.residuals = run.apply(thresholds);
        learning
    }

    /// reg-c7ecf6: verify runs reach gate-thresholds.json once they add up
    /// to `[learning] gate_threshold_flush_interval` observations, on a
    /// flush, and when the dispatcher is dropped.
    #[tokio::test]
    async fn gate_thresholds_are_written_every_flush_interval() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("learn").join("gate-thresholds.json");
        let feedback = GraphFeedbackContext {
            gate_thresholds_path: Some(path.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| config.learning.gate_threshold_flush_interval = 3,
            feedback,
        )
        .await;
        let spec = make_spec(&task);
        let compiles = || {
            let thresholds = GateThresholds::load_or_default(&path).expect("load thresholds");
            thresholds.rungs[&0].total_count
        };

        let first = steps(&[("compile", true), ("test", true)]);
        dispatcher.settle_gate_learning(&spec, &task, &first, None);
        assert_eq!(compiles(), 0, "two observations wait for a third");
        dispatcher.settle_gate_learning(&spec, &task, &steps(&[("compile", false)]), None);
        assert_eq!(compiles(), 2, "the third writes all three");
        dispatcher.settle_gate_learning(&spec, &task, &steps(&[("compile", true)]), None);
        assert_eq!(compiles(), 2, "the next run waits for more");
        dispatcher.gate_threshold_writes.flush();
        assert_eq!(compiles(), 3, "a flush writes what is held");
        dispatcher.settle_gate_learning(&spec, &task, &steps(&[("compile", true)]), None);
        drop(dispatcher);
        assert_eq!(
            compiles(),
            4,
            "dropping the dispatcher writes what was left"
        );
    }

    /// Fake Claude CLI that answers every prompt, and adds a line to
    /// `reflection-calls` beside itself for each post-gate reflection asked
    /// of it.
    const REFLECTING_PROVIDER: &str = r#"#!/bin/sh
set -eu
input="$(cat)"
case "$input" in
  *"Lesson (one sentence)"*) printf 'reflection\n' >> "$(dirname -- "$0")/reflection-calls" ;;
esac
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// One attempt whose verify step fails, in workspace `temp` with seeded
    /// gate thresholds, a flush interval of one and `[learning] frozen` set
    /// to `frozen`. The test model is the cheap helper model too, so a
    /// post-gate reflection is asked of it. Returns the thresholds' bytes
    /// before and after.
    async fn failed_verify_attempt(temp: &tempfile::TempDir, frozen: bool) -> (Vec<u8>, Vec<u8>) {
        let learn = temp.path().join("learn");
        std::fs::create_dir_all(&learn).expect("create the learn directory");
        let path = learn.join("gate-thresholds.json");
        let seeded = serde_json::to_vec(&GateThresholds::default()).expect("serialize thresholds");
        std::fs::write(&path, &seeded).expect("seed the thresholds");
        let feedback = GraphFeedbackContext {
            gate_thresholds_path: Some(path.clone()),
            post_gate_reflection_path: Some(learn.join("post-gate-reflections.json")),
            replan_on_gate_failure: true,
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) = make_test_dispatcher(
            temp,
            REFLECTING_PROVIDER,
            |config| {
                no_auto_fix(config);
                if let Some(model) = config.models.get_mut("stream-model") {
                    model.supports_tools = true;
                }
                config.learning.gate_threshold_flush_interval = 1;
                config.learning.frozen = frozen;
            },
            feedback,
        )
        .await;
        task.verify = vec![verify_step("test", "false")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the verify step fails");
        let after = std::fs::read(&path).expect("read the thresholds");
        (seeded, after)
    }

    /// Decision 2218: a frozen run's failed verify step leaves
    /// `gate-thresholds.json` as it was and asks the helper model for no
    /// post-gate reflection, while a live run records the step and asks for
    /// one. The diagnosis the retry prompt carries is within-run context, so
    /// both ask for that.
    #[tokio::test]
    async fn frozen_gate_failure_writes_no_thresholds_or_reflections() {
        let frozen = tempdir().expect("tempdir");
        let (seeded, after) = failed_verify_attempt(&frozen, true).await;
        assert_eq!(after, seeded, "a frozen run records nothing");
        let live = tempdir().expect("tempdir");
        let (seeded, after) = failed_verify_attempt(&live, false).await;
        assert_ne!(after, seeded, "a live run records its verify run");

        // The reflection runs in the background. Once the live run's has
        // asked, a frozen run's would have asked too.
        let live_asked = live.path().join("reflection-calls");
        for _ in 0..600 {
            if live_asked.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(live_asked.exists(), "a live run asks for a reflection");
        let frozen_asked = frozen.path().join("reflection-calls");
        assert!(!frozen_asked.exists(), "a frozen run asks for none");
        let reflections = frozen.path().join("learn/post-gate-reflections.json");
        assert!(!reflections.exists(), "a frozen run keeps none");
    }

    /// gap-7a3527: a Graph verify run moves the pass-rate EMA by
    /// `[gates] ema_alpha`, so a larger alpha weights the latest outcome more.
    #[tokio::test]
    async fn gate_threshold_ema_uses_the_configured_alpha() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("learn").join("gate-thresholds.json");
        let feedback = GraphFeedbackContext {
            gate_thresholds_path: Some(path.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                config.learning.gate_threshold_flush_interval = 1;
                config.gates.ema_alpha = 0.5;
            },
            feedback,
        )
        .await;
        let spec = make_spec(&task);

        // The first outcome replaces the prior; the failure then moves the
        // EMA halfway down (alpha 0.5), not a tenth of the way.
        dispatcher.settle_gate_learning(&spec, &task, &steps(&[("compile", true)]), None);
        dispatcher.settle_gate_learning(&spec, &task, &steps(&[("compile", false)]), None);
        let thresholds = GateThresholds::load_or_default(&path).expect("load thresholds");
        let ema = thresholds.rungs[&0].ema_pass_rate;
        assert!((ema - 0.5).abs() < 1e-9, "ema {ema}");
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
