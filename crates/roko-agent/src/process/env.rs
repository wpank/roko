//! Agent environment configuration.
//!
//! Provides [`AgentEnv`] for declaring environment variables that should be set
//! (or cleared) on an agent subprocess, and [`apply_agent_env`] to stamp them
//! onto a [`tokio::process::Command`] before spawn.
//!
//! [`apply_credential_scrub`] keeps provider credentials the agent does not
//! own out of a provider CLI's inherited environment; see
//! [`CredentialScrub`].

use std::collections::{BTreeSet, HashMap};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use roko_core::child_env::{self, CredentialScrub};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Environment configuration for an agent subprocess.
///
/// Stores key-value pairs to set, plus keys to explicitly unset (`env_remove`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentEnv {
    /// Variables to set in the child environment.
    pub vars: HashMap<String, String>,
    /// Variables to remove from the inherited environment.
    pub remove: Vec<String>,
    /// Working directory for the child process.
    pub working_dir: Option<PathBuf>,
}

impl AgentEnv {
    /// Create an empty environment config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable to be set in the child.
    #[must_use]
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Mark a variable for removal from the child environment.
    #[must_use]
    pub fn unset(mut self, key: impl Into<String>) -> Self {
        self.remove.push(key.into());
        self
    }

    /// Set the working directory for the child.
    #[must_use]
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Apply an [`AgentEnv`] to a [`Command`] before spawning.
///
/// Sets each variable from `env.vars`, removes each key in `env.remove`,
/// and sets the working directory if specified.
pub fn apply_agent_env(cmd: &mut Command, env: &AgentEnv) {
    for (key, value) in &env.vars {
        cmd.env(key, value);
    }
    for key in &env.remove {
        cmd.env_remove(key);
    }
    if let Some(dir) = &env.working_dir {
        cmd.current_dir(dir);
    }
}

/// Remove from `cmd` the inherited variables `scrub` strips: provider keys
/// the agent does not own, names roko loaded from its `.env` files, and
/// roko's own credentials. Variables set explicitly on `cmd` stay.
pub fn apply_credential_scrub(cmd: &mut Command, scrub: &CredentialScrub) {
    let explicit: BTreeSet<OsString> = cmd
        .as_std()
        .get_envs()
        .filter(|(_, value)| value.is_some())
        .map(|(name, _)| name.to_os_string())
        .collect();
    let inherited: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| !explicit.contains(OsStr::new(name)))
        .collect();
    for name in scrub.names_to_strip(
        inherited.iter().map(String::as_str),
        child_env::startup_dotenv(),
    ) {
        cmd.env_remove(name);
    }
}

/// Variable names `text` refers to as `${NAME}` or `${NAME:-default}`, in
/// order of first use. MCP configs use this form to hand a server a variable
/// from the agent's environment.
#[must_use]
pub fn referenced_env_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (start, _) in text.match_indices("${") {
        let rest = &text[start + 2..];
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let name = &rest[..len];
        let valid = name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
        if valid && !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// [`referenced_env_names`] of the config file at `path`; empty when it
/// cannot be read.
#[must_use]
pub fn config_file_env_names(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| referenced_env_names(&text))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_env_builder_pattern() {
        let env = AgentEnv::new()
            .set("FOO", "bar")
            .set("BAZ", "qux")
            .unset("SECRET")
            .with_working_dir("/tmp");

        assert_eq!(env.vars.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.vars.get("BAZ"), Some(&"qux".to_string()));
        assert!(env.remove.contains(&"SECRET".to_string()));
        assert_eq!(env.working_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn apply_agent_env_does_not_panic() {
        let env = AgentEnv::new().set("TEST_VAR", "hello").unset("PATH_EXTRA");
        let mut cmd = Command::new("echo");
        apply_agent_env(&mut cmd, &env);
        // Cannot inspect the command's env directly, but ensure no panic.
    }

    #[test]
    fn referenced_env_names_reads_placeholders() {
        let text = r#"{"env": {"A": "${PERPLEXITY_API_KEY}", "B": "${HOST:-localhost}",
            "C": "$NOT_BRACED", "D": "${PERPLEXITY_API_KEY}", "E": "${1BAD}", "F": "${}"}}"#;
        assert_eq!(
            referenced_env_names(text),
            vec!["PERPLEXITY_API_KEY".to_string(), "HOST".to_string()]
        );
    }

    #[test]
    fn credential_scrub_keeps_variables_set_on_the_command() {
        let mut cmd = Command::new("true");
        cmd.env("OPENAI_API_KEY", "set-explicitly");
        apply_credential_scrub(&mut cmd, &CredentialScrub::default());
        let key = cmd
            .as_std()
            .get_envs()
            .find(|(name, _)| *name == "OPENAI_API_KEY")
            .and_then(|(_, value)| value);
        assert_eq!(key, Some(std::ffi::OsStr::new("set-explicitly")));
    }

    #[test]
    fn default_env_is_empty() {
        let env = AgentEnv::default();
        assert!(env.vars.is_empty());
        assert!(env.remove.is_empty());
        assert!(env.working_dir.is_none());
    }
}
