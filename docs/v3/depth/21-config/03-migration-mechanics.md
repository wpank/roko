# 21-config / 03 -- Migration Mechanics

> How schema migrations transform TOML value trees before deserialization.
> Migrations run on `toml::Value`, not on deserialized structs, so renamed
> fields are never silently discarded by serde.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## 1. The Migration Chain

`ConfigMigrator` maintains an ordered `BTreeMap<u32, MigrationFn>` mapping
schema version N to its N->N+1 transform:

```rust
pub(crate) type MigrationFn = fn(&mut toml::Value) -> Result<(), String>;

pub(crate) struct ConfigMigrator {
    pub(crate) migrations: BTreeMap<u32, MigrationFn>,
    pub(crate) target_version: u32,  // CURRENT_SCHEMA_VERSION (currently 2)
}

impl ConfigMigrator {
    fn new() -> Self {
        let mut migrator = Self {
            migrations: BTreeMap::new(),
            target_version: CURRENT_SCHEMA_VERSION,
        };
        migrator.register(1, migrate_v1_to_v2);
        migrator
    }
}
```

### Migration Report

Each migration produces a `MigrationReport`:

```rust
pub(crate) struct MigrationReport {
    pub(crate) from_version: u32,
    pub(crate) to_version: u32,
    pub(crate) steps_applied: Vec<MigrationStep>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) struct MigrationStep {
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) description: String,
}
```

The chain walks from the detected version to the target. If a migration edge
is missing or fails, the chain stops and the report retains partial progress
with warnings. The live loader rejects reports that do not reach
`target_version`.

---

## 2. Version Detection

Schema version is extracted from the raw `toml::Value`:

```rust
let from_version = toml_value
    .get("schema_version")
    .and_then(toml::Value::as_integer)
    .and_then(|v| u32::try_from(v).ok())
    .unwrap_or(1);  // Missing schema_version defaults to v1
```

A separate `text_has_config_version()` function checks whether raw TOML text
contains an explicit `config_version` key to avoid spurious version-1 warnings
for partial configs (e.g., global `~/.roko/config.toml`) that legitimately
omit the field.

---

## 3. The v1->v2 Migration

The built-in v1->v2 migration performs six categories of transforms:

### 3.1 Agent Field Renames

```
agent.model    -> agent.default_model
agent.backend  -> agent.default_backend
agent.effort   -> agent.default_effort
```

Uses `rename_if_absent()`: if the new name already exists, the old name is
simply removed. Otherwise the old value is moved to the new name.

### 3.2 Budget Field Renames

```
budget.max_session_usd -> budget.max_plan_usd
budget.max_agent_usd   -> budget.max_turn_usd
```

### 3.3 Legacy Gate Array Migration

Top-level `[[gate]]` entries (tagged enum with `kind` field) become
`[[gates.rungs]]`:

```toml
# Before (v1):
[[gate]]
kind = "compile"
build_system = "cargo"
timeout_ms = 60000

# After (v2):
[[gates.rungs]]
name = "compile"
command = "cargo check --workspace"
timeout_secs = 60
required = true
```

The migration handles four gate kinds:

| Kind | Generated command |
|------|------------------|
| `shell` | Reconstructed from `program` + `args` with shell quoting |
| `compile` | `cargo check --workspace` / `npm run build` / `go build ./...` |
| `clippy` | `cargo clippy --workspace --no-deps -- -D warnings` |
| `test` | `cargo test --workspace` / `npm test` / `go test ./...` |

`timeout_ms` is converted to `timeout_secs` only when the value is an exact
multiple of 1000. Non-lossless values cause the migration to reject that gate
entry. Unknown build systems log a warning and skip the entry.

Gate rungs are only inserted if no existing `rungs` or `custom_rungs` key
exists in `[gates]`.

### 3.4 Dead Section Removal

The `[isfr]` section (dead rate-oracle vestige) is unconditionally removed.

### 3.5 Version Stamps

Both `schema_version` and `config_version` are set to 2.

---

## 4. Mori Format Conversion

Separate from the schema migration chain, `from_mori_toml()` in `compat.rs`
handles the older flat Mori `ConfigState` format. This maps flat fields like
`codex_default_model`, `cursor_default_model`, `role_models`, `clippy_enabled`,
`max_agents`, etc. into the hierarchical `RokoConfig` structure.

This is a one-way conversion -- the result can be written as a `roko.toml` but
the original Mori format is not preserved.

---

## 5. Adding a New Migration

To add a v2->v3 migration:

1. **Bump `CURRENT_SCHEMA_VERSION`** in `schema.rs` from 2 to 3.
2. **Write the migration function**:
   ```rust
   fn migrate_v2_to_v3(value: &mut toml::Value) -> Result<(), String> {
       let root = value.as_table_mut()
           .ok_or_else(|| "config root must be a TOML table".to_string())?;
       // Transform fields...
       root.insert("schema_version".to_string(), toml::Value::Integer(3));
       root.insert("config_version".to_string(), toml::Value::Integer(3));
       Ok(())
   }
   ```
3. **Register** in `ConfigMigrator::new()`:
   ```rust
   migrator.register(2, migrate_v2_to_v3);
   ```
4. **Test the chain**:
   ```rust
   #[test]
   fn v2_config_migrates_to_v3() {
       let toml_text = "schema_version = 2\nconfig_version = 2\n...";
       let mut value = toml_text.parse::<toml::Value>().unwrap();
       let migrator = ConfigMigrator::new();
       let report = migrator.migrate(&mut value);
       assert_eq!(report.to_version, 3);
       assert!(report.warnings.is_empty());
   }
   ```
5. **CLI migration**: `roko config migrate [--dry-run] [--yes]` runs the chain
   and optionally rewrites the file.

---

## 6. CLI Migration Command

```bash
# Preview what would change
roko config migrate --dry-run

# Apply migration and rewrite roko.toml
roko config migrate --yes

# Interactive (prompts for confirmation)
roko config migrate
```

The CLI reads the raw file, runs the migrator, and writes back the result.
`--dry-run` prints the migration report without modifying the file.
