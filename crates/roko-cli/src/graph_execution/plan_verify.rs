//! The whole-plan gate (gap-60233f): `[meta] verify`.
//!
//! A task's verify steps check that task. Once every task of a plan has
//! passed, the plan's `[meta] verify` steps check the plan's integrated
//! result: under `--worktree-per-task` the merge of its branch into the run's
//! batch branch, in the repository's regression checkout (the delivery's
//! regression check, see [`super::batch`]); otherwise the shared working tree
//! its tasks edited.
//! A plan whose tasks all passed but whose check fails does not succeed.
//! The check runs code the plan's agents wrote, so its steps get the
//! environment verify steps get, without provider keys (G09).
//!
//! A plan without `[meta] verify` in a Cargo workspace checks formatting,
//! lints and tests over the crates it affects: the ones its tasks write, and
//! every workspace crate that depends on them. That default never runs
//! `cargo test --workspace`. Other projects get no default.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use super::plan_check_events::PlanCheckEvents;
use super::plan_runner::PlanRunInterruptHandle;
use super::plan_set::{Area, PlanFootprint};
use crate::runner::plan_loader::Plan;
use crate::task_parser::VerifyStep;

/// The steps that check `plan`'s integrated result: its `[meta] verify`
/// steps when it has some, else [`default_plan_verify`] of its `footprint`,
/// which is `None` outside a Cargo workspace.
#[must_use]
pub fn plan_verify_steps(plan: &Plan, footprint: Option<&PlanFootprint>) -> Vec<VerifyStep> {
    if !plan.tasks.meta.verify.is_empty() {
        return plan.tasks.meta.verify.clone();
    }
    footprint.map(default_plan_verify).unwrap_or_default()
}

/// The default whole-plan check in a Cargo workspace: `cargo fmt --check`,
/// then clippy with `-D warnings`, then `cargo test`, each over the crates
/// the plan affects. When the package graph is unknown, formatting and lints
/// cover the workspace and no tests run. A plan that affects no crate gets
/// no steps.
#[must_use]
pub fn default_plan_verify(footprint: &PlanFootprint) -> Vec<VerifyStep> {
    let all_packages = footprint.affects.contains(&Area::AllPackages);
    let packages = footprint
        .affects
        .iter()
        .filter_map(|area| match area {
            Area::Package { name, .. } => Some(format!("-p {name}")),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    if all_packages {
        return vec![
            step("fmt", "cargo fmt --all -- --check"),
            step("lint", "cargo clippy --workspace --no-deps -- -D warnings"),
        ];
    }
    if packages.is_empty() {
        return Vec::new();
    }
    vec![
        step("fmt", format!("cargo fmt {packages} -- --check")),
        step(
            "lint",
            format!("cargo clippy {packages} --no-deps -- -D warnings"),
        ),
        step("test", format!("cargo test {packages}")),
    ]
}

fn step(phase: &str, command: impl Into<String>) -> VerifyStep {
    VerifyStep {
        phase: phase.to_string(),
        command: command.into(),
        fail_msg: None,
        timeout_ms: crate::task_parser::default_verify_timeout(),
        scope: Vec::new(),
        covers: Vec::new(),
        expect: None,
    }
}

/// Why a plan's check failed, or was stopped: the step, and the end of its
/// output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanVerifyFailure {
    /// The failed step's phase.
    pub phase: String,
    /// The failed step's command.
    pub command: String,
    /// The last lines the step wrote, or why it did not finish.
    pub output: String,
    /// What stopped the step before it ended, such as `cancel` or `SIGINT`,
    /// when the run was asked to stop (bug-7b62ff): the check did not fail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_by: Option<String>,
}

impl std::fmt::Display for PlanVerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ended = if self.stopped_by.is_some() {
            "stopped"
        } else {
            "failed"
        };
        write!(f, "`{}` {ended}: {}", self.command, self.output)
    }
}

/// Run `steps` in order in `dir` (`sh -c`), each within its timeout, and
/// stop at the first that fails. `events` publishes each step's start,
/// heartbeats and end (gap-6b6ec1).
///
/// Each step starts from the environment a task's verify steps get
/// ([`roko_gate::inherit_gate_env`]): roko's own, without provider keys,
/// other secrets or the person's GitHub login, plus the names
/// `env_passthrough` (`[gates] env_passthrough`) lists.
///
/// Each step leads a process group of its own. When `stop` asks the run to
/// stop, or the step runs out of time, the step is killed with everything it
/// started, such as cargo and its test binaries, and the check ends at once
/// (bug-7b62ff).
///
/// # Errors
///
/// The first step that exits unsuccessfully, cannot start (its environment
/// included), runs out of time, or is stopped (with
/// [`PlanVerifyFailure::stopped_by`] set).
pub async fn run_plan_verify(
    dir: &Path,
    steps: &[VerifyStep],
    env_passthrough: &[String],
    events: &PlanCheckEvents,
    stop: &PlanRunInterruptHandle,
) -> Result<(), PlanVerifyFailure> {
    let parent_env = roko_core::child_env::process_env();
    run_plan_verify_from(dir, steps, &parent_env, env_passthrough, events, stop).await
}

/// [`run_plan_verify`], with `parent_env` standing in for roko's own
/// environment. The gate policy filters it just the same.
async fn run_plan_verify_from(
    dir: &Path,
    steps: &[VerifyStep],
    parent_env: &[(String, OsString)],
    passthrough: &[String],
    events: &PlanCheckEvents,
    stop: &PlanRunInterruptHandle,
) -> Result<(), PlanVerifyFailure> {
    for (index, step) in steps.iter().enumerate() {
        let failure = |output: String| PlanVerifyFailure {
            phase: step.phase.clone(),
            command: step.command.clone(),
            output,
            stopped_by: None,
        };
        let mut command = tokio::process::Command::new("sh");
        command.arg("-c").arg(&step.command).current_dir(dir);
        let prepared =
            roko_gate::inherit_gate_env_from(&mut command, parent_env.iter().cloned(), passthrough);
        let watched = events.start(index, steps.len(), &step.command);
        if let Err(error) = prepared {
            let reason = roko_gate::check_env_error(&error);
            watched.finish(false, None, Some(reason.clone()));
            return Err(failure(reason));
        }
        let run = watched.watch(roko_gate::output_in_own_group(command));
        let timeout = Duration::from_millis(step.timeout_ms.max(1));
        // A stop drops the step, which kills its process group.
        let ran = tokio::select! {
            ran = tokio::time::timeout(timeout, run) => ran,
            interrupt = stop.stopped() => {
                let reason = watched.stopped(interrupt);
                return Err(PlanVerifyFailure {
                    stopped_by: Some(interrupt.label().to_string()),
                    ..failure(reason)
                });
            }
        };
        let output = match ran {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                let reason = format!("could not start: {error}");
                watched.finish(false, None, Some(reason.clone()));
                return Err(failure(reason));
            }
            Err(_) => {
                let reason = format!("timed out after {}s", timeout.as_secs());
                watched.finish(false, None, Some(reason.clone()));
                return Err(failure(reason));
            }
        };
        watched.finish(output.status.success(), output.status.code(), None);
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let text = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            let lines: Vec<&str> = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();
            let tail = lines[lines.len().saturating_sub(5)..].join(" | ");
            return Err(failure(match step.fail_msg.as_deref() {
                Some(message) => format!("{message}: {tail}"),
                None => tail,
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_step(command: &str) -> VerifyStep {
        step("plan", command)
    }

    /// gap-60233f: the steps run in order, and the first that fails stops
    /// the check and says why.
    #[tokio::test]
    async fn plan_verify_stops_at_the_first_failed_step() {
        let dir = tempfile::tempdir().expect("tempdir");
        let steps = [
            verify_step("touch first"),
            verify_step("echo 'crate b no longer builds' >&2; exit 1"),
            verify_step("touch never"),
        ];

        let none = PlanCheckEvents::none();
        let go = PlanRunInterruptHandle::default();
        let failure = run_plan_verify(dir.path(), &steps, &[], &none, &go)
            .await
            .expect_err("the second step fails");

        assert_eq!(failure.command, steps[1].command);
        assert!(
            failure.output.contains("crate b no longer builds"),
            "{failure}"
        );
        assert!(dir.path().join("first").exists());
        assert!(!dir.path().join("never").exists());
        assert!(
            run_plan_verify(dir.path(), &steps[..1], &[], &none, &go)
                .await
                .is_ok()
        );
    }

    /// gap-6b6ec1: each step of a plan's check publishes its start, a
    /// heartbeat while it runs, and its end; the step that fails is the last.
    #[tokio::test]
    async fn plan_check_publishes_start_heartbeat_and_finish() {
        use roko_core::DashboardEvent;

        use super::super::plan_check_events::CheckStage;

        let dir = tempfile::tempdir().expect("tempdir");
        let hub = crate::state_hub::StateHub::new(256);
        let mut published = hub.subscribe_events();
        let events = PlanCheckEvents::new(
            crate::runner::tui_bridge::TuiBridge::new(hub.sender()),
            "demo",
            CheckStage::InPlace,
            "cargo-default",
        )
        .with_heartbeat_every(Duration::from_millis(40));
        let steps = [
            verify_step("sleep 0.4"),
            verify_step("exit 3"),
            verify_step("touch never"),
        ];

        let go = PlanRunInterruptHandle::default();
        let failure = run_plan_verify(dir.path(), &steps, &[], &events, &go)
            .await
            .expect_err("the second step fails");

        assert_eq!(failure.command, "exit 3");
        let mut seen = Vec::new();
        while let Ok(envelope) = published.try_recv() {
            seen.push(envelope.payload);
        }
        let rows: Vec<String> = seen
            .iter()
            .map(|event| match event {
                DashboardEvent::PlanCheckStarted {
                    plan_id,
                    stage,
                    step,
                    steps,
                    name,
                    command,
                    scope,
                    source,
                    ..
                } => format!(
                    "started {plan_id} {stage} {step}/{steps} {name} {scope} {source}: {command}"
                ),
                DashboardEvent::PlanCheckHeartbeat {
                    step, elapsed_ms, ..
                } => {
                    assert!(*elapsed_ms > 0, "{event:?}");
                    format!("heartbeat {step}")
                }
                DashboardEvent::PlanCheckFinished {
                    step,
                    passed,
                    exit_code,
                    reason,
                    ..
                } => format!("finished {step} {passed} {exit_code:?} {reason:?}"),
                other => format!("{other:?}"),
            })
            .collect();
        let mut expected = vec!["started demo in_place 1/3 sleep command cargo-default: sleep 0.4"];
        let beats = rows.iter().filter(|row| *row == "heartbeat 1").count();
        assert!(beats >= 1, "{rows:#?}");
        expected.extend(std::iter::repeat_n("heartbeat 1", beats));
        expected.extend([
            "finished 1 true Some(0) None",
            "started demo in_place 2/3 exit command cargo-default: exit 3",
            "finished 2 false Some(3) None",
        ]);
        assert_eq!(rows, expected);
        assert!(!dir.path().join("never").exists());
    }

    /// G09: a step runs the agents' code with the environment verify steps
    /// get, so a provider key in roko's environment does not reach it, while
    /// a name in `[gates] env_passthrough` does.
    #[tokio::test]
    async fn plan_verify_step_sees_no_provider_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let parent_env = [
            ("PATH", path),
            ("HOME", OsString::from("/tmp/plan-verify-home")),
            ("OPENAI_API_KEY", OsString::from("sk-plan-verify-canary")),
            ("PLAN_VERIFY_PROBE", OsString::from("passed-through")),
        ]
        .map(|(name, value)| (name.to_string(), value));
        let steps = [verify_step("env > out.txt")];

        run_plan_verify_from(
            dir.path(),
            &steps,
            &parent_env,
            &["PLAN_VERIFY_PROBE".to_string()],
            &PlanCheckEvents::none(),
            &PlanRunInterruptHandle::default(),
        )
        .await
        .expect("the step runs");

        let env = std::fs::read_to_string(dir.path().join("out.txt")).expect("read out.txt");
        assert!(env.lines().any(|line| line.starts_with("PATH=")), "{env}");
        assert!(env.contains("PLAN_VERIFY_PROBE=passed-through"), "{env}");
        assert!(
            !env.contains("sk-plan-verify-canary"),
            "a provider key reached the plan check:\n{env}"
        );
    }

    /// bug-3c4600: a whole-plan check (`[[meta.verify]]`, or the default
    /// `cargo fmt`/`clippy`/`test` of the packages a plan writes) runs
    /// without GitHub tokens, the person's `gh` login or a key roko loaded,
    /// so a test that would publish with `gh` finds no login.
    #[cfg(unix)]
    #[test]
    fn plan_checks_get_the_check_environment() {
        crate::check_env_probe::run_child(
            "graph_execution::plan_verify::tests::plan_check_env_child",
        );
    }

    /// Run by [`plan_checks_get_the_check_environment`] only.
    #[cfg(unix)]
    #[tokio::test]
    async fn plan_check_env_child() {
        use crate::check_env_probe::{PASSED, assert_check_env, begin_child, dump_env_command};
        let Some(_run) = begin_child() else {
            return;
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("check.env");
        let steps = [verify_step(&dump_env_command(&out))];

        run_plan_verify(
            dir.path(),
            &steps,
            &[PASSED.to_string()],
            &PlanCheckEvents::none(),
            &PlanRunInterruptHandle::default(),
        )
        .await
        .expect("the check passes");

        assert_check_env(&out, "a whole-plan check");
    }

    /// Tells [`plan_check_fail_closed_child`] where its step would leave a
    /// mark.
    #[cfg(unix)]
    const FAIL_CLOSED_CHILD: &str = "ROKO_PLAN_CHECK_FAIL_CLOSED_CHILD";

    /// bug-3c4600: a whole-plan check whose environment cannot be set up
    /// fails, saying why, and its step does not run.
    #[cfg(unix)]
    #[test]
    fn a_plan_check_without_its_environment_does_not_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker = dir.path().join("ran");
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "graph_execution::plan_verify::tests::plan_check_fail_closed_child",
                "--nocapture",
            ])
            .env(FAIL_CLOSED_CHILD, &marker)
            .env("TMPDIR", dir.path().join("gone"))
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );
        assert!(!marker.exists(), "the step ran");
    }

    /// Run by [`a_plan_check_without_its_environment_does_not_run`] only.
    #[cfg(unix)]
    #[tokio::test]
    async fn plan_check_fail_closed_child() {
        let Some(marker) = std::env::var_os(FAIL_CLOSED_CHILD) else {
            return;
        };
        let dir = std::path::Path::new(&marker)
            .parent()
            .expect("the marker's directory");
        let command = format!("touch '{}'", std::path::Path::new(&marker).display());

        let steps = [verify_step(&command)];
        let go = PlanRunInterruptHandle::default();
        let failure = run_plan_verify(dir, &steps, &[], &PlanCheckEvents::none(), &go)
            .await
            .expect_err("no check environment, no check");

        assert!(
            failure.output.contains("the check did not run"),
            "{failure}"
        );
    }

    /// bug-7b62ff: a stop request ends a whole-plan check at once. The
    /// running step dies with everything it started (its process group), the
    /// failure says what stopped it, and the steps after it do not run.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_ends_the_plan_check_and_kills_its_steps() {
        use roko_core::DashboardEvent;

        use super::super::plan_check_events::CheckStage;
        use crate::check_process_probe::{sleeping_step, wait_for_exit, wait_for_pids};
        use crate::graph_execution::plan_runner::PlanRunInterrupt;

        let dir = tempfile::tempdir().expect("tempdir");
        let pids = dir.path().join("pids");
        let steps = [
            verify_step(&sleeping_step(&pids)),
            verify_step("touch never"),
        ];
        let hub = crate::state_hub::StateHub::new(64);
        let mut published = hub.subscribe_events();
        let events = PlanCheckEvents::new(
            crate::runner::tui_bridge::TuiBridge::new(hub.sender()),
            "demo",
            CheckStage::InPlace,
            "authored",
        );
        let stop = PlanRunInterruptHandle::default();
        let check = tokio::spawn({
            let dir = dir.path().to_path_buf();
            let stop = stop.clone();
            async move { run_plan_verify(&dir, &steps, &[], &events, &stop).await }
        });
        let running = wait_for_pids(&pids).await;

        stop.request(PlanRunInterrupt::Cancel);
        let failure = tokio::time::timeout(Duration::from_secs(10), check)
            .await
            .expect("the stopped check ends at once")
            .expect("the check task")
            .expect_err("a stopped check does not pass");

        assert_eq!(failure.stopped_by.as_deref(), Some("cancel"), "{failure}");
        assert!(
            failure
                .to_string()
                .contains("stopped: a cancel request stopped it"),
            "{failure}"
        );
        for pid in running {
            wait_for_exit(pid).await;
        }
        assert!(!dir.path().join("never").exists());
        // The step's end says it was stopped, not that it failed.
        let mut finished = Vec::new();
        while let Ok(envelope) = published.try_recv() {
            if let DashboardEvent::PlanCheckFinished {
                passed, stopped_by, ..
            } = envelope.payload
            {
                finished.push((passed, stopped_by));
            }
        }
        assert_eq!(finished, [(false, Some("cancel".to_string()))]);
    }
}
