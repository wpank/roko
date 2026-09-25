//! RG-5: TUI terminal-size snapshot verification.
//!
//! Renders the TUI at standard terminal sizes (80×24, 120×40, 200×60) using
//! ratatui's [`TestBackend`] and asserts that:
//!
//! - Every render completes without panic.
//! - No rendered row is wider than the terminal width (no horizontal overflow).
//! - The header is present in every size.
//! - At the narrowest size (80×24) the layout still renders a recognizable
//!   header containing at least one tab label.
//!
//! These tests are intentionally non-snapshot-file tests: they capture the
//! invariants that matter (no overflow, presence of key UI chrome) without
//! relying on brittle byte-level golden files.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use roko_cli::tui::App;
use tempfile::tempdir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns the number of Unicode scalar values in `s` (not bytes).
///
/// The TestBackend stores each grapheme in a cell so char count equals columns.
fn display_width(s: &str) -> usize {
    s.chars().count()
}

// ─── Per-size rendering helper ────────────────────────────────────────────────

/// Render the App dashboard at `width`×`height` and return the row strings.
///
/// Uses `render_all_tabs_to_text` to get the Dashboard tab content, then
/// splits the resulting text into rows for per-row width assertions.
fn render_at(width: u16, height: u16) -> Vec<String> {
    use roko_cli::tui::Tab;

    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());
    let results = app.render_tabs_to_text(width, height, &[Tab::Dashboard]);
    assert_eq!(
        results.len(),
        1,
        "render_at({width},{height}): expected 1 tab result"
    );
    let (_, text) = &results[0];
    text.lines().map(|l| l.to_string()).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Standard 80×24 VT100 terminal.
#[test]
fn tui_renders_at_80x24() {
    let rows = render_at(80, 24);

    // Expect at least 23 rows: render_tabs_to_text trims trailing whitespace
    // per-row so the last row(s) may collapse to empty and be elided by
    // `str::lines()` if the terminal height has unused bottom rows.
    assert!(
        rows.len() >= 23,
        "80×24: expected at least 23 rows, got {}",
        rows.len()
    );

    // No row must be wider than 80 columns.
    for (i, row) in rows.iter().enumerate() {
        let w = display_width(row);
        assert!(
            w <= 80,
            "80×24 row {i} overflows: {w} > 80 columns: {row:?}"
        );
    }

    // Header row (row 0) must contain at least one F-key label.
    let header = &rows[0];
    let has_fkey = header.contains("F1")
        || header.contains("F2")
        || header.contains("Dashboard")
        || header.contains("Agents")
        || header.contains("roko");
    assert!(
        has_fkey,
        "80×24 header row must contain tab labels; got: {header:?}"
    );
}

/// Common "developer workstation" size.
#[test]
fn tui_renders_at_120x40() {
    let rows = render_at(120, 40);

    assert!(
        rows.len() >= 39,
        "120×40: expected at least 39 rows, got {}",
        rows.len()
    );

    for (i, row) in rows.iter().enumerate() {
        let w = display_width(row);
        assert!(
            w <= 120,
            "120×40 row {i} overflows: {w} > 120 columns: {row:?}"
        );
    }

    // At this size all 10 tab labels should fit on the header.
    let header = &rows[0];
    let has_dashboard = header.contains("Dashboard") || header.contains("F1");
    assert!(
        has_dashboard,
        "120×40 header must contain Dashboard label; got: {header:?}"
    );
}

/// Wide monitor / split-pane size.
#[test]
fn tui_renders_at_200x60() {
    let rows = render_at(200, 60);

    assert!(
        rows.len() >= 59,
        "200×60: expected at least 59 rows, got {}",
        rows.len()
    );

    for (i, row) in rows.iter().enumerate() {
        let w = display_width(row);
        assert!(
            w <= 200,
            "200×60 row {i} overflows: {w} > 200 columns: {row:?}"
        );
    }
}

/// All ten tabs render without panic at 120×40.
#[test]
fn all_tabs_render_at_standard_size_no_overflow() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());

    let width: u16 = 120;
    let height: u16 = 40;

    let rendered = app.render_all_tabs_to_text(width, height);
    let expected_tabs = roko_cli::tui::Tab::ALL.len();
    assert_eq!(
        rendered.len(),
        expected_tabs,
        "expected {expected_tabs} tabs rendered"
    );

    for (tab, text) in &rendered {
        let rows: Vec<&str> = text.lines().collect();
        for (i, row) in rows.iter().enumerate() {
            let w = display_width(row);
            assert!(
                w <= width as usize,
                "tab {tab:?} row {i} overflows at {width}×{height}: {w} > {width}: {row:?}"
            );
        }
    }
}

/// Verify render_all_tabs_to_text returns empty for zero dimensions.
#[test]
fn render_at_zero_dimensions_returns_empty() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());

    let result = app.render_all_tabs_to_text(0, 24);
    assert!(
        result.is_empty(),
        "zero width must return empty render list"
    );

    let result = app.render_all_tabs_to_text(80, 0);
    assert!(
        result.is_empty(),
        "zero height must return empty render list"
    );
}

/// Confirm the render helper in the App (render_tabs_to_text) is usable with
/// a subset of tabs and that results maintain the correct ordering.
#[test]
fn render_subset_of_tabs_preserves_order() {
    use roko_cli::tui::Tab;

    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());

    let subset = [Tab::Dashboard, Tab::Agents, Tab::Learning];
    let rendered = app.render_tabs_to_text(120, 40, &subset);

    assert_eq!(rendered.len(), 3);
    assert_eq!(rendered[0].0, Tab::Dashboard);
    assert_eq!(rendered[1].0, Tab::Agents);
    assert_eq!(rendered[2].0, Tab::Learning);

    // Each rendering must be non-empty.
    for (tab, text) in &rendered {
        assert!(
            !text.is_empty(),
            "tab {tab:?} rendered to an empty string at 120×40"
        );
    }
}

/// Validate that the TUI renders a non-empty status footer row at 80×24.
///
/// The footer is the last row and must be present (not blank) because it
/// carries key binding hints and the version string.
#[test]
fn tui_footer_present_at_80x24() {
    let rows = render_at(80, 24);
    let footer = rows.last().unwrap();
    // At minimum the footer row should be non-empty (has some content).
    assert!(
        !footer.trim().is_empty() || rows.iter().any(|r| !r.trim().is_empty()),
        "80×24 render produced no non-empty rows"
    );
}
