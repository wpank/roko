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
//! A `command` rung runs its command as a verify step. A `citations` rung
//! (9122) checks the citations in its artefacts once the attempt's verify
//! steps pass ([`GraphTaskDispatcher::check_kind_rungs`]). The other kinds
//! are not built yet (9123 judge, 9124 schema, 9137 confirm, and receipt
//! with 9132's effects): an advisory or optional rung of such a kind is
//! skipped, and a task that must pass one fails before its agent runs.

use roko_core::config::GateRungConfig;
use roko_core::config::schema::RungKind;
use roko_core::{TaskDomain, Verdict};
use roko_learn::telemetry::VerifyStepVerdict;

use super::verification::{published_gate_output, rung_step_label};
use super::*;
use crate::task_parser::VerifyStep;

/// The most artefact files a rung reads.
const MAX_ARTEFACT_FILES: usize = 50;

/// The largest artefact file a rung reads, in bytes.
const MAX_ARTEFACT_BYTES: u64 = 1 << 20;

/// Whether rungs of `kind` have a check: `command` rungs run as verify
/// steps, and `citations` rungs through `roko_gate`'s citation check (9122).
fn is_built(kind: RungKind) -> bool {
    matches!(kind, RungKind::Command | RungKind::Citations)
}

/// What an attempt's rungs of kinds other than `command` found
/// ([`GraphTaskDispatcher::check_kind_rungs`]).
#[derive(Default)]
pub(super) struct KindRungs {
    /// A line for each rung that must pass and failed, as a failed verify
    /// step's line reads.
    pub(super) failures: Vec<String>,
    /// Whether a rung that must pass could not run, so that the attempt is
    /// not verified.
    pub(super) skipped: bool,
}

impl GraphTaskDispatcher {
    /// The rungs of the pack `task`'s work domain picks, before
    /// `verification` keeps those an attempt runs. A task of a domain other
    /// than `code` with no pack faces none, which is logged once per plan
    /// and domain.
    pub(super) fn pack_rungs(&self, spec: &TaskExecutionSpec, task: &TaskDef) -> &[GateRungConfig] {
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
            .find(|rung| !is_built(rung.kind))?;
        Some(RokoError::Rejected(format!(
            "task `{}` was not run: it must pass rung `{}`, and {}; mark the rung advisory \
             or optional, or take it out of its pack, until that kind is built",
            task.id,
            rung.name,
            not_built(rung)
        )))
    }

    /// Look citations up through `resolver` in place of the live one.
    #[must_use]
    pub fn with_citation_resolver(
        mut self,
        resolver: Arc<dyn roko_gate::CitationResolver>,
    ) -> Self {
        self.citation_resolver = resolver;
        self
    }

    /// The rungs of kinds other than `command` that an attempt at `task`
    /// faces and that have a check: [`Self::check_kind_rungs`] runs them.
    pub(super) fn kind_rungs(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
    ) -> Vec<GateRungConfig> {
        self.task_rungs(spec, task)
            .filter(|rung| !rung.kind.is_command() && is_built(rung.kind))
            .cloned()
            .collect()
    }

    /// Run `rungs` ([`Self::kind_rungs`]) over the work an attempt at `task`
    /// left in `workdir`, once its verify steps passed, and record each in
    /// `step_verdicts` and on the dashboard, its detail included.
    ///
    /// A `citations` rung looks up every citation in the files its
    /// `artefacts` match (`roko_gate::check_citations`); it fails when none
    /// matches. A rung that fails gives a line for the attempt's failure, and
    /// one that could not run (a lookup it could not make) leaves the attempt
    /// unverified, never passed.
    pub(super) async fn check_kind_rungs(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        rungs: &[GateRungConfig],
        workdir: &Path,
        step_verdicts: &mut Vec<VerifyStepVerdict>,
    ) -> KindRungs {
        let mut checked = KindRungs::default();
        for rung in rungs {
            let label = rung_step_label(&rung.name);
            let shown = format!("{} {}", rung.kind, rung.artefacts.join(" "));
            let started = Instant::now();
            let verdict = match rung.kind {
                RungKind::Citations => {
                    let artefacts = read_artefacts(workdir, &rung.artefacts);
                    if artefacts.is_empty() {
                        let globs = rung.artefacts.join(", ");
                        Verdict::fail(&label, format!("no artefact matches {globs}"))
                    } else {
                        let resolver = self.citation_resolver.as_ref();
                        roko_gate::check_citations(&label, &artefacts, resolver).await
                    }
                }
                // A rung of a kind not built yet refused the attempt before
                // its agent ran (`unbuilt_rung`).
                _ => continue,
            };
            let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let verdict = verdict.with_duration(elapsed);
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = %label,
                passed = verdict.passed,
                skipped = verdict.skipped,
                reason = %verdict.reason,
                "graph verify rung settled"
            );
            if let Some(tui) = &self.tui_bridge {
                let output = published_gate_output(&shown, &verdict);
                tui.gate_result_with_output(
                    &spec.plan_id,
                    &task.id,
                    &label,
                    verdict.passed,
                    Some(&output),
                );
            }
            step_verdicts.push(VerifyStepVerdict {
                rung: label.clone(),
                passed: (!verdict.skipped).then_some(verdict.passed),
                duration_ms: Some(verdict.duration_ms),
                skipped: verdict.skipped,
                skip_reason: verdict.skip_reason.clone(),
                ..VerifyStepVerdict::default()
            });
            if verdict.skipped {
                checked.skipped = true;
            } else if !verdict.passed {
                let detail = verdict.detail.as_deref().unwrap_or_default();
                checked
                    .failures
                    .push(format!("{label} (`{shown}`): {}\n{detail}", verdict.reason));
            }
        }
        checked
    }
}

/// The files `globs` match in `workdir`, each as its path in `workdir` and
/// its text: at most [`MAX_ARTEFACT_FILES`] files of at most
/// [`MAX_ARTEFACT_BYTES`] each, all inside `workdir`. A glob that is
/// absolute or climbs out with `..` matches nothing.
fn read_artefacts(workdir: &Path, globs: &[String]) -> Vec<(String, String)> {
    let Ok(root) = workdir.canonicalize() else {
        return Vec::new();
    };
    let mut artefacts: Vec<(String, String)> = Vec::new();
    for pattern in globs {
        let relative = Path::new(pattern.trim());
        let climbs = relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir));
        if relative.is_absolute() || climbs {
            continue;
        }
        let root_pattern = glob::Pattern::escape(&root.to_string_lossy());
        let full = format!("{root_pattern}/{}", relative.to_string_lossy());
        let Ok(paths) = glob::glob(&full) else {
            continue;
        };
        for path in paths.flatten() {
            // The file itself, not a link out of the workspace.
            let Ok(path) = path.canonicalize() else {
                continue;
            };
            let Ok(shown) = path.strip_prefix(&root) else {
                continue;
            };
            let shown = shown.to_string_lossy().into_owned();
            let small = std::fs::metadata(&path)
                .is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_ARTEFACT_BYTES);
            if !small || artefacts.iter().any(|(seen, _)| *seen == shown) {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                artefacts.push((shown, text));
            }
            if artefacts.len() == MAX_ARTEFACT_FILES {
                return artefacts;
            }
        }
    }
    artefacts
}

/// Why `rung`, of a kind other than `command`, cannot run yet.
fn not_built(rung: &GateRungConfig) -> String {
    format!("rung kind `{}` is not built yet", rung.kind)
}

/// `rung` as a verify step: a `command` rung runs its command. A rung of a
/// kind not built yet becomes a step that fails, saying so, so that no path
/// passes it unchecked; an attempt that faces one is refused before its
/// agent runs anyway ([`GraphTaskDispatcher::unbuilt_rung`]). A rung of
/// another built kind is no verify step, and gives `None`:
/// [`GraphTaskDispatcher::check_kind_rungs`] runs it.
pub(super) fn verify_step(rung: &GateRungConfig) -> Option<VerifyStep> {
    let mut step = VerifyStep::from(rung);
    if is_built(rung.kind) {
        return rung.kind.is_command().then_some(step);
    }
    let reason = not_built(rung);
    // A kind's label holds no quote, so the reason quotes as it is.
    step.command = format!("printf '%s\\n' '{reason}' >&2; exit 1");
    step.fail_msg = Some(reason);
    Some(step)
}

/// Name each workspace rung among `steps`, a task's verify steps as its
/// prompt lists them. A comment after a rung's step command, which a shell
/// ignores, gives the rung's name and kind; a rung of another built kind,
/// which is no verify step, is listed as a comment that says what it checks.
pub(super) fn name_rung_steps<'a>(
    steps: &mut Vec<VerifyStep>,
    rungs: impl IntoIterator<Item = &'a GateRungConfig>,
) {
    for rung in rungs {
        let Some(rung_step) = verify_step(rung) else {
            let mut listed = VerifyStep::from(rung);
            listed.command = format!("# rung `{}` ({}): {}", rung.name, rung.kind, checks(rung));
            steps.push(listed);
            continue;
        };
        if let Some(step) = steps
            .iter_mut()
            .find(|step| step.phase == rung_step.phase && step.command == rung_step.command)
        {
            step.command = format!("{}  # rung `{}` ({})", step.command, rung.name, rung.kind);
        }
    }
}

/// What a rung of a kind other than `command` checks, as the prompt says it.
fn checks(rung: &GateRungConfig) -> String {
    let artefacts = rung.artefacts.join(", ");
    match rung.kind {
        RungKind::Citations => {
            format!("every DOI, arXiv id and URL that {artefacts} cites must resolve")
        }
        _ => format!("it checks {artefacts}"),
    }
}

#[cfg(test)]
mod tests {
    use roko_core::config::schema::{GatePackConfig, RungKind};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, make_test_dispatcher_with, no_auto_fix,
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
        let judge = GateRungConfig {
            kind: RungKind::Judge,
            artefacts: vec!["report.md".to_string()],
            ..rung("rubric", "")
        };
        let confirm = GateRungConfig {
            kind: RungKind::Confirm,
            ..rung("sign-off", "")
        };
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![rung("lint", "true")];
                for (label, only) in [("research", judge), ("legal", confirm.clone())] {
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
            .expect_err("a required confirm rung fails closed");
        assert!(matches!(error, RokoError::Rejected(_)), "{error}");
        let refusal = error.to_string();
        assert!(
            refusal.contains("rung kind `confirm` is not built yet"),
            "{refusal}"
        );

        let step = verify_step(&confirm).expect("a step that fails");
        assert_eq!(
            step.command,
            "printf '%s\\n' 'rung kind `confirm` is not built yet' >&2; exit 1"
        );

        task.domain = None;
        let prompt = dispatcher.prompt_task(&make_spec(&task), &task);
        assert_eq!(prompt.verify.len(), 1, "{:?}", prompt.verify);
        assert_eq!(prompt.verify[0].command, "true  # rung `lint` (command)");
    }

    /// Knows the citations labelled `known`, or with `known` `None` cannot
    /// be reached.
    struct FakeResolver {
        known: Option<Vec<&'static str>>,
    }

    #[async_trait::async_trait]
    impl roko_gate::CitationResolver for FakeResolver {
        async fn resolve(
            &self,
            citation: &roko_gate::Citation,
        ) -> std::result::Result<roko_gate::Resolution, String> {
            let Some(known) = &self.known else {
                return Err("network unreachable".to_string());
            };
            let label = citation.to_string();
            Ok(if known.contains(&label.as_str()) {
                roko_gate::Resolution::Resolved {
                    source: "fake".to_string(),
                }
            } else {
                roko_gate::Resolution::Unresolved {
                    detail: "fake 404".to_string(),
                }
            })
        }
    }

    /// A `research` pack whose one rung checks the citations in `report.md`.
    fn citations_pack(config: &mut RokoConfig) {
        no_auto_fix(config);
        let sources = GateRungConfig {
            kind: RungKind::Citations,
            artefacts: vec!["report.md".to_string()],
            ..rung("sources", "")
        };
        let research = GatePackConfig {
            rungs: vec![sources],
        };
        config.gates.packs.insert("research".to_string(), research);
    }

    /// 9122: a `citations` rung looks up every citation in the files its
    /// artefacts match, once the steps pass. One that does not resolve fails
    /// the attempt, which names it; when every one resolves the attempt
    /// passes; when they cannot be looked up it ends unverified. The prompt
    /// says what the rung checks.
    #[tokio::test]
    async fn citations_rung_checks_what_the_artefacts_cite() {
        let temp = tempdir().expect("tempdir");
        let report = temp.path().join("report.md");
        std::fs::write(&report, "See doi:10.1000/real and doi:10.1000/made-up.\n").expect("report");
        let resolver = Arc::new(FakeResolver {
            known: Some(vec!["doi:10.1000/real"]),
        });
        let (dispatcher, mut task) = make_test_dispatcher_with(
            &temp,
            VERIFY_PROVIDER,
            citations_pack,
            GraphFeedbackContext::default(),
            |dispatcher| dispatcher.with_citation_resolver(resolver),
        )
        .await;
        task.verify.clear();
        task.domain = Some(TaskDomain::Research);

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("a made-up DOI fails the attempt");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(
            message.contains("1 of 2 citations do not resolve: doi:10.1000/made-up"),
            "{message}"
        );

        std::fs::write(&report, "See doi:10.1000/real.\n").expect("report");
        let passed = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("every citation resolves");
        assert_eq!(
            TaskGateVerdict::from_signals(&passed),
            Some(TaskGateVerdict::Passed)
        );
        let prompt = dispatcher.prompt_task(&make_spec(&task), &task);
        assert_eq!(prompt.verify.len(), 1, "{:?}", prompt.verify);
        assert!(prompt.verify[0].command.starts_with("# rung `sources` (citations): every DOI"));

        let offline = Arc::new(FakeResolver { known: None });
        let (dispatcher, _) = make_test_dispatcher_with(
            &temp,
            VERIFY_PROVIDER,
            citations_pack,
            GraphFeedbackContext::default(),
            |dispatcher| dispatcher.with_citation_resolver(offline),
        )
        .await;
        let unverified = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("citations that cannot be looked up do not fail the attempt");
        assert_eq!(
            TaskGateVerdict::from_signals(&unverified),
            Some(TaskGateVerdict::Unverified)
        );
    }
}
