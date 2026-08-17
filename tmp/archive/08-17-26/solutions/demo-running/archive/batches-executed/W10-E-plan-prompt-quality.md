# W10-E: Plan Prompt Quality and Config Fixes

**Priority**: P2 -- prompt quality and config UX issues
**Effort**: 2-3 hours
**Files to modify**: 2-3
**Dependencies**: W10-A (14.20 and 14.23 are subsumed into W10-A Change 3)

## Problem

Two remaining prompt quality and config issues not covered by W10-A:

1. **14.14**: Playbook usage is never recorded because of an ID mismatch. `self.playbook.record(&task_def.id, result.success)` looks up by task ID (`"T1"`, `"T2"`), but seeded playbooks have IDs like `"compile-check-loop"`, `"test-first"`. No match means playbook hit counts stay at zero and the system never learns which playbooks are effective.

2. **14.26**: `config_version` warning fires per-subprocess. Each agent child process has its own `static WARNED: Once` guard. With 3+ tasks, the warning appears 3+ times. The warning also fires falsely when `config_version = 2` is already set (the condition checks `config.config_version <= 1` but if the text has `config_version` explicitly set, the serde default never kicked in).

Note: 14.20 (plan.md always stub) and 14.23 (PRD type specs not embedded) are subsumed into W10-A Change 3 which rewrites the plan generation prompt to include both.

## Root Cause

### 14.14
`build_task_playbook(task_def)` creates a playbook with `id = task_def.id` (e.g., `"T1"`). But seeded playbooks from `seed_default_playbooks()` have semantic IDs like `"compile-check-loop"`. When `playbook.record("T1", success)` is called, it looks for a playbook file named `T1.json` -- no seeded playbook has that ID, so it returns `Ok(false)`. The seeded playbooks' hit counts never increment.

The fix: At record time, re-query the playbook store with the same context used at dispatch time, then record against the IDs of the matching playbooks.

### 14.26
The condition `config.config_version <= 1 && text_has_config_version(s)` fires when any config file has an explicit `config_version` field with value 1. But the real issue is it also fires for v2 configs because `config.config_version` is the deserialized value (which could be 2), but the condition says `<= 1`. The actual logic is correct but confusing -- the `static WARNED: Once` is per-process, so every subprocess spawned by the runner shows the warning independently. The fix is to compare against `CURRENT_CONFIG_VERSION` (which is already defined as `2` at line 38 of `schema.rs`) and extract the actual value from the text.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

#### Change 1 (14.14): Record playbook outcome by playbook IDs, not task ID

The enclosing function is `async fn record_task_success` (line 11121), so async playbook queries are available.

**Find this code** (lines 11339-11363):
```rust
        if let Some(task_def) = task_def.as_ref() {
            match self.playbook.record(&task_def.id, result.success).await {
                Ok(true) => {}
                Ok(false) if !result.success => {}
                Ok(false) => {
                    let playbook = build_task_playbook(task_def);
                    if let Err(err) = self.playbook.save(&playbook).await {
                        tracing::warn!(
                            plan_id = %plan_id,
                            task_id = %task_id,
                            error = %err,
                            "failed to persist inferred playbook"
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        plan_id = %plan_id,
                        task_id = %task_id,
                        error = %err,
                        "failed to record playbook outcome"
                    );
                }
            }
        }
```

**Replace with:**
```rust
        if let Some(task_def) = task_def.as_ref() {
            // Re-query which playbooks match this task (same query used at dispatch time).
            // Record outcome against their real IDs -- not task_def.id ("T1", "T2")
            // which never matches seeded playbooks like "compile-check-loop".
            let role = resolve_task_role(task_def.role.as_deref());
            let query_ctx = playbook_query_context(
                role,
                task_id,
                &task_def.title,
                Some(task_def),
            );
            match self.playbook.query(&query_ctx).await {
                Ok(matched) => {
                    for pb in &matched {
                        if let Err(err) = self.playbook.record(&pb.id, result.success).await {
                            tracing::warn!(
                                playbook_id = %pb.id,
                                task_id = %task_id,
                                error = %err,
                                "failed to record playbook outcome"
                            );
                        } else {
                            tracing::debug!(
                                playbook_id = %pb.id,
                                task_id = %task_id,
                                success = result.success,
                                "recorded playbook outcome"
                            );
                        }
                    }
                    if matched.is_empty() {
                        tracing::debug!(
                            task_id = %task_id,
                            "no matching playbooks found for outcome recording"
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        task_id = %task_id,
                        error = %err,
                        "failed to query playbooks for outcome recording"
                    );
                }
            }

            // Also save a task-specific inferred playbook on success.
            if result.success {
                let playbook = build_task_playbook(task_def);
                if let Err(err) = self.playbook.save(&playbook).await {
                    tracing::warn!(
                        plan_id = %plan_id,
                        task_id = %task_id,
                        error = %err,
                        "failed to persist inferred playbook"
                    );
                }
            }
        }
```

Note: `playbook_query_context` is imported at the top of orchestrate.rs (line ~199). `AgentRole` is imported at line 67. `resolve_task_role` is a private function defined at line 268 of orchestrate.rs -- it handles the `Option<&str>` -> `AgentRole` conversion via serde deserialization. The `TaskDef` struct has `role: Option<String>` and `title: String`.

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`

#### Change 2 (14.26): Fix config version warning logic

**Find this code** (lines 164-178):
```rust
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(s)?;
        // Only warn when the TOML text explicitly sets config_version (not when
        // the serde default of 1 kicks in for configs that omit the field, such
        // as the global config at ~/.roko/config.toml).
        if config.config_version <= 1 && text_has_config_version(s) {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "roko.toml uses config version 1 (no [providers] section)\n  hint: run `roko config migrate` to upgrade"
                );
            });
        }
        Ok(config)
    }
```

**Replace with:**
```rust
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(s)?;
        // Only warn when the TOML text explicitly sets config_version to a value
        // below CURRENT_CONFIG_VERSION. Skip if:
        //   - The field is absent (serde default kicks in; not a real v1 config)
        //   - The value matches or exceeds the current version
        //   - We've already warned in this process
        if text_has_config_version(s) {
            let explicit_version = extract_config_version_from_text(s);
            if explicit_version < CURRENT_CONFIG_VERSION {
                static WARNED: std::sync::Once = std::sync::Once::new();
                WARNED.call_once(|| {
                    tracing::warn!(
                        version = explicit_version,
                        current = CURRENT_CONFIG_VERSION,
                        "roko.toml uses config version {} (current is {})\n  \
                         hint: run `roko config migrate` to upgrade",
                        explicit_version,
                        CURRENT_CONFIG_VERSION,
                    );
                });
            }
        }
        Ok(config)
    }
```

**Add helper function** near `text_has_config_version` (which is at line ~44):

After the `text_has_config_version` function, add:

**Find this code** (lines 44-52 -- the `text_has_config_version` function):
```rust
fn text_has_config_version(s: &str) -> bool {
    s.lines()
        .any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("config_version")
                && trimmed[b"config_version".len()..]
                    .trim_start()
                    .starts_with('=')
        })
}
```

**Replace with:**
```rust
fn text_has_config_version(s: &str) -> bool {
    s.lines()
        .any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("config_version")
                && trimmed[b"config_version".len()..]
                    .trim_start()
                    .starts_with('=')
        })
}

/// Extract the numeric config_version value from raw TOML text.
/// Returns 1 as fallback if parsing fails.
fn extract_config_version_from_text(s: &str) -> u32 {
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("config_version") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(v) = val.trim().parse::<u32>() {
                    return v;
                }
            }
        }
    }
    1
}
```

Note: `CURRENT_CONFIG_VERSION` is already defined as `pub const CURRENT_CONFIG_VERSION: u32 = 2;` at line 38 of schema.rs, so it is in scope.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check
cargo check -p roko-cli -p roko-core 2>&1 | tail -5

# Verify playbook recording uses query + real IDs
grep -n 'playbook.record' crates/roko-cli/src/orchestrate.rs
# Should show recording against pb.id, not task_def.id

# Verify the old pattern is gone
grep -n 'record(&task_def.id' crates/roko-cli/src/orchestrate.rs
# Should return no results

# Verify config version check uses extracted version
grep -n 'extract_config_version_from_text' crates/roko-core/src/config/schema.rs
# Should show function definition and usage in from_toml

# Verify the old condition is gone
grep -n 'config.config_version <= 1' crates/roko-core/src/config/schema.rs
# Should return no results
```

## Agent Prompt

```
You are fixing two prompt quality and config bugs in the roko codebase. This is a Rust project at /Users/will/dev/nunchi/roko/roko.

NOTE: Issues 14.20 (plan.md stub) and 14.23 (PRD type specs) are handled by W10-A batch. This batch only covers 14.14 (playbook ID mismatch) and 14.26 (config version warning).

IMPORTANT: Read the source files FIRST before making changes.

### Fix 1 (14.14): Playbook ID mismatch

The problem: `self.playbook.record(&task_def.id, result.success)` uses task_def.id ("T1") but seeded playbooks have IDs like "compile-check-loop". Recording always misses.

Read these files:
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs lines 11339-11363 (the current record code)
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs lines 15436-15461 (dispatch-time query for reference)
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/learning_helpers.rs lines 513-525 (playbook_query_context)
- /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs lines 794-797 (PlaybookStore::query)
- /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs lines 954-956 (PlaybookStore::record)

Fix: Replace the single `self.playbook.record(&task_def.id, ...)` call with:
1. Resolve the role using `resolve_task_role(task_def.role.as_deref())` -- NOT `AgentRole::from_str` (AgentRole does not implement FromStr). `resolve_task_role` is defined at line 268 of orchestrate.rs. `task_def.role` is `Option<String>`.
2. Re-query playbooks using `playbook_query_context(role, task_id, &task_def.title, Some(task_def))`
3. Iterate matched playbooks and call `self.playbook.record(&pb.id, result.success)` for each
4. On success, also save an inferred task-specific playbook

The function is `async fn record_task_success`, so async queries work. `playbook_query_context` is imported at line ~199. `AgentRole` is imported at line 67. `resolve_task_role` is a private fn at line 268 (already in scope).

### Fix 2 (14.26): Config version warning fires per-subprocess and for v2 configs

Read /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs lines 164-178.

The current condition `config.config_version <= 1 && text_has_config_version(s)` is flawed:
- It uses the deserialized value (which defaults to 1 via serde), not the explicit text value
- `CURRENT_CONFIG_VERSION` is 2 (defined at line 38), so a v2 config correctly deserialized won't fire

The per-subprocess issue cannot be fully fixed (static Once is process-scoped, and child processes are separate). But the false-positive issue can be fixed by extracting the actual text value and comparing against CURRENT_CONFIG_VERSION.

Fix:
1. Add `extract_config_version_from_text(s: &str) -> u32` helper near `text_has_config_version`
2. Change the condition to: `if text_has_config_version(s) { let v = extract_config_version_from_text(s); if v < CURRENT_CONFIG_VERSION { ... } }`

After all changes, run:
```bash
cargo check -p roko-cli -p roko-core 2>&1 | tail -20
```
Then run the verification grep commands.
```

## Commit

This batch is committed with Wave 10. Do not commit individually.

## Checklist

- [ ] 14.14: Playbook outcomes recorded by actual playbook ID (re-query at record time)
- [ ] 14.14: `record(&task_def.id, ...)` removed, replaced with query+loop
- [ ] 14.14: Inferred task playbook still saved on success
- [ ] 14.26: `extract_config_version_from_text` helper added
- [ ] 14.26: Config version warning compares extracted value against `CURRENT_CONFIG_VERSION`
- [ ] 14.26: Old `config.config_version <= 1` condition removed
- [ ] `tracing::debug!` / `tracing::info!` instrumentation at key points
- [ ] `cargo check -p roko-cli -p roko-core` passes

## Audit Status

Audited: 2026-05-05. 2 issues fixed -- (1) `AgentRole::from_str` replaced with `resolve_task_role` (AgentRole does not implement FromStr), (2) note corrected: `task_def.role` is `Option<String>` not `String`
