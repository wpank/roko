//! The environment gate commands run with.
//!
//! roko loads provider API keys from its `.env` files into its own
//! environment, so a gate command must not inherit that environment
//! wholesale: a verify step or an agent-written test could read the keys and
//! spend on them. Every command a gate spawns starts from an empty
//! environment plus the inherited variables
//! [`roko_core::child_env::gate_env`] admits, plus the variables the gate
//! sets explicitly ([`roko_core::child_env::apply_gate_env`]).
//!
//! It also runs throttled ([`throttle`], bug-b3849c): its cargo builds and
//! test binaries get a job cap, and it starts at a lower priority, so a run's
//! checks leave the person's machine usable.

use std::ffi::OsString;

use roko_core::child_env::{self, GateLimits};
use tokio::process::Command;

/// Give `cmd` only the variables of roko's environment the gate policy admits.
///
/// `passthrough` holds extra names or `PREFIX*` patterns
/// (`[gates] env_passthrough`). Variables already set on `cmd` stay.
pub fn inherit_gate_env(cmd: &mut Command, passthrough: &[String]) {
    inherit_gate_env_from(cmd, child_env::process_env(), passthrough);
}

/// [`inherit_gate_env`] with `inherited` standing in for roko's own
/// environment.
///
/// The command is [throttled](throttle) by this process's
/// [gate limits](child_env::gate_limits).
pub fn inherit_gate_env_from(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (String, OsString)>,
    passthrough: &[String],
) {
    child_env::apply_gate_env(cmd.as_std_mut(), inherited, passthrough);
    throttle(cmd, &child_env::gate_limits());
}

/// Run `cmd` within `limits` (bug-b3849c).
///
/// `cmd`'s environment is the one [`inherit_gate_env_from`] set.
/// `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` cap its builds and tests unless
/// it already sets them, and it starts at niceness `limits.nice` unless it
/// would start nicer. Its children inherit both.
pub fn throttle(cmd: &mut Command, limits: &GateLimits) {
    child_env::apply_gate_job_cap(cmd.as_std_mut(), limits);
    lower_priority(cmd, limits.nice);
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn lower_priority(cmd: &mut Command, nice: i32) {
    if nice <= 0 {
        return;
    }
    // SAFETY: getpriority and setpriority are async-signal-safe system calls,
    // and the closure allocates nothing: it runs in the child between fork
    // and exec. Neither can fail the spawn: a priority that cannot be
    // lowered stays as it was.
    unsafe {
        cmd.pre_exec(move || {
            // -1 is a niceness as well as getpriority's error, so an error
            // only means one more setpriority.
            if libc::getpriority(libc::PRIO_PROCESS, 0) < nice {
                libc::setpriority(libc::PRIO_PROCESS, 0, nice);
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn lower_priority(_cmd: &mut Command, _nice: i32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(cmd: &Command) -> Vec<(String, Option<String>)> {
        let mut env: Vec<_> = cmd
            .as_std()
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect();
        env.sort();
        env
    }

    #[test]
    fn inherits_only_admitted_variables_and_keeps_explicit_ones() {
        let mut cmd = Command::new("true");
        cmd.env("CARGO_BUILD_JOBS", "2");
        cmd.env("OPENAI_API_KEY", "set-explicitly-by-the-gate");
        inherit_gate_env_from(
            &mut cmd,
            [
                ("PATH".to_string(), OsString::from("/usr/bin:/bin")),
                (
                    "OPENAI_API_KEY".to_string(),
                    OsString::from("sk-test-not-real"),
                ),
                ("GITHUB_TOKEN".to_string(), OsString::from("ghp_test")),
                ("DATABASE_URL".to_string(), OsString::from("postgres://db")),
            ],
            &["DATABASE_URL".to_string()],
        );
        // An explicit value wins over the inherited one, even a key's.
        assert_eq!(
            env_of(&cmd),
            vec![
                ("CARGO_BUILD_JOBS".to_string(), Some("2".to_string())),
                (
                    "DATABASE_URL".to_string(),
                    Some("postgres://db".to_string())
                ),
                (
                    "OPENAI_API_KEY".to_string(),
                    Some("set-explicitly-by-the-gate".to_string())
                ),
                ("PATH".to_string(), Some("/usr/bin:/bin".to_string())),
                (
                    "RUST_TEST_THREADS".to_string(),
                    Some(GateLimits::default_cargo_jobs().to_string())
                ),
            ]
        );
    }

    /// bug-b3849c: a gate command started with no job cap in roko's
    /// environment builds and tests with the default cap and runs at
    /// niceness 10, or nicer when roko itself is.
    #[cfg(unix)]
    #[tokio::test]
    async fn gate_commands_run_throttled() {
        let niceness = |pid: u32| {
            let ps = std::process::Command::new("ps")
                .args(["-o", "nice=", "-p", &pid.to_string()])
                .output()
                .expect("ps runs");
            String::from_utf8_lossy(&ps.stdout)
                .trim()
                .parse::<i32>()
                .expect("a niceness")
        };
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "echo \"$CARGO_BUILD_JOBS $RUST_TEST_THREADS $(ps -o nice= -p $$)\"",
        ]);
        inherit_gate_env_from(
            &mut cmd,
            [("PATH".to_string(), OsString::from("/usr/bin:/bin"))],
            &[],
        );

        let output = cmd.output().await.expect("the gate command runs");

        let seen = String::from_utf8_lossy(&output.stdout).into_owned();
        let jobs = GateLimits::default_cargo_jobs().to_string();
        let nice = GateLimits::DEFAULT_NICE.max(niceness(std::process::id()));
        assert_eq!(
            seen.split_whitespace().collect::<Vec<_>>(),
            [jobs.as_str(), jobs.as_str(), nice.to_string().as_str()],
            "{seen}"
        );
    }

    /// A job count roko's environment sets wins over the cap, and limits of
    /// `cargo_jobs = 0`, `nice = 0` change nothing.
    #[test]
    fn throttle_keeps_a_set_job_count_and_can_be_switched_off() {
        let parent = [
            ("PATH".to_string(), OsString::from("/usr/bin:/bin")),
            ("CARGO_BUILD_JOBS".to_string(), OsString::from("3")),
        ];
        let mut capped = Command::new("true");
        child_env::apply_gate_env(capped.as_std_mut(), parent.clone(), &[]);
        let limits = GateLimits {
            cargo_jobs: Some(5),
            nice: 0,
        };
        throttle(&mut capped, &limits);
        let mut off = Command::new("true");
        child_env::apply_gate_env(off.as_std_mut(), parent, &[]);
        let limits = GateLimits {
            cargo_jobs: None,
            nice: 0,
        };
        throttle(&mut off, &limits);

        let var = |cmd: &Command, name: &str| {
            env_of(cmd)
                .into_iter()
                .find(|(set, _)| set == name)
                .and_then(|(_, value)| value)
        };
        assert_eq!(var(&capped, "CARGO_BUILD_JOBS").as_deref(), Some("3"));
        assert_eq!(var(&capped, "RUST_TEST_THREADS").as_deref(), Some("5"));
        assert_eq!(var(&off, "CARGO_BUILD_JOBS").as_deref(), Some("3"));
        assert_eq!(var(&off, "RUST_TEST_THREADS"), None);
    }
}
