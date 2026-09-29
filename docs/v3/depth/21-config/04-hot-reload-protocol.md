# 21-config / 04 -- Hot Reload Protocol

> How config changes are detected, classified, and applied at runtime without
> restarting the process. The hot-reload system provides atomic section swaps,
> restart-required detection, and audit journaling.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## 1. Section Classification

Every config section is classified as either hot-reloadable or restart-required:

```rust
pub enum ConfigSection {
    Budget,     // hot-reloadable
    Tools,      // hot-reloadable
    Learning,   // hot-reloadable
    Gates,      // hot-reloadable
    Conductor,  // hot-reloadable
    Routing,    // hot-reloadable
    Agent,      // restart-required
    Providers,  // restart-required
    Models,     // restart-required
    Serve,      // restart-required
    Scheduler,  // restart-required
    Watcher,    // restart-required
    Other(String),  // restart-required
}

impl ConfigSection {
    pub const fn is_hot_reloadable(&self) -> bool {
        matches!(self, Budget | Tools | Learning | Gates | Conductor | Routing)
    }
}
```

Restart-required sections affect process-wide state (provider connections,
model registries, bind addresses) that cannot be changed after initialization.

---

## 2. Diff Detection

`config_diff()` compares two `RokoConfig` values section by section using
Rust's `PartialEq`. For each section that differs, a `ConfigChange` is emitted:

```rust
pub struct ConfigChange {
    pub section: ConfigSection,
    pub summary: String,
}

pub fn config_diff(old: &RokoConfig, new: &RokoConfig) -> Vec<ConfigChange> {
    let mut changes = Vec::new();
    if old.budget != new.budget {
        changes.push(ConfigChange {
            section: ConfigSection::Budget,
            summary: format!(
                "budget: plan ${:.2} -> ${:.2}, turn ${:.2} -> ${:.2}",
                old.budget.max_plan_usd, new.budget.max_plan_usd,
                old.budget.max_turn_usd, new.budget.max_turn_usd,
            ),
        });
    }
    // ... similar for each section
    changes
}
```

The diff is coarse-grained (section level, not field level) because the
atomic swap operates on entire sections. The `summary` provides a human-readable
description of what changed.

---

## 3. Atomic Section Swap

`apply_hot_reload()` iterates over detected changes and applies hot-reloadable
sections while flagging restart-required ones:

```rust
pub fn apply_hot_reload(
    current: &mut RokoConfig,
    new_config: &RokoConfig,
    changes: &[ConfigChange],
) -> HotReloadResult {
    let mut applied = Vec::new();
    let mut needs_restart = Vec::new();

    for change in changes {
        if change.section.is_hot_reloadable() {
            // Clone the new section value into current
            match &change.section {
                ConfigSection::Budget => current.budget = new_config.budget.clone(),
                ConfigSection::Tools => current.tools = new_config.tools.clone(),
                // ...
            }
            applied.push(change.clone());
        } else {
            needs_restart.push(change.clone());
        }
    }

    HotReloadResult { applied, needs_restart }
}
```

The gate section swap includes the pipeline config:
```rust
ConfigSection::Gates => {
    current.gates = new_config.gates.clone();
    current.pipeline = new_config.pipeline.clone();
}
```

---

## 4. try_reload: The Safe Reload Path

`try_reload()` is the primary reload entry point. It loads through the complete
unified pipeline and only mutates `current` on success:

```rust
pub fn try_reload(
    current: &mut RokoConfig,
    workdir: &Path,
) -> Result<HotReloadResult, String> {
    let new_config = load_config_unified(workdir).map_err(|e| e.to_string())?;
    let changes = config_diff(current, &new_config);
    Ok(apply_hot_reload(current, &new_config, &changes))
}
```

If the new config fails invariant validation (e.g., `max_turn_usd >
max_plan_usd`), `load_config_unified` returns an error and `current` is
left unchanged.

---

## 5. Reload Policy

```rust
pub struct ReloadPolicy {
    pub debounce_ms: u64,                // Default: 500ms
    pub auto_reload: bool,               // Default: false
    pub reject_on_invariant_error: bool,  // Default: true
}
```

- **debounce_ms**: Minimum interval between reload attempts.
- **auto_reload**: When true, automatically apply changes on file modification.
  When false (default), changes require explicit `try_reload()` call.
- **reject_on_invariant_error**: When true (default), invariant errors abort
  the reload. When false, invariant errors are logged as warnings.

---

## 6. Config Watcher Integration

The CLI owns the filesystem watcher; `roko-core` deliberately does not depend
on `notify`. The callback trait provides the hook:

```rust
pub trait ConfigWatchCallback {
    fn on_config_changed(&self, result: HotReloadResult);
    fn on_config_error(&self, error: String);
}
```

A `ConfigReloadRequest` captures the full context of each reload:

```rust
pub struct ConfigReloadRequest {
    pub old_config: RokoConfig,
    pub new_config: RokoConfig,
    pub changes: Vec<ConfigChange>,
    pub requested_at: DateTime<Utc>,
}
```

---

## 7. Config Journal

Every reload and `config set` operation appends a timestamped JSON entry to
the config journal:

```rust
pub fn append_config_journal(
    journal_path: &Path,     // .roko/state/config-journal.jsonl
    changes: &[ConfigChange],
    source: &str,            // "hot_reload", "config_set", etc.
) -> io::Result<()>
```

Each entry contains:

```json
{
    "timestamp": "2026-09-15T12:00:00Z",
    "source": "hot_reload",
    "changes": [
        {"section": "budget", "summary": "budget: plan $0.00 -> $50.00, ..."}
    ]
}
```

---

## 8. Freshness Tracking

Per-section review timestamps are persisted at
`.roko/state/config-freshness.json`:

```rust
pub struct ConfigFreshness {
    pub section_timestamps: HashMap<String, DateTime<Utc>>,
}
```

- `touch(section)` updates the timestamp to `Utc::now()`.
- `touch_changed(changes)` updates timestamps for all affected sections.
- `config_freshness_diagnostics()` warns when any tracked section exceeds
  the staleness threshold (default: 30 days).

The freshness data is loaded and saved to the canonical path
`.roko/state/config-freshness.json` beneath the workspace directory.
