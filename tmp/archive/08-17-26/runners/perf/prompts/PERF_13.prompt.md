# PERF_13: Source-hash gate skip (B08-b)

## Task

Cache compile/clippy/test verdicts keyed on the SHA-256 of the
modified-source set. On re-runs with no source change, emit a
`skipped: true` verdict instead of re-invoking cargo. Persist the cache
to `.roko/cache/gate-hashes.json`.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_13](../ISSUE-TRACKER.md#perf_13)
- Plan: `tmp/solutions/perf/implementation/11-source-hash-gate-skip.md`
- Bottleneck: B08 second leg (BOTTLENECK-ANALYSIS.md §B08)
- Performance contract: **C-12** (no-op repeat run skips compile/test)
- Priority: P1
- Effort: ≈3 h
- Depends on: none
- Wave: 1

## Problem

`roko run --gates compile,test` then immediately running it again
re-runs cargo even though no source has changed. Per
`BENCHMARK-RESULTS.md` §6, that's 500-2000 ms per gate per run.

The fix mirrors Bazel/Turborepo/sccache: hash the gate's input set
(source files), persist `(gate_name → hash)` after success, and skip
when the hash matches.

## Exact Changes

### Step 1 — Add deps

`crates/roko-gate/Cargo.toml`:

```toml
sha2 = "0.10"
ignore = "0.4"
```

(Both are tiny and already present transitively in many crates; verify
with `cargo tree -p roko-gate --depth 1` before assuming you must add.)

### Step 2 — New module `crates/roko-gate/src/source_hash.rs`

```rust
//! Source-hash gate skip cache (perf contract C-12).
//!
//! On a successful gate pass, record `gate_name → SHA-256(source set)`
//! to `.roko/cache/gate-hashes.json`. Before re-running the gate,
//! compute the current hash; if it matches the recorded one, emit a
//! `skipped: true` verdict with reason `source-hash:<prefix>`.
//!
//! **Cache key inputs:** `(gate_name, paths + bytes of input set)`.
//! **Invalidation:** automatic when any source file changes (the hash
//! flips). Manual via `--no-gate-cache` CLI flag.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Persisted hash → last successful pass mapping per gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateHashCache {
    /// Hex-encoded SHA-256 of the last successful input set per gate.
    pub last_pass: HashMap<String, String>,
}

impl GateHashCache {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist atomically via the standard write-tmp-rename helper.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        roko_fs::atomic::atomic_write_bytes(path, &bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))
    }
}

/// Compute SHA-256 of a source set: paths and bytes contribute.
pub fn compute_source_hash(workdir: &Path, inputs: &[PathBuf]) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    for rel in inputs {
        let abs = workdir.join(rel);
        let bytes = std::fs::read(&abs).unwrap_or_default();
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute the input set per gate. Respects `.gitignore` so build
/// artefacts (`target/`, `node_modules/`) are excluded.
pub fn gate_input_set(workdir: &Path, gate_name: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    let walker = ignore::WalkBuilder::new(workdir)
        .standard_filters(true)
        .build();
    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() { continue; }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue; };
        let rel = path.strip_prefix(workdir).unwrap_or(path).to_path_buf();

        let included = match gate_name {
            "compile" | "clippy" | "fmt" | "format-check" =>
                matches!(ext, "rs" | "toml" | "lock"),
            "test" =>
                matches!(ext, "rs" | "toml" | "lock") || rel.starts_with("tests"),
            _ => false,         // unknown gate → no skip
        };
        if included {
            v.push(rel);
        }
    }
    v.sort();           // stable hash regardless of walk order
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_rust_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\nversion=\"0.0.1\"\nedition=\"2021\"\n").unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main(){}").unwrap();
        dir
    }

    #[test]
    fn input_set_includes_rs_and_toml_for_compile() {
        let dir = small_rust_project();
        let inputs = gate_input_set(dir.path(), "compile");
        assert!(inputs.iter().any(|p| p == &PathBuf::from("Cargo.toml")));
        assert!(inputs.iter().any(|p| p == &PathBuf::from("src/main.rs")));
    }

    #[test]
    fn input_set_excludes_target_dir() {
        let dir = small_rust_project();
        std::fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        std::fs::write(dir.path().join("target/debug/leak"), "junk").unwrap();
        let inputs = gate_input_set(dir.path(), "compile");
        assert!(inputs.iter().all(|p| !p.starts_with("target/")));
    }

    #[test]
    fn hash_changes_on_file_edit() {
        let dir = small_rust_project();
        let inputs = gate_input_set(dir.path(), "compile");
        let h1 = compute_source_hash(dir.path(), &inputs).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "// touched\nfn main(){}").unwrap();
        let h2 = compute_source_hash(dir.path(), &inputs).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn cache_round_trips_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.json");
        let mut c = GateHashCache::default();
        c.last_pass.insert("compile".into(), "abc123".into());
        c.save(&path).unwrap();
        let loaded = GateHashCache::load(&path);
        assert_eq!(loaded.last_pass.get("compile").map(String::as_str), Some("abc123"));
    }
}
```

### Step 3 — Re-export from `crates/roko-gate/src/lib.rs`

```rust
pub mod source_hash;
pub use source_hash::{GateHashCache, compute_source_hash, gate_input_set};
```

### Step 4 — Plug into `GateService`

`crates/roko-gate/src/gate_service.rs`. Add fields:

```rust
use std::path::PathBuf;
use std::sync::Mutex;
use crate::source_hash::{GateHashCache, compute_source_hash, gate_input_set};

pub struct GateService {
    // ... existing fields ...
    hash_cache: Mutex<GateHashCache>,
    hash_cache_path: PathBuf,
    /// When true, every gate runs (no skip via source-hash). Set via
    /// the CLI `--no-gate-cache` flag.
    hash_cache_disabled: bool,
}

impl GateService {
    pub fn new() -> Self {
        Self::new_under(std::env::current_dir().unwrap_or_default())
    }

    pub fn new_under(workdir: PathBuf) -> Self {
        let hash_cache_path = workdir.join(".roko/cache/gate-hashes.json");
        let hash_cache = GateHashCache::load(&hash_cache_path);
        Self {
            // ... existing fields, defaulted ...
            hash_cache: Mutex::new(hash_cache),
            hash_cache_path,
            hash_cache_disabled: false,
        }
    }

    pub fn with_hash_cache_disabled(mut self) -> Self {
        self.hash_cache_disabled = true;
        self
    }
}
```

(Adjust the constructor pattern to whatever the existing
`GateService::new()` does. The point is: load the cache from
`.roko/cache/gate-hashes.json` at construction, then carry it as a
`Mutex<GateHashCache>`.)

In `run_gates`, inside the per-gate loop (after PERF_12's mode filter,
before the gate's `verify` call):

```rust
let input_set = gate_input_set(&config.workdir, &gate_name);
let current_hash = if !self.hash_cache_disabled && !input_set.is_empty() {
    Some(compute_source_hash(&config.workdir, &input_set)?)
} else {
    None
};

if let Some(ref h) = current_hash {
    let cache = self.hash_cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(prev) = cache.last_pass.get(&gate_name) {
        if prev == h {
            verdicts.push(skipped_gate_verdict(
                gate_name.clone(),
                "Skipped: source unchanged since last successful run",
                format!("source-hash:{}", &h[..12.min(h.len())]),
            ));
            continue;
        }
    }
}

// ... existing gate.verify call ...
let verdict = run_one_gate(&gate_name, &config).await?;

if verdict.passed {
    if let Some(h) = current_hash {
        let mut cache = self.hash_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.last_pass.insert(gate_name.clone(), h);
        if let Err(err) = cache.save(&self.hash_cache_path) {
            tracing::warn!(error = %err, "failed to persist gate-hash cache");
        }
    }
}

verdicts.push(verdict);
```

> **Anti-pattern check.** The cache write fails open: a disk-full
> situation logs a warning and proceeds, it does NOT fail the gate run
> (AP-PERSIST-4).

### Step 5 — CLI flag

`crates/roko-cli/src/main.rs`:

```rust
/// Disable the source-hash gate skip cache. Use only when debugging
/// gate behaviour. Restores the per-run cargo invocation cost.
#[clap(long, global = true)]
pub no_gate_cache: bool,
```

In the `run.rs` glue that constructs `GateService`:

```rust
let mut svc = GateService::new_under(workdir.to_path_buf());
if cli.no_gate_cache {
    svc = svc.with_hash_cache_disabled();
}
```

### Step 6 — Integration tests

Append to `crates/roko-gate/src/gate_service.rs`:

```rust
#[cfg(test)]
mod source_hash_tests {
    use super::*;
    use roko_runtime::pipeline_state::GateMode;

    fn ctx(workdir: &Path, gates: Vec<&str>) -> GateConfig {
        GateConfig {
            workdir: workdir.to_path_buf(),
            enabled_gates: gates.into_iter().map(String::from).collect(),
            shell_gates: vec![],
            gate_mode: GateMode::Full,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn second_run_with_unchanged_source_is_skipped() {
        let dir = small_rust_project();
        let svc = GateService::new_under(dir.path().to_path_buf());
        let _ = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();
        let report2 = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();
        let verdict = report2.verdicts.iter().find(|v| v.gate_name == "compile").unwrap();
        assert!(verdict.skipped);
        assert!(verdict.skip_reason.as_deref().unwrap_or("").starts_with("source-hash:"));
    }

    #[tokio::test]
    async fn modifying_a_file_invalidates_skip() {
        let dir = small_rust_project();
        let svc = GateService::new_under(dir.path().to_path_buf());
        let _ = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();

        std::fs::write(dir.path().join("src/main.rs"), "// touched\nfn main(){}").unwrap();
        let report = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();
        let verdict = report.verdicts.iter().find(|v| v.gate_name == "compile").unwrap();
        assert!(!verdict.skipped, "should re-run after edit");
    }

    #[tokio::test]
    async fn disabled_cache_runs_compile_every_time() {
        let dir = small_rust_project();
        let svc = GateService::new_under(dir.path().to_path_buf()).with_hash_cache_disabled();
        let r1 = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();
        let r2 = svc.run_gates(ctx(dir.path(), vec!["compile"])).await.unwrap();
        assert!(!r1.verdicts[0].skipped);
        assert!(!r2.verdicts[0].skipped);
    }

    fn small_rust_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.1\"\nedition=\"2021\"\n").unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main(){}").unwrap();
        dir
    }
}
```

> If `GateService::run_gates` is hard to drive in tests because it
> requires real cargo, mock the gate's `verify` impl. The skip logic
> is what we are testing, not cargo itself.

## Write Scope

- `crates/roko-gate/src/source_hash.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-gate/src/lib.rs`
- `crates/roko-gate/Cargo.toml`
- `crates/roko-cli/src/main.rs`

## Read-Only Context

- `crates/roko-gate/src/compile.rs`
- `crates/roko-gate/src/test_gate.rs`
- `crates/roko-fs/src/atomic.rs` (`atomic_write_bytes`)
- `tmp/solutions/perf/implementation/11-source-hash-gate-skip.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-PERSIST-3/4, AP-GATE-6/7)

## Acceptance Criteria

- [ ] `crates/roko-gate/src/source_hash.rs` exists with `GateHashCache` + `compute_source_hash` + `gate_input_set`.
- [ ] `sha2 = "0.10"` and `ignore = "0.4"` added to `crates/roko-gate/Cargo.toml` (or confirmed already present transitively).
- [ ] `gate_input_set(workdir, gate_name)` covers `compile`, `clippy`, `fmt`, `format-check`, `test`.
- [ ] `GateService` carries `hash_cache: Mutex<GateHashCache>` + `hash_cache_path: PathBuf`.
- [ ] `GateService::new_under(workdir)` constructor loads the cache.
- [ ] Skip emits `GateVerdict { skipped: true, skip_reason: Some("source-hash:<12>") }`.
- [ ] Cache persisted via `roko_fs::atomic::atomic_write_bytes`.
- [ ] CLI flag `--no-gate-cache` exists and disables the cache.
- [ ] Tests `second_run_with_unchanged_source_is_skipped`, `modifying_a_file_invalidates_skip`, `disabled_cache_runs_compile_every_time` pass.
- [ ] Cache write failures emit `tracing::warn!` and do NOT fail the gate (fails-open).

## Verify

```bash
# Module exports:
rg -n 'GateHashCache|compute_source_hash|gate_input_set' crates/roko-gate/src/lib.rs

# Cache write goes through atomic helper:
rg -n 'atomic_write_bytes' crates/roko-gate/src/source_hash.rs

# CLI surface:
./target/release/roko run --help | rg 'no-gate-cache'
```

## Do NOT

- Do NOT use mtime-based skip. Mtime is not reliable across `git
  checkout`, `cp -p`, atomic-rewrite editors. Hash-based skip is the
  contract.
- Do NOT include `target/`, `node_modules/`, `.roko/` in the gate
  input set (AP-GATE-7). Use `ignore` crate's `.gitignore` respect.
- Do NOT include the hash cache file itself in the input set.
  Recursive invalidation; will be debugged for an afternoon.
- Do NOT cache failure verdicts (AP-GATE-6). Only skip on success.
- Do NOT save to a network filesystem without using
  `roko_fs::atomic::atomic_write_bytes` (AP-PERSIST-3).
- Do NOT extend the hash to include arbitrary env vars. If you do,
  scope to a short, vetted list and document.
- Do NOT skip when `--release` and the prior pass was `--debug`. Today
  the gate config doesn't distinguish; if you add a release flag,
  include it in the hash key.
- Do NOT compose this with the deeper sccache idea. They tackle
  different problems and stacking is bookkeeping-heavy.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_13 done <commit-sha>
```
