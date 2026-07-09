# AUDIT: Batch R4_A02 — Collect workspace members and project kind

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R4_A02`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task
Collect workspace members and project kind

## Runner Context
You are working in runner `mega-parity`, batch R4_A02.
This batch is part of Runner 4: plan-grounding — Ground PRD/plan generation in the real repository and reject invalid artifacts.

## Problem
To ground generation prompts, we need to know what already exists in the workspace. This means parsing manifest files to extract workspace member names and determining the project kind. These members populate both `workspace_members` and `do_not_create` in the `RepoContextPack`.

## Architecture Contract
- Depends on `RepoContextPack` and `ProjectKind` from R4_A01 (already in `repo_context.rs`)
- Workspace member detection feeds `workspace_members` and `do_not_create` fields
- `ProjectKind::detect()` based on which manifest files exist at root
- All functions are infallible: on parse error, return empty vec (log a warning, no panic)

---

## Dependency Check

Before writing code, verify that R4_A01 completed:
```bash
test -f /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/repo_context.rs && echo "OK" || echo "MISSING — complete R4_A01 first"
```

Also verify available crate dependencies in `crates/roko-cli/Cargo.toml`:
- `toml = { workspace = true }` — YES, present at line 51
- `serde_json = { workspace = true }` — YES, present at line 50
- No `glob` crate in workspace — use `std::fs::read_dir` for glob expansion

---

## Step-by-Step Instructions

### Step 1: Add `ProjectKind::detect()` to `crates/roko-cli/src/repo_context.rs`

Add the following impl block after the `ProjectKind` enum definition:

```rust
impl ProjectKind {
    /// Detect project kind from manifest files at the given root.
    ///
    /// Returns `Mixed` when multiple kinds detected, `Unknown` when none found.
    #[must_use]
    pub fn detect(root: &Path) -> Self {
        let has_cargo = root.join("Cargo.toml").exists();
        let has_package_json = root.join("package.json").exists();
        let has_go = root.join("go.work").exists() || root.join("go.mod").exists();
        let has_python = root.join("pyproject.toml").exists()
            || root.join("setup.py").exists();

        let count = [has_cargo, has_package_json, has_go, has_python]
            .iter()
            .filter(|&&x| x)
            .count();

        match count {
            0 => Self::Unknown,
            1 if has_cargo => Self::Rust,
            1 if has_package_json => Self::TypeScript,
            1 if has_go => Self::Go,
            1 if has_python => Self::Python,
            _ => Self::Mixed,
        }
    }
}
```

### Step 2: Add workspace member collection functions to `repo_context.rs`

Add the following free functions to `crates/roko-cli/src/repo_context.rs`:

```rust
/// Collect workspace members from manifest files.
///
/// Returns `(members, do_not_create)` where `do_not_create` is initialized
/// as a copy of `members`.
///
/// On any parse or I/O error, returns empty vecs (never panics, never errors).
#[must_use]
pub fn collect_workspace_members(root: &Path, kind: &ProjectKind) -> (Vec<String>, Vec<String>) {
    let members = match kind {
        ProjectKind::Rust => collect_rust_members(root),
        ProjectKind::TypeScript => collect_ts_members(root),
        ProjectKind::Go => collect_go_members(root),
        ProjectKind::Python => vec![],
        ProjectKind::Mixed => {
            let mut all = collect_rust_members(root);
            all.extend(collect_ts_members(root));
            all.extend(collect_go_members(root));
            all.sort();
            all.dedup();
            all
        }
        ProjectKind::Unknown => vec![],
    };
    let do_not_create = members.clone();
    (members, do_not_create)
}

/// Parse Cargo.toml [workspace] members globs and resolve to crate names.
///
/// Algorithm:
/// 1. Read root Cargo.toml as text
/// 2. Find [workspace] section, extract members = [...] array
/// 3. For each glob pattern (e.g. "crates/*"), expand via read_dir
/// 4. For each matching directory, read its Cargo.toml [package] name
///    (fall back to directory name if reading fails)
/// 5. Return sorted, deduplicated names
fn collect_rust_members(root: &Path) -> Vec<String> {
    let cargo_toml_path = root.join("Cargo.toml");
    let content = match std::fs::read_to_string(&cargo_toml_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let members_array = match parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        Some(arr) => arr.clone(),
        None => return vec![],
    };

    let mut names = Vec::new();
    for member_value in &members_array {
        let pattern = match member_value.as_str() {
            Some(s) => s,
            None => continue,
        };
        // Simple glob: support "prefix/*" and literal paths
        if let Some(prefix) = pattern.strip_suffix("/*") {
            let dir = root.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if !entry_path.is_dir() {
                        continue;
                    }
                    let name = read_cargo_package_name(&entry_path)
                        .unwrap_or_else(|| {
                            entry_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
            }
        } else {
            // Literal path
            let dir = root.join(pattern);
            if dir.is_dir() {
                let name = read_cargo_package_name(&dir)
                    .unwrap_or_else(|| {
                        dir.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    });
                if !name.is_empty() {
                    names.push(name);
                }
            }
        }
    }

    names.sort();
    names.dedup();
    names
}

/// Read the `[package] name` field from a crate's Cargo.toml.
/// Returns None on any error.
fn read_cargo_package_name(crate_dir: &Path) -> Option<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(ToOwned::to_owned)
}

/// Parse package.json workspaces array and resolve to package names.
///
/// Supports:
/// - `"workspaces": ["packages/*", ...]`  (npm workspaces)
/// - `"workspaces": { "packages": ["packages/*", ...] }` (Yarn workspaces)
fn collect_ts_members(root: &Path) -> Vec<String> {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    // Extract the workspaces array (handle both npm and Yarn formats)
    let patterns: Vec<String> = {
        let ws = parsed.get("workspaces");
        if let Some(arr) = ws.and_then(|w| w.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(ToOwned::to_owned)
                .collect()
        } else if let Some(arr) = ws
            .and_then(|w| w.get("packages"))
            .and_then(|p| p.as_array())
        {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(ToOwned::to_owned)
                .collect()
        } else {
            return vec![];
        }
    };

    let mut names = Vec::new();
    for pattern in &patterns {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            let dir = root.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if !entry_path.is_dir() {
                        continue;
                    }
                    let name = read_npm_package_name(&entry_path)
                        .unwrap_or_else(|| {
                            entry_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
            }
        } else {
            let dir = root.join(pattern.as_str());
            if dir.is_dir() {
                let name = read_npm_package_name(&dir)
                    .unwrap_or_else(|| {
                        dir.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    });
                if !name.is_empty() {
                    names.push(name);
                }
            }
        }
    }

    names.sort();
    names.dedup();
    names
}

/// Read the `"name"` field from a package's package.json.
/// Returns None on any error.
fn read_npm_package_name(pkg_dir: &Path) -> Option<String> {
    let pkg_json = pkg_dir.join("package.json");
    let content = std::fs::read_to_string(&pkg_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed
        .get("name")
        .and_then(|n| n.as_str())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(ToOwned::to_owned)
}

/// Parse go.work `use` directives and return module directory names.
///
/// Format of go.work:
/// ```
/// go 1.21
///
/// use (
///     ./module-a
///     ./module-b
/// )
/// use ./module-c
/// ```
fn collect_go_members(root: &Path) -> Vec<String> {
    let go_work_path = root.join("go.work");
    if let Ok(content) = std::fs::read_to_string(&go_work_path) {
        return parse_go_work_members(&content);
    }
    // Fall back to go.mod module name
    let go_mod_path = root.join("go.mod");
    if let Ok(content) = std::fs::read_to_string(&go_mod_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(module) = trimmed.strip_prefix("module ") {
                let name = module.trim().to_string();
                if !name.is_empty() {
                    return vec![name];
                }
            }
        }
    }
    vec![]
}

/// Parse `use (...)` and `use ./path` directives from go.work content.
fn parse_go_work_members(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_use_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("use (") || trimmed == "use (" {
            in_use_block = true;
            continue;
        }
        if in_use_block && trimmed == ")" {
            in_use_block = false;
            continue;
        }
        if in_use_block {
            let path = trimmed.trim_start_matches("./");
            if !path.is_empty() && !path.starts_with("//") {
                // Use the last path segment as the name
                if let Some(name) = path.split('/').last() {
                    if !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
            }
            continue;
        }
        // Single-line `use ./path`
        if let Some(rest) = trimmed.strip_prefix("use ") {
            let path = rest.trim().trim_start_matches("./");
            if !path.is_empty() && !path.starts_with('(') {
                if let Some(name) = path.split('/').last() {
                    if !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }

    names.sort();
    names.dedup();
    names
}
```

### Step 3: Verify compilation and tests

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-cli 2>&1 | tail -20
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | tail -20
```

Quick smoke test — run this and verify Rust is detected with 18+ members:
```bash
cd /Users/will/dev/nunchi/roko/roko
cargo test -p roko-cli repo_context 2>&1 | tail -20
```

---

## Write Scope (files you may modify)
- `crates/roko-cli/src/repo_context.rs` (add to existing file from R4_A01)

## Read-Only Context (do not modify these)
- `crates/roko-cli/Cargo.toml` — toml and serde_json are workspace deps (lines 50–51)
- `Cargo.toml` (workspace root) — has `[workspace] members = ["crates/*"]`

## Acceptance Criteria
- [ ] `ProjectKind::detect()` correctly identifies Rust/TS/Go/Python/Mixed/Unknown
- [ ] `collect_workspace_members()` returns members for Rust workspaces (test with roko itself)
- [ ] `collect_workspace_members()` returns members for TS workspaces (npm and Yarn formats)
- [ ] `collect_workspace_members()` returns members for Go workspaces (go.work format)
- [ ] `do_not_create` is a copy of `workspace_members`
- [ ] Parse errors return empty vec (no panics)
- [ ] `read_cargo_package_name()` helper extracts crate names from `Cargo.toml`
- [ ] `cargo build -p roko-cli` succeeds
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` passes

## Verification
```bash
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-cli 2>&1 | tail -5
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | tail -5
```

## Do NOT
- Deep-parse Cargo.toml beyond `[workspace].members` and `[package].name`
- Require compilation or `cargo metadata` (too slow for time budget)
- Support monorepo tools (nx, turborepo, lerna) beyond basic `package.json` workspaces
- Add new crate dependencies not already in workspace (no `glob` crate)
- Panic on malformed manifests
- Use `unwrap()` without a fallback

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/repo_context.rs` (914 lines — truncated)

```rust
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MAX_PROMPT_SECTION_CHARS: usize = 32_000;
const MAX_WORKSPACE_MEMBERS_RENDERED: usize = 50;
const MAX_KEY_FILES_RENDERED: usize = 10;
const MAX_SYMBOLS_RENDERED: usize = 15;
const MAX_RELATED_ITEMS_RENDERED: usize = 5;
const MAX_DO_NOT_CREATE_RENDERED: usize = 50;
const MAX_SYMBOL_TEXT_CHARS: usize = 120;
const TRUNCATION_MARKER: &str = "[truncated]";

/// Bounded repository context for grounding PRD/plan generation prompts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoContextPack {
    /// Workspace root directory (absolute path).
    pub root: PathBuf,
    /// Detected project kind.
    pub project_kind: ProjectKind,
    /// Workspace member names (crate names for Rust, package names for TS/Go).
    pub workspace_members: Vec<String>,
    /// Key files relevant to the feature being generated (max 20).
    pub key_files: Vec<PathBuf>,
    /// Symbol matches from grep-like search (max 30).
    pub matching_symbols: Vec<SymbolHit>,
    /// Related PRD paths found in `.roko/prd/` (max 5).
    pub related_prds: Vec<PathBuf>,
    /// Related plan paths found in `.roko/plans/` (max 5).
    pub related_plans: Vec<PathBuf>,
    /// Names that already exist and must not be re-created (workspace members + known crates).
    pub do_not_create: Vec<String>,
    /// True when feature keywords match workspace members AND real source exists.
    pub context_root_verified: bool,
}

/// A single symbol match from repository search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolHit {
    /// File path relative to workspace root.
    pub file: PathBuf,
    /// Line number (1-indexed).
    pub line: u32,
    /// Matched line text (trimmed, max 200 chars).
    pub text: String,
}

/// Detected project kind based on manifest files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKind {
    Rust,
    TypeScript,
    Go,
    Python,
    Mixed,
    Unknown,
}

impl fmt::Display for ProjectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Python => "python",
            Self::Mixed => "mixed",
            Self::Unknown => "unknown",
        })
    }
}

impl ProjectKind {
    /// Detect project kind from manifest files at the given root.
    ///
    /// Returns `Mixed` when multiple kinds are detected, `Unknown` when none
    /// are found.
    #[must_use]
    pub fn detect(root: &Path) -> Self {
        let has_cargo = root.join("Cargo.toml").is_file();
        let has_package_json = root.join("package.json").is_file();
        let has_go = root.join("go.work").is_file() || root.join("go.mod").is_file();
        let has_python = root.join("pyproject.toml").is_file() || root.join("setup.py").is_file();

        let count = [has_cargo, has_package_json, has_go, has_python]
            .into_iter()
            .filter(|has| *has)
            .count();

        match count {
            0 => Self::Unknown,
            1 if has_cargo => Self::Rust,
            1 if has_package_json => Self::TypeScript,
            1 if has_go => Self::Go,
            1 if has_python => Self::Python,
            _ => Self::Mixed,
        }
    }
}

/// Collect workspace members from manifest files.
///
/// Returns `(members, do_not_create)` where `do_not_create` is initialized as
/// a copy of `members`.
#[must_use]
pub fn collect_workspace_members(root: &Path, kind: &ProjectKind) -> (Vec<String>, Vec<String>) {
    let mut members = match kind {
        ProjectKind::Rust => collect_rust_members(root),
        ProjectKind::TypeScript => collect_ts_members(root),
        ProjectKind::Go => collect_go_members(root),
        ProjectKind::Python | ProjectKind::Unknown => Vec::new(),
        ProjectKind::Mixed => {
            let mut all = collect_rust_members(root);
            all.extend(collect_ts_members(root));
            all.extend(collect_go_members(root));
            all
        }
    };

    members.sort_unstable();
    members.dedup();
    let do_not_create = members.clone();
    (members, do_not_create)
}

/// Parse `Cargo.toml` `[workspace].members` entries into crate names.
///
/// Supports literal paths and simple `prefix/*` member globs.
pub fn collect_rust_members(root: &Path) -> Vec<String> {
    let cargo_toml_path = root.join("Cargo.toml");
    let content = match std::fs::read_to_string(&cargo_toml_path) {
        Ok(content) => content,
        Err(err) => {
            if cargo_toml_path.exists() {
                tracing::warn!(
                    path = %cargo_toml_path.display(),
                    error = %err,
                    "failed to read Cargo.toml"
                );
            }
            return Vec::new();
        }
    };

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                path = %cargo_toml_path.display(),
                error = %err,
                "failed to parse Cargo.toml"
            );
            return Vec::new();
        }
    };

    let Some(members_array) = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(|members| members.as_array())
    else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for member_value in members_array {
        let Some(pattern) = member_value
            .as_str()
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
        else {
            continue;
        };

        for member_dir in resolve_workspace_member_dirs(root, pattern) {
            let name = read_cargo_package_name(&member_dir)
                .unwrap_or_else(|| fallback_dir_name(&member_dir));
            if !name.is_empty() {
                names.push(name);
            }
        }
    }

    names.sort_unstable();
    names.dedup();
    names
}

/// Read the `[package] name` field from a crate's `Cargo.toml`.
pub fn read_cargo_package_name(crate_dir: &Path) -> Option<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = match std::fs::read_to_string(&cargo_toml) {
        Ok(content) => content,
        Err(err) => {
            tracing::warn!(
                path = %cargo_toml.display(),
                error = %err,
                "failed to read crate Cargo.toml"
            );
// ... (514 lines omitted) ...
            .expect("workspace root")
            .to_path_buf()
    }

    #[test]
    fn repo_context_detect_project_kind_from_root_manifests() {
        let empty = tempdir().expect("tempdir");
        assert_eq!(ProjectKind::detect(empty.path()), ProjectKind::Unknown);

        let rust = tempdir().expect("tempdir");
        write_file(
            &rust.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        );
        assert_eq!(ProjectKind::detect(rust.path()), ProjectKind::Rust);

        let ts = tempdir().expect("tempdir");
        write_file(&ts.path().join("package.json"), "{}");
        assert_eq!(ProjectKind::detect(ts.path()), ProjectKind::TypeScript);

        let go = tempdir().expect("tempdir");
        write_file(&go.path().join("go.mod"), "module example.com/demo\n");
        assert_eq!(ProjectKind::detect(go.path()), ProjectKind::Go);

        let python = tempdir().expect("tempdir");
        write_file(
            &python.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\n",
        );
        assert_eq!(ProjectKind::detect(python.path()), ProjectKind::Python);

        let mixed = tempdir().expect("tempdir");
        write_file(
            &mixed.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        );
        write_file(&mixed.path().join("package.json"), "{}");
        assert_eq!(ProjectKind::detect(mixed.path()), ProjectKind::Mixed);
    }

    #[test]
    fn repo_context_collect_workspace_members_reads_real_roko_workspace() {
        let root = repo_root();
        assert_eq!(ProjectKind::detect(&root), ProjectKind::Rust);

        let (members, do_not_create) = collect_workspace_members(&root, &ProjectKind::Rust);
        assert!(
            members.len() >= 18,
            "expected at least 18 members, got {members:?}"
        );
        assert_eq!(members, do_not_create);
        assert!(members.contains(&"roko-cli".to_string()));
        assert!(members.contains(&"roko-core".to_string()));
    }

    #[test]
    fn repo_context_collect_workspace_members_rust_workspace() {
        let root = tempdir().expect("tempdir");
        write_file(
            &root.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "libs/shared"]
"#,
        );
        write_file(
            &root.path().join("crates/alpha/Cargo.toml"),
            "[package]\nname = \"alpha-core\"\nversion = \"0.1.0\"\n",
        );
        write_file(
            &root.path().join("crates/beta/Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
        );
        write_file(
            &root.path().join("libs/shared/Cargo.toml"),
            "[package]\nname = \"shared-lib\"\nversion = \"0.1.0\"\n",
        );

        let (members, do_not_create) = collect_workspace_members(root.path(), &ProjectKind::Rust);
        assert_eq!(
            members,
            vec![
                "alpha-core".to_string(),
                "beta".to_string(),
                "shared-lib".to_string(),
            ]
        );
        assert_eq!(members, do_not_create);
    }

    #[test]
    fn repo_context_collect_workspace_members_rust_manifest_parse_error_returns_empty() {
        let root = tempdir().expect("tempdir");
        write_file(
            &root.path().join("Cargo.toml"),
            "[workspace\nmembers = []\n",
        );

        let (members, do_not_create) = collect_workspace_members(root.path(), &ProjectKind::Rust);
        assert!(members.is_empty());
        assert!(do_not_create.is_empty());
    }

    #[test]
    fn repo_context_collect_workspace_members_ts_npm_workspace() {
        let root = tempdir().expect("tempdir");
        write_file(
            &root.path().join("package.json"),
            r#"{"workspaces":["packages/*","tools/pkg"]}"#,
        );
        write_file(
            &root.path().join("packages/app/package.json"),
            r#"{"name":"app-web"}"#,
        );
        write_file(
            &root.path().join("packages/lib/package.json"),
            r#"{"name":"shared-lib"}"#,
        );
        write_file(
            &root.path().join("tools/pkg/package.json"),
            r#"{"name":"tooling"}"#,
        );

        let (members, do_not_create) =
            collect_workspace_members(root.path(), &ProjectKind::TypeScript);
        assert_eq!(
            members,
            vec![
                "app-web".to_string(),
                "shared-lib".to_string(),
                "tooling".to_string(),
            ]
        );
        assert_eq!(members, do_not_create);
    }

    #[test]
    fn repo_context_collect_workspace_members_ts_yarn_workspace() {
        let root = tempdir().expect("tempdir");
        write_file(
            &root.path().join("package.json"),
            r#"{"workspaces":{"packages":["packages/*"]}}"#,
        );
        write_file(
            &root.path().join("packages/ui/package.json"),
            r#"{"name":"ui-kit"}"#,
        );
        write_file(
            &root.path().join("packages/api/package.json"),
            r#"{"name":"api-server"}"#,
        );

        let (members, do_not_create) =
            collect_workspace_members(root.path(), &ProjectKind::TypeScript);
        assert_eq!(
            members,
            vec!["api-server".to_string(), "ui-kit".to_string()]
        );
        assert_eq!(members, do_not_create);
    }

    #[test]
    fn repo_context_collect_workspace_members_go_workspace() {
        let root = tempdir().expect("tempdir");
        write_file(
            &root.path().join("go.work"),
            r#"go 1.21

use (
    ./module-a
    ./module-b
)
use ./module-c
"#,
        );
        fs::create_dir_all(root.path().join("module-a")).expect("create module-a");
        fs::create_dir_all(root.path().join("module-b")).expect("create module-b");
        fs::create_dir_all(root.path().join("module-c")).expect("create module-c");

        let (members, do_not_create) = collect_workspace_members(root.path(), &ProjectKind::Go);
        assert_eq!(
            members,
            vec![
                "module-a".to_string(),
                "module-b".to_string(),
                "module-c".to_string(),
            ]
        );
        assert_eq!(members, do_not_create);
    }

    #[test]
    fn repo_context_collect_workspace_members_go_mod_fallback() {
        let root = tempdir().expect("tempdir");
        write_file(&root.path().join("go.mod"), "module example.com/fallback\n");

        let (members, do_not_create) = collect_workspace_members(root.path(), &ProjectKind::Go);
        assert_eq!(members, vec!["example.com/fallback".to_string()]);
        assert_eq!(members, do_not_create);
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
