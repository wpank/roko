//! Environment variable backend for [`SecretStore`] (§43.2).
//!
//! Reads secrets from `ROKO_SECRET_<CATEGORY>_<PROVIDER>` env vars. Read-only:
//! [`EnvVarStore::set`] returns [`RokoError::Invalid`] because environment
//! variables are a presentation layer, not a store — writes should go to a
//! [`super::FileStore`] or a vault backend.

use super::{Namespace, SecretStore};
use crate::error::{Result, RokoError};
use std::sync::Arc;

/// Resolver: given an env var name, return its value (or `None`).
///
/// Defaults to `std::env::var`; injectable for tests and embedding hosts that
/// supply a scoped environment (e.g. a sandbox that mounts secrets as a map).
pub type EnvResolver = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Reads secrets from environment variables using the
/// `ROKO_SECRET_<CATEGORY>_<PROVIDER>` naming convention.
///
/// The prefix is configurable via [`EnvVarStore::with_prefix`] so tests and
/// embedding hosts can namespace their own env vars (e.g. `MY_APP_SECRET`).
/// The underlying resolver defaults to `std::env::var` and can be swapped
/// via [`EnvVarStore::with_resolver`] for hermetic tests.
#[derive(Clone)]
pub struct EnvVarStore {
    prefix: String,
    resolver: EnvResolver,
}

impl std::fmt::Debug for EnvVarStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvVarStore")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

impl EnvVarStore {
    /// New store with the default `ROKO_SECRET` prefix, reading from the
    /// process environment.
    pub fn new() -> Self {
        Self::with_prefix("ROKO_SECRET")
    }

    /// New store with a custom prefix. The full env var name is
    /// `{prefix}_{CATEGORY}_{PROVIDER}`.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            resolver: Arc::new(|name| std::env::var(name).ok()),
        }
    }

    /// Replace the resolver function. Primarily useful for tests, where
    /// mutating the process environment is unsafe in edition 2024.
    #[must_use]
    pub fn with_resolver(mut self, resolver: EnvResolver) -> Self {
        self.resolver = resolver;
        self
    }

    /// Compute the full env var name for a namespace under this store's prefix.
    pub fn var_name_for(&self, ns: &Namespace) -> String {
        format!("{}_{}", self.prefix, ns.env_var_suffix())
    }
}

impl Default for EnvVarStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for EnvVarStore {
    fn get(&self, ns: &Namespace) -> Result<Option<String>> {
        let var = self.var_name_for(ns);
        Ok((self.resolver)(&var))
    }

    fn set(&self, _ns: &Namespace, _value: String) -> Result<()> {
        Err(RokoError::invalid(
            "EnvVarStore is read-only; use a FileStore or vault backend to persist secrets",
        ))
    }

    fn name(&self) -> &'static str {
        "env"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixture_resolver(map: HashMap<String, String>) -> EnvResolver {
        Arc::new(move |name| map.get(name).cloned())
    }

    #[test]
    fn env_store_reads_known_var() {
        let mut map = HashMap::new();
        map.insert("ROKO_SECRET_LLM_ANTHROPIC".into(), "sk-test-123".into());
        let store = EnvVarStore::new().with_resolver(fixture_resolver(map));
        let ns = Namespace::new("llm", "anthropic");
        let got = store.get(&ns).unwrap();
        assert_eq!(got, Some("sk-test-123".to_string()));
    }

    #[test]
    fn env_store_returns_none_for_missing() {
        let store = EnvVarStore::new().with_resolver(fixture_resolver(HashMap::new()));
        let ns = Namespace::new("llm", "openai");
        assert_eq!(store.get(&ns).unwrap(), None);
    }

    #[test]
    fn env_store_set_returns_unsupported() {
        let store = EnvVarStore::new();
        let ns = Namespace::new("rpc", "alchemy");
        let err = store.set(&ns, "value".into()).unwrap_err();
        assert!(matches!(err, RokoError::Invalid(_)));
        assert!(format!("{err}").contains("read-only"));
    }

    #[test]
    fn env_store_default_prefix_matches_namespace_env_var_name() {
        let store = EnvVarStore::new();
        let ns = Namespace::new("webhook", "github");
        assert_eq!(store.var_name_for(&ns), ns.env_var_name());
    }

    #[test]
    fn env_store_custom_prefix_used_by_resolver() {
        let mut map = HashMap::new();
        map.insert("MY_APP_RPC_INFURA".into(), "infura-key".into());
        let store = EnvVarStore::with_prefix("MY_APP").with_resolver(fixture_resolver(map));
        let ns = Namespace::new("rpc", "infura");
        assert_eq!(store.get(&ns).unwrap(), Some("infura-key".into()));
    }

    #[test]
    fn env_store_name_is_env() {
        assert_eq!(EnvVarStore::new().name(), "env");
    }

    #[test]
    fn env_store_rotate_also_returns_unsupported() {
        let store = EnvVarStore::new();
        let ns = Namespace::new("llm", "cohere");
        let err = store.rotate(&ns, "value".into()).unwrap_err();
        assert!(matches!(err, RokoError::Invalid(_)));
    }
}
