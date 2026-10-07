//! FAST lane (`./dev.sh fast`) bounds for Graph plan runs.
//!
//! With `ROKO_FAST_MODE` truthy, a run gets `ROKO_FAST_PLAN_DEADLINE_SECS`
//! (default 300) of wall-clock time. `dev.sh` sets that deadline to its own
//! hard deadline minus settlement headroom, so the run stops cleanly before
//! the evidence wrapper kills it. At the deadline the run stops through its
//! [`PlanRunInterruptHandle`]: the running graph is cancelled, its checkpoint
//! is finalized as `interrupted`, and the run exits 143, as on SIGTERM. Its
//! summary and `--log-file` name the stop [`PlanRunInterrupt::Deadline`]
//! (`deadline`), not SIGTERM (gap-9efe8e).
//!
//! Each task attempt is bounded as well ([`FastAttemptBounds`], gap-4a6dcb):
//! at most `ROKO_FAST_MAX_AGENT_TURNS` turns (default 6) and 90 s, so it can
//! stay silent no longer either; a system prompt section that tells the agent
//! to patch and hand off without running Cargo; simple cargo verify commands
//! built in the `dev-fast` profile when the workspace declares it; and no
//! `cargo fix` after a failed verify. Verify step timeouts stay as authored.

use std::path::Path;
use std::time::Duration;

use super::plan_runner::{PlanRunInterrupt, PlanRunInterruptHandle};
use crate::dispatch::AgentDispatchRequest;
use crate::runner::cargo_command::{cargo_command_with_profile, cargo_profile_available};

/// Run deadline when `ROKO_FAST_PLAN_DEADLINE_SECS` is unset or invalid.
const DEFAULT_PLAN_DEADLINE: Duration = Duration::from_secs(300);

/// Turns a FAST attempt may take when `ROKO_FAST_MAX_AGENT_TURNS` is unset,
/// zero or invalid.
const DEFAULT_FAST_MAX_TURNS: u32 = 6;

/// The longest a FAST attempt may run, in milliseconds.
const FAST_ATTEMPT_TIMEOUT_MS: u64 = 90_000;

/// The Cargo profile FAST verify commands build in when the workspace
/// declares it: a build cache apart from `target/debug`.
const FAST_CARGO_PROFILE: &str = "dev-fast";

/// Appended to a FAST attempt's system prompt.
const FAST_PROMPT_SECTION: &str = "\n\n## FAST implementation mode\n\nProduce the smallest \
    correct patch and hand off quickly. Do not run cargo, tests, clippy, npm, builds, or \
    servers; the runner owns verification. Avoid broad refactors and unrelated edits. End with \
    a structured summary of files changed, behavior implemented, and verification the runner \
    should perform.\n";

/// How FAST mode bounds each task attempt of a run (gap-4a6dcb).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastAttemptBounds {
    /// The most turns an attempt may take.
    pub max_turns: u32,
    /// The profile simple cargo verify commands build in: `dev-fast` when
    /// the workspace's `Cargo.toml` declares it.
    pub cargo_profile: Option<&'static str>,
}

impl FastAttemptBounds {
    /// The bounds of a FAST run in `workdir`; `None` outside FAST mode.
    #[must_use]
    pub fn from_env(workdir: &Path) -> Option<Self> {
        if !env_flag_enabled("ROKO_FAST_MODE") {
            return None;
        }
        let max_turns = std::env::var("ROKO_FAST_MAX_AGENT_TURNS").ok();
        let profile = cargo_profile_available(workdir, FAST_CARGO_PROFILE);
        Some(Self {
            max_turns: fast_max_turns(max_turns.as_deref()),
            cargo_profile: profile.then_some(FAST_CARGO_PROFILE),
        })
    }

    /// Bound `request`: at most [`Self::max_turns`] turns and 90 s, never
    /// more than it asked for, and the patch-only section on its system
    /// prompt.
    pub(crate) fn bound(&self, request: &mut AgentDispatchRequest) {
        request.max_turns = capped(request.max_turns, self.max_turns);
        request.timeout_ms = capped(request.timeout_ms, FAST_ATTEMPT_TIMEOUT_MS);
        request.system_prompt.push_str(FAST_PROMPT_SECTION);
    }

    /// `command` as a FAST verify step runs it, when that differs: a simple
    /// cargo `check`, `clippy` or `test` builds in [`Self::cargo_profile`].
    /// A composed or quoted command runs as authored.
    pub(crate) fn verify_command(&self, command: &str) -> Option<String> {
        cargo_command_with_profile(command, self.cargo_profile?)
    }
}

/// `value` capped at `cap`; an unset value takes the cap.
fn capped<T: Ord + Copy>(value: Option<T>, cap: T) -> Option<T> {
    Some(value.map_or(cap, |value| value.min(cap)))
}

/// The FAST turn cap: `configured` when it is a positive number.
fn fast_max_turns(configured: Option<&str>) -> u32 {
    configured
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|turns| *turns > 0)
        .unwrap_or(DEFAULT_FAST_MAX_TURNS)
}

/// Stops its run when the FAST deadline elapses; dropping it disarms.
#[derive(Debug)]
pub struct FastPlanDeadline {
    timer: tokio::task::JoinHandle<()>,
}

impl Drop for FastPlanDeadline {
    fn drop(&mut self) {
        self.timer.abort();
    }
}

/// Arm the FAST run deadline for `interrupt`; `None` outside FAST mode.
pub fn arm_plan_deadline(interrupt: &PlanRunInterruptHandle) -> Option<FastPlanDeadline> {
    if !env_flag_enabled("ROKO_FAST_MODE") {
        return None;
    }
    let deadline = plan_deadline(
        std::env::var("ROKO_FAST_PLAN_DEADLINE_SECS")
            .ok()
            .as_deref(),
    );
    Some(arm_deadline(interrupt, deadline))
}

/// Stop `interrupt`'s run as [`PlanRunInterrupt::Deadline`] once `deadline`
/// has elapsed.
fn arm_deadline(interrupt: &PlanRunInterruptHandle, deadline: Duration) -> FastPlanDeadline {
    let interrupt = interrupt.clone();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(deadline).await;
        if interrupt.request(PlanRunInterrupt::Deadline) {
            tracing::warn!(
                deadline_secs = deadline.as_secs(),
                "FAST run deadline elapsed; stopping the plan run"
            );
        }
    });
    FastPlanDeadline { timer }
}

fn plan_deadline(configured: Option<&str>) -> Duration {
    configured
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map_or(DEFAULT_PLAN_DEADLINE, Duration::from_secs)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_deadline_parses_positive_seconds_only() {
        assert_eq!(plan_deadline(Some("270")), Duration::from_secs(270));
        assert_eq!(plan_deadline(Some(" 45 ")), Duration::from_secs(45));
        assert_eq!(plan_deadline(Some("0")), DEFAULT_PLAN_DEADLINE);
        assert_eq!(plan_deadline(Some("soon")), DEFAULT_PLAN_DEADLINE);
        assert_eq!(plan_deadline(None), DEFAULT_PLAN_DEADLINE);
    }

    /// gap-4a6dcb: the FAST turn cap comes from `ROKO_FAST_MAX_AGENT_TURNS`
    /// when it is a positive number, and an attempt's turns and time are
    /// capped, never raised.
    #[test]
    fn fast_attempts_are_capped_never_raised() {
        assert_eq!(fast_max_turns(Some("4")), 4);
        assert_eq!(fast_max_turns(Some(" 9 ")), 9);
        assert_eq!(fast_max_turns(Some("0")), DEFAULT_FAST_MAX_TURNS);
        assert_eq!(fast_max_turns(Some("many")), DEFAULT_FAST_MAX_TURNS);
        assert_eq!(fast_max_turns(None), DEFAULT_FAST_MAX_TURNS);
        assert_eq!(capped(Some(60), 6), Some(6));
        assert_eq!(capped(Some(3), 6), Some(3));
        assert_eq!(capped(None, 6), Some(6));
        let timeout = capped(Some(120_000), FAST_ATTEMPT_TIMEOUT_MS);
        assert_eq!(timeout, Some(90_000));
    }

    /// gap-4a6dcb: a simple cargo verify command builds in the FAST profile;
    /// a composed command, a non-cargo one, or a workspace without the
    /// profile runs as authored.
    #[test]
    fn fast_verify_commands_build_in_the_fast_profile() {
        let fast = FastAttemptBounds {
            max_turns: 6,
            cargo_profile: Some(FAST_CARGO_PROFILE),
        };
        assert_eq!(
            fast.verify_command("cargo test -p demo").as_deref(),
            Some("cargo test -p demo --profile dev-fast")
        );
        assert_eq!(fast.verify_command("cargo test && echo done"), None);
        assert_eq!(fast.verify_command("grep -q Trait src/lib.rs"), None);
        let without_profile = FastAttemptBounds {
            cargo_profile: None,
            ..fast
        };
        assert_eq!(without_profile.verify_command("cargo test -p demo"), None);
    }

    /// gap-9efe8e: an elapsed deadline stops its run as a deadline, which
    /// the summary and `--log-file` can tell apart from SIGTERM.
    #[tokio::test]
    async fn an_elapsed_deadline_stops_the_run_as_a_deadline() {
        let interrupt = PlanRunInterruptHandle::default();
        let _deadline = arm_deadline(&interrupt, Duration::from_millis(10));
        for _ in 0..200 {
            if interrupt.requested().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(interrupt.requested(), Some(PlanRunInterrupt::Deadline));
    }
}
