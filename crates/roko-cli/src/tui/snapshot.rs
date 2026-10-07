//! Headless TUI snapshot engine.
//!
//! Renders every TUI tab to text using `ratatui::backend::TestBackend` so that
//! AI agents (and humans in non-interactive contexts) can inspect the dashboard
//! without a real terminal.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::Serialize;

use super::app::App;
use super::screenshot_diff::{buffer_to_ansi, buffer_to_text};
use super::tabs::Tab;

const MIN_SNAPSHOT_WIDTH: u16 = 40;
const MIN_SNAPSHOT_HEIGHT: u16 = 12;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for a snapshot capture run.
pub struct SnapshotConfig {
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Directory to write snapshot files into.
    pub output_dir: PathBuf,
    /// If set, capture only these tabs (lowercase labels).
    pub tabs: Option<Vec<String>>,
    /// Human-readable label for this snapshot.
    pub label: Option<String>,
    /// Also write each tab with its colours and modifiers, as ANSI escapes,
    /// to `<tab>.ansi` beside its `.txt`.
    pub ansi: bool,
}

/// Result of a completed snapshot capture.
pub struct SnapshotResult {
    /// Directory where files were written.
    pub dir: PathBuf,
    /// Number of tabs captured.
    pub tabs_captured: usize,
    /// Path to the manifest file.
    pub manifest_path: PathBuf,
    /// The `.txt` file name of each captured tab, in capture order.
    pub files: Vec<String>,
}

/// Manifest entry for one captured tab.
#[derive(Debug, Serialize)]
pub struct TabEntry {
    tab: String,
    fkey: String,
    file: String,
    /// The tab's `.ansi` file, when the capture wrote one.
    #[serde(skip_serializing_if = "Option::is_none")]
    ansi_file: Option<String>,
}

/// Top-level manifest written alongside the snapshot files.
#[derive(Debug, Serialize)]
pub struct Manifest {
    schema_version: u32,
    renderer: String,
    timestamp: String,
    label: Option<String>,
    width: u16,
    height: u16,
    tabs: Vec<TabEntry>,
}

// ---------------------------------------------------------------------------
// Core engine
// ---------------------------------------------------------------------------

/// Filename for a tab snapshot (e.g. `f01-dashboard.txt`).
fn tab_filename(tab: Tab) -> String {
    let num = tab.index() + 1;
    let name = tab.label().to_ascii_lowercase();
    format!("f{num:02}-{name}.txt")
}

/// Run the full snapshot capture, writing text files and a manifest.
pub fn capture_snapshots(workdir: &Path, config: &SnapshotConfig) -> Result<SnapshotResult> {
    anyhow::ensure!(
        config.width >= MIN_SNAPSHOT_WIDTH,
        "snapshot width must be at least {MIN_SNAPSHOT_WIDTH} columns (got {})",
        config.width
    );
    anyhow::ensure!(
        config.height >= MIN_SNAPSHOT_HEIGHT,
        "snapshot height must be at least {MIN_SNAPSHOT_HEIGHT} rows (got {})",
        config.height
    );

    std::fs::create_dir_all(&config.output_dir).with_context(|| {
        format!(
            "failed to create output dir: {}",
            config.output_dir.display()
        )
    })?;

    // Build a headless App to get properly initialized state.
    let mut app = App::new(workdir);
    app.prepare_headless_capture();
    let tabs_to_capture = resolve_tabs(&config.tabs)?;
    let rendered = app.render_tabs_to_buffers(config.width, config.height, &tabs_to_capture);

    let mut entries = Vec::new();

    for (tab, buffer) in rendered {
        let filename = tab_filename(tab);
        let path = config.output_dir.join(&filename);
        std::fs::write(&path, buffer_to_text(&buffer))
            .with_context(|| format!("failed to write {}", path.display()))?;
        let ansi_file = if config.ansi {
            let ansi_name = format!("{}.ansi", filename.trim_end_matches(".txt"));
            let ansi_path = config.output_dir.join(&ansi_name);
            std::fs::write(&ansi_path, buffer_to_ansi(&buffer))
                .with_context(|| format!("failed to write {}", ansi_path.display()))?;
            Some(ansi_name)
        } else {
            None
        };

        entries.push(TabEntry {
            tab: tab.label().to_string(),
            fkey: format!("F{}", tab.index() + 1),
            file: filename,
            ansi_file,
        });
    }
    let files = entries.iter().map(|entry| entry.file.clone()).collect();

    let manifest = Manifest {
        schema_version: 2,
        renderer: "app.draw/full-frame".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        label: config.label.clone(),
        width: config.width,
        height: config.height,
        tabs: entries,
    };

    let manifest_path = config.output_dir.join("manifest.json");
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
    std::fs::write(&manifest_path, &manifest_json)
        .with_context(|| format!("failed to write manifest: {}", manifest_path.display()))?;

    Ok(SnapshotResult {
        tabs_captured: tabs_to_capture.len(),
        dir: config.output_dir.clone(),
        manifest_path,
        files,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve which tabs to capture based on an optional filter list.
fn resolve_tabs(filter: &Option<Vec<String>>) -> Result<Vec<Tab>> {
    let Some(filter) = filter else {
        return Ok(Tab::ALL.to_vec());
    };

    anyhow::ensure!(!filter.is_empty(), "at least one tab must be requested");
    let normalized = filter
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    anyhow::ensure!(!normalized.is_empty(), "at least one tab must be requested");

    let tabs = Tab::ALL
        .iter()
        .filter(|tab| {
            let label = tab.label().to_ascii_lowercase();
            let fkey = format!("f{}", tab.index() + 1);
            normalized
                .iter()
                .any(|item| item == &label || item == &fkey)
        })
        .copied()
        .collect::<Vec<_>>();

    let unknown = normalized
        .iter()
        .filter(|item| {
            !Tab::ALL.iter().any(|tab| {
                item.as_str() == tab.label().to_ascii_lowercase()
                    || item.as_str() == format!("f{}", tab.index() + 1)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    anyhow::ensure!(
        unknown.is_empty(),
        "unknown snapshot tab selector(s): {}; available: {}",
        unknown.join(", "),
        Tab::ALL
            .iter()
            .map(|tab| format!("f{}|{}", tab.index() + 1, tab.label().to_ascii_lowercase()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    anyhow::ensure!(!tabs.is_empty(), "no snapshot tabs matched the request");
    Ok(tabs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn tab_filename_format() {
        assert_eq!(tab_filename(Tab::Dashboard), "f01-dashboard.txt");
        assert_eq!(tab_filename(Tab::Plans), "f02-plans.txt");
        assert_eq!(tab_filename(Tab::Learning), "f09-learning.txt");
        assert_eq!(tab_filename(Tab::Providers), "f10-providers.txt");
    }

    #[test]
    fn resolve_tabs_none_returns_all() {
        let tabs = resolve_tabs(&None).unwrap();
        assert_eq!(tabs.len(), 10);
    }

    #[test]
    fn resolve_tabs_filters_by_label() {
        let tabs = resolve_tabs(&Some(vec!["dashboard".to_string(), "plans".to_string()])).unwrap();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0], Tab::Dashboard);
        assert_eq!(tabs[1], Tab::Plans);
    }

    #[test]
    fn resolve_tabs_filters_by_fkey() {
        let filter = vec!["f1".to_string(), "f9".to_string(), "f10".to_string()];
        let tabs = resolve_tabs(&Some(filter)).unwrap();
        assert_eq!(tabs, [Tab::Dashboard, Tab::Learning, Tab::Providers]);
    }

    #[test]
    fn rejects_unknown_tabs_and_unrenderable_dimensions() {
        assert!(resolve_tabs(&Some(vec!["bogus".to_string()])).is_err());

        let dir = tempdir().unwrap();
        let config = SnapshotConfig {
            width: 0,
            height: 0,
            output_dir: dir.path().join("shots"),
            tabs: None,
            label: None,
            ansi: false,
        };
        assert!(capture_snapshots(dir.path(), &config).is_err());
    }

    #[test]
    fn capture_uses_full_app_frame() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path().join("shots");
        let config = SnapshotConfig {
            width: 120,
            height: 30,
            output_dir: output_dir.clone(),
            tabs: Some(vec!["f1".to_string()]),
            label: Some("full-frame".to_string()),
            ansi: false,
        };

        let result = capture_snapshots(dir.path(), &config).unwrap();
        assert_eq!(result.tabs_captured, 1);
        let text = std::fs::read_to_string(output_dir.join("f01-dashboard.txt")).unwrap();
        assert!(text.contains("F1:dash"), "global tab bar missing: {text}");
        assert_eq!(text.split('\n').count(), 30);

        let manifest = std::fs::read_to_string(result.manifest_path).unwrap();
        assert!(manifest.contains("\"schema_version\": 2"));
        assert!(manifest.contains("app.draw/full-frame"));
    }

    #[test]
    fn capture_writes_ansi_beside_text_when_asked() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path().join("shots");
        let config = SnapshotConfig {
            width: 120,
            height: 30,
            output_dir: output_dir.clone(),
            tabs: Some(vec!["f1".to_string()]),
            label: None,
            ansi: true,
        };

        let result = capture_snapshots(dir.path(), &config).unwrap();
        assert_eq!(result.files, ["f01-dashboard.txt"]);
        let text = std::fs::read_to_string(output_dir.join("f01-dashboard.txt")).unwrap();
        assert!(!text.contains('\x1b'), "the .txt capture stays plain");
        let ansi = std::fs::read_to_string(output_dir.join("f01-dashboard.ansi")).unwrap();
        // Every row ends in a reset, and without its escapes the `.ansi`
        // file holds the same rows as the `.txt` one.
        assert!(ansi.lines().all(|row| row.ends_with("\x1b[0m")));
        let escapes = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
        let plain = escapes.replace_all(&ansi, "");
        let plain_rows: Vec<&str> = plain.lines().map(str::trim_end).collect();
        assert_eq!(plain_rows, text.split('\n').collect::<Vec<_>>());
        let manifest = std::fs::read_to_string(result.manifest_path).unwrap();
        assert!(manifest.contains("\"ansi_file\": \"f01-dashboard.ansi\""));
    }
}
