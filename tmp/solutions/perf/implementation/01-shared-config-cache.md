# 01 — Shared Config Cache (B02)

> Bottleneck: `roko.toml` is parsed from disk **4+ times** per `roko run`.
> Each parse is ~10 ms. Target savings: 30 ms / run.
> Effort: ≈2 h. Risk: low.

---

## Goal & success criteria

After this change, **a single `roko run` invocation parses `roko.toml`
exactly once.** The parsed config is shared as `Arc<RokoConfig>` (and
`Arc<Config>` where the legacy `Config` shim is still used) through the
entire dispatch chain, including `dispatch_agent`, `append_episode_log`,
`resolved_model`, and the routing/learning helpers.

Done when:

- `RUST_LOG=roko_cli=trace cargo run --release -p roko-cli -- run --gates none "echo hi"`
  shows **exactly one** `loading config from <path>` log line.
- A new test in `crates/roko-cli/src/run.rs` asserts that the loader is
  called once per `run_once`.
- Macro-benchmark p50 wall-time drops by ≥20 ms compared to baseline
  (`BENCHMARK-RESULTS.md` §3.1, line "Config load").

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B02.
- Original fix sketch: `OPTIMIZATION-PLAYBOOK.md` §1.
- Codebase currently calls `roko_core::config::load_config(workdir)` and
  `crate::config::load_layered(workdir)` from at least five distinct
  call sites (verified at the time of writing):

```text
crates/roko-cli/src/run.rs
  381  crate::config::load_layered(workdir)
  392  roko_core::config::load_config(workdir)
  1827 roko_core::config::load_config(workdir)        # dispatch_agent
  2397 roko_core::config::load_config(workdir)        # append_episode_log path
  2715 roko_core::config::load_config(Path::new("."))  # resolved_model fallback
```

Run a `rg -n "load_config|load_layered" crates/roko-cli/src/` before
starting and update the list — the count drifts as the codebase moves.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/run.rs` | Most call sites live here; understand `dispatch_agent`, `append_episode_log`, `resolved_model`, `run_once`. |
| `crates/roko-cli/src/config.rs` | `load_layered` definition + its `ResolvedConfig`. |
| `crates/roko-core/src/config/mod.rs` | `RokoConfig::load_config`, `apply_process_env`. |
| `crates/roko-core/src/config/provider.rs` | `merge_global_providers`. |
| `crates/roko-cli/src/main.rs` | The current CLI entry; this is where the load should happen. |

Skim only — you do not need to memorise these. Note where `RokoConfig`
appears as a function parameter today (it does, in `resolve_effective_model`,
so the borrow plumbing is mostly already in shape).

---

## Code-level plan

### Step 1 — Introduce a `ConfigBundle` helper

Add a small struct in `crates/roko-cli/src/config.rs`:

```rust
use std::sync::Arc;

/// A fully-loaded, environment-merged config bundle ready to pass through
/// the dispatch chain. Constructed once per CLI invocation.
#[derive(Clone)]
pub struct ConfigBundle {
    /// Layered legacy `Config` (the per-`[gate]`/`[prompt]` shim).
    pub legacy: Arc<Config>,
    /// Process-merged `RokoConfig` (the `[providers]`/`[models]` shim).
    pub roko: Arc<roko_core::config::RokoConfig>,
    /// Workdir the bundle was loaded from.
    pub workdir: PathBuf,
}

impl ConfigBundle {
    /// Load both the legacy `Config` and the modern `RokoConfig` from
    /// `workdir`, apply env overrides and global provider merges, and
    /// return them as a clonable bundle.
    pub fn load(workdir: &Path) -> anyhow::Result<Self> {
        let legacy = crate::config::load_layered(workdir)
            .map(|resolved| resolved.config)
            .unwrap_or_default();

        let mut roko = roko_core::config::load_config(workdir).unwrap_or_default();
        roko.apply_process_env();
        crate::config::merge_global_providers(&mut roko);
        roko.providers.extend(legacy.providers.clone());
        roko.models.extend(legacy.models.clone());

        Ok(Self {
            legacy: Arc::new(legacy),
            roko: Arc::new(roko),
            workdir: workdir.to_path_buf(),
        })
    }
}
```

> **Why a bundle?** Roko has *two* config types in flight: the legacy
> `crates/roko-cli/src/config.rs::Config` and the canonical
> `roko_core::config::RokoConfig`. Both need caching. Pretending only
> one exists will leave 50 % of the redundant loads in place.

### Step 2 — Build the bundle once at the CLI entry

`crates/roko-cli/src/main.rs` (or whichever `run`/`Cli::run` is the actual
entrypoint — find it via `rg "fn main"` first). Construct the bundle
**before** dispatching subcommands:

```rust
let bundle = ConfigBundle::load(&opts.workdir)
    .with_context(|| format!("load config from {}", opts.workdir.display()))?;
```

Pass `&bundle` (or `bundle.clone()`) into the subcommand handlers.

### Step 3 — Convert `run_once`, `dispatch_agent`, `append_episode_log`

In `crates/roko-cli/src/run.rs`, change the signatures:

```rust
pub async fn run_once(
    bundle: &ConfigBundle,
    prompt_text: &str,
    strategy: Option<BenchStrategy>,
    external_hub: Option<&StateHub>,
) -> Result<RunReport> { ... }

async fn dispatch_agent(
    bundle: &ConfigBundle,
    prompt: &Engram,
    prompt_text: &str,
    ctx: &Context,
    strategy: Option<BenchStrategy>,
) -> Result<DispatchOutcome> { ... }

async fn append_episode_log(
    bundle: &ConfigBundle,
    prompt: &Engram,
    final_output: &Engram,
    verdicts: &[(String, bool)],
    agent_result: &AgentResult,
) -> Result<()> { ... }
```

Within each function, replace:

```rust
let routing_config = roko_core::config::load_config(workdir)?;
```

with:

```rust
let routing_config = bundle.roko.as_ref(); // already merged + env-applied
```

…and similarly delete the `load_layered` re-loads. `bundle.workdir`
replaces the `&Path` argument those functions used to take.

### Step 4 — Fix `resolved_model`

The hidden re-load lives at `run.rs:2715`:

```rust
if let Ok(mut rc) = roko_core::config::load_config(std::path::Path::new(".")) {
    rc.apply_process_env();
    crate::config::merge_global_providers(&mut rc);
    if !rc.agent.default_model.is_empty() { return rc.agent.default_model; }
}
```

Pull the `default_model` from `bundle.roko.agent.default_model` instead.
**Do not** silently take `Path::new(".")` — that hides a config-fallback
behaviour that worked only because the CWD usually was the workspace
root. Make the dependency explicit by accepting `&ConfigBundle`.

### Step 5 — Update tests

Run the failing tests under `crates/roko-cli/src/run.rs` (after Step 3
the helpers will not compile until you propagate `bundle`). Use
`tempfile::tempdir()` to construct a workdir, write a minimal
`roko.toml`, build a `ConfigBundle`, and pass it. Mirror the existing
test scaffolding in `dispatch_agent_uses_exec_agent_for_plain_commands_without_routing`.

Add a new test asserting single-load:

```rust
#[tokio::test]
async fn run_once_loads_config_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_roko_toml(dir.path());
    let counter = install_load_counter();   // wraps fs::read_to_string
    let bundle = ConfigBundle::load(dir.path()).unwrap();
    let _ = run_once(&bundle, "echo hi", None, None).await;
    assert_eq!(counter.load(Ordering::Relaxed), 1, "config loaded > 1 time");
}
```

> If wrapping `fs::read_to_string` is too invasive, fall back to a
> log-capture assertion via `tracing-test` looking for the `loading
> config from` info line.

---

## Step-by-step execution

1. `git checkout -b perf/01-shared-config-cache`.
2. Add `ConfigBundle` to `crates/roko-cli/src/config.rs`.
3. `cargo build -p roko-cli` — should still compile (only adding a
   helper).
4. Wire it into `main.rs` → subcommand dispatch.
5. Refactor `run.rs` (Steps 3–4). Each iteration: change one call site,
   `cargo check -p roko-cli`, fix the compile chain. Do **not** change
   behaviour beyond eliminating reloads.
6. Add the single-load test (Step 5).
7. `cargo test -p roko-cli --release`.
8. Macro-benchmark before/after (`BENCHMARK-RESULTS.md` §11.1). Record in
   the PR description.
9. Open PR titled `perf(cli): cache config bundle for the entire run
   (B02)`.

---

## Anti-patterns / things NOT to do

- **Do NOT introduce a process-wide `static LazyLock<RokoConfig>`** for
  the workdir's config. `roko serve` runs one process for many workdirs;
  a static would leak the first-seen config to all others.
  `ConfigBundle` is per-invocation by design.
- **Do NOT change the layering / merge precedence** while you are at it.
  The current order (defaults → toml → env → CLI override) is load-bearing
  for users with global provider configs. Pull that into a follow-up if
  you find bugs.
- **Do NOT widen the bundle into a god-object.** It carries config only.
  No `LearningRuntime`, no `FileSubstrate`, no model caller. Plan 02
  threads `LearningRuntime` separately for that exact reason.
- **Do NOT silently swap the `Path::new(".")` fallback** for an
  `unwrap_or` of an empty config — that changes behaviour for users who
  rely on the CWD-discovery side-effect. If the bundle does not have a
  default model, surface a clear error.
- **Do NOT forget the `roko-orchestrator::ServiceFactory` path.** It also
  calls `load_config`. After the CLI bundle exists, audit
  `crates/roko-orchestrator/src/service_factory.rs` and pass the same
  config through (or document why it cannot, e.g., deferred until
  serve/HTTP path is also bundle-aware).

---

## Test plan

| Level | Test | Location |
|---|---|---|
| Unit | `ConfigBundle::load` produces same merged values as legacy chain | `crates/roko-cli/src/config.rs` |
| Unit | `run_once` calls config loader exactly once | `crates/roko-cli/src/run.rs` (new) |
| Unit | `dispatch_agent` honors `--model` override after caching | `crates/roko-cli/src/run.rs` (existing test, run unchanged) |
| Integration | `roko run --model gpt-4.1-nano "hi"` resolves to OpenAI provider | `tests/cli_smoke.rs` if exists, otherwise add |
| Macro-bench | Wall-clock improvement ≥20 ms vs baseline | manual `/usr/bin/time -l` |

---

## Rollback plan

- The change is backward-compatible: subcommand handlers that still take
  a `&Path` remain callable; the bundle is constructed for them too.
- To revert: `git revert <commit>` — every modified file's pre-image is
  in the legacy "load on demand" pattern that has been deployed for over
  a year, so the revert is mechanical.
- If `roko serve` regresses (it should not, since serve has its own
  `ConfigBundle::load` per request), an emergency mitigation is to call
  `ConfigBundle::load` inside the affected handler and pass the bundle
  in — same shape, no behaviour change.

---

## Status check (acceptance)

- [ ] All call sites of `roko_core::config::load_config` and
      `crate::config::load_layered` outside `ConfigBundle::load` are
      removed in the affected crates (`roko-cli`).
- [ ] `cargo test -p roko-cli` is green.
- [ ] `cargo clippy -p roko-cli --release -- -D warnings` is green.
- [ ] Macro-benchmark improvement recorded in PR description.
- [ ] PR description links back to this plan and to `BOTTLENECK-ANALYSIS.md` §B02.
