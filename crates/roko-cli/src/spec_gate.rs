//! The spec gate: what `plan run` does with a plan's task specs before any
//! agent starts (S07 §4.3, decision 3201).
//!
//! [`check_plans`] scores every task with the spec-quality rules
//! ([`roko_gate::spec_quality`]) and decides per task. In every
//! `[spec_quality] mode` but `off`, a task blocks its plan when a verify step
//! can never fail (HF2), or when every step already passes on the unchanged
//! base (HF3, known only when the red-on-base check ran). Nothing else refuses
//! those two. The other hard fails are refused before the gate runs, in their
//! own words: an implementer without a verify step by plan validation
//! (PLAN_035, unless the plan sets `allow_unverified`), a greenfield claim by
//! plan validation (PLAN_033), and a missing context file by the plan loader
//! (PLAN_CONTEXT_MISSING).
//!
//! Scores only advise, except under `mode = "enforce"` (benchmark runs),
//! where a score below `block_threshold` blocks its task as well. There is no
//! per-run override, so every refusal follows from `roko.toml`.
//!
//! `plan run` calls the gate from `validate_before_run`; the plan runner can
//! call it for serve and ACP runs too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use roko_core::config::{SpecQualityConfig, SpecQualityMode};
use roko_gate::spec_quality::{
    HARD_FAILS, RedOnBase, SpecQualityRecord, SpecQualityReport, lint_files_with,
};
use serde::Serialize;

/// The hard fails that block a plan at the gate: nothing else refuses them.
pub const BLOCKING_HARD_FAILS: [&str; 2] = ["HF2", "HF3"];

/// What the gate decided for one task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecGateAction {
    /// The task runs: no hard fail, and a score at or above the allow
    /// threshold.
    Allow,
    /// The task runs, and its score below the allow threshold is reported.
    Advise,
    /// The task does not run, so neither does its plan.
    Block,
}

/// Why the gate blocks a task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpecGateFinding {
    /// The hard fail (`HF2`, `HF3`), or `score` under `enforce`.
    pub rule: &'static str,
    /// What triggered it, such as ``step 1: ends in `|| echo PASS` ``.
    pub detail: String,
}

/// The gate's decision for one task.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpecGateDecision {
    /// The `tasks.toml` path, relative to the workspace root when inside it.
    pub plan_path: String,
    /// The task id.
    pub task_id: String,
    /// What the gate decided.
    pub action: SpecGateAction,
    /// The task's spec-quality score, 0–100.
    pub score: f64,
    /// The score's band, `A` to `D`.
    pub band: &'static str,
    /// Why the task is blocked; empty unless `action` is `Block`.
    pub findings: Vec<SpecGateFinding>,
}

/// The gate's decisions over a set of plans.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpecGateReport {
    /// The `[spec_quality] mode` the gate ran in.
    pub mode: SpecQualityMode,
    /// The spec-quality records the decisions rest on; `None` when the mode
    /// is `off`.
    pub quality: Option<SpecQualityReport>,
    /// One decision per scored task, in file order.
    pub decisions: Vec<SpecGateDecision>,
}

impl SpecGateReport {
    /// The decisions that block their task.
    pub fn blocked(&self) -> impl Iterator<Item = &SpecGateDecision> {
        self.decisions
            .iter()
            .filter(|decision| decision.action == SpecGateAction::Block)
    }

    /// Whether any task is blocked, which refuses the run.
    #[must_use]
    pub fn blocks(&self) -> bool {
        self.blocked().next().is_some()
    }
}

/// Score every task of `files` against the workspace at `workdir`, and
/// decide for each what `config` makes of it. `red_on_base` holds the
/// red-on-base results by (plan path, task id) when that check ran; HF3 can
/// only block a task that has one.
#[must_use]
pub fn check_plans(
    files: &[PathBuf],
    workdir: &Path,
    config: &SpecQualityConfig,
    red_on_base: &BTreeMap<(String, String), RedOnBase>,
) -> SpecGateReport {
    if !config.is_on() {
        return SpecGateReport {
            mode: config.mode,
            quality: None,
            decisions: Vec::new(),
        };
    }
    let quality = lint_files_with(files, workdir, red_on_base);
    let decisions = quality
        .tasks
        .iter()
        .map(|record| decide(record, config))
        .collect();
    SpecGateReport {
        mode: config.mode,
        quality: Some(quality),
        decisions,
    }
}

/// The gate's decision for one scored task.
#[must_use]
pub fn decide(record: &SpecQualityRecord, config: &SpecQualityConfig) -> SpecGateDecision {
    let mut findings = Vec::new();
    for &rule in &record.hard_fail {
        if !BLOCKING_HARD_FAILS.contains(&rule) {
            continue;
        }
        match record.hard_fail_detail.get(rule) {
            Some(details) if !details.is_empty() => {
                findings.extend(details.iter().map(|detail| SpecGateFinding {
                    rule,
                    detail: detail.clone(),
                }));
            }
            _ => findings.push(SpecGateFinding {
                rule,
                detail: hard_fail_name(rule).to_string(),
            }),
        }
    }
    if config.mode == SpecQualityMode::Enforce && record.score < config.block_threshold {
        findings.push(SpecGateFinding {
            rule: "score",
            detail: format!(
                "score {:.2} is below spec_quality.block_threshold ({})",
                record.score, config.block_threshold
            ),
        });
    }
    let action = if !findings.is_empty() {
        SpecGateAction::Block
    } else if record.score >= config.allow_threshold {
        SpecGateAction::Allow
    } else {
        SpecGateAction::Advise
    };
    SpecGateDecision {
        plan_path: record.plan_path.clone(),
        task_id: record.task_id.clone(),
        action,
        score: record.score,
        band: record.band,
        findings,
    }
}

/// What a hard fail means, as `HARD_FAILS` names it.
fn hard_fail_name(rule: &str) -> &'static str {
    HARD_FAILS
        .iter()
        .find(|(id, _)| *id == rule)
        .map_or("hard fail", |(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_plan(root: &Path, verify: &str) -> PathBuf {
        let dir = root.join("plans/demo");
        std::fs::create_dir_all(&dir).expect("plan dir");
        let path = dir.join("tasks.toml");
        let tasks = format!(
            r#"[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Retry limit"
description = "Add the retry limit to `parse_config`."
role = "implementer"
files = ["src/config.rs"]
depends_on = []
verify = [{{ phase = "test", command = "{verify}" }}]
"#
        );
        std::fs::write(&path, tasks).expect("write the plan");
        path
    }

    fn config(mode: SpecQualityMode) -> SpecQualityConfig {
        SpecQualityConfig {
            mode,
            ..SpecQualityConfig::default()
        }
    }

    /// 3211: a verify step that can never fail blocks its task with what
    /// makes it vacuous; `mode = "off"` scores nothing and blocks nothing;
    /// a low-scoring task without a hard fail is only advised.
    #[test]
    fn spec_gate_blocks_hard_fails_and_advises_on_scores() {
        let temp = tempfile::tempdir().expect("tempdir");
        let none = BTreeMap::new();

        let vacuous = write_plan(temp.path(), "cargo check -p demo || true");
        let advise = config(SpecQualityMode::Advise);
        let report = check_plans(&[vacuous.clone()], temp.path(), &advise, &none);
        assert!(report.blocks());
        let blocked: Vec<&SpecGateDecision> = report.blocked().collect();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].task_id, "T1");
        assert_eq!(
            blocked[0].findings,
            [SpecGateFinding {
                rule: "HF2",
                detail: "step 1: ends in `|| true`".to_string(),
            }]
        );

        let off = check_plans(
            &[vacuous],
            temp.path(),
            &config(SpecQualityMode::Off),
            &none,
        );
        assert!(!off.blocks());
        assert!(off.quality.is_none() && off.decisions.is_empty());

        let scoped = write_plan(temp.path(), "cargo test -p demo --lib retry");
        let report = check_plans(&[scoped.clone()], temp.path(), &advise, &none);
        assert!(!report.blocks(), "{report:?}");
        let decision = &report.decisions[0];
        assert!(decision.score < advise.allow_threshold, "{decision:?}");
        assert_eq!(decision.action, SpecGateAction::Advise);

        // Under enforce, a score below the block threshold blocks as well.
        let enforce = SpecQualityConfig {
            block_threshold: 70.0,
            ..config(SpecQualityMode::Enforce)
        };
        let report = check_plans(&[scoped], temp.path(), &enforce, &none);
        let blocked: Vec<&SpecGateDecision> = report.blocked().collect();
        assert_eq!(blocked.len(), 1, "{report:?}");
        assert_eq!(blocked[0].findings[0].rule, "score");
    }

    /// 3211: HF3 blocks only when the red-on-base check ran and every step
    /// passed on the base; the other hard fails are refused elsewhere.
    #[test]
    fn spec_gate_blocks_green_on_base_only_when_measured() {
        let temp = tempfile::tempdir().expect("tempdir");
        let plan = write_plan(temp.path(), "cargo test -p demo --lib retry");
        let advise = config(SpecQualityMode::Advise);
        let key = ("plans/demo/tasks.toml".to_string(), "T1".to_string());

        let green = BTreeMap::from([(key.clone(), RedOnBase::Pass)]);
        let report = check_plans(&[plan.clone()], temp.path(), &advise, &green);
        let blocked: Vec<&SpecGateDecision> = report.blocked().collect();
        assert_eq!(blocked.len(), 1, "{report:?}");
        assert_eq!(
            blocked[0].findings,
            [SpecGateFinding {
                rule: "HF3",
                detail: "verify passes on the unchanged base".to_string(),
            }]
        );

        let red = BTreeMap::from([(key, RedOnBase::Fail)]);
        assert!(!check_plans(&[plan], temp.path(), &advise, &red).blocks());

        // An implementer without a verify step (HF1) is plan validation's to
        // refuse, so the gate lets it through.
        let record = SpecQualityRecord {
            hard_fail: vec!["HF1", "HF4"],
            ..report.quality.expect("scored").tasks[0].clone()
        };
        assert_eq!(decide(&record, &advise).action, SpecGateAction::Advise);
    }
}
