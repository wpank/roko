//! `roko screenshot` command — captures every TUI tab to text files for
//! headless inspection (e.g. by Claude or CI).

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::{Args, ValueEnum};
use serde::Serialize;

use roko_cli::tui::screenshot_diff::{self, DiffRegion};
use roko_cli::tui::snapshot::{SnapshotConfig, capture_snapshots};

/// How `roko screenshot` writes each captured tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotFormat {
    /// Plain text, `<tab>.txt`.
    Text,
    /// Plain text plus `<tab>.ansi`, the tab with its colours and modifiers
    /// as ANSI escapes.
    Ansi,
}

/// Capture TUI tab screenshots as text files for visual inspection.
#[derive(Debug, Args)]
pub struct ScreenshotArgs {
    /// Output directory (default: .roko/screenshots/latest/).
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// Capture specific tabs only (comma-separated: dashboard,plans or f1,f2).
    #[arg(long)]
    pub tabs: Option<String>,

    /// Terminal width for rendering.
    #[arg(long, default_value = "240")]
    pub width: u16,

    /// Terminal height for rendering.
    #[arg(long, default_value = "60")]
    pub height: u16,

    /// Human-readable label for this snapshot.
    #[arg(long)]
    pub label: Option<String>,

    /// Working directory (default: cwd or --repo).
    #[arg(long)]
    pub workdir: Option<PathBuf>,

    /// Output format: `text` writes `<tab>.txt`; `ansi` also writes `<tab>.ansi` with colours.
    #[arg(long, value_enum, default_value_t = ScreenshotFormat::Text)]
    pub format: ScreenshotFormat,

    /// Compare the capture with the same-named `.txt` files of an earlier capture in REF_DIR
    /// and write `diff-report.json` beside it. Exits 1 when any tab differs or is new.
    #[arg(long, value_name = "REF_DIR")]
    pub compare: Option<PathBuf>,
}

pub fn cmd_screenshot(workdir: PathBuf, args: ScreenshotArgs) -> Result<i32> {
    // Resolve the reference first: `.roko/screenshots/latest` is repointed to
    // the new capture below, and comparing that with itself finds nothing.
    let reference = args
        .compare
        .map(|dir| {
            dir.canonicalize()
                .with_context(|| format!("--compare {}", dir.display()))
        })
        .transpose()?;
    let default_root = workdir.join(".roko").join("screenshots");
    let update_latest = args.dir.is_none();
    let output_dir = args.dir.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        default_root.join(format!("run-{timestamp}-{}", std::process::id()))
    });

    let tabs = args
        .tabs
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

    let config = SnapshotConfig {
        width: args.width,
        height: args.height,
        output_dir,
        tabs,
        label: args.label,
        ansi: args.format == ScreenshotFormat::Ansi,
    };

    let result = capture_snapshots(&workdir, &config)?;
    if update_latest {
        update_latest_link(&default_root.join("latest"), &result.dir)?;
    }

    println!(
        "Captured {} tabs to {}",
        result.tabs_captured,
        result.dir.display()
    );
    println!("Manifest: {}", result.manifest_path.display());

    let Some(reference) = reference else {
        return Ok(0);
    };
    let report = diff_against(&reference, &result.dir, &result.files)?;
    let report_path = result.dir.join("diff-report.json");
    let report_json = serde_json::to_string_pretty(&report).context("serialize diff report")?;
    std::fs::write(&report_path, report_json)
        .with_context(|| format!("write {}", report_path.display()))?;
    println!(
        "Compared {} tab(s) with {}: {} changed. Report: {}",
        report.files.len(),
        reference.display(),
        report.changed,
        report_path.display()
    );
    Ok(i32::from(report.changed > 0))
}

/// `diff-report.json`: how a capture differs from a reference capture.
#[derive(Debug, Serialize)]
struct DiffReport {
    reference: PathBuf,
    capture: PathBuf,
    /// Tabs that differ from the reference, or that it lacks.
    changed: usize,
    files: Vec<TabDiff>,
}

/// One captured tab compared with the reference file of the same name.
#[derive(Debug, Serialize)]
struct TabDiff {
    file: String,
    /// Whether the reference has the file; a tab it lacks counts as changed.
    in_reference: bool,
    identical: bool,
    /// Character cells that differ, ANSI escapes stripped, out of `total_cells`.
    diff_cells: usize,
    total_cells: usize,
    diff_percent: f64,
    /// 1-based numbers of the lines that differ.
    changed_lines: Vec<usize>,
    /// Rectangles (0-based column and row) where cells differ.
    regions: Vec<DiffRegion>,
}

/// Compare each captured file in `capture` with the file of the same name in
/// `reference`, through `screenshot_diff::compare`.
fn diff_against(reference: &Path, capture: &Path, files: &[String]) -> Result<DiffReport> {
    anyhow::ensure!(
        reference.is_dir(),
        "--compare {}: not a directory",
        reference.display()
    );
    let mut tabs = Vec::new();
    for file in files {
        let baseline = reference.join(file);
        if !baseline.is_file() {
            tabs.push(TabDiff {
                file: file.clone(),
                in_reference: false,
                identical: false,
                diff_cells: 0,
                total_cells: 0,
                diff_percent: 0.0,
                changed_lines: Vec::new(),
                regions: Vec::new(),
            });
            continue;
        }
        let diff = screenshot_diff::compare(&baseline, &capture.join(file))?;
        let mut changed_lines: Vec<usize> = diff
            .regions
            .iter()
            .flat_map(|region| region.y + 1..=region.y + region.height)
            .collect();
        changed_lines.sort_unstable();
        changed_lines.dedup();
        tabs.push(TabDiff {
            file: file.clone(),
            in_reference: true,
            identical: diff.is_identical(),
            diff_cells: diff.diff_count,
            total_cells: diff.total_cells,
            diff_percent: diff.diff_percentage,
            changed_lines,
            regions: diff.regions,
        });
    }
    let changed = tabs.iter().filter(|tab| !tab.identical).count();
    Ok(DiffReport {
        reference: reference.to_path_buf(),
        capture: capture.to_path_buf(),
        changed,
        files: tabs,
    })
}

fn update_latest_link(link: &std::path::Path, target: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(
        link.parent()
            .context("latest screenshot link has no parent directory")?,
    )?;

    if let Ok(metadata) = std::fs::symlink_metadata(link) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            std::fs::remove_file(link)
                .with_context(|| format!("remove stale latest link {}", link.display()))?;
        } else if metadata.is_dir() {
            // Older releases wrote captures directly into `latest/`. Preserve
            // that evidence rather than deleting it when migrating to a link.
            let backup = link.with_file_name(format!(
                "latest.previous-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f")
            ));
            std::fs::rename(link, &backup).with_context(|| {
                format!(
                    "preserve legacy screenshot directory {} as {}",
                    link.display(),
                    backup.display()
                )
            })?;
        }
    }

    let target = target
        .canonicalize()
        .with_context(|| format!("canonicalize screenshot run {}", target.display()))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, link)
        .with_context(|| format!("create latest screenshot link {}", link.display()))?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&target, link)
        .with_context(|| format!("create latest screenshot link {}", link.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_compare_reports_each_tab_against_the_reference() {
        let reference = tempfile::tempdir().expect("reference dir");
        let capture = tempfile::tempdir().expect("capture dir");
        let write = |dir: &Path, file: &str, text: &str| {
            std::fs::write(dir.join(file), text).expect("write capture");
        };
        write(reference.path(), "f01-dashboard.txt", "plans 1/2\nok");
        write(capture.path(), "f01-dashboard.txt", "plans 2/2\nok");
        write(reference.path(), "f02-plans.txt", "same");
        write(capture.path(), "f02-plans.txt", "same");
        write(capture.path(), "f03-agents.txt", "new tab");
        let files = ["f01-dashboard.txt", "f02-plans.txt", "f03-agents.txt"].map(String::from);

        let report = diff_against(reference.path(), capture.path(), &files).expect("report");

        assert_eq!(report.changed, 2, "the changed tab and the new one");
        let dashboard = &report.files[0];
        assert!(dashboard.in_reference && !dashboard.identical);
        assert_eq!(dashboard.diff_cells, 1);
        assert_eq!(dashboard.changed_lines, [1]);
        assert!(report.files[1].identical);
        assert!(!report.files[2].in_reference);
        let json = serde_json::to_value(&report).expect("report json");
        assert_eq!(json["files"][0]["regions"][0]["x"], 6);
        let missing = capture.path().join("missing");
        assert!(diff_against(&missing, capture.path(), &files).is_err());
    }
}
