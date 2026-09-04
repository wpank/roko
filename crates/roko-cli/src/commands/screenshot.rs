//! `roko screenshot` command — captures every TUI tab to text files for
//! headless inspection (e.g. by Claude or CI).

use std::path::PathBuf;

use anyhow::{Context as _, Result};
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

    Ok(0)
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
