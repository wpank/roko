//! Extension chain loader — scans `.roko/extensions/` and `plugins/` for
//! extension manifests and creates [`Extension`] implementations from them.
//!
//! Each discovered plugin manifest (`plugin.toml`) becomes a
//! [`PluginExtension`] that logs lifecycle events and can be extended to
//! execute declarative hooks (tool profiles, triggers, etc.) in the future.
//!
//! In addition, each subdirectory of `.roko/extensions/` is probed for an
//! `extension.toml` or `manifest.toml` file containing a full
//! [`ExtensionManifest`] (v2 spec §3). Manifests are validated on load and
//! any that fail validation are skipped with a warning.

use std::path::Path;

use roko_core::extension::{
    Extension, ExtensionChain, ExtensionLayer, ExtensionManifest, ExtensionMeta, GateEvent,
    InferenceRequest, InferenceResponse, ManifestValidationError, PackageTier,
};
use roko_fs::RokoLayout;
use tracing::{debug, info, warn};

// ─── LoadedExtension ────────────────────────────────────────────────────

/// A successfully loaded extension manifest together with its health state.
///
/// Returned by [`scan_extension_manifests`] for callers that need to inspect
/// which extensions were discovered before registering them with the chain.
#[derive(Debug, Clone)]
pub struct LoadedExtension {
    /// Parsed and validated manifest.
    pub manifest: ExtensionManifest,
    /// Path of the manifest file that was loaded.
    pub manifest_path: std::path::PathBuf,
    /// Whether the extension was disabled (present in `disable_extensions`).
    pub disabled: bool,
}

// ─── PluginExtension (kept from original) ──────────��───────────────────────────────────────

/// An [`Extension`] backed by a discovered plugin manifest.
///
/// Currently provides logging at each hook point. When a plugin declares
/// tool profiles or triggers, this wrapper is the right place to enforce
/// them.
struct PluginExtension {
    meta: ExtensionMeta,
    /// Number of prompt templates the plugin provides.
    prompt_count: usize,
    /// Number of declarative tools the plugin provides.
    tool_count: usize,
}

#[async_trait::async_trait]
impl Extension for PluginExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            extension = %self.meta.name,
            prompts = self.prompt_count,
            tools = self.tool_count,
            "plugin extension initialized"
        );
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(extension = %self.meta.name, "plugin extension shutting down");
        Ok(())
    }

    async fn pre_inference(
        &self,
        request: &mut InferenceRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            plan_id = %request.plan_id,
            task = %request.task,
            "plugin pre_inference hook"
        );
        Ok(())
    }

    async fn post_inference(
        &self,
        response: &mut InferenceResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            plan_id = %response.plan_id,
            task = %response.task,
            success = response.success,
            "plugin post_inference hook"
        );
        Ok(())
    }

    async fn on_gate(
        &self,
        event: &mut GateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            gate = %event.gate_name,
            passed = event.passed,
            "plugin on_gate hook"
        );
        Ok(())
    }
}

// ─── ManifestExtension ──────────────────────────────────────────────────

/// An [`Extension`] backed by a declarative [`ExtensionManifest`] TOML file.
///
/// Handles the `Prompts`, `Config`, and `Declarative` tiers. `Wasm` and
/// `NativeRust` tiers are parsed but not executed — a warning is emitted at
/// init and the extension is treated as a no-op.
struct ManifestExtension {
    meta: ExtensionMeta,
    tier: PackageTier,
    timeout_ms: Option<u64>,
}

#[async_trait::async_trait]
impl Extension for ManifestExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            extension = %self.meta.name,
            tier = ?self.tier,
            timeout_ms = ?self.timeout_ms,
            "manifest extension initialized"
        );
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(extension = %self.meta.name, "manifest extension shutting down");
        Ok(())
    }
}

// ─── Manifest loading ───────────────────────────────────────────────────

/// Load an [`ExtensionManifest`] from a TOML file.
///
/// The TOML may use an `[extension]` wrapper section or have all fields at the
/// top level. At minimum `name`, `version`, `layer`, and `tier` are required.
///
/// # Errors
///
/// Returns an error if the file cannot be read, the TOML is malformed, or the
/// manifest fails schema validation.
pub fn load_extension_manifest(
    path: &Path,
) -> Result<ExtensionManifest, Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read extension manifest at {}: {e}",
            path.display()
        )
    })?;
    parse_extension_manifest(&content).map_err(|e| e.to_string().into())
}

/// Parse an [`ExtensionManifest`] from a TOML string.
///
/// Accepts both a flat manifest (fields at top level) and one with an
/// `[extension]` wrapper section.
///
/// # Errors
///
/// Returns a [`ManifestValidationError`] if the TOML is invalid or validation fails.
pub fn parse_extension_manifest(
    content: &str,
) -> Result<ExtensionManifest, ManifestValidationError> {
    // Try parsing as a wrapped `[extension]` table first.
    let manifest = if let Ok(wrapped) = toml::from_str::<WrappedManifest>(content) {
        wrapped.extension
    } else {
        // Fall back to flat parsing (all fields at top level).
        toml::from_str::<ExtensionManifest>(content).map_err(|e| ManifestValidationError {
            message: format!("failed to parse TOML: {e}"),
        })?
    };

    manifest.validate()?;
    Ok(manifest)
}

/// Intermediate deserialization type for `[extension]`-wrapped TOML files.
#[derive(serde::Deserialize)]
struct WrappedManifest {
    extension: ExtensionManifest,
}

// ─── Directory scanning ─────────────────────────────────────────────────

/// Scan `.roko/extensions/` subdirectories for `extension.toml` or
/// `manifest.toml` files and return the successfully parsed manifests.
///
/// Subdirectories are scanned in alphabetical order for deterministic load
/// ordering. Manifests that fail validation are skipped with a warning.
///
/// The `disable_extensions` list marks matching extensions as disabled
/// (field `LoadedExtension::disabled = true`). Callers decide whether to skip
/// them before adding to the chain.
pub fn scan_extension_manifests(
    extensions_dir: &Path,
    disable_extensions: &[String],
) -> Vec<LoadedExtension> {
    let mut results = Vec::new();

    if !extensions_dir.exists() {
        debug!(
            dir = %extensions_dir.display(),
            "extensions directory does not exist, skipping manifest scan"
        );
        return results;
    }

    let entries = match std::fs::read_dir(extensions_dir) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                dir = %extensions_dir.display(),
                error = %err,
                "failed to read extensions directory"
            );
            return results;
        }
    };

    // Collect and sort for deterministic load order.
    let mut dirs: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();
    dirs.sort_by_key(|e| e.file_name());

    for entry in dirs {
        let dir = entry.path();

        // Probe candidate filenames in preference order.
        let candidates = [dir.join("extension.toml"), dir.join("manifest.toml")];

        let found = candidates.iter().find(|p| p.exists());
        let (manifest_path, manifest) = match found {
            None => {
                debug!(dir = %dir.display(), "no extension.toml or manifest.toml found");
                continue;
            }
            Some(p) => match load_extension_manifest(p) {
                Ok(m) => (p.clone(), m),
                Err(e) => {
                    warn!(
                        path = %p.display(),
                        error = %e,
                        "skipping invalid extension manifest"
                    );
                    continue;
                }
            },
        };

        let disabled = disable_extensions.iter().any(|n| n == &manifest.name);

        if disabled {
            debug!(extension = %manifest.name, "extension is in disable_extensions list");
        } else {
            info!(
                extension = %manifest.name,
                version = %manifest.version,
                layer = ?manifest.layer,
                tier = ?manifest.tier,
                path = %manifest_path.display(),
                "loaded extension manifest"
            );
        }

        results.push(LoadedExtension {
            manifest,
            manifest_path,
            disabled,
        });
    }

    results
}

// ─── Loader ───────────────────────────���─────────────────────────────────

/// Scan extension directories and populate the given [`ExtensionChain`].
///
/// Scans (in order):
/// 1. `<workdir>/.roko/extensions/`
/// 2. `<workdir>/plugins/`
///
/// Each directory is probed via [`roko_plugin::manifest::discover_plugins`].
/// Discovered plugins are wrapped as [`PluginExtension`] and added to the
/// chain sorted by layer.
///
/// Additionally, if `extension_names` is non-empty (from `roko.toml`
/// `[agent].extensions`), only extensions whose name appears in that list are
/// loaded. An empty list means "load all discovered extensions".
///
/// Also scans for `extension.toml` / `manifest.toml` files in
/// `.roko/extensions/` subdirectories. See [`scan_extension_manifests`].
pub fn load_extensions(
    workdir: &Path,
    extension_names: &[String],
    chain: &mut ExtensionChain,
) -> usize {
    load_extensions_with_disabled(workdir, extension_names, &[], chain)
}

/// Like [`load_extensions`] but also accepts an explicit `disable_extensions`
/// list from the agent config (`[agent].disable_extensions` in `roko.toml`).
pub fn load_extensions_with_disabled(
    workdir: &Path,
    extension_names: &[String],
    disable_extensions: &[String],
    chain: &mut ExtensionChain,
) -> usize {
    let layout = RokoLayout::for_project(workdir);
    let extensions_dir = layout.extensions_dir();
    let plugin_dirs = [extensions_dir.clone(), workdir.join("plugins")];

    let mut loaded = 0usize;

    // ── 1. Plugin discovery (plugin.toml files) ──────────────────────────
    for dir in &plugin_dirs {
        // Call discover_plugins directly — it returns Ok(empty) for non-existent
        // directories, avoiding a separate .exists() TOCTOU check.
        let plugins = match roko_plugin::manifest::discover_plugins(dir) {
            Ok(p) if p.is_empty() => {
                debug!(dir = %dir.display(), "no plugins found in extension directory");
                continue;
            }
            Ok(p) => p,
            Err(err) => {
                warn!(
                    dir = %dir.display(),
                    error = %err,
                    "failed to scan extension directory"
                );
                continue;
            }
        };

        for plugin in plugins {
            let name = &plugin.manifest.plugin.name;

            // If an allow-list is configured, skip plugins not in it.
            if !extension_names.is_empty() && !extension_names.iter().any(|n| n == name) {
                debug!(
                    plugin = %name,
                    "plugin not in configured extensions list, skipping"
                );
                continue;
            }

            // Skip plugins in the deny-list.
            if disable_extensions.iter().any(|n| n == name) {
                debug!(plugin = %name, "plugin is in disable_extensions list, skipping");
                continue;
            }

            let ext = PluginExtension {
                meta: ExtensionMeta {
                    name: name.clone(),
                    layer: ExtensionLayer::Cognition, // default layer for plugin extensions
                    optional: true,                   // plugins should never be fatal
                    depends_on: plugin
                        .manifest
                        .dependencies
                        .iter()
                        .map(|d| d.name.clone())
                        .collect(),
                    soft_depends_on: Vec::new(),
                    version: plugin.manifest.plugin.version.clone(),
                },
                prompt_count: plugin.manifest.prompts.len(),
                tool_count: plugin.manifest.tools.len(),
            };

            info!(
                plugin = %name,
                version = %plugin.manifest.plugin.version,
                prompts = ext.prompt_count,
                tools = ext.tool_count,
                dir = %plugin.base_dir.display(),
                "loaded plugin extension"
            );

            chain.add(Box::new(ext));
            loaded += 1;
        }
    }

    // ── 2. Extension manifest discovery (extension.toml / manifest.toml) ─
    let manifests = scan_extension_manifests(&extensions_dir, disable_extensions);
    for loaded_ext in manifests {
        // Skip disabled extensions.
        if loaded_ext.disabled {
            debug!(
                extension = %loaded_ext.manifest.name,
                "skipping disabled extension"
            );
            continue;
        }

        let name = &loaded_ext.manifest.name;

        // If an allow-list is configured, skip extensions not in it.
        if !extension_names.is_empty() && !extension_names.iter().any(|n| n == name) {
            debug!(
                extension = %name,
                "extension not in configured extensions list, skipping"
            );
            continue;
        }

        // Warn about unsupported tiers but still register the extension so the
        // manifest is visible in chain metadata.
        match loaded_ext.manifest.tier {
            PackageTier::Wasm => {
                warn!(
                    extension = %name,
                    "WASM extension tier is not yet supported; \
                     extension will be registered as a no-op"
                );
            }
            PackageTier::NativeRust => {
                warn!(
                    extension = %name,
                    "NativeRust extension tier requires in-tree compilation; \
                     extension will be registered as a no-op"
                );
            }
            PackageTier::Prompts | PackageTier::Config | PackageTier::Declarative => {}
        }

        let timeout_ms = loaded_ext.manifest.timeout_ms;
        let tier = loaded_ext.manifest.tier;
        let meta = loaded_ext.manifest.into_meta();
        let ext = ManifestExtension {
            meta,
            tier,
            timeout_ms,
        };

        chain.add(Box::new(ext));
        loaded += 1;
    }

    if loaded > 0 {
        chain.sort_by_layer();
        info!(
            count = loaded,
            "extension chain populated from discovered plugins and manifests"
        );
    } else {
        debug!("no plugin extensions discovered");
    }

    loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_nonexistent_dirs_returns_zero() {
        let mut chain = ExtensionChain::new();
        let count = load_extensions(Path::new("/nonexistent/workspace"), &[], &mut chain);
        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn load_with_empty_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        std::fs::create_dir_all(&ext_dir).unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 0);
    }

    #[test]
    fn load_discovers_plugin_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path())
            .extensions_dir()
            .join("test-ext");
        std::fs::create_dir_all(&ext_dir).unwrap();

        std::fs::write(
            ext_dir.join("plugin.toml"),
            r#"
[plugin]
name = "test-ext"
version = "0.1.0"
description = "A test extension"

[[prompts]]
name = "test-prompt"
template = "Hello, world!"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.len(), 1);

        let meta = chain.metadata();
        assert_eq!(meta[0].name, "test-ext");
        assert_eq!(meta[0].version, "0.1.0");
        assert!(meta[0].optional);
    }

    #[test]
    fn load_respects_allow_list() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = RokoLayout::for_project(tmp.path());
        let ext_dir = layout.extensions_dir().join("allowed");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(
            ext_dir.join("plugin.toml"),
            r#"
[plugin]
name = "allowed-ext"
version = "0.1.0"
"#,
        )
        .unwrap();

        let skip_dir = layout.extensions_dir().join("skipped");
        std::fs::create_dir_all(&skip_dir).unwrap();
        std::fs::write(
            skip_dir.join("plugin.toml"),
            r#"
[plugin]
name = "skipped-ext"
version = "0.1.0"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &["allowed-ext".to_string()], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.metadata()[0].name, "allowed-ext");
    }

    // ── parse_extension_manifest ──────────────────────────────────────────

    #[test]
    fn parse_extension_manifest_wrapped_section() {
        let toml_str = r#"
[extension]
name = "my-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
optional = true
timeout_ms = 10000
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.name, "my-ext");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.layer, ExtensionLayer::Action);
        assert_eq!(manifest.tier, PackageTier::Declarative);
        assert!(manifest.optional);
        assert_eq!(manifest.timeout_ms, Some(10000));
    }

    #[test]
    fn parse_extension_manifest_flat() {
        let toml_str = r#"
name = "flat-ext"
version = "0.2.0"
layer = "cognition"
tier = "config"
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.name, "flat-ext");
        assert_eq!(manifest.tier, PackageTier::Config);
    }

    #[test]
    fn parse_extension_manifest_with_tags_and_depends() {
        let toml_str = r#"
[extension]
name = "tagged-ext"
version = "1.1.0"
layer = "meta"
tier = "prompts"
description = "A tagged extension"
tags = ["observability", "metrics"]
depends_on = ["base-ext"]
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.tags, vec!["observability", "metrics"]);
        assert_eq!(manifest.depends_on, vec!["base-ext"]);
        assert_eq!(manifest.description, "A tagged extension");
    }

    #[test]
    fn parse_extension_manifest_rejects_empty_name() {
        let toml_str = r#"
[extension]
name = ""
version = "1.0.0"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn parse_extension_manifest_rejects_invalid_semver() {
        let toml_str = r#"
[extension]
name = "bad-version"
version = "not-semver"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("semver"));
    }

    #[test]
    fn parse_extension_manifest_rejects_invalid_name_chars() {
        let toml_str = r#"
[extension]
name = "bad name!"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid characters")
        );
    }

    // ── load_extension_manifest (file I/O) ───────────────────────────────

    #[test]
    fn load_extension_manifest_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("extension.toml");
        std::fs::write(
            &manifest_path,
            r#"
[extension]
name = "file-ext"
version = "2.0.0"
layer = "social"
tier = "prompts"
"#,
        )
        .unwrap();

        let manifest = load_extension_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.name, "file-ext");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.tier, PackageTier::Prompts);
    }

    #[test]
    fn load_extension_manifest_nonexistent_file() {
        let result = load_extension_manifest(Path::new("/nonexistent/extension.toml"));
        assert!(result.is_err());
    }

    // ── scan_extension_manifests ─────────────────────────────────────────

    #[test]
    fn scan_extension_manifests_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        std::fs::create_dir_all(&ext_dir).unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_extension_manifests_nonexistent_dir() {
        let results = scan_extension_manifests(Path::new("/nonexistent/extensions"), &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_extension_manifests_discovers_extension_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("my-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "my-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "my-ext");
        assert!(!results[0].disabled);
        assert!(results[0].manifest_path.ends_with("extension.toml"));
    }

    #[test]
    fn scan_extension_manifests_discovers_manifest_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("other-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("manifest.toml"),
            r#"
[extension]
name = "other-ext"
version = "0.5.0"
layer = "memory"
tier = "config"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "other-ext");
        assert!(results[0].manifest_path.ends_with("manifest.toml"));
    }

    #[test]
    fn scan_extension_manifests_prefers_extension_toml_over_manifest_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("dual-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        // Write both files — extension.toml should win.
        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "extension-wins"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();
        std::fs::write(
            ext_subdir.join("manifest.toml"),
            r#"
[extension]
name = "manifest-loses"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "extension-wins");
    }

    #[test]
    fn scan_extension_manifests_marks_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("disabled-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "disabled-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &["disabled-ext".to_string()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].disabled);
    }

    #[test]
    fn scan_extension_manifests_skips_invalid_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();

        // Write an invalid manifest (empty name).
        let bad_subdir = ext_dir.join("bad-ext");
        std::fs::create_dir_all(&bad_subdir).unwrap();
        std::fs::write(
            bad_subdir.join("extension.toml"),
            r#"
[extension]
name = ""
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        // Write a valid manifest.
        let good_subdir = ext_dir.join("good-ext");
        std::fs::create_dir_all(&good_subdir).unwrap();
        std::fs::write(
            good_subdir.join("extension.toml"),
            r#"
[extension]
name = "good-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "good-ext");
    }

    // ── load_extensions_with_disabled (manifest integration) ─────────────

    #[test]
    fn load_extensions_discovers_extension_toml_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("my-manifest-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "my-manifest-ext"
version = "1.0.0"
layer = "cognition"
tier = "declarative"
optional = true
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 1);
        let meta = chain.metadata();
        assert_eq!(meta[0].name, "my-manifest-ext");
        assert!(meta[0].optional);
    }

    #[test]
    fn load_extensions_with_disabled_skips_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("disabled-manifest-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "disabled-manifest-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions_with_disabled(
            tmp.path(),
            &[],
            &["disabled-manifest-ext".to_string()],
            &mut chain,
        );
        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn load_extensions_allow_list_filters_manifest_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();

        // Write two manifests.
        for (name, version) in [("allowed-mext", "1.0.0"), ("other-mext", "1.0.0")] {
            let subdir = ext_dir.join(name);
            std::fs::create_dir_all(&subdir).unwrap();
            std::fs::write(
                subdir.join("extension.toml"),
                format!(
                    r#"
[extension]
name = "{name}"
version = "{version}"
layer = "action"
tier = "declarative"
"#
                ),
            )
            .unwrap();
        }

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &["allowed-mext".to_string()], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.metadata()[0].name, "allowed-mext");
    }
}
