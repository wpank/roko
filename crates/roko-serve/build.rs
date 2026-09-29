#![allow(missing_docs)]

use std::env;
use std::path::Path;
use std::process::Command;

const DEMO_NOT_BUILT: &str = "demo/demo-app/dist/index.html is missing, so `/demo` serves the \
                              fallback page; run `npm ci && npm run build` in demo/demo-app first";

fn main() {
    println!("cargo:rustc-check-cfg=cfg(roko_frontend_fallback)");
    println!("cargo:rustc-check-cfg=cfg(roko_portal_fallback)");
    println!("cargo:rerun-if-env-changed=SKIP_FRONTEND_BUILD");
    println!("cargo:rerun-if-env-changed=ROKO_BUILD_FRONTEND");
    println!("cargo:rerun-if-env-changed=ROKO_REQUIRE_EMBEDDED_UI");
    println!("cargo:rerun-if-changed=../../demo/demo-app/src");
    println!("cargo:rerun-if-changed=../../demo/demo-app/index.html");
    println!("cargo:rerun-if-changed=../../demo/demo-app/package.json");
    println!("cargo:rerun-if-changed=../../demo/demo-app/vite.config.ts");
    println!("cargo:rerun-if-changed=../../demo/demo-app/tsconfig.json");
    // Rerun when the portal export appears or changes (no npm invoked for it).
    println!("cargo:rerun-if-changed=../../apps/portal/out/index.html");
    println!("cargo:rerun-if-changed=assets/frontend-fallback/index.html");

    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        embed_fallback(
            "roko_portal_fallback",
            "CARGO_MANIFEST_DIR is not set, so `/` serves the fallback page",
        );
        embed_fallback(
            "roko_frontend_fallback",
            "CARGO_MANIFEST_DIR is not set, so `/demo` serves the fallback page",
        );
        return;
    };

    // ── Portal embed ──────────────────────────────────────────────────────────
    // Never run npm for the portal. The static export is produced by running
    // `npm run build:export` in `apps/portal` outside of Cargo.
    let portal_out = Path::new(&manifest_dir).join("../../apps/portal/out");
    if !portal_out.join("index.html").is_file() {
        embed_fallback(
            "roko_portal_fallback",
            "apps/portal/out/index.html is missing, so `/` serves the fallback page; \
             run `npm ci && npm run build:export` in apps/portal first",
        );
    }

    // ── Demo app embed ────────────────────────────────────────────────────────
    let demo_app = Path::new(&manifest_dir).join("../../demo/demo-app");

    // The real dist/ is intentionally ignored. Use a tracked placeholder when
    // the frontend source is unavailable so rust-embed still has a directory.
    if !demo_app.join("package.json").exists() {
        embed_fallback(
            "roko_frontend_fallback",
            "demo/demo-app/package.json is missing, so `/demo` serves the fallback page",
        );
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
    let force_build = env_flag("ROKO_BUILD_FRONTEND");
    let release_build = env::var("PROFILE").is_ok_and(|profile| profile == "release");
    if env::var("SKIP_FRONTEND_BUILD").is_ok() || (!release_build && !force_build) {
        embed_fallback("roko_frontend_fallback", DEMO_NOT_BUILT);
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
                println!("cargo:warning=npm install exited with {status}");
                false
            }
            Err(error) => {
                println!("cargo:warning=npm install failed (is Node.js installed?): {error}");
                false
            }
        };
        if !installed {
            embed_fallback("roko_frontend_fallback", DEMO_NOT_BUILT);
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
            embed_fallback("roko_frontend_fallback", DEMO_NOT_BUILT);
        }
        Err(e) => {
            println!("cargo:warning=npm run build failed: {e}");
            embed_fallback("roko_frontend_fallback", DEMO_NOT_BUILT);
        }
    }
}

/// Embed `assets/frontend-fallback/` in place of a UI that was not built.
///
/// Release builds warn. With `ROKO_REQUIRE_EMBEDDED_UI=1`, which the release
/// workflow and the Dockerfiles set, the build fails instead, so a packaged
/// binary never serves the fallback page.
fn embed_fallback(cfg: &str, why: &str) {
    assert!(
        !env_flag("ROKO_REQUIRE_EMBEDDED_UI"),
        "ROKO_REQUIRE_EMBEDDED_UI is set but {why}"
    );
    if env::var("PROFILE").is_ok_and(|profile| profile == "release") {
        println!("cargo:warning={why}");
    }
    println!("cargo:rustc-cfg={cfg}");
}

/// True when the variable is set to 1, true, yes or on (any case).
fn env_flag(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}
