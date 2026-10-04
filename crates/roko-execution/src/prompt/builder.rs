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
#[derive(Debug, Clone, Default)]
pub struct PromptBuildHandle {
    /// Composition strategy from config.
    pub composition_strategy: ConfigCompositionStrategy,
}
