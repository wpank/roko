//! Retry budget of each Graph plan task, set by the adaptive gate thresholds.
//!
//! The Graph engine retries a failed task up to its `max_retries`. A task that
//! authors `max_retries` in `tasks.toml` keeps exactly that: the thresholds
//! never lower an author's budget, and never raise one either (fixtures and
//! tests author `max_retries = 0` to fail fast). The log only says when the
//! thresholds suggest another budget for it (P3-15). For the other tasks, each
//! verify step that maps to a canonical gate rung (compile, clippy, test)
//! suggests a budget from its rung's pass-rate EMA in
//! `.roko/learn/gate-thresholds.json`, which Graph verify runs keep current: a
//! rung that usually passes suggests few retries, one that often fails more.
//! Suggestions stay within `[gates] adaptive_min_retries..=adaptive_max_retries`,
//! and a rung with under five observations suggests their midpoint. Durable
//! knowledge that names a rung as failing counts it as likelier to fail while
//! it has under ten observations (P1-09). The task gets its likeliest-to-fail
//! rung's suggestion. A task with no such step keeps the default budget. While
//! the model ladder routes tasks, one that no `model_hint` or `preferred_model`
//! pins gets at least enough retries to climb it (gap-460230).
//! `plan run --max-retries` overrides all of this.
//!
//! M1 moves these budgets during a run (B2, decision 8101, 8126): the Graph
//! executor reads [`LiveRetryBudgets`] before each retry decision, which add
//! the `retry_delta` of the θ the task's chain runs to a budget the task does
//! not author ([`TaskRetryBudgets::max_retries_with_delta`]).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use roko_core::config::GatesConfig;
use roko_core::defaults::DEFAULT_GATE_RETRY_MIN_OBSERVATIONS;
use roko_gate::AdaptiveThresholds;
use roko_gate::rung_for_gate_name;
use roko_graph::cell::CellContext;
use roko_graph::cells::TaskExecutionSpec;
use roko_neuro::KnowledgeStore;

use super::GraphTaskDispatcher;
use crate::knowledge_helpers::apply_neuro_gate_hints;
use crate::task_parser::TaskDef;

/// Where a task's retry budget came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RetryBudgetSource {
    /// `max_retries` authored in `tasks.toml`.
    Authored,
    /// The default budget: no verify step maps to a canonical gate rung, or
    /// the run records no gate thresholds.
    Default,
    /// Suggested by the thresholds of the task's likeliest-to-fail rung.
    Adaptive {
        rung: u32,
        ema_pass_rate: f64,
        observations: u64,
    },
    /// Raised to what climbing the model ladder takes (gap-460230).
    Ladder,
}

/// A task's retry budget and where it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RetryBudget {
    pub(crate) max_retries: u32,
    pub(crate) source: RetryBudgetSource,
}

/// Retry budgets of one plan's tasks.
pub(crate) struct TaskRetryBudgets {
    /// `None` when the run records no gate thresholds.
    thresholds: Option<AdaptiveThresholds>,
    /// Ids of the tasks that author `max_retries`.
    authored: BTreeSet<String>,
    /// Least budget of a task that does not author `max_retries` and whose
    /// model is not pinned: `0`, or what climbing the model ladder takes
    /// while it routes tasks.
    ladder_min_retries: u32,
    /// `[gates] adaptive_min_retries..=adaptive_max_retries`, the range M1's
    /// `retry_delta` moves a budget within.
    adaptive_range: (u32, u32),
}

impl TaskRetryBudgets {
    /// Budgets from the thresholds in `thresholds_path` (a missing or
    /// unreadable file is a cold start), bounded by `gates`, for the plan in
    /// `tasks_toml`.
    pub(crate) fn load(
        thresholds_path: Option<&Path>,
        gates: &GatesConfig,
        tasks_toml: &Path,
    ) -> Self {
        let thresholds = thresholds_path.map(|path| {
            let mut thresholds = AdaptiveThresholds::load(path).unwrap_or_else(|error| {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        path = %path.display(),
                        %error,
                        "gate thresholds unreadable; retry budgets start cold"
                    );
                }
                AdaptiveThresholds::new()
            });
            thresholds.apply_gates_config(gates);
            thresholds
        });
        let authored = std::fs::read_to_string(tasks_toml)
            .map(|content| authored_max_retries(&content))
            .unwrap_or_default();
        let min = gates.adaptive_min_retries;
        Self {
            thresholds,
            authored,
            ladder_min_retries: 0,
            adaptive_range: (min, gates.adaptive_max_retries.max(min)),
        }
    }

    /// Bias the thresholds by what durable knowledge says about gate rungs
    /// (P1-09, [`apply_neuro_gate_hints`]): a rung that `knowledge` names as
    /// failing, with under ten observations, counts as likelier to fail, so
    /// its tasks get more retries. Only these budgets see the bias. The
    /// thresholds file is not written, so it never compounds across runs.
    #[must_use]
    pub(crate) fn with_neuro_gate_hints(mut self, knowledge: &KnowledgeStore) -> Self {
        if let Some(thresholds) = &mut self.thresholds {
            apply_neuro_gate_hints(knowledge, thresholds);
        }
        self
    }

    /// Give every task that does not author `max_retries` at least
    /// `min_retries`: two attempts on each rung the model ladder lets it climb
    /// (gap-460230). An authored budget is still kept exactly, and a task
    /// whose `model_hint` or `preferred_model` pins its model never climbs,
    /// so it keeps its own.
    #[must_use]
    pub(crate) fn with_ladder_min_retries(mut self, min_retries: u32) -> Self {
        self.ladder_min_retries = min_retries;
        self
    }

    /// `task`'s retry budget.
    pub(crate) fn for_task(&self, task: &TaskDef) -> RetryBudget {
        if self.authored.contains(&task.id) {
            return RetryBudget {
                max_retries: task.max_retries,
                source: RetryBudgetSource::Authored,
            };
        }
        let budget = self.suggested(task);
        if !pinned(task) && budget.max_retries < self.ladder_min_retries {
            return RetryBudget {
                max_retries: self.ladder_min_retries,
                source: RetryBudgetSource::Ladder,
            };
        }
        budget
    }

    /// `task`'s retry budget moved by M1's `retry_delta` (B2, decision 8101,
    /// 8126). An authored budget never changes. Any other moves by `delta`
    /// within `[gates] adaptive_min_retries..=adaptive_max_retries`, a budget
    /// already outside that range moving no further out, and an unpinned
    /// task keeps what climbing the model ladder takes. θ₀'s `delta` of 0
    /// changes nothing.
    pub(crate) fn max_retries_with_delta(&self, task: &TaskDef, delta: i32) -> u32 {
        let budget = self.for_task(task);
        if delta == 0 || budget.source == RetryBudgetSource::Authored {
            return budget.max_retries;
        }
        let (min, max) = self.adaptive_range;
        let low = i64::from(min.min(budget.max_retries));
        let high = i64::from(max.max(budget.max_retries));
        let moved = (i64::from(budget.max_retries) + i64::from(delta)).clamp(low, high);
        let floor = if pinned(task) {
            0
        } else {
            self.ladder_min_retries
        };
        u32::try_from(moved)
            .unwrap_or(budget.max_retries)
            .max(floor)
    }

    /// Budget of a task that does not author `max_retries`: the default, or
    /// its likeliest-to-fail rung's suggestion.
    fn suggested(&self, task: &TaskDef) -> RetryBudget {
        let default = RetryBudget {
            max_retries: task.max_retries,
            source: RetryBudgetSource::Default,
        };
        let Some(thresholds) = &self.thresholds else {
            return default;
        };
        let rungs: BTreeSet<u32> = task
            .verify
            .iter()
            .filter_map(|step| rung_for_gate_name(&step.phase))
            .map(|rung| rung.as_index())
            .collect();
        // Ties go to the lowest rung, so the choice is deterministic.
        let Some((max_retries, rung)) = rungs
            .into_iter()
            .map(|rung| (thresholds.suggested_max_retries(rung), rung))
            .max_by_key(|&(suggested, rung)| (suggested, std::cmp::Reverse(rung)))
        else {
            return default;
        };
        let stats = thresholds.rung_stats(rung);
        RetryBudget {
            max_retries,
            source: RetryBudgetSource::Adaptive {
                rung,
                ema_pass_rate: stats.map_or(0.0, |stats| stats.ema_pass_rate),
                observations: stats.map_or(0, |stats| stats.total_observations),
            },
        }
    }

    /// `task`'s retry budget, logging where an adaptive one came from, and
    /// what the thresholds would suggest instead of an authored one.
    pub(crate) fn max_retries(&self, plan_id: &str, task: &TaskDef) -> u32 {
        let budget = self.for_task(task);
        if let RetryBudgetSource::Adaptive {
            rung,
            ema_pass_rate,
            observations,
        } = budget.source
        {
            tracing::info!(
                plan_id,
                task_id = %task.id,
                max_retries = budget.max_retries,
                default_max_retries = task.max_retries,
                rung,
                ema_pass_rate,
                observations,
                "retry budget set by adaptive gate thresholds"
            );
        }
        if budget.source == RetryBudgetSource::Ladder {
            tracing::info!(
                plan_id,
                task_id = %task.id,
                max_retries = budget.max_retries,
                "retry budget raised so the task can climb the model ladder"
            );
        }
        if let Some(advice) = self.authored_budget_advice(task)
            && let RetryBudgetSource::Adaptive {
                rung,
                ema_pass_rate,
                observations,
            } = advice.source
        {
            tracing::info!(
                plan_id,
                task_id = %task.id,
                max_retries = budget.max_retries,
                suggested_max_retries = advice.max_retries,
                rung,
                ema_pass_rate,
                observations,
                "P3-15: the authored retry budget differs from what the gate thresholds suggest"
            );
        }
        budget.max_retries
    }

    /// The budget the thresholds would give a task that authors another one
    /// (P3-15). The authored budget is kept either way. A rung with under
    /// five observations suggests only the midpoint of the range, so it gives
    /// no advice.
    fn authored_budget_advice(&self, task: &TaskDef) -> Option<RetryBudget> {
        if !self.authored.contains(&task.id) {
            return None;
        }
        let suggested = self.suggested(task);
        match suggested.source {
            RetryBudgetSource::Adaptive { observations, .. }
                if observations >= DEFAULT_GATE_RETRY_MIN_OBSERVATIONS
                    && suggested.max_retries != task.max_retries =>
            {
                Some(suggested)
            }
            _ => None,
        }
    }
}

/// Whether `task`'s `model_hint` or `preferred_model` pins its model, so the
/// model ladder never climbs it.
fn pinned(task: &TaskDef) -> bool {
    task.model_hint.is_some() || task.hints.preferred_model.is_some()
}

/// The live retry budgets of one plan run's tasks (M1's B2, 8126), which the
/// Graph executor reads before each retry decision: each task's plan-load
/// budget moved by the `retry_delta` of the θ its chain runs now. A task
/// without M1 keeps its plan-load budget. Each budget it hands out becomes
/// the one M1's sink resolves the task's chain at.
struct LiveRetryBudgets {
    budgets: TaskRetryBudgets,
    dispatcher: Arc<GraphTaskDispatcher>,
}

impl roko_graph::cells::RetryBudgetSource for LiveRetryBudgets {
    fn max_retries(&self, spec: &TaskExecutionSpec, ctx: &CellContext) -> Option<u32> {
        let task: TaskDef = serde_json::from_str(&spec.task_def_json).ok()?;
        let theta = self.dispatcher.next_attempt_theta(spec, &task.id, ctx)?;
        let max_retries = self
            .budgets
            .max_retries_with_delta(&task, theta.retry_delta);
        if let Some(sink) = self.dispatcher.homeostasis_sink() {
            sink.set_retry_limits(&spec.plan_id, [(task.id, max_retries)]);
        }
        Some(max_retries)
    }
}

impl GraphTaskDispatcher {
    /// The live source of a plan run's retry `budgets` (B2, 8126), for each
    /// of its tasks' executor cells to read before a retry.
    pub(crate) fn live_retry_budgets(
        self: Arc<Self>,
        budgets: TaskRetryBudgets,
    ) -> Arc<dyn roko_graph::cells::RetryBudgetSource> {
        Arc::new(LiveRetryBudgets {
            budgets,
            dispatcher: self,
        })
    }
}

/// Ids of the tasks in `tasks_toml` content that author `max_retries`.
fn authored_max_retries(tasks_toml: &str) -> BTreeSet<String> {
    roko_graph::AuthoredPlan::from_tasks_toml(tasks_toml)
        .map(|plan| {
            plan.tasks
                .into_iter()
                .filter(|(_, task)| task.get("max_retries").is_some())
                .map(|(id, _)| id)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use roko_neuro::{KnowledgeEntry, KnowledgeKind};
    use tempfile::tempdir;

    use super::*;
    use crate::runner::persist::GateThresholds;
    use crate::task_parser::{TasksFile, VerifyStep};

    const TASKS_TOML: &str = r#"
[meta]
plan = "budget"

[[task]]
id = "AUTHORED"
title = "authors its budget"
max_retries = 0
[[task.verify]]
phase = "test"
command = "true"

[[task.verify]]
phase = "compile"
command = "true"

[[task]]
id = "TEST"
title = "gated by tests"
[[task.verify]]
phase = "test"
command = "true"

[[task]]
id = "MIXED"
title = "gated by compile and tests"
[[task.verify]]
phase = "compile"
command = "true"

[[task.verify]]
phase = "test"
command = "true"

[[task]]
id = "STRUCTURAL"
title = "no canonical rung"
[[task.verify]]
phase = "structural"
command = "true"
"#;

    fn task(id: &str) -> TaskDef {
        TasksFile::parse_str(TASKS_TOML)
            .expect("fixture plan")
            .tasks
            .into_iter()
            .find(|task| task.id == id)
            .expect("fixture task")
    }

    /// Plan fixture beside thresholds the Graph verify writer recorded:
    /// compile (rung 0) always passed, test (rung 2) mostly failed.
    fn budgets(dir: &Path, gates: &GatesConfig) -> TaskRetryBudgets {
        let tasks_toml = dir.join("tasks.toml");
        std::fs::write(&tasks_toml, TASKS_TOML).expect("write plan");
        let path = dir.join("gate-thresholds.json");
        let mut thresholds = GateThresholds::default();
        for passed in [true; 10] {
            thresholds.observe(0, passed);
        }
        for passed in [false, false, false, true, false, false, false, false] {
            thresholds.observe(2, passed);
        }
        thresholds.save(&path).expect("save thresholds");
        TaskRetryBudgets::load(Some(&path), gates, &tasks_toml)
    }

    #[test]
    fn an_authored_budget_is_kept_exactly() {
        let dir = tempdir().expect("tempdir");
        let budget = budgets(dir.path(), &GatesConfig::default()).for_task(&task("AUTHORED"));
        assert_eq!(budget.max_retries, 0);
        assert_eq!(budget.source, RetryBudgetSource::Authored);
    }

    /// P3-15: an authored budget is kept, and the thresholds' advice names
    /// the budget they would set once its rung has enough observations.
    #[test]
    fn an_authored_budget_gets_advice_only_from_a_warm_rung() {
        let dir = tempdir().expect("tempdir");
        let budgets = budgets(dir.path(), &GatesConfig::default());
        let authored = task("AUTHORED");
        assert_eq!(budgets.for_task(&authored).max_retries, 0);
        let advice = budgets
            .authored_budget_advice(&authored)
            .expect("advice from the warm test rung");
        assert!(
            matches!(advice.source, RetryBudgetSource::Adaptive { rung: 2, .. }),
            "{advice:?}"
        );
        assert!(advice.max_retries > 0, "{advice:?}");
        assert_eq!(budgets.authored_budget_advice(&task("TEST")), None);

        let cold = TaskRetryBudgets::load(
            Some(&dir.path().join("missing.json")),
            &GatesConfig::default(),
            &dir.path().join("tasks.toml"),
        );
        assert_eq!(cold.authored_budget_advice(&authored), None);
    }

    #[test]
    fn a_rung_that_often_fails_raises_the_default_budget() {
        let dir = tempdir().expect("tempdir");
        let budgets = budgets(dir.path(), &GatesConfig::default());
        let budget = budgets.for_task(&task("TEST"));
        let RetryBudgetSource::Adaptive {
            rung, observations, ..
        } = budget.source
        else {
            panic!("expected an adaptive budget, got {budget:?}");
        };
        assert_eq!((rung, observations), (2, 8));
        assert!(budget.max_retries > 3, "{budget:?}");
        assert!(budget.max_retries <= 5, "{budget:?}");
        // The likeliest-to-fail rung sets a mixed task's budget.
        assert_eq!(budgets.for_task(&task("MIXED")), budget);
    }

    #[test]
    fn a_rung_that_always_passes_lowers_it_to_the_configured_floor() {
        let dir = tempdir().expect("tempdir");
        let mut only_compile = task("TEST");
        only_compile.verify = vec![VerifyStep {
            phase: "compile".to_string(),
            command: "true".to_string(),
            fail_msg: None,
            timeout_ms: 1_000,
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        }];

        // The default floor is a task's default max_retries (3).
        let budget = budgets(dir.path(), &GatesConfig::default()).for_task(&only_compile);
        assert_eq!(budget.max_retries, 3);

        let gates = GatesConfig {
            adaptive_min_retries: 2,
            adaptive_max_retries: 8,
            ..GatesConfig::default()
        };
        let budgets = budgets(dir.path(), &gates);
        assert_eq!(budgets.for_task(&only_compile).max_retries, 2);
        assert!(budgets.for_task(&task("TEST")).max_retries <= 8);
    }

    #[test]
    fn a_cold_rung_suggests_the_midpoint_of_the_configured_range() {
        let dir = tempdir().expect("tempdir");
        let tasks_toml = dir.path().join("tasks.toml");
        std::fs::write(&tasks_toml, TASKS_TOML).expect("write plan");
        let missing = dir.path().join("gate-thresholds.json");

        // Midpoint of the default range 3..=5.
        let cold = TaskRetryBudgets::load(Some(&missing), &GatesConfig::default(), &tasks_toml);
        assert_eq!(cold.for_task(&task("TEST")).max_retries, 4);

        let gates = GatesConfig {
            adaptive_min_retries: 2,
            adaptive_max_retries: 6,
            ..GatesConfig::default()
        };
        let cold = TaskRetryBudgets::load(Some(&missing), &gates, &tasks_toml);
        assert_eq!(cold.for_task(&task("TEST")).max_retries, 4);
    }

    /// P1-09: durable knowledge that names a rung as failing raises the
    /// budget of a rung with few observations, and the thresholds file stays
    /// as the verify runs left it.
    #[test]
    fn knowledge_of_a_failing_rung_raises_a_young_rungs_budget() {
        let dir = tempdir().expect("tempdir");
        let tasks_toml = dir.path().join("tasks.toml");
        std::fs::write(&tasks_toml, TASKS_TOML).expect("write plan");
        // Six passes: enough to set a budget, under the ten knowledge can bias.
        let path = dir.path().join("gate-thresholds.json");
        let mut thresholds = GateThresholds::default();
        for _ in 0..6 {
            thresholds.observe(0, true);
        }
        thresholds.save(&path).expect("save thresholds");
        let saved = std::fs::read_to_string(&path).expect("read thresholds");
        let mut only_compile = task("TEST");
        only_compile.verify = vec![VerifyStep {
            phase: "compile".to_string(),
            command: "true".to_string(),
            fail_msg: None,
            timeout_ms: 1_000,
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        }];
        let knowledge = KnowledgeStore::for_workdir(dir.path());
        let budget = || {
            TaskRetryBudgets::load(Some(&path), &GatesConfig::default(), &tasks_toml)
                .with_neuro_gate_hints(&knowledge)
                .for_task(&only_compile)
        };

        // No knowledge yet: a rung that always passed gets the floor.
        assert_eq!(budget().max_retries, 3);

        knowledge
            .add(KnowledgeEntry {
                id: "compile-failures".into(),
                kind: KnowledgeKind::AntiKnowledge,
                content: "gate failure: compile errors keep failing rung 0".into(),
                confidence: 0.9,
                ..KnowledgeEntry::default()
            })
            .expect("persist gate knowledge");
        let hinted = budget();
        assert_eq!(hinted.max_retries, 4, "{hinted:?}");
        let RetryBudgetSource::Adaptive {
            rung, observations, ..
        } = hinted.source
        else {
            panic!("expected an adaptive budget, got {hinted:?}");
        };
        assert_eq!((rung, observations), (0, 6));
        assert_eq!(
            std::fs::read_to_string(&path).expect("reread thresholds"),
            saved
        );
    }

    /// gap-460230: while the ladder is on, a task that does not author
    /// `max_retries` gets enough retries to climb it. An authored budget, a
    /// pinned task's budget and a larger adaptive budget stay as they are.
    #[test]
    fn the_ladder_raises_unauthored_budgets_to_its_floor() {
        let dir = tempdir().expect("tempdir");
        let budgets = budgets(dir.path(), &GatesConfig::default()).with_ladder_min_retries(5);
        let structural = budgets.for_task(&task("STRUCTURAL"));
        assert_eq!(
            structural,
            RetryBudget {
                max_retries: 5,
                source: RetryBudgetSource::Ladder,
            }
        );
        assert_eq!(budgets.for_task(&task("AUTHORED")).max_retries, 0);
        let mut pinned = task("STRUCTURAL");
        pinned.model_hint = Some("claude-sonnet-4-6".to_string());
        assert_eq!(budgets.for_task(&pinned).source, RetryBudgetSource::Default);
        let mut preferred = task("STRUCTURAL");
        preferred.hints.preferred_model = Some("claude-sonnet-4-6".to_string());
        assert_eq!(
            budgets.for_task(&preferred).source,
            RetryBudgetSource::Default
        );
        let gates = GatesConfig {
            adaptive_min_retries: 6,
            adaptive_max_retries: 8,
            ..GatesConfig::default()
        };
        let generous = budgets_with(dir.path(), &gates, 5).for_task(&task("TEST"));
        assert!(
            matches!(generous.source, RetryBudgetSource::Adaptive { .. }),
            "{generous:?}"
        );
        assert!(generous.max_retries > 5, "{generous:?}");
    }

    fn budgets_with(dir: &Path, gates: &GatesConfig, ladder_min: u32) -> TaskRetryBudgets {
        budgets(dir, gates).with_ladder_min_retries(ladder_min)
    }

    #[test]
    fn tasks_without_a_canonical_rung_or_thresholds_keep_the_default() {
        let dir = tempdir().expect("tempdir");
        let structural = task("STRUCTURAL");
        let budget = budgets(dir.path(), &GatesConfig::default()).for_task(&structural);
        assert_eq!(budget.source, RetryBudgetSource::Default);
        assert_eq!(budget.max_retries, structural.max_retries);

        let untracked = TaskRetryBudgets::load(
            None,
            &GatesConfig::default(),
            &dir.path().join("tasks.toml"),
        );
        assert_eq!(
            untracked.for_task(&task("TEST")).source,
            RetryBudgetSource::Default
        );
    }

    /// M1's B2 (8126, decision 8101): `retry_delta` moves a budget the task
    /// does not author within `[gates] adaptive_min_retries..=
    /// adaptive_max_retries`, a budget already outside it no further out,
    /// and never below the ladder's floor; an authored budget never changes,
    /// and θ₀'s delta of 0 changes nothing. The executor's live source reads
    /// the delta of the θ the task's chain runs now.
    #[tokio::test]
    async fn retry_delta_never_changes_authored_budget() {
        use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
        use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
        use roko_core::config::schema::RokoConfig;
        use roko_learn::homeostasis::controller::Controller;
        use roko_learn::homeostasis::detect::Baseline;
        use roko_learn::homeostasis::policy::ViabilityPolicy;

        use crate::graph_task_dispatch::GraphFeedbackContext;
        use crate::graph_task_dispatch::tests::{VERIFY_PROVIDER, make_test_dispatcher, no_auto_fix};
        use crate::runtime_feedback::HomeostasisSink;

        let dir = tempdir().expect("tempdir");
        let gates = GatesConfig {
            adaptive_min_retries: 2,
            adaptive_max_retries: 8,
            ..GatesConfig::default()
        };
        let wide = budgets(dir.path(), &gates);
        let mut pinned = task("STRUCTURAL");
        pinned.model_hint = Some("claude-sonnet-4-6".to_string());
        for delta in [-2, -1, 0, 1, 2] {
            assert_eq!(wide.max_retries_with_delta(&task("AUTHORED"), delta), 0);
        }
        // STRUCTURAL keeps the default budget, 3.
        let moved = |budgets: &TaskRetryBudgets, task: &TaskDef| {
            [-2, -1, 0, 1, 9].map(|delta| budgets.max_retries_with_delta(task, delta))
        };
        assert_eq!(moved(&wide, &task("STRUCTURAL")), [2, 2, 3, 4, 8]);
        // While the ladder routes tasks it takes five retries to climb, which
        // an unpinned task keeps; a pinned one never climbs.
        let ladder = budgets_with(dir.path(), &gates, 5);
        assert_eq!(moved(&ladder, &task("STRUCTURAL")), [5, 5, 5, 6, 8]);
        assert_eq!(moved(&ladder, &pinned), [2, 2, 3, 4, 8]);
        // A budget above `adaptive_max_retries` goes no higher.
        let narrow = budgets_with(dir.path(), &GatesConfig::default(), 6);
        assert_eq!(moved(&narrow, &task("STRUCTURAL")), [6, 6, 6, 6, 6]);

        // The live source: θ₀, then a θ that adds one retry.
        const POLICY: &str = "policy_version = 1\n\
            ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
            ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";
        let config = RokoConfig::default();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let settings = HomeostasisConfig {
            mode: HomeostasisMode::On,
            ..HomeostasisConfig::default()
        };
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let controller = Controller::new(
            &settings,
            policy,
            theta0.clone(),
            ladders.clone(),
            baseline,
            0,
        );
        let sink = HomeostasisSink::new(dir.path(), Some(controller), None);
        let sink = Arc::new(sink.with_holdout(0.0));
        let feedback = GraphFeedbackContext {
            homeostasis: Some(Arc::clone(&sink)),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, _) =
            make_test_dispatcher(&dir, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let plan_budgets = dispatcher.task_retry_budgets(dir.path());
        let live = Arc::clone(&dispatcher).live_retry_budgets(plan_budgets);
        let ctx = CellContext::new().with_run_id("live".to_string());
        let read = |task: &TaskDef| {
            let spec = TaskExecutionSpec {
                plan_id: "budget".to_string(),
                max_retries: task.max_retries,
                task_def_json: serde_json::to_string(task).expect("serialize task"),
                ..TaskExecutionSpec::default()
            };
            live.max_retries(&spec, &ctx)
        };
        // The pinned task never climbs the ladder, so it keeps the default 3.
        let authored = task("AUTHORED");
        assert_eq!((read(&authored), read(&pinned)), (Some(0), Some(3)));
        let more = theta0
            .step(Knob::RetryDelta, Step::Up, &ladders)
            .expect("one more retry");
        assert_eq!(sink.handle().swap(more, "more"), 1);
        assert_eq!((read(&authored), read(&pinned)), (Some(0), Some(4)));
    }
}
