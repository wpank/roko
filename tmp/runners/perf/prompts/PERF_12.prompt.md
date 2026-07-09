# PERF_12: Express gate mode + auto-detect (B08-a)

## Task

Add a `GateMode` enum (`Full`, `Express`, `None`, `Auto`) to
`WorkflowConfig`. Add a `--gates {full|express|none|auto}` CLI flag.
Filter `GateService::run_gates` based on the mode. Implement
`detect_gate_mode(workdir)` for auto-detection from the git diff.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_12](../ISSUE-TRACKER.md#perf_12)
- Plan: `tmp/solutions/perf/implementation/10-express-gate-mode.md`
- Bottleneck: B08 first leg (BOTTLENECK-ANALYSIS.md §B08)
- Performance contract: **C-11** (express mode skips compile/test)
- Priority: P1
- Effort: ≈4 h
- Depends on: none
- Wave: 1

## Problem

Today, every workflow run executes the full configured gate pipeline,
even when the modified files are documentation only. Compile alone
takes 500-2000 ms for the roko workspace. For docs-only runs that's
pure waste.

The fix: a user-facing mode that runs only the cheap gates
(`diff`, `fmt`, `format-check`), and an `Auto` mode that picks based on
file extensions in the git diff.

## Exact Changes

### Step 1 — Add `GateMode` to `pipeline_state.rs`

`crates/roko-runtime/src/pipeline_state.rs`. Add near the existing
`WorkflowConfig` definition:

```rust
/// User-facing gate execution mode. Composes orthogonally with
/// adaptive thresholds (`AdaptiveThresholds`) and the source-hash
/// skip introduced in PERF_13.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GateMode {
    /// Run all configured gates. Default.
    #[default]
    Full,
    /// Run only cheap gates (`diff`, `fmt`, `format-check`). Skip
    /// compile, clippy, test, integration, judge.
    Express,
    /// Skip every gate. Debug aid; never appropriate for CI.
    None,
    /// Inspect git diff and pick `Full` / `Express` / `None`
    /// automatically based on file extensions.
    Auto,
}
```

Extend `WorkflowConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowConfig {
    pub has_strategy: bool,
    pub has_review: bool,
    pub max_iterations: u32,
    pub max_autofix_attempts: u32,
    /// Gate execution mode. Default: `Full`. Express workflow preset
    /// upgrades this to `Express`; standard / full presets keep `Full`.
    pub gate_mode: GateMode,
}
```

Update the three preset constructors:

```rust
impl WorkflowConfig {
    pub fn express() -> Self {
        Self {
            has_strategy: false,
            has_review: false,
            max_iterations: 1,
            max_autofix_attempts: 1,
            gate_mode: GateMode::Express,    // ← new
        }
    }

    pub fn standard() -> Self {
        Self {
            has_strategy: false,
            has_review: true,
            max_iterations: 2,
            max_autofix_attempts: 2,
            gate_mode: GateMode::Full,       // ← new
        }
    }

    pub fn full() -> Self {
        Self {
            has_strategy: true,
            has_review: true,
            max_iterations: 3,
            max_autofix_attempts: 2,
            gate_mode: GateMode::Full,       // ← new
        }
    }
}
```

Also extend `WorkflowConfigToml` so the field round-trips through
`from_toml_str`:

```rust
struct WorkflowConfigToml {
    template: Option<String>,
    has_strategy: Option<bool>,
    has_review: Option<bool>,
    max_iterations: Option<u32>,
    max_autofix_attempts: Option<u32>,
    gate_mode: Option<GateMode>,             // ← new
}
```

…and apply it in the parse:

```rust
if let Some(gate_mode) = raw.gate_mode {
    config.gate_mode = gate_mode;
}
```

### Step 2 — Add `gate_mode` to `GateConfig`

`crates/roko-core/src/foundation.rs`. Find `pub struct GateConfig`
(search `rg -n 'pub struct GateConfig' crates/roko-core/`). Add the
field:

```rust
#[derive(Debug, Clone, Default)]
pub struct GateConfig {
    pub workdir: PathBuf,
    // ... existing fields ...
    pub gate_mode: GateMode,
}
```

> If `GateMode` cannot be defined here (e.g., circular dep with
> `roko-runtime`), define it in `roko-core` and re-export from
> `roko-runtime::pipeline_state`. Pick the cleaner location.

### Step 3 — Define `EXPRESS_GATE_NAMES`

`crates/roko-gate/src/gate_service.rs`, near the top:

```rust
/// Gates that run in express mode. Always cheap, always informational.
/// Stays in sync with the rung mapping inside `ordered_gate_names`.
pub const EXPRESS_GATE_NAMES: &[&str] = &[
    "diff",
    "fmt",
    "format-check",
];
```

### Step 4 — Filter gates in `run_gates`

`crates/roko-gate/src/gate_service.rs::run_gates` (≈line 234).

Today's loop iterates over `Self::ordered_gate_names(&config)`.
Restructure the head of the function:

```rust
async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
    use roko_runtime::pipeline_state::GateMode;

    let payload = GatePayload::in_dir(config.workdir.clone());
    let signal = Engram::builder(Kind::Task).body(Body::from_json(&payload)?).build();
    let ctx = Context::now().with_attr("workdir", config.workdir.to_string_lossy());

    // Resolve effective mode (Auto → concrete mode based on diff).
    let mode = match config.gate_mode {
        GateMode::Auto => detect_gate_mode(&config.workdir),
        m => m,
    };

    let candidate_names = Self::ordered_gate_names(&config);
    let chosen_names: Vec<String> = match mode {
        GateMode::Full => candidate_names.clone(),
        GateMode::None => Vec::new(),
        GateMode::Express => candidate_names
            .iter()
            .filter(|n| EXPRESS_GATE_NAMES.contains(&n.as_str()))
            .cloned()
            .collect(),
        GateMode::Auto => unreachable!("auto resolved above"),
    };

    let mut verdicts: Vec<GateVerdict> = Vec::new();
    let mut shell_gates = config.shell_gates.iter();

    // Emit a skipped verdict for every gate filtered out by mode.
    for name in &candidate_names {
        if !chosen_names.iter().any(|c| c == name) {
            verdicts.push(skipped_gate_verdict(
                name.clone(),
                "Skipped by gate mode",
                format!("gate_mode={mode:?}"),
            ));
        }
    }

    for gate_name in chosen_names {
        // ... existing per-gate dispatch logic from the old loop body ...
    }

    Ok(GateReport { verdicts, /* other fields unchanged */ })
}
```

Move the existing per-gate body (the rung dispatch, the `judge`
intercept, the `custom`/shell-gate handling) into the new
`for gate_name in chosen_names` loop. **Do not** change the per-gate
dispatch semantics — only the outer loop structure changes.

### Step 5 — Implement `detect_gate_mode`

Same file, near the bottom (or in a new sibling helper module if you
prefer):

```rust
/// Auto-detect the appropriate gate mode from the workdir's git diff.
///
/// - Any code-extension change → `Full`.
/// - Only docs/config change → `Express`.
/// - No diff (clean tree) → `None`.
/// - `git` failure → conservatively returns `Full`.
pub fn detect_gate_mode(workdir: &std::path::Path) -> GateMode {
    use roko_runtime::pipeline_state::GateMode;

    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(workdir)
        .output();

    let Ok(out) = output else { return GateMode::Full; };
    if !out.status.success() { return GateMode::Full; }

    let names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines().map(str::to_string).collect();
    if names.is_empty() { return GateMode::None; }

    let has_code = names.iter().any(|f| {
        f.ends_with(".rs") || f.ends_with(".ts") || f.ends_with(".tsx")
        || f.ends_with(".js") || f.ends_with(".jsx") || f.ends_with(".py")
        || f.ends_with(".go") || f.ends_with(".java") || f.ends_with(".kt")
        || f.ends_with(".swift") || f.ends_with(".cpp") || f.ends_with(".c")
        || f.ends_with(".h") || f.ends_with(".hpp")
    });
    let has_docs_or_config = names.iter().any(|f| {
        f.ends_with(".md") || f.ends_with(".txt") || f.ends_with(".toml")
        || f.ends_with(".yaml") || f.ends_with(".yml") || f.ends_with(".json")
    });

    match (has_code, has_docs_or_config) {
        (true, _) => GateMode::Full,
        (false, true) => GateMode::Express,
        (false, false) => GateMode::None,
    }
}
```

### Step 6 — CLI flag

`crates/roko-cli/src/main.rs`. Find the global args struct (the one
clap reads `#[clap(long, global = true)]` from). Add:

```rust
use roko_runtime::pipeline_state::GateMode;

/// Gate execution mode.
///
/// - `full`: run all configured gates (default).
/// - `express`: run only cheap gates (diff, fmt). Use for docs/config edits.
/// - `none`: skip every gate. Debug aid; never appropriate for CI.
/// - `auto`: pick `full` / `express` / `none` from `git diff` extensions.
#[clap(long, value_enum, global = true)]
pub gates: Option<CliGates>,

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum CliGates { Full, Express, None, Auto }

impl From<CliGates> for GateMode {
    fn from(g: CliGates) -> Self {
        match g {
            CliGates::Full => GateMode::Full,
            CliGates::Express => GateMode::Express,
            CliGates::None => GateMode::None,
            CliGates::Auto => GateMode::Auto,
        }
    }
}
```

In `crates/roko-cli/src/run.rs`, after the workflow template default
is applied to `WorkflowConfig`, override:

```rust
if let Some(cli_gates) = cli.gates {
    workflow.gate_mode = cli_gates.into();
}
```

### Step 7 — Workflow engine wiring

`crates/roko-runtime/src/workflow_engine.rs`. Wherever the engine
constructs the `GateConfig` it passes to `gate_runner.run_gates(...)`
(grep for `GateConfig {` in workflow_engine.rs), set the field:

```rust
let gate_config = GateConfig {
    workdir: ctx.workdir.clone(),
    // ... existing fields ...
    gate_mode: self.workflow_config.gate_mode,
};
```

### Step 8 — Tests

`crates/roko-gate/src/gate_service.rs`:

```rust
#[cfg(test)]
mod gate_mode_tests {
    use super::*;
    use roko_runtime::pipeline_state::GateMode;

    fn cfg(workdir: &Path, mode: GateMode) -> GateConfig {
        GateConfig {
            workdir: workdir.to_path_buf(),
            gate_mode: mode,
            // ... fill required fields with sensible defaults ...
            enabled_gates: vec!["compile".into(), "fmt".into(), "test".into()],
            shell_gates: vec![],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn express_mode_skips_compile_and_test() {
        let dir = tempfile::tempdir().unwrap();
        let svc = GateService::new();
        let report = svc.run_gates(cfg(dir.path(), GateMode::Express)).await.unwrap();
        let skipped: Vec<_> = report.verdicts.iter()
            .filter(|v| v.skipped)
            .map(|v| v.gate_name.clone())
            .collect();
        assert!(skipped.contains(&"compile".to_string()));
        assert!(skipped.contains(&"test".to_string()));
        for v in &report.verdicts {
            if v.skipped {
                assert!(v.skip_reason.as_deref().unwrap_or("").starts_with("gate_mode="),
                    "skip_reason should be gate_mode=...; got {:?}", v.skip_reason);
            }
        }
    }

    #[tokio::test]
    async fn none_mode_skips_everything() {
        let dir = tempfile::tempdir().unwrap();
        let svc = GateService::new();
        let report = svc.run_gates(cfg(dir.path(), GateMode::None)).await.unwrap();
        assert!(report.verdicts.iter().all(|v| v.skipped));
    }

    fn git_init_with_changes(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let _ = std::process::Command::new("git").args(["init", "-q"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["config", "user.email", "t@t"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["config", "user.name", "t"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["commit", "--allow-empty", "-q", "-m", "init"]).current_dir(dir.path()).output();
        for (path, content) in files {
            let p = dir.path().join(path);
            if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).unwrap(); }
            std::fs::write(p, content).unwrap();
        }
        dir
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
        let dir = git_init_with_changes(&[]);
        assert_eq!(detect_gate_mode(dir.path()), GateMode::None);
    }
}
```

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-core/src/foundation.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

## Read-Only Context

- `crates/roko-gate/src/adaptive_threshold.rs` (orthogonal skip mechanism)
- `tmp/solutions/perf/implementation/10-express-gate-mode.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-GATE-1, AP-GATE-2)

## Acceptance Criteria

- [ ] `GateMode` enum (`Full`, `Express`, `None`, `Auto`) added to `pipeline_state.rs` with serde + `Default = Full`.
- [ ] `GateConfig` (in `roko_core::foundation`) carries `gate_mode: GateMode`.
- [ ] `WorkflowConfig.gate_mode` exists; presets default appropriately (`express()` → Express, others → Full).
- [ ] `EXPRESS_GATE_NAMES = ["diff", "fmt", "format-check"]` constant in `gate_service.rs`.
- [ ] `GateService::run_gates` filters by mode; emits `skipped: true` verdicts for filtered gates with `skip_reason = "gate_mode=..."`.
- [ ] `detect_gate_mode(workdir) -> GateMode` exists and covers `.rs`, `.ts`, `.py`, `.go`, `.md`, `.toml`, `.yaml`, `.json` (plus the rest in Step 5).
- [ ] CLI flag `--gates {full|express|none|auto}` added (clap).
- [ ] Tests `express_mode_skips_compile_and_test`, `none_mode_skips_everything`, `detect_gate_mode_*` pass.
- [ ] Workflow engine forwards `gate_mode` from `WorkflowConfig` into `GateConfig`.

## Verify

```bash
# Audit:
rg -n 'GateMode' crates/ --type rust
# Expected: definitions in pipeline_state.rs (or core::foundation), uses
# in gate_service.rs, workflow_engine.rs, run.rs, main.rs.

# CLI surface:
./target/release/roko run --help | rg -A 4 'gates'
# Expected: shows the four mode options.
```

## Do NOT

- Do NOT skip rung 0 (compile) in any mode that touches code (AP-GATE-1).
  When auto-detect finds a code change, escalate to Full. The match
  arm `(true, _) => Full` enforces this.
- Do NOT bypass the adaptive-threshold mechanism (AP-GATE-2). They
  compose orthogonally; mode filtering happens before adaptive skip is
  consulted, but each adds its own `skipped: true` verdict so audits
  can distinguish them.
- Do NOT remove gates from configuration just because mode skipped them
  once. The skip is per-run, not per-config.
- Do NOT auto-detect without timeout. `std::process::Command` has no
  timeout; if you switch to `tokio::process::Command`, wrap in
  `tokio::time::timeout(2.s, ...)` and treat timeout as Full.
- Do NOT extend `EXPRESS_GATE_NAMES` to "the gates I happen to like
  today". Express is contractually `[diff, fmt, format-check]`. Custom
  sets go via config (`[gates] express_extras = [...]`).
- Do NOT wire `--gates none` as the default in CI scripts. Document in
  CLI help that `none` is a debug aid.
- Do NOT pollute the `GateReport` with skipped-by-mode verdicts on
  output that is then ranked by the LLM judge. The judge expects
  failure verdicts to mean "actual failures"; skipped verdicts must
  carry `skipped: true` (they do — verify your test).
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_12 done <commit-sha>
```
