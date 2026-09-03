//! Model resolver handle -- resolves model names to provider/model pairs.
//!
//! The actual `ProviderDispatchResolver` lives in `roko-cli`'s dispatch_v2
//! module today. This handle provides the layer-3 contract that downstream
//! consumers (#244/#245/#283) will use after the resolution logic is
//! extracted.

use std::collections::HashSet;
use std::sync::Arc;

use roko_core::config::schema::RokoConfig;

/// A handle wrapping the model-to-provider resolution state.
///
/// Currently holds the resolved config and the set of configured model slugs.
/// After #244 extracts the resolution logic, this will own the resolver
/// directly.
#[derive(Debug, Clone)]
pub struct ModelResolverHandle {
    /// Effective config used for provider resolution.
    pub config: Arc<RokoConfig>,
    /// Set of model slugs that have a configured, credential-ready provider.
    pub configured_models: HashSet<String>,
}

impl ModelResolverHandle {
    /// Build from a validated config, extracting configured model slugs.
    pub fn from_config(config: Arc<RokoConfig>) -> Self {
        let configured_models: HashSet<String> = config
            .effective_models()
            .values()
            .map(|profile| profile.slug.clone())
            .collect();
        Self {
            config,
            configured_models,
        }
    }

    /// Create a minimal handle for testing.
    pub fn for_test() -> Self {
        Self {
            config: Arc::new(RokoConfig::default()),
            configured_models: HashSet::new(),
        }
    }
}
