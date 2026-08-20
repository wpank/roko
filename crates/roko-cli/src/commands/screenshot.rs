//! `roko screenshot` command — captures every TUI tab to text files for
//! headless inspection (e.g. by Claude or CI).

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use roko_cli::tui::snapshot::{SnapshotConfig, capture_snapshots};

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
}

pub fn cmd_screenshot(workdir: PathBuf, args: ScreenshotArgs) -> Result<i32> {
    let output_dir = args
        .dir
        .unwrap_or_else(|| workdir.join(".roko").join("screenshots").join("latest"));

    let tabs = args
        .tabs
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

    let config = SnapshotConfig {
        width: args.width,
        height: args.height,
        output_dir,
        tabs,
        label: args.label,
    };

    let result = capture_snapshots(&workdir, &config)?;

    println!(
        "Captured {} tabs to {}",
        result.tabs_captured,
        result.dir.display()
    );
    println!("Manifest: {}", result.manifest_path.display());

    Ok(0)
}
