# 10 — Express Gate Mode (B08)

> Bottleneck: every workflow run executes the full gate pipeline
> (compile → clippy → test → …) even when the task did not modify code
> at all (docs, config, research). Compile alone takes 500–2000 ms.
>
> Target savings: 500–2000 ms / non-code task.
> Effort: ≈4 h. Risk: medium (skipping gates reduces safety guarantees).

---

## Goal & success criteria

After this change:

1. A `GateMode` enum exists at the workflow-config layer:
   `Full`, `Express`, `None`, `Auto`.
2. The CLI accepts `--gates {full|express|none|auto}` and threads the
   choice into `WorkflowConfig`.
3. Express mode runs only the cheap, always-fast gates: `diff`, `fmt`,
   `format-check`. It explicitly skips `compile`, `clippy`, `test`,
   `judge`.
4. Auto mode inspects `git diff --name-only HEAD` and picks the right
   mode from the modified file extensions.
5. Skipped gates emit a `GateVerdict { skipped: true, skip_reason: ... }`
   so reviewers can audit what didn't run.

Done when:

- A new test runs the standard workflow with `--gates express` and
  asserts the compile / test gates were skipped.
- Auto-mode tests cover `.md`-only, `.toml`-only, `.rs`-touched, and
  mixed cases.
- Macro-benchmark on a docs-only edit shows ≥800 ms improvement vs the
  default-gate baseline.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B08,
  `OPTIMIZATION-PLAYBOOK.md` §9.
- Existing infra:
  - `crates/roko-gate/src/gate_service.rs::ordered_gate_names` — already
    knows the rung-to-name mapping; we layer the mode filter on top.
  - `crates/roko-gate/src/adaptive_threshold.rs` — already supports
    skipping rungs 1+ based on consecutive-pass streaks. Express mode
    is a complementary, user-controlled override, **not** a replacement.
  - `crates/roko-runtime/src/pipeline_state.rs::WorkflowConfig` — the
    config struct to extend.
  - The CLI flag wiring lives in `crates/roko-cli/src/main.rs` (clap
    args) and `crates/roko-cli/src/run.rs` (translation to
    `WorkflowConfig`).

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-runtime/src/pipeline_state.rs` (~120 LOC of `WorkflowConfig`) | Where `GateMode` will live. |
| `crates/roko-cli/src/main.rs` | CLI args. |
| `crates/roko-cli/src/run.rs::workflow_config_for_template` | How template names map to configs. |
| `crates/roko-gate/src/gate_service.rs` | `GateService::ordered_gate_names`, `should_skip_rung_adaptively`. |
| `crates/roko-gate/src/adaptive_threshold.rs` | The other skip mechanism (read for orthogonality). |

---

## Code-level plan

### Step 1 — Add `GateMode` to `WorkflowConfig`

```rust
// crates/roko-runtime/src/pipeline_state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GateMode {
    /// Run all configured gates.
    #[default]
    Full,
    /// Run only the cheap, fast gates (diff, fmt, format-check). Skip
    /// compile, clippy, test, integration, judge.
    Express,
    /// Skip all gates entirely.
    None,
    /// Inspect git diff and pick Full / Express / None automatically.
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowConfig {
    pub has_strategy: bool,
    pub has_review: bool,
    pub max_iterations: u32,
    pub max_autofix_attempts: u32,
    /// NEW: gate execution mode override.
    pub gate_mode: GateMode,
}
```

Update the three preset constructors (`express`, `standard`, `full`) to
set `gate_mode` to whatever default makes sense:

- `express()` workflow: `gate_mode = GateMode::Express` (matches the
  workflow's "fast path" intent).
- `standard()` and `full()`: `gate_mode = GateMode::Full`.

### Step 2 — Define the express gate set

Add a constant in `crates/roko-gate/src/gate_service.rs`:

```rust
/// Gates that run in express mode. Always cheap, always informational.
pub const EXPRESS_GATE_NAMES: &[&str] = &[
    "diff",          // git diff statistics
    "fmt",           // cargo fmt --check (or other lang equivalent)
    "format-check",  // generic format-check gate
];
```

(These are the rung-3/4-equivalents from `BENCHMARK-RESULTS.md` §6.)

### Step 3 — Filter gates in `ordered_gate_names`

Modify `GateService::run_gates` to consult the mode:

```rust
async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
    let mode = config.gate_mode;     // NEW field on GateConfig
    let candidate_names = Self::ordered_gate_names(&config);

    let chosen_names = match mode {
        GateMode::Full => candidate_names,
        GateMode::None => Vec::new(),
        GateMode::Express => candidate_names.into_iter()
            .filter(|name| EXPRESS_GATE_NAMES.contains(&name.as_str()))
            .collect(),
        GateMode::Auto => {
            let auto_mode = detect_gate_mode(&config.workdir);
            return self.run_gates(GateConfig { gate_mode: auto_mode, ..config }).await;
        }
    };

    // Emit a skipped verdict for every gate filtered out so the audit
    // trail is preserved.
    let skipped = candidate_names_minus_chosen(&config, &chosen_names);
    let mut verdicts = Vec::new();
    for name in skipped {
        verdicts.push(skipped_gate_verdict(
            name,
            "Skipped by gate mode",
            format!("gate_mode={:?}", mode),
        ));
    }

    // Existing loop, but iterating chosen_names.
    for gate_name in chosen_names { /* run as before */ }

    // Combine and return.
    Ok(GateReport { verdicts, /* ... */ })
}
```

> Add `gate_mode: GateMode` to `GateConfig` in
> `roko_core::foundation` (the trait is in core; the enum can mirror
> the runtime one or live in core directly to avoid duplication).

### Step 4 — Auto-detect mode

```rust
fn detect_gate_mode(workdir: &Path) -> GateMode {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(workdir).output();

    let Ok(out) = output else {
        // No git or diff failed — be conservative.
        return GateMode::Full;
    };
    if !out.status.success() {
        return GateMode::Full;
    }
    let names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines().map(str::to_string).collect();
    if names.is_empty() {
        return GateMode::None;          // nothing changed → nothing to gate
    }
    let has_code = names.iter().any(|f|
        f.ends_with(".rs") || f.ends_with(".ts") || f.ends_with(".tsx")
        || f.ends_with(".js") || f.ends_with(".py") || f.ends_with(".go")
    );
    let has_config_or_docs = names.iter().any(|f|
        f.ends_with(".md") || f.ends_with(".txt") || f.ends_with(".toml")
        || f.ends_with(".yaml") || f.ends_with(".yml") || f.ends_with(".json")
    );
    match (has_code, has_config_or_docs) {
        (true, _) => GateMode::Full,         // any code change → full
        (false, true) => GateMode::Express,  // docs/config only → express
        (false, false) => GateMode::None,    // unknown extensions → none
    }
}
```

> **Anti-pattern.** Do **not** call `git diff` async here. The whole
> `run_gates` is async; spawning `tokio::process::Command` is fine, but
> the auto-detect runs once before any gate, so synchronous + 30 ms is
> simpler. If you switch to async, do not forget `tokio::time::timeout`
> to avoid hanging on misconfigured git.

### Step 5 — CLI flag

In `crates/roko-cli/src/main.rs`:

```rust
/// Gate execution mode. Default: full.
#[clap(long, value_enum, default_value = "full", global = true)]
pub gates: CliGates,

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliGates {
    Full,
    Express,
    None,
    Auto,
}

impl From<CliGates> for GateMode {
    fn from(g: CliGates) -> GateMode {
        match g {
            CliGates::Full => GateMode::Full,
            CliGates::Express => GateMode::Express,
            CliGates::None => GateMode::None,
            CliGates::Auto => GateMode::Auto,
        }
    }
}
```

In `run_once` / `dispatch_agent`, set `workflow.gate_mode` from this
flag *after* the template default is applied.

### Step 6 — Workflow-engine integration

In `crates/roko-runtime/src/workflow_engine.rs`, ensure the
`gate_mode` from `WorkflowConfig` is forwarded to `GateConfig` when the
engine triggers `gate_runner.run_gates(...)`. Today the workflow engine
likely passes a fresh `GateConfig` per run; add the field.

---

## Step-by-step execution

1. `git checkout -b perf/10-express-gate-mode`.
2. Add `GateMode` to `pipeline_state.rs`. `cargo build -p roko-runtime`.
3. Add to `roko-core::foundation::GateConfig`. `cargo build -p roko-core`.
4. Add `EXPRESS_GATE_NAMES` and update `gate_service.rs`. Tests.
5. Add `detect_gate_mode` (Step 4). Tests.
6. CLI flag (Step 5).
7. Workflow engine wiring (Step 6).
8. Macro-benchmark on a docs-only edit.
9. PR `perf(gate): add express + auto gate modes (B08)`.

---

## Anti-patterns / things NOT to do

- **Do NOT skip rung 0 (compile) in any mode that touches code.** Even
  in express mode, when auto-detect finds a code change, escalate to
  Full. Skipping compile silently ships broken code.
- **Do NOT bypass the adaptive-threshold mechanism.** The two are
  orthogonal: mode is user intent, adaptive is learned skip-when-safe.
  Stack them: adaptive can skip a gate that the mode allowed.
- **Do NOT remove a gate from the configuration just because you
  skipped it once.** The verdict's `skipped: true` field exists for a
  reason — auditors need to see which gates were skipped, not pretend
  they weren't configured.
- **Do NOT auto-detect without timeout.** A misconfigured git
  repository (corrupted refs) can hang `git diff` for seconds.
  `std::process::Command` has no timeout; if you want one, switch to
  `tokio::process::Command` and `tokio::time::timeout`.
- **Do NOT extend express mode to "the gates I happen to like today".**
  Express is contractually the always-cheap, always-fast set:
  diff/fmt/format-check. If you want a custom set, expose it via
  config (`[gates] express_extras = ["my-gate"]`), not by editing the
  constant.
- **Do NOT wire `--gates none` into CI default.** Tempting; disastrous.
  Document in CLI help that `none` is a debug aid, not a release
  config.
- **Do NOT pollute the `GateReport` with skipped-by-mode verdicts on
  output that is then ranked by the LLM judge.** The judge expects
  failure verdicts to mean "actual failures". Skipped verdicts should
  carry a clear `skipped: true` flag and be filtered out before
  judging.

---

## Test plan

```rust
#[tokio::test]
async fn express_mode_skips_compile_and_test() {
    let cfg = GateConfig {
        workdir: tempdir().path().into(),
        gate_mode: GateMode::Express,
        // ... full enabled_gates list ...
    };
    let report = GateService::new().run_gates(cfg).await.unwrap();
    let skipped: Vec<_> = report.verdicts.iter()
        .filter(|v| v.skipped).map(|v| v.gate_name.clone()).collect();
    assert!(skipped.contains(&"compile".to_string()));
    assert!(skipped.contains(&"test".to_string()));
}

#[test]
fn detect_gate_mode_md_only_returns_express() {
    let dir = git_init_with_changes(&[("README.md", "hi")]);
    assert_eq!(detect_gate_mode(dir.path()), GateMode::Express);
}

#[test]
fn detect_gate_mode_rust_change_returns_full() {
    let dir = git_init_with_changes(&[("src/main.rs", "fn main() {}")]);
    assert_eq!(detect_gate_mode(dir.path()), GateMode::Full);
}

#[test]
fn detect_gate_mode_no_diff_returns_none() {
    let dir = git_init_clean();
    assert_eq!(detect_gate_mode(dir.path()), GateMode::None);
}
```

Macro-benchmark: run `roko run --gates auto "fix typo in README"`
against a workdir whose only diff is `README.md`. Compare wall time to
the baseline run with `--gates full`. Expect ≥800 ms improvement.

---

## Rollback plan

- `--gates full` (the default) restores the pre-change behaviour. Users
  who want to disable the auto-detect entirely can also unset
  `[conductor.workflow.gate_mode]` in `roko.toml`.
- `git revert` of the wiring commits removes the flag while keeping
  the constant + enum (harmless dead code).

---

## Status check (acceptance)

- [ ] `GateMode` enum + `GateConfig.gate_mode` exist.
- [ ] `--gates {full|express|none|auto}` works end-to-end.
- [ ] `detect_gate_mode` covers .rs/.ts/.py/.go/.md/.toml/.yaml.
- [ ] Skipped-gate verdicts include `gate_mode=...` in `skip_reason`.
- [ ] Tests above pass.
- [ ] Macro-benchmark improvement of ≥800 ms recorded for docs-only
      runs.
