# 21-config / 06 -- Environment Variable Registry

> Every hardcoded environment variable read across the workspace is catalogued
> in a central registry. The registry powers CLI documentation, secret
> redaction, and deprecation warnings.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## 1. Registry Structure

Each environment variable is described by an `EnvVarSpec`:

```rust
pub(crate) struct EnvVarSpec {
    pub(crate) name: &'static str,        // Variable name (e.g., "ANTHROPIC_API_KEY")
    pub(crate) sensitivity: Sensitivity,   // Public or Secret
    pub(crate) stability: Stability,       // Lifecycle stage
    pub(crate) section: &'static str,      // Functional category
    pub(crate) description: &'static str,  // Human-readable purpose
    pub(crate) aliases: &'static [&'static str],  // Deprecated names
}
```

### 1.1 Sensitivity

```rust
pub(crate) enum Sensitivity {
    Public,  // Safe to display value
    Secret,  // Redact value; show set/unset only
}
```

Secret variables include API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
`PERPLEXITY_API_KEY`, `GEMINI_API_KEY`, etc.), authentication tokens, and
any value that would be dangerous to expose in logs or CLI output.

### 1.2 Stability

```rust
pub(crate) enum Stability {
    Stable,      // Documented, semver-protected
    Unstable,    // Works but may change or be removed
    Deprecated,  // Superseded by a canonical name; emits a warning when read
    BuildTime,   // Only meaningful during cargo build / build.rs
    TestOnly,    // Only read inside #[cfg(test)] or integration tests
}
```

---

## 2. Variable Categories

The registry organizes variables by functional section:

### Provider API keys (Secret)

| Variable | Provider | Notes |
|----------|----------|-------|
| `ANTHROPIC_API_KEY` | Anthropic | Claude API |
| `OPENAI_API_KEY` | OpenAI-compatible | OpenAI, Azure, etc. |
| `PERPLEXITY_API_KEY` | Perplexity | Research/search |
| `GEMINI_API_KEY` | Google Gemini | Gemini API |
| `CEREBRAS_API_KEY` | Cerebras | Fast inference |

### Config overrides (Public, Stable)

| Variable | Config field | Description |
|----------|-------------|-------------|
| `ROKO_MODEL` | `agent.default_model` | Override default LLM model |
| `ROKO_BACKEND` | `agent.default_backend` | Override default backend |
| `ROKO_EFFORT` | `agent.default_effort` | Override default effort level |
| `ROKO_CONTEXT_LIMIT_K` | `agent.context_limit_k` | Context window limit |
| `ROKO_MAX_AGENTS` | `conductor.max_agents` | Maximum concurrent agents |
| `ROKO_BUDGET_USD` | `budget.max_plan_usd` | Plan spending ceiling |
| `ROKO_PARALLEL` | `conductor.parallel_enabled` | Enable parallel execution |
| `ROKO_EXPRESS` | `conductor.express_mode` | Express mode toggle |
| `ROKO_SKIP_TESTS` | `gates.skip_tests` | Skip test gates |
| `ROKO_CLIPPY` | `gates.clippy_enabled` | Enable clippy gate |
| `ROKO_CONFIG` | -- | Override config file path |

### Hierarchical overrides (Public, Stable)

The `ROKO__SECTION__FIELD` convention provides generic config overrides:

```
ROKO__CONDUCTOR__MAX_AGENTS=8       -> conductor.max_agents = 8
ROKO__BUDGET__MAX_PLAN_USD=200      -> budget.max_plan_usd = 200.0
ROKO__ROUTING__COST_WEIGHT=0.4      -> routing.cost_weight = 0.4
```

### Runtime variables (Public)

| Variable | Description |
|----------|-------------|
| `ROKO_LOG` | Log level filter (tracing-subscriber) |
| `ROKO_DATA_DIR` | Override `.roko/` data directory |
| `ROKO_NO_COLOR` | Disable colored output |

---

## 3. Deprecation Handling

When a deprecated variable is read, the registry emits a tracing warning:

```
WARN deprecated env var 'OLD_NAME' used; prefer 'NEW_NAME'
```

The alias list on each `EnvVarSpec` maps old names to their canonical
replacement. The value is still honored -- deprecation is a warning, not
an error.

---

## 4. CLI Integration

### List all variables

```bash
roko config env list           # Human-readable table
roko config env list --json    # Machine-readable JSON
```

Output includes name, section, sensitivity (values of Secret entries are
shown as `[SET]` or `[UNSET]`), stability, and description.

### Check secrets

```bash
roko config check-secrets
```

Reports which configured provider API key variables are set, unset, or
invalid. Uses the `Sensitivity::Secret` classification -- actual values
are never printed.

---

## 5. Adding a New Variable

1. Add an `EnvVarSpec` entry to the appropriate category function in
   `crates/roko-core/src/config/env_registry.rs`.
2. Specify correct `sensitivity` (Secret if it holds credentials) and
   `stability` (Stable if documented and semver-protected).
3. Run the source-comparison check to confirm the literal is covered.
4. The variable automatically appears in `roko config env list`.
