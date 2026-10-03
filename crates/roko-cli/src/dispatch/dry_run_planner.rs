//! The canary's dry-run planner over [`Dispatcher::plan`] (S03 §4.7; backlog 5128).
//!
//! P4 asks whether dispatch would carry the canary. [`DispatchPlanner`] plans
//! the canary task as dispatch would, routing it and assembling its prompt
//! without a provider call, and reports whether the prompt holds the canary's
//! `CANARY-<nonce>` artifact, and which model the route picked and why; the
//! canary's target judges them. The task's own text holds only the lowercase
//! category, so a prompt holds the artifact only when a reader put it there.
//!
//! `plan()` writes no learned state: it reads the stores, routes and
//! assembles. P5, a capped live call, needs a provider and is not offered
//! here: `execute_capped` keeps the trait's `None`, so the driver skips it and
//! every canary runs dry.

use std::path::{Path, PathBuf};

use roko_learn::loop_audit::canary::{CanaryTask, DryRunPlanner, PlanProbe};

use super::{DispatchContext, Dispatcher, ModelChoiceSource};
use crate::loop_canary::{canary_id, canary_task};
use crate::task_parser::TaskDef;

/// The plan id a canary's dispatch context names.
const CANARY_PLAN: &str = "loop-canary";

/// Plans canary tasks through a production [`Dispatcher`] in the workspace
/// whose stores the canary writers wrote to.
pub struct DispatchPlanner<'a> {
    dispatcher: &'a Dispatcher,
    workdir: PathBuf,
}

impl<'a> DispatchPlanner<'a> {
    /// A planner over `dispatcher` in the workspace `workdir`.
    #[must_use]
    pub fn new(dispatcher: &'a Dispatcher, workdir: &Path) -> Self {
        Self {
            dispatcher,
            workdir: workdir.to_path_buf(),
        }
    }
}

impl DryRunPlanner for DispatchPlanner<'_> {
    /// Plan the canary task: whether the assembled prompt holds the
    /// artifact, and the routed model with the source of its choice. The
    /// plan is dry whatever `dry_run` says: no provider is called.
    fn plan(&mut self, task: &CanaryTask, _dry_run: bool) -> Result<PlanProbe, String> {
        let canary = canary_task(&task.nonce);
        let ctx = canary_context(&canary, &self.workdir);
        let plan = self
            .dispatcher
            .plan(&canary, &ctx)
            .map_err(|error| format!("dispatch could not plan the canary: {error}"))?;
        let artifact = canary_id(&task.nonce);
        let prompt = &plan.prompt;
        let prompt_contains =
            prompt.system_prompt.contains(&artifact) || prompt.user_prompt.contains(&artifact);
        Ok(PlanProbe {
            prompt_contains,
            model: plan.model.slug.clone(),
            source: source_label(plan.source),
        })
    }
}

/// The dispatch context of the canary task `task` in the workspace
/// `workdir`: its role, no budget pressure, no experiment, no arms.
fn canary_context(task: &TaskDef, workdir: &Path) -> DispatchContext {
    DispatchContext {
        plan_id: CANARY_PLAN.to_string(),
        role: task.role.clone().unwrap_or_else(|| "implementer".to_string()),
        workdir: workdir.to_path_buf(),
        model_hint: None,
        force_backend: None,
        budget_remaining_usd: f64::MAX,
        attempt: 0,
        ladder_step: 0,
        prompt_experiment: None,
        gate_feedback: None,
        routing_context: None,
        dependency_outputs: Vec::new(),
        error_patterns_context: String::new(),
        cached_workspace_map: String::new(),
        cached_workspace_context: String::new(),
        concurrent_plans: Vec::new(),
        attempt_key: None,
        arm_set: None,
    }
}

/// The route decision row's label for a choice of kind `source`, e.g.
/// `router` or `task_hint`.
fn source_label(source: ModelChoiceSource) -> String {
    serde_json::to_value(source.decision_source())
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use roko_learn::loop_audit::canary::{CanaryTarget, CanaryWriter, trace};

    use super::*;
    use crate::dispatch::prompt_builder::PromptAssembler;
    use crate::dispatch::warm_pool::WarmPool;
    use crate::loop_canary::{KnowledgeCanary, canary_category};

    /// The canary's nonce.
    const NONCE: &str = "c-dry1";

    /// The tests that read L-know through the dispatcher take turns: fault
    /// flags are process-wide.
    static KNOWLEDGE_READS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A dispatcher with the production prompt sources and no router.
    fn dispatcher() -> Dispatcher {
        Dispatcher::new(
            None,
            PromptAssembler::new(),
            WarmPool::new(0),
            HashSet::new(),
        )
    }

    /// Every file under `dir`, by path, with its bytes.
    fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(path) = pending.pop() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                pending.extend(entries.map(|entry| entry.expect("directory entry").path()));
            } else if let Ok(bytes) = std::fs::read(&path) {
                files.insert(path, bytes);
            }
        }
        files
    }

    /// S03 §4.7 (backlog 5128): with L-know's canary written, a dry-run plan
    /// of the canary task assembles a prompt that holds the artifact and
    /// changes nothing under `.roko/learn`. Through the driver, P1-P4 pass
    /// and the trace stops at P6, since a dry run writes no decision row;
    /// P5 is skipped.
    #[test]
    fn dry_run_canary_reaches_p4_without_learned_writes() {
        let _turn = KNOWLEDGE_READS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().expect("temp dir");
        let learn = dir.path().join(".roko/learn");
        std::fs::create_dir_all(&learn).expect("the learn dir");
        std::fs::write(learn.join("efficiency.jsonl"), "").expect("a learn file");
        let dispatcher = dispatcher();
        let mut planner = DispatchPlanner::new(&dispatcher, dir.path());
        let mut writer = KnowledgeCanary::new(dir.path());
        let task = CanaryTask {
            loop_id: "L-know".to_string(),
            nonce: NONCE.to_string(),
            category: canary_category(NONCE),
            target: CanaryTarget::Prompt,
        };

        // The dry run alone writes no learned state.
        writer.write(NONCE).expect("write the canary");
        let before = files(&learn);
        let probe = planner.plan(&task, true).expect("a dry-run plan");
        assert!(probe.prompt_contains, "{probe:?}");
        let routed = !probe.model.is_empty() && !probe.source.is_empty();
        assert!(routed, "{probe:?}");
        assert_eq!(files(&learn), before, "a dry-run plan wrote learned state");
        writer.cleanup(NONCE);

        // Without the artifact the prompt lacks it: the task's own text does
        // not count.
        let probe = planner.plan(&task, true).expect("a dry-run plan");
        assert!(!probe.prompt_contains, "{probe:?}");

        let run_dir = dir.path().join(".roko/runs/gr-dry");
        let row = trace(&mut writer, &mut planner, &run_dir, &task, true);
        let probes: Vec<(&str, bool)> = row
            .probes
            .iter()
            .map(|probe| (probe.p.as_str(), probe.ok))
            .collect();
        let expected = [
            ("P1", true),
            ("P2", true),
            ("P3", true),
            ("P4", true),
            ("P6", false),
        ];
        assert_eq!(probes, expected, "{row:?}");
        assert_eq!(row.first_failure.as_deref(), Some("P6"));
        assert_eq!(files(&learn), before, "the trace left learned state behind");
    }
    /// S03 §4.9 (backlog 5129): a CUT flag on L-know empties the knowledge
    /// reader, so a dry-run plan of the canary task carries no knowledge, and
    /// clearing the flag restores it.
    #[cfg(feature = "fault-injection")]
    #[test]
    fn injected_cut_empties_knowledge_section() {
        use roko_learn::loop_audit::faults::{self, FaultActor, FaultKind, FaultSpec};

        let _turn = KNOWLEDGE_READS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().expect("temp dir");
        let dispatcher = dispatcher();
        let mut writer = KnowledgeCanary::new(dir.path());
        writer.write(NONCE).expect("write the canary");
        let task = canary_task(NONCE);
        let ctx = canary_context(&task, dir.path());
        let artifact = canary_id(NONCE);
        let plan = dispatcher.plan(&task, &ctx).expect("a dry-run plan");
        assert_eq!(plan.prompt.diagnostics.knowledge_ids, [artifact.clone()]);

        faults::enable(FaultActor::Env, dir.path().join("faults.jsonl"));
        let cut = FaultSpec {
            loop_id: "L-know".to_string(),
            kind: FaultKind::Cut,
            ttl_secs: 60,
            max_decisions: 10,
            spend_cap_usd: None,
        };
        faults::set(cut).expect("set a CUT flag");
        let plan = dispatcher.plan(&task, &ctx).expect("a dry-run plan");
        assert!(plan.prompt.diagnostics.knowledge_ids.is_empty());
        assert!(!plan.prompt.system_prompt.contains(&artifact));

        assert!(faults::clear("L-know"), "the flag was set");
        let plan = dispatcher.plan(&task, &ctx).expect("a dry-run plan");
        assert_eq!(plan.prompt.diagnostics.knowledge_ids, [artifact.clone()]);
        assert!(plan.prompt.system_prompt.contains(&artifact));
        faults::disable();
        writer.cleanup(NONCE);
    }
}
