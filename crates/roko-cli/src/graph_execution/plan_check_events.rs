//! Hub events of a plan's whole-plan check (gap-6b6ec1).
//!
//! Once a plan's tasks all passed, its whole-plan check runs (gap-60233f):
//! its `[meta] verify`, or the default check of a Cargo workspace. It runs in
//! the working tree the tasks edited, or under `--worktree-per-task` as the
//! delivery's post-merge regression check in the regression checkout. One
//! step can build a large workspace for tens of minutes, so each step
//! publishes `plan_check_started`, a `plan_check_heartbeat` every
//! [`HEARTBEAT_EVERY`] while it runs, and `plan_check_finished`. A host
//! following `GET /api/events` or the run's events index can then say what
//! the run is doing ("checking the workspace: cargo clippy, 3 min") instead
//! of "between tasks".

use std::future::Future;
use std::time::{Duration, Instant};

use roko_core::DashboardEvent;

use crate::runner::tui_bridge::TuiBridge;

/// How often a running step's heartbeat is published.
pub const HEARTBEAT_EVERY: Duration = Duration::from_secs(15);

/// Where a plan's check runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStage {
    /// In the working tree the plan's tasks edited.
    InPlace,
    /// As the delivery's post-merge regression check, in the regression
    /// checkout.
    Regression,
}

impl CheckStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::InPlace => "in_place",
            Self::Regression => "regression",
        }
    }
}

/// What a check step builds, as its events name it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckScope {
    /// What the step runs, in a word or two: `cargo clippy`, or the program
    /// it starts.
    pub name: String,
    /// `workspace`, `packages` or `command`.
    pub scope: &'static str,
    /// The packages of a `packages` step.
    pub packages: Vec<String>,
}

/// What `command` builds, from what it says: a cargo invocation with
/// `--workspace` or `--all`, or with no `-p` outside a `cd`, builds the
/// `workspace`; one naming `-p` packages builds those `packages`; anything
/// else is a `command`. The footprint of a plan
/// ([`super::plan_set::PlanFootprint`]) resolves the same flags against the
/// package graph; an event only repeats them.
#[must_use]
pub fn check_scope(command: &str) -> CheckScope {
    let mut name = None;
    let mut workspace = false;
    let mut packages = Vec::new();
    let mut moved = false;
    for segment in command.split(['&', '|', ';', '\n']) {
        let mut words = command_words(segment);
        let Some(program) = words.next() else {
            continue;
        };
        if program == "cd" {
            moved = true;
            continue;
        }
        let program = program.rsplit('/').next().unwrap_or(program);
        if program != "cargo" {
            name.get_or_insert_with(|| program.to_string());
            continue;
        }
        let cargo = CargoArgs::read(words);
        name.get_or_insert_with(|| format!("cargo {}", cargo.subcommand).trim_end().to_string());
        workspace |= cargo.workspace
            || (cargo.packages.is_empty()
                && !moved
                && super::plan_set::cargo_builds(cargo.subcommand));
        packages.extend(cargo.packages);
    }
    packages.sort();
    packages.dedup();
    let scope = if workspace {
        packages.clear();
        "workspace"
    } else if packages.is_empty() {
        "command"
    } else {
        "packages"
    };
    CheckScope {
        name: name.unwrap_or_default(),
        scope,
        packages,
    }
}

/// The words of one shell segment from its program on: the `env`, `time`,
/// `nice`, `exec`, `command`, `nohup`, `timeout <secs>` and `NAME=value`
/// words before it are skipped.
fn command_words(segment: &str) -> impl Iterator<Item = &str> {
    let mut words = segment
        .split_whitespace()
        .map(|word| word.trim_start_matches(['(', '{']).trim_end_matches(')'))
        .filter(|word| !word.is_empty())
        .peekable();
    while let Some(word) = words.peek() {
        if *word == "timeout" {
            words.next();
            words.next_if(|word| word.starts_with(|c: char| c.is_ascii_digit()));
        } else if matches!(
            *word,
            "env" | "time" | "nice" | "exec" | "command" | "nohup"
        ) || (word.contains('=') && !word.starts_with('-'))
        {
            words.next();
        } else {
            break;
        }
    }
    words
}

/// What one cargo invocation names.
struct CargoArgs<'a> {
    /// `check`, `clippy`, …; empty when there is none.
    subcommand: &'a str,
    /// `--workspace` or `--all`.
    workspace: bool,
    /// Its `-p` / `--package` packages.
    packages: Vec<String>,
}

impl<'a> CargoArgs<'a> {
    /// Read the arguments after `cargo`, up to `--`.
    fn read(args: impl Iterator<Item = &'a str>) -> Self {
        let mut args = args.filter(|arg| !arg.starts_with('+'));
        let mut read = Self {
            subcommand: args.next().unwrap_or_default(),
            workspace: false,
            packages: Vec::new(),
        };
        while let Some(arg) = args.next() {
            match arg {
                "--" => break,
                "--workspace" | "--all" => read.workspace = true,
                "-p" | "--package" => read.packages.extend(args.next().map(str::to_string)),
                _ => read.packages.extend(
                    arg.strip_prefix("--package=")
                        .or_else(|| arg.strip_prefix("-p").filter(|name| !name.is_empty()))
                        .map(str::to_string),
                ),
            }
        }
        read
    }
}

/// Publishes the events of one plan's check. Without a hub
/// ([`Self::none`]) it publishes nothing.
#[derive(Clone)]
pub struct PlanCheckEvents {
    tui: Option<TuiBridge>,
    plan_id: String,
    stage: CheckStage,
    /// `authored` or `cargo-default`.
    source: String,
    heartbeat_every: Duration,
}

impl std::fmt::Debug for PlanCheckEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanCheckEvents")
            .field("publishes", &self.tui.is_some())
            .field("plan_id", &self.plan_id)
            .field("stage", &self.stage)
            .finish_non_exhaustive()
    }
}

impl PlanCheckEvents {
    /// The events of `plan_id`'s check at `stage`, from `source` (`authored`
    /// or `cargo-default`), published through `tui`.
    #[must_use]
    pub fn new(tui: TuiBridge, plan_id: &str, stage: CheckStage, source: &str) -> Self {
        Self {
            tui: Some(tui),
            plan_id: plan_id.to_string(),
            stage,
            source: source.to_string(),
            heartbeat_every: HEARTBEAT_EVERY,
        }
    }

    /// Events nobody hears.
    #[must_use]
    pub fn none() -> Self {
        Self {
            tui: None,
            plan_id: String::new(),
            stage: CheckStage::InPlace,
            source: String::new(),
            heartbeat_every: HEARTBEAT_EVERY,
        }
    }

    /// Publish heartbeats every `every` instead of [`HEARTBEAT_EVERY`].
    #[must_use]
    pub fn with_heartbeat_every(mut self, every: Duration) -> Self {
        self.heartbeat_every = every;
        self
    }

    /// Publish that step `index` (from 0) of `steps`, which runs `command`,
    /// started, and return it for its heartbeats and its end.
    #[must_use]
    pub fn start(&self, index: usize, steps: usize, command: &str) -> CheckStep<'_> {
        let step = CheckStep {
            events: self,
            step: u32::try_from(index + 1).unwrap_or(u32::MAX),
            steps: u32::try_from(steps).unwrap_or(u32::MAX),
            scope: check_scope(command),
            started: Instant::now(),
        };
        step.publish(|scope| DashboardEvent::PlanCheckStarted {
            plan_id: self.plan_id.clone(),
            stage: self.stage.as_str().to_string(),
            step: step.step,
            steps: step.steps,
            name: scope.name,
            command: command.to_string(),
            scope: scope.scope.to_string(),
            packages: scope.packages,
            source: self.source.clone(),
        });
        step
    }
}

/// A running step of a plan's check.
pub struct CheckStep<'a> {
    events: &'a PlanCheckEvents,
    step: u32,
    steps: u32,
    scope: CheckScope,
    started: Instant,
}

impl CheckStep<'_> {
    /// Await `run`, the step's command, publishing a heartbeat every period
    /// it is still running.
    pub async fn watch<F: Future>(&self, run: F) -> F::Output {
        if self.events.tui.is_none() {
            return run.await;
        }
        let every = self.events.heartbeat_every;
        let mut beats = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
        beats.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut run = std::pin::pin!(run);
        loop {
            tokio::select! {
                output = &mut run => return output,
                _ = beats.tick() => self.publish(|scope| DashboardEvent::PlanCheckHeartbeat {
                    plan_id: self.events.plan_id.clone(),
                    stage: self.events.stage.as_str().to_string(),
                    step: self.step,
                    steps: self.steps,
                    name: scope.name,
                    scope: scope.scope.to_string(),
                    packages: scope.packages,
                    elapsed_ms: self.elapsed_ms(),
                }),
            }
        }
    }

    /// Publish that the step ended: whether it passed, its exit code, and,
    /// for a step that did not exit, why it ended.
    pub fn finish(&self, passed: bool, exit_code: Option<i32>, reason: Option<String>) {
        self.publish(|scope| DashboardEvent::PlanCheckFinished {
            plan_id: self.events.plan_id.clone(),
            stage: self.events.stage.as_str().to_string(),
            step: self.step,
            steps: self.steps,
            name: scope.name,
            scope: scope.scope.to_string(),
            packages: scope.packages,
            passed,
            exit_code,
            reason,
            elapsed_ms: self.elapsed_ms(),
        });
    }

    fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn publish(&self, event: impl FnOnce(CheckScope) -> DashboardEvent) {
        if let Some(tui) = &self.events.tui {
            tui.publish_event(event(self.scope.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(command: &str) -> (String, &'static str, Vec<String>) {
        let scope = check_scope(command);
        (scope.name, scope.scope, scope.packages)
    }

    fn named(packages: &[&str]) -> Vec<String> {
        packages.iter().map(ToString::to_string).collect()
    }

    /// gap-6b6ec1: a step's events name what it builds, as its command says.
    #[test]
    fn check_scope_names_what_a_step_builds() {
        assert_eq!(
            scope("cargo fmt --all -- --check"),
            ("cargo fmt".to_string(), "workspace", vec![])
        );
        assert_eq!(
            scope("cargo clippy --workspace --no-deps -- -D warnings"),
            ("cargo clippy".to_string(), "workspace", vec![])
        );
        assert_eq!(
            scope("cargo test -p roko-gate -p roko-cli"),
            (
                "cargo test".to_string(),
                "packages",
                named(&["roko-cli", "roko-gate"])
            )
        );
        assert_eq!(
            scope("CARGO_TERM_COLOR=never cargo +nightly fmt --package=demo -- --check"),
            ("cargo fmt".to_string(), "packages", named(&["demo"]))
        );
        // At the root, cargo without `-p` builds the workspace's members.
        assert_eq!(
            scope("timeout 600 cargo build"),
            ("cargo build".to_string(), "workspace", vec![])
        );
        // After a `cd` it builds that directory's package, which only the
        // package graph knows.
        assert_eq!(
            scope("cd demo/hello && cargo test"),
            ("cargo test".to_string(), "command", vec![])
        );
        assert_eq!(
            scope("rustc demo/main.rs -o /tmp/demo && /tmp/demo | grep -q hi"),
            ("rustc".to_string(), "command", vec![])
        );
        assert_eq!(
            scope("cargo metadata --format-version=1"),
            ("cargo metadata".to_string(), "command", vec![])
        );
    }
}
