//! `[audit]` in `roko.toml` (S05 §5, §4.9): M4's random deep audits.
//!
//! Audits stay off until the audit worker lands (7123). Then decision 7's
//! answer applies: 10% of green attempts on real work (`rho`), never below
//! the 5% floor. The floor is locked, so an `eps_floor` below it is an error:
//! only the author changes it. Only ρ and λ adapt at run time (S05 §4.9), and
//! nothing in a learning loop writes this section.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audit_home::{AUDIT_HOME_ENV, AuditVault, AuditVaultError};

/// The audit floor ε: no green unit is drawn with a lower probability.
pub const AUDIT_FLOOR: f64 = 0.05;

/// The highest base rate ρ may take (S05 §4.9).
pub const MAX_RHO: f64 = 0.5;

/// `[audit]` in `roko.toml`. Every key is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuditConfig {
    /// Whether green attempts are drawn for audits at all.
    pub enabled: bool,
    /// The vault root, overriding `~/.roko/audit`; `ROKO_AUDIT_HOME` wins
    /// over it. A leading `~` is the home directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<PathBuf>,
    /// ρ, the base audit rate, in [`eps_floor`, 0.5].
    pub rho: f64,
    /// ε, the floor every π_i stays at or above. Locked at 0.05 or more.
    pub eps_floor: f64,
    /// λ_max, the most the M3 risk score may tilt the draw.
    pub lambda_max: f64,
    /// ECE_ref: the tilt falls to 0 as M3's calibration error reaches it.
    pub ece_ref: f64,
    /// Green units in an estimation window.
    pub window_units: u32,
    /// Hours in an estimation window, whichever comes first.
    pub window_hours: u32,
    /// θ_max, the false-green rate the strictness ladder holds below.
    pub theta_max: f64,
    /// γ_max, the spec-gaming rate the strictness ladder holds below.
    pub gamma_max: f64,
    /// τ_mut, the mutation score below which an oracle is weak.
    pub tau_mut: f64,
    /// The share of phase-A-clean units that go on to phase B.
    pub phase_b_rate: f64,
    /// The share of a run's spend audits may take.
    pub budget_frac: f64,
    /// The most one audit may spend, in USD.
    pub per_audit_usd: f64,
    /// The most CPU seconds one audit may use.
    pub per_audit_cpu_secs: u64,
    /// Seconds a closing run waits for audits still running.
    pub drain_secs: u64,
    /// Whether a fix task may see the failing hidden tests; a shown suite
    /// is burned (S05 §4.4).
    pub show_failing_hidden_tests: bool,
    /// Model families for cross-family authors and reviewers: family name
    /// to model-name globs.
    pub families: BTreeMap<String, Vec<String>>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            home: None,
            rho: 0.10,
            eps_floor: AUDIT_FLOOR,
            lambda_max: 0.8,
            ece_ref: 0.10,
            window_units: 200,
            window_hours: 24,
            theta_max: 0.05,
            gamma_max: 0.02,
            tau_mut: 0.6,
            phase_b_rate: 0.5,
            budget_frac: 0.12,
            per_audit_usd: 0.30,
            per_audit_cpu_secs: 900,
            drain_secs: 300,
            show_failing_hidden_tests: false,
            families: default_families(),
        }
    }
}

/// S05 §5's families. gpt-oss is OpenAI's, so a gpt-oss implementer gets a
/// GLM or Kimi author.
fn default_families() -> BTreeMap<String, Vec<String>> {
    fn globs(globs: &[&str]) -> Vec<String> {
        globs.iter().map(ToString::to_string).collect()
    }
    BTreeMap::from([
        ("anthropic".to_string(), globs(&["claude-*"])),
        ("moonshot".to_string(), globs(&["kimi-*"])),
        ("openai".to_string(), globs(&["gpt-5*", "gpt-oss-*"])),
        ("zhipu".to_string(), globs(&["glm-*"])),
    ])
}

impl AuditConfig {
    /// Every problem with the section, as (key, problem).
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        if !(AUDIT_FLOOR..=MAX_RHO).contains(&self.eps_floor) {
            problems.push((
                "eps_floor",
                format!(
                    "eps_floor ({}) must lie in [{AUDIT_FLOOR}, {MAX_RHO}]: the floor is locked, \
                     and only the author changes it",
                    self.eps_floor
                ),
            ));
        }
        if !(self.eps_floor..=MAX_RHO).contains(&self.rho) {
            problems.push((
                "rho",
                format!(
                    "rho ({}) must lie in [eps_floor ({}), {MAX_RHO}]",
                    self.rho, self.eps_floor
                ),
            ));
        }
        let shares = [
            ("lambda_max", self.lambda_max),
            ("ece_ref", self.ece_ref),
            ("theta_max", self.theta_max),
            ("gamma_max", self.gamma_max),
            ("tau_mut", self.tau_mut),
            ("phase_b_rate", self.phase_b_rate),
            ("budget_frac", self.budget_frac),
        ];
        for (key, value) in shares {
            if !(0.0..=1.0).contains(&value) {
                problems.push((key, format!("{key} ({value}) must be between 0 and 1")));
            }
        }
        if self.per_audit_usd.is_nan() || self.per_audit_usd < 0.0 {
            let usd = self.per_audit_usd;
            problems.push((
                "per_audit_usd",
                format!("per_audit_usd ({usd}) cannot be negative"),
            ));
        }
        for (key, value) in [
            ("window_units", self.window_units),
            ("window_hours", self.window_hours),
        ] {
            if value == 0 {
                problems.push((key, format!("{key} must be at least 1")));
            }
        }
        let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
        for (family, globs) in &self.families {
            for glob in globs {
                if let Some(other) = owners.insert(glob, family) {
                    problems.push((
                        "families",
                        format!("the glob `{glob}` is in two families, {other} and {family}"),
                    ));
                }
            }
        }
        problems
    }

    /// Check the section.
    ///
    /// # Errors
    ///
    /// Every problem of [`Self::problems`], joined with `; `.
    pub fn validate(&self) -> Result<(), String> {
        let problems = self.problems();
        if problems.is_empty() {
            return Ok(());
        }
        Err(problems
            .into_iter()
            .map(|(key, problem)| format!("audit.{key}: {problem}"))
            .collect::<Vec<_>>()
            .join("; "))
    }

    /// The vault of the workspace at `workspace_root`: `ROKO_AUDIT_HOME`,
    /// else [`Self::home`], else `~/.roko/audit` ([`AuditVault`]).
    pub fn vault(&self, workspace_root: &Path) -> Result<AuditVault, AuditVaultError> {
        let env = std::env::var_os(AUDIT_HOME_ENV).filter(|value| !value.is_empty());
        let audit_home = env.map(PathBuf::from).or_else(|| self.home.clone());
        let home = std::env::var_os("HOME").filter(|value| !value.is_empty());
        AuditVault::resolve_with(
            workspace_root,
            audit_home.as_deref(),
            home.as_deref().map(Path::new),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::RokoConfig;
    use crate::config::validation::{InvariantSeverity, validate_invariants};

    #[test]
    fn audit_config_locks_the_floor() {
        // The defaults: off until the worker lands, decision 7's 10%, the 5% floor.
        let defaults = AuditConfig::default();
        assert!(!defaults.enabled);
        assert_eq!((defaults.rho, defaults.eps_floor), (0.10, 0.05));
        assert_eq!(defaults.validate(), Ok(()));
        let empty = RokoConfig::from_toml("").expect("an empty config");
        assert_eq!(empty.audit, defaults);
        let text = toml::to_string(&defaults).expect("serialize the defaults");
        let back: AuditConfig = toml::from_str(&text).expect("parse them back");
        assert_eq!(back, defaults);

        // A lower floor and a rate above 0.5 are rejected, and loading fails.
        for (toml_text, key) in [
            ("[audit]\neps_floor = 0.01\n", "audit.eps_floor"),
            ("[audit]\nrho = 0.9\n", "audit.rho"),
            ("[audit]\nrho = 0.02\n", "audit.rho"),
        ] {
            let config = RokoConfig::from_toml(toml_text).expect("the section parses");
            assert!(config.audit.validate().is_err(), "{toml_text}");
            let rejected = validate_invariants(&config);
            assert!(
                rejected.iter().any(|result| result.config_path == key
                    && result.severity == InvariantSeverity::Error),
                "{toml_text}: {rejected:?}"
            );
        }

        // [audit.families] replaces the default map; a glob owns one family.
        let config = RokoConfig::from_toml(
            "[audit]\nenabled = true\nrho = 0.15\n\n[audit.families]\n\
             openai = [\"gpt-5*\"]\nzhipu = [\"glm-*\"]\n",
        )
        .expect("families parse");
        assert!(config.audit.enabled);
        assert_eq!(config.audit.families.len(), 2);
        assert_eq!(config.audit.families["zhipu"], ["glm-*"]);
        assert_eq!(config.audit.validate(), Ok(()));
        let mut shared = config.audit;
        shared.families.insert("other".into(), vec!["glm-*".into()]);
        assert!(
            shared
                .validate()
                .is_err_and(|error| error.contains("two families"))
        );
        assert!(RokoConfig::from_toml("[audit]\nsurprise = 1\n").is_err());
    }
}
