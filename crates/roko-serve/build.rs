#![allow(missing_docs)]

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(roko_frontend_fallback)");
    println!("cargo:rerun-if-env-changed=SKIP_FRONTEND_BUILD");
    println!("cargo:rerun-if-changed=../../demo/demo-app/src");
    println!("cargo:rerun-if-changed=../../demo/demo-app/index.html");
    println!("cargo:rerun-if-changed=../../demo/demo-app/package.json");
    println!("cargo:rerun-if-changed=../../demo/demo-app/vite.config.ts");
    println!("cargo:rerun-if-changed=../../demo/demo-app/tsconfig.json");
    println!("cargo:rerun-if-changed=assets/frontend-fallback/index.html");

    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let demo_app = Path::new(&manifest_dir).join("../../demo/demo-app");

    // The real dist/ is intentionally ignored. Use a tracked placeholder when
    // the frontend source is unavailable so rust-embed still has a directory.
    if !demo_app.join("package.json").exists() {
        println!("cargo:rustc-cfg=roko_frontend_fallback");
        return;
    }

    // Explicitly skipping Node is primarily for quick Rust-only checks in a
    // fresh worktree, where the ignored dist/ directory does not exist yet.
    if env::var("SKIP_FRONTEND_BUILD").is_ok() {
        println!("cargo:rustc-cfg=roko_frontend_fallback");
        return;
    }

    // Install deps if node_modules is missing
    if !demo_app.join("node_modules").exists() {
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&demo_app)
            .status();

        if let Err(e) = status {
            println!("cargo:warning=npm install failed (is Node.js installed?): {e}");
            return;
        }
    }

    // Run the build
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(&demo_app)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=npm run build exited with {s}");
        }
        Err(e) => {
            println!("cargo:warning=npm run build failed: {e}");
        }
    }
}
