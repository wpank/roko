# W0-A: IndexMap Migration

## What This Is

The roko codebase stores LLM provider and model configurations in `HashMap<String, _>`.
HashMap iteration order in Rust is **non-deterministic** — the same config file produces
different default model selections across process restarts. This breaks IDE integration.

`IndexMap` preserves insertion order (= TOML declaration order in the config file), making
defaults deterministic.

## Workspace Layout

```
/Users/will/dev/nunchi/roko/roko/          ← workspace root
├── Cargo.toml                              ← workspace manifest (has [workspace.dependencies])
├── crates/
│   ├── roko-core/                          ← config schema lives here
│   │   ├── Cargo.toml
│   │   └── src/config/
│   │       ├── schema.rs                   ← RokoConfig struct (MAIN TARGET)
│   │       ├── loader.rs                   ← merge_global_into + interpolate
│   │       └── registry.rs                 ← ModelRegistry
│   ├── roko-agent/                         ← dispatch_resolver, provider/mod.rs
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── dispatch_resolver.rs
│   │       └── provider/mod.rs
│   ├── roko-cli/                           ← config.rs, model_selection, plan_validate, config_cmd
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── config.rs
│   │       ├── model_selection.rs
│   │       ├── plan_validate.rs
│   │       └── commands/config_cmd.rs
│   ├── roko-learn/                         ← cost_table.rs
│   │   ├── Cargo.toml
│   │   └── src/cost_table.rs
│   ├── roko-serve/                         ← routes/providers.rs
│   │   ├── Cargo.toml
│   │   └── src/routes/providers.rs
│   └── roko-acp/                           ← session.rs, bridge_events.rs (consumers)
│       └── src/
│           ├── session.rs
│           └── bridge_events.rs
└── tests/                                  ← plan_validation.rs (test file)
```

## Sub-Tasks (can be parallelized after W0-A-1)

### W0-A-1: Add indexmap dependency (MUST BE FIRST)

**File: `/Users/will/dev/nunchi/roko/roko/Cargo.toml`** (workspace root)

Find the `[workspace.dependencies]` section (around line 110-175). Add this line
alphabetically (after `indicatif` and before `jsonwebtoken`):

```toml
indexmap = { version = "2", features = ["serde"] }
```

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/Cargo.toml`**

Current content (lines 13-29):
```toml
[dependencies]
roko-primitives = { path = "../roko-primitives" }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
blake3 = "1"
ed25519-dalek = "2"
async-trait = "0.1"
futures-core = "0.3"
futures-util = { workspace = true }
parking_lot = "0.12"
regex = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
rkyv = { version = "0.8", optional = true }
```

Add after the `futures-util` line:
```toml
indexmap = { workspace = true }
```

**Also add to these crate Cargo.tomls** (same pattern: `indexmap = { workspace = true }`
in their `[dependencies]` section):
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml`

---

### W0-A-2: Change schema.rs (the core type change)

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`**

**Change 1 — Line 10 (imports):**

FIND:
```rust
use std::collections::{HashMap, HashSet};
```

REPLACE WITH:
```rust
use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;
```

(Keep HashMap — it's still used for other fields like `extra_headers`, `tier_models`, etc.)

**Change 2 — Line 71 (providers field):**

FIND:
```rust
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
```

REPLACE WITH:
```rust
    #[serde(default)]
    pub providers: IndexMap<String, ProviderConfig>,
```

**Change 3 — Line 73 (models field):**

FIND:
```rust
    #[serde(default)]
    pub models: HashMap<String, ModelProfile>,
```

REPLACE WITH:
```rust
    #[serde(default)]
    pub models: IndexMap<String, ModelProfile>,
```

**Change 4 — Lines 133-134 (Default impl):**

FIND:
```rust
            providers: HashMap::new(),
            models: HashMap::new(),
```

REPLACE WITH:
```rust
            providers: IndexMap::new(),
            models: IndexMap::new(),
```

**Change 5 — Line 204 (effective_providers return type):**

FIND:
```rust
    pub fn effective_providers(&self) -> HashMap<String, ProviderConfig> {
```

REPLACE WITH:
```rust
    pub fn effective_providers(&self) -> IndexMap<String, ProviderConfig> {
```

**Change 6 — Line 222 (empty return in effective_providers):**

FIND:
```rust
        HashMap::new()
```

REPLACE WITH:
```rust
        IndexMap::new()
```

**Change 7 — Line 232 (effective_models return type):**

FIND:
```rust
    pub fn effective_models(&self) -> HashMap<String, ModelProfile> {
```

REPLACE WITH:
```rust
    pub fn effective_models(&self) -> IndexMap<String, ModelProfile> {
```

**Change 8 — Line 499 (interpolate_env_vars_with parameter):**

FIND:
```rust
    fn interpolate_env_vars_with(
        providers: &mut HashMap<String, ProviderConfig>,
        env_fn: &dyn Fn(&str) -> Option<String>,
    ) {
```

REPLACE WITH:
```rust
    fn interpolate_env_vars_with(
        providers: &mut IndexMap<String, ProviderConfig>,
        env_fn: &dyn Fn(&str) -> Option<String>,
    ) {
```

---

### W0-A-3: Fix roko-agent references

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatch_resolver.rs`**

Find imports at top and add:
```rust
use indexmap::IndexMap;
```

Line 156 — FIND:
```rust
    providers: &'a HashMap<String, ProviderConfig>,
```
REPLACE WITH:
```rust
    providers: &'a IndexMap<String, ProviderConfig>,
```

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs`**

Find imports at top and add:
```rust
use indexmap::IndexMap;
```

Line 392 — FIND:
```rust
    providers: &HashMap<String, ProviderConfig>,
```
REPLACE WITH:
```rust
    providers: &IndexMap<String, ProviderConfig>,
```

Line 416 — FIND:
```rust
    pub fn new(configs: &HashMap<String, ProviderConfig>) -> Self {
```
REPLACE WITH:
```rust
    pub fn new(configs: &IndexMap<String, ProviderConfig>) -> Self {
```

---

### W0-A-4: Fix roko-cli references

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs`**

Find imports at top and add `use indexmap::IndexMap;`

Line 65 — FIND:
```rust
    pub providers: HashMap<String, ProviderConfig>,
```
REPLACE WITH:
```rust
    pub providers: IndexMap<String, ProviderConfig>,
```

Line 68 — FIND:
```rust
    pub models: HashMap<String, ModelProfile>,
```
REPLACE WITH:
```rust
    pub models: IndexMap<String, ModelProfile>,
```

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs`**

Add `use indexmap::IndexMap;` to imports.

Lines 339-340 — FIND:
```rust
    providers: &'a HashMap<String, ProviderConfig>,
```
REPLACE WITH:
```rust
    providers: &'a IndexMap<String, ProviderConfig>,
```

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_validate.rs`**

Add `use indexmap::IndexMap;` to imports.

Lines 102, 114, 122, 269, 941 — every occurrence of:
```rust
    models: Option<&HashMap<String, ModelProfile>>,
```
or:
```rust
    known_models: &HashMap<String, ModelProfile>,
```
REPLACE `HashMap` with `IndexMap` in each.

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`**

Lines 1043, 1065, 2027, 2049, 2080 — these use fully-qualified `std::collections::HashMap<String, ProviderConfig>` and `std::collections::HashMap<String, ModelProfile>`.

REPLACE each `std::collections::HashMap<String, ProviderConfig>` with `indexmap::IndexMap<String, ProviderConfig>`.
REPLACE each `std::collections::HashMap<String, ModelProfile>` with `indexmap::IndexMap<String, ModelProfile>`.

Or add `use indexmap::IndexMap;` at top and use `IndexMap<String, ...>` directly.

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/plan_validation.rs`**

Lines 38, 44 — REPLACE `HashMap<String, ModelProfile>` with `IndexMap<String, ModelProfile>`.
Add `use indexmap::IndexMap;` to imports.

---

### W0-A-5: Fix roko-learn + roko-serve + roko-core/registry references

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/registry.rs`**

Line 21 — FIND:
```rust
    profiles: HashMap<String, ModelProfile>,
```
REPLACE WITH:
```rust
    profiles: IndexMap<String, ModelProfile>,
```

Add `use indexmap::IndexMap;` to imports.

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cost_table.rs`**

Line 111 — FIND:
```rust
    pub fn from_config(models: &HashMap<String, ModelProfile>) -> Self {
```
REPLACE WITH:
```rust
    pub fn from_config(models: &IndexMap<String, ModelProfile>) -> Self {
```

Add `use indexmap::IndexMap;` to imports.

**File: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/providers.rs`**

Line 571 — FIND:
```rust
    models: &HashMap<String, ModelProfile>,
```
REPLACE WITH:
```rust
    models: &IndexMap<String, ModelProfile>,
```

Add `use indexmap::IndexMap;` to imports.

---

## What NOT to Change

- `HashMap<String, String>` for `tier_models`, `extra_headers`, `roles` — these don't need ordered iteration
- `HashMap` used in local variables for slug deduplication (loader.rs:241)
- `HashMap` in bridge_events.rs for tool handlers (`handlers: HashMap<String, Arc<dyn ToolHandler>>`)
- `HashSet` imports — keep those

## API Compatibility Note

`IndexMap` implements: `.get()`, `.contains_key()`, `.insert()`, `.entry()`, `.iter()`,
`.keys()`, `.values()`, `.values_mut()`, `.len()`, `.is_empty()`, `.clone()`,
`.into_iter()` — all methods used on these fields. No call-site changes needed.

The only API difference: `IndexMap::new()` instead of `HashMap::new()`.

## Verification (after ALL waves complete, Phase 2)

```bash
cargo build --workspace 2>&1 | head -50
# Should compile cleanly — if not, fix remaining HashMap references
```

## Estimated Effort

30-45 minutes total across all sub-tasks. Each sub-task is ~10 minutes of mechanical find-replace.
