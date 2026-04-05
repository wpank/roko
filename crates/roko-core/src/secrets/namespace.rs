//! Namespaced secret keys (§43.7).
//!
//! A [`Namespace`] is a `category.provider` pair such as `llm.anthropic`
//! or `rpc.alchemy`. Namespaces have canonical string, env-var, and TOML
//! forms so every backend agrees on naming.

use crate::error::{Result, RokoError};

/// A namespaced secret key, e.g. `llm.anthropic` or `rpc.alchemy`.
///
/// Namespaces are two-part: a top-level category (`llm`, `rpc`, `webhook`, ...)
/// and a concrete provider (`anthropic`, `alchemy`, `github`, ...). Canonical
/// string form is `category.provider`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Namespace {
    /// Top-level category: `llm`, `rpc`, `webhook`, ...
    pub category: String,
    /// Provider / target: `anthropic`, `openai`, `alchemy`, `github`, ...
    pub provider: String,
}

impl Namespace {
    /// Construct a namespace from raw strings. Neither component may be empty
    /// at retrieval time; this constructor does not validate (for ergonomics
    /// with static well-known namespaces). Use [`Namespace::parse`] to validate.
    pub fn new(category: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            provider: provider.into(),
        }
    }

    /// Parse `category.provider` string form.
    ///
    /// Returns `RokoError::Invalid` if the string is empty, missing the dot
    /// separator, has more than one dot, or has an empty component.
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(RokoError::invalid("namespace is empty"));
        }
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(RokoError::invalid(format!(
                "namespace {s:?} must have exactly one dot (category.provider)"
            )));
        }
        let category = parts[0];
        let provider = parts[1];
        if category.is_empty() || provider.is_empty() {
            return Err(RokoError::invalid(format!(
                "namespace {s:?} has empty category or provider"
            )));
        }
        Ok(Self::new(category, provider))
    }

    /// Canonical string form: `category.provider`.
    pub fn as_string(&self) -> String {
        format!("{}.{}", self.category, self.provider)
    }

    /// Default environment variable name: `ROKO_SECRET_CATEGORY_PROVIDER`
    /// (uppercased, `.` → `_`).
    ///
    /// This is the name used by [`EnvVarStore::new`] which defaults its prefix
    /// to `ROKO_SECRET`. For stores with a custom prefix, combine
    /// [`Namespace::env_var_suffix`] with the prefix instead.
    pub fn env_var_name(&self) -> String {
        format!("ROKO_SECRET_{}", self.env_var_suffix())
    }

    /// Environment variable suffix: `CATEGORY_PROVIDER` (uppercased).
    ///
    /// Combined with a store prefix via `{prefix}_{suffix}`.
    pub fn env_var_suffix(&self) -> String {
        format!(
            "{}_{}",
            self.category.to_ascii_uppercase(),
            self.provider.to_ascii_uppercase()
        )
    }

    /// TOML key: `category.provider` (mirrors canonical string form).
    pub fn toml_key(&self) -> String {
        self.as_string()
    }
}

/// Static well-known namespaces (§43.7).
///
/// These cover the initial provider set: LLM backends, RPC providers,
/// and webhook integrations. Additional providers are added here as
/// new backends land.
pub struct WellKnownNamespaces;

impl WellKnownNamespaces {
    /// `llm.anthropic` — Anthropic Claude API.
    pub fn llm_anthropic() -> Namespace {
        Namespace::new("llm", "anthropic")
    }
    /// `llm.openai` — `OpenAI` API.
    pub fn llm_openai() -> Namespace {
        Namespace::new("llm", "openai")
    }
    /// `llm.cohere` — Cohere API.
    pub fn llm_cohere() -> Namespace {
        Namespace::new("llm", "cohere")
    }
    /// `rpc.alchemy` — Alchemy RPC provider.
    pub fn rpc_alchemy() -> Namespace {
        Namespace::new("rpc", "alchemy")
    }
    /// `rpc.infura` — Infura RPC provider.
    pub fn rpc_infura() -> Namespace {
        Namespace::new("rpc", "infura")
    }
    /// `rpc.quicknode` — `QuickNode` RPC provider.
    pub fn rpc_quicknode() -> Namespace {
        Namespace::new("rpc", "quicknode")
    }
    /// `webhook.github` — `GitHub` webhook signing secret.
    pub fn webhook_github() -> Namespace {
        Namespace::new("webhook", "github")
    }
    /// `webhook.slack` — Slack webhook signing secret.
    pub fn webhook_slack() -> Namespace {
        Namespace::new("webhook", "slack")
    }

    /// All well-known namespaces, in declaration order.
    pub fn all() -> Vec<Namespace> {
        vec![
            Self::llm_anthropic(),
            Self::llm_openai(),
            Self::llm_cohere(),
            Self::rpc_alchemy(),
            Self::rpc_infura(),
            Self::rpc_quicknode(),
            Self::webhook_github(),
            Self::webhook_slack(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_name_uppercases() {
        let ns = Namespace::new("llm", "anthropic");
        assert_eq!(ns.env_var_name(), "ROKO_SECRET_LLM_ANTHROPIC");
    }

    #[test]
    fn env_var_name_uppercases_mixed_case_input() {
        let ns = Namespace::new("Rpc", "Alchemy");
        assert_eq!(ns.env_var_name(), "ROKO_SECRET_RPC_ALCHEMY");
    }

    #[test]
    fn parse_valid_form() {
        let ns = Namespace::parse("llm.openai").unwrap();
        assert_eq!(ns.category, "llm");
        assert_eq!(ns.provider, "openai");
    }

    #[test]
    fn parse_rejects_empty() {
        let err = Namespace::parse("").unwrap_err();
        assert!(matches!(err, RokoError::Invalid(_)));
    }

    #[test]
    fn parse_rejects_missing_dot() {
        let err = Namespace::parse("llmanthropic").unwrap_err();
        assert!(matches!(err, RokoError::Invalid(_)));
    }

    #[test]
    fn parse_rejects_three_parts() {
        let err = Namespace::parse("llm.anthropic.extra").unwrap_err();
        assert!(matches!(err, RokoError::Invalid(_)));
    }

    #[test]
    fn parse_rejects_empty_component() {
        assert!(Namespace::parse(".provider").is_err());
        assert!(Namespace::parse("category.").is_err());
        assert!(Namespace::parse(".").is_err());
    }

    #[test]
    fn as_string_roundtrips_parse() {
        let ns = Namespace::new("webhook", "github");
        let s = ns.as_string();
        assert_eq!(s, "webhook.github");
        let parsed = Namespace::parse(&s).unwrap();
        assert_eq!(parsed, ns);
    }

    #[test]
    fn toml_key_matches_canonical_string() {
        let ns = Namespace::new("rpc", "infura");
        assert_eq!(ns.toml_key(), ns.as_string());
    }

    #[test]
    fn well_known_namespaces_all_parseable() {
        for ns in WellKnownNamespaces::all() {
            let s = ns.as_string();
            let parsed = Namespace::parse(&s).expect("well-known namespace must parse");
            assert_eq!(parsed, ns, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn well_known_namespaces_non_empty() {
        assert!(!WellKnownNamespaces::all().is_empty());
        assert_eq!(WellKnownNamespaces::all().len(), 8);
    }
}
