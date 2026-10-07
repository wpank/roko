//! Shared path helpers for workspace-local `.roko` artifacts.

use std::path::{Path, PathBuf};

pub use crate::plan::plans_dir;

/// Return the relative plans directory (no workspace root prefix).
///
/// Used by cloud execution where the workspace root is not yet known.
#[must_use]
pub fn relative_plans_dir() -> PathBuf {
    PathBuf::from("plans")
}

/// The workspace plans directory ([`plans_dir`]) relative to `workdir`:
/// `plans`, or `.roko/plans` in a workspace that keeps its plans there. For
/// prompts that tell an agent where to write plans.
#[must_use]
pub fn workspace_relative_plans_dir(workdir: &Path) -> PathBuf {
    let dir = plans_dir(workdir);
    dir.strip_prefix(workdir)
        .map_or(dir.clone(), Path::to_path_buf)
}

/// Resolve the workspace-local `.roko` root.
#[must_use]
pub fn roko_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko")
}
