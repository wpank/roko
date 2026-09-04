//! Per-tab visual effects configuration.

use std::path::Path;

/// Presets for the visual effects stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectsPreset {
    /// Disable the new visual effects stack.
    Off,
    /// Enable restrained self-glow and sparse ambient particles.
    #[default]
    Minimal,
    /// Add a state-driven background field to the minimal treatment.
    Full,
}

impl EffectsPreset {
    /// Return the next preset in the cycle order.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::Minimal,
            Self::Minimal => Self::Full,
            Self::Full => Self::Off,
        }
    }

    /// Short user-facing label for notifications and logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Minimal => "Minimal",
            Self::Full => "Full",
        }
    }

    /// String value written to `roko.toml`.
    #[must_use]
    pub const fn as_toml_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Full => "full",
        }
    }

    /// Parse a preset from a TOML string.
    #[must_use]
    pub fn from_str(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "off" => Some(Self::Off),
            "minimal" => Some(Self::Minimal),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

/// Controls which post-processing effects are enabled.
#[derive(Debug, Clone)]
pub struct EffectsConfig {
    /// Master switch for all screen-level post-processing.
    pub screen_postfx: bool,
    /// Preset driving the new state-driven visual effects.
    pub preset: EffectsPreset,
    /// Enable NervViz guide-line/rain overlays.
    pub nerv_viz: bool,
    /// Enable floating particle dots.
    pub particles: bool,
    /// Enable bloom (glow bleed from bright cells). Off by default for performance.
    pub bloom_enabled: bool,
    /// Enable drop shadows behind panels.
    pub shadows_enabled: bool,
    /// Enable ambient VFX (orbs, atmosphere, color grading).
    pub vfx_enabled: bool,
    /// Bloom intensity multiplier (0.0..1.0).
    pub bloom_intensity: f64,
    /// Vignette intensity (0.0..1.0).
    pub vignette_intensity: f64,
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self::from_preset(EffectsPreset::Minimal)
    }
}

impl EffectsConfig {
    /// All effects disabled.
    #[must_use]
    pub fn none() -> Self {
        Self::from_preset(EffectsPreset::Off)
    }

    /// Build a config from a preset.
    #[must_use]
    pub fn from_preset(preset: EffectsPreset) -> Self {
        let mut config = Self {
            screen_postfx: !matches!(preset, EffectsPreset::Off),
            preset,
            nerv_viz: false,
            particles: false,
            bloom_enabled: false,
            shadows_enabled: false,
            vfx_enabled: false,
            bloom_intensity: 0.0,
            vignette_intensity: 0.0,
        };
        config.apply_preset(preset);
        config
    }

    /// Update the preset and derived effect flags in place.
    pub fn set_preset(&mut self, preset: EffectsPreset) {
        self.apply_preset(preset);
    }

    /// Cycle to the next preset and return the newly selected preset.
    pub fn cycle_preset(&mut self) -> EffectsPreset {
        let next = self.preset.next();
        self.apply_preset(next);
        next
    }

    /// Persist the current preset into `roko.toml`.
    pub fn save_preset(&self, root: &Path) -> Result<(), String> {
        save_preset_to_root(root, self.preset)
    }

    /// Load TUI effects from `roko.toml`, falling back to defaults on error.
    ///
    /// If `ROKO_REDUCED_MOTION` is set, forces all animations and effects off.
    #[must_use]
    pub fn load_from_root(root: &Path) -> Self {
        // Respect reduced-motion: disable all effects.
        if std::env::var_os("ROKO_REDUCED_MOTION").is_some() {
            return Self::none();
        }

        let mut config = Self::default();
        let Ok(content) = std::fs::read_to_string(root.join("roko.toml")) else {
            return config;
        };
        let Ok(value) = content.parse::<toml::Value>() else {
            return config;
        };

        if let Some(preset) = string_at_path(&value, &["tui", "effects", "preset"])
            .and_then(|preset| EffectsPreset::from_str(&preset))
        {
            config.apply_preset(preset);
            // A persisted preset establishes the screen-effect default for a
            // fresh session. An explicit screen_postfx value below can still
            // override it, while in-session preset cycling keeps Ctrl-E
            // independent.
            config.screen_postfx = !matches!(preset, EffectsPreset::Off);
        }
        config.screen_postfx = bool_at_path(&value, &["tui", "effects", "screen_postfx"])
            .unwrap_or(config.screen_postfx);

        config
    }

    /// Check if reduced-motion mode is active (via env var).
    #[must_use]
    pub fn is_reduced_motion() -> bool {
        std::env::var_os("ROKO_REDUCED_MOTION").is_some()
    }

    fn apply_preset(&mut self, preset: EffectsPreset) {
        self.preset = preset;
        self.screen_postfx = !matches!(preset, EffectsPreset::Off);
        self.nerv_viz = matches!(preset, EffectsPreset::Full);
        // Mori's default treatment keeps the interface still and legible.
        // Character particles are an explicit Full-preset flourish.
        self.particles = matches!(preset, EffectsPreset::Full);
        // Full preset enables the remaining dormant effects.
        self.bloom_enabled = matches!(preset, EffectsPreset::Full);
        self.shadows_enabled = matches!(preset, EffectsPreset::Full);
        self.vfx_enabled = matches!(preset, EffectsPreset::Full);
        self.bloom_intensity = if matches!(preset, EffectsPreset::Full) {
            0.15
        } else {
            0.0
        };
        self.vignette_intensity = if matches!(preset, EffectsPreset::Full) {
            0.20
        } else {
            0.0
        };
    }
}

/// Save the selected effects preset into `roko.toml`.
pub fn save_preset_to_root(root: &Path, preset: EffectsPreset) -> Result<(), String> {
    let config_path = root.join("roko.toml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(format!("read roko.toml: {err}")),
    };

    let mut document = if content.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("parse roko.toml: {e}"))?
    };

    let preset_item = &mut document["tui"]["effects"]["preset"];
    if let Some(existing) = preset_item.as_value_mut() {
        // Replacing an Item discards its inline decoration. Preserve the
        // existing whitespace/comment so cycling effects does not rewrite
        // an operator-maintained config file beyond the requested value.
        let decor = existing.decor().clone();
        *existing = toml_edit::Value::from(preset.as_toml_value());
        *existing.decor_mut() = decor;
    } else {
        *preset_item = toml_edit::value(preset.as_toml_value());
    }

    std::fs::write(&config_path, document.to_string())
        .map_err(|e| format!("write roko.toml: {e}"))?;
    Ok(())
}

fn bool_at_path(value: &toml::Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for segment in path {
        current = current.as_table()?.get(*segment)?;
    }
    current.as_bool()
}

fn string_at_path(value: &toml::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.as_table()?.get(*segment)?;
    }
    current.as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn preset_cycles_and_derives_flags() {
        let mut config = EffectsConfig::default();
        assert_eq!(config.preset, EffectsPreset::Minimal);
        assert!(config.screen_postfx);
        assert!(!config.nerv_viz);
        assert!(!config.particles);

        assert_eq!(config.cycle_preset(), EffectsPreset::Full);
        assert!(config.screen_postfx);
        assert!(config.nerv_viz);
        assert!(config.particles);
        assert!(config.bloom_enabled);
        assert!(config.shadows_enabled);
        assert!(config.vfx_enabled);

        assert_eq!(config.cycle_preset(), EffectsPreset::Off);
        assert!(!config.screen_postfx);
        assert!(!config.nerv_viz);
        assert!(!config.particles);
        assert!(!config.bloom_enabled);
        assert!(!config.shadows_enabled);
        assert!(!config.vfx_enabled);
    }

    #[test]
    fn load_prefers_preset_and_save_writes_preset() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "[tui.effects]\npreset = \"minimal\"\nscreen_postfx = true\n",
        )
        .expect("write roko.toml");

        let config = EffectsConfig::load_from_root(dir.path());
        assert_eq!(config.preset, EffectsPreset::Minimal);
        assert!(config.screen_postfx);
        assert!(!config.nerv_viz);
        assert!(!config.particles);

        config
            .save_preset(dir.path())
            .expect("save preset to roko.toml");
        let saved = std::fs::read_to_string(dir.path().join("roko.toml")).expect("read back");
        assert!(saved.contains("preset = \"minimal\""));
    }

    #[test]
    fn save_preserves_comments_and_existing_order() {
        let dir = tempdir().expect("tempdir");
        let original = "# operator note\n[project]\nname = \"demo\"\n\n[tui.effects]\n# keep this note\npreset = \"minimal\" # inline\nscreen_postfx = true\n";
        std::fs::write(dir.path().join("roko.toml"), original).expect("write roko.toml");

        save_preset_to_root(dir.path(), EffectsPreset::Full).expect("save full preset");
        let saved = std::fs::read_to_string(dir.path().join("roko.toml")).expect("read back");

        assert!(saved.starts_with("# operator note\n[project]"));
        assert!(saved.contains("# keep this note"));
        assert!(saved.contains("preset = \"full\" # inline"));
        assert!(saved.find("[project]") < saved.find("[tui.effects]"));
    }

    #[test]
    fn persisted_preset_sets_screen_default_without_overriding_explicit_toggle() {
        let dir = tempdir().expect("tempdir");

        std::fs::write(
            dir.path().join("roko.toml"),
            "[tui.effects]\npreset = \"off\"\n",
        )
        .expect("write off preset");
        assert!(!EffectsConfig::load_from_root(dir.path()).screen_postfx);

        std::fs::write(
            dir.path().join("roko.toml"),
            "[tui.effects]\npreset = \"full\"\n",
        )
        .expect("write full preset");
        assert!(EffectsConfig::load_from_root(dir.path()).screen_postfx);

        std::fs::write(
            dir.path().join("roko.toml"),
            "[tui.effects]\npreset = \"full\"\nscreen_postfx = false\n",
        )
        .expect("write explicit screen toggle");
        assert!(!EffectsConfig::load_from_root(dir.path()).screen_postfx);
    }
}
