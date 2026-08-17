# W15-E: Generalization -- Data-Driven Gates, Workspace Abstraction, Adaptive Budgets (IMPROVEMENTS 8.1-8.3)

**Priority**: P2 -- extensibility for non-Rust workspaces and varied model contexts
**Effort**: 4-5 hours
**Files to modify**: 3 files (1 existing, 2 new)
**Dependencies**: None

## Problem

Three hardcoded patterns prevent Roko from generalizing beyond its current Rust-specific setup:

1. **Hardcoded gate rungs** -- Gate rungs are defined in a `GatesConfig` struct (in `crates/roko-core/src/config/gates.rs`, lines 12-27) with boolean flags (`clippy_enabled`, `skip_tests`) and a `domain_gates` HashMap. There's no way to define custom gate commands in TOML. Adding a gate (e.g., `npm test` for TypeScript) requires code changes.

2. **Raw `PathBuf` for workspace paths** -- 15+ functions pass `workdir: &Path` with ad-hoc `.join(".roko")`, `.join("state")`, `.join("episodes.jsonl")` constructions. No validation that the directory is a roko workspace. No `Workspace` abstraction exists in `roko-core` (verified: no `workspace.rs` and no `pub mod workspace` in `lib.rs`).

3. **Hardcoded token budgets** -- Prompt section budgets in `common.rs` are fixed constants (e.g., `plan: 50_000`, `workspace_map: 20_000` in `budget_for()` at line 43). These were tuned for 200K context models. On a 128K model, the plan section alone takes 25% of context. On a 1M model, these budgets waste context.

## Exact Code to Change

### File 1: `crates/roko-core/src/config/gates.rs` (288 lines)

#### Change 1: Add data-driven gate rung configuration (8.1)

The existing `GatesConfig` struct at lines 12-27 has boolean flags but no custom command definitions:

```rust
pub struct GatesConfig {
    pub clippy_enabled: bool,
    pub skip_tests: bool,
    pub max_iterations: u32,
    pub domain_gates: HashMap<String, Vec<String>>,
}
```

**Find this code (lines 9-27):**

```rust
// ---- [gates] -------------------------------------------------------------

/// Verify (verification) settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatesConfig {
    /// Enable clippy / lint gate.
    #[serde(default = "default_true")]
    pub clippy_enabled: bool,
    /// Skip test gate entirely.
    #[serde(default)]
    pub skip_tests: bool,
    /// Max gate retry iterations before giving up.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Per-domain gate overrides. Keys are domain labels (e.g. "research", "docs"),
    /// values are shell commands to run as gates (e.g. `["shell:true"]`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub domain_gates: HashMap<String, Vec<String>>,
}
```

**Replace with:**

```rust
// ---- [gates] -------------------------------------------------------------

/// A single gate rung definition -- data-driven replacement for hardcoded gates.
///
/// Gate rungs are defined in `roko.toml` under `[[gates.custom_rungs]]` and can be
/// customized per project (e.g., `npm test` for TypeScript, `go vet` for Go).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRungConfig {
    /// Human-readable name (e.g., "compile", "lint", "test").
    pub name: String,
    /// Shell command to execute.
    pub command: String,
    /// Timeout in seconds for this rung.
    #[serde(default = "default_gate_rung_timeout")]
    pub timeout_secs: u64,
    /// If false, failure is a warning rather than a blocking error.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Names of rungs that can run in parallel with this one.
    #[serde(default)]
    pub parallel_with: Vec<String>,
}

fn default_gate_rung_timeout() -> u64 {
    120
}

impl GateRungConfig {
    /// Timeout as `Duration`.
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs)
    }
}

/// Verify (verification) settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatesConfig {
    /// Enable clippy / lint gate.
    #[serde(default = "default_true")]
    pub clippy_enabled: bool,
    /// Skip test gate entirely.
    #[serde(default)]
    pub skip_tests: bool,
    /// Max gate retry iterations before giving up.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Per-domain gate overrides. Keys are domain labels (e.g. "research", "docs"),
    /// values are shell commands to run as gates (e.g. `["shell:true"]`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub domain_gates: HashMap<String, Vec<String>>,
    /// Custom gate rung definitions. When set, these override the built-in
    /// compile/clippy/test rungs. When empty, built-in defaults are used.
    ///
    /// Example in roko.toml:
    /// ```toml
    /// [[gates.custom_rungs]]
    /// name = "typecheck"
    /// command = "npx tsc --noEmit"
    /// timeout_secs = 60
    /// required = true
    /// parallel_with = []
    /// ```
    #[serde(default)]
    pub custom_rungs: Vec<GateRungConfig>,
}
```

**Then** add the `effective_rungs()` method after the existing `GatesConfig` impl block. Find the `Default` impl (lines 33-41):

**Find this code (lines 33-42):**

```rust
impl Default for GatesConfig {
    fn default() -> Self {
        Self {
            clippy_enabled: default_true(),
            skip_tests: false,
            max_iterations: default_max_iterations(),
            domain_gates: HashMap::new(),
        }
    }
}
```

**Replace with:**

```rust
impl Default for GatesConfig {
    fn default() -> Self {
        Self {
            clippy_enabled: default_true(),
            skip_tests: false,
            max_iterations: default_max_iterations(),
            domain_gates: HashMap::new(),
            custom_rungs: Vec::new(),
        }
    }
}

impl GatesConfig {
    /// Return the effective gate rungs -- custom if defined, otherwise built-in defaults.
    pub fn effective_rungs(&self) -> Vec<GateRungConfig> {
        if !self.custom_rungs.is_empty() {
            return self.custom_rungs.clone();
        }
        // Built-in defaults for Rust workspaces
        vec![
            GateRungConfig {
                name: "compile".into(),
                command: "cargo check --workspace".into(),
                timeout_secs: 120,
                required: true,
                parallel_with: vec![],
            },
            GateRungConfig {
                name: "lint".into(),
                command: "cargo clippy --workspace --no-deps -- -D warnings".into(),
                timeout_secs: 60,
                required: true,
                parallel_with: vec!["compile".into()],
            },
            GateRungConfig {
                name: "test".into(),
                command: "cargo test --workspace".into(),
                timeout_secs: 300,
                required: true,
                parallel_with: vec![],
            },
        ]
    }
}
```

**IMPORTANT -- Construction site that must also be updated:**

`crates/roko-core/src/config/compat.rs` line 185 constructs `GatesConfig` with explicit fields (no `..Default`). You MUST add `custom_rungs: Vec::new(),` to this construction:

**Find this code (line 185-190 of compat.rs):**

```rust
    GatesConfig {
        clippy_enabled: m.clippy_enabled.unwrap_or(d.clippy_enabled),
        skip_tests: m.skip_tests.unwrap_or(d.skip_tests),
        max_iterations: m.max_iterations.unwrap_or(d.max_iterations),
        domain_gates: HashMap::new(),
    }
```

**Replace with:**

```rust
    GatesConfig {
        clippy_enabled: m.clippy_enabled.unwrap_or(d.clippy_enabled),
        skip_tests: m.skip_tests.unwrap_or(d.skip_tests),
        max_iterations: m.max_iterations.unwrap_or(d.max_iterations),
        domain_gates: HashMap::new(),
        custom_rungs: Vec::new(),
    }
```

Other construction sites (`presets.rs` lines 70, 134 and `orchestrate.rs` line 17655) use `..GatesConfig::default()` or `GatesConfig::default()` which will pick up the new field automatically.

**Then** add `GateRungConfig` to the re-exports in `crates/roko-core/src/config/mod.rs`:

**Find this code (line 54):**

```rust
    GatesConfig, GeminiConfig, GhostTurnConfig, GithubWebhookConfig, IterationLoopConfig,
```

**Replace with:**

```rust
    GateRungConfig, GatesConfig, GeminiConfig, GhostTurnConfig, GithubWebhookConfig, IterationLoopConfig,
```

Also add to `schema.rs` re-exports:

**Find this code (line 24 of schema.rs):**

```rust
pub use super::gates::*;
```

This wildcard re-export already covers `GateRungConfig`, so no change needed in schema.rs.

---

### File 2: `crates/roko-core/src/workspace.rs` (NEW FILE)

#### Change 2: Create workspace abstraction (8.2)

No `workspace.rs` exists in `roko-core`. No `pub mod workspace` in `lib.rs`.

**Create** `crates/roko-core/src/workspace.rs`:

```rust
//! Workspace path abstraction.
//!
//! Replaces scattered `workdir.join(".roko").join("state")` patterns with
//! typed path accessors. Validates that the directory is a roko workspace.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// A validated roko workspace.
///
/// Provides typed path accessors for all well-known workspace locations.
/// Created via `open()` (for existing workspaces) or `create()` (for new ones).
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    roko_dir: PathBuf,
}

impl Workspace {
    /// Open an existing roko workspace.
    ///
    /// Fails if the directory doesn't have a `.roko/` subdirectory.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .context("failed to canonicalize workspace root")?;
        let roko_dir = root.join(".roko");
        if !roko_dir.exists() {
            bail!(
                "not a roko workspace: {} (no .roko/ directory)",
                root.display()
            );
        }
        Ok(Self { root, roko_dir })
    }

    /// Create a new roko workspace, initializing the standard directory layout.
    pub fn create(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let roko_dir = root.join(".roko");
        std::fs::create_dir_all(&roko_dir)
            .context("failed to create .roko directory")?;
        std::fs::create_dir_all(roko_dir.join("state"))
            .context("failed to create .roko/state")?;
        std::fs::create_dir_all(roko_dir.join("runtime"))
            .context("failed to create .roko/runtime")?;
        std::fs::create_dir_all(roko_dir.join("plans"))
            .context("failed to create .roko/plans")?;
        std::fs::create_dir_all(roko_dir.join("learn"))
            .context("failed to create .roko/learn")?;
        Ok(Self { root, roko_dir })
    }

    /// Open if `.roko/` exists, otherwise create a minimal workspace.
    pub fn open_or_create(root: impl AsRef<Path>) -> Result<Self> {
        let root_path = root.as_ref();
        if root_path.join(".roko").exists() {
            Self::open(root_path)
        } else {
            Self::create(root_path)
        }
    }

    /// Workspace root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `.roko/` directory.
    #[must_use]
    pub fn roko_dir(&self) -> &Path {
        &self.roko_dir
    }

    /// State directory (`.roko/state/`).
    #[must_use]
    pub fn state_dir(&self) -> PathBuf {
        self.roko_dir.join("state")
    }

    /// Plans directory (`.roko/plans/`).
    #[must_use]
    pub fn plans_dir(&self) -> PathBuf {
        self.roko_dir.join("plans")
    }

    /// Runtime directory (`.roko/runtime/`).
    #[must_use]
    pub fn runtime_dir(&self) -> PathBuf {
        self.roko_dir.join("runtime")
    }

    /// Learning data directory (`.roko/learn/`).
    #[must_use]
    pub fn learn_dir(&self) -> PathBuf {
        self.roko_dir.join("learn")
    }

    /// Episode log path (`.roko/episodes.jsonl`).
    #[must_use]
    pub fn episodes_path(&self) -> PathBuf {
        self.roko_dir.join("episodes.jsonl")
    }

    /// Signal log path (`.roko/signals.jsonl`).
    #[must_use]
    pub fn signals_path(&self) -> PathBuf {
        self.roko_dir.join("signals.jsonl")
    }

    /// Roko log path (`.roko/roko.log`).
    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.roko_dir.join("roko.log")
    }

    /// Config file path (`roko.toml` at workspace root).
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.root.join("roko.toml")
    }

    /// PRD storage directory (`.roko/prd/`).
    #[must_use]
    pub fn prd_dir(&self) -> PathBuf {
        self.roko_dir.join("prd")
    }

    /// Research artifacts directory (`.roko/research/`).
    #[must_use]
    pub fn research_dir(&self) -> PathBuf {
        self.roko_dir.join("research")
    }

    /// Executor snapshot path (`.roko/state/executor.json`).
    #[must_use]
    pub fn executor_snapshot_path(&self) -> PathBuf {
        self.state_dir().join("executor.json")
    }

    /// Gate thresholds path (`.roko/learn/gate-thresholds.json`).
    #[must_use]
    pub fn gate_thresholds_path(&self) -> PathBuf {
        self.learn_dir().join("gate-thresholds.json")
    }

    /// Cascade router state path (`.roko/learn/cascade-router.json`).
    #[must_use]
    pub fn cascade_router_path(&self) -> PathBuf {
        self.learn_dir().join("cascade-router.json")
    }

    /// Efficiency log path (`.roko/learn/efficiency.jsonl`).
    #[must_use]
    pub fn efficiency_log_path(&self) -> PathBuf {
        self.learn_dir().join("efficiency.jsonl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_open_workspace() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = Workspace::create(tmp.path()).expect("create");
        assert!(ws.roko_dir().exists());
        assert!(ws.state_dir().exists());
        assert!(ws.plans_dir().exists());
        assert!(ws.learn_dir().exists());

        // Should be able to open the same workspace
        let ws2 = Workspace::open(tmp.path()).expect("open");
        assert_eq!(ws.root(), ws2.root());
    }

    #[test]
    fn open_nonexistent_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = Workspace::open(tmp.path().join("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn open_or_create_creates_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = Workspace::open_or_create(tmp.path()).expect("open_or_create");
        assert!(ws.roko_dir().exists());
    }

    #[test]
    fn path_accessors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = Workspace::create(tmp.path()).expect("create");
        assert!(ws.episodes_path().to_str().unwrap().contains("episodes.jsonl"));
        assert!(ws.signals_path().to_str().unwrap().contains("signals.jsonl"));
        assert!(ws.config_path().to_str().unwrap().contains("roko.toml"));
        assert!(ws.executor_snapshot_path().to_str().unwrap().contains("executor.json"));
    }
}
```

**Then** register the module in `crates/roko-core/src/lib.rs`. Find the module declarations section (around line 158):

**Find this code (lines 158-159):**

```rust
pub mod usage;
pub mod verdict;
```

**Replace with:**

```rust
pub mod usage;
pub mod verdict;
/// Workspace path abstraction -- typed path accessors for `.roko/` directory layout.
pub mod workspace;
```

**Then** add to the re-exports section. Find the last line of re-exports (around line 306):

**Find this code (line 306):**

```rust
pub use verdict::{Outcome, Selection, TestCount, Verdict};
```

**Replace with:**

```rust
pub use verdict::{Outcome, Selection, TestCount, Verdict};
pub use workspace::Workspace;
```

**Dependencies**: `anyhow` and `tempfile` (for tests) must be in `roko-core/Cargo.toml`.

**IMPORTANT**: `anyhow` is NOT currently a dependency of `roko-core` (it uses `thiserror` for typed errors). You MUST add it before creating `workspace.rs`. `tempfile` IS already in dev-dependencies.

Add `anyhow` to `crates/roko-core/Cargo.toml` under `[dependencies]`:

```toml
anyhow = { workspace = true }
```

Verify with:
```bash
grep -q 'anyhow' crates/roko-core/Cargo.toml && echo "anyhow ok" || echo "MUST add anyhow"
grep -q 'tempfile' crates/roko-core/Cargo.toml && echo "tempfile ok" || echo "add tempfile to dev-dependencies"
```

---

### File 3: `crates/roko-compose/src/templates/common.rs` (346 lines)

#### Change 3: Add adaptive budget computation (8.3)

The `PromptBudget` struct is at lines 16-36. The `budget_for()` function is at line 43. Add adaptive budget computation after the existing code.

**Find this code (end of `budget_for` function, lines 122-123):**

```rust
    }
}
```

(This is the closing brace of the `match` block and the `budget_for` function, right before the `// --- Reusable stanza constants ---` comment at line 125.)

**Add AFTER line 123 (before the stanza constants section):**

```rust

/// A relative budget that adapts to the model's context window.
///
/// Instead of hardcoding `plan: 50_000`, use `AdaptiveBudget { fraction: 0.25, min: 5_000, max: 50_000 }`.
/// On a 200K model: 50,000 chars (25%). On a 128K model: 32,000 chars (25%). On a 1M model: capped at 50,000.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveBudget {
    /// Fraction of total context window to allocate (0.0 - 1.0).
    pub fraction: f32,
    /// Minimum character budget (floor).
    pub min_chars: usize,
    /// Maximum character budget (ceiling).
    pub max_chars: usize,
}

impl AdaptiveBudget {
    /// Create a new adaptive budget.
    #[must_use]
    pub const fn new(fraction: f32, min_chars: usize, max_chars: usize) -> Self {
        Self {
            fraction,
            min_chars,
            max_chars,
        }
    }

    /// Compute the effective character budget for a given context window size.
    ///
    /// `total_context_chars` is the model's context window in characters
    /// (approximately 4 chars per token for English text).
    #[must_use]
    pub fn compute(&self, total_context_chars: usize) -> usize {
        let target = (total_context_chars as f32 * self.fraction) as usize;
        target.clamp(self.min_chars, self.max_chars)
    }
}

/// Compute an adaptive budget scaled to the model's context window.
///
/// Uses the hardcoded defaults as maximums but scales down proportionally
/// for smaller models. This is the "smart" alternative to `budget_for()`.
///
/// `model_context_tokens` is the model's token limit (e.g., 200_000).
/// Character budget is approximated as `tokens * 4`.
#[must_use]
pub fn adaptive_budget_for(role: AgentRole, model_context_tokens: usize) -> PromptBudget {
    let total_chars = model_context_tokens * 4; // rough chars-per-token
    let base = budget_for(role);

    // Each section gets a fraction of the context, clamped to the existing
    // hardcoded value as a maximum. This preserves current behavior for
    // 200K+ models while scaling down for smaller contexts.
    PromptBudget {
        plan: AdaptiveBudget::new(0.25, 5_000, base.plan).compute(total_chars),
        workspace_map: AdaptiveBudget::new(0.10, 2_000, base.workspace_map).compute(total_chars),
        prd2: AdaptiveBudget::new(0.06, 2_000, base.prd2).compute(total_chars),
        context: AdaptiveBudget::new(0.02, 1_000, base.context).compute(total_chars),
        brief: AdaptiveBudget::new(0.04, 2_000, base.brief).compute(total_chars),
        reviews: AdaptiveBudget::new(0.015, 1_000, base.reviews).compute(total_chars),
        instructions: AdaptiveBudget::new(0.02, 1_000, base.instructions).compute(total_chars),
        file_context: AdaptiveBudget::new(0.04, 2_000, base.file_context).compute(total_chars),
        skills: AdaptiveBudget::new(0.04, 2_000, base.skills).compute(total_chars),
    }
}
```

**Then** add tests. Find the end of the existing `#[cfg(test)]` block (line 345-346):

**Find this code (lines 345-346):**

```rust
    }
}
```

(The closing brace of `format_plan_list_multiple` test and the `mod tests` block.)

**Replace with:**

```rust
    }
}

#[cfg(test)]
mod adaptive_tests {
    use super::*;

    #[test]
    fn adaptive_budget_scales_down_for_small_model() {
        let budget = adaptive_budget_for(AgentRole::Implementer, 128_000);
        // 128K tokens * 4 = 512K chars. Plan = 25% = 128K, but capped at 50K
        assert_eq!(budget.plan, 50_000);
        // workspace_map = 10% = 51.2K, but capped at 20K
        assert_eq!(budget.workspace_map, 20_000);
    }

    #[test]
    fn adaptive_budget_matches_hardcoded_for_200k_model() {
        let budget = adaptive_budget_for(AgentRole::Implementer, 200_000);
        let base = budget_for(AgentRole::Implementer);
        // 200K tokens * 4 = 800K chars. All sections hit their caps.
        assert_eq!(budget.plan, base.plan);
        assert_eq!(budget.workspace_map, base.workspace_map);
    }

    #[test]
    fn adaptive_budget_respects_minimum_for_tiny_model() {
        let budget = adaptive_budget_for(AgentRole::Implementer, 8_000);
        // 8K tokens * 4 = 32K chars. Plan = 25% = 8K, above min 5K
        assert_eq!(budget.plan, 8_000);
        // workspace_map = 10% = 3.2K, above min 2K
        assert_eq!(budget.workspace_map, 3_200);
    }

    #[test]
    fn adaptive_budget_helper_compute() {
        let b = AdaptiveBudget::new(0.25, 5_000, 50_000);
        assert_eq!(b.compute(800_000), 50_000);  // 25% = 200K, capped at 50K
        assert_eq!(b.compute(40_000), 10_000);   // 25% = 10K
        assert_eq!(b.compute(10_000), 5_000);    // 25% = 2.5K, floored at 5K
    }
}
```

**Wiring**: Templates that call `budget_for(role)` can be updated to call `adaptive_budget_for(role, model_context_tokens)` when the model's context size is known. This requires threading `model_context_tokens: usize` through the `sections()` method of each template -- a follow-up change.

## Agent Prompt

This batch has 3 changes that create new abstractions:

1. **Start with `gates.rs`** -- add `GateRungConfig` struct and `custom_rungs` field to `GatesConfig`. Also update `compat.rs` line 185 to add `custom_rungs: Vec::new()` to the explicit construction site. Then add re-exports in `config/mod.rs`.
2. **Create `workspace.rs`** -- new file in `roko-core/src/`, register in `lib.rs`. Check that `anyhow` and `tempfile` are in `Cargo.toml`.
3. **Add adaptive budgets to `common.rs`** -- new structs and functions after existing code. No changes to existing code.

For Change 1, the `default_true` function is imported from `super::agent::default_true` (line 7 of gates.rs). The new `GateRungConfig` struct uses it, so no new import is needed.

For Change 2, `anyhow` is NOT currently in `roko-core/Cargo.toml` -- you MUST add `anyhow = { workspace = true }` to the `[dependencies]` section before creating the file. The crate currently uses `thiserror` for typed errors, but `anyhow` is used by most other crates in the workspace and is defined in `[workspace.dependencies]` in the root `Cargo.toml`. `tempfile` is already in `[dev-dependencies]`.

For Change 3, `AgentRole` is already imported at line 8 of `common.rs` (`use roko_core::AgentRole;`). No new imports needed.

## Verification

```bash
# 1. Build
cargo check -p roko-core -p roko-compose

# 2. Run tests
cargo test -p roko-core -p roko-compose

# 3. Verify new files exist
test -f crates/roko-core/src/workspace.rs && echo "workspace.rs exists"
grep -q 'pub mod workspace' crates/roko-core/src/lib.rs && echo "workspace module registered"

# 4. Verify GateRungConfig exists
grep -q 'GateRungConfig' crates/roko-core/src/config/gates.rs && echo "gate rung config exists"

# 5. Verify adaptive budget exists
grep -q 'AdaptiveBudget' crates/roko-compose/src/templates/common.rs && echo "adaptive budget exists"
grep -q 'adaptive_budget_for' crates/roko-compose/src/templates/common.rs && echo "adaptive function exists"

# 6. Run the new workspace tests
cargo test -p roko-core -- workspace

# 7. Run the new adaptive budget tests
cargo test -p roko-compose -- adaptive

# 8. Clippy
cargo clippy -p roko-core -p roko-compose --no-deps -- -D warnings
```

## Why This Matters

- **Data-driven gate rungs** let users configure Roko for any language/toolchain via TOML instead of modifying Rust enums. A TypeScript project can add `npx tsc --noEmit` and `npx jest` as gates without touching roko source code.
- **Workspace abstraction** eliminates path construction bugs (typos in `.join("staet")` vs `.join("state")`) and ensures workspace validation happens once at entry rather than scattered across call sites.
- **Adaptive budgets** prevent prompt overflow on smaller models and under-utilization on larger ones. A 128K model currently gets the same 50K plan section as a 1M model -- adaptive scaling uses context proportionally, improving agent performance across different model tiers.

## Audit Status

Audited: 2026-05-05. 1 issue fixed (added missing GatesConfig construction site in compat.rs line 185 that uses explicit fields -- would cause compile error without custom_rungs field)
