# Context Injection Scoping

**Priority**: P1
**Size**: M (2-3 days)

---

## Problem

Every agent dispatch applies a flat context budget regardless of role. An implementer
doing a file edit and a strategist reviewing plan structure both receive the same
file-level context, error patterns, and playbook rules. This produces two failures:

1. **Strategists are noisy.** They receive file-level context (file intel, error patterns)
   they cannot act on — wasted tokens that dilute plan-level reasoning.

2. **Playbook rules are too broad.** The current `PlaybookRules::select()` call supplies
   only the task's files as match context, not the files the current *plan* touches.
   Rules from unrelated plans can fire on loosely-matching tags.

### What already exists (do NOT rebuild)

**`ContextScopingConfig`** and `role_context_limits()` are live in the runner event loop
(lines 227-373 of `crates/roko-cli/src/runner/event_loop.rs`). They solve the per-role
*episode* recall problem — strategists already receive zero similar-episode context:

```rust
pub struct ContextScopingConfig {
    pub max_file_intel_entries: usize,   // 10 / 3 / 0 per role
    pub max_warning_entries: usize,      // 5 / 3 / 0
    pub max_error_patterns: usize,       // 5 / 3 / 0
    pub max_similar_episodes: usize,     // 3 / 5 / 0
}
```

**`PlaybookRules`** (`crates/roko-learn/src/playbook_rules.rs`) has `select(ctx, limit)`.
`MatchContext` carries files, tags, category, error_signature, and role.

### What is missing

1. **`KnowledgeConfig`** — a config struct loadable from `roko.toml [knowledge]` that
   holds per-section enable toggles and size overrides. The `ContextScopingConfig` presets
   are compile-time constants with no operator-configurable overrides.
2. **`PlaybookScope`** — restricts rule matching to files touched by the current plan,
   preventing cross-plan rule bleed.
3. **`collect_plan_playbook_scope()`** — walks all tasks in a plan, unions their files,
   returns a `PlaybookScope` for pre-filtering the rule store.
4. **Config wiring** — `roko.toml [knowledge]` is documented but no config struct is
   parsed or applied.

---

## Solution

### 1. `KnowledgeConfig` (new config section)

Add to `crates/roko-core/src/config/` and wire into `RokoConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KnowledgeConfig {
    pub file_intel_enabled: bool,            // default: true
    pub file_intel_max_entries: usize,       // default: 10 (implementer)
    pub file_intel_reviewer_entries: usize,  // default: 3
    pub warnings_enabled: bool,              // default: true
    pub warning_max_entries: usize,          // default: 5
    pub warning_reviewer_entries: usize,     // default: 3
    pub error_patterns_enabled: bool,        // default: true
    pub error_pattern_max_entries: usize,    // default: 5
    pub error_pattern_min_cluster: usize,    // default: 3
    pub wave_context_enabled: bool,          // default: false
    pub plan_scoped_playbook_enabled: bool,  // default: true
}
```

### 2. Per-role filtering table

| Section | Implementer | Reviewer | Strategist |
|---|---|---|---|
| File intel | 10 (full) | 3 (summary) | 0 |
| Warnings | 5 | 3 | 0 |
| Error patterns | 5 | 3 | 0 |
| Episodes | 3 | 5 | 0 |

Strategist is always zero and not configurable.

### 3. `PlaybookScope` (new, in `crates/roko-compose/src/context_scoping.rs`)

```rust
pub struct PlaybookScope {
    pub plan_files: Vec<String>,  // union of all task.files in the plan
    pub plan_tags: Vec<String>,   // union of all task.tags
}

pub fn collect_plan_playbook_scope(plan_id: &str, tasks: &[TaskDef]) -> PlaybookScope;
pub fn apply_plan_scope(ctx: MatchContext, scope: &PlaybookScope, enabled: bool) -> MatchContext;
```

Computed once at plan startup, stored in executor state, threaded into every dispatch.

### 4. Wiring into `dispatch_agent_with()`

At the existing `role_context_limits()` call site in `event_loop.rs`:
1. Load `KnowledgeConfig` from executor config.
2. Override `ContextScopingConfig` presets from config values.
3. Pass `PlaybookScope` into `MatchContext` construction.
4. Call `apply_plan_scope()` when `plan_scoped_playbook_enabled == true`.
5. Skip file-intel/warning sections entirely when max == 0 (strategist).

---

## Where to implement

| Component | Path |
|---|---|
| `KnowledgeConfig` struct | `crates/roko-core/src/config/knowledge.rs` (new) |
| Config registration | `crates/roko-core/src/config/mod.rs` |
| `PlaybookScope` + helpers | `crates/roko-compose/src/context_scoping.rs` (new) |
| Module export | `crates/roko-compose/src/lib.rs` |
| Dispatch wiring | `crates/roko-cli/src/runner/event_loop.rs` |

Files explicitly NOT modified: `playbook_rules.rs` (scoping applied by caller, not store).

---

## Acceptance criteria

1. `KnowledgeConfig` loads from `roko.toml [knowledge]` — overriding `file_intel_max_entries = 3`
   produces the correct value; omitted fields take defaults.
2. Strategist prompts contain zero file-intel, zero warnings, zero error-patterns.
3. Implementer prompts receive up to `file_intel_max_entries` file-intel entries.
4. Reviewer prompts receive at most `file_intel_reviewer_entries` entries.
5. Plan-scoped playbook matching: a rule with `trigger_files = ["crates/bar/**"]` does not
   fire during a plan that only touches `crates/foo/**`.
6. `plan_scoped_playbook_enabled = false` restores pre-scoping behavior.
7. `file_intel_enabled = false` suppresses file-intel in all prompts regardless of role.

### Token impact estimate

| Role | Before | After | Saving |
|---|---|---|---|
| Strategist | ~2,400 tok | ~0 tok | ~2,400/dispatch |
| Reviewer | ~2,400 tok | ~720 tok | ~1,680/dispatch |
| Implementer | ~2,400 tok | ~2,400 tok | 0 |

For a typical plan (2 strategist + 3 reviewer + 8 implementer tasks): ~9,840 tokens
saved per plan execution (~15% of prompt token spend).

### Out of scope

- Dynamic budget adjustment per file complexity.
- Wave context injection logic (toggle field included, logic deferred).
- Changes to `PlaybookRules::select()` internals.
