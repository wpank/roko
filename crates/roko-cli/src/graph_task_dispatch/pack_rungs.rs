//! Verifier packs on the Graph path (9120): the workspace rungs a task faces
//! by its work domain, and how an attempt runs a rung by its kind.
//!
//! `[gates.packs.<domain>]` declares a domain's rungs (`GatesConfig::pack_for`).
//! A task with no domain faces `[[gates.rungs]]`, today's ladder, and so does
//! a `code` task unless a `code` pack is declared; a task of another domain
//! faces its pack, or only its own verify steps when it has none, so that it
//! ends unverified rather than run cargo on a brief. `[meta] workspace_rungs
//! = false` still opts a plan out of all of them.
//!
//! A `command` rung runs its command as a verify step. The other kinds are
//! not built yet (9122 citations, 9123 judge, 9124 schema, 9137 confirm, and
//! receipt with 9132's effects): an advisory or optional rung of such a kind
//! is skipped, and a task that must pass one fails before its agent runs.

use roko_core::TaskDomain;
use roko_core::config::GateRungConfig;

use super::*;
use crate::task_parser::VerifyStep;

impl GraphTaskDispatcher {
    /// The rungs of the pack `task`'s work domain picks, before
    /// `verification` keeps those an attempt runs. A task of a domain other
    /// than `code` with no pack faces none, which is logged once per plan
    /// and domain.
    pub(super) fn pack_rungs(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
    ) -> &[GateRungConfig] {
        let gates = &self.config.gates;
        let domain = task.effective_domain(self.config.project.default_domain.as_ref());
        let unpacked = domain.as_ref().filter(|domain| {
            !matches!(domain, TaskDomain::Code) && !gates.packs.contains_key(domain.label())
        });
        if let Some(domain) = unpacked {
            let key = (spec.plan_id.clone(), domain.label().to_string());
            if self.unpacked_domains.lock().insert(key) {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    domain = domain.label(),
                    "no [gates.packs] entry for this work domain: its tasks run only their own \
                     verify steps"
                );
            }
        }
        gates.pack_for(domain.as_ref())
    }

    /// Why an attempt at `task` is refused before its agent runs: it must
    /// pass a rung whose kind is not built yet ([`verify_step`]). The
    /// refusal is not retried; a resume runs the task again.
    pub(super) fn unbuilt_rung(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
    ) -> Option<RokoError> {
        let rung = self
            .task_rungs(spec, task)
            .find(|rung| !rung.kind.is_command())?;
        Some(RokoError::Rejected(format!(
            "task `{}` was not run: it must pass rung `{}`, and {}; mark the rung advisory \
             or optional, or take it out of its pack, until that kind is built",
            task.id,
            rung.name,
            not_built(rung)
        )))
    }
}

/// Why `rung`, of a kind other than `command`, cannot run yet.
fn not_built(rung: &GateRungConfig) -> String {
    format!("rung kind `{}` is not built yet", rung.kind)
}

/// `rung` as a verify step. A `command` rung runs its command. A rung of a
/// kind not built yet becomes a step that fails, saying so, so that no path
/// passes it unchecked; an attempt that faces one is refused before its
/// agent runs anyway ([`GraphTaskDispatcher::unbuilt_rung`]).
pub(super) fn verify_step(rung: &GateRungConfig) -> VerifyStep {
    let mut step = VerifyStep::from(rung);
    if !rung.kind.is_command() {
        let reason = not_built(rung);
        // A kind's label holds no quote, so the reason quotes as it is.
        step.command = format!("printf '%s\\n' '{reason}' >&2; exit 1");
        step.fail_msg = Some(reason);
    }
    step
}

/// Name each workspace rung's step among `steps`, a task's verify steps as
/// its prompt lists them: a comment after the step's command, which a shell
/// ignores, gives the rung's name and kind.
pub(super) fn name_rung_steps<'a>(
    steps: &mut [VerifyStep],
    rungs: impl IntoIterator<Item = &'a GateRungConfig>,
) {
    for rung in rungs {
        let rung_step = verify_step(rung);
        if let Some(step) = steps
            .iter_mut()
            .find(|step| step.phase == rung_step.phase && step.command == rung_step.command)
        {
            step.command = format!("{}  # rung `{}` ({})", step.command, rung.name, rung.kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_core::config::schema::{GatePackConfig, RungKind};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix,
    };

    /// A required `command` rung `name` that runs `command`.
    fn rung(name: &str, command: &str) -> GateRungConfig {
        GateRungConfig {
            name: name.to_string(),
            command: command.to_string(),
            timeout_secs: 10,
            ..GateRungConfig::default()
        }
    }

    /// 9120: a `research` task faces `[gates.packs.research]`, whose failing
    /// rung fails it, and not the ladder. A `code` task and a task with no
    /// domain run `[[gates.rungs]]` and not the research pack. A `docs` task,
    /// with no pack, runs only its own verify steps and ends unverified.
    #[tokio::test]
    async fn research_task_runs_the_research_pack() {
        let temp = tempdir().expect("tempdir");
        let ladder_ran = temp.path().join("ladder-ran");
        let ladder = rung("ladder", &format!("touch {}", ladder_ran.display()));
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![ladder];
                let research = GatePackConfig {
                    rungs: vec![rung("sources", "exit 3")],
                };
                config.gates.packs.insert("research".to_string(), research);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify.clear();

        task.domain = Some(TaskDomain::Research);
        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the research pack's failing rung fails the task");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("rung[sources] (`exit 3`)"), "{message}");
        assert!(!ladder_ran.exists(), "a research task skips the ladder");

        for domain in [Some(TaskDomain::Code), None] {
            task.domain = domain;
            let passed = dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
                .await
                .expect("the ladder passes and the research pack does not run");
            assert_eq!(
                TaskGateVerdict::from_signals(&passed),
                Some(TaskGateVerdict::Passed)
            );
            std::fs::remove_file(&ladder_ran).expect("the ladder ran");
        }

        task.domain = Some(TaskDomain::Docs);
        let unverified = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("a docs task without a pack runs only its own verify steps");
        assert_eq!(
            TaskGateVerdict::from_signals(&unverified),
            Some(TaskGateVerdict::Unverified)
        );
        assert!(!ladder_ran.exists(), "a docs task skips the ladder");
    }

    /// 9120: a task that must pass a rung of a kind not built yet is refused
    /// before its agent runs, and the rejection is not retried. An advisory
    /// rung of such a kind is skipped. The prompt names each rung and its
    /// kind.
    #[tokio::test]
    async fn a_required_rung_of_an_unbuilt_kind_fails_closed() {
        let temp = tempdir().expect("tempdir");
        let artefacts = vec!["report.md".to_string()];
        let judge = GateRungConfig {
            kind: RungKind::Judge,
            artefacts: artefacts.clone(),
            ..rung("rubric", "")
        };
        let citations = GateRungConfig {
            kind: RungKind::Citations,
            artefacts,
            ..rung("sources", "")
        };
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![rung("lint", "true")];
                for (label, only) in [("research", judge), ("legal", citations.clone())] {
                    let pack = GatePackConfig { rungs: vec![only] };
                    config.gates.packs.insert(label.to_string(), pack);
                }
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify.clear();

        task.domain = Some(TaskDomain::Research);
        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("an advisory judge rung is skipped");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Unverified)
        );

        task.domain = TaskDomain::from_label("legal");
        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("a required citations rung fails closed");
        assert!(matches!(error, RokoError::Rejected(_)), "{error}");
        let refusal = error.to_string();
        assert!(
            refusal.contains("rung kind `citations` is not built yet"),
            "{refusal}"
        );

        let step = verify_step(&citations);
        assert_eq!(
            step.command,
            "printf '%s\\n' 'rung kind `citations` is not built yet' >&2; exit 1"
        );

        task.domain = None;
        let prompt = dispatcher.prompt_task(&make_spec(&task), &task);
        assert_eq!(prompt.verify.len(), 1, "{:?}", prompt.verify);
        assert_eq!(prompt.verify[0].command, "true  # rung `lint` (command)");
    }
}
