# 06 — PromptAssemblyService Convention Cache (B12 + B14)

> Bottleneck: every `PromptAssemblyService::assemble` call walks `src/`,
> reads up to 12 source files, and re-detects project conventions —
> 30–100 ms per call, repeated for every agent dispatch.
>
> Target savings: 50–150 ms / run (more for plans with many tasks).
> Effort: ≈3 h. Risk: medium (stale conventions if files change).

---

## Goal & success criteria

After this change:

1. `detect_workdir_conventions(workdir)` is computed at most once per
   `(workdir, mtime-fingerprint)` pair per process.
2. `collect_source_context(workdir)` returns from cache when neither
   `src/` nor `Cargo.toml` has changed.
3. The cache is bounded (LRU, 8 entries by default) and exposes a
   single `clear_for_test()` method for test isolation.
4. The same `PromptAssemblyService` instance is reused across multiple
   dispatches in the same run (verified by tracing).

Done when:

- A new test asserts the second `assemble` call for the same workdir
  does **not** call `std::fs::read_dir`.
- A new test asserts mtime change invalidates the cache.
- Macro-benchmark p50 wall-time drops by ≥40 ms vs the plan-05 baseline
  on the standard workflow.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B12 + §B14,
  `OPTIMIZATION-PLAYBOOK.md` §6.
- Current implementation (verified live):

  ```text
  crates/roko-compose/src/prompt_assembly_service.rs
   504  fn conventions_for_spec(spec, default) {
   505      spec.workdir.as_deref().and_then(detect_workdir_conventions)
   506          .or_else(|| default.map(ToOwned::to_owned))
   507  }
   650  fn detect_workdir_conventions(workdir) -> Option<String> {
   651      let cargo_toml = read_to_string_if_exists(&workdir.join("Cargo.toml"))...
   652      let (source_samples, file_listing) = collect_source_context(workdir);
   ...   }
   669  fn collect_source_context(workdir) {
   672      collect_source_context_from(&workdir.join("src"), workdir, ...);
   ...   }
   681  fn collect_source_context_from(dir, root, samples, listing) {
   687      let Ok(entries) = std::fs::read_dir(dir) else { return; };
   ...   }
   ```

- The async assembler awaits a synchronous `std::fs::read_dir` walk —
  this also blocks the Tokio runtime, mentioned in
  `BOTTLENECK-ANALYSIS.md` as a side concern. We address both in this
  plan.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-compose/src/prompt_assembly_service.rs` (≈1 050 LOC) | Primary edit site. |
| `crates/roko-compose/src/conventions.rs` | `detect_conventions` returns the structure that becomes the prompt fragment. |
| `crates/roko-compose/src/lib.rs` | Public re-exports; verify `PromptAssemblyService` is the only entry. |
| `crates/roko-runtime/src/effect_driver.rs` | Confirms how `EffectServices::prompt_assembler` is shared across dispatches in a run. |

---

## Code-level plan

### Step 1 — Define a per-workdir cache entry

Add inside `prompt_assembly_service.rs`:

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

const CONVENTION_CACHE_CAPACITY: usize = 8;

#[derive(Clone)]
struct ConventionCacheEntry {
    /// The serialized prompt fragment ready to inject.
    fragment: Option<String>,
    /// Source samples + file listing, used by `workspace_map_for_spec`.
    source_samples: Vec<String>,
    file_listing: Vec<String>,
    /// Cache key inputs:
    cargo_mtime: Option<SystemTime>,
    src_dir_mtime: Option<SystemTime>,
}
```

**Cache key inputs.** The triple `(cargo_mtime, src_dir_mtime,
src_root_present)` covers the practical change surface:

- Editing `Cargo.toml` changes its mtime.
- Adding/removing/renaming a file in `src/` (or any subdir) changes
  the **directory's** mtime on every Unix variant we ship to.

**What this misses:** edits to existing source files inside `src/`
typically do not change `src/`'s own mtime. That's acceptable here —
conventions (build system, lint config, naming style) rarely change
when you edit a function body. If a future user reports stale
conventions after editing source, downgrade the cache to TTL-based
invalidation (e.g., 30 s), not a recursive mtime walk.

### Step 2 — Wire the cache into the service

```rust
pub struct PromptAssemblyService {
    // ... existing fields ...
    convention_cache: Mutex<lru::LruCache<PathBuf, ConventionCacheEntry>>,
}

impl PromptAssemblyService {
    pub fn new() -> Self {
        Self {
            // ... existing init ...
            convention_cache: Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(CONVENTION_CACHE_CAPACITY).unwrap(),
            )),
        }
    }
}
```

Add `lru = "0.12"` to `crates/roko-compose/Cargo.toml` if it isn't
already there (`cargo add lru -p roko-compose`).

### Step 3 — Replace `detect_workdir_conventions` and `collect_source_context` callers

Add a method on the service:

```rust
impl PromptAssemblyService {
    fn cached_conventions(&self, workdir: &Path) -> Option<String> {
        let cargo_mtime = mtime_of(&workdir.join("Cargo.toml"));
        let src_dir_mtime = mtime_of(&workdir.join("src"));

        // Cache hit?
        if let Ok(mut cache) = self.convention_cache.lock() {
            if let Some(entry) = cache.get(workdir) {
                if entry.cargo_mtime == cargo_mtime && entry.src_dir_mtime == src_dir_mtime {
                    return entry.fragment.clone();
                }
            }
        }

        // Miss: compute fresh.
        let entry = compute_convention_entry(workdir, cargo_mtime, src_dir_mtime);
        let fragment = entry.fragment.clone();
        if let Ok(mut cache) = self.convention_cache.lock() {
            cache.put(workdir.to_path_buf(), entry);
        }
        fragment
    }
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}
```

`compute_convention_entry` is a renamed merge of the old
`detect_workdir_conventions` + `collect_source_context`:

```rust
fn compute_convention_entry(
    workdir: &Path,
    cargo_mtime: Option<SystemTime>,
    src_dir_mtime: Option<SystemTime>,
) -> ConventionCacheEntry {
    let cargo_toml = read_to_string_if_exists(&workdir.join("Cargo.toml")).unwrap_or_default();
    let mut source_samples = Vec::new();
    let mut file_listing = Vec::new();
    collect_source_context_from(&workdir.join("src"), workdir,
        &mut source_samples, &mut file_listing);

    let fragment = if cargo_toml.is_empty() && source_samples.is_empty() && file_listing.is_empty() {
        None
    } else {
        let source_refs: Vec<&str> = source_samples.iter().map(String::as_str).collect();
        let file_refs: Vec<&str> = file_listing.iter().map(String::as_str).collect();
        let conventions = detect_conventions(&cargo_toml, &source_refs, &file_refs);
        let f = conventions.to_prompt_fragment();
        (!f.trim().is_empty()).then_some(f)
    };

    ConventionCacheEntry {
        fragment,
        source_samples,
        file_listing,
        cargo_mtime,
        src_dir_mtime,
    }
}
```

Then change the call sites:

```rust
fn conventions_for_spec(spec: &PromptSpec, default: Option<&str>, svc: &PromptAssemblyService) -> Option<String> {
    spec.workdir.as_deref()
        .and_then(|wd| svc.cached_conventions(wd))
        .or_else(|| default.map(ToOwned::to_owned))
}

fn workspace_map_for_spec(spec: &PromptSpec, svc: &PromptAssemblyService) -> Option<String> {
    let workdir = spec.workdir.as_deref()?;
    // Look up the cached file listing rather than walking the filesystem again.
    let listing = svc.cached_file_listing(workdir);
    workspace_map_from_file_listing(&listing)
}
```

`cached_file_listing` is a thin accessor that hits the same cache and
returns the pre-computed `file_listing` from the entry.

### Step 4 — Async wrapper for tokio safety

If you keep `compute_convention_entry` synchronous (it is; uses
`std::fs`), make sure `cached_conventions` is called outside any async
context that benefits from non-blocking IO. Currently the assembler is
called from `EffectDriver::spawn_agent` which is async — the synchronous
walk blocks the Tokio runtime.

Two acceptable approaches:

1. **Cheap (recommended).** Wrap the *cache miss* branch in
   `tokio::task::spawn_blocking`:

   ```rust
   if entry.is_none() {
       let workdir = workdir.to_path_buf();
       let entry = tokio::task::spawn_blocking(move || {
           compute_convention_entry(&workdir, cargo_mtime, src_dir_mtime)
       }).await.expect("spawn_blocking");
       // ...store and return...
   }
   ```

   To do this, change `cached_conventions` to `async fn` and the
   `PromptAssembler::assemble` impl threads the await through.

2. **Pure async.** Replace the body with `tokio::fs::read_dir` +
   `tokio::fs::read_to_string`. More invasive; do not bundle into this
   plan unless you have measured contention.

Pick option 1.

### Step 5 — Confirm the service is reused

Audit `crates/roko-runtime/src/effect_driver.rs` and the `EffectServices`
construction in `crates/roko-cli/src/run.rs::build_workflow_effect_services`.
Verify that `prompt_assembler: Arc<dyn PromptAssembler>` is constructed
once per workflow run (it is, via `ServiceFactory::build`). If a future
refactor reconstructs it per dispatch, the cache is useless.

Add an info-level trace at construction so future regressions surface:

```rust
// inside ServiceFactory or the construction site
tracing::info!(target: "roko_perf", "PromptAssemblyService instantiated");
```

A correct run logs this exactly once.

---

## Step-by-step execution

1. `git checkout -b perf/06-prompt-assembly-cache`.
2. Add `lru` dependency.
3. Add the cache fields, methods, and `compute_convention_entry`.
4. Wire `cached_conventions` and `cached_file_listing` into the
   call sites. `cargo build -p roko-compose`.
5. Make `cached_conventions` async; thread the await through
   `PromptAssembler::assemble` (already async).
6. Wrap miss branch in `spawn_blocking` (Step 4).
7. Add tests (below).
8. `cargo test -p roko-compose --release`; macro-benchmark.
9. Open PR `perf(compose): cache workdir convention detection (B12+B14)`.

---

## Anti-patterns / things NOT to do

- **Do NOT make the cache `static`.** Multiple workdirs in `roko serve`
  must not share entries. Per-service-instance is correct.
- **Do NOT use the file-content hash as cache key.** Hashing 12 files
  each call costs more than re-walking the directory. Mtime is fine
  here; conventions are stable across edits.
- **Do NOT walk `src/` recursively for invalidation.** Stat-ing every
  file is O(N) where N is the project size; that defeats the purpose.
  The directory mtime check is O(1) and is sufficient for the
  `add/remove/rename` change classes that matter for conventions.
- **Do NOT block the Tokio runtime** with synchronous `std::fs` in
  the async assembler. This was an existing bug; do not perpetuate it.
  `spawn_blocking` is the contract; if you skip it, expect tail latency
  spikes during heavy concurrency in `roko serve`.
- **Do NOT forget to drop the cache lock before the await** in the
  async path. The pattern is:

  ```rust
  let cached = {
      let mut cache = self.convention_cache.lock().unwrap();
      cache.get(workdir).cloned()
  };
  // ... await stuff that recompute ...
  ```

  Holding `Mutex` (`std::sync::Mutex`) across `.await` either deadlocks
  or fails compilation depending on the runtime. Use the scope trick
  above OR switch to `tokio::sync::Mutex`.
- **Do NOT cache the `PromptSpec`'s task description.** Tasks vary per
  call; only the workdir-derived facts (conventions, file listing) are
  cacheable.
- **Do NOT increase `SOURCE_SAMPLE_LIMIT` "to make conventions richer"**
  in the same plan — that adds IO work, not removes it. Keep the
  existing cap.

---

## Test plan

```rust
#[tokio::test]
async fn cached_conventions_avoid_disk_on_second_call() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_rust_project(dir.path());
    let svc = PromptAssemblyService::new();

    // Prime cache.
    let _ = svc.cached_conventions(dir.path()).await;

    // Second call must not invoke read_dir.
    let counter = install_readdir_counter();
    let _ = svc.cached_conventions(dir.path()).await;
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn cache_invalidates_on_cargo_toml_mtime_change() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_rust_project(dir.path());
    let svc = PromptAssemblyService::new();
    let _ = svc.cached_conventions(dir.path()).await;

    // Touch Cargo.toml — bumps mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let cargo = dir.path().join("Cargo.toml");
    std::fs::write(&cargo, std::fs::read_to_string(&cargo).unwrap() + "\n").unwrap();

    let counter = install_readdir_counter();
    let _ = svc.cached_conventions(dir.path()).await;
    assert!(counter.load(Ordering::Relaxed) > 0,
        "cache must invalidate when Cargo.toml mtime changes");
}

#[tokio::test]
async fn lru_evicts_oldest_workdir() {
    let svc = PromptAssemblyService::new();
    for i in 0..(CONVENTION_CACHE_CAPACITY + 2) {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_rust_project(dir.path());
        let _ = svc.cached_conventions(dir.path()).await;
    }
    // Internal assertion via a test-only accessor.
    assert_eq!(svc.cache_len_for_test(), CONVENTION_CACHE_CAPACITY);
}
```

`install_readdir_counter` requires either swapping `std::fs::read_dir`
behind a private trait OR (acceptable shortcut) checking that the
returned `Vec`s are deeply equal between calls (proves cache hit).

---

## Rollback plan

- All changes are local to `prompt_assembly_service.rs`. `git revert`
  is mechanical.
- Emergency disable: set `CONVENTION_CACHE_CAPACITY` to `1` and bypass
  the cache in `cached_conventions` if a stale-conventions bug is
  reported in production. This restores the pre-cache behaviour at the
  cost of the perf win.

---

## Status check (acceptance)

- [ ] `cached_conventions` and `cached_file_listing` cover all callers
      of `detect_workdir_conventions` / `collect_source_context`.
- [ ] Async assembler no longer blocks the runtime on the miss path
      (uses `spawn_blocking`).
- [ ] All three tests pass.
- [ ] Macro-benchmark improvement of ≥40 ms recorded.
