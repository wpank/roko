# PERF_06: PromptAssemblyService convention cache (B12+B14)

## Task

`PromptAssemblyService::assemble` walks `src/`, reads up to 12 source
files, and re-detects project conventions on every dispatch (30-100 ms
per call). Cache the result keyed on `(workdir, Cargo.toml mtime, src/
mtime)`; bound the cache with `lru::LruCache` (cap 8); ensure the
async-context cache miss does NOT block the Tokio runtime.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_06](../ISSUE-TRACKER.md#perf_06)
- Plan: `tmp/solutions/perf/implementation/06-prompt-assembly-cache.md`
- Bottlenecks: B12 + B14 (BOTTLENECK-ANALYSIS.md)
- Performance contract: **C-6** (one read_dir per workdir+run)
- Priority: P1
- Effort: ≈3 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-compose/src/prompt_assembly_service.rs:650-711`:

```rust
fn detect_workdir_conventions(workdir: &Path) -> Option<String> {
    let cargo_toml = read_to_string_if_exists(&workdir.join("Cargo.toml")).unwrap_or_default();
    let (source_samples, file_listing) = collect_source_context(workdir);
    // ... build conventions fragment ...
}

fn collect_source_context_from(dir, root, samples, listing) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };  // ← SYNC IO
    // recursive walk, reads up to 12 .rs files
}
```

Two problems:

1. **B12.** Recompute on every `assemble()` call. For a standard
   workflow (≥2 dispatches), that's 2× the work; for a 10-task plan, 10×.
2. **B14.** Synchronous `std::fs::read_dir` inside an `async fn` blocks
   the Tokio runtime. (See AP-ASYNC-1.)

Cache invariants:

- Conventions depend on `Cargo.toml` content + `src/` file *names*.
- Editing a function body inside an existing `.rs` file does NOT change
  conventions (the convention struct captures language/build/style, not
  function bodies).
- Add/remove/rename a file in `src/` bumps the directory's mtime.
- Editing `Cargo.toml` bumps its mtime.

So the cache key `(workdir, cargo_mtime, src_dir_mtime)` is **necessary
and sufficient** for the practical change surface.

## Exact Changes

### Step 1 — Add `lru` to `crates/roko-compose/Cargo.toml`

In the `[dependencies]` section:

```toml
lru = "0.12"
```

If `lru` is already present (check first with `rg lru crates/roko-compose/Cargo.toml`),
skip this step.

### Step 2 — Add cache types to `prompt_assembly_service.rs`

At the top of `crates/roko-compose/src/prompt_assembly_service.rs`,
import:

```rust
use std::num::NonZeroUsize;
use std::time::SystemTime;
use lru::LruCache;
```

Add the cache entry type (place near the existing helper functions, e.g.,
above `fn detect_workdir_conventions`):

```rust
const CONVENTION_CACHE_CAPACITY: usize = 8;

/// Cached convention-detection result for one workdir.
///
/// **Cache key inputs:** `(workdir, cargo_mtime, src_dir_mtime)`.
/// `Cargo.toml` mtime captures any project-config change; `src/`
/// directory mtime captures add/remove/rename of files. Editing a file
/// body inside `src/` does NOT change `src/`'s mtime — that's
/// acceptable because conventions (build system, lint config, naming
/// style) are stable across body edits.
///
/// **TTL:** none. mtime comparison is sufficient.
#[derive(Clone)]
struct ConventionCacheEntry {
    fragment: Option<String>,
    source_samples: Vec<String>,
    file_listing: Vec<String>,
    cargo_mtime: Option<SystemTime>,
    src_dir_mtime: Option<SystemTime>,
}
```

### Step 3 — Add the cache field to `PromptAssemblyService`

Find the `pub struct PromptAssemblyService { ... }` declaration
(≈line 47). Add a new field:

```rust
pub struct PromptAssemblyService {
    // ... existing fields unchanged ...

    /// Per-workdir cache of convention-detection results. See
    /// `ConventionCacheEntry` for the invalidation contract.
    convention_cache: Mutex<LruCache<PathBuf, ConventionCacheEntry>>,
}
```

Update `PromptAssemblyService::new()`:

```rust
pub fn new() -> Self {
    Self {
        // ... existing initialisers unchanged ...
        convention_cache: Mutex::new(LruCache::new(
            NonZeroUsize::new(CONVENTION_CACHE_CAPACITY).unwrap(),
        )),
    }
}
```

Add `tracing::info!` at construction so PERF regression sniffers can
spot accidental per-dispatch reconstruction:

```rust
pub fn new() -> Self {
    tracing::info!(target: "roko_perf", "PromptAssemblyService instantiated");
    Self { /* ... */ }
}
```

### Step 4 — Add `cached_conventions` and `cached_file_listing`

```rust
fn mtime_of(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn compute_convention_entry(
    workdir: &Path,
    cargo_mtime: Option<SystemTime>,
    src_dir_mtime: Option<SystemTime>,
) -> ConventionCacheEntry {
    let cargo_toml = read_to_string_if_exists(&workdir.join("Cargo.toml")).unwrap_or_default();
    let mut source_samples = Vec::new();
    let mut file_listing = Vec::new();
    collect_source_context_from(
        &workdir.join("src"),
        workdir,
        &mut source_samples,
        &mut file_listing,
    );

    let fragment = if cargo_toml.is_empty() && source_samples.is_empty() && file_listing.is_empty() {
        None
    } else {
        let source_refs: Vec<&str> = source_samples.iter().map(String::as_str).collect();
        let file_refs: Vec<&str> = file_listing.iter().map(String::as_str).collect();
        let conventions = detect_conventions(&cargo_toml, &source_refs, &file_refs);
        let fragment = conventions.to_prompt_fragment();
        (!fragment.trim().is_empty()).then_some(fragment)
    };

    ConventionCacheEntry {
        fragment,
        source_samples,
        file_listing,
        cargo_mtime,
        src_dir_mtime,
    }
}

impl PromptAssemblyService {
    /// Async cache lookup for convention fragment. On miss, the
    /// (synchronous) computation is moved off-runtime via
    /// `tokio::task::spawn_blocking`.
    async fn cached_conventions(&self, workdir: &Path) -> Option<String> {
        let cargo_mtime = mtime_of(&workdir.join("Cargo.toml"));
        let src_dir_mtime = mtime_of(&workdir.join("src"));

        // Hot path: scope the cache lock so it does NOT cross the await
        // below (AP-ASYNC-2).
        {
            let mut cache = self.convention_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = cache.get(workdir) {
                if entry.cargo_mtime == cargo_mtime && entry.src_dir_mtime == src_dir_mtime {
                    return entry.fragment.clone();
                }
            }
        } // ← lock released

        // Cache miss: run the directory walk on a blocking thread so
        // the Tokio runtime is not stalled (AP-ASYNC-1).
        let workdir_owned = workdir.to_path_buf();
        let entry = tokio::task::spawn_blocking(move || {
            compute_convention_entry(&workdir_owned, cargo_mtime, src_dir_mtime)
        })
        .await
        .expect("convention_entry spawn_blocking");

        let fragment = entry.fragment.clone();
        let mut cache = self.convention_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.put(workdir.to_path_buf(), entry);
        fragment
    }

    /// Same cache, returning the file listing only (used by the
    /// workspace-map injector).
    async fn cached_file_listing(&self, workdir: &Path) -> Vec<String> {
        // Reuse cached_conventions to populate the entry, then read
        // the listing field.
        let _ = self.cached_conventions(workdir).await;
        let cache = self.convention_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.peek(workdir)                     // peek does not bump LRU recency
            .map(|e| e.file_listing.clone())
            .unwrap_or_default()
    }

    /// Test-only accessor for cache size.
    #[cfg(test)]
    pub fn cache_len_for_test(&self) -> usize {
        self.convention_cache
            .lock()
            .map(|c| c.len())
            .unwrap_or(0)
    }
}
```

> **Critical anti-pattern check.** Re-read your `cached_conventions`
> body. The `parking_lot::Mutex` / `std::sync::Mutex` guard MUST be
> dropped before the `.await` on `spawn_blocking`. The pattern above
> uses an inner scope `{ ... }` to ensure this. Do NOT remove it.

### Step 5 — Replace `detect_workdir_conventions` callers

Old:

```rust
fn conventions_for_spec(spec: &PromptSpec, default_conventions: Option<&str>) -> Option<String> {
    spec.workdir.as_deref()
        .and_then(detect_workdir_conventions)
        .or_else(|| default_conventions.map(ToOwned::to_owned))
}
```

New (note: now async + takes `&PromptAssemblyService`):

```rust
async fn conventions_for_spec(
    spec: &PromptSpec,
    default_conventions: Option<&str>,
    svc: &PromptAssemblyService,
) -> Option<String> {
    if let Some(workdir) = spec.workdir.as_deref() {
        if let Some(frag) = svc.cached_conventions(workdir).await {
            return Some(frag);
        }
    }
    default_conventions.map(ToOwned::to_owned)
}
```

Same conversion for `workspace_map_for_spec`:

```rust
async fn workspace_map_for_spec(
    spec: &PromptSpec,
    svc: &PromptAssemblyService,
) -> Option<String> {
    let workdir = spec.workdir.as_deref()?;
    let listing = svc.cached_file_listing(workdir).await;
    workspace_map_from_file_listing(&listing)
}
```

### Step 6 — Thread the awaits through `PromptAssembler::assemble`

`impl PromptAssembler for PromptAssemblyService { async fn assemble(...) }`
is already async. Update its body to call the new async helpers and
pass `self`:

```rust
let conventions = conventions_for_spec(&spec, self.default_conventions.as_deref(), self).await;
let workspace_map = workspace_map_for_spec(&spec, self).await;
```

If `conventions_for_spec` and `workspace_map_for_spec` are free
functions (not methods), keep them as `async fn` and pass `&self`
through. Either pattern works.

### Step 7 — Tests

Append to `crates/roko-compose/src/prompt_assembly_service.rs`'s test
module:

```rust
#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn write_minimal_rust_project(dir: &Path) {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.0.1"
edition = "2021"
"#,
        ).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn x() {}").unwrap();
    }

    #[tokio::test]
    async fn cached_conventions_avoid_disk_on_second_call() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_rust_project(dir.path());

        let svc = PromptAssemblyService::new();

        let _ = svc.cached_conventions(dir.path()).await;
        // Second call: cache hit, no spawn_blocking.
        let cache_len_before = svc.cache_len_for_test();
        let _ = svc.cached_conventions(dir.path()).await;
        let cache_len_after = svc.cache_len_for_test();
        assert_eq!(cache_len_before, cache_len_after, "no new entry on cache hit");
        assert_eq!(cache_len_after, 1);
    }

    #[tokio::test]
    async fn cache_invalidates_on_cargo_toml_mtime_change() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_rust_project(dir.path());
        let svc = PromptAssemblyService::new();

        let first = svc.cached_conventions(dir.path()).await;
        assert!(first.is_some());

        // Bump mtime: rewrite Cargo.toml.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let cargo = dir.path().join("Cargo.toml");
        let mut existing = std::fs::read_to_string(&cargo).unwrap();
        existing.push_str("\n# touched\n");
        std::fs::write(&cargo, existing).unwrap();

        // Second call recomputes (we can verify by content equality of
        // the fragment, since it should still describe the same project).
        let second = svc.cached_conventions(dir.path()).await;
        assert!(second.is_some());
        // The cache entry was overwritten in place, so length stays at 1.
        assert_eq!(svc.cache_len_for_test(), 1);
    }

    #[tokio::test]
    async fn lru_evicts_oldest_workdir() {
        let svc = PromptAssemblyService::new();
        let dirs: Vec<_> = (0..(CONVENTION_CACHE_CAPACITY + 2))
            .map(|i| {
                let dir = tempfile::tempdir().unwrap();
                write_minimal_rust_project(dir.path());
                dir
            })
            .collect();
        for d in &dirs {
            let _ = svc.cached_conventions(d.path()).await;
        }
        assert_eq!(svc.cache_len_for_test(), CONVENTION_CACHE_CAPACITY);
    }
}
```

### Step 8 — Confirm the service is reused (audit, no code change)

Run:

```bash
rg -n 'PromptAssemblyService::new\|Arc::new(PromptAssemblyService' crates/ --type rust
```

The `tracing::info!` line you added in Step 3 should fire **once** per
`roko run`. If `ServiceFactory::build` reconstructs the service on
every dispatch, the cache is useless. Document any reconstruction site
in the commit body as a follow-up — do NOT fix it in this batch.

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-compose/Cargo.toml`

## Read-Only Context

- `crates/roko-compose/src/conventions.rs`
- `crates/roko-compose/src/lib.rs`
- `crates/roko-runtime/src/effect_driver.rs`
- `tmp/solutions/perf/implementation/06-prompt-assembly-cache.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-CACHE-*, AP-ASYNC-1, AP-ASYNC-2)

## Acceptance Criteria

- [ ] `lru` dependency added to `crates/roko-compose/Cargo.toml`.
- [ ] `ConventionCacheEntry` struct + `compute_convention_entry` helper added.
- [ ] `PromptAssemblyService` carries `convention_cache: Mutex<LruCache<PathBuf, ConventionCacheEntry>>` (cap 8).
- [ ] `cached_conventions` and `cached_file_listing` accessors exist.
- [ ] All callers of `detect_workdir_conventions` / `collect_source_context` route through the cache.
- [ ] Async path uses `tokio::task::spawn_blocking` for the cache miss branch.
- [ ] No `std::sync::Mutex` held across `.await` (the inner scope `{ ... }` pattern is in place).
- [ ] `tracing::info!(target: "roko_perf", "PromptAssemblyService instantiated")` emitted in `new()`.
- [ ] Tests `cached_conventions_avoid_disk_on_second_call`, `cache_invalidates_on_cargo_toml_mtime_change`, `lru_evicts_oldest_workdir` pass.

## Verify

```bash
# Audit: free-function detect_workdir_conventions should not be
# called from outside the cache helper anymore.
rg -n 'detect_workdir_conventions' crates/roko-compose/src/
# Expected: only inside compute_convention_entry.

rg -n 'collect_source_context' crates/roko-compose/src/
# Expected: only inside compute_convention_entry + helper itself.

# Lock-across-await audit:
rg -nU --multiline 'lock\(\).*?\.await' crates/roko-compose/src/prompt_assembly_service.rs
# Expected: empty.
```

## Do NOT

- Do NOT make the cache `static`. Multiple workdirs in `roko serve`
  must not share entries.
- Do NOT use file-content hash as cache key (AP-CACHE-2/4). Hashing 12
  files each call costs more than re-walking.
- Do NOT walk `src/` recursively for invalidation. Stat-ing every file
  is O(N); the directory mtime check is O(1) and sufficient for
  `add/remove/rename` change classes.
- Do NOT block the Tokio runtime with synchronous `std::fs` in the
  async assembler path (AP-ASYNC-1). Use `spawn_blocking`.
- Do NOT hold the cache lock across the `.await` (AP-ASYNC-2). The
  inner scope `{ ... }` pattern is mandatory.
- Do NOT cache the `PromptSpec`'s task description. Tasks vary per
  call; only the workdir-derived facts are cacheable.
- Do NOT increase `SOURCE_SAMPLE_LIMIT` "to make conventions richer"
  in the same plan — that adds IO work, not removes it.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_06 done <commit-sha>
```
