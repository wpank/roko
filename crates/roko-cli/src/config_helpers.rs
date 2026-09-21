//! Configuration and path helpers (originally extracted from the legacy orchestrator).
//!
//! This module contains:
//! - `.roko/` layout path constructors
//! - Model routing helpers (provider maps, role overrides)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use roko_core::config::schema::{RokoConfig, RoleOverride};

// ── Path helpers ──────────────────────────────────────────────────────

pub(crate) fn daimon_state_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("daimon").join("affect.json")
}

// ── Routing model helpers ─────────────────────────────────────────────

pub(crate) fn routing_model_provider_map(config: &RokoConfig) -> HashMap<String, String> {
    let mut providers = HashMap::new();
    for (model_key, profile) in config.effective_models() {
        providers.insert(model_key, profile.provider.clone());
        providers.entry(profile.slug).or_insert(profile.provider);
    }
    providers
}

pub(crate) fn find_role_override<'a>(
    config: &'a RokoConfig,
    role_label: &str,
) -> Option<&'a RoleOverride> {
    config.agent.roles.get(role_label).or_else(|| {
        config
            .agent
            .roles
            .iter()
            .find_map(|(section_name, override_cfg)| {
                (override_cfg.resolved_role_name(section_name) == role_label)
                    .then_some(override_cfg)
            })
    })
}

/// Check whether `role_label` is enabled for dispatch.
///
/// Returns `true` when the role has no override or the override has
/// `enabled == true`. Returns `false` only when an explicit
/// `enabled = false` is configured.
pub(crate) fn is_role_enabled(config: &RokoConfig, role_label: &str) -> bool {
    find_role_override(config, role_label).map_or(true, |r| r.enabled)
}
