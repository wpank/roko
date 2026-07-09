# Config System — IDE Integration Design

## Current State

The IDE uses a separate config at `~/.nunchi/roko/roko.toml`, isolated from
the user's `~/.roko/config.toml`. This works but creates duplication and drift.

### Why the IDE uses a separate config

1. `~/.roko/config.toml` may have CLI-specific settings (serve port, auth)
2. The IDE needs `bare_mode = true` to suppress workspace features
3. The IDE may want different provider routing (e.g., avoid openrouter if no key)
4. The IDE manages its own `--no-serve` lifecycle

### Current config contents (`~/.nunchi/roko/roko.toml`)

```toml
config_version = 2
schema_version = 2

[project]
name = "nunchi-ide-local"

[server]
bind = "127.0.0.1"
port = 6677

[serve]
port = 6677
auto_orchestrate = true

[serve.auth]
enabled = false

[agent]
command = "cat"
model = "sonnet"
bare_mode = true
timeout_ms = 300000

[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[models.sonnet]
provider = "openai"
slug = "gpt-4o"
supports_tools = true
context_window = 128000
max_output = 16000

[models.haiku]
provider = "openai"
slug = "gpt-4o-mini"
supports_tools = true
context_window = 128000
max_output = 16000
```

## Issues

### 1. Provider duplication

The user defines providers in `~/.roko/config.toml`. The IDE re-defines them
in its own config. When the user adds a new provider or changes an API key,
they must update both files.

### 2. `agent.model` must match `[models.*]` keys

If the IDE config has `model = "sonnet"` but no `[models.sonnet]` section,
the default is non-deterministic (see issue #03).

### 3. IDE-specific fields pollute shared config

Fields like `bare_mode`, `command = "cat"`, `serve.auth.enabled = false` are
IDE-specific and shouldn't leak into user's CLI config.

### 4. `max_output` default is too low

The default `max_output` (900 tokens) is designed for CLI quick answers.
IDE agents need much more (16000+). This must be overridden in every IDE config.

## Proposed Design: Config Layering with Profiles

### Concept

Instead of separate config files, use a profile system within one config:

```toml
config_version = 2

# Shared: providers and models (used by all consumers)
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[models.sonnet]
provider = "openai"
slug = "gpt-4o"
supports_tools = true
context_window = 128000
max_output = 16000

# Default agent settings (used by CLI)
[agent]
model = "sonnet"
timeout_ms = 120000

# IDE profile overrides
[profiles.ide]
bare_mode = true
timeout_ms = 300000
max_output = 16000
auto_orchestrate = false
```

### Resolution

```
roko acp --profile ide  →  merge [agent] with [profiles.ide]
roko (CLI)              →  use [agent] directly
```

### Implementation

In `crates/roko-core/src/config/loader.rs`:

```rust
pub struct LoadOptions {
    pub config_path: Option<PathBuf>,
    pub profile: Option<String>,      // NEW
    pub merge_global: bool,
}

impl RokoConfig {
    pub fn with_profile(mut self, profile: &str) -> Self {
        if let Some(overrides) = self.profiles.get(profile) {
            self.agent.merge(overrides);
        }
        self
    }
}
```

### Benefits

1. One file to maintain providers/models
2. IDE-specific settings isolated in a profile
3. CLI and IDE share the same credential resolution
4. Adding a new model works everywhere immediately
5. `roko config show --profile ide` shows effective config

## Alternative: Command-Line Overrides

If profiles are too heavy, allow key overrides via CLI:

```bash
roko acp --set agent.bare_mode=true --set agent.timeout_ms=300000
```

This is simpler but less ergonomic for repeated use.

## Recommended Short-Term Fix

Until profiles are implemented, the current separate-config approach works.
But fix these issues in roko:

1. **Document `max_output` default** and recommend 16000+ for IDE use
2. **Make `bare_mode` properly suppress workspace noise** (currently some
   workspace commands still appear in `availableCommands`)
3. **Allow `--config` to merge with global** instead of replacing it:
   ```bash
   roko acp --config ~/.nunchi/roko/roko.toml --merge-global
   ```
   This way the IDE config only needs overrides, inheriting providers/models
   from `~/.roko/config.toml`.

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-core/src/config/schema.rs` | Add `profiles: HashMap<String, AgentOverrides>` |
| `crates/roko-core/src/config/loader.rs` | Add profile resolution in `load()` |
| `crates/roko-cli/src/commands/acp.rs` | Add `--profile` flag |
| `crates/roko-core/src/config/schema.rs` | Change `max_output` default to 4096 |
