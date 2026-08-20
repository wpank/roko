//! Headless TUI snapshot engine.
//!
//! Renders every TUI tab to text using `ratatui::backend::TestBackend` so that
//! AI agents (and humans in non-interactive contexts) can inspect the dashboard
//! without a real terminal.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde::Serialize;

use super::app::App;
use super::dashboard::DashboardData;
use super::state::TuiState;
use super::tabs::Tab;
use super::views::{self, ViewState};

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
}

/// Result of a completed snapshot capture.
pub struct SnapshotResult {
    /// Directory where files were written.
    pub dir: PathBuf,
    /// Number of tabs captured.
    pub tabs_captured: usize,
    /// Path to the manifest file.
    pub manifest_path: PathBuf,
}

/// Manifest entry for one captured tab.
#[derive(Debug, Serialize)]
struct TabEntry {
    tab: String,
    fkey: String,
    file: String,
}

/// Top-level manifest written alongside the snapshot files.
#[derive(Debug, Serialize)]
struct Manifest {
    timestamp: String,
    label: Option<String>,
    width: u16,
    height: u16,
    tabs: Vec<TabEntry>,
}

// ---------------------------------------------------------------------------
// Core engine
// ---------------------------------------------------------------------------

/// Extract the visible text content from a `TestBackend` terminal buffer.
///
/// Each row is joined into a single string and trailing whitespace is trimmed.
/// Rows are separated by newlines.
pub fn buffer_to_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content
        .chunks(width)
        .map(|row| {
            let line: String = row.iter().map(|cell| cell.symbol()).collect();
            line.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Filename for a tab snapshot (e.g. `f01-dashboard.txt`).
fn tab_filename(tab: Tab) -> String {
    let num = tab.index() + 1;
    let name = tab.label().to_ascii_lowercase();
    format!("f{num:02}-{name}.txt")
}

/// Run the full snapshot capture, writing text files and a manifest.
pub fn capture_snapshots(workdir: &Path, config: &SnapshotConfig) -> Result<SnapshotResult> {
    std::fs::create_dir_all(&config.output_dir).with_context(|| {
        format!(
            "failed to create output dir: {}",
            config.output_dir.display()
        )
    })?;

    // Build a headless App to get properly initialized state.
    let app = App::new(workdir);
    let data = &app.data;
    let tui_state = &app.tui_state;

    let tabs_to_capture = resolve_tabs(&config.tabs);

    let mut entries = Vec::new();

    for &tab in &tabs_to_capture {
        let text = render_tab_to_text(config.width, config.height, tab, data, tui_state);

        let filename = tab_filename(tab);
        let path = config.output_dir.join(&filename);
        std::fs::write(&path, &text)
            .with_context(|| format!("failed to write {}", path.display()))?;

        entries.push(TabEntry {
            tab: tab.label().to_string(),
            fkey: format!("F{}", tab.index() + 1),
            file: filename,
        });
    }

    let manifest = Manifest {
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
    })
}

/// Render a single tab to text using a headless `TestBackend`.
pub fn render_tab_to_text(
    width: u16,
    height: u16,
    tab: Tab,
    data: &DashboardData,
    tui_state: &TuiState,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal creation cannot fail");

    let view_state = ViewState {
        scroll: 0,
        selected: 0,
        sub_tab: 0,
        secondary_selected: 0,
        auto_tail: false,
        search_query: String::new(),
    };

    let theme = super::dashboard::Theme::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            views::render_tab_content(frame, area, tab, data, tui_state, &view_state, &theme);
        })
        .expect("TestBackend draw cannot fail");

    buffer_to_text(&terminal)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve which tabs to capture based on an optional filter list.
fn resolve_tabs(filter: &Option<Vec<String>>) -> Vec<Tab> {
    let Some(filter) = filter else {
        return Tab::ALL.to_vec();
    };

    Tab::ALL
        .iter()
        .filter(|tab| {
            let label = tab.label().to_ascii_lowercase();
            let fkey = format!("f{}", tab.index() + 1);
            filter.iter().any(|f| {
                let f = f.to_ascii_lowercase();
                f == label || f == fkey
            })
        })
        .copied()
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_to_text_extracts_content() {
        let backend = TestBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let block = ratatui::widgets::Block::default()
                    .title("Hi")
                    .borders(ratatui::widgets::Borders::ALL);
                frame.render_widget(block, area);
            })
            .unwrap();

        let text = buffer_to_text(&terminal);
        assert!(
            text.contains("Hi"),
            "expected 'Hi' in rendered text: {text}"
        );
    }

    #[test]
    fn tab_filename_format() {
        assert_eq!(tab_filename(Tab::Dashboard), "f01-dashboard.txt");
        assert_eq!(tab_filename(Tab::Plans), "f02-plans.txt");
        assert_eq!(tab_filename(Tab::Learning), "f10-learning.txt");
    }

    #[test]
    fn resolve_tabs_none_returns_all() {
        let tabs = resolve_tabs(&None);
        assert_eq!(tabs.len(), 10);
    }

    #[test]
    fn resolve_tabs_filters_by_label() {
        let tabs = resolve_tabs(&Some(vec!["dashboard".to_string(), "plans".to_string()]));
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0], Tab::Dashboard);
        assert_eq!(tabs[1], Tab::Plans);
    }

    #[test]
    fn resolve_tabs_filters_by_fkey() {
        let tabs = resolve_tabs(&Some(vec!["f1".to_string(), "f10".to_string()]));
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0], Tab::Dashboard);
        assert_eq!(tabs[1], Tab::Learning);
    }

    #[test]
    fn render_tab_produces_nonempty_text() {
        let data = DashboardData::default();
        let tui_state = TuiState::new();
        let text = render_tab_to_text(80, 24, Tab::Dashboard, &data, &tui_state);
        assert!(!text.is_empty(), "rendered text should not be empty");
    }
}
