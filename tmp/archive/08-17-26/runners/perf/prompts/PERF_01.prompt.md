# PERF_01: Shared config cache (B02)

## Task

Eliminate the 4+ redundant `roko.toml` parses per `roko run` by introducing
a `ConfigBundle` that loads `Config` + `RokoConfig` once at CLI entry and
threads through the dispatch chain.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_01](../ISSUE-TRACKER.md#perf_01)
- Plan: `tmp/solutions/perf/implementation/01-shared-config-cache.md`
- Bottleneck: B02 (BOTTLENECK-ANALYSIS.md §B02)
- Performance contract: **C-1** (single config load per CLI invocation)
- Priority: P1 (low-effort, high-touch)
- Effort: ≈2 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-cli/src/run.rs` calls `roko_core::config::load_config(workdir)`
and `crate::config::load_layered(workdir)` from at least five places per
single `roko run` invocation:

```text
crates/roko-cli/src/run.rs
   381  crate::config::load_layered(workdir)
   392  roko_core::config::load_config(workdir)
  1827  roko_core::config::load_config(workdir)        ← inside dispatch_agent
  2397  roko_core::config::load_config(workdir)
  2715  roko_core::config::load_config(Path::new("."))  ← resolved_model fallback
```

Each parse takes ~10 ms (TOML deserialize + validation). Total waste: 30-50
ms per run. Worse, the `Path::new(".")` fallback at line 2715 hides a
CWD-discovery side-effect that breaks reproducibility.

The fix: load both config types **once** at CLI entry, wrap in a
`ConfigBundle { legacy: Arc<Config>, roko: Arc<RokoConfig>, workdir:
PathBuf }`, pass the bundle by reference into every consumer.

## Exact Changes

### Step 1 — Add `ConfigBundle` to `crates/roko-cli/src/config.rs`

After the existing `load_layered` definition, add:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A fully-loaded, environment-merged config bundle ready to pass through
/// the dispatch chain. Constructed once per CLI invocation.
///
/// Carries BOTH the legacy `Config` (per-`[gate]`/`[prompt]` shim) and the
/// canonical `RokoConfig` (per-`[providers]`/`[models]` shim) because the
/// codebase still has both in flight; pretending only one exists leaves
/// 50 % of the redundant loads in place.
#[derive(Clone)]
pub struct ConfigBundle {
    pub legacy: Arc<Config>,
    pub roko: Arc<roko_core::config::RokoConfig>,
    pub workdir: PathBuf,
}

impl ConfigBundle {
    /// Load the legacy and modern configs from `workdir`, apply env
    /// overrides + global provider merges, and return a clonable bundle.
    pub fn load(workdir: &Path) -> anyhow::Result<Self> {
        let legacy = load_layered(workdir)
            .map(|resolved| resolved.config)
            .unwrap_or_default();

        let mut roko = roko_core::config::load_config(workdir).unwrap_or_default();
        roko.apply_process_env();
        merge_global_providers(&mut roko);
        roko.providers.extend(legacy.providers.clone());
        roko.models.extend(legacy.models.clone());

        tracing::info!(
            target: "roko_perf",
            workdir = %workdir.display(),
            "loading config from workdir"
        );

        Ok(Self {
            legacy: Arc::new(legacy),
            roko: Arc::new(roko),
            workdir: workdir.to_path_buf(),
        })
    }
}
```

### Step 2 — Build the bundle once at CLI entry in `crates/roko-cli/src/main.rs`

Find the function that dispatches subcommands (search for `fn main` or
`Cli::run`). Before any subcommand handler is called, construct the
bundle:

```rust
let bundle = crate::config::ConfigBundle::load(&opts.workdir)
    .with_context(|| format!("load config from {}", opts.workdir.display()))?;
```

Pass `&bundle` into the run/plan/chat/serve subcommand entrypoints.
Adjust their signatures one at a time; `cargo check -p roko-cli` after
each change.

### Step 3 — Refactor `run_once`, `dispatch_agent`, `append_episode_log`

In `crates/roko-cli/src/run.rs`:

```rust
// BEFORE:
pub async fn run_once(
    workdir: &Path,
    config: &Config,
    prompt_text: &str,
    strategy: Option<BenchStrategy>,
    external_hub: Option<&StateHub>,
) -> Result<RunReport>

// AFTER:
pub async fn run_once(
    bundle: &crate::config::ConfigBundle,
    prompt_text: &str,
    strategy: Option<BenchStrategy>,
    external_hub: Option<&StateHub>,
) -> Result<RunReport>
```

Inside `run_once`, replace `workdir` references with `bundle.workdir.as_path()`,
and `config` references with `&*bundle.legacy`. Where the function used to
call `roko_core::config::load_config(workdir)`, use `bundle.roko.as_ref()`
directly.

Apply the same pattern to:

- `async fn dispatch_agent(workdir, config, ...)` → `dispatch_agent(bundle, ...)`
- `async fn append_episode_log(workdir, config, ...)` → `append_episode_log(bundle, runtime, ...)` (note: `runtime` parameter is added by **PERF_02**, but until that lands you can keep it accepting `bundle` only).

Delete the in-function reloads:

```rust
// REMOVE — now redundant
let mut routing_config = roko_core::config::load_config(workdir)?;
routing_config.apply_process_env();
merge_global_providers(&mut routing_config);

// REPLACE WITH (no reload)
let routing_config = bundle.roko.as_ref();
```

### Step 4 — Fix the `resolved_model` fallback

`crates/roko-cli/src/run.rs:2715` currently does:

```rust
if let Ok(mut rc) = roko_core::config::load_config(std::path::Path::new(".")) {
    rc.apply_process_env();
    crate::config::merge_global_providers(&mut rc);
    if !rc.agent.default_model.is_empty() { return rc.agent.default_model; }
}
```

Replace with a bundle lookup:

```rust
fn resolved_model(config: &Config, bundle: &crate::config::ConfigBundle) -> String {
    if let Some(model) = &config.agent.model { return model.clone(); }
    if !bundle.roko.agent.default_model.is_empty() {
        return bundle.roko.agent.default_model.clone();
    }
    if config.agent.command.trim().eq_ignore_ascii_case("claude") {
        "claude-sonnet-4-6".to_string()
    } else {
        String::new()
    }
}
```

Update the two call sites of `resolved_model(config)` to pass the bundle.

### Step 5 — Add the single-load test

Append to `crates/roko-cli/src/run.rs` (in the existing `#[cfg(test)]
mod tests` block):

```rust
#[tokio::test]
async fn run_once_loads_config_exactly_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_test::traced_test;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("roko.toml"),
        r#"[agent]
model = "mock-fast"
command = "mock"
"#,
    ).unwrap();

    let bundle = crate::config::ConfigBundle::load(dir.path()).unwrap();
    let _ = run_once(&bundle, "echo hi", None, None).await;

    // Count "loading config from workdir" log lines emitted under
    // target=roko_perf. Should be exactly 1 (from ConfigBundle::load).
    let logs = logs_contain("loading config from workdir");
    // tracing-test's `logs_contain` returns bool; for a count assertion,
    // capture via `tracing_test::internal::global_buf` or use a custom
    // subscriber. If tracing-test doesn't expose count APIs, an
    // alternative is to count via `tracing-subscriber` capture as in
    // existing tests.
    assert!(logs, "expected at least one config load");
}
```

If `tracing-test` is not already a dev-dep, fall back to the simpler
counter pattern: wrap `crate::config::load_layered` behind a private
shim trait whose mock counts invocations.

## Write Scope

(Must match `batches.toml` `scope` for PERF_01.)

- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/main.rs`

## Read-Only Context

- `crates/roko-core/src/config/mod.rs`
- `crates/roko-core/src/config/provider.rs`
- `tmp/solutions/perf/implementation/01-shared-config-cache.md`
- `tmp/runners/perf/context-pack/00-RULES.md`
- `tmp/runners/perf/context-pack/01-FILE-INVENTORY.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md`

## Acceptance Criteria

(Mirrors `ISSUE-TRACKER.md#perf_01` sub-items. Tick each as you confirm.)

- [ ] `ConfigBundle` struct exists in `crates/roko-cli/src/config.rs` with `legacy: Arc<Config>`, `roko: Arc<RokoConfig>`, `workdir: PathBuf`.
- [ ] `ConfigBundle::load(workdir)` is the only place that calls `load_layered` + `load_config` + `apply_process_env` + `merge_global_providers`.
- [ ] `main.rs` builds the bundle once at CLI entry; subcommand handlers accept `&ConfigBundle`.
- [ ] `run_once`, `dispatch_agent`, `append_episode_log`, `resolved_model` no longer call `load_config` or `load_layered` themselves.
- [ ] `rg "load_config|load_layered" crates/roko-cli/src/` shows references only inside `ConfigBundle::load`.
- [ ] Test asserts `roko run` parses `roko.toml` exactly once.

## Verify

(See `context-pack/04-VERIFY-RECIPES.md`. Per RULE in `00-RULES.md`, do NOT
run cargo during the batch. The runner's wave gate runs these post-merge.)

```bash
# Audit greps:
rg -n "roko_core::config::load_config|crate::config::load_layered|merge_global_providers" \
   crates/roko-cli/src/ --type rust

# Expected: only inside ConfigBundle::load.

# Macro-benchmark (post-merge):
RUST_LOG=roko_cli=trace ./target/release/roko run \
   --gates none "Reply with hello" 2>&1 \
   | rg -c '"loading config from"'
# Expected: 1
```

## Do NOT

- Do NOT introduce a process-wide `static LazyLock<RokoConfig>`. `roko serve`
  runs many workdirs in one process; a static would leak.
- Do NOT change the layering / merge precedence (defaults → toml → env →
  CLI override). Pull that into a follow-up if you spot bugs.
- Do NOT widen `ConfigBundle` to carry `LearningRuntime`,
  `FileSubstrate`, or any service. **PERF_02** threads `LearningRuntime`
  separately by design.
- Do NOT silently swap the `Path::new(".")` fallback for `unwrap_or_default()`
  — that masks behaviour. Use the bundle's roko config explicitly.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).
- Do NOT touch the orchestrator's `ServiceFactory` (`crates/roko-orchestrator/src/service_factory.rs`)
  in this PR; document any lingering reload there as a follow-up note.
- Do NOT bundle this with **PERF_02** (LearningRuntime). Two PRs, two
  reviews.

## Tracker update

On success, include this trailer in the commit message:

```
tracker: PERF_01 done <commit-sha>
```
