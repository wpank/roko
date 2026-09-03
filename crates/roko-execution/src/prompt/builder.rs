//! Prompt build handle -- wraps the prompt assembler configuration.
//!
//! The actual `PromptAssembler` lives in `roko-cli`'s dispatch/prompt_builder.rs
//! today. This handle provides the layer-3 contract for prompt assembly
//! configuration. CLI code will delegate through this handle after migration.

use roko_core::config::schema::ConfigCompositionStrategy;

/// Configuration for prompt assembly.
///
/// Constructed by [`RuntimeServicesBuilder`](crate::builder::RuntimeServicesBuilder)
/// from the validated config and passed to the prompt assembly pipeline.
#[derive(Debug, Clone)]
pub struct PromptBuildHandle {
    /// Composition strategy from config.
    pub composition_strategy: ConfigCompositionStrategy,
    /// Number of VCG warmup observations before the optimizer kicks in.
    pub vcg_warmup_observations: usize,
}

impl Default for PromptBuildHandle {
    fn default() -> Self {
        Self {
            composition_strategy: ConfigCompositionStrategy::default(),
            vcg_warmup_observations: 20,
        }
    }
}
