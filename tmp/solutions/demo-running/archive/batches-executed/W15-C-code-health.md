# W15-C: Code Health -- unwrap, Hardcoded Models, Timeout Centralization (IMPROVEMENTS 6.1-6.3)

**Priority**: P2 -- reduce panic surface, centralize configuration
**Effort**: 6-8 hours (incremental; start with priority files)
**Files to modify**: 4-6 priority files + 2 new/existing files
**Dependencies**: None

## Problem

Three codebase-wide code health issues create fragility:

1. **`unwrap()` calls in non-test production code**. Priority files by count:
   - `main.rs` (123 -- many acceptable for CLI arg parsing)
   - `roko-learn/src/skill_library.rs` (100)
   - `roko-learn/src/runtime_feedback.rs` (85)
   - `roko-fs/src/file_substrate.rs` (81)
   - `roko-cli/src/config.rs` (73)
   - `orchestrate.rs` (1 non-test: line 16348)
   - `roko-orchestrator/src/dag.rs` (57)
   - `roko-orchestrator/src/worktree.rs` (56)

2. **Hardcoded model name references** -- strings like `"claude-haiku-4-5"`, `"claude-sonnet-4-6"`, `"claude-opus-4-6"` across many crates (981 total occurrences across 156 files, most in tests). `defaults.rs` already has `MODEL_FAST`, `MODEL_FOCUSED`, `MODEL_DEEP` constants (lines 301-313) but many production call sites don't use them.

3. **No `TimeoutConfig` struct** -- `Duration::from_secs(600)`, `Duration::from_secs(120)`, etc. are scattered everywhere. `defaults.rs` has some timeout constants but there's no TOML-configurable struct.

## Exact Code to Change

### Priority 1: Top unwrap() Replacements (6.1)

Focus on the highest-risk production code paths.

#### File 1: `crates/roko-cli/src/orchestrate.rs` (1 non-test unwrap)

Only 1 `unwrap()` is in production code (the other 59 are in `#[cfg(test)]` blocks).

**Find this code (line 16348):**

```rust
                    Arc::clone(self.chain_client.as_ref().unwrap()),
```

The guard at line 16346 (`if self.chain_client.is_some()`) makes this unwrap safe, but it's brittle. Replace with `expect` to document the invariant:

**Replace with:**

```rust
                    Arc::clone(self.chain_client.as_ref().expect("guarded by is_some() check above")),
```

#### File 2: `crates/roko-orchestrator/src/dag.rs` (1 production unwrap, rest in tests)

**IMPORTANT**: This file has ~57 total unwraps, but only **1** is in production code (line 861). The `#[cfg(test)]` module starts at line 1817 and contains all other unwraps. Test unwraps are acceptable -- do NOT fix them.

The one production unwrap at line 861:

**Find this code (line 861):**

```rust
                let tail = chain.last().unwrap();
```

**Replace with:**

```rust
                let Some(tail) = chain.last() else {
                    // chain is guaranteed non-empty by the loop above, but avoid panic
                    continue;
                };
```

**NOTE**: This crate does NOT have `anyhow` as a dependency -- it uses `thiserror` with typed errors (`DagError`, `DagMutationError`). Do NOT use `anyhow::anyhow!()` or `anyhow::Context`. If you need to return errors, use the existing `DagError` variants or `DagMutationError::UnknownTask`.

#### File 3: `crates/roko-orchestrator/src/worktree.rs` (0 production unwraps)

**IMPORTANT**: All unwraps in this file are in the `#[cfg(test)]` module starting at line 757. There are **zero** production unwraps. Skip this file entirely -- no changes needed.

**NOTE**: Like `dag.rs`, this crate uses `thiserror` with `WorktreeError`, not `anyhow`. Do NOT add `anyhow` imports.

#### File 4: `crates/roko-learn/src/skill_library.rs` (determine production vs test split)

Many of these are likely in test code. Determine the split:
```bash
grep -n '\.unwrap()' crates/roko-learn/src/skill_library.rs | head -20
# Also find where the test module starts:
grep -n '#\[cfg(test)\]' crates/roko-learn/src/skill_library.rs
```

For production code, replace with `.unwrap_or_default()` or `?` depending on context. Leave test unwraps alone.

**Minimum goal**: Fix the 1 production unwrap in `orchestrate.rs` (line 16348) and the 1 production unwrap in `dag.rs` (line 861). The other files are either test-only or follow-up.

---

### Priority 2: Hardcoded Model Name Extraction (6.2)

#### File: `crates/roko-core/src/defaults.rs` (lines 301-313)

The constants already exist -- no changes needed here:

```rust
pub const MODEL_DEEP: &str = "claude-opus-4-6";
pub const MODEL_FOCUSED: &str = "claude-sonnet-4-6";
pub const MODEL_FAST: &str = "claude-haiku-4-5";
pub const MODEL_ESCALATION_LADDER: [&str; 3] = [MODEL_FAST, MODEL_FOCUSED, MODEL_DEEP];
```

**The work is in the callers.** Focus on production code (not tests, not snapshots):

**Priority caller 1: `crates/roko-neuro/src/distiller.rs` line 25:**

**Find this code:**

```rust
const DEFAULT_MODEL: &str = "claude-haiku-4-5";
```

**Replace with:**

```rust
const DEFAULT_MODEL: &str = roko_core::defaults::MODEL_FAST;
```

**Priority caller 2: `crates/roko-primitives/src/tier.rs` lines 64-102:**

This file has 11 occurrences (3 in production code, 2 in doc comments, 6 in tests). The `select_model()` method returns hardcoded model names. Replace each production instance:

**Find this code (line 70):**

```rust
            InferenceTier::T1 => Some("claude-haiku-4-5"),
```

**Replace with:**

```rust
            InferenceTier::T1 => Some(roko_core::defaults::MODEL_FAST),
```

Apply the same for all `"claude-sonnet-4-6"` -> `MODEL_FOCUSED` and `"claude-opus-4-6"` -> `MODEL_DEEP` in this file.

**Note**: `roko-primitives` must have `roko-core` as a dependency. Check `crates/roko-primitives/Cargo.toml`. If `roko-core` is not listed, this caller cannot use the constants -- skip it and note the dependency issue. An alternative is to duplicate the constants into `roko-primitives/src/defaults.rs`.

**Other priority callers** (run the grep to find them):
```bash
grep -rn '"claude-haiku-4-5"\|"claude-sonnet-4-6"\|"claude-opus-4-6"' crates/ --include='*.rs' | grep -v target/ | grep -v '#\[test\]' | grep -v '#\[cfg(test)\]' | grep -v '\.snap' | grep -v '/tests/' | head -30
```

Replace each non-test reference with the appropriate constant:

| Pattern | Replace with |
|---------|-------------|
| `"claude-haiku-4-5"` | `roko_core::defaults::MODEL_FAST` |
| `"claude-sonnet-4-6"` | `roko_core::defaults::MODEL_FOCUSED` |
| `"claude-opus-4-6"` | `roko_core::defaults::MODEL_DEEP` |

**Note**: Test code can keep hardcoded model names for readability. Focus on production code paths only.

---

### Priority 3: Timeout Centralization (6.3)

#### New File: `crates/roko-core/src/config/timeouts.rs`

**Create** this new file:

```rust
//! Configurable timeout values for the Roko runtime.
//!
//! Replaces hardcoded `Duration::from_secs(N)` values scattered across crates
//! with a single, TOML-configurable struct. Each field has a sensible default
//! that matches the current hardcoded value.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// TOML-configurable timeout values.
///
/// Maps to `[timeouts]` in `roko.toml`. Each field defaults to the value
/// that was previously hardcoded across the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    /// Agent dispatch timeout (seconds). Default: 600.
    pub agent_dispatch_secs: u64,
    /// Gate compile check timeout (seconds). Default: 120.
    pub gate_compile_secs: u64,
    /// Gate test timeout (seconds). Default: 300.
    pub gate_test_secs: u64,
    /// Gate clippy timeout (seconds). Default: 60.
    pub gate_clippy_secs: u64,
    /// LLM API call timeout (seconds). Default: 120.
    pub llm_call_secs: u64,
    /// HTTP request timeout (seconds). Default: 30.
    pub http_request_secs: u64,
    /// Workspace lock acquisition timeout (seconds). Default: 5.
    pub workspace_lock_secs: u64,
    /// Health check timeout (seconds). Default: 3.
    pub health_check_secs: u64,
    /// Total plan execution timeout (seconds). Default: 3600.
    pub plan_total_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            agent_dispatch_secs: 600,
            gate_compile_secs: 120,
            gate_test_secs: 300,
            gate_clippy_secs: 60,
            llm_call_secs: 120,
            http_request_secs: 30,
            workspace_lock_secs: 5,
            health_check_secs: 3,
            plan_total_secs: 3600,
        }
    }
}

impl TimeoutConfig {
    /// Agent dispatch timeout as `Duration`.
    #[must_use]
    pub fn agent_dispatch(&self) -> Duration {
        Duration::from_secs(self.agent_dispatch_secs)
    }

    /// Gate compile timeout as `Duration`.
    #[must_use]
    pub fn gate_compile(&self) -> Duration {
        Duration::from_secs(self.gate_compile_secs)
    }

    /// Gate test timeout as `Duration`.
    #[must_use]
    pub fn gate_test(&self) -> Duration {
        Duration::from_secs(self.gate_test_secs)
    }

    /// Gate clippy timeout as `Duration`.
    #[must_use]
    pub fn gate_clippy(&self) -> Duration {
        Duration::from_secs(self.gate_clippy_secs)
    }

    /// LLM call timeout as `Duration`.
    #[must_use]
    pub fn llm_call(&self) -> Duration {
        Duration::from_secs(self.llm_call_secs)
    }

    /// HTTP request timeout as `Duration`.
    #[must_use]
    pub fn http_request(&self) -> Duration {
        Duration::from_secs(self.http_request_secs)
    }

    /// Workspace lock timeout as `Duration`.
    #[must_use]
    pub fn workspace_lock(&self) -> Duration {
        Duration::from_secs(self.workspace_lock_secs)
    }

    /// Health check timeout as `Duration`.
    #[must_use]
    pub fn health_check(&self) -> Duration {
        Duration::from_secs(self.health_check_secs)
    }

    /// Total plan execution timeout as `Duration`.
    #[must_use]
    pub fn plan_total(&self) -> Duration {
        Duration::from_secs(self.plan_total_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let tc = TimeoutConfig::default();
        assert!(tc.agent_dispatch_secs > 0);
        assert!(tc.gate_compile_secs > 0);
        assert!(tc.gate_test_secs > tc.gate_compile_secs);
        assert!(tc.plan_total_secs > tc.agent_dispatch_secs);
        assert!(tc.health_check_secs < tc.http_request_secs);
    }

    #[test]
    fn duration_helpers_match_secs() {
        let tc = TimeoutConfig::default();
        assert_eq!(tc.agent_dispatch(), Duration::from_secs(600));
        assert_eq!(tc.gate_compile(), Duration::from_secs(120));
        assert_eq!(tc.health_check(), Duration::from_secs(3));
    }
}
```

**Then** register the module in `crates/roko-core/src/config/mod.rs`:

**Find this code (lines 17-18):**

```rust
pub mod gates;
pub mod hot_reload;
```

**Replace with:**

```rust
pub mod gates;
pub mod hot_reload;
pub mod timeouts;
```

#### Wire into `RokoConfig` in `crates/roko-core/src/config/schema.rs`

**Find this code (lines 88-89 of schema.rs):**

```rust
    #[serde(default)]
    pub tui: TuiConfig,
```

**Replace with:**

```rust
    #[serde(default)]
    pub tui: TuiConfig,
    /// Timeout configuration. Maps to `[timeouts]` in roko.toml.
    #[serde(default)]
    pub timeouts: super::timeouts::TimeoutConfig,
```

**Then** add `TimeoutConfig` to the re-exports in `crates/roko-core/src/config/mod.rs`:

**Find this code (lines 50-51):**

```rust
pub use schema::{
    AgentBudget, AgentConfig, AgentDefinition, AgentMode, AgentThresholds, ApiKeyEntry,
```

**Replace with:**

```rust
pub use timeouts::TimeoutConfig;
pub use schema::{
    AgentBudget, AgentConfig, AgentDefinition, AgentMode, AgentThresholds, ApiKeyEntry,
```

**Progressive replacement**: Callers that use `Duration::from_secs(600)` can be migrated to `config.timeouts.agent_dispatch()` incrementally. This batch creates the struct and wires it into config. The Duration replacements are follow-up.

## Agent Prompt

This batch has 3 priorities. Work in this order:

1. **Create `timeouts.rs`** and wire it into config (self-contained, no dependencies)
2. **Replace hardcoded model names** -- run the grep, replace production callers
3. **Fix the 2 production unwraps** -- `orchestrate.rs` line 16348 and `dag.rs` line 861

For the unwrap fixes: `orchestrate.rs` line 16348 has an `unwrap()` guarded by `is_some()` -- replace with `expect(...)`. `dag.rs` line 861 has `chain.last().unwrap()` -- replace with `let Some(tail) = chain.last() else { continue; }`. Do NOT fix unwraps in test code -- only production paths. `worktree.rs` has zero production unwraps (skip it).

**IMPORTANT**: `roko-orchestrator` does NOT have `anyhow` -- it uses `thiserror` with typed errors (`DagError`, `WorktreeError`). Do NOT use `anyhow::Context` or `anyhow::anyhow!()` in these files.

For model names: verify that `roko-primitives` depends on `roko-core` before using the constants. Check `Cargo.toml`. (It does NOT -- skip `tier.rs` or duplicate constants.)

## Verification

```bash
# 1. Build affected crates
cargo check -p roko-core -p roko-cli

# 2. Run tests
cargo test -p roko-core -p roko-cli

# 3. Check hardcoded model names reduced
grep -rn '"claude-' crates/ --include='*.rs' | grep -v target/ | grep -v test | grep -v '\.snap' | grep -v '/tests/' | wc -l
# Should decrease from current count

# 4. Verify TimeoutConfig exists and compiles
grep -q 'TimeoutConfig' crates/roko-core/src/config/timeouts.rs && echo "exists"

# 5. Verify TimeoutConfig is in RokoConfig
grep -q 'timeouts' crates/roko-core/src/config/schema.rs && echo "wired"

# 6. Clippy
cargo clippy -p roko-core -p roko-cli --no-deps -- -D warnings
```

## Why This Matters

- Every `unwrap()` in production code is a potential panic. `orchestrate.rs` (1 at line 16348) and `dag.rs` (1 at line 861) are the priority fixes. `worktree.rs` has zero production unwraps (all are in tests).
- Hardcoded model names break when users run non-Claude providers. Using constants from `defaults.rs` makes provider changes a single-point update.
- `TimeoutConfig` allows operators to tune timeouts via `roko.toml` without code changes, essential for different deployment environments.

## Audit Status

Audited: 2026-05-05. 3 issues fixed (1: dag.rs has 1 production unwrap not 57 -- rest are in tests; 2: worktree.rs has 0 production unwraps not 56 -- all in tests; 3: roko-orchestrator uses thiserror not anyhow -- corrected advice to avoid anyhow::Context. Also fixed tier.rs method name from model_slug() to select_model())
