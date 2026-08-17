# 11 — Source-Hash Gate Skip (B08, second leg)

> Bottleneck: when an iteration of the workflow runs gates twice with no
> intervening source change (e.g., autofix retries that don't actually
> modify code, or `roko run` invoked twice in a row), the compile / test
> gates re-run unnecessarily.
>
> Target savings: 500–1500 ms per re-run skip.
> Effort: ≈3 h. Risk: medium (false negatives if hash misses a change).

---

## Goal & success criteria

After this change:

1. `GateService` records, on each successful gate pass, the SHA-256
   hash of the modified-source set (paths + bytes).
2. Before invoking compile/clippy/test, the service compares the
   current source hash to the last successful hash.
3. On a match, the gate emits a "skipped: hash unchanged" verdict
   instead of re-running.
4. The hash record persists at `.roko/cache/gate-hashes.json` so it
   survives across CLI invocations.

Done when:

- A unit test runs the compile gate twice on identical source and
  asserts the second run is skipped.
- A unit test modifies one file and asserts the second run executes.
- Macro-benchmark on `roko run` followed immediately by an identical
  `roko run` shows ≥1 s improvement on the second run.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B08 ("Source hash gate
  guard"), `OPTIMIZATION-PLAYBOOK.md` §9 #2.
- The trick is well-known in CI systems (Bazel, Turborepo, sccache):
  hash the inputs of an action; if the inputs match a prior successful
  run, skip the action.
- Roko's gate pipeline is a natural fit because each gate has a
  well-defined input set:
  - **compile**: the workspace's source files (`*.rs`, `Cargo.toml`,
    `Cargo.lock`).
  - **clippy**: same.
  - **test**: same + test fixtures under `tests/`.
  - **fmt / format-check**: same.
- The hash key for each gate must include the gate name, so a `compile`
  pass does not falsely satisfy a `test` skip.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-gate/src/gate_service.rs` | Where the skip logic plugs in (alongside adaptive thresholds). |
| `crates/roko-gate/src/compile.rs` | The compile gate; understand its input set. |
| `crates/roko-gate/src/test_gate.rs` | Same for tests. |
| `crates/roko-gate/src/adaptive_threshold.rs` | The other skip mechanism — how to compose. |
| `crates/roko-fs/src/atomic.rs` | `atomic_write_bytes` for safe persistence of the hash file. |

---

## Code-level plan

### Step 1 — Define the hash record

New file: `crates/roko-gate/src/source_hash.rs`.

```rust
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateHashCache {
    /// Last successful hash per gate name, hex-encoded.
    pub last_pass: HashMap<String, String>,
}

impl GateHashCache {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        roko_fs::atomic::atomic_write_bytes(path, &bytes)
    }
}

/// Compute the SHA-256 hash of a source set.
///
/// `inputs` is a list of file paths relative to `workdir`. Each file's
/// path and bytes contribute to the hash.
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
```

Add `sha2 = "0.10"` to `crates/roko-gate/Cargo.toml`.

### Step 2 — Determine the input set per gate

```rust
// In gate_service.rs (or a sibling helper)

fn gate_input_set(workdir: &Path, gate_name: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    let walker = ignore::WalkBuilder::new(workdir)
        .standard_filters(true)        // respect .gitignore
        .build();
    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() { continue; }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue; };
        let rel = path.strip_prefix(workdir).unwrap_or(path).to_path_buf();

        let included = match gate_name {
            "compile" | "clippy" | "fmt" | "format-check" => matches!(ext, "rs" | "toml" | "lock"),
            "test" => matches!(ext, "rs" | "toml" | "lock") || rel.starts_with("tests/"),
            _ => false,         // unknown gate → no skip
        };
        if included {
            v.push(rel);
        }
    }
    v.sort();           // stable hash
    v
}
```

Add `ignore = "0.4"` to `Cargo.toml` (the same crate `ripgrep` uses; it
respects `.gitignore` for free).

### Step 3 — Plug the skip check into `GateService::run_gates`

Inside the per-gate loop:

```rust
let input_set = gate_input_set(&config.workdir, &gate_name);
if !input_set.is_empty() {
    let current_hash = compute_source_hash(&config.workdir, &input_set)?;
    if let Some(last_hash) = self.hash_cache.lock().unwrap().last_pass.get(&gate_name) {
        if &current_hash == last_hash {
            verdicts.push(skipped_gate_verdict(
                gate_name.clone(),
                "Skipped: source unchanged since last successful run",
                format!("source-hash:{}", &current_hash[..12]),
            ));
            continue;       // skip the actual gate run
        }
    }
}

// ... run the gate as before ...
let verdict = gate.verify(&signal, &ctx).await;
if verdict.passed {
    let mut cache = self.hash_cache.lock().unwrap();
    cache.last_pass.insert(gate_name.clone(), current_hash);
    let _ = cache.save(&self.hash_cache_path);
}
verdicts.push(verdict);
```

Add `hash_cache: Mutex<GateHashCache>` and `hash_cache_path: PathBuf`
to `GateService`. Initialise in `new()` from
`workdir.join(".roko/cache/gate-hashes.json")`.

### Step 4 — Make skip a config opt-in

Add a flag to `GateConfig` or `WorkflowConfig`:

```rust
pub struct GateConfig {
    // ...
    pub source_hash_skip: bool,   // default true; off when --no-cache
}
```

Wire `--no-gate-cache` to set this to `false` from the CLI.

### Step 5 — Compose with adaptive thresholds and gate mode

Order of skip checks (top wins):
1. `gate_mode` filter (Plan 10) — explicit user intent.
2. Adaptive threshold skip (existing) — learned safety.
3. Source-hash skip (this plan) — input-cached safety.
4. Run the gate.

Each should produce a `skipped: true` verdict with a unique
`skip_reason` so audits can tell them apart.

---

## Step-by-step execution

1. `git checkout -b perf/11-source-hash-gate-skip`.
2. Add `source_hash.rs` (Step 1). Tests: hash determinism, file-change
   detection, missing-file tolerance.
3. Add `gate_input_set` (Step 2). Test against a fixture workdir.
4. Plug the skip into `run_gates` (Step 3).
5. Add the CLI flag (Step 4).
6. Macro-benchmark (Step 5).
7. PR `perf(gate): skip compile/test/clippy when source unchanged
   (B08)`.

---

## Anti-patterns / things NOT to do

- **Do NOT use mtime-based skip.** Mtime is not reliable across `git
  checkout`, `cp -p`, or some editors that rewrite files atomically.
  Hash-based skip is the contract.
- **Do NOT include build artifacts in the input set.** `target/`,
  `node_modules/`, `.roko/` change every build. Use the `ignore` crate
  with `.gitignore` respect to avoid them.
- **Do NOT include the hash cache itself in the input set.** Recursive
  invalidation; hilarious; spent-an-afternoon-debugging.
- **Do NOT cache failure verdicts.** Only skip on success. A failed
  run's source state isn't a meaningful "skip" candidate.
- **Do NOT write the hash cache from inside an async task without a
  mutex.** The cache is shared across gates running in the same
  service. `Mutex<GateHashCache>` is the contract.
- **Do NOT save the cache to a network filesystem** without an atomic
  write helper. `roko_fs::atomic::atomic_write_bytes` does the
  write-tmp-rename dance — use it, don't roll your own.
- **Do NOT extend the hash to include environment variables.** Two
  builds differing only in `RUSTFLAGS` would share a hash falsely.
  But for **local development workflows**, env-var sensitivity is
  rarely needed and adds complexity. If you do add it, scope to a
  short, vetted list (`RUSTFLAGS`, `CARGO_BUILD_JOBS`, `RUST_VERSION`).
- **Do NOT skip when `--release` and the prior pass was `--debug`.**
  Today the gate config doesn't distinguish; if you add a release
  flag, include it in the hash key.
- **Do NOT compose this with the `roko-gate-cache` filesystem snapshot
  (sccache equivalent) idea** without measuring. They tackle different
  problems and stacking them is bookkeeping-heavy.

---

## Test plan

```rust
#[tokio::test]
async fn second_run_with_unchanged_source_is_skipped() {
    let dir = setup_minimal_rust_project();
    let svc = GateService::new();
    let cfg = GateConfig::compile_only(dir.path());

    let report1 = svc.run_gates(cfg.clone()).await.unwrap();
    let report2 = svc.run_gates(cfg).await.unwrap();

    assert_eq!(report1.verdicts[0].passed, true);
    assert!(report2.verdicts[0].skipped);
    assert!(report2.verdicts[0].skip_reason.as_deref()
        .unwrap_or("").starts_with("source-hash:"));
}

#[tokio::test]
async fn modifying_a_file_invalidates_skip() {
    let dir = setup_minimal_rust_project();
    let svc = GateService::new();
    let cfg = GateConfig::compile_only(dir.path());

    let _ = svc.run_gates(cfg.clone()).await.unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "// touched\n").unwrap();
    let report = svc.run_gates(cfg).await.unwrap();
    assert!(!report.verdicts[0].skipped, "should re-run after edit");
}

#[test]
fn input_set_excludes_target_dir() {
    let dir = setup_minimal_rust_project();
    std::fs::create_dir_all(dir.path().join("target/debug")).unwrap();
    std::fs::write(dir.path().join("target/debug/leak"), "junk").unwrap();
    let inputs = gate_input_set(dir.path(), "compile");
    assert!(inputs.iter().all(|p| !p.starts_with("target/")));
}
```

Macro-benchmark: run `roko run --gates compile,test "noop"` twice in a
row. Second run should be ≥1 s faster.

---

## Rollback plan

- `--no-gate-cache` flag disables the skip immediately.
- Delete `.roko/cache/gate-hashes.json` and `git revert` the wiring;
  the dead `source_hash.rs` module compiles fine.

---

## Status check (acceptance)

- [ ] `GateHashCache` and `compute_source_hash` exist with tests.
- [ ] `gate_input_set` covers compile/clippy/fmt/format-check/test.
- [ ] `GateService::run_gates` consults the cache and emits skipped
      verdicts with `source-hash:` prefix.
- [ ] `--no-gate-cache` CLI flag exists.
- [ ] Macro-benchmark improvement ≥1 s recorded on a no-op repeat run.
