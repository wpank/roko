#![allow(missing_docs)]

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(roko_frontend_fallback)");
    println!("cargo:rerun-if-env-changed=SKIP_FRONTEND_BUILD");
    println!("cargo:rerun-if-env-changed=ROKO_BUILD_FRONTEND");
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

    // Release/Docker automation builds the SPA explicitly before Cargo. Reuse
    // that immutable output instead of invoking npm a second time from a Rust
    // build script.
    if demo_app.join("dist/index.html").is_file() {
        return;
    }

    // Ordinary debug/check builds must never install packages or invoke the
    // frontend toolchain. Production release builds retain the embedded SPA,
    // and developers can explicitly request the same work with
    // ROKO_BUILD_FRONTEND=1.
    let force_build = env::var("ROKO_BUILD_FRONTEND").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let release_build = env::var("PROFILE").is_ok_and(|profile| profile == "release");
    if env::var("SKIP_FRONTEND_BUILD").is_ok() || (!release_build && !force_build) {
        println!("cargo:rustc-cfg=roko_frontend_fallback");
        return;
    }

    // Install deps if node_modules is missing
    if !demo_app.join("node_modules").exists() {
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&demo_app)
            .status();

        let installed = match status {
            Ok(status) if status.success() => true,
            Ok(status) => {
                println!("cargo:warning=npm install exited with {status}; embedding fallback UI");
                false
            }
            Err(error) => {
                println!(
                    "cargo:warning=npm install failed (is Node.js installed?): {error}; embedding fallback UI"
                );
                false
            }
        };
        if !installed {
            println!("cargo:rustc-cfg=roko_frontend_fallback");
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
            println!("cargo:rustc-cfg=roko_frontend_fallback");
        }
        Err(e) => {
            println!("cargo:warning=npm run build failed: {e}");
            println!("cargo:rustc-cfg=roko_frontend_fallback");
        }
    }
}
