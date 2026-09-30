//! The environment gate commands run with.
//!
//! roko loads provider API keys from its `.env` files into its own
//! environment, so a gate command must not inherit that environment
//! wholesale: a verify step or an agent-written test could read the keys and
//! spend on them. Every command a gate spawns starts from an empty
//! environment plus the inherited variables
//! [`roko_core::child_env::gate_env`] admits, plus the variables the gate
//! sets explicitly ([`roko_core::child_env::apply_gate_env`]).

use std::ffi::OsString;

use roko_core::child_env;
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
pub fn inherit_gate_env_from(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (String, OsString)>,
    passthrough: &[String],
) {
    child_env::apply_gate_env(cmd.as_std_mut(), inherited, passthrough);
}

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
            ]
        );
    }
}
