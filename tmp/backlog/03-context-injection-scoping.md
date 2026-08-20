# 03 — Context Injection Scoping

**Priority**: P1 — Directly reduces token cost and improves prompt quality (~15% savings per plan)
**Size**: M (2-3 days)
**Crates**: `crates/roko-core` (config), `crates/roko-compose` (new module), `crates/roko-cli` (runner wiring)
**Depends on**: None

---

## Background

Roko dispatches agents in distinct roles: **Implementer** (writes code), **Reviewer** (audits output), and **Strategist** (plans high-level structure). Before each LLM call, the runner assembles a prompt that includes several context sections: file intelligence (neuro store entries for files the task touches), recent error patterns, lint warnings, similar past episodes, and playbook rules (learned best practices).

The per-role limits for these context sections already exist in the runner. An Implementer receives up to 10 file-intel entries, a Reviewer gets 3, and a Strategist gets 0. These presets are enforced by `ContextScopingConfig` at `crates/roko-cli/src/runner/event_loop.rs` (lines 227-373).

The problem has two parts:

**Part 1 — No operator overrides**: The `ContextScopingConfig` presets are compile-time constants with no way for an operator to tune them via `roko.toml`. A workspace that wants to reduce Reviewer context to 1 entry (to save cost) or increase Implementer context to 15 entries (for a complex codebase) has no mechanism to do so. The CLAUDE.md documents a `[knowledge]` section as configurable, but no config struct for it exists in `crates/roko-core/src/config/`.

**Part 2 — Playbook rules are not scoped to the current plan**: `PlaybookRules::select()` in `crates/roko-learn/src/playbook_rules.rs` accepts a `MatchContext` (files, tags, category, error_signature, role). The runner passes only the current *task's* files as match context. This means a playbook rule for `crates/bar/**` can fire during a plan that only touches `crates/foo/**`, if the task happens to share a tag or role. Cross-plan rule bleed produces irrelevant advice that dilutes the signal-to-noise ratio for the agent.

## Current State

1. **`ContextScopingConfig`** exists at `crates/roko-cli/src/runner/event_loop.rs` lines 240-250. It has four fields: `max_file_intel_entries`, `max_warning_entries`, `max_error_patterns`, `max_similar_episodes`. Three compile-time presets exist: `IMPLEMENTER`, `REVIEWER`, `STRATEGIST` (lines 317-343).

2. **`role_context_limits()`** at line 355 maps `AgentRole` enum values to one of the three presets. This function is already called in the runner before prompt assembly (line 9880).

3. **`PlaybookRules::select(ctx, limit)`** in `crates/roko-learn/src/playbook_rules.rs` takes a `MatchContext` struct. `MatchContext` has fields: `files: Vec<String>`, `tags: Vec<String>`, `category: Option<String>`, `error_signature: Option<String>`, `role: Option<String>`. All matching is against these fields. There is no plan-level scope filtering.

4. **No `KnowledgeConfig` struct exists** anywhere in `crates/roko-core/src/config/`. Confirmed by searching all `.rs` files in that directory — the struct is absent.

5. **`RokoConfig`** in `crates/roko-core/src/config/schema.rs` does not have a `knowledge` field (line 89-176). Adding one requires adding the field and a corresponding submodule.

6. **`crates/roko-compose/`** has many modules but no `context_scoping.rs` file. This is the correct home for `PlaybookScope` and its helpers since it is the prompt-composition layer.

7. **`crates/roko-compose/src/lib.rs`** exists and must be edited to export the new `context_scoping` module.

## Implementation Plan

### Step 1: Create `KnowledgeConfig` in `crates/roko-core/src/config/knowledge.rs` (new file)

```rust
//! Knowledge context injection configuration for roko.toml [knowledge].

use serde::{Deserialize, Serialize};

/// Operator overrides for per-role context injection limits.
///
/// All fields have defaults that match the compile-time `ContextScopingConfig`
/// presets so existing configurations continue to behave identically.
///
/// Example roko.toml:
/// ```toml
/// [knowledge]
/// file_intel_max_entries = 5       # Implementer default: 10
/// file_intel_reviewer_entries = 2  # Reviewer default: 3
/// plan_scoped_playbook_enabled = true
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KnowledgeConfig {
    /// Enable file intelligence context injection (default: true).
    pub file_intel_enabled: bool,
    /// Max file-intel entries for Implementer role (default: 10).
    pub file_intel_max_entries: usize,
    /// Max file-intel entries for Reviewer role (default: 3).
    /// Strategist always receives 0 regardless of this setting.
    pub file_intel_reviewer_entries: usize,
    /// Enable warning context injection (default: true).
    pub warnings_enabled: bool,
    /// Max warning entries for Implementer (default: 5).
    pub warning_max_entries: usize,
    /// Max warning entries for Reviewer (default: 3).
    pub warning_reviewer_entries: usize,
    /// Enable error pattern injection (default: true).
    pub error_patterns_enabled: bool,
    /// Max error pattern entries for Implementer (default: 5).
    pub error_pattern_max_entries: usize,
    /// Min cluster size before an error pattern is injected (default: 3).
    pub error_pattern_min_cluster: usize,
    /// When true, playbook rule matching is pre-filtered to files and tags
    /// touched by the current plan only. Prevents cross-plan rule bleed (default: true).
    pub plan_scoped_playbook_enabled: bool,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            file_intel_enabled: true,
            file_intel_max_entries: 10,
            file_intel_reviewer_entries: 3,
            warnings_enabled: true,
            warning_max_entries: 5,
            warning_reviewer_entries: 3,
            error_patterns_enabled: true,
            error_pattern_max_entries: 5,
            error_pattern_min_cluster: 3,
            plan_scoped_playbook_enabled: true,
        }
    }
}
```

### Step 2: Register the config in `crates/roko-core/src/config/mod.rs`

Add:
```rust
pub mod knowledge;
pub use knowledge::KnowledgeConfig;
```

### Step 3: Add `knowledge` field to `RokoConfig` in `crates/roko-core/src/config/schema.rs`

After the `pub prompt: PromptConfig,` field (around line 172), add:

```rust
    /// Context injection tuning for per-role knowledge sections.
    #[serde(default)]
    pub knowledge: KnowledgeConfig,
```

Also add `pub use super::knowledge::*;` to the re-export block at the top of `schema.rs` (around line 34).

### Step 4: Create `crates/roko-compose/src/context_scoping.rs` (new file)

```rust
//! Plan-scoped playbook filtering helpers.
//!
//! `PlaybookScope` captures the files and tags a plan touches. It is computed
//! once at plan startup and threaded into every dispatch call so that playbook
//! rule matching is restricted to rules relevant to the current plan.

use roko_core::task::TaskDef;

/// Files and tags touched by all tasks in a plan.
/// Used to pre-filter playbook rule selection.
#[derive(Debug, Clone, Default)]
pub struct PlaybookScope {
    /// Union of all `task.files` across every task in the plan.
    pub plan_files: Vec<String>,
    /// Union of all `task.tags` across every task in the plan.
    pub plan_tags: Vec<String>,
}

impl PlaybookScope {
    /// Build a scope from all tasks in a plan.
    pub fn from_tasks(tasks: &[TaskDef]) -> Self {
        let mut plan_files: Vec<String> = Vec::new();
        let mut plan_tags: Vec<String> = Vec::new();

        for task in tasks {
            for f in &task.files {
                if !plan_files.contains(f) {
                    plan_files.push(f.clone());
                }
            }
            for t in &task.tags {
                if !plan_tags.contains(t) {
                    plan_tags.push(t.clone());
                }
            }
        }

        Self { plan_files, plan_tags }
    }

    /// Apply this scope to a `MatchContext` by intersecting its files and tags.
    ///
    /// When `enabled` is false, `ctx` is returned unchanged (disables scoping).
    /// When `enabled` is true and the plan has no files/tags (empty scope),
    /// the context is also returned unchanged (permissive fallback).
    pub fn apply_to_match_context(
        &self,
        mut ctx: roko_learn::playbook_rules::MatchContext,
        enabled: bool,
    ) -> roko_learn::playbook_rules::MatchContext {
        if !enabled || (self.plan_files.is_empty() && self.plan_tags.is_empty()) {
            return ctx;
        }
        // Restrict files to those that overlap with plan scope.
        ctx.files.retain(|f| {
            self.plan_files.iter().any(|pf| {
                // Simple prefix/substring match; glob matching can be added later.
                f.starts_with(pf.as_str()) || pf.starts_with(f.as_str())
            })
        });
        // Restrict tags to those that overlap with plan scope.
        ctx.tags.retain(|t| self.plan_tags.contains(t));
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scope_is_permissive() {
        let scope = PlaybookScope::default();
        let ctx = roko_learn::playbook_rules::MatchContext {
            files: vec!["crates/foo/src/lib.rs".to_string()],
            tags: vec!["rust".to_string()],
            category: None,
            error_signature: None,
            role: None,
        };
        let result = scope.apply_to_match_context(ctx.clone(), true);
        assert_eq!(result.files, ctx.files); // unchanged when scope is empty
    }

    #[test]
    fn scope_filters_unrelated_files() {
        let scope = PlaybookScope {
            plan_files: vec!["crates/foo".to_string()],
            plan_tags: vec![],
        };
        let ctx = roko_learn::playbook_rules::MatchContext {
            files: vec![
                "crates/foo/src/lib.rs".to_string(),
                "crates/bar/src/lib.rs".to_string(),
            ],
            tags: vec![],
            category: None,
            error_signature: None,
            role: None,
        };
        let result = scope.apply_to_match_context(ctx, true);
        assert_eq!(result.files, vec!["crates/foo/src/lib.rs".to_string()]);
    }

    #[test]
    fn disabled_scope_passes_through_unchanged() {
        let scope = PlaybookScope {
            plan_files: vec!["crates/foo".to_string()],
            plan_tags: vec![],
        };
        let ctx = roko_learn::playbook_rules::MatchContext {
            files: vec!["crates/bar/src/lib.rs".to_string()],
            tags: vec![],
            category: None,
            error_signature: None,
            role: None,
        };
        let result = scope.apply_to_match_context(ctx.clone(), false);
        assert_eq!(result.files, ctx.files); // unchanged when disabled
    }
}
```

### Step 5: Export the module in `crates/roko-compose/src/lib.rs`

Add one line to the module list:

```rust
/// Plan-scoped playbook filtering helpers.
pub mod context_scoping;
```

### Step 6: Wire into `crates/roko-cli/src/runner/event_loop.rs`

There are two integration points.

**6a. Override `ContextScopingConfig` from `KnowledgeConfig`**

Locate `role_context_limits(role_enum)` call at line 9880. After this call, apply the operator overrides from `KnowledgeConfig`:

```rust
let mut context_scope = role_context_limits(role_enum);
// Apply operator overrides from roko.toml [knowledge] if present.
if let Some(kc) = &config.knowledge_config_ref() {
    // Strategist presets are always zero; do not override.
    if context_scope.max_file_intel_entries > 0 {
        if !kc.file_intel_enabled {
            context_scope.max_file_intel_entries = 0;
            context_scope.max_warning_entries = 0;
            context_scope.max_error_patterns = 0;
        } else {
            // Role-specific overrides.
            use roko_core::AgentRole as R;
            match role_enum {
                R::Auditor | R::QuickReviewer | R::Critic
                | R::SpecDriftDetector | R::RegressionDetector
                | R::DocVerifier | R::SnapshotComparator => {
                    context_scope.max_file_intel_entries = kc.file_intel_reviewer_entries;
                    context_scope.max_warning_entries = kc.warning_reviewer_entries;
                    context_scope.max_error_patterns = kc.error_pattern_max_entries;
                }
                _ => {
                    // Implementer-class
                    context_scope.max_file_intel_entries = kc.file_intel_max_entries;
                    context_scope.max_warning_entries = kc.warning_max_entries;
                    context_scope.max_error_patterns = kc.error_pattern_max_entries;
                }
            }
        }
    }
}
```

The `KnowledgeConfig` is accessed from `RunConfig` or the executor config. Add a method like `config.knowledge()` that returns a reference to `KnowledgeConfig` from `RokoConfig`.

**6b. Apply `PlaybookScope` before playbook rule selection**

The `PlaybookScope` must be computed once at plan startup (when the plan's `TaskDef` list is known) and stored in `RunState` or threaded into each dispatch. Add a field:

```rust
// In RunState or the per-plan state:
pub playbook_scope: roko_compose::context_scoping::PlaybookScope,
```

Initialize it when loading a plan's tasks:

```rust
let playbook_scope = roko_compose::context_scoping::PlaybookScope::from_tasks(&task_defs);
```

At the `PlaybookRules::select()` call site in `event_loop.rs`, wrap the `MatchContext` construction with:

```rust
let scoped_ctx = playbook_scope.apply_to_match_context(raw_ctx, knowledge_config.plan_scoped_playbook_enabled);
let rules = playbook_store.select(&scoped_ctx, limit);
```

To find the `PlaybookRules::select` call site, search `event_loop.rs` for `playbook_rules` or `playbook_store.select`.

## Acceptance Criteria

1. Adding `[knowledge]` to `roko.toml` with `file_intel_max_entries = 3` causes the runner to inject at most 3 file-intel entries for Implementer roles. Verified by unit test.

2. Omitting `[knowledge]` entirely produces the same behavior as the current compile-time presets (backwards compatible).

3. `file_intel_enabled = false` in `[knowledge]` suppresses file-intel in prompts for all roles, including Implementer.

4. Strategist prompts contain zero file-intel entries regardless of `[knowledge]` settings.

5. With `plan_scoped_playbook_enabled = true` (default): a playbook rule with `trigger_files = ["crates/bar/**"]` does not fire during a plan whose `PlaybookScope.plan_files` contains only `crates/foo/` paths.

6. With `plan_scoped_playbook_enabled = false`: the pre-scoping behavior is restored — cross-plan rules can fire.

7. `PlaybookScope::from_tasks(&[])` returns an empty scope (permissive — does not filter anything).

8. Unit tests for `context_scoping.rs` all pass: empty scope is permissive, non-empty scope filters unrelated files, disabled flag passes through unchanged.

## Verification Checklist

- [ ] `cargo build --workspace` passes after adding `KnowledgeConfig` and the new field to `RokoConfig`
- [ ] `cargo test -p roko-core` passes — add a TOML round-trip test for `[knowledge]` section
- [ ] `cargo test -p roko-compose context_scoping` passes all three unit tests in the new module
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean
- [ ] Manually verify: add `[knowledge]\nfile_intel_max_entries = 2` to `roko.toml`, run `cargo run -p roko-cli -- plan run plans/ --engine runner-v2` on a small plan, confirm prompt logs show at most 2 file-intel entries for Implementer tasks
- [ ] Confirm Strategist task prompts still contain zero file-intel entries even with non-zero `file_intel_max_entries` in config
- [ ] Confirm `plan_scoped_playbook_enabled = false` in config restores pre-scoping behavior

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/config/knowledge.rs` | **New file** — `KnowledgeConfig` struct with defaults |
| `crates/roko-core/src/config/mod.rs` | Add `pub mod knowledge; pub use knowledge::KnowledgeConfig;` |
| `crates/roko-core/src/config/schema.rs` | Add `pub knowledge: KnowledgeConfig` field to `RokoConfig`; add `pub use super::knowledge::*` to re-exports |
| `crates/roko-compose/src/context_scoping.rs` | **New file** — `PlaybookScope`, `from_tasks()`, `apply_to_match_context()` with unit tests |
| `crates/roko-compose/src/lib.rs` | Add `pub mod context_scoping;` |
| `crates/roko-cli/src/runner/event_loop.rs` | Override `ContextScopingConfig` from `KnowledgeConfig` after `role_context_limits()` call (line ~9880); add `PlaybookScope` to `RunState` and apply it before `select()` |
