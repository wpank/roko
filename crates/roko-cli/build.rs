#![allow(missing_docs)]
/// Build script for roko-cli: captures git hash and rustc version at compile time.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Git short hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());
    println!("cargo:rustc-env=ROKO_GIT_HASH={git_hash}");

    // Whether tracked files differed from HEAD at build time (`git describe
    // --dirty` semantics: untracked files do not count). `unknown` without
    // git. A run manifest also checks the source tree when the run starts,
    // since edits after the build do not rerun this script.
    let git_dirty = Command::new("git")
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=no",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or("unknown", |o| {
            if o.stdout.iter().all(u8::is_ascii_whitespace) {
                "false"
            } else {
                "true"
            }
        });
    println!("cargo:rustc-env=ROKO_GIT_DIRTY={git_dirty}");

    // rustc version
    let rustc_version = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());
    println!("cargo:rustc-env=ROKO_RUSTC_VERSION={rustc_version}");

    // Target triple
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=ROKO_TARGET={target}");
    } else {
        println!("cargo:rustc-env=ROKO_TARGET=unknown");
    }

    // Rebuild when HEAD moves (so the hash stays fresh), and when the index
    // changes (staging, commits and checkouts, so the dirty flag does).
    for path in git_watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
}

/// The git files whose changes rerun this script.
///
/// In a git worktree `.git` is a file, so the real directories come from
/// `git rev-parse`: `HEAD`, `index` and the `HEAD` reflog live in the
/// worktree's git dir, and branch refs in the common dir. The reflog changes
/// on every commit, checkout and reset. Of the refs, only the loose ref
/// `HEAD` points at is watched, not all of `refs/`, so commits in other
/// worktrees do not rebuild this one. Cargo treats a missing watched path as
/// always stale and would rebuild on every build, so only paths that exist
/// are returned, and none without git.
fn git_watch_paths() -> Vec<PathBuf> {
    let Some(dirs) = git_stdout(&["rev-parse", "--git-dir", "--git-common-dir"]) else {
        return Vec::new();
    };
    let mut lines = dirs.lines();
    let (Some(git_dir), Some(common_dir)) = (lines.next(), lines.next()) else {
        return Vec::new();
    };
    // `git rev-parse` prints these relative to the current directory (the
    // package directory, for a build script) or as absolute paths.
    let cwd = std::env::current_dir().unwrap_or_default();
    let git_dir = cwd.join(git_dir.trim());
    let common_dir = cwd.join(common_dir.trim());

    let mut paths = vec![
        git_dir.join("HEAD"),
        git_dir.join("index"),
        git_dir.join("logs").join("HEAD"),
    ];
    // Detached HEAD has no ref; `HEAD` itself then names the commit.
    if let Some(head_ref) = git_stdout(&["symbolic-ref", "-q", "HEAD"]) {
        paths.push(common_dir.join(head_ref.trim()));
    }
    paths.retain(|path| path.exists());
    paths
}

/// Stdout of a `git` command that succeeded, or `None`.
fn git_stdout(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
}
