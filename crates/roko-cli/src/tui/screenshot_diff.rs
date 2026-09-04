//! Screenshot diff comparison for TUI visual regression testing.
//!
//! Compares two TUI snapshots -- either text-based `.txt` captures or (when
//! the `tui-png` feature is enabled) pixel-level PNG images -- and produces a
//! structured diff report.
//!
//! The text-based backend works without any image dependencies and serves as
//! the default comparison path. It compares ANSI-stripped character grids cell
//! by cell, reporting character-level differences and the regions where they
//! cluster.
//!
//! # Feature gate
//!
//! The full module is gated behind `#[cfg(feature = "tui-png")]`. The
//! text-based comparison functions are additionally available through the
//! [`crate::tui::snapshot`] module's public API for use without the feature
//! flag -- see [`compare_text`].

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context as _, Result};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A rectangular region where differences were detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRegion {
    /// Top-left column (0-indexed, character cells for text mode).
    pub x: usize,
    /// Top-left row (0-indexed).
    pub y: usize,
    /// Width of the region in columns/pixels.
    pub width: usize,
    /// Height of the region in rows/pixels.
    pub height: usize,
    /// Number of differing cells/pixels within this region.
    pub diff_count: usize,
}

/// Comparison mode used to produce the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    /// Character-by-character comparison of stripped text grids.
    Text,
    /// Pixel-by-pixel comparison of PNG images.
    ///
    /// TODO(#152): Implement when image dependencies are added.
    Pixel,
    /// Cell-by-cell comparison of ratatui `Buffer`s (content + style).
    Buffer,
}

/// Structured result of comparing two snapshots.
#[derive(Debug, Clone)]
pub struct ScreenshotDiff {
    /// Comparison mode used.
    pub mode: DiffMode,
    /// Total number of differing cells (characters or pixels).
    pub diff_count: usize,
    /// Total number of cells compared.
    pub total_cells: usize,
    /// Fraction of cells that differ (0.0 = identical, 1.0 = completely different).
    pub diff_percentage: f64,
    /// Contiguous regions where differences cluster.
    pub regions: Vec<DiffRegion>,
    /// Grid dimensions: (columns, rows) for text mode or (width, height) for pixel mode.
    pub dimensions: (usize, usize),
}

impl ScreenshotDiff {
    /// Returns `true` if the two snapshots are identical.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.diff_count == 0
    }

    /// Returns `true` if the diff percentage is at or below the given threshold.
    #[must_use]
    pub fn within_threshold(&self, max_diff_percentage: f64) -> bool {
        self.diff_percentage <= max_diff_percentage
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Compare two snapshot files and return a structured diff.
///
/// The comparison mode is inferred from the file extensions:
/// - `.txt` / `.ansi` files use text-based comparison.
/// - `.png` files use pixel-based comparison (requires `tui-png` feature).
///
/// If the extensions are mixed or unrecognized, text-based comparison is used
/// as a fallback after reading the files as UTF-8.
pub fn compare(baseline: &Path, candidate: &Path) -> Result<ScreenshotDiff> {
    let baseline_ext = baseline
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let candidate_ext = candidate
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if baseline_ext == "png" && candidate_ext == "png" {
        compare_png(baseline, candidate)
    } else {
        // Default: text-based comparison.
        let baseline_text = std::fs::read_to_string(baseline)
            .with_context(|| format!("read baseline: {}", baseline.display()))?;
        let candidate_text = std::fs::read_to_string(candidate)
            .with_context(|| format!("read candidate: {}", candidate.display()))?;
        Ok(compare_text(&baseline_text, &candidate_text))
    }
}

// ---------------------------------------------------------------------------
// Text-based comparison
// ---------------------------------------------------------------------------

/// Compare two text snapshots character by character.
///
/// ANSI escape sequences are stripped before comparison so that styling
/// differences (e.g., a color change without a content change) do not register
/// as diffs.
///
/// This function is usable without the `tui-png` feature and serves as the
/// baseline comparison backend.
pub fn compare_text(baseline: &str, candidate: &str) -> ScreenshotDiff {
    let baseline_grid = strip_to_grid(baseline);
    let candidate_grid = strip_to_grid(candidate);

    let rows = baseline_grid.len().max(candidate_grid.len());
    let cols = baseline_grid
        .iter()
        .chain(candidate_grid.iter())
        .map(|row| row.len())
        .max()
        .unwrap_or(0);

    let mut diff_map = vec![vec![false; cols]; rows];
    let mut diff_count: usize = 0;
    let total_cells = rows * cols;

    for row in 0..rows {
        let b_row = baseline_grid.get(row);
        let c_row = candidate_grid.get(row);
        for col in 0..cols {
            let b_ch = b_row.and_then(|r| r.get(col)).copied().unwrap_or(' ');
            let c_ch = c_row.and_then(|r| r.get(col)).copied().unwrap_or(' ');
            if b_ch != c_ch {
                diff_map[row][col] = true;
                diff_count += 1;
            }
        }
    }

    let diff_percentage = if total_cells == 0 {
        0.0
    } else {
        (diff_count as f64) / (total_cells as f64) * 100.0
    };

    let regions = extract_regions(&diff_map, rows, cols);

    ScreenshotDiff {
        mode: DiffMode::Text,
        diff_count,
        total_cells,
        diff_percentage,
        regions,
        dimensions: (cols, rows),
    }
}

// ---------------------------------------------------------------------------
// PNG comparison (stub)
// ---------------------------------------------------------------------------

/// Compare two PNG screenshots pixel by pixel.
///
/// TODO(#152): Implement proper pixel comparison when image dependencies are
/// added. For now, this falls back to reading the files and comparing raw
/// bytes, which is a correct but coarse approximation (any metadata or
/// compression difference counts as a diff).
fn compare_png(baseline: &Path, candidate: &Path) -> Result<ScreenshotDiff> {
    let baseline_bytes = std::fs::read(baseline)
        .with_context(|| format!("read baseline PNG: {}", baseline.display()))?;
    let candidate_bytes = std::fs::read(candidate)
        .with_context(|| format!("read candidate PNG: {}", candidate.display()))?;

    let diff_count = if baseline_bytes == candidate_bytes {
        0
    } else {
        // Byte-level diff count (coarse approximation).
        let max_len = baseline_bytes.len().max(candidate_bytes.len());
        let mut diffs = 0usize;
        for i in 0..max_len {
            let b = baseline_bytes.get(i).copied().unwrap_or(0);
            let c = candidate_bytes.get(i).copied().unwrap_or(0);
            if b != c {
                diffs += 1;
            }
        }
        diffs
    };

    let total_cells = baseline_bytes.len().max(candidate_bytes.len()).max(1);

    Ok(ScreenshotDiff {
        mode: DiffMode::Pixel,
        diff_count,
        total_cells,
        diff_percentage: (diff_count as f64) / (total_cells as f64) * 100.0,
        regions: Vec::new(), // TODO(#152): region extraction from decoded pixels
        dimensions: (0, 0),  // TODO(#152): actual image dimensions
    })
}

// ---------------------------------------------------------------------------
// Buffer ANSI serialization (P2.3)
// ---------------------------------------------------------------------------

/// Serialize a ratatui `Buffer` to an ANSI escape-sequence string that
/// preserves foreground/background colors and text modifiers (bold, dim,
/// italic, underline, reverse, crossed-out).
///
/// Each row is terminated by an SGR reset (`\x1b[0m`) and a newline. Only
/// style transitions emit escape codes -- identical consecutive styles are
/// coalesced to keep the output compact.
pub fn buffer_to_ansi(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::with_capacity(area.width as usize * area.height as usize * 4);

    for y in area.y..area.y + area.height {
        let mut prev_fg: Color = Color::Reset;
        let mut prev_bg: Color = Color::Reset;
        let mut prev_mods = Modifier::empty();

        for x in area.x..area.x + area.width {
            let Some(cell) = buf.cell((x, y)) else {
                continue;
            };

            let fg = cell.fg;
            let bg = cell.bg;
            let mods = cell.modifier;

            let style_changed = fg != prev_fg || bg != prev_bg || mods != prev_mods;

            if style_changed {
                // Reset first, then re-apply everything that is active.
                out.push_str("\x1b[0");
                emit_modifier_codes(&mut out, mods);
                emit_fg_code(&mut out, fg);
                emit_bg_code(&mut out, bg);
                out.push('m');
                prev_fg = fg;
                prev_bg = bg;
                prev_mods = mods;
            }

            out.push_str(cell.symbol());
        }

        // Reset at end of line, then newline.
        out.push_str("\x1b[0m\n");
    }

    out
}

/// Append SGR sub-codes for active modifiers.
fn emit_modifier_codes(out: &mut String, mods: Modifier) {
    if mods.contains(Modifier::BOLD) {
        out.push_str(";1");
    }
    if mods.contains(Modifier::DIM) {
        out.push_str(";2");
    }
    if mods.contains(Modifier::ITALIC) {
        out.push_str(";3");
    }
    if mods.contains(Modifier::UNDERLINED) {
        out.push_str(";4");
    }
    if mods.contains(Modifier::SLOW_BLINK) {
        out.push_str(";5");
    }
    if mods.contains(Modifier::RAPID_BLINK) {
        out.push_str(";6");
    }
    if mods.contains(Modifier::REVERSED) {
        out.push_str(";7");
    }
    if mods.contains(Modifier::HIDDEN) {
        out.push_str(";8");
    }
    if mods.contains(Modifier::CROSSED_OUT) {
        out.push_str(";9");
    }
}

/// Append an SGR foreground color sub-code.
fn emit_fg_code(out: &mut String, color: Color) {
    match color {
        Color::Reset => {}
        Color::Black => out.push_str(";30"),
        Color::Red => out.push_str(";31"),
        Color::Green => out.push_str(";32"),
        Color::Yellow => out.push_str(";33"),
        Color::Blue => out.push_str(";34"),
        Color::Magenta => out.push_str(";35"),
        Color::Cyan => out.push_str(";36"),
        Color::White => out.push_str(";37"),
        Color::DarkGray => out.push_str(";90"),
        Color::LightRed => out.push_str(";91"),
        Color::LightGreen => out.push_str(";92"),
        Color::LightYellow => out.push_str(";93"),
        Color::LightBlue => out.push_str(";94"),
        Color::LightMagenta => out.push_str(";95"),
        Color::LightCyan => out.push_str(";96"),
        Color::Gray => out.push_str(";97"),
        Color::Indexed(i) => {
            let _ = write!(out, ";38;5;{i}");
        }
        Color::Rgb(r, g, b) => {
            let _ = write!(out, ";38;2;{r};{g};{b}");
        }
    }
}

/// Append an SGR background color sub-code.
fn emit_bg_code(out: &mut String, color: Color) {
    match color {
        Color::Reset => {}
        Color::Black => out.push_str(";40"),
        Color::Red => out.push_str(";41"),
        Color::Green => out.push_str(";42"),
        Color::Yellow => out.push_str(";43"),
        Color::Blue => out.push_str(";44"),
        Color::Magenta => out.push_str(";45"),
        Color::Cyan => out.push_str(";46"),
        Color::White => out.push_str(";47"),
        Color::DarkGray => out.push_str(";100"),
        Color::LightRed => out.push_str(";101"),
        Color::LightGreen => out.push_str(";102"),
        Color::LightYellow => out.push_str(";103"),
        Color::LightBlue => out.push_str(";104"),
        Color::LightMagenta => out.push_str(";105"),
        Color::LightCyan => out.push_str(";106"),
        Color::Gray => out.push_str(";107"),
        Color::Indexed(i) => {
            let _ = write!(out, ";48;5;{i}");
        }
        Color::Rgb(r, g, b) => {
            let _ = write!(out, ";48;2;{r};{g};{b}");
        }
    }
}

// ---------------------------------------------------------------------------
// Buffer cell-level comparison (P2.4)
// ---------------------------------------------------------------------------

/// Per-cell change descriptor indicating what aspects differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellChange {
    /// Column position within the buffer.
    pub x: u16,
    /// Row position within the buffer.
    pub y: u16,
    /// Whether the character content differs.
    pub content_changed: bool,
    /// Whether the foreground color differs.
    pub fg_changed: bool,
    /// Whether the background color differs.
    pub bg_changed: bool,
    /// Whether the modifier flags differ.
    pub modifier_changed: bool,
}

/// Detailed cell-level diff between two ratatui `Buffer`s.
#[derive(Debug, Clone)]
pub struct BufferDiff {
    /// Total number of cells compared (max of the two buffer areas).
    pub total_cells: usize,
    /// Number of cells that differ in any aspect.
    pub changed_cells: usize,
    /// Similarity percentage (100.0 = identical, 0.0 = completely different).
    pub similarity_percent: f64,
    /// Per-cell change details for every differing cell.
    pub changes: Vec<CellChange>,
    /// Dimensions of the comparison grid (cols, rows).
    pub dimensions: (u16, u16),
    /// Whether the two buffers have different dimensions.
    pub size_mismatch: bool,
}

/// Compare two ratatui `Buffer`s cell by cell, examining content,
/// foreground color, background color, and modifier flags.
///
/// If the buffers have different dimensions, the comparison covers the
/// union of both areas. Cells that exist only in one buffer are treated
/// as differing from a blank (default-styled space) cell.
pub fn compare_buffers(baseline: &Buffer, candidate: &Buffer) -> BufferDiff {
    let b_area = baseline.area;
    let c_area = candidate.area;

    let cols = b_area.width.max(c_area.width);
    let rows = b_area.height.max(c_area.height);
    let total_cells = cols as usize * rows as usize;
    let size_mismatch = b_area.width != c_area.width || b_area.height != c_area.height;

    let mut changes = Vec::new();

    for y in 0..rows {
        for x in 0..cols {
            let b_cell = baseline.cell((b_area.x + x, b_area.y + y));
            let c_cell = candidate.cell((c_area.x + x, c_area.y + y));

            let (b_sym, b_fg, b_bg, b_mod) = match b_cell {
                Some(cell) => (cell.symbol(), cell.fg, cell.bg, cell.modifier),
                None => (" ", Color::Reset, Color::Reset, Modifier::empty()),
            };
            let (c_sym, c_fg, c_bg, c_mod) = match c_cell {
                Some(cell) => (cell.symbol(), cell.fg, cell.bg, cell.modifier),
                None => (" ", Color::Reset, Color::Reset, Modifier::empty()),
            };

            let content_changed = b_sym != c_sym;
            let fg_changed = b_fg != c_fg;
            let bg_changed = b_bg != c_bg;
            let modifier_changed = b_mod != c_mod;

            if content_changed || fg_changed || bg_changed || modifier_changed {
                changes.push(CellChange {
                    x,
                    y,
                    content_changed,
                    fg_changed,
                    bg_changed,
                    modifier_changed,
                });
            }
        }
    }

    let changed_cells = changes.len();
    let similarity_percent = if total_cells == 0 {
        100.0
    } else {
        (1.0 - changed_cells as f64 / total_cells as f64) * 100.0
    };

    BufferDiff {
        total_cells,
        changed_cells,
        similarity_percent,
        changes,
        dimensions: (cols, rows),
        size_mismatch,
    }
}

/// Compute a similarity percentage between two `Buffer`s.
///
/// Returns `100.0` if the buffers are identical (content and style),
/// `0.0` if every cell differs. This is a convenience wrapper around
/// [`compare_buffers`].
#[must_use]
pub fn buffer_similarity(baseline: &Buffer, candidate: &Buffer) -> f64 {
    compare_buffers(baseline, candidate).similarity_percent
}

/// Compare two `Buffer`s and return a [`ScreenshotDiff`] suitable for
/// threshold checking and region-based reporting.
///
/// This bridges the Buffer-level comparison into the same `ScreenshotDiff`
/// type used by the text and PNG comparison paths.
pub fn compare_buffers_as_diff(baseline: &Buffer, candidate: &Buffer) -> ScreenshotDiff {
    let bdiff = compare_buffers(baseline, candidate);

    let cols = bdiff.dimensions.0 as usize;
    let rows = bdiff.dimensions.1 as usize;

    // Build a boolean diff map for region extraction.
    let mut diff_map = vec![vec![false; cols]; rows];
    for change in &bdiff.changes {
        if (change.y as usize) < rows && (change.x as usize) < cols {
            diff_map[change.y as usize][change.x as usize] = true;
        }
    }

    let regions = extract_regions(&diff_map, rows, cols);
    let diff_percentage = if bdiff.total_cells == 0 {
        0.0
    } else {
        (bdiff.changed_cells as f64) / (bdiff.total_cells as f64) * 100.0
    };

    ScreenshotDiff {
        mode: DiffMode::Buffer,
        diff_count: bdiff.changed_cells,
        total_cells: bdiff.total_cells,
        diff_percentage,
        regions,
        dimensions: (cols, rows),
    }
}

// ---------------------------------------------------------------------------
// Region extraction
// ---------------------------------------------------------------------------

/// Extract contiguous rectangular regions from a boolean diff map.
///
/// Uses a simple row-scanning approach: consecutive diff cells on the same row
/// are grouped into horizontal runs, then vertically adjacent runs that
/// overlap horizontally are merged into rectangular regions.
fn extract_regions(diff_map: &[Vec<bool>], rows: usize, cols: usize) -> Vec<DiffRegion> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }

    // Step 1: collect horizontal runs per row.
    struct Run {
        col_start: usize,
        col_end: usize, // exclusive
        row: usize,
    }

    let mut runs = Vec::new();
    for (row_idx, row) in diff_map.iter().enumerate() {
        let mut col = 0;
        while col < row.len() {
            if row[col] {
                let start = col;
                while col < row.len() && row[col] {
                    col += 1;
                }
                runs.push(Run {
                    col_start: start,
                    col_end: col,
                    row: row_idx,
                });
            } else {
                col += 1;
            }
        }
    }

    if runs.is_empty() {
        return Vec::new();
    }

    // Step 2: merge vertically adjacent, horizontally overlapping runs into
    // bounding-box regions using a simple greedy pass.
    let mut regions: Vec<DiffRegion> = Vec::new();

    for run in &runs {
        let merged = regions.iter_mut().find(|region| {
            // The run must be on the row immediately below the region's bottom edge
            // and horizontally overlap.
            let region_bottom = region.y + region.height;
            run.row == region_bottom
                && run.col_start < region.x + region.width
                && run.col_end > region.x
        });

        if let Some(region) = merged {
            let new_x = region.x.min(run.col_start);
            let new_right = (region.x + region.width).max(run.col_end);
            region.x = new_x;
            region.width = new_right - new_x;
            region.height = run.row - region.y + 1;
            region.diff_count += run.col_end - run.col_start;
        } else {
            regions.push(DiffRegion {
                x: run.col_start,
                y: run.row,
                width: run.col_end - run.col_start,
                height: 1,
                diff_count: run.col_end - run.col_start,
            });
        }
    }

    regions
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences and split into a character grid.
fn strip_to_grid(text: &str) -> Vec<Vec<char>> {
    static ANSI_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = ANSI_RE
        .get_or_init(|| regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ANSI regex"));

    text.lines()
        .map(|line| {
            let stripped = re.replace_all(line, "");
            stripped.chars().collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn identical_texts_produce_zero_diff() {
        let text = "hello\nworld";
        let diff = compare_text(text, text);
        assert!(diff.is_identical());
        assert_eq!(diff.diff_count, 0);
        assert_eq!(diff.diff_percentage, 0.0);
        assert!(diff.regions.is_empty());
        assert_eq!(diff.dimensions, (5, 2));
    }

    #[test]
    fn single_character_change_detected() {
        let baseline = "hello\nworld";
        let candidate = "heLlo\nworld";
        let diff = compare_text(baseline, candidate);
        assert_eq!(diff.diff_count, 1);
        assert!(!diff.is_identical());
        assert_eq!(diff.regions.len(), 1);
        assert_eq!(diff.regions[0].x, 2);
        assert_eq!(diff.regions[0].y, 0);
        assert_eq!(diff.regions[0].diff_count, 1);
    }

    #[test]
    fn ansi_codes_are_stripped_before_comparison() {
        let baseline = "\x1b[31mhello\x1b[0m";
        let candidate = "\x1b[32mhello\x1b[0m";
        let diff = compare_text(baseline, candidate);
        assert!(diff.is_identical());
    }

    #[test]
    fn different_line_counts_handled() {
        let baseline = "line1\nline2\nline3";
        let candidate = "line1\nline2";
        let diff = compare_text(baseline, candidate);
        assert!(!diff.is_identical());
        // line3 has 5 chars, all missing from candidate.
        assert_eq!(diff.diff_count, 5);
    }

    #[test]
    fn different_line_lengths_handled() {
        let baseline = "short";
        let candidate = "short!!";
        let diff = compare_text(baseline, candidate);
        assert_eq!(diff.diff_count, 2); // the two trailing '!' characters
    }

    #[test]
    fn within_threshold_check() {
        let diff = ScreenshotDiff {
            mode: DiffMode::Text,
            diff_count: 5,
            total_cells: 100,
            diff_percentage: 5.0,
            regions: Vec::new(),
            dimensions: (10, 10),
        };
        assert!(diff.within_threshold(5.0));
        assert!(diff.within_threshold(10.0));
        assert!(!diff.within_threshold(4.9));
    }

    #[test]
    fn region_extraction_merges_vertical_neighbors() {
        // Two diff cells stacked vertically should merge into one region.
        let diff_map = vec![
            vec![false, true, false],
            vec![false, true, false],
            vec![false, false, false],
        ];
        let regions = extract_regions(&diff_map, 3, 3);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].x, 1);
        assert_eq!(regions[0].y, 0);
        assert_eq!(regions[0].width, 1);
        assert_eq!(regions[0].height, 2);
        assert_eq!(regions[0].diff_count, 2);
    }

    #[test]
    fn region_extraction_separates_non_adjacent_diffs() {
        let diff_map = vec![
            vec![true, false, false],
            vec![false, false, false],
            vec![false, false, true],
        ];
        let regions = extract_regions(&diff_map, 3, 3);
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn compare_file_dispatches_by_extension() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "hello").unwrap();
        std::fs::write(&b, "hello").unwrap();
        let diff = compare(&a, &b).unwrap();
        assert!(diff.is_identical());
        assert_eq!(diff.mode, DiffMode::Text);
    }

    #[test]
    fn compare_png_files_identical() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        let content = vec![137, 80, 78, 71, 13, 10, 26, 10]; // PNG magic
        std::fs::write(&a, &content).unwrap();
        std::fs::write(&b, &content).unwrap();
        let diff = compare(&a, &b).unwrap();
        assert!(diff.is_identical());
        assert_eq!(diff.mode, DiffMode::Pixel);
    }

    #[test]
    fn compare_png_files_different() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        std::fs::write(&a, &[137, 80, 78, 71, 0, 0]).unwrap();
        std::fs::write(&b, &[137, 80, 78, 71, 1, 1]).unwrap();
        let diff = compare(&a, &b).unwrap();
        assert!(!diff.is_identical());
        assert_eq!(diff.diff_count, 2);
    }

    #[test]
    fn empty_inputs_produce_zero_diff() {
        let diff = compare_text("", "");
        assert!(diff.is_identical());
        assert_eq!(diff.dimensions, (0, 0));
    }

    // -- Buffer ANSI serialization tests --

    #[test]
    fn buffer_to_ansi_preserves_plain_content() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        // Write "hello" into the buffer.
        for (i, ch) in "hello".chars().enumerate() {
            buf[(i as u16, 0u16)].set_char(ch);
        }
        let ansi = buffer_to_ansi(&buf);
        // Should contain "hello" somewhere in the output.
        let stripped = strip_ansi(&ansi);
        assert_eq!(stripped.trim(), "hello");
    }

    #[test]
    fn buffer_to_ansi_emits_fg_color() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('R').set_fg(Color::Red);
        buf[(1, 0)].set_char('G').set_fg(Color::Green);
        buf[(2, 0)].set_char('B').set_fg(Color::Blue);
        let ansi = buffer_to_ansi(&buf);
        // Should contain SGR codes for red (31), green (32), blue (34).
        assert!(ansi.contains(";31"), "missing red fg code");
        assert!(ansi.contains(";32"), "missing green fg code");
        assert!(ansi.contains(";34"), "missing blue fg code");
        let stripped = strip_ansi(&ansi);
        assert_eq!(stripped.trim(), "RGB");
    }

    #[test]
    fn buffer_to_ansi_emits_bg_and_modifiers() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)]
            .set_char('X')
            .set_bg(Color::Yellow)
            .set_fg(Color::Reset);
        buf[(1, 0)]
            .set_char('Y')
            .set_fg(Color::Reset)
            .set_bg(Color::Reset);
        let ansi = buffer_to_ansi(&buf);
        // Yellow background = ;43
        assert!(ansi.contains(";43"), "missing yellow bg code");
    }

    #[test]
    fn buffer_to_ansi_emits_rgb_colors() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)]
            .set_char('C')
            .set_fg(Color::Rgb(255, 128, 0))
            .set_bg(Color::Rgb(10, 20, 30));
        let ansi = buffer_to_ansi(&buf);
        assert!(
            ansi.contains(";38;2;255;128;0"),
            "missing RGB fg: {ansi}"
        );
        assert!(
            ansi.contains(";48;2;10;20;30"),
            "missing RGB bg: {ansi}"
        );
    }

    #[test]
    fn buffer_to_ansi_emits_indexed_colors() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)]
            .set_char('I')
            .set_fg(Color::Indexed(208))
            .set_bg(Color::Indexed(235));
        let ansi = buffer_to_ansi(&buf);
        assert!(
            ansi.contains(";38;5;208"),
            "missing indexed fg: {ansi}"
        );
        assert!(
            ansi.contains(";48;5;235"),
            "missing indexed bg: {ansi}"
        );
    }

    #[test]
    fn buffer_to_ansi_emits_bold_modifier() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('B');
        // Add bold modifier.
        let style = ratatui::style::Style::default().add_modifier(Modifier::BOLD);
        buf[(0, 0)].set_style(style);
        let ansi = buffer_to_ansi(&buf);
        // Bold = ;1
        assert!(ansi.contains(";1"), "missing bold code: {ansi}");
    }

    #[test]
    fn buffer_to_ansi_resets_at_eol() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 2, 2);
        let buf = Buffer::empty(area);
        let ansi = buffer_to_ansi(&buf);
        // Each line should end with a reset.
        for line in ansi.lines() {
            assert!(
                line.ends_with("\x1b[0m"),
                "line does not end with reset: {line:?}"
            );
        }
    }

    #[test]
    fn buffer_to_ansi_coalesces_identical_styles() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        for x in 0..4 {
            buf[(x, 0)].set_char('A').set_fg(Color::Red);
        }
        let ansi = buffer_to_ansi(&buf);
        // Count occurrences of ";31" -- should appear once (at the start),
        // not four times.
        let count = ansi.matches(";31").count();
        assert_eq!(count, 1, "style should be coalesced: {ansi}");
    }

    // -- Buffer cell-level comparison tests --

    #[test]
    fn identical_buffers_produce_zero_diff() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        for (i, ch) in "hello".chars().enumerate() {
            buf[(i as u16, 0)].set_char(ch);
        }
        let diff = compare_buffers(&buf, &buf);
        assert_eq!(diff.changed_cells, 0);
        assert!((diff.similarity_percent - 100.0).abs() < f64::EPSILON);
        assert!(diff.changes.is_empty());
        assert!(!diff.size_mismatch);
    }

    #[test]
    fn content_change_detected_in_buffer() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 3, 1);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a[(0, 0)].set_char('A');
        a[(1, 0)].set_char('B');
        a[(2, 0)].set_char('C');
        b[(0, 0)].set_char('A');
        b[(1, 0)].set_char('X'); // changed
        b[(2, 0)].set_char('C');
        let diff = compare_buffers(&a, &b);
        assert_eq!(diff.changed_cells, 1);
        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].x, 1);
        assert_eq!(diff.changes[0].y, 0);
        assert!(diff.changes[0].content_changed);
        assert!(!diff.changes[0].fg_changed);
    }

    #[test]
    fn style_only_change_detected_in_buffer() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 2, 1);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a[(0, 0)].set_char('A').set_fg(Color::Red);
        b[(0, 0)].set_char('A').set_fg(Color::Blue); // same char, different fg
        a[(1, 0)].set_char('B');
        b[(1, 0)].set_char('B');
        let diff = compare_buffers(&a, &b);
        assert_eq!(diff.changed_cells, 1);
        assert!(!diff.changes[0].content_changed);
        assert!(diff.changes[0].fg_changed);
        assert!(!diff.changes[0].bg_changed);
    }

    #[test]
    fn bg_change_detected_in_buffer() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 1, 1);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a[(0, 0)].set_char('Z').set_bg(Color::Green);
        b[(0, 0)].set_char('Z').set_bg(Color::Yellow);
        let diff = compare_buffers(&a, &b);
        assert_eq!(diff.changed_cells, 1);
        assert!(!diff.changes[0].content_changed);
        assert!(diff.changes[0].bg_changed);
    }

    #[test]
    fn modifier_change_detected_in_buffer() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 1, 1);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a[(0, 0)].set_char('M');
        let bold = ratatui::style::Style::default().add_modifier(Modifier::BOLD);
        b[(0, 0)].set_char('M');
        b[(0, 0)].set_style(bold);
        let diff = compare_buffers(&a, &b);
        assert_eq!(diff.changed_cells, 1);
        assert!(!diff.changes[0].content_changed);
        assert!(diff.changes[0].modifier_changed);
    }

    #[test]
    fn different_sized_buffers_detected() {
        use ratatui::layout::Rect;

        let a = Buffer::empty(Rect::new(0, 0, 5, 3));
        let b = Buffer::empty(Rect::new(0, 0, 3, 2));
        let diff = compare_buffers(&a, &b);
        assert!(diff.size_mismatch);
        assert_eq!(diff.dimensions, (5, 3));
        // Extra cells in `a` that are absent in `b` should not count as
        // changed if both are blank/default.
        // The union area is 5x3=15 cells. `b` only covers 3x2=6.
        // All are blank spaces with default style, so no changes.
        assert_eq!(diff.changed_cells, 0);
    }

    #[test]
    fn buffer_similarity_100_for_identical() {
        use ratatui::layout::Rect;

        let buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        assert!((buffer_similarity(&buf, &buf) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn buffer_similarity_decreases_with_changes() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 10, 1);
        let a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        // Change 5 out of 10 cells.
        for x in 0..5 {
            b[(x, 0)].set_char('X');
        }
        let sim = buffer_similarity(&a, &b);
        assert!((sim - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compare_buffers_as_diff_produces_regions() {
        use ratatui::layout::Rect;

        let area = Rect::new(0, 0, 5, 3);
        let a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        // Create a 2-wide run on row 1.
        b[(1, 1)].set_char('X');
        b[(2, 1)].set_char('Y');
        let diff = compare_buffers_as_diff(&a, &b);
        assert_eq!(diff.mode, DiffMode::Buffer);
        assert_eq!(diff.diff_count, 2);
        assert_eq!(diff.regions.len(), 1);
        assert_eq!(diff.regions[0].x, 1);
        assert_eq!(diff.regions[0].y, 1);
        assert_eq!(diff.regions[0].width, 2);
    }

    /// Helper: strip ANSI escapes from a string (for test assertions).
    fn strip_ansi(s: &str) -> String {
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| {
            regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ANSI regex")
        });
        re.replace_all(s, "").into_owned()
    }
}
