# 21-config / 02 -- Provenance Internals

> How the loader tracks where every config value came from. The provenance
> system makes config resolution auditable: for any field in the effective
> config, tooling can report which layer provided it and why.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## 1. Source Priority

`ConfigSource` defines nine source categories with explicit priority ordering:

```rust
impl ConfigSource {
    pub const fn priority(&self) -> u8 {
        match self {
            Self::ApiOverride => 5,     // Runtime HTTP API mutation
            Self::CliOverride => 4,     // --model, --budget CLI flags
            Self::Env => 3,             // ROKO_* and ROKO__*__* env vars
            Self::File | Self::LocalOverride => 2,  // roko.toml files
            Self::Evolved | Self::Migration => 1,   // Learned/migrated values
            Self::Default | Self::Composed => 0,    // Compiled defaults
        }
    }
}
```

File and LocalOverride share priority 2 but differ in trust: `LocalOverride`
permits safety-sensitive settings like `dangerously_skip_permissions` that
`File` (shared config) would reject under strict validation.

A tie-breaker ordering distinguishes sources at the same priority for
deterministic `Ord` implementation:

```
Default(0) < Composed(1) < Migration(2) < Evolved(3) < File(4) <
LocalOverride(5) < Env(6) < CliOverride(7) < ApiOverride(8)
```

---

## 2. Provenance Structures

### ConfigProvenance

Machine-readable trace retained in `ValidatedConfig::provenance`:

```rust
pub struct ConfigProvenance {
    pub source: ConfigSource,
    pub path: Option<PathBuf>,     // File path when source is File/LocalOverride
    pub key: String,               // Dotted config path
    pub reason: Option<String>,    // Human-readable explanation
}
```

Constructors for each source category:

| Constructor | Source | Path | Example |
|-------------|--------|------|---------|
| `ConfigProvenance::file(path, key)` | File | Yes | `file("roko.toml", "providers.anthropic.kind")` |
| `ConfigProvenance::default(key, reason)` | Default | No | `default("agent.default_model", "built-in fallback")` |
| `ConfigProvenance::migration(key, reason)` | Migration | No | `migration("schema_version", "migrated v1 to v2")` |
| `ConfigProvenance::env(key, reason)` | Env | No | `env("agent.default_model", "ROKO_MODEL present")` |
| `ConfigProvenance::local_override(path, key, reason)` | LocalOverride | Yes | Override from `.roko/local-overrides.toml` |
| `ConfigProvenance::cli_override(key, reason)` | CliOverride | No | `cli_override("agent.default_model", "--model")` |
| `ConfigProvenance::evolved(key, reason)` | Evolved | No | `evolved("routing.weights.cost", "learned reward")` |
| `ConfigProvenance::api_override(key, reason)` | ApiOverride | No | `api_override("budget.max_plan_usd", "operator request")` |

### FieldProvenance

Runtime-only per-field resolution metadata in `MergeContext`:

```rust
pub struct FieldProvenance {
    pub key: String,
    pub value_source: ConfigSource,
    pub priority: u8,
    pub reason: Option<String>,
}
```

### MergeContext

Tracks the winning source for each config field:

```rust
pub struct MergeContext {
    pub field_provenance: Vec<FieldProvenance>,
}

impl MergeContext {
    pub fn record(&mut self, provenance: FieldProvenance) {
        // Replace existing entry only when new source has >= priority.
        // Equal-priority sources applied in specificity order by the loader.
    }
}
```

---

## 3. The Merge Algorithm in Detail

The loader records provenance at each step:

### Step 1: File source

When `roko.toml` exists, all explicit leaf paths are recorded as `File`
provenance. When no file exists, a single `Default` sentinel is recorded.

### Step 2: Global merge

Before and after merging `~/.roko/config.toml`, the loader snapshots the
config, serializes both to `toml::Value`, and diffs every leaf path. Changed
paths are recorded as `File` provenance with reason `"global config merge"`.

After the global merge, explicit project fields are re-applied to prevent
shadowing. The re-application function (`reapply_explicit_fields`) walks the
list of paths that were explicitly set in the project config and restores
their values from the pre-merge snapshot.

### Step 3: Named env overrides

Same before/after diff. Changed paths recorded as `Env` with reason
`"named ROKO_* environment override"`.

### Step 4: Hierarchical env overrides

Changed paths recorded as `Env` with reason
`"hierarchical ROKO__* environment override"`.

### Step 5: Migration steps

Each `MigrationStep` in the `MigrationReport` becomes a `Migration`
provenance entry on `"schema_version"`.

---

## 4. TOML Diff Algorithm

The loader uses `flatten_toml()` to convert a `toml::Value` tree into a
`BTreeMap<String, toml::Value>` of dotted paths to leaf values. Two flattened
trees are compared key-by-key; any path present in one but not the other, or
present in both with different values, appears in the diff.

```rust
fn changed_toml_paths(before: &toml::Value, after: &toml::Value) -> Vec<String> {
    let mut before_values = BTreeMap::new();
    let mut after_values = BTreeMap::new();
    flatten_toml(before, "", &mut before_values);
    flatten_toml(after, "", &mut after_values);
    // Union of all keys, filtered to those where values differ
}
```

---

## 5. ValidatedConfig

The final output of `load_config_validated()` bundles everything:

```rust
pub struct ValidatedConfig {
    pub raw: RokoConfig,                    // Parsed-only config (before runtime layers)
    pub migrated: RokoConfig,               // Fully resolved config (post all layers)
    pub diagnostics: Vec<ConfigDiagnostic>, // Soft warnings
    pub provenance: Vec<ConfigProvenance>,  // Structured provenance trail
    pub merge_context: MergeContext,        // Runtime-only field resolution metadata
}
```

- `config()` returns `&migrated` (the authoritative value).
- `into_config()` consumes the wrapper and returns `migrated`.
- `diagnostics()` returns soft warnings (hard errors are `LoadConfigError`).
- `provenance()` returns the structured provenance trail.

Callers that don't need provenance (most runtime paths) use
`load_config_unified()` which returns a bare `RokoConfig`. Callers that need
the audit trail (CLI `config show`, `config validate`, dashboard) use
`load_config_validated()`.
