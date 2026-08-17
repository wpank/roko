# W5-A: Config Profile System (Lower Priority)

## Why

The IDE uses a separate `~/.nunchi/roko/roko.toml` that duplicates providers/models from
the user's `~/.roko/config.toml`. When the user adds a new provider or changes an API key,
they must update both files. This creates drift and maintenance burden.

## Design Options

### Option A: Profile System (full)

Add `[profiles.ide]` sections to roko.toml:

```toml
config_version = 2

# Shared across all consumers
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[models.sonnet]
provider = "openai"
slug = "gpt-4o"
max_output = 16000

# CLI defaults
[agent]
model = "sonnet"
timeout_ms = 120000

# IDE overrides
[profiles.ide]
bare_mode = true
timeout_ms = 300000
```

Usage: `roko acp --profile ide`

Resolution: `[agent]` merged with `[profiles.ide]`, profile wins on conflicts.

### Option B: --merge-global (simpler)

Keep separate IDE config but allow inheriting providers/models from global:

```bash
roko acp --config ~/.nunchi/roko/ide.toml --merge-global
```

The IDE config only needs:
```toml
config_version = 2
[agent]
bare_mode = true
timeout_ms = 300000
# providers and models inherited from ~/.roko/config.toml
```

This is simpler than profiles but less flexible.

### Option C: --set overrides (simplest)

```bash
roko acp --set agent.bare_mode=true --set agent.timeout_ms=300000
```

No config file changes needed. But less ergonomic for repeated use.

## Recommended: Option A + B

Implement profiles (Option A) as the long-term solution. Add `--merge-global` (Option B)
as a quick win that works immediately.

## Implementation Sketch (Option A)

### 1. Schema change

**File:** `crates/roko-core/src/config/schema.rs`

```rust
pub struct RokoConfig {
    // ...existing fields...
    /// Named configuration profiles for different consumers (IDE, CLI, CI).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub profiles: IndexMap<String, ProfileOverrides>,
}

/// Overrides that a profile can apply to the base config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bare_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_effort: Option<String>,
    // Keep it focused — only fields that vary between consumers
}
```

### 2. Profile application

**File:** `crates/roko-core/src/config/loader.rs`

```rust
impl RokoConfig {
    pub fn with_profile(mut self, profile_name: &str) -> Result<Self> {
        if let Some(overrides) = self.profiles.get(profile_name) {
            if let Some(bare_mode) = overrides.bare_mode {
                self.agent.bare_mode = bare_mode;
            }
            if let Some(timeout_ms) = overrides.timeout_ms {
                self.agent.timeout_ms = Some(timeout_ms);
            }
            if let Some(ref model) = overrides.default_model {
                self.agent.default_model = model.clone();
            }
            if let Some(ref effort) = overrides.default_effort {
                self.agent.default_effort = effort.clone();
            }
            Ok(self)
        } else {
            anyhow::bail!("profile '{}' not found in config", profile_name)
        }
    }
}
```

### 3. CLI flag

**File:** `crates/roko-cli/src/commands/acp.rs`

```rust
#[derive(Parser)]
pub struct AcpArgs {
    // ...existing args...
    /// Apply a named config profile (defined in [profiles.<name>] in roko.toml).
    #[arg(long)]
    pub profile: Option<String>,
}
```

In the ACP startup:
```rust
let mut config = load_config()?;
if let Some(ref profile) = args.profile {
    config = config.with_profile(profile)?;
}
```

## Implementation Sketch (Option B — --merge-global)

### 1. CLI flag

```rust
#[arg(long, help = "Merge providers/models from ~/.roko/config.toml")]
pub merge_global: bool,
```

### 2. In ACP startup

```rust
if args.merge_global {
    roko_core::config::loader::merge_global_into(&mut config);
}
```

This already exists in the loader! (loader.rs:358-401). The issue is that when `--config`
is specified, global merging is bypassed. The fix is to re-enable it when `--merge-global`
is also passed.

## Priority

Lower priority. The current separate-config approach works. This becomes important when:
- Users manage multiple providers and want one source of truth
- The IDE is distributed to users who already have roko configured

## Estimated Effort

- Option A (profiles): 2-3 hours
- Option B (--merge-global): 30 minutes
- Both: 3-4 hours
