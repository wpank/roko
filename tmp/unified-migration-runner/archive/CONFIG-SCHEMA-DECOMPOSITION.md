# config/schema.rs Decomposition — Implementation Prompt

> **Goal**: Split `crates/roko-core/src/config/schema.rs` (6,061 lines, 60 impl blocks)
> into focused section files. Zero behavior change.

## Context

`schema.rs` defines the entire `RokoConfig` tree. Every config section (`[project]`,
`[agent]`, `[server]`, `[budget]`, `[compose]`, `[learning]`, `[chain]`, `[deploy]`,
`[serve]`, etc.) is a struct with serde derive and impl block, all in one file.

### Files to read first
```
crates/roko-core/src/config/schema.rs  — the 6,061-line file
crates/roko-core/src/config/mod.rs     — config module structure
```

---

## Tasks

### CS001 — Map all config section types

**Steps**:
1. `grep -n 'pub struct.*Config' crates/roko-core/src/config/schema.rs` to find all types
2. List each struct, its line range, field count
3. Group by logical section: project, agent, server, budget, compose, learning, etc.
4. Write map to `tmp/refactoring/config-schema-map.md`

---

### CS002 — Extract RokoConfig root + from_toml/to_toml

**Objective**: Keep `schema.rs` as the root file with just `RokoConfig` and its core methods.

**Keeps** in schema.rs:
- `pub struct RokoConfig` (with section fields)
- `from_toml()`, `to_toml()`, `to_toml_pretty()`, `is_stale()`
- `effective_providers()`, `effective_models()`
- Constants: `CURRENT_SCHEMA_VERSION`, `CURRENT_CONFIG_VERSION`
- Imports from new submodules

---

### CS003 — Extract agent config types

**Objective**: Move to `crates/roko-core/src/config/agent.rs`.

**Moves**: `AgentConfig`, `AgentRoleConfig`, `TierModelConfig`, `PoolConfig`, related impls.

---

### CS004 — Extract server/serve config types

**Objective**: Move to `crates/roko-core/src/config/server.rs`.

**Moves**: `ServerConfig`, `ServeConfig`, `ServeAuthConfig`, `CorsConfig`, related impls.

---

### CS005 — Extract budget config

**Objective**: Move to `crates/roko-core/src/config/budget.rs`.

**Moves**: `BudgetConfig`, budget-related impls and helpers.

---

### CS006 — Extract learning/compose/conductor config

**Objective**: Move to `crates/roko-core/src/config/learning.rs`.

**Moves**: `LearningConfig`, `ComposeConfig`, `ConductorConfig`, `ExperimentConfig`.

---

### CS007 — Extract deploy/chain/relay config

**Objective**: Move to `crates/roko-core/src/config/deploy.rs`.

**Moves**: `DeployConfig`, `ChainConfig`, `RelayConfig`, provider configs.

---

### CS008 — Extract provider config

**Objective**: Move to `crates/roko-core/src/config/providers.rs`.

**Moves**: `ProviderConfig`, `ModelConfig`, provider-related impls.

---

### CS009 — Move tests to per-module test files

**Steps**:
1. Identify all `#[cfg(test)]` blocks in schema.rs
2. Move each test to the relevant submodule's test block
3. Ensure all tests pass

**Verification**:
```bash
cargo test -p roko-core -- config
wc -l crates/roko-core/src/config/schema.rs  # should be <1000 lines
```

---

## Expected Result

```
crates/roko-core/src/config/
  mod.rs          — re-exports
  schema.rs       — RokoConfig root + from_toml (~800 lines)
  agent.rs        — agent/role/pool config (~600 lines)
  server.rs       — server/serve/auth config (~500 lines)
  budget.rs       — budget config (~300 lines)
  learning.rs     — learning/compose/conductor/experiment config (~700 lines)
  deploy.rs       — deploy/chain/relay config (~400 lines)
  providers.rs    — provider/model config (~500 lines)
```
