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

use std::path::Path;

use anyhow::{Context as _, Result};

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
        dimensions: (0, 0), // TODO(#152): actual image dimensions
    })
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
    let re = ANSI_RE.get_or_init(|| {
        regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ANSI regex")
    });

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
}
