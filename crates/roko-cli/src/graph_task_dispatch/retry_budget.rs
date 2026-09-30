//! Retry budget of each Graph plan task, set by the adaptive gate thresholds.
//!
//! The Graph engine retries a failed task up to its `max_retries`. A task that
//! authors `max_retries` in `tasks.toml` keeps exactly that: the thresholds
//! never lower an author's budget, and never raise one either (fixtures and
//! tests author `max_retries = 0` to fail fast). For the other tasks, each
//! verify step that maps to a canonical gate rung (compile, clippy, test)
//! suggests a budget from its rung's pass-rate EMA in
//! `.roko/learn/gate-thresholds.json`, which Graph verify runs keep current: a
//! rung that usually passes suggests few retries, one that often fails more.
//! Suggestions stay within `[gates] adaptive_min_retries..=adaptive_max_retries`,
//! and a rung with under five observations suggests their midpoint. The task
//! gets its likeliest-to-fail rung's suggestion. A task with no such step
//! keeps the default budget. While the model ladder routes tasks, one without
//! a `model_hint` gets at least enough retries to climb it (gap-460230).
//! `plan run --max-retries` overrides all of this.

use std::collections::BTreeSet;
use std::path::Path;

use roko_core::config::GatesConfig;
use roko_gate::AdaptiveThresholds;
use roko_gate::rung_for_gate_name;

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
    /// Least budget of a task that does not author `max_retries` and has no
    /// `model_hint`: `0`, or what climbing the model ladder takes while it
    /// routes tasks.
    ladder_min_retries: u32,
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
        Self {
            thresholds,
            authored,
            ladder_min_retries: 0,
        }
    }

    /// Give every task that does not author `max_retries` at least
    /// `min_retries`: two attempts on each rung the model ladder lets it climb
    /// (gap-460230). An authored budget is still kept exactly, and a task
    /// whose `model_hint` pins its model never climbs, so it keeps its own.
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
        if task.model_hint.is_none() && budget.max_retries < self.ladder_min_retries {
            return RetryBudget {
                max_retries: self.ladder_min_retries,
                source: RetryBudgetSource::Ladder,
            };
        }
        budget
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

    /// `task`'s retry budget, logging where an adaptive one came from.
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
        budget.max_retries
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
}
