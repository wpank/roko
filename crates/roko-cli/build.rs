#![allow(missing_docs)]
/// Build script for roko-cli: captures git hash and rustc version at compile time.
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

    // Rebuild when git HEAD changes (so hash stays fresh), and when the
    // index does (staging, commits and checkouts, so the dirty flag does).
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/");
    println!("cargo:rerun-if-changed=../../.git/index");
    println!("cargo:rerun-if-changed=build.rs");
}
